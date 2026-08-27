# Deterministic Virtual Time, Scheduling, and Replay

- Status: Proposed Phase 1 Architecture Invariant
- Decision ID: ADR-0004
- Date: 2026-08-27
- Approval: Not yet recorded; directive-mandated boundary documented for review before implementation depends on it
- Scope: PLC scheduler, timers/counters, process simulation, HMI publication, diagnostics, trace, scenarios, Learning Lens, Teacher Mode, and assessment
- Supersedes: None

## Context

An educational simulator must reproduce engineering consequences reliably. Wall-clock timers, UI event-loop timing, unseeded randomness, worker races, host scheduling, and undefined tie-breaking would make diagnostics, process behavior, traces, HMI updates, and grades vary between runs or machines.

The simulator therefore needs an authoritative virtual clock, explicit scan boundaries, stable event ordering, and a replay identity that binds every semantic input. Human-facing timestamps can use wall-clock metadata, but they cannot control PLC/process execution.

This decision implements `PES-DET-0001`-`PES-DET-0007`, `PES-ARC-0002`, `PES-ARC-0015`, `PES-IR-0003`, `PES-IR-0005`, `PES-DIA-0002`, `PES-DIA-0006`, `PES-DEV-0005`, `PES-DOC-0003`, `PES-EDU-0003`, and `PES-FID-0003`-`PES-FID-0007`.

## Decision

All authoritative simulation behavior uses **simulator-controlled monotonic virtual time**. The scheduler owns advancement of that time. Wall-clock time, browser timers, frame timing, worker scheduling, CPU speed, and UI load cannot determine PLC or process semantics.

Virtual time governs:

- PLC scheduling and scan execution;
- timers and counters with temporal behavior;
- deterministic process physics;
- trace sampling and publication;
- runtime diagnostic lifecycle timing;
- HMI value publication ordering;
- scenario and lesson triggers;
- Learning Lens pause/step/slow behavior;
- assessment timing and results.

`setTimeout`, animation frames, host clocks, and equivalent facilities may request UI work or an opportunity to advance simulation, but they cannot be authoritative PLC/process time (`PES-DET-0002`).

## Time domains

| Time domain | Purpose | Authority |
|---|---|---|
| Virtual timestamp | PLC/runtime/process/scenario/trace/HMI/assessment semantics | Authoritative, simulator-controlled, monotonic |
| Engineering timestamp | Human-facing audit/display metadata | Non-authoritative wall-clock metadata; cannot order or alter simulation |
| UI/render timing | Rendering, input collection, animation smoothness | Non-authoritative; may not affect semantic results |

Records that expose both time domains must label them explicitly. An engineering timestamp cannot be substituted for a missing virtual timestamp (`PES-DET-0005`).

## Event ordering

Events with the same virtual timestamp and semantic priority have a stable deterministic order (`PES-DET-0003`). At minimum, the scheduler records an explicit monotonic event sequence so replay does not depend on collection iteration, thread race, insertion timing from the host event loop, or operating-system scheduling.

The concrete priority/preemption matrix and controller-family OB behavior are not defined in Phase 1. They require a verified TrainingProfile specification and are subject to `PES-DEC-0003`. Until then, the scheduler may implement only profile-independent behavior that is explicitly specified and tested; it may not imitate undocumented vendor priorities.

## Reserved scan boundaries

The scheduler contract reserves these semantic boundaries in order (`PES-DET-0007`):

1. scan start;
2. input sample;
3. program execution;
4. output commit;
5. process update;
6. trace, diagnostic, and HMI publication;
7. scan end.

Phase 2 must define the exact data visibility and fault/publication rules at each boundary. Implementations cannot collapse or reorder them in a way that changes observable semantics without an approved requirement/change record.

## Replay identity

A deterministic replay identity includes at least (`PES-DET-0004`):

- deterministic seed;
- ordered event sequence;
- TrainingProfile hash;
- immutable build hash;
- initial runtime/process snapshot hash;
- simulator version;
- scheduler version.

Project schema, compiler, IR, process-model, scenario, and assessment versions that can affect behavior must be transitively bound by those hashes or recorded explicitly. Missing or mismatched replay inputs fail visibly; the product does not claim equivalent replay after silently substituting a version.

## Deterministic input and randomness rules

- All pseudo-random behavior uses a deterministic generator seeded from replay identity.
- Unseeded host randomness, locale-dependent ordering, unordered-map iteration, environment variables, current wall-clock time, and race-dependent message order cannot affect semantic state.
- User, Teacher Mode, lesson, scenario, fault, and UI actions enter execution as validated domain commands/events with explicit virtual-time placement and deterministic sequence.
- External network/device events cannot exist because VirtualUniverse has no physical adapter.
- Floating-point or numeric behavior that may vary by platform must have an explicitly supported deterministic contract and cross-platform verification before use in grading or claimed replay equivalence.

## Pause, step, speed, and Learning Lens

Pause stops virtual-time advancement; it does not rewrite state. Step advances only the explicitly defined semantic unit. Speed changes how quickly the scheduler is permitted to advance relative to the user, not the amount or order of semantic work performed at each virtual timestamp.

Learning Lens may pause, step, slow, inspect, annotate, and explain the real execution. Instrumentation and explanations must be observational and cannot alter program truth, type/compiler results, device state, timing rules, or assessment outcomes (`PES-EDU-0003`, `PES-IR-0005`).

## Workers and concurrency

Runtime/process work executes in isolated workers with typed messages so simulation cannot freeze the UI (`PES-DEV-0005`). Worker isolation does not authorize semantic concurrency races:

- the scheduler determines semantic order;
- messages crossing workers are tagged, bounded, validated, and ordered explicitly;
- late host delivery cannot retroactively change a committed virtual-time boundary;
- cancellation, pause, reset, fault, and load commands have defined transaction boundaries;
- presentation consumes published snapshots/events and cannot read a half-committed semantic state.

## Observable equivalence contract

For the same supported build, initial snapshot, TrainingProfile, seed, and ordered events, execution must produce equivalent observable (`PES-DET-0006`):

- tag streams and committed outputs;
- virtual process state;
- runtime diagnostic events and lifecycle order;
- trace samples;
- HMI updates;
- monitoring/watch values;
- assessment results.

Engineering timestamps, render frame counts, host thread IDs, and non-semantic performance measurements are not required to be identical and cannot be used to mask a semantic divergence.

## Persistence and state consequences

- Runtime snapshots record all state required for deterministic continuation, including virtual time, scheduler/event sequence, seed/generator state, loaded build/profile identity, retained/runtime/process state, forces, and pending deterministic events as defined later.
- Editable project source, saved state, build artifact, loaded artifact, runtime values, process values, modifications, forces, and snapshots remain separate (`PES-ARC-0022`).
- Virtual Download and reset establish explicit replay boundaries; they cannot silently compile or synchronize state (`PES-ARC-0023`, `PES-ARC-0024`).
- Event/audit records retain deterministic ordering, affected object IDs, before/after hashes, and command provenance (`PES-ARC-0015`).

## Diagnostics and trace consequences

Build diagnostics are immutable and not timed by the runtime. Runtime diagnostic events carry virtual timestamps, engineering timestamps, lifecycle, source object identity, and deterministic ordering (`PES-DIA-0002`, `PES-DIA-0006`). Trace, HMI, Learning Lens, and assessment use shared semantic instrumentation points and publication boundaries; none may schedule a parallel execution or perturb the observed run.

## Decision consequences

### Benefits

- Lessons, troubleshooting, diagnostics, traces, HMI, and grading are reproducible.
- Pause, step, and speed controls can explain authentic execution without changing it.
- Bugs can be reproduced from hashes, seed, event stream, and scheduler identity.
- Runtime behavior is independent of UI load, host CPU speed, frame rate, network presence, and wall-clock drift.

### Costs

- Every stateful process model, timer, trace source, scenario, and assessment rule must use virtual time explicitly.
- Event ordering, snapshots, numeric behavior, and replay compatibility require versioned contracts and golden/property tests.
- Performance optimizations may not introduce race-dependent or host-dependent observable results.
- Unsupported profile-specific scheduling details remain unavailable until researched and verified.

## Alternatives rejected

| Alternative | Reason rejected |
|---|---|
| Browser `setTimeout` or animation frames as PLC time | Host scheduling and UI load would change semantics. |
| Wall-clock timestamps as the event order | Clock adjustments, precision, and platform behavior are nondeterministic. |
| Worker arrival order as semantic order | Thread scheduling races would alter state and replay. |
| Unseeded/random process faults | Scenarios and grading could not be reproduced. |
| Separate lesson or Learning Lens clock/runtime | Explanations would diverge from the real kernel and could alter outcomes. |
| Approximate replay without build/profile/snapshot hashes | Different semantics could be presented as equivalent. |
| Guess vendor/controller priority rules | Violates mandatory research stop `PES-DEC-0003`. |

## Verification obligations

Before deterministic execution can be marked VERIFIED, tests must cover:

- monotonic virtual-time advancement and explicit pause/step/speed semantics;
- negative proof that wall-clock/browser/UI/worker timing cannot change semantic outputs;
- stable ordering for equal virtual timestamps/priorities;
- all reserved scan boundaries and their specified visibility/commit rules;
- timer/counter, process, diagnostic, trace, HMI, scenario, lesson, and assessment use of virtual time;
- replay identity completeness and visible rejection of missing/mismatched inputs;
- repeat-run equivalence of tag/output/process/diagnostic/trace/HMI/assessment streams;
- deterministic seeded faults/randomness and snapshot continuation;
- cross-platform/worker-load/UI-load equivalence within the supported numeric contract;
- event/audit ordering, before/after hashes, command provenance, cancellation, rollback, reset, and Virtual Download boundaries;
- instrumentation non-interference for monitoring, trace, Learning Lens, diagnostics, and assessment;
- isolation proof that no external network/device event source or forbidden WASM import exists.

Golden outputs must be versioned and traceable to requirements. A divergence, skipped test, flaky result, unsupported platform, manual waiver, or inconclusive comparison keeps the milestone open (`PES-CI-0001`, `PES-QLT-0008`).

## Follow-up specification gate

Phase 2 must specify scheduler units, supported event priorities, scan-boundary visibility, timer/counter semantics, snapshot contents, numeric determinism, reset/load behavior, and replay compatibility before implementation depends on those details. Exact controller-family OB numbers, preemption, nesting, recursion, and vendor-specific behavior remain BLOCKED pending verified research under `PES-DEC-0003`.
