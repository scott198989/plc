#!/usr/bin/env python3
"""Run the Phase 1 verifier against an externally trusted baseline manifest."""

from __future__ import annotations

import argparse
import hashlib
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

EXPECTED_PYTHON = (3, 13, 12)
EXPECTED_NODE = "v24.19.0"
MANIFEST_PROJECT_PATH = "tests/phase1/trusted-baseline.json"

EXIT_PASS = 0
EXIT_POLICY_FAILURE = 1
EXIT_TOOL_ERROR = 2


def error(message: str) -> int:
    print(f"ERROR VER-RUN-0001 {message}", file=sys.stderr)
    return EXIT_TOOL_ERROR


def candidate_executables(name: str, explicit_env: str, bundled_parts: tuple[str, ...]) -> list[Path]:
    candidates: list[Path] = []
    explicit = os.environ.get(explicit_env)
    if explicit:
        candidates.append(Path(explicit))

    from_path = shutil.which(name)
    if from_path:
        candidates.append(Path(from_path))
    candidates.append(Path.home().joinpath(*bundled_parts))

    unique: list[Path] = []
    seen: set[str] = set()
    for candidate in candidates:
        normalized = str(candidate.resolve(strict=False)).casefold()
        if normalized not in seen:
            seen.add(normalized)
            unique.append(candidate)
    return unique


def candidate_nodes() -> list[Path]:
    name = "node.exe" if os.name == "nt" else "node"
    return candidate_executables(
        "node",
        "PHASE1_NODE",
        (
            ".cache",
            "codex-runtimes",
            "codex-primary-runtime",
            "dependencies",
            "node",
            "bin",
            name,
        ),
    )


def candidate_gits() -> list[Path]:
    bundled = (
        ".cache",
        "codex-runtimes",
        "codex-primary-runtime",
        "dependencies",
        "native",
        "git",
        "cmd" if os.name == "nt" else "bin",
        "git.exe" if os.name == "nt" else "git",
    )
    return candidate_executables("git", "PHASE1_GIT", bundled)


def executable_version(candidate: Path, arguments: list[str]) -> str | None:
    if not candidate.is_file():
        return None
    try:
        result = subprocess.run(
            [str(candidate), *arguments],
            check=False,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if result.returncode != 0:
        return None
    return (result.stdout or result.stderr).strip()


def find_node() -> tuple[Path | None, list[str]]:
    observations: list[str] = []
    for candidate in candidate_nodes():
        version = executable_version(candidate, ["--version"])
        if version == EXPECTED_NODE:
            return candidate, observations
        if version:
            observations.append(version)
    return None, observations


def find_git() -> Path | None:
    for candidate in candidate_gits():
        if executable_version(candidate, ["--version"]):
            return candidate
    return None


def resolve_git_manifest(root: Path, git: Path, baseline_ref: str) -> tuple[bytes, str] | None:
    resolved = subprocess.run(
        [str(git), "rev-parse", "--verify", f"{baseline_ref}^{{commit}}"],
        cwd=root,
        check=False,
        capture_output=True,
        text=True,
        timeout=20,
    )
    commit = resolved.stdout.strip()
    if resolved.returncode != 0 or not re.fullmatch(r"[0-9a-fA-F]{40}", commit):
        return None

    shown = subprocess.run(
        [str(git), "show", f"{commit}:{MANIFEST_PROJECT_PATH}"],
        cwd=root,
        check=False,
        capture_output=True,
        timeout=20,
    )
    if shown.returncode != 0:
        return None
    return shown.stdout, commit.lower()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--baseline-ref", help="Git ref whose commit contains the trusted baseline manifest")
    source.add_argument("--baseline-manifest", type=Path, help="External manifest path for an isolated no-.git subject")
    parser.add_argument("--baseline-manifest-sha256", help="Required with --baseline-manifest")
    parser.add_argument("--baseline-commit", help="Required full commit ID with --baseline-manifest")
    return parser.parse_args()


def run_verifier(node: Path, root: Path, manifest: Path, manifest_sha256: str, baseline_commit: str) -> int:
    verifier = root / "tools" / "phase1" / "verify-phase1.mjs"
    try:
        completed = subprocess.run(
            [
                str(node),
                str(verifier),
                "--baseline-manifest",
                str(manifest),
                "--baseline-manifest-sha256",
                manifest_sha256,
                "--baseline-commit",
                baseline_commit,
            ],
            check=False,
            cwd=root,
            env={**os.environ, "PHASE1_PYTHON": sys.executable},
        )
    except OSError as exc:
        return error(f"Unable to start the Node verifier: {exc}")
    if completed.returncode not in {EXIT_PASS, EXIT_POLICY_FAILURE, EXIT_TOOL_ERROR}:
        return error(f"Node verifier returned undocumented exit code {completed.returncode}")
    return completed.returncode


def main() -> int:
    if sys.version_info[:3] != EXPECTED_PYTHON:
        actual = ".".join(str(part) for part in sys.version_info[:3])
        expected = ".".join(str(part) for part in EXPECTED_PYTHON)
        return error(f"Python runtime mismatch: expected {expected}, got {actual}")

    args = parse_args()
    root = Path(__file__).resolve().parents[2]
    node, node_observations = find_node()
    if node is None:
        observed = ", ".join(dict.fromkeys(node_observations)) or "none"
        return error(f"Node runtime mismatch: required {EXPECTED_NODE}; observed {observed}")

    if args.baseline_ref:
        git = find_git()
        if git is None:
            return error("Git is unavailable; cannot resolve the trusted baseline commit")
        try:
            resolved = resolve_git_manifest(root, git, args.baseline_ref)
        except (OSError, subprocess.TimeoutExpired) as exc:
            return error(f"Unable to resolve trusted baseline ref: {exc}")
        if resolved is None:
            return error(
                f"Baseline ref {args.baseline_ref!r} does not resolve to a commit containing {MANIFEST_PROJECT_PATH}"
            )
        manifest_bytes, baseline_commit = resolved
        manifest_sha256 = hashlib.sha256(manifest_bytes).hexdigest().upper()
        with tempfile.TemporaryDirectory(prefix="phase1-trusted-baseline-") as temporary:
            manifest = Path(temporary) / "trusted-baseline.json"
            manifest.write_bytes(manifest_bytes)
            return run_verifier(node, root, manifest, manifest_sha256, baseline_commit)

    if not args.baseline_manifest_sha256 or not args.baseline_commit:
        return error("--baseline-manifest requires --baseline-manifest-sha256 and --baseline-commit")
    if not re.fullmatch(r"[0-9A-Fa-f]{64}", args.baseline_manifest_sha256):
        return error("--baseline-manifest-sha256 must contain exactly 64 hexadecimal characters")
    if not re.fullmatch(r"[0-9A-Fa-f]{40}", args.baseline_commit):
        return error("--baseline-commit must contain exactly 40 hexadecimal characters")

    manifest = args.baseline_manifest.resolve(strict=False)
    try:
        manifest.relative_to(root)
    except ValueError:
        pass
    else:
        return error("The external trusted baseline manifest must be outside the subject repository")
    if not manifest.is_file():
        return error(f"External trusted baseline manifest is missing: {manifest}")
    return run_verifier(
        node,
        root,
        manifest,
        args.baseline_manifest_sha256.upper(),
        args.baseline_commit.lower(),
    )


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        raise SystemExit(error("Interrupted"))
