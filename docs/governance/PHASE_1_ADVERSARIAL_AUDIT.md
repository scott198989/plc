# Phase 1 Adversarial Audit

**Audit date:** 2026-08-27  
**Repository:** `C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC`  
**Audit posture:** defect-seeking, read-only inspection of existing Phase 1 artifacts  
**Source directive:** `PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx`  
**Source directive SHA-256:** `EBF074E2CEAB752F09E6DB63D88E100991729DA13C1EB874290A6B337DA72612`  
**Frozen research:** `Govs PLC project Research Report.md`  
**Frozen research SHA-256:** `F05C08323B5CC9483BEB1FEB3C7312CCB9A45EBE3B527E6DAE069C181D3FBF55`

## Executive finding

This audit found material defects. The requirement registry is not a complete atomic representation of the directive: the reverse walk found 546 in-scope normative statement units, of which 48 are unmapped, while 20 issued-ID records remain unsplit compound records. Five of twelve isolated mutations escaped the full governance suite. Of the suite's 163 passing check instances, 63 are tautological and 36 are structural; only 64 have a defensible numbered directive anchor, and that classification does not establish sufficiency. The current Phase 1 record therefore is **not trustworthy enough to authorize Phase 2 implementation yet**. The detailed verdict and required preconditions are at the end of this report.

## Evidence boundary and reproducibility

The audit treated the supplied DOCX, frozen research report, raw filesystem contents, and verbatim console output reproduced below as evidence. `IMPLEMENTATION_MATRIX.json`, `EVIDENCE_REGISTER.json`, `RISK_REGISTER.md`, `.phase1-verification/phase1-report.json`, and verifier output were inspected only as audit targets and were never used to prove their own correctness. Source-page anchors come from a fresh, read-only Microsoft Word export of the supplied DOCX to a temporary 40-page PDF; the fresh export was independently reported as 40 pages by Poppler. Normalized text extracted from every fresh page matched the existing local render page-for-page (`40/40`; combined text SHA-256 `DD5FAA0B213307CC6D4FBB8D5087FBC59751B777009CD2997BA9EF453E02FF`). This page-rendering method supplies location anchors, not approval or Phase 1 acceptance evidence.

Existing Phase 1 files were not edited. Mutation testing used one isolated scratch copy per mutation outside the repository, with an unmodified scratch baseline first and one mutation at a time. All per-case copies were removed immediately after execution; the entire audit scratch root was deleted after this report was assembled and checked. No commit was created, no truth state was promoted to `VERIFIED`, and no requirement, ADR, or register was added.

## Defects

<!-- DEFECTS -->

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

### Complete source recall ledger

<!-- LEDGER_START -->

#### Page 1 — 1 statement(s)

- **T2-0001** — p. 1; § Front matter; explicit; mapping: ``PES-MSN-0003``, ``PES-SCP-0002``
  - Verbatim: ``The product shall simulate engineering decisions and consequences with high training-transfer fidelity while remaining permanently incapable of communicating with or operating physical industrial equipment.``
  - Basis: materially equivalent numbered statement elsewhere in the source.

#### Page 2 — 4 statement(s)

- **T2-0002** — p. 2; § Front matter; explicit; mapping: ``PES-MSN-0003``, ``PES-FID-0002``
  - Verbatim: ``It shall provide high causal, behavioral, workflow, and training-transfer fidelity inside a wholly fictional VirtualUniverse.``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0003** — p. 2; § Front matter; explicit; mapping: ``PES-SCP-0002``, ``PES-ISO-0001``, ``PES-ISO-0002``
  - Verbatim: ``It shall never communicate with, discover, configure, commission, download to, or operate physical industrial equipment.``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0004** — p. 2; § Front matter; explicit; mapping: ``PES-ACC-0007``
  - Verbatim: ``Unless Scott separately orders otherwise, Codex shall not begin product implementation from this incomplete directive.``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0005** — p. 2; § Front matter; explicit; mapping: ``PES-GOV-0019``, ``PES-ACC-0007``
  - Verbatim: ``Reserved headings are not implementation requirements and shall not be inferred.``
  - Basis: materially equivalent numbered statement elsewhere in the source.

#### Page 3 — 2 statement(s)

- **T2-0006** — p. 3; § How to Use This Directive; explicit; mapping: ``PES-REQ-0004``
  - Verbatim: ``Never renumber or reuse one.``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0007** — p. 3; § How to Use This Directive; explicit; mapping: ``PES-ISO-0001``, ``PES-CRM-0001``, ``PES-FID-0002``
  - Verbatim: ``Never trade away the safety wall, clean-room rules, or causal-fidelity doctrine for speed, convenience, visual similarity, or a demo.``
  - Basis: materially equivalent numbered statement elsewhere in the source.

#### Page 4 — 17 statement(s)

- **T2-0008** — p. 4; § Normative Keywords; explicit table row; mapping: **UNMAPPED**
  - Verbatim: ``MUST / SHALL | Required. Violation blocks merge, release, or acceptance.``
  - Basis: UNMAPPED.
- **T2-0009** — p. 4; § Normative Keywords; explicit table row; mapping: **UNMAPPED**
  - Verbatim: ``MUST NOT / SHALL NOT | Prohibited. Presence blocks merge, release, or acceptance.``
  - Basis: UNMAPPED.
- **T2-0010** — p. 4; § Normative Keywords; explicit table row; mapping: **UNMAPPED**
  - Verbatim: ``MAY | Optional and permitted only inside the approved scope.``
  - Basis: UNMAPPED.
- **T2-0011** — p. 4; § 1.1 Authority hierarchy; explicit; mapping: ``PES-GOV-0001``
  - Verbatim: ``[PES-GOV-0001] MUST interpret this project using the following order:``
  - Basis: owning source requirement ID.
- **T2-0012** — p. 4; § 1.1 Authority hierarchy; inherited bullet; mapping: ``PES-GOV-0001``
  - Verbatim: ``Applicable law, binding licenses, and the immutable product safety constraints in this directive form the outer boundary.``
  - Modal lead-in: ``[PES-GOV-0001] MUST interpret this project using the following order:``
  - Basis: owning source requirement ID.
- **T2-0013** — p. 4; § 1.1 Authority hierarchy; inherited bullet; mapping: ``PES-GOV-0001``
  - Verbatim: ``Scott's explicit, approved product decisions govern product intent.``
  - Modal lead-in: ``[PES-GOV-0001] MUST interpret this project using the following order:``
  - Basis: owning source requirement ID.
- **T2-0014** — p. 4; § 1.1 Authority hierarchy; inherited bullet; mapping: ``PES-GOV-0001``
  - Verbatim: ``This living Codex Master Implementation Directive governs what shall be built.``
  - Modal lead-in: ``[PES-GOV-0001] MUST interpret this project using the following order:``
  - Basis: owning source requirement ID.
- **T2-0015** — p. 4; § 1.1 Authority hierarchy; inherited bullet; mapping: ``PES-GOV-0001``
  - Verbatim: ``The frozen research report supplies technical, workflow, and risk evidence.``
  - Modal lead-in: ``[PES-GOV-0001] MUST interpret this project using the following order:``
  - Basis: owning source requirement ID.
- **T2-0016** — p. 4; § 1.1 Authority hierarchy; inherited bullet; mapping: ``PES-GOV-0001``
  - Verbatim: ``Approved decision records and ADRs govern implementation choices only within the authority left to them.``
  - Modal lead-in: ``[PES-GOV-0001] MUST interpret this project using the following order:``
  - Basis: owning source requirement ID.
- **T2-0017** — p. 4; § 1.1 Authority hierarchy; inherited bullet; mapping: ``PES-GOV-0001``
  - Verbatim: ``Code, tests, tickets, comments, mockups, and developer assumptions are subordinate to all items above.``
  - Modal lead-in: ``[PES-GOV-0001] MUST interpret this project using the following order:``
  - Basis: owning source requirement ID.
- **T2-0018** — p. 4; § 1.1 Authority hierarchy; explicit; mapping: ``PES-GOV-0002``
  - Verbatim: ``[PES-GOV-0002] MUST NOT use a lower authority to weaken, reinterpret, or silently override a higher authority.``
  - Basis: owning source requirement ID.
- **T2-0019** — p. 4; § 1.1 Authority hierarchy; explicit; mapping: ``PES-GOV-0003``
  - Verbatim: ``[PES-GOV-0003] MUST treat the research report's labels accurately:``
  - Basis: owning source requirement ID.
- **T2-0020** — p. 4; § 1.1 Authority hierarchy; inherited bullet; mapping: ``PES-GOV-0003``
  - Verbatim: ``DOCUMENTED identifies publicly supported behavior or facts.``
  - Modal lead-in: ``[PES-GOV-0003] MUST treat the research report's labels accurately:``
  - Basis: owning source requirement ID.
- **T2-0021** — p. 4; § 1.1 Authority hierarchy; inherited bullet; mapping: ``PES-GOV-0003``
  - Verbatim: ``INFERENCE identifies a reasoned conclusion, not a documented exact behavior.``
  - Modal lead-in: ``[PES-GOV-0003] MUST treat the research report's labels accurately:``
  - Basis: owning source requirement ID.
- **T2-0022** — p. 4; § 1.1 Authority hierarchy; inherited bullet; mapping: ``PES-GOV-0003``
  - Verbatim: ``PROPOSED identifies simulator behavior recommended by the report.``
  - Modal lead-in: ``[PES-GOV-0003] MUST treat the research report's labels accurately:``
  - Basis: owning source requirement ID.
- **T2-0023** — p. 4; § 1.1 Authority hierarchy; inherited bullet; mapping: ``PES-GOV-0003``
  - Verbatim: ``LEGAL INTERPRETATION is risk analysis, not legal advice.``
  - Modal lead-in: ``[PES-GOV-0003] MUST treat the research report's labels accurately:``
  - Basis: owning source requirement ID.
- **T2-0024** — p. 4; § 1.1 Authority hierarchy; inherited bullet; mapping: ``PES-GOV-0003``
  - Verbatim: ``ENGINEERING RECOMMENDATION is an implementation or product judgment.``
  - Modal lead-in: ``[PES-GOV-0003] MUST treat the research report's labels accurately:``
  - Basis: owning source requirement ID.

#### Page 5 — 13 statement(s)

- **T2-0025** — p. 5; § 1.1 Authority hierarchy; explicit; mapping: ``PES-GOV-0004``
  - Verbatim: ``[PES-GOV-0004] MUST treat adopted requirements in this directive as normative regardless of the research label from which they originated.``
  - Basis: owning source requirement ID.
- **T2-0026** — p. 5; § 1.1 Authority hierarchy; explicit; mapping: ``PES-GOV-0004``
  - Verbatim: ``The source label remains attached for traceability and shall not be rewritten as stronger evidence.``
  - Basis: owning source requirement ID.
- **T2-0027** — p. 5; § 1.1 Authority hierarchy; explicit; mapping: ``PES-GOV-0005``
  - Verbatim: ``[PES-GOV-0005] MUST NOT claim that the research report is a legal opinion, patent clearance, trademark clearance, freedom-to-operate analysis, or guarantee of legality.``
  - Basis: owning source requirement ID.
- **T2-0028** — p. 5; § 1.2 Conflict protocol; explicit; mapping: ``PES-GOV-0006``
  - Verbatim: ``[PES-GOV-0006] MUST create a BLOCKED decision record when two authorities appear to conflict.``
  - Basis: owning source requirement ID.
- **T2-0029** — p. 5; § 1.2 Conflict protocol; explicit; mapping: ``PES-GOV-0006``
  - Verbatim: ``The record shall quote or precisely identify both statements, explain the conflict, list the affected requirement IDs and components, and state the minimum decision needed.``
  - Basis: owning source requirement ID.
- **T2-0030** — p. 5; § 1.2 Conflict protocol; explicit; mapping: ``PES-GOV-0007``
  - Verbatim: ``[PES-GOV-0007] MUST NOT resolve an authority conflict by selecting the easiest implementation, the closest vendor behavior, or the broadest feature scope.``
  - Basis: owning source requirement ID.
- **T2-0031** — p. 5; § 1.2 Conflict protocol; explicit; mapping: ``PES-GOV-0008``
  - Verbatim: ``[PES-GOV-0008] MUST construe ambiguity conservatively when physical capability, external communication, proprietary expression, user data, assessment integrity, or safety claims could be affected.``
  - Basis: owning source requirement ID.
- **T2-0032** — p. 5; § 1.2 Conflict protocol; explicit; mapping: ``PES-GOV-0009``
  - Verbatim: ``[PES-GOV-0009] MUST treat any proposal to add physical industrial communication as a proposal for a different product.``
  - Basis: owning source requirement ID.
- **T2-0033** — p. 5; § 1.3 Frozen research baseline; explicit; mapping: ``PES-GOV-0010``
  - Verbatim: ``[PES-GOV-0010] MUST use the research report identified by filename and SHA-256 in Document Control as the Phase 1 baseline.``
  - Basis: owning source requirement ID.
- **T2-0034** — p. 5; § 1.3 Frozen research baseline; explicit; mapping: ``PES-GOV-0011``
  - Verbatim: ``[PES-GOV-0011] MUST NOT expand scope automatically when a newer TIA release, IEC edition, browser capability, framework version, or industrial technology appears.``
  - Basis: owning source requirement ID.
- **T2-0035** — p. 5; § 1.3 Frozen research baseline; explicit; mapping: ``PES-GOV-0012``
  - Verbatim: ``[PES-GOV-0012] MUST process new research through an evidence record and an approved change record before it changes a normative requirement.``
  - Basis: owning source requirement ID.
- **T2-0036** — p. 5; § 1.3 Frozen research baseline; explicit; mapping: ``PES-GOV-0013``
  - Verbatim: ``[PES-GOV-0013] MUST replace fragile research citation tokens with stable evidence records containing source title, publisher, version/date, durable location, access date, and the claim supported.``
  - Basis: owning source requirement ID.
- **T2-0037** — p. 5; § 1.3 Frozen research baseline; explicit; mapping: ``PES-GOV-0013``
  - Verbatim: ``An unresolved source remains unresolved; Codex shall not invent bibliographic details.``
  - Basis: owning source requirement ID.

#### Page 6 — 21 statement(s)

- **T2-0038** — p. 6; § 2.1 Mission; explicit; mapping: ``PES-MSN-0001``
  - Verbatim: ``[PES-MSN-0001] MUST build an original, professional PLC engineering and automation simulation environment for classroom learning, independent study, guided troubleshooting, and instructor-led assessment.``
  - Basis: owning source requirement ID.
- **T2-0039** — p. 6; § 2.1 Mission; explicit; mapping: ``PES-MSN-0002``
  - Verbatim: ``[PES-MSN-0002] MUST make the student perform a recognizable modern PLC engineering lifecycle:``
  - Basis: owning source requirement ID.
- **T2-0040** — p. 6; § 2.1 Mission; inherited bullet; mapping: ``PES-MSN-0002``
  - Verbatim: ``Create or open a simulator-native project.``
  - Modal lead-in: ``[PES-MSN-0002] MUST make the student perform a recognizable modern PLC engineering lifecycle:``
  - Basis: owning source requirement ID.
- **T2-0041** — p. 6; § 2.1 Mission; inherited bullet; mapping: ``PES-MSN-0002``
  - Verbatim: ``Add fictional virtual controllers and devices.``
  - Modal lead-in: ``[PES-MSN-0002] MUST make the student perform a recognizable modern PLC engineering lifecycle:``
  - Basis: owning source requirement ID.
- **T2-0042** — p. 6; § 2.1 Mission; inherited bullet; mapping: ``PES-MSN-0002``
  - Verbatim: ``Configure virtual racks, modules, channels, addresses, logical networks, and topology.``
  - Modal lead-in: ``[PES-MSN-0002] MUST make the student perform a recognizable modern PLC engineering lifecycle:``
  - Basis: owning source requirement ID.
- **T2-0043** — p. 6; § 2.1 Mission; inherited bullet; mapping: ``PES-MSN-0002``
  - Verbatim: ``Create tags, constants, data types, and data blocks.``
  - Modal lead-in: ``[PES-MSN-0002] MUST make the student perform a recognizable modern PLC engineering lifecycle:``
  - Basis: owning source requirement ID.
- **T2-0044** — p. 6; § 2.1 Mission; inherited bullet; mapping: ``PES-MSN-0002``
  - Verbatim: ``create OB, FC, FB, instance DB, and global DB program structures.``
  - Modal lead-in: ``[PES-MSN-0002] MUST make the student perform a recognizable modern PLC engineering lifecycle:``
  - Basis: owning source requirement ID.
- **T2-0045** — p. 6; § 2.1 Mission; inherited bullet; mapping: ``PES-MSN-0002``
  - Verbatim: ``Program in LAD, FBD, and SCL (Structured Text).``
  - Modal lead-in: ``[PES-MSN-0002] MUST make the student perform a recognizable modern PLC engineering lifecycle:``
  - Basis: owning source requirement ID.
- **T2-0046** — p. 6; § 2.1 Mission; inherited bullet; mapping: ``PES-MSN-0002``
  - Verbatim: ``Compile genuine project, hardware, language, type, and dependency semantics.``
  - Modal lead-in: ``[PES-MSN-0002] MUST make the student perform a recognizable modern PLC engineering lifecycle:``
  - Basis: owning source requirement ID.
- **T2-0047** — p. 6; § 2.1 Mission; inherited bullet; mapping: ``PES-MSN-0002``
  - Verbatim: ``Repair real inconsistencies and rebuild.``
  - Modal lead-in: ``[PES-MSN-0002] MUST make the student perform a recognizable modern PLC engineering lifecycle:``
  - Basis: owning source requirement ID.
- **T2-0048** — p. 6; § 2.1 Mission; inherited bullet; mapping: ``PES-MSN-0002``
  - Verbatim: ``Start a fictional controller instance.``
  - Modal lead-in: ``[PES-MSN-0002] MUST make the student perform a recognizable modern PLC engineering lifecycle:``
  - Basis: owning source requirement ID.
- **T2-0049** — p. 6; § 2.1 Mission; inherited bullet; mapping: ``PES-MSN-0002``
  - Verbatim: ``Review an internal Virtual Load Preview.``
  - Modal lead-in: ``[PES-MSN-0002] MUST make the student perform a recognizable modern PLC engineering lifecycle:``
  - Basis: owning source requirement ID.
- **T2-0050** — p. 6; § 2.1 Mission; inherited bullet; mapping: ``PES-MSN-0002``
  - Verbatim: ``Perform an atomic Virtual Download to a VirtualControllerId.``
  - Modal lead-in: ``[PES-MSN-0002] MUST make the student perform a recognizable modern PLC engineering lifecycle:``
  - Basis: owning source requirement ID.
- **T2-0051** — p. 6; § 2.1 Mission; inherited bullet; mapping: ``PES-MSN-0002``
  - Verbatim: ``Use RUN, STOP, monitoring, watch, modify, force, and trace semantics.``
  - Modal lead-in: ``[PES-MSN-0002] MUST make the student perform a recognizable modern PLC engineering lifecycle:``
  - Basis: owning source requirement ID.
- **T2-0052** — p. 6; § 2.1 Mission; inherited bullet; mapping: ``PES-MSN-0002``
  - Verbatim: ``Operate a deterministic virtual process and virtual HMI.``
  - Modal lead-in: ``[PES-MSN-0002] MUST make the student perform a recognizable modern PLC engineering lifecycle:``
  - Basis: owning source requirement ID.
- **T2-0053** — p. 6; § 2.1 Mission; inherited bullet; mapping: ``PES-MSN-0002``
  - Verbatim: ``Diagnose causal code, hardware, network-graph, process, and HMI faults.``
  - Modal lead-in: ``[PES-MSN-0002] MUST make the student perform a recognizable modern PLC engineering lifecycle:``
  - Basis: owning source requirement ID.
- **T2-0054** — p. 6; § 2.1 Mission; inherited bullet; mapping: ``PES-MSN-0002``
  - Verbatim: ``Correct the underlying cause and verify the result.``
  - Modal lead-in: ``[PES-MSN-0002] MUST make the student perform a recognizable modern PLC engineering lifecycle:``
  - Basis: owning source requirement ID.
- **T2-0055** — p. 6; § 2.1 Mission; explicit; mapping: ``PES-MSN-0003``
  - Verbatim: ``[PES-MSN-0003] MUST make the engineering decisions and consequences authentic enough to transfer into an authorized laboratory workflow while keeping product identity, code, assets, devices, project formats, runtime, and communication capability original.``
  - Basis: owning source requirement ID.
- **T2-0056** — p. 6; § 2.1 Mission; explicit; mapping: ``PES-MSN-0004``
  - Verbatim: ``[PES-MSN-0004] MUST NOT be industrial-control software, a hardware configuration utility, a protocol client, a controller emulator, a firmware emulator, a TIA clone, or a Siemens-branded product.``
  - Basis: owning source requirement ID.
- **T2-0057** — p. 6; § 2.1 Mission; explicit; mapping: ``PES-MSN-0005``
  - Verbatim: ``[PES-MSN-0005] MUST define "fully functional" as fully functional inside VirtualUniverse.``
  - Basis: owning source requirement ID.
- **T2-0058** — p. 6; § 2.1 Mission; explicit; mapping: ``PES-MSN-0005``
  - Verbatim: ``The phrase never implies physical compatibility, real controller deployment, vendor-project compatibility, safety certification, or industrial suitability.``
  - Basis: owning source requirement ID.

#### Page 7 — 8 statement(s)

- **T2-0059** — p. 7; § 2.2 Intended users and environments; explicit; mapping: ``PES-MSN-0007``
  - Verbatim: ``[PES-MSN-0007] MUST operate in an offline classroom or home-study environment with no cloud service, remote font, CDN, telemetry service, license server, analytics endpoint, or internet connection required.``
  - Basis: owning source requirement ID.
- **T2-0060** — p. 7; § 2.2 Intended users and environments; explicit; mapping: ``PES-MSN-0008``
  - Verbatim: ``[PES-MSN-0008] MUST support a professional unassisted workflow for students and a separate explanatory/teaching experience without changing the underlying engineering semantics.``
  - Basis: owning source requirement ID.
- **T2-0061** — p. 7; § 2.2 Intended users and environments; explicit; mapping: ``PES-MSN-0009``
  - Verbatim: ``[PES-MSN-0009] MUST keep all claims educational.``
  - Basis: owning source requirement ID.
- **T2-0062** — p. 7; § 2.2 Intended users and environments; explicit; mapping: ``PES-MSN-0009``
  - Verbatim: ``It shall not claim certification, equivalence, endorsement, production readiness, or suitability for real machine control.``
  - Basis: owning source requirement ID.
- **T2-0063** — p. 7; § 2.3 Governing success definition; explicit; mapping: ``PES-ACC-0001``
  - Verbatim: ``[PES-ACC-0001] MUST judge success by causal and workflow transfer: the same kinds of engineering decisions should produce the same kinds of engineering consequences.``
  - Basis: owning source requirement ID.
- **T2-0064** — p. 7; § 2.3 Governing success definition; explicit; mapping: ``PES-ACC-0002``
  - Verbatim: ``[PES-ACC-0002] MUST count the product as failing if a polished UI masks fake compilation, canned diagnostics, a non-semantic editor, nondeterministic runtime, conflated offline/online state, or scenario-specific hard-coding.``
  - Basis: owning source requirement ID.
- **T2-0065** — p. 7; § 2.3 Governing success definition; explicit; mapping: ``PES-ACC-0003``
  - Verbatim: ``[PES-ACC-0003] MUST count the product as failing if any production code path can address, enumerate, connect to, send to, load to, commission, or operate a physical industrial target.``
  - Basis: owning source requirement ID.
- **T2-0066** — p. 7; § 2.3 Governing success definition; explicit; mapping: ``PES-ACC-0004``
  - Verbatim: ``[PES-ACC-0004] MUST target zero students leaving training with the belief that a simulator project can be loaded into a physical PLC.``
  - Basis: owning source requirement ID.

#### Page 8 — 12 statement(s)

- **T2-0067** — p. 8; § 3.1 Canonical product vocabulary; explicit; mapping: ``PES-VOC-0001``
  - Verbatim: ``[PES-VOC-0001] MUST use Teacher Mode as the public and schema/API canonical name.``
  - Basis: owning source requirement ID.
- **T2-0068** — p. 8; § 3.1 Canonical product vocabulary; explicit; mapping: ``PES-VOC-0002``
  - Verbatim: ``[PES-VOC-0002] MUST present the textual PLC language as SCL (Structured Text) on first use and may use SCL thereafter.``
  - Basis: owning source requirement ID.
- **T2-0069** — p. 8; § 3.1 Canonical product vocabulary; explicit; mapping: ``PES-VOC-0003``
  - Verbatim: ``[PES-VOC-0003] MUST use fictional brand-neutral device categories such as Compact Controller, Modular Controller, Performance Controller, Technology Controller, Distributed I/O Station, Basic Operator Panel, Advanced Operator Panel, Variable-Speed Drive, and Servo Drive.``
  - Basis: owning source requirement ID.
- **T2-0070** — p. 8; § 3.1 Canonical product vocabulary; explicit; mapping: ``PES-VOC-0004``
  - Verbatim: ``[PES-VOC-0004] MUST NOT use actual Siemens model numbers or marks as active catalog identities.``
  - Basis: owning source requirement ID.
- **T2-0071** — p. 8; § 3.1 Canonical product vocabulary; explicit; mapping: ``PES-VOC-0005``
  - Verbatim: ``[PES-VOC-0005] MUST NOT use shorthand such as "clone TIA," "simulate an S7," "connect virtually to an IP," "download like the real target," or "fully compatible" in requirements, UI, documentation, tests, marketing, or code comments.``
  - Basis: owning source requirement ID.
- **T2-0072** — p. 8; § 3.2 Product modes share one kernel; explicit; mapping: ``PES-EDU-0001``
  - Verbatim: ``[PES-EDU-0001] MUST make Engineering Mode, Learning Lens, and Teacher Mode use one project model, compiler, diagnostic system, virtual runtime, process state, HMI state, and persistence system.``
  - Basis: owning source requirement ID.
- **T2-0073** — p. 8; § 3.2 Product modes share one kernel; explicit; mapping: ``PES-EDU-0002``
  - Verbatim: ``[PES-EDU-0002] MUST NOT implement a separate simplified compiler or runtime for lessons.``
  - Basis: owning source requirement ID.
- **T2-0074** — p. 8; § 3.2 Product modes share one kernel; explicit; mapping: ``PES-EDU-0003``
  - Verbatim: ``[PES-EDU-0003] MUST keep Learning Lens observational.``
  - Basis: owning source requirement ID.
- **T2-0075** — p. 8; § 3.2 Product modes share one kernel; explicit; mapping: ``PES-EDU-0003``
  - Verbatim: ``It may pause, step, slow, inspect, annotate, and explain deterministic execution; it shall not alter program truth, compiler results, type rules, device state, or grading outcomes.``
  - Basis: owning source requirement ID.
- **T2-0076** — p. 8; § 3.2 Product modes share one kernel; explicit; mapping: ``PES-EDU-0004``
  - Verbatim: ``[PES-EDU-0004] MUST make Teacher Mode act through ordinary domain commands, scenario events, process faults, and virtual hardware faults.``
  - Basis: owning source requirement ID.
- **T2-0077** — p. 8; § 3.2 Product modes share one kernel; explicit; mapping: ``PES-EDU-0005``
  - Verbatim: ``[PES-EDU-0005] MUST NOT let Teacher Mode insert compiler errors, runtime diagnostics, HMI alarms, expected values, or "correct" program state directly.``
  - Basis: owning source requirement ID.
- **T2-0078** — p. 8; § 3.2 Product modes share one kernel; explicit; mapping: ``PES-EDU-0006``
  - Verbatim: ``[PES-EDU-0006] MUST NOT make Teacher Mode an online AI dependency.``
  - Basis: owning source requirement ID.

#### Page 9 — 17 statement(s)

- **T2-0079** — p. 9; § 3.3 Profile and version claims; explicit; mapping: ``PES-PROF-0001``
  - Verbatim: ``[PES-PROF-0001] MUST use a V21-era workflow as the principal frozen training reference for the first complete product baseline.``
  - Basis: owning source requirement ID.
- **T2-0080** — p. 9; § 3.3 Profile and version claims; explicit; mapping: ``PES-PROF-0002``
  - Verbatim: ``[PES-PROF-0002] MUST implement a version-neutral semantic core and declarative TrainingProfile capability manifests.``
  - Basis: owning source requirement ID.
- **T2-0081** — p. 9; § 3.3 Profile and version claims; explicit; mapping: ``PES-PROF-0003``
  - Verbatim: ``[PES-PROF-0003] MUST keep project schema version independent from TrainingProfile version.``
  - Basis: owning source requirement ID.
- **T2-0082** — p. 9; § 3.3 Profile and version claims; explicit; mapping: ``PES-PROF-0004``
  - Verbatim: ``[PES-PROF-0004] MUST pin a project's selected profile and capability-manifest version.``
  - Basis: owning source requirement ID.
- **T2-0083** — p. 9; § 3.3 Profile and version claims; explicit; mapping: ``PES-PROF-0004``
  - Verbatim: ``Opening or migrating a project shall not silently change runtime semantics.``
  - Basis: owning source requirement ID.
- **T2-0084** — p. 9; § 3.3 Profile and version claims; explicit; mapping: ``PES-PROF-0005``
  - Verbatim: ``[PES-PROF-0005] MUST treat V19-era and V20-era profiles as the first compatibility targets after the primary profile.``
  - Basis: owning source requirement ID.
- **T2-0085** — p. 9; § 3.3 Profile and version claims; explicit; mapping: ``PES-PROF-0005``
  - Verbatim: ``Until their behavior is specified and tested, they shall be marked DEFERRED rather than presented as functioning profiles.``
  - Basis: owning source requirement ID.
- **T2-0086** — p. 9; § 3.3 Profile and version claims; explicit; mapping: ``PES-PROF-0006``
  - Verbatim: ``[PES-PROF-0006] MUST NOT claim exact controller-family fidelity for behavior the research report marks unresolved, including detailed OB priorities, recursion, optimized layouts, force edge cases, or vendor-specific SCL extensions.``
  - Basis: owning source requirement ID.
- **T2-0087** — p. 9; § 4.1 Required fidelity; explicit; mapping: ``PES-FID-0001``
  - Verbatim: ``[PES-FID-0001] MUST prioritize fidelity in this order:``
  - Basis: owning source requirement ID.
- **T2-0088** — p. 9; § 4.1 Required fidelity; inherited bullet; mapping: ``PES-FID-0001``
  - Verbatim: ``Safety and physical isolation.``
  - Modal lead-in: ``[PES-FID-0001] MUST prioritize fidelity in this order:``
  - Basis: owning source requirement ID.
- **T2-0089** — p. 9; § 4.1 Required fidelity; inherited bullet; mapping: ``PES-FID-0001``
  - Verbatim: ``Correct domain semantics and causality.``
  - Modal lead-in: ``[PES-FID-0001] MUST prioritize fidelity in this order:``
  - Basis: owning source requirement ID.
- **T2-0090** — p. 9; § 4.1 Required fidelity; inherited bullet; mapping: ``PES-FID-0001``
  - Verbatim: ``Training-transfer workflow and state consequences.``
  - Modal lead-in: ``[PES-FID-0001] MUST prioritize fidelity in this order:``
  - Basis: owning source requirement ID.
- **T2-0091** — p. 9; § 4.1 Required fidelity; inherited bullet; mapping: ``PES-FID-0001``
  - Verbatim: ``Determinism, inspectability, and diagnostic navigation.``
  - Modal lead-in: ``[PES-FID-0001] MUST prioritize fidelity in this order:``
  - Basis: owning source requirement ID.
- **T2-0092** — p. 9; § 4.1 Required fidelity; inherited bullet; mapping: ``PES-FID-0001``
  - Verbatim: ``Professional interaction quality and accessibility.``
  - Modal lead-in: ``[PES-FID-0001] MUST prioritize fidelity in this order:``
  - Basis: owning source requirement ID.
- **T2-0093** — p. 9; § 4.1 Required fidelity; inherited bullet; mapping: ``PES-FID-0001``
  - Verbatim: ``Original visual polish.``
  - Modal lead-in: ``[PES-FID-0001] MUST prioritize fidelity in this order:``
  - Basis: owning source requirement ID.
- **T2-0094** — p. 9; § 4.1 Required fidelity; explicit; mapping: ``PES-FID-0002``
  - Verbatim: ``[PES-FID-0002] MUST implement causal fidelity rather than screenshot fidelity.``
  - Basis: owning source requirement ID.
- **T2-0095** — p. 9; § 4.1 Required fidelity; explicit; mapping: ``PES-FID-0003``
  - Verbatim: ``[PES-FID-0003] MUST preserve meaningful distinctions that commercial engineering software teaches, including saved versus unsaved, built versus dirty, hardware build versus software build, offline source versus loaded artifact, loaded versus matching, RUN versus STOP, monitoring off versus on, raw process value versus CPU-visible value, modify versus force, initial versus actual versus retained value, and incoming versus cleared diagnostics.``
  - Basis: owning source requirement ID.

#### Page 10 — 8 statement(s)

- **T2-0096** — p. 10; § 4.1 Required fidelity; explicit; mapping: ``PES-FID-0004``
  - Verbatim: ``[PES-FID-0004] MUST make failure arise from a domain invariant, parser, resolver, type checker, compiler rule, build state, runtime state, process state, HMI state, or explicit virtual fault.``
  - Basis: owning source requirement ID.
- **T2-0097** — p. 10; § 4.1 Required fidelity; explicit; mapping: ``PES-FID-0005``
  - Verbatim: ``[PES-FID-0005] MUST keep invalid structures editable where appropriate while preventing invalid executable output.``
  - Basis: owning source requirement ID.
- **T2-0098** — p. 10; § 4.1 Required fidelity; explicit; mapping: ``PES-FID-0005``
  - Verbatim: ``An invalid LAD or FBD graph shall not produce executable IR.``
  - Basis: owning source requirement ID.
- **T2-0099** — p. 10; § 4.1 Required fidelity; explicit; mapping: ``PES-FID-0006``
  - Verbatim: ``[PES-FID-0006] MUST preserve stable identity through rename, maintain unresolved references through deletion, and restore original identity through undo.``
  - Basis: owning source requirement ID.
- **T2-0100** — p. 10; § 4.1 Required fidelity; explicit; mapping: ``PES-FID-0007``
  - Verbatim: ``[PES-FID-0007] MUST make diagnostics navigable to stable object identity and, when applicable, source range, graph node, pin, slot, channel, tag, or related object.``
  - Basis: owning source requirement ID.
- **T2-0101** — p. 10; § 4.1 Required fidelity; explicit; mapping: ``PES-FID-0008``
  - Verbatim: ``[PES-FID-0008] MUST NOT treat visual resemblance, screenshots, animated progress, or canned demo success as fidelity evidence.``
  - Basis: owning source requirement ID.
- **T2-0102** — p. 10; § 4.2 Included product envelope; explicit; mapping: ``PES-SCP-0001``
  - Verbatim: ``[PES-SCP-0001] MUST interpret this envelope as a promise to specify and implement genuine behavior in later phases, not authorization to create empty panels or placeholder APIs now.``
  - Basis: owning source requirement ID.
- **T2-0103** — p. 10; § 4.3 Permanently excluded; explicit; mapping: ``PES-SCP-0002``
  - Verbatim: ``[PES-SCP-0002] MUST NOT include any capability to communicate with or operate physical PLCs, HMIs, drives, remote I/O, gateways, instruments, sensors, actuators, robots, industrial networks, or host-connected devices.``
  - Basis: owning source requirement ID.

#### Page 11 — 12 statement(s)

- **T2-0104** — p. 11; § 4.3 Permanently excluded; explicit; mapping: ``PES-SCP-0003``
  - Verbatim: ``[PES-SCP-0003] MUST NOT implement Siemens firmware, binaries, proprietary project formats, protocol payloads, device packages, engineering APIs, iconography, diagnostic prose, hardware illustrations, or vendor load artifacts.``
  - Basis: owning source requirement ID.
- **T2-0105** — p. 11; § 4.3 Permanently excluded; explicit; mapping: ``PES-SCP-0004``
  - Verbatim: ``[PES-SCP-0004] MUST NOT import or export a file intended to be accepted by a physical PLC, HMI, drive, vendor engineering system, or industrial communication tool.``
  - Basis: owning source requirement ID.
- **T2-0106** — p. 11; § 4.3 Permanently excluded; explicit; mapping: ``PES-SCP-0005``
  - Verbatim: ``[PES-SCP-0005] MUST NOT provide safety-rated programming, validation, certification, or claims.``
  - Basis: owning source requirement ID.
- **T2-0107** — p. 11; § 4.3 Permanently excluded; explicit; mapping: ``PES-SCP-0005``
  - Verbatim: ``Ordinary educational interlocks shall be labeled non-safety-rated.``
  - Basis: owning source requirement ID.
- **T2-0108** — p. 11; § 4.3 Permanently excluded; explicit; mapping: ``PES-SCP-0006``
  - Verbatim: ``[PES-SCP-0006] MUST NOT provide remote collaboration, a cloud project server, telemetry, cloud grading, cloud AI, or a production local HTTP/WebSocket server.``
  - Basis: owning source requirement ID.
- **T2-0109** — p. 11; § 4.4 Deferred and gated features; explicit; mapping: ``PES-SCP-0008``
  - Verbatim: ``[PES-SCP-0008] MUST mark those features DEFERRED until later phases define semantics, risks, and acceptance tests.``
  - Basis: owning source requirement ID.
- **T2-0110** — p. 11; § 4.4 Deferred and gated features; explicit; mapping: ``PES-SCP-0009``
  - Verbatim: ``[PES-SCP-0009] MUST NOT expose a deferred feature as an enabled control, working catalog item, selectable profile, successful command, or release claim.``
  - Basis: owning source requirement ID.
- **T2-0111** — p. 11; § 4.4 Deferred and gated features; explicit; mapping: ``PES-SCP-0010``
  - Verbatim: ``[PES-SCP-0010] MUST require professional legal review before implementing behaviorally close auto-tuning, advanced motion trajectories, specialized drive models, unusual commissioning workflows, advanced digital-twin algorithms, or any Class 7 or Class 8 item.``
  - Basis: owning source requirement ID.
- **T2-0112** — p. 11; § 5.1 Constitutional invariant; explicit; mapping: ``PES-ISO-0001``
  - Verbatim: ``[PES-ISO-0001] MUST enforce this statement as a permanent product-scope and security invariant.``
  - Basis: owning source requirement ID.
- **T2-0113** — p. 11; § 5.1 Constitutional invariant; explicit; mapping: ``PES-ISO-0002``
  - Verbatim: ``[PES-ISO-0002] MUST NOT create a disabled adapter, generic PLC connection interface, transport provider, driver abstraction, protocol plugin, future-facing physical seam, feature flag, experimental connector, or "simulator now, hardware later" architecture.``
  - Basis: owning source requirement ID.
- **T2-0114** — p. 11; § 5.1 Constitutional invariant; explicit; mapping: ``PES-ISO-0003``
  - Verbatim: ``[PES-ISO-0003] MUST model a controller session only by opaque VirtualControllerId and simulator state.``
  - Basis: owning source requirement ID.
- **T2-0115** — p. 11; § 5.1 Constitutional invariant; explicit; mapping: ``PES-ISO-0003``
  - Verbatim: ``The domain API shall contain no hostname, IP endpoint, URL, port, socket, interface index, MAC address used as a host target, USB identity, serial handle, Bluetooth identity, or generic connection string.``
  - Basis: owning source requirement ID.

#### Page 12 — 21 statement(s)

- **T2-0116** — p. 12; § 5.1 Constitutional invariant; explicit; mapping: ``PES-ISO-0004``
  - Verbatim: ``[PES-ISO-0004] MUST represent virtual addresses with opaque domain value types such as VirtualIpAddress.``
  - Basis: owning source requirement ID.
- **T2-0117** — p. 12; § 5.1 Constitutional invariant; explicit; mapping: ``PES-ISO-0004``
  - Verbatim: ``They shall not convert into host endpoint types.``
  - Basis: owning source requirement ID.
- **T2-0118** — p. 12; § 5.1 Constitutional invariant; explicit; mapping: ``PES-ISO-0005``
  - Verbatim: ``[PES-ISO-0005] MUST implement fictional device discovery only as an in-memory query whose result is a subset of VirtualUniverse devices.``
  - Basis: owning source requirement ID.
- **T2-0119** — p. 12; § 5.1 Constitutional invariant; explicit; mapping: ``PES-ISO-0006``
  - Verbatim: ``[PES-ISO-0006] MUST implement Virtual Download only as an atomic internal build-artifact transaction against a VirtualControllerId.``
  - Basis: owning source requirement ID.
- **T2-0120** — p. 12; § 5.1 Constitutional invariant; explicit; mapping: ``PES-ISO-0007``
  - Verbatim: ``[PES-ISO-0007] MUST implement controller/process/HMI value exchange only through typed internal messages and InternalTagBus.``
  - Basis: owning source requirement ID.
- **T2-0121** — p. 12; § 5.2 Forbidden communication capabilities; explicit; mapping: ``PES-ISO-0008``
  - Verbatim: ``[PES-ISO-0008] MUST NOT contain or expose implementations of:``
  - Basis: owning source requirement ID.
- **T2-0122** — p. 12; § 5.2 Forbidden communication capabilities; inherited bullet; mapping: ``PES-ISO-0008``
  - Verbatim: ``S7, S7comm, or S7comm-plus;``
  - Modal lead-in: ``[PES-ISO-0008] MUST NOT contain or expose implementations of:``
  - Basis: owning source requirement ID.
- **T2-0123** — p. 12; § 5.2 Forbidden communication capabilities; inherited bullet; mapping: ``PES-ISO-0008``
  - Verbatim: ``PROFINET DCP, PROFINET I/O, or PROFIBUS;``
  - Modal lead-in: ``[PES-ISO-0008] MUST NOT contain or expose implementations of:``
  - Basis: owning source requirement ID.
- **T2-0124** — p. 12; § 5.2 Forbidden communication capabilities; inherited bullet; mapping: ``PES-ISO-0008``
  - Verbatim: ``EtherNet/IP or CIP;``
  - Modal lead-in: ``[PES-ISO-0008] MUST NOT contain or expose implementations of:``
  - Basis: owning source requirement ID.
- **T2-0125** — p. 12; § 5.2 Forbidden communication capabilities; inherited bullet; mapping: ``PES-ISO-0008``
  - Verbatim: ``Modbus TCP or RTU;``
  - Modal lead-in: ``[PES-ISO-0008] MUST NOT contain or expose implementations of:``
  - Basis: owning source requirement ID.
- **T2-0126** — p. 12; § 5.2 Forbidden communication capabilities; inherited bullet; mapping: ``PES-ISO-0008``
  - Verbatim: ``external OPC UA;``
  - Modal lead-in: ``[PES-ISO-0008] MUST NOT contain or expose implementations of:``
  - Basis: owning source requirement ID.
- **T2-0127** — p. 12; § 5.2 Forbidden communication capabilities; inherited bullet; mapping: ``PES-ISO-0008``
  - Verbatim: ``EtherCAT, CAN, CANopen, DeviceNet, BACnet, MQTT, or other physical/industrial transports;``
  - Modal lead-in: ``[PES-ISO-0008] MUST NOT contain or expose implementations of:``
  - Basis: owning source requirement ID.
- **T2-0128** — p. 12; § 5.2 Forbidden communication capabilities; inherited bullet; mapping: ``PES-ISO-0008``
  - Verbatim: ``vendor PLC, HMI, drive, or I/O SDKs;``
  - Modal lead-in: ``[PES-ISO-0008] MUST NOT contain or expose implementations of:``
  - Basis: owning source requirement ID.
- **T2-0129** — p. 12; § 5.2 Forbidden communication capabilities; inherited bullet; mapping: ``PES-ISO-0008``
  - Verbatim: ``TIA Openness, Siemens engineering DLLs, or PLCSIM APIs;``
  - Modal lead-in: ``[PES-ISO-0008] MUST NOT contain or expose implementations of:``
  - Basis: owning source requirement ID.
- **T2-0130** — p. 12; § 5.2 Forbidden communication capabilities; inherited bullet; mapping: ``PES-ISO-0008``
  - Verbatim: ``physical device discovery or host NIC enumeration;``
  - Modal lead-in: ``[PES-ISO-0008] MUST NOT contain or expose implementations of:``
  - Basis: owning source requirement ID.
- **T2-0131** — p. 12; § 5.2 Forbidden communication capabilities; inherited bullet; mapping: ``PES-ISO-0008``
  - Verbatim: ``raw Ethernet, packet capture, or industrial protocol frames.``
  - Modal lead-in: ``[PES-ISO-0008] MUST NOT contain or expose implementations of:``
  - Basis: owning source requirement ID.
- **T2-0132** — p. 12; § 5.2 Forbidden communication capabilities; explicit; mapping: ``PES-ISO-0009``
  - Verbatim: ``[PES-ISO-0009] MUST NOT let shipped production code invoke or expose:``
  - Basis: owning source requirement ID.
- **T2-0133** — p. 12; § 5.2 Forbidden communication capabilities; inherited bullet; mapping: ``PES-ISO-0009``
  - Verbatim: ``TCP, UDP, raw sockets, TLS, DNS, HTTP, HTTPS, local HTTP, localhost servers, or generic socket APIs;``
  - Modal lead-in: ``[PES-ISO-0009] MUST NOT let shipped production code invoke or expose:``
  - Basis: owning source requirement ID.
- **T2-0134** — p. 12; § 5.2 Forbidden communication capabilities; inherited bullet; mapping: ``PES-ISO-0009``
  - Verbatim: ``fetch, XMLHttpRequest, WebSocket, WebRTC, EventSource to an endpoint, sendBeacon, WebTransport, or service-worker network interception;``
  - Modal lead-in: ``[PES-ISO-0009] MUST NOT let shipped production code invoke or expose:``
  - Basis: owning source requirement ID.
- **T2-0135** — p. 12; § 5.2 Forbidden communication capabilities; inherited bullet; mapping: ``PES-ISO-0009``
  - Verbatim: ``WebSerial, WebUSB, WebBluetooth, WebHID, WebNFC, WebMIDI, or later equivalent device APIs;``
  - Modal lead-in: ``[PES-ISO-0009] MUST NOT let shipped production code invoke or expose:``
  - Basis: owning source requirement ID.
- **T2-0136** — p. 12; § 5.2 Forbidden communication capabilities; inherited bullet; mapping: ``PES-ISO-0009``
  - Verbatim: ``serial ports, USB, Bluetooth, pcap, native device enumeration, or arbitrary filesystem devices;``
  - Modal lead-in: ``[PES-ISO-0009] MUST NOT let shipped production code invoke or expose:``
  - Basis: owning source requirement ID.

#### Page 13 — 17 statement(s)

- **T2-0137** — p. 13; § 5.2 Forbidden communication capabilities; inherited bullet; mapping: ``PES-ISO-0009``
  - Verbatim: ``child-process execution, shell commands, dynamic library loading, native FFI, dlopen, arbitrary native bridges, or plugins able to reach those capabilities.``
  - Modal lead-in: ``[PES-ISO-0009] MUST NOT let shipped production code invoke or expose:``
  - Basis: owning source requirement ID.
- **T2-0138** — p. 13; § 5.2 Forbidden communication capabilities; explicit; mapping: ``PES-ISO-0010``
  - Verbatim: ``[PES-ISO-0010] MUST apply the prohibition to the entire shipped classroom product, including UI, project model, compiler, runtime, process engine, diagnostics, HMI, Teacher Mode, Learning Lens, importers, exporters, scripting, packaging glue, and production dependencies.``
  - Basis: owning source requirement ID.
- **T2-0139** — p. 13; § 5.3 Narrow host allowlist; explicit; mapping: ``PES-SEC-0001``
  - Verbatim: ``[PES-SEC-0001] MAY permit only these host capabilities in the production product:``
  - Basis: owning source requirement ID.
- **T2-0140** — p. 13; § 5.3 Narrow host allowlist; inherited bullet; mapping: ``PES-SEC-0001``
  - Verbatim: ``local rendering and user interaction;``
  - Modal lead-in: ``[PES-SEC-0001] MAY permit only these host capabilities in the production product:``
  - Basis: owning source requirement ID.
- **T2-0141** — p. 13; § 5.3 Narrow host allowlist; inherited bullet; mapping: ``PES-SEC-0001``
  - Verbatim: ``explicit user-initiated open/save of simulator-native project, archive, CSV, JSON, image, or report files approved by later requirements;``
  - Modal lead-in: ``[PES-SEC-0001] MAY permit only these host capabilities in the production product:``
  - Basis: owning source requirement ID.
- **T2-0142** — p. 13; § 5.3 Narrow host allowlist; inherited bullet; mapping: ``PES-SEC-0001``
  - Verbatim: ``controlled application-local persistence;``
  - Modal lead-in: ``[PES-SEC-0001] MAY permit only these host capabilities in the production product:``
  - Basis: owning source requirement ID.
- **T2-0143** — p. 13; § 5.3 Narrow host allowlist; inherited bullet; mapping: ``PES-SEC-0001``
  - Verbatim: ``typed UI-to-worker messaging;``
  - Modal lead-in: ``[PES-SEC-0001] MAY permit only these host capabilities in the production product:``
  - Basis: owning source requirement ID.
- **T2-0144** — p. 13; § 5.3 Narrow host allowlist; inherited bullet; mapping: ``PES-SEC-0001``
  - Verbatim: ``memory allocation;``
  - Modal lead-in: ``[PES-SEC-0001] MAY permit only these host capabilities in the production product:``
  - Basis: owning source requirement ID.
- **T2-0145** — p. 13; § 5.3 Narrow host allowlist; inherited bullet; mapping: ``PES-SEC-0001``
  - Verbatim: ``simulator-controlled monotonic virtual time inputs;``
  - Modal lead-in: ``[PES-SEC-0001] MAY permit only these host capabilities in the production product:``
  - Basis: owning source requirement ID.
- **T2-0146** — p. 13; § 5.3 Narrow host allowlist; inherited bullet; mapping: ``PES-SEC-0001``
  - Verbatim: ``printing or local document export only when a later requirement approves it and no external resource is loaded.``
  - Modal lead-in: ``[PES-SEC-0001] MAY permit only these host capabilities in the production product:``
  - Basis: owning source requirement ID.
- **T2-0147** — p. 13; § 5.3 Narrow host allowlist; explicit; mapping: ``PES-SEC-0002``
  - Verbatim: ``[PES-SEC-0002] MUST expose file persistence as bounded document operations, not arbitrary path traversal, executable launch, shell access, device files, or general-purpose host filesystem access.``
  - Basis: owning source requirement ID.
- **T2-0148** — p. 13; § 5.3 Narrow host allowlist; explicit; mapping: ``PES-SEC-0003``
  - Verbatim: ``[PES-SEC-0003] MUST ensure typed UI-to-worker IPC carries domain messages only.``
  - Basis: owning source requirement ID.
- **T2-0149** — p. 13; § 5.3 Narrow host allowlist; explicit; mapping: ``PES-SEC-0003``
  - Verbatim: ``It shall not accept arbitrary code, URLs, shell strings, native method names, or generic transport descriptors.``
  - Basis: owning source requirement ID.
- **T2-0150** — p. 13; § 5.4 Production versus development boundary; explicit; mapping: ``PES-SEC-0005``
  - Verbatim: ``[PES-SEC-0005] MUST keep those development capabilities outside production dependency graphs, shipped bundles, runtime permissions, and user-reachable code.``
  - Basis: owning source requirement ID.
- **T2-0151** — p. 13; § 5.4 Production versus development boundary; explicit; mapping: ``PES-SEC-0006``
  - Verbatim: ``[PES-SEC-0006] MUST build the production classroom application without a local web server.``
  - Basis: owning source requirement ID.
- **T2-0152** — p. 13; § 5.4 Production versus development boundary; explicit; mapping: ``PES-SEC-0006``
  - Verbatim: ``Assets, workers, fonts, WASM, help, and examples shall be bundled and loaded locally without HTTP or WebSocket.``
  - Basis: owning source requirement ID.
- **T2-0153** — p. 13; § 5.4 Production versus development boundary; explicit; mapping: ``PES-SEC-0007``
  - Verbatim: ``[PES-SEC-0007] MUST enforce a production Content Security Policy with at least connect-src 'none' and default-deny restrictions on external scripts, styles, fonts, images, media, objects, frames, forms, manifests, base-URI changes, and unsolicited navigation.``
  - Basis: owning source requirement ID.

#### Page 14 — 14 statement(s)

- **T2-0154** — p. 14; § 5.4 Production versus development boundary; explicit; mapping: ``PES-SEC-0008``
  - Verbatim: ``[PES-SEC-0008] MUST NOT include a network updater inside the trusted product.``
  - Basis: owning source requirement ID.
- **T2-0155** — p. 14; § 5.4 Production versus development boundary; explicit; mapping: ``PES-SEC-0008``
  - Verbatim: ``If a future updater is approved, it must be a separately packaged, separately permissioned product absent from classroom builds and unable to be invoked by trusted simulator code.``
  - Basis: owning source requirement ID.
- **T2-0156** — p. 14; § 5.5 Threat and claim boundary; explicit; mapping: ``PES-SEC-0009``
  - Verbatim: ``[PES-SEC-0009] MUST claim that the unmodified shipped product has no physical-industrial communication code path or capability.``
  - Basis: owning source requirement ID.
- **T2-0157** — p. 14; § 5.5 Threat and claim boundary; explicit; mapping: ``PES-SEC-0009``
  - Verbatim: ``It shall not claim that a maliciously modified binary or compromised host operating system is metaphysically incapable of networking.``
  - Basis: owning source requirement ID.
- **T2-0158** — p. 14; § 5.5 Threat and claim boundary; explicit; mapping: ``PES-SEC-0010``
  - Verbatim: ``[PES-SEC-0010] MUST make zero-egress evidence process-scoped to the application and its child processes, while distinguishing unrelated host traffic.``
  - Basis: owning source requirement ID.
- **T2-0159** — p. 14; § 5.5 Threat and claim boundary; explicit; mapping: ``PES-SEC-0011``
  - Verbatim: ``[PES-SEC-0011] MUST fail release on attempted network syscalls or endpoint resolution even if a firewall blocks packets.``
  - Basis: owning source requirement ID.
- **T2-0160** — p. 14; § 5.6 Untrusted files and scripting; explicit; mapping: ``PES-SEC-0012``
  - Verbatim: ``[PES-SEC-0012] MUST treat every imported project, archive, CSV, JSON, library, scenario, image, or future script as untrusted input.``
  - Basis: owning source requirement ID.
- **T2-0161** — p. 14; § 5.6 Untrusted files and scripting; explicit; mapping: ``PES-SEC-0013``
  - Verbatim: ``[PES-SEC-0013] MUST apply schema validation, canonical path validation, archive traversal prevention, duplicate-entry detection, compression-ratio limits, uncompressed-size limits, file-count limits, nesting limits, string/array/object limits, image-dimension limits, and deterministic resource budgets.``
  - Basis: owning source requirement ID.
- **T2-0162** — p. 14; § 5.6 Untrusted files and scripting; explicit; mapping: ``PES-SEC-0014``
  - Verbatim: ``[PES-SEC-0014] MUST NOT execute code from a project, archive, library, scenario, HMI object, lesson, or sample.``
  - Basis: owning source requirement ID.
- **T2-0163** — p. 14; § 5.6 Untrusted files and scripting; explicit; mapping: ``PES-SEC-0015``
  - Verbatim: ``[PES-SEC-0015] MUST NOT use eval, Function constructors, dynamic native modules, arbitrary JavaScript, arbitrary WebAssembly, macros, shell commands, or executable embedded content.``
  - Basis: owning source requirement ID.
- **T2-0164** — p. 14; § 5.6 Untrusted files and scripting; explicit; mapping: ``PES-SEC-0016``
  - Verbatim: ``[PES-SEC-0016] MUST make any future HMI or assessment scripting a capability-limited original DSL or interpreter with deterministic execution, explicit resource limits, no host objects, no dynamic imports, no network, no filesystem, no process access, and no escape to general-purpose code.``
  - Basis: owning source requirement ID.
- **T2-0165** — p. 14; § 5.7 Release-blocking isolation proof; explicit; mapping: ``PES-ISO-0011``
  - Verbatim: ``[PES-ISO-0011] MUST make every isolation test release-blocking.``
  - Basis: owning source requirement ID.
- **T2-0166** — p. 14; § 5.7 Release-blocking isolation proof; explicit; mapping: ``PES-ISO-0012``
  - Verbatim: ``[PES-ISO-0012] MUST scan production dependency graphs, lockfiles, optional dependencies, aliases, native modules, dynamic imports, WASM imports, and packaged output for prohibited capabilities.``
  - Basis: owning source requirement ID.
- **T2-0167** — p. 14; § 5.7 Release-blocking isolation proof; explicit; mapping: ``PES-ISO-0013``
  - Verbatim: ``[PES-ISO-0013] MUST statically scan trusted and shipped source for forbidden browser, Node, native, FFI, subprocess, device, networking, and industrial APIs.``
  - Basis: owning source requirement ID.

#### Page 15 — 12 statement(s)

- **T2-0168** — p. 15; § 5.7 Release-blocking isolation proof; explicit; mapping: ``PES-ISO-0014``
  - Verbatim: ``[PES-ISO-0014] MUST inspect every semantic/runtime WASM module.``
  - Basis: owning source requirement ID.
- **T2-0169** — p. 15; § 5.7 Release-blocking isolation proof; explicit; mapping: ``PES-ISO-0015``
  - Verbatim: ``[PES-ISO-0015] MUST run the complete product and course suite with all network adapters disabled or removed.``
  - Basis: owning source requirement ID.
- **T2-0170** — p. 15; § 5.7 Release-blocking isolation proof; explicit; mapping: ``PES-ISO-0016``
  - Verbatim: ``[PES-ISO-0016] MUST run zero-egress and zero-attempt tests covering project creation, virtual addresses, discovery, compilation, virtual load, RUN/STOP, HMI, monitoring, watch, modify, force, trace, diagnostics, faults, lessons, grading, save, and export.``
  - Basis: owning source requirement ID.
- **T2-0171** — p. 15; § 5.7 Release-blocking isolation proof; explicit; mapping: ``PES-ISO-0017``
  - Verbatim: ``[PES-ISO-0017] MUST fuzz all user-text and address-bearing fields with loopback, private, public, multicast, broadcast, IPv6, hostnames, URLs, industrial-looking ports, UNC paths, device paths, and malformed endpoint strings.``
  - Basis: owning source requirement ID.
- **T2-0172** — p. 15; § 5.7 Release-blocking isolation proof; explicit; mapping: ``PES-ISO-0017``
  - Verbatim: ``All values shall remain inert data.``
  - Basis: owning source requirement ID.
- **T2-0173** — p. 15; § 5.7 Release-blocking isolation proof; explicit; mapping: ``PES-ISO-0018``
  - Verbatim: ``[PES-ISO-0018] MUST prove that device discovery results remain unchanged in the presence of a live LAN containing real or PLC-like devices.``
  - Basis: owning source requirement ID.
- **T2-0174** — p. 15; § 5.7 Release-blocking isolation proof; explicit; mapping: ``PES-ISO-0019``
  - Verbatim: ``[PES-ISO-0019] MUST prove at type, deserialization, reflection, and UI boundaries that Virtual Download accepts only VirtualControllerId.``
  - Basis: owning source requirement ID.
- **T2-0175** — p. 15; § 5.7 Release-blocking isolation proof; explicit; mapping: ``PES-ISO-0020``
  - Verbatim: ``[PES-ISO-0020] MUST prove every HMI binding resolves only through InternalTagBus.``
  - Basis: owning source requirement ID.
- **T2-0176** — p. 15; § 5.7 Release-blocking isolation proof; explicit; mapping: ``PES-ISO-0021``
  - Verbatim: ``[PES-ISO-0021] MUST prove exports contain no vendor project, firmware, load binary, deployable industrial payload, protocol frame, executable, or file directly accepted by a physical industrial tool.``
  - Basis: owning source requirement ID.
- **T2-0177** — p. 15; § 5.7 Release-blocking isolation proof; explicit; mapping: ``PES-ISO-0022``
  - Verbatim: ``[PES-ISO-0022] MUST retain machine-readable evidence for each isolation gate with artifact hash, test version, date, platform, result, and logs sufficient to reproduce the test.``
  - Basis: owning source requirement ID.
- **T2-0178** — p. 15; § 6.1 Independent implementation; explicit; mapping: ``PES-CRM-0001``
  - Verbatim: ``[PES-CRM-0001] MUST use original expression and independent implementation.``
  - Basis: owning source requirement ID.
- **T2-0179** — p. 15; § 6.1 Independent implementation; explicit; mapping: ``PES-CRM-0002``
  - Verbatim: ``[PES-CRM-0002] MUST treat educational purpose as the mission, not permission to copy and not a substitute for legal analysis.``
  - Basis: owning source requirement ID.

#### Page 16 — 18 statement(s)

- **T2-0180** — p. 16; § 6.1 Independent implementation; explicit; mapping: ``PES-CRM-0004``
  - Verbatim: ``[PES-CRM-0004] MUST NOT copy Siemens screens, layout composition, icons, help prose, diagnostic prose or numbers, artwork, device illustrations, completion databases, project formats, compiler components, firmware behavior, or proprietary algorithms.``
  - Basis: owning source requirement ID.
- **T2-0181** — p. 16; § 6.1 Independent implementation; explicit; mapping: ``PES-CRM-0005``
  - Verbatim: ``[PES-CRM-0005] MUST use original names, event codes, visual language, device identities, project structures, schemas, source representations, sample projects, and user documentation.``
  - Basis: owning source requirement ID.
- **T2-0182** — p. 16; § 6.2 IP classification; explicit; mapping: ``PES-CRM-0007``
  - Verbatim: ``Every externally inspired requirement shall be classified before implementation:``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0183** — p. 16; § 6.2 IP classification; inherited table row; mapping: ``PES-CRM-0001``
  - Verbatim: ``1 | Functional behavior | Independently implement``
  - Modal lead-in: ``Every externally inspired requirement shall be classified before implementation:``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0184** — p. 16; § 6.2 IP classification; inherited table row; mapping: ``PES-CRM-0008``
  - Verbatim: ``2 | Industry or IEC convention | Implement from lawfully licensed standards or public behavior``
  - Modal lead-in: ``Every externally inspired requirement shall be classified before implementation:``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0185** — p. 16; § 6.2 IP classification; inherited table row; mapping: ``PES-CRM-0003``, ``PES-CRM-0004``, ``PES-CRM-0005``
  - Verbatim: ``3 | Workflow behavior | Preserve useful workflow logic; redesign visuals and expression``
  - Modal lead-in: ``Every externally inspired requirement shall be classified before implementation:``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0186** — p. 16; § 6.2 IP classification; inherited table row; mapping: ``PES-CRM-0004``, ``PES-CRM-0005``
  - Verbatim: ``4 | Vendor-specific expression | Redesign``
  - Modal lead-in: ``Every externally inspired requirement shall be classified before implementation:``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0187** — p. 16; § 6.2 IP classification; inherited table row; mapping: ``PES-CRM-0012``, ``PES-CRM-0013``
  - Verbatim: ``5 | Branding or trademark | Replace or exclude``
  - Modal lead-in: ``Every externally inspired requirement shall be classified before implementation:``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0188** — p. 16; § 6.2 IP classification; inherited table row; mapping: ``PES-CRM-0001``, ``PES-CRM-0004``, ``PES-CRM-0005``
  - Verbatim: ``6 | Proprietary technology | Create an original simulated equivalent``
  - Modal lead-in: ``Every externally inspired requirement shall be classified before implementation:``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0189** — p. 16; § 6.2 IP classification; inherited table row; mapping: ``PES-SCP-0010``
  - Verbatim: ``7 | Patent or licensing concern | BLOCKED pending focused review``
  - Modal lead-in: ``Every externally inspired requirement shall be classified before implementation:``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0190** — p. 16; § 6.2 IP classification; inherited table row; mapping: ``PES-CRM-0006``, ``PES-SCP-0010``
  - Verbatim: ``8 | Uncertain or high-risk | BLOCKED pending professional legal review``
  - Modal lead-in: ``Every externally inspired requirement shall be classified before implementation:``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0191** — p. 16; § 6.2 IP classification; inherited table row; mapping: ``PES-SCP-0002``, ``PES-ISO-0001``, ``PES-ISO-0002``
  - Verbatim: ``9 | Physical industrial communication | Permanently EXCLUDED``
  - Modal lead-in: ``Every externally inspired requirement shall be classified before implementation:``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0192** — p. 16; § 6.2 IP classification; explicit; mapping: ``PES-CRM-0006``
  - Verbatim: ``[PES-CRM-0006] MUST default an unclassified or uncertain item to Class 8, not "probably permitted."``
  - Basis: owning source requirement ID.
- **T2-0193** — p. 16; § 6.2 IP classification; explicit; mapping: ``PES-CRM-0007``
  - Verbatim: ``[PES-CRM-0007] MUST NOT begin implementation of a research-derived behavior until its requirement record contains an IP classification and disposition.``
  - Basis: owning source requirement ID.
- **T2-0194** — p. 16; § 6.3 Permitted and forbidden sources; explicit; mapping: ``PES-CRM-0009``
  - Verbatim: ``[PES-CRM-0009] MUST NOT use:``
  - Basis: owning source requirement ID.
- **T2-0195** — p. 16; § 6.3 Permitted and forbidden sources; inherited bullet; mapping: ``PES-CRM-0009``
  - Verbatim: ``Siemens source code, leaked code, leaked manuals, partner-only material, or confidential training material;``
  - Modal lead-in: ``[PES-CRM-0009] MUST NOT use:``
  - Basis: owning source requirement ID.
- **T2-0196** — p. 16; § 6.3 Permitted and forbidden sources; inherited bullet; mapping: ``PES-CRM-0009``
  - Verbatim: ``decompiled or disassembled output;``
  - Modal lead-in: ``[PES-CRM-0009] MUST NOT use:``
  - Basis: owning source requirement ID.
- **T2-0197** — p. 16; § 6.3 Permitted and forbidden sources; inherited bullet; mapping: ``PES-CRM-0009``
  - Verbatim: ``executable resources, extracted icons, resource packages, or memory scraping;``
  - Modal lead-in: ``[PES-CRM-0009] MUST NOT use:``
  - Basis: owning source requirement ID.

#### Page 17 — 20 statement(s)

- **T2-0198** — p. 17; § 6.3 Permitted and forbidden sources; inherited bullet; mapping: ``PES-CRM-0009``
  - Verbatim: ``protocol captures intended to reproduce vendor communications;``
  - Modal lead-in: ``[PES-CRM-0009] MUST NOT use:``
  - Basis: owning source requirement ID.
- **T2-0199** — p. 17; § 6.3 Permitted and forbidden sources; inherited bullet; mapping: ``PES-CRM-0009``
  - Verbatim: ``encrypted project-format cracking;``
  - Modal lead-in: ``[PES-CRM-0009] MUST NOT use:``
  - Basis: owning source requirement ID.
- **T2-0200** — p. 17; § 6.3 Permitted and forbidden sources; inherited bullet; mapping: ``PES-CRM-0009``
  - Verbatim: ``pirated software, license bypass, access-control circumvention, or API hooking;``
  - Modal lead-in: ``[PES-CRM-0009] MUST NOT use:``
  - Basis: owning source requirement ID.
- **T2-0201** — p. 17; § 6.3 Permitted and forbidden sources; inherited bullet; mapping: ``PES-CRM-0009``
  - Verbatim: ``screenshots, manual diagrams, copied tables, copied hardware illustrations, or copied diagnostic text as implementation assets.``
  - Modal lead-in: ``[PES-CRM-0009] MUST NOT use:``
  - Basis: owning source requirement ID.
- **T2-0202** — p. 17; § 6.3 Permitted and forbidden sources; explicit; mapping: ``PES-CRM-0010``
  - Verbatim: ``[PES-CRM-0010] MUST prohibit observation of an installed TIA Portal product for implementation verification until counsel reviews the applicable license terms and approves a written observation procedure.``
  - Basis: owning source requirement ID.
- **T2-0203** — p. 17; § 6.3 Permitted and forbidden sources; explicit; mapping: ``PES-CRM-0011``
  - Verbatim: ``[PES-CRM-0011] MUST keep screenshots and vendor assets out of production source, design files, tickets, code-generation prompts, training corpora, mockups, and asset pipelines unless counsel approves a quarantined evidence process.``
  - Basis: owning source requirement ID.
- **T2-0204** — p. 17; § 6.3 Permitted and forbidden sources; explicit; mapping: ``PES-CRM-0011``
  - Verbatim: ``Quarantined evidence shall never be shipped.``
  - Basis: owning source requirement ID.
- **T2-0205** — p. 17; § 6.4 Trademark, trade dress, and public language; explicit; mapping: ``PES-CRM-0012``
  - Verbatim: ``[PES-CRM-0012] MUST NOT use Siemens, SIMATIC, TIA Portal, S7, WinCC, or PLCSIM marks as product identity, catalog identity, installer branding, repository branding, splash-screen branding, store listing, domain name, or implied affiliation.``
  - Basis: owning source requirement ID.
- **T2-0206** — p. 17; § 6.4 Trademark, trade dress, and public language; explicit; mapping: ``PES-CRM-0013``
  - Verbatim: ``[PES-CRM-0013] MUST NOT copy or sample a Siemens color system, icon silhouette family, device illustration style, typography, spacing system, screen composition, or overall trade dress.``
  - Basis: owning source requirement ID.
- **T2-0207** — p. 17; § 6.4 Trademark, trade dress, and public language; explicit; mapping: ``PES-CRM-0014``
  - Verbatim: ``[PES-CRM-0014] MUST hold every public comparative statement mentioning Siemens or TIA Portal as BLOCKED until trademark counsel approves the exact wording and notices.``
  - Basis: owning source requirement ID.
- **T2-0208** — p. 17; § 6.4 Trademark, trade dress, and public language; explicit; mapping: ``PES-CRM-0015``
  - Verbatim: ``[PES-CRM-0015] MUST treat the working title in this directive as descriptive only.``
  - Basis: owning source requirement ID.
- **T2-0209** — p. 17; § 6.5 Evidence and contamination control; explicit; mapping: ``PES-CRM-0016``
  - Verbatim: ``[PES-CRM-0016] MUST create CLEAN_ROOM_POLICY.md before feature implementation.``
  - Basis: owning source requirement ID.
- **T2-0210** — p. 17; § 6.5 Evidence and contamination control; explicit; mapping: ``PES-CRM-0017``
  - Verbatim: ``[PES-CRM-0017] MUST maintain a requirement evidence register with:``
  - Basis: owning source requirement ID.
- **T2-0211** — p. 17; § 6.5 Evidence and contamination control; inherited bullet; mapping: ``PES-CRM-0017``
  - Verbatim: ``requirement ID;``
  - Modal lead-in: ``[PES-CRM-0017] MUST maintain a requirement evidence register with:``
  - Basis: owning source requirement ID.
- **T2-0212** — p. 17; § 6.5 Evidence and contamination control; inherited bullet; mapping: ``PES-CRM-0017``
  - Verbatim: ``paraphrased observed behavior;``
  - Modal lead-in: ``[PES-CRM-0017] MUST maintain a requirement evidence register with:``
  - Basis: owning source requirement ID.
- **T2-0213** — p. 17; § 6.5 Evidence and contamination control; inherited bullet; mapping: ``PES-CRM-0017``
  - Verbatim: ``source title, publisher, version/date, durable location, and access date;``
  - Modal lead-in: ``[PES-CRM-0017] MUST maintain a requirement evidence register with:``
  - Basis: owning source requirement ID.
- **T2-0214** — p. 17; § 6.5 Evidence and contamination control; inherited bullet; mapping: ``PES-CRM-0017``
  - Verbatim: ``report classification;``
  - Modal lead-in: ``[PES-CRM-0017] MUST maintain a requirement evidence register with:``
  - Basis: owning source requirement ID.
- **T2-0215** — p. 17; § 6.5 Evidence and contamination control; inherited bullet; mapping: ``PES-CRM-0017``
  - Verbatim: ``IP class and disposition;``
  - Modal lead-in: ``[PES-CRM-0017] MUST maintain a requirement evidence register with:``
  - Basis: owning source requirement ID.
- **T2-0216** — p. 17; § 6.5 Evidence and contamination control; inherited bullet; mapping: ``PES-CRM-0017``
  - Verbatim: ``simulator-owned implementation requirement;``
  - Modal lead-in: ``[PES-CRM-0017] MUST maintain a requirement evidence register with:``
  - Basis: owning source requirement ID.
- **T2-0217** — p. 17; § 6.5 Evidence and contamination control; inherited bullet; mapping: ``PES-CRM-0017``
  - Verbatim: ``forbidden shortcut;``
  - Modal lead-in: ``[PES-CRM-0017] MUST maintain a requirement evidence register with:``
  - Basis: owning source requirement ID.

#### Page 18 — 22 statement(s)

- **T2-0218** — p. 18; § 6.5 Evidence and contamination control; inherited bullet; mapping: ``PES-CRM-0017``
  - Verbatim: ``author;``
  - Modal lead-in: ``[PES-CRM-0017] MUST maintain a requirement evidence register with:``
  - Basis: owning source requirement ID.
- **T2-0219** — p. 18; § 6.5 Evidence and contamination control; inherited bullet; mapping: ``PES-CRM-0017``
  - Verbatim: ``reviewer;``
  - Modal lead-in: ``[PES-CRM-0017] MUST maintain a requirement evidence register with:``
  - Basis: owning source requirement ID.
- **T2-0220** — p. 18; § 6.5 Evidence and contamination control; inherited bullet; mapping: ``PES-CRM-0017``
  - Verbatim: ``review status and date;``
  - Modal lead-in: ``[PES-CRM-0017] MUST maintain a requirement evidence register with:``
  - Basis: owning source requirement ID.
- **T2-0221** — p. 18; § 6.5 Evidence and contamination control; inherited bullet; mapping: ``PES-CRM-0017``
  - Verbatim: ``implementation component;``
  - Modal lead-in: ``[PES-CRM-0017] MUST maintain a requirement evidence register with:``
  - Basis: owning source requirement ID.
- **T2-0222** — p. 18; § 6.5 Evidence and contamination control; inherited bullet; mapping: ``PES-CRM-0017``
  - Verbatim: ``verification IDs.``
  - Modal lead-in: ``[PES-CRM-0017] MUST maintain a requirement evidence register with:``
  - Basis: owning source requirement ID.
- **T2-0223** — p. 18; § 6.5 Evidence and contamination control; explicit; mapping: ``PES-CRM-0018``
  - Verbatim: ``[PES-CRM-0018] MUST quarantine a contribution suspected of contamination.``
  - Basis: owning source requirement ID.
- **T2-0224** — p. 18; § 6.5 Evidence and contamination control; explicit; mapping: ``PES-CRM-0018``
  - Verbatim: ``It shall not enter builds, prompts, generated assets, or derived work until reviewed.``
  - Basis: owning source requirement ID.
- **T2-0225** — p. 18; § 6.5 Evidence and contamination control; explicit; mapping: ``PES-CRM-0019``
  - Verbatim: ``[PES-CRM-0019] MUST perform a clean rewrite without reusing tainted code, prose, assets, naming, layouts, or extracted structure when contamination is confirmed.``
  - Basis: owning source requirement ID.
- **T2-0226** — p. 18; § 6.5 Evidence and contamination control; explicit; mapping: ``PES-CRM-0020``
  - Verbatim: ``[PES-CRM-0020] MUST require contributor attestation that no forbidden source, asset, reverse-engineering output, protocol capture, or confidential material was used.``
  - Basis: owning source requirement ID.
- **T2-0227** — p. 18; § 6.6 Asset and dependency provenance; explicit; mapping: ``PES-CRM-0021``
  - Verbatim: ``[PES-CRM-0021] MUST register every shipped image, icon, font, sound, animation, template, sample project, translation, and other non-code asset with:``
  - Basis: owning source requirement ID.
- **T2-0228** — p. 18; § 6.6 Asset and dependency provenance; inherited bullet; mapping: ``PES-CRM-0021``
  - Verbatim: ``asset ID;``
  - Modal lead-in: ``[PES-CRM-0021] MUST register every shipped image, icon, font, sound, animation, template, sample project, translation, and other non-code asset with:``
  - Basis: owning source requirement ID.
- **T2-0229** — p. 18; § 6.6 Asset and dependency provenance; inherited bullet; mapping: ``PES-CRM-0021``
  - Verbatim: ``author/source;``
  - Modal lead-in: ``[PES-CRM-0021] MUST register every shipped image, icon, font, sound, animation, template, sample project, translation, and other non-code asset with:``
  - Basis: owning source requirement ID.
- **T2-0230** — p. 18; § 6.6 Asset and dependency provenance; inherited bullet; mapping: ``PES-CRM-0021``
  - Verbatim: ``license and evidence location;``
  - Modal lead-in: ``[PES-CRM-0021] MUST register every shipped image, icon, font, sound, animation, template, sample project, translation, and other non-code asset with:``
  - Basis: owning source requirement ID.
- **T2-0231** — p. 18; § 6.6 Asset and dependency provenance; inherited bullet; mapping: ``PES-CRM-0021``
  - Verbatim: ``created date;``
  - Modal lead-in: ``[PES-CRM-0021] MUST register every shipped image, icon, font, sound, animation, template, sample project, translation, and other non-code asset with:``
  - Basis: owning source requirement ID.
- **T2-0232** — p. 18; § 6.6 Asset and dependency provenance; inherited bullet; mapping: ``PES-CRM-0021``
  - Verbatim: ``hash algorithm and original hash;``
  - Modal lead-in: ``[PES-CRM-0021] MUST register every shipped image, icon, font, sound, animation, template, sample project, translation, and other non-code asset with:``
  - Basis: owning source requirement ID.
- **T2-0233** — p. 18; § 6.6 Asset and dependency provenance; inherited bullet; mapping: ``PES-CRM-0021``
  - Verbatim: ``derivative chain and modifications;``
  - Modal lead-in: ``[PES-CRM-0021] MUST register every shipped image, icon, font, sound, animation, template, sample project, translation, and other non-code asset with:``
  - Basis: owning source requirement ID.
- **T2-0234** — p. 18; § 6.6 Asset and dependency provenance; inherited bullet; mapping: ``PES-CRM-0021``
  - Verbatim: ``generated-asset disclosure where applicable;``
  - Modal lead-in: ``[PES-CRM-0021] MUST register every shipped image, icon, font, sound, animation, template, sample project, translation, and other non-code asset with:``
  - Basis: owning source requirement ID.
- **T2-0235** — p. 18; § 6.6 Asset and dependency provenance; inherited bullet; mapping: ``PES-CRM-0021``
  - Verbatim: ``reviewer, review status, and approval date.``
  - Modal lead-in: ``[PES-CRM-0021] MUST register every shipped image, icon, font, sound, animation, template, sample project, translation, and other non-code asset with:``
  - Basis: owning source requirement ID.
- **T2-0236** — p. 18; § 6.6 Asset and dependency provenance; explicit; mapping: ``PES-CRM-0022``
  - Verbatim: ``[PES-CRM-0022] MUST reject unregistered or unapproved assets in CI.``
  - Basis: owning source requirement ID.
- **T2-0237** — p. 18; § 6.6 Asset and dependency provenance; explicit; mapping: ``PES-CRM-0023``
  - Verbatim: ``[PES-CRM-0023] MUST NOT trace screenshots or icons, redraw vendor artwork, recolor vendor assets, or sample vendor branding as proof of originality.``
  - Basis: owning source requirement ID.
- **T2-0238** — p. 18; § 6.6 Asset and dependency provenance; explicit; mapping: ``PES-CRM-0024``
  - Verbatim: ``[PES-CRM-0024] MUST generate an SBOM for release artifacts and review direct, transitive, optional, native, font, and asset licenses.``
  - Basis: owning source requirement ID.
- **T2-0239** — p. 18; § 6.6 Asset and dependency provenance; explicit; mapping: ``PES-CRM-0025``
  - Verbatim: ``[PES-CRM-0025] MUST block dependencies whose license obligations are incompatible with the intended distribution or cannot be satisfied and documented.``
  - Basis: owning source requirement ID.

#### Page 19 — 11 statement(s)

- **T2-0240** — p. 19; § 6.7 Original native file boundary; explicit; mapping: ``PES-PRJ-0001``
  - Verbatim: ``[PES-PRJ-0001] MUST use only simulator-native, brand-neutral project and archive formats.``
  - Basis: owning source requirement ID.
- **T2-0241** — p. 19; § 6.7 Original native file boundary; explicit; mapping: ``PES-PRJ-0002``
  - Verbatim: ``[PES-PRJ-0002] MUST use the provisional internal extensions .vlabproj for a project package and .vlabarchive for an archive until an approved product-name decision replaces them.``
  - Basis: owning source requirement ID.
- **T2-0242** — p. 19; § 6.7 Original native file boundary; explicit; mapping: ``PES-PRJ-0003``
  - Verbatim: ``[PES-PRJ-0003] MUST make a project package a documented, versioned, non-executable container of canonical UTF-8 data and simulator-owned binary-neutral assets.``
  - Basis: owning source requirement ID.
- **T2-0243** — p. 19; § 6.7 Original native file boundary; explicit; mapping: ``PES-PRJ-0004``
  - Verbatim: ``[PES-PRJ-0004] MUST place a manifest in every project/archive containing schema version, pinned TrainingProfile ID and version, object-index version, required capabilities, file inventory, SHA-256 hashes, creation application version, and migration history.``
  - Basis: owning source requirement ID.
- **T2-0244** — p. 19; § 6.7 Original native file boundary; explicit; mapping: ``PES-PRJ-0005``
  - Verbatim: ``[PES-PRJ-0005] MUST make project integrity failures visible and fail closed.``
  - Basis: owning source requirement ID.
- **T2-0245** — p. 19; § 6.7 Original native file boundary; explicit; mapping: ``PES-PRJ-0005``
  - Verbatim: ``It shall not silently discard unknown, corrupt, oversized, or hash-mismatched content.``
  - Basis: owning source requirement ID.
- **T2-0246** — p. 19; § 6.7 Original native file boundary; explicit; mapping: ``PES-PRJ-0006``
  - Verbatim: ``[PES-PRJ-0006] MUST NOT use .apXX, .zapXX, a Siemens library format, PLCopen XML, vendor source export, or another real-tool format unless a later separately researched and legally approved directive explicitly adds a non-physical interoperability feature.``
  - Basis: owning source requirement ID.
- **T2-0247** — p. 19; § 6.7 Original native file boundary; explicit; mapping: ``PES-PRJ-0007``
  - Verbatim: ``[PES-PRJ-0007] MUST distinguish simulator-native CSV/JSON interchange from vendor or physical deployment.``
  - Basis: owning source requirement ID.
- **T2-0248** — p. 19; § 6.7 Original native file boundary; explicit; mapping: ``PES-PRJ-0007``
  - Verbatim: ``Native exports shall contain no executable code and shall be documented as simulator-only.``
  - Basis: owning source requirement ID.
- **T2-0249** — p. 19; § 7.1 Trust zones; explicit table row; mapping: ``PES-SEC-0012``, ``PES-SEC-0013``, ``PES-SEC-0014``
  - Verbatim: ``Untrusted content | imported projects, archives, CSV/JSON, images, future libraries/scenarios/scripts | Validate, limit, never execute``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0250** — p. 19; § 7.1 Trust zones; explicit table row; mapping: ``PES-SEC-0004``, ``PES-SEC-0005``
  - Verbatim: ``Development environment | package managers, compilers, test servers, CI tools | May use development capabilities but shall not enter production``
  - Basis: materially equivalent numbered statement elsewhere in the source.

#### Page 20 — 20 statement(s)

- **T2-0251** — p. 20; § 7.1 Trust zones; explicit; mapping: ``PES-SEC-0017``
  - Verbatim: ``[PES-SEC-0017] MUST document the trust boundary in SECURITY_INVARIANTS.md and keep package ownership aligned to it.``
  - Basis: owning source requirement ID.
- **T2-0252** — p. 20; § 7.1 Trust zones; explicit; mapping: ``PES-SEC-0018``
  - Verbatim: ``[PES-SEC-0018] MUST keep persistence and presentation code from bypassing domain commands or writing trusted semantic state directly.``
  - Basis: owning source requirement ID.
- **T2-0253** — p. 20; § 7.1 Trust zones; explicit; mapping: ``PES-SEC-0019``
  - Verbatim: ``[PES-SEC-0019] MUST use explicit serialization schemas at trust boundaries.``
  - Basis: owning source requirement ID.
- **T2-0254** — p. 20; § 7.1 Trust zones; explicit; mapping: ``PES-SEC-0019``
  - Verbatim: ``"Any," untagged arbitrary maps, dynamic class loading, and reflection-based invocation shall not cross into the semantic core.``
  - Basis: owning source requirement ID.
- **T2-0255** — p. 20; § 7.1 Trust zones; explicit; mapping: ``PES-SEC-0020``
  - Verbatim: ``[PES-SEC-0020] MUST validate message kind, schema version, payload size, object IDs, capability authorization, and state preconditions before a worker or core service executes a command.``
  - Basis: owning source requirement ID.
- **T2-0256** — p. 20; § 7.2 Teacher/student data boundary; explicit; mapping: ``PES-TCH-0001``
  - Verbatim: ``[PES-TCH-0001] MUST keep teacher-authored answer keys, hidden faults, checkpoints, and scoring rules logically separate from student-visible project state.``
  - Basis: owning source requirement ID.
- **T2-0257** — p. 20; § 7.2 Teacher/student data boundary; explicit; mapping: ``PES-TCH-0002``
  - Verbatim: ``[PES-TCH-0002] MUST be honest that an offline local file cannot provide absolute secrecy against a student with full filesystem or process access.``
  - Basis: owning source requirement ID.
- **T2-0258** — p. 20; § 7.2 Teacher/student data boundary; explicit; mapping: ``PES-TCH-0002``
  - Verbatim: ``It shall provide role-appropriate UI separation, protected packaging where useful, integrity checking, and audit evidence without claiming cryptographic impossibility unless a later design proves it.``
  - Basis: owning source requirement ID.
- **T2-0259** — p. 20; § 7.2 Teacher/student data boundary; explicit; mapping: ``PES-TCH-0003``
  - Verbatim: ``[PES-TCH-0003] MUST store student identity minimally.``
  - Basis: owning source requirement ID.
- **T2-0260** — p. 20; § 7.2 Teacher/student data boundary; explicit; mapping: ``PES-TCH-0003``
  - Verbatim: ``The default classroom model shall support local pseudonymous student IDs and shall not require names, email addresses, cloud accounts, or telemetry.``
  - Basis: owning source requirement ID.
- **T2-0261** — p. 20; § 7.2 Teacher/student data boundary; explicit; mapping: ``PES-TCH-0004``
  - Verbatim: ``[PES-TCH-0004] MUST let teachers configure local audit-log retention and export.``
  - Basis: owning source requirement ID.
- **T2-0262** — p. 20; § 7.2 Teacher/student data boundary; explicit; mapping: ``PES-TCH-0004``
  - Verbatim: ``Later phases shall define exact defaults and privacy behavior before Teacher Mode is released.``
  - Basis: owning source requirement ID.
- **T2-0263** — p. 20; § 7.2 Teacher/student data boundary; explicit; mapping: ``PES-TCH-0005``
  - Verbatim: ``[PES-TCH-0005] MUST NOT transmit grades, logs, project files, or identifiers outside the local product.``
  - Basis: owning source requirement ID.
- **T2-0264** — p. 20; § 7.3 Security acceptance posture; explicit; mapping: ``PES-SEC-0021``
  - Verbatim: ``[PES-SEC-0021] MUST fuzz every parser and deserializer that accepts untrusted content.``
  - Basis: owning source requirement ID.
- **T2-0265** — p. 20; § 7.3 Security acceptance posture; explicit; mapping: ``PES-SEC-0022``
  - Verbatim: ``[PES-SEC-0022] MUST make parse, validation, and migration failures structured and recoverable.``
  - Basis: owning source requirement ID.
- **T2-0266** — p. 20; § 7.3 Security acceptance posture; explicit; mapping: ``PES-SEC-0022``
  - Verbatim: ``Catch-and-return-success is forbidden.``
  - Basis: owning source requirement ID.
- **T2-0267** — p. 20; § 7.3 Security acceptance posture; explicit; mapping: ``PES-SEC-0023``
  - Verbatim: ``[PES-SEC-0023] MUST keep resource use deterministic or explicitly bounded.``
  - Basis: owning source requirement ID.
- **T2-0268** — p. 20; § 7.3 Security acceptance posture; explicit; mapping: ``PES-SEC-0023``
  - Verbatim: ``A project shall not allocate unbounded memory, unbounded recursion, unbounded event queues, or unbounded archive expansion.``
  - Basis: owning source requirement ID.
- **T2-0269** — p. 20; § 7.3 Security acceptance posture; explicit; mapping: ``PES-SEC-0024``
  - Verbatim: ``[PES-SEC-0024] MUST disable recursion unless a later verified TrainingProfile explicitly enables and constrains it.``
  - Basis: owning source requirement ID.
- **T2-0270** — p. 20; § 7.3 Security acceptance posture; explicit; mapping: ``PES-SEC-0025``
  - Verbatim: ``[PES-SEC-0025] MUST record security-relevant changes to CSP, package trust boundaries, import/export surfaces, worker IPC, file formats, or scripting in an ADR and threat-model update.``
  - Basis: owning source requirement ID.

#### Page 21 — 8 statement(s)

- **T2-0271** — p. 21; § 8.1 System topology; explicit; mapping: ``PES-ARC-0001``
  - Verbatim: ``[PES-ARC-0001] MUST enforce dependency direction.``
  - Basis: owning source requirement ID.
- **T2-0272** — p. 21; § 8.1 System topology; explicit; mapping: ``PES-ARC-0001``
  - Verbatim: ``UI code shall not contain authoritative PLC semantics; runtime code shall not parse editor layout; Teacher Mode shall not bypass commands; persistence shall not manufacture valid state.``
  - Basis: owning source requirement ID.
- **T2-0273** — p. 21; § 8.1 System topology; explicit; mapping: ``PES-ARC-0002``
  - Verbatim: ``[PES-ARC-0002] MUST keep the semantic core platform-neutral and deterministic.``
  - Basis: owning source requirement ID.
- **T2-0274** — p. 21; § 8.1 System topology; explicit; mapping: ``PES-ARC-0003``
  - Verbatim: ``[PES-ARC-0003] MUST make every extension point domain-specific.``
  - Basis: owning source requirement ID.
- **T2-0275** — p. 21; § 8.2 Stable identity and project graph; explicit; mapping: ``PES-ARC-0004``
  - Verbatim: ``[PES-ARC-0004] MUST represent every semantically referenceable project, hardware, network, language, HMI, library, process, lesson, scenario, assessment, diagnostic source, and runtime target object with an immutable UUID.``
  - Basis: owning source requirement ID.
- **T2-0276** — p. 21; § 8.2 Stable identity and project graph; explicit; mapping: ``PES-ARC-0005``
  - Verbatim: ``[PES-ARC-0005] MUST use RFC 9562 UUID version 4 by default for newly created objects.``
  - Basis: owning source requirement ID.
- **T2-0277** — p. 21; § 8.2 Stable identity and project graph; explicit; mapping: ``PES-ARC-0005``
  - Verbatim: ``Display names, addresses, paths, array positions, block numbers, and source coordinates shall not serve as identity.``
  - Basis: owning source requirement ID.
- **T2-0278** — p. 21; § 8.2 Stable identity and project graph; explicit; mapping: ``PES-ARC-0006``
  - Verbatim: ``[PES-ARC-0006] MUST preserve UUID on rename, move, readdress, regroup, interface-compatible edit, and undo restoration.``
  - Basis: owning source requirement ID.

#### Page 22 — 22 statement(s)

- **T2-0279** — p. 22; § 8.2 Stable identity and project graph; explicit; mapping: ``PES-ARC-0007``
  - Verbatim: ``[PES-ARC-0007] MUST create a new UUID for copy, template instantiation when independent, and imported objects intentionally duplicated as new objects.``
  - Basis: owning source requirement ID.
- **T2-0280** — p. 22; § 8.2 Stable identity and project graph; explicit; mapping: ``PES-ARC-0008``
  - Verbatim: ``[PES-ARC-0008] MUST retain a tombstone for deleted referenced objects for as long as a live reference, undo record, migration record, diagnostic, audit event, or snapshot requires it.``
  - Basis: owning source requirement ID.
- **T2-0281** — p. 22; § 8.2 Stable identity and project graph; explicit; mapping: ``PES-ARC-0009``
  - Verbatim: ``[PES-ARC-0009] MUST represent unresolved references explicitly with the target UUID and reference kind.``
  - Basis: owning source requirement ID.
- **T2-0282** — p. 22; § 8.2 Stable identity and project graph; explicit; mapping: ``PES-ARC-0009``
  - Verbatim: ``Deletion shall not silently erase or retarget usages.``
  - Basis: owning source requirement ID.
- **T2-0283** — p. 22; § 8.2 Stable identity and project graph; explicit; mapping: ``PES-ARC-0010``
  - Verbatim: ``[PES-ARC-0010] MUST detect UUID collision on import.``
  - Basis: owning source requirement ID.
- **T2-0284** — p. 22; § 8.2 Stable identity and project graph; explicit; mapping: ``PES-ARC-0010``
  - Verbatim: ``It shall reject ambiguous merge or perform an explicit, fully traced remap only when the import operation is defined to create independent objects.``
  - Basis: owning source requirement ID.
- **T2-0285** — p. 22; § 8.2 Stable identity and project graph; explicit; mapping: ``PES-ARC-0011``
  - Verbatim: ``[PES-ARC-0011] MUST maintain typed dependency edges and source/editor locations sufficient for where-used, caller/callee, type/DB usage, HMI binding, hardware-to-tag mapping, unresolved-reference filtering, and diagnostic navigation.``
  - Basis: owning source requirement ID.
- **T2-0286** — p. 22; § 8.3 Command, transaction, event, and audit model; explicit; mapping: ``PES-ARC-0012``
  - Verbatim: ``Every meaningful mutation shall be a domain command.``
  - Basis: materially equivalent numbered statement elsewhere in the source.
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
- **T2-0295** — p. 22; § 8.3 Command, transaction, event, and audit model; explicit; mapping: ``PES-ARC-0012``
  - Verbatim: ``[PES-ARC-0012] MUST route create, rename, delete, restore, move, copy, retype, bind, connect, disconnect, configure, compile request, load request, CPU state change, modify, force, fault, reset, lesson action, and migration through typed domain commands or explicitly read-only queries.``
  - Basis: owning source requirement ID.
- **T2-0296** — p. 22; § 8.3 Command, transaction, event, and audit model; explicit; mapping: ``PES-ARC-0013``
  - Verbatim: ``[PES-ARC-0013] MUST make commands atomic with respect to their declared transaction boundary.``
  - Basis: owning source requirement ID.
- **T2-0297** — p. 22; § 8.3 Command, transaction, event, and audit model; explicit; mapping: ``PES-ARC-0013``
  - Verbatim: ``Failure shall leave either the previous valid state or a separately modeled unresolved/invalid engineering state, never a half-applied hidden mutation.``
  - Basis: owning source requirement ID.
- **T2-0298** — p. 22; § 8.3 Command, transaction, event, and audit model; explicit; mapping: ``PES-ARC-0014``
  - Verbatim: ``[PES-ARC-0014] MUST make undo/redo use command/event semantics and restore exact stable identity where the original object is restored.``
  - Basis: owning source requirement ID.
- **T2-0299** — p. 22; § 8.3 Command, transaction, event, and audit model; explicit; mapping: ``PES-ARC-0015``
  - Verbatim: ``[PES-ARC-0015] MUST record deterministic event ordering, affected object IDs, before/after hashes, and command provenance sufficient for crash recovery, replay, Teacher Mode audit, and testing.``
  - Basis: owning source requirement ID.
- **T2-0300** — p. 22; § 8.3 Command, transaction, event, and audit model; explicit; mapping: ``PES-ARC-0016``
  - Verbatim: ``[PES-ARC-0016] MUST NOT let UI components write domain objects directly or let a lesson mutate serialized files behind the domain model.``
  - Basis: owning source requirement ID.

#### Page 23 — 13 statement(s)

- **T2-0301** — p. 23; § 8.4 Canonical type system and semantic editors; explicit; mapping: ``PES-TYP-0001``
  - Verbatim: ``[PES-TYP-0001] MUST create one canonical recursive type system shared by tags, DBs, block interfaces, LAD, FBD, SCL, addresses, runtime memory, watch/modify/force, trace, HMI bindings, assessment expressions, and profiles.``
  - Basis: owning source requirement ID.
- **T2-0302** — p. 23; § 8.4 Canonical type system and semantic editors; explicit; mapping: ``PES-TYP-0002``
  - Verbatim: ``[PES-TYP-0002] MUST keep named-type identity distinct from structural shape and give type members stable identity.``
  - Basis: owning source requirement ID.
- **T2-0303** — p. 23; § 8.4 Canonical type system and semantic editors; explicit; mapping: ``PES-ARC-0017``
  - Verbatim: ``[PES-ARC-0017] MUST represent LAD as a semantic graph/AST.``
  - Basis: owning source requirement ID.
- **T2-0304** — p. 23; § 8.4 Canonical type system and semantic editors; explicit; mapping: ``PES-ARC-0018``
  - Verbatim: ``[PES-ARC-0018] MUST represent FBD as a typed port graph with stable node, port, and edge identity plus explicit execution dependencies.``
  - Basis: owning source requirement ID.
- **T2-0305** — p. 23; § 8.4 Canonical type system and semantic editors; explicit; mapping: ``PES-ARC-0019``
  - Verbatim: ``[PES-ARC-0019] MUST represent SCL with an independently implemented lexer, parser, AST, source ranges, scope resolver, control-flow model, and original language-service metadata.``
  - Basis: owning source requirement ID.
- **T2-0306** — p. 23; § 8.4 Canonical type system and semantic editors; explicit; mapping: ``PES-ARC-0020``
  - Verbatim: ``[PES-ARC-0020] MUST share instruction definitions, type checking, conversions, call signatures, diagnostics, and runtime semantics across language frontends.``
  - Basis: owning source requirement ID.
- **T2-0307** — p. 23; § 8.4 Canonical type system and semantic editors; explicit; mapping: ``PES-ARC-0021``
  - Verbatim: ``[PES-ARC-0021] MUST NOT execute LAD from screen coordinates, use a regex-only compiler, use eval for SCL, or maintain separate inconsistent runtimes for LAD, FBD, and SCL.``
  - Basis: owning source requirement ID.
- **T2-0308** — p. 23; § 8.5 Unified typed IR and one runtime; explicit; mapping: ``PES-IR-0001``
  - Verbatim: ``[PES-IR-0001] MUST lower LAD, FBD, and SCL semantic models into one versioned, typed, serializable PLC IR.``
  - Basis: owning source requirement ID.
- **T2-0309** — p. 23; § 8.5 Unified typed IR and one runtime; explicit; mapping: ``PES-IR-0002``
  - Verbatim: ``[PES-IR-0002] MUST centralize arithmetic, conversions, comparisons, calls, timers, counters, storage access, monitor probes, source mappings, and error semantics in the shared compiler/runtime path.``
  - Basis: owning source requirement ID.
- **T2-0310** — p. 23; § 8.5 Unified typed IR and one runtime; explicit; mapping: ``PES-IR-0003``
  - Verbatim: ``[PES-IR-0003] MUST make build artifacts immutable and fingerprinted.``
  - Basis: owning source requirement ID.
- **T2-0311** — p. 23; § 8.5 Unified typed IR and one runtime; explicit; mapping: ``PES-IR-0003``
  - Verbatim: ``A build shall identify project snapshot hash, compiler version, IR version, TrainingProfile ID/version, dependency closure, diagnostics, and source map.``
  - Basis: owning source requirement ID.
- **T2-0312** — p. 23; § 8.5 Unified typed IR and one runtime; explicit; mapping: ``PES-IR-0004``
  - Verbatim: ``[PES-IR-0004] MUST NOT produce a runnable artifact when a blocking error exists.``
  - Basis: owning source requirement ID.
- **T2-0313** — p. 23; § 8.5 Unified typed IR and one runtime; explicit; mapping: ``PES-IR-0005``
  - Verbatim: ``[PES-IR-0005] MUST reserve instrumentation points keyed by semantic node and source identity so monitoring, trace, Learning Lens, diagnostics, and assessment can observe one execution without changing it.``
  - Basis: owning source requirement ID.

#### Page 24 — 22 statement(s)

- **T2-0314** — p. 24; § 8.6 Deterministic virtual time, scheduling, and replay; explicit; mapping: ``PES-DET-0001``
  - Verbatim: ``[PES-DET-0001] MUST use simulator-controlled monotonic virtual time for the PLC scheduler, timers, counters with temporal behavior, process physics, trace, scenarios, lesson triggers, and assessment timing.``
  - Basis: owning source requirement ID.
- **T2-0315** — p. 24; § 8.6 Deterministic virtual time, scheduling, and replay; explicit; mapping: ``PES-DET-0002``
  - Verbatim: ``[PES-DET-0002] MUST NOT use wall-clock timers such as browser setTimeout as authoritative PLC or process time.``
  - Basis: owning source requirement ID.
- **T2-0316** — p. 24; § 8.6 Deterministic virtual time, scheduling, and replay; explicit; mapping: ``PES-DET-0003``
  - Verbatim: ``[PES-DET-0003] MUST define stable ordering for events sharing the same virtual timestamp and priority.``
  - Basis: owning source requirement ID.
- **T2-0317** — p. 24; § 8.6 Deterministic virtual time, scheduling, and replay; explicit; mapping: ``PES-DET-0004``
  - Verbatim: ``[PES-DET-0004] MUST include deterministic seed, event sequence, TrainingProfile hash, build hash, initial snapshot hash, simulator version, and scheduler version in replay identity.``
  - Basis: owning source requirement ID.
- **T2-0318** — p. 24; § 8.6 Deterministic virtual time, scheduling, and replay; explicit; mapping: ``PES-DET-0005``
  - Verbatim: ``[PES-DET-0005] MUST distinguish virtual timestamp from engineering-display wall-clock timestamp.``
  - Basis: owning source requirement ID.
- **T2-0319** — p. 24; § 8.6 Deterministic virtual time, scheduling, and replay; explicit; mapping: ``PES-DET-0006``
  - Verbatim: ``[PES-DET-0006] MUST guarantee that the same supported build, snapshot, profile, seed, and ordered events produce equivalent observable tag streams, outputs, diagnostics, trace data, HMI updates, and assessment results.``
  - Basis: owning source requirement ID.
- **T2-0320** — p. 24; § 8.6 Deterministic virtual time, scheduling, and replay; explicit; mapping: ``PES-DET-0007``
  - Verbatim: ``[PES-DET-0007] MUST reserve scan-start, input-sample, program-execution, output-commit, process-update, trace/diagnostic/HMI publication, and scan-end boundaries.``
  - Basis: owning source requirement ID.
- **T2-0321** — p. 24; § 8.7 Separate state layers; explicit; mapping: ``PES-ARC-0022``
  - Verbatim: ``[PES-ARC-0022] MUST keep these layers distinct:``
  - Basis: owning source requirement ID.
- **T2-0322** — p. 24; § 8.7 Separate state layers; inherited bullet; mapping: ``PES-ARC-0022``
  - Verbatim: ``editable offline project source;``
  - Modal lead-in: ``[PES-ARC-0022] MUST keep these layers distinct:``
  - Basis: owning source requirement ID.
- **T2-0323** — p. 24; § 8.7 Separate state layers; inherited bullet; mapping: ``PES-ARC-0022``
  - Verbatim: ``saved project state;``
  - Modal lead-in: ``[PES-ARC-0022] MUST keep these layers distinct:``
  - Basis: owning source requirement ID.
- **T2-0324** — p. 24; § 8.7 Separate state layers; inherited bullet; mapping: ``PES-ARC-0022``
  - Verbatim: ``hardware build state;``
  - Modal lead-in: ``[PES-ARC-0022] MUST keep these layers distinct:``
  - Basis: owning source requirement ID.
- **T2-0325** — p. 24; § 8.7 Separate state layers; inherited bullet; mapping: ``PES-ARC-0022``
  - Verbatim: ``software build state;``
  - Modal lead-in: ``[PES-ARC-0022] MUST keep these layers distinct:``
  - Basis: owning source requirement ID.
- **T2-0326** — p. 24; § 8.7 Separate state layers; inherited bullet; mapping: ``PES-ARC-0022``
  - Verbatim: ``HMI build state;``
  - Modal lead-in: ``[PES-ARC-0022] MUST keep these layers distinct:``
  - Basis: owning source requirement ID.
- **T2-0327** — p. 24; § 8.7 Separate state layers; inherited bullet; mapping: ``PES-ARC-0022``
  - Verbatim: ``immutable build artifact;``
  - Modal lead-in: ``[PES-ARC-0022] MUST keep these layers distinct:``
  - Basis: owning source requirement ID.
- **T2-0328** — p. 24; § 8.7 Separate state layers; inherited bullet; mapping: ``PES-ARC-0022``
  - Verbatim: ``loaded virtual-controller artifact;``
  - Modal lead-in: ``[PES-ARC-0022] MUST keep these layers distinct:``
  - Basis: owning source requirement ID.
- **T2-0329** — p. 24; § 8.7 Separate state layers; inherited bullet; mapping: ``PES-ARC-0022``
  - Verbatim: ``current virtual runtime values;``
  - Modal lead-in: ``[PES-ARC-0022] MUST keep these layers distinct:``
  - Basis: owning source requirement ID.
- **T2-0330** — p. 24; § 8.7 Separate state layers; inherited bullet; mapping: ``PES-ARC-0022``
  - Verbatim: ``declared initial/start values;``
  - Modal lead-in: ``[PES-ARC-0022] MUST keep these layers distinct:``
  - Basis: owning source requirement ID.
- **T2-0331** — p. 24; § 8.7 Separate state layers; inherited bullet; mapping: ``PES-ARC-0022``
  - Verbatim: ``loaded baselines;``
  - Modal lead-in: ``[PES-ARC-0022] MUST keep these layers distinct:``
  - Basis: owning source requirement ID.
- **T2-0332** — p. 24; § 8.7 Separate state layers; inherited bullet; mapping: ``PES-ARC-0022``
  - Verbatim: ``retained values;``
  - Modal lead-in: ``[PES-ARC-0022] MUST keep these layers distinct:``
  - Basis: owning source requirement ID.
- **T2-0333** — p. 24; § 8.7 Separate state layers; inherited bullet; mapping: ``PES-ARC-0022``
  - Verbatim: ``raw virtual-process values;``
  - Modal lead-in: ``[PES-ARC-0022] MUST keep these layers distinct:``
  - Basis: owning source requirement ID.
- **T2-0334** — p. 24; § 8.7 Separate state layers; inherited bullet; mapping: ``PES-ARC-0022``
  - Verbatim: ``CPU-visible values;``
  - Modal lead-in: ``[PES-ARC-0022] MUST keep these layers distinct:``
  - Basis: owning source requirement ID.
- **T2-0335** — p. 24; § 8.7 Separate state layers; inherited bullet; mapping: ``PES-ARC-0022``
  - Verbatim: ``one-shot modifications;``
  - Modal lead-in: ``[PES-ARC-0022] MUST keep these layers distinct:``
  - Basis: owning source requirement ID.

#### Page 25 — 11 statement(s)

- **T2-0336** — p. 25; § 8.7 Separate state layers; inherited bullet; mapping: ``PES-ARC-0022``
  - Verbatim: ``persistent force overlays;``
  - Modal lead-in: ``[PES-ARC-0022] MUST keep these layers distinct:``
  - Basis: owning source requirement ID.
- **T2-0337** — p. 25; § 8.7 Separate state layers; inherited bullet; mapping: ``PES-ARC-0022``
  - Verbatim: ``runtime snapshots;``
  - Modal lead-in: ``[PES-ARC-0022] MUST keep these layers distinct:``
  - Basis: owning source requirement ID.
- **T2-0338** — p. 25; § 8.7 Separate state layers; inherited bullet; mapping: ``PES-ARC-0022``
  - Verbatim: ``project/runtime equality or mismatch;``
  - Modal lead-in: ``[PES-ARC-0022] MUST keep these layers distinct:``
  - Basis: owning source requirement ID.
- **T2-0339** — p. 25; § 8.7 Separate state layers; inherited bullet; mapping: ``PES-ARC-0022``
  - Verbatim: ``monitoring active state.``
  - Modal lead-in: ``[PES-ARC-0022] MUST keep these layers distinct:``
  - Basis: owning source requirement ID.
- **T2-0340** — p. 25; § 8.7 Separate state layers; explicit; mapping: ``PES-ARC-0023``
  - Verbatim: ``[PES-ARC-0023] MUST model "go online" as a virtual session and comparison.``
  - Basis: owning source requirement ID.
- **T2-0341** — p. 25; § 8.7 Separate state layers; explicit; mapping: ``PES-ARC-0023``
  - Verbatim: ``It shall not automatically compile, download, synchronize, or equalize states.``
  - Basis: owning source requirement ID.
- **T2-0342** — p. 25; § 8.7 Separate state layers; explicit; mapping: ``PES-ARC-0024``
  - Verbatim: ``[PES-ARC-0024] MUST model Virtual Download as preview plus explicit approval/cancellation and atomic commit/rollback.``
  - Basis: owning source requirement ID.
- **T2-0343** — p. 25; § 8.7 Separate state layers; explicit; mapping: ``PES-ARC-0025``
  - Verbatim: ``[PES-ARC-0025] MUST make ForceRegistry global runtime state independent of any open table or pane.``
  - Basis: owning source requirement ID.
- **T2-0344** — p. 25; § 8.8 Diagnostics and causal faults; explicit; mapping: ``PES-DIA-0001``
  - Verbatim: ``[PES-DIA-0001] MUST use original simulator codes and prose.``
  - Basis: owning source requirement ID.
- **T2-0345** — p. 25; § 8.8 Diagnostics and causal faults; explicit; mapping: ``PES-DIA-0001``
  - Verbatim: ``It shall not copy vendor numbers or messages.``
  - Basis: owning source requirement ID.
- **T2-0346** — p. 25; § 8.8 Diagnostics and causal faults; explicit; mapping: ``PES-DIA-0002``
  - Verbatim: ``[PES-DIA-0002] MUST distinguish immutable build diagnostics from lifecycle-bearing runtime diagnostic events while permitting a unified UI.``
  - Basis: owning source requirement ID.

#### Page 26 — 10 statement(s)

- **T2-0347** — p. 26; § 8.8 Diagnostics and causal faults; explicit; mapping: ``PES-DIA-0003``
  - Verbatim: ``[PES-DIA-0003] MUST derive diagnostics from ordinary validators, compiler rules, runtime transitions, device/process state, HMI consistency, persistence validation, or fault providers.``
  - Basis: owning source requirement ID.
- **T2-0348** — p. 26; § 8.8 Diagnostics and causal faults; explicit; mapping: ``PES-DIA-0004``
  - Verbatim: ``[PES-DIA-0004] MUST let Teacher Mode invoke commands such as RemoveModule, DisconnectVirtualLink, ChangeTagType, SetSensorFault, or SetActuatorFault and let ordinary engines derive the consequence.``
  - Basis: owning source requirement ID.
- **T2-0349** — p. 26; § 8.8 Diagnostics and causal faults; explicit; mapping: ``PES-DIA-0005``
  - Verbatim: ``[PES-DIA-0005] MUST NOT let a scenario, lesson, demo, or UI directly insert an expected compiler diagnostic, runtime fault, alarm, trace, monitored value, or passing assessment result.``
  - Basis: owning source requirement ID.
- **T2-0350** — p. 26; § 8.8 Diagnostics and causal faults; explicit; mapping: ``PES-DIA-0006``
  - Verbatim: ``[PES-DIA-0006] MUST retain navigation targets, related object identities, virtual timestamps, lifecycle correlation, and deterministic replay ordering.``
  - Basis: owning source requirement ID.
- **T2-0351** — p. 26; § 8.9 Internal buses and future seams; explicit; mapping: ``PES-ARC-0026``
  - Verbatim: ``[PES-ARC-0026] MUST define InternalTagBus as typed, quality-aware, timestamped internal publication/subscription.``
  - Basis: owning source requirement ID.
- **T2-0352** — p. 26; § 8.9 Internal buses and future seams; explicit; mapping: ``PES-ARC-0026``
  - Verbatim: ``It shall operate by in-process calls or typed worker IPC, never localhost or network transport.``
  - Basis: owning source requirement ID.
- **T2-0353** — p. 26; § 8.9 Internal buses and future seams; explicit; mapping: ``PES-ARC-0027``
  - Verbatim: ``[PES-ARC-0027] MUST reserve typed domain registries for project object kinds, editors, properties, commands, validators, compilers, capability gates, navigation targets, fault providers, serializers, and migrations.``
  - Basis: owning source requirement ID.
- **T2-0354** — p. 26; § 8.9 Internal buses and future seams; explicit; mapping: ``PES-ARC-0028``
  - Verbatim: ``[PES-ARC-0028] MUST reserve safe schemas for scenario packages, lesson conditions, assessment expressions, snapshots, fault capabilities, and deterministic events without enabling arbitrary code.``
  - Basis: owning source requirement ID.
- **T2-0355** — p. 26; § 8.9 Internal buses and future seams; explicit; mapping: ``PES-ARC-0029``
  - Verbatim: ``[PES-ARC-0029] MUST allow later HMI, library, trace, technology object, source view, localization, and scenario types without replacing the canonical project graph.``
  - Basis: owning source requirement ID.
- **T2-0356** — p. 26; § 8.9 Internal buses and future seams; explicit; mapping: ``PES-ARC-0030``
  - Verbatim: ``[PES-ARC-0030] MUST NOT satisfy "reserved architecture" with empty buttons, no-op objects, placeholder transports, user-visible coming-soon panels, or generic interfaces that create forbidden capability.``
  - Basis: owning source requirement ID.

#### Page 27 — 27 statement(s)

- **T2-0357** — p. 27; § 9.1 Adopted stack; explicit; mapping: ``PES-DEV-0004``
  - Verbatim: ``[PES-DEV-0004] MUST implement the trusted project semantics, compiler, typed IR, scheduler, and PLC runtime in Rust compiled to capability-limited WebAssembly, unless Scott approves an ADR demonstrating an equally deterministic and more strongly isolated alternative.``
  - Basis: owning source requirement ID.
- **T2-0358** — p. 27; § 9.1 Adopted stack; explicit; mapping: ``PES-DEV-0005``
  - Verbatim: ``[PES-DEV-0005] MUST execute virtual runtime/process work in isolated workers using typed messages so simulation cannot freeze the UI.``
  - Basis: owning source requirement ID.
- **T2-0359** — p. 27; § 9.1 Adopted stack; explicit; mapping: ``PES-DEV-0007``
  - Verbatim: ``[PES-DEV-0007] MUST bundle all production dependencies, fonts, WASM, help, scenarios, and assets locally.``
  - Basis: owning source requirement ID.
- **T2-0360** — p. 27; § 9.1 Adopted stack; explicit; mapping: ``PES-DEV-0008``
  - Verbatim: ``[PES-DEV-0008] MUST keep the trusted core free of OS networking, native FFI, arbitrary filesystem, shell, and device capabilities even if the desktop shell or browser engine internally supports them.``
  - Basis: owning source requirement ID.
- **T2-0361** — p. 27; § 9.1 Adopted stack; explicit; mapping: ``PES-DEV-0009``
  - Verbatim: ``[PES-DEV-0009] MUST record the chosen desktop/classroom packaging model in a BLOCKED product decision before public artifact work begins.``
  - Basis: owning source requirement ID.
- **T2-0362** — p. 27; § 9.1 Adopted stack; explicit; mapping: ``PES-DEV-0009``
  - Verbatim: ``The decision shall define initial supported operating systems, installer versus portable delivery, local-file permissions, Chromium/WebView background networking controls, code signing, update separation, and offline verification.``
  - Basis: owning source requirement ID.
- **T2-0363** — p. 27; § 9.2 Required top-level governance files; explicit; mapping: **UNMAPPED**
  - Verbatim: ``Before feature implementation, the repository shall contain:``
  - Basis: UNMAPPED.
- **T2-0364** — p. 27; § 9.2 Required top-level governance files; inherited line; mapping: ``PES-CRM-0016``
  - Verbatim: ``CLEAN_ROOM_POLICY.md``
  - Modal lead-in: ``Before feature implementation, the repository shall contain:``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0365** — p. 27; § 9.2 Required top-level governance files; inherited line; mapping: ``PES-SEC-0017``
  - Verbatim: ``SECURITY_INVARIANTS.md``
  - Modal lead-in: ``Before feature implementation, the repository shall contain:``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0366** — p. 27; § 9.2 Required top-level governance files; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``LEGAL_REVIEW_CHECKLIST.md``
  - Modal lead-in: ``Before feature implementation, the repository shall contain:``
  - Basis: UNMAPPED.
- **T2-0367** — p. 27; § 9.2 Required top-level governance files; inherited line; mapping: ``PES-CRM-0020``
  - Verbatim: ``CONTRIBUTOR_CLEAN_ROOM_ATTESTATION.md``
  - Modal lead-in: ``Before feature implementation, the repository shall contain:``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0368** — p. 27; § 9.2 Required top-level governance files; inherited line; mapping: ``PES-SEC-0025``
  - Verbatim: ``THREAT_MODEL.md``
  - Modal lead-in: ``Before feature implementation, the repository shall contain:``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0369** — p. 27; § 9.2 Required top-level governance files; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``REQUIREMENTS.md``
  - Modal lead-in: ``Before feature implementation, the repository shall contain:``
  - Basis: UNMAPPED.
- **T2-0370** — p. 27; § 9.2 Required top-level governance files; inherited line; mapping: **UNMAPPED**
  - Verbatim: ``IMPLEMENTATION_MATRIX.*``
  - Modal lead-in: ``Before feature implementation, the repository shall contain:``
  - Basis: UNMAPPED.
- **T2-0371** — p. 27; § 9.2 Required top-level governance files; inherited line; mapping: ``PES-CRM-0017``
  - Verbatim: ``EVIDENCE_REGISTER.*``
  - Modal lead-in: ``Before feature implementation, the repository shall contain:``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0372** — p. 27; § 9.2 Required top-level governance files; inherited line; mapping: ``PES-CRM-0021``
  - Verbatim: ``ASSET_PROVENANCE.*``
  - Modal lead-in: ``Before feature implementation, the repository shall contain:``
  - Basis: materially equivalent numbered statement elsewhere in the source.
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
- **T2-0376** — p. 27; § 9.2 Required top-level governance files; inherited line; mapping: ``PES-GOV-0014``, ``PES-GOV-0015``, ``PES-GOV-0016``
  - Verbatim: ``CHANGELOG_DIRECTIVE.md``
  - Modal lead-in: ``Before feature implementation, the repository shall contain:``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0377** — p. 27; § 9.2 Required top-level governance files; inherited line; mapping: ``PES-DOC-0001``, ``PES-DOC-0003``
  - Verbatim: ``ADR/``
  - Modal lead-in: ``Before feature implementation, the repository shall contain:``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0378** — p. 27; § 9.2 Required top-level governance files; inherited line; mapping: ``PES-DOC-0001``
  - Verbatim: ``0001-no-physical-industrial-communication.md``
  - Modal lead-in: ``Before feature implementation, the repository shall contain:``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0379** — p. 27; § 9.2 Required top-level governance files; inherited line; mapping: ``PES-DOC-0003``
  - Verbatim: ``0002-original-project-format.md``
  - Modal lead-in: ``Before feature implementation, the repository shall contain:``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0380** — p. 27; § 9.2 Required top-level governance files; inherited line; mapping: ``PES-DOC-0003``
  - Verbatim: ``0003-unified-plc-ir.md``
  - Modal lead-in: ``Before feature implementation, the repository shall contain:``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0381** — p. 27; § 9.2 Required top-level governance files; inherited line; mapping: ``PES-DOC-0003``
  - Verbatim: ``0004-deterministic-virtual-time.md``
  - Modal lead-in: ``Before feature implementation, the repository shall contain:``
  - Basis: materially equivalent numbered statement elsewhere in the source.
- **T2-0382** — p. 27; § 9.2 Required top-level governance files; explicit; mapping: ``PES-DOC-0001``
  - Verbatim: ``[PES-DOC-0001] MUST create ADR-0001 with title "Physical Industrial Communication Is Permanently Out of Scope" and status "Project Safety Invariant."``
  - Basis: owning source requirement ID.
- **T2-0383** — p. 27; § 9.2 Required top-level governance files; explicit; mapping: ``PES-DOC-0002``
  - Verbatim: ``[PES-DOC-0002] MUST state in ADR-0001 that physical capability cannot be added within this product through an ADR amendment.``
  - Basis: owning source requirement ID.

#### Page 28 — 6 statement(s)

- **T2-0384** — p. 28; § 9.2 Required top-level governance files; explicit; mapping: ``PES-DOC-0003``
  - Verbatim: ``[PES-DOC-0003] MUST document original project format, unified IR, and deterministic virtual time before implementation depends on them.``
  - Basis: owning source requirement ID.
- **T2-0385** — p. 28; § 9.2 Required top-level governance files; explicit; mapping: ``PES-DOC-0004``
  - Verbatim: ``[PES-DOC-0004] MUST keep evidence records and research notes separate from production assets.``
  - Basis: owning source requirement ID.
- **T2-0386** — p. 28; § 9.3 Required package boundaries; explicit; mapping: ``PES-DEV-0010``
  - Verbatim: ``[PES-DEV-0010] MUST treat this shape as a responsibility map, not permission to create empty packages for completion credit.``
  - Basis: owning source requirement ID.
- **T2-0387** — p. 28; § 9.3 Required package boundaries; explicit; mapping: ``PES-DEV-0011``
  - Verbatim: ``It shall record the reason in an ADR.``
  - Basis: owning source requirement ID.
- **T2-0388** — p. 28; § 9.3 Required package boundaries; explicit; mapping: ``PES-DEV-0012``
  - Verbatim: ``[PES-DEV-0012] MUST NOT create a network, transport, device-connector, vendor-adapter, protocol, external-HMI, remote-collaboration, or plugin-host package.``
  - Basis: owning source requirement ID.
- **T2-0389** — p. 28; § 9.4 Baseline CI policy; explicit; mapping: ``PES-CI-0001``
  - Verbatim: ``[PES-CI-0001] MUST fail production merge or release when:``
  - Basis: owning source requirement ID.

#### Page 29 — 26 statement(s)

- **T2-0390** — p. 29; § 9.4 Baseline CI policy; inherited bullet; mapping: ``PES-CI-0001``
  - Verbatim: ``a forbidden dependency or capability is added;``
  - Modal lead-in: ``[PES-CI-0001] MUST fail production merge or release when:``
  - Basis: owning source requirement ID.
- **T2-0391** — p. 29; § 9.4 Baseline CI policy; inherited bullet; mapping: ``PES-CI-0001``
  - Verbatim: ``a prohibited source API or WASM import appears;``
  - Modal lead-in: ``[PES-CI-0001] MUST fail production merge or release when:``
  - Basis: owning source requirement ID.
- **T2-0392** — p. 29; § 9.4 Baseline CI policy; inherited bullet; mapping: ``PES-CI-0001``
  - Verbatim: ``a remote asset, CDN, telemetry, analytics, or cloud dependency appears;``
  - Modal lead-in: ``[PES-CI-0001] MUST fail production merge or release when:``
  - Basis: owning source requirement ID.
- **T2-0393** — p. 29; § 9.4 Baseline CI policy; inherited bullet; mapping: ``PES-CI-0001``
  - Verbatim: ``an asset lacks provenance or approval;``
  - Modal lead-in: ``[PES-CI-0001] MUST fail production merge or release when:``
  - Basis: owning source requirement ID.
- **T2-0394** — p. 29; § 9.4 Baseline CI policy; inherited bullet; mapping: ``PES-CI-0001``
  - Verbatim: ``a vendor screenshot, logo, icon, device illustration, or copied prose enters production;``
  - Modal lead-in: ``[PES-CI-0001] MUST fail production merge or release when:``
  - Basis: owning source requirement ID.
- **T2-0395** — p. 29; § 9.4 Baseline CI policy; inherited bullet; mapping: ``PES-CI-0001``
  - Verbatim: ``an unclassified research-derived requirement enters implementation;``
  - Modal lead-in: ``[PES-CI-0001] MUST fail production merge or release when:``
  - Basis: owning source requirement ID.
- **T2-0396** — p. 29; § 9.4 Baseline CI policy; inherited bullet; mapping: ``PES-CI-0001``
  - Verbatim: ``a required test is skipped or flaky;``
  - Modal lead-in: ``[PES-CI-0001] MUST fail production merge or release when:``
  - Basis: owning source requirement ID.
- **T2-0397** — p. 29; § 9.4 Baseline CI policy; inherited bullet; mapping: ``PES-CI-0001``
  - Verbatim: ``determinism/replay diverges;``
  - Modal lead-in: ``[PES-CI-0001] MUST fail production merge or release when:``
  - Basis: owning source requirement ID.
- **T2-0398** — p. 29; § 9.4 Baseline CI policy; inherited bullet; mapping: ``PES-CI-0001``
  - Verbatim: ``migration loses identity or data;``
  - Modal lead-in: ``[PES-CI-0001] MUST fail production merge or release when:``
  - Basis: owning source requirement ID.
- **T2-0399** — p. 29; § 9.4 Baseline CI policy; inherited bullet; mapping: ``PES-CI-0001``
  - Verbatim: ``a lesson bypasses ordinary domain/diagnostic behavior;``
  - Modal lead-in: ``[PES-CI-0001] MUST fail production merge or release when:``
  - Basis: owning source requirement ID.
- **T2-0400** — p. 29; § 9.4 Baseline CI policy; inherited bullet; mapping: ``PES-CI-0001``
  - Verbatim: ``Virtual Download accepts any endpoint-like value;``
  - Modal lead-in: ``[PES-CI-0001] MUST fail production merge or release when:``
  - Basis: owning source requirement ID.
- **T2-0401** — p. 29; § 9.4 Baseline CI policy; inherited bullet; mapping: ``PES-CI-0001``
  - Verbatim: ``HMI uses any transport other than InternalTagBus;``
  - Modal lead-in: ``[PES-CI-0001] MUST fail production merge or release when:``
  - Basis: owning source requirement ID.
- **T2-0402** — p. 29; § 9.4 Baseline CI policy; inherited bullet; mapping: ``PES-CI-0001``
  - Verbatim: ``an exported artifact resembles or is accepted as a real industrial deployment artifact;``
  - Modal lead-in: ``[PES-CI-0001] MUST fail production merge or release when:``
  - Basis: owning source requirement ID.
- **T2-0403** — p. 29; § 9.4 Baseline CI policy; inherited bullet; mapping: ``PES-CI-0001``
  - Verbatim: ``traceability between a verified requirement and its tests is missing.``
  - Modal lead-in: ``[PES-CI-0001] MUST fail production merge or release when:``
  - Basis: owning source requirement ID.
- **T2-0404** — p. 29; § 9.4 Baseline CI policy; explicit; mapping: ``PES-CI-0002``
  - Verbatim: ``[PES-CI-0002] MUST scan the packaged artifact, not only source and lockfiles.``
  - Basis: owning source requirement ID.
- **T2-0405** — p. 29; § 9.4 Baseline CI policy; explicit; mapping: ``PES-CI-0003``
  - Verbatim: ``[PES-CI-0003] MUST produce an SBOM, license notice set, asset manifest, requirement-verification report, and isolation report for a release candidate.``
  - Basis: owning source requirement ID.
- **T2-0406** — p. 29; § 10.1 Stable identifiers; explicit; mapping: ``PES-REQ-0001``
  - Verbatim: ``[PES-REQ-0001] MUST identify product requirements as PES-AREA-NNNN.``
  - Basis: owning source requirement ID.
- **T2-0407** — p. 29; § 10.1 Stable identifiers; explicit; mapping: ``PES-REQ-0002``
  - Verbatim: ``[PES-REQ-0002] MUST NOT encode authoring phase, software release, priority, status, or document section in a requirement ID.``
  - Basis: owning source requirement ID.
- **T2-0408** — p. 29; § 10.1 Stable identifiers; explicit; mapping: ``PES-REQ-0003``
  - Verbatim: ``[PES-REQ-0003] MUST identify supporting records separately:``
  - Basis: owning source requirement ID.
- **T2-0409** — p. 29; § 10.1 Stable identifiers; inherited table row; mapping: ``PES-REQ-0003``
  - Verbatim: ``Source/evidence | SRC-NNNN``
  - Modal lead-in: ``[PES-REQ-0003] MUST identify supporting records separately:``
  - Basis: owning source requirement ID.
- **T2-0410** — p. 29; § 10.1 Stable identifiers; inherited table row; mapping: ``PES-REQ-0003``
  - Verbatim: ``Architecture decision | ADR-NNNN``
  - Modal lead-in: ``[PES-REQ-0003] MUST identify supporting records separately:``
  - Basis: owning source requirement ID.
- **T2-0411** — p. 29; § 10.1 Stable identifiers; inherited table row; mapping: ``PES-REQ-0003``
  - Verbatim: ``Product decision | DEC-NNNN``
  - Modal lead-in: ``[PES-REQ-0003] MUST identify supporting records separately:``
  - Basis: owning source requirement ID.
- **T2-0412** — p. 29; § 10.1 Stable identifiers; inherited table row; mapping: ``PES-REQ-0003``
  - Verbatim: ``Open question | OQ-NNNN``
  - Modal lead-in: ``[PES-REQ-0003] MUST identify supporting records separately:``
  - Basis: owning source requirement ID.
- **T2-0413** — p. 29; § 10.1 Stable identifiers; inherited table row; mapping: ``PES-REQ-0003``
  - Verbatim: ``Risk | RSK-NNNN``
  - Modal lead-in: ``[PES-REQ-0003] MUST identify supporting records separately:``
  - Basis: owning source requirement ID.
- **T2-0414** — p. 29; § 10.1 Stable identifiers; inherited table row; mapping: ``PES-REQ-0003``
  - Verbatim: ``Change record | CR-NNNN``
  - Modal lead-in: ``[PES-REQ-0003] MUST identify supporting records separately:``
  - Basis: owning source requirement ID.
- **T2-0415** — p. 29; § 10.1 Stable identifiers; inherited table row; mapping: ``PES-REQ-0003``
  - Verbatim: ``Verification case | VER-AREA-NNNN``
  - Modal lead-in: ``[PES-REQ-0003] MUST identify supporting records separately:``
  - Basis: owning source requirement ID.

#### Page 30 — 23 statement(s)

- **T2-0416** — p. 30; § 10.1 Stable identifiers; explicit; mapping: ``PES-REQ-0004``
  - Verbatim: ``[PES-REQ-0004] MUST keep retired IDs as tombstones with a supersession or rejection reason.``
  - Basis: owning source requirement ID.
- **T2-0417** — p. 30; § 10.1 Stable identifiers; explicit; mapping: ``PES-REQ-0004``
  - Verbatim: ``IDs shall never be recycled.``
  - Basis: owning source requirement ID.
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
- **T2-0435** — p. 30; § 10.2 Atomic record schema; explicit; mapping: ``PES-REQ-0005``
  - Verbatim: ``[PES-REQ-0005] MUST split compound requirements when one part could pass and another fail.``
  - Basis: owning source requirement ID.
- **T2-0436** — p. 30; § 10.2 Atomic record schema; explicit; mapping: ``PES-REQ-0006``
  - Verbatim: ``[PES-REQ-0006] MUST map every implemented behavior to at least one requirement and every MUST/MUST NOT requirement to positive or negative verification.``
  - Basis: owning source requirement ID.
- **T2-0437** — p. 30; § 10.2 Atomic record schema; explicit; mapping: ``PES-REQ-0007``
  - Verbatim: ``[PES-REQ-0007] MUST map every test to the requirements it verifies.``
  - Basis: owning source requirement ID.
- **T2-0438** — p. 30; § 10.2 Atomic record schema; explicit; mapping: ``PES-REQ-0007``
  - Verbatim: ``Orphan tests and unverified requirements shall be visible in CI reports.``
  - Basis: owning source requirement ID.

#### Page 31 — 15 statement(s)

- **T2-0439** — p. 31; § 10.3 Truth states; explicit; mapping: ``PES-REQ-0008``
  - Verbatim: ``[PES-REQ-0008] MUST use VERIFIED as the only state equivalent to complete.``
  - Basis: owning source requirement ID.
- **T2-0440** — p. 31; § 10.3 Truth states; explicit; mapping: ``PES-REQ-0009``
  - Verbatim: ``[PES-REQ-0009] MUST NOT calculate percent complete from file count, package count, UI controls, lines of code, passing compilation, or SCAFFOLDED/PARTIAL items.``
  - Basis: owning source requirement ID.
- **T2-0441** — p. 31; § 10.4 Change control; explicit; mapping: ``PES-GOV-0014``
  - Verbatim: ``[PES-GOV-0014] MUST create a change record for any alteration to a Phase 1 requirement, authority rule, safety boundary, clean-room rule, canonical term, or architecture invariant.``
  - Basis: owning source requirement ID.
- **T2-0442** — p. 31; § 10.4 Change control; explicit; mapping: ``PES-GOV-0015``
  - Verbatim: ``[PES-GOV-0015] MUST include reason, affected IDs, research/evidence impact, security/IP impact, migration impact, test impact, decision authority, approval date, and supersession links.``
  - Basis: owning source requirement ID.
- **T2-0443** — p. 31; § 10.4 Change control; explicit; mapping: ``PES-GOV-0016``
  - Verbatim: ``[PES-GOV-0016] MUST NOT edit a controlling requirement only in code or an ADR.``
  - Basis: owning source requirement ID.
- **T2-0444** — p. 31; § 10.4 Change control; explicit; mapping: ``PES-GOV-0016``
  - Verbatim: ``The directive and traceability records shall change first or in the same approved change.``
  - Basis: owning source requirement ID.
- **T2-0445** — p. 31; § 11.1 Decisions Codex may make; explicit; mapping: ``PES-DEC-0001``
  - Verbatim: ``[PES-DEC-0001] MAY let Codex decide an implementation detail without asking only when every plausible choice:``
  - Basis: owning source requirement ID.
- **T2-0446** — p. 31; § 11.1 Decisions Codex may make; inherited bullet; mapping: ``PES-DEC-0001``
  - Verbatim: ``is internal and reversible;``
  - Modal lead-in: ``[PES-DEC-0001] MAY let Codex decide an implementation detail without asking only when every plausible choice:``
  - Basis: owning source requirement ID.
- **T2-0447** — p. 31; § 11.1 Decisions Codex may make; inherited bullet; mapping: ``PES-DEC-0001``
  - Verbatim: ``preserves observable semantics and file compatibility;``
  - Modal lead-in: ``[PES-DEC-0001] MAY let Codex decide an implementation detail without asking only when every plausible choice:``
  - Basis: owning source requirement ID.
- **T2-0448** — p. 31; § 11.1 Decisions Codex may make; inherited bullet; mapping: ``PES-DEC-0001``
  - Verbatim: ``adds no network, device, native, process, plugin, cloud, AI, or credential capability;``
  - Modal lead-in: ``[PES-DEC-0001] MAY let Codex decide an implementation detail without asking only when every plausible choice:``
  - Basis: owning source requirement ID.
- **T2-0449** — p. 31; § 11.1 Decisions Codex may make; inherited bullet; mapping: ``PES-DEC-0001``
  - Verbatim: ``does not affect IP classification, branding, public claims, grading, teacher/student separation, privacy, or safety;``
  - Modal lead-in: ``[PES-DEC-0001] MAY let Codex decide an implementation detail without asking only when every plausible choice:``
  - Basis: owning source requirement ID.
- **T2-0450** — p. 31; § 11.1 Decisions Codex may make; inherited bullet; mapping: ``PES-DEC-0001``
  - Verbatim: ``stays within approved technology and dependency policy;``
  - Modal lead-in: ``[PES-DEC-0001] MAY let Codex decide an implementation detail without asking only when every plausible choice:``
  - Basis: owning source requirement ID.
- **T2-0451** — p. 31; § 11.1 Decisions Codex may make; inherited bullet; mapping: ``PES-DEC-0001``
  - Verbatim: ``can be objectively verified;``
  - Modal lead-in: ``[PES-DEC-0001] MAY let Codex decide an implementation detail without asking only when every plausible choice:``
  - Basis: owning source requirement ID.
- **T2-0452** — p. 31; § 11.1 Decisions Codex may make; inherited bullet; mapping: ``PES-DEC-0001``
  - Verbatim: ``satisfies all higher requirements.``
  - Modal lead-in: ``[PES-DEC-0001] MAY let Codex decide an implementation detail without asking only when every plausible choice:``
  - Basis: owning source requirement ID.
- **T2-0453** — p. 31; § 11.1 Decisions Codex may make; explicit; mapping: ``PES-DEC-0001``
  - Verbatim: ``Meaningful autonomous decisions shall still be recorded in an ADR or implementation note.``
  - Basis: materially equivalent numbered statement elsewhere in the source.

#### Page 32 — 27 statement(s)

- **T2-0454** — p. 32; § 11.2 Mandatory stop categories; explicit; mapping: ``PES-DEC-0002``
  - Verbatim: ``[PES-DEC-0002] MUST stop the affected work and ask Scott when a choice:``
  - Basis: owning source requirement ID.
- **T2-0455** — p. 32; § 11.2 Mandatory stop categories; inherited bullet; mapping: ``PES-DEC-0002``
  - Verbatim: ``touches physical/network capability or could weaken the VirtualUniverse wall;``
  - Modal lead-in: ``[PES-DEC-0002] MUST stop the affected work and ask Scott when a choice:``
  - Basis: owning source requirement ID.
- **T2-0456** — p. 32; § 11.2 Mandatory stop categories; inherited bullet; mapping: ``PES-DEC-0002``
  - Verbatim: ``uses or resembles vendor assets, protocols, APIs, formats, names, model numbers, diagnostics, branding, or trade dress;``
  - Modal lead-in: ``[PES-DEC-0002] MUST stop the affected work and ask Scott when a choice:``
  - Basis: owning source requirement ID.
- **T2-0457** — p. 32; § 11.2 Mandatory stop categories; inherited bullet; mapping: ``PES-DEC-0002``
  - Verbatim: ``changes public workflow semantics, TrainingProfile behavior, file format, migration, grading, Teacher Mode visibility, or student data handling;``
  - Modal lead-in: ``[PES-DEC-0002] MUST stop the affected work and ask Scott when a choice:``
  - Basis: owning source requirement ID.
- **T2-0458** — p. 32; § 11.2 Mandatory stop categories; inherited bullet; mapping: ``PES-DEC-0002``
  - Verbatim: ``risks data loss, irreversible schema change, or backward incompatibility;``
  - Modal lead-in: ``[PES-DEC-0002] MUST stop the affected work and ask Scott when a choice:``
  - Basis: owning source requirement ID.
- **T2-0459** — p. 32; § 11.2 Mandatory stop categories; inherited bullet; mapping: ``PES-DEC-0002``
  - Verbatim: ``requires cloud, credentials, telemetry, remote services, external AI, or an updater;``
  - Modal lead-in: ``[PES-DEC-0002] MUST stop the affected work and ask Scott when a choice:``
  - Basis: owning source requirement ID.
- **T2-0460** — p. 32; § 11.2 Mandatory stop categories; inherited bullet; mapping: ``PES-DEC-0002``
  - Verbatim: ``adds eval, arbitrary scripting, FFI, child process, shell, native bridge, host device, generic transport, or executable plugin capability;``
  - Modal lead-in: ``[PES-DEC-0002] MUST stop the affected work and ask Scott when a choice:``
  - Basis: owning source requirement ID.
- **T2-0461** — p. 32; § 11.2 Mandatory stop categories; inherited bullet; mapping: ``PES-DEC-0002``
  - Verbatim: ``is marked NEEDS MORE RESEARCH, Class 7, Class 8, or professional legal review;``
  - Modal lead-in: ``[PES-DEC-0002] MUST stop the affected work and ask Scott when a choice:``
  - Basis: owning source requirement ID.
- **T2-0462** — p. 32; § 11.2 Mandatory stop categories; inherited bullet; mapping: ``PES-DEC-0002``
  - Verbatim: ``makes a safety, certification, compatibility, equivalence, endorsement, or production claim;``
  - Modal lead-in: ``[PES-DEC-0002] MUST stop the affected work and ask Scott when a choice:``
  - Basis: owning source requirement ID.
- **T2-0463** — p. 32; § 11.2 Mandatory stop categories; inherited bullet; mapping: ``PES-DEC-0002``
  - Verbatim: ``conflicts with higher authority;``
  - Modal lead-in: ``[PES-DEC-0002] MUST stop the affected work and ask Scott when a choice:``
  - Basis: owning source requirement ID.
- **T2-0464** — p. 32; § 11.2 Mandatory stop categories; inherited bullet; mapping: ``PES-DEC-0002``
  - Verbatim: ``cannot be verified objectively;``
  - Modal lead-in: ``[PES-DEC-0002] MUST stop the affected work and ask Scott when a choice:``
  - Basis: owning source requirement ID.
- **T2-0465** — p. 32; § 11.2 Mandatory stop categories; inherited bullet; mapping: ``PES-DEC-0002``
  - Verbatim: ``would choose initial operating systems or the production packaging model;``
  - Modal lead-in: ``[PES-DEC-0002] MUST stop the affected work and ask Scott when a choice:``
  - Basis: owning source requirement ID.
- **T2-0466** — p. 32; § 11.2 Mandatory stop categories; inherited bullet; mapping: ``PES-DEC-0002``
  - Verbatim: ``would expand scope beyond the authored phases.``
  - Modal lead-in: ``[PES-DEC-0002] MUST stop the affected work and ask Scott when a choice:``
  - Basis: owning source requirement ID.
- **T2-0467** — p. 32; § 11.2 Mandatory stop categories; explicit; mapping: ``PES-DEC-0003``
  - Verbatim: ``[PES-DEC-0003] MUST stop and request additional verified research rather than invent exact controller-family OB numbers, priority/preemption matrices, nesting limits, recursion behavior, proprietary optimized DB layouts, vendor-specific conversions/built-ins, force edge cases, diagnostic numbers/prose, auto-tuning, complex motion, or legacy-language semantics.``
  - Basis: owning source requirement ID.
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
- **T2-0480** — p. 32; § 11.3 BLOCKED decision record; explicit; mapping: ``PES-DEC-0004``
  - Verbatim: ``[PES-DEC-0004] MUST bundle related questions so Scott receives the smallest coherent decision request.``
  - Basis: owning source requirement ID.

#### Page 33 — 24 statement(s)

- **T2-0481** — p. 33; § 11.3 BLOCKED decision record; explicit; mapping: ``PES-DEC-0005``
  - Verbatim: ``[PES-DEC-0005] MUST continue unrelated work while the affected area remains blocked.``
  - Basis: owning source requirement ID.
- **T2-0482** — p. 33; § 11.3 BLOCKED decision record; explicit; mapping: ``PES-DEC-0006``
  - Verbatim: ``[PES-DEC-0006] MUST NOT treat silence, elapsed time, a placeholder, "close as possible," "educational," or an implementation guess as approval.``
  - Basis: owning source requirement ID.
- **T2-0483** — p. 33; § 12.1 Forbidden implementation theater; explicit; mapping: ``PES-QLT-0001``
  - Verbatim: ``[PES-QLT-0001] MUST NOT count a feature as implemented because a pane, button, menu item, type, interface, package, schema field, animation, sample, or mocked path exists.``
  - Basis: owning source requirement ID.
- **T2-0484** — p. 33; § 12.1 Forbidden implementation theater; explicit; mapping: ``PES-QLT-0002``
  - Verbatim: ``[PES-QLT-0002] MUST NOT ship:``
  - Basis: owning source requirement ID.
- **T2-0485** — p. 33; § 12.1 Forbidden implementation theater; inherited bullet; mapping: ``PES-QLT-0002``
  - Verbatim: ``no-op commands;``
  - Modal lead-in: ``[PES-QLT-0002] MUST NOT ship:``
  - Basis: owning source requirement ID.
- **T2-0486** — p. 33; § 12.1 Forbidden implementation theater; inherited bullet; mapping: ``PES-QLT-0002``
  - Verbatim: ``hard-coded success or catch-and-return-success;``
  - Modal lead-in: ``[PES-QLT-0002] MUST NOT ship:``
  - Basis: owning source requirement ID.
- **T2-0487** — p. 33; § 12.1 Forbidden implementation theater; inherited bullet; mapping: ``PES-QLT-0002``
  - Verbatim: ``fake compile, load, online, scan, force, HMI, or diagnostic animations;``
  - Modal lead-in: ``[PES-QLT-0002] MUST NOT ship:``
  - Basis: owning source requirement ID.
- **T2-0488** — p. 33; § 12.1 Forbidden implementation theater; inherited bullet; mapping: ``PES-QLT-0002``
  - Verbatim: ``canned errors or predetermined lesson results;``
  - Modal lead-in: ``[PES-QLT-0002] MUST NOT ship:``
  - Basis: owning source requirement ID.
- **T2-0489** — p. 33; § 12.1 Forbidden implementation theater; inherited bullet; mapping: ``PES-QLT-0002``
  - Verbatim: ``sample-specific PLC/process logic in general engines;``
  - Modal lead-in: ``[PES-QLT-0002] MUST NOT ship:``
  - Basis: owning source requirement ID.
- **T2-0490** — p. 33; § 12.1 Forbidden implementation theater; inherited bullet; mapping: ``PES-QLT-0002``
  - Verbatim: ``offline values displayed as monitored runtime values;``
  - Modal lead-in: ``[PES-QLT-0002] MUST NOT ship:``
  - Basis: owning source requirement ID.
- **T2-0491** — p. 33; § 12.1 Forbidden implementation theater; inherited bullet; mapping: ``PES-QLT-0002``
  - Verbatim: ``HMI animation disconnected from InternalTagBus;``
  - Modal lead-in: ``[PES-QLT-0002] MUST NOT ship:``
  - Basis: owning source requirement ID.
- **T2-0492** — p. 33; § 12.1 Forbidden implementation theater; inherited bullet; mapping: ``PES-QLT-0002``
  - Verbatim: ``mock/test doubles reachable in production;``
  - Modal lead-in: ``[PES-QLT-0002] MUST NOT ship:``
  - Basis: owning source requirement ID.
- **T2-0493** — p. 33; § 12.1 Forbidden implementation theater; inherited bullet; mapping: ``PES-QLT-0002``
  - Verbatim: ``a regex-only compiler;``
  - Modal lead-in: ``[PES-QLT-0002] MUST NOT ship:``
  - Basis: owning source requirement ID.
- **T2-0494** — p. 33; § 12.1 Forbidden implementation theater; inherited bullet; mapping: ``PES-QLT-0002``
  - Verbatim: ``SCL eval;``
  - Modal lead-in: ``[PES-QLT-0002] MUST NOT ship:``
  - Basis: owning source requirement ID.
- **T2-0495** — p. 33; § 12.1 Forbidden implementation theater; inherited bullet; mapping: ``PES-QLT-0002``
  - Verbatim: ``LAD/FBD execution based on screen coordinates;``
  - Modal lead-in: ``[PES-QLT-0002] MUST NOT ship:``
  - Basis: owning source requirement ID.
- **T2-0496** — p. 33; § 12.1 Forbidden implementation theater; inherited bullet; mapping: ``PES-QLT-0002``
  - Verbatim: ``hidden physical adapters disabled by configuration;``
  - Modal lead-in: ``[PES-QLT-0002] MUST NOT ship:``
  - Basis: owning source requirement ID.
- **T2-0497** — p. 33; § 12.1 Forbidden implementation theater; inherited bullet; mapping: ``PES-QLT-0002``
  - Verbatim: ``scenario code that directly awards a pass;``
  - Modal lead-in: ``[PES-QLT-0002] MUST NOT ship:``
  - Basis: owning source requirement ID.
- **T2-0498** — p. 33; § 12.1 Forbidden implementation theater; inherited bullet; mapping: ``PES-QLT-0002``
  - Verbatim: ``a "coming soon" control that appears operational.``
  - Modal lead-in: ``[PES-QLT-0002] MUST NOT ship:``
  - Basis: owning source requirement ID.
- **T2-0499** — p. 33; § 12.1 Forbidden implementation theater; explicit; mapping: ``PES-QLT-0003``
  - Verbatim: ``[PES-QLT-0003] MUST fail closed when a feature is unavailable.``
  - Basis: owning source requirement ID.
- **T2-0500** — p. 33; § 12.1 Forbidden implementation theater; explicit; mapping: ``PES-QLT-0003``
  - Verbatim: ``The UI shall honestly disable or omit the action and identify the unmet capability without pretending success.``
  - Basis: owning source requirement ID.
- **T2-0501** — p. 33; § 12.2 Permitted scaffolding; explicit; mapping: ``PES-QLT-0004``
  - Verbatim: ``[PES-QLT-0004] MAY create scaffolding only when it:``
  - Basis: owning source requirement ID.
- **T2-0502** — p. 33; § 12.2 Permitted scaffolding; inherited bullet; mapping: ``PES-QLT-0004``
  - Verbatim: ``is not user-visible or release-reachable;``
  - Modal lead-in: ``[PES-QLT-0004] MAY create scaffolding only when it:``
  - Basis: owning source requirement ID.
- **T2-0503** — p. 33; § 12.2 Permitted scaffolding; inherited bullet; mapping: ``PES-QLT-0004``
  - Verbatim: ``contains no forbidden capability;``
  - Modal lead-in: ``[PES-QLT-0004] MAY create scaffolding only when it:``
  - Basis: owning source requirement ID.
- **T2-0504** — p. 33; § 12.2 Permitted scaffolding; inherited bullet; mapping: ``PES-QLT-0004``
  - Verbatim: ``fails closed;``
  - Modal lead-in: ``[PES-QLT-0004] MAY create scaffolding only when it:``
  - Basis: owning source requirement ID.

#### Page 34 — 23 statement(s)

- **T2-0505** — p. 34; § 12.2 Permitted scaffolding; inherited bullet; mapping: ``PES-QLT-0004``
  - Verbatim: ``is labeled SCAFFOLDED in the implementation matrix;``
  - Modal lead-in: ``[PES-QLT-0004] MAY create scaffolding only when it:``
  - Basis: owning source requirement ID.
- **T2-0506** — p. 34; § 12.2 Permitted scaffolding; inherited bullet; mapping: ``PES-QLT-0004``
  - Verbatim: ``has an owner and removal/completion target;``
  - Modal lead-in: ``[PES-QLT-0004] MAY create scaffolding only when it:``
  - Basis: owning source requirement ID.
- **T2-0507** — p. 34; § 12.2 Permitted scaffolding; inherited bullet; mapping: ``PES-QLT-0004``
  - Verbatim: ``earns zero completion credit.``
  - Modal lead-in: ``[PES-QLT-0004] MAY create scaffolding only when it:``
  - Basis: owning source requirement ID.
- **T2-0508** — p. 34; § 12.2 Permitted scaffolding; explicit; mapping: ``PES-QLT-0005``
  - Verbatim: ``[PES-QLT-0005] MUST NOT create an abstract physical connection, generic transport, executable plugin host, network-capable HMI provider, or arbitrary scripting engine even as scaffolding.``
  - Basis: owning source requirement ID.
- **T2-0509** — p. 34; § 12.3 Universal milestone Definition of Done; explicit; mapping: ``PES-QLT-0006``
  - Verbatim: ``[PES-QLT-0006] MUST require every software milestone, when later authorized, to include:``
  - Basis: owning source requirement ID.
- **T2-0510** — p. 34; § 12.3 Universal milestone Definition of Done; inherited bullet; mapping: ``PES-QLT-0006``
  - Verbatim: ``domain model and ownership;``
  - Modal lead-in: ``[PES-QLT-0006] MUST require every software milestone, when later authorized, to include:``
  - Basis: owning source requirement ID.
- **T2-0511** — p. 34; § 12.3 Universal milestone Definition of Done; inherited bullet; mapping: ``PES-QLT-0006``
  - Verbatim: ``invariants;``
  - Modal lead-in: ``[PES-QLT-0006] MUST require every software milestone, when later authorized, to include:``
  - Basis: owning source requirement ID.
- **T2-0512** — p. 34; § 12.3 Universal milestone Definition of Done; inherited bullet; mapping: ``PES-QLT-0006``
  - Verbatim: ``positive behavior;``
  - Modal lead-in: ``[PES-QLT-0006] MUST require every software milestone, when later authorized, to include:``
  - Basis: owning source requirement ID.
- **T2-0513** — p. 34; § 12.3 Universal milestone Definition of Done; inherited bullet; mapping: ``PES-QLT-0006``
  - Verbatim: ``negative behavior;``
  - Modal lead-in: ``[PES-QLT-0006] MUST require every software milestone, when later authorized, to include:``
  - Basis: owning source requirement ID.
- **T2-0514** — p. 34; § 12.3 Universal milestone Definition of Done; inherited bullet; mapping: ``PES-QLT-0006``
  - Verbatim: ``enumerated failure cases and recovery;``
  - Modal lead-in: ``[PES-QLT-0006] MUST require every software milestone, when later authorized, to include:``
  - Basis: owning source requirement ID.
- **T2-0515** — p. 34; § 12.3 Universal milestone Definition of Done; inherited bullet; mapping: ``PES-QLT-0006``
  - Verbatim: ``stable identity and dependency behavior;``
  - Modal lead-in: ``[PES-QLT-0006] MUST require every software milestone, when later authorized, to include:``
  - Basis: owning source requirement ID.
- **T2-0516** — p. 34; § 12.3 Universal milestone Definition of Done; inherited bullet; mapping: ``PES-QLT-0006``
  - Verbatim: ``persistence, migration, and undo where applicable;``
  - Modal lead-in: ``[PES-QLT-0006] MUST require every software milestone, when later authorized, to include:``
  - Basis: owning source requirement ID.
- **T2-0517** — p. 34; § 12.3 Universal milestone Definition of Done; inherited bullet; mapping: ``PES-QLT-0006``
  - Verbatim: ``real UI integration where applicable;``
  - Modal lead-in: ``[PES-QLT-0006] MUST require every software milestone, when later authorized, to include:``
  - Basis: owning source requirement ID.
- **T2-0518** — p. 34; § 12.3 Universal milestone Definition of Done; inherited bullet; mapping: ``PES-QLT-0006``
  - Verbatim: ``end-to-end workflow;``
  - Modal lead-in: ``[PES-QLT-0006] MUST require every software milestone, when later authorized, to include:``
  - Basis: owning source requirement ID.
- **T2-0519** — p. 34; § 12.3 Universal milestone Definition of Done; inherited bullet; mapping: ``PES-QLT-0006``
  - Verbatim: ``deterministic unit/integration tests;``
  - Modal lead-in: ``[PES-QLT-0006] MUST require every software milestone, when later authorized, to include:``
  - Basis: owning source requirement ID.
- **T2-0520** — p. 34; § 12.3 Universal milestone Definition of Done; inherited bullet; mapping: ``PES-QLT-0006``
  - Verbatim: ``property/fuzz/golden tests where applicable;``
  - Modal lead-in: ``[PES-QLT-0006] MUST require every software milestone, when later authorized, to include:``
  - Basis: owning source requirement ID.
- **T2-0521** — p. 34; § 12.3 Universal milestone Definition of Done; inherited bullet; mapping: ``PES-QLT-0006``
  - Verbatim: ``isolation/security tests;``
  - Modal lead-in: ``[PES-QLT-0006] MUST require every software milestone, when later authorized, to include:``
  - Basis: owning source requirement ID.
- **T2-0522** — p. 34; § 12.3 Universal milestone Definition of Done; inherited bullet; mapping: ``PES-QLT-0006``
  - Verbatim: ``clean-room evidence and asset provenance;``
  - Modal lead-in: ``[PES-QLT-0006] MUST require every software milestone, when later authorized, to include:``
  - Basis: owning source requirement ID.
- **T2-0523** — p. 34; § 12.3 Universal milestone Definition of Done; inherited bullet; mapping: ``PES-QLT-0006``
  - Verbatim: ``documentation;``
  - Modal lead-in: ``[PES-QLT-0006] MUST require every software milestone, when later authorized, to include:``
  - Basis: owning source requirement ID.
- **T2-0524** — p. 34; § 12.3 Universal milestone Definition of Done; inherited bullet; mapping: ``PES-QLT-0006``
  - Verbatim: ``requirement-to-test traceability;``
  - Modal lead-in: ``[PES-QLT-0006] MUST require every software milestone, when later authorized, to include:``
  - Basis: owning source requirement ID.
- **T2-0525** — p. 34; § 12.3 Universal milestone Definition of Done; inherited bullet; mapping: ``PES-QLT-0006``
  - Verbatim: ``reproducible verification evidence.``
  - Modal lead-in: ``[PES-QLT-0006] MUST require every software milestone, when later authorized, to include:``
  - Basis: owning source requirement ID.
- **T2-0526** — p. 34; § 12.3 Universal milestone Definition of Done; explicit; mapping: ``PES-QLT-0007``
  - Verbatim: ``[PES-QLT-0007] MUST NOT advance a milestone on screenshots, a successful build, a smoke test, a happy-path demo, or manual assertion alone.``
  - Basis: owning source requirement ID.
- **T2-0527** — p. 34; § 12.3 Universal milestone Definition of Done; explicit; mapping: ``PES-QLT-0008``
  - Verbatim: ``[PES-QLT-0008] MUST keep a milestone open if any required test is skipped, flaky, unavailable, manually waived, or inconclusive.``
  - Basis: owning source requirement ID.

#### Page 35 — 13 statement(s)

- **T2-0528** — p. 35; § 13.1 One document, four authoring phases; explicit; mapping: ``PES-GOV-0017``
  - Verbatim: ``[PES-GOV-0017] MUST append and revise this same document for Phases 2-4 while preserving:``
  - Basis: owning source requirement ID.
- **T2-0529** — p. 35; § 13.1 One document, four authoring phases; inherited bullet; mapping: ``PES-GOV-0017``
  - Verbatim: ``exact filename;``
  - Modal lead-in: ``[PES-GOV-0017] MUST append and revise this same document for Phases 2-4 while preserving:``
  - Basis: owning source requirement ID.
- **T2-0530** — p. 35; § 13.1 One document, four authoring phases; inherited bullet; mapping: ``PES-GOV-0017``
  - Verbatim: ``style system;``
  - Modal lead-in: ``[PES-GOV-0017] MUST append and revise this same document for Phases 2-4 while preserving:``
  - Basis: owning source requirement ID.
- **T2-0531** — p. 35; § 13.1 One document, four authoring phases; inherited bullet; mapping: ``PES-GOV-0017``
  - Verbatim: ``requirement IDs;``
  - Modal lead-in: ``[PES-GOV-0017] MUST append and revise this same document for Phases 2-4 while preserving:``
  - Basis: owning source requirement ID.
- **T2-0532** — p. 35; § 13.1 One document, four authoring phases; inherited bullet; mapping: ``PES-GOV-0017``
  - Verbatim: ``cross references;``
  - Modal lead-in: ``[PES-GOV-0017] MUST append and revise this same document for Phases 2-4 while preserving:``
  - Basis: owning source requirement ID.
- **T2-0533** — p. 35; § 13.1 One document, four authoring phases; inherited bullet; mapping: ``PES-GOV-0017``
  - Verbatim: ``source hash history;``
  - Modal lead-in: ``[PES-GOV-0017] MUST append and revise this same document for Phases 2-4 while preserving:``
  - Basis: owning source requirement ID.
- **T2-0534** — p. 35; § 13.1 One document, four authoring phases; inherited bullet; mapping: ``PES-GOV-0017``
  - Verbatim: ``change ledger;``
  - Modal lead-in: ``[PES-GOV-0017] MUST append and revise this same document for Phases 2-4 while preserving:``
  - Basis: owning source requirement ID.
- **T2-0535** — p. 35; § 13.1 One document, four authoring phases; inherited bullet; mapping: ``PES-GOV-0017``
  - Verbatim: ``superseded requirement tombstones;``
  - Modal lead-in: ``[PES-GOV-0017] MUST append and revise this same document for Phases 2-4 while preserving:``
  - Basis: owning source requirement ID.
- **T2-0536** — p. 35; § 13.1 One document, four authoring phases; inherited bullet; mapping: ``PES-GOV-0017``
  - Verbatim: ``open decisions and risk records.``
  - Modal lead-in: ``[PES-GOV-0017] MUST append and revise this same document for Phases 2-4 while preserving:``
  - Basis: owning source requirement ID.
- **T2-0537** — p. 35; § 13.1 One document, four authoring phases; explicit; mapping: ``PES-GOV-0018``
  - Verbatim: ``[PES-GOV-0018] MUST NOT create separate competing master directives for later phases.``
  - Basis: owning source requirement ID.
- **T2-0538** — p. 35; § 13.1 One document, four authoring phases; explicit; mapping: ``PES-GOV-0019``
  - Verbatim: ``[PES-GOV-0019] MUST label unauthored later-phase material as reserved.``
  - Basis: owning source requirement ID.
- **T2-0539** — p. 35; § 13.1 One document, four authoring phases; explicit; mapping: ``PES-GOV-0019``
  - Verbatim: ``It shall not create empty chapters that could be mistaken for complete requirements.``
  - Basis: owning source requirement ID.
- **T2-0540** — p. 35; § 13.1 One document, four authoring phases; explicit; mapping: ``PES-GOV-0020``
  - Verbatim: ``[PES-GOV-0020] MUST perform a cross-phase contradiction and coverage audit after every authoring phase.``
  - Basis: owning source requirement ID.

#### Page 36 — 4 statement(s)

- **T2-0541** — p. 36; § 13.2 Phase 1 acceptance checklist; explicit; mapping: ``PES-ACC-0005``
  - Verbatim: ``[PES-ACC-0005] MUST mark this revision "Phase 1 authored; Phases 2-4 not yet authored."``
  - Basis: owning source requirement ID.
- **T2-0542** — p. 36; § 13.2 Phase 1 acceptance checklist; explicit; mapping: ``PES-ACC-0006``
  - Verbatim: ``[PES-ACC-0006] MUST NOT treat completion of Phase 1 authoring as completion of the master directive.``
  - Basis: owning source requirement ID.
- **T2-0543** — p. 36; § 13.2 Phase 1 acceptance checklist; explicit; mapping: ``PES-ACC-0007``
  - Verbatim: ``[PES-ACC-0007] MUST NOT authorize product coding from this incomplete directive unless Scott separately gives explicit implementation authorization before Phases 2-4 are complete.``
  - Basis: owning source requirement ID.
- **T2-0544** — p. 36; § 13.3 Open decisions carried forward; explicit table row; mapping: **UNMAPPED**
  - Verbatim: ``OQ-0008 | Accessibility conformance target and performance/capacity budgets | Must be objective before experience acceptance | Phase 3``
  - Basis: UNMAPPED.

#### Page 37 — 0 statement(s)

No in-scope modal statement on this page.

#### Page 38 — 1 statement(s)

- **T2-0545** — p. 38; § Appendix A. Canonical Glossary; explicit table row; mapping: ``PES-DET-0002``, ``PES-DET-0005``
  - Verbatim: ``Engineering timestamp | Human-facing wall-clock metadata; never authoritative simulation time``
  - Basis: materially equivalent numbered statement elsewhere in the source.

#### Page 39 — 0 statement(s)

No in-scope modal statement on this page.

#### Page 40 — 1 statement(s)

- **T2-0546** — p. 40; § Appendix F. Phase 1 "Do Not Build This" Register; explicit; mapping: ``PES-SCP-0001``, ``PES-ISO-0001``, ``PES-CRM-0001``, ``PES-DET-0001``, ``PES-FID-0002``
  - Verbatim: ``The product may become broad, realistic, polished, and deeply functional only inside these boundaries.``
  - Basis: materially equivalent numbered statement elsewhere in the source.

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

## Task 5 — Check independence

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

<!-- TASK6 -->

## Task 7 — Scope leak

<!-- TASK7 -->

## Task 8 — Interpretation log

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
| 43 | `.github/workflows/phase1-governance.yml:1-69` | Selects GitHub Actions, `windows-2025`, concurrency, 10-minute timeout, four action SHAs, exact runtime setup, commands, artifact name, hidden-file inclusion, warning behavior, and 14-day retention. | `PES-CI-0001`–`0003`, pp.28-29; stop rule `PES-DEC-0002`, p.32. | Entire remote design is a proposal and all exact service choices are gap-filled. Literal `if: ${{ false }}` prevents execution, but commit identity/provenance and service terms are unknown. |
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

### Bottom line

The repository is unusually candid about its limits: zero VERIFIED requirements, no Phase 2–4 feature roots, remote CI disabled, all tools unapproved, contributor/reviewer work incomplete, and visual evidence non-gating. The adversarial weakness is not a hidden product-completion claim; it is that a 163/163 `PASS` can look stronger than it is. Under this audit, only 64 instances have a specific directive anchor, and even those are mostly guardrails. The remaining 99 are structural or circular. The correct interpretation is: **the Phase 1 governance snapshot is internally consistent under its own implementation-authored contract, while atomicity, independent review, tool admission, evidence approval, and the Phase 1 exit gate remain open.**

## Not verifiable

<!-- NOT_VERIFIABLE -->

## Phase 2 verdict

<!-- VERDICT -->
