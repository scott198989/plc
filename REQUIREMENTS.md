# Requirement Governance

## Authority and purpose

The supplied Phase 1 directive is the controlling source for Phase 1 product
requirements, as corrected by the bounded Phase 1 Corrective Addendum recorded
as `CR-0001`. The frozen research report supplies evidence and context; it does
not independently change normative scope. Requirement IDs are permanent and are
never reused.

The canonical supplied references retain their approved filenames and hashes at
these repository-relative paths:

- `References for Codex from Scott/Govs PLC project Research Report.md`
- `References for Codex from Scott/PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx`
- `References for Codex from Scott/PLC Engineering Simulator - Phase 1 Corrective Addendum - Closure and Trusted Baseline.docx`

`CR-0001` records Scott's path relocation. It does not create a duplicate source,
rename any canonical filename, change a source hash, or alter the authority
hierarchy.

The machine-readable register is
`requirements/phase1-requirements.json`. `IMPLEMENTATION_MATRIX.json` is the
status and traceability view. `requirements/phase1-reconciliation.json` is the
independently reproduced 546-source-unit recall ledger, gap-disposition ledger,
and compound-split ledger. All three snapshots are deterministically generated
from the hash-bound directive and the approved `CR-0001` reconciliation policy.

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

`CR-0001` allocates atomic child IDs deterministically in directive body order,
then clause order, using the next previously unused number in the applicable
area. Gap-closing IDs use the same permanent next-unused-number policy. An issued
parent or child is never deleted, renumbered, or reused after allocation.

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
- an atomicity and acceptance-maturity assessment;
- completion eligibility and lifecycle lineage; and
- exact source-unit IDs, source wording, and any inherited modal lead-in.

The extracted register preserves the directive wording. `CR-0001` confirms 20
compound source records. Each original issued ID remains a
`SUPERSEDED_COMPOUND_PARENT` lineage record with zero completion eligibility,
and its 190 separately testable clauses are issued as atomic children. The
source wording is preserved verbatim; a child combines the inherited modal
lead-in with its exact governed clause. `PES-REQ-0003` preserves the real table
header and rows and no longer injects the nonexistent `Table row:` prefix.

### WP-0C reconciled counts

- Source parent IDs extracted from the directive: **247**
- Atomic child IDs issued for 20 compound parents: **190**
- Atomic IDs issued to close the 48 recall gaps: **47**
- Total issued IDs, including historical compound parents: **484**
- Superseded compound parent records: **20**
- Atomic records: **464**
- Completion-eligible atomic records after express document-sequencing supersession: **463**
- Independently reproduced source statement units: **546**
- Mapped source statement units: **546**
- Unmapped source statement units: **0**
- Source-unit-to-issued-ID relationships: **789**

The relationship count is not a requirement count. A modal lead-in maps to each
atomic clause it governs, and the two top-level register-file aliases map to all
atomic fields in their named contracts. Historical parent lineage is stored
separately and is not added to the 789 relationship total.

The 546-unit adversarial recall population uses the recorded trigger scope:
`shall`, `must`, `never`, explicit required/prohibited wording, limited
permission (`may ... only`), and inherited modal children. Ten already-issued
plain advisory records (`SHOULD` or unrestricted `MAY`) are intentionally not
double-counted as units in that compulsory/prohibitive recall population. They
remain hash-bound registry records with exact heading, body-block, modal, and
verbatim source pointers; their empty `sourceUnitIds` arrays are therefore an
explicit method boundary, not an unexplained mapping gap.

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

The Phase 1 register intentionally leaves PLC-domain and later-phase product
behavior as `NOT_STARTED`, `DEFERRED`, `EXCLUDED`, or `BLOCKED`. The corrective
addendum separately authorizes one bounded non-PLC technical foundation: a local
health UI, typed command/result contract, isolated worker, and deterministic
zero-import Rust/WASM health path. Those artifacts can be
`IMPLEMENTED_UNVERIFIED` without implying that a compiler, controller runtime,
scan cycle, editor, HMI, process model, lesson, scenario, assessment, packaging,
or physical communication feature exists.

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
`docs/governance/PHASE_1_VERIFICATION_PLAN.md`. The current foundation gate
builds and semantically inspects its embedded WASM, validates the typed
`DomainResult`, scans the single-file bundle for prohibited capabilities, and
runs the artifact offline with zero remote requests. Release-level PLC-product
isolation, offline-course, migration, export, Virtual Download, InternalTagBus,
and packaged-classroom proofs remain unclaimable until later phases create the
corresponding product surfaces.

Committed JSON snapshots must exactly match a fresh extractor run. The active
local closure gate and checked-in CI configuration execute the extractor in
`--check` mode before foundation, verifier, and mutation checks and produce
hash-bound local evidence. The configuration does not authorize creating a
remote, pushing the repository, hosted execution, credentials, service terms,
or report upload. A passing report is evidence for that run; it does not rewrite
the registry, impersonate an independent reviewer, or pass the Phase 1 exit
gate.
