#!/usr/bin/env python3
"""Generate or check the deterministic Phase 1 trusted-baseline manifest."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
from pathlib import Path

EXCLUDED_ROOTS = sorted(
    {
        ".git",
        ".idea",
        ".phase1-verification",
        ".pnpm-store",
        ".vscode",
        "__pycache__",
        "coverage",
        "dist",
        "node_modules",
        "playwright-report",
        "target",
        "test-results",
    }
)
EXCLUDED_PATHS = ["apps/foundation-shell/src/generated"]
MANIFEST_PROJECT_PATH = "tests/phase1/trusted-baseline.json"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest().upper()


def collect_files(root: Path, output: Path) -> list[dict[str, object]]:
    entries: list[dict[str, object]] = []
    for current, directory_names, file_names in os.walk(root, topdown=True, followlinks=False):
        current_path = Path(current)
        directory_names[:] = sorted(
            name
            for name in directory_names
            if name not in EXCLUDED_ROOTS
            and (current_path / name).relative_to(root).as_posix() not in EXCLUDED_PATHS
        )
        for name in sorted(file_names):
            path = current_path / name
            if path.is_symlink():
                raise RuntimeError(f"Refusing to baseline symbolic link: {path}")
            if path.resolve(strict=False) == output.resolve(strict=False):
                continue
            relative = path.relative_to(root).as_posix()
            entries.append(
                {
                    "path": relative,
                    "bytes": path.stat().st_size,
                    "sha256": sha256(path),
                }
            )
    entries.sort(key=lambda item: str(item["path"]))
    return entries


def manifest_bytes(root: Path, output: Path, baseline_id: str) -> bytes:
    payload = {
        "schemaVersion": 1,
        "baselineId": baseline_id,
        "hashAlgorithm": "SHA-256",
        "manifestPath": MANIFEST_PROJECT_PATH,
        "scope": "exact-project-file-set",
        "excludedRoots": EXCLUDED_ROOTS,
        "excludedPaths": EXCLUDED_PATHS,
        "files": collect_files(root, output),
    }
    return (json.dumps(payload, indent=2, ensure_ascii=False) + "\n").encode("utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--baseline-id", default="phase1-closure-candidate-v1")
    parser.add_argument("--check", action="store_true", help="Compare without writing")
    parser.add_argument("--force", action="store_true", help="Replace an existing manifest")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = args.root.resolve(strict=True)
    output = args.output if args.output.is_absolute() else root / args.output
    output = output.resolve(strict=False)
    try:
        output.relative_to(root)
    except ValueError:
        pass
    else:
        if output.relative_to(root).as_posix() != MANIFEST_PROJECT_PATH:
            print(
                f"ERROR VER-INT-0001 In-repository output must be {MANIFEST_PROJECT_PATH}",
                file=sys.stderr,
            )
            return 2

    try:
        expected = manifest_bytes(root, output, args.baseline_id)
    except (OSError, RuntimeError) as exc:
        print(f"ERROR VER-INT-0001 Unable to construct baseline: {exc}", file=sys.stderr)
        return 2

    if args.check:
        if not output.is_file():
            print(f"FAIL VER-INT-0001 Trusted baseline manifest is missing: {output}")
            return 1
        actual = output.read_bytes()
        if actual != expected:
            print(f"FAIL VER-INT-0001 Trusted baseline manifest is stale: {output}")
            return 1
    else:
        if output.exists() and not args.force:
            print(f"ERROR VER-INT-0001 Refusing to replace existing manifest without --force: {output}", file=sys.stderr)
            return 2
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(expected)

    print(f"MANIFEST_PATH={output}")
    print(f"MANIFEST_SHA256={hashlib.sha256(expected).hexdigest().upper()}")
    print(f"FILE_COUNT={len(json.loads(expected)['files'])}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
