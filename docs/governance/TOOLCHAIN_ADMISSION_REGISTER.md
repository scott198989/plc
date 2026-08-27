# Phase 1 Toolchain Admission Register

Status: **LOCAL BOOTSTRAP CONTAINED; REMOTE CI BLOCKED; ALL TOOLS UNAPPROVED**  
Inventory date: 2026-08-27  
Scope: Phase 1 development, governance extraction, verification, and CI tooling only  
Production reachability: Prohibited

Governing requirements: `PES-CRM-0024`, `PES-CRM-0025`, `PES-ISO-0012`-`PES-ISO-0015`, `PES-SEC-0004`-`PES-SEC-0008`, `PES-DEV-0006`-`PES-DEV-0009`, and `PES-CI-0001`-`PES-CI-0003`.

## 1. Purpose and truth statement

This register inventories the development/build tools currently declared or directly observed for the Phase 1 governance repository. It is the initial admission record required by `DEPENDENCY_POLICY.md`; it is not a license opinion, supply-chain approval, security approval, or release authorization.

**No tool in this register is `APPROVED`.** Every entry remains provisional and unreviewed until an identified reviewer completes the policy's license, provenance, capability, integrity, maintenance, reproducibility, production-reachability, and evidence requirements. Unknown facts remain `UNKNOWN`; a local version string or executable hash does not prove upstream provenance or license compliance.

Pre-existing local tools may be used only within the offline Phase 1 bootstrap containment in `DEPENDENCY_POLICY.md` Section 5.1. No remote action or hosted service may execute, and no tool may be acquired, upgraded, network-enabled, or treated as admitted, until its required review and decision are recorded. Executables, package managers, compilers, test runners, credentials, network capabilities, and services must not enter a classroom bundle, production dependency graph, runtime permission, generated runtime payload, or user-reachable path. Generated output receives independent review.

## 2. Local observation method

The local inventory was produced with read-only commands: command resolution, `--version`, `rustup which`, manifest inspection, and SHA-256 hashing. No upstream registry, license database, package download, or remote service was used for these records. GitHub Action tag/release mappings, upstream identity, signatures, publication history, vulnerability status, licenses, and source-to-bundle correspondence are not asserted or relied upon; only the proposed repository names and 40-character commit declarations are recorded.

Paths under `C:\Users\Scott\.cache\codex-runtimes\` are workspace-provided development dependencies, not repository-owned or production-approved files. Paths under `C:\Program Files`, `AppData`, `.rustup`, and `.cargo` are host-local observations and are not reproducible repository declarations.

## 3. Inventory summary

| Tool | Declared or intended version/reference | Locally observed version | Scope | Current disposition |
|---|---|---|---|---|
| Node.js | Exact `24.19.0` in `package.json` | Bundled runtime `v24.19.0`; ordinary PATH runtime `v24.14.0` | Development verification scripts | `PROVISIONAL_UNREVIEWED`; exact version is fail-closed, but executable identity/provenance remains unapproved |
| pnpm | Exact `11.19.0` in `packageManager` and `engines.pnpm` | `11.19.0` through bundled wrapper | Development package/workspace management | `PROVISIONAL_UNREVIEWED` |
| Python | Exact `3.13.12` in `.python-version`; the launcher and extractor both reject other versions | PATH runtime `3.13.12`; bundled alternative `3.12.13` | Development-only directive extraction | `PROVISIONAL_UNREVIEWED`; exact version is fail-closed, but executable identity/provenance remains unapproved |
| Rust compiler | `1.94.0` | `rustc 1.94.0 (4a4ef493e 2026-03-02)` | Development compiler; no product crate exists | `PROVISIONAL_UNREVIEWED` |
| Cargo | Coupled to Rust toolchain `1.94.0` | `cargo 1.94.0 (85eff7c80 2026-01-15)` | Development build/workspace tool; empty workspace | `PROVISIONAL_UNREVIEWED` |
| GitHub-hosted runner | Proposed mutable label `windows-2025` | No exact image revision, OS build, or preinstalled-tool manifest is pinned locally | Proposed remote development CI | `BLOCKED_DEC_0002_AND_ADMISSION`; workflow job is disabled |
| `actions/checkout` | Proposed commit `3d3c42e5aac5ba805825da76410c181273ba90b1` | Workflow declaration only; source/tag/provenance not verified | Proposed remote checkout | `BLOCKED_DEC_0002_AND_ADMISSION` |
| `actions/setup-node` | Proposed commit `820762786026740c76f36085b0efc47a31fe5020` | Workflow declaration only; source/tag/provenance not verified | Proposed remote Node `24.19.0` setup | `BLOCKED_DEC_0002_AND_ADMISSION` |
| `actions/setup-python` | Proposed commit `5fda3b95a4ea91299a34e894583c3862153e4b97` | Workflow declaration only; source/tag/provenance not verified | Proposed remote Python `3.13.12` setup | `BLOCKED_DEC_0002_AND_ADMISSION` |
| `actions/upload-artifact` | Proposed commit `043fb46d1a93c77aae656e7c1c64a875d1fc6a0a` | Workflow declaration only; source/tag/provenance not verified | Proposed remote report retention | `BLOCKED_DEC_0002_AND_ADMISSION`; no upload authorized |
| Microsoft Word | Executable version `16.0.20326.20100` | Host-local Word 2021 executable; source opened read-only | Offline DOCX-to-PDF visual observation | `PROVISIONAL_UNREVIEWED`; output is an ignored non-gating observation only |
| Poppler tools | `pdfinfo` and `pdftoppm` `26.05.0` | Workspace-bundled executables | Offline PDF metadata inspection and 40-page PNG rendering | `PROVISIONAL_UNREVIEWED`; output is an ignored non-gating observation only |
| PDF QA Python stack | Python `3.12.13`; `pdfplumber 0.11.9`; `Pillow 12.3.0`; `pypdf 6.10.0` | Workspace-bundled alternative runtime/packages | Offline page geometry, text, pixel, and contact-sheet observation | `PROVISIONAL_UNREVIEWED`; distinct from the pinned extractor runtime and not gate evidence |

## 4. Tool admission records

### TC-0001 — Node.js 24.19.0 development runtime

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Execute `tools/phase1/verify-phase1.mjs` through the exact-version launcher `tools/phase1/run_phase1_verification.py`; the verifier invokes the exact Python extractor in read-only `--check` mode before reporting; development/governance only |
| Dependency class | Development runtime; must not ship |
| Repository declaration | `package.json` sets `engines.node` to exact version `24.19.0` |
| Intended Phase 1 baseline | `24.19.0` |
| Observed conforming executable | `C:\Users\Scott\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe`, reports `v24.19.0` |
| Observed executable SHA-256 | `3602F2BB1A10F2CBAB4C36886218A33C1AB3DB87290E73B033C46C77147D0237` |
| Conflicting PATH observation | `C:\Program Files\nodejs\node.exe`, reports `v24.14.0`, SHA-256 `63C259C81E5D472B5F11C8D506070130CB04A1ECF84B80377A34ED6EC9048088` |
| Network/process capability | General development runtime capable of network, filesystem, and child-process APIs; scripts must be reviewed and capability-limited by use |
| Local license evidence | `UNKNOWN`; no Node license file was found at the immediate bundled-runtime root during this audit |
| Upstream source/signature | `UNKNOWN`; bundled binary provenance and upstream signature were not verified |
| Production reachability | Forbidden; Node runtime and APIs are not approved for the classroom artifact |
| Reviewer/decision/date | `UNASSIGNED` / `NOT_REVIEWED` / `null` |
| Blockers | Preserve fail-closed exact-version selection; bind an approved executable identity/path and record authoritative source, license text, distribution obligations, integrity/signature chain, security/maintenance review, and production-bundle absence proof |

### TC-0002 — pnpm 11.19.0

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Manage the empty Phase 1 pnpm workspace and run governance scripts |
| Dependency class | Development package manager; must not ship |
| Repository declaration | `packageManager: pnpm@11.19.0`; `engines.pnpm` exact version `11.19.0` |
| Observed executable | Bundled `pnpm.cmd` wrapper reports `11.19.0` and invokes the bundled Node plus `pnpm.mjs` |
| Wrapper SHA-256 | `1B93F82A7506B0F644F5ADC64A77470351D589DCBE4291D621147B72827F1D96` |
| Local package metadata | Bundled `pnpm/package.json` identifies version `11.19.0`, claims `MIT`, and has SHA-256 `862525B82C79860ED1A196DD2E08D1543A444FC9118474A5C4F581644290D892` |
| Local license evidence | A bundled `LICENSE` file exists, but its copyright, obligations, and correspondence to the exact resolved tool have not been reviewed |
| Observed configured registry | `https://registry.npmjs.org/`; observation is not registry approval |
| Network/process capability | May contact registries and execute lifecycle/build tooling in development; installs are blocked until source and package admissions are approved |
| Upstream provenance/signature | `UNKNOWN`; package-manager acquisition chain and authoritative upstream integrity were not verified |
| Production reachability | Forbidden; package manager, cache, registry configuration, and lifecycle scripts must not ship |
| Reviewer/decision/date | `UNASSIGNED` / `NOT_REVIEWED` / `null` |
| Blockers | Review authoritative source, exact license/notices, bundled dependency graph, acquisition/integrity chain, lifecycle behavior, security history, registry policy, and clean production-bundle exclusion |

### TC-0003 — Python directive-extractor runtime

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Execute `tools/phase1/extract_directive_requirements.py` to read the controlled DOCX and regenerate governance JSON |
| Dependency class | Development extraction runtime; must not ship |
| Repository declaration | `.python-version` pins `3.13.12`; `package.json` invokes the launcher through `python`, and both the launcher and extractor reject any runtime other than `3.13.12` before useful work |
| Current PATH observation | `C:\Users\Scott\AppData\Local\Programs\Python\Python313\python.exe`, reports `Python 3.13.12` |
| Current PATH executable SHA-256 | `A2A4EFF8D0B0C845284C607D50A3B5B966AC5A3121736A2E38E165BD6644D9FE` |
| Bundled alternative | `C:\Users\Scott\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe`, reports `Python 3.12.13`, SHA-256 `D8E3F0ADF246DB00358C0C4ED349CF714898178F9558FB0E944F79F5C07F8EAA` |
| Script dependency observation | Static inspection shows Python standard-library imports only; this is not a complete runtime/SBOM review |
| Local license evidence | Host Python 3.13 installation contains `LICENSE.txt`; it has not been reviewed or bound to an approved distribution decision |
| Upstream source/signature | `UNKNOWN`; installer origin, signature, standard-library inventory, and maintenance posture were not verified |
| Network/process capability | General development interpreter is capable of host access; the current extractor must remain a reviewed local-file transformation only |
| Production reachability | Forbidden; Python and the extractor are governance tooling, not classroom runtime dependencies |
| Reviewer/decision/date | `UNASSIGNED` / `NOT_REVIEWED` / `null` |
| Blockers | Preserve fail-closed exact-version checks; bind an approved executable identity/path, record license/provenance/signature/SBOM evidence, and verify deterministic output plus production-bundle exclusion |

### TC-0004 — Rust compiler 1.94.0

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Compile future trusted Rust semantics to capability-limited WebAssembly after later-phase authorization; currently supports an empty workspace only |
| Dependency class | Development compiler/toolchain; compiler must not ship |
| Repository declaration | `rust-toolchain.toml` channel `1.94.0`; `Cargo.toml` `rust-version = 1.94.0` |
| Observed executable | `C:\Users\Scott\.rustup\toolchains\1.94.0-x86_64-pc-windows-msvc\bin\rustc.exe` |
| Version evidence | `rustc 1.94.0 (4a4ef493e 2026-03-02)` |
| Executable SHA-256 | `6A0699E427EE9C1492EF1C9EA967D035DC4660E92C7FE32F2C6A1038116700E5` |
| Target/feature scope | No target, component, or generated WASM is admitted by this record; exact targets and components are `UNKNOWN` |
| Local license evidence | `UNKNOWN`; toolchain license/notices were not located and reviewed during this audit |
| Upstream source/signature | `UNKNOWN`; rustup acquisition, manifest signatures/checksums, and upstream source correspondence were not verified |
| Network/process capability | Compiler may access files and invoke toolchain components in development; no permission extends to generated production capability |
| Production reachability | Compiler forbidden from shipment; every generated WASM import table and artifact remains separately reviewable |
| Reviewer/decision/date | `UNASSIGNED` / `NOT_REVIEWED` / `null` |
| Blockers | Complete license, rustup/toolchain provenance, checksum/signature, component/target, security, reproducibility, and generated-artifact capability review |

### TC-0005 — Cargo 1.94.0

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Manage the empty Rust workspace and future exact crate resolution after authorization |
| Dependency class | Development build/package tool; must not ship |
| Repository declaration | Coupled to the exact `1.94.0` Rust toolchain; `Cargo.lock` currently contains no package resolution |
| Observed executable | `C:\Users\Scott\.rustup\toolchains\1.94.0-x86_64-pc-windows-msvc\bin\cargo.exe` |
| Version evidence | `cargo 1.94.0 (85eff7c80 2026-01-15)` |
| Executable SHA-256 | `CBFDFC04B61BA49D184C6D3996502A00391D570CB5CB71A00FAEB8C0CE12A4C9` |
| Registry/network scope | No crate registry is approved by this record; future resolution/download requires exact package admission and controlled development networking |
| Local license evidence | `UNKNOWN`; Cargo/toolchain license and notices were not reviewed |
| Upstream source/signature | `UNKNOWN`; acquisition and integrity chain were not verified |
| Production reachability | Forbidden; Cargo, registry configuration, cache, credentials, and build scripts must not ship |
| Reviewer/decision/date | `UNASSIGNED` / `NOT_REVIEWED` / `null` |
| Blockers | Complete toolchain provenance/license/integrity review; approve registry and build-script policy; verify exact lock resolution and production exclusion before crates are added |

### TC-0006 — GitHub-hosted `windows-2025` runner

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Proposed remote execution of the Phase 1 governance workflow after `DEC-0002` and admission review |
| Dependency class | External development CI environment; never a classroom runtime dependency |
| Repository declaration | `.github/workflows/phase1-governance.yml` uses `runs-on: windows-2025` |
| Exact image/version | `UNKNOWN`; `windows-2025` is a mutable image label and does not pin its current image revision, OS build, or installed-tool manifest. The disabled workflow would report `ImageOS` and `ImageVersion` if later approved; no completed run evidence is admitted here |
| Local integrity evidence | None; the hosted runner image is not present locally and was not hashed or inspected |
| Service/license/terms evidence | `UNKNOWN` and `NOT_REVIEWED` |
| Network/credential capability | If approved, the hosted runner would use external GitHub services and network access; the proposed disabled workflow declares repository contents read-only and checkout credential persistence false |
| Production reachability | Forbidden; no runner image, token, service dependency, or hosted-network assumption may enter the product |
| Reviewer/decision/date | `UNASSIGNED` / `NOT_REVIEWED` / `null` |
| Blockers | Replace or supplement the mutable label with an approved reproducibility strategy; record exact image/tool inventory per run, service/terms/data/credential assumptions, supply-chain controls, and evidence retention |

### TC-0007 — `actions/checkout` pinned commit

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Proposed repository checkout in remote development CI after `DEC-0002` and admission review |
| Dependency class | Third-party development CI action; must not ship |
| Repository declaration | `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1` |
| Version/tag claim | `UNKNOWN`; no tag mapping is asserted or treated as evidence |
| Local source/hash evidence | None; only the 40-character commit reference is present locally; action source and resolved bundle were not vendored or inspected |
| License/provenance/signature | `UNKNOWN` and `NOT_REVIEWED` |
| Network/credential capability | If approved, would execute within GitHub-hosted CI and access the repository service; the disabled proposal requests read-only contents and sets `persist-credentials: false` |
| Production reachability | Forbidden; action code, Node runtime, service tokens, and checkout metadata must not ship |
| Reviewer/decision/date | `UNASSIGNED` / `NOT_REVIEWED` / `null` |
| Blockers | Verify commit ownership/source, source-to-bundle correspondence, license/notices, action dependency graph, security history, credential behavior, and retained workflow evidence |

### TC-0008 — `actions/setup-node` pinned commit

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Proposed installation of exact Node.js `24.19.0` in remote Phase 1 CI after `DEC-0002` and admission review |
| Dependency class | Third-party development CI action; must not ship |
| Repository declaration | `actions/setup-node@820762786026740c76f36085b0efc47a31fe5020` with `node-version: 24.19.0` and `package-manager-cache: false` |
| Version/tag claim | `UNKNOWN`; no tag mapping is asserted or treated as evidence |
| Local source/hash evidence | None; only the 40-character commit reference is present locally. Action source, resolved distribution bundle, transitive dependencies, and downloaded Node payload were not vendored or inspected locally |
| License/provenance/signature | Repository ownership, exact license obligations, source-to-bundle correspondence, commit/signature chain, release attestations, and downloaded Node provenance remain `UNKNOWN` and `NOT_REVIEWED` |
| Cache behavior | Workflow explicitly sets `package-manager-cache: false`; this does not disable action download, Node acquisition, general runner networking, or unreviewed behavior in the action bundle |
| Network/credential capability | If approved, would execute in GitHub-hosted CI and may access GitHub and Node distribution services. No current network scope is permitted; endpoints, redirects, integrity verification, credentials, and retained evidence require review |
| Production reachability | Forbidden; action code, downloaded installer/cache material, service tokens, runner metadata, and setup behavior must not ship |
| Reviewer/decision/date | `UNASSIGNED` / `NOT_REVIEWED` / `null` |
| Blockers | Resolve `DEC-0002`; establish durable authoritative provenance; review exact license/notices, source-to-bundle correspondence, action dependency graph, Node download endpoint and integrity/signature behavior, credential behavior, security history, runner evidence, and packaged-artifact exclusion |

### TC-0009 — `actions/setup-python` pinned commit

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Proposed installation of exact Python `3.13.12` in remote CI after `DEC-0002` and admission review |
| Dependency class | Third-party development CI action; must not ship |
| Repository declaration | `actions/setup-python@5fda3b95a4ea91299a34e894583c3862153e4b97` with `python-version: 3.13.12` |
| Version/tag claim | `UNKNOWN`; no tag mapping is asserted or treated as evidence |
| Local source/hash evidence | None; action source, resolved bundle, transitive dependencies, and downloaded Python payload were not vendored or inspected locally |
| License/provenance/signature | `UNKNOWN` and `NOT_REVIEWED`; the workflow declaration does not prove repository ownership or provenance |
| Network/credential capability | If approved, would execute on GitHub-hosted CI and may access GitHub/Python distribution services; no current network scope, package installation, or dependency cache is authorized |
| Production reachability | Forbidden; action code, interpreter, downloaded material, caches, tokens, and setup behavior must not ship |
| Reviewer/decision/date | `UNASSIGNED` / `NOT_REVIEWED` / `null` |
| Blockers | Review license/notices, source-to-bundle correspondence, dependencies, acquisition integrity/signatures, endpoint behavior, security history, credentials, retained evidence, and packaged-artifact exclusion |

### TC-0010 — `actions/upload-artifact` pinned commit

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Proposed retention of `.phase1-verification/phase1-report.json` after `DEC-0002` and service/data review; no source document, production asset, secret, or student data is selected |
| Dependency class | Third-party development CI action and external evidence-storage service; must not ship |
| Repository declaration | `actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a`; artifact name `phase1-governance-report`; retention `14` days |
| Version/tag claim | `UNKNOWN`; no tag mapping is asserted or treated as evidence |
| Local source/hash evidence | None; action source, resolved bundle, service implementation, and transitive dependencies were not vendored or inspected locally |
| Data/network scope | If later approved, would upload one generated JSON report containing check results and hashes through GitHub Actions artifact storage. Current execution and upload are blocked by `DEC-0002`; the disabled workflow warns rather than fabricating evidence when the file is absent |
| License/service/privacy/security evidence | `UNKNOWN` and `NOT_REVIEWED`; service terms, residency, access control, encryption, retention deletion, telemetry, and incident history require review |
| Production reachability | Forbidden; uploader, service client, credentials, artifact URLs, and hosted-storage assumptions must not enter the classroom product |
| Reviewer/decision/date | `UNASSIGNED` / `NOT_REVIEWED` / `null` |
| Blockers | Review action and service licenses/terms, data handling, source-to-bundle correspondence, dependencies, authentication/permissions, integrity, retention/deletion, security history, and production exclusion |

### TC-0011 — Microsoft Word 2021 read-only QA renderer

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Open the supplied directive read-only and export a local PDF for a non-gating visual observation |
| Dependency class | Pre-existing host-local document QA tool; must not ship |
| Observed executable | `C:\Program Files\Microsoft Office\root\Office16\WINWORD.EXE`; version `16.0.20326.20100` |
| Observed executable SHA-256 | `AB39524C857C0F48D75FD52486597E87D7560C3185258562A08E56028EB00DA5` |
| Execution containment | Hidden local COM session; source opened read-only; alerts disabled; no save to the source; derived output only under ignored `.phase1-verification/` |
| Authorization boundary | The observed run was outside the standard-library-only bootstrap exception. Its output cannot satisfy a directive gate or authorize reuse; admission must precede any rerun. |
| Network/process capability | General desktop application with broad host/process/network capability; no such capability is admitted to the product, and offline behavior was not independently proven |
| License/provenance/signature | `UNKNOWN` and `NOT_REVIEWED`; local product/version/hash observation is not license or provenance approval |
| Production reachability | Forbidden; Word, COM automation, Office metadata, and derived QA files must not ship |
| Reviewer/decision/date | `UNASSIGNED` / `NOT_REVIEWED` / `null` |
| Blockers | Review installation provenance, signature, license/terms, update/network behavior, process/data handling, exact rendering reproducibility, and production exclusion |

### TC-0012 — Poppler PDF inspection and rendering tools

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Inspect the derived PDF and render all 40 pages to local PNGs for a non-gating visual observation |
| Dependency class | Workspace-bundled development/QA executables; must not ship |
| Observed version | `pdfinfo 26.05.0`; `pdftoppm 26.05.0` |
| Observed executable SHA-256 | `pdfinfo`: `BC2C0F980C9A2A29CD1E06AACD8D1C7B67A5304E9D1D6F75190BDEB9C81A4365`; `pdftoppm`: `742CBBD9A00931AD16C6618410BC40471375D639A45C61C1D86F3DCFC54B6388` |
| Execution containment | Local file input/output only; no endpoint, credential, package acquisition, or remote service was requested |
| Authorization boundary | The observed run was outside the standard-library-only bootstrap exception. Its output cannot satisfy a directive gate or authorize reuse; admission must precede any rerun. |
| License/provenance/signature | `UNKNOWN` and `NOT_REVIEWED`; version/copyright output and local hashes do not establish the bundled source/license graph |
| Production reachability | Forbidden; executables, libraries, fonts, configuration, and rendered QA output must not ship |
| Reviewer/decision/date | `UNASSIGNED` / `NOT_REVIEWED` / `null` |
| Blockers | Review authoritative source, exact licenses/notices, bundled native libraries/fonts, acquisition integrity/signatures, vulnerability history, rendering determinism, and production exclusion |

### TC-0013 — Bundled Python PDF visual-analysis stack

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Analyze page geometry/text/pixels and compose contact sheets for a non-gating local visual observation |
| Dependency class | Workspace-bundled development/QA runtime and packages; must not ship |
| Observed versions | Python `3.12.13`; `pdfplumber 0.11.9`; `Pillow 12.3.0`; `pypdf 6.10.0` |
| Observed Python executable SHA-256 | `D8E3F0ADF246DB00358C0C4ED349CF714898178F9558FB0E944F79F5C07F8EAA` |
| Execution containment | Local derived PDF/PNG input and ignored evidence output only; no package installation, network request, credential, or external service |
| Authorization boundary | The observed run was outside the standard-library-only bootstrap exception. Its output cannot satisfy a directive gate or authorize reuse; admission must precede any rerun. |
| License/provenance/signature | `UNKNOWN` and `NOT_REVIEWED`; package metadata, native/transitive dependencies, licenses, and bundled provenance were not established |
| Production reachability | Forbidden; interpreter, packages, native libraries, caches, and QA outputs must not ship |
| Reviewer/decision/date | `UNASSIGNED` / `NOT_REVIEWED` / `null` |
| Blockers | Review authoritative sources, exact package/native dependency graph, licenses/notices, integrity/signatures, vulnerability history, generated-output behavior, and production exclusion |

## 5. Controlled development networking

Development networking is deny-by-default except for a recorded, narrowly scoped development purpose. A future approval must identify the tool, exact endpoint or service class, action, credentials, expected artifacts, retention, and evidence. Network capability remains outside production graphs and packaged artifacts.

Permissible categories may include approved dependency acquisition, source checkout, security/advisory retrieval, or CI service operation after their admission reviews. This does **not** authorize:

- browsing or observing an installed TIA Portal product for implementation verification;
- vendor account, support, partner, or confidential-material access;
- protocol capture, device discovery, host-NIC inspection for product behavior, or physical-equipment communication;
- uploading project evidence, source documents, student data, credentials, or forbidden assets to an external service;
- unrecorded registry fallback, telemetry, analytics, update checks, or remote runtime assets.

Installed-product observation remains BLOCKED by `PES-CRM-0010` until counsel approves the exact license terms and written observation protocol. Development-tool networking under `PES-SEC-0004` cannot bypass that gate.

## 6. Admission blockers common to every tool

Before any entry can become `APPROVED`, its review record must contain:

1. exact tool identity, version, component/features, platform, and authoritative source;
2. acquisition path, integrity hash/signature chain, and reproducible installation evidence;
3. exact license text, copyright/notices, patent/trademark terms, redistribution obligations, and intended-distribution analysis;
4. dependency, bundled-component, native-code, WASM, install/build-script, dynamic-import, and generated-output inventory;
5. capability review covering network, filesystem, process, shell, FFI, device, credential, update, and telemetry behavior;
6. maintenance, security history, ownership, end-of-life, and supply-chain risk review;
7. development-only reachability proof and final packaged-artifact exclusion plan;
8. owner, independent reviewer, decision, date, approved scope, and verification IDs.

An unknown field leaves the entry `BLOCKED` or `PROVISIONAL_UNREVIEWED`. Presence in this register, a local version match, a pinned commit, a lockfile, or a successful governance run is never approval.

## 7. Change and review procedure

Any version, path, runner image, action commit, registry, feature, component, target, script, or network-use change requires updating this record before use. The reviewer must compare the actual resolved development graph and generated output with this admission scope.

This register cannot authorize a production network/device/process/FFI capability, a trusted-core native bridge, a physical-industrial tool, or a packaging decision blocked by OQ-0001. Those matters follow the mandatory stop rules in `PES-DEC-0002` and the immutable exclusion in ADR-0001.
