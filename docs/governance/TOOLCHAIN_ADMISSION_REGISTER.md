# Phase 1 Toolchain Admission Register

Status: **FOUNDATION IMPLEMENTED; LOCAL GATES OBSERVED; CI ACTIVE BUT UNEXECUTED; ALL TOOLS UNAPPROVED**
Inventory date: 2026-08-27  
Scope: Phase 1 development, governance extraction, verification, and CI tooling only  
Production reachability: Prohibited

Governing requirements: `PES-CRM-0024`, `PES-CRM-0025`, `PES-ISO-0012`-`PES-ISO-0015`, `PES-SEC-0004`-`PES-SEC-0008`, `PES-DEV-0006`-`PES-DEV-0009`, and `PES-CI-0001`-`PES-CI-0003`.

## 1. Purpose and truth statement

This register inventories the development/build tools currently declared or directly observed for the Phase 1 governance repository. It is the initial admission record required by `DEPENDENCY_POLICY.md`; it is not a license opinion, supply-chain approval, security approval, or release authorization.

**No tool in this register is `APPROVED`.** Every entry remains provisional and unreviewed until an identified reviewer completes the policy's license, provenance, capability, integrity, maintenance, reproducibility, production-reachability, and evidence requirements. Unknown facts remain `UNKNOWN`; a local version string or executable hash does not prove upstream provenance or license compliance.

The Corrective Addendum authorized the exact local dependency acquisition and foundation implementation recorded in `DEPENDENCY_POLICY.md` Section 5.2. That scoped authorization does not mark a tool or package approved and does not create release eligibility. Executables, package managers, compilers, test runners, credentials, and development-service capabilities must not enter the classroom artifact. Generated output receives independent review.

## 2. Local observation method

The initial inventory used read-only command resolution, version, manifest, and hash inspection. The Corrective Addendum run then acquired the exact lockfile-bound npm packages and Rust components recorded below. Registry integrity fields and local package metadata are evidence, not proof of upstream identity, signatures, source correspondence, vulnerability status, or license approval. No GitHub-hosted workflow execution has been observed in this record.

Paths under `C:\Users\Scott\.cache\codex-runtimes\` are workspace-provided development dependencies, not repository-owned or production-approved files. Paths under `C:\Program Files`, `AppData`, `.rustup`, and `.cargo` are host-local observations and are not reproducible repository declarations.

## 3. Inventory summary

| Tool | Declared or intended version/reference | Locally observed version | Scope | Current disposition |
|---|---|---|---|---|
| Node.js | Exact `24.19.0` in `package.json` | Bundled runtime `v24.19.0`; ordinary PATH runtime `v24.14.0` | Development verification scripts | `PROVISIONAL_UNREVIEWED`; exact version is fail-closed, but executable identity/provenance remains unapproved |
| pnpm | Exact `11.19.0` in `packageManager` and `engines.pnpm` | `11.19.0` through bundled wrapper | Development package/workspace management | `PROVISIONAL_UNREVIEWED` |
| Python | Exact `3.13.12` in `.python-version`; the launcher and extractor both reject other versions | PATH runtime `3.13.12`; bundled alternative `3.12.13` | Development-only directive extraction | `PROVISIONAL_UNREVIEWED`; exact version is fail-closed, but executable identity/provenance remains unapproved |
| Rust compiler | `1.94.0`; `clippy`, `rustfmt`, `wasm32-unknown-unknown` | `rustc 1.94.0 (4a4ef493e 2026-03-02)` | Compiles the dependency-free first-party health crate to WASM | `PROVISIONAL_UNREVIEWED` |
| Cargo | Coupled to Rust toolchain `1.94.0` | `cargo 1.94.0 (85eff7c80 2026-01-15)` | Builds/tests the one-member Rust workspace; no registry crates | `PROVISIONAL_UNREVIEWED` |
| TypeScript | Exact `6.0.3` in the workspace catalog | Locally executed through pnpm | Strict source/test type checking | `CANDIDATE_UNREVIEWED`; development only |
| Vite | Exact `8.2.2` | Locally executed through pnpm | Production bundle generation only; no server used | `CANDIDATE_UNREVIEWED`; development only |
| Vitest | Exact `4.1.10` | Locally executed through pnpm | Contract and state-model unit tests | `CANDIDATE_UNREVIEWED`; development only |
| Playwright Core | Exact `1.62.1` | Locally executed against Chrome `151.0.7922.174` | `file://` browser interaction/isolation evidence | `CANDIDATE_UNREVIEWED`; development only |
| System Chrome | Host `151.0.7922.174` | `C:\Program Files\Google\Chrome\Application\chrome.exe` | Headless local browser test target; not a packaging choice | `PROVISIONAL_UNREVIEWED` |
| GitHub-hosted runner | Active mutable label `windows-2025` | No exact image revision, OS build, or preinstalled-tool manifest is pinned locally | Active CI declaration; no run observed | `ACTIVE_UNEXECUTED_UNREVIEWED` |
| `actions/checkout` | Proposed commit `3d3c42e5aac5ba805825da76410c181273ba90b1` | Workflow declaration only; source/tag/provenance not verified | Proposed remote checkout | `BLOCKED_DEC_0002_AND_ADMISSION` |
| `actions/setup-node` | Proposed commit `820762786026740c76f36085b0efc47a31fe5020` | Workflow declaration only; source/tag/provenance not verified | Proposed remote Node `24.19.0` setup | `BLOCKED_DEC_0002_AND_ADMISSION` |
| `actions/setup-python` | Proposed commit `5fda3b95a4ea91299a34e894583c3862153e4b97` | Workflow declaration only; source/tag/provenance not verified | Proposed remote Python `3.13.12` setup | `BLOCKED_DEC_0002_AND_ADMISSION` |
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
| Requested purpose | Manage the Phase 1 pnpm workspace; build, typecheck, test, and verify the foundation and governance records |
| Dependency class | Development package manager; must not ship |
| Repository declaration | `packageManager: pnpm@11.19.0`; `engines.pnpm` exact version `11.19.0` |
| Observed executable | Bundled `pnpm.cmd` wrapper reports `11.19.0` and invokes the bundled Node plus `pnpm.mjs` |
| Wrapper SHA-256 | `1B93F82A7506B0F644F5ADC64A77470351D589DCBE4291D621147B72827F1D96` |
| Local package metadata | Bundled `pnpm/package.json` identifies version `11.19.0`, claims `MIT`, and has SHA-256 `862525B82C79860ED1A196DD2E08D1543A444FC9118474A5C4F581644290D892` |
| Local license evidence | A bundled `LICENSE` file exists, but its copyright, obligations, and correspondence to the exact resolved tool have not been reviewed |
| Observed configured registry | `https://registry.npmjs.org/`; observation is not registry approval |
| Network/process capability | Exact lockfile-bound acquisition was authorized for the corrective run; lifecycle scripts were disabled. Future registry changes still require review. |
| Upstream provenance/signature | `UNKNOWN`; package-manager acquisition chain and authoritative upstream integrity were not verified |
| Production reachability | Forbidden; package manager, cache, registry configuration, and lifecycle scripts must not ship |
| Reviewer/decision/date | `UNASSIGNED` / `NOT_REVIEWED` / `null` |
| Blockers | Review authoritative source, exact license/notices, bundled dependency graph, acquisition/integrity chain, lifecycle behavior, security history, registry policy, and clean production-bundle exclusion |

### TC-0003 — Python directive-extractor runtime

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Execute `tools/phase1/extract_directive_requirements.py` to read the controlled DOCX and regenerate governance JSON |
| Dependency class | Development extraction runtime; must not ship |
| Repository declaration | `.python-version` pins `3.13.12`; `package.json` invokes `tools/phase1/run_pinned_python.mjs`, which selects and verifies the exact Python runtime before extraction or verification |
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
| Requested purpose | Compile the Phase 1 dependency-free health function to capability-limited WebAssembly; no PLC semantics are present |
| Dependency class | Development compiler/toolchain; compiler must not ship |
| Repository declaration | `rust-toolchain.toml` channel `1.94.0`; `Cargo.toml` `rust-version = 1.94.0` |
| Observed executable | `C:\Users\Scott\.rustup\toolchains\1.94.0-x86_64-pc-windows-msvc\bin\rustc.exe` |
| Version evidence | `rustc 1.94.0 (4a4ef493e 2026-03-02)` |
| Executable SHA-256 | `6A0699E427EE9C1492EF1C9EA967D035DC4660E92C7FE32F2C6A1038116700E5` |
| Target/feature scope | Exact `wasm32-unknown-unknown` target plus `clippy` and `rustfmt`; generated module is 247 bytes with zero imports and three required exports (`memory`, `foundation_health`, `foundation_health_len`) |
| Local license evidence | `UNKNOWN`; toolchain license/notices were not located and reviewed during this audit |
| Upstream source/signature | `UNKNOWN`; rustup acquisition, manifest signatures/checksums, and upstream source correspondence were not verified |
| Network/process capability | Compiler may access files and invoke toolchain components in development; no permission extends to generated production capability |
| Production reachability | Compiler forbidden from shipment; every generated WASM import table and artifact remains separately reviewable |
| Reviewer/decision/date | `UNASSIGNED` / `NOT_REVIEWED` / `null` |
| Blockers | Complete license, rustup/toolchain provenance, checksum/signature, component/target, security, reproducibility, and generated-artifact capability review |

### TC-0005 — Cargo 1.94.0

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Build and test the one-member `foundation-wasm` workspace without registry crates |
| Dependency class | Development build/package tool; must not ship |
| Repository declaration | Coupled to the exact `1.94.0` Rust toolchain; `Cargo.lock` resolves only first-party `foundation-wasm 0.0.0` |
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
| Requested purpose | Execute the active Phase 1 closure gate declared by the Corrective Addendum; no hosted run is evidenced here |
| Dependency class | External development CI environment; never a classroom runtime dependency |
| Repository declaration | `.github/workflows/phase1-governance.yml` uses `runs-on: windows-2025` |
| Exact image/version | `UNKNOWN`; `windows-2025` is a mutable image label and does not pin its current image revision, OS build, or installed-tool manifest. The active workflow reports `ImageOS` and `ImageVersion`; no completed run evidence is admitted here |
| Local integrity evidence | None; the hosted runner image is not present locally and was not hashed or inspected |
| Service/license/terms evidence | `UNKNOWN` and `NOT_REVIEWED` |
| Network/credential capability | If triggered, the hosted runner uses external GitHub, runtime-distribution, Rust, and npm services. The workflow declares repository contents read-only and checkout credential persistence false. Service behavior remains unreviewed. |
| Production reachability | Forbidden; no runner image, token, service dependency, or hosted-network assumption may enter the product |
| Reviewer/decision/date | `UNASSIGNED` / `NOT_REVIEWED` / `null` |
| Blockers | Replace or supplement the mutable label with an approved reproducibility strategy; record exact image/tool inventory per run, service/terms/data/credential assumptions, supply-chain controls, and evidence retention |

### TC-0007 — `actions/checkout` pinned commit

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Repository checkout in the active but not-yet-observed closure workflow |
| Dependency class | Third-party development CI action; must not ship |
| Repository declaration | `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1` |
| Version/tag claim | `UNKNOWN`; no tag mapping is asserted or treated as evidence |
| Local source/hash evidence | None; only the 40-character commit reference is present locally; action source and resolved bundle were not vendored or inspected |
| License/provenance/signature | `UNKNOWN` and `NOT_REVIEWED` |
| Network/credential capability | If triggered, executes within GitHub-hosted CI and accesses the repository service; the workflow requests read-only contents and sets `persist-credentials: false` |
| Production reachability | Forbidden; action code, Node runtime, service tokens, and checkout metadata must not ship |
| Reviewer/decision/date | `UNASSIGNED` / `NOT_REVIEWED` / `null` |
| Blockers | Verify commit ownership/source, source-to-bundle correspondence, license/notices, action dependency graph, security history, credential behavior, and retained workflow evidence |

### TC-0008 — `actions/setup-node` pinned commit

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Install exact Node.js `24.19.0` in the active but not-yet-observed closure workflow |
| Dependency class | Third-party development CI action; must not ship |
| Repository declaration | `actions/setup-node@820762786026740c76f36085b0efc47a31fe5020` with `node-version: 24.19.0` and `package-manager-cache: false` |
| Version/tag claim | `UNKNOWN`; no tag mapping is asserted or treated as evidence |
| Local source/hash evidence | None; only the 40-character commit reference is present locally. Action source, resolved distribution bundle, transitive dependencies, and downloaded Node payload were not vendored or inspected locally |
| License/provenance/signature | Repository ownership, exact license obligations, source-to-bundle correspondence, commit/signature chain, release attestations, and downloaded Node provenance remain `UNKNOWN` and `NOT_REVIEWED` |
| Cache behavior | Workflow explicitly sets `package-manager-cache: false`; this does not disable action download, Node acquisition, general runner networking, or unreviewed behavior in the action bundle |
| Network/credential capability | If triggered, may access GitHub and Node distribution services. Endpoints, redirects, integrity verification, credentials, and retained evidence remain unreviewed. |
| Production reachability | Forbidden; action code, downloaded installer/cache material, service tokens, runner metadata, and setup behavior must not ship |
| Reviewer/decision/date | `UNASSIGNED` / `NOT_REVIEWED` / `null` |
| Blockers | Resolve the remaining external-operation boundary of `DEC-0002`; establish durable authoritative provenance; review exact license/notices, source-to-bundle correspondence, action dependency graph, Node download endpoint and integrity/signature behavior, credential behavior, security history, runner evidence, and packaged-artifact exclusion |

### TC-0009 — `actions/setup-python` pinned commit

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Install exact Python `3.13.12` in the active but not-yet-observed closure workflow |
| Dependency class | Third-party development CI action; must not ship |
| Repository declaration | `actions/setup-python@5fda3b95a4ea91299a34e894583c3862153e4b97` with `python-version: 3.13.12` |
| Version/tag claim | `UNKNOWN`; no tag mapping is asserted or treated as evidence |
| Local source/hash evidence | None; action source, resolved bundle, transitive dependencies, and downloaded Python payload were not vendored or inspected locally |
| License/provenance/signature | `UNKNOWN` and `NOT_REVIEWED`; the workflow declaration does not prove repository ownership or provenance |
| Network/credential capability | If triggered, may access GitHub and Python distribution services; exact endpoint and cache behavior remain unreviewed |
| Production reachability | Forbidden; action code, interpreter, downloaded material, caches, tokens, and setup behavior must not ship |
| Reviewer/decision/date | `UNASSIGNED` / `NOT_REVIEWED` / `null` |
| Blockers | Review license/notices, source-to-bundle correspondence, dependencies, acquisition integrity/signatures, endpoint behavior, security history, credentials, retained evidence, and packaged-artifact exclusion |

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

### TC-0014 — TypeScript 6.0.3

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Strict type checking for the contract, React shell, Worker, and tests |
| Exact declaration/integrity | Workspace catalog `6.0.3`; `sha512-y2TvuxSZPDyQakkFRPZHKFm+KKVqIisdg9/CZwm9ftvKXLP8NRWj38/ODjNbr43SsoXqNuAisEf1GdCxqWcdBw==` |
| License evidence | Installed package metadata claims Apache-2.0; exact notice and source correspondence are not independently reviewed |
| Capability/containment | General compiler and filesystem tool; development only; static isolation proves it is absent from `dist/index.html` |
| Reviewer/decision/date | `UNASSIGNED` / `CANDIDATE_UNREVIEWED` / `null` |

### TC-0015 — Vite 8.2.2

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Bundle the foundation into staging assets consumed by the reviewed single-file inliner; no development or preview server is used |
| Exact declaration/integrity | Workspace catalog `8.2.2`; `sha512-cFKLV/PRgAUlIRm5WjMjJ86jrftzpqcgH+Us+DS8mI3CDNiH30Whrz8uHL3+MOLPAgqbMBAqWdAHAphOAM+z/Q==` |
| License evidence | Installed package metadata claims MIT; transitive and optional native build graph is lockfile-captured but not independently approved |
| Capability/containment | Development bundler with process/filesystem capability; installation ran with `--ignore-scripts`; excluded from the single production file |
| Reviewer/decision/date | `UNASSIGNED` / `CANDIDATE_UNREVIEWED` / `null` |

### TC-0016 — Vitest 4.1.10

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Run deterministic contract and UI-state unit tests without a browser server |
| Exact declaration/integrity | Workspace catalog `4.1.10`; `sha512-R9jUTe5S4Qb0HCd4TNqpC7oGcrMssMRGXLW80ubjWsW9VH5GF8y1Y0SFLY9AbqSk6nt0PnOx4H4WNJYZ13GUPw==` |
| License evidence | Installed package metadata claims MIT; exact dependency notices remain unreviewed |
| Capability/containment | Development test runner; excluded from production output |
| Reviewer/decision/date | `UNASSIGNED` / `CANDIDATE_UNREVIEWED` / `null` |

### TC-0017 — Playwright Core 1.62.1 and system Chrome 151.0.7922.174

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Launch a pre-existing system Chrome executable headlessly against the built `file://` artifact for interaction, responsive, deterministic-repeat, and page-request evidence |
| Exact package declaration/integrity | Root `playwright-core 1.62.1`; `sha512-wPYSwEBJY9GHraISXqyqtx0na0LpO3XEX7jNDhntbex7tzUS7kLnZsOlFruFJB4Hi/rhDMjXGqHewDZ68nYZVw==`; package has zero dependencies |
| Browser observation | `C:\Program Files\Google\Chrome\Application\chrome.exe`; file/product version `151.0.7922.174` |
| License evidence | Playwright package metadata claims Apache-2.0; Chrome distribution/license, provenance, updater/background behavior, and packaging suitability remain unreviewed |
| Capability/containment | Browser automation has host/process capability; development test only. The harness uses `offline: true`, background-network suppression flags, and records zero page-level remote requests; this is not process-scoped syscall/packet proof. |
| Reviewer/decision/date | `UNASSIGNED` / `CANDIDATE_UNREVIEWED` / `null` |

### TC-0018 — React production candidates

| Field | Current evidence or disposition |
|---|---|
| Requested purpose | Local deterministic presentation and document binding only |
| Exact graph | `react 19.2.8` and `react-dom 19.2.8` direct; `scheduler 0.27.0` transitive; exact integrity values are recorded in `DEPENDENCY_POLICY.md` Section 5.2 and `pnpm-lock.yaml` |
| License evidence | Installed package metadata claims MIT for all three; authoritative source, notices, security history, and redistribution review remain incomplete |
| Capability/containment | Reachable in `dist/index.html`; static and browser gates reject remote resources and page requests. React diagnostic and W3C namespace URIs are inert-string allowlist entries, not fetched resources. |
| Asset disposition | No React package asset file, third-party icon, font, image, or media is shipped; `ASSET_PROVENANCE.json` remains a zero-entry register |
| Reviewer/decision/date | `UNASSIGNED` / `CANDIDATE_UNREVIEWED` / `null` |

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
