# Unresolved Source Tokens

Status: Phase 1 evidence-control inventory; underlying external sources are unresolved  
Scanned file: `Govs PLC project Research Report.md`  
Verified SHA-256: `F05C08323B5CC9483BEB1FEB3C7312CCB9A45EBE3B527E6DAE069C181D3FBF55`  
Scan date: 2026-08-27  
Governing requirements: PES-GOV-0003 through PES-GOV-0013; PES-CRM-0006 through PES-CRM-0017; PES-DOC-0004

## 1. Finding

The frozen report contains rendered browsing-citation markers in the form:

```text
citeturn11search2
```

These identifiers are conversation-scoped retrieval tokens, not durable bibliographic citations. The report does not embed a recoverable mapping from each token to a source title, publisher, version/date, URL, or access date. The token text alone cannot establish the identity, authority, license, or continued availability of an underlying source.

The inventory found:

- 162 citation marker groups;
- 250 token occurrences within those groups;
- 99 unique tokens;
- only `turn...search...` tokens in the matched set; no durable URL is encoded by that syntax.

This file does not assert that the underlying claims are false. It records that the external source metadata required by PES-GOV-0013 and PES-CRM-0017 is absent from the frozen artifact. Until a source is recovered and verified, the corresponding external citation remains `UNRESOLVED` and cannot independently satisfy an implementation evidence gate.

## 2. Rules for use

1. Do not invent or infer publisher, title, version, date, URL, access date, quotation, or license from a token or nearby prose.
2. Do not convert a report label such as `DOCUMENTED` into verified bibliographic status merely because it has a token.
3. Preserve the report's original classification when a source is recovered; do not upgrade `INFERENCE`, `PROPOSED`, `LEGAL INTERPRETATION`, or `ENGINEERING RECOMMENDATION`.
4. The frozen report remains the Phase 1 baseline at its verified hash, but its unresolved external tokens are not stable source records.
5. Research-derived implementation stays blocked when its only supporting external evidence is unresolved and the directive requires verified behavior or legal review.
6. Evidence files and any lawfully retained source notes remain outside production assets and shipped bundles under PES-DOC-0004.

## 3. Resolution procedure

For each token relied upon by a requirement:

1. Recover the original browsing or research record if it still exists; do not search by conclusion and assume the first matching page was the original source.
2. Verify that the recovered page directly supports the precise paraphrased claim.
3. Prefer the authoritative primary source: official documentation, lawfully licensed standard, statute, judicial opinion, or other source category allowed by PES-CRM-0008.
4. Record a new stable `SRC-NNNN` entry with title, publisher, version/date, durable location, access date, supported claim, report classification, license/usage notes, and content hash when lawfully retained.
5. Assign the requirement's IP class and disposition. Uncertainty defaults to Class 8 `BLOCKED`.
6. Have an identified reviewer approve the mapping and link verification IDs before implementation relies on it.
7. If the original source cannot be recovered, leave the token unresolved. A substitute corroborating source must receive its own source ID and must not be represented as the original citation.

## 4. Exact token inventory

Counts below are literal occurrences in the verified report file. They are not source-quality scores and do not indicate the number of distinct publications.

```csv
token,occurrences
turn0search8,6
turn10search1,6
turn10search2,1
turn10search4,3
turn10search5,6
turn11search2,8
turn11search4,5
turn11search9,3
turn12search1,1
turn12search3,1
turn12search6,1
turn13search3,1
turn13search7,1
turn14search0,1
turn14search1,1
turn14search2,1
turn15search0,1
turn15search4,1
turn16search0,2
turn16search1,2
turn16search11,1
turn16search12,1
turn16search3,2
turn16search5,1
turn16search9,1
turn17search0,9
turn17search1,2
turn17search9,1
turn19search10,5
turn19search13,4
turn19search2,2
turn19search7,3
turn19search8,2
turn1search10,1
turn1search15,2
turn1search7,3
turn20search0,3
turn20search1,1
turn20search12,1
turn20search14,3
turn20search15,6
turn20search2,4
turn20search3,1
turn20search4,2
turn20search5,3
turn21search0,1
turn21search1,4
turn21search10,3
turn21search11,3
turn21search13,5
turn21search15,3
turn21search17,1
turn21search2,4
turn21search4,2
turn21search5,1
turn21search9,1
turn22search0,4
turn22search1,3
turn22search12,2
turn22search15,2
turn22search3,2
turn22search4,4
turn22search7,6
turn2search10,2
turn2search11,5
turn2search12,4
turn2search2,1
turn3search11,3
turn3search12,1
turn3search14,5
turn3search18,1
turn3search2,2
turn3search20,1
turn3search8,1
turn4search0,1
turn5search0,3
turn5search1,1
turn5search13,2
turn5search17,2
turn5search2,2
turn5search3,1
turn5search6,5
turn5search7,1
turn5search8,1
turn6search0,3
turn6search1,5
turn6search13,2
turn6search14,4
turn6search3,2
turn6search4,4
turn6search5,2
turn7search2,3
turn7search3,1
turn8search0,2
turn8search3,2
turn8search8,2
turn9search1,1
turn9search5,2
turn9search6,3
```

## 5. Current disposition

- Publisher metadata recovered: **No**.
- Source titles recovered from token mapping: **No**.
- Durable locations recovered from token mapping: **No**.
- Original access dates recovered from token mapping: **No**.
- Stable `SRC-NNNN` mappings for these 99 tokens: **None yet**.
- Implementation evidence status: **BLOCKED where a requirement depends on one of these tokens as its sole external support**.

The register may be amended only with verified source metadata and an evidence/change record. The original token and report location should remain in the audit trail after resolution.

