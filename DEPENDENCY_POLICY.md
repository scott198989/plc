# Dependency Policy

Status: Phase 1 controlled policy  
Applies to: all direct, transitive, optional, native, WebAssembly, font, asset, build, development, test, packaging, and installer dependencies  
Governing requirements: PES-CRM-0024; PES-CRM-0025; PES-ISO-0008 through PES-ISO-0015; PES-SEC-0004 through PES-SEC-0008; PES-DEV-0006 through PES-DEV-0009; PES-CI-0001 through PES-CI-0003

## 1. Purpose

Dependencies must preserve the product's independent implementation, offline operation, reproducibility, license compliance, deterministic semantic core, and immutable separation from PhysicalUniverse. A package is not approved because it is popular, already present in a lockfile, transitive, optional, disabled by configuration, or development-only by convention. Approval depends on its actual resolved code, capabilities, licenses, build behavior, packaged output, and reachability.

This policy does not approve any specific dependency. Approval is explicit, version-specific, scope-specific, and reviewable.

## 2. Dependency classes

### 2.1 Production dependency

A dependency is production if any of its code, assets, fonts, data, native modules, WebAssembly, licenses, metadata, install hooks, or generated output is shipped, loaded, imported, reachable, or required by the classroom application. Optional and dynamically imported packages are production dependencies when a shipped path can reach them.

### 2.2 Development dependency

A dependency is development-only only when it is used exclusively to author, compile, lint, test, render, package, scan, or verify the product and none of its code or capability enters a production graph, bundle, runtime permission, installer behavior, user-reachable path, or generated executable payload.

Development tools may download packages, start test servers, invoke compilers, and use child processes in controlled developer or CI environments under PES-SEC-0004. That permission does not extend to production. A bundler, test runner, preview server, browser automation tool, package manager, compiler, linker, signing tool, or scanner must be proven absent from shipped runtime reachability unless its output is separately reviewed.

### 2.3 Tool-generated output

Generated code, compiled WebAssembly, fonts, images, templates, notices, lockfiles, and installer content inherit review obligations from both their generating tool and their own contents. “Generated” is not a provenance or license exemption.

## 3. Absolute production capability bans

No production dependency, including a transitive, optional, native, aliased, dynamically loaded, or generated dependency, may provide or expose:

- TCP, UDP, raw sockets, TLS, DNS, HTTP, HTTPS, localhost servers, generic socket APIs, or endpoint resolution;
- browser networking such as `fetch`, `XMLHttpRequest`, WebSocket, WebRTC, EventSource, `sendBeacon`, WebTransport, service-worker network interception, or an updater/checker;
- WebSerial, WebUSB, WebBluetooth, WebHID, WebNFC, WebMIDI, serial ports, USB, Bluetooth, pcap, host NIC enumeration, device discovery, or arbitrary device files;
- S7/S7comm/S7comm-plus, PROFINET, PROFIBUS, EtherNet/IP/CIP, Modbus, external OPC UA, EtherCAT, CAN/CANopen, DeviceNet, BACnet, MQTT, or another industrial/physical protocol;
- Siemens engineering DLLs, TIA Openness, PLCSIM APIs, vendor PLC/HMI/drive/I/O SDKs, vendor project formats, firmware, device packages, load artifacts, or protocol payloads;
- native FFI, arbitrary dynamic-library loading, shell access, child-process execution, executable plugins, macros, arbitrary JavaScript or WebAssembly execution, or native bridges;
- arbitrary filesystem traversal, executable launch, host credentials, cloud services, telemetry, analytics, remote fonts, CDNs, license servers, remote grading, external AI, or production update services.

The list is illustrative; the category-wide ban controls renamed, wrapped, indirect, future, and equivalent capability. A disabled feature, unused export, compile-time flag, tree-shaken path, sandbox option, or firewall is not an acceptable control when the forbidden implementation remains in a shipped production dependency. These rules implement PES-ISO-0008 through PES-ISO-0010.

## 4. Production allowlist

A production dependency may be considered only when its resolved capability is limited to an approved need such as:

- deterministic rendering and local user interaction;
- pure data structures, parsing, validation, serialization, formatting, or algorithms without forbidden side effects;
- controlled application-local persistence or explicitly initiated bounded document open/save;
- typed UI-to-worker domain messaging;
- deterministic memory management and capability-limited WebAssembly;
- locally bundled, originally created, or properly licensed fonts/assets that load without network access;
- later-approved local printing or report export that loads no external resource.

Presence on this list is not automatic approval. The package still needs license, provenance, maintenance, reproducibility, capability, clean-room, and packaged-artifact review.

## 5. Admission record

Before a dependency is added or its version/range changes, the change record must include:

1. package/crate/tool name, ecosystem, exact resolved version, and registry or repository location;
2. direct, transitive, optional, native, WebAssembly, font, asset, build, development, test, or packaging classification;
3. requested component and business/technical purpose;
4. why an existing approved dependency or project-owned implementation is insufficient;
5. complete license expression, copyright notices, patent terms, attribution, source-offer, redistribution, trademark, and generated-asset obligations;
6. integrity hash/signature information and the lockfile entries that pin it;
7. dependency tree, optional features, build scripts, install hooks, dynamic imports, native code, WebAssembly imports, generated files, and bundled assets;
8. capability review against Sections 3 and 4;
9. maintenance and supply-chain review, including owner, release recency, security history, provenance, and abandonment risk;
10. production-bundle reachability and offline/no-adapter verification plan;
11. reviewer, decision, decision date, approved scope, and verification IDs.

Unknown fields are recorded as unknown and the dependency remains `BLOCKED`; they are not filled by assumption.

Current Phase 1 development/build declarations are inventoried in `docs/governance/TOOLCHAIN_ADMISSION_REGISTER.md`. An entry in that register records review scope and unknowns; it does not approve the tool, its license, its acquisition path, its network use, or any generated production output.

### 5.1 Local Phase 1 bootstrap containment

Scott's authorization to begin Phase 1 permits pre-existing host tools to run
repository-owned, standard-library-only governance extraction and verification
locally while this admission system is being established, but only when the run:

- performs no package acquisition, registry access, telemetry, remote service,
  credential use, updater action, lifecycle hook, or external upload;
- creates only governance records and reproducible local verification evidence,
  never product code, a classroom artifact, or release evidence;
- records the exact observed tool/version and leaves it unapproved;
- treats all output as `IMPLEMENTED_UNVERIFIED` until independently reviewed;
- stops if command resolution, content, capabilities, or observed behavior differs
  from the inventoried baseline.

This is containment for an already-present local bootstrap tool, not dependency
admission. It does not authorize adding a package/action, changing a version,
using the network, enabling hosted CI, selecting a cloud provider, or uploading
an artifact. Those actions still require the full admission record and any
mandatory Scott decision, including `DEC-0002` for remote CI/service use.

## 6. License disposition

No license name creates automatic approval. The intended distribution model, jurisdictions, combination/linking method, modifications, shipped artifacts, and notice/source obligations must be reviewed.

### 6.1 Normal review candidates

Permissive licenses such as MIT, ISC, BSD-2-Clause, BSD-3-Clause, and Apache-2.0 may be candidates for normal project review when the exact text, notices, copyright owners, patent clauses, modifications, and redistribution obligations are verified. They remain unapproved until that review is recorded.

### 6.2 Focused review required

The following are `BLOCKED` pending distribution-specific legal or specialist review:

- unknown, missing, ambiguous, custom, or conflicting licenses;
- copyleft or weak-copyleft licenses, including GPL, AGPL, LGPL, MPL, EPL, and similarly conditioned licenses;
- source-available, noncommercial, field-of-use, ethical-use, research-only, evaluation-only, or no-derivatives terms;
- dependencies with separate commercial terms, click-through conditions, patent restrictions, trademark conditions, data-use terms, or network-service terms;
- font, icon, media, model, dataset, template, or generated-asset licenses whose embedding or derivative rights are unclear;
- packages copied from examples, snippets, forums, gists, archives, vendored trees, or mirrors without authoritative provenance;
- any license obligation incompatible with the selected installer/portable distribution, closed/open source decision, app store, school deployment, or offline packaging model.

PES-CRM-0025 requires rejection when obligations are incompatible or cannot be satisfied and documented.

### 6.3 Forbidden provenance

Reject dependencies containing or derived from vendor source, leaked/confidential material, decompiled/disassembled output, extracted resources, copied vendor assets, protocol-reproduction captures, pirated content, license bypass, or access-control circumvention. Quarantine suspected contamination under `CLEAN_ROOM_POLICY.md`.

## 7. Versioning, locking, and reproducibility

- Direct dependency declarations must use a project-approved exact-version strategy; release resolution must be captured in committed lockfiles.
- Floating tags, unbounded ranges, remote branches, mutable URLs, runtime downloads, CDN imports, and unpinned binary fetches are forbidden for production.
- Package-manager lifecycle/install scripts are denied by default and require explicit review when needed for development. They must never create an unreviewed production payload.
- Registry integrity values, downloaded binary hashes, compiler/toolchain versions, target triples, feature flags, and build configuration must be recorded for reproducible releases.
- Optional features default off. Each enabled feature is reviewed as dependency code, not as a label.
- Vendoring does not remove license, provenance, update, or capability obligations.
- Reproducibility checks compare final artifact hashes where deterministic tooling permits and otherwise record explained, bounded differences.

These controls support PES-DEV-0006, PES-DEV-0007, PES-CRM-0024, and PES-CI-0002.

## 8. Native code and WebAssembly

Native code has two distinct scopes that must not be conflated:

1. **Trusted core and application dependencies:** a native module, native add-on, FFI binding, arbitrary dynamic-library loader, executable plugin, or native bridge reachable by the semantic core or application is categorically forbidden. It cannot be approved through an ordinary dependency exception. The trusted core remains Rust compiled to capability-limited WebAssembly under PES-ISO-0009 and PES-DEV-0008.
2. **Eventual desktop packaging shell:** the separately decided desktop shell may itself be implemented with native platform code only after OQ-0001 and PES-DEV-0009 are resolved through the required product, security, legal, and architecture review. Any approved shell must be capability-minimized, must preserve the immutable VirtualUniverse wall, and must expose no networking, device, process, shell, FFI, arbitrary library-loading, plugin, or endpoint-bearing API to application or semantic-core code. This narrow packaging possibility is not a native-module exception and must not be represented by a placeholder transport, plugin, or bridge seam while the decision remains blocked.

All other proposed production native code triggers a mandatory stop. A configuration flag, wrapper, sandbox claim, or unused path cannot make a forbidden native capability admissible.

Every semantic/runtime WebAssembly module must have its import and export tables inspected. Allowed imports are limited to memory, deterministic typed host messaging, controlled simulator clock inputs, and narrowly controlled persistence. WASI sockets, arbitrary WASI filesystem, process, networking, native FFI, dynamic code loading, and undeclared host functions are forbidden under PES-ISO-0014.

## 9. Fonts, assets, and documentation packages

- All production fonts, icons, images, sounds, animations, templates, samples, and translations must be local and registered in `ASSET_PROVENANCE.json`.
- A package that embeds assets must disclose and register those assets separately; package-level license approval alone is insufficient.
- Remote fonts, analytics pixels, CDN resources, documentation web calls, and runtime examples that fetch resources are forbidden.
- Vendor screenshots, logos, icons, device illustrations, diagnostic prose, or copied manuals cannot be admitted through a dependency.
- Documentation generators and syntax highlighters must use original product themes and project-owned language metadata where required by the directive.

## 10. Development-tool containment

Development and CI tools may use networking, local servers, subprocesses, package registries, browsers, compilers, and signing services only for a recorded acquisition, build, test, security-review, signing, or CI purpose in their controlled environment. Networking is deny-by-default and each permitted use must identify the admitted tool, exact service or endpoint class, requested action, credentials, expected artifacts, retention, and evidence. Containment requires:

- separate development and production dependency graphs or an equivalently auditable boundary;
- production build configuration that excludes development code, preview servers, test fixtures, mocks, browser automation, source maps containing evidence, and credentials;
- no generated runtime code that retains a development endpoint or update check;
- clean-environment production builds with network disabled after dependencies are materialized;
- packaged-artifact scans proving development capabilities are absent;
- no production behavior that depends on a development server, package registry, cloud service, remote font, CDN, or license server.

Development-tool networking does not authorize installed TIA Portal observation, vendor account/support/partner or confidential-material access, protocol capture, device discovery, host-network inspection for product behavior, physical-equipment communication, remote runtime assets, telemetry, or upload of research evidence, source documents, student data, credentials, or forbidden assets. Installed-product observation remains `BLOCKED` by PES-CRM-0010 unless counsel approves the exact license terms and written observation protocol. PES-SEC-0004 cannot be used to bypass that gate.

This distinction implements PES-SEC-0004 through PES-SEC-0008. It does not weaken the production ban or any clean-room restriction.

## 11. Security maintenance

- Security advisories and integrity changes for approved dependencies are reviewed against the pinned version and actual reachable features.
- An emergency update still follows capability, license, provenance, deterministic, and packaged-output review.
- Unsupported or abandoned dependencies receive a recorded replace, fork-with-license-review, or remove decision.
- Dependency confusion, typosquatting, registry takeover, compromised maintainer, unexpected owner transfer, and unexplained binary changes trigger quarantine.
- Secrets and registry credentials remain outside source, lockfiles, logs, generated artifacts, and classroom builds.

## 12. CI and release gates

CI must fail merge or release when:

- a dependency or enabled feature is absent from the admission record;
- a lockfile changes without corresponding review;
- a forbidden capability, API, import, binary, asset, protocol, SDK, remote resource, telemetry path, or updater appears;
- a production bundle contains a development server/tool, native module, dynamic import, WebAssembly import, or optional package that was not approved;
- an asset or font lacks provenance and approval;
- a license is unknown, incompatible, or has unsatisfied notice/source/attribution obligations;
- an SBOM omits a direct, transitive, optional, native, WebAssembly, font, or asset component;
- packaged-artifact, offline, zero-egress, or reproducibility verification is skipped, flaky, unavailable, or inconclusive.

Release candidates must produce an SBOM, license notice set, dependency review report, asset manifest, isolation report, and hashes sufficient to reproduce the reviewed result, as required by PES-CRM-0024 and PES-CI-0003.

## 13. Exceptions and removal

An ADR may document an implementation choice but cannot approve physical capability, a generic transport, a forbidden production dependency, incompatible licensing, or a weakening of the VirtualUniverse wall. Any proposed exception touching those areas is a mandatory stop; physical communication requires a different product and repository.

When a dependency is removed, delete its reachable code/assets from the production graph, update lockfiles and notices, regenerate the SBOM, verify migration/data compatibility, and rescan the packaged artifact. Historical records remain as tombstones for auditability.
