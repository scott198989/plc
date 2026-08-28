#!/usr/bin/env python3
"""Create the conservative Phase 2 evidence/status ledger from source catalogs."""

from __future__ import annotations

import argparse
import json
from pathlib import Path, PurePosixPath

import reviewed_requirement_mapping
from verify_phase2 import initial_status_ledger, load_json, sha256_file


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument(
        "--output", type=Path, default=Path("evidence/phase2/PHASE2_IMPLEMENTATION_STATUS.json")
    )
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = args.root.resolve(strict=True)
    output = args.output if args.output.is_absolute() else root / args.output
    registry_path = root / "requirements" / "phase2-requirements.json"
    catalog_path = root / "requirements" / "phase2-verification-catalog.json"
    reviewed_mapping_path = root / reviewed_requirement_mapping.REVIEWED_MAPPING_PATH
    registry = load_json(registry_path)
    catalog = load_json(catalog_path)
    reviewed_mapping = load_json(reviewed_mapping_path)
    directive = registry.get("directive", {}).get("path")
    if not isinstance(directive, str):
        raise ValueError("Phase 2 requirement registry has no directive path")
    directive_path = root / PurePosixPath(directive.replace("\\", "/"))
    payload = (
        json.dumps(
            initial_status_ledger(
                registry,
                catalog,
                reviewed_mapping,
                requirement_registry_sha256=sha256_file(registry_path),
                verification_catalog_sha256=sha256_file(catalog_path),
                directive_sha256=sha256_file(directive_path),
            ),
            indent=2,
            ensure_ascii=False,
        )
        + "\n"
    ).encode("utf-8")
    if args.check:
        if not output.is_file() or output.read_bytes() != payload:
            print(f"STALE {output}")
            return 1
        print(f"CURRENT {output}")
        return 0
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(payload)
    print(f"WROTE {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
