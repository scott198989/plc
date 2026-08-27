# Requirement Governance

## Authority and purpose

The supplied Phase 1 directive is the controlling source for Phase 1 product
requirements. The frozen research report supplies evidence and context; it does
not independently change normative scope. Requirement IDs are permanent and are
never reused.

The machine-readable register is
`requirements/phase1-requirements.json`. `IMPLEMENTATION_MATRIX.json` is the
status and traceability view. Both are mechanically extracted from the directive
and checked by `tools/phase1/verify-phase1.mjs`.

## Identifier system

- Product requirements: `PES-AREA-NNNN`
- Evidence: `SRC-NNNN`
- Architecture decisions: `ADR-NNNN`
- Product decisions: `DEC-NNNN`
- Open questions: `OQ-NNNN`
- Risks: `RSK-NNNN`
- Change records: `CR-NNNN`
- Verification cases: `VER-AREA-NNNN`

Retired IDs remain tombstones with their disposition and supersession reason.
IDs do not encode phase, release, priority, status, or document section.

## Record contract

Every requirement record contains:

- stable ID and short title;
- normative keyword and controlling statement;
- rationale and component scope;
- source pointer and source classification;
- IP class and required disposition;
- blocking dependencies, non-blocking related requirements, dependency-review maturity, and target milestone;
- current truth state;
- positive and negative acceptance statements;
- verification, ADR, decision, and change links;
- owner and reviewer;
- an atomicity and acceptance-maturity assessment.

The extracted register preserves the directive wording. Where a directive ID
contains a list or compound obligation, the record is flagged for atomicity
review rather than silently inventing or renumbering requirements. Any split or
rewrite requires an approved change record under `PES-GOV-0014` through
`PES-GOV-0016`.

`dependencies` contains only prerequisites that block a truth-state promotion.
`relatedRequirements` records traceability relationships that do not themselves
block the current-scope control. An empty dependency list is authoritative only
when `dependencyMaturity` says that the relationship review is curated; later
requirements deliberately label the field `UNRESOLVED_BASELINE` rather than
asserting that they have no dependencies.

## Truth states

`NOT_STARTED`, `SCAFFOLDED`, `PARTIAL`, `IMPLEMENTED_UNVERIFIED`, `VERIFIED`,
`BLOCKED`, `DEFERRED`, and `EXCLUDED` are the only permitted states. Only
`VERIFIED` means complete.

The Phase 1 register intentionally leaves product behavior as `NOT_STARTED`,
`DEFERRED`, `EXCLUDED`, or `BLOCKED`. Governance artifacts may be verified by
Phase 1 checks, but that never implies that the simulator feature governed by a
policy exists.

The deterministic extractor never promotes a requirement to `VERIFIED` based on
its own generated content. Current foundation artifacts remain
`IMPLEMENTED_UNVERIFIED`; an external verifier report records the outcome for a
hash-bound snapshot, and reviewer acceptance is a separate governance action.

## Verification contract

Each implementation must map to at least one requirement. Each `MUST` and
`MUST NOT` must eventually map to positive and negative verification. Every
verification case must map back to one or more requirements. Orphan tests and
unverified requirements remain visible and block any applicable completion
claim.

Phase 1 verification IDs are documented in
`docs/governance/PHASE_1_VERIFICATION_PLAN.md`. Release-level isolation,
packaged-artifact, WASM-import, offline-course, and zero-egress verification is
defined now but remains unclaimable until later phases create the corresponding
product artifact.

Committed JSON snapshots must exactly match a fresh extractor run. CI executes
the extractor in `--check` mode before the verifier and retains the verifier's
artifact-hash-bound JSON report. A passing report is evidence for that run; it
does not rewrite the registry, impersonate an independent reviewer, or pass the
Phase 1 exit gate.
