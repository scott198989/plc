# Phase 2 requirement extraction

`extract_phase2_requirements.py` reads the canonical Phase 2 DOCX directly
from its WordprocessingML package using only the Python standard library. It
verifies the fixed source SHA-256 before parsing and writes:

- `requirements/phase2-requirements.json`
- `requirements/phase2-verification-catalog.json`
- `requirements/phase2-extraction-audit.json`

Generate the registries with the repository's pinned Python runtime:

```powershell
python -B tools/phase2/extract_phase2_requirements.py --root .
```

Check that committed output is byte-current without writing:

```powershell
python -B tools/phase2/extract_phase2_requirements.py --root . --check
```

Run the dependency-free unit suite:

```powershell
python -B -m unittest discover -s tools/phase2 -p "test_*.py" -v
```

The extractor preserves all unique bracketed `PES-*` definitions, every
non-heading normative/default paragraph in Sections 14 through Appendix L,
and every table in that scope. Source pointers bind records to the canonical
document hash and Word body coordinates.

Truth is intentionally conservative. Requirements default to `NOT_STARTED`.
Only `PES-GOV-0040` may become `IMPLEMENTED_UNVERIFIED`, and only when the
validated P2-00 evidence record is present. The extractor never emits
`VERIFIED`.

The audit reports duplicate definitions, unknown area tokens, unknown or
orphan requirement and verification references, Phase 1 ID reuse, retired ID
reuse, and unresolved placeholder tokens. Findings are recorded rather than
silently renumbered or rewritten. A current extraction baseline is not a
software-completion or acceptance result.

Exit codes:

- `0`: generated successfully, or all checked files are current;
- `1`: `--check` found missing or stale generated output;
- `2`: source integrity, package parsing, or required-input failure.
