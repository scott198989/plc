# Phase 1 pre-remediation starting state

Captured: `2026-08-27T16:04:13.9574932-05:00`

Purpose: evidence snapshot required by the Phase 1 Corrective Addendum WP-0A. This records the subject before corrective edits. It is evidence, not the trusted acceptance baseline.

## Canonical-source and addendum hashes

Command:

```powershell
Get-FileHash -Algorithm SHA256 -LiteralPath 'Govs PLC project Research Report.md','PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx','C:\Users\Scott\Downloads\PLC Engineering Simulator - Phase 1 Corrective Addendum - Closure and Trusted Baseline.docx' | Format-List Algorithm,Hash,Path
```

Raw output:

```text

Algorithm : SHA256
Hash      : F05C08323B5CC9483BEB1FEB3C7312CCB9A45EBE3B527E6DAE069C181D3FBF55
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\Govs PLC project Research Report.md

Algorithm : SHA256
Hash      : EBF074E2CEAB752F09E6DB63D88E100991729DA13C1EB874290A6B337DA72612
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\PLC Engineering Simulator - Codex Master Implementation 
            Directive Phase 1.docx

Algorithm : SHA256
Hash      : 950C5112C34D0218FD1E59CF6C051ACCD01AB92674CD70C96C08A5F1DA2E5A1C
Path      : C:\Users\Scott\Downloads\PLC Engineering Simulator - Phase 1 Corrective Addendum - Closure and Trusted 
            Baseline.docx
```

Result: both canonical repository sources exist and exactly match the 64-character SHA-256 values stated by the corrective addendum. The addendum itself hashes to `950C5112C34D0218FD1E59CF6C051ACCD01AB92674CD70C96C08A5F1DA2E5A1C`.

## Git state before remediation

Commands included `git status --porcelain=v2 --branch --untracked-files=all`, `git rev-parse --verify HEAD`, `git remote -v`, `git tag --list`, and tracked/untracked/ignored enumerations.

```text
IS_WORK_TREE=true
warning: unable to access 'C:\Users\Scott/.config/git/ignore': Permission denied
# branch.oid (initial)
# branch.head main
? .editorconfig
? .gitattributes
? .github/workflows/phase1-governance.yml
? .gitignore
? .python-version
? ADR/0001-no-physical-industrial-communication.md
? ADR/0002-original-project-format.md
? ADR/0003-unified-plc-ir.md
? ADR/0004-deterministic-virtual-time.md
? ASSET_PROVENANCE.json
? CHANGELOG_DIRECTIVE.md
? CLEAN_ROOM_POLICY.md
? CODEX AUDIT AFTER RED TEAM.docx
? CODEX RED TEAM AUDIT PHASE 1.docx
? CONTRIBUTOR_CLEAN_ROOM_ATTESTATION.md
? Cargo.lock
? Cargo.toml
? DEPENDENCY_POLICY.md
? EVIDENCE_REGISTER.json
? Govs PLC project Research Report.md
? IMPLEMENTATION_MATRIX.json
? LEGAL_REVIEW_CHECKLIST.md
? OPEN_DECISIONS.md
? PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx
? README.md
? REQUIREMENTS.md
? RISK_REGISTER.md
? SECURITY_INVARIANTS.md
? THREAT_MODEL.md
? docs/governance/DOCX_VISUAL_QA.md
? docs/governance/PHASE_1_ADVERSARIAL_AUDIT.md
? docs/governance/PHASE_1_SCOPE_AUDIT.md
? docs/governance/PHASE_1_VERIFICATION_PLAN.md
? docs/governance/TOOLCHAIN_ADMISSION_REGISTER.md
? docs/research/UNRESOLVED_SOURCE_TOKENS.md
? package.json
? pnpm-lock.yaml
? pnpm-workspace.yaml
? requirements/phase1-requirements.json
? rust-toolchain.toml
? tests/phase1/policy-contract.json
? tools/phase1/extract_directive_requirements.py
? tools/phase1/run_phase1_verification.py
? tools/phase1/verify-phase1.mjs
HEAD_BEGIN
fatal: Needed a single revision
HEAD_EXIT=128
REMOTES_BEGIN
TAGS_BEGIN
TRACKED_COUNT=0
warning: unable to access 'C:\Users\Scott/.config/git/ignore': Permission denied
UNTRACKED_COUNT=44
warning: unable to access 'C:\Users\Scott/.config/git/ignore': Permission denied
IGNORED_COUNT=52
UNTRACKED_BEGIN
warning: unable to access 'C:\Users\Scott/.config/git/ignore': Permission denied
.editorconfig
.gitattributes
.github/workflows/phase1-governance.yml
.gitignore
.python-version
ADR/0001-no-physical-industrial-communication.md
ADR/0002-original-project-format.md
ADR/0003-unified-plc-ir.md
ADR/0004-deterministic-virtual-time.md
ASSET_PROVENANCE.json
CHANGELOG_DIRECTIVE.md
CLEAN_ROOM_POLICY.md
CODEX AUDIT AFTER RED TEAM.docx
CODEX RED TEAM AUDIT PHASE 1.docx
CONTRIBUTOR_CLEAN_ROOM_ATTESTATION.md
Cargo.lock
Cargo.toml
DEPENDENCY_POLICY.md
EVIDENCE_REGISTER.json
Govs PLC project Research Report.md
IMPLEMENTATION_MATRIX.json
LEGAL_REVIEW_CHECKLIST.md
OPEN_DECISIONS.md
PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx
README.md
REQUIREMENTS.md
RISK_REGISTER.md
SECURITY_INVARIANTS.md
THREAT_MODEL.md
docs/governance/DOCX_VISUAL_QA.md
docs/governance/PHASE_1_ADVERSARIAL_AUDIT.md
docs/governance/PHASE_1_SCOPE_AUDIT.md
docs/governance/PHASE_1_VERIFICATION_PLAN.md
docs/governance/TOOLCHAIN_ADMISSION_REGISTER.md
docs/research/UNRESOLVED_SOURCE_TOKENS.md
package.json
pnpm-lock.yaml
pnpm-workspace.yaml
requirements/phase1-requirements.json
rust-toolchain.toml
tests/phase1/policy-contract.json
tools/phase1/extract_directive_requirements.py
tools/phase1/run_phase1_verification.py
tools/phase1/verify-phase1.mjs
IGNORED_BEGIN
warning: unable to access 'C:\Users\Scott/.config/git/ignore': Permission denied
.phase1-verification/docx-visual/analyze_pdf.py
.phase1-verification/docx-visual/contact/contact-01.png
.phase1-verification/docx-visual/contact/contact-02.png
.phase1-verification/docx-visual/contact/contact-03.png
.phase1-verification/docx-visual/contact/contact-04.png
.phase1-verification/docx-visual/make_contact_sheets.py
.phase1-verification/docx-visual/pages/page-01.png
.phase1-verification/docx-visual/pages/page-02.png
.phase1-verification/docx-visual/pages/page-03.png
.phase1-verification/docx-visual/pages/page-04.png
.phase1-verification/docx-visual/pages/page-05.png
.phase1-verification/docx-visual/pages/page-06.png
.phase1-verification/docx-visual/pages/page-07.png
.phase1-verification/docx-visual/pages/page-08.png
.phase1-verification/docx-visual/pages/page-09.png
.phase1-verification/docx-visual/pages/page-10.png
.phase1-verification/docx-visual/pages/page-11.png
.phase1-verification/docx-visual/pages/page-12.png
.phase1-verification/docx-visual/pages/page-13.png
.phase1-verification/docx-visual/pages/page-14.png
.phase1-verification/docx-visual/pages/page-15.png
.phase1-verification/docx-visual/pages/page-16.png
.phase1-verification/docx-visual/pages/page-17.png
.phase1-verification/docx-visual/pages/page-18.png
.phase1-verification/docx-visual/pages/page-19.png
.phase1-verification/docx-visual/pages/page-20.png
.phase1-verification/docx-visual/pages/page-21.png
.phase1-verification/docx-visual/pages/page-22.png
.phase1-verification/docx-visual/pages/page-23.png
.phase1-verification/docx-visual/pages/page-24.png
.phase1-verification/docx-visual/pages/page-25.png
.phase1-verification/docx-visual/pages/page-26.png
.phase1-verification/docx-visual/pages/page-27.png
.phase1-verification/docx-visual/pages/page-28.png
.phase1-verification/docx-visual/pages/page-29.png
.phase1-verification/docx-visual/pages/page-30.png
.phase1-verification/docx-visual/pages/page-31.png
.phase1-verification/docx-visual/pages/page-32.png
.phase1-verification/docx-visual/pages/page-33.png
.phase1-verification/docx-visual/pages/page-34.png
.phase1-verification/docx-visual/pages/page-35.png
.phase1-verification/docx-visual/pages/page-36.png
.phase1-verification/docx-visual/pages/page-37.png
.phase1-verification/docx-visual/pages/page-38.png
.phase1-verification/docx-visual/pages/page-39.png
.phase1-verification/docx-visual/pages/page-40.png
.phase1-verification/docx-visual/phase1-directive-render.pdf
.phase1-verification/docx-visual/visual-analysis.json
.phase1-verification/phase1-report.json
node_modules/.package-map.json
node_modules/.pnpm-workspace-state-v1.json
tools/phase1/__pycache__/extract_directive_requirements.cpython-313.pyc
```

Finding: the directory was already a Git work tree on branch `main`, but it had no HEAD, commits, remotes, tags, or tracked files. There were 44 non-ignored untracked files and 52 ignored files.

## Toolchain and render tooling

```text
OS=Microsoft Windows NT 10.0.26200.0
POWERSHELL=7.6.5
GIT_PATH=C:\Program Files\Git\cmd\git.exe
git version 2.50.1.windows.1
GIT_LOCAL_NAME=
GIT_LOCAL_EMAIL=
PYTHON_PINNED_PATH=C:\Users\Scott\AppData\Local\Programs\Python\Python313\python.exe
Python 3.13.12
PYTHON_BUNDLED_PATH=C:\Users\Scott\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe
Python 3.12.13
NODE_PATH=C:\Program Files\nodejs\node.exe
v24.14.0
PNPM_PATH=C:\Users\Scott\.cache\codex-runtimes\codex-primary-runtime\dependencies\bin\fallback\pnpm.cmd
11.19.0
RUSTC_PATH=C:\Users\Scott\.cargo\bin\rustc.exe
rustc 1.94.0 (4a4ef493e 2026-03-02)
CARGO_PATH=C:\Users\Scott\.cargo\bin\cargo.exe
cargo 1.94.0 (85eff7c80 2026-01-15)
RUSTUP_PATH=C:\Users\Scott\.cargo\bin\rustup.exe
rustup 1.28.2 (e4f3ad6f8 2025-04-28)
RUST_TOOLCHAINS
stable-x86_64-pc-windows-msvc (default)
1.94.0-x86_64-pc-windows-msvc (active)
RUST_TARGETS
x86_64-pc-windows-msvc
pdfinfo version 26.05.0
pdftoppm version 26.05.0
SOFFICE=NOT_FOUND
WORD_VERSION=16.0
WORD_BUILD=16.0.20326
PYTHON_DOCX=1.2.0
PYPDF=6.10.0
PYPDFIUM2=5.13.0
```

Microsoft Word 16.0 build 16.0.20326 rendered the supplied corrective addendum read-only to a 12-page PDF. Poppler 26.05.0 rasterized all pages. LibreOffice/soffice was not installed. The workspace-bundled Python is 3.12.13, while the repository-pinned verifier interpreter is Python 3.13.12; both are recorded because both were used during the broader audit history.

## Filesystem metadata

```text
ITEMS_TOTAL=113
FILES_TOTAL=96
DIRECTORIES_TOTAL=17
REPARSE_ITEMS=113
SYMLINKS=0
ALTERNATE_STREAMS_BEGIN

FileName
--------                                                                                                               
C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\Govs PLC project Research Report.md                                  
C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\PLC Engineering Simulator - Codex Master Implementation Directive Ph…
```

Alternate data streams:

```text
C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\Govs PLC project Research Report.md	Zone.Identifier	56
C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx	Zone.Identifier	378
```

Every visible OneDrive item reported the reparse attribute, but no symbolic link or junction was present. The two canonical source files have Windows `Zone.Identifier` streams; their primary file-stream hashes are recorded above and in `file-inventory.tsv`.

## File inventory

`file-inventory.tsv` contains the 96-file pre-remediation population excluding `.git` internals, with relative path, primary-stream byte length, and independently recomputed SHA-256. Aggregate primary-stream size: 16,107,489 bytes.

## Intentionally ignored classes present at capture

- `.phase1-verification/**`: local generated report/render/PNG/contact-sheet/analysis/helper residue.
- `node_modules/**`: package-manager metadata only at capture; no installed dependency packages.
- `tools/phase1/__pycache__/**`: Python bytecode cache.

These ignored files were preserved during the snapshot. They are not treated as portable closure evidence; final closure evidence must be tracked separately under `evidence/phase1-closure/`.

## Pre-edit conclusion

The canonical sources pass the addendum's stop condition. Corrective work may proceed. This snapshot does not assert that Phase 1 is trustworthy or closed.
