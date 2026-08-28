#!/usr/bin/env python3
"""Fail-closed Phase 2 governance-contract audit.

This tool is deliberately read-only.  It independently inventories governance
material that sits outside the requirement extractor's Section 14-Appendix L
requirement scope, and it consumes the extractor in memory for requirement,
reference, area, and Appendix H coverage checks.  It never updates an evidence
ledger, changes a truth state, or grants verification/acceptance credit.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import Counter
from pathlib import Path
from typing import Any, Iterable, Sequence

import extract_phase2_requirements as extractor


EXPECTED_DIRECTIVE_SHA256 = (
    "938A0958F0CF15739A2DC8ED674F7C9F25D531DCE32CCA6A4CEEE5D638E68536"
)
EXPECTED_COUNTS = {
    "requirementDefinitionOccurrences": 937,
    "uniqueRequirementCount": 937,
    "normativeRecordCount": 160,
    "areaCount": 52,
    "verificationRecordCount": 44,
}
EXPECTED_KEYWORD_COUNTS = {
    "MUST": 860,
    "MUST NOT": 73,
    "MAY": 3,
    "SHOULD NOT": 1,
}
EXPECTED_REQUIREMENT_TRUTH_COUNTS = {
    "NOT_STARTED": 936,
    "IMPLEMENTED_UNVERIFIED": 1,
}
EXPECTED_VERIFICATION_TRUTH_COUNTS = {"NOT_STARTED": 44}
EXPECTED_REUSED_PHASE1_IDS = (
    "PES-ACC-0008",
    "PES-ACC-0009",
    "PES-ACC-0010",
    "PES-GOV-0021",
    "PES-GOV-0022",
    "PES-GOV-0023",
    "PES-GOV-0024",
    "PES-GOV-0025",
    "PES-GOV-0026",
    "PES-GOV-0027",
    "PES-GOV-0028",
    "PES-GOV-0029",
    "PES-GOV-0030",
    "PES-GOV-0031",
    "PES-GOV-0032",
    "PES-GOV-0033",
    "PES-GOV-0034",
    "PES-GOV-0035",
    "PES-GOV-0036",
    "PES-GOV-0037",
    "PES-GOV-0038",
    "PES-GOV-0039",
)
EXPECTED_REUSED_RETIRED_PHASE1_IDS = ("PES-GOV-0032",)

AUTHORITY_HEADING = (
    "Phase 2 Directive Contents",
    "Authority and Conflict Rule",
)
AUTHORITY_PARAGRAPHS = (
    "Use this order:",
    "Applicable law, binding licenses, and the immutable PhysicalUniverse prohibition.",
    "Scott's explicit product decisions and this Phase 2 start order.",
    "This Phase 2 directive for Phase 2 scope and execution.",
    "The accepted Phase 1 directive, corrective addendum, ADRs, and invariants.",
    "The frozen research report as technical and contextual evidence.",
    "Repository implementation choices, tickets, comments, and assumptions.",
    (
        "If authorities conflict, preserve the higher authority, block only the "
        "affected work, record the smallest decision needed, and continue unrelated "
        "implementation. The old combined-draft wording that described Phase 2 as "
        "authoring-only, required one cumulative directive file, or postponed the "
        "executable work program until Phase 4 is superseded and shall not be followed."
    ),
)

NORMATIVE_HEADING = ("Phase 2 Directive Contents", "Normative Keywords")
NORMATIVE_ROWS = (
    ("Keyword", "Meaning"),
    (
        "MUST / SHALL",
        "Required; violation blocks the affected package or Phase 2 gate.",
    ),
    (
        "MUST NOT / SHALL NOT",
        "Prohibited; presence blocks merge or acceptance.",
    ),
    (
        "SHOULD",
        "Expected unless an ADR proves an equal or stronger result without changing intent.",
    ),
    ("MAY", "Optional inside approved Phase 2 scope."),
    (
        "BLOCKED",
        "Cannot proceed without a decision/evidence; unaffected work continues.",
    ),
    (
        "DEFERRED",
        "Assigned to Phase 3 or 4; not implemented or represented as implemented.",
    ),
    (
        "EXCLUDED",
        "Permanently outside this product; never a backlog item.",
    ),
)
KEYWORD_FAMILY = {
    "MUST": "MUST / SHALL",
    "SHALL": "MUST / SHALL",
    "MUST NOT": "MUST NOT / SHALL NOT",
    "SHALL NOT": "MUST NOT / SHALL NOT",
    "SHOULD": "SHOULD",
    "SHOULD NOT": "SHOULD",
    "MAY": "MAY",
}

CLARIFICATION_HEADING = (
    "Appendix J. Phase 2 Change and Decision Ledger",
    "J.1 Adopted Phase 2 clarifications carried from Phase 1",
)
CLARIFICATION_ROW_HASHES = (
    ("Change", "0B654DD1BC14A7A113AFE877453DC538295FBD1B4D9B0A6613A15DEC60AA5DFB"),
    ("CR-0001", "74700B31F28263782BD00C4ED1AAF76B9E26B8617D0F4AE6BD4E21ED048F9AF8"),
    ("CR-0002", "C82EE43D89F3040836EE367D49F33926FABAD6900355408CABEFC05EAF618531"),
    ("CR-0003", "B3DF9F290D86E334824C198E4DBD0403FB7EB406B0FFA2AD0F14ADE9B9951030"),
    ("CR-0004", "6B1D22BAB6592E5E4920BA3347B9E85B66A02CBF7C73D2A92FC61569587A3266"),
    ("CR-0005", "C2BD5CA69F9F76A106E877CC5D730B7D3BD3F91220F5C851516588E341FD7506"),
    ("CR-0006", "29D15BCB64814620B4FD1F4FB3EECC8A357D99292A7F5E51D48F1A2E3B6A3A36"),
    ("CR-0007", "7A3AC81C1068CBBEE23018DEDE461365C6775F2723C5929D3522F8FDBCFAEE4D"),
    ("CR-0008", "77039186B58EBC58228C52C285959724251FAE5998EC10B022A106BF6B01CDBB"),
    ("CR-0009", "D0DD7A11439B405C7912ECDBDE67A9F36E40298F3507A8BDD1EB0FAAFF49B214"),
    ("CR-0010", "CF52E245A150C4EBFD29377B959147A240E3A142BB9B64F99CF361A7B9E35A10"),
    ("CR-0011", "ECCCD4E1E4BBA2372D47A0BB90DC369154F4FCD926A8588BD8E572BEDF03F9B3"),
    ("CR-0012", "A0339258A3291C4A11B6B9A1D69C5F9DC81CFB8623D49B625BE99D24CC98DEB1"),
    ("CR-0013", "C6F9AF5B5948CD37AE231A5EEEA60937268C0F3E70790C1D307B6D24D5FDC417"),
    ("CR-0014", "1B71B4AE90BEB9A46B338B4F462717B35FB64BB6E0E3331E4EE4AB287A49EABD"),
)
OPEN_QUESTION_DISPOSITION_HEADING = (
    "Appendix J. Phase 2 Change and Decision Ledger",
    "J.2 Phase 1 open-question disposition",
)
OPEN_QUESTION_DISPOSITION_ROW_HASHES = (
    ("Open question", "257B0062BF657D66120634E70FE7FE946B6AAA65CC48DA66B78D260637AE9D78"),
    ("OQ-0003", "E21057EE6EA9BD0C3B2776DB910944E53A75DDB977DCA0E94A1F6657937E0917"),
    ("OQ-0004", "AC366A6C015DAD2DE086D3398755B79A63C87581267BED950653D1AB6F012718"),
    ("OQ-0005", "B2459F6D3E40069D2F41CDDFAC8C49F4EA9AAAC7DF4E200AA6F897817B10316A"),
    ("OQ-0006", "AE483D050EAD6689F2093CF83E75D2EFAF3AF9DA2AF82AAE5FAA23579E1A8ED7"),
)

PHASE_RESERVATION_HEADING = (
    "14. Phase 2 Authorization, Outcome, and Execution Order",
    "14.6 Phase 2 reservations",
)
PHASE_RESERVATION_HASHES = (
    "36F58583DDD84B1389D4DC63AA29DE93EA326E4E7D5C9C5ABAAB2317A1EC38FE",
    "3C79CC7A457B25ED2CC7F2043B74F061EB6E1B3556ACFED0005F9A4D58999118",
)
CAPABILITY_HEADING = ("Appendix B. Allowed and Forbidden Capability Summary",)
CAPABILITY_ROW_HASHES = (
    ("Allowed inside production", "0BF36AC1FF785B307C649DCC6615EFFE9DAD8C27CC4AA68BF15B43C898AFDB7B"),
    ("Typed domain commands and queries", "F0CE0A1E2F31924CB9F352646650A4069C46DA1D88637C9BFAF9A2CCB48AECA9"),
    ("In-memory VirtualNetwork graph", "82AB0553E2414EEEC1009B24F44783F5C3FA085D2B8EC866EBA365B236A69371"),
    ("VirtualControllerId", "3ABBEF82C184D9A7BB98472923241641587CD1E9FB7DB7B28464169C2D55ACA6"),
    ("InternalTagBus and typed worker IPC", "B4A5CA0D8671B2E2C39FB46A7F670DB10450C7824FF8A4F850653A1DBB7F41AF"),
    ("User-initiated bounded fixed-local project open/save through FileAccessBroker", "F1A5C75E777F2E030B55E11EC009F8505CC61BBE6E6255193B61A9A39FF98E99"),
    ("Simulator-controlled virtual clock", "1D0D7C619A27AA4997C09CE0FDE6110488F4156407208497121A9BDD3A9F38EB"),
    ("Locally bundled assets and help", "F49E74E1FB88F12BAC678C3C1D29153BCBA9CAED4EB114896F1737E7E73DE5CF"),
    ("Simulator-native non-executable import/export", "86D270AA566C1FC034D59E4BC72F82D28D74D9C8AF73690E086B3298ADAB9006"),
    ("Capability-limited declarative DSL, if later approved", "2BBC4F45BC63AD47C514FB4C25A2A4B5155822B8B27111A296AAD347276CF858"),
)
EXCLUSION_HEADING = ('Appendix F. Master "Do Not Build This" Register',)
EXCLUSION_PARAGRAPH_HASHES = (
    "D1CA9572519346B4E89F9E0B78EBE54B4CC7599FB516AD7CBAFC49B31D656470",
    "DA0EDD879B78EB63BEEC88AB895BBE331E523A26C60B6F7D0D8566CF63D4C604",
    "C6A4944F0FAF22277309BB1508A5EDECA06758A46B126F4775EC2AD30F681DCD",
    "4FC7F7F19A55DC0A60C4304A4F2C46C465A86352A82DD33AE00CDC1ED44F6155",
    "D9995E8056B84427107A0A1C83D010A62FE8344FA053544753850FF5EBDCFFC5",
    "9E773E36138A936649706FCDF35B177183B304A7BE277E13038378D0DF904BA2",
    "B0BB2D91403A33C83E6920B0491EC27D8C4836BD77FA54A1278643411CE1FF80",
    "5FB76EF829EBA289DCBEA59C2CF9B163F376BAAE33525CD5F1DC6F28E030546E",
    "F28C79CE57E5FB9042DAEB439AE0CC220E868BCD88102995820ECD73DC8FB3E0",
    "30868CF42E8B29530389696604990A9DD49120650F17CAB023292ED39900E594",
    "35ACB798F4AD550A0048012BCFC6C0DCA15DD857ED1E89C6C24D55A868F5B539",
    "E2FDF2C65E85EDD4A7807F609C20163F8C579122B6C3E5D3262F45C275F48CF7",
    "ACA7E10AA366FB292F74F4DC41281B077FA808C783D15EACEC349D7F1EADFE95",
    "718FD225DE909FA2DEACB32791D4D1D68740CB6F046173EF9F99BD70905153D0",
    "E007CF2A7EFE53ADB52039B0946D579E6F2D1A0953DD2B05519C3E52E87BE075",
    "EF548DFF257726851E3952DE4AF23C01C1A790A46C51477AA6DB6EEB42960C07",
    "602171539A4670A0876387AD5CADCB77601311FCF07FBD89FE0929719DEF3731",
    "52CA4E721B355AF07DBC7E4153F6F5CB73F5AD054A0687FB9A54D590A099A31C",
    "6CB8F3B3FF964D263D22C5983C8AFC20031BA1C242E851A4CCEEA9350B50AD06",
    "385D27924482D2CE812D68DA815227D5E1A2BCE64D9411826E55996968F8C19F",
    "A864829A53FE7B64578BE83BF8CB87FCD5AB3272CF9EF5794AC2C25010379902",
    "39604560FCECDEE3852015EF2968C01CCB052671632C8F77FE4913E6FBFB5683",
    "82CD03303547BF621E86C37FCFEC922C78AABBC5BE4259499865C2B8BC277C22",
    "AE4C90B9A1DF3F4B9EA0D547F08116C951063A0E1A0E6F493D1B0847E75F8D76",
)
OPEN_DECISION_HEADING = (
    "Appendix K. Remaining Decisions, Risks, and Reserved Work",
    "K.1 Decisions still open",
)
OPEN_DECISION_ROW_HASHES = (
    ("ID", "D05B86D7808DF32392DBAEDE01FDEE9CBED97F9763DC314D729C1D7AC8ECC339"),
    ("OQ-0001", "C967CCA226A05FF85B2475F203A3A32D16A8DA462C2877214242FAE174487C36"),
    ("OQ-0002", "03A70B69CDCFE8697093E26BDDCA54506648019D2A03FAF15524665D8110F552"),
    ("OQ-0007", "60562C04707F849CBCD5F77D931569C4EA12436D088CC8F79550918DD8849816"),
    ("OQ-0008", "F0C83A0689BB5289ACD41B4AC71699CD0D05B9CE437E3DBE76FB52C91694C84A"),
    ("OQ-0009", "3900094134397BE77698B1F5ACFC0A3F387F5E1CBB19642063E8A68A60B3A7F9"),
    ("OQ-0010", "2E79971CAF566D25A0922A032FAD8CCECEB79CBC7E733FF8A6AC718E7C708D01"),
    ("OQ-0011", "9ABD52578A2623C207B4E369D20B3CEBC72E40A256EC03CAC1A5115E2CB603BD"),
)
K2_HEADING = (
    "Appendix K. Remaining Decisions, Risks, and Reserved Work",
    "K.2 Phase 2 risks and controls",
)
K2_RESERVATION_HASH = (
    "6C57EC791D11A46A1775250064E8A2FD5B7AB58BA97E2C6BF0AD9331E7510A8E"
)

DECISION_TEMPLATE_HEADING = ("Appendix D. Decision Request Template",)
DECISION_TEMPLATE_FIELDS = (
    "Decision ID:",
    "Date opened:",
    "Owner:",
    "Affected requirement IDs:",
    "Known facts and evidence:",
    "Unknown or conflict:",
    "Why this cannot be decided autonomously:",
    "Option A:",
    "Option B:",
    "Option C, if useful:",
    "Recommendation:",
    "Security/IP/data/migration/UX impacts:",
    "Exact approval or evidence needed:",
    "Unblocked work that will continue:",
    "Resolution:",
    "Approval and date:",
)
STOP_REQUIREMENT_HASHES = {
    "PES-GOV-0034": "923686909658800A47C764D75FB7D8EEB3CFCE61CDB35B8FBDB7F3A8ED910B8A",
    "PES-GOV-0038": "45FF9E3D20224FB8F281DEAB2F6CD9AC746C43C993B57ED3800EDE86EE38A5A4",
    "PES-GOV-0044": "C78883E70FA830C121C533DD3E931AC01BCB5CCD4781C93EE32773231A2E96CE",
    "PES-GOV-0048": "279AEC381B78A5D7F20E072601DF47365A78CB0F4273AEAB1772FF49FF7D6070",
    "PES-GOV-0049": "5A2F38762F95BDB719E89E3300C1464FB7895B208AD15637FD79D8270038501D",
}

EXIT_HEADING = ("Appendix L. Phase 2 Implementation Exit Record Template",)
EXIT_ROW_HASHES = (
    ("Exit item", "EC5CA32E29931769A5CFB8668F6774528EE3655497AA9A193D4F97D3045CB2D6"),
    ("Incoming Phase 1 baseline", "00FE692EA2D21D2EFFC470A07E3D2D83A2A8E313FD413952B02C909A25826FCB"),
    ("Phase 2 candidate", "2C3517C64F5B82FCCBFA78B0FD6D8F64041847BB1B6CE6E7134C3450A9DB49D3"),
    ("Clean checkout", "57DA7DC881CAE84E4E6D80033B4A066C29C4F81F0BCB93FB3BB14EA1A0000742"),
    ("Work packages", "7CC37E1B425A64C7FA32DDE3E7A2FEC06E394DFDF74C3CD8B2F7502334101CD5"),
    ("Acceptance journeys", "5C45439111913B97923CC984A39F375B9462C39CE7E31F4CC8381A4F38C0CF25"),
    ("G2 gates", "43C3D5B2201294D57BB94F2FEED46E37B48BC2AAB4090652A8E36221BC64D7E5"),
    ("Determinism", "8BDD872E139B9DA72D42B3317CF9B6E0789A5B61569B3228A983D337719E2400"),
    ("Isolation", "0AA05DBFBC2A948629A87B567DBBE56CEFAC4CF11CD8BC8345D4B9B04613517F"),
    ("Clean room", "30A45752F5CB40A549AC56F83BB9251197AF4E03C463F41AF9B34D9E58B90D4F"),
    ("Open defects", "33E61E416759AFFE68982D975E5BCE40AD7DC56DB2BC91FEE24F3D2D54001C9B"),
    ("Reserved work", "1119DAD57412CD53A1BCF9BB8A33BD1004645599BBDE102314B3F6A20D09093E"),
    ("Final verdict", "7DA85A82B29599FB14AC1ECBEF3EA92CCF725CC8192917888EF33475B8F41B1C"),
)
EXIT_PREFILL_RULE = "Codex shall populate this record from the final clean candidate. Do not prefill PASS."
EXIT_HANDOFF_STOP_RULE = (
    "The final response is a handoff, not self-authorization. After it is delivered, stop."
)
ALLOWED_EXIT_VERDICTS = (
    "BLOCKED",
    "PHASE 2 IMPLEMENTATION CANDIDATE - AWAITING SCOTT ACCEPTANCE",
)

HARD_EXTRACTION_FINDINGS = (
    "duplicateRequirementDefinitions",
    "unknownAreaTokens",
    "unknownRequirementReferences",
    "unresolvedNormativePlaceholders",
    "duplicateVerificationRecords",
    "unknownVerificationAreaTokens",
    "unknownVerificationReferences",
    "orphanVerificationRecords",
    "requirementsWithoutVerificationCandidates",
)
REQUIREMENT_ID_RE = re.compile(r"\bPES-[A-Z][A-Z0-9]*-[0-9]{4}\b")


class GovernanceAuditError(RuntimeError):
    """Raised when the canonical inputs cannot be read or trusted."""


def sha256_text(value: str) -> str:
    """Return the uppercase SHA-256 of UTF-8 text."""

    return hashlib.sha256(value.encode("utf-8")).hexdigest().upper()


def row_hash(row: Sequence[str]) -> str:
    """Return the canonical hash for one visible table row."""

    return sha256_text("\t".join(row))


def _finding(
    findings: list[dict[str, Any]],
    code: str,
    category: str,
    detail: str,
    *,
    expected: Any | None = None,
    observed: Any | None = None,
) -> None:
    finding: dict[str, Any] = {
        "code": code,
        "category": category,
        "detail": detail,
    }
    if expected is not None:
        finding["expected"] = expected
    if observed is not None:
        finding["observed"] = observed
    findings.append(finding)


def _paragraphs_at(
    blocks: Iterable[extractor.DocumentBlock], heading: tuple[str, ...]
) -> list[extractor.ParagraphBlock]:
    result: list[extractor.ParagraphBlock] = []
    for block in blocks:
        if not isinstance(block, extractor.ParagraphBlock):
            continue
        if block.heading_path != heading:
            continue
        if not block.exact_text.strip() or block.exact_text == heading[-1]:
            continue
        result.append(block)
    return result


def _tables_at(
    blocks: Iterable[extractor.DocumentBlock], heading: tuple[str, ...]
) -> list[extractor.TableBlock]:
    return [
        block
        for block in blocks
        if isinstance(block, extractor.TableBlock) and block.heading_path == heading
    ]


def _paragraph_inventory(blocks: Sequence[extractor.ParagraphBlock]) -> list[dict[str, Any]]:
    return [
        {
            "bodyBlockIndex": block.body_block_index,
            "paragraphOrdinal": block.paragraph_ordinal,
            "text": block.exact_text,
            "textSha256": sha256_text(block.exact_text),
        }
        for block in blocks
    ]


def _audit_exact_paragraphs(
    *,
    blocks: Sequence[extractor.DocumentBlock],
    heading: tuple[str, ...],
    expected_hashes: Sequence[str],
    code_prefix: str,
    category: str,
    findings: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    paragraphs = _paragraphs_at(blocks, heading)
    observed_hashes = [sha256_text(block.exact_text) for block in paragraphs]
    if len(paragraphs) != len(expected_hashes):
        _finding(
            findings,
            f"{code_prefix}-COUNT",
            category,
            "The canonical paragraph inventory is incomplete or duplicated.",
            expected=len(expected_hashes),
            observed=len(paragraphs),
        )
    if observed_hashes != list(expected_hashes):
        _finding(
            findings,
            f"{code_prefix}-DRIFT",
            category,
            "The ordered canonical paragraph hashes drifted.",
            expected=list(expected_hashes),
            observed=observed_hashes,
        )
    return _paragraph_inventory(paragraphs)


def _audit_table(
    *,
    blocks: Sequence[extractor.DocumentBlock],
    heading: tuple[str, ...],
    expected_rows: Sequence[tuple[str, str]],
    code_prefix: str,
    category: str,
    findings: list[dict[str, Any]],
) -> dict[str, Any]:
    tables = _tables_at(blocks, heading)
    if len(tables) != 1:
        _finding(
            findings,
            f"{code_prefix}-TABLE-COUNT",
            category,
            "Expected exactly one canonical table.",
            expected=1,
            observed=len(tables),
        )
    if not tables:
        return {"headingPath": list(heading), "rows": []}

    table = tables[0]
    rows = [
        {
            "rowIndex": index,
            "key": row[0] if row else "",
            "columns": list(row),
            "rowSha256": row_hash(row),
        }
        for index, row in enumerate(table.rows)
    ]
    observed_keys = [row["key"] for row in rows]
    expected_keys = [item[0] for item in expected_rows]
    duplicate_keys = sorted(
        key for key, count in Counter(observed_keys[1:]).items() if count > 1
    )
    if duplicate_keys:
        _finding(
            findings,
            f"{code_prefix}-DUPLICATE-KEY",
            category,
            "The canonical table contains duplicate record keys.",
            observed=duplicate_keys,
        )
    if observed_keys != expected_keys:
        _finding(
            findings,
            f"{code_prefix}-ROW-KEYS",
            category,
            "The ordered canonical table keys are incomplete, duplicated, or reordered.",
            expected=expected_keys,
            observed=observed_keys,
        )
    observed_hashes = [row["rowSha256"] for row in rows]
    expected_hashes = [item[1] for item in expected_rows]
    if observed_hashes != expected_hashes:
        _finding(
            findings,
            f"{code_prefix}-ROW-DRIFT",
            category,
            "One or more canonical table rows drifted.",
            expected=expected_hashes,
            observed=observed_hashes,
        )
    return {
        "headingPath": list(heading),
        "bodyBlockIndex": table.body_block_index,
        "tableOrdinal": table.table_ordinal,
        "rows": rows,
    }


def _audit_authority(
    blocks: Sequence[extractor.DocumentBlock], findings: list[dict[str, Any]]
) -> dict[str, Any]:
    paragraphs = _paragraphs_at(blocks, AUTHORITY_HEADING)
    observed = [block.exact_text for block in paragraphs]
    if observed != list(AUTHORITY_PARAGRAPHS):
        _finding(
            findings,
            "GOV-AUTHORITY-ORDER",
            "authorityHierarchy",
            "Authority precedence or its conflict rule is omitted, duplicated, reordered, or changed.",
            expected=list(AUTHORITY_PARAGRAPHS),
            observed=observed,
        )
    return {
        "headingPath": list(AUTHORITY_HEADING),
        "orderedParagraphs": _paragraph_inventory(paragraphs),
    }


def _audit_normative_vocabulary(
    blocks: Sequence[extractor.DocumentBlock],
    requirements: Sequence[dict[str, Any]],
    findings: list[dict[str, Any]],
) -> dict[str, Any]:
    expected = tuple((row[0], row_hash(row)) for row in NORMATIVE_ROWS)
    table = _audit_table(
        blocks=blocks,
        heading=NORMATIVE_HEADING,
        expected_rows=expected,
        code_prefix="GOV-NORMATIVE",
        category="normativeVocabulary",
        findings=findings,
    )

    counts: Counter[str] = Counter()
    family_counts: Counter[str] = Counter()
    mismatches: list[dict[str, Any]] = []
    undefined: list[dict[str, Any]] = []
    for record in requirements:
        requirement_id = record.get("id")
        requirement_text = record.get("requirementText")
        recorded_keyword = record.get("normativeKeyword")
        if not isinstance(requirement_text, str) or not isinstance(
            recorded_keyword, str
        ):
            undefined.append(
                {"requirementId": requirement_id, "keyword": recorded_keyword}
            )
            continue
        recomputed = extractor.keyword_from_requirement(requirement_text)
        if recomputed != recorded_keyword:
            mismatches.append(
                {
                    "requirementId": requirement_id,
                    "recorded": recorded_keyword,
                    "recomputed": recomputed,
                }
            )
        family = KEYWORD_FAMILY.get(recorded_keyword)
        if family is None:
            undefined.append(
                {"requirementId": requirement_id, "keyword": recorded_keyword}
            )
            continue
        counts[recorded_keyword] += 1
        family_counts[family] += 1

    if mismatches:
        _finding(
            findings,
            "GOV-NORMATIVE-CLASSIFICATION",
            "normativeVocabulary",
            "Requirement keyword metadata does not match its exact requirement text.",
            observed=mismatches,
        )
    if undefined:
        _finding(
            findings,
            "GOV-NORMATIVE-UNDEFINED",
            "normativeVocabulary",
            "A requirement uses no defined normative keyword family.",
            observed=undefined,
        )
    if dict(counts) != EXPECTED_KEYWORD_COUNTS:
        _finding(
            findings,
            "GOV-NORMATIVE-DISTRIBUTION",
            "normativeVocabulary",
            "The independent requirement keyword inventory drifted.",
            expected=EXPECTED_KEYWORD_COUNTS,
            observed=dict(sorted(counts.items())),
        )
    return {
        "definitionTable": table,
        "requirementKeywordCounts": dict(sorted(counts.items())),
        "requirementKeywordFamilyCounts": dict(sorted(family_counts.items())),
        "unclassifiedRequirementIds": [
            item["requirementId"] for item in undefined
        ],
    }


def _audit_extraction_contract(
    outputs: dict[str, dict[str, Any]], findings: list[dict[str, Any]]
) -> dict[str, Any]:
    requirements_output = outputs["requirements"]
    verification_output = outputs["verification"]
    audit_output = outputs["audit"]
    requirements = requirements_output.get("requirements", [])
    verification_records = verification_output.get("verificationRecords", [])
    extraction_findings = audit_output.get("findings", {})

    observed_counts = audit_output.get("counts", {})
    count_drift = {
        key: {"expected": value, "observed": observed_counts.get(key)}
        for key, value in EXPECTED_COUNTS.items()
        if observed_counts.get(key) != value
    }
    if count_drift:
        _finding(
            findings,
            "GOV-EXTRACTION-COUNTS",
            "requirementCoverage",
            "Canonical requirement/area/verification counts drifted.",
            observed=count_drift,
        )

    actual_record_counts = {
        "uniqueRequirementCount": len(requirements),
        "areaCount": len(requirements_output.get("areas", [])),
        "verificationRecordCount": len(verification_records),
        "mappingSkeletonCount": len(
            verification_output.get("requirementMappingSkeleton", [])
        ),
    }
    expected_record_counts = {
        "uniqueRequirementCount": EXPECTED_COUNTS["uniqueRequirementCount"],
        "areaCount": EXPECTED_COUNTS["areaCount"],
        "verificationRecordCount": EXPECTED_COUNTS["verificationRecordCount"],
        "mappingSkeletonCount": EXPECTED_COUNTS["uniqueRequirementCount"],
    }
    if actual_record_counts != expected_record_counts:
        _finding(
            findings,
            "GOV-EXTRACTION-ARRAY-COUNTS",
            "requirementCoverage",
            "Registry arrays do not contain the complete canonical inventory.",
            expected=expected_record_counts,
            observed=actual_record_counts,
        )

    requirement_ids = [record.get("id") for record in requirements]
    duplicate_requirement_ids = sorted(
        requirement_id
        for requirement_id, count in Counter(requirement_ids).items()
        if requirement_id is not None and count > 1
    )
    if duplicate_requirement_ids:
        _finding(
            findings,
            "GOV-REQUIREMENT-DUPLICATE",
            "requirementCoverage",
            "Requirement records contain duplicate IDs.",
            observed=duplicate_requirement_ids,
        )

    verification_ids = [record.get("verificationId") for record in verification_records]
    duplicate_verification_ids = sorted(
        verification_id
        for verification_id, count in Counter(verification_ids).items()
        if verification_id is not None and count > 1
    )
    if duplicate_verification_ids:
        _finding(
            findings,
            "GOV-VERIFICATION-DUPLICATE",
            "requirementCoverage",
            "Appendix H records contain duplicate IDs.",
            observed=duplicate_verification_ids,
        )

    nonempty_hard_findings = {
        key: extraction_findings.get(key)
        for key in HARD_EXTRACTION_FINDINGS
        if extraction_findings.get(key)
    }
    p2_entry_errors = [
        item
        for item in extraction_findings.get("p2EntryEvidence", [])
        if item.get("severity") == "ERROR"
    ]
    if p2_entry_errors:
        nonempty_hard_findings["p2EntryEvidence"] = p2_entry_errors
    if nonempty_hard_findings:
        _finding(
            findings,
            "GOV-EXTRACTION-HARD-FINDINGS",
            "requirementCoverage",
            "The deterministic extraction audit has unresolved hard findings.",
            observed=nonempty_hard_findings,
        )

    reused_ids = tuple(
        item.get("requirementId")
        for item in extraction_findings.get("reusedPhase1Ids", [])
    )
    retired_ids = tuple(
        item.get("requirementId")
        for item in extraction_findings.get("reusedRetiredPhase1Ids", [])
    )
    if reused_ids != EXPECTED_REUSED_PHASE1_IDS or retired_ids != EXPECTED_REUSED_RETIRED_PHASE1_IDS:
        _finding(
            findings,
            "GOV-REUSED-ID-INVENTORY-DRIFT",
            "requirementCoverage",
            "The reported cross-phase reused-ID inventory drifted.",
            expected={
                "phase1Ids": list(EXPECTED_REUSED_PHASE1_IDS),
                "retiredPhase1Ids": list(EXPECTED_REUSED_RETIRED_PHASE1_IDS),
            },
            observed={
                "phase1Ids": list(reused_ids),
                "retiredPhase1Ids": list(retired_ids),
            },
        )

    requirement_truth_counts = Counter(
        record.get("truthState") for record in requirements
    )
    verification_truth_counts = Counter(
        record.get("truthState") for record in verification_records
    )
    if dict(requirement_truth_counts) != EXPECTED_REQUIREMENT_TRUTH_COUNTS:
        _finding(
            findings,
            "GOV-REQUIREMENT-TRUTH-DRIFT",
            "truthCredit",
            "Extraction truth states changed or verification was inferred.",
            expected=EXPECTED_REQUIREMENT_TRUTH_COUNTS,
            observed=dict(requirement_truth_counts),
        )
    if dict(verification_truth_counts) != EXPECTED_VERIFICATION_TRUTH_COUNTS:
        _finding(
            findings,
            "GOV-VERIFICATION-TRUTH-DRIFT",
            "truthCredit",
            "Appendix H truth states changed or verification was inferred.",
            expected=EXPECTED_VERIFICATION_TRUTH_COUNTS,
            observed=dict(verification_truth_counts),
        )

    gov_verification = [
        record
        for record in verification_records
        if record.get("verificationId") == "VER-GOV-0001"
    ]
    if len(gov_verification) != 1:
        _finding(
            findings,
            "GOV-VER-GOV-0001-COUNT",
            "requirementCoverage",
            "VER-GOV-0001 must occur exactly once in Appendix H.",
            expected=1,
            observed=len(gov_verification),
        )

    stop_rules: list[dict[str, Any]] = []
    requirements_by_id = {
        record.get("id"): record for record in requirements if record.get("id")
    }
    for requirement_id, expected_hash in STOP_REQUIREMENT_HASHES.items():
        record = requirements_by_id.get(requirement_id)
        observed_hash = record.get("textSha256") if record else None
        stop_rules.append(
            {
                "requirementId": requirement_id,
                "textSha256": observed_hash,
                "exactText": record.get("exactText") if record else None,
            }
        )
        if observed_hash != expected_hash:
            _finding(
                findings,
                "GOV-STOP-RULE-DRIFT",
                "decisionAndStopRules",
                "A mandatory phase/decision/stop requirement is missing or changed.",
                expected={"requirementId": requirement_id, "textSha256": expected_hash},
                observed={"requirementId": requirement_id, "textSha256": observed_hash},
            )

    return {
        "counts": {key: observed_counts.get(key) for key in EXPECTED_COUNTS},
        "hardFindings": nonempty_hard_findings,
        "reportedReusedPhase1Ids": list(reused_ids),
        "reportedReusedRetiredPhase1Ids": list(retired_ids),
        "requirementTruthCounts": dict(sorted(requirement_truth_counts.items())),
        "verificationTruthCounts": dict(sorted(verification_truth_counts.items())),
        "stopAndReservationRequirements": stop_rules,
    }


def audit_contract(
    *,
    blocks: Sequence[extractor.DocumentBlock],
    outputs: dict[str, dict[str, Any]],
    source_path: str,
    source_sha256: str,
    known_requirement_ids: set[str] | None = None,
) -> dict[str, Any]:
    """Audit parsed canonical blocks and in-memory extractor outputs.

    This lower-level entry point exists so tests can inject omissions,
    duplicates, and text drift without modifying the protected DOCX.
    """

    findings: list[dict[str, Any]] = []
    if source_sha256 != EXPECTED_DIRECTIVE_SHA256:
        _finding(
            findings,
            "GOV-SOURCE-HASH",
            "sourceAuthority",
            "The directive is not the pinned canonical Phase 2 source.",
            expected=EXPECTED_DIRECTIVE_SHA256,
            observed=source_sha256,
        )

    requirements = outputs["requirements"].get("requirements", [])
    extraction_inventory = _audit_extraction_contract(outputs, findings)
    authority_inventory = _audit_authority(blocks, findings)
    normative_inventory = _audit_normative_vocabulary(
        blocks, requirements, findings
    )

    clarification_inventory = _audit_table(
        blocks=blocks,
        heading=CLARIFICATION_HEADING,
        expected_rows=CLARIFICATION_ROW_HASHES,
        code_prefix="GOV-CLARIFICATION",
        category="clarificationLedger",
        findings=findings,
    )
    disposition_inventory = _audit_table(
        blocks=blocks,
        heading=OPEN_QUESTION_DISPOSITION_HEADING,
        expected_rows=OPEN_QUESTION_DISPOSITION_ROW_HASHES,
        code_prefix="GOV-QUESTION-DISPOSITION",
        category="clarificationLedger",
        findings=findings,
    )

    if known_requirement_ids is not None:
        unknown_ledger_refs: list[dict[str, Any]] = []
        for row in clarification_inventory.get("rows", [])[1:]:
            columns = row.get("columns", [])
            affected = columns[1] if len(columns) > 1 else ""
            for requirement_id in REQUIREMENT_ID_RE.findall(affected):
                if requirement_id not in known_requirement_ids:
                    unknown_ledger_refs.append(
                        {"changeId": row.get("key"), "requirementId": requirement_id}
                    )
        if unknown_ledger_refs:
            _finding(
                findings,
                "GOV-CLARIFICATION-UNKNOWN-REFERENCE",
                "clarificationLedger",
                "The clarification ledger cites unknown requirement IDs.",
                observed=unknown_ledger_refs,
            )

    phase_reservation_inventory = _audit_exact_paragraphs(
        blocks=blocks,
        heading=PHASE_RESERVATION_HEADING,
        expected_hashes=PHASE_RESERVATION_HASHES,
        code_prefix="GOV-PHASE-RESERVATION",
        category="phaseReservationsAndProhibitions",
        findings=findings,
    )
    capability_inventory = _audit_table(
        blocks=blocks,
        heading=CAPABILITY_HEADING,
        expected_rows=CAPABILITY_ROW_HASHES,
        code_prefix="GOV-CAPABILITY",
        category="phaseReservationsAndProhibitions",
        findings=findings,
    )
    exclusion_inventory = _audit_exact_paragraphs(
        blocks=blocks,
        heading=EXCLUSION_HEADING,
        expected_hashes=EXCLUSION_PARAGRAPH_HASHES,
        code_prefix="GOV-EXCLUSION",
        category="phaseReservationsAndProhibitions",
        findings=findings,
    )
    open_decision_inventory = _audit_table(
        blocks=blocks,
        heading=OPEN_DECISION_HEADING,
        expected_rows=OPEN_DECISION_ROW_HASHES,
        code_prefix="GOV-OPEN-DECISION",
        category="decisionAndStopRules",
        findings=findings,
    )
    k2_inventory = _audit_exact_paragraphs(
        blocks=blocks,
        heading=K2_HEADING,
        expected_hashes=(K2_RESERVATION_HASH,),
        code_prefix="GOV-K2-RESERVATION",
        category="phaseReservationsAndProhibitions",
        findings=findings,
    )

    decision_paragraphs = _paragraphs_at(blocks, DECISION_TEMPLATE_HEADING)
    decision_fields = (
        decision_paragraphs[0].exact_text.splitlines()
        if len(decision_paragraphs) == 1
        else []
    )
    if len(decision_paragraphs) != 1 or tuple(decision_fields) != DECISION_TEMPLATE_FIELDS:
        _finding(
            findings,
            "GOV-DECISION-TEMPLATE-DRIFT",
            "decisionAndStopRules",
            "The mandatory decision-request fields are missing, duplicated, reordered, or changed.",
            expected=list(DECISION_TEMPLATE_FIELDS),
            observed=decision_fields,
        )

    exit_table = _audit_table(
        blocks=blocks,
        heading=EXIT_HEADING,
        expected_rows=EXIT_ROW_HASHES,
        code_prefix="GOV-EXIT",
        category="terminalVerdict",
        findings=findings,
    )
    exit_paragraphs = _paragraphs_at(blocks, EXIT_HEADING)
    exit_text = [block.exact_text for block in exit_paragraphs]
    expected_exit_text = [EXIT_PREFILL_RULE, EXIT_HANDOFF_STOP_RULE]
    if exit_text != expected_exit_text:
        _finding(
            findings,
            "GOV-EXIT-HANDOFF-RULE-DRIFT",
            "terminalVerdict",
            "The no-prefill and handoff-stop rules are missing, duplicated, reordered, or changed.",
            expected=expected_exit_text,
            observed=exit_text,
        )
    verdict_rows = [
        row for row in exit_table.get("rows", []) if row.get("key") == "Final verdict"
    ]
    observed_verdicts: tuple[str, ...] = ()
    if len(verdict_rows) == 1 and len(verdict_rows[0].get("columns", [])) == 2:
        observed_verdicts = tuple(verdict_rows[0]["columns"][1].split(" or "))
    if observed_verdicts != ALLOWED_EXIT_VERDICTS:
        _finding(
            findings,
            "GOV-EXIT-VERDICT-LANGUAGE",
            "terminalVerdict",
            "The terminal verdict vocabulary is incomplete or changed.",
            expected=list(ALLOWED_EXIT_VERDICTS),
            observed=list(observed_verdicts),
        )

    status = "PASS" if not findings else "BLOCKED"
    return {
        "schemaVersion": 1,
        "auditKind": "PHASE_2_GOVERNANCE_COMPLETENESS_AUDIT",
        "auditStatus": status,
        "scope": "VER-GOV-0001 governance-contract completeness only",
        "source": {"path": source_path, "sha256": source_sha256},
        "truthPolicy": {
            "verificationCreditGranted": False,
            "phase2ExitVerdictIssued": False,
            "scottAcceptanceInferred": False,
            "generatedEvidenceMutated": False,
        },
        "counts": {
            "findingCount": len(findings),
            "clarificationCount": max(
                0, len(clarification_inventory.get("rows", [])) - 1
            ),
            "resolvedOpenQuestionCount": max(
                0, len(disposition_inventory.get("rows", [])) - 1
            ),
            "openDecisionCount": max(
                0, len(open_decision_inventory.get("rows", [])) - 1
            ),
            "exclusionParagraphCount": len(exclusion_inventory),
        },
        "inventory": {
            "authorityHierarchy": authority_inventory,
            "normativeVocabulary": normative_inventory,
            "requirementCoverage": extraction_inventory,
            "clarificationLedger": {
                "adoptedClarifications": clarification_inventory,
                "openQuestionDispositions": disposition_inventory,
            },
            "phaseReservationsAndProhibitions": {
                "phaseAllocation": phase_reservation_inventory,
                "capabilitySummary": capability_inventory,
                "excludedNotBacklog": exclusion_inventory,
                "remainingPhaseReservations": k2_inventory,
            },
            "decisionAndStopRules": {
                "decisionTemplateFields": decision_fields,
                "openDecisions": open_decision_inventory,
                "requirements": extraction_inventory[
                    "stopAndReservationRequirements"
                ],
            },
            "terminalVerdict": {
                "allowedExitVerdicts": list(ALLOWED_EXIT_VERDICTS),
                "prefillRule": EXIT_PREFILL_RULE,
                "handoffStopRule": EXIT_HANDOFF_STOP_RULE,
                "exitRecord": exit_table,
            },
        },
        "reportedNotices": [
            {
                "code": "GOV-REUSED-PHASE1-IDS",
                "disposition": "REPORTED_NOT_WAIVED_AND_NO_VERIFICATION_CREDIT",
                "requirementIds": extraction_inventory[
                    "reportedReusedPhase1Ids"
                ],
            },
            {
                "code": "GOV-REUSED-RETIRED-PHASE1-IDS",
                "disposition": "REPORTED_NOT_WAIVED_AND_NO_VERIFICATION_CREDIT",
                "requirementIds": extraction_inventory[
                    "reportedReusedRetiredPhase1Ids"
                ],
            },
        ],
        "findings": findings,
    }


def _load_phase1_ids(path: Path) -> set[str]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise GovernanceAuditError(
            f"Unable to read the Phase 1 requirement registry: {exc}"
        ) from exc
    records = value.get("requirements")
    if not isinstance(records, list):
        raise GovernanceAuditError(
            "The Phase 1 requirement registry has no requirements array"
        )
    return {
        record["id"]
        for record in records
        if isinstance(record, dict) and isinstance(record.get("id"), str)
    }


def build_audit(
    *,
    root: Path,
    directive_path: Path,
    phase1_registry_path: Path,
    p2_00_evidence_path: Path,
) -> dict[str, Any]:
    """Build the read-only audit from canonical repository inputs."""

    root = root.resolve(strict=True)
    directive_path = directive_path.resolve(strict=True)
    observed_sha256 = extractor.sha256_file(directive_path)
    if observed_sha256 != EXPECTED_DIRECTIVE_SHA256:
        raise GovernanceAuditError(
            "Canonical Phase 2 directive SHA-256 mismatch: "
            f"expected {EXPECTED_DIRECTIVE_SHA256}, got {observed_sha256}"
        )
    try:
        blocks = extractor.parse_docx(directive_path)
        outputs = extractor.build_outputs(
            root=root,
            directive_path=directive_path,
            phase1_registry_path=phase1_registry_path,
            p2_00_evidence_path=p2_00_evidence_path,
        )
    except (extractor.ExtractionError, OSError) as exc:
        raise GovernanceAuditError(str(exc)) from exc

    known_ids = _load_phase1_ids(phase1_registry_path)
    known_ids.update(
        record["id"]
        for record in outputs["requirements"]["requirements"]
        if isinstance(record.get("id"), str)
    )
    return audit_contract(
        blocks=blocks,
        outputs=outputs,
        source_path=extractor.project_path(root, directive_path),
        source_sha256=observed_sha256,
        known_requirement_ids=known_ids,
    )


def _resolve(root: Path, value: Path) -> Path:
    return value if value.is_absolute() else root / value


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Read-only VER-GOV-0001 governance completeness audit"
    )
    parser.add_argument("--root", type=Path)
    parser.add_argument("--source", type=Path, default=Path(extractor.DEFAULT_DIRECTIVE_PATH))
    parser.add_argument(
        "--phase1-registry",
        type=Path,
        default=Path(extractor.DEFAULT_PHASE1_REGISTRY_PATH),
    )
    parser.add_argument(
        "--p2-00-evidence",
        type=Path,
        default=Path(extractor.DEFAULT_P2_00_EVIDENCE_PATH),
    )
    parser.add_argument(
        "--compact", action="store_true", help="Emit compact JSON instead of indented JSON"
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    root = (
        args.root.resolve(strict=True)
        if args.root is not None
        else Path(__file__).resolve(strict=True).parents[2]
    )
    try:
        report = build_audit(
            root=root,
            directive_path=_resolve(root, args.source),
            phase1_registry_path=_resolve(root, args.phase1_registry),
            p2_00_evidence_path=_resolve(root, args.p2_00_evidence),
        )
    except (GovernanceAuditError, OSError) as exc:
        print(f"ERROR PHASE2-GOVERNANCE-AUDIT {exc}", file=sys.stderr)
        return 2

    json.dump(
        report,
        sys.stdout,
        ensure_ascii=False,
        indent=None if args.compact else 2,
        sort_keys=False,
    )
    sys.stdout.write("\n")
    return 0 if report["auditStatus"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
