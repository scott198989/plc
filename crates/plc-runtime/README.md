# PLC Runtime

`plc-runtime` is the deterministic execution core for a simulator-owned
`VirtualController`. It is a dependency-free `no_std + alloc` library. Its public
surface accepts typed, in-memory artifacts and commands; it has no clock, file,
network, device, process, or entropy capability.

The crate owns:

- the seven Phase 2 CPU states and their guarded transitions;
- a fixed 10 ms virtual scheduler with startup, timed-cyclic, and cyclic work;
- distinct raw, natural, effective, and delivered I/O layers;
- loaded starts, actual memory, retain storage, and stateful instruction storage;
- guarded atomic artifact installation plus epoch-safe clone/replacement staging;
- deterministic work-unit watchdog faults and causal diagnostic records;
- immutable, content-addressed snapshots and canonical state/replay hashes.

Run the isolated verification suite with:

```text
cargo test --manifest-path crates/plc-runtime/Cargo.toml
```
