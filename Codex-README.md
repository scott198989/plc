# PLC Engineering Simulator

> Comprehensive internal handoff for the next engineer or Codex instance.
>
> **Read this before changing code.** This repository contains a substantial
> Phase 2 implementation, but it is **not an accepted Phase 2 release**. The
> current formal verdict is `BLOCKED`, Scott must accept any candidate, and
> Phase 3 and Phase 4 work remains unauthorized.

This project is a professional, brand-neutral, offline educational PLC
engineering simulator. It is intended to teach the engineering decisions,
cause-and-effect relationships, commissioning habits, and diagnostic reasoning
used in a modern PLC workflow without copying a vendor product and without ever
connecting to real industrial equipment.

The shortest accurate description is:

> Build fictional PLC projects, compile LAD/FBD/SCL into one typed IR, run them
> deterministically against an internal virtual controller, and inspect the
> result with realistic engineering and commissioning tools—all inside a sealed
> virtual universe.

The permanent product boundary is:

> **`VirtualUniverse` has no adapter to `PhysicalUniverse`.**

That sentence is not a slogan. It is an architectural, safety, governance, and
test invariant. If a proposed change weakens it, the change is out of scope.

---

## Status at a glance

This table is the fastest way to avoid the most expensive misunderstandings.

| Question | Current answer |
|---|---|
| What phase is authorized? | Phase 2: Runnable PLC Engineering Core |
| Is Phase 2 implemented? | A broad runnable implementation exists across the UI, compiler, runtime, commissioning, observability, persistence, and native file shell |
| Is Phase 2 verified or accepted? | No. The formal Phase 2 evidence ledger is incomplete and the current verdict is `BLOCKED` |
| Who can accept Phase 2? | Scott—not Codex, not a passing local test, and not the terminal gate by itself |
| May work begin on Phase 3? | No |
| May work begin on packaging or public release? | No; that is Phase 4 and remains unauthorized |
| Does “Go online” contact a device? | Never. It means connect to an in-process virtual controller only |
| Can the product import/export a vendor PLC project? | No |
| Is there a configured Git remote or demonstrated hosted build? | No |
| Is there an installer or production deployment? | No |
| Current Git snapshot used for this handoff | `main` at `58bd6f81c023735a1ebbf2b1db1342c3483a8bd7` before this README edit |
| Current implementation identity | `plc-engineering-core@0.2.0` |

The root `package.json` still says `0.0.0-phase1` and has a Phase 1-only
description. That metadata is stale; do not use it to infer actual capability
or phase status.

## If you have only five minutes

1. Read the [Phase 2 master directive](<References for Codex from Scott/PLC Engineering Simulator - Codex Master Implementation Directive - Phase 2 of 4 - Runnable PLC Engineering Core.docx>). It is the controlling implementation instruction for current work.
2. Read the [research report](<References for Codex from Scott/Govs PLC project Research Report.md>) for product intent, workflow, fidelity, architecture, and long-range context—but do not let it override the directive.
3. Read [this repository's Phase 2 tooling guide](tools/phase2/README.md) before collecting or finalizing evidence.
4. Run `git status --short` before touching anything. At this handoff, there are intentional user-owned changes described in [Current working-tree state](#current-working-tree-state).
5. Put the exact Node `24.19.0` runtime first on `PATH`; the ordinary host `node` currently resolves to `24.14.0`.
6. Do not confuse implemented behavior, static coverage, verification credit, and acceptance. They are four separate states.
7. Do not start Phase 3 features such as a process lab, HMI, Teacher Mode, Learning Lens, lessons, scenarios, or assessment.
8. Do not add networking, industrial protocols, device discovery, endpoints, telemetry, remote fonts, CDNs, or generic connector abstractions.
9. Treat ignored `.phase2-verification/` material and `Phase 2 Review Package/` as local/generated context, not canonical source or current candidate evidence.
10. When Phase 2 evidence eventually passes, stop at **“Phase 2 implementation candidate—awaiting Scott acceptance.”** Do not self-accept and continue.

## Purpose

The simulator exists to let a learner practice a coherent PLC engineering
workflow without access to physical automation equipment and without creating a
path that could later be pointed at such equipment.

The intended end-to-end experience is:

1. Create a simulator-native project.
2. Configure fictional controller, rack, I/O, and virtual-network objects.
3. Define tags, addresses, scalar types, aggregate types, and data blocks.
4. Author organization blocks, functions, function blocks, and data blocks.
5. Program in genuine LAD, FBD, and SCL representations.
6. Build all languages through one compiler into one typed intermediate
   representation.
7. Preview and atomically commit a load to an internal virtual controller.
8. Move the virtual CPU through power, STOP, and RUN states.
9. Execute deterministic scans using virtual time and virtual process images.
10. Monitor values, modify values, force values, trace execution, inspect
    diagnostics, and navigate back to source.
11. Save and reopen the original `.vlabproj` project format through the narrow,
    approved Windows file broker.
12. Reproduce behavior from snapshots and replay packages.

The fidelity goal is **causal fidelity, not screenshot fidelity**. The learner
should make the same kinds of engineering decisions and see the same kinds of
consequences found in a modern PLC workflow. The product's code, assets,
identity, hardware universe, project format, and communications remain original
and isolated.

This is not intended to be:

- a Siemens emulator, clone, skin, training copy, or file-compatible substitute;
- an industrial control system;
- a gateway, soft PLC, protocol client, device browser, or engineering station
  for real hardware;
- a tool that produces deployable industrial controller artifacts;
- a public product, installer, or supported distribution in the current phase.

## Non-negotiable safety and clean-room boundary

The product must remain useful precisely because it is internally complete, not
because it borrows capability from the physical world.

### Permanently prohibited

- Physical PLC, HMI, drive, I/O, sensor, actuator, or controller access.
- Industrial protocol stacks or protocol-shaped adapters.
- Host NIC enumeration, network discovery, device discovery, broadcast,
  multicast, socket, serial, USB, Bluetooth, fieldbus, or gateway behavior.
- URLs, endpoints, cloud synchronization, remote APIs, telemetry, analytics,
  remote logging, remote fonts, remote scripts, or CDNs.
- Vendor project import/export, vendor device identities, vendor package
  compatibility, or physical download/upload.
- A generic “connector,” “transport,” “driver,” “device,” or “provider”
  interface that could later be implemented for physical hardware.
- External HMI connections or external clients controlling the simulated PLC.
- Raw filesystem paths crossing into the renderer.
- Any interpretation of “Go online” other than the internal simulator.

### Required structural properties

- `VirtualControllerId` identifies only an internal simulated controller.
- Virtual Download targets only the internal `VirtualUniverse`.
- The WebAssembly engineering core remains capability-limited and import-free.
- The browser artifact remains self-contained and denies network connections.
- Native file access remains a narrow, typed, local-project capability rather
  than a general filesystem API.
- Runtime behavior remains deterministic under identical canonical inputs,
  project state, virtual time, and seed.
- All three programming representations lower into one semantic IR and execute
  through one runtime.
- The UI consumes domain read models and receipts; it does not become a second
  authoritative PLC implementation.

Before adding a dependency or host API, ask: “Could this create an adapter from
the virtual universe to the physical universe, now or through an obvious future
implementation?” If yes, stop.

The controlling safety documents are [ADR-0001](ADR/0001-no-physical-industrial-communication.md),
[SECURITY_INVARIANTS.md](SECURITY_INVARIANTS.md),
[THREAT_MODEL.md](THREAT_MODEL.md), and
[CLEAN_ROOM_POLICY.md](CLEAN_ROOM_POLICY.md).

## Source authority: what wins when documents disagree

This repository contains original directives, research, audits, extracted JSON,
implementation notes, generated evidence, and older status registers. They do
not all have equal authority.

Apply this order, highest first:

1. Applicable law, binding licenses, and the immutable prohibition on any
   `PhysicalUniverse` adapter.
2. Scott's explicit decisions and start orders.
3. The Phase 2 master directive for all Phase 2 work.
4. The accepted Phase 1 directive, corrective addendum, ADRs, and permanent
   invariants.
5. The frozen research report as technical and contextual evidence.
6. Repository implementation choices, comments, derived registers, and local
   assumptions.

If two sources conflict, preserve the higher authority, block only the affected
work, record the smallest necessary decision, and continue unrelated work. Do
not silently “harmonize” a contradiction by rewriting IDs or changing the
meaning of a lower-level record.

### Scott's core reference set

The original files are preserved in
[`References for Codex from Scott/`](<References for Codex from Scott/>). Their
hashes are included so a future instance can establish that it is reading the
same material.

| Document | SHA-256 | Role |
|---|---|---|
| [Govs PLC project Research Report.md](<References for Codex from Scott/Govs PLC project Research Report.md>) | `F05C08323B5CC9483BEB1FEB3C7312CCB9A45EBE3B527E6DAE069C181D3FBF55` | Primary product research and research-to-build specification; important context, not the highest normative authority |
| [Phase 1 master directive](<References for Codex from Scott/PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx>) | `EBF074E2CEAB752F09E6DB63D88E100991729DA13C1EB874290A6B337DA72612` | Phase 1 governance, clean-room, requirements, and foundation instructions |
| [Phase 1 corrective addendum](<References for Codex from Scott/PLC Engineering Simulator - Phase 1 Corrective Addendum - Closure and Trusted Baseline.docx>) | `950C5112C34D0218FD1E59CF6C051ACCD01AB92674CD70C96C08A5F1DA2E5A1C` | Closure and trusted-baseline corrections that govern the accepted Phase 1 foundation |
| [Phase 2 master directive](<References for Codex from Scott/PLC Engineering Simulator - Codex Master Implementation Directive - Phase 2 of 4 - Runnable PLC Engineering Core.docx>) | `938A0958F0CF15739A2DC8ED674F7C9F25D531DCE32CCA6A4CEEE5D638E68536` | Current controlling implementation directive and Phase 2 start order |
| [Pre-remediation red-team audit](<References for Codex from Scott/CODEX RED TEAM AUDIT PHASE 1.docx>) | `515BD6A844AF1A2E71C829BA567214F73CA4E455278157960A3A11F25B8DF5B3` | Historical evidence that found Phase 1 extraction and mutation weaknesses; not a current directive |
| [Post-red-team audit](<References for Codex from Scott/CODEX AUDIT AFTER RED TEAM.docx>) | `4D85D71CEB36AC0E4892CBF91FCDB4B6CE8FFC27E80DA20D754EE8008ABEB87F` | Historical follow-up handoff; useful context but superseded by final closure records |
| [Final Phase 1 adversarial audit](docs/governance/PHASE_1_ADVERSARIAL_AUDIT.docx) | `2075F9BF6D082E1F82898693823475B9C49BD284DB013A8AF34A529D9CD4E8AC` | Final closure audit. The copy in Scott's references is intentionally present locally as an untracked byte-identical duplicate |

The `.docx` files are binary canonical sources. Extracted requirements and
Markdown reports make them testable and reviewable, but they do not replace the
original documents. When exact language matters, read the original document
and verify its hash.

### What the research report contributes

The research report is titled **“Clean-Room High-Fidelity PLC Engineering
Simulator: Authoritative Research-to-Build Specification.”** It provides the
best zero-context explanation of what the product is supposed to feel like and
why. In particular, it defines or motivates:

- the modern, V21-era workflow baseline while requiring fictional devices and
  original product identity;
- the sealed simulation wall and the distinction between causal fidelity and
  visual copying;
- a project-centric engineering workspace and realistic compile/load/online
  state transitions;
- genuine LAD, FBD, and SCL semantics feeding a unified IR;
- deterministic scan-cycle, process-image, virtual-time, fault, snapshot, and
  replay behavior;
- monitoring, watch tables, Modify, Force, trace, diagnostics, and
  source-navigation behavior;
- the long-range three-mode vision: Engineering Mode, Learning Lens, and
  Instructor Mode;
- proposed architecture, domain-command boundaries, a technology stack,
  roadmap, unresolved fidelity questions, and acceptance criteria;
- a detailed set of `MUST`, `MUST NOT`, `SHOULD`, and `MAY` marching orders.

The report's three modes describe the long-range product vision. They are not
permission to implement all three now. Phase 2 implements the engineering core;
Learning Lens and Instructor/Teacher functionality remain Phase 3 work.

The report uses evidence labels such as `DOCUMENTED`, `INFERENCE`, `PROPOSED`,
`LEGAL INTERPRETATION`, and `ENGINEERING RECOMMENDATION`. Preserve those
distinctions. Research language becomes a binding Phase 2 requirement only when
the controlling directive adopts it.

There is one critical provenance limitation: the report contains
conversation-scoped citation tokens for which the repository has no durable URL
or bibliography mapping. The inventory in
[docs/research/UNRESOLVED_SOURCE_TOKENS.md](docs/research/UNRESOLVED_SOURCE_TOKENS.md)
records 162 citation-marker groups, 250 token occurrences, and 99 unique
`turn...search...` tokens. Do not invent sources, convert those tokens into
citations, or use them as independent evidence for a release gate.

Useful research-report areas for a future instance are:

- Product definition and workflow: opening sections.
- V21-era baseline and fictional device policy: early product-baseline section.
- Simulation wall: the safety and isolation section.
- Engineering/Learning/Instructor modes: mode model and causal-fault rule.
- Runtime and isolation architecture: simulation/runtime technology section.
- Domain-command model, unified IR, and recommended stack: architecture section.
- Build matrix, gaps, roadmap, and definition of done: later planning sections.
- Unresolved fidelity questions: final research-gap section.
- Normative marching orders and final acceptance model: closing sections.

### Derived records that matter

- [requirements/phase2-requirements.json](requirements/phase2-requirements.json)
  contains the deterministic Phase 2 extraction.
- [requirements/phase2-verification-catalog.json](requirements/phase2-verification-catalog.json)
  contains the Appendix H proof catalog.
- [requirements/phase2-reviewed-requirement-mapping.json](requirements/phase2-reviewed-requirement-mapping.json)
  contains the reviewed static implementation mapping.
- [requirements/phase2-extraction-audit.json](requirements/phase2-extraction-audit.json)
  records extraction integrity issues that still require owner review.
- [evidence/phase2/PHASE2_IMPLEMENTATION_STATUS.json](evidence/phase2/PHASE2_IMPLEMENTATION_STATUS.json)
  is the fail-closed terminal status ledger.
- [evidence/phase2/PHASE2_COVERAGE_AUDIT.md](evidence/phase2/PHASE2_COVERAGE_AUDIT.md)
  is static coverage analysis only and grants zero verification credit.

## Authorized scope

### Phase 2: current authorized work

Phase 2 is the **Runnable PLC Engineering Core**. Its authorized packages are:

| Work package | Intended result |
|---|---|
| P2-00 | Entry gate and accepted Phase 1 baseline |
| P2-01 | Canonical project kernel, persistence, migration, recovery, undo/redo |
| P2-02 | Functional project-centric engineering workbench |
| P2-03 | Fictional EDU-21 hardware, virtual network, tags, addresses, and types |
| P2-04 | OB/FC/FB/DB program model, interfaces, instances, calls, and dependencies |
| P2-05 | Compiler, SCL frontend, typed IR, diagnostics, and source maps |
| P2-06 | Semantic LAD model, editing, validation, and lowering |
| P2-07 | FBD and SCL language-service behavior |
| P2-08 | Deterministic virtual PLC runtime, scheduler, memory, I/O, faults, snapshots, and replay |
| P2-09 | Virtual commissioning, load preview/commit, online/offline, and CPU state transitions |
| P2-10 | Monitoring, watch, Modify, Force, trace, diagnostics, and navigation |
| P2-11 | Integrated journeys, anti-theater mutations, exact-candidate evidence, and closure |

The fictional Phase 2 hardware profile is **EDU-21 Core 1.0**. It is a product
of this project, not a vendor catalog.

### Phase 3: explicitly reserved and currently unauthorized

- Process or physics lab.
- Rich process-fault authoring.
- HMI engineering or HMI runtime.
- Teacher Mode or Instructor Mode.
- Learning Lens.
- Lessons, guided labs, scenarios, grading, assessment, or learner analytics.
- Reusable library lifecycle and training-content systems.
- Advanced teaching overlays and accessibility completion.

Do not add placeholder tabs, disabled buttons, dead routes, or speculative data
models for these features. “Just scaffolding it” is still Phase 3 work unless a
higher-authority decision says otherwise.

### Phase 4: explicitly reserved and currently unauthorized

- Installer, updater, code signing, packaging, public distribution, or hosted
  deployment.
- Platform hardening and supported-operating-system claims.
- Measured production performance claims.
- Final accessibility completion.
- Supply-chain/release evidence and public legal/IP review.
- Transfer study, final product acceptance, and public handoff.
- Public branding claims.

## What is implemented today

The old README said Phase 2 had not begun. That is no longer true. The
repository now contains a large, integrated Phase 2 codebase.

### Supported engineering journey

The workbench can currently exercise this core path:

1. Create or open a simulator-native project.
2. Add a fictional virtual network, controller, rack, and I/O.
3. Create and edit tags, named types, data blocks, and program blocks.
4. Author SCL, LAD, and FBD content.
5. Build the project and receive structured diagnostics and source mappings.
6. Power on the internal controller.
7. Preview a virtual load and atomically commit it.
8. Go online with that virtual controller.
9. Enter RUN, set virtual raw input values, and execute deterministic scans.
10. Observe virtual outputs and runtime state.
11. Monitor program values and use watch, Modify, Force, trace, diagnostics,
    snapshot, restore, and replay-verification tools.
12. Save/open `.vlabproj` through the approved Windows native file boundary.

The task-oriented usage guide is
[PHASE_2_PREVIEW_USER_GUIDE.md](PHASE_2_PREVIEW_USER_GUIDE.md).

### Capability map

| Area | Implemented capability | Important boundary |
|---|---|---|
| Project model | Canonical UUID graph, commands, transactions, undo/redo, deterministic serialization, migration, recovery | `.vlabproj` is original and simulator-native |
| Hardware | EDU-21 profile, fictional network/controller/rack/I/O, addresses, symbols, process images, hardware conditions | No vendor hardware identity or physical adapter |
| Types | Canonical scalar and aggregate values, literals, named types, type validation | Only semantics admitted by Phase 2 should be added |
| Program model | OB, FC, FB, DB, interfaces, instances, calls, dependencies, invalidation | One canonical domain model |
| LAD | Semantic graph, editing operations, validation, lowering | Runtime core is richer than the current visual editor |
| FBD | Semantic editor model, diagnostics, lowering | Must feed the same IR as LAD/SCL |
| SCL | Lexer, parser, semantics, language services, control flow, lowering | It is an original supported language subset, not a vendor compiler |
| Compiler | Build attempts/reports, diagnostics, build modes, one typed IR, source maps, mixed-language composition | No separate per-language runtime |
| Runtime | Deterministic CPU lifecycle, scheduler, virtual time, process images, memory, instructions, faults, snapshots | Internal virtual execution only |
| Commissioning | Virtual controller universe, load preview, atomic install, online sessions, state separation | `VirtualControllerId` only; no physical download |
| Observability | Monitor, online policy, watch, Modify, Force, trace triggers, diagnostics, navigation | Observation must not mutate authority accidentally |
| Replay | Canonical replay packages, deterministic re-execution, independent verifier | Final evidence must bind to exact candidate and inputs |
| Workbench | React project home, project tree, editors, properties, runtime controls, diagnostics | UI is a projection over domain receipts/read models |
| Persistence shell | Windows WebView2 host and constrained Rust project-file broker | Fixed local project area, opaque grants, no raw path API |
| Evidence system | Extraction, governance audit, coverage map, status finalizer, mutation harness, external observer, isolation assembly | Most final runtime evidence is not yet collected |

### Known product-facing limitations

- The LAD core supports semantic branches, but the current visual UI mainly
  edits the provided contact and coil. It cannot yet freely draw arbitrary
  contacts, parallel branches, or a complete Start/Stop seal-in circuit.
- Browser-only preview can run the in-memory engineering experience but cannot
  provide approved native save/open backing.
- Native save/open rejects remote, removable, provider-backed, redirected,
  special, or unverifiable targets by design.
- The current supported languages and instructions are Phase 2 subsets, not a
  claim of full feature parity with any commercial engineering suite.
- There is no HMI, process lab, teaching mode, lesson engine, or assessment
  system.
- There is no vendor project import/export and no real-PLC deployment artifact.
- There is no installer, updater, signing, hosted service, or support lifecycle.

## Architecture

The core architectural rule is that there must be one authoritative domain
path. The UI asks the domain to do work, receives typed results, and renders
projections. It must not reproduce PLC semantics in React.

```text
Human operator
    |
    v
React/Vite workbench
    |
    | typed schema-v2 commands and DomainResult receipts
    v
Dedicated Web Worker
    |
    v
Embedded zero-import plc-engineering-wasm
    |
    v
Rust EngineeringSession / plc-system
    |
    +--> canonical project and persistence
    +--> fictional hardware and program graph
    +--> LAD / FBD / SCL frontends
    +--> one compiler and one typed IR
    +--> deterministic runtime and commissioning
    +--> observability, snapshots, and replay

Native save/open is a separate narrow boundary:

React renderer
    |
    | govsProjectFileBrokerV1 typed request
    v
Windows WebView2 C++ host
    |
    | private framed process protocol
    v
Rust windows-project-broker
    |
    v
approved local %LocalAppData%\GovsPLC\Projects storage only
```

### Why the worker and WASM boundary matters

- It keeps authoritative PLC behavior outside the React component tree.
- It provides one serialized command/result boundary that can be fuzzed and
  mutation-tested.
- It prevents the browser from acquiring host capabilities through the domain
  core.
- It lets the final artifact embed the engineering core without a server.
- It makes “same input, same canonical output” a testable contract.

### Workspace components

The Cargo workspace has 15 packages:

| Path | Responsibility |
|---|---|
| [`crates/foundation-wasm`](crates/foundation-wasm/) | Minimal Phase 1 health kernel; accepted baseline artifact is 247 bytes with zero imports |
| [`crates/plc-types`](crates/plc-types/) | Canonical scalar and aggregate PLC values |
| [`crates/plc-core`](crates/plc-core/) | Project identity graph, command engine, journal, transactions, JSON, package, migration, recovery |
| [`crates/plc-hardware`](crates/plc-hardware/) | EDU-21 fictional hardware/network/profile, addresses, symbols, process images, diagnostics |
| [`crates/plc-program`](crates/plc-program/) | Block/type/interface/call/dependency/instruction model |
| [`crates/plc-lad`](crates/plc-lad/) | Coordinate-independent LAD semantic graph, edit, validate, lower |
| [`crates/plc-language-tools`](crates/plc-language-tools/) | FBD model/editor/lowering and SCL language services |
| [`crates/plc-compiler`](crates/plc-compiler/) | Compiler pipeline, diagnostics, typed IR, SCL frontend, lowering, composition, source maps |
| [`crates/plc-runtime`](crates/plc-runtime/) | Deterministic CPU, scheduler, memory/process images, instruction execution, lifecycle and fault policy |
| [`crates/plc-commissioning`](crates/plc-commissioning/) | Virtual universe, controller identities, load flow, online sessions |
| [`crates/plc-observability`](crates/plc-observability/) | Monitor, Modify, Force, trace, diagnostics, navigation, target policy |
| [`crates/plc-system`](crates/plc-system/) | Top-level orchestration, projections, build product, replay executor/package, EngineeringSession |
| [`crates/plc-engineering-wasm`](crates/plc-engineering-wasm/) | Capability-limited WASM bridge from typed commands to the Rust system |
| [`crates/windows-project-broker`](crates/windows-project-broker/) | Constrained native project-file access and broker protocol |
| [`crates/phase2-independent-replay-verifier`](crates/phase2-independent-replay-verifier/) | External fixed-workflow replay verification |

The TypeScript/UI workspace contains:

| Path | Responsibility |
|---|---|
| [`apps/foundation-shell`](apps/foundation-shell/) | Full React/Vite engineering workbench despite the historical “foundation” name |
| [`apps/windows-shell`](apps/windows-shell/) | Native Windows WebView2 host and typed bridge |
| [`packages/foundation-contract`](packages/foundation-contract/) | Minimal Phase 1 health command/result contract |
| [`packages/plc-contract`](packages/plc-contract/) | Strict PLC command/model/validation contract |

Supporting policy, requirements, evidence, tests, and build tooling live under
`ADR/`, `requirements/`, `evidence/`, `tests/`, and `tools/`.

### Canonical design decisions

- [ADR-0001](ADR/0001-no-physical-industrial-communication.md): no physical
  industrial communications.
- [ADR-0002](ADR/0002-original-project-format.md): original simulator project
  format.
- [ADR-0003](ADR/0003-unified-plc-ir.md): unified PLC IR.
- [ADR-0004](ADR/0004-deterministic-virtual-time.md): deterministic virtual
  time.
- [ADR-0005](ADR/0005-phase-2-native-isolation-shell.md): narrow Phase 2 native
  Windows isolation shell and file broker.

ADRs 0002 through 0004 still carry older “Proposed” wording even though the
corresponding architecture is implemented and required by the Phase 2
directive. Reconcile their status deliberately; do not infer that the code
should be removed.

## Work completed so far

### Phase 1: governance and trusted foundation

Phase 1 established the rules and proof machinery needed before PLC product
implementation could begin. The accepted baseline is commit
`84be813b0c7ccae6d4a18ff060d7106cc168366c`, tagged
`phase1-closure-candidate-v1`.

Completed Phase 1 work includes:

- Canonical source preservation and hashing.
- Clean-room and dependency policies.
- Permanent safety invariants and threat model.
- Deterministic directive extraction and reconciled requirement records.
- Trusted Git-object manifest and exact-source verification.
- Adversarial audit and remediation.
- Twelve-case mutation gate.
- Zero open Critical/High defects at closure.
- A minimal React → worker → typed contract → Rust/WASM vertical slice.
- A dependency-free 247-byte zero-import foundation WASM artifact.
- Closure evidence preserved in Git notes under
  `refs/notes/phase1-closure-evidence`.

The Phase 2 start order explicitly accepts this baseline for internal
development. That does not mean public release was approved.

### Phase 2: implementation chronology

Phase 2 began at commit `d19b931` and tag `phase2-start-v1`. The repository then
grew from a minimal health slice into the current engineering system.

Important milestones:

| Commit | Milestone |
|---|---|
| `da85afc` | Deterministic extraction baseline: 937 Phase 2 requirements and 44 proof obligations |
| `8acfbcf` | Strict PLC TypeScript domain contract |
| `2e04cd1` | Canonical project/workbench kernel and Rust/WASM engineering bridge |
| `e06de68` | Deterministic runtime, commissioning, hardware, and program core |
| `94897e3` | Compiler pipeline and typed IR |
| `db8dfb9` | FBD and SCL language tooling |
| `daf5e4b` | Semantic LAD frontend |
| `3bf5751` | Canonical project integrated with the virtual PLC system |
| `2fc7fd3` | Production runtime connected through the system boundary |
| `bab28ed` | Runnable core exercised through the production workbench |
| `59c2495` | Phase 2 fail-closed evidence gate |
| `2e17982` | Evidence finalizer |
| `5724fe5` | Journey H anti-theater mutation harness |
| `21f3057` | Reviewed requirement-to-implementation mapping |
| `d9d4590` | Independent replay verifier |
| `e9cb417` | Windows WebView2 shell and typed project-file broker |
| `7d18344` | Hardened external Windows observer |
| `58bd6f8` | Latest pre-README checkpoint; native evidence file-sharing retry |

Between those milestones, implementation added:

- Canonical SCL, LAD, and FBD authoring.
- Mixed-language calls and dependency invalidation.
- Persistence, migration, recovery, and identity preservation.
- Scalar, aggregate, and named type handling.
- Build modes, diagnostics, source maps, and navigation.
- Virtual hardware states and hardware-fault matrices.
- CPU state and runtime fault-policy matrices.
- Monitoring, watch/Modify/Force, traces, and diagnostic layers.
- Snapshots, restore, deterministic replay, and external replay verification.
- Load preview, atomic commit, online/offline state, and commissioning policy.
- A runnable, inline, offline React workbench.
- A constrained native Windows file shell.
- Exact-candidate evidence tools, native observer, topology protocol, and
  isolation-closure assembly.

At the pre-README snapshot, the branch contained 113 total commits. Phase 2
accounted for 107 commits after the accepted Phase 1 baseline, touching 331
files with roughly 341,000 added lines. These figures describe implementation
effort, not verification credit.

## Implementation truth, verification truth, and acceptance truth

This distinction is the single most important handoff detail.

### 1. Implementation truth

The code and tests show that a capability has been built. Static coverage can
map requirements to code and tests. That is useful engineering evidence, but it
does not prove the candidate executed the required journey under the required
observer and environment.

### 2. Verification truth

Only accepted, content-addressed, exact-candidate evidence can move a terminal
status to `VERIFIED` or a journey/gate to `PASS`. Generated or ignored local
files, a passing unit test, or a screenshot alone do not qualify.

### 3. Candidate truth

Evidence must bind to one clean, committed candidate, exact inputs, toolchain,
environment, and output hashes. Mixing results from multiple commits is not a
candidate.

### 4. Acceptance truth

Even a completely passing gate may only declare:

> `PHASE 2 IMPLEMENTATION CANDIDATE - AWAITING SCOTT ACCEPTANCE`

Scott's acceptance is a separate required event. Codex may not self-accept the
phase or use a gate result as permission to start Phase 3.

### Current formal Phase 2 state

The tracked terminal status ledger is deliberately conservative:

- 937 of 937 requirements: `NOT_STARTED`.
- 44 of 44 Appendix H proofs: `NOT_STARTED`.
- Journeys A through H: `NOT_STARTED`.
- Gates G2-01 through G2-15: `NOT_STARTED`.
- Evidence records: 0.
- Candidate binding: commit `null`, tag `null`.
- Verdict: `BLOCKED`.

The extracted source-requirement record separately marks P2-00 as
`IMPLEMENTED_UNVERIFIED` and the other 936 requirements as `NOT_STARTED`.
Neither record should be interpreted as saying the codebase contains no Phase 2
implementation. They are fail-closed verification ledgers.

The mandatory integrated journeys are:

- **A:** complete author/build/load/run/observe/tools/save journey.
- **B:** mixed-language project behavior.
- **C:** honest handling of invalid engineering.
- **D:** identity and persistence.
- **E:** determinism.
- **F:** commissioning state separation.
- **G:** isolation counterfactual.
- **H:** anti-theater mutations.

The terminal gate also evaluates G2-01 through G2-15. See the Phase 2 directive
and [tools/phase2/README.md](tools/phase2/README.md) for exact evidence rules.

## Prerequisites and toolchain

### Required versions

| Tool | Required version or condition |
|---|---|
| Node.js | Exactly `24.19.0` |
| pnpm | Exactly `11.19.0` |
| Rust/Cargo | `1.94.0` from [`rust-toolchain.toml`](rust-toolchain.toml) |
| Rust components | `clippy`, `rustfmt` |
| Rust target | `wasm32-unknown-unknown` |
| Phase 1 Python | `3.13.12` through the Phase 1 wrapper |
| Phase 2 Python | `3.12.13` through the Phase 2 wrapper |
| Native Windows build | Windows x64, Visual Studio VC x64 tools, `vswhere.exe`, WebView2 Runtime |

The WebView2 SDK and static loader are vendored at
`vendor/microsoft-webview2/1.0.4129.50/`. The WebView2 **Runtime** is not
vendored and must be installed on the test machine.

### Current-host Node trap

On the machine used for this handoff, ordinary `node` resolves to `24.14.0` and
will fail `pnpm check:toolchain`. The admitted Node `24.19.0` is available at:

```text
C:\Users\Scott\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin
```

For the current PowerShell session:

```powershell
$env:Path = 'C:\Users\Scott\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin;' + $env:Path
node --version
pnpm check:toolchain
```

Do not replace `$HOME`, `$env:USERPROFILE`, or another system variable while
setting this up.

### Install dependencies

The frozen dependency graph must be installed without lifecycle scripts:

```powershell
pnpm install --frozen-lockfile --ignore-scripts
```

If an admitted local pnpm store is already populated, an offline restore may be
used with `--offline` and the intended `--store-dir`.

At this handoff, local `node_modules` is incomplete: the TypeScript test runner
encountered a missing `tinyrainbow/index.js`. Restore dependencies before
treating a JavaScript test failure as a product defect.

Direct `python` is not expected to work on this host. Use the wrappers:

```powershell
node tools/phase2/run_pinned_python.mjs --version
node tools/phase1/run_pinned_python.mjs --version
```

The Phase 2 wrapper currently resolves Python `3.12.13`. The Phase 1 wrapper's
required Python `3.13.12` was not available at this snapshot, so the full Phase
1 closure gate was not rerun during the README audit.

No `.env` file, application secret, Docker environment, Makefile, or Taskfile is
required. There is no configured deployment environment.

## Build and launch

### Build the full offline workbench

```powershell
pnpm build:foundation
```

The name is historical. This command now builds the **entire Phase 2
workbench**, not merely the Phase 1 foundation. It:

1. Builds and embeds foundation WASM.
2. Builds and embeds engineering WASM.
3. Builds the React/Vite application.
4. Inlines the result into one ignored `dist/index.html` artifact.
5. Applies a restrictive content-security policy including
   `connect-src 'none'`.

The engineering WASM size and hash may change with source changes. Do not copy a
diagnostic build's hash into final evidence; regenerate and bind it to the exact
candidate. The required property is a zero-import capability boundary verified
by the build/isolation tooling.

### Launch the standalone browser artifact

```powershell
pnpm launch:foundation
```

On Windows this asks Explorer to open `dist/index.html`. It does not start a web
server. At the handoff snapshot, `dist/index.html` was absent and needed to be
built.

The `http://127.0.0.1:43180/` URL mentioned in the preview guide is an
operator-created ephemeral preview convention. No repository script owns port
43180, and that port was not listening at the handoff. Do not waste time
searching for a missing `start` script for it.

### Build the native Windows evidence shell

```powershell
pnpm build:phase2:native
```

This builds the WebView2 shell, Rust file broker, native launcher, and associated
observer/evidence pieces under ignored `.phase2-verification/` state. It is an
internal verification artifact, not an installer.

The exact procedure is in
[tools/phase2/NATIVE_E2E_WORKFLOW.md](tools/phase2/NATIVE_E2E_WORKFLOW.md).

## How to use the current simulator

The exact labels can evolve, so use
[PHASE_2_PREVIEW_USER_GUIDE.md](PHASE_2_PREVIEW_USER_GUIDE.md) alongside this
overview.

### 1. Create or open a project

- Launch the built workbench.
- Create a simulator-native project, or open a `.vlabproj` through the native
  shell.
- Expect a project tree, central editor/workspace, properties, diagnostics, and
  runtime tools rather than a single demo screen.

### 2. Configure the fictional controller

- Add a virtual network.
- Add an EDU-21 virtual controller.
- Add its rack and virtual input/output modules.
- Create symbolic tags and bind them to valid virtual addresses.
- Treat all identities and addresses as simulator-domain values.

### 3. Define program and data

- Add OB, FC, FB, and DB objects as the exercise requires.
- Define interfaces, function-block instances, named types, and dependencies.
- Create LAD, FBD, or SCL source through the available editors.
- Mixed-language calls are supported through the common semantic model.

### 4. Build

- Start a build from the workbench.
- Review the structured build report, diagnostics, invalidation behavior, and
  source navigation.
- A failed or stale build must not become a runnable load product.

### 5. Commission the virtual controller

- Power on the virtual controller.
- Preview the load before committing it.
- Commit the load atomically.
- Go online with the internal simulator.
- Move the virtual CPU into RUN only when its state and load permit it.

Preview and commit are intentionally separate. Offline project state, built
artifact state, loaded controller state, online-session state, and executing
runtime state must not collapse into one boolean.

### 6. Exercise runtime behavior

- Set a virtual raw input.
- Execute one or more deterministic scans.
- Observe the resulting virtual output and process-image state.
- Repeating the same canonical inputs from the same state should produce the
  same canonical result.

### 7. Observe and diagnose

- Monitor program and runtime values.
- Use watch targets.
- Use Modify for permitted transient changes.
- Use Force only through the explicit force policy and lifecycle.
- Configure traces and triggers.
- Inspect CPU, program, hardware, online, and runtime diagnostics.
- Navigate from diagnostic/runtime context back to canonical source.
- Capture/restore snapshots and verify replay where appropriate.

### 8. Save or reopen

- Native Save/Open uses `.vlabproj` through the approved file broker.
- The renderer receives opaque grants and typed receipts, not arbitrary paths.
- Rejection of a remote, redirected, removable, provider-backed, special, or
  unverifiable target is correct behavior.
- Browser-only mode cannot claim approved durable persistence.

## Development and test commands

Run commands from the repository root in PowerShell after admitting the exact
toolchain.

### Routine development checks

```powershell
pnpm check:toolchain
pnpm requirements:phase2:check
pnpm audit:phase2:governance
pnpm lint
pnpm typecheck
pnpm test:unit
pnpm build:foundation
pnpm verify:isolation
pnpm test:e2e:phase2
```

What they mean:

- `check:toolchain` rejects version drift.
- `requirements:phase2:check` proves deterministic extraction still matches the
  canonical directive and checked-in records.
- `audit:phase2:governance` checks structural governance consistency.
- `lint` embeds both WASM modules, runs source policy, formatting, and Clippy.
- `typecheck` checks the contracts and workbench.
- `test:unit` builds both WASM modules, runs the whole Rust workspace, then the
  TypeScript package/workbench tests.
- `build:foundation` creates the complete inline workbench.
- `verify:isolation` inspects the standalone artifact's isolation properties.
- `test:e2e:phase2` exercises the browser workbench.

### Focused Phase 2 checks

```powershell
pnpm coverage:phase2:check
pnpm test:phase2:gate
pnpm lint:phase2:source
pnpm test:unit:phase2:native
pnpm test:unit:phase2:external-evidence
pnpm verify:phase2:mutations
```

### Phase 1 historical closure gate

```powershell
pnpm gate:closure
```

This still matters because the Phase 2 candidate inherits the Phase 1 trusted
baseline. It requires the exact Phase 1 Python runtime.

### Phase 2 terminal truth gate

```powershell
pnpm gate:phase2
```

This is **not** a substitute for the complete development/build/E2E/evidence
workflow. It validates extraction, governance, coverage freshness, gate-unit
tests, source policy, and the finalized evidence ledger. It is expected to fail
closed until the exact-candidate evidence is complete.

## Exact-candidate evidence workflow

Read [tools/phase2/README.md](tools/phase2/README.md) before running any finalizer.
The core collector uses a clean committed Git candidate as its identity.

```powershell
node tools/phase2/run_pinned_python.mjs -B `
  tools/phase2/collect_phase2_evidence.py `
  --root . --candidate-ref HEAD --mode core
```

The collector's core sequence is:

1. `pnpm check:toolchain`
2. `pnpm requirements:phase2:check`
3. `pnpm audit:phase2:governance`
4. `pnpm coverage:phase2:check`
5. `pnpm test:phase2:gate`
6. `pnpm lint`
7. `pnpm typecheck`
8. `pnpm test:unit`
9. `pnpm build:foundation`
10. `pnpm verify:isolation`
11. `pnpm test:e2e:phase2`
12. `pnpm gate:closure`

It writes generated evidence under ignored `.phase2-verification/`. A dirty
tree, an uncommitted candidate, missing sidecar, mismatched hash, or mixed
candidate history is supposed to block finalization.

### Native interactive run

After a successful native build:

1. Open `.phase2-verification\native-build\Run-Native-E2E.exe` manually through
   Explorer under the real interactive Windows user.
2. Allow the workflow to produce its native journey, broker, identity,
   diagnostic, and observer outputs.
3. Finalize them only after the process completes:

```powershell
pnpm finalize:phase2:native
```

The interactive-user identity requirement is intentional. Do not replace it
with a background/session-zero shortcut just to make the evidence pass.

### Strict isolation counterfactual still required

The final isolation proof needs all of the following for one exact candidate:

- A finalized native adapters-on run.
- Two genuine, operator-controlled live-LAN scenarios with different stable
  topology fingerprints and identical controlled input/output hashes.
- A packaged adapters-off run with real Windows pre/post adapter snapshots.
- Boundary-fuzz and export-rejection records with content-addressed sidecars.
- Complete external observer, network, process, and raw execution logs.
- Assembly and verification of the exact-candidate isolation closure.

The procedures are:

- [LIVE_LAN_TOPOLOGY_PROTOCOL.md](tools/phase2/LIVE_LAN_TOPOLOGY_PROTOCOL.md)
- [ISOLATION_COUNTERFACTUAL.md](tools/phase2/ISOLATION_COUNTERFACTUAL.md)
- [NATIVE_E2E_WORKFLOW.md](tools/phase2/NATIVE_E2E_WORKFLOW.md)

At this handoff, no current finalized native build/evidence manifest and no
strict isolation closure existed locally.

## Verification snapshot from this README audit

These results describe the dirty local worktree on 2026-08-29. They are useful
diagnostics, not candidate evidence.

| Check | Result |
|---|---|
| `pnpm requirements:phase2:check` | Pass: 937 requirements and 44 proofs |
| `pnpm audit:phase2:governance` | Pass: zero findings |
| `pnpm lint:phase2:source` | Pass: all 15 workspace crates scanned |
| `pnpm test:unit:phase2:external-evidence` | Pass: 6 of 6 |
| `pnpm coverage:phase2:check` | Fail: checked-in generated coverage outputs are stale |
| `pnpm test:phase2:gate` | 64 of 65 pass; the only failure is stale coverage output |
| `pnpm check:toolchain` on ordinary `PATH` | Fail: host Node is 24.14.0 |
| `pnpm check:toolchain` after admitting bundled Node | Pass |
| Rust portion of `pnpm test:unit` | Pass across the workspace |
| TypeScript portion of `pnpm test:unit` | Did not start because local `node_modules` is incomplete (`tinyrainbow/index.js` missing) |
| Full Phase 1 closure | Not rerun; Python 3.13.12 unavailable locally |
| Browser Phase 2 E2E | Not rerun |
| Native Phase 2 E2E and strict isolation | Not rerun; no current finalized evidence |

The last checked-in coverage audit reports 36 ready, 8 partial, and 0 missing.
A current local regeneration computed 41 ready, 3 partial, and 0 missing with 6
remaining gaps, but those generated files are ignored and grant zero
verification credit. The generator changed after the last committed report.

Some remaining generated ISO-0003/ISO-0004 gap wording still says the native
host is unimplemented even though the host now exists. Reconcile that wording
with the actual implementation before regenerating and committing the coverage
report. Runtime isolation evidence remains genuinely incomplete regardless.

## Current working-tree state

At the pre-README handoff snapshot, these user-owned changes already existed:

- Modified `tests/phase2/external-isolation-evidence.unit.mjs`.
- Modified `tools/phase2/LIVE_LAN_TOPOLOGY_PROTOCOL.md`.
- Modified `tools/phase2/README.md`.
- Untracked `References for Codex from Scott/PHASE_1_ADVERSARIAL_AUDIT.docx`.

Preserve them. Do not revert, overwrite, stage, rename, or delete them merely to
obtain a clean tree.

The untracked audit DOCX is not random debris. It is 187,063 bytes and is
byte-identical to the tracked
[`docs/governance/PHASE_1_ADVERSARIAL_AUDIT.docx`](docs/governance/PHASE_1_ADVERSARIAL_AUDIT.docx),
with SHA-256
`2075F9BF6D082E1F82898693823475B9C49BD284DB013A8AF34A529D9CD4E8AC`.
The P2-00 entry record accounts for it. Do not casually track the duplicate or
remove Scott's reference copy.

Common ignored/generated local state includes:

- `.phase1-verification/`
- `.phase2-verification/`
- `.pnpm-store/`
- `node_modules/`
- Cargo `target/` directories
- `dist/`
- generated embedded-WASM TypeScript
- `Phase 2 Review Package/`

Ignored does not mean worthless, but it also does not mean authoritative. Check
the candidate binding and generation time before relying on anything there.

## Known repository drift and traps

These are the things a new instance is most likely to misread.

### Stale or historical documents

- The previous root README was a Phase 1 description. This file replaces it.
- Root `package.json` still advertises `0.0.0-phase1` and a Phase 1-only
  description.
- `build:foundation` builds the full Phase 2 workbench despite its old name.
- [IMPLEMENTATION_MATRIX.json](IMPLEMENTATION_MATRIX.json) is a Phase 1-era
  historical matrix and is not the Phase 2 capability map.
- [RISK_REGISTER.md](RISK_REGISTER.md) still contains statements that the
  compiler/runtime are not started and that Phase 2 evidence does not exist.
  The code has overtaken those statements; the Phase 2 directive's current risk
  appendix also has entries absent from the register.
- [OPEN_DECISIONS.md](OPEN_DECISIONS.md) still marks OQ-0003 through OQ-0006 as
  deferred, although the Phase 2 directive resolves the current EDU-21 scope.
  The directive wins. OQ-0011 is also not represented there.
- ADRs 0002 through 0004 have stale “Proposed” statuses despite implemented,
  directive-backed decisions.
- The tracked coverage report is stale relative to its generator.
- An ignored `Phase 2 Review Package/` is older than the current branch. It is
  neither source truth nor current evidence.
- `.phase2-verification/current-gate-report.json`, if present, may be bound to
  old commit `59c2495`; inspect its candidate hash before reading it as current.

### Requirement-ID integrity issue

[`requirements/phase2-extraction-audit.json`](requirements/phase2-extraction-audit.json)
reports 22 Phase 1 IDs reused with different text and one reused retired ID.
That conflicts with permanent-ID expectations and needs deliberate owner review.
Do not silently renumber, normalize, or delete records to make the audit green.

### Governance and release gaps

- [CONTRIBUTOR_CLEAN_ROOM_ATTESTATION.md](CONTRIBUTOR_CLEAN_ROOM_ATTESTATION.md)
  remains blank.
- [LEGAL_REVIEW_CHECKLIST.md](LEGAL_REVIEW_CHECKLIST.md) remains
  `NOT_REVIEWED`/`BLOCKED`.
- WebView2 use is authorized for this narrow Phase 2 shell but remains
  release-unreviewed.
- The only tracked GitHub workflow is
  [`.github/workflows/phase1-governance.yml`](.github/workflows/phase1-governance.yml).
  It runs the Phase 1 closure gate, not the Phase 2 terminal evidence workflow.
- There is no configured Git remote, hosted run, artifact upload, installer,
  signing pipeline, updater, or production deployment.
- There is no general project release license in this repository. Vendored
  third-party material has its own license/provenance records; that is not
  public-release authorization for this project.

### Evidence traps

- Passing unit tests does not populate the terminal status ledger.
- Static “ready” coverage does not mean `VERIFIED`.
- A screenshot is not a complete runtime proof.
- Evidence from a dirty tree is diagnostic only.
- Evidence from two commits cannot be assembled into one exact candidate.
- An ignored report is not automatically stale, but its embedded candidate hash
  and sidecars must be verified.
- A locally generated report must not overwrite tracked truth until reviewed.
- `gate:phase2` is a truth validator, not a magic evidence generator.
- The live-LAN comparison is about proving output independence from external
  topology. It does not authorize the product to communicate with the LAN.

## Recommended continuation order

Unless Scott gives a newer instruction, the next instance should proceed in
this order:

1. Read the Phase 2 directive and this README completely.
2. Run `git status --short` and preserve every pre-existing user change.
3. Admit exact Node `24.19.0`; confirm pnpm and Rust versions.
4. Restore the frozen dependency graph so TypeScript tests can run.
5. Review and finish the in-progress external-isolation sidecar/protocol/tooling
   changes without discarding them.
6. Reconcile coverage-generator gap wording with the implemented native shell.
7. Review the Phase 2 reused-ID extraction issue with Scott before changing any
   permanent identifier.
8. Regenerate and review the Phase 2 coverage audit.
9. Bring routine lint, typecheck, unit, build, isolation, and browser E2E checks
   to green.
10. Create a clean, committed exact-candidate checkpoint only after preserving
    and intentionally incorporating the right changes.
11. Run the exact-candidate core evidence collector.
12. Build the native evidence shell.
13. Manually execute the native launcher under the interactive Windows user.
14. Finalize native evidence.
15. Collect genuine live-LAN A and B topology observations and the adapters-off
    packaged run.
16. Finalize boundary-fuzz/export-rejection sidecars and external observer
    records.
17. Assemble the strict isolation counterfactual closure.
18. Assemble reusable core and isolation records into one candidate evidence
    set.
19. Finalize the Phase 2 status ledger and run the terminal gate.
20. Stop at “Phase 2 implementation candidate—awaiting Scott acceptance.”

Do not broaden this list into Phase 3 work merely because a Phase 2 test is
blocked. Solve the smallest Phase 2 problem or ask Scott for a decision.

## Definition of done for the current mission

Phase 2 is not done when the UI “looks finished.” It is done only when all of
the following are true for one clean exact candidate:

- Every Phase 2 requirement has an admissible terminal status and all required
  requirements are `VERIFIED`.
- Every Appendix H proof obligation is verified.
- Journeys A through H pass with the mandated evidence.
- G2-01 through G2-15 pass.
- The project builds and runs through the production workbench rather than a
  test-only duplicate.
- LAD, FBD, and SCL demonstrate shared compiler/IR/runtime semantics.
- Persistence, identity, determinism, commissioning state separation, and
  isolation counterfactuals are proven.
- Journey H mutations prove that the gates fail when the implementation or
  evidence is theater.
- No open Critical or High defect remains.
- All evidence, sidecars, logs, toolchain facts, and outputs bind to the exact
  candidate.
- The only automated verdict is
  `PHASE 2 IMPLEMENTATION CANDIDATE - AWAITING SCOTT ACCEPTANCE`.
- Scott reviews and accepts the candidate before any next-phase start order.

## Handoff checklist for another Codex instance

Before claiming progress, be able to answer these questions:

- Which source in the authority ladder authorizes this change?
- Is the change Phase 2, or is it actually reserved Phase 3/4 work?
- Does it preserve the permanent virtual/physical separation structurally?
- Is the behavior in the authoritative Rust/domain path or duplicated in UI
  state?
- Does it preserve one typed IR and one deterministic runtime?
- Does it preserve canonical IDs, serialization, migration, and replay?
- Are you reporting implementation evidence, verification credit, candidate
  status, or Scott acceptance—and using the right word?
- Are all current user changes still present?
- Is the exact toolchain admitted?
- Is any evidence tied to the exact current commit rather than an older local
  artifact?
- Have you avoided inventing citations for unresolved research tokens?
- Will you stop at the Phase 2 acceptance boundary?

If a future engineer says, “I wish someone had told me this at the beginning,”
the intended answer is: **the repository is much further along than its old
metadata suggests, much less formally verified than its code volume suggests,
and far more constrained by safety, source authority, and exact-candidate
evidence than an ordinary application repository.**
