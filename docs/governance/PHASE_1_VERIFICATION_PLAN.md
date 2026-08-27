# Phase 1 Verification Plan

## Scope

This plan defines the corrective Phase 1 closure verifier. It separates a
trusted expected baseline from the subject under test, requires named failures
for mutation credit, and retains the original structural checks with their
claim boundaries. It does not authorize Phase 2 PLC product work.

## Executable checks

A verification ID identifies a current-snapshot check, not automatic completion
of every requirement it touches. The final column deliberately distinguishes a
guardrail or structural check from completion evidence.

| Verification ID | Current-snapshot evidence | Claim boundary |
|---|---|---|
| `VER-GOV-0001` | All five canonical source/reference files under `References for Codex from Scott/` exist and match their recorded SHA-256 values | Source-integrity guardrail for `PES-GOV-0010` and `PES-GOV-0013`; reviewer acceptance and citation-resolution remain separate |
| `VER-INT-0001` | The manifest supplied from outside the subject matches its sealed SHA-256, has the fixed schema/exclusion policy, identifies a full Git commit, and exactly matches the subject's committed manifest bytes | Establishes manifest identity and provenance; Scott's acceptance of the candidate commit remains external |
| `VER-INT-0002` | The complete non-local project path set, byte lengths, and SHA-256 values match the externally supplied Git-object manifest | Independent subject-versus-expected integrity comparison; observed hashes in the generated report are not expected values |
| `VER-DOC-0001` | Required Phase 1 files are present; the three proposed boundary ADRs contain their mandated decisions without claiming approval; the all-page DOCX observation record is source-hash-bound; ignored local render evidence is count/hash-validated when present and explicitly reported absent otherwise | Structural and observation-record guardrail for `PES-DOC-0003`; the unapproved renderer output is not admissible visual-QA, policy, tool-admission, or reviewer-acceptance evidence |
| `VER-DOC-0002` | ADR-0001 has the exact title/status and immutable separate-product clause | Current-scope evidence for `PES-DOC-0001` and `PES-DOC-0002` only |
| `VER-ADR-0001` | ADR-0001 through ADR-0004 each exist as non-empty regular files | Controlled missing-file failure; ADR content/approval is evaluated separately |
| `VER-REQ-0001` | Schema-v3 reconciliation contains 484 issued IDs, 20 historical compound parents, 464 atomic records, 463 completion-eligible atomic records, 546/546 mapped source units, and 789 reciprocal source-unit relationships; all snapshots are extractor- and source-bound | Independent derived coverage/lineage guardrail for `PES-REQ-0001` through `PES-REQ-0004`; it still does not convert unreviewed requirement semantics into acceptance |
| `VER-REQ-0002` | Records/matrix share exact coverage and states; curated foundation mappings are contract-bound; later dependency/acceptance baselines are explicitly unresolved | Current-scope structural evidence for `PES-REQ-0008` and `PES-REQ-0009`; `PES-REQ-0005` through `PES-REQ-0007` remain unimplemented |
| `VER-CRM-0001` | Clean-room controls are substantive; evidence/asset schemas, hashes, empty-inventory state, and unsigned attestation are truthfully represented | Guardrail evidence for `PES-CRM-0016` and `PES-DOC-0004`; registers under `PES-CRM-0017`/`PES-CRM-0021` remain unreviewed, not approved |
| `VER-DEC-0001` | `OQ-0001` through `OQ-0010`, recorded decisions, and `RSK-0001` through `RSK-0010` are present; `DEC-0001` has its exact resolution and `DEC-0002` has its exact partial resolution plus blocked external-operation boundary | Decision-truth guardrail only; the active checked-in CI/local gate is authorized, while remote creation, push, hosted execution, credentials, logs, upload, and retention remain blocked |
| `VER-RSK-0001` | Every risk has a parseable status; a CLOSED/ACCEPTED risk requires one complete closure record linked to approved/verified evidence, passing checks, review, and change control | Validates closure-evidence structure and references; it does not independently prove that a reviewer judgment is correct |
| `VER-QLT-0001` | Repository inventory and status documents remain internally consistent and reject unauthorized completion claims | Current-scope guardrail; product-foundation authorization is checked separately by `VER-SCP-0001` |
| `VER-SCP-0001` | Every file under `apps/`, `packages/`, or `crates/` exactly matches the explicit Phase 1 foundation allowlist | Prevents unreviewed product-root additions; the allowlist does not prove feature correctness |
| `VER-FND-0001` | The exact typed health command, DomainResult envelope, UI-worker-WASM round trip, deterministic Rust payload, negative contract tests, repeated offline browser result, and product-source capability ban are all present | Independent semantic guardrail for WP-0G; the full `gate:foundation` still executes build, lint/typecheck, unit, isolation, and browser tests |
| `VER-ISO-0001` | Safety/threat controls are substantive and no forbidden connector/transport/plugin boundary or symlink exists | Current-scope negative guardrail for `PES-SEC-0017`, `PES-DEV-0012`, and `PES-QLT-0005`; it is not product isolation proof |
| `VER-OFF-0001` | No unauthorized absolute FTP, HTTP(S), or WebSocket URL occurs in exact restricted product/config scopes | Scoped static guardrail; research, legal, provenance, policy-checker source, and audit references are outside this scan |
| `VER-OFF-0002` | No unauthorized localhost or loopback endpoint occurs in exact restricted production/config scopes | Scoped static guardrail; policy-checker regex literals and runtime zero-attempt testing are outside this static scan |
| `VER-BRN-0001` | No Siemens, SIMATIC, TIA Portal, WinCC, or PLCSIM mark occurs in exact user-facing scopes without a literal counsel/change-control allowlist entry | Product-identity guardrail; quarantined research/legal references are deliberately outside this scan |
| `VER-DEP-0001` | Every direct npm and Cargo declaration exactly matches the bounded Phase 1 candidate allowlist, every entry remains `CANDIDATE_UNREVIEWED`, and known network-capable names receive an explicit failure | `CR-0001/WP-0G` authorizes bounded use/evaluation, not legal, security, or release approval; lockfile bytes are baseline-bound and full resolved capability review remains required |
| `VER-CI-0001` | Pinned local tool versions, private root identity, required scripts, safe workspace/lock formats, active push/PR/manual CI, exact action pins, and the shared full closure command are declared; remote artifact upload is absent | Executable gate configuration only; it does not claim that a hosted run occurred or authorize report publication/data retention |

## Trusted-baseline protocol

`tests/phase1/trusted-baseline.json` is deliberately not self-hashed. Its
sorted file records cover every non-local project file except itself and the
fixed ignored build/tool directories and exact generated WASM-module path.
There is no filename-pattern exemption for `.env`, `*.local`, temporary, log,
or operating-system metadata files; any such file outside an excluded directory
must be removed or explicitly baselined. The manifest's own bytes are protected by the
immutable Git commit from which the launcher reads it. The verifier then
compares the worktree copy byte-for-byte with that externally supplied copy.

The final manifest must be generated only after the audit, foundation, policy,
test, and evidence files are complete:

```powershell
node tools/phase1/run_pinned_python.mjs -B tools/phase1/generate_trusted_baseline.py `
  --root . `
  --output tests/phase1/trusted-baseline.json
```

After committing that exact tree, resolve the commit once and verify against
its Git object:

```powershell
$BaselineCommit = git rev-parse --verify 'HEAD^{commit}'
node tools/phase1/run_pinned_python.mjs -B tools/phase1/run_phase1_verification.py --baseline-ref $BaselineCommit
```

The verifier never falls back to hashes recalculated from the subject. The
generated JSON report may contain observed hashes for diagnostics, but marks
them explicitly as observations.

Exit codes are part of the interface:

- `0`: all named checks passed.
- `1`: at least one named policy check failed.
- `2`: runtime, usage, manifest trust, timeout, parser, or internal tool error.

## Mutation gate

The controlled PowerShell harness resolves one commit, creates one Git archive,
and copies all twelve subjects from that frozen archive. It also runs the clean
baseline first and a separate sealed-manifest tamper test. A prescribed
mutation receives credit only when it exits `1`, prints its intended `FAIL`
check and detail, prints no `ERROR`, and produces no crash signature.

```text
node tools/phase1/run_phase1_mutations.mjs --baseline-ref HEAD
```

The harness returns `0` only for a clean baseline, 12/12 intended named
detections, and a successful manifest-tamper rejection. Generic nonzero exits,
environment failures, and the former ADR-deletion `ENOENT` receive no credit.
The wrapper resolves Python 3.13.12 through `PHASE1_PYTHON`, `PATH`, or the
Windows per-user Python 3.13 installation and creates a unique task-specific
scratch directory under the operating-system temporary root.
Before removing that unique scratch directory, the wrapper copies the complete
JSON transcript (raw outputs, expected/actual detectors, exits, crash flags,
hashes, and tamper result) to the ignored durable path
`.phase1-verification/mutations/mutation-results.json` and prints its absolute
`EVIDENCE_PATH`.

The root-package entries used by both local and fixed-runner CI are:

```text
verify:mutations = node tools/phase1/run_phase1_mutations.mjs --baseline-ref HEAD
gate:closure = pnpm requirements:check && pnpm gate:foundation && pnpm verify:phase1 && pnpm verify:mutations
```

## Explicit scopes and allowlists

`tests/phase1/policy-contract.json` owns four narrow lists bound to the admitted
Phase 1 foundation:

- `phase1FoundationFileAllowlist`: every allowed file under `apps/`,
  `packages/`, and `crates/`;
- `authorizedCandidateDependencies`: exact npm/Cargo manifest, field/section,
  name, declaration, authorization record, and `CANDIDATE_UNREVIEWED` status;
- `scopedContentPolicy.restrictedTextFiles`: files scanned for external URLs
  and loopback endpoints;
- `scopedContentPolicy.userFacingTextFiles`: files scanned for vendor-facing
  identity text.

The product-path allowlist contains the exact 22 checked-in shell, typed
contract, and Rust/WASM files. The candidate-dependency inventory contains the
exact 13 direct npm declarations and no Cargo dependency. The generated WASM
module has one fixed excluded path and is never a committed baseline file.
Research, legal, provenance, requirements, tests, policy-checker source, and
audit evidence are not globally scanned for marks or URLs because those
references or detection literals are expected there.

## Risk-closure evidence

`EVIDENCE_REGISTER.json` defines `riskClosureEvidence`. While all risks are
OPEN, the array is empty. A later CLOSED or ACCEPTED status requires exactly one
record with approved and verified evidence references, passing verification
IDs, unique/resolved ADR or decision linkage, a recorded `CR-NNNN` change
record, reviewer/date, rationale, and residual risk. Unknown-risk records,
duplicate references, or editing only the Markdown status are named
`VER-RSK-0001` failures.

## Defined but not yet executable

The following release proofs remain `NOT_STARTED` because the authorized
foundation is not a packaged or release product artifact: production dependency closure, trusted-source capability scanning,
semantic/runtime WASM import inspection, network-adapter-disabled course runs,
process-scoped zero-attempt tests, inert endpoint fuzzing across product fields,
Virtual Download boundary proof, InternalTagBus-only HMI proof, export artifact
proof, SBOM/license notices, and packaged-artifact scans.

Marking any of those checks passed before the affected product exists would
violate the anti-placeholder policy.
