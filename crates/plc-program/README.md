# `plc-program`

`plc-program` is the isolated P2-04 canonical program-organization crate. It is
`no_std + alloc`, dependency-free, and accepts only caller-supplied identities
and in-memory values. It has no filesystem, clock, network, process, device, or
entropy capability.

It owns the structural engineering model for:

- one program aggregate per controller;
- OB, FC, FB, global DB, and instance DB units with engineering-number spaces;
- stable-ID Input, Output, InOut, Static, Temp, Constant, and Return members;
- FC and FB call sites with formal-ID bindings and explicit FB instance owners;
- single-instance DB and nested multi-instance state paths;
- canonical call, data-use, instance-of, and multi-instance dependency edges;
- blocking recursion, state-layout cycle, binding, direction, type, and alias
  diagnostics;
- CyclicMain, Startup, and TimedCyclic OB declarations;
- the single versioned Phase 2 instruction metadata registry; and
- deterministic interface-change invalidation closures and explanations.

The crate does **not** parse LAD/FBD/SCL, generate IR or artifacts, execute PLC
instructions, load controllers, persist files, render UI, or implement Phase 3
features.

## Canonical behavior

Maps and sets use ordered collections. Interfaces, instructions, calls, and
formal bindings have validated stable-ID order. Validation records are sorted,
deduplicated, localization-free structures. Call cycles are normalized to a
stable identity path. Invalidation chooses the shortest dependency path and a
lexicographic tie-break, producing the same explanation regardless of insertion
order.

Stateful edge, timer, and counter uses require explicitly typed state storage.
FB calls require either a matching instance DB or a matching Static
multi-instance slot. `InstancePath` preserves the root instance DB and each
stable nested slot, so two parent FB instances cannot collapse into shared child
state.

## Standalone verification

```text
cargo fmt --manifest-path crates/plc-program/Cargo.toml -- --check
cargo clippy --manifest-path crates/plc-program/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path crates/plc-program/Cargo.toml
```
