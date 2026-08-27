# Task 7 — Repository Scope and Artifact Inventory

- **Audit root:** `C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC`
- **Snapshot:** 2026-08-27T15:06:49.4251608-05:00
- **Evidence boundary:** raw filesystem entries, raw file bytes, Git plumbing/status, and direct source/manifest inspection. The supplied DOCX and frozen research report are authoritative inputs. Registry, matrix, report, verifier-result, and visual-analysis claims were **not** used as proof of their own correctness.
- **Excluded:** `.git` internals, as directed. Git commands were used only to establish repository and ignore state.
- **Write boundary:** no repository file was created, edited, deleted, executed as a project task, or regenerated. This report is the only write and is outside the repository.

## Result

| Question | Result | Raw basis |
|---|---|---|
| Every repository file enumerated? | **YES** | 93 filesystem files; 15,692,126 bytes. PowerShell and `rg --files -uu` independently returned the same set with zero symmetric difference. |
| Phase 2 or product implementation present? | **NO** | No product root, UI/runtime entrypoint, product-language source, industrial project/source file, executable/native binary, WASM, or product media/asset tree exists. The only executable text/bytecode files are three Phase 1 governance tools, two ignored visual-QA helpers, and one ignored cache file. |
| Product dependencies installed or locked? | **NO** | No JavaScript dependency fields; empty pnpm root importer; no pnpm package/snapshot sections; empty Cargo workspace; zero Cargo package blocks; no product Python manifest; `node_modules` has two metadata files and no package directory. |
| Dependency references of any kind present? | **YES** | Exact toolchain pins; four remote GitHub Action commit references in a disabled job; two ignored QA helpers import undeclared `pdfplumber` and Pillow/PIL. |
| Runtime scaffolding present? | **NO product runtime scaffold. YES repository/toolchain scaffold.** | `package.json`, `pnpm-workspace.yaml`, `Cargo.toml`, toolchain pins, empty locks, Phase 1 scripts, and a disabled proposed CI workflow exist. The future workspace patterns `apps/*` and `packages/*` resolve to no directories. |
| Generated material present? | **YES** | 47 ignored verification/render outputs; two ignored pnpm metadata files; one ignored `.pyc`; two non-ignored deterministic governance snapshots; and two non-ignored empty generated lockfiles. |
| Product-scope leak present? | **NO** | No product code, product assets, vendor project file, connector/protocol implementation, runtime package, executable, or release/build output was found. |
| Repository-state or hygiene qualifications? | **YES** | There is no `HEAD`, no remote, and zero tracked files: 41 files are non-ignored but untracked, and 52 are ignored. Two source inputs have Windows `Zone.Identifier` alternate streams. Every visible item has a OneDrive reparse attribute, but none is a symlink or junction. |

The defensible conclusion is **“no Phase 2/product scope leak, but not artifact-free.”** Phase 1 governance/tooling scaffolding and local generated evidence/residue exist and are itemized below.

## Substantive findings

### T7-01 — No Phase 2 or product implementation

The expected product/application roots are absent:

```text
apps                  False
packages              False
profiles              False
scenarios             False
assets                False
assets/original       False
src                   False
public                False
dist                  False
build                 False
target                False
coverage              False
artifacts             False
test-results          False
playwright-report      False
.pnpm-store            False
```

No product application source or entrypoint exists. No file matches TypeScript/TSX/JSX, Rust, C/C++, C#, Java/Kotlin, Swift, Go, HTML/CSS, Vue/Svelte, WASM, executable/native, or common industrial project/source formats. The sole `.mjs` file is the Phase 1 verifier.

Complete executable-text/bytecode census:

```text
EXECUTABLE_TEXT_OR_BYTECODE_COUNT=6
IGN_QA_HELPER     .phase1-verification/docx-visual/analyze_pdf.py
IGN_QA_HELPER     .phase1-verification/docx-visual/make_contact_sheets.py
IGN_PYC           tools/phase1/__pycache__/extract_directive_requirements.cpython-313.pyc
P1_TOOL           tools/phase1/extract_directive_requirements.py
P1_TOOL           tools/phase1/run_phase1_verification.py
P1_TOOL           tools/phase1/verify-phase1.mjs
UNCLASSIFIED_EXECUTABLE_COUNT=0
```

The controlled JavaScript imports only `node:crypto`, `node:child_process`, `node:fs`, `node:path`, and `node:url`. The controlled Python files import only standard-library modules. Their local process spawning is governance orchestration, not a product runtime. Future-feature/protocol/Phase-2 words inside source documents, policies, requirements, ADRs, registries, matrices, and threat-model prose are not executable implementation.

### T7-02 — Repository/toolchain scaffolding exists; product runtime scaffolding does not

Present bootstrap infrastructure:

- Node 24.19.0, pnpm 11.19.0, Python 3.13.12, Rust 1.94.0 declarations;
- private governance package with only `test`, `verify:phase1`, `requirements:extract`, and `requirements:check`;
- future globs `apps/*` and `packages/*`, with neither directory present;
- empty pnpm and Cargo locks;
- three Phase 1 governance tools and one policy-contract test-data file;
- one proposed workflow whose single job is disabled by literal `if: ${{ false }}`.

This is Phase 1 repository/toolchain scaffolding, not an app shell, simulator runtime, UI, backend, PLC engine, desktop shell, native core, product package, or packaging target.

### T7-03 — Dependency truth

```text
package.json:
dependencies=ABSENT
devDependencies=ABSENT
optionalDependencies=ABSENT
peerDependencies=ABSENT
bundledDependencies=ABSENT

pnpm-lock.yaml:
PNPM_EMPTY_ROOT_IMPORTER=1
PNPM_PACKAGES_SECTIONS=0
PNPM_SNAPSHOTS_SECTIONS=0

Cargo:
CARGO_WORKSPACE_MEMBERS_LINE=members = []
CARGO_PACKAGE_BLOCKS=0

node_modules:
file count=2
.package-map.json -> workspace self-reference only
.pnpm-workspace-state-v1.json -> workspace/settings metadata
package directories=0
```

Therefore the repository has zero resolved third-party JavaScript packages and zero Rust packages. It has no Python dependency manifest.

Two qualifications:

1. The disabled CI proposal names four remote Action dependencies by full commit:

   - `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1`
   - `actions/setup-node@820762786026740c76f36085b0efc47a31fe5020`
   - `actions/setup-python@5fda3b95a4ea91299a34e894583c3862153e4b97`
   - `actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a`

   The raw workflow is `workflow_dispatch` only, has one literal false job gate, and targets `windows-2025`. These are proposed infrastructure references, not installed repository packages and not evidence of execution or approval.

2. The ignored visual-QA helpers import `pdfplumber` and `PIL`/Pillow. Neither is in a repository manifest. Probing the exact PATH Python 3.13.12 returned:

   ```text
   pdfplumber=NOT_AVAILABLE_IN_DECLARED_PATH_RUNTIME
   PIL=NOT_AVAILABLE_IN_DECLARED_PATH_RUNTIME
   ```

   This is an **ignored local-QA reproducibility gap**, not a product dependency or Phase 2 implementation. It does not prove which external runtime originally produced the existing renders.

### T7-04 — Generated and ignored material exists

The 52 ignored files total 14,267,908 bytes:

| Ignored class | Files | Bytes | Contents |
|---|---:|---:|---|
| Verification outputs | 47 | 14,235,017 | 40 full-page PNGs, four contact-sheet PNGs, one rendered PDF, `visual-analysis.json`, and `phase1-report.json` |
| Local QA helper source | 2 | 4,043 | PDF analysis and contact-sheet scripts |
| pnpm metadata | 2 | 1,093 | package-map and workspace-state metadata; no packages |
| Python bytecode cache | 1 | 27,755 | compiled extractor cache |

The two ignored helpers are artifacts in the broad filesystem sense, not generated output themselves. Thus 50 ignored files are generated output/residue and two are ignored helper source.

Four non-ignored files are generated-format material:

- `requirements/phase1-requirements.json` and `IMPLEMENTATION_MATRIX.json` are the two output paths named by the extractor's raw output map;
- `pnpm-lock.yaml` is an empty generated pnpm lock;
- `Cargo.lock` self-identifies as automatically generated and has no package blocks.

This audit does not rely on the PASS/VERIFIED content of any generated report, registry, matrix, or visual-analysis file.

### T7-05 — No product assets, binaries, vendor projects, or release output

```text
PE_MZ_COUNT=0
ELF_COUNT=0
WASM_COUNT=0
PDF_COUNT=1
PNG_COUNT=44
ZIP_COUNT=1

INDUSTRIAL_PROJECT_OR_SOURCE_COUNT=0
EXECUTABLE_OR_NATIVE_COUNT=0
WEB_UI_COUNT=0
BRANDING_MEDIA_OR_FONT_COUNT=0
PACKAGE_ARCHIVE_COUNT=0
IMAGES_OUTSIDE_PHASE1_VERIFICATION=0
```

The sole ZIP-signature file is the supplied DOCX container. The sole PDF and all 44 PNGs are under ignored `.phase1-verification/docx-visual/`. There are no fonts, product/brand assets, vendor binaries, sample PLC projects, compiled outputs, installer packages, release archives, or application media.

### T7-06 — Git and filesystem qualifications

```text
GIT_TOP=C:/Users/Scott/OneDrive/Desktop/Codex - GOV's PLC
HEAD_COUNT=fatal: ambiguous argument 'HEAD': unknown revision or path not in the working tree.
REMOTES=none
TRACKED_COUNT=0
STATUS_ENTRIES=93
UNTRACKED=41
IGNORED=52
OTHER=0
```

Accordingly, `??` below means non-ignored/untracked and `!!` means ignored. Nothing is tracked or committed.

PowerShell reports the OneDrive `ReparsePoint` attribute on all 110 visible items (93 files and 17 directories), while `LinkType`/target inspection finds zero symlinks and zero junctions. Recursive enumeration did not traverse an external linked tree.

Two NTFS alternate data streams exist and are not counted as separate files:

```text
Govs PLC project Research Report.md                                  Zone.Identifier  56 bytes
PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx  Zone.Identifier  378 bytes
```

They are Windows origin metadata, not product content. Their contents were not required or exposed.

## Inventory class totals

| Role | Files | Bytes |
|---|---:|---:|
| `SOURCE` | 2 | 208,177 |
| `P1_DOC` | 21 | 590,038 |
| `P1_MACHINE` | 2 | 30,702 |
| `P1_GENERATED_SNAPSHOT` | 2 | 863,102 |
| `P1_TOOL` | 3 | 83,683 |
| `P1_TEST_DATA` | 1 | 7,469 |
| `REPO_CONFIG` | 8 | 2,166 |
| `EMPTY_LOCK` | 2 | 217 |
| `DISABLED_CI` | 1 | 2,452 |
| `IGN_QA_HELPER` | 2 | 4,043 |
| `IGN_VERIFY_OUTPUT` | 47 | 14,235,017 |
| `IGN_PNPM_META` | 2 | 1,093 |
| `IGN_PYC` | 1 | 27,755 |
| **Total** | **94** | **16,055,914** |

Legend:

- `SOURCE`: supplied directive or frozen research.
- `P1_DOC`, `P1_MACHINE`: Phase 1 governance/documentation data.
- `P1_GENERATED_SNAPSHOT`: non-ignored deterministic governance output.
- `P1_TOOL`, `P1_TEST_DATA`: Phase 1 governance tooling/test data.
- `REPO_CONFIG`, `EMPTY_LOCK`: repository/toolchain bootstrap or empty lock.
- `DISABLED_CI`: proposed workflow with literal false job gate.
- `IGN_*`: ignored helper, output, metadata, or cache.

## Complete byte-level file inventory

SHA-256 is over each raw file's bytes. Git status is from `git status --porcelain=v1 --ignored --untracked-files=all`.

| Path | Bytes | SHA-256 | Git | Role |
|---|---:|---|---:|---|
| `.editorconfig` | 213 | `8e608db783a129601ce441d08bdde9a05dff1811f9e5c011681c9a2eef9e70a5` | `??` | `REPO_CONFIG` |
| `.gitattributes` | 425 | `d51465fdceda1ac0ebadaf1abca6f5a362f55991c3a29a0269cffdacd0c230ff` | `??` | `REPO_CONFIG` |
| `.github/workflows/phase1-governance.yml` | 2,452 | `dea8e72c5682caf67ba0a6395503c38394bd0ab8f635ae29f5f57e6aae5ff158` | `??` | `DISABLED_CI` |
| `.gitignore` | 530 | `12031969a664c6ee63537e39fead9087c5f62ec9e959adb33378f0384dac5b7e` | `??` | `REPO_CONFIG` |
| `.phase1-verification/docx-visual/analyze_pdf.py` | 2,900 | `7d3a7f4b3f3e48dfba9b92770c6a67f0e35e41bcd37636f8846580a979891670` | `!!` | `IGN_QA_HELPER` |
| `.phase1-verification/docx-visual/contact/contact-01.png` | 1,265,625 | `6c0c44a20795c02794c16c0b92c06353c2ba87e2ed45dc338122156729529452` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/contact/contact-02.png` | 1,392,943 | `1e7f9516ce6f395b897c69c22651067c9064755de96e541011a884e9a52bc18f` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/contact/contact-03.png` | 1,177,350 | `862232fd73841efd652f3f3ea6dad5be73d14f2e8e08972495a93bb0067adb8f` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/contact/contact-04.png` | 1,119,215 | `bac405070c3a56334f41137e971ba87e4670cbe68492df286e08bf7460a8bf24` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/make_contact_sheets.py` | 1,143 | `b92c5939c083fa07e33c2cb05e70ba7fa4a4a823a410f8030a2fa12d589c5619` | `!!` | `IGN_QA_HELPER` |
| `.phase1-verification/docx-visual/pages/page-01.png` | 102,036 | `ed76fcb4ad284e97ed90389104da44b787e7b84756362fcd8edd3861803c9d9d` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-02.png` | 191,372 | `4d41743c69673c82df1dfae63dba4eafa2edf66b5c9340f63aa617ef88bd12f5` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-03.png` | 228,483 | `f0234a80210301a1b69640ffead50d4fec529ad863e3baa687ec90b5ef0f2ce4` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-04.png` | 221,451 | `dbd9a7b44985c3c480b85eba48e54542bf8c802324c06e11d74ee3c71681ac0e` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-05.png` | 235,287 | `fb1fc0248bceb6c49f1149151d07b2b0d5fe7c533e05a7e727c5361d9ad06d14` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-06.png` | 217,507 | `f73d121dbd96ad4816783fd180dc35e96ece9a90abf1b9e730206f544d86dd1a` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-07.png` | 244,397 | `ed912e101e03b69b9231407c7567cc9b34ff39f91880d46001c5ae00432fbe9b` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-08.png` | 278,441 | `c8a166ea943604f7d2f0a8a9c1e7e9fc103fb21d6eea3e5ddf7585c981f0ec54` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-09.png` | 220,399 | `fadfc8185c3bc304bb432926fd49bd2b6a23e2044e1334fd96d022c065ccf4ce` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-10.png` | 236,399 | `f0ab7c8513fb07da9b99afc1619a7ce8f7284b5d381763f79e155614003a1b99` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-11.png` | 270,018 | `7d963ff710c5c103dc395d90770ae4c3623a845b7cfeceeec4079566a9606b17` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-12.png` | 213,803 | `8117433af7c9fc4a79073b4416a9aef429d56b68940e36062ca1306e49bbcf76` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-13.png` | 243,936 | `2f5ed2643b221110619c7be99e90b88c42930d94cf5d13b7c61c54a34211943c` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-14.png` | 261,778 | `3ba68d1279de2c7fddd3860aa7da2c33440313cbeec8f3be2eac4bd36318ac00` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-15.png` | 264,370 | `7c61dad08cee5567b9aa5491028cb8e22f67c66e0185dab7c661eef856123287` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-16.png` | 222,719 | `d47c1258ceeb6a318647d365d5a5f7d7243690fc0c801ff7a5107a157afc813b` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-17.png` | 226,538 | `e1cb3e7c1bbfb71a54b7ab5e4aab1f8618f6d70aaa8cbc5447906eb3e0913f0d` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-18.png` | 185,439 | `abd6a6d79e0948ca2dfbf694297d25c5b2c4c68564999842ee7ba985a603d9d6` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-19.png` | 250,269 | `c8af7018c5e5194f88ebb34d98a7c310ac9d56d0c684c2e234d8b45d7fd72243` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-20.png` | 266,257 | `252330652a6c62520bf7988570c8838806c6b29475f4d0148dd3bf221532dd31` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-21.png` | 237,494 | `782001349f32725be98c0f24e0b58cc7503ac0321015ecd6f0c6ba0e7e4fab44` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-22.png` | 239,156 | `0d330e2d82ecc568ccfdc69b95b52eeba07e0bfdd4ba1b47e1d3a42af4d53629` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-23.png` | 231,467 | `f9dd757d4e3ada10e91e1088ff919c0902e6c26d7b4e56c814c6bbb46fbda219` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-24.png` | 189,775 | `aab17cb04ac6e925acad8d3062cecfae822b202f58c7b9722318d05528b455f9` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-25.png` | 156,449 | `8833688cbb143b36f681dc60432feccdaf0634735f63b206641ba1ea3c6130e4` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-26.png` | 254,430 | `4836888c11bd9eb179a0e75dfbc29084936c4eb811f498cef1b6a53f1e0a6847` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-27.png` | 251,382 | `b4578fd21c2768a0dfba265e9ece18f517f3ebab7572e2bb2537ec8e6b2fcc56` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-28.png` | 163,315 | `5b2af3dd32487e3688196bdd1df6e9b839bfb13c1b7a4f83d6a26cb1a76784c5` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-29.png` | 199,831 | `897d2d03bdf77274909792c2ec8d51e8b8449fb9ed063f6122b707fc5a8763c8` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-30.png` | 158,721 | `fd1c17a152bd2cd41f1dd4ba446e51f10cf04894e8409c9583d98444c5d5fc11` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-31.png` | 205,247 | `c55813867a9d2b9636179a9ee7e9dce64303f799a9ebb31f2e74785f03de0c97` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-32.png` | 217,388 | `f7ffcf76ce0dfa40636d1c062a2e62b016d636beb3483690ab0d2d0e78e08597` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-33.png` | 189,294 | `84779d16ba16dccf4122f91395f3f2265b66b8c69a0eb7b10fb2985964160baf` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-34.png` | 159,488 | `2878a20796c1edeb0f76cdec19c34322beef7e3cf2c89d8e147ebf3e3ad0ccc8` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-35.png` | 197,164 | `6bf80d3e227689f0d02629a2349cc5fe51846431e1e8563cc909ae840eb5736a` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-36.png` | 226,817 | `f0d3b4c1dffdab1d1256e5deafa00bd51b9aa1edd837c7fc9a22abc3f6fccdae` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-37.png` | 127,813 | `60c2f19c7f53728d36adf381bd2a9a217bfc2e94ea8896bceb4adb9481afeee8` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-38.png` | 240,419 | `b333850cd1a2376970715d5aae492a0c7dae6d139b873b6b892f20f2f9cd00c3` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-39.png` | 186,706 | `86be90876d0fb7f91695d557ae2e9e5fad279eefdbc4d7759547556a7ab44b71` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/pages/page-40.png` | 219,088 | `6937c579f6a873d9514b23f8d33458a1491d28dade6dd745d040999286fd3a46` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/phase1-directive-render.pdf` | 594,593 | `35dd6eb0a4cd87487b4597cddd1285d7f47101dd2ee39dc2746bcd99a8589b79` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/docx-visual/visual-analysis.json` | 22,486 | `e375d621054a4c0f584379ebb70afdd521a21f11e50e39c389e6d72677ff5a63` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.phase1-verification/phase1-report.json` | 30,462 | `6402846d9a63b308cde06402c42dbb7164a7df9a3a9907b0fc103f37512be5ff` | `!!` | `IGN_VERIFY_OUTPUT` |
| `.python-version` | 8 | `40411dc2c2726b4c7024d38318dd8df45c528b49491efb45939dd4a829847d94` | `??` | `REPO_CONFIG` |
| `ADR/0001-no-physical-industrial-communication.md` | 9,656 | `78a052ec3776cb8049dc1566f105c473f95fcc55c35dd2f93d0838f8d9aca34b` | `??` | `P1_DOC` |
| `ADR/0002-original-project-format.md` | 11,458 | `098247216f077bc4b6a42a16faa212643625654e87b11eb5c8591e941c966e4d` | `??` | `P1_DOC` |
| `ADR/0003-unified-plc-ir.md` | 11,972 | `7aa4628d3e83db346e77548a720c06f80e7cdf5ae2d2397ea5861b51faa2ad70` | `??` | `P1_DOC` |
| `ADR/0004-deterministic-virtual-time.md` | 11,853 | `6ecd2475a92d113bcaeca128be7f70550596e7c3deedf5f63051e2ce2075bb60` | `??` | `P1_DOC` |
| `ASSET_PROVENANCE.json` | 3,262 | `885f33ae6347f9f478d656c9bf3bbad3401f13f36b783bf4bcf668f731624afe` | `??` | `P1_MACHINE` |
| `Cargo.lock` | 103 | `cab3c319a062072cd280df54ff4786200bf6cfe548a2c8674c0289dfa35ae214` | `??` | `EMPTY_LOCK` |
| `Cargo.toml` | 120 | `2e81188a62e6f5a6cdf693697a528730ed825be15e186957cf45f3ad96a18d9a` | `??` | `REPO_CONFIG` |
| `CHANGELOG_DIRECTIVE.md` | 7,370 | `9e04fe0dded3c3cc23373f8a0f384829326998e25d0577a1d53e66a537e54fb2` | `??` | `P1_DOC` |
| `CLEAN_ROOM_POLICY.md` | 14,713 | `a30305a3a52d68fdecf5077bf1ff479a54e33484c42d43d138f68ef72c213693` | `??` | `P1_DOC` |
| `CONTRIBUTOR_CLEAN_ROOM_ATTESTATION.md` | 8,842 | `20b2ccdfc392fdfae03e3466cdaec7777bbdd5fedf5905bf3599995cdecd505b` | `??` | `P1_DOC` |
| `DEPENDENCY_POLICY.md` | 17,934 | `6071a19d9d84b53a09f410f2924853723807ed8d63bb7dc841c7590fc19463c9` | `??` | `P1_DOC` |
| `docs/governance/DOCX_VISUAL_QA.md` | 4,690 | `ac9772982347bbac2f1d5886c366b05b145c7d82a279b2dbe2ccaf64b2c73eb1` | `??` | `P1_DOC` |
| `docs/governance/PHASE_1_ADVERSARIAL_AUDIT.md` | 363,788 | `300deaea1d0db713316dd3e3069ec8894bb9e9ef87f8017e24a6d1aa618aeee4` | `??` | `P1_DOC` |
| `docs/governance/PHASE_1_SCOPE_AUDIT.md` | 7,080 | `368e90e7c707668f411c373c5193fdcbff4513529ea853111dd362da2bceba9d` | `??` | `P1_DOC` |
| `docs/governance/PHASE_1_VERIFICATION_PLAN.md` | 4,112 | `904703338536c9e98674044e9f898aeba64a7c88c1136a5b6fcb1102c2b9396d` | `??` | `P1_DOC` |
| `docs/governance/TOOLCHAIN_ADMISSION_REGISTER.md` | 30,065 | `9d04271beabef158b723d7a93561b30a73f096bc55b763a04ece43452d7a61d0` | `??` | `P1_DOC` |
| `docs/research/UNRESOLVED_SOURCE_TOKENS.md` | 5,831 | `f13ccd241714bd144d1f373d7f90769cc957751bf1f10b7494abf765a3314d5b` | `??` | `P1_DOC` |
| `EVIDENCE_REGISTER.json` | 27,440 | `e15078b534260e3733e6842710e8440e32bbf769b521c86d0d17a6c49b75f5f9` | `??` | `P1_MACHINE` |
| `Govs PLC project Research Report.md` | 127,132 | `f05c08323b5cc9483beb1feb3c7312ccb9a45ebe3b527e6dae069c181d3fbf55` | `??` | `SOURCE` |
| `IMPLEMENTATION_MATRIX.json` | 127,268 | `40164335374065aa40572940924bc019d4f3f0accf3f4ade1acf6aeb6fbd41dc` | `??` | `P1_GENERATED_SNAPSHOT` |
| `LEGAL_REVIEW_CHECKLIST.md` | 12,666 | `0f6bef7ef42a5286de3b5cf06d98b5d0899d625db4e29f14b2dec6cf04f93b8a` | `??` | `P1_DOC` |
| `node_modules/.package-map.json` | 92 | `dcb0f89ac8c6c75143abdaac1c18cba1a06be1c7c0d59ecf753e516c972fb657` | `!!` | `IGN_PNPM_META` |
| `node_modules/.pnpm-workspace-state-v1.json` | 1,001 | `e39538eecf11e8ac11a430c00a6828ec052d664838cbeec2f58d8ad6eb230423` | `!!` | `IGN_PNPM_META` |
| `OPEN_DECISIONS.md` | 11,173 | `d73b32efcce6e1be28fd6130fa8d32d011dec352d4f52193e85267763f51b1de` | `??` | `P1_DOC` |
| `package.json` | 670 | `38341e2c218b30d2a6c9aba08ca3e5823afda277132bbfc261e3a2a401d4bf1a` | `??` | `REPO_CONFIG` |
| `PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx` | 81,045 | `ebf074e2ceab752f09e6db63d88e100991729da13c1eb874290a6b337da72612` | `??` | `SOURCE` |
| `pnpm-lock.yaml` | 114 | `17c814b167307942d3609c7b9d916ceddb85839573ab39baa114e30edb132a1a` | `??` | `EMPTY_LOCK` |
| `pnpm-workspace.yaml` | 149 | `412e3f1c1a5209edc108815ed37cdf8364e8bbdc4a568d8e63b3c44548376609` | `??` | `REPO_CONFIG` |
| `README.md` | 3,057 | `cee4bd51b4a8a3e11ccf026f0b919ea14276f55f1537b0fc7b461e2d65cef9f4` | `??` | `P1_DOC` |
| `REQUIREMENTS.md` | 3,991 | `876725c7bc22be372ee97905ee00578a1c34af0a48705f42fa6c4fa9df07f469` | `??` | `P1_DOC` |
| `requirements/phase1-requirements.json` | 735,834 | `83a892abb0151580689d5ac111ca2529e2f3c330bb8dfc9414d404c59ca5b32b` | `??` | `P1_GENERATED_SNAPSHOT` |
| `RISK_REGISTER.md` | 3,955 | `944881f0f0905a6d24d0fd8a0bab583232d4dcdc3c14d3e1ae8f794e978ce5ba` | `??` | `P1_DOC` |
| `rust-toolchain.toml` | 51 | `dd510b4a3ab011309bb12f6fc3450596da2080670bd4f922eaff7cad1ca65794` | `??` | `REPO_CONFIG` |
| `SECURITY_INVARIANTS.md` | 23,275 | `056ffe579da7483f049a5fdc5ffdac02665ab298e4877a9f6be6bb6222109f9f` | `??` | `P1_DOC` |
| `tests/phase1/policy-contract.json` | 7,469 | `0d96706b62eb446fab0b35598e252cb84fbf87f43ea57fecf28e8a9080a0f013` | `??` | `P1_TEST_DATA` |
| `THREAT_MODEL.md` | 22,557 | `6455c5505ef2cd8d6c0d5edb55181c628d36881686e64ecf1417e9c5546bd064` | `??` | `P1_DOC` |
| `tools/phase1/__pycache__/extract_directive_requirements.cpython-313.pyc` | 27,755 | `d279daf1143505d557b0e71744d7c38e3346ceb6fbe2d81c49497074c73c92e5` | `!!` | `IGN_PYC` |
| `tools/phase1/extract_directive_requirements.py` | 38,110 | `299f052ab6e6249fa7019a5209b18403c6422a81627503417f06bef2f26b9618` | `??` | `P1_TOOL` |
| `tools/phase1/run_phase1_verification.py` | 2,653 | `7a0d383ab95dc005b03aba98f5ffad4db878236cb93699434921749f28d713c7` | `??` | `P1_TOOL` |
| `tools/phase1/verify-phase1.mjs` | 42,920 | `958d987bac94bae7f197cc8fcb658e9ca6e4511ea53eeb105618b8105d7877cc` | `??` | `P1_TOOL` |

## Exact commands and raw evidence

Commands were run read-only from the audit root.

### 1. Enumeration and independent reconciliation

```powershell
$repo=(Get-Location).Path
$ps=@(
  Get-ChildItem -LiteralPath $repo -Recurse -Force -File |
    Where-Object { $_.FullName -notlike "$repo\.git\*" } |
    ForEach-Object { $_.FullName.Substring($repo.Length+1).Replace('\','/') } |
    Sort-Object
)
$rg=@(
  rg --files -uu -g '!.git/**' |
    ForEach-Object { $_.Replace('\','/') } |
    Sort-Object
)
"POWERSHELL_COUNT=$($ps.Count)"
"RIPGREP_COUNT=$($rg.Count)"
"PS_ONLY_COUNT=$(@(Compare-Object $ps $rg | Where-Object SideIndicator -eq '<=').Count)"
"RG_ONLY_COUNT=$(@(Compare-Object $ps $rg | Where-Object SideIndicator -eq '=>').Count)"
```

```text
POWERSHELL_COUNT=93
RIPGREP_COUNT=93
PS_ONLY_COUNT=0
RG_ONLY_COUNT=0
```

Per-file hashes were produced with `Get-FileHash -Algorithm SHA256`; ignore state used `git check-ignore -q -- <path>`. An independent deterministic byte calculation returned:

```text
FILE_COUNT=93
TOTAL_BYTES=15692126
INVENTORY_SHA256=8291ff724cb0f4a7b3922ab12186d10797492d2497be79522595c60bd11f0dcb
```

### 2. Git and ignore state

```powershell
git rev-parse --show-toplevel
git rev-list --count HEAD
git remote -v
@(git ls-files).Count
$s=@(git status --porcelain=v1 --ignored --untracked-files=all)
"STATUS_ENTRIES=$($s.Count)"
"UNTRACKED=$(@($s | Where-Object { $_.StartsWith('??') }).Count)"
"IGNORED=$(@($s | Where-Object { $_.StartsWith('!!') }).Count)"
$all | git check-ignore -v --stdin
```

```text
no HEAD
no remotes
TRACKED_COUNT=0
STATUS_ENTRIES=93
UNTRACKED=41
IGNORED=52
.gitignore:37  .phase1-verification/  -> 49 files
.gitignore:2   node_modules/          -> 2 files
.gitignore:5   __pycache__/           -> 1 file
```

### 3. Product-root/path scan

```powershell
@(
  'apps','packages','profiles','scenarios','assets','assets/original',
  'src','public','dist','build','target','coverage','artifacts',
  'test-results','playwright-report','.pnpm-store'
) | ForEach-Object { "{0}`t{1}" -f $_,(Test-Path -LiteralPath $_) }

rg --files -uu -g '!.git/**' |
  Where-Object {
    $_ -match '(^|[\\/])(apps|packages|src|public|profiles|scenarios|assets([\\/]original)?|dist|build|target|coverage|artifacts|test-results|playwright-report)([\\/]|$)' -or
    $_ -match '\.(ts|tsx|jsx|rs|wasm|html|css|vue|svelte|exe|dll|so|dylib)$'
  }
```

Raw result: every tested root was `False`; the path/extension scan returned no matches.

### 4. Manifest and lock structure

```powershell
$p=Get-Content -Raw package.json | ConvertFrom-Json
@('dependencies','devDependencies','optionalDependencies','peerDependencies','bundledDependencies') |
  ForEach-Object {
    $prop=$p.PSObject.Properties[$_]
    if($null -eq $prop){ "$_=ABSENT" } else { "$_=PRESENT;COUNT=$(@($prop.Value.PSObject.Properties).Count)" }
  }
"PNPM_EMPTY_ROOT_IMPORTER=$(@(Select-String pnpm-lock.yaml -Pattern '^  \.: \{\}$').Count)"
"PNPM_PACKAGES_SECTIONS=$(@(Select-String pnpm-lock.yaml -Pattern '^packages:$').Count)"
"PNPM_SNAPSHOTS_SECTIONS=$(@(Select-String pnpm-lock.yaml -Pattern '^snapshots:$').Count)"
"CARGO_PACKAGE_BLOCKS=$(@(Select-String Cargo.lock -Pattern '^\[\[package\]\]$').Count)"
(Select-String Cargo.toml -Pattern '^members = .*').Line
Get-ChildItem node_modules -Recurse -Force
```

Raw results are in T7-03; `node_modules` has exactly the two inventoried metadata files and no directory.

### 5. Import, artifact-format, and byte-signature scans

```powershell
rg -n '^(import|export .* from|const .*require\()' tools/phase1 -g '*.mjs' -g '*.js' -g '*.cjs'

python -B -c "import ast,pathlib; root=pathlib.Path('.'); ps=sorted([*root.glob('tools/phase1/*.py'),*pathlib.Path('.phase1-verification').rglob('*.py')],key=lambda p:p.as_posix()); [(print(p.as_posix()+': '+', '.join(sorted({(n.module or '').split('.')[0] for n in ast.walk(ast.parse(p.read_text(encoding='utf-8-sig'))) if isinstance(n,ast.ImportFrom)}|{a.name.split('.')[0] for n in ast.walk(ast.parse(p.read_text(encoding='utf-8-sig'))) if isinstance(n,ast.Import) for a in n.names})))) for p in ps]"

python -B -c "from pathlib import Path; root=Path('.'); fs=sorted((p for p in root.rglob('*') if p.is_file() and '.git' not in p.parts),key=lambda p:p.as_posix()); sigs={'PE_MZ':bytes.fromhex('4d5a'),'ELF':bytes.fromhex('7f454c46'),'WASM':bytes.fromhex('0061736d'),'PDF':b'%PDF','PNG':bytes.fromhex('89504e470d0a1a0a'),'ZIP':bytes.fromhex('504b0304')}; hits={k:[] for k in sigs}; [(hits[k].append(p.as_posix()) if p.open('rb').read(16).startswith(v) else None) for p in fs for k,v in sigs.items()]; [print(k+'_COUNT='+str(len(v))) or [print(k+' '+x) for x in v] for k,v in hits.items()]"
```

Raw Python imports:

```text
.phase1-verification/docx-visual/analyze_pdf.py: __future__, json, pathlib, pdfplumber, re
.phase1-verification/docx-visual/make_contact_sheets.py: PIL, pathlib
tools/phase1/extract_directive_requirements.py: __future__, argparse, dataclasses, hashlib, json, pathlib, re, sys, typing, xml, zipfile
tools/phase1/run_phase1_verification.py: __future__, os, pathlib, shutil, subprocess, sys
tools/phase1/verify-phase1.mjs: node:crypto, node:child_process, node:fs, node:path, node:url
```

Signature counts are in T7-05.

### 6. CI false gate and remote references

```powershell
Select-String -LiteralPath .github/workflows/phase1-governance.yml `
  -Pattern '^\s*workflow_dispatch:','^\s*if: \$\{\{ false \}\}$','^\s*runs-on:','^\s*uses:' |
  ForEach-Object { "{0}: {1}" -f $_.LineNumber,$_.Line.Trim() }
```

```text
4: workflow_dispatch:
18: if: ${{ false }}
19: runs-on: windows-2025
24: uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
30: uses: actions/setup-node@820762786026740c76f36085b0efc47a31fe5020
37: uses: actions/setup-python@5fda3b95a4ea91299a34e894583c3862153e4b97
63: uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a
```

### 7. Reparse/link/alternate-stream audit

```powershell
$items=Get-ChildItem -LiteralPath $repo -Recurse -Force |
  Where-Object { $_.FullName -ne "$repo\.git" -and $_.FullName -notlike "$repo\.git\*" }
"ITEMS_TOTAL=$($items.Count)"
"FILES_TOTAL=$(@($items | Where-Object {-not $_.PSIsContainer}).Count)"
"DIRECTORIES_TOTAL=$(@($items | Where-Object {$_.PSIsContainer}).Count)"
"REPARSE_ITEMS=$(@($items | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint }).Count)"
"SYMLINKS=$(@($items | Where-Object { $_.LinkType -in @('SymbolicLink','Junction') }).Count)"

Get-ChildItem -LiteralPath $repo -Recurse -Force -File |
  Where-Object { $_.FullName -notlike "$repo\.git\*" } |
  ForEach-Object { Get-Item -LiteralPath $_.FullName -Stream * } |
  Where-Object { $_.Stream -ne ':$DATA' }
```

```text
ITEMS_TOTAL=110
FILES_TOTAL=93
DIRECTORIES_TOTAL=17
REPARSE_ITEMS=110
SYMLINKS=0
ALTERNATE_STREAMS=2
```

## Final determination

No Phase 2/product implementation, product runtime package, installed dependency graph, product shell, industrial-communication implementation, native/WASM executable, vendor project, product asset, build, distribution, or release artifact exists.

The repository is not empty and must not be described that way: it contains Phase 1 governance/tooling bootstrap files, empty future-workspace declarations, a disabled proposed remote workflow, deterministic snapshots/locks, and ignored local verification/cache/package-manager residue. Zero commits/tracked files and the two undeclared/unavailable ignored QA imports are the material qualifications.

