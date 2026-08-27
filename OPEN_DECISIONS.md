# Open Decisions

## Purpose and handling rules

This register records unresolved product decisions carried by the Phase 1
directive and preserves resolved decision history. It does not amend the
directive and does not treat an unanswered question as approval.

- `BLOCKED` means only the affected work must stop.
- `DEFERRED` means the decision belongs to the stated later authoring phase and is not implemented.
- Unaffected Phase 1 governance, safety, clean-room, traceability, and repository-foundation work may continue.
- A resolution is valid only when Scott supplies the required approval or evidence and the resulting directive change, if any, is recorded under `PES-GOV-0014`, `PES-GOV-0015`, and `PES-GOV-0016`.
- Silence, elapsed time, a placeholder, or an implementation guess is not a resolution.

## Product decisions

### DEC-0001 - Controlling document and source-file identity

**Status:** RESOLVED BY `CR-0001`

**Date opened:** 2026-08-27

**Date resolved:** 2026-08-27

**Owner and decision authority:** Scott

**Affected requirement IDs:** `PES-GOV-0001`, `PES-GOV-0002`, `PES-GOV-0006`, `PES-GOV-0010`, `PES-GOV-0014`, `PES-GOV-0015`, `PES-GOV-0016`, `PES-GOV-0017`, `PES-GOV-0018`, `PES-GOV-0019`, `PES-GOV-0020`, `PES-DEC-0002`, `PES-DEC-0004`, `PES-DEC-0005`, `PES-DEC-0006`, `PES-ACC-0005`, `PES-ACC-0006`, `PES-ACC-0007`

#### Resolution

Scott's Phase 1 Corrective Addendum selected four separate implementation
directives operating on one continuing repository. It establishes
`References for Codex from Scott/Govs PLC project Research Report.md` at SHA-256
`F05C08323B5CC9483BEB1FEB3C7312CCB9A45EBE3B527E6DAE069C181D3FBF55` and
`References for Codex from Scott/PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx`
at SHA-256
`EBF074E2CEAB752F09E6DB63D88E100991729DA13C1EB874290A6B337DA72612` as the
canonical supplied Phase 1 sources. The original Phase 1 DOCX remains preserved
as historical evidence and must not be renamed or overwritten for Phase 2.
Scott subsequently relocated the canonical references into
`References for Codex from Scott/` without changing their filenames, hashes, or
authority. `CR-0001` records the repository-relative path change without creating
duplicates.

`PES-GOV-0017` and `PES-GOV-0018` are superseded only to the extent that they
require one cumulative master document or prohibit separate phase directives.
The decision does not weaken any mission, clean-room, safety, offline,
brand-neutrality, or physical-isolation rule. It does not authorize Phase 2.
`PES-ACC-0005` remains independently unresolved because the preserved historical
DOCX's status text differs from its mandated exact wording; `CR-0001` does not
silently edit that source.

#### Known facts

1. The supplied research file is named `Govs PLC project Research Report.md`.
2. Its SHA-256 is `F05C08323B5CC9483BEB1FEB3C7312CCB9A45EBE3B527E6DAE069C181D3FBF55`.
3. The directive records the same research hash but identifies the baseline filename as `Govs PLC project.md`.
4. The supplied directive is named `PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx`.
5. The directive's Document Control table declares the final filename to be `PLC Engineering Simulator - Codex Master Implementation Directive.docx`.
6. The directive requires Phases 2-4 to append to and revise one living directive and prohibits competing master directives.
7. The user described prompts according to phase number that will live in the project folder and stated that each phase prompt should build on the last.
8. The user authorized beginning Phase 1, but did not authorize renaming, copying, replacing, or editing either source document.
9. Neither supplied source document has been renamed or modified by the Phase 1 repository-foundation work.
10. The supplied directive's status text says `Phase 1 authored; Phases 2-4 reserved`, while `PES-ACC-0005` mandates the exact status `Phase 1 authored; Phases 2-4 not yet authored.` No source edit has been authorized, so `PES-ACC-0005` remains `BLOCKED` rather than silently normalized.

#### Historical unknown or conflicting point

At opening, it was not clear whether the phase-numbered DOCX files were intended
to be non-controlling implementation prompts, successive revisions, or four
independently controlling directives. `CR-0001` resolves the document identity
and sequencing questions. The exact historical status-text mismatch remains a
separate unresolved acceptance issue under `PES-ACC-0005`.

#### Why this cannot be decided autonomously

The choice changes authoritative document identity, source provenance, cross-phase change control, filename stability, and the risk of competing directives. Silently renaming, copying, or treating separate phase prompts as separate master authorities could violate the directive's authority hierarchy and one-document rule.

#### Historical options

**Option A - One canonical living directive plus non-controlling phase prompts (recommended).**  
Treat phase-numbered files as source prompts or revision inputs. After explicit approval, establish `PLC Engineering Simulator - Codex Master Implementation Directive.docx` as the sole controlling living directive, preserve phase prompts as clearly labeled non-controlling source records, and correct the research filename through an approved change record. This best preserves the one-document rule while accommodating the user's phase-prompt workflow.

**Option B - The Phase 1-suffixed file is the living directive.**  
Keep `PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx` as the one controlling file and approve a change to the Document Control filename rule. Later phases would revise that same file rather than create independently controlling phase files.

**Option C - Separate controlling directive for each phase.**  
Approve a substantive change that supersedes the one-living-document rule and defines precedence, supersession, cross-reference, and contradiction handling for four controlling documents. This creates the greatest governance and divergence risk.

#### Historical recommendation

Approve Option A. It preserves one controlling product truth, permits separate phase prompts as inputs, and avoids silently treating filenames as authority.

#### Approval supplied and remaining boundary

Scott supplied the document identity, separate-directive sequencing, canonical
filenames, canonical hashes, and supersession authority through `CR-0001`. No
rename, overwrite, Phase 2 authorization, or exact status-text correction was
supplied.

#### Work that can continue

Phase 1 governance files, clean-room controls, safety invariants, threat modeling, requirements/evidence registers, ADRs, repository boundaries, the expressly authorized minimal non-PLC technical foundation, and executable local/CI closure guardrails may continue. No source DOCX rename or edit, public branding, packaging decision, remote operation, or Phase 2-4 feature implementation is authorized by this record.

### DEC-0002 - Remote CI provider and verification-report upload

**Status:** PARTIALLY RESOLVED BY `CR-0001` - ACTIVE CONFIGURATION AND LOCAL GATE AUTHORIZED; EXTERNAL OPERATIONS AND UPLOAD REMAIN BLOCKED
**Date opened:** 2026-08-27  
**Date partially resolved:** 2026-08-27
**Owner and decision authority:** Scott  
**Affected requirement IDs:** `PES-DEC-0001`, `PES-DEC-0002`, `PES-DEC-0004`, `PES-DEC-0005`, `PES-DEC-0006`, `PES-SEC-0004`, `PES-CI-0001`, `PES-CI-0003`

#### Partial resolution

The Phase 1 Corrective Addendum explicitly requires an active checked-in CI
configuration and a complete executable local closure gate. `CR-0001` therefore
authorizes configuration of that gate and local execution needed to produce
closure evidence. This narrow authorization supersedes the original proposal
to leave the workflow disabled.

It does not authorize creating or configuring a remote repository, publishing
or pushing project data, supplying credentials, accepting provider service
terms, starting a hosted run, retaining hosted logs, downloading tools during a
hosted run, uploading verification reports, or selecting remote artifact
retention. Those external actions remain blocked until Scott separately
authorizes their exact provider, data, credential, and retention boundaries.

#### Known facts

1. The project-local Git repository has no configured remote.
2. The root `gate:closure` command executes deterministic requirement checking, the complete minimal-foundation gate, the independent Phase 1 verifier, and controlled mutations.
3. The checked-in GitHub Actions file is active executable configuration and invokes the same closure gate; it is not described as disabled.
4. The configuration itself is repository content. Its presence does not prove or authorize remote publication, hosted execution, provider acceptance, or data transfer.
5. Report upload and remote artifact-retention steps are outside the partial resolution and must remain absent while this decision boundary is blocked.
6. No remote creation, push, hosted run, credential use, hosted log, report upload, or remote-retention evidence is claimed.

#### Remaining blocked boundary

Hosted CI introduces remote-service, credential, runtime-download, logging,
provider-policy, and potential data-retention behavior. Report upload adds a
separate disclosure, readership, retention, and deletion decision. The
corrective addendum did not supply those external approvals.

#### Exact additional approval needed

Before any hosted execution, Scott must approve the provider, repository
visibility, credential model, triggers, admitted actions/tool downloads,
service terms, logs, and exact project data that may leave the host. Before any
report upload, Scott must additionally approve report contents, readers,
retention, deletion, and incident handling. The resolution must be recorded by
change control; configuration presence or silence is not approval.

#### Work that can continue

The active checked-in configuration and complete local Phase 1 closure gate may
be maintained and executed. Remote creation, publication, push, hosted
execution, credentials, service acceptance, hosted logs, report upload, and
data retention remain blocked.

## Open questions carried forward from the directive

| ID | Decision | Why open | Required no later than | Current affected-work state |
|---|---|---|---|---|
| `OQ-0001` | Initial supported operating systems and packaged classroom shell | Platform-neutral core does not choose a deployable artifact | Before product packaging work | `BLOCKED` for packaging and public artifact work; Phase 1 governance may continue |
| `OQ-0002` | Final public product name, logo, and compatibility language | Requires original design and trademark review | Before public branding | `BLOCKED` for public branding and comparative claims |
| `OQ-0003` | Concrete fictional controller/module profile values | Research defines the model, not exact generic limits | Phase 2 hardware specification | `DEFERRED` to Phase 2 |
| `OQ-0004` | Identifier grammar, case rules, scope/shadowing, and automatic address allocation | Observable semantics require precise rules | Phase 2 tag/type specification | `DEFERRED` to Phase 2 |
| `OQ-0005` | Exact project object schema and migration/downgrade policy | Phase 1 fixes boundaries but not the full domain schema | Phase 2 persistence specification | `DEFERRED` to Phase 2 |
| `OQ-0006` | LAD/FBD legality corpus and vendor-specific SCL edge behavior | Research flags unresolved fidelity details | Phase 2; unsupported details remain blocked | `DEFERRED`; unsupported or vendor-specific details remain `BLOCKED` |
| `OQ-0007` | Teacher/student file protection and local audit retention defaults | Needs UX/privacy decisions | Phase 3 Teacher Mode specification | `DEFERRED` to Phase 3 |
| `OQ-0008` | Accessibility conformance target and performance/capacity budgets (`PES-ACC-0008` through `PES-ACC-0010`) | Must be objective before experience acceptance | Phase 3 | `DEFERRED` to Phase 3 |
| `OQ-0009` | PID, motion, SFC, classic HMI, safety-awareness, and legacy-language scope | Deferred and partly legal/research-gated | Phase 4 or separate addendum | `DEFERRED`; Class 7/8 and safety-related work remains `BLOCKED` |
| `OQ-0010` | Human training-transfer study design and authorized lab baseline | Needs course/instructor context | Phase 4 | `DEFERRED` to Phase 4 |

## Resolution ledger

- `DEC-0001` was resolved by `CR-0001` on 2026-08-27 for canonical source identity and separate phase-directive sequencing. The exact `PES-ACC-0005` status-text acceptance issue remains unresolved.
- `DEC-0002` was partially resolved by `CR-0001` on 2026-08-27 for active checked-in CI configuration and the executable local closure gate only. Remote creation/publication/push, credentials, hosted execution, service terms, hosted logs, report upload, and retention remain blocked.
