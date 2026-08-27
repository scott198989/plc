# Risk Register

## Status

This register carries forward `RSK-0001` through `RSK-0010` from the Phase 1 directive. Each risk is `OPEN`. The directive defines a Phase 1 control, but repository implementation and verification evidence are not yet complete. A written control is not proof that the risk has been mitigated.

Risk acceptance, closure, or material control changes require traceable evidence and the applicable decision, ADR, or directive change record. No risk may be silently waived.

| ID | Risk | Phase 1 control | Current status and evidence |
|---|---|---|---|
| `RSK-0001` | A "virtual network" abstraction accidentally becomes host networking | Opaque value types, no endpoint API, product-wide capability ban, zero-egress proof | `OPEN`; architectural rule recorded, implementation and zero-egress evidence not yet available |
| `RSK-0002` | UI fidelity drifts into copying or trade dress | Original expression, evidence classification, asset provenance, counsel-gated public comparisons | `OPEN`; governance control recorded, design/provenance enforcement not yet verified |
| `RSK-0003` | Lessons become canned demonstrations | Command-based causality and explicit prohibition on injected messages/results | `OPEN`; later-phase implementation and causal verification not yet available |
| `RSK-0004` | Separate language runtimes diverge | Unified typed IR and one runtime | `OPEN`; constitutional architecture recorded, compiler/runtime implementation not started |
| `RSK-0005` | Time and replay become nondeterministic | Simulator clock, event ordering, profile/build/snapshot hashes | `OPEN`; proposed ADR-0004 records the boundary without claiming approval, and no runtime/replay evidence exists |
| `RSK-0006` | Untrusted projects become an execution or resource attack | No code execution, strict schemas, archive limits, fuzzing | `OPEN`; security/threat-model controls exist, but no project parser, archive-limit harness, or fuzz evidence exists |
| `RSK-0007` | Scaffolding is presented as progress | Truth states, zero credit for scaffolding, milestone Definition of Done | `OPEN`; the matrix and CI guardrails are implemented, but remain `IMPLEMENTED_UNVERIFIED`/`PARTIAL`, and no product milestone evidence exists |
| `RSK-0008` | New research silently moves the target | Frozen baseline and approved evidence/change records | `OPEN`; baseline hash verified, filename discrepancy remains in `DEC-0001` |
| `RSK-0009` | Safety education is mistaken for safety engineering | No safety-rated claims or feature set without separate addendum | `OPEN`; scope prohibition recorded, release/public-claim enforcement not yet verified |
| `RSK-0010` | Browser or desktop shell background activity violates zero egress | Packaging decision gate, bundled assets, CSP, process-scoped syscall/packet tests | `OPEN`; packaging is blocked by `OQ-0001`, and isolation evidence is not yet available |

## Review triggers

Review this register when any of the following occurs:

- A directive phase is authored or amended.
- A product or architecture decision is approved.
- A dependency, asset, parser, persistence surface, worker boundary, packaging model, or public claim is introduced or changed.
- An isolation, determinism, provenance, migration, or causal-fidelity verification fails or becomes inconclusive.
- A risk obtains reproducible verification evidence sufficient to consider control effectiveness or closure.

## Current risk posture

- No risk is closed.
- No risk has been accepted as residual risk.
- No Phase 2-4 implementation evidence exists.
- Automated governance checks do not self-promote registry records to `VERIFIED`; reviewer acceptance is not recorded.
- Evidence, asset, and toolchain records remain provisional or unreviewed, and the contributor attestation remains unsigned.
- `DEC-0001`, `DEC-0002`, `OQ-0001`, and `OQ-0002` block only their affected work; unrelated Phase 1 foundation work may continue.
