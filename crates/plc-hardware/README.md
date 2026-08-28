# plc-hardware

`plc-hardware` is the deterministic, capability-free engineering domain for the shipped EDU-21 Core 1.0 training profile. It is an ordinary Rust library that also compiles for `wasm32-unknown-unknown`.

The crate owns:

- the immutable EDU-21 manifest, catalog, resource limits, restart/retention matrix, canonical manifest bytes, frozen SHA-256 pin, and shipped-profile allowlist;
- fictional controller, rack, station, module, channel, scaling, address-allocation, and immutable hardware-artifact models;
- canonical PLC value/type shapes needed by hardware and symbol engineering;
- case-folded symbol scopes, declarations, UUID-bound references, cross-references, rename previews, constants, tag tables, PLC addresses, tag validation, and deterministic tag allocation;
- in-memory `VirtualNetwork` devices, interfaces, subnets, ports, links, configured/runtime state separation, topology validation, deterministic discovery, and canonical fingerprints.

The library has no filesystem, clock, device-enumeration, socket, DNS, HTTP, process, or physical-protocol API. `VirtualIpAddress` and `VirtualDeviceName` are inert simulator-owned values; discovery accepts only a `VirtualInterfaceId` plus a simulator-domain filter and can return only devices already present in the `VirtualNetwork` object graph.

Runtime fault commands and scan-time process-image effects consume these configuration and artifact types in the runtime domain; this crate deliberately provides the distinct configured/runtime state and quality models without owning a scheduler or host integration. Compiler conversion/arithmetic semantics likewise consume the canonical type/value model in the language/compiler domain.

## Local verification

```text
cargo fmt --manifest-path crates/plc-hardware/Cargo.toml -- --check
cargo clippy --manifest-path crates/plc-hardware/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path crates/plc-hardware/Cargo.toml --no-fail-fast
cargo check --manifest-path crates/plc-hardware/Cargo.toml --target wasm32-unknown-unknown
```

The integration suite freezes the shipped profile, representative hardware, and network fingerprints and exercises legal construction, automatic-allocation stability, stale preview rejection, illegal slots, overlap/capacity/alignment failures, diagnostic capability tampering, provider-module misuse, virtual discovery faults, symbol rename/delete/rebind behavior, constants, table relocation, retention rules, malformed address-like text, and generated negative corpora.
