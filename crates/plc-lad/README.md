# `plc-lad`

`plc-lad` is the capability-free, coordinate-independent Ladder Diagram model
for the Phase 2 PLC engineering core. It owns authored LAD semantics; layout is
stored separately and cannot affect validation, execution order, or generated
IR.

The crate provides:

- stable identities for documents, networks, nodes, ports, edges, operands,
  branches, branch paths, calls, and stateful instruction instances;
- series and ordered parallel power-flow graphs, including nested branches;
- typed contacts, coils, instruction boxes, block calls, and explicit state
  ownership;
- atomic structural edit batches with exact undo and editable-invalid source;
- deterministic structural, binding, type, alias, and resource-limit
  diagnostics;
- fail-closed lowering into independently verified shared `plc-compiler` IR,
  source-map, and probe contracts, including EN/ENO activation, explicit
  stateful instruction identities, and FC/FB call frames.

Malformed graphs remain available for repair, but they never emit executable
IR. This crate contains no renderer, simulator, runtime executor, host I/O,
filesystem, clock, network, process, FFI, or Phase 3 behavior.

General data-block operands remain an explicit typed lowering gap because the
current shared IR addresses executing-block members only. Such operands stay
editable and validate against the program model, but are never reinterpreted as
same-numbered caller members and never produce an artifact.

The crate remains a standalone workspace while its Phase 2 integration point is
reviewed. Its only dependencies are local, capability-free Phase 2 crates.
