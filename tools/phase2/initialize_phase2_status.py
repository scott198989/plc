#!/usr/bin/env python3
"""Create the conservative Phase 2 evidence/status ledger from source catalogs."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from verify_phase2 import initial_status_ledger, load_json


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
    registry = load_json(root / "requirements" / "phase2-requirements.json")
    catalog = load_json(root / "requirements" / "phase2-verification-catalog.json")
    payload = (
        json.dumps(initial_status_ledger(registry, catalog), indent=2, ensure_ascii=False) + "\n"
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
