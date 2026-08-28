# PLC Commissioning

`plc-commissioning` is the capability-free, in-memory orchestration layer for
virtual controller instances, verified load previews, atomic load transactions,
online sessions, epoch checks, and offline/online comparison.

Instance creation and exact-state cloning are typed commands. Destructive
reset, replacement, and removal are separately previewed and approval-bound;
epoch-changing actions invalidate prior session bindings. Actual virtual
hardware presence/fault state remains distinct from offline configuration.

Every artifact-changing action is previewed and approval-bound to the candidate,
target identity, controller epoch, CPU mode, complete target state hash, force
registry hash, hardware state, and offline build state. Candidate runtime state
is staged outside the live target and becomes visible through one atomic commit.
Requested post-load RUN is a separate transition after that commit. An internal
failure restores the exact target and retains an audit event.

Run the isolated verification suite with:

```text
cargo test --manifest-path crates/plc-commissioning/Cargo.toml
```
