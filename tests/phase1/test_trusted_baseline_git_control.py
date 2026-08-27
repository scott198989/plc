#!/usr/bin/env python3
"""Regression checks for root-versus-nested .git baseline handling."""

from __future__ import annotations

import importlib.util
import tempfile
from pathlib import Path
from types import ModuleType


def load_generator(project_root: Path) -> ModuleType:
    path = project_root / "tools" / "phase1" / "generate_trusted_baseline.py"
    specification = importlib.util.spec_from_file_location("phase1_baseline_generator", path)
    if specification is None or specification.loader is None:
        raise RuntimeError(f"Unable to load baseline generator: {path}")
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


def collected_paths(generator: ModuleType, subject: Path, output: Path) -> set[str]:
    return {str(entry["path"]) for entry in generator.collect_files(subject, output)}


def main() -> int:
    project_root = Path(__file__).resolve().parents[2]
    generator = load_generator(project_root)

    with tempfile.TemporaryDirectory(prefix="phase1-git-control-regression-") as temporary:
        temporary_root = Path(temporary)
        subject = temporary_root / "subject"
        subject.mkdir()
        output = temporary_root / "trusted-baseline.json"

        (subject / "ordinary.txt").write_text("ordinary\n", encoding="utf-8")
        (subject / ".git").write_text("gitdir: external-control-directory\n", encoding="utf-8")
        nested = subject / "nested"
        nested.mkdir()
        (nested / ".git").write_text("ordinary nested file\n", encoding="utf-8")
        nested_directory = subject / "embedded" / ".git"
        nested_directory.mkdir(parents=True)
        (nested_directory / "config").write_text("ordinary nested directory file\n", encoding="utf-8")

        worktree_paths = collected_paths(generator, subject, output)
        if ".git" in worktree_paths:
            raise AssertionError("Root linked-worktree .git control file was baselined")
        if "nested/.git" not in worktree_paths:
            raise AssertionError("Nested .git file was incorrectly excluded")
        if "embedded/.git/config" not in worktree_paths:
            raise AssertionError("Nested .git directory was incorrectly excluded")

        (subject / ".git").unlink()
        root_git_directory = subject / ".git"
        root_git_directory.mkdir()
        (root_git_directory / "config").write_text("root control metadata\n", encoding="utf-8")

        checkout_paths = collected_paths(generator, subject, output)
        if any(path == ".git" or path.startswith(".git/") for path in checkout_paths):
            raise AssertionError("Root checkout .git control directory was baselined")
        if "nested/.git" not in checkout_paths or "embedded/.git/config" not in checkout_paths:
            raise AssertionError("Nested .git paths lost exact-project-path coverage")

    print("PASS VER-INT-0001 root .git file/directory excluded; nested .git paths baselined")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
