# Phase 1 Adversarial Audit - Task 1 Precision and Task 3 Count Reconciliation

**Audit mode:** read-only repository; generated registry/matrix/report/verifier artifacts treated only as audit targets  
**Audit date:** 2026-08-27  
**Supplied DOCX SHA-256:** EBF074E2CEAB752F09E6DB63D88E100991729DA13C1EB874290A6B337DA72612  
**Frozen research SHA-256:** F05C08323B5CC9483BEB1FEB3C7312CCB9A45EBE3B527E6DAE069C181D3FBF55  
**Output scope:** deterministic 21-position sample plus all PES requirements referenced by SI-GATE-12 through SI-GATE-15 and TM-11 through TM-22

## Executive findings

1. The supplied DOCX contains **247 unique issued PES IDs**. Every area is contiguous from 0001 through its observed maximum; there are no duplicate or missing sequence numbers inside an area.
2. Fresh raw PDF text extraction mapped **all 247 issued markers exactly once** across **40 pages**. Page values below are marker-start pages and are locators only; requirement wording comes from the DOCX OOXML text.
3. The deterministic positional sample contains **21 IDs**. The selected security/threat rows expand to **94 unique PES IDs**. Eight overlap, yielding **107 audited rows**.
4. Selected-row result: **107 FAITHFUL, 0 DRIFTED, 0 UNSUPPORTED, 0 MISSING_SOURCE**.
5. A whole-population text comparison, performed as a cross-check beyond the required selection, found **one out-of-sample verbatim drift: PES-REQ-0003**. Its registry atomicRequirement injects the words "Table row:" before every source table row. The table data and meaning are preserved, but the field is not verbatim source text.
6. The supplied DOCX contains **no explicit numeric statement that the requirement total is 247**: the token 247 does not occur in DOCX textual content, and no requirement-total/count statement was found. The frozen research contains neither issued PES markers nor the token 247. Thus **247 is an independently observed issued-ID count, not a directive-stated total**.
7. Each of the 247 issued IDs begins with exactly one recognized leading normative keyword. This proves 247 issued requirement records, but not 247 semantically atomic normative statements; compound sentences, subordinate shall/may clauses, and list continuations prevent a one-record/one-atomic-statement inference.

## Evidence boundary and method

- Source authority for wording: the supplied DOCX only. The frozen research was hash-checked and inspected only to confirm it does not issue PES IDs or state a 247 total.
- Independent extraction: OOXML body blocks were walked in document order using the bundled document runtime. Heading paths came from Word heading styles. Table cells were preserved as cell | cell textual rows.
- "Verbatim source" below means exact OOXML textual content. Auto-number and bullet glyphs are Word numbering metadata rather than w:t text; block order and words are preserved.
- Page locator: raw text extraction from the current 40-page PDF render. Every [PES-...] marker occurred on exactly one page.
- Registry comparison: requirements/phase1-requirements.json was parsed as an audit target. A row is FAITHFUL only when atomicRequirement, independently derived heading path, source filename, source hash, and SRC-0002 pointer all match.
- DRIFTED: source and registry record both exist but differ. UNSUPPORTED: target wording/pointer lacks support in the located source. MISSING_SOURCE: no issued source marker exists.
- SECURITY_INVARIANTS.md and THREAT_MODEL.md were used only to determine the parent-requested selection; their claims were not treated as proof of directive wording.

## Deterministic positional sample

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

## Expanded gate/threat reference selection

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

## Task 1 precision rows

Rows are sorted by full requirement ID. A row can have both a positional-sample origin and one or more gate/threat origins.

### PES-ACC-0001

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

### PES-ARC-0004

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

### PES-ARC-0005

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

### PES-ARC-0006

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

### PES-ARC-0007

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

### PES-ARC-0008

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

### PES-ARC-0009

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

### PES-ARC-0010

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

### PES-ARC-0011

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

### PES-ARC-0012

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

### PES-ARC-0013

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

### PES-ARC-0014

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

### PES-ARC-0015

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

### PES-ARC-0018

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

### PES-ARC-0026

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

### PES-ARC-0030

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

### PES-CI-0001

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

### PES-CI-0002

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

### PES-CI-0003

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

### PES-CRM-0001

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

### PES-CRM-0002

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

### PES-CRM-0003

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

### PES-CRM-0004

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

### PES-CRM-0005

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

### PES-CRM-0006

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

### PES-CRM-0007

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

### PES-CRM-0008

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

### PES-CRM-0009

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

### PES-CRM-0010

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

### PES-CRM-0011

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

### PES-CRM-0012

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

### PES-CRM-0013

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

### PES-CRM-0014

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

### PES-CRM-0015

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

### PES-CRM-0016

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

### PES-CRM-0017

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

### PES-CRM-0018

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

### PES-CRM-0019

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

### PES-CRM-0020

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

### PES-CRM-0021

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

### PES-CRM-0022

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

### PES-CRM-0023

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

### PES-CRM-0024

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

### PES-CRM-0025

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

### PES-DET-0001

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

### PES-DET-0002

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

### PES-DET-0003

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

### PES-DET-0004

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

### PES-DET-0005

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

### PES-DET-0006

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

### PES-DET-0007

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

### PES-DEV-0005

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

### PES-DEV-0007

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

### PES-DEV-0012

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

### PES-DIA-0003

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

### PES-DIA-0004

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

### PES-DIA-0005

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

### PES-DOC-0001

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

### PES-DOC-0004

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

### PES-EDU-0004

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

### PES-EDU-0005

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

### PES-FID-0003

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

### PES-GOV-0007

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

### PES-GOV-0019

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

### PES-ISO-0005

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

### PES-ISO-0006

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

### PES-ISO-0007

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

### PES-ISO-0011

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

### PES-ISO-0018

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

### PES-ISO-0019

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

### PES-ISO-0020

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

### PES-ISO-0021

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

### PES-ISO-0022

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

### PES-MSN-0007

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

### PES-MSN-0008

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

### PES-PRJ-0001

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

### PES-PRJ-0002

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

### PES-PRJ-0003

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

### PES-PRJ-0004

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

### PES-PRJ-0005

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

### PES-PRJ-0006

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

### PES-PRJ-0007

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

### PES-PROF-0004

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

### PES-QLT-0005

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

### PES-QLT-0006

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

### PES-QLT-0008

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

### PES-REQ-0002

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

### PES-SCP-0005

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

### PES-SCP-0006

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

### PES-SEC-0006

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

### PES-SEC-0007

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

### PES-SEC-0008

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

### PES-SEC-0010

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

### PES-SEC-0011

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

### PES-SEC-0014

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

### PES-SEC-0015

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

### PES-SEC-0016

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

### PES-SEC-0018

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

### PES-SEC-0019

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

### PES-SEC-0020

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

### PES-SEC-0022

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

### PES-TCH-0001

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

### PES-TCH-0002

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

### PES-TCH-0003

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

### PES-TCH-0004

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

### PES-TCH-0005

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

### PES-TYP-0001

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

## Out-of-sample whole-population defect

### PES-REQ-0003 - DRIFTED

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

## Task 3 count reconciliation

### 3.1 What the directive actually states

- No explicit numeric requirement total occurs in the supplied DOCX.
- Independent marker count: **247 unique issued IDs**, with no duplicates.
- Every issued ID has one recognized leading keyword. Distribution: **MUST 184**, **MUST NOT 50**, **SHOULD 5**, **MAY 8**; no issued lead uses SHALL/SHALL NOT/SHOULD NOT.
- The frozen research has **0 issued PES markers** and **0 occurrences of the token 247**.

### 3.2 Area-by-area issued-ID reconciliation

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

### 3.3 Normative-statement signals versus issued IDs

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

### 3.4 Un-ID'd modal-bearing source blocks requiring recall-stream inclusion rules

The following source blocks are outside issued requirement bodies. They are provided verbatim so the Task 2 recall stream can be reconciled without treating a regex count as a semantic decision.

#### U-01

- **Container:** paragraph; Normal
- **Matched modal tokens:** shall
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
The product shall simulate engineering decisions and consequences with high training-transfer fidelity while remaining permanently incapable of communicating with or operating physical industrial equipment.
~~~~

#### U-02

- **Container:** paragraph; Normal
- **Matched modal tokens:** shall, shall, may
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
CONTROLLING PRODUCT TRUTH: Build a professional, brand-neutral PLC engineering and simulation environment for education. It shall provide high causal, behavioral, workflow, and training-transfer fidelity inside a wholly fictional VirtualUniverse. It shall never communicate with, discover, configure, commission, download to, or operate physical industrial equipment. No adapter to a physical universe may exist.
~~~~

#### U-03

- **Container:** paragraph; Normal
- **Matched modal tokens:** shall not
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
CONSTRUCTION STATUS: This is the first authoring phase of one living directive. Unless Scott separately orders otherwise, Codex shall not begin product implementation from this incomplete directive.
~~~~

#### U-04

- **Container:** paragraph; Normal
- **Matched modal tokens:** shall not
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
Later phases are intentionally absent from this revision. Reserved headings are not implementation requirements and shall not be inferred.
~~~~

#### U-05

- **Container:** table-row; no paragraph style
- **Matched modal tokens:** MUST, SHALL
- **Clearly meta/definitional:** yes

~~~~text
MUST / SHALL | Required. Violation blocks merge, release, or acceptance.
~~~~

#### U-06

- **Container:** table-row; no paragraph style
- **Matched modal tokens:** MUST NOT, SHALL NOT
- **Clearly meta/definitional:** yes

~~~~text
MUST NOT / SHALL NOT | Prohibited. Presence blocks merge, release, or acceptance.
~~~~

#### U-07

- **Container:** table-row; no paragraph style
- **Matched modal tokens:** SHOULD
- **Clearly meta/definitional:** yes

~~~~text
SHOULD | Expected unless a documented ADR proves an equal or stronger result without changing product intent.
~~~~

#### U-08

- **Container:** table-row; no paragraph style
- **Matched modal tokens:** SHOULD NOT
- **Clearly meta/definitional:** yes

~~~~text
SHOULD NOT | Avoid unless a documented ADR proves necessity and preserves all higher requirements.
~~~~

#### U-09

- **Container:** table-row; no paragraph style
- **Matched modal tokens:** MAY
- **Clearly meta/definitional:** yes

~~~~text
MAY | Optional and permitted only inside the approved scope.
~~~~

#### U-10

- **Container:** paragraph; Normal
- **Matched modal tokens:** shall
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
Every externally inspired requirement shall be classified before implementation:
~~~~

#### U-11

- **Container:** table-row; no paragraph style
- **Matched modal tokens:** May, shall not
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
Development environment | package managers, compilers, test servers, CI tools | May use development capabilities but shall not enter production
~~~~

#### U-12

- **Container:** paragraph; Normal
- **Matched modal tokens:** shall
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
Every meaningful mutation shall be a domain command. The minimum conceptual result is:
~~~~

#### U-13

- **Container:** paragraph; Normal
- **Matched modal tokens:** shall
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
Before feature implementation, the repository shall contain:
~~~~

#### U-14

- **Container:** paragraph; Normal
- **Matched modal tokens:** shall
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
Every requirement record shall contain:
~~~~

#### U-15

- **Container:** paragraph; Heading 2
- **Matched modal tokens:** may
- **Clearly meta/definitional:** yes

~~~~text
11.1 Decisions Codex may make
~~~~

#### U-16

- **Container:** paragraph; Normal
- **Matched modal tokens:** shall
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
Every blocked decision request shall contain:
~~~~

#### U-17

- **Container:** table-row; no paragraph style
- **Matched modal tokens:** Must
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
OQ-0008 | Accessibility conformance target and performance/capacity budgets | Must be objective before experience acceptance | Phase 3
~~~~

#### U-18

- **Container:** table-row; no paragraph style
- **Matched modal tokens:** MUST, MUST NOT
- **Clearly meta/definitional:** yes

~~~~text
Binding MUST/MUST NOT rules | Final Codex marching orders
~~~~

#### U-19

- **Container:** paragraph; Normal
- **Matched modal tokens:** may, may
- **Clearly meta/definitional:** no; recall-stream rule required

~~~~text
Phase 1 closing rule: The foundation is now explicit. The product may become broad, realistic, polished, and deeply functional only inside these boundaries. No later phase may buy fidelity by weakening originality, determinism, causal behavior, or physical isolation.
~~~~

### 3.5 Recall-stream dependency and reconciliation conclusion

The Task 2 recall stream was not supplied to this audit worker. Therefore its semantic statement count, boundaries, exclusions, and ordering cannot be asserted or reverse-engineered from registry output.

What can be reconciled independently is:

1. 247 exactly reconciles to unique issued IDs and to issued lead normative clauses.
2. 247 does **not** establish that the directive contains only 247 semantically atomic obligations.
3. A recall count above 247 can be legitimate if it splits secondary shall/may clauses, coordinated obligations, colon-introduced lists, or un-ID'd controlling prose.
4. A recall count derived from raw modal-token frequency alone is not reliable: the source contains definitions and category-name mentions that are not newly issued requirements.
5. Final Task 3 equality/difference arithmetic must wait for the actual recall stream. When available, each recalled statement should be matched to an issued ID, one of the un-ID'd blocks above, or marked unsupported; only then can a semantic total be defended.

## Audit conclusion

The required 107-row sample is fully source-faithful. There are no missing-source or unsupported rows in that selection. The principal precision defect is the out-of-sample PES-REQ-0003 registry transformation, and the principal count defect is representational: repository artifacts present 247 as a generated requirement count even though the directive itself does not state that numeric total and issued IDs are not demonstrably identical to semantically atomic normative statements.



