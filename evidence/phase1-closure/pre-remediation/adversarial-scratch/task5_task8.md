# Phase 1 Adversarial Audit — Tasks 5 and 8

Audit date: 2026-08-27  
Repository: `C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC`  
Audit mode: read-only repository inspection; this report is the only output  
Verifier snapshot inspected: `.phase1-verification/phase1-report.json`, suite `1.2.0`, generated `2026-08-27T19:19:17.441Z`

## Executive result

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

## Task 5 — complete 163-instance classification

### Classification rule

- **A — Derived:** the checked property is traceable to at least one specific `PES-*` requirement. The table cites the most direct ID(s) and the rendered directive page/section.
- **B — Structural:** the check validates a filename, schema, parser boundary, hash binding, local evidence shape, or repository convention selected by the implementation. It may support the directive, but the exact predicate is not a numbered requirement.
- **C — Tautological:** the expected value and actual representation are both controlled by the repository implementation (often duplicated between the extractor, verifier, policy contract, plan, register, matrix, or workflow). Such a check can catch accidental drift but cannot independently prove the chosen value is correct.

### Full table

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

## Task 8 — interpretation, inference, normalization, and gap-filling ledger

### Method and source-location note

I treated text copied verbatim from a numbered directive record as literal extraction. Everything else—record boundaries, titles, schema, state, milestone, dependency relation, IP disposition, acceptance criterion, implementation mapping, test oracle, operational policy, or current-status judgment—was examined as potential interpretation or gap-filling.

Directive page numbers below come from the current 40-page local render solely to make the source easy to locate. That render remains an unapproved observation and is not used here as Phase 1 acceptance evidence. The authoritative anchors are the quoted `PES-*` IDs and DOCX section headings.

### Exhaustive artifact ledger

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

### Generated-record enrichment counts

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

### Material adversarial findings

#### High — the suite is not an independent correctness oracle

The policy contract, extractor tables, verifier literals, verification plan, generated registry, matrix, toolchain register, and workflow repeat the same choices. The strongest example is `tools/phase1/verify-phase1.mjs:462-497`, which calls the contract allowlist “independent” while comparing it to mappings generated from `tools/phase1/extract_directive_requirements.py:62-225`; both were authored within the same implementation. The 63 class-C instances are useful drift alarms but cannot validate the underlying interpretation.

#### High — atomic requirement work is explicitly incomplete

`PES-REQ-0005` requires compound requirements to be split when parts can pass/fail separately. The extractor instead preserves source IDs and marks 20 records `COMPOUND_SOURCE_REQUIRES_REVIEW` (`tools/phase1/extract_directive_requirements.py:600-604`; `REQUIREMENTS.md:45-49`). This is defensible change-control caution, but it means the record set is not yet the atomic/testable system the directive requires. “247 requirements” should always be read as “247 source requirement IDs.”

#### High — current-state and mapping claims are authored judgments

The 30 non-default/current-foundation IDs, 20 curated acceptance records, state assignments, related-requirement lists, component paths, and `VER-*` mappings are all hard-coded enrichment. Their `IMPLEMENTED_UNVERIFIED`/`PARTIAL`/`SCAFFOLDED` labels are not derived merely by parsing the DOCX and have no named independent reviewer. The repository generally discloses this, but a passing verifier cannot promote their epistemic status.

#### Medium — evidence/asset checks are selective and partly vacuous

The evidence register validates only three self-listed local sources and 20 manually selected rows. The asset registry has `assets: []`; therefore its per-asset completeness predicate is vacuously true. Absence of reserved product roots helps, but the register itself is not a discovery scan or completed provenance audit.

#### Medium — exact toolchain and CI values are unapproved implementation choices

Node 24.19.0, pnpm 11.19.0, Python 3.13.12, Rust 1.94.0, GitHub Actions, `windows-2025`, four action SHAs, 10-minute timeout, and 14-day retention do not come from the directive. The register correctly leaves all 13 tools unapproved and the remote job disabled. Their presence is a proposal/scaffold, not evidence that those versions, binaries, services, or licenses are admissible.

#### Medium — the bootstrap exception is a self-authored authority bridge

`DEPENDENCY_POLICY.md:77-96` interprets Scott's “begin Phase 1” authorization and `PES-SEC-0004` as permission to use pre-existing, standard-library-only local tools before admission. This enabled the extractor/verifier, while the later Word/Poppler/PDF-Python run fell outside it. The boundary is now accurately disclosed, but the exception itself should receive explicit owner approval if it is to remain a governance authority rather than a temporary implementation assumption.

#### Medium — visual QA is a truthful but non-portable observation

The current local PDF, 40 PNGs, and analysis JSON match the recorded hashes; all-page review found no stored-render defect. Those files are ignored, absent on a clean checkout, and created with unapproved tools. The verifier intentionally records `ABSENT_IGNORED_LOCAL_EVIDENCE` without failure on a clean checkout. This is internally truthful, but the Markdown observation cannot become gate evidence until an admitted toolchain and reviewer perform a complete rerun.

### Coverage statement

This ledger covers every controlled Phase 1 artifact class: the two generated requirement views; all four ADRs; clean-room, security, threat, legal, contributor, dependency, evidence, asset, decision, risk, scope, QA, verification-plan, toolchain, README, and changelog records; Node/pnpm/Python/Rust/workflow/repository configuration; policy contract; extractor; launcher; verifier; and latest report. The two supplied source documents were treated as inputs and were not attributed repository-authored interpretations.

## Bottom line

The repository is unusually candid about its limits: zero VERIFIED requirements, no Phase 2–4 feature roots, remote CI disabled, all tools unapproved, contributor/reviewer work incomplete, and visual evidence non-gating. The adversarial weakness is not a hidden product-completion claim; it is that a 163/163 `PASS` can look stronger than it is. Under this audit, only 64 instances have a specific directive anchor, and even those are mostly guardrails. The remaining 99 are structural or circular. The correct interpretation is: **the Phase 1 governance snapshot is internally consistent under its own implementation-authored contract, while atomicity, independent review, tool admission, evidence approval, and the Phase 1 exit gate remain open.**
