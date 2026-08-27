# Security Invariants

Status: Binding Phase 1 product constitution  
Applies to: all production code, production dependencies, packaged artifacts, simulator-native files, workers, UI surfaces, HMI, education features, tests, and release evidence  
Controlling statement: **VirtualUniverse has no adapter to PhysicalUniverse.**

## 1. Purpose and authority

This document defines the security boundary that every implementation and release of the PLC Engineering Simulator must preserve. It operationalizes the Phase 1 directive without expanding its scope. If this document conflicts with the directive, applicable law, an approved decision from Scott, or a higher safety constraint, the higher authority controls and the affected work is BLOCKED under `PES-GOV-0006`, `PES-GOV-0008`, and `PES-DEC-0002`.

The boundary is a product constraint, not a feature toggle. No implementation, dependency, packaging choice, profile, lesson, plugin, test mode, edition, or ADR may create a path from VirtualUniverse to physical equipment, a host endpoint, an industrial protocol, or an external service (`PES-GOV-0009`, `PES-ISO-0001`, `PES-ISO-0002`).

## 2. Security properties

The following properties are release invariants:

| Invariant | Binding rule | Primary requirements |
|---|---|---|
| SI-01: sole automation universe | Every addressable controller, device, network, process, HMI, tag, and runtime target exists only in VirtualUniverse. | `PES-ISO-0001`-`PES-ISO-0007` |
| SI-02: no physical adapter | No disabled, hidden, generic, experimental, premium, future-facing, or test-only adapter to physical equipment or host networking exists. | `PES-ISO-0002`, `PES-QLT-0005` |
| SI-03: opaque runtime target | A controller session and Virtual Download accept only `VirtualControllerId`; endpoint-like values cannot enter the target API. | `PES-ISO-0003`, `PES-ISO-0019` |
| SI-04: inert virtual addressing | `VirtualIpAddress` and similar values support fictional topology semantics only and cannot convert to host endpoint types. | `PES-ISO-0004`, `PES-ISO-0017` |
| SI-05: internal-only value exchange | Controller, process, HMI, monitoring, and education value exchange uses typed in-process calls or typed worker IPC through `InternalTagBus`. | `PES-ISO-0007`, `PES-ISO-0020`, `PES-ARC-0026` |
| SI-06: explicit trust crossings | Every command and serialized payload crossing a trust boundary is tagged, versioned, bounded, authorized, and validated before execution. | `PES-SEC-0018`-`PES-SEC-0020` |
| SI-07: untrusted data never becomes code | Projects, archives, data files, assets, libraries, lessons, scenarios, and HMI content are data only and are never executed. | `PES-SEC-0012`-`PES-SEC-0016` |
| SI-08: production is offline-complete | The shipped classroom product requires no server, network, cloud, remote font, CDN, telemetry, updater, or external AI. | `PES-MSN-0007`, `PES-SCP-0006`, `PES-SEC-0004`-`PES-SEC-0008` |
| SI-09: causal state ownership | Presentation, persistence, lessons, and scenarios cannot bypass domain commands, manufacture trusted state, or inject expected outcomes. | `PES-SEC-0017`, `PES-ARC-0001`, `PES-ARC-0012`-`PES-ARC-0016`, `PES-DIA-0003`-`PES-DIA-0005` |
| SI-10: bounded execution | Parsing, migration, simulation, queues, recursion, and archive handling are deterministic or explicitly bounded and fail closed. | `PES-SEC-0013`, `PES-SEC-0021`-`PES-SEC-0024` |
| SI-11: minimal local student data | Student identity is local and pseudonymous by default; grades, logs, projects, and identifiers never leave the product. | `PES-TCH-0001`-`PES-TCH-0005` |
| SI-12: proof before release | Isolation evidence is release-blocking; unavailable, skipped, flaky, waived, or inconclusive security tests fail the gate. | `PES-ISO-0011`-`PES-ISO-0022`, `PES-QLT-0008` |

## 3. Trust zones and ownership

Package ownership must follow these zones. A package may be combined with another package only when an approved ADR shows that the trust boundary, dependency direction, ownership, and testability remain intact (`PES-DEV-0011`).

| Zone | Contents | May trust | Must not do |
|---|---|---|---|
| Trusted semantic core | Project domain, canonical type system, validators, dependency engine, compiler, typed IR, runtime, virtual hardware/network, process engine, diagnostics | Validated typed commands and deterministic internal services | Call presentation code; parse editor layout in runtime; use OS networking, native FFI, arbitrary filesystem, shell, device APIs, dynamic invocation, or unbounded resources |
| Trusted presentation | Engineering UI, semantic editors, property surfaces, Learning Lens, Teacher Mode UI, HMI renderer | Read-only queries, domain results, typed events, approved commands | Own authoritative PLC semantics; mutate domain objects directly; synthesize diagnostics, values, grades, or valid state |
| Controlled persistence | Simulator-native open/save, application-local persistence, recovery journal, snapshots | Explicit schemas, canonical object model, bounded document operations | Execute content; traverse arbitrary paths; silently discard unknown/corrupt data; write semantic state behind the domain model |
| Untrusted content | Imported projects, archives, CSV/JSON, images, libraries, scenarios, lessons, future declarative scripts | Nothing until validation succeeds | Execute code; supply host objects; select a process/device/endpoint; exceed declared resource budgets |
| Development environment | Package managers, compilers, bundlers, test servers, CI tools, signing tools | Development-only configuration and credentials | Enter production dependency graphs, shipped bundles, runtime permissions, examples, or user-reachable code paths |
| Forbidden universe | Physical PLCs/HMIs/drives/I/O, host NICs/endpoints, industrial protocols, device APIs, external services | Nothing | Be represented by an adapter, connection interface, transport descriptor, protocol provider, or executable extension |

The Engineering UI issues typed commands to the Project Domain. The Project Domain owns semantic objects and invariants. Validators and compilers analyze those objects. Valid language models lower to one typed IR, which is executed by one deterministic virtual runtime. Process, HMI, diagnostics, monitoring, Learning Lens, Teacher Mode, and assessment use typed internal contracts. This dependency direction is mandatory under `PES-ARC-0001`-`PES-ARC-0003`.

## 4. Production capability allowlist

Production code may use only the narrow capabilities below (`PES-SEC-0001`-`PES-SEC-0003`):

1. Local rendering, accessibility, and user interaction.
2. Explicit user-initiated open/save for simulator-native project, archive, CSV, JSON, image, or report formats approved by a normative requirement.
3. Controlled application-local persistence and recovery.
4. Typed UI-to-worker domain messaging.
5. Memory allocation within deterministic or explicit limits.
6. Simulator-controlled monotonic virtual-time inputs.
7. Printing or local document export only after a later requirement approves the format and proves that no external resource is loaded.

File access is a bounded document operation. It is not a general filesystem API and cannot expose arbitrary traversal, executable launch, device files, shell access, or host-device access. UI-to-worker messages contain domain commands, queries, events, or results only; they cannot carry arbitrary code, URLs, shell strings, native method names, or generic transport descriptors.

Anything not on this allowlist is denied until a controlling requirement and security review explicitly permit it. This deny-by-default rule cannot be used to approve a capability categorically forbidden elsewhere in the directive.

## 5. Category-wide forbidden capabilities

The prohibition is semantic and category-wide. Renaming, wrapping, transitive inclusion, optional loading, a feature flag, an experimental branch, or a test-only path does not make a forbidden capability permissible (`PES-ISO-0008`-`PES-ISO-0010`). Production source, dependencies, generated code, WASM, workers, packaging glue, and shipped artifacts must not contain or expose:

- S7, S7comm, S7comm-plus, PROFINET DCP/I/O, PROFIBUS, EtherNet/IP, CIP, Modbus TCP/RTU, external OPC UA, EtherCAT, CAN/CANopen, DeviceNet, BACnet, MQTT, or equivalent physical/industrial transports;
- vendor PLC, HMI, drive, or I/O SDKs; TIA Openness; Siemens engineering libraries; PLCSIM APIs; firmware or device packages;
- physical discovery, host NIC enumeration/selection, raw Ethernet, packet capture, industrial frames, or conversions from virtual addresses to endpoints;
- TCP, UDP, raw sockets, TLS, DNS, HTTP/HTTPS, localhost servers, generic socket APIs, or endpoint resolution;
- `fetch`, `XMLHttpRequest`, `WebSocket`, `WebRTC`, endpoint `EventSource`, `sendBeacon`, `WebTransport`, service-worker network interception, or later equivalents;
- WebSerial, WebUSB, WebBluetooth, WebHID, WebNFC, WebMIDI, serial, USB, Bluetooth, pcap, native device enumeration, or later equivalents;
- child-process execution, shell commands, dynamic native library loading, native FFI, `dlopen`, arbitrary native bridges, or executable plugins;
- `eval`, `Function` constructors, arbitrary JavaScript, arbitrary WebAssembly, macros, arbitrary project/HMI/lesson scripts, dynamic imports selected by untrusted content, or executable embedded content;
- external HMI transports, cloud services, telemetry, analytics, remote assets/fonts, license servers, cloud grading, external AI, remote collaboration, or an in-product network updater.

No package named or functioning as a network, transport, connector, vendor adapter, protocol, external HMI, remote collaboration, or plugin host may be created (`PES-DEV-0012`). No placeholder interface may reserve such a seam (`PES-ARC-0030`, `PES-QLT-0005`).

## 6. Production versus development boundary

Development capability is not production capability. The following separation is mandatory (`PES-SEC-0004`-`PES-SEC-0008`):

| Concern | Development/CI | Production classroom artifact |
|---|---|---|
| Dependency acquisition | Package managers may contact registries under controlled development policy. | All dependencies are bundled locally; no runtime acquisition or remote fallback. |
| Compilers and bundlers | May invoke child processes and local build tools. | No compiler shelling, process execution, native bridge, or dynamic tool download. |
| Test servers | May be used by development-only test harnesses. | No local HTTP, HTTPS, WebSocket, or localhost server. Assets/workers/WASM/help load without network transport. |
| Credentials and signing | CI may use separately controlled credentials. | No embedded credentials, account requirement, license server, or credential prompt for core use. |
| Network controls | Only approved dependency acquisition, release/signing infrastructure, and isolated local test harnesses may use development networking. Test harnesses must not contact vendor products, vendor APIs/services, physical industrial equipment, or industrial endpoints. Observation of an installed vendor engineering product remains blocked until the counsel-approved protocol required by `PES-CRM-0010` exists. | Application and child processes make no network syscall or endpoint-resolution attempt. Firewall-blocked attempts still fail. |
| Updates | Build/release infrastructure may publish artifacts. | No trusted in-product updater. Any later updater requires separate packaging and permissions and must be absent from classroom builds. |
| Debug/test doubles | May exist in test-only graphs. | No mock, test double, debug bridge, test server, or bypass reachable from production. |

Production dependency resolution must prove that dev-only packages, optional dependencies, aliases, native modules, test utilities, and dynamic imports are absent from the shipped graph. The packaged artifact, not merely source and lockfiles, is authoritative evidence (`PES-ISO-0012`, `PES-CI-0002`).

The final desktop/classroom shell, supported operating systems, installation model, file permissions, code signing, background-network suppression, and updater separation remain BLOCKED by `PES-DEV-0009`, `PES-DEC-0002`, and OQ-0001. No document or implementation may silently choose them.

## 7. Content Security Policy and local resources

The production Content Security Policy must include `connect-src 'none'` and default-deny controls for external scripts, styles, fonts, images, media, objects, frames, forms, manifests, base-URI changes, and unsolicited navigation (`PES-SEC-0007`).

All production JavaScript, styles, fonts, images, media, WASM, workers, help, examples, scenarios, and translations are bundled locally (`PES-DEV-0007`). A CSP is defense in depth; it does not excuse prohibited networking code or failed zero-attempt evidence.

## 8. Typed command and worker boundary

Every meaningful mutation is a domain command under `PES-ARC-0012`-`PES-ARC-0016`. At each UI, worker, persistence, or service boundary, validation occurs before dispatch:

| Validation | Required outcome |
|---|---|
| Message discriminator | Known domain command/query/event/result kind only |
| Schema version | Explicitly supported version; unknown versions fail closed |
| Payload bounds | Declared byte, string, collection, nesting, and object-count limits |
| Object identity | Well-formed UUIDs with existence/tombstone/collision rules applied |
| Capability authorization | Caller may invoke the specific domain operation in the current mode |
| State preconditions | Command is legal for the current project/build/runtime state |
| Field semantics | Endpoint-like data remains inert where permitted and is rejected from target/transport positions |
| Transaction boundary | Success is atomic; failure leaves prior state or an explicit editable-invalid state |

Untagged arbitrary maps, reflection-selected methods, dynamic class names, native method names, URLs, executable source, and transport descriptors are invalid messages (`PES-SEC-0019`, `PES-SEC-0020`).

## 9. Untrusted file and archive boundary

All imported content is untrusted (`PES-SEC-0012`). Before it can enter controlled persistence or the semantic core, the importer must enforce:

- an explicit schema and supported schema version;
- canonical path validation and archive traversal prevention;
- duplicate-entry rejection;
- compressed/uncompressed size and compression-ratio limits;
- file-count and nesting limits;
- string, array, object, token, and image-dimension limits;
- deterministic parse/migration resource budgets;
- manifest inventory and hash validation for simulator-native packages;
- UUID collision detection and explicit traced remapping only where the operation is defined to create independent objects;
- structured, actionable failure with no silent discard or catch-and-success.

Unknown, corrupt, oversized, hash-mismatched, ambiguous, or executable content fails closed (`PES-PRJ-0003`-`PES-PRJ-0007`, `PES-SEC-0013`, `PES-SEC-0022`). A future limited DSL may exist only after a later approved requirement defines a deterministic interpreter, capability model, resource bounds, and security tests; it can never expose host objects, network, filesystem, process, dynamic imports, or general-purpose code (`PES-SEC-0015`, `PES-SEC-0016`). This statement does not reserve an executable plugin or scripting seam.

## 10. WASM boundary

Trusted Rust semantics, compiler, IR, scheduler, and runtime are compiled to capability-limited WebAssembly unless Scott approves the alternative required by `PES-DEV-0004`. Every semantic/runtime module is inspected before release (`PES-ISO-0014`).

The Phase 1 foundation exercises this boundary without implementing PLC
semantics. A versioned, exact-key `foundation.health` command crosses one
inline Web Worker boundary and invokes an embedded first-party Rust module. The
module is 247 bytes in the observed closure build, declares zero imports, and
exports only linear memory plus `foundation_health` and
`foundation_health_len`. Its deterministic payload is converted to the
reconciled `DomainResult` envelope: `success`, optional `value`, `events`,
`diagnostics`, `affectedObjectIds`, optional `undoToken`, `beforeHash`, and
`afterHash`. A health check emits empty event/diagnostic/object arrays and fixed
equal 64-hex before/after hashes; failures use the same envelope with one
bounded diagnostic.

The single `file://` artifact uses `default-src 'none'`, `connect-src 'none'`,
hash-bound inline script/style, `worker-src blob:`, and the narrow
`'wasm-unsafe-eval'` token required for browser compilation of the embedded
module. That token does not authorize arbitrary module input: there is no file,
URL, dynamic import, network, or user-content path to WebAssembly bytes, and
the build/runtime checks reject any WASM import. Changing this CSP token,
module byte source, import table, command schema, or DomainResult members
triggers this document's review rule.

Permitted imports are limited to:

- memory;
- deterministic typed host messaging;
- a controlled simulator clock;
- narrowly controlled persistence operations that satisfy this document.

Forbidden imports include networking or endpoint resolution, WASI sockets, arbitrary filesystem, process execution, environment-derived nondeterminism, device APIs, native FFI, and dynamic host invocation. An unused forbidden import fails the gate because its presence is capability.

## 11. Teacher/student data boundary

Teacher-authored answer keys, hidden faults, checkpoints, and scoring rules are logically separate from student-visible project state (`PES-TCH-0001`). Teacher Mode acts through ordinary commands, fault providers, scenario events, and assessment rules; it does not insert diagnostics, alarms, values, traces, or passes (`PES-EDU-0004`, `PES-EDU-0005`, `PES-DIA-0004`, `PES-DIA-0005`).

The local offline threat model does not promise secrecy against a student with unrestricted filesystem or process access (`PES-TCH-0002`). The product must provide role-appropriate UI separation, package integrity, and audit evidence without claiming cryptographic impossibility. Student identity is pseudonymous by default and requires no name, email, account, telemetry, or network service (`PES-TCH-0003`). Audit retention/export defaults remain blocked for Phase 3, but no grade, log, project, or identifier may be transmitted (`PES-TCH-0004`, `PES-TCH-0005`).

## 12. Security verification and release gates

Gate applicability is not discretionary. The approved requirement and implementation matrix determines the milestone scope, and every user-reachable or release-reachable workflow inherits every gate that traces to that scope. If required behavior or its harness does not exist, the requirement remains incomplete and the milestone stays open; absence never turns a gate into `N/A`. A gate may be removed from a milestone only by an approved `EXCLUDED` or `DEFERRED` scope decision recorded against the affected requirement IDs. Formal test records must use `VER-AREA-NNNN` identifiers in the requirement/verification register; these control names are stable security-gate labels, not a substitute for those records.

| Gate | Required verification | Pass condition | Requirements |
|---|---|---|---|
| SI-GATE-01 Dependency and artifact scan | Scan production graphs, lockfiles, optional/aliased/transitive dependencies, native modules, dynamic imports, WASM imports, and packaged output. | No forbidden capability or unapproved dependency is present. | `PES-ISO-0012`, `PES-CI-0001`, `PES-CI-0002` |
| SI-GATE-02 Source capability scan | Scan trusted and shipped source/generated code for browser, Node, native, FFI, subprocess, device, networking, and industrial APIs. | Zero prohibited references outside explicit development/test exclusions that cannot enter production. | `PES-ISO-0013` |
| SI-GATE-03 WASM import audit | Inspect every semantic/runtime module and its transitive imports. | Imports are limited to memory, typed messaging, controlled simulator clock, and narrow persistence. | `PES-ISO-0014` |
| SI-GATE-04 Offline completeness | Run the complete product and course suite with network adapters disabled or removed. | No engineering, runtime, HMI, monitoring, diagnostic, education, grading, save, or export capability is lost. | `PES-ISO-0015` |
| SI-GATE-05 Zero egress and zero attempt | Monitor application and child-process syscalls, DNS/endpoint resolution, and packets across all representative workflows. | No network attempt or endpoint resolution occurs; unrelated host traffic is distinguished. | `PES-SEC-0009`-`PES-SEC-0011`, `PES-ISO-0016` |
| SI-GATE-06 Endpoint-inert fuzzing | Fuzz all text and address fields with loopback/private/public/multicast/broadcast/IPv6, hostnames, URLs, industrial ports, UNC/device paths, and malformed endpoints. | Values remain inert data and cannot select a host target or transport. | `PES-ISO-0017` |
| SI-GATE-07 LAN independence | Compare fictional discovery with and without a live LAN containing PLC-like devices. | Results and timing-relevant semantic outputs are equivalent. | `PES-ISO-0018` |
| SI-GATE-08 Virtual target proof | Exercise type, deserialization, reflection, IPC, and UI boundaries for Virtual Download. | Only `VirtualControllerId` is accepted; endpoint-like alternatives fail closed. | `PES-ISO-0019` |
| SI-GATE-09 HMI bus proof | Trace every HMI binding and runtime update path. | Every path resolves only through `InternalTagBus`; no external/local transport exists. | `PES-ISO-0020` |
| SI-GATE-10 Export isolation | Inspect and adversarially test every exported artifact. | No vendor project, firmware, load binary, protocol frame, executable, or file directly accepted by a physical industrial tool is produced. | `PES-ISO-0021` |
| SI-GATE-11 Untrusted-input fuzzing | Fuzz every parser/deserializer and archive/migration path, including budget boundaries. | Failures are bounded, structured, recoverable, deterministic, and never execute content. | `PES-SEC-0012`-`PES-SEC-0016`, `PES-SEC-0021`-`PES-SEC-0024` |
| SI-GATE-12 Typed IPC negative tests | Send unknown kinds/versions, oversized payloads, invalid IDs, unauthorized operations, stale preconditions, executable strings, URLs, and transport descriptors. | Every invalid message is rejected before command execution with no partial mutation. | `PES-SEC-0018`-`PES-SEC-0020`, `PES-ARC-0013` |
| SI-GATE-13 Teacher/student separation | Attempt to expose hidden teacher content, transmit student data, or inject outcomes through lesson/scenario/UI paths. | UI separation and integrity controls hold; no external transmission or outcome injection occurs. | `PES-TCH-0001`-`PES-TCH-0005`, `PES-DIA-0005` |
| SI-GATE-14 Evidence integrity | Record evidence for every isolation gate. | Evidence includes artifact hash, test version, date, platform, result, and reproducible logs. | `PES-ISO-0022`, `PES-QLT-0006` |
| SI-GATE-15 Production-bundle evidence exclusion | Compare a declared production allowlist with every packaged file and inspect source maps, archives, caches, generated resources, and installers. | Research notes, evidence records, quarantined material, downloaded manuals, screenshots, citation caches, and source archives are absent from every production bundle. | `PES-DOC-0004`, `PES-CRM-0021`, `PES-CRM-0022`, `PES-CI-0002`, `PES-CI-0003` |

An unavailable harness, skipped case, flaky result, manual waiver, firewall-blocked attempt, or inconclusive observation is failure (`PES-ISO-0011`, `PES-QLT-0008`). Release evidence must include an SBOM, license notices, asset manifest, requirement-verification report, and isolation report (`PES-CI-0003`).

## 13. Change control and exceptions

There is no exception process for physical industrial communication. Such a proposal is a different product and is governed by ADR-0001.

Any change to the CSP, trust zones, package ownership, import/export surfaces, IPC, project/archive format, persistence boundary, WASM imports, or scripting model requires:

1. a change record satisfying `PES-GOV-0014`-`PES-GOV-0016`;
2. an ADR identifying affected requirement IDs;
3. an update to `THREAT_MODEL.md`;
4. positive, negative, fuzz/property, and isolation verification updates;
5. Scott's decision whenever a mandatory stop category in `PES-DEC-0002` is touched.

No ADR, dependency, implementation note, or code change may silently weaken this document. If objective verification is impossible, the affected work remains BLOCKED.
