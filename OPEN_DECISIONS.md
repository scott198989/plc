# Open Decisions

## Purpose and handling rules

This register records unresolved product decisions carried by the Phase 1 directive. It does not amend the directive and does not treat an unanswered question as approval.

- `BLOCKED` means only the affected work must stop.
- `DEFERRED` means the decision belongs to the stated later authoring phase and is not implemented.
- Unaffected Phase 1 governance, safety, clean-room, traceability, and repository-foundation work may continue.
- A resolution is valid only when Scott supplies the required approval or evidence and the resulting directive change, if any, is recorded under `PES-GOV-0014`, `PES-GOV-0015`, and `PES-GOV-0016`.
- Silence, elapsed time, a placeholder, or an implementation guess is not a resolution.

## BLOCKED product decisions

### DEC-0001 - Controlling document and source-file identity

**Status:** BLOCKED  
**Date opened:** 2026-08-27  
**Owner and decision authority:** Scott  
**Affected requirement IDs:** `PES-GOV-0001`, `PES-GOV-0002`, `PES-GOV-0006`, `PES-GOV-0010`, `PES-GOV-0014`, `PES-GOV-0015`, `PES-GOV-0016`, `PES-GOV-0017`, `PES-GOV-0018`, `PES-GOV-0019`, `PES-GOV-0020`, `PES-DEC-0002`, `PES-DEC-0004`, `PES-DEC-0005`, `PES-DEC-0006`, `PES-ACC-0005`, `PES-ACC-0006`, `PES-ACC-0007`

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

#### Unknown or conflicting point

It is not yet clear whether the phase-numbered DOCX files are intended to be non-controlling implementation prompts feeding one canonical living directive, successive revisions of that one directive, or four independently controlling directives. It is also unclear which exact filename Scott wants to control the living directive, whether the research filename in Document Control should be corrected to the filename actually supplied, and whether Scott authorizes replacing the observed status text with the exact wording required by `PES-ACC-0005`.

#### Why this cannot be decided autonomously

The choice changes authoritative document identity, source provenance, cross-phase change control, filename stability, and the risk of competing directives. Silently renaming, copying, or treating separate phase prompts as separate master authorities could violate the directive's authority hierarchy and one-document rule.

#### Options

**Option A - One canonical living directive plus non-controlling phase prompts (recommended).**  
Treat phase-numbered files as source prompts or revision inputs. After explicit approval, establish `PLC Engineering Simulator - Codex Master Implementation Directive.docx` as the sole controlling living directive, preserve phase prompts as clearly labeled non-controlling source records, and correct the research filename through an approved change record. This best preserves the one-document rule while accommodating the user's phase-prompt workflow.

**Option B - The Phase 1-suffixed file is the living directive.**  
Keep `PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx` as the one controlling file and approve a change to the Document Control filename rule. Later phases would revise that same file rather than create independently controlling phase files.

**Option C - Separate controlling directive for each phase.**  
Approve a substantive change that supersedes the one-living-document rule and defines precedence, supersession, cross-reference, and contradiction handling for four controlling documents. This creates the greatest governance and divergence risk.

#### Recommendation

Approve Option A. It preserves one controlling product truth, permits separate phase prompts as inputs, and avoids silently treating filenames as authority.

#### Exact approval needed

Scott must:

1. Select Option A, B, or C.
2. Confirm the exact controlling master-directive filename.
3. Confirm whether future phase-numbered DOCX files are non-controlling prompts, revisions of one file, or independent authorities.
4. Confirm that Document Control should identify the research baseline as `Govs PLC project Research Report.md` with the verified SHA-256 above.
5. Confirm whether the controlling directive status must be changed to the exact `PES-ACC-0005` wording: `Phase 1 authored; Phases 2-4 not yet authored.`
6. Authorize the corresponding change record and any later DOCX rename, copy, status correction, or edit.

#### Work that can continue

Phase 1 governance files, clean-room controls, safety invariants, threat modeling, requirements/evidence registers, ADRs, repository boundaries, and non-packaging CI guardrails may continue. No source DOCX rename or edit, public branding, packaging decision, or Phase 2-4 feature implementation is authorized by this record.

### DEC-0002 - Remote CI provider and verification-report upload

**Status:** BLOCKED  
**Date opened:** 2026-08-27  
**Owner and decision authority:** Scott  
**Affected requirement IDs:** `PES-DEC-0001`, `PES-DEC-0002`, `PES-DEC-0004`, `PES-DEC-0005`, `PES-DEC-0006`, `PES-SEC-0004`, `PES-CI-0001`, `PES-CI-0003`

#### Known facts

1. The project-local Git repository has no configured remote.
2. Local offline extraction and verification use pre-existing host tools under the narrow bootstrap containment in `DEPENDENCY_POLICY.md` Section 5.1.
3. A proposed GitHub Actions workflow declares a GitHub-hosted `windows-2025` runner, three setup/checkout actions, and an action that would upload the generated verification report for 14-day retention.
4. All action commits, the mutable runner, their licenses/provenance, service terms, and data-handling controls remain unapproved in `docs/governance/TOOLCHAIN_ADMISSION_REGISTER.md`.
5. The workflow has only a manual trigger and its sole job has the literal condition `${{ false }}`. It cannot execute or upload through the declared job without a reviewed repository change.
6. No completed hosted-run or remote-upload evidence is claimed.

#### Why this cannot be decided autonomously

Selecting hosted CI and artifact storage introduces cloud, remote-service,
credential, data-retention, and provider-policy behavior. `PES-DEC-0002`
requires Scott's decision before that affected work proceeds.

#### Options

**Option A - Keep Phase 1 verification local-only for now (recommended).**  
Retain the disabled workflow as a reviewable proposal. Continue offline local checks and reconsider a CI provider when repository hosting is intentionally selected.

**Option B - Approve GitHub-hosted verification without artifact upload.**  
After tool/action admission review, enable protected-branch and/or push/PR checks but remove remote report retention. Logs would still leave the machine and require a data/service review.

**Option C - Approve GitHub-hosted verification and report retention.**  
After tool/action and service/data review, enable the proposed workflow and approve the exact report fields, repository visibility, access controls, retention period, and deletion expectations.

#### Exact approval needed

Scott must select A, B, or C. For B or C, Scott must also approve the provider,
repository visibility/credential model, trigger policy, action-admission review,
and any data leaving the host. For C, Scott must approve the report contents,
artifact readers, and retention/deletion policy. The enabling change must remove
the literal false gate through a recorded review; setting a variable cannot
silently bypass it.

#### Work that can continue

Offline local Phase 1 governance extraction, review, and verification may
continue. Remote workflow execution, hosted logs, checkout credentials, runtime
downloads, and artifact upload remain blocked.

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
| `OQ-0008` | Accessibility conformance target and performance/capacity budgets | Must be objective before experience acceptance | Phase 3 | `DEFERRED` to Phase 3 |
| `OQ-0009` | PID, motion, SFC, classic HMI, safety-awareness, and legacy-language scope | Deferred and partly legal/research-gated | Phase 4 or separate addendum | `DEFERRED`; Class 7/8 and safety-related work remains `BLOCKED` |
| `OQ-0010` | Human training-transfer study design and authorized lab baseline | Needs course/instructor context | Phase 4 | `DEFERRED` to Phase 4 |

## Resolution ledger

No decision in this register is resolved as of 2026-08-27.
