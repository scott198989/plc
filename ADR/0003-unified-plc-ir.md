# Unified Typed PLC Intermediate Representation

- Status: Proposed Phase 1 Architecture Invariant
- Decision ID: ADR-0003
- Date: 2026-08-27
- Approval: Not yet recorded; directive-mandated boundary documented for review before implementation depends on it
- Scope: LAD, FBD, SCL, validation, compilation, build artifacts, runtime, diagnostics, monitoring, trace, Learning Lens, and assessment
- Supersedes: None

## Context

The simulator must teach genuine engineering consequences across LAD, FBD, and SCL without implementing three divergent compilers or runtimes. Executing editor geometry, evaluating source text, using regex-only parsing, or scripting canned outcomes would make semantics language-specific, nondeterministic, unsafe, and difficult to verify.

A shared typed IR is the boundary between independently implemented language frontends and one deterministic virtual controller runtime. It also provides one place for type semantics, source mapping, monitoring instrumentation, diagnostics, build fingerprints, and replay identity.

This decision implements `PES-TYP-0001`, `PES-TYP-0002`, `PES-ARC-0017`-`PES-ARC-0021`, `PES-IR-0001`-`PES-IR-0005`, `PES-DIA-0001`-`PES-DIA-0006`, `PES-DET-0001`-`PES-DET-0007`, `PES-PROF-0002`-`PES-PROF-0005`, `PES-DEV-0004`, `PES-DEV-0005`, and `PES-DOC-0003`.

## Decision

LAD, FBD, and SCL compile through their own semantic frontends into **one versioned, typed, serializable PLC IR**, and **one virtual controller runtime** executes that IR.

No language frontend has a private runtime or alternate instruction semantics. Shared instruction definitions, conversions, type checking, call signatures, arithmetic, comparisons, timers, counters, storage, error behavior, monitoring probes, and source mappings are centralized in the common compiler/runtime path.

## Frontend contracts

| Frontend | Required semantic model | Layout/source boundary |
|---|---|---|
| LAD | Semantic graph/AST with stable node and connection identity | Screen coordinates, wire routing, zoom, and placement are editor layout only and never executable semantics (`PES-ARC-0017`) |
| FBD | Typed port graph with stable node, port, and edge identity plus explicit execution dependencies | Visual placement and routing do not define dataflow or execution order (`PES-ARC-0018`) |
| SCL | Independently implemented lexer, parser, AST, source ranges, scope resolver, control-flow model, and original language-service metadata | Source is parsed and resolved; it is never passed to `eval`, a Function constructor, or a regex-only compiler (`PES-ARC-0019`, `PES-ARC-0021`) |

Invalid semantic structures remain editable where appropriate but cannot lower to runnable IR (`PES-FID-0005`, `PES-IR-0004`).

## Canonical type system

One recursive type system is authoritative for tags, data blocks, block interfaces, language frontends, addresses, runtime memory, watch/modify/force, trace, HMI bindings, assessment expressions, and TrainingProfiles (`PES-TYP-0001`).

Named-type identity is distinct from structural shape, and type members have stable identity (`PES-TYP-0002`). Frontends cannot create private coercion, call, timer, counter, storage, or error rules that disagree with the canonical system.

Exact identifier grammar, scope/shadowing, address allocation, profile-specific built-ins, controller-family edge behavior, and complete instruction legality are not decided here. OQ-0004 and OQ-0006 remain BLOCKED for Phase 2; unsupported details cannot be guessed (`PES-DEC-0003`).

## IR contract

The IR is:

- **typed:** every operation and storage access has resolved canonical types;
- **versioned:** the artifact records the exact IR version and compatibility is explicit;
- **serializable:** the representation can be fingerprinted, persisted as an internal build artifact, inspected, and replayed without editor state;
- **platform-neutral:** semantics do not depend on UI framework, operating system, wall clock, or host locale;
- **deterministic:** operation order and observable behavior are defined by semantic dependencies, the scheduler, and virtual time;
- **source-mapped:** operations retain stable links to project object IDs and language source/graph identities;
- **instrumentable:** probes are keyed to semantic node/source identity and observe rather than alter execution;
- **capability-limited:** IR contains no host endpoint, URL, filesystem path used as a capability, shell/native call, arbitrary code, transport descriptor, or physical target.

The concrete opcode set, binary/wire layout, optimization passes, and compatibility window are Phase 2 specification work. This ADR fixes their invariants, not their exact representation.

## Immutable build artifact

A successful build produces an immutable, fingerprinted simulator-owned artifact containing or identifying at least:

- project snapshot hash;
- compiler version;
- IR version;
- pinned TrainingProfile ID and version;
- dependency closure;
- immutable build diagnostics;
- source map.

A blocking error produces no runnable artifact (`PES-IR-0003`, `PES-IR-0004`). The artifact is distinct from editable source, saved project state, hardware/software/HMI build states, the artifact loaded into a virtual controller, live values, start/retained values, modifications, forces, snapshots, and online/offline match state (`PES-ARC-0022`).

Virtual Download commits an approved immutable artifact atomically to a `VirtualControllerId`; the runtime does not compile editor state implicitly (`PES-ARC-0023`, `PES-ARC-0024`).

## Runtime contract

One virtual runtime executes supported IR. It must:

1. use simulator-controlled monotonic virtual time;
2. follow stable scheduling/event ordering;
3. implement shared instructions and error behavior identically regardless of source language;
4. exchange process/HMI/monitoring values through typed internal contracts only;
5. preserve separate runtime value, modify, force, retention, raw-process, CPU-visible, and snapshot layers;
6. emit original lifecycle-bearing diagnostic events from actual runtime/device/process/HMI state;
7. expose instrumentation for monitoring, trace, Learning Lens, diagnostics, and assessment without changing execution;
8. run in an isolated worker using typed messages so simulation cannot freeze the UI;
9. remain free of network, device, native FFI, process, arbitrary filesystem, and shell capabilities.

The trusted implementation is Rust compiled to capability-limited WebAssembly unless Scott approves the narrowly allowed alternative in `PES-DEV-0004`.

## Diagnostics and source navigation

Build diagnostics are immutable results tied to a build ID. Runtime diagnostics are lifecycle-bearing events tied to virtual and engineering timestamps. A unified UI may display both, but the underlying records remain distinct (`PES-DIA-0002`).

Diagnostics use original simulator codes and prose and arise from validators, compiler rules, runtime transitions, hardware/process state, HMI consistency, persistence validation, or fault providers (`PES-DIA-0001`, `PES-DIA-0003`). They retain object identity, source/graph ranges where applicable, related object/tag IDs, navigation targets, lifecycle correlation, and deterministic ordering.

Teacher Mode, lessons, scenarios, demos, and UI code cannot inject a diagnostic, monitored value, alarm, trace, or passing result. They invoke ordinary domain/fault commands and the normal engines derive the outcome (`PES-DIA-0004`, `PES-DIA-0005`).

## Profiles, versions, and compatibility

- The semantic core is version-neutral and profiles are declarative capability manifests (`PES-PROF-0002`).
- Project schema version, compiler version, IR version, runtime/scheduler version, and TrainingProfile version are separate identities.
- A project pins its profile and capability-manifest version; opening/migration does not silently change runtime semantics (`PES-PROF-0003`, `PES-PROF-0004`).
- The runtime loads only an explicitly supported IR/profile/build combination and reports incompatibility structurally rather than guessing or silently upgrading.
- V19/V20-era compatibility profiles remain DEFERRED until specified and verified (`PES-PROF-0005`).

## Decision consequences

### Benefits

- LAD, FBD, and SCL share one type system, instruction contract, runtime, diagnostic behavior, and verification corpus.
- Cross-language calls and mixed-language projects can be reasoned about through stable semantic identities.
- Monitoring, trace, Learning Lens, and assessment observe one execution rather than parallel simulations.
- Build artifacts and replay can be fingerprinted independently of editor layout.
- Causal faults and diagnostics remain ordinary engine behavior rather than lesson scripts.

### Costs

- Frontends must perform real parsing/resolution/type checking and cannot shortcut through UI state or source evaluation.
- IR and runtime versioning require compatibility policy, migrations or rebuild rules, golden tests, and source maps.
- Language-specific features must either lower to shared semantics or remain unsupported; they cannot introduce a private runtime.
- Optimization cannot erase required source identity, deterministic behavior, or instrumentation.

## Alternatives rejected

| Alternative | Reason rejected |
|---|---|
| Separate LAD, FBD, and SCL runtimes | Semantics and diagnostics would diverge; violates `PES-ARC-0020`, `PES-ARC-0021`, and `PES-IR-0001`. |
| Execute LAD/FBD from screen geometry | Layout changes would alter behavior and invalid graphs could appear executable. |
| Evaluate SCL as JavaScript or another host language | Introduces executable-code and host-capability risk and cannot guarantee PLC semantics. |
| Regex-only SCL compiler | Cannot provide a genuine AST, scope/type/control-flow model, or reliable diagnostics. |
| Compile directly into UI callbacks | Couples semantics to presentation, loses stable source identity, and undermines workers/replay. |
| Allow lessons to replace runtime/compiler output | Produces canned demonstrations rather than causal learning. |
| Put endpoint, transport, or native-call operations in IR | Violates the immutable safety wall and capability-limited core. |

## Verification obligations

Before the unified pipeline can be marked VERIFIED, the verification matrix must include:

- frontend parser/graph validity, resolver, type, call-signature, conversion, and control-flow positive/negative tests;
- cross-language equivalence tests for every shared supported instruction and error case;
- property/golden tests showing editor layout does not change LAD/FBD semantics;
- negative proof that invalid graphs and blocking errors produce no runnable artifact;
- artifact fingerprint tests for snapshot/compiler/IR/profile/dependency/diagnostic/source-map changes;
- artifact immutability and atomic Virtual Download/rollback tests;
- source-map and stable-navigation tests across rename, move, delete/tombstone, undo, and rebuild;
- instrumentation non-interference tests for monitoring, trace, Learning Lens, diagnostics, and assessment;
- deterministic replay equivalence across supported platforms and execution speeds;
- runtime isolation, typed IPC, WASM-import, and zero-egress tests;
- negative tests proving no `eval`, Function constructor, regex-only execution, coordinate execution, private runtime, or direct lesson outcome injection is production-reachable.

Every test maps to the requirements it verifies, and every MUST/MUST NOT has positive or negative verification under `PES-REQ-0006` and `PES-REQ-0007`.

## Follow-up specification gate

Phase 2 must define the canonical type rules, language legality, exact IR schema/opcodes, compatibility policy, build pipeline, runtime behavior, and objective test corpus before implementation depends on them. Vendor-specific or controller-family details identified by `PES-DEC-0003` remain BLOCKED pending verified research and may not be inferred by this ADR.
