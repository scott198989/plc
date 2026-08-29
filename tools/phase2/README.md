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

## Phase 2 implementation/evidence gate

`verify_phase2.py` is the fail-closed implementation exit gate. It does not
infer product truth from code volume, test names, fixtures, screenshots, or a
successful build. The tracked
`evidence/phase2/PHASE2_IMPLEMENTATION_STATUS.json` deliberately enumerates all
937 extracted requirements, all 44 Appendix H proof obligations, Journeys A-H,
and G2-01 through G2-15 as `NOT_STARTED`.

Run the gate with the pinned runtime:

```powershell
pnpm gate:phase2
```

The gate requires the candidate ref to be the current exact HEAD, a clean
tracked worktree, and no untracked files except byte-current paths already
accounted by P2-00. It binds evidence to the candidate commit, tree,
production-source manifest, test-source manifest, requirement-source manifest,
requirement registry, Appendix H catalog, and directive SHA-256 values. Use the
read-only binding view when producing an external evidence package:

```powershell
node tools/phase2/run_pinned_python.mjs -B tools/phase2/verify_phase2.py `
  --root . --candidate-ref HEAD --binding-only
```

Evidence can be supplied in an external status ledger using `--status`. This
avoids a self-referential evidence commit: immutable evidence may bind to an
already-created candidate commit without changing that commit. Every credited
record must have exact binding fields, an exit-zero command transcript, valid
timestamps, explicit non-skipped/non-flaky/non-crashed/non-canned flags, a real
production-path assertion, and at least one non-empty byte/hash-verified log.
Mutation evidence also requires the expected named detector to equal the
actual detector and a nonzero detector rejection without crash credit.
Isolation evidence requires complete instrumentation, named platform/config
coverage, and an explicit zero-external-attempt result.

The candidate verdict is reachable only when all requirements and Appendix H
rows are `VERIFIED`, all journeys and G2 gates are `PASS`, no Critical/High
defect remains open, mappings and evidence are complete, extraction is current,
and the complete-workspace source policy passes. Otherwise the verdict is
`BLOCKED` and the process exits nonzero.

`source_policy.py` discovers every `crates/<name>` member from the workspace
Cargo.toml instead of using a fixed list. It scans every member's manifest,
build script, and production Rust source, plus production TypeScript and
dependency manifests. Consequently a newly added Phase 2 crate cannot evade
the PhysicalUniverse capability scan.

Run the dependency-free regression suite:

```powershell
pnpm test:phase2:gate
```

### Exact-candidate execution collection

`collect_phase2_evidence.py` runs only against a clean, already-committed exact
candidate. Its `core` stage executes the complete local Phase 2 regression,
native-host, browser, closure, and source-policy suite, then runs all eight
named Journey H mutations in disposable Git archives. Records and transcripts
are written only below the ignored `.phase2-verification/` directory and are
bound to the candidate commit, tree, source manifests, registries, and pinned
directive hash.

```powershell
node tools/phase2/run_pinned_python.mjs -B `
  tools/phase2/collect_phase2_evidence.py `
  --root . --candidate-ref HEAD --mode core
```

The isolation harness is deliberately separate because it requires real
Windows platform/configuration runs. Once a strict exact-candidate isolation
directory exists, assemble the reusable core records and strict isolation
record into the finalizer input:

```powershell
node tools/phase2/run_pinned_python.mjs -B `
  tools/phase2/collect_phase2_evidence.py `
  --root . --candidate-ref HEAD --mode assemble --reuse `
  --isolation-dir .phase2-verification/<strict-isolation-run>
```

Collection never declares requirement truth, Scott acceptance, or a Phase 2
verdict. `finalize_phase2_status.py` consumes the execution index, and
`verify_phase2.py` remains the terminal fail-closed gate. A missing native
host run, adapters-off run, controlled live-LAN pair, artifact hash, or exact
candidate binding is a blocker rather than partial credit.

`governance_audit.py` independently checks the canonical directive hash,
authority hierarchy, all 937 normative requirement rows, Appendix H inventory,
clarification ledger, open decisions, exclusions, phase reservations, and the
mandatory verdict/stop vocabulary. It is read-only and grants no implementation
or verification credit. Run it directly with:

```powershell
pnpm audit:phase2:governance
```

## Static coverage audit

`generate_phase2_coverage_audit.py` deterministically inventories all 937
requirements and all 44 Appendix H minimum proofs against the current
production/test surface. Its three classifications are deliberately below a
verification verdict:

- `IMPLEMENTED_EVIDENCE_READY`: static production and directly applicable test
  paths cover the full minimum-proof clause, but current-candidate execution
  evidence is still required;
- `PARTIAL`: some support exists and every uncovered clause is named;
- `MISSING`: no directly applicable implementation/evidence harness exists.

The generated JSON preserves unreviewed requirement mappings, contains empty
execution-evidence lists, and grants zero verification credit. Regenerate or
check it with:

```powershell
pnpm coverage:phase2:generate
pnpm coverage:phase2:check
```
