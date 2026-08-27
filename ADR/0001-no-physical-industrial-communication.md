# Physical Industrial Communication Is Permanently Out of Scope

- Status: Project Safety Invariant
- Decision ID: ADR-0001
- Date: 2026-08-27
- Scope: Entire product and every production artifact
- Supersedes: None
- Amended by: None

## Context

The product teaches modern PLC engineering by simulating decisions and consequences inside a fictional VirtualUniverse. Training-transfer fidelity does not require, and may never introduce, a path to physical controllers, HMIs, drives, I/O, instruments, devices, host networks, or industrial protocols.

An apparently harmless abstraction such as a disabled connector, generic transport, endpoint provider, localhost bus, protocol plugin, or “simulator now, hardware later” interface would create latent physical capability. Configuration does not remove capability, and a firewall-blocked packet is still an attempted communication. The product must therefore be incapable by construction, dependency policy, packaging, and release proof—not merely disconnected by default.

This decision implements `PES-GOV-0009`, `PES-SCP-0002`-`PES-SCP-0004`, `PES-ISO-0001`-`PES-ISO-0022`, `PES-SEC-0001`-`PES-SEC-0011`, `PES-DEV-0008`, `PES-DEV-0012`, `PES-ARC-0003`, `PES-ARC-0026`, `PES-ARC-0030`, and `PES-QLT-0005`.

## Decision

**VirtualUniverse has no adapter to PhysicalUniverse.**

Physical industrial communication is permanently excluded from this product. The exclusion is category-wide, product-wide, and applies to production source, generated code, dependencies, optional dependencies, native modules, WASM imports, workers, UI, HMI, Teacher Mode, Learning Lens, assessment, import/export, persistence, packaging glue, tests exposed to users, and final packaged artifacts.

### Runtime target and virtual addressing

- A controller session and Virtual Download target are identified only by an opaque `VirtualControllerId`.
- Hostnames, URLs, IP endpoints, ports, sockets, interface indexes, host MAC targets, USB identities, serial handles, Bluetooth identities, and generic connection strings cannot enter the target API.
- `VirtualIpAddress` and similar values are inert domain data used only for formatting, parsing, duplicate detection, subnet comparison, and fictional topology.
- A virtual address cannot convert to a host endpoint or select a host interface.
- Fictional discovery is an in-memory query over VirtualUniverse devices only.
- Virtual Download is an atomic internal transaction that loads a simulator-owned immutable artifact into a virtual controller object.
- Runtime/process/HMI value exchange uses typed in-process calls or typed worker IPC through `InternalTagBus` only.

### Forbidden capability categories

No implementation may include or expose:

- S7/S7comm/S7comm-plus, PROFINET, PROFIBUS, EtherNet/IP/CIP, Modbus, external OPC UA, EtherCAT, CAN/CANopen, DeviceNet, BACnet, MQTT, vendor SDKs, engineering DLLs/APIs, PLCSIM APIs, firmware interfaces, or equivalent industrial communication;
- host NIC/device enumeration, physical discovery, raw Ethernet, packet capture, protocol frames, TCP/UDP/raw sockets, TLS, DNS, HTTP/HTTPS, localhost servers, generic socket APIs, or endpoint resolution;
- browser networking or device APIs including `fetch`, `XMLHttpRequest`, `WebSocket`, `WebRTC`, `EventSource` to endpoints, `sendBeacon`, `WebTransport`, WebSerial, WebUSB, WebBluetooth, WebHID, WebNFC, WebMIDI, or later equivalents;
- serial, USB, Bluetooth, pcap, native device enumeration, native FFI, dynamic libraries, child processes, shell commands, native bridges, or executable plugins in production;
- external HMI providers, network-capable transports, remote collaboration, cloud services, telemetry, or a trusted in-product updater.

This list is illustrative; the category-wide ban controls renamed, wrapped, transitive, future, and equivalent capabilities.

### Production and development separation

Development and CI may acquire dependencies, run compilers, invoke build processes, and use development-only test servers. Those capabilities must remain outside production dependency graphs, shipped bundles, runtime permissions, examples, and user-reachable paths. Production runs without a local server, loads all assets locally, and enforces a default-deny CSP including `connect-src 'none'`.

The eventual desktop/classroom shell remains a BLOCKED product decision under `PES-DEV-0009`. No packaging choice is accepted until background networking, update separation, file permissions, signing, and process-scoped offline verification are approved.

### Immutability of this decision

This ADR **cannot be amended, superseded, relaxed, feature-flagged, or reinterpreted to add physical capability within this product**.

A product that can communicate with physical industrial equipment is a different product. It requires, at minimum:

1. a separate repository;
2. separate explicit authorization from Scott;
3. separate legal and licensing analysis;
4. a separate threat model and safety analysis;
5. separate governance, requirements, claims, packaging, verification, and release evidence.

No ordinary ADR, change record, profile, milestone, plugin, edition, branch, experiment, or customer request can authorize that capability here.

## Decision consequences

### Required consequences

- Architecture uses domain-specific internal contracts rather than generic transport abstractions.
- Virtual networks are semantic graphs, never host networking wrappers.
- HMI, process, monitoring, and education systems share `InternalTagBus` and typed domain events.
- Production dependencies and packaged output are scanned for forbidden capabilities.
- Every semantic/runtime WASM module is inspected for forbidden imports.
- The complete product/course suite must work with network adapters disabled or removed.
- Zero-egress evidence includes zero attempted syscalls and endpoint resolution, not only zero successful packets.
- Endpoint-like values are fuzzed across every text/address field and remain inert.
- Discovery is proven independent of a live LAN.
- Virtual Download is proven to accept only `VirtualControllerId` at all boundaries.
- Exports are proven non-executable, simulator-native, and unusable as physical/vendor deployment artifacts.

### Accepted costs

- Real PLC/HMI/drive integration, external OPC UA, industrial protocol laboratories, vendor project import/export, and hardware-in-the-loop are unavailable by design.
- Some demonstrations must be modeled as original fictional process/hardware behavior rather than connected to external simulators or devices.
- Packaging and dependency choices are narrower because any latent communication capability is unacceptable.
- Tests and release evidence must inspect the final artifact and runtime attempts on each supported platform.

### Benefits

- The student cannot mistake a simulator project for a deployable physical artifact.
- The trusted core remains deterministic, inspectable, and suitable for offline classrooms.
- The product has a precise, defensible safety claim tied to observable evidence.
- Future functionality can grow deeply inside VirtualUniverse without inheriting physical-control risk.

## Alternatives rejected

| Alternative | Reason rejected |
|---|---|
| Ship a disabled physical adapter | Capability remains present and may be activated accidentally or deliberately; violates `PES-ISO-0002`. |
| Define a generic transport/provider interface for future use | Creates the prohibited architectural seam even if no provider ships. |
| Use localhost HTTP/WebSocket for internal buses | Introduces networking and server capability; typed in-process/worker IPC is required. |
| Allow external OPC UA or another “read-only” protocol | Read-only communication is still physical/external industrial capability and can discover or monitor real targets. |
| Depend on firewall rules | Blocked attempts still violate the zero-attempt requirement and are outside product control. |
| Put physical features in a plugin, premium edition, or experimental branch | The exclusion applies to every edition/extension inside this product and repository. |
| Reserve a UI button or placeholder package | Creates misleading completion and a forbidden future seam. |

## Verification obligations

Release acceptance requires every isolation gate traced from the approved requirement and implementation matrix to the milestone scope. Applicability is not discretionary: missing required behavior or a missing harness keeps the affected requirement and milestone open, and only an approved `EXCLUDED` or `DEFERRED` scope decision linked to the affected requirement IDs may remove a gate. Required gates include:

- dependency, source, WASM-import, and packaged-artifact capability scans;
- offline-completeness execution with host network adapters disabled/removed;
- process-scoped zero-egress and zero-attempt monitoring over all supported workflows;
- endpoint-inert field fuzzing;
- live-LAN independence of discovery;
- `VirtualControllerId`-only Virtual Download proof;
- `InternalTagBus`-only HMI proof;
- export isolation and non-deployability proof;
- machine-readable evidence bound to artifact hash, test version, date, platform, result, and reproducible logs.

Skipped, unavailable, flaky, waived, or inconclusive isolation evidence fails release under `PES-ISO-0011` and `PES-QLT-0008`.

## Review rule

Review this ADR for continued compliance when architecture, packaging, dependencies, IPC, WASM imports, HMI, persistence, import/export, or public claims change. Review may strengthen the invariant or its evidence. It may not authorize physical capability.
