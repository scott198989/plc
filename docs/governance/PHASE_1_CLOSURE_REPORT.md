# Phase 1 Closure Report

**Report date:** 2026-08-27

**Verdict:** PHASE 1 CLOSURE CANDIDATE — AWAITING SCOTT ACCEPTANCE

**Acceptance authority:** Scott

**Phase 2 authorization:** not granted

## Controlling outcome

The corrective Phase 1 work satisfies the fourteen technical closure gates
below. This is a candidate for Scott's review, not Scott's acceptance, not a
legal or release approval, and not authority to begin Phase 2.

The final commit cannot embed its own identity in tracked content. Resolve the
immutable candidate and its post-freeze evidence with:

```powershell
git rev-parse 'phase1-closure-candidate-v1^{}'
git notes --ref=phase1-closure-evidence show phase1-closure-candidate-v1
```

The Git note records the exact candidate commit, annotated-tag object, trusted
manifest, final audit DOCX, final verifier report, mutation transcript, and
clean-checkout gate results without creating a self-reference.

## G0 closure gates

| Gate | Result | Direct evidence |
|---|---|---|
| `G0-01` Canonical sources match approved hashes | PASS | `evidence/phase1-closure/canonical-source-hashes.txt`; `tests/phase1/policy-contract.json`; `VER-GOV-0001` |
| `G0-02` Tasks 1–8, limitations, defects, verdict, and checked DOCX are complete | PASS | `docs/governance/PHASE_1_ADVERSARIAL_AUDIT.md`; `docs/governance/PHASE_1_ADVERSARIAL_AUDIT.docx`; `docs/governance/DOCX_VISUAL_QA.md`; reproducible local render support under `.phase1-verification/audit-docx/` |
| `G0-03` Every in-scope source unit is mapped or validly bounded | PASS | `requirements/phase1-reconciliation.json`: 546/546 mapped, zero unexplained gaps; `REQUIREMENTS.md` records the advisory-unit method boundary |
| `G0-04` Confirmed compounds are atomically split with lineage | PASS | `requirements/phase1-requirements.json`; `requirements/phase1-reconciliation.json`: 20 parents, 190 stable children |
| `G0-05` `PES-REQ-0003` and source-fidelity drift are corrected | PASS | `tools/phase1/extract_directive_requirements.py`; regenerated registry/matrix/reconciliation; `VER-REQ-0001` |
| `G0-06` Clean baseline and all twelve intended mutations pass | PASS | `.phase1-verification/mutations/mutation-results.json`: 12/12; `tools/phase1/run_phase1_mutations.mjs` |
| `G0-07` No crash, unrelated failure, or environment error earns credit | PASS | Mutation transcript: every M01–M12 exit `1`, intended detector true, `crashed=false`, no `ERROR` |
| `G0-08` Integrity uses a trusted immutable expected baseline | PASS | `tools/phase1/run_phase1_verification.py`; Git-object manifest; sealed-manifest tamper test passes |
| `G0-09` Digest and filename conflicts have reproducible resolution | PASS | `evidence/phase1-closure/hash-recomputation/`; `OPEN_DECISIONS.md`; `CHANGELOG_DIRECTIVE.md` (`CR-0001`) |
| `G0-10` History, clean candidate, and active shared gate exist | PASS | pre-remediation commit/tag; candidate tag; `.github/workflows/phase1-governance.yml`; root `gate:closure`; Git evidence note |
| `G0-11` Minimal foundation installs, builds, tests, launches, and round-trips | PASS | `pnpm gate:foundation`; `dist/index.html`; `design-qa.md`; 20 unit tests; desktop/mobile E2E |
| `G0-12` Separate read-only final review confirms closure evidence | PASS | `docs/governance/PHASE_1_FINAL_READ_ONLY_REVIEW.md`; clean-checkout rerun in Git evidence note |
| `G0-13` Physical industrial communication remains prohibited and tested | PASS | ADR-0001; `SECURITY_INVARIANTS.md`; `THREAT_MODEL.md`; `VER-ISO-0001`; `VER-OFF-0001/0002`; zero WASM imports and browser requests |
| `G0-14` No Phase 2 feature and no unresolved Critical/High defect remain | PASS | `docs/governance/PHASE_1_SCOPE_AUDIT.md`; `evidence/phase1-closure/defect-register.json`; `VER-SCP-0001` |

## Requirement reconciliation

- 247 original source-parent IDs.
- 484 issued IDs after preserving all parents and issuing children.
- 20 superseded compound parents and 190 stable atomic child IDs.
- 464 atomic records; 463 are completion eligible.
- 546 independently reproduced source statement units; 546 mapped; zero
  unmapped; 789 explicit relationships.
- All 48 pre-remediation recall gaps have explicit mapped dispositions.
- `PES-REQ-0003` now preserves the real table header/rows without the invented
  `Table row:` prefix.
- Ten already-issued plain `SHOULD` or unrestricted `MAY` records remain exact
  advisory source pointers and are not double-counted in the recorded
  compulsory/prohibitive 546-unit recall population.
- Truth states across all 484 records: 2 `BLOCKED`, 3 `DEFERRED`, 100
  `IMPLEMENTED_UNVERIFIED`, 363 `NOT_STARTED`, 16 `PARTIAL`, and zero
  `VERIFIED`.
- Only `VERIFIED` can count as complete; completion remains zero.

## Immutable verifier and mutation evidence

The repaired verifier reads `tests/phase1/trusted-baseline.json` from an
immutable Git object, copies it outside the subject repository, seals the copy
with SHA-256, verifies the full commit identity, and compares the subject's
exact controlled path set, lengths, and hashes. It never falls back to values
recomputed from the subject.

Immutable validation commit
`c859c7a7126a2f7c36409e7e884afb61ca40bbaa` passed its clean baseline with
manifest SHA-256
`BAEE99688C8DCB75B961D793ADF79C25F1C0AAA1116873428BCD3925BB9A71D9`.
The candidate's own exact results are attached in the post-freeze Git note.

| Case | Intended detector | Result |
|---|---|---|
| M01 controlled-file byte | `VER-INT-0002` | PASS |
| M02 frozen-research byte | `VER-GOV-0001` | PASS |
| M03 matrix deletion | `VER-REQ-0002` | PASS |
| M04 false `VERIFIED` promotion | `VER-REQ-0002` | PASS |
| M05 source contradiction | `VER-REQ-0001` | PASS |
| M06 external URL | `VER-OFF-0001` | PASS |
| M07 loopback endpoint | `VER-OFF-0002` | PASS |
| M08 vendor-facing text | `VER-BRN-0001` | PASS |
| M09 network-capable dependencies | `VER-DEP-0001` | PASS |
| M10 unauthorized product root | `VER-SCP-0001` | PASS |
| M11 unsupported risk closure | `VER-RSK-0001` | PASS |
| M12 missing ADR | `VER-ADR-0001` | PASS |

The clean baseline exits `0`; each named policy mutation exits `1`; trust/tool
errors exit `2`. The manifest-tamper test passes, no crash or `ERROR` receives
credit, no case directory remains, and wrapper scratch is removed after the
durable ignored transcript is copied.

## Canonical names, hashes, and digest correction

| Item | Canonical path or value | SHA-256 / disposition |
|---|---|---|
| Research | `References for Codex from Scott/Govs PLC project Research Report.md` | `F05C08323B5CC9483BEB1FEB3C7312CCB9A45EBE3B527E6DAE069C181D3FBF55` |
| Phase 1 directive | `References for Codex from Scott/PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx` | `EBF074E2CEAB752F09E6DB63D88E100991729DA13C1EB874290A6B337DA72612` |
| Corrective addendum | `References for Codex from Scott/PLC Engineering Simulator - Phase 1 Corrective Addendum - Closure and Trusted Baseline.docx` | `950C5112C34D0218FD1E59CF6C051ACCD01AB92674CD70C96C08A5F1DA2E5A1C` |
| Red-team audit | `References for Codex from Scott/CODEX RED TEAM AUDIT PHASE 1.docx` | `515BD6A844AF1A2E71C829BA567214F73CA4E455278157960A3A11F25B8DF5B3` |
| Post-red-team audit | `References for Codex from Scott/CODEX AUDIT AFTER RED TEAM.docx` | `4D85D71CEB36AC0E4892CBF91FCDB4B6CE8FFC27E80DA20D754EE8008ABEB87F` |
| Historical malformed render digest | `DD5FAA0B213307CC6D4FBB8D5087FBC59751B777009CD2997BA9EF453E02FF` | Preserved as the 62-character defect |
| Correct normalized render digest | `44B089F87E65B2FC6A2D40DEA9D19B326A252E28CEA846E119A9381A5A0B1728` | Recomputed from preserved raw page text |

Scott's relocation of the five canonical references is recorded as 100% Git
renames. No root duplicate or silent source rewrite was created.

## Repository and CI state

- Pre-remediation commit:
  `90705431d13eddbdef519f52356caef5f5f07c96`.
- Pre-remediation annotated tag: `phase1-pre-remediation-v1`; tag object
  `bec2d44991b53b73b50b1536dea8708506376094`.
- Final candidate: annotated tag `phase1-closure-candidate-v1`, resolved through
  the command at the start of this report.
- Evidence note: `refs/notes/phase1-closure-evidence`.
- Active CI triggers: push, pull request, and manual dispatch.
- Active CI/local command: exactly one `pnpm gate:closure`.
- No upload step, repository remote, hosted execution, credentials, or hosted
  retention is claimed.
- The local exact gate uses Node `24.19.0`, pnpm `11.19.0`, Python `3.13.12`,
  Rust/Cargo `1.94.0`, and `wasm32-unknown-unknown`.

## Minimal foundation

Install and build:

```powershell
pnpm install --frozen-lockfile --ignore-scripts
pnpm build:foundation
```

Verify and launch:

```powershell
pnpm gate:foundation
pnpm launch:foundation
```

The foundation passes exact-toolchain admission, source policy, `rustfmt`,
`clippy`, strict TypeScript, 2 Rust tests, 16 contract tests, 2 UI tests,
production build, isolation, and desktop/mobile real-browser tests. The single
209,552-byte `dist/index.html` has SHA-256
`B3CEB08111F27004D19022F8B9C7AFD3437EC9F13FFE635B18340CEBC89E0039`.
It performs a real UI → typed command → Web Worker → 247-byte zero-import
Rust/WASM → validated `DomainResult` → rendered `HEALTHY` round trip. The WASM
SHA-256 is
`D77FBFB13096417B94749387D0AF74230A2E04E148001E62475E116CE7DF72A3`.
Repeated desktop/mobile runs are deterministic and record zero page network
requests.

## Files added, changed, superseded, and untouched

Added:

- The minimal `apps/foundation-shell/`, `packages/foundation-contract/`, and
  `crates/foundation-wasm/` foundation.
- Foundation and corrective-verifier tools/tests.
- Schema-v3 reconciliation and closure evidence under
  `evidence/phase1-closure/`.
- Closure, verification, visual-QA, and final-review reports under
  `docs/governance/`.

Changed:

- Requirements, matrix, evidence/asset/dependency registers, decisions,
  changelog, safety/threat controls, toolchain admission, workspace/toolchain
  pins, workflow, README, scope audit, and repository attributes/ignores.

Superseded without deletion:

- Twenty compound requirement parents remain non-completion-bearing lineage
  records with stable child links.
- Pre-remediation verifier/audit conclusions remain preserved and explicitly
  labeled historical.
- `CR-0001` supersedes only the one-living-document/separate-phase-directive
  conflict recorded in `OPEN_DECISIONS.md`.

Untouched in bytes:

- All five canonical source/audit-reference documents; only their
  repository-relative folder location changed at Scott's instruction.
- The physical-isolation invariant and all later-phase safety, clean-room,
  trademark, legal-review, evidence, and mandatory-stop boundaries.

## Open external defects and decisions

No Critical or High defect remains open. Three lower-severity external
boundaries remain visible and do not invalidate the Phase 1 safety/source/
verification closure candidate:

- `DEF-019` Medium: toolchain/dependency legal, provenance, and security
  admission. All candidates remain `CANDIDATE_UNREVIEWED`, local, bounded, and
  non-release.
- `DEF-020` Medium: historical `PES-ACC-0005` exact wording. Scott must decide;
  the source DOCX was not silently changed.
- `DEF-021` Low: no remote or hosted CI run. Remote creation, push, execution,
  credentials, logs, upload, and retention remain blocked by `DEC-0002`.

Scott's acceptance remains open by design.

Phase 2 product implementation was not begun under this order.
