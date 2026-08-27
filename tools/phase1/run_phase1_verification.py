#!/usr/bin/env python3
"""Run the Phase 1 verifier with the exact declared local runtime versions."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

EXPECTED_PYTHON = (3, 13, 12)
EXPECTED_NODE = "v24.19.0"


def candidate_nodes() -> list[Path]:
    candidates: list[Path] = []
    explicit = os.environ.get("PHASE1_NODE")
    if explicit:
        candidates.append(Path(explicit))

    path_node = shutil.which("node")
    if path_node:
        candidates.append(Path(path_node))

    bundled_name = "node.exe" if os.name == "nt" else "node"
    candidates.append(
        Path.home()
        / ".cache"
        / "codex-runtimes"
        / "codex-primary-runtime"
        / "dependencies"
        / "node"
        / "bin"
        / bundled_name
    )

    unique: list[Path] = []
    seen: set[str] = set()
    for candidate in candidates:
        normalized = str(candidate.resolve(strict=False)).casefold()
        if normalized not in seen:
            seen.add(normalized)
            unique.append(candidate)
    return unique


def node_version(candidate: Path) -> str | None:
    if not candidate.is_file():
        return None
    try:
        result = subprocess.run(
            [str(candidate), "--version"],
            check=False,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except OSError:
        return None
    return result.stdout.strip() if result.returncode == 0 else None


def main() -> int:
    if sys.version_info[:3] != EXPECTED_PYTHON:
        actual = ".".join(str(part) for part in sys.version_info[:3])
        expected = ".".join(str(part) for part in EXPECTED_PYTHON)
        raise SystemExit(f"Python runtime mismatch: expected {expected}, got {actual}")

    observations: list[str] = []
    for candidate in candidate_nodes():
        version = node_version(candidate)
        if version == EXPECTED_NODE:
            root = Path(__file__).resolve().parents[2]
            verifier = root / "tools" / "phase1" / "verify-phase1.mjs"
            completed = subprocess.run(
                [str(candidate), str(verifier)],
                check=False,
                cwd=root,
            )
            return completed.returncode
        if version:
            observations.append(version)

    observed = ", ".join(dict.fromkeys(observations)) or "none"
    raise SystemExit(
        f"Node runtime mismatch: required {EXPECTED_NODE}; observed {observed}. "
        "Set PHASE1_NODE to the exact executable after toolchain review."
    )


if __name__ == "__main__":
    raise SystemExit(main())
