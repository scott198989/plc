# Directive and Phase 1 Foundation Log

## Purpose

This file records directive-adjacent governance and repository-foundation activity. An entry here does not amend the controlling DOCX, change a requirement, resolve a decision, or prove acceptance. A normative directive change requires the change-control process in `PES-GOV-0014`, `PES-GOV-0015`, and `PES-GOV-0016`.

## Implementation log

### P1-LOG-0001 - Project-local repository boundary initialized

- **Date:** 2026-08-27
- **Type:** Repository foundation
- **Action:** A project-local Git repository was initialized inside `Codex - GOV's PLC`, separating future project history from the unrelated ancestor Git repository rooted at the Desktop.
- **Directive text changed:** No.
- **Source documents changed or renamed:** No.
- **Completion effect:** None. Repository initialization is infrastructure and does not satisfy a product requirement or Phase 1 exit gate by itself.

### P1-LOG-0002 - Phase 1 foundation work opened

- **Date:** 2026-08-27
- **Type:** Governance foundation
- **Action:** Phase 1 repository work began with governance, ADR, documentation, test, and tooling boundaries reserved for substantive Phase 1 artifacts. Empty structure earns no completion credit under the anti-placeholder policy.
- **Artifacts recorded by this workstream:** `OPEN_DECISIONS.md`, `RISK_REGISTER.md`, `CHANGELOG_DIRECTIVE.md`, and `docs/governance/PHASE_1_SCOPE_AUDIT.md`.
- **Directive text changed:** No.
- **Source documents changed or renamed:** No.
- **Completion effect:** Phase 1 foundation is in progress. No requirement is marked `VERIFIED` by this entry.

### P1-LOG-0003 - Source and controlling-document discrepancy blocked

- **Date:** 2026-08-27
- **Type:** Decision and change control
- **Action:** `DEC-0001` was opened as a bundled `BLOCKED` decision covering the research filename mismatch, the master-directive filename mismatch, and the relationship between phase-numbered prompts and the directive's one-living-document rule.
- **Directive text changed:** No.
- **Source documents changed or renamed:** No.
- **Completion effect:** Only document-identity changes and dependent acceptance claims are blocked. Unaffected Phase 1 foundation work may continue.

### P1-LOG-0004 - Phase boundary and DOCX QA status recorded

- **Date:** 2026-08-27
- **Type:** Scope and verification evidence
- **Action:** The Phase 1 scope audit records that no Phase 2-4 product feature work was performed. This entry originally recorded structural content extraction; `P1-LOG-0008` and `P1-LOG-0009` record the subsequent current-hash visual observation and its evidence boundary.
- **Directive text changed:** No.
- **Source documents changed or renamed:** No.
- **Completion effect:** The Phase 1 exit gate is not claimed as passed.

### P1-LOG-0005 - Deterministic requirement baseline generated

- **Date:** 2026-08-27
- **Type:** Traceability foundation
- **Action:** All 247 unique directive requirement IDs were extracted into a source-heading- and hash-bound registry plus a matching implementation matrix. Curated current-scope acceptance criteria and exact check/component mappings are separated from unresolved later-phase baselines.
- **Truthfulness control:** The generator never self-promotes an entry to `VERIFIED`; implemented foundation controls remain `IMPLEMENTED_UNVERIFIED` pending external evidence and reviewer acceptance.
- **Directive text changed:** No.
- **Completion effect:** No product or Phase 1 completion claim.

### P1-LOG-0006 - Phase 1 governance verifier and CI guardrails added

- **Date:** 2026-08-27
- **Type:** Verification foundation
- **Action:** A standard-library verifier now checks source hashes, policy substance, exact registry/matrix mappings, unreviewed evidence truthfulness, forbidden-surface absence, toolchain pins, and phase-boundary language. A proposed workflow is configured to reject stale generated snapshots, syntax-check the verifier, and retain a hash-bound JSON report if remote CI is later approved.
- **Remote-execution status:** `BLOCKED` by `DEC-0002`; the workflow is manual-trigger-only and its job is literally disabled. No hosted-run or upload evidence is claimed.
- **Evidence boundary:** A passing report applies only to its recorded artifact hashes. The report is not a completed contributor attestation, independent approval, DOCX visual QA, release proof, or Phase 1 exit decision.
- **Directive text changed:** No.
- **Completion effect:** No requirement is automatically promoted to `VERIFIED` and the Phase 1 exit gate remains open.

### P1-LOG-0007 - Remote CI/service choice stopped for Scott

- **Date:** 2026-08-27
- **Type:** Mandatory decision stop
- **Action:** `DEC-0002` records that choosing GitHub-hosted CI and remote report storage would add cloud, credential, runtime-download, log, and retention behavior. The proposed workflow remains reviewable but cannot execute.
- **Directive text changed:** No.
- **Completion effect:** Local offline governance verification may continue; remote execution and artifact upload remain blocked.

### P1-LOG-0008 - Current-hash DOCX visual observation performed

- **Date:** 2026-08-27
- **Type:** Source-document verification evidence
- **Action:** The supplied directive was opened read-only in Word, exported to a 40-page ignored local PDF, rendered completely, inspected through four contact sheets and selected full-page views, and checked for page geometry, headers/footers, and corrupt glyphs.
- **Result:** Observation pass for the current source hash; no clipping, overlap, broken table, missing page content, out-of-bounds word, or replacement glyph was found in the stored renders.
- **Evidence boundary:** Exact method and hashes are in `docs/governance/DOCX_VISUAL_QA.md`. The source was not changed. The renderer and inspection stack remain unapproved, so this output is not admissible visual-QA gate evidence or independent reviewer acceptance.
- **Completion effect:** Records a clean observation only; the visual-QA gate and Phase 1 exit remain open, and `DEC-0001`/`DEC-0002` remain unresolved.

### P1-LOG-0009 - Full-page visual review and evidence classification corrected

- **Date:** 2026-08-27
- **Type:** Verification correction
- **Action:** Independent reviewers inspected every stored page at full-page resolution. Multi-image previews intermittently masked margin and list content; isolated review plus raw-pixel comparisons proved those apparent defects were preview artifacts. The local evidence hashes/count were added to the verifier contract.
- **Authorization finding:** Word, Poppler, and the PDF-QA Python stack fall outside the standard-library-only bootstrap exception and remain unapproved. Their output is retained only as an ignored observation and cannot satisfy the directive gate or authorize reuse.
- **Directive text changed:** No.
- **Source documents changed or renamed:** No.
- **Completion effect:** Corrects the record and strengthens reproducibility without passing the visual-QA gate, Phase 1 exit, or any requirement to `VERIFIED`.

## Directive change ledger

No controlling-directive text change, supersession, renumbering, or normative requirement-state change has been approved or performed in this repository as of 2026-08-27. Generated registry states are current repository observations, not directive amendments.
