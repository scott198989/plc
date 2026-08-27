#!/usr/bin/env python3
"""Recompute the normalized all-page text digest reported by the adversarial audit."""

from __future__ import annotations

import argparse
import hashlib
import re
import sys
from pathlib import Path

import pdfplumber


def sha256_bytes(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest().upper()


def normalize(text: str) -> str:
    text = text.replace("\u00ad", "").replace("\n", " ")
    text = re.sub(r"([A-Za-z])-\s+([A-Za-z])", r"\1\2", text)
    return re.sub(r"[^a-z0-9]+", " ", text.lower()).strip()


def normalized_pages(path: Path) -> list[str]:
    with pdfplumber.open(path) as pdf:
        return [
            normalize(page.extract_text(x_tolerance=2, y_tolerance=2) or "")
            for page in pdf.pages
        ]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifact", type=Path, required=True)
    parser.add_argument("--comparison-artifact", type=Path)
    parser.add_argument("--expected-pages", type=int, default=40)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    artifact = args.artifact.resolve(strict=True)
    pages = normalized_pages(artifact)
    combined = "\n".join(pages).encode("utf-8")
    digest = hashlib.sha256(combined).hexdigest().upper()

    print(f"PYTHON_VERSION={sys.version.split()[0]}")
    print(f"PDFPLUMBER_VERSION={pdfplumber.__version__}")
    print(f"ARTIFACT_PATH={artifact}")
    print(f"ARTIFACT_BYTES={artifact.stat().st_size}")
    print(f"ARTIFACT_SHA256={sha256_bytes(artifact)}")
    print(f"PAGE_COUNT={len(pages)}")
    print("NORMALIZER=soft-hyphen-remove; newline-to-space; ASCII-alpha-dehyphenate; lowercase; non-[a-z0-9]-run-to-space; strip-page")
    print("COMBINATION=normalized pages joined by one LF; no terminal LF; UTF-8")
    print(f"NORMALIZED_TEXT_SHA256={digest}")

    valid = len(pages) == args.expected_pages and len(digest) == 64
    if args.comparison_artifact:
        comparison = args.comparison_artifact.resolve(strict=True)
        comparison_pages = normalized_pages(comparison)
        matches = sum(left == right for left, right in zip(pages, comparison_pages))
        print(f"COMPARISON_ARTIFACT_PATH={comparison}")
        print(f"COMPARISON_ARTIFACT_BYTES={comparison.stat().st_size}")
        print(f"COMPARISON_ARTIFACT_SHA256={sha256_bytes(comparison)}")
        print(f"COMPARISON_PAGE_COUNT={len(comparison_pages)}")
        print(f"PAGE_MATCHES={matches}/{max(len(pages), len(comparison_pages))}")
        valid = valid and pages == comparison_pages
    return 0 if valid else 1


if __name__ == "__main__":
    raise SystemExit(main())
