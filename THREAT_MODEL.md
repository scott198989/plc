# Threat Model

Status: Binding Phase 1 threat model  
Companion control document: `SECURITY_INVARIANTS.md`  
Safety invariant: **VirtualUniverse has no adapter to PhysicalUniverse.**

## 1. Scope

This threat model covers the unmodified shipped classroom product and its child processes, production dependency graph, packaged assets, workers, capability-limited WASM, simulator-native files, import/export surfaces, UI, HMI, Learning Lens, Teacher Mode, assessment, and controlled persistence.

It does not claim that a maliciously modified binary or a compromised host operating system is incapable of networking. The product claim is narrower and testable: the unmodified shipped product has no physical-industrial communication code path or capability, and it makes no network syscall or endpoint-resolution attempt during supported use (`PES-SEC-0009`-`PES-SEC-0011`). Evidence is process-scoped and distinguishes unrelated host traffic.

ADR-0005 authorizes a Windows-first WebView2 shell only for the Phase 2 typed simulator-native project-file and fixed-local-backing boundary. The final supported operating systems, public packaging/distribution, installer/portable model, signing, updater/runtime servicing, and release support remain BLOCKED under `PES-DEV-0009`, `PES-DEC-0002`, and `OQ-0001`.

## 2. Security objectives

1. **Physical isolation:** no code or data path can discover, address, connect to, configure, commission, download to, monitor, or operate physical industrial equipment (`PES-ISO-0001`-`PES-ISO-0010`).
2. **Zero production egress:** no production code attempts network communication or endpoint resolution; all resources are local (`PES-SEC-0004`-`PES-SEC-0011`).
3. **Semantic integrity:** trusted state changes only through validated atomic domain commands; UI, persistence, lessons, and scenarios cannot manufacture valid state or outcomes (`PES-ARC-0001`, `PES-ARC-0012`-`PES-ARC-0016`).
4. **Untrusted-input containment:** imported content is bounded validated data and never executable code (`PES-SEC-0012`-`PES-SEC-0016`, `PES-SEC-0021`-`PES-SEC-0024`).
5. **Deterministic execution:** the semantic core, scheduler, process, replay, and assessments use controlled state and virtual time (`PES-ARC-0002`, `PES-DET-0001`-`PES-DET-0007`).
6. **Teacher/student separation:** hidden educational content is logically separated, student identity is minimal, and no classroom data is transmitted (`PES-TCH-0001`-`PES-TCH-0005`).
7. **Supply-chain and evidence integrity:** dependencies/assets are local, approved, traceable, and scanned in the packaged artifact; security evidence is reproducible (`PES-CRM-0021`-`PES-CRM-0025`, `PES-ISO-0012`, `PES-ISO-0022`, `PES-CI-0001`-`PES-CI-0003`).

## 3. Assets to protect

| Asset | Security property |
|---|---|
| VirtualUniverse boundary | No adapter, endpoint, protocol, or physical target can exist |
| Project semantic state | Integrity, stable identity, atomic mutation, explicit invalid/unresolved states |
| Build artifacts and typed IR | Integrity, immutability, fingerprinting, source/profile/build traceability |
| Runtime/process/HMI state | Determinism, causal provenance, separation of raw/CPU-visible/modify/force layers |
| Simulator-native files | Non-executable, schema-valid, bounded, hashed, versioned, migratable without silent loss |
| Teacher-only content | Logical separation and integrity without overstating local secrecy |
| Student projects, grades, IDs, and audit logs | Locality, minimization, integrity, configurable retention/export |
| Production dependency and asset graph | No forbidden capability, remote fallback, unapproved license, or unregistered asset |
| Verification evidence | Reproducibility, artifact binding, platform/date/test-version traceability |
| Original product expression | No contaminated/vendor assets, formats, prose, diagnostics, or trade dress |

## 4. Actors and trust assumptions

| Actor | Capability assumed | Trust posture |
|---|---|---|
| Student user | Can create malformed projects, enter arbitrary text, import hostile files, repeat operations, and inspect files available to their local account | Untrusted input source; normal UI access does not authorize teacher content or host capability |
| Teacher user | Can author lessons/faults/assessments and export local audit data | Trusted for course intent, not trusted to bypass domain invariants or introduce executable content |
| File producer | Can supply malicious archives, CSV/JSON, images, libraries, scenarios, or future declarative content | Fully untrusted until schema, integrity, provenance, and resource validation pass |
| Contributor/dependency | Can accidentally or deliberately introduce prohibited APIs, copied assets, native modules, remote resources, or dynamic behavior | Controlled by clean-room policy, review, provenance, lockfiles, scans, and packaged-artifact inspection |
| Development/CI environment | May use approved dependency acquisition, compilers, isolated local test harnesses, and separately controlled release/signing credentials | Separate zone; these capabilities cannot enter or be callable from production. Development and test work may not contact vendor products, vendor APIs/services, physical industrial equipment, or industrial endpoints. Installed vendor-product observation remains blocked until the counsel-approved protocol required by `PES-CRM-0010` exists. |
| Host operating system | Provides rendering, bounded file dialogs, local persistence, worker/WASM execution, and eventual packaging runtime | Assumed non-malicious for the product claim; compromise is outside the claim boundary |
| Physical/LAN device | May be present and advertise PLC-like services or respond to network probes | Forbidden universe; must not influence discovery or product behavior |

## 5. Trust boundaries and data flows

1. **User/UI → Project Domain:** typed commands and read-only queries only. Validate kind, schema version, bounds, IDs, authorization, and state preconditions before execution (`PES-SEC-0018`-`PES-SEC-0020`).
2. **UI → Worker/WASM:** typed domain messages only. No URLs, shell strings, native method names, transport descriptors, arbitrary maps, or executable code (`PES-SEC-0003`, `PES-SEC-0019`).
3. **File picker → Controlled Persistence:** explicit user-selected simulator documents only. Canonicalize and validate content, not arbitrary host paths or devices (`PES-SEC-0001`, `PES-SEC-0002`).
4. **Persistence → Semantic Core:** explicit supported schemas and migrations. Persistence cannot write trusted state behind domain commands (`PES-SEC-0017`, `PES-ARC-0016`).
5. **Compiler → Build Artifact → Virtual Runtime:** valid semantic models lower to one immutable typed IR; blocking diagnostics produce no runnable artifact (`PES-IR-0001`-`PES-IR-0005`).
6. **Runtime ↔ Process/HMI/Monitoring/Education:** typed in-process calls or typed worker IPC; HMI resolves only through `InternalTagBus` (`PES-ISO-0007`, `PES-ISO-0020`, `PES-ARC-0026`).
7. **Development/CI → Release Artifact:** one-way build output subject to production graph, static capability, WASM-import, provenance, license, CSP, and packaged-artifact gates (`PES-ISO-0012`-`PES-ISO-0014`, `PES-CI-0001`-`PES-CI-0003`).

There is no trust boundary or data flow from the product to a network endpoint, physical device, industrial protocol, external service, cloud, updater, or executable plugin.

### Phase 1 foundation evidence boundary

The current foundation is intentionally narrower than the future product: a
local React UI sends one exact, bounded health command to an inline Worker,
which instantiates one embedded dependency-free Rust/WASM module and returns a
validated deterministic `DomainResult`. The observed module is 247 bytes with
zero imports. The built output is one `dist/index.html` with no external
resource reference and a CSP whose `connect-src` is `none`; the
`'wasm-unsafe-eval'` script token exists solely for the fixed embedded module.
Static scans and headless `file://` interaction tests validate this narrow
path. The browser harness observed zero page-level remote requests, but it is
not process-scoped DNS/syscall/packet monitoring and therefore does not close
TM-05 or authorize a release/packaging choice.

## 6. Threats, mitigations, and verification

| ID | Threat and attack path | Impact | Required mitigations | Release verification | Requirement trace |
|---|---|---|---|---|---|
| TM-01 | A generic or disabled “future connector” is introduced and later bound to a physical target. | Constitutional safety-wall breach. | No adapter, transport provider, endpoint type, connector interface, protocol package, executable plugin, or feature flag; ADR-0001 cannot be amended to permit one. | Architecture/dependency review plus source and packaged-artifact forbidden-capability scans. | `PES-GOV-0009`, `PES-ISO-0001`, `PES-ISO-0002`, `PES-DEV-0012`, `PES-QLT-0005` |
| TM-02 | `VirtualIpAddress`, a display string, or project data is converted into a hostname/socket/URL. | Host or industrial communication becomes reachable through an apparently virtual feature. | Opaque `VirtualControllerId`; inert virtual address value types; no endpoint conversion API; typed target boundary. | Endpoint-inert fuzzing and proof at type, deserialization, reflection, IPC, and UI boundaries. | `PES-ISO-0003`, `PES-ISO-0004`, `PES-ISO-0017`, `PES-ISO-0019` |
| TM-03 | A dependency or source module adds sockets, DNS, HTTP, WebSocket, WebRTC, device APIs, FFI, shell, or subprocess capability. | Network/device/process escape from the trusted product. | Deny category-wide in production; lock exact dependencies; inspect transitive/optional/native/dynamic paths and packaged output. | Dependency/lockfile scan, source scan, SBOM review, packaged-artifact scan, runtime zero-attempt test. | `PES-ISO-0008`-`PES-ISO-0013`, `PES-CI-0001`-`PES-CI-0003` |
| TM-04 | A development test server, debug bridge, mock, or build tool enters a production bundle. | Localhost/network or process capability becomes user-reachable; fake behavior may ship. | Strict dev/production graphs; no production local server; no production-reachable mocks/test doubles. | Production dependency graph comparison, bundle inspection, launch without adapters, route/API negative tests. | `PES-SEC-0004`-`PES-SEC-0006`, `PES-QLT-0002` |
| TM-05 | The Phase 2 Windows WebView2 shell performs background networking, crash reporting, certificate checks, update checks, prefetching, file ingress, or navigation. | Violates zero egress or escapes the typed project-file boundary despite application code being clean. | Pin and hash-bind the reviewed SDK/loader and observed runtime; local packaged assets only; disable background features, navigation, downloads, external drop, browser file pickers, printing, permissions, and external resources; fail closed when the exact typed broker is absent. Public packaging/signing/updater work remains blocked. | Exact packaged process/child-process syscall, DNS, endpoint, packet, NetLog, file, bridge, and runtime-identity evidence on the supported Windows configuration. | ADR-0005, `PES-SEC-0006`-`PES-SEC-0011`, `PES-DEV-0007`, `PES-DEV-0009`, `RSK-0010` |
| TM-06 | A semantic/runtime WASM module imports WASI sockets, filesystem, process, nondeterministic environment, or a generic host-call bridge. | Bypasses TypeScript/source-level controls and compromises determinism/isolation. | Capability-limited WASM; imports limited to memory, typed messaging, controlled simulator clock, and narrow persistence. | Inspect every module and transitive import; reject an unused forbidden import as capability. | `PES-ISO-0014`, `PES-DEV-0004`, `PES-DEV-0008` |
| TM-07 | IPC accepts arbitrary maps, reflection targets, URLs, native methods, or oversized payloads. | Command injection, capability confusion, denial of service, or semantic bypass. | Tagged/versioned schemas; kind/size/ID/authorization/precondition validation; no reflection or dynamic invocation. | Negative/property tests for unknown kinds/versions, bounds, IDs, stale state, URLs, code, and transport descriptors. | `PES-SEC-0003`, `PES-SEC-0018`-`PES-SEC-0020` |
| TM-08 | UI, persistence, lesson, or scenario code mutates trusted state directly. | Hidden partial state, forged diagnostics/grades, replay divergence, or invalid state presented as valid. | Every mutation is an atomic typed command; presentation/persistence remain non-authoritative; audit before/after hashes and provenance. | Dependency-rule tests, API-surface tests, command atomicity/rollback tests, replay/audit comparison. | `PES-ARC-0001`, `PES-ARC-0012`-`PES-ARC-0016`, `PES-SEC-0017` |
| TM-09 | A crafted archive uses `..`, absolute/device paths, symlinks, duplicate entries, case collisions, or ambiguous names. | Writes outside controlled persistence, overwrites content, or creates inconsistent interpretation. | Canonical path validation; traversal and device-path rejection; duplicate/collision detection; bounded document extraction. | Corpus/property fuzzing on all supported platforms; verify no path escapes and failure is atomic. | `PES-SEC-0002`, `PES-SEC-0013`, `PES-PRJ-0003`-`PES-PRJ-0005` |
| TM-10 | A zip bomb, deeply nested object, huge string/array, high-resolution image, recursion cycle, or event flood consumes resources. | Application freeze, memory exhaustion, or nondeterministic denial of service. | Compression, size, count, nesting, image, queue, recursion, and execution budgets; recursion disabled until a verified profile constrains it. | Boundary/fuzz tests assert deterministic failure and recovery at and beyond every limit. | `PES-SEC-0013`, `PES-SEC-0021`-`PES-SEC-0024` |
| TM-11 | Project, library, scenario, lesson, HMI object, or embedded content executes JS/WASM/macros/shell/native code. | Full escape to host/network/device capability. | Content is non-executable; no eval/Function, arbitrary JS/WASM, macros, dynamic native modules, or embedded executables. Any later DSL is original, deterministic, capability-limited, and separately approved. | Malicious corpus and structural scan; verify executable content is rejected rather than ignored or run. | `PES-SEC-0014`-`PES-SEC-0016`, `PES-QLT-0005` |
| TM-12 | HMI binds to OPC UA, HTTP, WebSocket, localhost, fieldbus, or a provider abstraction. | Creates an external/physical communication seam. | `InternalTagBus` is the sole typed, quality-aware, timestamped HMI value path; no external-HMI/transport package. | Static graph analysis plus end-to-end binding-path proof and zero-attempt runtime test. | `PES-ISO-0007`, `PES-ISO-0020`, `PES-ARC-0026`, `PES-DEV-0012` |
| TM-13 | Fictional discovery observes a live LAN or host NIC and returns physical/PLC-like devices. | Student may interact with real equipment; safety and product claims fail. | Discovery is an in-memory query over VirtualUniverse only; no NIC/device enumeration. | Compare identical project discovery with adapters absent and with a live PLC-like LAN; results must be equivalent. | `PES-ISO-0005`, `PES-ISO-0018` |
| TM-14 | Virtual Download emits a vendor/physical load artifact or accepts an endpoint-like target. | Real-tool interoperability or physical deployment becomes possible. | Internal immutable build artifact only; target is `VirtualControllerId`; simulator-native export formats only. | Boundary proof, structural/signature inspection, and simulator-owned hostile fixtures. Vendor-tool observation remains blocked pending `PES-CRM-0010`; physical equipment is never an acceptance-test target. | `PES-ISO-0006`, `PES-ISO-0019`, `PES-ISO-0021`, `PES-PRJ-0001`-`PES-PRJ-0007` |
| TM-15 | Teacher Mode or a scenario inserts expected diagnostics, values, alarms, or passes. | Assessment integrity fails and training becomes canned. | Teacher actions use ordinary domain/fault commands; ordinary engines derive consequences; hidden content is separate. | Attempt direct outcome injection through UI, file, scenario, IPC, and persistence paths; all fail. | `PES-EDU-0004`, `PES-EDU-0005`, `PES-DIA-0003`-`PES-DIA-0005`, `PES-TCH-0001` |
| TM-16 | Student-visible project state reveals answer keys, hidden faults, or scoring rules. | Course integrity loss. | Logical state/package separation, role-appropriate UI, integrity checks, local audit evidence; honest local secrecy claim. | Student-role access and tamper tests; document that unrestricted host access is outside the secrecy guarantee. | `PES-TCH-0001`, `PES-TCH-0002` |
| TM-17 | Student IDs, grades, logs, or projects are transmitted, over-collected, or retained without local control. | Privacy breach and violation of offline product claim. | Local pseudonymous IDs; no required personal data/account/telemetry; no external transmission; configurable local retention/export before Teacher Mode release. | Data-flow/static analysis, zero-egress runtime test, local retention/export acceptance tests when specified. | `PES-TCH-0003`-`PES-TCH-0005` |
| TM-18 | A remote font, CDN asset, telemetry SDK, analytics package, license client, external AI, or updater is added. | Egress, supply-chain exposure, offline failure, or undisclosed data flow. | Bundle everything locally; CI blocks remote dependencies/assets/services; no trusted updater or online AI. | Offline completeness, URL/string/dependency scan, CSP test, zero-attempt monitoring. | `PES-MSN-0007`, `PES-SCP-0006`, `PES-SEC-0006`-`PES-SEC-0008`, `PES-CI-0001` |
| TM-19 | A vendor screenshot, icon, prose, diagnostic, library, format, contaminated contribution, research note, or evidence archive enters production. | IP/trademark risk, compromised clean-room evidence, or collapse of the evidence/production boundary. | Original assets/expression, source/IP classification, provenance, quarantine, contributor attestation, CI rejection, and a production-bundle allowlist that excludes research/evidence material. | Provenance/license/asset manifest review, contamination response drill, and packaged-file negative scan proving that research notes, evidence records, quarantined material, manuals, screenshots, citation caches, and source archives are absent. | `PES-CRM-0001`-`PES-CRM-0025`, `PES-DOC-0004`, `PES-CI-0002`, `PES-CI-0003` |
| TM-20 | A corrupt/unknown/hash-mismatched project is silently accepted, partially migrated, or has identities retargeted. | Data loss, forged state, unresolved references hidden, or nondeterministic behavior. | Fail closed; immutable UUIDs/tombstones; collision detection; explicit migrations; structured errors; atomic transactions. | Golden/property migration tests, corruption/hash tests, collision/identity/undo tests, rollback verification. | `PES-PRJ-0003`-`PES-PRJ-0005`, `PES-ARC-0004`-`PES-ARC-0015`, `PES-SEC-0022` |
| TM-21 | Wall-clock timers, scheduling races, unseeded randomness, or uncontrolled event ordering alter runtime or grades. | Non-reproducible diagnostics, HMI, traces, and assessments. | Simulator-controlled monotonic time; stable event ordering; replay identity hashes/seed/version; authoritative work in isolated workers. | Repeat/replay/property tests across supported platforms and execution speeds; compare observable streams. | `PES-DET-0001`-`PES-DET-0007`, `PES-DEV-0005`, `RSK-0005` |
| TM-22 | Security tests are skipped, unavailable, flaky, manually waived, or observe only blocked packets rather than attempts. | A release is accepted without proof of isolation. | Every isolation gate is release-blocking; attempted syscalls/resolution fail; evidence bound to artifact/platform/test version. | CI enforces hard failure and produces machine-readable logs/reports. | `PES-ISO-0011`, `PES-SEC-0010`, `PES-SEC-0011`, `PES-ISO-0022`, `PES-QLT-0008` |

## 7. Required misuse and abuse cases

The following negative workflows must remain part of the verification corpus:

- Paste a URL, hostname, loopback address, UNC/device path, IPv6 literal, industrial-looking port, or malformed endpoint into every text/address-bearing field.
- Import projects/archives containing traversal paths, absolute paths, duplicate entries, oversized data, extreme compression, nested archives, invalid UTF-8, unknown schemas, hash mismatches, executable extensions, scripts, WASM, and malicious images.
- Attempt Virtual Download with a string, URL, host/port pair, serialized endpoint, reflected object, stale controller ID, deleted/tombstoned ID, and unauthorized ID.
- Run fictional discovery with host network adapters absent, host network adapters disabled, and an active LAN containing PLC-like equipment.
- Attempt HMI binding through a URL, OPC-style string, localhost, provider name, socket descriptor, or injected transport field.
- Attempt outcome injection through Teacher Mode, lesson/scenario packages, UI state, persistence, IPC, and assessment files.
- Launch every supported workflow while monitoring application and child-process network syscalls, DNS/endpoint resolution, and packets.
- Compare source/lockfile capability scans with the final packaged artifact and every WASM import table.
- Repeat supported deterministic runs across pause, step, speed changes, UI load, and supported platforms using the same replay identity.

## 8. Verification evidence and acceptance

Every isolation/security result is machine-readable and records:

- artifact SHA-256;
- application/build version;
- test and harness version;
- requirement and formal verification IDs;
- platform and packaging identity;
- date;
- inputs, seed, profile/build/snapshot identities where relevant;
- pass/fail result;
- logs sufficient to reproduce the test;
- distinction between application/child-process activity and unrelated host traffic.

The release candidate must also provide the SBOM, license notice set, asset manifest, requirement-verification report, and isolation report required by `PES-CI-0003`. A test blocked by the firewall, an unavailable network monitor, an unsupported platform, or missing logs is not a pass.

## 9. Residual risks and explicit claim limits

| Residual risk | Accepted boundary or required action |
|---|---|
| A student with unrestricted host/process access can inspect or tamper with local files. | Do not claim absolute secrecy. Provide logical separation, integrity evidence, and local audit behavior under `PES-TCH-0002`. |
| A maliciously modified binary or compromised OS can add networking. | Outside the narrow product claim. Sign/hash releases when the packaging decision is approved and bind evidence to the unmodified artifact. |
| Desktop/browser shells may have platform-specific background behavior. | ADR-0005 authorizes only the Phase 2 Windows typed project-file verification boundary. No configuration is accepted until exact-candidate process-scoped zero-attempt evidence passes; broader `OQ-0001` work remains blocked. |
| Exact resource limits and parser budgets are not yet specified. | They must be objective before the corresponding format/feature is implemented; affected work remains BLOCKED rather than using unbounded defaults. |
| Future declarative lesson/HMI behavior could grow toward general scripting. | No executable seam is reserved. Any future DSL requires separate requirements, a capability proof, threat-model update, and deterministic resource tests. |

## 10. Review triggers

Update this threat model before merging any change to CSP, packaging, trust-zone ownership, dependencies, native/WASM imports, workers/IPC, persistence, project/archive schema, import/export, HMI bindings, Teacher Mode data, audit retention, scripting/DSL behavior, or public safety/offline claims (`PES-SEC-0025`).

The update requires the change record and approval chain in `PES-GOV-0014`-`PES-GOV-0016`. Any change touching a mandatory stop category remains BLOCKED until Scott supplies the exact approval or evidence required by `PES-DEC-0002`-`PES-DEC-0006`.
