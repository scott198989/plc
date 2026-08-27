# Phase 1 Adversarial Audit

**Audit date:** 2026-08-27  
**Repository:** `C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC`  
**Audit posture:** defect-seeking, read-only inspection of existing Phase 1 artifacts  
**Source directive:** `References for Codex from Scott/PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx`
**Source directive SHA-256:** `EBF074E2CEAB752F09E6DB63D88E100991729DA13C1EB874290A6B337DA72612`  
**Frozen research:** `References for Codex from Scott/Govs PLC project Research Report.md`
**Frozen research SHA-256:** `F05C08323B5CC9483BEB1FEB3C7312CCB9A45EBE3B527E6DAE069C181D3FBF55`

## Executive finding

> Historical note: the long-form finding and captured outputs immediately below
> record the pre-remediation adversarial audit. They are intentionally preserved
> as evidence. Current remediation evidence, controlled mutation results,
> read-only review, and the controlling closure verdict are appended near the end
> of this report.

This audit found material defects. The requirement registry is not a complete atomic representation of the directive: the reverse walk found 546 in-scope normative statement units, of which 48 are unmapped, while 20 issued-ID records remain unsplit compound records. Five of twelve isolated mutations escaped the full governance suite. Of the suite's 163 passing check instances, 63 are tautological and 36 are structural; only 64 have a defensible numbered directive anchor, and that classification does not establish sufficiency. The current Phase 1 record therefore is **not trustworthy enough to authorize Phase 2 implementation yet**. The detailed verdict and required preconditions are at the end of this report.

## Evidence boundary and reproducibility

The audit treated the supplied DOCX, frozen research report, raw filesystem contents, and verbatim console output reproduced below as evidence. `IMPLEMENTATION_MATRIX.json`, `EVIDENCE_REGISTER.json`, `RISK_REGISTER.md`, `.phase1-verification/phase1-report.json`, and verifier output were inspected only as audit targets and were never used to prove their own correctness. Source-page anchors come from a fresh, read-only Microsoft Word export of the supplied DOCX to a temporary 40-page PDF; the fresh export was independently reported as 40 pages by Poppler. Normalized text extracted from every fresh page matched the existing local render page-for-page (`40/40`; combined text SHA-256 `DD5FAA0B213307CC6D4FBB8D5087FBC59751B777009CD2997BA9EF453E02FF`). This page-rendering method supplies location anchors, not approval or Phase 1 acceptance evidence.

Existing Phase 1 files were not edited. Mutation testing used one isolated scratch copy per mutation outside the repository, with an unmodified scratch baseline first and one mutation at a time. All per-case copies were removed immediately after execution; the entire audit scratch root was deleted after this report was assembled and checked. No commit was created, no truth state was promoted to `VERIFIED`, and no requirement, ADR, or register was added.

## Defects

The machine-readable controlling defect ledger is
`evidence/phase1-closure/defect-register.json`. The table below preserves both
the pre-remediation finding and the current corrective disposition; it does not
convert Scott's acceptance into an implementation-agent judgment.

| ID | Severity | Defect | Current corrective disposition |
|---|---|---|---|
| `DEF-001` | High | Audit incomplete and closure placeholders present | Resolved: all sections complete; final Markdown/DOCX and all-page QA are closure evidence |
| `DEF-002` | Critical | 48 source-recall gaps lacked explicit dispositions | Resolved: all 546 source units map in the reconciliation ledger |
| `DEF-003` | High | 20 compounds were not atomically split | Resolved: parents preserved and 190 stable children issued |
| `DEF-004` | High | `PES-REQ-0003` source field drifted | Resolved in deterministic extraction and reconciliation |
| `DEF-005` | Critical | Integrity oracle was self-recomputed from the subject | Resolved: external sealed Git-object manifest, tamper rejection, and immutable validation pass |
| `DEF-006` | High | `.editorconfig` mutation escaped | Resolved by exact-file-set `VER-INT-0002` control |
| `DEF-007` | Critical | External URL mutation escaped | Resolved by scoped `VER-OFF-0001` control |
| `DEF-008` | Critical | Loopback endpoint mutation escaped | Resolved by scoped `VER-OFF-0002` control |
| `DEF-009` | High | Vendor-facing text mutation escaped | Resolved by user-facing `VER-BRN-0001` control |
| `DEF-010` | High | Risk closure could lack evidence | Resolved by linked-evidence `VER-RSK-0001` control |
| `DEF-011` | High | Missing ADR caused a crash | Resolved by controlled named `VER-ADR-0001` failure |
| `DEF-012` | Medium | Historical render digest had only 62 hexadecimal characters | Resolved by reproducible 64-character recomputation |
| `DEF-013` | High | Canonical filenames/directive sequencing conflicted | Resolved through `CR-0001` and Scott's reference-folder relocation |
| `DEF-014` | High | No preserved pre-remediation Git history | Resolved by commit and annotated tag `phase1-pre-remediation-v1` |
| `DEF-015` | High | CI disabled; no shared complete closure gate | Resolved by active exact local/CI `gate:closure`, with upload absent |
| `DEF-016` | High | No runnable minimal technical foundation | Resolved by bounded offline React/Worker/Rust/WASM health round trip |
| `DEF-017` | Medium | Historical scratch-deletion statement contradicted disk state | Resolved by preserving and labeling the exact scratch evidence |
| `DEF-018` | Medium | Asset/dependency checks were selective or vacuous | Resolved for current candidate by exact asset truth and complete direct-dependency inventory |
| `DEF-019` | Medium | Legal/provenance/security admission is incomplete | Open external review; candidates remain `CANDIDATE_UNREVIEWED` and non-release |
| `DEF-020` | Medium | Historical status wording differs from `PES-ACC-0005` | Open Scott decision; source remains immutable |
| `DEF-021` | Low | No remote or hosted CI run evidence exists | Open external operation under `DEC-0002`; exact local gate remains in scope |
| `DEF-022` | High | Linked-worktree root `.git` control file was misclassified as project content | Resolved by exact-root metadata handling plus an automatic root-file/root-directory/nested-file/nested-directory regression |

## Task 1 — Requirement extraction: precision

### Executive findings

1. The supplied DOCX contains **247 unique issued PES IDs**. Every area is contiguous from 0001 through its observed maximum; there are no duplicate or missing sequence numbers inside an area.
2. Fresh raw PDF text extraction mapped **all 247 issued markers exactly once** across **40 pages**. Page values below are marker-start pages and are locators only; requirement wording comes from the DOCX OOXML text.
3. The deterministic positional sample contains **21 IDs**. The selected security/threat rows expand to **94 unique PES IDs**. Eight overlap, yielding **107 audited rows**.
4. Selected-row result: **107 FAITHFUL, 0 DRIFTED, 0 UNSUPPORTED, 0 MISSING_SOURCE**.
5. A whole-population text comparison, performed as a cross-check beyond the required selection, found **one out-of-sample verbatim drift: PES-REQ-0003**. Its registry atomicRequirement injects the words "Table row:" before every source table row. The table data and meaning are preserved, but the field is not verbatim source text.
6. The supplied DOCX contains **no explicit numeric statement that the requirement total is 247**: the token 247 does not occur in DOCX textual content, and no requirement-total/count statement was found. The frozen research contains neither issued PES markers nor the token 247. Thus **247 is an independently observed issued-ID count, not a directive-stated total**.
7. Each of the 247 issued IDs begins with exactly one recognized leading normative keyword. This proves 247 issued requirement records, but not 247 semantically atomic normative statements; compound sentences, subordinate shall/may clauses, and list continuations prevent a one-record/one-atomic-statement inference.

### Evidence boundary and method

- Source authority for wording: the supplied DOCX only. The frozen research was hash-checked and inspected only to confirm it does not issue PES IDs or state a 247 total.
- Independent extraction: OOXML body blocks were walked in document order using the bundled document runtime. Heading paths came from Word heading styles. Table cells were preserved as cell | cell textual rows.
- "Verbatim source" below means exact OOXML textual content. Auto-number and bullet glyphs are Word numbering metadata rather than w:t text; block order and words are preserved.
- Page locator: raw text extraction from the current 40-page PDF render. Every [PES-...] marker occurred on exactly one page.
- Registry comparison: requirements/phase1-requirements.json was parsed as an audit target. A row is FAITHFUL only when atomicRequirement, independently derived heading path, source filename, source hash, and SRC-0002 pointer all match.
- DRIFTED: source and registry record both exist but differ. UNSUPPORTED: target wording/pointer lacks support in the located source. MISSING_SOURCE: no issued source marker exists.
- SECURITY_INVARIANTS.md and THREAT_MODEL.md were used only to determine the parent-requested selection; their claims were not treated as proof of directive wording.

### Deterministic positional sample

The 247 full IDs were sorted ASCII-lexicographically by the complete zero-padded ID, then positions 1, 13, 25, ..., 241 were selected.

| Position | ID |
|---:|---|
| 1 | PES-ACC-0001 |
| 13 | PES-ARC-0006 |
| 25 | PES-ARC-0018 |
| 37 | PES-ARC-0030 |
| 49 | PES-CRM-0009 |
| 61 | PES-CRM-0021 |
| 73 | PES-DET-0002 |
| 85 | PES-DEV-0007 |
| 97 | PES-DOC-0001 |
| 109 | PES-FID-0003 |
| 121 | PES-GOV-0007 |
| 133 | PES-GOV-0019 |
| 145 | PES-ISO-0006 |
| 157 | PES-ISO-0018 |
| 169 | PES-MSN-0008 |
| 181 | PES-PROF-0004 |
| 193 | PES-REQ-0002 |
| 205 | PES-SCP-0005 |
| 217 | PES-SEC-0007 |
| 229 | PES-SEC-0019 |
| 241 | PES-TYP-0001 |

### Expanded gate/threat reference selection

| Selector | Expanded PES IDs |
|---|---|
| SI-GATE-12 | PES-ARC-0013, PES-SEC-0018, PES-SEC-0019, PES-SEC-0020 |
| SI-GATE-13 | PES-DIA-0005, PES-TCH-0001, PES-TCH-0002, PES-TCH-0003, PES-TCH-0004, PES-TCH-0005 |
| SI-GATE-14 | PES-ISO-0022, PES-QLT-0006 |
| SI-GATE-15 | PES-CI-0002, PES-CI-0003, PES-CRM-0021, PES-CRM-0022, PES-DOC-0004 |
| TM-11 | PES-QLT-0005, PES-SEC-0014, PES-SEC-0015, PES-SEC-0016 |
| TM-12 | PES-ARC-0026, PES-DEV-0012, PES-ISO-0007, PES-ISO-0020 |
| TM-13 | PES-ISO-0005, PES-ISO-0018 |
| TM-14 | PES-CRM-0010, PES-ISO-0006, PES-ISO-0019, PES-ISO-0021, PES-PRJ-0001, PES-PRJ-0002, PES-PRJ-0003, PES-PRJ-0004, PES-PRJ-0005, PES-PRJ-0006, PES-PRJ-0007 |
| TM-15 | PES-DIA-0003, PES-DIA-0004, PES-DIA-0005, PES-EDU-0004, PES-EDU-0005, PES-TCH-0001 |
| TM-16 | PES-TCH-0001, PES-TCH-0002 |
| TM-17 | PES-TCH-0003, PES-TCH-0004, PES-TCH-0005 |
| TM-18 | PES-CI-0001, PES-MSN-0007, PES-SCP-0006, PES-SEC-0006, PES-SEC-0007, PES-SEC-0008 |
| TM-19 | PES-CI-0002, PES-CI-0003, PES-CRM-0001, PES-CRM-0002, PES-CRM-0003, PES-CRM-0004, PES-CRM-0005, PES-CRM-0006, PES-CRM-0007, PES-CRM-0008, PES-CRM-0009, PES-CRM-0010, PES-CRM-0011, PES-CRM-0012, PES-CRM-0013, PES-CRM-0014, PES-CRM-0015, PES-CRM-0016, PES-CRM-0017, PES-CRM-0018, PES-CRM-0019, PES-CRM-0020, PES-CRM-0021, PES-CRM-0022, PES-CRM-0023, PES-CRM-0024, PES-CRM-0025, PES-DOC-0004 |
| TM-20 | PES-ARC-0004, PES-ARC-0005, PES-ARC-0006, PES-ARC-0007, PES-ARC-0008, PES-ARC-0009, PES-ARC-0010, PES-ARC-0011, PES-ARC-0012, PES-ARC-0013, PES-ARC-0014, PES-ARC-0015, PES-PRJ-0003, PES-PRJ-0004, PES-PRJ-0005, PES-SEC-0022 |
| TM-21 | PES-DET-0001, PES-DET-0002, PES-DET-0003, PES-DET-0004, PES-DET-0005, PES-DET-0006, PES-DET-0007, PES-DEV-0005 |
| TM-22 | PES-ISO-0011, PES-ISO-0022, PES-QLT-0008, PES-SEC-0010, PES-SEC-0011 |

TM-21 also references RSK-0005. It is not a PES requirement ID and therefore is not one of the 247 issued requirements; it is noted rather than silently coerced into the sample.

### Task 1 precision rows

Rows are sorted by full requirement ID. A row can have both a positional-sample origin and one or more gate/threat origins.

#### PES-ACC-0001

- **Selection basis:** sorted position 1
- **Page/section:** page 7; 2. Product Charter > 2.3 Governing success definition
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ACC-0001] MUST judge success by causal and workflow transfer: the same kinds of engineering decisions should produce the same kinds of engineering consequences.
~~~~

**Registry text**

~~~~text
MUST judge success by causal and workflow transfer: the same kinds of engineering decisions should produce the same kinds of engineering consequences.
~~~~

#### PES-ARC-0004

- **Selection basis:** TM-20
- **Page/section:** page 21; 8. Constitutional Architecture Invariants > 8.2 Stable identity and project graph
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ARC-0004] MUST represent every semantically referenceable project, hardware, network, language, HMI, library, process, lesson, scenario, assessment, diagnostic source, and runtime target object with an immutable UUID.
~~~~

**Registry text**

~~~~text
MUST represent every semantically referenceable project, hardware, network, language, HMI, library, process, lesson, scenario, assessment, diagnostic source, and runtime target object with an immutable UUID.
~~~~

#### PES-ARC-0005

- **Selection basis:** TM-20
- **Page/section:** page 21; 8. Constitutional Architecture Invariants > 8.2 Stable identity and project graph
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ARC-0005] MUST use RFC 9562 UUID version 4 by default for newly created objects. Display names, addresses, paths, array positions, block numbers, and source coordinates shall not serve as identity.
~~~~

**Registry text**

~~~~text
MUST use RFC 9562 UUID version 4 by default for newly created objects. Display names, addresses, paths, array positions, block numbers, and source coordinates shall not serve as identity.
~~~~

#### PES-ARC-0006

- **Selection basis:** sorted position 13, TM-20
- **Page/section:** page 21; 8. Constitutional Architecture Invariants > 8.2 Stable identity and project graph
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ARC-0006] MUST preserve UUID on rename, move, readdress, regroup, interface-compatible edit, and undo restoration.
~~~~

**Registry text**

~~~~text
MUST preserve UUID on rename, move, readdress, regroup, interface-compatible edit, and undo restoration.
~~~~

#### PES-ARC-0007

- **Selection basis:** TM-20
- **Page/section:** page 22; 8. Constitutional Architecture Invariants > 8.2 Stable identity and project graph
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ARC-0007] MUST create a new UUID for copy, template instantiation when independent, and imported objects intentionally duplicated as new objects.
~~~~

**Registry text**

~~~~text
MUST create a new UUID for copy, template instantiation when independent, and imported objects intentionally duplicated as new objects.
~~~~

#### PES-ARC-0008

- **Selection basis:** TM-20
- **Page/section:** page 22; 8. Constitutional Architecture Invariants > 8.2 Stable identity and project graph
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ARC-0008] MUST retain a tombstone for deleted referenced objects for as long as a live reference, undo record, migration record, diagnostic, audit event, or snapshot requires it.
~~~~

**Registry text**

~~~~text
MUST retain a tombstone for deleted referenced objects for as long as a live reference, undo record, migration record, diagnostic, audit event, or snapshot requires it.
~~~~

#### PES-ARC-0009

- **Selection basis:** TM-20
- **Page/section:** page 22; 8. Constitutional Architecture Invariants > 8.2 Stable identity and project graph
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ARC-0009] MUST represent unresolved references explicitly with the target UUID and reference kind. Deletion shall not silently erase or retarget usages.
~~~~

**Registry text**

~~~~text
MUST represent unresolved references explicitly with the target UUID and reference kind. Deletion shall not silently erase or retarget usages.
~~~~

#### PES-ARC-0010

- **Selection basis:** TM-20
- **Page/section:** page 22; 8. Constitutional Architecture Invariants > 8.2 Stable identity and project graph
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ARC-0010] MUST detect UUID collision on import. It shall reject ambiguous merge or perform an explicit, fully traced remap only when the import operation is defined to create independent objects.
~~~~

**Registry text**

~~~~text
MUST detect UUID collision on import. It shall reject ambiguous merge or perform an explicit, fully traced remap only when the import operation is defined to create independent objects.
~~~~

#### PES-ARC-0011

- **Selection basis:** TM-20
- **Page/section:** page 22; 8. Constitutional Architecture Invariants > 8.2 Stable identity and project graph
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ARC-0011] MUST maintain typed dependency edges and source/editor locations sufficient for where-used, caller/callee, type/DB usage, HMI binding, hardware-to-tag mapping, unresolved-reference filtering, and diagnostic navigation.
~~~~

**Registry text**

~~~~text
MUST maintain typed dependency edges and source/editor locations sufficient for where-used, caller/callee, type/DB usage, HMI binding, hardware-to-tag mapping, unresolved-reference filtering, and diagnostic navigation.
~~~~

#### PES-ARC-0012

- **Selection basis:** TM-20
- **Page/section:** page 22; 8. Constitutional Architecture Invariants > 8.3 Command, transaction, event, and audit model
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ARC-0012] MUST route create, rename, delete, restore, move, copy, retype, bind, connect, disconnect, configure, compile request, load request, CPU state change, modify, force, fault, reset, lesson action, and migration through typed domain commands or explicitly read-only queries.
~~~~

**Registry text**

~~~~text
MUST route create, rename, delete, restore, move, copy, retype, bind, connect, disconnect, configure, compile request, load request, CPU state change, modify, force, fault, reset, lesson action, and migration through typed domain commands or explicitly read-only queries.
~~~~

#### PES-ARC-0013

- **Selection basis:** SI-GATE-12, TM-20
- **Page/section:** page 22; 8. Constitutional Architecture Invariants > 8.3 Command, transaction, event, and audit model
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ARC-0013] MUST make commands atomic with respect to their declared transaction boundary. Failure shall leave either the previous valid state or a separately modeled unresolved/invalid engineering state, never a half-applied hidden mutation.
~~~~

**Registry text**

~~~~text
MUST make commands atomic with respect to their declared transaction boundary. Failure shall leave either the previous valid state or a separately modeled unresolved/invalid engineering state, never a half-applied hidden mutation.
~~~~

#### PES-ARC-0014

- **Selection basis:** TM-20
- **Page/section:** page 22; 8. Constitutional Architecture Invariants > 8.3 Command, transaction, event, and audit model
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ARC-0014] MUST make undo/redo use command/event semantics and restore exact stable identity where the original object is restored.
~~~~

**Registry text**

~~~~text
MUST make undo/redo use command/event semantics and restore exact stable identity where the original object is restored.
~~~~

#### PES-ARC-0015

- **Selection basis:** TM-20
- **Page/section:** page 22; 8. Constitutional Architecture Invariants > 8.3 Command, transaction, event, and audit model
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ARC-0015] MUST record deterministic event ordering, affected object IDs, before/after hashes, and command provenance sufficient for crash recovery, replay, Teacher Mode audit, and testing.
~~~~

**Registry text**

~~~~text
MUST record deterministic event ordering, affected object IDs, before/after hashes, and command provenance sufficient for crash recovery, replay, Teacher Mode audit, and testing.
~~~~

#### PES-ARC-0018

- **Selection basis:** sorted position 25
- **Page/section:** page 23; 8. Constitutional Architecture Invariants > 8.4 Canonical type system and semantic editors
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ARC-0018] MUST represent FBD as a typed port graph with stable node, port, and edge identity plus explicit execution dependencies.
~~~~

**Registry text**

~~~~text
MUST represent FBD as a typed port graph with stable node, port, and edge identity plus explicit execution dependencies.
~~~~

#### PES-ARC-0026

- **Selection basis:** TM-12
- **Page/section:** page 26; 8. Constitutional Architecture Invariants > 8.9 Internal buses and future seams
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ARC-0026] MUST define InternalTagBus as typed, quality-aware, timestamped internal publication/subscription. It shall operate by in-process calls or typed worker IPC, never localhost or network transport.
~~~~

**Registry text**

~~~~text
MUST define InternalTagBus as typed, quality-aware, timestamped internal publication/subscription. It shall operate by in-process calls or typed worker IPC, never localhost or network transport.
~~~~

#### PES-ARC-0030

- **Selection basis:** sorted position 37
- **Page/section:** page 26; 8. Constitutional Architecture Invariants > 8.9 Internal buses and future seams
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ARC-0030] MUST NOT satisfy "reserved architecture" with empty buttons, no-op objects, placeholder transports, user-visible coming-soon panels, or generic interfaces that create forbidden capability.
~~~~

**Registry text**

~~~~text
MUST NOT satisfy "reserved architecture" with empty buttons, no-op objects, placeholder transports, user-visible coming-soon panels, or generic interfaces that create forbidden capability.
~~~~

#### PES-CI-0001

- **Selection basis:** TM-18
- **Page/section:** page 28; 9. Binding Technology and Repository Foundation > 9.4 Baseline CI policy
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CI-0001] MUST fail production merge or release when:
a forbidden dependency or capability is added;
a prohibited source API or WASM import appears;
a remote asset, CDN, telemetry, analytics, or cloud dependency appears;
an asset lacks provenance or approval;
a vendor screenshot, logo, icon, device illustration, or copied prose enters production;
an unclassified research-derived requirement enters implementation;
a required test is skipped or flaky;
determinism/replay diverges;
migration loses identity or data;
a lesson bypasses ordinary domain/diagnostic behavior;
Virtual Download accepts any endpoint-like value;
HMI uses any transport other than InternalTagBus;
an exported artifact resembles or is accepted as a real industrial deployment artifact;
traceability between a verified requirement and its tests is missing.
~~~~

**Registry text**

~~~~text
MUST fail production merge or release when:
a forbidden dependency or capability is added;
a prohibited source API or WASM import appears;
a remote asset, CDN, telemetry, analytics, or cloud dependency appears;
an asset lacks provenance or approval;
a vendor screenshot, logo, icon, device illustration, or copied prose enters production;
an unclassified research-derived requirement enters implementation;
a required test is skipped or flaky;
determinism/replay diverges;
migration loses identity or data;
a lesson bypasses ordinary domain/diagnostic behavior;
Virtual Download accepts any endpoint-like value;
HMI uses any transport other than InternalTagBus;
an exported artifact resembles or is accepted as a real industrial deployment artifact;
traceability between a verified requirement and its tests is missing.
~~~~

#### PES-CI-0002

- **Selection basis:** SI-GATE-15, TM-19
- **Page/section:** page 29; 9. Binding Technology and Repository Foundation > 9.4 Baseline CI policy
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CI-0002] MUST scan the packaged artifact, not only source and lockfiles.
~~~~

**Registry text**

~~~~text
MUST scan the packaged artifact, not only source and lockfiles.
~~~~

#### PES-CI-0003

- **Selection basis:** SI-GATE-15, TM-19
- **Page/section:** page 29; 9. Binding Technology and Repository Foundation > 9.4 Baseline CI policy
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CI-0003] MUST produce an SBOM, license notice set, asset manifest, requirement-verification report, and isolation report for a release candidate.
~~~~

**Registry text**

~~~~text
MUST produce an SBOM, license notice set, asset manifest, requirement-verification report, and isolation report for a release candidate.
~~~~

#### PES-CRM-0001

- **Selection basis:** TM-19
- **Page/section:** page 15; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.1 Independent implementation
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0001] MUST use original expression and independent implementation.
~~~~

**Registry text**

~~~~text
MUST use original expression and independent implementation.
~~~~

#### PES-CRM-0002

- **Selection basis:** TM-19
- **Page/section:** page 15; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.1 Independent implementation
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0002] MUST treat educational purpose as the mission, not permission to copy and not a substitute for legal analysis.
~~~~

**Registry text**

~~~~text
MUST treat educational purpose as the mission, not permission to copy and not a substitute for legal analysis.
~~~~

#### PES-CRM-0003

- **Selection basis:** TM-19
- **Page/section:** page 15; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.1 Independent implementation
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0003] MAY independently implement functional concepts such as compilation, project dependencies, scan execution, tag resolution, stateful blocks, hardware consistency, online/offline state, watch/force semantics, diagnostic navigation, contextual properties, and generic commands.
~~~~

**Registry text**

~~~~text
MAY independently implement functional concepts such as compilation, project dependencies, scan execution, tag resolution, stateful blocks, hardware consistency, online/offline state, watch/force semantics, diagnostic navigation, contextual properties, and generic commands.
~~~~

#### PES-CRM-0004

- **Selection basis:** TM-19
- **Page/section:** page 16; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.1 Independent implementation
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0004] MUST NOT copy Siemens screens, layout composition, icons, help prose, diagnostic prose or numbers, artwork, device illustrations, completion databases, project formats, compiler components, firmware behavior, or proprietary algorithms.
~~~~

**Registry text**

~~~~text
MUST NOT copy Siemens screens, layout composition, icons, help prose, diagnostic prose or numbers, artwork, device illustrations, completion databases, project formats, compiler components, firmware behavior, or proprietary algorithms.
~~~~

#### PES-CRM-0005

- **Selection basis:** TM-19
- **Page/section:** page 16; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.1 Independent implementation
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0005] MUST use original names, event codes, visual language, device identities, project structures, schemas, source representations, sample projects, and user documentation.
~~~~

**Registry text**

~~~~text
MUST use original names, event codes, visual language, device identities, project structures, schemas, source representations, sample projects, and user documentation.
~~~~

#### PES-CRM-0006

- **Selection basis:** TM-19
- **Page/section:** page 16; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.2 IP classification
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0006] MUST default an unclassified or uncertain item to Class 8, not "probably permitted."
~~~~

**Registry text**

~~~~text
MUST default an unclassified or uncertain item to Class 8, not "probably permitted."
~~~~

#### PES-CRM-0007

- **Selection basis:** TM-19
- **Page/section:** page 16; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.2 IP classification
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0007] MUST NOT begin implementation of a research-derived behavior until its requirement record contains an IP classification and disposition.
~~~~

**Registry text**

~~~~text
MUST NOT begin implementation of a research-derived behavior until its requirement record contains an IP classification and disposition.
~~~~

#### PES-CRM-0008

- **Selection basis:** TM-19
- **Page/section:** page 16; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.3 Permitted and forbidden sources
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0008] MAY use:
public Siemens documentation, SCE material, and public product/support pages as behavioral evidence;
IEC descriptions or standards lawfully licensed to the team;
public statutes and published judicial opinions;
independent textbooks and tutorials for corroboration;
independently created observations only under a written, counsel-approved observation protocol.
~~~~

**Registry text**

~~~~text
MAY use:
public Siemens documentation, SCE material, and public product/support pages as behavioral evidence;
IEC descriptions or standards lawfully licensed to the team;
public statutes and published judicial opinions;
independent textbooks and tutorials for corroboration;
independently created observations only under a written, counsel-approved observation protocol.
~~~~

#### PES-CRM-0009

- **Selection basis:** sorted position 49, TM-19
- **Page/section:** page 16; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.3 Permitted and forbidden sources
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0009] MUST NOT use:
Siemens source code, leaked code, leaked manuals, partner-only material, or confidential training material;
decompiled or disassembled output;
executable resources, extracted icons, resource packages, or memory scraping;
protocol captures intended to reproduce vendor communications;
encrypted project-format cracking;
pirated software, license bypass, access-control circumvention, or API hooking;
screenshots, manual diagrams, copied tables, copied hardware illustrations, or copied diagnostic text as implementation assets.
~~~~

**Registry text**

~~~~text
MUST NOT use:
Siemens source code, leaked code, leaked manuals, partner-only material, or confidential training material;
decompiled or disassembled output;
executable resources, extracted icons, resource packages, or memory scraping;
protocol captures intended to reproduce vendor communications;
encrypted project-format cracking;
pirated software, license bypass, access-control circumvention, or API hooking;
screenshots, manual diagrams, copied tables, copied hardware illustrations, or copied diagnostic text as implementation assets.
~~~~

#### PES-CRM-0010

- **Selection basis:** TM-14, TM-19
- **Page/section:** page 17; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.3 Permitted and forbidden sources
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0010] MUST prohibit observation of an installed TIA Portal product for implementation verification until counsel reviews the applicable license terms and approves a written observation procedure.
~~~~

**Registry text**

~~~~text
MUST prohibit observation of an installed TIA Portal product for implementation verification until counsel reviews the applicable license terms and approves a written observation procedure.
~~~~

#### PES-CRM-0011

- **Selection basis:** TM-19
- **Page/section:** page 17; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.3 Permitted and forbidden sources
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0011] MUST keep screenshots and vendor assets out of production source, design files, tickets, code-generation prompts, training corpora, mockups, and asset pipelines unless counsel approves a quarantined evidence process. Quarantined evidence shall never be shipped.
~~~~

**Registry text**

~~~~text
MUST keep screenshots and vendor assets out of production source, design files, tickets, code-generation prompts, training corpora, mockups, and asset pipelines unless counsel approves a quarantined evidence process. Quarantined evidence shall never be shipped.
~~~~

#### PES-CRM-0012

- **Selection basis:** TM-19
- **Page/section:** page 17; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.4 Trademark, trade dress, and public language
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0012] MUST NOT use Siemens, SIMATIC, TIA Portal, S7, WinCC, or PLCSIM marks as product identity, catalog identity, installer branding, repository branding, splash-screen branding, store listing, domain name, or implied affiliation.
~~~~

**Registry text**

~~~~text
MUST NOT use Siemens, SIMATIC, TIA Portal, S7, WinCC, or PLCSIM marks as product identity, catalog identity, installer branding, repository branding, splash-screen branding, store listing, domain name, or implied affiliation.
~~~~

#### PES-CRM-0013

- **Selection basis:** TM-19
- **Page/section:** page 17; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.4 Trademark, trade dress, and public language
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0013] MUST NOT copy or sample a Siemens color system, icon silhouette family, device illustration style, typography, spacing system, screen composition, or overall trade dress.
~~~~

**Registry text**

~~~~text
MUST NOT copy or sample a Siemens color system, icon silhouette family, device illustration style, typography, spacing system, screen composition, or overall trade dress.
~~~~

#### PES-CRM-0014

- **Selection basis:** TM-19
- **Page/section:** page 17; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.4 Trademark, trade dress, and public language
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0014] MUST hold every public comparative statement mentioning Siemens or TIA Portal as BLOCKED until trademark counsel approves the exact wording and notices.
~~~~

**Registry text**

~~~~text
MUST hold every public comparative statement mentioning Siemens or TIA Portal as BLOCKED until trademark counsel approves the exact wording and notices.
~~~~

#### PES-CRM-0015

- **Selection basis:** TM-19
- **Page/section:** page 17; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.4 Trademark, trade dress, and public language
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0015] MUST treat the working title in this directive as descriptive only. It is not public-name clearance.
~~~~

**Registry text**

~~~~text
MUST treat the working title in this directive as descriptive only. It is not public-name clearance.
~~~~

#### PES-CRM-0016

- **Selection basis:** TM-19
- **Page/section:** page 17; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.5 Evidence and contamination control
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0016] MUST create CLEAN_ROOM_POLICY.md before feature implementation.
~~~~

**Registry text**

~~~~text
MUST create CLEAN_ROOM_POLICY.md before feature implementation.
~~~~

#### PES-CRM-0017

- **Selection basis:** TM-19
- **Page/section:** page 17; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.5 Evidence and contamination control
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0017] MUST maintain a requirement evidence register with:
requirement ID;
paraphrased observed behavior;
source title, publisher, version/date, durable location, and access date;
report classification;
IP class and disposition;
simulator-owned implementation requirement;
forbidden shortcut;
author;
reviewer;
review status and date;
implementation component;
verification IDs.
~~~~

**Registry text**

~~~~text
MUST maintain a requirement evidence register with:
requirement ID;
paraphrased observed behavior;
source title, publisher, version/date, durable location, and access date;
report classification;
IP class and disposition;
simulator-owned implementation requirement;
forbidden shortcut;
author;
reviewer;
review status and date;
implementation component;
verification IDs.
~~~~

#### PES-CRM-0018

- **Selection basis:** TM-19
- **Page/section:** page 18; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.5 Evidence and contamination control
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0018] MUST quarantine a contribution suspected of contamination. It shall not enter builds, prompts, generated assets, or derived work until reviewed.
~~~~

**Registry text**

~~~~text
MUST quarantine a contribution suspected of contamination. It shall not enter builds, prompts, generated assets, or derived work until reviewed.
~~~~

#### PES-CRM-0019

- **Selection basis:** TM-19
- **Page/section:** page 18; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.5 Evidence and contamination control
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0019] MUST perform a clean rewrite without reusing tainted code, prose, assets, naming, layouts, or extracted structure when contamination is confirmed.
~~~~

**Registry text**

~~~~text
MUST perform a clean rewrite without reusing tainted code, prose, assets, naming, layouts, or extracted structure when contamination is confirmed.
~~~~

#### PES-CRM-0020

- **Selection basis:** TM-19
- **Page/section:** page 18; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.5 Evidence and contamination control
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0020] MUST require contributor attestation that no forbidden source, asset, reverse-engineering output, protocol capture, or confidential material was used.
~~~~

**Registry text**

~~~~text
MUST require contributor attestation that no forbidden source, asset, reverse-engineering output, protocol capture, or confidential material was used.
~~~~

#### PES-CRM-0021

- **Selection basis:** sorted position 61, SI-GATE-15, TM-19
- **Page/section:** page 18; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.6 Asset and dependency provenance
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0021] MUST register every shipped image, icon, font, sound, animation, template, sample project, translation, and other non-code asset with:
asset ID;
author/source;
license and evidence location;
created date;
hash algorithm and original hash;
derivative chain and modifications;
generated-asset disclosure where applicable;
reviewer, review status, and approval date.
~~~~

**Registry text**

~~~~text
MUST register every shipped image, icon, font, sound, animation, template, sample project, translation, and other non-code asset with:
asset ID;
author/source;
license and evidence location;
created date;
hash algorithm and original hash;
derivative chain and modifications;
generated-asset disclosure where applicable;
reviewer, review status, and approval date.
~~~~

#### PES-CRM-0022

- **Selection basis:** SI-GATE-15, TM-19
- **Page/section:** page 18; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.6 Asset and dependency provenance
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0022] MUST reject unregistered or unapproved assets in CI.
~~~~

**Registry text**

~~~~text
MUST reject unregistered or unapproved assets in CI.
~~~~

#### PES-CRM-0023

- **Selection basis:** TM-19
- **Page/section:** page 18; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.6 Asset and dependency provenance
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0023] MUST NOT trace screenshots or icons, redraw vendor artwork, recolor vendor assets, or sample vendor branding as proof of originality.
~~~~

**Registry text**

~~~~text
MUST NOT trace screenshots or icons, redraw vendor artwork, recolor vendor assets, or sample vendor branding as proof of originality.
~~~~

#### PES-CRM-0024

- **Selection basis:** TM-19
- **Page/section:** page 18; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.6 Asset and dependency provenance
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0024] MUST generate an SBOM for release artifacts and review direct, transitive, optional, native, font, and asset licenses.
~~~~

**Registry text**

~~~~text
MUST generate an SBOM for release artifacts and review direct, transitive, optional, native, font, and asset licenses.
~~~~

#### PES-CRM-0025

- **Selection basis:** TM-19
- **Page/section:** page 18; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.6 Asset and dependency provenance
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-CRM-0025] MUST block dependencies whose license obligations are incompatible with the intended distribution or cannot be satisfied and documented.
~~~~

**Registry text**

~~~~text
MUST block dependencies whose license obligations are incompatible with the intended distribution or cannot be satisfied and documented.
~~~~

#### PES-DET-0001

- **Selection basis:** TM-21
- **Page/section:** page 24; 8. Constitutional Architecture Invariants > 8.6 Deterministic virtual time, scheduling, and replay
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-DET-0001] MUST use simulator-controlled monotonic virtual time for the PLC scheduler, timers, counters with temporal behavior, process physics, trace, scenarios, lesson triggers, and assessment timing.
~~~~

**Registry text**

~~~~text
MUST use simulator-controlled monotonic virtual time for the PLC scheduler, timers, counters with temporal behavior, process physics, trace, scenarios, lesson triggers, and assessment timing.
~~~~

#### PES-DET-0002

- **Selection basis:** sorted position 73, TM-21
- **Page/section:** page 24; 8. Constitutional Architecture Invariants > 8.6 Deterministic virtual time, scheduling, and replay
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-DET-0002] MUST NOT use wall-clock timers such as browser setTimeout as authoritative PLC or process time.
~~~~

**Registry text**

~~~~text
MUST NOT use wall-clock timers such as browser setTimeout as authoritative PLC or process time.
~~~~

#### PES-DET-0003

- **Selection basis:** TM-21
- **Page/section:** page 24; 8. Constitutional Architecture Invariants > 8.6 Deterministic virtual time, scheduling, and replay
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-DET-0003] MUST define stable ordering for events sharing the same virtual timestamp and priority.
~~~~

**Registry text**

~~~~text
MUST define stable ordering for events sharing the same virtual timestamp and priority.
~~~~

#### PES-DET-0004

- **Selection basis:** TM-21
- **Page/section:** page 24; 8. Constitutional Architecture Invariants > 8.6 Deterministic virtual time, scheduling, and replay
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-DET-0004] MUST include deterministic seed, event sequence, TrainingProfile hash, build hash, initial snapshot hash, simulator version, and scheduler version in replay identity.
~~~~

**Registry text**

~~~~text
MUST include deterministic seed, event sequence, TrainingProfile hash, build hash, initial snapshot hash, simulator version, and scheduler version in replay identity.
~~~~

#### PES-DET-0005

- **Selection basis:** TM-21
- **Page/section:** page 24; 8. Constitutional Architecture Invariants > 8.6 Deterministic virtual time, scheduling, and replay
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-DET-0005] MUST distinguish virtual timestamp from engineering-display wall-clock timestamp.
~~~~

**Registry text**

~~~~text
MUST distinguish virtual timestamp from engineering-display wall-clock timestamp.
~~~~

#### PES-DET-0006

- **Selection basis:** TM-21
- **Page/section:** page 24; 8. Constitutional Architecture Invariants > 8.6 Deterministic virtual time, scheduling, and replay
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-DET-0006] MUST guarantee that the same supported build, snapshot, profile, seed, and ordered events produce equivalent observable tag streams, outputs, diagnostics, trace data, HMI updates, and assessment results.
~~~~

**Registry text**

~~~~text
MUST guarantee that the same supported build, snapshot, profile, seed, and ordered events produce equivalent observable tag streams, outputs, diagnostics, trace data, HMI updates, and assessment results.
~~~~

#### PES-DET-0007

- **Selection basis:** TM-21
- **Page/section:** page 24; 8. Constitutional Architecture Invariants > 8.6 Deterministic virtual time, scheduling, and replay
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-DET-0007] MUST reserve scan-start, input-sample, program-execution, output-commit, process-update, trace/diagnostic/HMI publication, and scan-end boundaries.
~~~~

**Registry text**

~~~~text
MUST reserve scan-start, input-sample, program-execution, output-commit, process-update, trace/diagnostic/HMI publication, and scan-end boundaries.
~~~~

#### PES-DEV-0005

- **Selection basis:** TM-21
- **Page/section:** page 27; 9. Binding Technology and Repository Foundation > 9.1 Adopted stack
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-DEV-0005] MUST execute virtual runtime/process work in isolated workers using typed messages so simulation cannot freeze the UI.
~~~~

**Registry text**

~~~~text
MUST execute virtual runtime/process work in isolated workers using typed messages so simulation cannot freeze the UI.
~~~~

#### PES-DEV-0007

- **Selection basis:** sorted position 85
- **Page/section:** page 27; 9. Binding Technology and Repository Foundation > 9.1 Adopted stack
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-DEV-0007] MUST bundle all production dependencies, fonts, WASM, help, scenarios, and assets locally.
~~~~

**Registry text**

~~~~text
MUST bundle all production dependencies, fonts, WASM, help, scenarios, and assets locally.
~~~~

#### PES-DEV-0012

- **Selection basis:** TM-12
- **Page/section:** page 28; 9. Binding Technology and Repository Foundation > 9.3 Required package boundaries
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-DEV-0012] MUST NOT create a network, transport, device-connector, vendor-adapter, protocol, external-HMI, remote-collaboration, or plugin-host package.
~~~~

**Registry text**

~~~~text
MUST NOT create a network, transport, device-connector, vendor-adapter, protocol, external-HMI, remote-collaboration, or plugin-host package.
~~~~

#### PES-DIA-0003

- **Selection basis:** TM-15
- **Page/section:** page 26; 8. Constitutional Architecture Invariants > 8.8 Diagnostics and causal faults
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-DIA-0003] MUST derive diagnostics from ordinary validators, compiler rules, runtime transitions, device/process state, HMI consistency, persistence validation, or fault providers.
~~~~

**Registry text**

~~~~text
MUST derive diagnostics from ordinary validators, compiler rules, runtime transitions, device/process state, HMI consistency, persistence validation, or fault providers.
~~~~

#### PES-DIA-0004

- **Selection basis:** TM-15
- **Page/section:** page 26; 8. Constitutional Architecture Invariants > 8.8 Diagnostics and causal faults
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-DIA-0004] MUST let Teacher Mode invoke commands such as RemoveModule, DisconnectVirtualLink, ChangeTagType, SetSensorFault, or SetActuatorFault and let ordinary engines derive the consequence.
~~~~

**Registry text**

~~~~text
MUST let Teacher Mode invoke commands such as RemoveModule, DisconnectVirtualLink, ChangeTagType, SetSensorFault, or SetActuatorFault and let ordinary engines derive the consequence.
~~~~

#### PES-DIA-0005

- **Selection basis:** SI-GATE-13, TM-15
- **Page/section:** page 26; 8. Constitutional Architecture Invariants > 8.8 Diagnostics and causal faults
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-DIA-0005] MUST NOT let a scenario, lesson, demo, or UI directly insert an expected compiler diagnostic, runtime fault, alarm, trace, monitored value, or passing assessment result.
~~~~

**Registry text**

~~~~text
MUST NOT let a scenario, lesson, demo, or UI directly insert an expected compiler diagnostic, runtime fault, alarm, trace, monitored value, or passing assessment result.
~~~~

#### PES-DOC-0001

- **Selection basis:** sorted position 97
- **Page/section:** page 27; 9. Binding Technology and Repository Foundation > 9.2 Required top-level governance files
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-DOC-0001] MUST create ADR-0001 with title "Physical Industrial Communication Is Permanently Out of Scope" and status "Project Safety Invariant."
~~~~

**Registry text**

~~~~text
MUST create ADR-0001 with title "Physical Industrial Communication Is Permanently Out of Scope" and status "Project Safety Invariant."
~~~~

#### PES-DOC-0004

- **Selection basis:** SI-GATE-15, TM-19
- **Page/section:** page 28; 9. Binding Technology and Repository Foundation > 9.2 Required top-level governance files
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-DOC-0004] MUST keep evidence records and research notes separate from production assets.
~~~~

**Registry text**

~~~~text
MUST keep evidence records and research notes separate from production assets.
~~~~

#### PES-EDU-0004

- **Selection basis:** TM-15
- **Page/section:** page 8; 3. Canonical Vocabulary, Modes, and Claims > 3.2 Product modes share one kernel
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-EDU-0004] MUST make Teacher Mode act through ordinary domain commands, scenario events, process faults, and virtual hardware faults.
~~~~

**Registry text**

~~~~text
MUST make Teacher Mode act through ordinary domain commands, scenario events, process faults, and virtual hardware faults.
~~~~

#### PES-EDU-0005

- **Selection basis:** TM-15
- **Page/section:** page 8; 3. Canonical Vocabulary, Modes, and Claims > 3.2 Product modes share one kernel
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-EDU-0005] MUST NOT let Teacher Mode insert compiler errors, runtime diagnostics, HMI alarms, expected values, or "correct" program state directly.
~~~~

**Registry text**

~~~~text
MUST NOT let Teacher Mode insert compiler errors, runtime diagnostics, HMI alarms, expected values, or "correct" program state directly.
~~~~

#### PES-FID-0003

- **Selection basis:** sorted position 109
- **Page/section:** page 9; 4. Scope, Non-Goals, and Fidelity Doctrine > 4.1 Required fidelity
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-FID-0003] MUST preserve meaningful distinctions that commercial engineering software teaches, including saved versus unsaved, built versus dirty, hardware build versus software build, offline source versus loaded artifact, loaded versus matching, RUN versus STOP, monitoring off versus on, raw process value versus CPU-visible value, modify versus force, initial versus actual versus retained value, and incoming versus cleared diagnostics.
~~~~

**Registry text**

~~~~text
MUST preserve meaningful distinctions that commercial engineering software teaches, including saved versus unsaved, built versus dirty, hardware build versus software build, offline source versus loaded artifact, loaded versus matching, RUN versus STOP, monitoring off versus on, raw process value versus CPU-visible value, modify versus force, initial versus actual versus retained value, and incoming versus cleared diagnostics.
~~~~

#### PES-GOV-0007

- **Selection basis:** sorted position 121
- **Page/section:** page 5; 1. Authority and Conflict Resolution > 1.2 Conflict protocol
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-GOV-0007] MUST NOT resolve an authority conflict by selecting the easiest implementation, the closest vendor behavior, or the broadest feature scope.
~~~~

**Registry text**

~~~~text
MUST NOT resolve an authority conflict by selecting the easiest implementation, the closest vendor behavior, or the broadest feature scope.
~~~~

#### PES-GOV-0019

- **Selection basis:** sorted position 133
- **Page/section:** page 35; 13. Living-Directive Governance and Phase 1 Exit Gate > 13.1 One document, four authoring phases
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-GOV-0019] MUST label unauthored later-phase material as reserved. It shall not create empty chapters that could be mistaken for complete requirements.
~~~~

**Registry text**

~~~~text
MUST label unauthored later-phase material as reserved. It shall not create empty chapters that could be mistaken for complete requirements.
~~~~

#### PES-ISO-0005

- **Selection basis:** TM-13
- **Page/section:** page 12; 5. Immutable VirtualUniverse Safety Wall > 5.1 Constitutional invariant
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ISO-0005] MUST implement fictional device discovery only as an in-memory query whose result is a subset of VirtualUniverse devices.
~~~~

**Registry text**

~~~~text
MUST implement fictional device discovery only as an in-memory query whose result is a subset of VirtualUniverse devices.
~~~~

#### PES-ISO-0006

- **Selection basis:** sorted position 145, TM-14
- **Page/section:** page 12; 5. Immutable VirtualUniverse Safety Wall > 5.1 Constitutional invariant
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ISO-0006] MUST implement Virtual Download only as an atomic internal build-artifact transaction against a VirtualControllerId.
~~~~

**Registry text**

~~~~text
MUST implement Virtual Download only as an atomic internal build-artifact transaction against a VirtualControllerId.
~~~~

#### PES-ISO-0007

- **Selection basis:** TM-12
- **Page/section:** page 12; 5. Immutable VirtualUniverse Safety Wall > 5.1 Constitutional invariant
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ISO-0007] MUST implement controller/process/HMI value exchange only through typed internal messages and InternalTagBus.
~~~~

**Registry text**

~~~~text
MUST implement controller/process/HMI value exchange only through typed internal messages and InternalTagBus.
~~~~

#### PES-ISO-0011

- **Selection basis:** TM-22
- **Page/section:** page 14; 5. Immutable VirtualUniverse Safety Wall > 5.7 Release-blocking isolation proof
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ISO-0011] MUST make every isolation test release-blocking. Skipped, unavailable, flaky, or inconclusive isolation tests equal failure.
~~~~

**Registry text**

~~~~text
MUST make every isolation test release-blocking. Skipped, unavailable, flaky, or inconclusive isolation tests equal failure.
~~~~

#### PES-ISO-0018

- **Selection basis:** sorted position 157, TM-13
- **Page/section:** page 15; 5. Immutable VirtualUniverse Safety Wall > 5.7 Release-blocking isolation proof
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ISO-0018] MUST prove that device discovery results remain unchanged in the presence of a live LAN containing real or PLC-like devices.
~~~~

**Registry text**

~~~~text
MUST prove that device discovery results remain unchanged in the presence of a live LAN containing real or PLC-like devices.
~~~~

#### PES-ISO-0019

- **Selection basis:** TM-14
- **Page/section:** page 15; 5. Immutable VirtualUniverse Safety Wall > 5.7 Release-blocking isolation proof
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ISO-0019] MUST prove at type, deserialization, reflection, and UI boundaries that Virtual Download accepts only VirtualControllerId.
~~~~

**Registry text**

~~~~text
MUST prove at type, deserialization, reflection, and UI boundaries that Virtual Download accepts only VirtualControllerId.
~~~~

#### PES-ISO-0020

- **Selection basis:** TM-12
- **Page/section:** page 15; 5. Immutable VirtualUniverse Safety Wall > 5.7 Release-blocking isolation proof
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ISO-0020] MUST prove every HMI binding resolves only through InternalTagBus.
~~~~

**Registry text**

~~~~text
MUST prove every HMI binding resolves only through InternalTagBus.
~~~~

#### PES-ISO-0021

- **Selection basis:** TM-14
- **Page/section:** page 15; 5. Immutable VirtualUniverse Safety Wall > 5.7 Release-blocking isolation proof
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ISO-0021] MUST prove exports contain no vendor project, firmware, load binary, deployable industrial payload, protocol frame, executable, or file directly accepted by a physical industrial tool.
~~~~

**Registry text**

~~~~text
MUST prove exports contain no vendor project, firmware, load binary, deployable industrial payload, protocol frame, executable, or file directly accepted by a physical industrial tool.
~~~~

#### PES-ISO-0022

- **Selection basis:** SI-GATE-14, TM-22
- **Page/section:** page 15; 5. Immutable VirtualUniverse Safety Wall > 5.7 Release-blocking isolation proof
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-ISO-0022] MUST retain machine-readable evidence for each isolation gate with artifact hash, test version, date, platform, result, and logs sufficient to reproduce the test.
~~~~

**Registry text**

~~~~text
MUST retain machine-readable evidence for each isolation gate with artifact hash, test version, date, platform, result, and logs sufficient to reproduce the test.
~~~~

#### PES-MSN-0007

- **Selection basis:** TM-18
- **Page/section:** page 7; 2. Product Charter > 2.2 Intended users and environments
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-MSN-0007] MUST operate in an offline classroom or home-study environment with no cloud service, remote font, CDN, telemetry service, license server, analytics endpoint, or internet connection required.
~~~~

**Registry text**

~~~~text
MUST operate in an offline classroom or home-study environment with no cloud service, remote font, CDN, telemetry service, license server, analytics endpoint, or internet connection required.
~~~~

#### PES-MSN-0008

- **Selection basis:** sorted position 169
- **Page/section:** page 7; 2. Product Charter > 2.2 Intended users and environments
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-MSN-0008] MUST support a professional unassisted workflow for students and a separate explanatory/teaching experience without changing the underlying engineering semantics.
~~~~

**Registry text**

~~~~text
MUST support a professional unassisted workflow for students and a separate explanatory/teaching experience without changing the underlying engineering semantics.
~~~~

#### PES-PRJ-0001

- **Selection basis:** TM-14
- **Page/section:** page 19; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.7 Original native file boundary
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-PRJ-0001] MUST use only simulator-native, brand-neutral project and archive formats.
~~~~

**Registry text**

~~~~text
MUST use only simulator-native, brand-neutral project and archive formats.
~~~~

#### PES-PRJ-0002

- **Selection basis:** TM-14
- **Page/section:** page 19; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.7 Original native file boundary
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-PRJ-0002] MUST use the provisional internal extensions .vlabproj for a project package and .vlabarchive for an archive until an approved product-name decision replaces them.
~~~~

**Registry text**

~~~~text
MUST use the provisional internal extensions .vlabproj for a project package and .vlabarchive for an archive until an approved product-name decision replaces them.
~~~~

#### PES-PRJ-0003

- **Selection basis:** TM-14, TM-20
- **Page/section:** page 19; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.7 Original native file boundary
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-PRJ-0003] MUST make a project package a documented, versioned, non-executable container of canonical UTF-8 data and simulator-owned binary-neutral assets. A ZIP container is permitted only with the archive defenses in this directive.
~~~~

**Registry text**

~~~~text
MUST make a project package a documented, versioned, non-executable container of canonical UTF-8 data and simulator-owned binary-neutral assets. A ZIP container is permitted only with the archive defenses in this directive.
~~~~

#### PES-PRJ-0004

- **Selection basis:** TM-14, TM-20
- **Page/section:** page 19; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.7 Original native file boundary
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-PRJ-0004] MUST place a manifest in every project/archive containing schema version, pinned TrainingProfile ID and version, object-index version, required capabilities, file inventory, SHA-256 hashes, creation application version, and migration history.
~~~~

**Registry text**

~~~~text
MUST place a manifest in every project/archive containing schema version, pinned TrainingProfile ID and version, object-index version, required capabilities, file inventory, SHA-256 hashes, creation application version, and migration history.
~~~~

#### PES-PRJ-0005

- **Selection basis:** TM-14, TM-20
- **Page/section:** page 19; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.7 Original native file boundary
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-PRJ-0005] MUST make project integrity failures visible and fail closed. It shall not silently discard unknown, corrupt, oversized, or hash-mismatched content.
~~~~

**Registry text**

~~~~text
MUST make project integrity failures visible and fail closed. It shall not silently discard unknown, corrupt, oversized, or hash-mismatched content.
~~~~

#### PES-PRJ-0006

- **Selection basis:** TM-14
- **Page/section:** page 19; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.7 Original native file boundary
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-PRJ-0006] MUST NOT use .apXX, .zapXX, a Siemens library format, PLCopen XML, vendor source export, or another real-tool format unless a later separately researched and legally approved directive explicitly adds a non-physical interoperability feature. No such approval exists in this revision.
~~~~

**Registry text**

~~~~text
MUST NOT use .apXX, .zapXX, a Siemens library format, PLCopen XML, vendor source export, or another real-tool format unless a later separately researched and legally approved directive explicitly adds a non-physical interoperability feature. No such approval exists in this revision.
~~~~

#### PES-PRJ-0007

- **Selection basis:** TM-14
- **Page/section:** page 19; 6. Clean-Room, IP, Trademark, and Evidence Policy > 6.7 Original native file boundary
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-PRJ-0007] MUST distinguish simulator-native CSV/JSON interchange from vendor or physical deployment. Native exports shall contain no executable code and shall be documented as simulator-only.
~~~~

**Registry text**

~~~~text
MUST distinguish simulator-native CSV/JSON interchange from vendor or physical deployment. Native exports shall contain no executable code and shall be documented as simulator-only.
~~~~

#### PES-PROF-0004

- **Selection basis:** sorted position 181
- **Page/section:** page 9; 3. Canonical Vocabulary, Modes, and Claims > 3.3 Profile and version claims
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-PROF-0004] MUST pin a project's selected profile and capability-manifest version. Opening or migrating a project shall not silently change runtime semantics.
~~~~

**Registry text**

~~~~text
MUST pin a project's selected profile and capability-manifest version. Opening or migrating a project shall not silently change runtime semantics.
~~~~

#### PES-QLT-0005

- **Selection basis:** TM-11
- **Page/section:** page 34; 12. No-Fake-Completion and Anti-Placeholder Policy > 12.2 Permitted scaffolding
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-QLT-0005] MUST NOT create an abstract physical connection, generic transport, executable plugin host, network-capable HMI provider, or arbitrary scripting engine even as scaffolding.
~~~~

**Registry text**

~~~~text
MUST NOT create an abstract physical connection, generic transport, executable plugin host, network-capable HMI provider, or arbitrary scripting engine even as scaffolding.
~~~~

#### PES-QLT-0006

- **Selection basis:** SI-GATE-14
- **Page/section:** page 34; 12. No-Fake-Completion and Anti-Placeholder Policy > 12.3 Universal milestone Definition of Done
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-QLT-0006] MUST require every software milestone, when later authorized, to include:
domain model and ownership;
invariants;
positive behavior;
negative behavior;
enumerated failure cases and recovery;
stable identity and dependency behavior;
persistence, migration, and undo where applicable;
real UI integration where applicable;
end-to-end workflow;
deterministic unit/integration tests;
property/fuzz/golden tests where applicable;
isolation/security tests;
clean-room evidence and asset provenance;
documentation;
requirement-to-test traceability;
reproducible verification evidence.
~~~~

**Registry text**

~~~~text
MUST require every software milestone, when later authorized, to include:
domain model and ownership;
invariants;
positive behavior;
negative behavior;
enumerated failure cases and recovery;
stable identity and dependency behavior;
persistence, migration, and undo where applicable;
real UI integration where applicable;
end-to-end workflow;
deterministic unit/integration tests;
property/fuzz/golden tests where applicable;
isolation/security tests;
clean-room evidence and asset provenance;
documentation;
requirement-to-test traceability;
reproducible verification evidence.
~~~~

#### PES-QLT-0008

- **Selection basis:** TM-22
- **Page/section:** page 34; 12. No-Fake-Completion and Anti-Placeholder Policy > 12.3 Universal milestone Definition of Done
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-QLT-0008] MUST keep a milestone open if any required test is skipped, flaky, unavailable, manually waived, or inconclusive.
~~~~

**Registry text**

~~~~text
MUST keep a milestone open if any required test is skipped, flaky, unavailable, manually waived, or inconclusive.
~~~~

#### PES-REQ-0002

- **Selection basis:** sorted position 193
- **Page/section:** page 29; 10. Normative Requirement, Decision, and Change System > 10.1 Stable identifiers
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-REQ-0002] MUST NOT encode authoring phase, software release, priority, status, or document section in a requirement ID.
~~~~

**Registry text**

~~~~text
MUST NOT encode authoring phase, software release, priority, status, or document section in a requirement ID.
~~~~

#### PES-SCP-0005

- **Selection basis:** sorted position 205
- **Page/section:** page 11; 4. Scope, Non-Goals, and Fidelity Doctrine > 4.3 Permanently excluded
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-SCP-0005] MUST NOT provide safety-rated programming, validation, certification, or claims. Safety objects, safety instruction sets, and safety certification simulation are excluded from Phases 1-4 unless Scott approves a separately researched legal/domain addendum. Ordinary educational interlocks shall be labeled non-safety-rated.
~~~~

**Registry text**

~~~~text
MUST NOT provide safety-rated programming, validation, certification, or claims. Safety objects, safety instruction sets, and safety certification simulation are excluded from Phases 1-4 unless Scott approves a separately researched legal/domain addendum. Ordinary educational interlocks shall be labeled non-safety-rated.
~~~~

#### PES-SCP-0006

- **Selection basis:** TM-18
- **Page/section:** page 11; 4. Scope, Non-Goals, and Fidelity Doctrine > 4.3 Permanently excluded
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-SCP-0006] MUST NOT provide remote collaboration, a cloud project server, telemetry, cloud grading, cloud AI, or a production local HTTP/WebSocket server.
~~~~

**Registry text**

~~~~text
MUST NOT provide remote collaboration, a cloud project server, telemetry, cloud grading, cloud AI, or a production local HTTP/WebSocket server.
~~~~

#### PES-SEC-0006

- **Selection basis:** TM-18
- **Page/section:** page 13; 5. Immutable VirtualUniverse Safety Wall > 5.4 Production versus development boundary
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-SEC-0006] MUST build the production classroom application without a local web server. Assets, workers, fonts, WASM, help, and examples shall be bundled and loaded locally without HTTP or WebSocket.
~~~~

**Registry text**

~~~~text
MUST build the production classroom application without a local web server. Assets, workers, fonts, WASM, help, and examples shall be bundled and loaded locally without HTTP or WebSocket.
~~~~

#### PES-SEC-0007

- **Selection basis:** sorted position 217, TM-18
- **Page/section:** page 13; 5. Immutable VirtualUniverse Safety Wall > 5.4 Production versus development boundary
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-SEC-0007] MUST enforce a production Content Security Policy with at least connect-src 'none' and default-deny restrictions on external scripts, styles, fonts, images, media, objects, frames, forms, manifests, base-URI changes, and unsolicited navigation.
~~~~

**Registry text**

~~~~text
MUST enforce a production Content Security Policy with at least connect-src 'none' and default-deny restrictions on external scripts, styles, fonts, images, media, objects, frames, forms, manifests, base-URI changes, and unsolicited navigation.
~~~~

#### PES-SEC-0008

- **Selection basis:** TM-18
- **Page/section:** page 14; 5. Immutable VirtualUniverse Safety Wall > 5.4 Production versus development boundary
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-SEC-0008] MUST NOT include a network updater inside the trusted product. If a future updater is approved, it must be a separately packaged, separately permissioned product absent from classroom builds and unable to be invoked by trusted simulator code.
~~~~

**Registry text**

~~~~text
MUST NOT include a network updater inside the trusted product. If a future updater is approved, it must be a separately packaged, separately permissioned product absent from classroom builds and unable to be invoked by trusted simulator code.
~~~~

#### PES-SEC-0010

- **Selection basis:** TM-22
- **Page/section:** page 14; 5. Immutable VirtualUniverse Safety Wall > 5.5 Threat and claim boundary
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-SEC-0010] MUST make zero-egress evidence process-scoped to the application and its child processes, while distinguishing unrelated host traffic.
~~~~

**Registry text**

~~~~text
MUST make zero-egress evidence process-scoped to the application and its child processes, while distinguishing unrelated host traffic.
~~~~

#### PES-SEC-0011

- **Selection basis:** TM-22
- **Page/section:** page 14; 5. Immutable VirtualUniverse Safety Wall > 5.5 Threat and claim boundary
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-SEC-0011] MUST fail release on attempted network syscalls or endpoint resolution even if a firewall blocks packets.
~~~~

**Registry text**

~~~~text
MUST fail release on attempted network syscalls or endpoint resolution even if a firewall blocks packets.
~~~~

#### PES-SEC-0014

- **Selection basis:** TM-11
- **Page/section:** page 14; 5. Immutable VirtualUniverse Safety Wall > 5.6 Untrusted files and scripting
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-SEC-0014] MUST NOT execute code from a project, archive, library, scenario, HMI object, lesson, or sample.
~~~~

**Registry text**

~~~~text
MUST NOT execute code from a project, archive, library, scenario, HMI object, lesson, or sample.
~~~~

#### PES-SEC-0015

- **Selection basis:** TM-11
- **Page/section:** page 14; 5. Immutable VirtualUniverse Safety Wall > 5.6 Untrusted files and scripting
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-SEC-0015] MUST NOT use eval, Function constructors, dynamic native modules, arbitrary JavaScript, arbitrary WebAssembly, macros, shell commands, or executable embedded content.
~~~~

**Registry text**

~~~~text
MUST NOT use eval, Function constructors, dynamic native modules, arbitrary JavaScript, arbitrary WebAssembly, macros, shell commands, or executable embedded content.
~~~~

#### PES-SEC-0016

- **Selection basis:** TM-11
- **Page/section:** page 14; 5. Immutable VirtualUniverse Safety Wall > 5.6 Untrusted files and scripting
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-SEC-0016] MUST make any future HMI or assessment scripting a capability-limited original DSL or interpreter with deterministic execution, explicit resource limits, no host objects, no dynamic imports, no network, no filesystem, no process access, and no escape to general-purpose code.
~~~~

**Registry text**

~~~~text
MUST make any future HMI or assessment scripting a capability-limited original DSL or interpreter with deterministic execution, explicit resource limits, no host objects, no dynamic imports, no network, no filesystem, no process access, and no escape to general-purpose code.
~~~~

#### PES-SEC-0018

- **Selection basis:** SI-GATE-12
- **Page/section:** page 20; 7. Security and Trust Model > 7.1 Trust zones
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-SEC-0018] MUST keep persistence and presentation code from bypassing domain commands or writing trusted semantic state directly.
~~~~

**Registry text**

~~~~text
MUST keep persistence and presentation code from bypassing domain commands or writing trusted semantic state directly.
~~~~

#### PES-SEC-0019

- **Selection basis:** sorted position 229, SI-GATE-12
- **Page/section:** page 20; 7. Security and Trust Model > 7.1 Trust zones
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-SEC-0019] MUST use explicit serialization schemas at trust boundaries. "Any," untagged arbitrary maps, dynamic class loading, and reflection-based invocation shall not cross into the semantic core.
~~~~

**Registry text**

~~~~text
MUST use explicit serialization schemas at trust boundaries. "Any," untagged arbitrary maps, dynamic class loading, and reflection-based invocation shall not cross into the semantic core.
~~~~

#### PES-SEC-0020

- **Selection basis:** SI-GATE-12
- **Page/section:** page 20; 7. Security and Trust Model > 7.1 Trust zones
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-SEC-0020] MUST validate message kind, schema version, payload size, object IDs, capability authorization, and state preconditions before a worker or core service executes a command.
~~~~

**Registry text**

~~~~text
MUST validate message kind, schema version, payload size, object IDs, capability authorization, and state preconditions before a worker or core service executes a command.
~~~~

#### PES-SEC-0022

- **Selection basis:** TM-20
- **Page/section:** page 20; 7. Security and Trust Model > 7.3 Security acceptance posture
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-SEC-0022] MUST make parse, validation, and migration failures structured and recoverable. Catch-and-return-success is forbidden.
~~~~

**Registry text**

~~~~text
MUST make parse, validation, and migration failures structured and recoverable. Catch-and-return-success is forbidden.
~~~~

#### PES-TCH-0001

- **Selection basis:** SI-GATE-13, TM-15, TM-16
- **Page/section:** page 20; 7. Security and Trust Model > 7.2 Teacher/student data boundary
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-TCH-0001] MUST keep teacher-authored answer keys, hidden faults, checkpoints, and scoring rules logically separate from student-visible project state.
~~~~

**Registry text**

~~~~text
MUST keep teacher-authored answer keys, hidden faults, checkpoints, and scoring rules logically separate from student-visible project state.
~~~~

#### PES-TCH-0002

- **Selection basis:** SI-GATE-13, TM-16
- **Page/section:** page 20; 7. Security and Trust Model > 7.2 Teacher/student data boundary
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-TCH-0002] MUST be honest that an offline local file cannot provide absolute secrecy against a student with full filesystem or process access. It shall provide role-appropriate UI separation, protected packaging where useful, integrity checking, and audit evidence without claiming cryptographic impossibility unless a later design proves it.
~~~~

**Registry text**

~~~~text
MUST be honest that an offline local file cannot provide absolute secrecy against a student with full filesystem or process access. It shall provide role-appropriate UI separation, protected packaging where useful, integrity checking, and audit evidence without claiming cryptographic impossibility unless a later design proves it.
~~~~

#### PES-TCH-0003

- **Selection basis:** SI-GATE-13, TM-17
- **Page/section:** page 20; 7. Security and Trust Model > 7.2 Teacher/student data boundary
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-TCH-0003] MUST store student identity minimally. The default classroom model shall support local pseudonymous student IDs and shall not require names, email addresses, cloud accounts, or telemetry.
~~~~

**Registry text**

~~~~text
MUST store student identity minimally. The default classroom model shall support local pseudonymous student IDs and shall not require names, email addresses, cloud accounts, or telemetry.
~~~~

#### PES-TCH-0004

- **Selection basis:** SI-GATE-13, TM-17
- **Page/section:** page 20; 7. Security and Trust Model > 7.2 Teacher/student data boundary
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-TCH-0004] MUST let teachers configure local audit-log retention and export. Later phases shall define exact defaults and privacy behavior before Teacher Mode is released.
~~~~

**Registry text**

~~~~text
MUST let teachers configure local audit-log retention and export. Later phases shall define exact defaults and privacy behavior before Teacher Mode is released.
~~~~

#### PES-TCH-0005

- **Selection basis:** SI-GATE-13, TM-17
- **Page/section:** page 20; 7. Security and Trust Model > 7.2 Teacher/student data boundary
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-TCH-0005] MUST NOT transmit grades, logs, project files, or identifiers outside the local product.
~~~~

**Registry text**

~~~~text
MUST NOT transmit grades, logs, project files, or identifiers outside the local product.
~~~~

#### PES-TYP-0001

- **Selection basis:** sorted position 241
- **Page/section:** page 23; 8. Constitutional Architecture Invariants > 8.4 Canonical type system and semantic editors
- **Classification:** **FAITHFUL**
- **Basis:** Registry atomicRequirement exactly equals the DOCX textual content after the issued-ID marker; heading path and source pointer also match.

**Verbatim source**

~~~~text
[PES-TYP-0001] MUST create one canonical recursive type system shared by tags, DBs, block interfaces, LAD, FBD, SCL, addresses, runtime memory, watch/modify/force, trace, HMI bindings, assessment expressions, and profiles.
~~~~

**Registry text**

~~~~text
MUST create one canonical recursive type system shared by tags, DBs, block interfaces, LAD, FBD, SCL, addresses, runtime memory, watch/modify/force, trace, HMI bindings, assessment expressions, and profiles.
~~~~

### Out-of-sample whole-population defect

#### PES-REQ-0003 - DRIFTED

- **Page/section:** page 29; 10. Normative Requirement, Decision, and Change System > 10.1 Stable identifiers
- **Defect:** atomicRequirement adds "Table row:" before each table row. Those words do not occur in the supplied DOCX.

**Verbatim source**

~~~~text
[PES-REQ-0003] MUST identify supporting records separately:
Record | Identifier
Source/evidence | SRC-NNNN
Architecture decision | ADR-NNNN
Product decision | DEC-NNNN
Open question | OQ-NNNN
Risk | RSK-NNNN
Change record | CR-NNNN
Verification case | VER-AREA-NNNN
~~~~

**Registry text**

~~~~text
MUST identify supporting records separately:
Table row: Record | Identifier
Table row: Source/evidence | SRC-NNNN
Table row: Architecture decision | ADR-NNNN
Table row: Product decision | DEC-NNNN
Table row: Open question | OQ-NNNN
Table row: Risk | RSK-NNNN
Table row: Change record | CR-NNNN
Table row: Verification case | VER-AREA-NNNN
~~~~

## Task 2 — Requirement extraction: recall

### Audit result

- In-scope normative statements: **546**
- Mapped to one or more Phase 1 requirement IDs: **498**
- UNMAPPED: **48**
- Source-page coverage: **40/40 pages walked**; pages 37 and 39 contain no in-scope statement.

The 48 unmapped statements are substantive recall defects: they are normative source obligations but have no owning or materially equivalent numbered Phase 1 requirement statement. The complete 546-statement ledger follows below; repository registries, matrices, reports, and verifier output were not used as source truth.

### Source evidence and method

- Source DOCX: ``C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx``
- DOCX SHA-256: ``EBF074E2CEAB752F09E6DB63D88E100991729DA13C1EB874290A6B337DA72612``
- Full-page rendered PDF used only to anchor source page numbers: ``C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\.phase1-verification\docx-visual\phase1-directive-render.pdf``
- Rendered PDF SHA-256: ``842EC99ACC6AE9901D89DB027F95628DEB2AB817B781BA9C6E6DFBF8BA17EDBC``
- Extraction: the DOCX body was walked in document order directly from `word/document.xml`; every one of the 40 freshly rendered pages was extracted to assign page anchors. A separate `pypdf` comparison found normalized extracted text identical on all 40 pages between the fresh export and the prior local render (combined text SHA-256 `DD5FAA0B213307CC6D4FBB8D5087FBC59751B777009CD2997BA9EF453E02FF30`).
- Trigger scope: case-insensitive `shall`, `must` (including `must not`), `never`, `is prohibited`, `is forbidden`, `is required`, and limited-permission `may … only` forms. A modal list/table/line lead-in propagates to its individual children. The DomainResult minimum-field list was conservatively included because its two-sentence introductory paragraph contains `shall` and ends with the field-list lead-in.
- Unit of count: a modal lead-in is one statement; each separately testable child bullet, table row, or schema line governed by that lead-in is another statement. Separate modal sentences in one paragraph are separate statements.
- Exclusions: structural braces (`DomainResult {` and `}`) and Appendix E's descriptive crosswalk row `Binding MUST/MUST NOT rules | Final Codex marching orders` are not normative statements.
- Mapping rule: an embedded/owning `PES-*` ID controls its sentence and modal children. An unnumbered statement is cross-mapped only where a numbered statement elsewhere in the same source materially restates the obligation. Topic similarity, a repository implementation, or a verifier assertion is insufficient.
- Statement forms counted: explicit=294, explicit table row=7, inherited bullet=192, inherited line=37, inherited table row=16.

### Major defects

- **Unnumbered normative-keyword semantics: 3 statements** (p. 4).
- **Unnumbered DomainResult minimum-field schema: 8 statements** (p. 22).
- **Top-level governance-file obligations lacking a materially equivalent numbered owner: 7 statements** (p. 27).
- **Unnumbered atomic requirement-record schema: 17 statements** (p. 30).
- **Unnumbered BLOCKED-decision record schema: 12 statements** (p. 32).
- **Open-question acceptance-budget obligation without a PES requirement owner: 1 statements** (p. 36).

### Every unmapped source statement

- **T2-0008** — p. 4; § Normative Keywords; explicit table row; mapping: **UNMAPPED**
  - Verbatim: ``MUST / SHALL | Required. Violation blocks merge, release, or acceptance.``
  - Basis: UNMAPPED.
- **T2-0009** — p. 4; § Normative Keywords; explicit table row; mapping: **UNMAPPED**
  - Verbatim: ``MUST NOT / SHALL NOT | Prohibited. Presence blocks merge, release, or acceptance.``
  - Basis: UNMAPPED.
- **T2-0010** — p. 4; § Normative Keywords; explicit table row; mapping: **UNMAPPED**
  - Verbatim: ``MAY | Optional and permitted only inside the approved scope.``
  - Basis: UNMAPPED.
- **T2-0287** — p. 22; § 8.3 Command, transaction, event, and audit model; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``success``
  - Modal lead-in: ``Every meaningful mutation shall be a domain command. The minimum conceptual result is:``
  - Basis: UNMAPPED.
- **T2-0288** — p. 22; § 8.3 Command, transaction, event, and audit model; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``value?``
  - Modal lead-in: ``Every meaningful mutation shall be a domain command. The minimum conceptual result is:``
  - Basis: UNMAPPED.
- **T2-0289** — p. 22; § 8.3 Command, transaction, event, and audit model; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``events[]``
  - Modal lead-in: ``Every meaningful mutation shall be a domain command. The minimum conceptual result is:``
  - Basis: UNMAPPED.
- **T2-0290** — p. 22; § 8.3 Command, transaction, event, and audit model; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``diagnostics[]``
  - Modal lead-in: ``Every meaningful mutation shall be a domain command. The minimum conceptual result is:``
  - Basis: UNMAPPED.
- **T2-0291** — p. 22; § 8.3 Command, transaction, event, and audit model; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``affectedObjectIds[]``
  - Modal lead-in: ``Every meaningful mutation shall be a domain command. The minimum conceptual result is:``
  - Basis: UNMAPPED.
- **T2-0292** — p. 22; § 8.3 Command, transaction, event, and audit model; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``undoToken?``
  - Modal lead-in: ``Every meaningful mutation shall be a domain command. The minimum conceptual result is:``
  - Basis: UNMAPPED.
- **T2-0293** — p. 22; § 8.3 Command, transaction, event, and audit model; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``beforeHash``
  - Modal lead-in: ``Every meaningful mutation shall be a domain command. The minimum conceptual result is:``
  - Basis: UNMAPPED.
- **T2-0294** — p. 22; § 8.3 Command, transaction, event, and audit model; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``afterHash``
  - Modal lead-in: ``Every meaningful mutation shall be a domain command. The minimum conceptual result is:``
  - Basis: UNMAPPED.
- **T2-0363** — p. 27; § 9.2 Required top-level governance files; explicit; mapping: **UNMAPPED**
  - Verbatim: ``Before feature implementation, the repository shall contain:``
  - Basis: UNMAPPED.
- **T2-0366** — p. 27; § 9.2 Required top-level governance files; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``LEGAL_REVIEW_CHECKLIST.md``
  - Modal lead-in: ``Before feature implementation, the repository shall contain:``
  - Basis: UNMAPPED.
- **T2-0369** — p. 27; § 9.2 Required top-level governance files; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``REQUIREMENTS.md``
  - Modal lead-in: ``Before feature implementation, the repository shall contain:``
  - Basis: UNMAPPED.
- **T2-0370** — p. 27; § 9.2 Required top-level governance files; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``IMPLEMENTATION_MATRIX.*``
  - Modal lead-in: ``Before feature implementation, the repository shall contain:``
  - Basis: UNMAPPED.
- **T2-0373** — p. 27; § 9.2 Required top-level governance files; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``DEPENDENCY_POLICY.*``
  - Modal lead-in: ``Before feature implementation, the repository shall contain:``
  - Basis: UNMAPPED.
- **T2-0374** — p. 27; § 9.2 Required top-level governance files; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``OPEN_DECISIONS.md``
  - Modal lead-in: ``Before feature implementation, the repository shall contain:``
  - Basis: UNMAPPED.
- **T2-0375** — p. 27; § 9.2 Required top-level governance files; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``RISK_REGISTER.md``
  - Modal lead-in: ``Before feature implementation, the repository shall contain:``
  - Basis: UNMAPPED.
- **T2-0418** — p. 30; § 10.2 Atomic record schema; explicit; mapping: **UNMAPPED**
  - Verbatim: ``Every requirement record shall contain:``
  - Basis: UNMAPPED.
- **T2-0419** — p. 30; § 10.2 Atomic record schema; inherited bullet; mapping: **UNMAPPED**
  - Verbatim: ``stable ID;``
  - Modal lead-in: ``Every requirement record shall contain:``
  - Basis: UNMAPPED.
- **T2-0420** — p. 30; § 10.2 Atomic record schema; inherited bullet; mapping: **UNMAPPED**
  - Verbatim: ``short title;``
  - Modal lead-in: ``Every requirement record shall contain:``
  - Basis: UNMAPPED.
- **T2-0421** — p. 30; § 10.2 Atomic record schema; inherited bullet; mapping: **UNMAPPED**
  - Verbatim: ``normative keyword;``
  - Modal lead-in: ``Every requirement record shall contain:``
  - Basis: UNMAPPED.
- **T2-0422** — p. 30; § 10.2 Atomic record schema; inherited bullet; mapping: **UNMAPPED**
  - Verbatim: ``one atomic, testable statement;``
  - Modal lead-in: ``Every requirement record shall contain:``
  - Basis: UNMAPPED.
- **T2-0423** — p. 30; § 10.2 Atomic record schema; inherited bullet; mapping: **UNMAPPED**
  - Verbatim: ``rationale;``
  - Modal lead-in: ``Every requirement record shall contain:``
  - Basis: UNMAPPED.
- **T2-0424** — p. 30; § 10.2 Atomic record schema; inherited bullet; mapping: **UNMAPPED**
  - Verbatim: ``scope/component;``
  - Modal lead-in: ``Every requirement record shall contain:``
  - Basis: UNMAPPED.
- **T2-0425** — p. 30; § 10.2 Atomic record schema; inherited bullet; mapping: **UNMAPPED**
  - Verbatim: ``source pointer and research classification;``
  - Modal lead-in: ``Every requirement record shall contain:``
  - Basis: UNMAPPED.
- **T2-0426** — p. 30; § 10.2 Atomic record schema; inherited bullet; mapping: **UNMAPPED**
  - Verbatim: ``IP class and disposition;``
  - Modal lead-in: ``Every requirement record shall contain:``
  - Basis: UNMAPPED.
- **T2-0427** — p. 30; § 10.2 Atomic record schema; inherited bullet; mapping: **UNMAPPED**
  - Verbatim: ``dependencies;``
  - Modal lead-in: ``Every requirement record shall contain:``
  - Basis: UNMAPPED.
- **T2-0428** — p. 30; § 10.2 Atomic record schema; inherited bullet; mapping: **UNMAPPED**
  - Verbatim: ``target software release or milestone;``
  - Modal lead-in: ``Every requirement record shall contain:``
  - Basis: UNMAPPED.
- **T2-0429** — p. 30; § 10.2 Atomic record schema; inherited bullet; mapping: **UNMAPPED**
  - Verbatim: ``current truth state;``
  - Modal lead-in: ``Every requirement record shall contain:``
  - Basis: UNMAPPED.
- **T2-0430** — p. 30; § 10.2 Atomic record schema; inherited bullet; mapping: **UNMAPPED**
  - Verbatim: ``positive acceptance condition;``
  - Modal lead-in: ``Every requirement record shall contain:``
  - Basis: UNMAPPED.
- **T2-0431** — p. 30; § 10.2 Atomic record schema; inherited bullet; mapping: **UNMAPPED**
  - Verbatim: ``negative acceptance condition;``
  - Modal lead-in: ``Every requirement record shall contain:``
  - Basis: UNMAPPED.
- **T2-0432** — p. 30; § 10.2 Atomic record schema; inherited bullet; mapping: **UNMAPPED**
  - Verbatim: ``verification IDs;``
  - Modal lead-in: ``Every requirement record shall contain:``
  - Basis: UNMAPPED.
- **T2-0433** — p. 30; § 10.2 Atomic record schema; inherited bullet; mapping: **UNMAPPED**
  - Verbatim: ``ADR/decision/change links;``
  - Modal lead-in: ``Every requirement record shall contain:``
  - Basis: UNMAPPED.
- **T2-0434** — p. 30; § 10.2 Atomic record schema; inherited bullet; mapping: **UNMAPPED**
  - Verbatim: ``owner and reviewer.``
  - Modal lead-in: ``Every requirement record shall contain:``
  - Basis: UNMAPPED.
- **T2-0468** — p. 32; § 11.3 BLOCKED decision record; explicit; mapping: **UNMAPPED**
  - Verbatim: ``Every blocked decision request shall contain:``
  - Basis: UNMAPPED.
- **T2-0469** — p. 32; § 11.3 BLOCKED decision record; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``Decision ID:``
  - Modal lead-in: ``Every blocked decision request shall contain:``
  - Basis: UNMAPPED.
- **T2-0470** — p. 32; § 11.3 BLOCKED decision record; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``Affected requirement IDs:``
  - Modal lead-in: ``Every blocked decision request shall contain:``
  - Basis: UNMAPPED.
- **T2-0471** — p. 32; § 11.3 BLOCKED decision record; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``Known facts:``
  - Modal lead-in: ``Every blocked decision request shall contain:``
  - Basis: UNMAPPED.
- **T2-0472** — p. 32; § 11.3 BLOCKED decision record; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``Unknown or conflicting point:``
  - Modal lead-in: ``Every blocked decision request shall contain:``
  - Basis: UNMAPPED.
- **T2-0473** — p. 32; § 11.3 BLOCKED decision record; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``Why Codex cannot decide safely:``
  - Modal lead-in: ``Every blocked decision request shall contain:``
  - Basis: UNMAPPED.
- **T2-0474** — p. 32; § 11.3 BLOCKED decision record; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``Option A and impact:``
  - Modal lead-in: ``Every blocked decision request shall contain:``
  - Basis: UNMAPPED.
- **T2-0475** — p. 32; § 11.3 BLOCKED decision record; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``Option B and impact:``
  - Modal lead-in: ``Every blocked decision request shall contain:``
  - Basis: UNMAPPED.
- **T2-0476** — p. 32; § 11.3 BLOCKED decision record; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``Option C and impact, if useful:``
  - Modal lead-in: ``Every blocked decision request shall contain:``
  - Basis: UNMAPPED.
- **T2-0477** — p. 32; § 11.3 BLOCKED decision record; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``Recommended option:``
  - Modal lead-in: ``Every blocked decision request shall contain:``
  - Basis: UNMAPPED.
- **T2-0478** — p. 32; § 11.3 BLOCKED decision record; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``Exact approval or evidence needed:``
  - Modal lead-in: ``Every blocked decision request shall contain:``
  - Basis: UNMAPPED.
- **T2-0479** — p. 32; § 11.3 BLOCKED decision record; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``Work that can continue:``
  - Modal lead-in: ``Every blocked decision request shall contain:``
  - Basis: UNMAPPED.
- **T2-0544** — p. 36; § 13.3 Open decisions carried forward; explicit table row; mapping: **UNMAPPED**
  - Verbatim: ``OQ-0008 | Accessibility conformance target and performance/capacity budgets | Must be objective before experience acceptance | Phase 3``
  - Basis: UNMAPPED.

<!-- FINAL_GAP_DISPOSITIONS_START -->

### Final dispositions for all 48 reported recall gaps

All 48 pre-remediation gaps are `MAPPED`; none was excluded. The source text remains verbatim in this table and the active IDs resolve in the schema-v3 registry.

| Source unit | Page / section | Exact source text | Final disposition | Active atomic requirement ID(s) | Acceptance method |
|---|---|---|---|---|---|
| `T2-0008` | 4 / Normative Keywords | MUST / SHALL \| Required. Violation blocks merge, release, or acceptance. | `MAPPED` | `PES-REQ-0010` | The gate treats MUST and SHALL as required and blocks a violated obligation. |
| `T2-0009` | 4 / Normative Keywords | MUST NOT / SHALL NOT \| Prohibited. Presence blocks merge, release, or acceptance. | `MAPPED` | `PES-REQ-0011` | The gate treats MUST NOT and SHALL NOT as prohibited and blocks their presence. |
| `T2-0010` | 4 / Normative Keywords | MAY \| Optional and permitted only inside the approved scope. | `MAPPED` | `PES-REQ-0012` | A MAY record is accepted only when its behavior remains inside approved scope. |
| `T2-0287` | 22 / 8.3 Command, transaction, event, and audit model | success | `MAPPED` | `PES-ARC-0031` | The typed foundation DomainResult declares and strictly validates the success member, the worker constructs and validates the envelope around the deterministic zero-import Rust/WASM health result, and the local UI receives that validated result through the isolated worker path. |
| `T2-0288` | 22 / 8.3 Command, transaction, event, and audit model | value? | `MAPPED` | `PES-ARC-0032` | The typed foundation DomainResult declares and strictly validates the value? member, the worker constructs and validates the envelope around the deterministic zero-import Rust/WASM health result, and the local UI receives that validated result through the isolated worker path. |
| `T2-0289` | 22 / 8.3 Command, transaction, event, and audit model | events[] | `MAPPED` | `PES-ARC-0033` | The typed foundation DomainResult declares and strictly validates the events[] member, the worker constructs and validates the envelope around the deterministic zero-import Rust/WASM health result, and the local UI receives that validated result through the isolated worker path. |
| `T2-0290` | 22 / 8.3 Command, transaction, event, and audit model | diagnostics[] | `MAPPED` | `PES-ARC-0034` | The typed foundation DomainResult declares and strictly validates the diagnostics[] member, the worker constructs and validates the envelope around the deterministic zero-import Rust/WASM health result, and the local UI receives that validated result through the isolated worker path. |
| `T2-0291` | 22 / 8.3 Command, transaction, event, and audit model | affectedObjectIds[] | `MAPPED` | `PES-ARC-0035` | The typed foundation DomainResult declares and strictly validates the affectedObjectIds[] member, the worker constructs and validates the envelope around the deterministic zero-import Rust/WASM health result, and the local UI receives that validated result through the isolated worker path. |
| `T2-0292` | 22 / 8.3 Command, transaction, event, and audit model | undoToken? | `MAPPED` | `PES-ARC-0036` | The typed foundation DomainResult declares and strictly validates the undoToken? member, the worker constructs and validates the envelope around the deterministic zero-import Rust/WASM health result, and the local UI receives that validated result through the isolated worker path. |
| `T2-0293` | 22 / 8.3 Command, transaction, event, and audit model | beforeHash | `MAPPED` | `PES-ARC-0037` | The typed foundation DomainResult declares and strictly validates the beforeHash member, the worker constructs and validates the envelope around the deterministic zero-import Rust/WASM health result, and the local UI receives that validated result through the isolated worker path. |
| `T2-0294` | 22 / 8.3 Command, transaction, event, and audit model | afterHash | `MAPPED` | `PES-ARC-0038` | The typed foundation DomainResult declares and strictly validates the afterHash member, the worker constructs and validates the envelope around the deterministic zero-import Rust/WASM health result, and the local UI receives that validated result through the isolated worker path. |
| `T2-0363` | 27 / 9.2 Required top-level governance files | Before feature implementation, the repository shall contain: | `MAPPED` | `PES-DOC-0005`<br>`PES-DOC-0006`<br>`PES-DOC-0007`<br>`PES-DOC-0008`<br>`PES-DOC-0009`<br>`PES-DOC-0010` | LEGAL_REVIEW_CHECKLIST.md exists at the exact top-level path, is nonempty, and is controlled by the trusted baseline. REQUIREMENTS.md exists at the exact top-level path, is nonempty, and is controlled by the trusted baseline. IMPLEMENTATION_MATRIX.json exists at the exact top-level path, is nonempty, and is controlled by the trusted baseline. DEPENDENCY_POLICY.md exists at the exact top-level path, is nonempty, and is controlled by the trusted baseline. OPEN_DECISIONS.md exists at the exact top-level path, is nonempty, and is controlled by the trusted baseline. RISK_REGISTER.md exists at the exact top-level path, is nonempty, and is controlled by the trusted baseline. |
| `T2-0366` | 27 / 9.2 Required top-level governance files | LEGAL_REVIEW_CHECKLIST.md | `MAPPED` | `PES-DOC-0005` | LEGAL_REVIEW_CHECKLIST.md exists at the exact top-level path, is nonempty, and is controlled by the trusted baseline. |
| `T2-0369` | 27 / 9.2 Required top-level governance files | REQUIREMENTS.md | `MAPPED` | `PES-DOC-0006` | REQUIREMENTS.md exists at the exact top-level path, is nonempty, and is controlled by the trusted baseline. |
| `T2-0370` | 27 / 9.2 Required top-level governance files | IMPLEMENTATION_MATRIX.* | `MAPPED` | `PES-DOC-0007` | IMPLEMENTATION_MATRIX.json exists at the exact top-level path, is nonempty, and is controlled by the trusted baseline. |
| `T2-0373` | 27 / 9.2 Required top-level governance files | DEPENDENCY_POLICY.* | `MAPPED` | `PES-DOC-0008` | DEPENDENCY_POLICY.md exists at the exact top-level path, is nonempty, and is controlled by the trusted baseline. |
| `T2-0374` | 27 / 9.2 Required top-level governance files | OPEN_DECISIONS.md | `MAPPED` | `PES-DOC-0009` | OPEN_DECISIONS.md exists at the exact top-level path, is nonempty, and is controlled by the trusted baseline. |
| `T2-0375` | 27 / 9.2 Required top-level governance files | RISK_REGISTER.md | `MAPPED` | `PES-DOC-0010` | RISK_REGISTER.md exists at the exact top-level path, is nonempty, and is controlled by the trusted baseline. |
| `T2-0418` | 30 / 10.2 Atomic record schema | Every requirement record shall contain: | `MAPPED` | `PES-REQ-0020`<br>`PES-REQ-0021`<br>`PES-REQ-0022`<br>`PES-REQ-0023`<br>`PES-REQ-0024`<br>`PES-REQ-0025`<br>`PES-REQ-0026`<br>`PES-REQ-0027`<br>`PES-REQ-0028`<br>`PES-REQ-0029`<br>`PES-REQ-0030`<br>`PES-REQ-0031`<br>`PES-REQ-0032`<br>`PES-REQ-0033`<br>`PES-REQ-0034`<br>`PES-REQ-0035` | Schema validation rejects a requirement record that omits or invalidates stable ID; Schema validation rejects a requirement record that omits or invalidates short title; Schema validation rejects a requirement record that omits or invalidates normative keyword; Schema validation rejects a requirement record that omits or invalidates one atomic, testable statement; Schema validation rejects a requirement record that omits or invalidates rationale; Schema validation rejects a requirement record that omits or invalidates scope/component; Schema validation rejects a requirement record that omits or invalidates source pointer and research classification; Schema validation rejects a requirement record that omits or invalidates IP class and disposition; Schema validation rejects a requirement record that omits or invalidates dependencies; Schema validation rejects a requirement record that omits or invalidates target software release or milestone; Schema validation rejects a requirement record that omits or invalidates current truth state; Schema validation rejects a requirement record that omits or invalidates positive acceptance condition; Schema validation rejects a requirement record that omits or invalidates negative acceptance condition; Schema validation rejects a requirement record that omits or invalidates verification IDs; Schema validation rejects a requirement record that omits or invalidates ADR/decision/change links; Schema validation rejects a requirement record that omits or invalidates owner and reviewer. |
| `T2-0419` | 30 / 10.2 Atomic record schema | stable ID; | `MAPPED` | `PES-REQ-0020` | Schema validation rejects a requirement record that omits or invalidates stable ID; |
| `T2-0420` | 30 / 10.2 Atomic record schema | short title; | `MAPPED` | `PES-REQ-0021` | Schema validation rejects a requirement record that omits or invalidates short title; |
| `T2-0421` | 30 / 10.2 Atomic record schema | normative keyword; | `MAPPED` | `PES-REQ-0022` | Schema validation rejects a requirement record that omits or invalidates normative keyword; |
| `T2-0422` | 30 / 10.2 Atomic record schema | one atomic, testable statement; | `MAPPED` | `PES-REQ-0023` | Schema validation rejects a requirement record that omits or invalidates one atomic, testable statement; |
| `T2-0423` | 30 / 10.2 Atomic record schema | rationale; | `MAPPED` | `PES-REQ-0024` | Schema validation rejects a requirement record that omits or invalidates rationale; |
| `T2-0424` | 30 / 10.2 Atomic record schema | scope/component; | `MAPPED` | `PES-REQ-0025` | Schema validation rejects a requirement record that omits or invalidates scope/component; |
| `T2-0425` | 30 / 10.2 Atomic record schema | source pointer and research classification; | `MAPPED` | `PES-REQ-0026` | Schema validation rejects a requirement record that omits or invalidates source pointer and research classification; |
| `T2-0426` | 30 / 10.2 Atomic record schema | IP class and disposition; | `MAPPED` | `PES-REQ-0027` | Schema validation rejects a requirement record that omits or invalidates IP class and disposition; |
| `T2-0427` | 30 / 10.2 Atomic record schema | dependencies; | `MAPPED` | `PES-REQ-0028` | Schema validation rejects a requirement record that omits or invalidates dependencies; |
| `T2-0428` | 30 / 10.2 Atomic record schema | target software release or milestone; | `MAPPED` | `PES-REQ-0029` | Schema validation rejects a requirement record that omits or invalidates target software release or milestone; |
| `T2-0429` | 30 / 10.2 Atomic record schema | current truth state; | `MAPPED` | `PES-REQ-0030` | Schema validation rejects a requirement record that omits or invalidates current truth state; |
| `T2-0430` | 30 / 10.2 Atomic record schema | positive acceptance condition; | `MAPPED` | `PES-REQ-0031` | Schema validation rejects a requirement record that omits or invalidates positive acceptance condition; |
| `T2-0431` | 30 / 10.2 Atomic record schema | negative acceptance condition; | `MAPPED` | `PES-REQ-0032` | Schema validation rejects a requirement record that omits or invalidates negative acceptance condition; |
| `T2-0432` | 30 / 10.2 Atomic record schema | verification IDs; | `MAPPED` | `PES-REQ-0033` | Schema validation rejects a requirement record that omits or invalidates verification IDs; |
| `T2-0433` | 30 / 10.2 Atomic record schema | ADR/decision/change links; | `MAPPED` | `PES-REQ-0034` | Schema validation rejects a requirement record that omits or invalidates ADR/decision/change links; |
| `T2-0434` | 30 / 10.2 Atomic record schema | owner and reviewer. | `MAPPED` | `PES-REQ-0035` | Schema validation rejects a requirement record that omits or invalidates owner and reviewer. |
| `T2-0468` | 32 / 11.3 BLOCKED decision record | Every blocked decision request shall contain: | `MAPPED` | `PES-DEC-0027`<br>`PES-DEC-0028`<br>`PES-DEC-0029`<br>`PES-DEC-0030`<br>`PES-DEC-0031`<br>`PES-DEC-0032`<br>`PES-DEC-0033`<br>`PES-DEC-0034`<br>`PES-DEC-0035`<br>`PES-DEC-0036`<br>`PES-DEC-0037` | Decision-record validation rejects a blocked request that omits Decision ID: Decision-record validation rejects a blocked request that omits Affected requirement IDs: Decision-record validation rejects a blocked request that omits Known facts: Decision-record validation rejects a blocked request that omits Unknown or conflicting point: Decision-record validation rejects a blocked request that omits Why Codex cannot decide safely: Decision-record validation rejects a blocked request that omits Option A and impact: Decision-record validation rejects a blocked request that omits Option B and impact: Decision-record validation rejects a blocked request that omits Option C and impact, if useful: Decision-record validation rejects a blocked request that omits Recommended option: Decision-record validation rejects a blocked request that omits Exact approval or evidence needed: Decision-record validation rejects a blocked request that omits Work that can continue: |
| `T2-0469` | 32 / 11.3 BLOCKED decision record | Decision ID: | `MAPPED` | `PES-DEC-0027` | Decision-record validation rejects a blocked request that omits Decision ID: |
| `T2-0470` | 32 / 11.3 BLOCKED decision record | Affected requirement IDs: | `MAPPED` | `PES-DEC-0028` | Decision-record validation rejects a blocked request that omits Affected requirement IDs: |
| `T2-0471` | 32 / 11.3 BLOCKED decision record | Known facts: | `MAPPED` | `PES-DEC-0029` | Decision-record validation rejects a blocked request that omits Known facts: |
| `T2-0472` | 32 / 11.3 BLOCKED decision record | Unknown or conflicting point: | `MAPPED` | `PES-DEC-0030` | Decision-record validation rejects a blocked request that omits Unknown or conflicting point: |
| `T2-0473` | 32 / 11.3 BLOCKED decision record | Why Codex cannot decide safely: | `MAPPED` | `PES-DEC-0031` | Decision-record validation rejects a blocked request that omits Why Codex cannot decide safely: |
| `T2-0474` | 32 / 11.3 BLOCKED decision record | Option A and impact: | `MAPPED` | `PES-DEC-0032` | Decision-record validation rejects a blocked request that omits Option A and impact: |
| `T2-0475` | 32 / 11.3 BLOCKED decision record | Option B and impact: | `MAPPED` | `PES-DEC-0033` | Decision-record validation rejects a blocked request that omits Option B and impact: |
| `T2-0476` | 32 / 11.3 BLOCKED decision record | Option C and impact, if useful: | `MAPPED` | `PES-DEC-0034` | Decision-record validation rejects a blocked request that omits Option C and impact, if useful: |
| `T2-0477` | 32 / 11.3 BLOCKED decision record | Recommended option: | `MAPPED` | `PES-DEC-0035` | Decision-record validation rejects a blocked request that omits Recommended option: |
| `T2-0478` | 32 / 11.3 BLOCKED decision record | Exact approval or evidence needed: | `MAPPED` | `PES-DEC-0036` | Decision-record validation rejects a blocked request that omits Exact approval or evidence needed: |
| `T2-0479` | 32 / 11.3 BLOCKED decision record | Work that can continue: | `MAPPED` | `PES-DEC-0037` | Decision-record validation rejects a blocked request that omits Work that can continue: |
| `T2-0544` | 36 / 13.3 Open decisions carried forward | OQ-0008 \| Accessibility conformance target and performance/capacity budgets \| Must be objective before experience acceptance \| Phase 3 | `MAPPED` | `PES-ACC-0008`<br>`PES-ACC-0009`<br>`PES-ACC-0010` | Experience acceptance remains blocked until an objective accessibility conformance target is approved and recorded. Experience acceptance remains blocked until an objective performance budget is approved and recorded. Experience acceptance remains blocked until an objective capacity budget is approved and recorded. |

<!-- FINAL_GAP_DISPOSITIONS_END -->

### Complete source recall ledger

<!-- LEDGER_START -->

#### Page 1 — 1 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0001` | Front matter | `explicit` | The product shall simulate engineering decisions and consequences with high training-transfer fidelity while remaining permanently incapable of communicating with or operating physical industrial equipment. | `MAPPED` | `PES-MSN-0003`<br>`PES-SCP-0002` | — |

#### Page 2 — 4 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0002` | Front matter | `explicit` | It shall provide high causal, behavioral, workflow, and training-transfer fidelity inside a wholly fictional VirtualUniverse. | `MAPPED` | `PES-MSN-0003`<br>`PES-FID-0002` | — |
| `T2-0003` | Front matter | `explicit` | It shall never communicate with, discover, configure, commission, download to, or operate physical industrial equipment. | `MAPPED` | `PES-SCP-0002`<br>`PES-ISO-0001`<br>`PES-ISO-0002` | — |
| `T2-0004` | Front matter | `explicit` | Unless Scott separately orders otherwise, Codex shall not begin product implementation from this incomplete directive. | `MAPPED` | `PES-ACC-0007` | — |
| `T2-0005` | Front matter | `explicit` | Reserved headings are not implementation requirements and shall not be inferred. | `MAPPED` | `PES-GOV-0019`<br>`PES-ACC-0007` | — |

#### Page 3 — 2 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0006` | How to Use This Directive | `explicit` | Never renumber or reuse one. | `MAPPED` | `PES-REQ-0004` | — |
| `T2-0007` | How to Use This Directive | `explicit` | Never trade away the safety wall, clean-room rules, or causal-fidelity doctrine for speed, convenience, visual similarity, or a demo. | `MAPPED` | `PES-ISO-0001`<br>`PES-CRM-0001`<br>`PES-FID-0002` | — |

#### Page 4 — 17 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0008` | Normative Keywords | `explicit table row` | MUST / SHALL \| Required. Violation blocks merge, release, or acceptance. | `MAPPED` | `PES-REQ-0010` | — |
| `T2-0009` | Normative Keywords | `explicit table row` | MUST NOT / SHALL NOT \| Prohibited. Presence blocks merge, release, or acceptance. | `MAPPED` | `PES-REQ-0011` | — |
| `T2-0010` | Normative Keywords | `explicit table row` | MAY \| Optional and permitted only inside the approved scope. | `MAPPED` | `PES-REQ-0012` | — |
| `T2-0011` | 1.1 Authority hierarchy | `explicit` | [PES-GOV-0001] MUST interpret this project using the following order: | `MAPPED` | `PES-GOV-0021`<br>`PES-GOV-0022`<br>`PES-GOV-0023`<br>`PES-GOV-0024`<br>`PES-GOV-0025`<br>`PES-GOV-0026` | `PES-GOV-0001` |
| `T2-0012` | 1.1 Authority hierarchy | `inherited bullet` | Applicable law, binding licenses, and the immutable product safety constraints in this directive form the outer boundary. | `MAPPED` | `PES-GOV-0021` | `PES-GOV-0001` |
| `T2-0013` | 1.1 Authority hierarchy | `inherited bullet` | Scott's explicit, approved product decisions govern product intent. | `MAPPED` | `PES-GOV-0022` | `PES-GOV-0001` |
| `T2-0014` | 1.1 Authority hierarchy | `inherited bullet` | This living Codex Master Implementation Directive governs what shall be built. | `MAPPED` | `PES-GOV-0023` | `PES-GOV-0001` |
| `T2-0015` | 1.1 Authority hierarchy | `inherited bullet` | The frozen research report supplies technical, workflow, and risk evidence. | `MAPPED` | `PES-GOV-0024` | `PES-GOV-0001` |
| `T2-0016` | 1.1 Authority hierarchy | `inherited bullet` | Approved decision records and ADRs govern implementation choices only within the authority left to them. | `MAPPED` | `PES-GOV-0025` | `PES-GOV-0001` |
| `T2-0017` | 1.1 Authority hierarchy | `inherited bullet` | Code, tests, tickets, comments, mockups, and developer assumptions are subordinate to all items above. | `MAPPED` | `PES-GOV-0026` | `PES-GOV-0001` |
| `T2-0018` | 1.1 Authority hierarchy | `explicit` | [PES-GOV-0002] MUST NOT use a lower authority to weaken, reinterpret, or silently override a higher authority. | `MAPPED` | `PES-GOV-0002` | — |
| `T2-0019` | 1.1 Authority hierarchy | `explicit` | [PES-GOV-0003] MUST treat the research report's labels accurately: | `MAPPED` | `PES-GOV-0027`<br>`PES-GOV-0028`<br>`PES-GOV-0029`<br>`PES-GOV-0030`<br>`PES-GOV-0031` | `PES-GOV-0003` |
| `T2-0020` | 1.1 Authority hierarchy | `inherited bullet` | DOCUMENTED identifies publicly supported behavior or facts. | `MAPPED` | `PES-GOV-0027` | `PES-GOV-0003` |
| `T2-0021` | 1.1 Authority hierarchy | `inherited bullet` | INFERENCE identifies a reasoned conclusion, not a documented exact behavior. | `MAPPED` | `PES-GOV-0028` | `PES-GOV-0003` |
| `T2-0022` | 1.1 Authority hierarchy | `inherited bullet` | PROPOSED identifies simulator behavior recommended by the report. | `MAPPED` | `PES-GOV-0029` | `PES-GOV-0003` |
| `T2-0023` | 1.1 Authority hierarchy | `inherited bullet` | LEGAL INTERPRETATION is risk analysis, not legal advice. | `MAPPED` | `PES-GOV-0030` | `PES-GOV-0003` |
| `T2-0024` | 1.1 Authority hierarchy | `inherited bullet` | ENGINEERING RECOMMENDATION is an implementation or product judgment. | `MAPPED` | `PES-GOV-0031` | `PES-GOV-0003` |

#### Page 5 — 13 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0025` | 1.1 Authority hierarchy | `explicit` | [PES-GOV-0004] MUST treat adopted requirements in this directive as normative regardless of the research label from which they originated. | `MAPPED` | `PES-GOV-0004` | — |
| `T2-0026` | 1.1 Authority hierarchy | `explicit` | The source label remains attached for traceability and shall not be rewritten as stronger evidence. | `MAPPED` | `PES-GOV-0004` | — |
| `T2-0027` | 1.1 Authority hierarchy | `explicit` | [PES-GOV-0005] MUST NOT claim that the research report is a legal opinion, patent clearance, trademark clearance, freedom-to-operate analysis, or guarantee of legality. | `MAPPED` | `PES-GOV-0005` | — |
| `T2-0028` | 1.2 Conflict protocol | `explicit` | [PES-GOV-0006] MUST create a BLOCKED decision record when two authorities appear to conflict. | `MAPPED` | `PES-GOV-0006` | — |
| `T2-0029` | 1.2 Conflict protocol | `explicit` | The record shall quote or precisely identify both statements, explain the conflict, list the affected requirement IDs and components, and state the minimum decision needed. | `MAPPED` | `PES-GOV-0006` | — |
| `T2-0030` | 1.2 Conflict protocol | `explicit` | [PES-GOV-0007] MUST NOT resolve an authority conflict by selecting the easiest implementation, the closest vendor behavior, or the broadest feature scope. | `MAPPED` | `PES-GOV-0007` | — |
| `T2-0031` | 1.2 Conflict protocol | `explicit` | [PES-GOV-0008] MUST construe ambiguity conservatively when physical capability, external communication, proprietary expression, user data, assessment integrity, or safety claims could be affected. | `MAPPED` | `PES-GOV-0008` | — |
| `T2-0032` | 1.2 Conflict protocol | `explicit` | [PES-GOV-0009] MUST treat any proposal to add physical industrial communication as a proposal for a different product. | `MAPPED` | `PES-GOV-0009` | — |
| `T2-0033` | 1.3 Frozen research baseline | `explicit` | [PES-GOV-0010] MUST use the research report identified by filename and SHA-256 in Document Control as the Phase 1 baseline. | `MAPPED` | `PES-GOV-0010` | — |
| `T2-0034` | 1.3 Frozen research baseline | `explicit` | [PES-GOV-0011] MUST NOT expand scope automatically when a newer TIA release, IEC edition, browser capability, framework version, or industrial technology appears. | `MAPPED` | `PES-GOV-0011` | — |
| `T2-0035` | 1.3 Frozen research baseline | `explicit` | [PES-GOV-0012] MUST process new research through an evidence record and an approved change record before it changes a normative requirement. | `MAPPED` | `PES-GOV-0012` | — |
| `T2-0036` | 1.3 Frozen research baseline | `explicit` | [PES-GOV-0013] MUST replace fragile research citation tokens with stable evidence records containing source title, publisher, version/date, durable location, access date, and the claim supported. | `MAPPED` | `PES-GOV-0013` | — |
| `T2-0037` | 1.3 Frozen research baseline | `explicit` | An unresolved source remains unresolved; Codex shall not invent bibliographic details. | `MAPPED` | `PES-GOV-0013` | — |

#### Page 6 — 21 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0038` | 2.1 Mission | `explicit` | [PES-MSN-0001] MUST build an original, professional PLC engineering and automation simulation environment for classroom learning, independent study, guided troubleshooting, and instructor-led assessment. | `MAPPED` | `PES-MSN-0001` | — |
| `T2-0039` | 2.1 Mission | `explicit` | [PES-MSN-0002] MUST make the student perform a recognizable modern PLC engineering lifecycle: | `MAPPED` | `PES-MSN-0010`<br>`PES-MSN-0011`<br>`PES-MSN-0012`<br>`PES-MSN-0013`<br>`PES-MSN-0014`<br>`PES-MSN-0015`<br>`PES-MSN-0016`<br>`PES-MSN-0017`<br>`PES-MSN-0018`<br>`PES-MSN-0019`<br>`PES-MSN-0020`<br>`PES-MSN-0021`<br>`PES-MSN-0022`<br>`PES-MSN-0023`<br>`PES-MSN-0024` | `PES-MSN-0002` |
| `T2-0040` | 2.1 Mission | `inherited bullet` | Create or open a simulator-native project. | `MAPPED` | `PES-MSN-0010` | `PES-MSN-0002` |
| `T2-0041` | 2.1 Mission | `inherited bullet` | Add fictional virtual controllers and devices. | `MAPPED` | `PES-MSN-0011` | `PES-MSN-0002` |
| `T2-0042` | 2.1 Mission | `inherited bullet` | Configure virtual racks, modules, channels, addresses, logical networks, and topology. | `MAPPED` | `PES-MSN-0012` | `PES-MSN-0002` |
| `T2-0043` | 2.1 Mission | `inherited bullet` | Create tags, constants, data types, and data blocks. | `MAPPED` | `PES-MSN-0013` | `PES-MSN-0002` |
| `T2-0044` | 2.1 Mission | `inherited bullet` | create OB, FC, FB, instance DB, and global DB program structures. | `MAPPED` | `PES-MSN-0014` | `PES-MSN-0002` |
| `T2-0045` | 2.1 Mission | `inherited bullet` | Program in LAD, FBD, and SCL (Structured Text). | `MAPPED` | `PES-MSN-0015` | `PES-MSN-0002` |
| `T2-0046` | 2.1 Mission | `inherited bullet` | Compile genuine project, hardware, language, type, and dependency semantics. | `MAPPED` | `PES-MSN-0016` | `PES-MSN-0002` |
| `T2-0047` | 2.1 Mission | `inherited bullet` | Repair real inconsistencies and rebuild. | `MAPPED` | `PES-MSN-0017` | `PES-MSN-0002` |
| `T2-0048` | 2.1 Mission | `inherited bullet` | Start a fictional controller instance. | `MAPPED` | `PES-MSN-0018` | `PES-MSN-0002` |
| `T2-0049` | 2.1 Mission | `inherited bullet` | Review an internal Virtual Load Preview. | `MAPPED` | `PES-MSN-0019` | `PES-MSN-0002` |
| `T2-0050` | 2.1 Mission | `inherited bullet` | Perform an atomic Virtual Download to a VirtualControllerId. | `MAPPED` | `PES-MSN-0020` | `PES-MSN-0002` |
| `T2-0051` | 2.1 Mission | `inherited bullet` | Use RUN, STOP, monitoring, watch, modify, force, and trace semantics. | `MAPPED` | `PES-MSN-0021` | `PES-MSN-0002` |
| `T2-0052` | 2.1 Mission | `inherited bullet` | Operate a deterministic virtual process and virtual HMI. | `MAPPED` | `PES-MSN-0022` | `PES-MSN-0002` |
| `T2-0053` | 2.1 Mission | `inherited bullet` | Diagnose causal code, hardware, network-graph, process, and HMI faults. | `MAPPED` | `PES-MSN-0023` | `PES-MSN-0002` |
| `T2-0054` | 2.1 Mission | `inherited bullet` | Correct the underlying cause and verify the result. | `MAPPED` | `PES-MSN-0024` | `PES-MSN-0002` |
| `T2-0055` | 2.1 Mission | `explicit` | [PES-MSN-0003] MUST make the engineering decisions and consequences authentic enough to transfer into an authorized laboratory workflow while keeping product identity, code, assets, devices, project formats, runtime, and communication capability original. | `MAPPED` | `PES-MSN-0003` | — |
| `T2-0056` | 2.1 Mission | `explicit` | [PES-MSN-0004] MUST NOT be industrial-control software, a hardware configuration utility, a protocol client, a controller emulator, a firmware emulator, a TIA clone, or a Siemens-branded product. | `MAPPED` | `PES-MSN-0004` | — |
| `T2-0057` | 2.1 Mission | `explicit` | [PES-MSN-0005] MUST define "fully functional" as fully functional inside VirtualUniverse. | `MAPPED` | `PES-MSN-0005` | — |
| `T2-0058` | 2.1 Mission | `explicit` | The phrase never implies physical compatibility, real controller deployment, vendor-project compatibility, safety certification, or industrial suitability. | `MAPPED` | `PES-MSN-0005` | — |

#### Page 7 — 8 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0059` | 2.2 Intended users and environments | `explicit` | [PES-MSN-0007] MUST operate in an offline classroom or home-study environment with no cloud service, remote font, CDN, telemetry service, license server, analytics endpoint, or internet connection required. | `MAPPED` | `PES-MSN-0007` | — |
| `T2-0060` | 2.2 Intended users and environments | `explicit` | [PES-MSN-0008] MUST support a professional unassisted workflow for students and a separate explanatory/teaching experience without changing the underlying engineering semantics. | `MAPPED` | `PES-MSN-0008` | — |
| `T2-0061` | 2.2 Intended users and environments | `explicit` | [PES-MSN-0009] MUST keep all claims educational. | `MAPPED` | `PES-MSN-0009` | — |
| `T2-0062` | 2.2 Intended users and environments | `explicit` | It shall not claim certification, equivalence, endorsement, production readiness, or suitability for real machine control. | `MAPPED` | `PES-MSN-0009` | — |
| `T2-0063` | 2.3 Governing success definition | `explicit` | [PES-ACC-0001] MUST judge success by causal and workflow transfer: the same kinds of engineering decisions should produce the same kinds of engineering consequences. | `MAPPED` | `PES-ACC-0001` | — |
| `T2-0064` | 2.3 Governing success definition | `explicit` | [PES-ACC-0002] MUST count the product as failing if a polished UI masks fake compilation, canned diagnostics, a non-semantic editor, nondeterministic runtime, conflated offline/online state, or scenario-specific hard-coding. | `MAPPED` | `PES-ACC-0002` | — |
| `T2-0065` | 2.3 Governing success definition | `explicit` | [PES-ACC-0003] MUST count the product as failing if any production code path can address, enumerate, connect to, send to, load to, commission, or operate a physical industrial target. | `MAPPED` | `PES-ACC-0003` | — |
| `T2-0066` | 2.3 Governing success definition | `explicit` | [PES-ACC-0004] MUST target zero students leaving training with the belief that a simulator project can be loaded into a physical PLC. | `MAPPED` | `PES-ACC-0004` | — |

#### Page 8 — 12 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0067` | 3.1 Canonical product vocabulary | `explicit` | [PES-VOC-0001] MUST use Teacher Mode as the public and schema/API canonical name. | `MAPPED` | `PES-VOC-0001` | — |
| `T2-0068` | 3.1 Canonical product vocabulary | `explicit` | [PES-VOC-0002] MUST present the textual PLC language as SCL (Structured Text) on first use and may use SCL thereafter. | `MAPPED` | `PES-VOC-0002` | — |
| `T2-0069` | 3.1 Canonical product vocabulary | `explicit` | [PES-VOC-0003] MUST use fictional brand-neutral device categories such as Compact Controller, Modular Controller, Performance Controller, Technology Controller, Distributed I/O Station, Basic Operator Panel, Advanced Operator Panel, Variable-Speed Drive, and Servo Drive. | `MAPPED` | `PES-VOC-0003` | — |
| `T2-0070` | 3.1 Canonical product vocabulary | `explicit` | [PES-VOC-0004] MUST NOT use actual Siemens model numbers or marks as active catalog identities. | `MAPPED` | `PES-VOC-0004` | — |
| `T2-0071` | 3.1 Canonical product vocabulary | `explicit` | [PES-VOC-0005] MUST NOT use shorthand such as "clone TIA," "simulate an S7," "connect virtually to an IP," "download like the real target," or "fully compatible" in requirements, UI, documentation, tests, marketing, or code comments. | `MAPPED` | `PES-VOC-0005` | — |
| `T2-0072` | 3.2 Product modes share one kernel | `explicit` | [PES-EDU-0001] MUST make Engineering Mode, Learning Lens, and Teacher Mode use one project model, compiler, diagnostic system, virtual runtime, process state, HMI state, and persistence system. | `MAPPED` | `PES-EDU-0001` | — |
| `T2-0073` | 3.2 Product modes share one kernel | `explicit` | [PES-EDU-0002] MUST NOT implement a separate simplified compiler or runtime for lessons. | `MAPPED` | `PES-EDU-0002` | — |
| `T2-0074` | 3.2 Product modes share one kernel | `explicit` | [PES-EDU-0003] MUST keep Learning Lens observational. | `MAPPED` | `PES-EDU-0003` | — |
| `T2-0075` | 3.2 Product modes share one kernel | `explicit` | It may pause, step, slow, inspect, annotate, and explain deterministic execution; it shall not alter program truth, compiler results, type rules, device state, or grading outcomes. | `MAPPED` | `PES-EDU-0003` | — |
| `T2-0076` | 3.2 Product modes share one kernel | `explicit` | [PES-EDU-0004] MUST make Teacher Mode act through ordinary domain commands, scenario events, process faults, and virtual hardware faults. | `MAPPED` | `PES-EDU-0004` | — |
| `T2-0077` | 3.2 Product modes share one kernel | `explicit` | [PES-EDU-0005] MUST NOT let Teacher Mode insert compiler errors, runtime diagnostics, HMI alarms, expected values, or "correct" program state directly. | `MAPPED` | `PES-EDU-0005` | — |
| `T2-0078` | 3.2 Product modes share one kernel | `explicit` | [PES-EDU-0006] MUST NOT make Teacher Mode an online AI dependency. | `MAPPED` | `PES-EDU-0006` | — |

#### Page 9 — 17 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0079` | 3.3 Profile and version claims | `explicit` | [PES-PROF-0001] MUST use a V21-era workflow as the principal frozen training reference for the first complete product baseline. | `MAPPED` | `PES-PROF-0001` | — |
| `T2-0080` | 3.3 Profile and version claims | `explicit` | [PES-PROF-0002] MUST implement a version-neutral semantic core and declarative TrainingProfile capability manifests. | `MAPPED` | `PES-PROF-0002` | — |
| `T2-0081` | 3.3 Profile and version claims | `explicit` | [PES-PROF-0003] MUST keep project schema version independent from TrainingProfile version. | `MAPPED` | `PES-PROF-0003` | — |
| `T2-0082` | 3.3 Profile and version claims | `explicit` | [PES-PROF-0004] MUST pin a project's selected profile and capability-manifest version. | `MAPPED` | `PES-PROF-0004` | — |
| `T2-0083` | 3.3 Profile and version claims | `explicit` | Opening or migrating a project shall not silently change runtime semantics. | `MAPPED` | `PES-PROF-0004` | — |
| `T2-0084` | 3.3 Profile and version claims | `explicit` | [PES-PROF-0005] MUST treat V19-era and V20-era profiles as the first compatibility targets after the primary profile. | `MAPPED` | `PES-PROF-0005` | — |
| `T2-0085` | 3.3 Profile and version claims | `explicit` | Until their behavior is specified and tested, they shall be marked DEFERRED rather than presented as functioning profiles. | `MAPPED` | `PES-PROF-0005` | — |
| `T2-0086` | 3.3 Profile and version claims | `explicit` | [PES-PROF-0006] MUST NOT claim exact controller-family fidelity for behavior the research report marks unresolved, including detailed OB priorities, recursion, optimized layouts, force edge cases, or vendor-specific SCL extensions. | `MAPPED` | `PES-PROF-0006` | — |
| `T2-0087` | 4.1 Required fidelity | `explicit` | [PES-FID-0001] MUST prioritize fidelity in this order: | `MAPPED` | `PES-FID-0009`<br>`PES-FID-0010`<br>`PES-FID-0011`<br>`PES-FID-0012`<br>`PES-FID-0013`<br>`PES-FID-0014` | `PES-FID-0001` |
| `T2-0088` | 4.1 Required fidelity | `inherited bullet` | Safety and physical isolation. | `MAPPED` | `PES-FID-0009` | `PES-FID-0001` |
| `T2-0089` | 4.1 Required fidelity | `inherited bullet` | Correct domain semantics and causality. | `MAPPED` | `PES-FID-0010` | `PES-FID-0001` |
| `T2-0090` | 4.1 Required fidelity | `inherited bullet` | Training-transfer workflow and state consequences. | `MAPPED` | `PES-FID-0011` | `PES-FID-0001` |
| `T2-0091` | 4.1 Required fidelity | `inherited bullet` | Determinism, inspectability, and diagnostic navigation. | `MAPPED` | `PES-FID-0012` | `PES-FID-0001` |
| `T2-0092` | 4.1 Required fidelity | `inherited bullet` | Professional interaction quality and accessibility. | `MAPPED` | `PES-FID-0013` | `PES-FID-0001` |
| `T2-0093` | 4.1 Required fidelity | `inherited bullet` | Original visual polish. | `MAPPED` | `PES-FID-0014` | `PES-FID-0001` |
| `T2-0094` | 4.1 Required fidelity | `explicit` | [PES-FID-0002] MUST implement causal fidelity rather than screenshot fidelity. | `MAPPED` | `PES-FID-0002` | — |
| `T2-0095` | 4.1 Required fidelity | `explicit` | [PES-FID-0003] MUST preserve meaningful distinctions that commercial engineering software teaches, including saved versus unsaved, built versus dirty, hardware build versus software build, offline source versus loaded artifact, loaded versus matching, RUN versus STOP, monitoring off versus on, raw process value versus CPU-visible value, modify versus force, initial versus actual versus retained value, and incoming versus cleared diagnostics. | `MAPPED` | `PES-FID-0003` | — |

#### Page 10 — 8 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0096` | 4.1 Required fidelity | `explicit` | [PES-FID-0004] MUST make failure arise from a domain invariant, parser, resolver, type checker, compiler rule, build state, runtime state, process state, HMI state, or explicit virtual fault. | `MAPPED` | `PES-FID-0004` | — |
| `T2-0097` | 4.1 Required fidelity | `explicit` | [PES-FID-0005] MUST keep invalid structures editable where appropriate while preventing invalid executable output. | `MAPPED` | `PES-FID-0005` | — |
| `T2-0098` | 4.1 Required fidelity | `explicit` | An invalid LAD or FBD graph shall not produce executable IR. | `MAPPED` | `PES-FID-0005` | — |
| `T2-0099` | 4.1 Required fidelity | `explicit` | [PES-FID-0006] MUST preserve stable identity through rename, maintain unresolved references through deletion, and restore original identity through undo. | `MAPPED` | `PES-FID-0006` | — |
| `T2-0100` | 4.1 Required fidelity | `explicit` | [PES-FID-0007] MUST make diagnostics navigable to stable object identity and, when applicable, source range, graph node, pin, slot, channel, tag, or related object. | `MAPPED` | `PES-FID-0007` | — |
| `T2-0101` | 4.1 Required fidelity | `explicit` | [PES-FID-0008] MUST NOT treat visual resemblance, screenshots, animated progress, or canned demo success as fidelity evidence. | `MAPPED` | `PES-FID-0008` | — |
| `T2-0102` | 4.2 Included product envelope | `explicit` | [PES-SCP-0001] MUST interpret this envelope as a promise to specify and implement genuine behavior in later phases, not authorization to create empty panels or placeholder APIs now. | `MAPPED` | `PES-SCP-0001` | — |
| `T2-0103` | 4.3 Permanently excluded | `explicit` | [PES-SCP-0002] MUST NOT include any capability to communicate with or operate physical PLCs, HMIs, drives, remote I/O, gateways, instruments, sensors, actuators, robots, industrial networks, or host-connected devices. | `MAPPED` | `PES-SCP-0002` | — |

#### Page 11 — 12 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0104` | 4.3 Permanently excluded | `explicit` | [PES-SCP-0003] MUST NOT implement Siemens firmware, binaries, proprietary project formats, protocol payloads, device packages, engineering APIs, iconography, diagnostic prose, hardware illustrations, or vendor load artifacts. | `MAPPED` | `PES-SCP-0003` | — |
| `T2-0105` | 4.3 Permanently excluded | `explicit` | [PES-SCP-0004] MUST NOT import or export a file intended to be accepted by a physical PLC, HMI, drive, vendor engineering system, or industrial communication tool. | `MAPPED` | `PES-SCP-0004` | — |
| `T2-0106` | 4.3 Permanently excluded | `explicit` | [PES-SCP-0005] MUST NOT provide safety-rated programming, validation, certification, or claims. | `MAPPED` | `PES-SCP-0005` | — |
| `T2-0107` | 4.3 Permanently excluded | `explicit` | Ordinary educational interlocks shall be labeled non-safety-rated. | `MAPPED` | `PES-SCP-0005` | — |
| `T2-0108` | 4.3 Permanently excluded | `explicit` | [PES-SCP-0006] MUST NOT provide remote collaboration, a cloud project server, telemetry, cloud grading, cloud AI, or a production local HTTP/WebSocket server. | `MAPPED` | `PES-SCP-0006` | — |
| `T2-0109` | 4.4 Deferred and gated features | `explicit` | [PES-SCP-0008] MUST mark those features DEFERRED until later phases define semantics, risks, and acceptance tests. | `MAPPED` | `PES-SCP-0008` | — |
| `T2-0110` | 4.4 Deferred and gated features | `explicit` | [PES-SCP-0009] MUST NOT expose a deferred feature as an enabled control, working catalog item, selectable profile, successful command, or release claim. | `MAPPED` | `PES-SCP-0009` | — |
| `T2-0111` | 4.4 Deferred and gated features | `explicit` | [PES-SCP-0010] MUST require professional legal review before implementing behaviorally close auto-tuning, advanced motion trajectories, specialized drive models, unusual commissioning workflows, advanced digital-twin algorithms, or any Class 7 or Class 8 item. | `MAPPED` | `PES-SCP-0010` | — |
| `T2-0112` | 5.1 Constitutional invariant | `explicit` | [PES-ISO-0001] MUST enforce this statement as a permanent product-scope and security invariant. | `MAPPED` | `PES-ISO-0001` | — |
| `T2-0113` | 5.1 Constitutional invariant | `explicit` | [PES-ISO-0002] MUST NOT create a disabled adapter, generic PLC connection interface, transport provider, driver abstraction, protocol plugin, future-facing physical seam, feature flag, experimental connector, or "simulator now, hardware later" architecture. | `MAPPED` | `PES-ISO-0002` | — |
| `T2-0114` | 5.1 Constitutional invariant | `explicit` | [PES-ISO-0003] MUST model a controller session only by opaque VirtualControllerId and simulator state. | `MAPPED` | `PES-ISO-0003` | — |
| `T2-0115` | 5.1 Constitutional invariant | `explicit` | The domain API shall contain no hostname, IP endpoint, URL, port, socket, interface index, MAC address used as a host target, USB identity, serial handle, Bluetooth identity, or generic connection string. | `MAPPED` | `PES-ISO-0003` | — |

#### Page 12 — 21 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0116` | 5.1 Constitutional invariant | `explicit` | [PES-ISO-0004] MUST represent virtual addresses with opaque domain value types such as VirtualIpAddress. | `MAPPED` | `PES-ISO-0004` | — |
| `T2-0117` | 5.1 Constitutional invariant | `explicit` | They shall not convert into host endpoint types. | `MAPPED` | `PES-ISO-0004` | — |
| `T2-0118` | 5.1 Constitutional invariant | `explicit` | [PES-ISO-0005] MUST implement fictional device discovery only as an in-memory query whose result is a subset of VirtualUniverse devices. | `MAPPED` | `PES-ISO-0005` | — |
| `T2-0119` | 5.1 Constitutional invariant | `explicit` | [PES-ISO-0006] MUST implement Virtual Download only as an atomic internal build-artifact transaction against a VirtualControllerId. | `MAPPED` | `PES-ISO-0006` | — |
| `T2-0120` | 5.1 Constitutional invariant | `explicit` | [PES-ISO-0007] MUST implement controller/process/HMI value exchange only through typed internal messages and InternalTagBus. | `MAPPED` | `PES-ISO-0007` | — |
| `T2-0121` | 5.2 Forbidden communication capabilities | `explicit` | [PES-ISO-0008] MUST NOT contain or expose implementations of: | `MAPPED` | `PES-ISO-0023`<br>`PES-ISO-0024`<br>`PES-ISO-0025`<br>`PES-ISO-0026`<br>`PES-ISO-0027`<br>`PES-ISO-0028`<br>`PES-ISO-0029`<br>`PES-ISO-0030`<br>`PES-ISO-0031`<br>`PES-ISO-0032`<br>`PES-ISO-0033` | `PES-ISO-0008` |
| `T2-0122` | 5.2 Forbidden communication capabilities | `inherited bullet` | S7, S7comm, or S7comm-plus; | `MAPPED` | `PES-ISO-0023` | `PES-ISO-0008` |
| `T2-0123` | 5.2 Forbidden communication capabilities | `inherited bullet` | PROFINET DCP, PROFINET I/O, or PROFIBUS; | `MAPPED` | `PES-ISO-0024` | `PES-ISO-0008` |
| `T2-0124` | 5.2 Forbidden communication capabilities | `inherited bullet` | EtherNet/IP or CIP; | `MAPPED` | `PES-ISO-0025` | `PES-ISO-0008` |
| `T2-0125` | 5.2 Forbidden communication capabilities | `inherited bullet` | Modbus TCP or RTU; | `MAPPED` | `PES-ISO-0026` | `PES-ISO-0008` |
| `T2-0126` | 5.2 Forbidden communication capabilities | `inherited bullet` | external OPC UA; | `MAPPED` | `PES-ISO-0027` | `PES-ISO-0008` |
| `T2-0127` | 5.2 Forbidden communication capabilities | `inherited bullet` | EtherCAT, CAN, CANopen, DeviceNet, BACnet, MQTT, or other physical/industrial transports; | `MAPPED` | `PES-ISO-0028` | `PES-ISO-0008` |
| `T2-0128` | 5.2 Forbidden communication capabilities | `inherited bullet` | vendor PLC, HMI, drive, or I/O SDKs; | `MAPPED` | `PES-ISO-0029` | `PES-ISO-0008` |
| `T2-0129` | 5.2 Forbidden communication capabilities | `inherited bullet` | TIA Openness, Siemens engineering DLLs, or PLCSIM APIs; | `MAPPED` | `PES-ISO-0030` | `PES-ISO-0008` |
| `T2-0130` | 5.2 Forbidden communication capabilities | `inherited bullet` | physical device discovery or host NIC enumeration; | `MAPPED` | `PES-ISO-0031` | `PES-ISO-0008` |
| `T2-0131` | 5.2 Forbidden communication capabilities | `inherited bullet` | raw Ethernet, packet capture, or industrial protocol frames. | `MAPPED` | `PES-ISO-0032` | `PES-ISO-0008` |
| `T2-0132` | 5.2 Forbidden communication capabilities | `explicit` | [PES-ISO-0009] MUST NOT let shipped production code invoke or expose: | `MAPPED` | `PES-ISO-0034`<br>`PES-ISO-0035`<br>`PES-ISO-0036`<br>`PES-ISO-0037`<br>`PES-ISO-0038` | `PES-ISO-0009` |
| `T2-0133` | 5.2 Forbidden communication capabilities | `inherited bullet` | TCP, UDP, raw sockets, TLS, DNS, HTTP, HTTPS, local HTTP, localhost servers, or generic socket APIs; | `MAPPED` | `PES-ISO-0034` | `PES-ISO-0009` |
| `T2-0134` | 5.2 Forbidden communication capabilities | `inherited bullet` | fetch, XMLHttpRequest, WebSocket, WebRTC, EventSource to an endpoint, sendBeacon, WebTransport, or service-worker network interception; | `MAPPED` | `PES-ISO-0035` | `PES-ISO-0009` |
| `T2-0135` | 5.2 Forbidden communication capabilities | `inherited bullet` | WebSerial, WebUSB, WebBluetooth, WebHID, WebNFC, WebMIDI, or later equivalent device APIs; | `MAPPED` | `PES-ISO-0036` | `PES-ISO-0009` |
| `T2-0136` | 5.2 Forbidden communication capabilities | `inherited bullet` | serial ports, USB, Bluetooth, pcap, native device enumeration, or arbitrary filesystem devices; | `MAPPED` | `PES-ISO-0037` | `PES-ISO-0009` |

#### Page 13 — 17 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0137` | 5.2 Forbidden communication capabilities | `inherited bullet` | child-process execution, shell commands, dynamic library loading, native FFI, dlopen, arbitrary native bridges, or plugins able to reach those capabilities. | `MAPPED` | `PES-ISO-0038` | `PES-ISO-0009` |
| `T2-0138` | 5.2 Forbidden communication capabilities | `explicit` | [PES-ISO-0010] MUST apply the prohibition to the entire shipped classroom product, including UI, project model, compiler, runtime, process engine, diagnostics, HMI, Teacher Mode, Learning Lens, importers, exporters, scripting, packaging glue, and production dependencies. | `MAPPED` | `PES-ISO-0010` | — |
| `T2-0139` | 5.3 Narrow host allowlist | `explicit` | [PES-SEC-0001] MAY permit only these host capabilities in the production product: | `MAPPED` | `PES-SEC-0026`<br>`PES-SEC-0027`<br>`PES-SEC-0028`<br>`PES-SEC-0029`<br>`PES-SEC-0030`<br>`PES-SEC-0031`<br>`PES-SEC-0032` | `PES-SEC-0001` |
| `T2-0140` | 5.3 Narrow host allowlist | `inherited bullet` | local rendering and user interaction; | `MAPPED` | `PES-SEC-0026` | `PES-SEC-0001` |
| `T2-0141` | 5.3 Narrow host allowlist | `inherited bullet` | explicit user-initiated open/save of simulator-native project, archive, CSV, JSON, image, or report files approved by later requirements; | `MAPPED` | `PES-SEC-0027` | `PES-SEC-0001` |
| `T2-0142` | 5.3 Narrow host allowlist | `inherited bullet` | controlled application-local persistence; | `MAPPED` | `PES-SEC-0028` | `PES-SEC-0001` |
| `T2-0143` | 5.3 Narrow host allowlist | `inherited bullet` | typed UI-to-worker messaging; | `MAPPED` | `PES-SEC-0029` | `PES-SEC-0001` |
| `T2-0144` | 5.3 Narrow host allowlist | `inherited bullet` | memory allocation; | `MAPPED` | `PES-SEC-0030` | `PES-SEC-0001` |
| `T2-0145` | 5.3 Narrow host allowlist | `inherited bullet` | simulator-controlled monotonic virtual time inputs; | `MAPPED` | `PES-SEC-0031` | `PES-SEC-0001` |
| `T2-0146` | 5.3 Narrow host allowlist | `inherited bullet` | printing or local document export only when a later requirement approves it and no external resource is loaded. | `MAPPED` | `PES-SEC-0032` | `PES-SEC-0001` |
| `T2-0147` | 5.3 Narrow host allowlist | `explicit` | [PES-SEC-0002] MUST expose file persistence as bounded document operations, not arbitrary path traversal, executable launch, shell access, device files, or general-purpose host filesystem access. | `MAPPED` | `PES-SEC-0002` | — |
| `T2-0148` | 5.3 Narrow host allowlist | `explicit` | [PES-SEC-0003] MUST ensure typed UI-to-worker IPC carries domain messages only. | `MAPPED` | `PES-SEC-0003` | — |
| `T2-0149` | 5.3 Narrow host allowlist | `explicit` | It shall not accept arbitrary code, URLs, shell strings, native method names, or generic transport descriptors. | `MAPPED` | `PES-SEC-0003` | — |
| `T2-0150` | 5.4 Production versus development boundary | `explicit` | [PES-SEC-0005] MUST keep those development capabilities outside production dependency graphs, shipped bundles, runtime permissions, and user-reachable code. | `MAPPED` | `PES-SEC-0005` | — |
| `T2-0151` | 5.4 Production versus development boundary | `explicit` | [PES-SEC-0006] MUST build the production classroom application without a local web server. | `MAPPED` | `PES-SEC-0006` | — |
| `T2-0152` | 5.4 Production versus development boundary | `explicit` | Assets, workers, fonts, WASM, help, and examples shall be bundled and loaded locally without HTTP or WebSocket. | `MAPPED` | `PES-SEC-0006` | — |
| `T2-0153` | 5.4 Production versus development boundary | `explicit` | [PES-SEC-0007] MUST enforce a production Content Security Policy with at least connect-src 'none' and default-deny restrictions on external scripts, styles, fonts, images, media, objects, frames, forms, manifests, base-URI changes, and unsolicited navigation. | `MAPPED` | `PES-SEC-0007` | — |

#### Page 14 — 14 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0154` | 5.4 Production versus development boundary | `explicit` | [PES-SEC-0008] MUST NOT include a network updater inside the trusted product. | `MAPPED` | `PES-SEC-0008` | — |
| `T2-0155` | 5.4 Production versus development boundary | `explicit` | If a future updater is approved, it must be a separately packaged, separately permissioned product absent from classroom builds and unable to be invoked by trusted simulator code. | `MAPPED` | `PES-SEC-0008` | — |
| `T2-0156` | 5.5 Threat and claim boundary | `explicit` | [PES-SEC-0009] MUST claim that the unmodified shipped product has no physical-industrial communication code path or capability. | `MAPPED` | `PES-SEC-0009` | — |
| `T2-0157` | 5.5 Threat and claim boundary | `explicit` | It shall not claim that a maliciously modified binary or compromised host operating system is metaphysically incapable of networking. | `MAPPED` | `PES-SEC-0009` | — |
| `T2-0158` | 5.5 Threat and claim boundary | `explicit` | [PES-SEC-0010] MUST make zero-egress evidence process-scoped to the application and its child processes, while distinguishing unrelated host traffic. | `MAPPED` | `PES-SEC-0010` | — |
| `T2-0159` | 5.5 Threat and claim boundary | `explicit` | [PES-SEC-0011] MUST fail release on attempted network syscalls or endpoint resolution even if a firewall blocks packets. | `MAPPED` | `PES-SEC-0011` | — |
| `T2-0160` | 5.6 Untrusted files and scripting | `explicit` | [PES-SEC-0012] MUST treat every imported project, archive, CSV, JSON, library, scenario, image, or future script as untrusted input. | `MAPPED` | `PES-SEC-0012` | — |
| `T2-0161` | 5.6 Untrusted files and scripting | `explicit` | [PES-SEC-0013] MUST apply schema validation, canonical path validation, archive traversal prevention, duplicate-entry detection, compression-ratio limits, uncompressed-size limits, file-count limits, nesting limits, string/array/object limits, image-dimension limits, and deterministic resource budgets. | `MAPPED` | `PES-SEC-0013` | — |
| `T2-0162` | 5.6 Untrusted files and scripting | `explicit` | [PES-SEC-0014] MUST NOT execute code from a project, archive, library, scenario, HMI object, lesson, or sample. | `MAPPED` | `PES-SEC-0014` | — |
| `T2-0163` | 5.6 Untrusted files and scripting | `explicit` | [PES-SEC-0015] MUST NOT use eval, Function constructors, dynamic native modules, arbitrary JavaScript, arbitrary WebAssembly, macros, shell commands, or executable embedded content. | `MAPPED` | `PES-SEC-0015` | — |
| `T2-0164` | 5.6 Untrusted files and scripting | `explicit` | [PES-SEC-0016] MUST make any future HMI or assessment scripting a capability-limited original DSL or interpreter with deterministic execution, explicit resource limits, no host objects, no dynamic imports, no network, no filesystem, no process access, and no escape to general-purpose code. | `MAPPED` | `PES-SEC-0016` | — |
| `T2-0165` | 5.7 Release-blocking isolation proof | `explicit` | [PES-ISO-0011] MUST make every isolation test release-blocking. | `MAPPED` | `PES-ISO-0011` | — |
| `T2-0166` | 5.7 Release-blocking isolation proof | `explicit` | [PES-ISO-0012] MUST scan production dependency graphs, lockfiles, optional dependencies, aliases, native modules, dynamic imports, WASM imports, and packaged output for prohibited capabilities. | `MAPPED` | `PES-ISO-0012` | — |
| `T2-0167` | 5.7 Release-blocking isolation proof | `explicit` | [PES-ISO-0013] MUST statically scan trusted and shipped source for forbidden browser, Node, native, FFI, subprocess, device, networking, and industrial APIs. | `MAPPED` | `PES-ISO-0013` | — |

#### Page 15 — 12 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0168` | 5.7 Release-blocking isolation proof | `explicit` | [PES-ISO-0014] MUST inspect every semantic/runtime WASM module. | `MAPPED` | `PES-ISO-0014` | — |
| `T2-0169` | 5.7 Release-blocking isolation proof | `explicit` | [PES-ISO-0015] MUST run the complete product and course suite with all network adapters disabled or removed. | `MAPPED` | `PES-ISO-0015` | — |
| `T2-0170` | 5.7 Release-blocking isolation proof | `explicit` | [PES-ISO-0016] MUST run zero-egress and zero-attempt tests covering project creation, virtual addresses, discovery, compilation, virtual load, RUN/STOP, HMI, monitoring, watch, modify, force, trace, diagnostics, faults, lessons, grading, save, and export. | `MAPPED` | `PES-ISO-0016` | — |
| `T2-0171` | 5.7 Release-blocking isolation proof | `explicit` | [PES-ISO-0017] MUST fuzz all user-text and address-bearing fields with loopback, private, public, multicast, broadcast, IPv6, hostnames, URLs, industrial-looking ports, UNC paths, device paths, and malformed endpoint strings. | `MAPPED` | `PES-ISO-0017` | — |
| `T2-0172` | 5.7 Release-blocking isolation proof | `explicit` | All values shall remain inert data. | `MAPPED` | `PES-ISO-0017` | — |
| `T2-0173` | 5.7 Release-blocking isolation proof | `explicit` | [PES-ISO-0018] MUST prove that device discovery results remain unchanged in the presence of a live LAN containing real or PLC-like devices. | `MAPPED` | `PES-ISO-0018` | — |
| `T2-0174` | 5.7 Release-blocking isolation proof | `explicit` | [PES-ISO-0019] MUST prove at type, deserialization, reflection, and UI boundaries that Virtual Download accepts only VirtualControllerId. | `MAPPED` | `PES-ISO-0019` | — |
| `T2-0175` | 5.7 Release-blocking isolation proof | `explicit` | [PES-ISO-0020] MUST prove every HMI binding resolves only through InternalTagBus. | `MAPPED` | `PES-ISO-0020` | — |
| `T2-0176` | 5.7 Release-blocking isolation proof | `explicit` | [PES-ISO-0021] MUST prove exports contain no vendor project, firmware, load binary, deployable industrial payload, protocol frame, executable, or file directly accepted by a physical industrial tool. | `MAPPED` | `PES-ISO-0021` | — |
| `T2-0177` | 5.7 Release-blocking isolation proof | `explicit` | [PES-ISO-0022] MUST retain machine-readable evidence for each isolation gate with artifact hash, test version, date, platform, result, and logs sufficient to reproduce the test. | `MAPPED` | `PES-ISO-0022` | — |
| `T2-0178` | 6.1 Independent implementation | `explicit` | [PES-CRM-0001] MUST use original expression and independent implementation. | `MAPPED` | `PES-CRM-0001` | — |
| `T2-0179` | 6.1 Independent implementation | `explicit` | [PES-CRM-0002] MUST treat educational purpose as the mission, not permission to copy and not a substitute for legal analysis. | `MAPPED` | `PES-CRM-0002` | — |

#### Page 16 — 18 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0180` | 6.1 Independent implementation | `explicit` | [PES-CRM-0004] MUST NOT copy Siemens screens, layout composition, icons, help prose, diagnostic prose or numbers, artwork, device illustrations, completion databases, project formats, compiler components, firmware behavior, or proprietary algorithms. | `MAPPED` | `PES-CRM-0004` | — |
| `T2-0181` | 6.1 Independent implementation | `explicit` | [PES-CRM-0005] MUST use original names, event codes, visual language, device identities, project structures, schemas, source representations, sample projects, and user documentation. | `MAPPED` | `PES-CRM-0005` | — |
| `T2-0182` | 6.2 IP classification | `explicit` | Every externally inspired requirement shall be classified before implementation: | `MAPPED` | `PES-CRM-0007` | — |
| `T2-0183` | 6.2 IP classification | `inherited table row` | 1 \| Functional behavior \| Independently implement | `MAPPED` | `PES-CRM-0001` | — |
| `T2-0184` | 6.2 IP classification | `inherited table row` | 2 \| Industry or IEC convention \| Implement from lawfully licensed standards or public behavior | `MAPPED` | `PES-CRM-0027` | `PES-CRM-0008` |
| `T2-0185` | 6.2 IP classification | `inherited table row` | 3 \| Workflow behavior \| Preserve useful workflow logic; redesign visuals and expression | `MAPPED` | `PES-CRM-0003`<br>`PES-CRM-0004`<br>`PES-CRM-0005` | — |
| `T2-0186` | 6.2 IP classification | `inherited table row` | 4 \| Vendor-specific expression \| Redesign | `MAPPED` | `PES-CRM-0004`<br>`PES-CRM-0005` | — |
| `T2-0187` | 6.2 IP classification | `inherited table row` | 5 \| Branding or trademark \| Replace or exclude | `MAPPED` | `PES-CRM-0012`<br>`PES-CRM-0013` | — |
| `T2-0188` | 6.2 IP classification | `inherited table row` | 6 \| Proprietary technology \| Create an original simulated equivalent | `MAPPED` | `PES-CRM-0001`<br>`PES-CRM-0004`<br>`PES-CRM-0005` | — |
| `T2-0189` | 6.2 IP classification | `inherited table row` | 7 \| Patent or licensing concern \| BLOCKED pending focused review | `MAPPED` | `PES-SCP-0010` | — |
| `T2-0190` | 6.2 IP classification | `inherited table row` | 8 \| Uncertain or high-risk \| BLOCKED pending professional legal review | `MAPPED` | `PES-CRM-0006`<br>`PES-SCP-0010` | — |
| `T2-0191` | 6.2 IP classification | `inherited table row` | 9 \| Physical industrial communication \| Permanently EXCLUDED | `MAPPED` | `PES-SCP-0002`<br>`PES-ISO-0001`<br>`PES-ISO-0002` | — |
| `T2-0192` | 6.2 IP classification | `explicit` | [PES-CRM-0006] MUST default an unclassified or uncertain item to Class 8, not "probably permitted." | `MAPPED` | `PES-CRM-0006` | — |
| `T2-0193` | 6.2 IP classification | `explicit` | [PES-CRM-0007] MUST NOT begin implementation of a research-derived behavior until its requirement record contains an IP classification and disposition. | `MAPPED` | `PES-CRM-0007` | — |
| `T2-0194` | 6.3 Permitted and forbidden sources | `explicit` | [PES-CRM-0009] MUST NOT use: | `MAPPED` | `PES-CRM-0031`<br>`PES-CRM-0032`<br>`PES-CRM-0033`<br>`PES-CRM-0034`<br>`PES-CRM-0035`<br>`PES-CRM-0036`<br>`PES-CRM-0037` | `PES-CRM-0009` |
| `T2-0195` | 6.3 Permitted and forbidden sources | `inherited bullet` | Siemens source code, leaked code, leaked manuals, partner-only material, or confidential training material; | `MAPPED` | `PES-CRM-0031` | `PES-CRM-0009` |
| `T2-0196` | 6.3 Permitted and forbidden sources | `inherited bullet` | decompiled or disassembled output; | `MAPPED` | `PES-CRM-0032` | `PES-CRM-0009` |
| `T2-0197` | 6.3 Permitted and forbidden sources | `inherited bullet` | executable resources, extracted icons, resource packages, or memory scraping; | `MAPPED` | `PES-CRM-0033` | `PES-CRM-0009` |

#### Page 17 — 20 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0198` | 6.3 Permitted and forbidden sources | `inherited bullet` | protocol captures intended to reproduce vendor communications; | `MAPPED` | `PES-CRM-0034` | `PES-CRM-0009` |
| `T2-0199` | 6.3 Permitted and forbidden sources | `inherited bullet` | encrypted project-format cracking; | `MAPPED` | `PES-CRM-0035` | `PES-CRM-0009` |
| `T2-0200` | 6.3 Permitted and forbidden sources | `inherited bullet` | pirated software, license bypass, access-control circumvention, or API hooking; | `MAPPED` | `PES-CRM-0036` | `PES-CRM-0009` |
| `T2-0201` | 6.3 Permitted and forbidden sources | `inherited bullet` | screenshots, manual diagrams, copied tables, copied hardware illustrations, or copied diagnostic text as implementation assets. | `MAPPED` | `PES-CRM-0037` | `PES-CRM-0009` |
| `T2-0202` | 6.3 Permitted and forbidden sources | `explicit` | [PES-CRM-0010] MUST prohibit observation of an installed TIA Portal product for implementation verification until counsel reviews the applicable license terms and approves a written observation procedure. | `MAPPED` | `PES-CRM-0010` | — |
| `T2-0203` | 6.3 Permitted and forbidden sources | `explicit` | [PES-CRM-0011] MUST keep screenshots and vendor assets out of production source, design files, tickets, code-generation prompts, training corpora, mockups, and asset pipelines unless counsel approves a quarantined evidence process. | `MAPPED` | `PES-CRM-0011` | — |
| `T2-0204` | 6.3 Permitted and forbidden sources | `explicit` | Quarantined evidence shall never be shipped. | `MAPPED` | `PES-CRM-0011` | — |
| `T2-0205` | 6.4 Trademark, trade dress, and public language | `explicit` | [PES-CRM-0012] MUST NOT use Siemens, SIMATIC, TIA Portal, S7, WinCC, or PLCSIM marks as product identity, catalog identity, installer branding, repository branding, splash-screen branding, store listing, domain name, or implied affiliation. | `MAPPED` | `PES-CRM-0012` | — |
| `T2-0206` | 6.4 Trademark, trade dress, and public language | `explicit` | [PES-CRM-0013] MUST NOT copy or sample a Siemens color system, icon silhouette family, device illustration style, typography, spacing system, screen composition, or overall trade dress. | `MAPPED` | `PES-CRM-0013` | — |
| `T2-0207` | 6.4 Trademark, trade dress, and public language | `explicit` | [PES-CRM-0014] MUST hold every public comparative statement mentioning Siemens or TIA Portal as BLOCKED until trademark counsel approves the exact wording and notices. | `MAPPED` | `PES-CRM-0014` | — |
| `T2-0208` | 6.4 Trademark, trade dress, and public language | `explicit` | [PES-CRM-0015] MUST treat the working title in this directive as descriptive only. | `MAPPED` | `PES-CRM-0015` | — |
| `T2-0209` | 6.5 Evidence and contamination control | `explicit` | [PES-CRM-0016] MUST create CLEAN_ROOM_POLICY.md before feature implementation. | `MAPPED` | `PES-CRM-0016` | — |
| `T2-0210` | 6.5 Evidence and contamination control | `explicit` | [PES-CRM-0017] MUST maintain a requirement evidence register with: | `MAPPED` | `PES-CRM-0038`<br>`PES-CRM-0039`<br>`PES-CRM-0040`<br>`PES-CRM-0041`<br>`PES-CRM-0042`<br>`PES-CRM-0043`<br>`PES-CRM-0044`<br>`PES-CRM-0045`<br>`PES-CRM-0046`<br>`PES-CRM-0047`<br>`PES-CRM-0048`<br>`PES-CRM-0049` | `PES-CRM-0017` |
| `T2-0211` | 6.5 Evidence and contamination control | `inherited bullet` | requirement ID; | `MAPPED` | `PES-CRM-0038` | `PES-CRM-0017` |
| `T2-0212` | 6.5 Evidence and contamination control | `inherited bullet` | paraphrased observed behavior; | `MAPPED` | `PES-CRM-0039` | `PES-CRM-0017` |
| `T2-0213` | 6.5 Evidence and contamination control | `inherited bullet` | source title, publisher, version/date, durable location, and access date; | `MAPPED` | `PES-CRM-0040` | `PES-CRM-0017` |
| `T2-0214` | 6.5 Evidence and contamination control | `inherited bullet` | report classification; | `MAPPED` | `PES-CRM-0041` | `PES-CRM-0017` |
| `T2-0215` | 6.5 Evidence and contamination control | `inherited bullet` | IP class and disposition; | `MAPPED` | `PES-CRM-0042` | `PES-CRM-0017` |
| `T2-0216` | 6.5 Evidence and contamination control | `inherited bullet` | simulator-owned implementation requirement; | `MAPPED` | `PES-CRM-0043` | `PES-CRM-0017` |
| `T2-0217` | 6.5 Evidence and contamination control | `inherited bullet` | forbidden shortcut; | `MAPPED` | `PES-CRM-0044` | `PES-CRM-0017` |

#### Page 18 — 22 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0218` | 6.5 Evidence and contamination control | `inherited bullet` | author; | `MAPPED` | `PES-CRM-0045` | `PES-CRM-0017` |
| `T2-0219` | 6.5 Evidence and contamination control | `inherited bullet` | reviewer; | `MAPPED` | `PES-CRM-0046` | `PES-CRM-0017` |
| `T2-0220` | 6.5 Evidence and contamination control | `inherited bullet` | review status and date; | `MAPPED` | `PES-CRM-0047` | `PES-CRM-0017` |
| `T2-0221` | 6.5 Evidence and contamination control | `inherited bullet` | implementation component; | `MAPPED` | `PES-CRM-0048` | `PES-CRM-0017` |
| `T2-0222` | 6.5 Evidence and contamination control | `inherited bullet` | verification IDs. | `MAPPED` | `PES-CRM-0049` | `PES-CRM-0017` |
| `T2-0223` | 6.5 Evidence and contamination control | `explicit` | [PES-CRM-0018] MUST quarantine a contribution suspected of contamination. | `MAPPED` | `PES-CRM-0018` | — |
| `T2-0224` | 6.5 Evidence and contamination control | `explicit` | It shall not enter builds, prompts, generated assets, or derived work until reviewed. | `MAPPED` | `PES-CRM-0018` | — |
| `T2-0225` | 6.5 Evidence and contamination control | `explicit` | [PES-CRM-0019] MUST perform a clean rewrite without reusing tainted code, prose, assets, naming, layouts, or extracted structure when contamination is confirmed. | `MAPPED` | `PES-CRM-0019` | — |
| `T2-0226` | 6.5 Evidence and contamination control | `explicit` | [PES-CRM-0020] MUST require contributor attestation that no forbidden source, asset, reverse-engineering output, protocol capture, or confidential material was used. | `MAPPED` | `PES-CRM-0020` | — |
| `T2-0227` | 6.6 Asset and dependency provenance | `explicit` | [PES-CRM-0021] MUST register every shipped image, icon, font, sound, animation, template, sample project, translation, and other non-code asset with: | `MAPPED` | `PES-CRM-0050`<br>`PES-CRM-0051`<br>`PES-CRM-0052`<br>`PES-CRM-0053`<br>`PES-CRM-0054`<br>`PES-CRM-0055`<br>`PES-CRM-0056`<br>`PES-CRM-0057` | `PES-CRM-0021` |
| `T2-0228` | 6.6 Asset and dependency provenance | `inherited bullet` | asset ID; | `MAPPED` | `PES-CRM-0050` | `PES-CRM-0021` |
| `T2-0229` | 6.6 Asset and dependency provenance | `inherited bullet` | author/source; | `MAPPED` | `PES-CRM-0051` | `PES-CRM-0021` |
| `T2-0230` | 6.6 Asset and dependency provenance | `inherited bullet` | license and evidence location; | `MAPPED` | `PES-CRM-0052` | `PES-CRM-0021` |
| `T2-0231` | 6.6 Asset and dependency provenance | `inherited bullet` | created date; | `MAPPED` | `PES-CRM-0053` | `PES-CRM-0021` |
| `T2-0232` | 6.6 Asset and dependency provenance | `inherited bullet` | hash algorithm and original hash; | `MAPPED` | `PES-CRM-0054` | `PES-CRM-0021` |
| `T2-0233` | 6.6 Asset and dependency provenance | `inherited bullet` | derivative chain and modifications; | `MAPPED` | `PES-CRM-0055` | `PES-CRM-0021` |
| `T2-0234` | 6.6 Asset and dependency provenance | `inherited bullet` | generated-asset disclosure where applicable; | `MAPPED` | `PES-CRM-0056` | `PES-CRM-0021` |
| `T2-0235` | 6.6 Asset and dependency provenance | `inherited bullet` | reviewer, review status, and approval date. | `MAPPED` | `PES-CRM-0057` | `PES-CRM-0021` |
| `T2-0236` | 6.6 Asset and dependency provenance | `explicit` | [PES-CRM-0022] MUST reject unregistered or unapproved assets in CI. | `MAPPED` | `PES-CRM-0022` | — |
| `T2-0237` | 6.6 Asset and dependency provenance | `explicit` | [PES-CRM-0023] MUST NOT trace screenshots or icons, redraw vendor artwork, recolor vendor assets, or sample vendor branding as proof of originality. | `MAPPED` | `PES-CRM-0023` | — |
| `T2-0238` | 6.6 Asset and dependency provenance | `explicit` | [PES-CRM-0024] MUST generate an SBOM for release artifacts and review direct, transitive, optional, native, font, and asset licenses. | `MAPPED` | `PES-CRM-0024` | — |
| `T2-0239` | 6.6 Asset and dependency provenance | `explicit` | [PES-CRM-0025] MUST block dependencies whose license obligations are incompatible with the intended distribution or cannot be satisfied and documented. | `MAPPED` | `PES-CRM-0025` | — |

#### Page 19 — 11 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0240` | 6.7 Original native file boundary | `explicit` | [PES-PRJ-0001] MUST use only simulator-native, brand-neutral project and archive formats. | `MAPPED` | `PES-PRJ-0001` | — |
| `T2-0241` | 6.7 Original native file boundary | `explicit` | [PES-PRJ-0002] MUST use the provisional internal extensions .vlabproj for a project package and .vlabarchive for an archive until an approved product-name decision replaces them. | `MAPPED` | `PES-PRJ-0002` | — |
| `T2-0242` | 6.7 Original native file boundary | `explicit` | [PES-PRJ-0003] MUST make a project package a documented, versioned, non-executable container of canonical UTF-8 data and simulator-owned binary-neutral assets. | `MAPPED` | `PES-PRJ-0003` | — |
| `T2-0243` | 6.7 Original native file boundary | `explicit` | [PES-PRJ-0004] MUST place a manifest in every project/archive containing schema version, pinned TrainingProfile ID and version, object-index version, required capabilities, file inventory, SHA-256 hashes, creation application version, and migration history. | `MAPPED` | `PES-PRJ-0004` | — |
| `T2-0244` | 6.7 Original native file boundary | `explicit` | [PES-PRJ-0005] MUST make project integrity failures visible and fail closed. | `MAPPED` | `PES-PRJ-0005` | — |
| `T2-0245` | 6.7 Original native file boundary | `explicit` | It shall not silently discard unknown, corrupt, oversized, or hash-mismatched content. | `MAPPED` | `PES-PRJ-0005` | — |
| `T2-0246` | 6.7 Original native file boundary | `explicit` | [PES-PRJ-0006] MUST NOT use .apXX, .zapXX, a Siemens library format, PLCopen XML, vendor source export, or another real-tool format unless a later separately researched and legally approved directive explicitly adds a non-physical interoperability feature. | `MAPPED` | `PES-PRJ-0006` | — |
| `T2-0247` | 6.7 Original native file boundary | `explicit` | [PES-PRJ-0007] MUST distinguish simulator-native CSV/JSON interchange from vendor or physical deployment. | `MAPPED` | `PES-PRJ-0007` | — |
| `T2-0248` | 6.7 Original native file boundary | `explicit` | Native exports shall contain no executable code and shall be documented as simulator-only. | `MAPPED` | `PES-PRJ-0007` | — |
| `T2-0249` | 7.1 Trust zones | `explicit table row` | Untrusted content \| imported projects, archives, CSV/JSON, images, future libraries/scenarios/scripts \| Validate, limit, never execute | `MAPPED` | `PES-SEC-0012`<br>`PES-SEC-0013`<br>`PES-SEC-0014` | — |
| `T2-0250` | 7.1 Trust zones | `explicit table row` | Development environment \| package managers, compilers, test servers, CI tools \| May use development capabilities but shall not enter production | `MAPPED` | `PES-SEC-0004`<br>`PES-SEC-0005` | — |

#### Page 20 — 20 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0251` | 7.1 Trust zones | `explicit` | [PES-SEC-0017] MUST document the trust boundary in SECURITY_INVARIANTS.md and keep package ownership aligned to it. | `MAPPED` | `PES-SEC-0017` | — |
| `T2-0252` | 7.1 Trust zones | `explicit` | [PES-SEC-0018] MUST keep persistence and presentation code from bypassing domain commands or writing trusted semantic state directly. | `MAPPED` | `PES-SEC-0018` | — |
| `T2-0253` | 7.1 Trust zones | `explicit` | [PES-SEC-0019] MUST use explicit serialization schemas at trust boundaries. | `MAPPED` | `PES-SEC-0019` | — |
| `T2-0254` | 7.1 Trust zones | `explicit` | "Any," untagged arbitrary maps, dynamic class loading, and reflection-based invocation shall not cross into the semantic core. | `MAPPED` | `PES-SEC-0019` | — |
| `T2-0255` | 7.1 Trust zones | `explicit` | [PES-SEC-0020] MUST validate message kind, schema version, payload size, object IDs, capability authorization, and state preconditions before a worker or core service executes a command. | `MAPPED` | `PES-SEC-0020` | — |
| `T2-0256` | 7.2 Teacher/student data boundary | `explicit` | [PES-TCH-0001] MUST keep teacher-authored answer keys, hidden faults, checkpoints, and scoring rules logically separate from student-visible project state. | `MAPPED` | `PES-TCH-0001` | — |
| `T2-0257` | 7.2 Teacher/student data boundary | `explicit` | [PES-TCH-0002] MUST be honest that an offline local file cannot provide absolute secrecy against a student with full filesystem or process access. | `MAPPED` | `PES-TCH-0002` | — |
| `T2-0258` | 7.2 Teacher/student data boundary | `explicit` | It shall provide role-appropriate UI separation, protected packaging where useful, integrity checking, and audit evidence without claiming cryptographic impossibility unless a later design proves it. | `MAPPED` | `PES-TCH-0002` | — |
| `T2-0259` | 7.2 Teacher/student data boundary | `explicit` | [PES-TCH-0003] MUST store student identity minimally. | `MAPPED` | `PES-TCH-0003` | — |
| `T2-0260` | 7.2 Teacher/student data boundary | `explicit` | The default classroom model shall support local pseudonymous student IDs and shall not require names, email addresses, cloud accounts, or telemetry. | `MAPPED` | `PES-TCH-0003` | — |
| `T2-0261` | 7.2 Teacher/student data boundary | `explicit` | [PES-TCH-0004] MUST let teachers configure local audit-log retention and export. | `MAPPED` | `PES-TCH-0004` | — |
| `T2-0262` | 7.2 Teacher/student data boundary | `explicit` | Later phases shall define exact defaults and privacy behavior before Teacher Mode is released. | `MAPPED` | `PES-TCH-0004` | — |
| `T2-0263` | 7.2 Teacher/student data boundary | `explicit` | [PES-TCH-0005] MUST NOT transmit grades, logs, project files, or identifiers outside the local product. | `MAPPED` | `PES-TCH-0005` | — |
| `T2-0264` | 7.3 Security acceptance posture | `explicit` | [PES-SEC-0021] MUST fuzz every parser and deserializer that accepts untrusted content. | `MAPPED` | `PES-SEC-0021` | — |
| `T2-0265` | 7.3 Security acceptance posture | `explicit` | [PES-SEC-0022] MUST make parse, validation, and migration failures structured and recoverable. | `MAPPED` | `PES-SEC-0022` | — |
| `T2-0266` | 7.3 Security acceptance posture | `explicit` | Catch-and-return-success is forbidden. | `MAPPED` | `PES-SEC-0022` | — |
| `T2-0267` | 7.3 Security acceptance posture | `explicit` | [PES-SEC-0023] MUST keep resource use deterministic or explicitly bounded. | `MAPPED` | `PES-SEC-0023` | — |
| `T2-0268` | 7.3 Security acceptance posture | `explicit` | A project shall not allocate unbounded memory, unbounded recursion, unbounded event queues, or unbounded archive expansion. | `MAPPED` | `PES-SEC-0023` | — |
| `T2-0269` | 7.3 Security acceptance posture | `explicit` | [PES-SEC-0024] MUST disable recursion unless a later verified TrainingProfile explicitly enables and constrains it. | `MAPPED` | `PES-SEC-0024` | — |
| `T2-0270` | 7.3 Security acceptance posture | `explicit` | [PES-SEC-0025] MUST record security-relevant changes to CSP, package trust boundaries, import/export surfaces, worker IPC, file formats, or scripting in an ADR and threat-model update. | `MAPPED` | `PES-SEC-0025` | — |

#### Page 21 — 8 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0271` | 8.1 System topology | `explicit` | [PES-ARC-0001] MUST enforce dependency direction. | `MAPPED` | `PES-ARC-0001` | — |
| `T2-0272` | 8.1 System topology | `explicit` | UI code shall not contain authoritative PLC semantics; runtime code shall not parse editor layout; Teacher Mode shall not bypass commands; persistence shall not manufacture valid state. | `MAPPED` | `PES-ARC-0001` | — |
| `T2-0273` | 8.1 System topology | `explicit` | [PES-ARC-0002] MUST keep the semantic core platform-neutral and deterministic. | `MAPPED` | `PES-ARC-0002` | — |
| `T2-0274` | 8.1 System topology | `explicit` | [PES-ARC-0003] MUST make every extension point domain-specific. | `MAPPED` | `PES-ARC-0003` | — |
| `T2-0275` | 8.2 Stable identity and project graph | `explicit` | [PES-ARC-0004] MUST represent every semantically referenceable project, hardware, network, language, HMI, library, process, lesson, scenario, assessment, diagnostic source, and runtime target object with an immutable UUID. | `MAPPED` | `PES-ARC-0004` | — |
| `T2-0276` | 8.2 Stable identity and project graph | `explicit` | [PES-ARC-0005] MUST use RFC 9562 UUID version 4 by default for newly created objects. | `MAPPED` | `PES-ARC-0005` | — |
| `T2-0277` | 8.2 Stable identity and project graph | `explicit` | Display names, addresses, paths, array positions, block numbers, and source coordinates shall not serve as identity. | `MAPPED` | `PES-ARC-0005` | — |
| `T2-0278` | 8.2 Stable identity and project graph | `explicit` | [PES-ARC-0006] MUST preserve UUID on rename, move, readdress, regroup, interface-compatible edit, and undo restoration. | `MAPPED` | `PES-ARC-0006` | — |

#### Page 22 — 22 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0279` | 8.2 Stable identity and project graph | `explicit` | [PES-ARC-0007] MUST create a new UUID for copy, template instantiation when independent, and imported objects intentionally duplicated as new objects. | `MAPPED` | `PES-ARC-0007` | — |
| `T2-0280` | 8.2 Stable identity and project graph | `explicit` | [PES-ARC-0008] MUST retain a tombstone for deleted referenced objects for as long as a live reference, undo record, migration record, diagnostic, audit event, or snapshot requires it. | `MAPPED` | `PES-ARC-0008` | — |
| `T2-0281` | 8.2 Stable identity and project graph | `explicit` | [PES-ARC-0009] MUST represent unresolved references explicitly with the target UUID and reference kind. | `MAPPED` | `PES-ARC-0009` | — |
| `T2-0282` | 8.2 Stable identity and project graph | `explicit` | Deletion shall not silently erase or retarget usages. | `MAPPED` | `PES-ARC-0009` | — |
| `T2-0283` | 8.2 Stable identity and project graph | `explicit` | [PES-ARC-0010] MUST detect UUID collision on import. | `MAPPED` | `PES-ARC-0010` | — |
| `T2-0284` | 8.2 Stable identity and project graph | `explicit` | It shall reject ambiguous merge or perform an explicit, fully traced remap only when the import operation is defined to create independent objects. | `MAPPED` | `PES-ARC-0010` | — |
| `T2-0285` | 8.2 Stable identity and project graph | `explicit` | [PES-ARC-0011] MUST maintain typed dependency edges and source/editor locations sufficient for where-used, caller/callee, type/DB usage, HMI binding, hardware-to-tag mapping, unresolved-reference filtering, and diagnostic navigation. | `MAPPED` | `PES-ARC-0011` | — |
| `T2-0286` | 8.3 Command, transaction, event, and audit model | `explicit` | Every meaningful mutation shall be a domain command. | `MAPPED` | `PES-ARC-0012` | — |
| `T2-0287` | 8.3 Command, transaction, event, and audit model | `inherited line` | success | `MAPPED` | `PES-ARC-0031` | — |
| `T2-0288` | 8.3 Command, transaction, event, and audit model | `inherited line` | value? | `MAPPED` | `PES-ARC-0032` | — |
| `T2-0289` | 8.3 Command, transaction, event, and audit model | `inherited line` | events[] | `MAPPED` | `PES-ARC-0033` | — |
| `T2-0290` | 8.3 Command, transaction, event, and audit model | `inherited line` | diagnostics[] | `MAPPED` | `PES-ARC-0034` | — |
| `T2-0291` | 8.3 Command, transaction, event, and audit model | `inherited line` | affectedObjectIds[] | `MAPPED` | `PES-ARC-0035` | — |
| `T2-0292` | 8.3 Command, transaction, event, and audit model | `inherited line` | undoToken? | `MAPPED` | `PES-ARC-0036` | — |
| `T2-0293` | 8.3 Command, transaction, event, and audit model | `inherited line` | beforeHash | `MAPPED` | `PES-ARC-0037` | — |
| `T2-0294` | 8.3 Command, transaction, event, and audit model | `inherited line` | afterHash | `MAPPED` | `PES-ARC-0038` | — |
| `T2-0295` | 8.3 Command, transaction, event, and audit model | `explicit` | [PES-ARC-0012] MUST route create, rename, delete, restore, move, copy, retype, bind, connect, disconnect, configure, compile request, load request, CPU state change, modify, force, fault, reset, lesson action, and migration through typed domain commands or explicitly read-only queries. | `MAPPED` | `PES-ARC-0012` | — |
| `T2-0296` | 8.3 Command, transaction, event, and audit model | `explicit` | [PES-ARC-0013] MUST make commands atomic with respect to their declared transaction boundary. | `MAPPED` | `PES-ARC-0013` | — |
| `T2-0297` | 8.3 Command, transaction, event, and audit model | `explicit` | Failure shall leave either the previous valid state or a separately modeled unresolved/invalid engineering state, never a half-applied hidden mutation. | `MAPPED` | `PES-ARC-0013` | — |
| `T2-0298` | 8.3 Command, transaction, event, and audit model | `explicit` | [PES-ARC-0014] MUST make undo/redo use command/event semantics and restore exact stable identity where the original object is restored. | `MAPPED` | `PES-ARC-0014` | — |
| `T2-0299` | 8.3 Command, transaction, event, and audit model | `explicit` | [PES-ARC-0015] MUST record deterministic event ordering, affected object IDs, before/after hashes, and command provenance sufficient for crash recovery, replay, Teacher Mode audit, and testing. | `MAPPED` | `PES-ARC-0015` | — |
| `T2-0300` | 8.3 Command, transaction, event, and audit model | `explicit` | [PES-ARC-0016] MUST NOT let UI components write domain objects directly or let a lesson mutate serialized files behind the domain model. | `MAPPED` | `PES-ARC-0016` | — |

#### Page 23 — 13 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0301` | 8.4 Canonical type system and semantic editors | `explicit` | [PES-TYP-0001] MUST create one canonical recursive type system shared by tags, DBs, block interfaces, LAD, FBD, SCL, addresses, runtime memory, watch/modify/force, trace, HMI bindings, assessment expressions, and profiles. | `MAPPED` | `PES-TYP-0001` | — |
| `T2-0302` | 8.4 Canonical type system and semantic editors | `explicit` | [PES-TYP-0002] MUST keep named-type identity distinct from structural shape and give type members stable identity. | `MAPPED` | `PES-TYP-0002` | — |
| `T2-0303` | 8.4 Canonical type system and semantic editors | `explicit` | [PES-ARC-0017] MUST represent LAD as a semantic graph/AST. | `MAPPED` | `PES-ARC-0017` | — |
| `T2-0304` | 8.4 Canonical type system and semantic editors | `explicit` | [PES-ARC-0018] MUST represent FBD as a typed port graph with stable node, port, and edge identity plus explicit execution dependencies. | `MAPPED` | `PES-ARC-0018` | — |
| `T2-0305` | 8.4 Canonical type system and semantic editors | `explicit` | [PES-ARC-0019] MUST represent SCL with an independently implemented lexer, parser, AST, source ranges, scope resolver, control-flow model, and original language-service metadata. | `MAPPED` | `PES-ARC-0019` | — |
| `T2-0306` | 8.4 Canonical type system and semantic editors | `explicit` | [PES-ARC-0020] MUST share instruction definitions, type checking, conversions, call signatures, diagnostics, and runtime semantics across language frontends. | `MAPPED` | `PES-ARC-0020` | — |
| `T2-0307` | 8.4 Canonical type system and semantic editors | `explicit` | [PES-ARC-0021] MUST NOT execute LAD from screen coordinates, use a regex-only compiler, use eval for SCL, or maintain separate inconsistent runtimes for LAD, FBD, and SCL. | `MAPPED` | `PES-ARC-0021` | — |
| `T2-0308` | 8.5 Unified typed IR and one runtime | `explicit` | [PES-IR-0001] MUST lower LAD, FBD, and SCL semantic models into one versioned, typed, serializable PLC IR. | `MAPPED` | `PES-IR-0001` | — |
| `T2-0309` | 8.5 Unified typed IR and one runtime | `explicit` | [PES-IR-0002] MUST centralize arithmetic, conversions, comparisons, calls, timers, counters, storage access, monitor probes, source mappings, and error semantics in the shared compiler/runtime path. | `MAPPED` | `PES-IR-0002` | — |
| `T2-0310` | 8.5 Unified typed IR and one runtime | `explicit` | [PES-IR-0003] MUST make build artifacts immutable and fingerprinted. | `MAPPED` | `PES-IR-0003` | — |
| `T2-0311` | 8.5 Unified typed IR and one runtime | `explicit` | A build shall identify project snapshot hash, compiler version, IR version, TrainingProfile ID/version, dependency closure, diagnostics, and source map. | `MAPPED` | `PES-IR-0003` | — |
| `T2-0312` | 8.5 Unified typed IR and one runtime | `explicit` | [PES-IR-0004] MUST NOT produce a runnable artifact when a blocking error exists. | `MAPPED` | `PES-IR-0004` | — |
| `T2-0313` | 8.5 Unified typed IR and one runtime | `explicit` | [PES-IR-0005] MUST reserve instrumentation points keyed by semantic node and source identity so monitoring, trace, Learning Lens, diagnostics, and assessment can observe one execution without changing it. | `MAPPED` | `PES-IR-0005` | — |

#### Page 24 — 22 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0314` | 8.6 Deterministic virtual time, scheduling, and replay | `explicit` | [PES-DET-0001] MUST use simulator-controlled monotonic virtual time for the PLC scheduler, timers, counters with temporal behavior, process physics, trace, scenarios, lesson triggers, and assessment timing. | `MAPPED` | `PES-DET-0001` | — |
| `T2-0315` | 8.6 Deterministic virtual time, scheduling, and replay | `explicit` | [PES-DET-0002] MUST NOT use wall-clock timers such as browser setTimeout as authoritative PLC or process time. | `MAPPED` | `PES-DET-0002` | — |
| `T2-0316` | 8.6 Deterministic virtual time, scheduling, and replay | `explicit` | [PES-DET-0003] MUST define stable ordering for events sharing the same virtual timestamp and priority. | `MAPPED` | `PES-DET-0003` | — |
| `T2-0317` | 8.6 Deterministic virtual time, scheduling, and replay | `explicit` | [PES-DET-0004] MUST include deterministic seed, event sequence, TrainingProfile hash, build hash, initial snapshot hash, simulator version, and scheduler version in replay identity. | `MAPPED` | `PES-DET-0004` | — |
| `T2-0318` | 8.6 Deterministic virtual time, scheduling, and replay | `explicit` | [PES-DET-0005] MUST distinguish virtual timestamp from engineering-display wall-clock timestamp. | `MAPPED` | `PES-DET-0005` | — |
| `T2-0319` | 8.6 Deterministic virtual time, scheduling, and replay | `explicit` | [PES-DET-0006] MUST guarantee that the same supported build, snapshot, profile, seed, and ordered events produce equivalent observable tag streams, outputs, diagnostics, trace data, HMI updates, and assessment results. | `MAPPED` | `PES-DET-0006` | — |
| `T2-0320` | 8.6 Deterministic virtual time, scheduling, and replay | `explicit` | [PES-DET-0007] MUST reserve scan-start, input-sample, program-execution, output-commit, process-update, trace/diagnostic/HMI publication, and scan-end boundaries. | `MAPPED` | `PES-DET-0007` | — |
| `T2-0321` | 8.7 Separate state layers | `explicit` | [PES-ARC-0022] MUST keep these layers distinct: | `MAPPED` | `PES-ARC-0039`<br>`PES-ARC-0040`<br>`PES-ARC-0041`<br>`PES-ARC-0042`<br>`PES-ARC-0043`<br>`PES-ARC-0044`<br>`PES-ARC-0045`<br>`PES-ARC-0046`<br>`PES-ARC-0047`<br>`PES-ARC-0048`<br>`PES-ARC-0049`<br>`PES-ARC-0050`<br>`PES-ARC-0051`<br>`PES-ARC-0052`<br>`PES-ARC-0053`<br>`PES-ARC-0054`<br>`PES-ARC-0055`<br>`PES-ARC-0056` | `PES-ARC-0022` |
| `T2-0322` | 8.7 Separate state layers | `inherited bullet` | editable offline project source; | `MAPPED` | `PES-ARC-0039` | `PES-ARC-0022` |
| `T2-0323` | 8.7 Separate state layers | `inherited bullet` | saved project state; | `MAPPED` | `PES-ARC-0040` | `PES-ARC-0022` |
| `T2-0324` | 8.7 Separate state layers | `inherited bullet` | hardware build state; | `MAPPED` | `PES-ARC-0041` | `PES-ARC-0022` |
| `T2-0325` | 8.7 Separate state layers | `inherited bullet` | software build state; | `MAPPED` | `PES-ARC-0042` | `PES-ARC-0022` |
| `T2-0326` | 8.7 Separate state layers | `inherited bullet` | HMI build state; | `MAPPED` | `PES-ARC-0043` | `PES-ARC-0022` |
| `T2-0327` | 8.7 Separate state layers | `inherited bullet` | immutable build artifact; | `MAPPED` | `PES-ARC-0044` | `PES-ARC-0022` |
| `T2-0328` | 8.7 Separate state layers | `inherited bullet` | loaded virtual-controller artifact; | `MAPPED` | `PES-ARC-0045` | `PES-ARC-0022` |
| `T2-0329` | 8.7 Separate state layers | `inherited bullet` | current virtual runtime values; | `MAPPED` | `PES-ARC-0046` | `PES-ARC-0022` |
| `T2-0330` | 8.7 Separate state layers | `inherited bullet` | declared initial/start values; | `MAPPED` | `PES-ARC-0047` | `PES-ARC-0022` |
| `T2-0331` | 8.7 Separate state layers | `inherited bullet` | loaded baselines; | `MAPPED` | `PES-ARC-0048` | `PES-ARC-0022` |
| `T2-0332` | 8.7 Separate state layers | `inherited bullet` | retained values; | `MAPPED` | `PES-ARC-0049` | `PES-ARC-0022` |
| `T2-0333` | 8.7 Separate state layers | `inherited bullet` | raw virtual-process values; | `MAPPED` | `PES-ARC-0050` | `PES-ARC-0022` |
| `T2-0334` | 8.7 Separate state layers | `inherited bullet` | CPU-visible values; | `MAPPED` | `PES-ARC-0051` | `PES-ARC-0022` |
| `T2-0335` | 8.7 Separate state layers | `inherited bullet` | one-shot modifications; | `MAPPED` | `PES-ARC-0052` | `PES-ARC-0022` |

#### Page 25 — 11 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0336` | 8.7 Separate state layers | `inherited bullet` | persistent force overlays; | `MAPPED` | `PES-ARC-0053` | `PES-ARC-0022` |
| `T2-0337` | 8.7 Separate state layers | `inherited bullet` | runtime snapshots; | `MAPPED` | `PES-ARC-0054` | `PES-ARC-0022` |
| `T2-0338` | 8.7 Separate state layers | `inherited bullet` | project/runtime equality or mismatch; | `MAPPED` | `PES-ARC-0055` | `PES-ARC-0022` |
| `T2-0339` | 8.7 Separate state layers | `inherited bullet` | monitoring active state. | `MAPPED` | `PES-ARC-0056` | `PES-ARC-0022` |
| `T2-0340` | 8.7 Separate state layers | `explicit` | [PES-ARC-0023] MUST model "go online" as a virtual session and comparison. | `MAPPED` | `PES-ARC-0023` | — |
| `T2-0341` | 8.7 Separate state layers | `explicit` | It shall not automatically compile, download, synchronize, or equalize states. | `MAPPED` | `PES-ARC-0023` | — |
| `T2-0342` | 8.7 Separate state layers | `explicit` | [PES-ARC-0024] MUST model Virtual Download as preview plus explicit approval/cancellation and atomic commit/rollback. | `MAPPED` | `PES-ARC-0024` | — |
| `T2-0343` | 8.7 Separate state layers | `explicit` | [PES-ARC-0025] MUST make ForceRegistry global runtime state independent of any open table or pane. | `MAPPED` | `PES-ARC-0025` | — |
| `T2-0344` | 8.8 Diagnostics and causal faults | `explicit` | [PES-DIA-0001] MUST use original simulator codes and prose. | `MAPPED` | `PES-DIA-0001` | — |
| `T2-0345` | 8.8 Diagnostics and causal faults | `explicit` | It shall not copy vendor numbers or messages. | `MAPPED` | `PES-DIA-0001` | — |
| `T2-0346` | 8.8 Diagnostics and causal faults | `explicit` | [PES-DIA-0002] MUST distinguish immutable build diagnostics from lifecycle-bearing runtime diagnostic events while permitting a unified UI. | `MAPPED` | `PES-DIA-0002` | — |

#### Page 26 — 10 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0347` | 8.8 Diagnostics and causal faults | `explicit` | [PES-DIA-0003] MUST derive diagnostics from ordinary validators, compiler rules, runtime transitions, device/process state, HMI consistency, persistence validation, or fault providers. | `MAPPED` | `PES-DIA-0003` | — |
| `T2-0348` | 8.8 Diagnostics and causal faults | `explicit` | [PES-DIA-0004] MUST let Teacher Mode invoke commands such as RemoveModule, DisconnectVirtualLink, ChangeTagType, SetSensorFault, or SetActuatorFault and let ordinary engines derive the consequence. | `MAPPED` | `PES-DIA-0004` | — |
| `T2-0349` | 8.8 Diagnostics and causal faults | `explicit` | [PES-DIA-0005] MUST NOT let a scenario, lesson, demo, or UI directly insert an expected compiler diagnostic, runtime fault, alarm, trace, monitored value, or passing assessment result. | `MAPPED` | `PES-DIA-0005` | — |
| `T2-0350` | 8.8 Diagnostics and causal faults | `explicit` | [PES-DIA-0006] MUST retain navigation targets, related object identities, virtual timestamps, lifecycle correlation, and deterministic replay ordering. | `MAPPED` | `PES-DIA-0006` | — |
| `T2-0351` | 8.9 Internal buses and future seams | `explicit` | [PES-ARC-0026] MUST define InternalTagBus as typed, quality-aware, timestamped internal publication/subscription. | `MAPPED` | `PES-ARC-0026` | — |
| `T2-0352` | 8.9 Internal buses and future seams | `explicit` | It shall operate by in-process calls or typed worker IPC, never localhost or network transport. | `MAPPED` | `PES-ARC-0026` | — |
| `T2-0353` | 8.9 Internal buses and future seams | `explicit` | [PES-ARC-0027] MUST reserve typed domain registries for project object kinds, editors, properties, commands, validators, compilers, capability gates, navigation targets, fault providers, serializers, and migrations. | `MAPPED` | `PES-ARC-0027` | — |
| `T2-0354` | 8.9 Internal buses and future seams | `explicit` | [PES-ARC-0028] MUST reserve safe schemas for scenario packages, lesson conditions, assessment expressions, snapshots, fault capabilities, and deterministic events without enabling arbitrary code. | `MAPPED` | `PES-ARC-0028` | — |
| `T2-0355` | 8.9 Internal buses and future seams | `explicit` | [PES-ARC-0029] MUST allow later HMI, library, trace, technology object, source view, localization, and scenario types without replacing the canonical project graph. | `MAPPED` | `PES-ARC-0029` | — |
| `T2-0356` | 8.9 Internal buses and future seams | `explicit` | [PES-ARC-0030] MUST NOT satisfy "reserved architecture" with empty buttons, no-op objects, placeholder transports, user-visible coming-soon panels, or generic interfaces that create forbidden capability. | `MAPPED` | `PES-ARC-0030` | — |

#### Page 27 — 27 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0357` | 9.1 Adopted stack | `explicit` | [PES-DEV-0004] MUST implement the trusted project semantics, compiler, typed IR, scheduler, and PLC runtime in Rust compiled to capability-limited WebAssembly, unless Scott approves an ADR demonstrating an equally deterministic and more strongly isolated alternative. | `MAPPED` | `PES-DEV-0004` | — |
| `T2-0358` | 9.1 Adopted stack | `explicit` | [PES-DEV-0005] MUST execute virtual runtime/process work in isolated workers using typed messages so simulation cannot freeze the UI. | `MAPPED` | `PES-DEV-0005` | — |
| `T2-0359` | 9.1 Adopted stack | `explicit` | [PES-DEV-0007] MUST bundle all production dependencies, fonts, WASM, help, scenarios, and assets locally. | `MAPPED` | `PES-DEV-0007` | — |
| `T2-0360` | 9.1 Adopted stack | `explicit` | [PES-DEV-0008] MUST keep the trusted core free of OS networking, native FFI, arbitrary filesystem, shell, and device capabilities even if the desktop shell or browser engine internally supports them. | `MAPPED` | `PES-DEV-0008` | — |
| `T2-0361` | 9.1 Adopted stack | `explicit` | [PES-DEV-0009] MUST record the chosen desktop/classroom packaging model in a BLOCKED product decision before public artifact work begins. | `MAPPED` | `PES-DEV-0009` | — |
| `T2-0362` | 9.1 Adopted stack | `explicit` | The decision shall define initial supported operating systems, installer versus portable delivery, local-file permissions, Chromium/WebView background networking controls, code signing, update separation, and offline verification. | `MAPPED` | `PES-DEV-0009` | — |
| `T2-0363` | 9.2 Required top-level governance files | `explicit` | Before feature implementation, the repository shall contain: | `MAPPED` | `PES-DOC-0005`<br>`PES-DOC-0006`<br>`PES-DOC-0007`<br>`PES-DOC-0008`<br>`PES-DOC-0009`<br>`PES-DOC-0010` | — |
| `T2-0364` | 9.2 Required top-level governance files | `inherited line` | CLEAN_ROOM_POLICY.md | `MAPPED` | `PES-CRM-0016` | — |
| `T2-0365` | 9.2 Required top-level governance files | `inherited line` | SECURITY_INVARIANTS.md | `MAPPED` | `PES-SEC-0017` | — |
| `T2-0366` | 9.2 Required top-level governance files | `inherited line` | LEGAL_REVIEW_CHECKLIST.md | `MAPPED` | `PES-DOC-0005` | — |
| `T2-0367` | 9.2 Required top-level governance files | `inherited line` | CONTRIBUTOR_CLEAN_ROOM_ATTESTATION.md | `MAPPED` | `PES-CRM-0020` | — |
| `T2-0368` | 9.2 Required top-level governance files | `inherited line` | THREAT_MODEL.md | `MAPPED` | `PES-SEC-0025` | — |
| `T2-0369` | 9.2 Required top-level governance files | `inherited line` | REQUIREMENTS.md | `MAPPED` | `PES-DOC-0006` | — |
| `T2-0370` | 9.2 Required top-level governance files | `inherited line` | IMPLEMENTATION_MATRIX.* | `MAPPED` | `PES-DOC-0007` | — |
| `T2-0371` | 9.2 Required top-level governance files | `inherited line` | EVIDENCE_REGISTER.* | `MAPPED` | `PES-CRM-0038`<br>`PES-CRM-0039`<br>`PES-CRM-0040`<br>`PES-CRM-0041`<br>`PES-CRM-0042`<br>`PES-CRM-0043`<br>`PES-CRM-0044`<br>`PES-CRM-0045`<br>`PES-CRM-0046`<br>`PES-CRM-0047`<br>`PES-CRM-0048`<br>`PES-CRM-0049` | `PES-CRM-0017` |
| `T2-0372` | 9.2 Required top-level governance files | `inherited line` | ASSET_PROVENANCE.* | `MAPPED` | `PES-CRM-0050`<br>`PES-CRM-0051`<br>`PES-CRM-0052`<br>`PES-CRM-0053`<br>`PES-CRM-0054`<br>`PES-CRM-0055`<br>`PES-CRM-0056`<br>`PES-CRM-0057` | `PES-CRM-0021` |
| `T2-0373` | 9.2 Required top-level governance files | `inherited line` | DEPENDENCY_POLICY.* | `MAPPED` | `PES-DOC-0008` | — |
| `T2-0374` | 9.2 Required top-level governance files | `inherited line` | OPEN_DECISIONS.md | `MAPPED` | `PES-DOC-0009` | — |
| `T2-0375` | 9.2 Required top-level governance files | `inherited line` | RISK_REGISTER.md | `MAPPED` | `PES-DOC-0010` | — |
| `T2-0376` | 9.2 Required top-level governance files | `inherited line` | CHANGELOG_DIRECTIVE.md | `MAPPED` | `PES-GOV-0014`<br>`PES-GOV-0015`<br>`PES-GOV-0016` | — |
| `T2-0377` | 9.2 Required top-level governance files | `inherited line` | ADR/ | `MAPPED` | `PES-DOC-0001`<br>`PES-DOC-0003` | — |
| `T2-0378` | 9.2 Required top-level governance files | `inherited line` | 0001-no-physical-industrial-communication.md | `MAPPED` | `PES-DOC-0001` | — |
| `T2-0379` | 9.2 Required top-level governance files | `inherited line` | 0002-original-project-format.md | `MAPPED` | `PES-DOC-0003` | — |
| `T2-0380` | 9.2 Required top-level governance files | `inherited line` | 0003-unified-plc-ir.md | `MAPPED` | `PES-DOC-0003` | — |
| `T2-0381` | 9.2 Required top-level governance files | `inherited line` | 0004-deterministic-virtual-time.md | `MAPPED` | `PES-DOC-0003` | — |
| `T2-0382` | 9.2 Required top-level governance files | `explicit` | [PES-DOC-0001] MUST create ADR-0001 with title "Physical Industrial Communication Is Permanently Out of Scope" and status "Project Safety Invariant." | `MAPPED` | `PES-DOC-0001` | — |
| `T2-0383` | 9.2 Required top-level governance files | `explicit` | [PES-DOC-0002] MUST state in ADR-0001 that physical capability cannot be added within this product through an ADR amendment. | `MAPPED` | `PES-DOC-0002` | — |

#### Page 28 — 6 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0384` | 9.2 Required top-level governance files | `explicit` | [PES-DOC-0003] MUST document original project format, unified IR, and deterministic virtual time before implementation depends on them. | `MAPPED` | `PES-DOC-0003` | — |
| `T2-0385` | 9.2 Required top-level governance files | `explicit` | [PES-DOC-0004] MUST keep evidence records and research notes separate from production assets. | `MAPPED` | `PES-DOC-0004` | — |
| `T2-0386` | 9.3 Required package boundaries | `explicit` | [PES-DEV-0010] MUST treat this shape as a responsibility map, not permission to create empty packages for completion credit. | `MAPPED` | `PES-DEV-0010` | — |
| `T2-0387` | 9.3 Required package boundaries | `explicit` | It shall record the reason in an ADR. | `MAPPED` | `PES-DEV-0011` | — |
| `T2-0388` | 9.3 Required package boundaries | `explicit` | [PES-DEV-0012] MUST NOT create a network, transport, device-connector, vendor-adapter, protocol, external-HMI, remote-collaboration, or plugin-host package. | `MAPPED` | `PES-DEV-0012` | — |
| `T2-0389` | 9.4 Baseline CI policy | `explicit` | [PES-CI-0001] MUST fail production merge or release when: | `MAPPED` | `PES-CI-0004`<br>`PES-CI-0005`<br>`PES-CI-0006`<br>`PES-CI-0007`<br>`PES-CI-0008`<br>`PES-CI-0009`<br>`PES-CI-0010`<br>`PES-CI-0011`<br>`PES-CI-0012`<br>`PES-CI-0013`<br>`PES-CI-0014`<br>`PES-CI-0015`<br>`PES-CI-0016`<br>`PES-CI-0017` | `PES-CI-0001` |

#### Page 29 — 26 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0390` | 9.4 Baseline CI policy | `inherited bullet` | a forbidden dependency or capability is added; | `MAPPED` | `PES-CI-0004` | `PES-CI-0001` |
| `T2-0391` | 9.4 Baseline CI policy | `inherited bullet` | a prohibited source API or WASM import appears; | `MAPPED` | `PES-CI-0005` | `PES-CI-0001` |
| `T2-0392` | 9.4 Baseline CI policy | `inherited bullet` | a remote asset, CDN, telemetry, analytics, or cloud dependency appears; | `MAPPED` | `PES-CI-0006` | `PES-CI-0001` |
| `T2-0393` | 9.4 Baseline CI policy | `inherited bullet` | an asset lacks provenance or approval; | `MAPPED` | `PES-CI-0007` | `PES-CI-0001` |
| `T2-0394` | 9.4 Baseline CI policy | `inherited bullet` | a vendor screenshot, logo, icon, device illustration, or copied prose enters production; | `MAPPED` | `PES-CI-0008` | `PES-CI-0001` |
| `T2-0395` | 9.4 Baseline CI policy | `inherited bullet` | an unclassified research-derived requirement enters implementation; | `MAPPED` | `PES-CI-0009` | `PES-CI-0001` |
| `T2-0396` | 9.4 Baseline CI policy | `inherited bullet` | a required test is skipped or flaky; | `MAPPED` | `PES-CI-0010` | `PES-CI-0001` |
| `T2-0397` | 9.4 Baseline CI policy | `inherited bullet` | determinism/replay diverges; | `MAPPED` | `PES-CI-0011` | `PES-CI-0001` |
| `T2-0398` | 9.4 Baseline CI policy | `inherited bullet` | migration loses identity or data; | `MAPPED` | `PES-CI-0012` | `PES-CI-0001` |
| `T2-0399` | 9.4 Baseline CI policy | `inherited bullet` | a lesson bypasses ordinary domain/diagnostic behavior; | `MAPPED` | `PES-CI-0013` | `PES-CI-0001` |
| `T2-0400` | 9.4 Baseline CI policy | `inherited bullet` | Virtual Download accepts any endpoint-like value; | `MAPPED` | `PES-CI-0014` | `PES-CI-0001` |
| `T2-0401` | 9.4 Baseline CI policy | `inherited bullet` | HMI uses any transport other than InternalTagBus; | `MAPPED` | `PES-CI-0015` | `PES-CI-0001` |
| `T2-0402` | 9.4 Baseline CI policy | `inherited bullet` | an exported artifact resembles or is accepted as a real industrial deployment artifact; | `MAPPED` | `PES-CI-0016` | `PES-CI-0001` |
| `T2-0403` | 9.4 Baseline CI policy | `inherited bullet` | traceability between a verified requirement and its tests is missing. | `MAPPED` | `PES-CI-0017` | `PES-CI-0001` |
| `T2-0404` | 9.4 Baseline CI policy | `explicit` | [PES-CI-0002] MUST scan the packaged artifact, not only source and lockfiles. | `MAPPED` | `PES-CI-0002` | — |
| `T2-0405` | 9.4 Baseline CI policy | `explicit` | [PES-CI-0003] MUST produce an SBOM, license notice set, asset manifest, requirement-verification report, and isolation report for a release candidate. | `MAPPED` | `PES-CI-0003` | — |
| `T2-0406` | 10.1 Stable identifiers | `explicit` | [PES-REQ-0001] MUST identify product requirements as PES-AREA-NNNN. | `MAPPED` | `PES-REQ-0001` | — |
| `T2-0407` | 10.1 Stable identifiers | `explicit` | [PES-REQ-0002] MUST NOT encode authoring phase, software release, priority, status, or document section in a requirement ID. | `MAPPED` | `PES-REQ-0002` | — |
| `T2-0408` | 10.1 Stable identifiers | `explicit` | [PES-REQ-0003] MUST identify supporting records separately: | `MAPPED` | `PES-REQ-0013`<br>`PES-REQ-0014`<br>`PES-REQ-0015`<br>`PES-REQ-0016`<br>`PES-REQ-0017`<br>`PES-REQ-0018`<br>`PES-REQ-0019` | `PES-REQ-0003` |
| `T2-0409` | 10.1 Stable identifiers | `inherited table row` | Source/evidence \| SRC-NNNN | `MAPPED` | `PES-REQ-0013` | `PES-REQ-0003` |
| `T2-0410` | 10.1 Stable identifiers | `inherited table row` | Architecture decision \| ADR-NNNN | `MAPPED` | `PES-REQ-0014` | `PES-REQ-0003` |
| `T2-0411` | 10.1 Stable identifiers | `inherited table row` | Product decision \| DEC-NNNN | `MAPPED` | `PES-REQ-0015` | `PES-REQ-0003` |
| `T2-0412` | 10.1 Stable identifiers | `inherited table row` | Open question \| OQ-NNNN | `MAPPED` | `PES-REQ-0016` | `PES-REQ-0003` |
| `T2-0413` | 10.1 Stable identifiers | `inherited table row` | Risk \| RSK-NNNN | `MAPPED` | `PES-REQ-0017` | `PES-REQ-0003` |
| `T2-0414` | 10.1 Stable identifiers | `inherited table row` | Change record \| CR-NNNN | `MAPPED` | `PES-REQ-0018` | `PES-REQ-0003` |
| `T2-0415` | 10.1 Stable identifiers | `inherited table row` | Verification case \| VER-AREA-NNNN | `MAPPED` | `PES-REQ-0019` | `PES-REQ-0003` |

#### Page 30 — 23 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0416` | 10.1 Stable identifiers | `explicit` | [PES-REQ-0004] MUST keep retired IDs as tombstones with a supersession or rejection reason. | `MAPPED` | `PES-REQ-0004` | — |
| `T2-0417` | 10.1 Stable identifiers | `explicit` | IDs shall never be recycled. | `MAPPED` | `PES-REQ-0004` | — |
| `T2-0418` | 10.2 Atomic record schema | `explicit` | Every requirement record shall contain: | `MAPPED` | `PES-REQ-0020`<br>`PES-REQ-0021`<br>`PES-REQ-0022`<br>`PES-REQ-0023`<br>`PES-REQ-0024`<br>`PES-REQ-0025`<br>`PES-REQ-0026`<br>`PES-REQ-0027`<br>`PES-REQ-0028`<br>`PES-REQ-0029`<br>`PES-REQ-0030`<br>`PES-REQ-0031`<br>`PES-REQ-0032`<br>`PES-REQ-0033`<br>`PES-REQ-0034`<br>`PES-REQ-0035` | — |
| `T2-0419` | 10.2 Atomic record schema | `inherited bullet` | stable ID; | `MAPPED` | `PES-REQ-0020` | — |
| `T2-0420` | 10.2 Atomic record schema | `inherited bullet` | short title; | `MAPPED` | `PES-REQ-0021` | — |
| `T2-0421` | 10.2 Atomic record schema | `inherited bullet` | normative keyword; | `MAPPED` | `PES-REQ-0022` | — |
| `T2-0422` | 10.2 Atomic record schema | `inherited bullet` | one atomic, testable statement; | `MAPPED` | `PES-REQ-0023` | — |
| `T2-0423` | 10.2 Atomic record schema | `inherited bullet` | rationale; | `MAPPED` | `PES-REQ-0024` | — |
| `T2-0424` | 10.2 Atomic record schema | `inherited bullet` | scope/component; | `MAPPED` | `PES-REQ-0025` | — |
| `T2-0425` | 10.2 Atomic record schema | `inherited bullet` | source pointer and research classification; | `MAPPED` | `PES-REQ-0026` | — |
| `T2-0426` | 10.2 Atomic record schema | `inherited bullet` | IP class and disposition; | `MAPPED` | `PES-REQ-0027` | — |
| `T2-0427` | 10.2 Atomic record schema | `inherited bullet` | dependencies; | `MAPPED` | `PES-REQ-0028` | — |
| `T2-0428` | 10.2 Atomic record schema | `inherited bullet` | target software release or milestone; | `MAPPED` | `PES-REQ-0029` | — |
| `T2-0429` | 10.2 Atomic record schema | `inherited bullet` | current truth state; | `MAPPED` | `PES-REQ-0030` | — |
| `T2-0430` | 10.2 Atomic record schema | `inherited bullet` | positive acceptance condition; | `MAPPED` | `PES-REQ-0031` | — |
| `T2-0431` | 10.2 Atomic record schema | `inherited bullet` | negative acceptance condition; | `MAPPED` | `PES-REQ-0032` | — |
| `T2-0432` | 10.2 Atomic record schema | `inherited bullet` | verification IDs; | `MAPPED` | `PES-REQ-0033` | — |
| `T2-0433` | 10.2 Atomic record schema | `inherited bullet` | ADR/decision/change links; | `MAPPED` | `PES-REQ-0034` | — |
| `T2-0434` | 10.2 Atomic record schema | `inherited bullet` | owner and reviewer. | `MAPPED` | `PES-REQ-0035` | — |
| `T2-0435` | 10.2 Atomic record schema | `explicit` | [PES-REQ-0005] MUST split compound requirements when one part could pass and another fail. | `MAPPED` | `PES-REQ-0005` | — |
| `T2-0436` | 10.2 Atomic record schema | `explicit` | [PES-REQ-0006] MUST map every implemented behavior to at least one requirement and every MUST/MUST NOT requirement to positive or negative verification. | `MAPPED` | `PES-REQ-0006` | — |
| `T2-0437` | 10.2 Atomic record schema | `explicit` | [PES-REQ-0007] MUST map every test to the requirements it verifies. | `MAPPED` | `PES-REQ-0007` | — |
| `T2-0438` | 10.2 Atomic record schema | `explicit` | Orphan tests and unverified requirements shall be visible in CI reports. | `MAPPED` | `PES-REQ-0007` | — |

#### Page 31 — 15 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0439` | 10.3 Truth states | `explicit` | [PES-REQ-0008] MUST use VERIFIED as the only state equivalent to complete. | `MAPPED` | `PES-REQ-0008` | — |
| `T2-0440` | 10.3 Truth states | `explicit` | [PES-REQ-0009] MUST NOT calculate percent complete from file count, package count, UI controls, lines of code, passing compilation, or SCAFFOLDED/PARTIAL items. | `MAPPED` | `PES-REQ-0009` | — |
| `T2-0441` | 10.4 Change control | `explicit` | [PES-GOV-0014] MUST create a change record for any alteration to a Phase 1 requirement, authority rule, safety boundary, clean-room rule, canonical term, or architecture invariant. | `MAPPED` | `PES-GOV-0014` | — |
| `T2-0442` | 10.4 Change control | `explicit` | [PES-GOV-0015] MUST include reason, affected IDs, research/evidence impact, security/IP impact, migration impact, test impact, decision authority, approval date, and supersession links. | `MAPPED` | `PES-GOV-0015` | — |
| `T2-0443` | 10.4 Change control | `explicit` | [PES-GOV-0016] MUST NOT edit a controlling requirement only in code or an ADR. | `MAPPED` | `PES-GOV-0016` | — |
| `T2-0444` | 10.4 Change control | `explicit` | The directive and traceability records shall change first or in the same approved change. | `MAPPED` | `PES-GOV-0016` | — |
| `T2-0445` | 11.1 Decisions Codex may make | `explicit` | [PES-DEC-0001] MAY let Codex decide an implementation detail without asking only when every plausible choice: | `MAPPED` | `PES-DEC-0007`<br>`PES-DEC-0008`<br>`PES-DEC-0009`<br>`PES-DEC-0010`<br>`PES-DEC-0011`<br>`PES-DEC-0012`<br>`PES-DEC-0013`<br>`PES-DEC-0014` | `PES-DEC-0001` |
| `T2-0446` | 11.1 Decisions Codex may make | `inherited bullet` | is internal and reversible; | `MAPPED` | `PES-DEC-0007` | `PES-DEC-0001` |
| `T2-0447` | 11.1 Decisions Codex may make | `inherited bullet` | preserves observable semantics and file compatibility; | `MAPPED` | `PES-DEC-0008` | `PES-DEC-0001` |
| `T2-0448` | 11.1 Decisions Codex may make | `inherited bullet` | adds no network, device, native, process, plugin, cloud, AI, or credential capability; | `MAPPED` | `PES-DEC-0009` | `PES-DEC-0001` |
| `T2-0449` | 11.1 Decisions Codex may make | `inherited bullet` | does not affect IP classification, branding, public claims, grading, teacher/student separation, privacy, or safety; | `MAPPED` | `PES-DEC-0010` | `PES-DEC-0001` |
| `T2-0450` | 11.1 Decisions Codex may make | `inherited bullet` | stays within approved technology and dependency policy; | `MAPPED` | `PES-DEC-0011` | `PES-DEC-0001` |
| `T2-0451` | 11.1 Decisions Codex may make | `inherited bullet` | can be objectively verified; | `MAPPED` | `PES-DEC-0012` | `PES-DEC-0001` |
| `T2-0452` | 11.1 Decisions Codex may make | `inherited bullet` | satisfies all higher requirements. | `MAPPED` | `PES-DEC-0013` | `PES-DEC-0001` |
| `T2-0453` | 11.1 Decisions Codex may make | `explicit` | Meaningful autonomous decisions shall still be recorded in an ADR or implementation note. | `MAPPED` | `PES-DEC-0014` | `PES-DEC-0001` |

#### Page 32 — 27 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0454` | 11.2 Mandatory stop categories | `explicit` | [PES-DEC-0002] MUST stop the affected work and ask Scott when a choice: | `MAPPED` | `PES-DEC-0015`<br>`PES-DEC-0016`<br>`PES-DEC-0017`<br>`PES-DEC-0018`<br>`PES-DEC-0019`<br>`PES-DEC-0020`<br>`PES-DEC-0021`<br>`PES-DEC-0022`<br>`PES-DEC-0023`<br>`PES-DEC-0024`<br>`PES-DEC-0025`<br>`PES-DEC-0026` | `PES-DEC-0002` |
| `T2-0455` | 11.2 Mandatory stop categories | `inherited bullet` | touches physical/network capability or could weaken the VirtualUniverse wall; | `MAPPED` | `PES-DEC-0015` | `PES-DEC-0002` |
| `T2-0456` | 11.2 Mandatory stop categories | `inherited bullet` | uses or resembles vendor assets, protocols, APIs, formats, names, model numbers, diagnostics, branding, or trade dress; | `MAPPED` | `PES-DEC-0016` | `PES-DEC-0002` |
| `T2-0457` | 11.2 Mandatory stop categories | `inherited bullet` | changes public workflow semantics, TrainingProfile behavior, file format, migration, grading, Teacher Mode visibility, or student data handling; | `MAPPED` | `PES-DEC-0017` | `PES-DEC-0002` |
| `T2-0458` | 11.2 Mandatory stop categories | `inherited bullet` | risks data loss, irreversible schema change, or backward incompatibility; | `MAPPED` | `PES-DEC-0018` | `PES-DEC-0002` |
| `T2-0459` | 11.2 Mandatory stop categories | `inherited bullet` | requires cloud, credentials, telemetry, remote services, external AI, or an updater; | `MAPPED` | `PES-DEC-0019` | `PES-DEC-0002` |
| `T2-0460` | 11.2 Mandatory stop categories | `inherited bullet` | adds eval, arbitrary scripting, FFI, child process, shell, native bridge, host device, generic transport, or executable plugin capability; | `MAPPED` | `PES-DEC-0020` | `PES-DEC-0002` |
| `T2-0461` | 11.2 Mandatory stop categories | `inherited bullet` | is marked NEEDS MORE RESEARCH, Class 7, Class 8, or professional legal review; | `MAPPED` | `PES-DEC-0021` | `PES-DEC-0002` |
| `T2-0462` | 11.2 Mandatory stop categories | `inherited bullet` | makes a safety, certification, compatibility, equivalence, endorsement, or production claim; | `MAPPED` | `PES-DEC-0022` | `PES-DEC-0002` |
| `T2-0463` | 11.2 Mandatory stop categories | `inherited bullet` | conflicts with higher authority; | `MAPPED` | `PES-DEC-0023` | `PES-DEC-0002` |
| `T2-0464` | 11.2 Mandatory stop categories | `inherited bullet` | cannot be verified objectively; | `MAPPED` | `PES-DEC-0024` | `PES-DEC-0002` |
| `T2-0465` | 11.2 Mandatory stop categories | `inherited bullet` | would choose initial operating systems or the production packaging model; | `MAPPED` | `PES-DEC-0025` | `PES-DEC-0002` |
| `T2-0466` | 11.2 Mandatory stop categories | `inherited bullet` | would expand scope beyond the authored phases. | `MAPPED` | `PES-DEC-0026` | `PES-DEC-0002` |
| `T2-0467` | 11.2 Mandatory stop categories | `explicit` | [PES-DEC-0003] MUST stop and request additional verified research rather than invent exact controller-family OB numbers, priority/preemption matrices, nesting limits, recursion behavior, proprietary optimized DB layouts, vendor-specific conversions/built-ins, force edge cases, diagnostic numbers/prose, auto-tuning, complex motion, or legacy-language semantics. | `MAPPED` | `PES-DEC-0003` | — |
| `T2-0468` | 11.3 BLOCKED decision record | `explicit` | Every blocked decision request shall contain: | `MAPPED` | `PES-DEC-0027`<br>`PES-DEC-0028`<br>`PES-DEC-0029`<br>`PES-DEC-0030`<br>`PES-DEC-0031`<br>`PES-DEC-0032`<br>`PES-DEC-0033`<br>`PES-DEC-0034`<br>`PES-DEC-0035`<br>`PES-DEC-0036`<br>`PES-DEC-0037` | — |
| `T2-0469` | 11.3 BLOCKED decision record | `inherited line` | Decision ID: | `MAPPED` | `PES-DEC-0027` | — |
| `T2-0470` | 11.3 BLOCKED decision record | `inherited line` | Affected requirement IDs: | `MAPPED` | `PES-DEC-0028` | — |
| `T2-0471` | 11.3 BLOCKED decision record | `inherited line` | Known facts: | `MAPPED` | `PES-DEC-0029` | — |
| `T2-0472` | 11.3 BLOCKED decision record | `inherited line` | Unknown or conflicting point: | `MAPPED` | `PES-DEC-0030` | — |
| `T2-0473` | 11.3 BLOCKED decision record | `inherited line` | Why Codex cannot decide safely: | `MAPPED` | `PES-DEC-0031` | — |
| `T2-0474` | 11.3 BLOCKED decision record | `inherited line` | Option A and impact: | `MAPPED` | `PES-DEC-0032` | — |
| `T2-0475` | 11.3 BLOCKED decision record | `inherited line` | Option B and impact: | `MAPPED` | `PES-DEC-0033` | — |
| `T2-0476` | 11.3 BLOCKED decision record | `inherited line` | Option C and impact, if useful: | `MAPPED` | `PES-DEC-0034` | — |
| `T2-0477` | 11.3 BLOCKED decision record | `inherited line` | Recommended option: | `MAPPED` | `PES-DEC-0035` | — |
| `T2-0478` | 11.3 BLOCKED decision record | `inherited line` | Exact approval or evidence needed: | `MAPPED` | `PES-DEC-0036` | — |
| `T2-0479` | 11.3 BLOCKED decision record | `inherited line` | Work that can continue: | `MAPPED` | `PES-DEC-0037` | — |
| `T2-0480` | 11.3 BLOCKED decision record | `explicit` | [PES-DEC-0004] MUST bundle related questions so Scott receives the smallest coherent decision request. | `MAPPED` | `PES-DEC-0004` | — |

#### Page 33 — 24 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0481` | 11.3 BLOCKED decision record | `explicit` | [PES-DEC-0005] MUST continue unrelated work while the affected area remains blocked. | `MAPPED` | `PES-DEC-0005` | — |
| `T2-0482` | 11.3 BLOCKED decision record | `explicit` | [PES-DEC-0006] MUST NOT treat silence, elapsed time, a placeholder, "close as possible," "educational," or an implementation guess as approval. | `MAPPED` | `PES-DEC-0006` | — |
| `T2-0483` | 12.1 Forbidden implementation theater | `explicit` | [PES-QLT-0001] MUST NOT count a feature as implemented because a pane, button, menu item, type, interface, package, schema field, animation, sample, or mocked path exists. | `MAPPED` | `PES-QLT-0001` | — |
| `T2-0484` | 12.1 Forbidden implementation theater | `explicit` | [PES-QLT-0002] MUST NOT ship: | `MAPPED` | `PES-QLT-0009`<br>`PES-QLT-0010`<br>`PES-QLT-0011`<br>`PES-QLT-0012`<br>`PES-QLT-0013`<br>`PES-QLT-0014`<br>`PES-QLT-0015`<br>`PES-QLT-0016`<br>`PES-QLT-0017`<br>`PES-QLT-0018`<br>`PES-QLT-0019`<br>`PES-QLT-0020`<br>`PES-QLT-0021`<br>`PES-QLT-0022` | `PES-QLT-0002` |
| `T2-0485` | 12.1 Forbidden implementation theater | `inherited bullet` | no-op commands; | `MAPPED` | `PES-QLT-0009` | `PES-QLT-0002` |
| `T2-0486` | 12.1 Forbidden implementation theater | `inherited bullet` | hard-coded success or catch-and-return-success; | `MAPPED` | `PES-QLT-0010` | `PES-QLT-0002` |
| `T2-0487` | 12.1 Forbidden implementation theater | `inherited bullet` | fake compile, load, online, scan, force, HMI, or diagnostic animations; | `MAPPED` | `PES-QLT-0011` | `PES-QLT-0002` |
| `T2-0488` | 12.1 Forbidden implementation theater | `inherited bullet` | canned errors or predetermined lesson results; | `MAPPED` | `PES-QLT-0012` | `PES-QLT-0002` |
| `T2-0489` | 12.1 Forbidden implementation theater | `inherited bullet` | sample-specific PLC/process logic in general engines; | `MAPPED` | `PES-QLT-0013` | `PES-QLT-0002` |
| `T2-0490` | 12.1 Forbidden implementation theater | `inherited bullet` | offline values displayed as monitored runtime values; | `MAPPED` | `PES-QLT-0014` | `PES-QLT-0002` |
| `T2-0491` | 12.1 Forbidden implementation theater | `inherited bullet` | HMI animation disconnected from InternalTagBus; | `MAPPED` | `PES-QLT-0015` | `PES-QLT-0002` |
| `T2-0492` | 12.1 Forbidden implementation theater | `inherited bullet` | mock/test doubles reachable in production; | `MAPPED` | `PES-QLT-0016` | `PES-QLT-0002` |
| `T2-0493` | 12.1 Forbidden implementation theater | `inherited bullet` | a regex-only compiler; | `MAPPED` | `PES-QLT-0017` | `PES-QLT-0002` |
| `T2-0494` | 12.1 Forbidden implementation theater | `inherited bullet` | SCL eval; | `MAPPED` | `PES-QLT-0018` | `PES-QLT-0002` |
| `T2-0495` | 12.1 Forbidden implementation theater | `inherited bullet` | LAD/FBD execution based on screen coordinates; | `MAPPED` | `PES-QLT-0019` | `PES-QLT-0002` |
| `T2-0496` | 12.1 Forbidden implementation theater | `inherited bullet` | hidden physical adapters disabled by configuration; | `MAPPED` | `PES-QLT-0020` | `PES-QLT-0002` |
| `T2-0497` | 12.1 Forbidden implementation theater | `inherited bullet` | scenario code that directly awards a pass; | `MAPPED` | `PES-QLT-0021` | `PES-QLT-0002` |
| `T2-0498` | 12.1 Forbidden implementation theater | `inherited bullet` | a "coming soon" control that appears operational. | `MAPPED` | `PES-QLT-0022` | `PES-QLT-0002` |
| `T2-0499` | 12.1 Forbidden implementation theater | `explicit` | [PES-QLT-0003] MUST fail closed when a feature is unavailable. | `MAPPED` | `PES-QLT-0003` | — |
| `T2-0500` | 12.1 Forbidden implementation theater | `explicit` | The UI shall honestly disable or omit the action and identify the unmet capability without pretending success. | `MAPPED` | `PES-QLT-0003` | — |
| `T2-0501` | 12.2 Permitted scaffolding | `explicit` | [PES-QLT-0004] MAY create scaffolding only when it: | `MAPPED` | `PES-QLT-0023`<br>`PES-QLT-0024`<br>`PES-QLT-0025`<br>`PES-QLT-0026`<br>`PES-QLT-0027`<br>`PES-QLT-0028` | `PES-QLT-0004` |
| `T2-0502` | 12.2 Permitted scaffolding | `inherited bullet` | is not user-visible or release-reachable; | `MAPPED` | `PES-QLT-0023` | `PES-QLT-0004` |
| `T2-0503` | 12.2 Permitted scaffolding | `inherited bullet` | contains no forbidden capability; | `MAPPED` | `PES-QLT-0024` | `PES-QLT-0004` |
| `T2-0504` | 12.2 Permitted scaffolding | `inherited bullet` | fails closed; | `MAPPED` | `PES-QLT-0025` | `PES-QLT-0004` |

#### Page 34 — 23 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0505` | 12.2 Permitted scaffolding | `inherited bullet` | is labeled SCAFFOLDED in the implementation matrix; | `MAPPED` | `PES-QLT-0026` | `PES-QLT-0004` |
| `T2-0506` | 12.2 Permitted scaffolding | `inherited bullet` | has an owner and removal/completion target; | `MAPPED` | `PES-QLT-0027` | `PES-QLT-0004` |
| `T2-0507` | 12.2 Permitted scaffolding | `inherited bullet` | earns zero completion credit. | `MAPPED` | `PES-QLT-0028` | `PES-QLT-0004` |
| `T2-0508` | 12.2 Permitted scaffolding | `explicit` | [PES-QLT-0005] MUST NOT create an abstract physical connection, generic transport, executable plugin host, network-capable HMI provider, or arbitrary scripting engine even as scaffolding. | `MAPPED` | `PES-QLT-0005` | — |
| `T2-0509` | 12.3 Universal milestone Definition of Done | `explicit` | [PES-QLT-0006] MUST require every software milestone, when later authorized, to include: | `MAPPED` | `PES-QLT-0029`<br>`PES-QLT-0030`<br>`PES-QLT-0031`<br>`PES-QLT-0032`<br>`PES-QLT-0033`<br>`PES-QLT-0034`<br>`PES-QLT-0035`<br>`PES-QLT-0036`<br>`PES-QLT-0037`<br>`PES-QLT-0038`<br>`PES-QLT-0039`<br>`PES-QLT-0040`<br>`PES-QLT-0041`<br>`PES-QLT-0042`<br>`PES-QLT-0043`<br>`PES-QLT-0044` | `PES-QLT-0006` |
| `T2-0510` | 12.3 Universal milestone Definition of Done | `inherited bullet` | domain model and ownership; | `MAPPED` | `PES-QLT-0029` | `PES-QLT-0006` |
| `T2-0511` | 12.3 Universal milestone Definition of Done | `inherited bullet` | invariants; | `MAPPED` | `PES-QLT-0030` | `PES-QLT-0006` |
| `T2-0512` | 12.3 Universal milestone Definition of Done | `inherited bullet` | positive behavior; | `MAPPED` | `PES-QLT-0031` | `PES-QLT-0006` |
| `T2-0513` | 12.3 Universal milestone Definition of Done | `inherited bullet` | negative behavior; | `MAPPED` | `PES-QLT-0032` | `PES-QLT-0006` |
| `T2-0514` | 12.3 Universal milestone Definition of Done | `inherited bullet` | enumerated failure cases and recovery; | `MAPPED` | `PES-QLT-0033` | `PES-QLT-0006` |
| `T2-0515` | 12.3 Universal milestone Definition of Done | `inherited bullet` | stable identity and dependency behavior; | `MAPPED` | `PES-QLT-0034` | `PES-QLT-0006` |
| `T2-0516` | 12.3 Universal milestone Definition of Done | `inherited bullet` | persistence, migration, and undo where applicable; | `MAPPED` | `PES-QLT-0035` | `PES-QLT-0006` |
| `T2-0517` | 12.3 Universal milestone Definition of Done | `inherited bullet` | real UI integration where applicable; | `MAPPED` | `PES-QLT-0036` | `PES-QLT-0006` |
| `T2-0518` | 12.3 Universal milestone Definition of Done | `inherited bullet` | end-to-end workflow; | `MAPPED` | `PES-QLT-0037` | `PES-QLT-0006` |
| `T2-0519` | 12.3 Universal milestone Definition of Done | `inherited bullet` | deterministic unit/integration tests; | `MAPPED` | `PES-QLT-0038` | `PES-QLT-0006` |
| `T2-0520` | 12.3 Universal milestone Definition of Done | `inherited bullet` | property/fuzz/golden tests where applicable; | `MAPPED` | `PES-QLT-0039` | `PES-QLT-0006` |
| `T2-0521` | 12.3 Universal milestone Definition of Done | `inherited bullet` | isolation/security tests; | `MAPPED` | `PES-QLT-0040` | `PES-QLT-0006` |
| `T2-0522` | 12.3 Universal milestone Definition of Done | `inherited bullet` | clean-room evidence and asset provenance; | `MAPPED` | `PES-QLT-0041` | `PES-QLT-0006` |
| `T2-0523` | 12.3 Universal milestone Definition of Done | `inherited bullet` | documentation; | `MAPPED` | `PES-QLT-0042` | `PES-QLT-0006` |
| `T2-0524` | 12.3 Universal milestone Definition of Done | `inherited bullet` | requirement-to-test traceability; | `MAPPED` | `PES-QLT-0043` | `PES-QLT-0006` |
| `T2-0525` | 12.3 Universal milestone Definition of Done | `inherited bullet` | reproducible verification evidence. | `MAPPED` | `PES-QLT-0044` | `PES-QLT-0006` |
| `T2-0526` | 12.3 Universal milestone Definition of Done | `explicit` | [PES-QLT-0007] MUST NOT advance a milestone on screenshots, a successful build, a smoke test, a happy-path demo, or manual assertion alone. | `MAPPED` | `PES-QLT-0007` | — |
| `T2-0527` | 12.3 Universal milestone Definition of Done | `explicit` | [PES-QLT-0008] MUST keep a milestone open if any required test is skipped, flaky, unavailable, manually waived, or inconclusive. | `MAPPED` | `PES-QLT-0008` | — |

#### Page 35 — 13 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0528` | 13.1 One document, four authoring phases | `explicit` | [PES-GOV-0017] MUST append and revise this same document for Phases 2-4 while preserving: | `MAPPED` | `PES-GOV-0032`<br>`PES-GOV-0033`<br>`PES-GOV-0034`<br>`PES-GOV-0035`<br>`PES-GOV-0036`<br>`PES-GOV-0037`<br>`PES-GOV-0038`<br>`PES-GOV-0039` | `PES-GOV-0017` |
| `T2-0529` | 13.1 One document, four authoring phases | `inherited bullet` | exact filename; | `MAPPED` | `PES-GOV-0032` | `PES-GOV-0017` |
| `T2-0530` | 13.1 One document, four authoring phases | `inherited bullet` | style system; | `MAPPED` | `PES-GOV-0033` | `PES-GOV-0017` |
| `T2-0531` | 13.1 One document, four authoring phases | `inherited bullet` | requirement IDs; | `MAPPED` | `PES-GOV-0034` | `PES-GOV-0017` |
| `T2-0532` | 13.1 One document, four authoring phases | `inherited bullet` | cross references; | `MAPPED` | `PES-GOV-0035` | `PES-GOV-0017` |
| `T2-0533` | 13.1 One document, four authoring phases | `inherited bullet` | source hash history; | `MAPPED` | `PES-GOV-0036` | `PES-GOV-0017` |
| `T2-0534` | 13.1 One document, four authoring phases | `inherited bullet` | change ledger; | `MAPPED` | `PES-GOV-0037` | `PES-GOV-0017` |
| `T2-0535` | 13.1 One document, four authoring phases | `inherited bullet` | superseded requirement tombstones; | `MAPPED` | `PES-GOV-0038` | `PES-GOV-0017` |
| `T2-0536` | 13.1 One document, four authoring phases | `inherited bullet` | open decisions and risk records. | `MAPPED` | `PES-GOV-0039` | `PES-GOV-0017` |
| `T2-0537` | 13.1 One document, four authoring phases | `explicit` | [PES-GOV-0018] MUST NOT create separate competing master directives for later phases. | `MAPPED` | `PES-GOV-0018` | — |
| `T2-0538` | 13.1 One document, four authoring phases | `explicit` | [PES-GOV-0019] MUST label unauthored later-phase material as reserved. | `MAPPED` | `PES-GOV-0019` | — |
| `T2-0539` | 13.1 One document, four authoring phases | `explicit` | It shall not create empty chapters that could be mistaken for complete requirements. | `MAPPED` | `PES-GOV-0019` | — |
| `T2-0540` | 13.1 One document, four authoring phases | `explicit` | [PES-GOV-0020] MUST perform a cross-phase contradiction and coverage audit after every authoring phase. | `MAPPED` | `PES-GOV-0020` | — |

#### Page 36 — 4 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0541` | 13.2 Phase 1 acceptance checklist | `explicit` | [PES-ACC-0005] MUST mark this revision "Phase 1 authored; Phases 2-4 not yet authored." | `MAPPED` | `PES-ACC-0005` | — |
| `T2-0542` | 13.2 Phase 1 acceptance checklist | `explicit` | [PES-ACC-0006] MUST NOT treat completion of Phase 1 authoring as completion of the master directive. | `MAPPED` | `PES-ACC-0006` | — |
| `T2-0543` | 13.2 Phase 1 acceptance checklist | `explicit` | [PES-ACC-0007] MUST NOT authorize product coding from this incomplete directive unless Scott separately gives explicit implementation authorization before Phases 2-4 are complete. | `MAPPED` | `PES-ACC-0007` | — |
| `T2-0544` | 13.3 Open decisions carried forward | `explicit table row` | OQ-0008 \| Accessibility conformance target and performance/capacity budgets \| Must be objective before experience acceptance \| Phase 3 | `MAPPED` | `PES-ACC-0008`<br>`PES-ACC-0009`<br>`PES-ACC-0010` | — |

#### Page 37 — 0 statement unit(s)

No in-scope statement unit was identified on this page.

#### Page 38 — 1 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0545` | Appendix A. Canonical Glossary | `explicit table row` | Engineering timestamp \| Human-facing wall-clock metadata; never authoritative simulation time | `MAPPED` | `PES-DET-0002`<br>`PES-DET-0005` | — |

#### Page 39 — 0 statement unit(s)

No in-scope statement unit was identified on this page.

#### Page 40 — 1 statement unit(s)

| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |
|---|---|---|---|---|---|---|
| `T2-0546` | Appendix F. Phase 1 "Do Not Build This" Register | `explicit` | The product may become broad, realistic, polished, and deeply functional only inside these boundaries. | `MAPPED` | `PES-SCP-0001`<br>`PES-ISO-0001`<br>`PES-CRM-0001`<br>`PES-DET-0001`<br>`PES-FID-0002` | — |

<!-- LEDGER_END -->

### Coverage conclusion

The ledger accounts for all 40 pages and all 546 in-scope source statements under the stated trigger and inheritance rules. Of those, 498 have a direct, inherited, or materially equivalent source requirement-ID mapping; 48 remain UNMAPPED. No repository file was treated as authoritative evidence for either recall or mapping.

## Task 3 — Count reconciliation

#### 3.1 What the directive actually states

- No explicit numeric requirement total occurs in the supplied DOCX.
- Independent marker count: **247 unique issued IDs**, with no duplicates.
- Every issued ID has one recognized leading keyword. Distribution: **MUST 184**, **MUST NOT 50**, **SHOULD 5**, **MAY 8**; no issued lead uses SHALL/SHALL NOT/SHOULD NOT.
- The frozen research has **0 issued PES markers** and **0 occurrences of the token 247**.

#### 3.2 Area-by-area issued-ID reconciliation

| Area | Count | Observed range | Internal gaps |
|---|---:|---|---|
| ACC | 7 | 0001-0007 | none |
| ARC | 30 | 0001-0030 | none |
| CI | 3 | 0001-0003 | none |
| CRM | 25 | 0001-0025 | none |
| DEC | 6 | 0001-0006 | none |
| DET | 7 | 0001-0007 | none |
| DEV | 12 | 0001-0012 | none |
| DIA | 6 | 0001-0006 | none |
| DOC | 4 | 0001-0004 | none |
| EDU | 6 | 0001-0006 | none |
| FID | 8 | 0001-0008 | none |
| GOV | 20 | 0001-0020 | none |
| IR | 5 | 0001-0005 | none |
| ISO | 22 | 0001-0022 | none |
| MSN | 9 | 0001-0009 | none |
| PRJ | 7 | 0001-0007 | none |
| PROF | 6 | 0001-0006 | none |
| QLT | 8 | 0001-0008 | none |
| REQ | 9 | 0001-0009 | none |
| SCP | 10 | 0001-0010 | none |
| SEC | 25 | 0001-0025 | none |
| TCH | 5 | 0001-0005 | none |
| TYP | 2 | 0001-0002 | none |
| VOC | 5 | 0001-0005 | none |
| **Total** | **247** | - | - |

#### 3.3 Normative-statement signals versus issued IDs

- Case-sensitive uppercase normative-keyword matches inside issued record bodies: **249**. The two matches beyond the 247 leads are the category names MUST and MUST NOT inside PES-REQ-0006; they are not two newly issued requirements.
- Case-insensitive modal matches inside issued record bodies: **302**. **47 IDs** contain more than one modal match; there are **55** matches beyond the lead keywords. Two of those are the PES-REQ-0006 category-name references, leaving 53 secondary deontic-looking tokens.
- Case-insensitive modal matches across all DOCX body text: **328**. Of the **26** matches outside issued record bodies, **10** are clearly keyword definitions/headings/labels and **16** occur in un-ID'd prose or decision-table text whose inclusion requires a semantic rule.
- These lexical counts are not an atomic requirement total. One issued ID may govern multiple coordinated clauses or list items without repeating a modal; conversely, a modal token may be a label, quotation, description, or subordinate condition.

IDs with more than one case-insensitive modal token:

| ID | Modal-token count |
|---|---:|
| PES-ACC-0001 | 2 |
| PES-ARC-0001 | 5 |
| PES-ARC-0005 | 2 |
| PES-ARC-0009 | 2 |
| PES-ARC-0010 | 2 |
| PES-ARC-0013 | 2 |
| PES-ARC-0023 | 2 |
| PES-ARC-0026 | 2 |
| PES-CRM-0011 | 2 |
| PES-CRM-0018 | 2 |
| PES-DEC-0001 | 2 |
| PES-DEV-0009 | 2 |
| PES-DEV-0011 | 2 |
| PES-DIA-0001 | 2 |
| PES-EDU-0003 | 3 |
| PES-FID-0005 | 2 |
| PES-GOV-0001 | 2 |
| PES-GOV-0004 | 2 |
| PES-GOV-0006 | 2 |
| PES-GOV-0013 | 2 |
| PES-GOV-0016 | 2 |
| PES-GOV-0019 | 2 |
| PES-IR-0003 | 2 |
| PES-ISO-0003 | 2 |
| PES-ISO-0004 | 3 |
| PES-ISO-0015 | 2 |
| PES-ISO-0017 | 2 |
| PES-MSN-0009 | 2 |
| PES-PRJ-0005 | 2 |
| PES-PRJ-0007 | 3 |
| PES-PROF-0004 | 2 |
| PES-PROF-0005 | 2 |
| PES-QLT-0003 | 2 |
| PES-REQ-0004 | 2 |
| PES-REQ-0006 | 3 |
| PES-REQ-0007 | 2 |
| PES-SCP-0005 | 2 |
| PES-SEC-0003 | 2 |
| PES-SEC-0006 | 2 |
| PES-SEC-0008 | 2 |
| PES-SEC-0009 | 2 |
| PES-SEC-0019 | 2 |
| PES-SEC-0023 | 2 |
| PES-TCH-0002 | 2 |
| PES-TCH-0003 | 3 |
| PES-TCH-0004 | 2 |
| PES-VOC-0002 | 2 |

#### 3.4 Un-ID'd modal-bearing source blocks requiring recall-stream inclusion rules

The following source blocks are outside issued requirement bodies. They are provided verbatim so the Task 2 recall stream can be reconciled without treating a regex count as a semantic decision.

##### U-01

- **Container:** paragraph; Normal
- **Matched modal tokens:** shall
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
The product shall simulate engineering decisions and consequences with high training-transfer fidelity while remaining permanently incapable of communicating with or operating physical industrial equipment.
~~~~

##### U-02

- **Container:** paragraph; Normal
- **Matched modal tokens:** shall, shall, may
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
CONTROLLING PRODUCT TRUTH: Build a professional, brand-neutral PLC engineering and simulation environment for education. It shall provide high causal, behavioral, workflow, and training-transfer fidelity inside a wholly fictional VirtualUniverse. It shall never communicate with, discover, configure, commission, download to, or operate physical industrial equipment. No adapter to a physical universe may exist.
~~~~

##### U-03

- **Container:** paragraph; Normal
- **Matched modal tokens:** shall not
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
CONSTRUCTION STATUS: This is the first authoring phase of one living directive. Unless Scott separately orders otherwise, Codex shall not begin product implementation from this incomplete directive.
~~~~

##### U-04

- **Container:** paragraph; Normal
- **Matched modal tokens:** shall not
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
Later phases are intentionally absent from this revision. Reserved headings are not implementation requirements and shall not be inferred.
~~~~

##### U-05

- **Container:** table-row; no paragraph style
- **Matched modal tokens:** MUST, SHALL
- **Clearly meta/definitional:** yes

~~~~text
MUST / SHALL | Required. Violation blocks merge, release, or acceptance.
~~~~

##### U-06

- **Container:** table-row; no paragraph style
- **Matched modal tokens:** MUST NOT, SHALL NOT
- **Clearly meta/definitional:** yes

~~~~text
MUST NOT / SHALL NOT | Prohibited. Presence blocks merge, release, or acceptance.
~~~~

##### U-07

- **Container:** table-row; no paragraph style
- **Matched modal tokens:** SHOULD
- **Clearly meta/definitional:** yes

~~~~text
SHOULD | Expected unless a documented ADR proves an equal or stronger result without changing product intent.
~~~~

##### U-08

- **Container:** table-row; no paragraph style
- **Matched modal tokens:** SHOULD NOT
- **Clearly meta/definitional:** yes

~~~~text
SHOULD NOT | Avoid unless a documented ADR proves necessity and preserves all higher requirements.
~~~~

##### U-09

- **Container:** table-row; no paragraph style
- **Matched modal tokens:** MAY
- **Clearly meta/definitional:** yes

~~~~text
MAY | Optional and permitted only inside the approved scope.
~~~~

##### U-10

- **Container:** paragraph; Normal
- **Matched modal tokens:** shall
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
Every externally inspired requirement shall be classified before implementation:
~~~~

##### U-11

- **Container:** table-row; no paragraph style
- **Matched modal tokens:** May, shall not
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
Development environment | package managers, compilers, test servers, CI tools | May use development capabilities but shall not enter production
~~~~

##### U-12

- **Container:** paragraph; Normal
- **Matched modal tokens:** shall
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
Every meaningful mutation shall be a domain command. The minimum conceptual result is:
~~~~

##### U-13

- **Container:** paragraph; Normal
- **Matched modal tokens:** shall
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
Before feature implementation, the repository shall contain:
~~~~

##### U-14

- **Container:** paragraph; Normal
- **Matched modal tokens:** shall
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
Every requirement record shall contain:
~~~~

##### U-15

- **Container:** paragraph; Heading 2
- **Matched modal tokens:** may
- **Clearly meta/definitional:** yes

~~~~text
11.1 Decisions Codex may make
~~~~

##### U-16

- **Container:** paragraph; Normal
- **Matched modal tokens:** shall
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
Every blocked decision request shall contain:
~~~~

##### U-17

- **Container:** table-row; no paragraph style
- **Matched modal tokens:** Must
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
OQ-0008 | Accessibility conformance target and performance/capacity budgets | Must be objective before experience acceptance | Phase 3
~~~~

##### U-18

- **Container:** table-row; no paragraph style
- **Matched modal tokens:** MUST, MUST NOT
- **Clearly meta/definitional:** yes

~~~~text
Binding MUST/MUST NOT rules | Final Codex marching orders
~~~~

##### U-19

- **Container:** paragraph; Normal
- **Matched modal tokens:** may, may
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
Phase 1 closing rule: The foundation is now explicit. The product may become broad, realistic, polished, and deeply functional only inside these boundaries. No later phase may buy fidelity by weakening originality, determinism, causal behavior, or physical isolation.
~~~~

### 3.5 Final reconciliation against the completed recall ledger

The directive does not state a numeric requirement total. Exact searches of the DOCX text found neither the token `247` nor a requirement-total/count claim; the 247 figure is an independently counted population of unique issued `[PES-*]` markers, not a source-declared total. The full recall walk uses a conservative grammatical rule: 301 statement units contain an explicit trigger (294 prose/bullet statements plus 7 table rows), and 245 separately testable children inherit a modal lead-in (192 bullets, 37 schema lines, and 16 table rows).

The arithmetic is:

- **Statement units:** `546 = 301 explicit-trigger units + 245 modal-inherited children`.
- **Recall mapping:** `546 = 498 mapped + 48 UNMAPPED`.
- **Issued IDs:** `247 = 149 IDs mapping to exactly one statement + 92 IDs mapping to more than one statement + 6 IDs mapping to zero in-scope statements`.
- **Mapping links:** the 498 mapped statements produce 525 ID↔statement links because 17 statements are materially governed by more than one issued ID.
- **Atomicity:** 20 issued-ID records are separately labeled `COMPOUND_SOURCE_REQUIRES_REVIEW`; therefore neither the 247 markers nor the 546 recall units is an accepted atomic-requirement total.

The six zero-mapping IDs are not missing source text; they fall outside the audit directive's trigger vocabulary:

| Issued ID | Directive page | Modal | Explanation |
|---|---:|---|---|
| `PES-DEV-0001` | 26 | `SHOULD` | The recall trigger set intentionally excludes `SHOULD`. |
| `PES-DEV-0002` | 26 | `SHOULD` | The recall trigger set intentionally excludes `SHOULD`. |
| `PES-DEV-0003` | 26 | `SHOULD` | The recall trigger set intentionally excludes `SHOULD`. |
| `PES-DEV-0006` | 27 | `SHOULD` | The recall trigger set intentionally excludes `SHOULD`. |
| `PES-MSN-0006` | 7 | `SHOULD` | The recall trigger set intentionally excludes `SHOULD`. |
| `PES-SCP-0007` | 11 | `MAY` | This is unrestricted `MAY`, not the directive's requested `may … only` trigger. |

Every issued ID mapping to more than one in-scope source statement is listed below. The Task 2 ledger IDs are direct anchors to the page, section, grammatical form, modal lead-in when inherited, and verbatim source text.

| Issued ID | Statement count | Task 2 ledger anchors | Reconciliation |
|---|---:|---|---|
| `PES-ACC-0007` | 3 | `T2-0004`, `T2-0005`, `T2-0543` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-ARC-0001` | 2 | `T2-0271`, `T2-0272` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-ARC-0005` | 2 | `T2-0276`, `T2-0277` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-ARC-0009` | 2 | `T2-0281`, `T2-0282` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-ARC-0010` | 2 | `T2-0283`, `T2-0284` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-ARC-0012` | 2 | `T2-0286`, `T2-0295` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-ARC-0013` | 2 | `T2-0296`, `T2-0297` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-ARC-0022` | 19 | `T2-0321`, `T2-0322`, `T2-0323`, `T2-0324`, `T2-0325`, `T2-0326`, `T2-0327`, `T2-0328`, `T2-0329`, `T2-0330`, `T2-0331`, `T2-0332`, `T2-0333`, `T2-0334`, `T2-0335`, `T2-0336`, `T2-0337`, `T2-0338`, `T2-0339` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-ARC-0023` | 2 | `T2-0340`, `T2-0341` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-ARC-0026` | 2 | `T2-0351`, `T2-0352` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-CI-0001` | 15 | `T2-0389`, `T2-0390`, `T2-0391`, `T2-0392`, `T2-0393`, `T2-0394`, `T2-0395`, `T2-0396`, `T2-0397`, `T2-0398`, `T2-0399`, `T2-0400`, `T2-0401`, `T2-0402`, `T2-0403` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-CRM-0001` | 5 | `T2-0007`, `T2-0178`, `T2-0183`, `T2-0188`, `T2-0546` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-CRM-0004` | 4 | `T2-0180`, `T2-0185`, `T2-0186`, `T2-0188` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-CRM-0005` | 4 | `T2-0181`, `T2-0185`, `T2-0186`, `T2-0188` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-CRM-0006` | 2 | `T2-0190`, `T2-0192` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-CRM-0007` | 2 | `T2-0182`, `T2-0193` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-CRM-0009` | 8 | `T2-0194`, `T2-0195`, `T2-0196`, `T2-0197`, `T2-0198`, `T2-0199`, `T2-0200`, `T2-0201` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-CRM-0011` | 2 | `T2-0203`, `T2-0204` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-CRM-0012` | 2 | `T2-0187`, `T2-0205` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-CRM-0013` | 2 | `T2-0187`, `T2-0206` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-CRM-0016` | 2 | `T2-0209`, `T2-0364` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-CRM-0017` | 14 | `T2-0210`, `T2-0211`, `T2-0212`, `T2-0213`, `T2-0214`, `T2-0215`, `T2-0216`, `T2-0217`, `T2-0218`, `T2-0219`, `T2-0220`, `T2-0221`, `T2-0222`, `T2-0371` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-CRM-0018` | 2 | `T2-0223`, `T2-0224` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-CRM-0020` | 2 | `T2-0226`, `T2-0367` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-CRM-0021` | 10 | `T2-0227`, `T2-0228`, `T2-0229`, `T2-0230`, `T2-0231`, `T2-0232`, `T2-0233`, `T2-0234`, `T2-0235`, `T2-0372` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-DEC-0001` | 9 | `T2-0445`, `T2-0446`, `T2-0447`, `T2-0448`, `T2-0449`, `T2-0450`, `T2-0451`, `T2-0452`, `T2-0453` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-DEC-0002` | 13 | `T2-0454`, `T2-0455`, `T2-0456`, `T2-0457`, `T2-0458`, `T2-0459`, `T2-0460`, `T2-0461`, `T2-0462`, `T2-0463`, `T2-0464`, `T2-0465`, `T2-0466` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-DET-0001` | 2 | `T2-0314`, `T2-0546` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-DET-0002` | 2 | `T2-0315`, `T2-0545` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-DET-0005` | 2 | `T2-0318`, `T2-0545` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-DEV-0009` | 2 | `T2-0361`, `T2-0362` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-DIA-0001` | 2 | `T2-0344`, `T2-0345` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-DOC-0001` | 3 | `T2-0377`, `T2-0378`, `T2-0382` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-DOC-0003` | 5 | `T2-0377`, `T2-0379`, `T2-0380`, `T2-0381`, `T2-0384` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-EDU-0003` | 2 | `T2-0074`, `T2-0075` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-FID-0001` | 7 | `T2-0087`, `T2-0088`, `T2-0089`, `T2-0090`, `T2-0091`, `T2-0092`, `T2-0093` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-FID-0002` | 4 | `T2-0002`, `T2-0007`, `T2-0094`, `T2-0546` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-FID-0005` | 2 | `T2-0097`, `T2-0098` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-GOV-0001` | 7 | `T2-0011`, `T2-0012`, `T2-0013`, `T2-0014`, `T2-0015`, `T2-0016`, `T2-0017` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-GOV-0003` | 6 | `T2-0019`, `T2-0020`, `T2-0021`, `T2-0022`, `T2-0023`, `T2-0024` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-GOV-0004` | 2 | `T2-0025`, `T2-0026` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-GOV-0006` | 2 | `T2-0028`, `T2-0029` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-GOV-0013` | 2 | `T2-0036`, `T2-0037` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-GOV-0014` | 2 | `T2-0376`, `T2-0441` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-GOV-0015` | 2 | `T2-0376`, `T2-0442` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-GOV-0016` | 3 | `T2-0376`, `T2-0443`, `T2-0444` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-GOV-0017` | 9 | `T2-0528`, `T2-0529`, `T2-0530`, `T2-0531`, `T2-0532`, `T2-0533`, `T2-0534`, `T2-0535`, `T2-0536` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-GOV-0019` | 3 | `T2-0005`, `T2-0538`, `T2-0539` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-IR-0003` | 2 | `T2-0310`, `T2-0311` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-ISO-0001` | 5 | `T2-0003`, `T2-0007`, `T2-0112`, `T2-0191`, `T2-0546` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-ISO-0002` | 3 | `T2-0003`, `T2-0113`, `T2-0191` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-ISO-0003` | 2 | `T2-0114`, `T2-0115` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-ISO-0004` | 2 | `T2-0116`, `T2-0117` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-ISO-0008` | 11 | `T2-0121`, `T2-0122`, `T2-0123`, `T2-0124`, `T2-0125`, `T2-0126`, `T2-0127`, `T2-0128`, `T2-0129`, `T2-0130`, `T2-0131` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-ISO-0009` | 6 | `T2-0132`, `T2-0133`, `T2-0134`, `T2-0135`, `T2-0136`, `T2-0137` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-ISO-0017` | 2 | `T2-0171`, `T2-0172` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-MSN-0002` | 16 | `T2-0039`, `T2-0040`, `T2-0041`, `T2-0042`, `T2-0043`, `T2-0044`, `T2-0045`, `T2-0046`, `T2-0047`, `T2-0048`, `T2-0049`, `T2-0050`, `T2-0051`, `T2-0052`, `T2-0053`, `T2-0054` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-MSN-0003` | 3 | `T2-0001`, `T2-0002`, `T2-0055` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-MSN-0005` | 2 | `T2-0057`, `T2-0058` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-MSN-0009` | 2 | `T2-0061`, `T2-0062` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-PRJ-0005` | 2 | `T2-0244`, `T2-0245` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-PRJ-0007` | 2 | `T2-0247`, `T2-0248` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-PROF-0004` | 2 | `T2-0082`, `T2-0083` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-PROF-0005` | 2 | `T2-0084`, `T2-0085` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-QLT-0002` | 15 | `T2-0484`, `T2-0485`, `T2-0486`, `T2-0487`, `T2-0488`, `T2-0489`, `T2-0490`, `T2-0491`, `T2-0492`, `T2-0493`, `T2-0494`, `T2-0495`, `T2-0496`, `T2-0497`, `T2-0498` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-QLT-0003` | 2 | `T2-0499`, `T2-0500` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-QLT-0004` | 7 | `T2-0501`, `T2-0502`, `T2-0503`, `T2-0504`, `T2-0505`, `T2-0506`, `T2-0507` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-QLT-0006` | 17 | `T2-0509`, `T2-0510`, `T2-0511`, `T2-0512`, `T2-0513`, `T2-0514`, `T2-0515`, `T2-0516`, `T2-0517`, `T2-0518`, `T2-0519`, `T2-0520`, `T2-0521`, `T2-0522`, `T2-0523`, `T2-0524`, `T2-0525` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-REQ-0003` | 8 | `T2-0408`, `T2-0409`, `T2-0410`, `T2-0411`, `T2-0412`, `T2-0413`, `T2-0414`, `T2-0415` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-REQ-0004` | 3 | `T2-0006`, `T2-0416`, `T2-0417` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-REQ-0007` | 2 | `T2-0437`, `T2-0438` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-SCP-0001` | 2 | `T2-0102`, `T2-0546` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-SCP-0002` | 4 | `T2-0001`, `T2-0003`, `T2-0103`, `T2-0191` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-SCP-0005` | 2 | `T2-0106`, `T2-0107` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-SCP-0010` | 3 | `T2-0111`, `T2-0189`, `T2-0190` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-SEC-0001` | 8 | `T2-0139`, `T2-0140`, `T2-0141`, `T2-0142`, `T2-0143`, `T2-0144`, `T2-0145`, `T2-0146` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-SEC-0003` | 2 | `T2-0148`, `T2-0149` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-SEC-0005` | 2 | `T2-0150`, `T2-0250` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-SEC-0006` | 2 | `T2-0151`, `T2-0152` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-SEC-0008` | 2 | `T2-0154`, `T2-0155` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-SEC-0009` | 2 | `T2-0156`, `T2-0157` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-SEC-0012` | 2 | `T2-0160`, `T2-0249` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-SEC-0013` | 2 | `T2-0161`, `T2-0249` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-SEC-0014` | 2 | `T2-0162`, `T2-0249` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-SEC-0017` | 2 | `T2-0251`, `T2-0365` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-SEC-0019` | 2 | `T2-0253`, `T2-0254` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-SEC-0022` | 2 | `T2-0265`, `T2-0266` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-SEC-0023` | 2 | `T2-0267`, `T2-0268` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-SEC-0025` | 2 | `T2-0270`, `T2-0368` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-TCH-0002` | 2 | `T2-0257`, `T2-0258` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-TCH-0003` | 2 | `T2-0259`, `T2-0260` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |
| `PES-TCH-0004` | 2 | `T2-0261`, `T2-0262` | One issued source record or a materially equivalent source rule governs multiple separately testable sentence/list/table units; see the cited Task 2 ledger entries for page and verbatim text. |

The 17 statements mapped to more than one ID, which explain why 525 relationship links exceed 498 mapped statements, are:

| Task 2 ledger ID | Page | Mapped issued IDs |
|---|---:|---|
| `T2-0001` | 1 | `PES-MSN-0003`, `PES-SCP-0002` |
| `T2-0002` | 2 | `PES-MSN-0003`, `PES-FID-0002` |
| `T2-0003` | 2 | `PES-SCP-0002`, `PES-ISO-0001`, `PES-ISO-0002` |
| `T2-0005` | 2 | `PES-GOV-0019`, `PES-ACC-0007` |
| `T2-0007` | 3 | `PES-ISO-0001`, `PES-CRM-0001`, `PES-FID-0002` |
| `T2-0185` | 16 | `PES-CRM-0003`, `PES-CRM-0004`, `PES-CRM-0005` |
| `T2-0186` | 16 | `PES-CRM-0004`, `PES-CRM-0005` |
| `T2-0187` | 16 | `PES-CRM-0012`, `PES-CRM-0013` |
| `T2-0188` | 16 | `PES-CRM-0001`, `PES-CRM-0004`, `PES-CRM-0005` |
| `T2-0190` | 16 | `PES-CRM-0006`, `PES-SCP-0010` |
| `T2-0191` | 16 | `PES-SCP-0002`, `PES-ISO-0001`, `PES-ISO-0002` |
| `T2-0249` | 19 | `PES-SEC-0012`, `PES-SEC-0013`, `PES-SEC-0014` |
| `T2-0250` | 19 | `PES-SEC-0004`, `PES-SEC-0005` |
| `T2-0376` | 27 | `PES-GOV-0014`, `PES-GOV-0015`, `PES-GOV-0016` |
| `T2-0377` | 27 | `PES-DOC-0001`, `PES-DOC-0003` |
| `T2-0545` | 38 | `PES-DET-0002`, `PES-DET-0005` |
| `T2-0546` | 40 | `PES-SCP-0001`, `PES-ISO-0001`, `PES-CRM-0001`, `PES-DET-0001`, `PES-FID-0002` |

This reconciles the populations without treating any repository-generated matrix, register, report, or verifier output as source truth. It also exposes the central discrepancy: **247 is the count of issued source markers, while 546 is the conservative count of triggered or grammatically inherited normative statement units; the directive supplies no authoritative atomic total.**

<!-- COMPOUND_SPLIT_LEDGER_START -->

### 3.6 Post-remediation count and atomicity reconciliation

- Source parent IDs preserved: **247**.
- Total issued IDs after stable child allocation: **484**.
- Historical superseded compound parents: **20**.
- Atomic records: **464**; completion-eligible atomic records: **463**.
- Independently walked source statement units: **546**.
- Mapped / unmapped source units: **546 / 0**.
- Active source-unit-to-issued-ID relationships: **789**.

Counting rule: Every atomic child links to its modal lead-in and exact clause. Historical compound parents remain lineage only. File-inventory aliases that name a complete register fan out to every atomic child of that register contract.

The earlier 770-edge estimate was an audit hypothesis, not an authority. Rebuilding the full graph exposed legitimate complete-register alias fan-out and produced 789 edges. Historical parent lineage is recorded separately and is not counted as an active mapping edge.

#### Complete 20-parent / 190-child compound split ledger

##### PES-GOV-0001 → PES-GOV-0021, PES-GOV-0022, PES-GOV-0023, PES-GOV-0024, PES-GOV-0025, PES-GOV-0026

Historical parent source text: MUST interpret this project using the following order:<br>Applicable law, binding licenses, and the immutable product safety constraints in this directive form the outer boundary.<br>Scott's explicit, approved product decisions govern product intent.<br>This living Codex Master Implementation Directive governs what shall be built.<br>The frozen research report supplies technical, workflow, and risk evidence.<br>Approved decision records and ADRs govern implementation choices only within the authority left to them.<br>Code, tests, tickets, comments, mockups, and developer assumptions are subordinate to all items above.

| Ordinal | Atomic child ID | Exact governed clause |
|---:|---|---|
| 1 | `PES-GOV-0021` | Applicable law, binding licenses, and the immutable product safety constraints in this directive form the outer boundary. |
| 2 | `PES-GOV-0022` | Scott's explicit, approved product decisions govern product intent. |
| 3 | `PES-GOV-0023` | This living Codex Master Implementation Directive governs what shall be built. |
| 4 | `PES-GOV-0024` | The frozen research report supplies technical, workflow, and risk evidence. |
| 5 | `PES-GOV-0025` | Approved decision records and ADRs govern implementation choices only within the authority left to them. |
| 6 | `PES-GOV-0026` | Code, tests, tickets, comments, mockups, and developer assumptions are subordinate to all items above. |

##### PES-GOV-0003 → PES-GOV-0027, PES-GOV-0028, PES-GOV-0029, PES-GOV-0030, PES-GOV-0031

Historical parent source text: MUST treat the research report's labels accurately:<br>DOCUMENTED identifies publicly supported behavior or facts.<br>INFERENCE identifies a reasoned conclusion, not a documented exact behavior.<br>PROPOSED identifies simulator behavior recommended by the report.<br>LEGAL INTERPRETATION is risk analysis, not legal advice.<br>ENGINEERING RECOMMENDATION is an implementation or product judgment.

| Ordinal | Atomic child ID | Exact governed clause |
|---:|---|---|
| 1 | `PES-GOV-0027` | DOCUMENTED identifies publicly supported behavior or facts. |
| 2 | `PES-GOV-0028` | INFERENCE identifies a reasoned conclusion, not a documented exact behavior. |
| 3 | `PES-GOV-0029` | PROPOSED identifies simulator behavior recommended by the report. |
| 4 | `PES-GOV-0030` | LEGAL INTERPRETATION is risk analysis, not legal advice. |
| 5 | `PES-GOV-0031` | ENGINEERING RECOMMENDATION is an implementation or product judgment. |

##### PES-MSN-0002 → PES-MSN-0010, PES-MSN-0011, PES-MSN-0012, PES-MSN-0013, PES-MSN-0014, PES-MSN-0015, PES-MSN-0016, PES-MSN-0017, PES-MSN-0018, PES-MSN-0019, PES-MSN-0020, PES-MSN-0021, PES-MSN-0022, PES-MSN-0023, PES-MSN-0024

Historical parent source text: MUST make the student perform a recognizable modern PLC engineering lifecycle:<br>Create or open a simulator-native project.<br>Add fictional virtual controllers and devices.<br>Configure virtual racks, modules, channels, addresses, logical networks, and topology.<br>Create tags, constants, data types, and data blocks.<br>create OB, FC, FB, instance DB, and global DB program structures.<br>Program in LAD, FBD, and SCL (Structured Text).<br>Compile genuine project, hardware, language, type, and dependency semantics.<br>Repair real inconsistencies and rebuild.<br>Start a fictional controller instance.<br>Review an internal Virtual Load Preview.<br>Perform an atomic Virtual Download to a VirtualControllerId.<br>Use RUN, STOP, monitoring, watch, modify, force, and trace semantics.<br>Operate a deterministic virtual process and virtual HMI.<br>Diagnose causal code, hardware, network-graph, process, and HMI faults.<br>Correct the underlying cause and verify the result.

| Ordinal | Atomic child ID | Exact governed clause |
|---:|---|---|
| 1 | `PES-MSN-0010` | Create or open a simulator-native project. |
| 2 | `PES-MSN-0011` | Add fictional virtual controllers and devices. |
| 3 | `PES-MSN-0012` | Configure virtual racks, modules, channels, addresses, logical networks, and topology. |
| 4 | `PES-MSN-0013` | Create tags, constants, data types, and data blocks. |
| 5 | `PES-MSN-0014` | create OB, FC, FB, instance DB, and global DB program structures. |
| 6 | `PES-MSN-0015` | Program in LAD, FBD, and SCL (Structured Text). |
| 7 | `PES-MSN-0016` | Compile genuine project, hardware, language, type, and dependency semantics. |
| 8 | `PES-MSN-0017` | Repair real inconsistencies and rebuild. |
| 9 | `PES-MSN-0018` | Start a fictional controller instance. |
| 10 | `PES-MSN-0019` | Review an internal Virtual Load Preview. |
| 11 | `PES-MSN-0020` | Perform an atomic Virtual Download to a VirtualControllerId. |
| 12 | `PES-MSN-0021` | Use RUN, STOP, monitoring, watch, modify, force, and trace semantics. |
| 13 | `PES-MSN-0022` | Operate a deterministic virtual process and virtual HMI. |
| 14 | `PES-MSN-0023` | Diagnose causal code, hardware, network-graph, process, and HMI faults. |
| 15 | `PES-MSN-0024` | Correct the underlying cause and verify the result. |

##### PES-FID-0001 → PES-FID-0009, PES-FID-0010, PES-FID-0011, PES-FID-0012, PES-FID-0013, PES-FID-0014

Historical parent source text: MUST prioritize fidelity in this order:<br>Safety and physical isolation.<br>Correct domain semantics and causality.<br>Training-transfer workflow and state consequences.<br>Determinism, inspectability, and diagnostic navigation.<br>Professional interaction quality and accessibility.<br>Original visual polish.

| Ordinal | Atomic child ID | Exact governed clause |
|---:|---|---|
| 1 | `PES-FID-0009` | Safety and physical isolation. |
| 2 | `PES-FID-0010` | Correct domain semantics and causality. |
| 3 | `PES-FID-0011` | Training-transfer workflow and state consequences. |
| 4 | `PES-FID-0012` | Determinism, inspectability, and diagnostic navigation. |
| 5 | `PES-FID-0013` | Professional interaction quality and accessibility. |
| 6 | `PES-FID-0014` | Original visual polish. |

##### PES-ISO-0008 → PES-ISO-0023, PES-ISO-0024, PES-ISO-0025, PES-ISO-0026, PES-ISO-0027, PES-ISO-0028, PES-ISO-0029, PES-ISO-0030, PES-ISO-0031, PES-ISO-0032, PES-ISO-0033

Historical parent source text: MUST NOT contain or expose implementations of:<br>S7, S7comm, or S7comm-plus;<br>PROFINET DCP, PROFINET I/O, or PROFIBUS;<br>EtherNet/IP or CIP;<br>Modbus TCP or RTU;<br>external OPC UA;<br>EtherCAT, CAN, CANopen, DeviceNet, BACnet, MQTT, or other physical/industrial transports;<br>vendor PLC, HMI, drive, or I/O SDKs;<br>TIA Openness, Siemens engineering DLLs, or PLCSIM APIs;<br>physical device discovery or host NIC enumeration;<br>raw Ethernet, packet capture, or industrial protocol frames.<br>The list is illustrative. The category-wide ban controls renamed, wrapped, transitive, future, or equivalent capabilities.

| Ordinal | Atomic child ID | Exact governed clause |
|---:|---|---|
| 1 | `PES-ISO-0023` | S7, S7comm, or S7comm-plus; |
| 2 | `PES-ISO-0024` | PROFINET DCP, PROFINET I/O, or PROFIBUS; |
| 3 | `PES-ISO-0025` | EtherNet/IP or CIP; |
| 4 | `PES-ISO-0026` | Modbus TCP or RTU; |
| 5 | `PES-ISO-0027` | external OPC UA; |
| 6 | `PES-ISO-0028` | EtherCAT, CAN, CANopen, DeviceNet, BACnet, MQTT, or other physical/industrial transports; |
| 7 | `PES-ISO-0029` | vendor PLC, HMI, drive, or I/O SDKs; |
| 8 | `PES-ISO-0030` | TIA Openness, Siemens engineering DLLs, or PLCSIM APIs; |
| 9 | `PES-ISO-0031` | physical device discovery or host NIC enumeration; |
| 10 | `PES-ISO-0032` | raw Ethernet, packet capture, or industrial protocol frames. |
| 11 | `PES-ISO-0033` | The list is illustrative. The category-wide ban controls renamed, wrapped, transitive, future, or equivalent capabilities. |

##### PES-ISO-0009 → PES-ISO-0034, PES-ISO-0035, PES-ISO-0036, PES-ISO-0037, PES-ISO-0038

Historical parent source text: MUST NOT let shipped production code invoke or expose:<br>TCP, UDP, raw sockets, TLS, DNS, HTTP, HTTPS, local HTTP, localhost servers, or generic socket APIs;<br>fetch, XMLHttpRequest, WebSocket, WebRTC, EventSource to an endpoint, sendBeacon, WebTransport, or service-worker network interception;<br>WebSerial, WebUSB, WebBluetooth, WebHID, WebNFC, WebMIDI, or later equivalent device APIs;<br>serial ports, USB, Bluetooth, pcap, native device enumeration, or arbitrary filesystem devices;<br>child-process execution, shell commands, dynamic library loading, native FFI, dlopen, arbitrary native bridges, or plugins able to reach those capabilities.

| Ordinal | Atomic child ID | Exact governed clause |
|---:|---|---|
| 1 | `PES-ISO-0034` | TCP, UDP, raw sockets, TLS, DNS, HTTP, HTTPS, local HTTP, localhost servers, or generic socket APIs; |
| 2 | `PES-ISO-0035` | fetch, XMLHttpRequest, WebSocket, WebRTC, EventSource to an endpoint, sendBeacon, WebTransport, or service-worker network interception; |
| 3 | `PES-ISO-0036` | WebSerial, WebUSB, WebBluetooth, WebHID, WebNFC, WebMIDI, or later equivalent device APIs; |
| 4 | `PES-ISO-0037` | serial ports, USB, Bluetooth, pcap, native device enumeration, or arbitrary filesystem devices; |
| 5 | `PES-ISO-0038` | child-process execution, shell commands, dynamic library loading, native FFI, dlopen, arbitrary native bridges, or plugins able to reach those capabilities. |

##### PES-SEC-0001 → PES-SEC-0026, PES-SEC-0027, PES-SEC-0028, PES-SEC-0029, PES-SEC-0030, PES-SEC-0031, PES-SEC-0032

Historical parent source text: MAY permit only these host capabilities in the production product:<br>local rendering and user interaction;<br>explicit user-initiated open/save of simulator-native project, archive, CSV, JSON, image, or report files approved by later requirements;<br>controlled application-local persistence;<br>typed UI-to-worker messaging;<br>memory allocation;<br>simulator-controlled monotonic virtual time inputs;<br>printing or local document export only when a later requirement approves it and no external resource is loaded.

| Ordinal | Atomic child ID | Exact governed clause |
|---:|---|---|
| 1 | `PES-SEC-0026` | local rendering and user interaction; |
| 2 | `PES-SEC-0027` | explicit user-initiated open/save of simulator-native project, archive, CSV, JSON, image, or report files approved by later requirements; |
| 3 | `PES-SEC-0028` | controlled application-local persistence; |
| 4 | `PES-SEC-0029` | typed UI-to-worker messaging; |
| 5 | `PES-SEC-0030` | memory allocation; |
| 6 | `PES-SEC-0031` | simulator-controlled monotonic virtual time inputs; |
| 7 | `PES-SEC-0032` | printing or local document export only when a later requirement approves it and no external resource is loaded. |

##### PES-CRM-0008 → PES-CRM-0026, PES-CRM-0027, PES-CRM-0028, PES-CRM-0029, PES-CRM-0030

Historical parent source text: MAY use:<br>public Siemens documentation, SCE material, and public product/support pages as behavioral evidence;<br>IEC descriptions or standards lawfully licensed to the team;<br>public statutes and published judicial opinions;<br>independent textbooks and tutorials for corroboration;<br>independently created observations only under a written, counsel-approved observation protocol.

| Ordinal | Atomic child ID | Exact governed clause |
|---:|---|---|
| 1 | `PES-CRM-0026` | public Siemens documentation, SCE material, and public product/support pages as behavioral evidence; |
| 2 | `PES-CRM-0027` | IEC descriptions or standards lawfully licensed to the team; |
| 3 | `PES-CRM-0028` | public statutes and published judicial opinions; |
| 4 | `PES-CRM-0029` | independent textbooks and tutorials for corroboration; |
| 5 | `PES-CRM-0030` | independently created observations only under a written, counsel-approved observation protocol. |

##### PES-CRM-0009 → PES-CRM-0031, PES-CRM-0032, PES-CRM-0033, PES-CRM-0034, PES-CRM-0035, PES-CRM-0036, PES-CRM-0037

Historical parent source text: MUST NOT use:<br>Siemens source code, leaked code, leaked manuals, partner-only material, or confidential training material;<br>decompiled or disassembled output;<br>executable resources, extracted icons, resource packages, or memory scraping;<br>protocol captures intended to reproduce vendor communications;<br>encrypted project-format cracking;<br>pirated software, license bypass, access-control circumvention, or API hooking;<br>screenshots, manual diagrams, copied tables, copied hardware illustrations, or copied diagnostic text as implementation assets.

| Ordinal | Atomic child ID | Exact governed clause |
|---:|---|---|
| 1 | `PES-CRM-0031` | Siemens source code, leaked code, leaked manuals, partner-only material, or confidential training material; |
| 2 | `PES-CRM-0032` | decompiled or disassembled output; |
| 3 | `PES-CRM-0033` | executable resources, extracted icons, resource packages, or memory scraping; |
| 4 | `PES-CRM-0034` | protocol captures intended to reproduce vendor communications; |
| 5 | `PES-CRM-0035` | encrypted project-format cracking; |
| 6 | `PES-CRM-0036` | pirated software, license bypass, access-control circumvention, or API hooking; |
| 7 | `PES-CRM-0037` | screenshots, manual diagrams, copied tables, copied hardware illustrations, or copied diagnostic text as implementation assets. |

##### PES-CRM-0017 → PES-CRM-0038, PES-CRM-0039, PES-CRM-0040, PES-CRM-0041, PES-CRM-0042, PES-CRM-0043, PES-CRM-0044, PES-CRM-0045, PES-CRM-0046, PES-CRM-0047, PES-CRM-0048, PES-CRM-0049

Historical parent source text: MUST maintain a requirement evidence register with:<br>requirement ID;<br>paraphrased observed behavior;<br>source title, publisher, version/date, durable location, and access date;<br>report classification;<br>IP class and disposition;<br>simulator-owned implementation requirement;<br>forbidden shortcut;<br>author;<br>reviewer;<br>review status and date;<br>implementation component;<br>verification IDs.

| Ordinal | Atomic child ID | Exact governed clause |
|---:|---|---|
| 1 | `PES-CRM-0038` | requirement ID; |
| 2 | `PES-CRM-0039` | paraphrased observed behavior; |
| 3 | `PES-CRM-0040` | source title, publisher, version/date, durable location, and access date; |
| 4 | `PES-CRM-0041` | report classification; |
| 5 | `PES-CRM-0042` | IP class and disposition; |
| 6 | `PES-CRM-0043` | simulator-owned implementation requirement; |
| 7 | `PES-CRM-0044` | forbidden shortcut; |
| 8 | `PES-CRM-0045` | author; |
| 9 | `PES-CRM-0046` | reviewer; |
| 10 | `PES-CRM-0047` | review status and date; |
| 11 | `PES-CRM-0048` | implementation component; |
| 12 | `PES-CRM-0049` | verification IDs. |

##### PES-CRM-0021 → PES-CRM-0050, PES-CRM-0051, PES-CRM-0052, PES-CRM-0053, PES-CRM-0054, PES-CRM-0055, PES-CRM-0056, PES-CRM-0057

Historical parent source text: MUST register every shipped image, icon, font, sound, animation, template, sample project, translation, and other non-code asset with:<br>asset ID;<br>author/source;<br>license and evidence location;<br>created date;<br>hash algorithm and original hash;<br>derivative chain and modifications;<br>generated-asset disclosure where applicable;<br>reviewer, review status, and approval date.

| Ordinal | Atomic child ID | Exact governed clause |
|---:|---|---|
| 1 | `PES-CRM-0050` | asset ID; |
| 2 | `PES-CRM-0051` | author/source; |
| 3 | `PES-CRM-0052` | license and evidence location; |
| 4 | `PES-CRM-0053` | created date; |
| 5 | `PES-CRM-0054` | hash algorithm and original hash; |
| 6 | `PES-CRM-0055` | derivative chain and modifications; |
| 7 | `PES-CRM-0056` | generated-asset disclosure where applicable; |
| 8 | `PES-CRM-0057` | reviewer, review status, and approval date. |

##### PES-ARC-0022 → PES-ARC-0039, PES-ARC-0040, PES-ARC-0041, PES-ARC-0042, PES-ARC-0043, PES-ARC-0044, PES-ARC-0045, PES-ARC-0046, PES-ARC-0047, PES-ARC-0048, PES-ARC-0049, PES-ARC-0050, PES-ARC-0051, PES-ARC-0052, PES-ARC-0053, PES-ARC-0054, PES-ARC-0055, PES-ARC-0056

Historical parent source text: MUST keep these layers distinct:<br>editable offline project source;<br>saved project state;<br>hardware build state;<br>software build state;<br>HMI build state;<br>immutable build artifact;<br>loaded virtual-controller artifact;<br>current virtual runtime values;<br>declared initial/start values;<br>loaded baselines;<br>retained values;<br>raw virtual-process values;<br>CPU-visible values;<br>one-shot modifications;<br>persistent force overlays;<br>runtime snapshots;<br>project/runtime equality or mismatch;<br>monitoring active state.

| Ordinal | Atomic child ID | Exact governed clause |
|---:|---|---|
| 1 | `PES-ARC-0039` | editable offline project source; |
| 2 | `PES-ARC-0040` | saved project state; |
| 3 | `PES-ARC-0041` | hardware build state; |
| 4 | `PES-ARC-0042` | software build state; |
| 5 | `PES-ARC-0043` | HMI build state; |
| 6 | `PES-ARC-0044` | immutable build artifact; |
| 7 | `PES-ARC-0045` | loaded virtual-controller artifact; |
| 8 | `PES-ARC-0046` | current virtual runtime values; |
| 9 | `PES-ARC-0047` | declared initial/start values; |
| 10 | `PES-ARC-0048` | loaded baselines; |
| 11 | `PES-ARC-0049` | retained values; |
| 12 | `PES-ARC-0050` | raw virtual-process values; |
| 13 | `PES-ARC-0051` | CPU-visible values; |
| 14 | `PES-ARC-0052` | one-shot modifications; |
| 15 | `PES-ARC-0053` | persistent force overlays; |
| 16 | `PES-ARC-0054` | runtime snapshots; |
| 17 | `PES-ARC-0055` | project/runtime equality or mismatch; |
| 18 | `PES-ARC-0056` | monitoring active state. |

##### PES-CI-0001 → PES-CI-0004, PES-CI-0005, PES-CI-0006, PES-CI-0007, PES-CI-0008, PES-CI-0009, PES-CI-0010, PES-CI-0011, PES-CI-0012, PES-CI-0013, PES-CI-0014, PES-CI-0015, PES-CI-0016, PES-CI-0017

Historical parent source text: MUST fail production merge or release when:<br>a forbidden dependency or capability is added;<br>a prohibited source API or WASM import appears;<br>a remote asset, CDN, telemetry, analytics, or cloud dependency appears;<br>an asset lacks provenance or approval;<br>a vendor screenshot, logo, icon, device illustration, or copied prose enters production;<br>an unclassified research-derived requirement enters implementation;<br>a required test is skipped or flaky;<br>determinism/replay diverges;<br>migration loses identity or data;<br>a lesson bypasses ordinary domain/diagnostic behavior;<br>Virtual Download accepts any endpoint-like value;<br>HMI uses any transport other than InternalTagBus;<br>an exported artifact resembles or is accepted as a real industrial deployment artifact;<br>traceability between a verified requirement and its tests is missing.

| Ordinal | Atomic child ID | Exact governed clause |
|---:|---|---|
| 1 | `PES-CI-0004` | a forbidden dependency or capability is added; |
| 2 | `PES-CI-0005` | a prohibited source API or WASM import appears; |
| 3 | `PES-CI-0006` | a remote asset, CDN, telemetry, analytics, or cloud dependency appears; |
| 4 | `PES-CI-0007` | an asset lacks provenance or approval; |
| 5 | `PES-CI-0008` | a vendor screenshot, logo, icon, device illustration, or copied prose enters production; |
| 6 | `PES-CI-0009` | an unclassified research-derived requirement enters implementation; |
| 7 | `PES-CI-0010` | a required test is skipped or flaky; |
| 8 | `PES-CI-0011` | determinism/replay diverges; |
| 9 | `PES-CI-0012` | migration loses identity or data; |
| 10 | `PES-CI-0013` | a lesson bypasses ordinary domain/diagnostic behavior; |
| 11 | `PES-CI-0014` | Virtual Download accepts any endpoint-like value; |
| 12 | `PES-CI-0015` | HMI uses any transport other than InternalTagBus; |
| 13 | `PES-CI-0016` | an exported artifact resembles or is accepted as a real industrial deployment artifact; |
| 14 | `PES-CI-0017` | traceability between a verified requirement and its tests is missing. |

##### PES-REQ-0003 → PES-REQ-0013, PES-REQ-0014, PES-REQ-0015, PES-REQ-0016, PES-REQ-0017, PES-REQ-0018, PES-REQ-0019

Historical parent source text: MUST identify supporting records separately:<br>Record \| Identifier<br>Source/evidence \| SRC-NNNN<br>Architecture decision \| ADR-NNNN<br>Product decision \| DEC-NNNN<br>Open question \| OQ-NNNN<br>Risk \| RSK-NNNN<br>Change record \| CR-NNNN<br>Verification case \| VER-AREA-NNNN

| Ordinal | Atomic child ID | Exact governed clause |
|---:|---|---|
| 1 | `PES-REQ-0013` | Source/evidence \| SRC-NNNN |
| 2 | `PES-REQ-0014` | Architecture decision \| ADR-NNNN |
| 3 | `PES-REQ-0015` | Product decision \| DEC-NNNN |
| 4 | `PES-REQ-0016` | Open question \| OQ-NNNN |
| 5 | `PES-REQ-0017` | Risk \| RSK-NNNN |
| 6 | `PES-REQ-0018` | Change record \| CR-NNNN |
| 7 | `PES-REQ-0019` | Verification case \| VER-AREA-NNNN |

##### PES-DEC-0001 → PES-DEC-0007, PES-DEC-0008, PES-DEC-0009, PES-DEC-0010, PES-DEC-0011, PES-DEC-0012, PES-DEC-0013, PES-DEC-0014

Historical parent source text: MAY let Codex decide an implementation detail without asking only when every plausible choice:<br>is internal and reversible;<br>preserves observable semantics and file compatibility;<br>adds no network, device, native, process, plugin, cloud, AI, or credential capability;<br>does not affect IP classification, branding, public claims, grading, teacher/student separation, privacy, or safety;<br>stays within approved technology and dependency policy;<br>can be objectively verified;<br>satisfies all higher requirements.<br>Meaningful autonomous decisions shall still be recorded in an ADR or implementation note.

| Ordinal | Atomic child ID | Exact governed clause |
|---:|---|---|
| 1 | `PES-DEC-0007` | is internal and reversible; |
| 2 | `PES-DEC-0008` | preserves observable semantics and file compatibility; |
| 3 | `PES-DEC-0009` | adds no network, device, native, process, plugin, cloud, AI, or credential capability; |
| 4 | `PES-DEC-0010` | does not affect IP classification, branding, public claims, grading, teacher/student separation, privacy, or safety; |
| 5 | `PES-DEC-0011` | stays within approved technology and dependency policy; |
| 6 | `PES-DEC-0012` | can be objectively verified; |
| 7 | `PES-DEC-0013` | satisfies all higher requirements. |
| 8 | `PES-DEC-0014` | Meaningful autonomous decisions shall still be recorded in an ADR or implementation note. |

##### PES-DEC-0002 → PES-DEC-0015, PES-DEC-0016, PES-DEC-0017, PES-DEC-0018, PES-DEC-0019, PES-DEC-0020, PES-DEC-0021, PES-DEC-0022, PES-DEC-0023, PES-DEC-0024, PES-DEC-0025, PES-DEC-0026

Historical parent source text: MUST stop the affected work and ask Scott when a choice:<br>touches physical/network capability or could weaken the VirtualUniverse wall;<br>uses or resembles vendor assets, protocols, APIs, formats, names, model numbers, diagnostics, branding, or trade dress;<br>changes public workflow semantics, TrainingProfile behavior, file format, migration, grading, Teacher Mode visibility, or student data handling;<br>risks data loss, irreversible schema change, or backward incompatibility;<br>requires cloud, credentials, telemetry, remote services, external AI, or an updater;<br>adds eval, arbitrary scripting, FFI, child process, shell, native bridge, host device, generic transport, or executable plugin capability;<br>is marked NEEDS MORE RESEARCH, Class 7, Class 8, or professional legal review;<br>makes a safety, certification, compatibility, equivalence, endorsement, or production claim;<br>conflicts with higher authority;<br>cannot be verified objectively;<br>would choose initial operating systems or the production packaging model;<br>would expand scope beyond the authored phases.

| Ordinal | Atomic child ID | Exact governed clause |
|---:|---|---|
| 1 | `PES-DEC-0015` | touches physical/network capability or could weaken the VirtualUniverse wall; |
| 2 | `PES-DEC-0016` | uses or resembles vendor assets, protocols, APIs, formats, names, model numbers, diagnostics, branding, or trade dress; |
| 3 | `PES-DEC-0017` | changes public workflow semantics, TrainingProfile behavior, file format, migration, grading, Teacher Mode visibility, or student data handling; |
| 4 | `PES-DEC-0018` | risks data loss, irreversible schema change, or backward incompatibility; |
| 5 | `PES-DEC-0019` | requires cloud, credentials, telemetry, remote services, external AI, or an updater; |
| 6 | `PES-DEC-0020` | adds eval, arbitrary scripting, FFI, child process, shell, native bridge, host device, generic transport, or executable plugin capability; |
| 7 | `PES-DEC-0021` | is marked NEEDS MORE RESEARCH, Class 7, Class 8, or professional legal review; |
| 8 | `PES-DEC-0022` | makes a safety, certification, compatibility, equivalence, endorsement, or production claim; |
| 9 | `PES-DEC-0023` | conflicts with higher authority; |
| 10 | `PES-DEC-0024` | cannot be verified objectively; |
| 11 | `PES-DEC-0025` | would choose initial operating systems or the production packaging model; |
| 12 | `PES-DEC-0026` | would expand scope beyond the authored phases. |

##### PES-QLT-0002 → PES-QLT-0009, PES-QLT-0010, PES-QLT-0011, PES-QLT-0012, PES-QLT-0013, PES-QLT-0014, PES-QLT-0015, PES-QLT-0016, PES-QLT-0017, PES-QLT-0018, PES-QLT-0019, PES-QLT-0020, PES-QLT-0021, PES-QLT-0022

Historical parent source text: MUST NOT ship:<br>no-op commands;<br>hard-coded success or catch-and-return-success;<br>fake compile, load, online, scan, force, HMI, or diagnostic animations;<br>canned errors or predetermined lesson results;<br>sample-specific PLC/process logic in general engines;<br>offline values displayed as monitored runtime values;<br>HMI animation disconnected from InternalTagBus;<br>mock/test doubles reachable in production;<br>a regex-only compiler;<br>SCL eval;<br>LAD/FBD execution based on screen coordinates;<br>hidden physical adapters disabled by configuration;<br>scenario code that directly awards a pass;<br>a "coming soon" control that appears operational.

| Ordinal | Atomic child ID | Exact governed clause |
|---:|---|---|
| 1 | `PES-QLT-0009` | no-op commands; |
| 2 | `PES-QLT-0010` | hard-coded success or catch-and-return-success; |
| 3 | `PES-QLT-0011` | fake compile, load, online, scan, force, HMI, or diagnostic animations; |
| 4 | `PES-QLT-0012` | canned errors or predetermined lesson results; |
| 5 | `PES-QLT-0013` | sample-specific PLC/process logic in general engines; |
| 6 | `PES-QLT-0014` | offline values displayed as monitored runtime values; |
| 7 | `PES-QLT-0015` | HMI animation disconnected from InternalTagBus; |
| 8 | `PES-QLT-0016` | mock/test doubles reachable in production; |
| 9 | `PES-QLT-0017` | a regex-only compiler; |
| 10 | `PES-QLT-0018` | SCL eval; |
| 11 | `PES-QLT-0019` | LAD/FBD execution based on screen coordinates; |
| 12 | `PES-QLT-0020` | hidden physical adapters disabled by configuration; |
| 13 | `PES-QLT-0021` | scenario code that directly awards a pass; |
| 14 | `PES-QLT-0022` | a "coming soon" control that appears operational. |

##### PES-QLT-0004 → PES-QLT-0023, PES-QLT-0024, PES-QLT-0025, PES-QLT-0026, PES-QLT-0027, PES-QLT-0028

Historical parent source text: MAY create scaffolding only when it:<br>is not user-visible or release-reachable;<br>contains no forbidden capability;<br>fails closed;<br>is labeled SCAFFOLDED in the implementation matrix;<br>has an owner and removal/completion target;<br>earns zero completion credit.

| Ordinal | Atomic child ID | Exact governed clause |
|---:|---|---|
| 1 | `PES-QLT-0023` | is not user-visible or release-reachable; |
| 2 | `PES-QLT-0024` | contains no forbidden capability; |
| 3 | `PES-QLT-0025` | fails closed; |
| 4 | `PES-QLT-0026` | is labeled SCAFFOLDED in the implementation matrix; |
| 5 | `PES-QLT-0027` | has an owner and removal/completion target; |
| 6 | `PES-QLT-0028` | earns zero completion credit. |

##### PES-QLT-0006 → PES-QLT-0029, PES-QLT-0030, PES-QLT-0031, PES-QLT-0032, PES-QLT-0033, PES-QLT-0034, PES-QLT-0035, PES-QLT-0036, PES-QLT-0037, PES-QLT-0038, PES-QLT-0039, PES-QLT-0040, PES-QLT-0041, PES-QLT-0042, PES-QLT-0043, PES-QLT-0044

Historical parent source text: MUST require every software milestone, when later authorized, to include:<br>domain model and ownership;<br>invariants;<br>positive behavior;<br>negative behavior;<br>enumerated failure cases and recovery;<br>stable identity and dependency behavior;<br>persistence, migration, and undo where applicable;<br>real UI integration where applicable;<br>end-to-end workflow;<br>deterministic unit/integration tests;<br>property/fuzz/golden tests where applicable;<br>isolation/security tests;<br>clean-room evidence and asset provenance;<br>documentation;<br>requirement-to-test traceability;<br>reproducible verification evidence.

| Ordinal | Atomic child ID | Exact governed clause |
|---:|---|---|
| 1 | `PES-QLT-0029` | domain model and ownership; |
| 2 | `PES-QLT-0030` | invariants; |
| 3 | `PES-QLT-0031` | positive behavior; |
| 4 | `PES-QLT-0032` | negative behavior; |
| 5 | `PES-QLT-0033` | enumerated failure cases and recovery; |
| 6 | `PES-QLT-0034` | stable identity and dependency behavior; |
| 7 | `PES-QLT-0035` | persistence, migration, and undo where applicable; |
| 8 | `PES-QLT-0036` | real UI integration where applicable; |
| 9 | `PES-QLT-0037` | end-to-end workflow; |
| 10 | `PES-QLT-0038` | deterministic unit/integration tests; |
| 11 | `PES-QLT-0039` | property/fuzz/golden tests where applicable; |
| 12 | `PES-QLT-0040` | isolation/security tests; |
| 13 | `PES-QLT-0041` | clean-room evidence and asset provenance; |
| 14 | `PES-QLT-0042` | documentation; |
| 15 | `PES-QLT-0043` | requirement-to-test traceability; |
| 16 | `PES-QLT-0044` | reproducible verification evidence. |

##### PES-GOV-0017 → PES-GOV-0032, PES-GOV-0033, PES-GOV-0034, PES-GOV-0035, PES-GOV-0036, PES-GOV-0037, PES-GOV-0038, PES-GOV-0039

Historical parent source text: MUST append and revise this same document for Phases 2-4 while preserving:<br>exact filename;<br>style system;<br>requirement IDs;<br>cross references;<br>source hash history;<br>change ledger;<br>superseded requirement tombstones;<br>open decisions and risk records.

| Ordinal | Atomic child ID | Exact governed clause |
|---:|---|---|
| 1 | `PES-GOV-0032` | exact filename; |
| 2 | `PES-GOV-0033` | style system; |
| 3 | `PES-GOV-0034` | requirement IDs; |
| 4 | `PES-GOV-0035` | cross references; |
| 5 | `PES-GOV-0036` | source hash history; |
| 6 | `PES-GOV-0037` | change ledger; |
| 7 | `PES-GOV-0038` | superseded requirement tombstones; |
| 8 | `PES-GOV-0039` | open decisions and risk records. |

<!-- COMPOUND_SPLIT_LEDGER_END -->

## Task 4 — Baseline and hash integrity

### 4.1 Independent source hashes

The following is raw PowerShell output from the live filesystem; neither a project verifier nor a generated register produced these values.

```powershell
Get-FileHash -Algorithm SHA256 -LiteralPath 'Govs PLC project Research Report.md','PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx' | Format-List Algorithm,Hash,Path
```

```text

Algorithm : SHA256
Hash      : F05C08323B5CC9483BEB1FEB3C7312CCB9A45EBE3B527E6DAE069C181D3FBF55
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\Govs PLC project Research Report.md

Algorithm : SHA256
Hash      : EBF074E2CEAB752F09E6DB63D88E100991729DA13C1EB874290A6B337DA72612
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\PLC Engineering Simulator - Codex Master Implementation 
            Directive Phase 1.docx
```

### 4.2 All 41 declared controlled-file hashes

The population below is the 2 `sourceFiles` paths plus the 39 `requiredFiles` paths currently declared by `tests/phase1/policy-contract.json`. That repository-authored contract defines the set and therefore cannot prove that the set itself is complete; this command independently hashes every member it names.

```powershell
$contract = Get-Content -LiteralPath 'tests\\phase1\\policy-contract.json' -Raw | ConvertFrom-Json
$controlledPaths = @($contract.sourceFiles.path) + @($contract.requiredFiles)
Get-FileHash -Algorithm SHA256 -LiteralPath $controlledPaths | Sort-Object Path | Format-List Algorithm,Hash,Path
```

```text

Algorithm : SHA256
Hash      : 8E608DB783A129601CE441D08BDDE9A05DFF1811F9E5C011681C9A2EEF9E70A5
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\.editorconfig

Algorithm : SHA256
Hash      : D51465FDCEDA1AC0EBADAF1ABCA6F5A362F55991C3A29A0269CFFDACD0C230FF
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\.gitattributes

Algorithm : SHA256
Hash      : DEA8E72C5682CAF67BA0A6395503C38394BD0AB8F635AE29F5F57E6AAE5FF158
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\.github\workflows\phase1-governance.yml

Algorithm : SHA256
Hash      : 12031969A664C6EE63537E39FEAD9087C5F62EC9E959ADB33378F0384DAC5B7E
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\.gitignore

Algorithm : SHA256
Hash      : 40411DC2C2726B4C7024D38318DD8DF45C528B49491EFB45939DD4A829847D94
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\.python-version

Algorithm : SHA256
Hash      : 78A052EC3776CB8049DC1566F105C473F95FCC55C35DD2F93D0838F8D9ACA34B
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\ADR\0001-no-physical-industrial-communication.md

Algorithm : SHA256
Hash      : 098247216F077BC4B6A42A16FAA212643625654E87B11EB5C8591E941C966E4D
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\ADR\0002-original-project-format.md

Algorithm : SHA256
Hash      : 7AA4628D3E83DB346E77548A720C06F80E7CDF5AE2D2397EA5861B51FAA2AD70
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\ADR\0003-unified-plc-ir.md

Algorithm : SHA256
Hash      : 6ECD2475A92D113BCAECA128BE7F70550596E7C3DEEDF5F63051E2CE2075BB60
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\ADR\0004-deterministic-virtual-time.md

Algorithm : SHA256
Hash      : 885F33AE6347F9F478D656C9BF3BBAD3401F13F36B783BF4BCF668F731624AFE
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\ASSET_PROVENANCE.json

Algorithm : SHA256
Hash      : CAB3C319A062072CD280DF54FF4786200BF6CFE548A2C8674C0289DFA35AE214
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\Cargo.lock

Algorithm : SHA256
Hash      : 2E81188A62E6F5A6CDF693697A528730ED825BE15E186957CF45F3AD96A18D9A
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\Cargo.toml

Algorithm : SHA256
Hash      : 9E04FE0DDED3C3CC23373F8A0F384829326998E25D0577A1D53E66A537E54FB2
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\CHANGELOG_DIRECTIVE.md

Algorithm : SHA256
Hash      : A30305A3A52D68FDECF5077BF1FF479A54E33484C42D43D138F68EF72C213693
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\CLEAN_ROOM_POLICY.md

Algorithm : SHA256
Hash      : 20B2CCDFC392FDFAE03E3466CDAEC7777BBDD5FEDF5905BF3599995CDECD505B
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\CONTRIBUTOR_CLEAN_ROOM_ATTESTATION.md

Algorithm : SHA256
Hash      : 6071A19D9D84B53A09F410F2924853723807ED8D63BB7DC841C7590FC19463C9
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\DEPENDENCY_POLICY.md

Algorithm : SHA256
Hash      : AC9772982347BBAC2F1D5886C366B05B145C7D82A279B2DBE2CCAF64B2C73EB1
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\docs\governance\DOCX_VISUAL_QA.md

Algorithm : SHA256
Hash      : 368E90E7C707668F411C373C5193FDCBFF4513529EA853111DD362DA2BCEBA9D
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\docs\governance\PHASE_1_SCOPE_AUDIT.md

Algorithm : SHA256
Hash      : 904703338536C9E98674044E9F898AEBA64A7C88C1136A5B6FCB1102C2B9396D
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\docs\governance\PHASE_1_VERIFICATION_PLAN.md

Algorithm : SHA256
Hash      : 9D04271BEABEF158B723D7A93561B30A73F096BC55B763A04ECE43452D7A61D0
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\docs\governance\TOOLCHAIN_ADMISSION_REGISTER.md

Algorithm : SHA256
Hash      : F13CCD241714BD144D1F373D7F90769CC957751BF1F10B7494ABF765A3314D5B
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\docs\research\UNRESOLVED_SOURCE_TOKENS.md

Algorithm : SHA256
Hash      : E15078B534260E3733E6842710E8440E32BBF769B521C86D0D17A6C49B75F5F9
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\EVIDENCE_REGISTER.json

Algorithm : SHA256
Hash      : F05C08323B5CC9483BEB1FEB3C7312CCB9A45EBE3B527E6DAE069C181D3FBF55
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\Govs PLC project Research Report.md

Algorithm : SHA256
Hash      : 40164335374065AA40572940924BC019D4F3F0ACCF3F4ADE1ACF6AEB6FBD41DC
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\IMPLEMENTATION_MATRIX.json

Algorithm : SHA256
Hash      : 0F6BEF7EF42A5286DE3B5CF06D98B5D0899D625DB4E29F14B2DEC6CF04F93B8A
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\LEGAL_REVIEW_CHECKLIST.md

Algorithm : SHA256
Hash      : D73B32EFCCE6E1BE28FD6130FA8D32D011DEC352D4F52193E85267763F51B1DE
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\OPEN_DECISIONS.md

Algorithm : SHA256
Hash      : 38341E2C218B30D2A6C9ABA08CA3E5823AFDA277132BBFC261E3A2A401D4BF1A
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\package.json

Algorithm : SHA256
Hash      : EBF074E2CEAB752F09E6DB63D88E100991729DA13C1EB874290A6B337DA72612
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\PLC Engineering Simulator - Codex Master Implementation 
            Directive Phase 1.docx

Algorithm : SHA256
Hash      : 17C814B167307942D3609C7B9D916CEDDB85839573AB39BAA114E30EDB132A1A
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\pnpm-lock.yaml

Algorithm : SHA256
Hash      : 412E3F1C1A5209EDC108815ED37CDF8364E8BBDC4A568D8E63B3C44548376609
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\pnpm-workspace.yaml

Algorithm : SHA256
Hash      : CEE4BD51B4A8A3E11CCF026F0B919EA14276F55F1537B0FC7B461E2D65CEF9F4
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\README.md

Algorithm : SHA256
Hash      : 876725C7BC22BE372EE97905EE00578A1C34AF0A48705F42FA6C4FA9DF07F469
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\REQUIREMENTS.md

Algorithm : SHA256
Hash      : 83A892ABB0151580689D5AC111CA2529E2F3C330BB8DFC9414D404C59CA5B32B
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\requirements\phase1-requirements.json

Algorithm : SHA256
Hash      : 944881F0F0905A6D24D0FD8A0BAB583232D4DCDC3C14D3E1AE8F794E978CE5BA
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\RISK_REGISTER.md

Algorithm : SHA256
Hash      : DD510B4A3AB011309BB12F6FC3450596DA2080670BD4F922EAFF7CAD1CA65794
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\rust-toolchain.toml

Algorithm : SHA256
Hash      : 056FFE579DA7483F049A5FDC5FFDAC02665AB298E4877A9F6BE6BB6222109F9F
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\SECURITY_INVARIANTS.md

Algorithm : SHA256
Hash      : 0D96706B62EB446FAB0B35598E252CB84FBF87F43EA57FECF28E8A9080A0F013
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\tests\phase1\policy-contract.json

Algorithm : SHA256
Hash      : 6455C5505EF2CD8D6C0D5EDB55181C628D36881686E64ECF1417E9C5546BD064
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\THREAT_MODEL.md

Algorithm : SHA256
Hash      : 299F052AB6E6249FA7019A5209B18403C6422A81627503417F06BEF2F26B9618
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\tools\phase1\extract_directive_requirements.py

Algorithm : SHA256
Hash      : 7A0D383AB95DC005B03ABA98F5FFAD4DB878236CB93699434921749F28D713C7
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\tools\phase1\run_phase1_verification.py

Algorithm : SHA256
Hash      : 958D987BAC94BAE7F197CC8FCB658E9CA6E4511EA53EEB105618B8105D7877CC
Path      : C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\tools\phase1\verify-phase1.mjs

CONTROLLED_COUNT=41
HASH_RESULT_COUNT=41
```

### 4.3 Comparison with recorded artifact hashes

The comparison below audits `.phase1-verification/phase1-report.json`; the report is an inadmissible audit target and is not treated as proof. It shows that its 41 recorded values agree with the independent live hashes and that neither representation has an extra path relative to the other. Because both the population contract and recorded set were authored inside this repository, zero mismatches establishes current agreement only—not independent baseline completeness or immutability.

```text
ACTUAL_COUNT=41
RECORDED_COUNT=41
MISMATCH_COUNT=0
MISSING_RECORDED_COUNT=0
EXTRA_RECORDED_COUNT=0
CONTRACT_DUPLICATE_COUNT=0
```

At the pre-audit measurement point, no file declared controlled by the policy contract was missing from the recorded hash set. The newly required `docs/governance/PHASE_1_ADVERSARIAL_AUDIT.md` is intentionally outside that old 41-file set; it did not exist at the baseline measurement and was added solely by this audit directive.

### 4.4 DEC-0001 filename discrepancy

The fresh source render says on page 1: `Research baseline Govs PLC project.md | SHA-256 f05c0832...d3fbf55`. Page 3 says: `Final filename PLC Engineering Simulator - Codex Master Implementation Directive.docx` and `Research authority Govs PLC project.md, frozen at the hash above`. Raw disk and exact-search facts are:

```text

Name                                                                           Length
----                                                                           ------
CHANGELOG_DIRECTIVE.md                                                           7370
Govs PLC project Research Report.md                                            127132
PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx  81045

DISK_HAS_DIRECTIVE_RESEARCH_NAME=False
DISK_HAS_DIRECTIVE_FINAL_NAME=False
DISK_HAS_ACTUAL_RESEARCH_NAME=True
DISK_HAS_ACTUAL_DIRECTIVE_NAME=True
RG_EXIT_GovsPLCprojectMD=1
RG_EXIT_ActualResearchFilename=1
RG_EXIT_MasterDirectiveFilename=1
```

| Fact | Result | Evidence anchor |
|---|---|---|
| Actual research filename on disk | `Govs PLC project Research Report.md` | Raw `Get-ChildItem` output above; current SHA-256 `F05C08323B5CC9483BEB1FEB3C7312CCB9A45EBE3B527E6DAE069C181D3FBF55`. |
| Research filename named by directive | `Govs PLC project.md` | Supplied directive p.1 cover and p.3 Document Control. No such disk path exists. |
| Research report's own filename reference | None | The three exact `rg` searches above return exit 1; the frozen report names neither candidate nor the master-directive filename. |
| Actual directive filename on disk | `PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx` | Raw `Get-ChildItem` output above; current SHA-256 `EBF074E2CEAB752F09E6DB63D88E100991729DA13C1EB874290A6B337DA72612`. |
| Final directive filename named by directive | `PLC Engineering Simulator - Codex Master Implementation Directive.docx` | Supplied directive p.3 Document Control. No such disk path exists. |

For this audit, the **actual user-supplied paths and their current bytes are the input authority**, because the user explicitly supplied those two paths and the actual files are the only extant candidates. The directive remains the authority for product semantics, but its exact-filename fields conflict with the supplied filenames and do not authorize a silent rename. `DEC-0001` therefore remains unresolved. The research file's current hash matches the 60-bit abbreviated prefix/suffix printed by the directive, but no admissible external full-hash receipt exists for either original transfer; strict byte-identity to the moment of supply is listed under **Not verifiable**.

## Task 5 — Pre-remediation check independence

### Executive result

The report contains 163 passing **check instances**, but only 10 distinct verification IDs. On an adversarial, mutually exclusive classification:

| Class | Meaning | Count | Share (rounded) |
|---|---|---:|---:|
| A | Derived from a specific numbered directive requirement | 64 | 39.3% |
| B | Structural/integrity/parse/shape check chosen by this implementation | 36 | 22.1% |
| C | Tautological or circular check of one repository-authored constant/representation against another | 63 | 38.7% |
| **Total** |  | **163** | **100%** |

This classification does **not** treat the report's `passed: true` fields, the verifier's descriptions, or the policy contract as proof of correctness. A class-A label means the predicate has a defensible numbered directive anchor; it does not mean the predicate is sufficient acceptance evidence. Many A checks are only existence, absence, regex, or keyword-presence guardrails.

The most material conclusions are:

1. The `PASS` result primarily demonstrates repository self-consistency. Sixty-three instances compare duplicated repository choices, and thirty-six validate implementation-selected structure.
2. The supposedly “independent” foundation-mapping allowlist is not independent evidence: the extractor constants and policy-contract constants were authored together and are compared to each other.
3. The register contains 247 source IDs, not 247 atomic requirements. Twenty records are explicitly `COMPOUND_SOURCE_REQUIRES_REVIEW`; the required split under `PES-REQ-0005` has not occurred.
4. Current truth states, milestones, dependencies, acceptance criteria, IP dispositions, component mappings, and verification mappings are substantial Codex-authored enrichment, not literal DOCX extraction.
5. The repository correctly discloses that Word/Poppler/PDF-Python output is an unapproved observation, that ignored render evidence can be absent on a clean checkout, and that Phase 1 exit, reviewer acceptance, tool admission, and Phase 2–4 product work remain incomplete.

### Task 5 — complete 163-instance classification

#### Classification rule

- **A — Derived:** the checked property is traceable to at least one specific `PES-*` requirement. The table cites the most direct ID(s) and the rendered directive page/section.
- **B — Structural:** the check validates a filename, schema, parser boundary, hash binding, local evidence shape, or repository convention selected by the implementation. It may support the directive, but the exact predicate is not a numbered requirement.
- **C — Tautological:** the expected value and actual representation are both controlled by the repository implementation (often duplicated between the extractor, verifier, policy contract, plan, register, matrix, or workflow). Such a check can catch accidental drift but cannot independently prove the chosen value is correct.

#### Full table

`v:` line references are to `tools/phase1/verify-phase1.mjs`.

| # | Verification ID | Instance under test | Class | Directive anchor or adversarial basis | Verifier |
|---:|---|---|:---:|---|---:|
| 001 | VER-CI-0001 | Node runtime equals 24.19.0 | B | Exact runtime version is an implementation/toolchain selection, not a numbered directive value. | v:127-131 |
| 002 | VER-REQ-0001 | Fresh extractor output equals both committed snapshots | B | Determinism/freshness guardrail; it reruns the same enrichment code and does not validate that its interpretations are correct. | v:132-143 |
| 003 | VER-GOV-0001 | Research report exists | A | `PES-GOV-0010`, p.5 §1.3, requires the identified research baseline. | v:145-158 |
| 004 | VER-GOV-0001 | Research report hash matches | A | `PES-GOV-0010`, p.5 §1.3, expressly binds filename and SHA-256. | v:145-158 |
| 005 | VER-GOV-0001 | Supplied directive exists | A | `PES-GOV-0001`, p.4 §1.1, makes the living directive controlling authority. | v:145-158 |
| 006 | VER-GOV-0001 | Supplied directive hash matches local contract | B | The directive does not contain its own current SHA-256 requirement; the hash is a repository-selected snapshot binding. | v:145-158 |
| 007 | VER-DOC-0001 | `.editorconfig` exists/non-empty | B | Conventional repository configuration selected locally. | v:160-164 |
| 008 | VER-DOC-0001 | `.gitattributes` exists/non-empty | B | Conventional repository configuration selected locally. | v:160-164 |
| 009 | VER-DOC-0001 | `.gitignore` exists/non-empty | B | Conventional repository configuration selected locally. | v:160-164 |
| 010 | VER-DOC-0001 | Proposed workflow exists/non-empty | B | Exact workflow file is an implementation choice; numbered CI requirements specify behavior, not this file. | v:160-164 |
| 011 | VER-DOC-0001 | `.python-version` exists/non-empty | B | Exact pin file and version are implementation choices. | v:160-164 |
| 012 | VER-DOC-0001 | README exists/non-empty | B | README is useful governance, but no numbered requirement mandates this exact artifact. | v:160-164 |
| 013 | VER-DOC-0001 | Clean-room policy exists/non-empty | A | `PES-CRM-0016`, p.17 §6.5, expressly requires `CLEAN_ROOM_POLICY.md`. | v:160-164 |
| 014 | VER-DOC-0001 | Security invariants exist/non-empty | A | `PES-SEC-0017`, p.20 §7.1, expressly requires `SECURITY_INVARIANTS.md`. | v:160-164 |
| 015 | VER-DOC-0001 | Legal checklist exists/non-empty | B | Named in unnumbered §9.2 repository prose, but existence alone has no specific `PES-*` requirement anchor. | v:160-164 |
| 016 | VER-DOC-0001 | Contributor attestation exists/non-empty | A | `PES-CRM-0020`, p.18 §6.5, requires contributor attestation. | v:160-164 |
| 017 | VER-DOC-0001 | Threat model exists/non-empty | A | `PES-SEC-0025`, p.20 §7.3, requires threat-model updates for security-relevant changes; §9.2 also names the file. | v:160-164 |
| 018 | VER-DOC-0001 | Requirements document exists/non-empty | A | `PES-REQ-0001`–`0009`, pp.29-31 §10, require the identifier/record/truth-state system. | v:160-164 |
| 019 | VER-DOC-0001 | Implementation matrix exists/non-empty | A | `PES-REQ-0006`–`0009`, pp.30-31 §10.2-10.3, require mapping and completion truth. | v:160-164 |
| 020 | VER-DOC-0001 | Evidence register exists/non-empty | A | `PES-GOV-0013`, p.5 §1.3, and `PES-CRM-0017`, p.17 §6.5. | v:160-164 |
| 021 | VER-DOC-0001 | Asset provenance register exists/non-empty | A | `PES-CRM-0021`, p.18 §6.6, requires asset registration fields. | v:160-164 |
| 022 | VER-DOC-0001 | Dependency policy exists/non-empty | A | `PES-CRM-0024`–`0025`, p.18 §6.6, plus `PES-DEV-0006`, p.27 §9.1. | v:160-164 |
| 023 | VER-DOC-0001 | Open-decision register exists/non-empty | A | `PES-GOV-0006`, p.5 §1.2, and `PES-DEC-0004`–`0006`, pp.32-33 §11.3. | v:160-164 |
| 024 | VER-DOC-0001 | Risk register exists/non-empty | A | `PES-GOV-0017`, p.35 §13.1, requires preservation of risk records; §9.2 names the register. | v:160-164 |
| 025 | VER-DOC-0001 | Directive changelog exists/non-empty | A | `PES-GOV-0014`–`0016`, p.31 §10.4, require controlled change records. | v:160-164 |
| 026 | VER-DOC-0001 | Cargo lockfile exists/non-empty | A | `PES-DEV-0006`, p.27 §9.1, recommends an exact-version Cargo workspace. | v:160-164 |
| 027 | VER-DOC-0001 | Cargo manifest exists/non-empty | A | `PES-DEV-0006`, p.27 §9.1. | v:160-164 |
| 028 | VER-DOC-0001 | ADR-0001 exists/non-empty | A | `PES-DOC-0001`–`0002`, p.27 §9.2. | v:160-164 |
| 029 | VER-DOC-0001 | Original-format ADR exists/non-empty | A | `PES-DOC-0003`, p.28 §9.2. | v:160-164 |
| 030 | VER-DOC-0001 | Unified-IR ADR exists/non-empty | A | `PES-DOC-0003`, p.28 §9.2. | v:160-164 |
| 031 | VER-DOC-0001 | Deterministic-time ADR exists/non-empty | A | `PES-DOC-0003`, p.28 §9.2. | v:160-164 |
| 032 | VER-DOC-0001 | Machine-readable requirement register exists | A | `PES-REQ-0001`–`0009`, pp.29-31 §10. | v:160-164 |
| 033 | VER-DOC-0001 | Unresolved-source-token inventory exists | A | `PES-GOV-0013`, p.5 §1.3, requires unresolved sources to remain unresolved rather than invented. | v:160-164 |
| 034 | VER-DOC-0001 | Verification plan exists/non-empty | A | `PES-REQ-0006`–`0007`, p.30 §10.2, require requirement↔verification mapping. | v:160-164 |
| 035 | VER-DOC-0001 | Scope audit exists/non-empty | B | Implementation-authored audit artifact; no numbered requirement mandates this exact file. | v:160-164 |
| 036 | VER-DOC-0001 | DOCX visual-observation record exists | B | QA method/record chosen by implementation; it is explicitly non-gating. | v:160-164 |
| 037 | VER-DOC-0001 | Toolchain register exists/non-empty | B | Admission-register format and filename are implementation-selected operational controls. | v:160-164 |
| 038 | VER-DOC-0001 | `package.json` exists/non-empty | A | `PES-DEV-0006`, p.27 §9.1, recommends pnpm workspace/version locking. | v:160-164 |
| 039 | VER-DOC-0001 | pnpm lockfile exists/non-empty | A | `PES-DEV-0006`, p.27 §9.1. | v:160-164 |
| 040 | VER-DOC-0001 | pnpm workspace exists/non-empty | A | `PES-DEV-0006`, p.27 §9.1. | v:160-164 |
| 041 | VER-DOC-0001 | Rust toolchain pin exists/non-empty | A | `PES-DEV-0006`, p.27 §9.1, requires exact versions/reproducibility for the Rust workspace. | v:160-164 |
| 042 | VER-DOC-0001 | Policy contract exists/non-empty | B | Repository-authored test oracle; not a directive artifact. | v:160-164 |
| 043 | VER-DOC-0001 | Extractor exists/non-empty | B | Implementation mechanism selected locally. | v:160-164 |
| 044 | VER-DOC-0001 | Runtime launcher exists/non-empty | B | Implementation mechanism selected locally. | v:160-164 |
| 045 | VER-DOC-0001 | Verifier exists/non-empty | B | Implementation mechanism selected locally. | v:160-164 |
| 046 | VER-DOC-0002 | ADR-0001 exact title | A | Exact text required by `PES-DOC-0001`, p.27 §9.2. | v:169-173 |
| 047 | VER-DOC-0002 | ADR-0001 exact status | A | Exact text required by `PES-DOC-0001`, p.27 §9.2. | v:174-178 |
| 048 | VER-DOC-0002 | Separate authorized repository clause | A | `PES-DOC-0002`, p.27 §9.2. | v:179-183 |
| 049 | VER-DOC-0002 | Isolation applicability/host-adapter language | A | `PES-ISO-0011`, p.14 §5.7, and `PES-ISO-0015`, p.15 §5.7. | v:184-188 |
| 050 | VER-DOC-0001 | Original-format ADR boundary/proposed status | A | `PES-DOC-0003`, p.28 §9.2, with `PES-PRJ-0001`–`0007`, p.18 §6.7. | v:191-213 |
| 051 | VER-DOC-0001 | Unified-IR ADR boundary/proposed status | A | `PES-DOC-0003`, p.28 §9.2, and `PES-IR-0001`, p.23 §8.5. | v:191-213 |
| 052 | VER-DOC-0001 | Deterministic-time ADR boundary/proposed status | A | `PES-DOC-0003`, p.28 §9.2, and `PES-DET-0001`, p.24 §8.6. | v:191-213 |
| 053 | VER-DOC-0001 | Visual record contains selected phrases/source hash | B | Keyword/record-shape check for an implementation-created observation, not admissible gate evidence. | v:215-235 |
| 054 | VER-DOC-0001 | Ignored PDF matches contract hash | B | Local evidence-integrity check; does not establish an approved renderer or visual acceptance. | v:237-290 |
| 055 | VER-DOC-0001 | Ignored analysis JSON matches contract hash | B | Local evidence-integrity check. | v:237-290 |
| 056 | VER-DOC-0001 | Ignored page set contains 40 PNGs | B | Implementation-selected rendering/count check. | v:237-290 |
| 057 | VER-DOC-0001 | Page-image manifest matches contract hash | B | Implementation-selected evidence-integrity check. | v:237-290 |
| 058 | VER-ISO-0001 | Security-invariant keywords present | A | `PES-SEC-0017`, p.20 §7.1; vendor-observation stop also traces to `PES-CRM-0010`, p.16 §6.3. | v:293-306 |
| 059 | VER-ISO-0001 | Threat-model keywords present | A | `PES-SEC-0017`, p.20 §7.1; `PES-ISO-0015`, p.15 §5.7; `PES-DOC-0004`, p.28 §9.2. | v:307-316 |
| 060 | VER-REQ-0001 | Registry contains 247 records | B | 247 is the observed marker count and a policy-contract constant, not a numbered requirement value. | v:327-331 |
| 061 | VER-REQ-0001 | Requirement IDs are unique | A | `PES-REQ-0001` and `PES-REQ-0004`, pp.29-30 §10.1. | v:332 |
| 062 | VER-REQ-0001 | IDs match `PES-AREA-NNNN` | A | `PES-REQ-0001`–`0002`, p.29 §10.1. | v:333-337 |
| 063 | VER-REQ-0002 | Every record has implementation-selected schema fields | B | Exact JSON field set is structural enrichment; source §10.2 prose is not itself a numbered record and fields were expanded. | v:339-373 |
| 064 | VER-REQ-0001 | Heading path/body-block/hash pointer shape | B | Source-pointer representation and body-block numbering are extractor conventions. | v:374-387 |
| 065 | VER-REQ-0001 | No later marker/structural-spill regex hit | B | Parser boundary and spill-pattern heuristic selected by implementation. | v:388-400 |
| 066 | VER-REQ-0002 | Truth state is in allowed vocabulary | A | `PES-REQ-0008`–`0009`, p.31 §10.3, and the directive truth-state table. | v:401-405 |
| 067 | VER-REQ-0001 | Snapshot schema v2 and extractor hash match | B | Generator-provenance convention selected by implementation. | v:406-414 |
| 068 | VER-REQ-0002 | Reviewed/default IP fields separated from keyword triage | A | `PES-GOV-0003`–`0004`, pp.4-5 §1.1, and `PES-CRM-0006`–`0007`, pp.15-16 §6.2. | v:415-433 |
| 069 | VER-REQ-0002 | Class 7/8 rows not implemented/complete | A | `PES-CRM-0006`–`0007`, pp.15-16 §6.2, and `PES-DEC-0002`, p.32 §11.2. | v:434-440 |
| 070 | VER-REQ-0002 | Four representative executable obligations remain later/not-started | A | `PES-ACC-0007`, p.36 §13.2, and scope-stop rule `PES-DEC-0002`, p.32 §11.2. | v:441-453 |
| 071 | VER-REQ-0002 | No self-certified VERIFIED rows | A | `PES-REQ-0008`, p.31 §10.3, and `PES-QLT-0001`, p.33 §12.1. | v:454-460 |
| 072 | VER-REQ-0002 | Curated acceptance IDs equal contract allowlist | C | Same 20-ID choice is duplicated in extractor and policy contract; “independent” is not methodologically independent. | v:462-472 |
| 073 | VER-REQ-0002 | PES-CRM-0016 mapping/state equals contract | C | Verification/component/state expectations are duplicated repository constants. | v:473-497 |
| 074 | VER-REQ-0002 | PES-SEC-0017 mapping/state equals contract | C | Same circular mapping check. | v:473-497 |
| 075 | VER-REQ-0002 | PES-ARC-0030 mapping/state equals contract | C | Same circular mapping check. | v:473-497 |
| 076 | VER-REQ-0002 | PES-DOC-0001 mapping/state equals contract | C | Same circular mapping check. | v:473-497 |
| 077 | VER-REQ-0002 | PES-DOC-0002 mapping/state equals contract | C | Same circular mapping check. | v:473-497 |
| 078 | VER-REQ-0002 | PES-DOC-0003 mapping/state equals contract | C | Same circular mapping check. | v:473-497 |
| 079 | VER-REQ-0002 | PES-DOC-0004 mapping/state equals contract | C | Same circular mapping check. | v:473-497 |
| 080 | VER-REQ-0002 | PES-DEV-0010 mapping/state equals contract | C | Same circular mapping check. | v:473-497 |
| 081 | VER-REQ-0002 | PES-DEV-0012 mapping/state equals contract | C | Same circular mapping check. | v:473-497 |
| 082 | VER-REQ-0002 | PES-REQ-0001 mapping/state equals contract | C | Same circular mapping check. | v:473-497 |
| 083 | VER-REQ-0002 | PES-REQ-0002 mapping/state equals contract | C | Same circular mapping check. | v:473-497 |
| 084 | VER-REQ-0002 | PES-REQ-0003 mapping/state equals contract | C | Same circular mapping check. | v:473-497 |
| 085 | VER-REQ-0002 | PES-REQ-0004 mapping/state equals contract | C | Same circular mapping check. | v:473-497 |
| 086 | VER-REQ-0002 | PES-REQ-0008 mapping/state equals contract | C | Same circular mapping check. | v:473-497 |
| 087 | VER-REQ-0002 | PES-REQ-0009 mapping/state equals contract | C | Same circular mapping check. | v:473-497 |
| 088 | VER-REQ-0002 | PES-QLT-0001 mapping/state equals contract | C | Same circular mapping check. | v:473-497 |
| 089 | VER-REQ-0002 | PES-QLT-0004 mapping/state equals contract | C | Same circular mapping check. | v:473-497 |
| 090 | VER-REQ-0002 | PES-QLT-0005 mapping/state equals contract | C | Same circular mapping check. | v:473-497 |
| 091 | VER-REQ-0002 | PES-ACC-0006 mapping/state equals contract | C | Same circular mapping check. | v:473-497 |
| 092 | VER-REQ-0002 | PES-ACC-0007 mapping/state equals contract | C | Same circular mapping check. | v:473-497 |
| 093 | VER-REQ-0002 | Later empty dependencies labeled unresolved | B | `dependencyMaturity` and the blocking-vs-related split are local schema conventions. | v:498-509 |
| 094 | VER-REQ-0002 | Matrix contains 247 entries | B | Structural cardinality/freshness check. | v:511-518 |
| 095 | VER-REQ-0002 | Registry and matrix have identical ID coverage | C | Both views are emitted by the same extractor from the same in-memory records. | v:519-525 |
| 096 | VER-REQ-0002 | Matrix equals registry state/mapping/components | C | Both representations are emitted by the same generator and repeat the same enrichment choices. | v:526-538 |
| 097 | VER-REQ-0002 | Matrix state-count summary equals entries | B | Internal aggregate-consistency check, not a correctness oracle for assigned states. | v:539-544 |
| 098 | VER-REQ-0002 | Only VERIFIED means complete; no percentage | A | `PES-REQ-0008`–`0009`, p.31 §10.3. | v:545-550 |
| 099 | VER-QLT-0001 | Exactly PES-DEV-0006 is scaffolded with chosen owner/target | C | Exact ID, owner, milestone, and disposition are extractor-authored constants checked against themselves. | v:551-560 |
| 100 | VER-REQ-0002 | Every contract verification ID appears in plan | C | Both ID list and plan are repository-authored; content/adequacy is not checked. | v:563-570 |
| 101 | VER-CRM-0001 | Evidence register has ≥3 sources, rows, and selected fields | B | Exact schema and minimum cardinalities are implementation normalization; report happens to show 3/20. | v:572-620 |
| 102 | VER-CRM-0001 | Source/evidence IDs uniquely match SRC-NNNN | A | `PES-REQ-0003`, p.29 §10.1. | v:621-625 |
| 103 | VER-CRM-0001 | Evidence rows resolve and approval needs reviewer/date/checks | A | `PES-CRM-0017`, p.17 §6.5. | v:626-643 |
| 104 | VER-CRM-0001 | Project-local source hashes match register | B | Local evidence-integrity mechanism; the three-source selection is implementation-authored. | v:644-652 |
| 105 | VER-CRM-0001 | Evidence remains unreviewed and citations blocked | A | `PES-GOV-0013`, p.5 §1.3, and `PES-CRM-0017`, p.17 §6.5. | v:653-660 |
| 106 | VER-CRM-0001 | Asset schema exists; empty asset list passes | C | With `assets: []`, per-asset predicate is vacuous and the inventory is self-declared. | v:663-683 |
| 107 | VER-CRM-0001 | Asset summary counts equal asset list | C | Summary and empty list are two fields in the same repository-authored JSON. | v:684-691 |
| 108 | VER-CRM-0001 | Listed evidence-only files are outside root/hash-bound | B | Integrity/placement check over a self-selected evidence-only list. | v:692-701 |
| 109 | VER-CRM-0001 | Clean-room policy contains selected headings/terms | A | `PES-CRM-0016`–`0019`, pp.17-18 §6.5. | v:704-721 |
| 110 | VER-CRM-0001 | Blank attestation contains selected disclosure terms | A | `PES-CRM-0020`, p.18 §6.5, and `PES-CRM-0024`, p.18 §6.6. | v:722-734 |
| 111 | VER-CI-0001 | Toolchain register repeats selected versions/pins | C | Expected versions, action SHAs, runner and register prose are coordinated repository choices. | v:737-758 |
| 112 | VER-CI-0001 | Exactly 13 tool records have exact unreviewed row | C | Count/status sentence is duplicated between policy contract and register; no admission evidence is assessed. | v:759-775 |
| 113 | VER-DEC-0001 | OQ-0001 ID present | C | Contract ID searched in repository-authored register; decision text/disposition is not compared to directive Appendix C. | v:777-781 |
| 114 | VER-DEC-0001 | OQ-0002 ID present | C | Same ID-presence tautology. | v:777-781 |
| 115 | VER-DEC-0001 | OQ-0003 ID present | C | Same ID-presence tautology. | v:777-781 |
| 116 | VER-DEC-0001 | OQ-0004 ID present | C | Same ID-presence tautology. | v:777-781 |
| 117 | VER-DEC-0001 | OQ-0005 ID present | C | Same ID-presence tautology. | v:777-781 |
| 118 | VER-DEC-0001 | OQ-0006 ID present | C | Same ID-presence tautology. | v:777-781 |
| 119 | VER-DEC-0001 | OQ-0007 ID present | C | Same ID-presence tautology. | v:777-781 |
| 120 | VER-DEC-0001 | OQ-0008 ID present | C | Same ID-presence tautology. | v:777-781 |
| 121 | VER-DEC-0001 | OQ-0009 ID present | C | Same ID-presence tautology. | v:777-781 |
| 122 | VER-DEC-0001 | OQ-0010 ID present | C | Same ID-presence tautology. | v:777-781 |
| 123 | VER-DEC-0001 | DEC-0001 ID present | C | Locally invented decision ID is listed in contract and searched in its own register. | v:777-781 |
| 124 | VER-DEC-0001 | DEC-0002 ID present | C | Locally invented decision ID is listed in contract and searched in its own register. | v:777-781 |
| 125 | VER-DEC-0001 | DEC-0001 is marked BLOCKED | A | `PES-GOV-0006`, p.5 §1.2, requires a BLOCKED conflict record. | v:782-786 |
| 126 | VER-DEC-0001 | DEC-0002 is marked BLOCKED | A | `PES-DEC-0002`, p.32 §11.2, requires stop/ask before remote-service choice. | v:787-791 |
| 127 | VER-DEC-0001 | RSK-0001 ID present | C | Contract ID searched in repository-authored register; risk text/control is not compared to directive Appendix D. | v:792-794 |
| 128 | VER-DEC-0001 | RSK-0002 ID present | C | Same ID-presence tautology. | v:792-794 |
| 129 | VER-DEC-0001 | RSK-0003 ID present | C | Same ID-presence tautology. | v:792-794 |
| 130 | VER-DEC-0001 | RSK-0004 ID present | C | Same ID-presence tautology. | v:792-794 |
| 131 | VER-DEC-0001 | RSK-0005 ID present | C | Same ID-presence tautology. | v:792-794 |
| 132 | VER-DEC-0001 | RSK-0006 ID present | C | Same ID-presence tautology. | v:792-794 |
| 133 | VER-DEC-0001 | RSK-0007 ID present | C | Same ID-presence tautology. | v:792-794 |
| 134 | VER-DEC-0001 | RSK-0008 ID present | C | Same ID-presence tautology. | v:792-794 |
| 135 | VER-DEC-0001 | RSK-0009 ID present | C | Same ID-presence tautology. | v:792-794 |
| 136 | VER-DEC-0001 | RSK-0010 ID present | C | Same ID-presence tautology. | v:792-794 |
| 137 | VER-QLT-0001 | `apps` root absent | A | `PES-DEV-0010`, p.28 §9.3; `PES-ARC-0030`, p.26 §8.9; `PES-QLT-0001`, p.33 §12.1. | v:796-813 |
| 138 | VER-QLT-0001 | `packages` root absent | A | Same no-empty-package/no-placeholder requirements. | v:796-813 |
| 139 | VER-QLT-0001 | `profiles` root absent | A | `PES-QLT-0001`, p.33 §12.1, and `PES-ACC-0007`, p.36 §13.2. | v:796-813 |
| 140 | VER-QLT-0001 | `scenarios` root absent | A | `PES-QLT-0001`, p.33 §12.1, and `PES-ACC-0007`, p.36 §13.2. | v:796-813 |
| 141 | VER-QLT-0001 | `assets/original` root absent | A | `PES-DOC-0004`, p.28 §9.2; `PES-DEV-0010`, p.28 §9.3. | v:796-813 |
| 142 | VER-QLT-0001 | `artifacts` root absent | A | `PES-ACC-0007`, p.36 §13.2; later release duties `PES-CI-0002`–`0003`, p.29 §9.4. | v:796-813 |
| 143 | VER-QLT-0001 | `build` root absent | A | `PES-DEV-0010`, p.28 §9.3, and `PES-ACC-0007`, p.36 §13.2. | v:796-813 |
| 144 | VER-QLT-0001 | `dist` root absent | A | `PES-ACC-0007`, p.36 §13.2; `PES-CI-0002`–`0003`, p.29 §9.4. | v:796-813 |
| 145 | VER-QLT-0001 | Status documents deny Phase 1/product completion | A | `PES-ACC-0006`–`0007`, p.36 §13.2, and `PES-REQ-0008`–`0009`, p.31 §10.3. | v:815-826 |
| 146 | VER-QLT-0001 | Every project file appears in policy-contract manifest | B | Completeness of an implementation-authored controlled-file list, not directive semantics. | v:840-853 |
| 147 | VER-CI-0001 | All 41 controlled inputs present | B | Structural count against the same implementation-authored manifest. | v:854-858 |
| 148 | VER-ISO-0001 | No repository symlink | B | Sensible hardening, but no numbered directive requirement bans repository symlinks as such. | v:859-863 |
| 149 | VER-REQ-0001 | All PES references resolve to registry | A | Stable/non-recycled traceability under `PES-REQ-0001` and `PES-REQ-0004`, pp.29-30 §10.1. | v:865-881 |
| 150 | VER-CI-0001 | Root dependency count is zero | A | No-empty-product-root rule `PES-DEV-0010`, p.28 §9.3, and anti-placeholder `PES-QLT-0001`, p.33 §12.1. | v:883-908 |
| 151 | VER-CI-0001 | Package manifest exactly equals verifier object | C | The complete expected object is hard-coded in the verifier and compared to the repository file. | v:883-913 |
| 152 | VER-CI-0001 | Node/pnpm declarations equal contract pins | C | Both sides are repository-authored version constants. | v:914-920 |
| 153 | VER-CI-0001 | Python pin equals contract pin | C | Both sides are repository-authored version constants. | v:923-927 |
| 154 | VER-CI-0001 | Rust pin equals contract pin and exact text | C | Both sides are repository-authored constants. | v:928-934 |
| 155 | VER-CI-0001 | Git attributes contain hard-coded lines | C | Verifier literals are compared to repository configuration authored with them. | v:935-947 |
| 156 | VER-CI-0001 | Cargo manifest equals hard-coded empty manifest | C | Exact expected text is duplicated in verifier. | v:948-954 |
| 157 | VER-CI-0001 | Cargo lock equals hard-coded empty lockfile | C | Exact expected text is duplicated in verifier. | v:955-960 |
| 158 | VER-CI-0001 | pnpm workspace equals hard-coded text | C | Exact expected text is duplicated in verifier. | v:961-966 |
| 159 | VER-CI-0001 | pnpm lockfile equals hard-coded text | C | Exact expected text is duplicated in verifier. | v:967-972 |
| 160 | VER-CI-0001 | Workflow action refs equal contract SHAs | C | Proposed action SHAs are repository choices duplicated in workflow and contract; provenance/tag correctness is explicitly unknown. | v:974-987 |
| 161 | VER-CI-0001 | Workflow contains selected hard-coded strings | C | Verifier literals and contract pins restate the workflow authored with them; no remote behavior executes. | v:988-1003 |
| 162 | VER-DEC-0001 | Remote workflow has one literal-false job | A | Enforces affected-work stop under `PES-DEC-0002`, p.32 §11.2. | v:1004-1013 |
| 163 | VER-REQ-0002 | Every contract verification ID was emitted | C | The same verifier emits the IDs and compares them to its repository-authored contract list; it does not establish adequacy. | v:1016-1021 |

## Task 6 — Mutation testing

The pre-remediation harness detected only seven prescribed subjects. The
corrective harness freezes one immutable commit, runs the clean baseline first,
tests a separately sealed-manifest tamper, creates one isolated Git-archive copy
per mutation, and credits a mutation only when it exits `1`, emits the intended
named `FAIL`, emits no `ERROR`, and has no crash signature. Exit `2`, a generic
nonzero result, an unrelated check, or an exception earns no credit.

| Case | Single mutation | Intended detector | Required credited result |
|---|---|---|---|
| `M01` | Change `.editorconfig` bytes | `VER-INT-0002` | Named policy failure, exit 1, no crash |
| `M02` | Change frozen research bytes | `VER-GOV-0001` | Named policy failure, exit 1, no crash |
| `M03` | Delete a matrix record | `VER-REQ-0002` | Named policy failure, exit 1, no crash |
| `M04` | Promote one requirement to `VERIFIED` | `VER-REQ-0002` | Named policy failure, exit 1, no crash |
| `M05` | Introduce a source contradiction | `VER-REQ-0001` | Named policy failure, exit 1, no crash |
| `M06` | Insert `https://example.com` in production scope | `VER-OFF-0001` | Named policy failure, exit 1, no crash |
| `M07` | Insert `localhost:8080` in production scope | `VER-OFF-0002` | Named policy failure, exit 1, no crash |
| `M08` | Insert vendor-facing product text | `VER-BRN-0001` | Named policy failure, exit 1, no crash |
| `M09` | Add network-capable npm/Cargo dependencies | `VER-DEP-0001` | Named policy failure, exit 1, no crash |
| `M10` | Add an unauthorized runtime product-root file | `VER-SCP-0001` | Named policy failure, exit 1, no crash |
| `M11` | Close a risk without admissible evidence | `VER-RSK-0001` | Named policy failure, exit 1, no crash |
| `M12` | Remove ADR-0001 | `VER-ADR-0001` | Named policy failure, exit 1, no crash |

The corrective suite was executed against immutable validation commit
`c859c7a7126a2f7c36409e7e884afb61ca40bbaa`. The externally extracted manifest
had SHA-256
`BAEE99688C8DCB75B961D793ADF79C25F1C0AAA1116873428BCD3925BB9A71D9`.
The clean baseline exited `0`. Every M01–M12 subject exited `1`, printed its
intended named detector, printed no `ERROR`, and had `crashed=false`. The
separately corrupted sealed manifest was rejected with trust/tool exit `2` by
`VER-INT-0001`. Final score: **12/12**, manifest tamper test: **pass**, overall:
**pass**, remaining case directories: **0**, wrapper scratch removed: **true**.
The full raw transcript is retained at the ignored local evidence path
`.phase1-verification/mutations/mutation-results.json`; exact post-freeze
candidate evidence is attached through `refs/notes/phase1-closure-evidence`.

## Task 7 — Scope leak

The current product-root inventory is restricted by
`tests/phase1/policy-contract.json` to 22 exact checked-in files under
`apps/`, `packages/`, and `crates/`. They implement only the authorized minimal
foundation: a React screen, typed `foundation.health` command and
`DomainResult`, a Web Worker boundary, a dependency-free Rust/WASM health
function, and unit-test/build configuration. The generated WASM module is
excluded from the tracked manifest at one exact generated path and is rebuilt
deterministically.

No PLC project model, device or module catalog, tag/type system, address
allocator, LAD/FBD/SCL editor, compiler, scan-cycle runtime, virtual controller,
HMI designer/runtime, process model, scenario, lesson, assessment, teacher
mode, export, industrial protocol, device adapter, physical communication path,
packaging system, or public brand implementation exists. The verifier rejects
any unlisted product-root file through `VER-SCP-0001`.

The physical-isolation invariant remains unchanged: `VirtualUniverse` has no
adapter to `PhysicalUniverse`. Static source policy, dependency inventory, WASM
import inspection, standalone `file://` execution, and browser request capture
provide current foundation-level negative evidence. They do not pre-verify
later PLC runtime or packaged-product isolation requirements.

## Task 8 — Pre-remediation interpretation log

#### Method and source-location note

I treated text copied verbatim from a numbered directive record as literal extraction. Everything else—record boundaries, titles, schema, state, milestone, dependency relation, IP disposition, acceptance criterion, implementation mapping, test oracle, operational policy, or current-status judgment—was examined as potential interpretation or gap-filling.

Directive page numbers below come from the current 40-page local render solely to make the source easy to locate. That render remains an unapproved observation and is not used here as Phase 1 acceptance evidence. The authoritative anchors are the quoted `PES-*` IDs and DOCX section headings.

#### Exhaustive artifact ledger

| # | Artifact and line range | Interpretation / normalization / gap-filling | Source anchor | Adversarial assessment |
|---:|---|---|---|---|
| 1 | `OPEN_DECISIONS.md:15-71` | Creates `DEC-0001`, bundles three discrepancies, decides affected IDs, authors three options, and recommends Option A. The user said phase prompts would live in the folder; the record infers that this may conflict with the one-living-directive model. | Document Control, p.1; `PES-GOV-0006`, p.5 §1.2; `PES-GOV-0017`–`0020`, p.35 §13.1; `PES-ACC-0005`, p.36 §13.2. | Appropriate conservative escalation, but options/recommendation and the precise conflict bundle are Codex judgments, not source text or user approval. |
| 2 | `OPEN_DECISIONS.md:24-37` | Treats two filename mismatches, a status-wording mismatch, and the phase-prompt workflow as unresolved controlling-document identity. | Document Control, p.1; §§1.1-1.3, pp.4-5; §13.1, p.35. | Facts are source/user observations; the decision that they form one blocking identity conflict is interpretation. |
| 3 | `OPEN_DECISIONS.md:73-119` | Creates `DEC-0002` after choosing GitHub Actions as a disabled proposal; authors local-only, hosted-no-upload, and hosted-with-upload options and recommends local-only. | `PES-DEC-0002`, p.32 §11.2; `PES-SEC-0004`–`0005`, p.13 §5.4; `PES-CI-0001`–`0003`, pp.28-29 §9.4. | Stopping remote execution is source-derived. Selecting GitHub, action commits, runner family, logs, and a report-upload design before approval is gap-filling, even though the job is literally disabled. |
| 4 | `OPEN_DECISIONS.md:121-138` | Copies OQ-0001…0010 but adds `BLOCKED`/`DEFERRED` current states and affected-work wording. | Appendix C, p.36. | IDs/questions/deadlines are source material; present-state dispositions and their granularity are repository-authored interpretations. |
| 5 | `RISK_REGISTER.md:3-20` | Copies RSK-0001…0010, declares every risk `OPEN`, and adds a current-evidence narrative for each. | Appendix D, pp.36-37. | Risk/control pairs are source-derived; `OPEN`, evidence sufficiency, and current-control effectiveness are audit judgments. |
| 6 | `RISK_REGISTER.md:22-39` | Adds review triggers, residual-risk rules, and a global posture summary. | §10.4, p.31; §11, pp.31-33; §13.2, pp.35-36. | Sensible operationalization, not literal directive content. |
| 7 | `README.md:1-64`; `docs/governance/PHASE_1_SCOPE_AUDIT.md:3-83`; `CHANGELOG_DIRECTIVE.md:1-94` | Defines the repository as “Phase 1 governance foundation,” concludes product implementation is not started, records what work counts as foundation, and narrates the current gate state. | `PES-ACC-0006`–`0007`, p.36 §13.2; `PES-QLT-0001`–`0009`, pp.33-35 §12. | Correctly conservative status framing, but the exact boundary of “foundation work” and each implementation-log claim is a repository audit judgment. |
| 8 | `tools/phase1/extract_directive_requirements.py:21-30` | Hard-codes observed filenames/hashes and defines the accepted requirement-ID and normative-keyword grammars. | `PES-GOV-0010`, p.5 §1.3; `PES-REQ-0001`–`0002`, p.29 §10.1. | Research hash is directive-bound. Directive hash, filename choice pending DEC-0001, and parser regex are local snapshot/parse choices. |
| 9 | `tools/phase1/extract_directive_requirements.py:34-59` | Maps each area code to a human-readable `scopeComponent`. | Requirement IDs throughout; §10.1, p.29. | Every component phrase is Codex-authored metadata; the directive defines stable areas but not this wording. |
| 10 | `tools/phase1/extract_directive_requirements.py:286-335` | Normalizes Word runs/tabs/breaks to strings, falls back to `Normal` style, joins multiple cell paragraphs with ` / `, and later joins table cells with ` | `. | DOCX structure; §10.2, p.30. | Deterministic, but lossy: formatting, list semantics, merged-cell semantics, and some table structure are flattened into authored delimiters. |
| 11 | `tools/phase1/extract_directive_requirements.py:322-383` | Recognizes headings only when the style name matches `Heading N`; uses current heading stack; starts a record only on `[PES-*]`; appends every later non-heading paragraph/table until the next requirement/heading. | All numbered directive sections. | This parser defines requirement boundaries. It is not a semantic parser and can absorb continuations or tables based on layout rather than obligation meaning. |
| 12 | `tools/phase1/extract_directive_requirements.py:386-400` | Defaults a missing normative keyword to `MUST`; generates short titles from the first line, splits at punctuation, and truncates at 88 characters. | §10.2, p.30. | The `MUST` fallback would strengthen source text if ever used; current audit found zero fallback records. All 247 titles are generated summaries, not directive titles. |
| 13 | `tools/phase1/extract_directive_requirements.py:600-604`; `REQUIREMENTS.md:45-49` | Calls a record compound only when it has more than one continuation block or any table row, then leaves it unsplit to avoid changing IDs. | `PES-REQ-0005`, p.30 §10.2; change control `PES-GOV-0014`–`0016`, p.31 §10.4. | Material open gap: 20 records are `COMPOUND_SOURCE_REQUIRES_REVIEW`. The registry has 247 source markers, not 247 atomic/testable requirements. |
| 14 | `tools/phase1/extract_directive_requirements.py:62-121` | Curates requirement→verification IDs and requirement→implementation-component paths. | `PES-REQ-0006`–`0007`, p.30 §10.2. | Necessary traceability enrichment, but the selected mappings are not extracted from the directive and have no independent approval. |
| 15 | `tools/phase1/extract_directive_requirements.py:124-225` | Authors positive/negative acceptance criteria and “dependency” relationships for exactly 20 foundation requirements. | Corresponding requirements; generic record schema in §10.2, p.30. | Substantive engineering interpretation. Several predicates equate file/keyword absence with requirement implementation; none has reviewer acceptance. |
| 16 | `tools/phase1/extract_directive_requirements.py:228-244` | Selects 30 “Phase 1 foundation” IDs and assigns five of them `BLOCKED`, including the exact reasons for each. | §§1, 9-13, pp.4-36. | The block choices are conservative but manually curated. The DOCX does not state current repository truth for these IDs. |
| 17 | `tools/phase1/extract_directive_requirements.py:247-261` | Selects four requirements for curated Class 9 and a set of later-release verification IDs. | IP classes §6.2, pp.15-16; release gates §§5.7/9.4, pp.14-15/28-29. | Requirement-ID membership is hand-authored classification/phase allocation. |
| 18 | `tools/phase1/extract_directive_requirements.py:403-459` | Produces non-normative IP candidates by keyword matches for physical, trademark, expression, patent, uncertainty, proprietary format, workflow, and IEC concepts. | `PES-CRM-0006`–`0007`, pp.15-16 §6.2. | Explicitly labeled triage, but it is a heuristic inference: wording, synonyms, negation, and context can misclassify. |
| 19 | `tools/phase1/extract_directive_requirements.py:463-486` | Assigns curated Class 9 to four IDs, curated Class 1 to the other foundation IDs, and default Class 8 to all remaining IDs. | IP-class table and `PES-CRM-0006`–`0007`, pp.15-16. | Current counts: 30 curated IDs and 217 default-Class-8 IDs. “Reviewed for Phase 1 governance scope” is an internal review label, not professional legal/IP approval. |
| 20 | `tools/phase1/extract_directive_requirements.py:489-515` | Assigns milestones and `phase1Disposition` from membership/area-code rules. | Four-phase model §13.1, p.35; mandatory scope stop `PES-DEC-0002`, p.32. | Per-record phase allocation is inferred from broad domains, not authored individually in the source. |
| 21 | `tools/phase1/extract_directive_requirements.py:518-560` | Assigns truth states from hard-coded IDs and current file claims. | Truth-state table §10.3, pp.30-31. | Current output—5 BLOCKED, 22 IMPLEMENTED_UNVERIFIED, 217 NOT_STARTED, 2 PARTIAL, 1 SCAFFOLDED—is a Codex status assessment, not literal extraction or independent verification. |
| 22 | `tools/phase1/extract_directive_requirements.py:564-586` | Gives generic positive/negative acceptance text to 227 non-curated requirements based only on whether the keyword contains `NOT`. | Record schema §10.2, p.30. | Placeholder-quality normalization is honestly labeled unresolved, but it is not subject-aware acceptance and does not satisfy `PES-REQ-0005`–`0007`. |
| 23 | `tools/phase1/extract_directive_requirements.py:590-663`; `REQUIREMENTS.md:29-56` | Adds atomicity, rationale, scope, source pointer, research-classification boilerplate, empty blocking dependencies, related requirements, dependency maturity, phase disposition, decisions, components, owner `Scott`, reviewer state, and acceptance maturity. | §10.2, p.30; authority labels §§1.1-1.3, pp.4-5. | Most record fields are enriched. The blocking-dependency vs non-blocking-related distinction is repository-designed. Twenty are called curated; 227 explicitly remain unresolved. Owner assignment to Scott is inferred. |
| 24 | `tools/phase1/extract_directive_requirements.py:665-678,779-807`; `IMPLEMENTATION_MATRIX.json:1-2886`; `requirements/phase1-requirements.json:1-13401` | Emits the same states/mappings into two views, adds schema version 2, date, generator hash, completion rule, state summary, and scope. | `PES-REQ-0006`–`0009`, pp.30-31. | Matrix/register agreement is largely construction by the same generator, not independent corroboration. Generated JSON inherits every enrichment decision above. |
| 25 | `tools/phase1/extract_directive_requirements.py:681-728` | Treats exactly 247 as expected, requires heading paths, rejects embedded later markers, and validates only curated relationship ID existence. | `PES-REQ-0001`–`0005`, pp.29-30. | Structural safeguards. They do not prove semantic completeness, atomicity, correct continuation capture, or correct acceptance. |
| 26 | `EVIDENCE_REGISTER.json:1-110` | Defines schema v1, a shared `SRC-NNNN` namespace for sources and evidence rows, an implementation gate, three normalized sources, access dates, statuses, and an unresolved-citation state. | `PES-GOV-0013`, p.5; `PES-CRM-0017`, p.17; `PES-REQ-0003`, p.29. | Fields mostly follow the directive, but IDs, source grouping, access date, source status, source type, and treating the token inventory as `SRC-0003` are local normalization choices. |
| 27 | `EVIDENCE_REGISTER.json:111-511` | Authors 20 requirement-evidence records with titles, paraphrases, IP classes/dispositions, simulator-owned requirements, forbidden shortcuts, components, states, and notes. | Clean-room/evidence requirements §6, pp.15-18. | These are substantive interpretations by “Codex Phase 1 implementation agent.” Every row is unreviewed, and verification mappings are empty; they are not approved evidence. |
| 28 | `docs/research/UNRESOLVED_SOURCE_TOKENS.md:9-35,49-165` | Defines browsing markers as non-durable, uses a regex-like token interpretation, counts 162 marker groups/250 occurrences/99 tokens, and blocks reliance when sole support. | `PES-GOV-0013`, p.5; `PES-CRM-0017`, p.17. | Counts are literal to the frozen report. The conclusion that tokens cannot identify sources is a sound evidence-quality judgment, but token→claim mapping and underlying bibliography remain unresolved. |
| 29 | `ASSET_PROVENANCE.json:1-90` | Defines schema v1, chooses `assets/original/` as production root, adds approval/failure policy, required fields, forbidden-source examples, three evidence-only files, and declares an empty inventory/release-ineligible state. | `PES-CRM-0021`–`0023`, p.18 §6.6; repo shape p.28 §9.3; `PES-DOC-0004`, p.28. | Schema is legitimate gap-filling. Empty `assets: []` is a declaration, not by itself proof that no asset exists elsewhere; the verifier's per-asset check is vacuous when empty. |
| 30 | `CLEAN_ROOM_POLICY.md:25-169` | Operationalizes permitted/forbidden sources, evidence fields, roles, original-expression rules, generative-material handling, quarantine workflow, dependency controls, and merge/release gates. | §6, pp.15-18. | Mostly conservative elaboration. Role names, process sequence, and some merge mechanics are project policy choices, not literal source. |
| 31 | `SECURITY_INVARIANTS.md:13-190`; `THREAT_MODEL.md:15-136` | Converts safety requirements into named properties, trust zones, ownership, host allowlist, actors, assets, data flows, misuse cases, verification corpus, residual risks, and review triggers. | Safety wall §5, pp.10-15; trust model §7, pp.19-20; architecture §8, pp.21-26. | Strong source coverage, but actors, flow decomposition, threat taxonomy, and many test/ownership details are architectural interpretation pending product implementation. |
| 32 | `LEGAL_REVIEW_CHECKLIST.md:1-151` | Creates a controlled checklist, approval fields, legal gates, and sign-off structure. | Professional-review stops in §6, pp.15-18, and `PES-DEC-0002`, p.32. | Process gap-filling only; the file is expressly not legal advice and contains no completed approval. |
| 33 | `CONTRIBUTOR_CLEAN_ROOM_ATTESTATION.md:1-113` | Creates identity, source, generative-tool, asset/dependency, exposure, certification, and reviewer fields. | `PES-CRM-0020`, p.18 §6.5; `PES-CRM-0024`, p.18 §6.6. | Form design is implementation-authored; status is `UNCOMPLETED`, so it is not evidence that a contributor complied. |
| 34 | `DEPENDENCY_POLICY.md:13-193` | Defines dependency classes, admission-record fields, license-review categories, native/WASM policy, development-tool containment, maintenance gates, and exception/removal procedure. | `PES-CRM-0024`–`0025`, p.18; `PES-ISO-0008`–`0014`, pp.12-14; `PES-DEV-0006`–`0009`, pp.27-28. | Substantial policy interpretation. License-category treatment, exact evidence fields, and lifecycle are not source literals. |
| 35 | `DEPENDENCY_POLICY.md:77-96` | Creates a local bootstrap exception allowing pre-existing, standard-library-only tools before full admission, inferred from Scott's authorization to begin Phase 1. | `PES-SEC-0004`–`0005`, p.13 §5.4; `PES-DEC-0001`, p.31. | Material self-authorization risk: the directive permits ordinary dev tools but does not spell out this exception. The policy limits it, yet the exception was authored by the same implementation it authorizes. |
| 36 | `ADR/0001-no-physical-industrial-communication.md:18-124` | Expands the mandated invariant into runtime target, virtual-address rules, forbidden categories, production/dev separation, rejected alternatives, and verification obligations. | `PES-DOC-0001`–`0002`, p.27; safety wall §5, pp.10-15. | Title/status/separate-product rule are literal. The exact decomposition and verification program are derived architectural elaboration; the ADR is a binding safety invariant by source instruction. |
| 37 | `ADR/0002-original-project-format.md:18-158` | Defines project/archive boundary, manifest, UUID/reference behavior, canonicalization goals, untrusted-input controls, migration rules, and deferred schema questions. | `PES-PRJ-0001`–`0007`, p.18 §6.7; `PES-DOC-0003`, p.28. | Many invariants are source-derived. Internal layout, canonical encoding, compatibility window, downgrade behavior, and migration graph remain explicitly undecided under OQ-0005. |
| 38 | `ADR/0003-unified-plc-ir.md:18-152` | Defines one semantic lowering path, frontend contracts, type system, IR properties, artifact metadata, runtime contract, diagnostics, profiles, and later specification gates. | Architecture/type/IR §§8.1-8.5, pp.21-23; `PES-DOC-0003`, p.28. | Strongly derived but still a proposed architecture synthesis. Opcode set, wire format, optimization, and compatibility remain open; “status proposed” correctly avoids acceptance. |
| 39 | `ADR/0004-deterministic-virtual-time.md:18-176` | Defines authoritative time domains, event ordering, scan boundaries, replay identity, randomness, pause/step/speed semantics, concurrency, and equivalence contract. | `PES-DET-0001`–`0007`, p.24 §8.6; `PES-DOC-0003`, p.28. | Derived architectural synthesis. Priority/preemption and controller-family behavior remain expressly blocked; the ADR is proposed, not approved. |
| 40 | `docs/governance/TOOLCHAIN_ADMISSION_REGISTER.md:1-40,44-253` | Inventories 13 tools and assigns exact paths, versions, hashes, purposes, dispositions, blockers, and production-exclusion statements. | Stack/CI §9, pp.26-29; dependency/provenance §6.6, p.18. | Observed facts are useful. Tool choice, exact version baseline, record schema, and disposition labels are local judgments; every reviewer row remains unassigned/not reviewed. |
| 41 | `tools/phase1/run_phase1_verification.py:12-89` | Requires Python 3.13.12/Node 24.19.0, accepts `PHASE1_NODE`, searches PATH and a Codex-specific cache path, deduplicates resolved paths, and selects the first exact-version Node. | `PES-DEV-0006`, p.27 §9.1, requires exact/reproducible versions but not these versions or paths. | Platform-specific gap-filling; version equality does not establish executable provenance/admission. |
| 42 | `package.json:1-18`; `.python-version:1`; `Cargo.toml:1-8`; `Cargo.lock:1-3`; `pnpm-workspace.yaml:1-8`; `pnpm-lock.yaml:1-9`; `rust-toolchain.toml:1-3` | Selects project/package name, semantic placeholder version, Node/pnpm/Python/Rust versions, workspace layouts, resolver/edition, pnpm settings, and scripts. | `PES-DEV-0006`, p.27 §9.1; `PES-DEV-0010`, p.28 §9.3. | Reversible repository scaffolding, but all exact values are implementation choices and tools remain unapproved. Empty workspaces are intentionally SCAFFOLDED, not product implementation. |
| 43 | `.github/workflows/phase1-governance.yml:1-69` | Selects GitHub Actions, `windows-2025`, concurrency, 10-minute timeout, four action SHAs, exact runtime setup, commands, artifact name, hidden-file inclusion, warning behavior, and 14-day retention. | `PES-CI-0001`–`0003`, pp.28-29; stop rule `PES-DEC-0002`, p.32. | Entire remote design was a proposal and all exact service choices were gap-filled. A literal-false job condition prevented execution; the corrective workflow is now active and contains no upload step. |
| 44 | `.editorconfig:1-16`; `.gitattributes:1-21`; `.gitignore:1-37` | Chooses UTF-8/LF/indent rules, byte-preservation rules, ignored dependency/build/secret/editor/temp paths, and ignores all `.phase1-verification` evidence. | Reproducibility and evidence separation themes in §§6/9/10. | Conventional structural choices, not directive semantics. Ignoring evidence means a clean checkout cannot reproduce the visual observation without rerunning unapproved tools. |
| 45 | `tests/phase1/policy-contract.json:1-226` | Defines the test oracle: 247/13 counts, source and visual hashes, 41 required inputs, exact tool versions, action SHAs, empty VERIFIED list, 20 foundation mappings, ten verification IDs, state vocabulary, forbidden package names, and decision/risk IDs. | Many sections, especially §§9-13, pp.26-36. | Central circularity source. It is controlled input authored alongside the artifacts under test, not an external oracle. Direct source constants are mixed with implementation choices without per-field provenance. |
| 46 | `tools/phase1/verify-phase1.mjs:18-119` | Chooses ignored paths/files, file traversal, symlink treatment, exact hash implementation, page-manifest naming, JSON handling, and case-insensitive substring matching. | General governance/CI requirements. | Structural design. Keyword checks prove term presence, not semantic adequacy; ignored paths are outside controlled-file enumeration. |
| 47 | `tools/phase1/verify-phase1.mjs:125-1021` | Implements the 163 predicates, including hard-coded file content, allowlists, exact object/string comparisons, selected representative IDs, vacuous empty-asset checks, and repository-vs-contract comparisons. | Specific anchors itemized in Task 5. | Only 64 instances have a defensible numbered requirement anchor; 63 are circular and 36 structural under this audit rubric. |
| 48 | `tools/phase1/verify-phase1.mjs:1023-1055`; `.phase1-verification/phase1-report.json:1-885` | Defines report schema/version, absolute repository path, artifact-manifest scope, suite `PASS`, local visual-evidence status, and limitations. | `PES-REQ-0007`–`0009`, pp.30-31; CI §9.4, pp.28-29. | `PASS` means only “no emitted verifier predicate failed.” It is not Phase 1 exit, directive correctness, independent review, legal approval, tool admission, or product verification; current limitations state this clearly. |
| 49 | `docs/governance/DOCX_VISUAL_QA.md:1-52`; `tests/phase1/policy-contract.json:15-29`; `tools/phase1/verify-phase1.mjs:215-290` | Converts a Word→PDF→PNG/Python inspection into `OBSERVATION_PASS`, defines exact evidence hashes/count, records preview masking as false positives, and lets clean-checkout evidence absence pass with an explicit status. | Phase 1 document QA/exit context; §13.2, pp.35-36. | Observation judgment is not independently accepted. Word, Poppler, and PDF Python were outside the bootstrap exception and remain unapproved; the repository correctly says this cannot satisfy the visual-QA gate or authorize reuse. |
| 50 | `docs/governance/PHASE_1_VERIFICATION_PLAN.md:9-38` | Creates ten broad `VER-*` IDs, maps them to current-snapshot claims, and separates structural guardrails from later executable proof. | `PES-REQ-0006`–`0007`, p.30 §10.2. | Verification taxonomy and mapping are implementation-authored. A single ID is reused for many heterogeneous instances, so “10 checks” and “163 instances” answer different questions. |

#### Generated-record enrichment counts

These counts were recomputed from the current generated register; they are descriptive, not accepted correctness evidence:

| Enrichment dimension | Current count | Interpretation source |
|---|---:|---|
| Source IDs extracted | 247 | `[PES-*]` markers and extractor boundary rules |
| `COMPOUND_SOURCE_REQUIRES_REVIEW` | 20 | Continuation/table heuristic; not split |
| Curated Phase 1 acceptance criteria | 20 | `FOUNDATION_ACCEPTANCE` |
| Generic unresolved acceptance criteria | 227 | keyword-only fallback |
| Curated Phase 1 IP classification | 30 | requirement-ID allowlists |
| Default unresolved Class 8 | 217 | catch-all rule |
| Curated relationship maturity | 20 | same acceptance allowlist |
| Unresolved dependency baseline | 227 | catch-all rule |
| BLOCKED | 5 | manually selected current states |
| IMPLEMENTED_UNVERIFIED | 22 | manually selected current states |
| PARTIAL | 2 | manually selected current states |
| SCAFFOLDED | 1 | manually selected current state |
| NOT_STARTED | 217 | default current state |

#### Material adversarial findings

##### High — the suite is not an independent correctness oracle

The policy contract, extractor tables, verifier literals, verification plan, generated registry, matrix, toolchain register, and workflow repeat the same choices. The strongest example is `tools/phase1/verify-phase1.mjs:462-497`, which calls the contract allowlist “independent” while comparing it to mappings generated from `tools/phase1/extract_directive_requirements.py:62-225`; both were authored within the same implementation. The 63 class-C instances are useful drift alarms but cannot validate the underlying interpretation.

##### High — atomic requirement work is explicitly incomplete

`PES-REQ-0005` requires compound requirements to be split when parts can pass/fail separately. The extractor instead preserves source IDs and marks 20 records `COMPOUND_SOURCE_REQUIRES_REVIEW` (`tools/phase1/extract_directive_requirements.py:600-604`; `REQUIREMENTS.md:45-49`). This is defensible change-control caution, but it means the record set is not yet the atomic/testable system the directive requires. “247 requirements” should always be read as “247 source requirement IDs.”

##### High — current-state and mapping claims are authored judgments

The 30 non-default/current-foundation IDs, 20 curated acceptance records, state assignments, related-requirement lists, component paths, and `VER-*` mappings are all hard-coded enrichment. Their `IMPLEMENTED_UNVERIFIED`/`PARTIAL`/`SCAFFOLDED` labels are not derived merely by parsing the DOCX and have no named independent reviewer. The repository generally discloses this, but a passing verifier cannot promote their epistemic status.

##### Medium — evidence/asset checks are selective and partly vacuous

The evidence register validates only three self-listed local sources and 20 manually selected rows. The asset registry has `assets: []`; therefore its per-asset completeness predicate is vacuously true. Absence of reserved product roots helps, but the register itself is not a discovery scan or completed provenance audit.

##### Medium — exact toolchain and CI values are unapproved implementation choices

Node 24.19.0, pnpm 11.19.0, Python 3.13.12, Rust 1.94.0, GitHub Actions, `windows-2025`, four action SHAs, 10-minute timeout, and 14-day retention do not come from the directive. The register correctly leaves all 13 tools unapproved and the remote job disabled. Their presence is a proposal/scaffold, not evidence that those versions, binaries, services, or licenses are admissible.

##### Medium — the bootstrap exception is a self-authored authority bridge

`DEPENDENCY_POLICY.md:77-96` interprets Scott's “begin Phase 1” authorization and `PES-SEC-0004` as permission to use pre-existing, standard-library-only local tools before admission. This enabled the extractor/verifier, while the later Word/Poppler/PDF-Python run fell outside it. The boundary is now accurately disclosed, but the exception itself should receive explicit owner approval if it is to remain a governance authority rather than a temporary implementation assumption.

##### Medium — visual QA is a truthful but non-portable observation

The current local PDF, 40 PNGs, and analysis JSON match the recorded hashes; all-page review found no stored-render defect. Those files are ignored, absent on a clean checkout, and created with unapproved tools. The verifier intentionally records `ABSENT_IGNORED_LOCAL_EVIDENCE` without failure on a clean checkout. This is internally truthful, but the Markdown observation cannot become gate evidence until an admitted toolchain and reviewer perform a complete rerun.

#### Coverage statement

This ledger covers every controlled Phase 1 artifact class: the two generated requirement views; all four ADRs; clean-room, security, threat, legal, contributor, dependency, evidence, asset, decision, risk, scope, QA, verification-plan, toolchain, README, and changelog records; Node/pnpm/Python/Rust/workflow/repository configuration; policy contract; extractor; launcher; verifier; and latest report. The two supplied source documents were treated as inputs and were not attributed repository-authored interpretations.

### Bottom line (pre-remediation)

The repository is unusually candid about its limits: zero VERIFIED requirements, no Phase 2–4 feature roots, remote CI disabled, all tools unapproved, contributor/reviewer work incomplete, and visual evidence non-gating. The adversarial weakness is not a hidden product-completion claim; it is that a 163/163 `PASS` can look stronger than it is. Under this audit, only 64 instances have a specific directive anchor, and even those are mostly guardrails. The remaining 99 are structural or circular. The correct interpretation is: **the Phase 1 governance snapshot is internally consistent under its own implementation-authored contract, while atomicity, independent review, tool admission, evidence approval, and the Phase 1 exit gate remain open.**

## Not verifiable

The following matters cannot be proven or decided by this repository closure
run and remain explicit boundaries:

1. Scott's acceptance of the closure candidate and authorization to begin
   Phase 2.
2. Professional legal, trademark, license, provenance, privacy, and security
   approval of tools, dependencies, assets, or distribution terms.
3. Hosted CI behavior: there is no remote, push, credential, provider run,
   hosted log, report upload, or retention evidence.
4. The historical `PES-ACC-0005` exact-status-wording discrepancy; the source
   DOCX was not rewritten.
5. Later-phase product fidelity, PLC semantics, course transfer, packaging,
   performance, accessibility acceptance, SBOM/notices, and release safety.
6. Any observation requiring forbidden vendor-product access, physical device
   communication, packet capture, or a separately approved legal protocol.

## Phase 2 verdict

**READY FOR SCOTT REVIEW — PHASE 1 CLOSURE CANDIDATE.** All Critical and High
defects in the controlling defect register are resolved; G0-01 through G0-14
have direct evidence; the clean immutable baseline and all twelve intended
mutation detectors pass without crash credit; and the bounded health foundation
is real, offline, deterministic, and remains outside PLC product scope. This is
not Scott's acceptance, does not mark any requirement `VERIFIED`, and does not
authorize Phase 2. Phase 2 remains blocked pending Scott's explicit acceptance
and separate instruction.
