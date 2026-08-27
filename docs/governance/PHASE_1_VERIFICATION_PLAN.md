# Phase 1 Verification Plan

## Scope

This plan verifies the repository constitution and governance foundation only.
It does not claim that any PLC engineering feature, runtime, HMI, lesson,
packaged classroom application, or release artifact exists.

## Executable checks

A verification ID identifies a current-snapshot check, not automatic completion
of every requirement it touches. The final column deliberately distinguishes a
guardrail or structural check from completion evidence.

| Verification ID | Current-snapshot evidence | Claim boundary |
|---|---|---|
| `VER-GOV-0001` | Both frozen source files exist and match their recorded SHA-256 values | Source-integrity guardrail for `PES-GOV-0010` and `PES-GOV-0013`; filename and citation-resolution work remains open |
| `VER-DOC-0001` | Required Phase 1 files are present; the three proposed boundary ADRs contain their mandated decisions without claiming approval; the all-page DOCX observation record is source-hash-bound; ignored local render evidence is count/hash-validated when present and explicitly reported absent otherwise | Structural and observation-record guardrail for `PES-DOC-0003`; the unapproved renderer output is not admissible visual-QA, policy, tool-admission, or reviewer-acceptance evidence |
| `VER-DOC-0002` | ADR-0001 has the exact title/status and immutable separate-product clause | Current-scope evidence for `PES-DOC-0001` and `PES-DOC-0002` only |
| `VER-REQ-0001` | Exactly 247 unique IDs are extracted with source headings, no structural spillover, valid cross-references, and snapshots bound to the extractor hash | Current registry guardrail for `PES-REQ-0001` through `PES-REQ-0004`; it does not resolve later requirement semantics |
| `VER-REQ-0002` | Records/matrix share exact coverage and states; curated foundation mappings are contract-bound; later dependency/acceptance baselines are explicitly unresolved | Current-scope structural evidence for `PES-REQ-0008` and `PES-REQ-0009`; `PES-REQ-0005` through `PES-REQ-0007` remain unimplemented |
| `VER-CRM-0001` | Clean-room controls are substantive; evidence/asset schemas, hashes, empty-inventory state, and unsigned attestation are truthfully represented | Guardrail evidence for `PES-CRM-0016` and `PES-DOC-0004`; registers under `PES-CRM-0017`/`PES-CRM-0021` remain unreviewed, not approved |
| `VER-DEC-0001` | `OQ-0001` through `OQ-0010`, blocked decisions, and `RSK-0001` through `RSK-0010` are present | Presence/stop-state guardrail only; it does not resolve decisions or satisfy change control |
| `VER-QLT-0001` | Reserved product roots are absent, exactly one workspace item is scaffolded with zero credit, and status documents reject Phase 1/product completion claims | Current-scope guardrail for `PES-ARC-0030`, `PES-DEV-0010`, `PES-QLT-0001`, `PES-QLT-0004`, `PES-ACC-0006`, and `PES-ACC-0007` |
| `VER-ISO-0001` | Safety/threat controls are substantive and no forbidden connector/transport/plugin boundary or symlink exists | Current-scope negative guardrail for `PES-SEC-0017`, `PES-DEV-0012`, and `PES-QLT-0005`; it is not product isolation proof |
| `VER-CI-0001` | Exact empty manifests/lockfiles, local tool versions, proposed action commits, stale-snapshot rejection, verifier syntax, and report retention are declared | Local scaffold evidence for `PES-DEV-0006`; the proposed remote workflow is disabled by `DEC-0002`, so hosted execution and release gates in `PES-CI-0001` through `PES-CI-0003` remain partial or not executable |

## Defined but not yet executable

The following release proofs remain `NOT_STARTED` because no product artifact
exists: production dependency closure, trusted-source capability scanning,
semantic/runtime WASM import inspection, network-adapter-disabled course runs,
process-scoped zero-attempt tests, inert endpoint fuzzing across product fields,
Virtual Download boundary proof, InternalTagBus-only HMI proof, export artifact
proof, SBOM/license notices, and packaged-artifact scans.

Marking any of those checks passed before the affected product exists would
violate the anti-placeholder policy.
