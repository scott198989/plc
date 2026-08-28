#!/usr/bin/env python3
"""Extract and audit the canonical Phase 2 directive deterministically.

The extractor intentionally uses only the Python standard library.  It reads
visible WordprocessingML from the canonical DOCX, preserves exact requirement
and normative-support text, and emits candidate registries without claiming
that product behavior is verified.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable
from xml.etree import ElementTree as ET
from zipfile import BadZipFile, ZipFile


EXPECTED_DIRECTIVE_SHA256 = (
    "938A0958F0CF15739A2DC8ED674F7C9F25D531DCE32CCA6A4CEEE5D638E68536"
)
DEFAULT_DIRECTIVE_PATH = (
    "References for Codex from Scott/"
    "PLC Engineering Simulator - Codex Master Implementation Directive - "
    "Phase 2 of 4 - Runnable PLC Engineering Core.docx"
)
DEFAULT_PHASE1_REGISTRY_PATH = "requirements/phase1-requirements.json"
DEFAULT_P2_00_EVIDENCE_PATH = "evidence/phase2/P2-00_ENTRY_GATE.json"
OUTPUT_PATHS = {
    "requirements": "requirements/phase2-requirements.json",
    "verification": "requirements/phase2-verification-catalog.json",
    "audit": "requirements/phase2-extraction-audit.json",
}

W_NS = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
W = f"{{{W_NS}}}"
REQUIREMENT_DEFINITION_RE = re.compile(
    r"^\[(PES-([A-Z][A-Z0-9]*)-([0-9]{4}))\]\s+(.*)$", re.DOTALL
)
REQUIREMENT_ID_RE = re.compile(r"\bPES-[A-Z][A-Z0-9]*-[0-9]{4}\b")
VERIFICATION_ID_RE = re.compile(r"\bVER-[A-Z][A-Z0-9]*-[0-9]{4}\b")
AREA_TOKEN_RE = re.compile(r"^[A-Z][A-Z0-9]{1,7}$")
HEADING_RE = re.compile(r"^Heading\s*([1-9])$", re.IGNORECASE)
PLACEHOLDER_RE = re.compile(
    r"(?:\bTBD\b|\bTODO\b|\bFIXME\b|\bXXX\b|"
    r"<\s*placeholder\s*>|\{\{[^{}\n]+\}\}|\[\s*INSERT[^\]]*\])",
    re.IGNORECASE,
)
DEFAULT_RULE_RE = re.compile(
    r"\b(?:default|baseline|exact|exactly|fixed|controlling rule|"
    r"shall|must|forbidden|permitted|allowed|defined as|consists of|includes)\b",
    re.IGNORECASE,
)

P2_00_IMPLEMENTED_REQUIREMENTS = {"PES-GOV-0040"}


class ExtractionError(RuntimeError):
    """Raised when the canonical source cannot be trusted or parsed."""


@dataclass(frozen=True)
class ParagraphBlock:
    body_block_index: int
    paragraph_ordinal: int
    style_id: str
    style_name: str
    heading_path: tuple[str, ...]
    exact_text: str
    heading_level: int | None


@dataclass(frozen=True)
class TableBlock:
    body_block_index: int
    table_ordinal: int
    heading_path: tuple[str, ...]
    rows: tuple[tuple[str, ...], ...]


DocumentBlock = ParagraphBlock | TableBlock


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest().upper()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest().upper()


def json_bytes(value: Any) -> bytes:
    return (json.dumps(value, indent=2, ensure_ascii=False) + "\n").encode("utf-8")


def project_path(root: Path, path: Path) -> str:
    try:
        return path.resolve(strict=False).relative_to(root.resolve(strict=True)).as_posix()
    except ValueError:
        return path.resolve(strict=False).as_posix()


def visible_text(paragraph: ET.Element) -> str:
    """Return displayed paragraph text in document order.

    Field instructions and tracked deletions are not displayed.  Tracked
    insertions, hyperlinks, smart tags, and ordinary runs remain visible.
    """

    parts: list[str] = []

    def walk(node: ET.Element, deleted: bool = False) -> None:
        local_deleted = deleted or node.tag == f"{W}del"
        if local_deleted:
            return
        if node.tag == f"{W}t":
            parts.append(node.text or "")
            return
        if node.tag == f"{W}tab":
            parts.append("\t")
            return
        if node.tag in {f"{W}br", f"{W}cr"}:
            parts.append("\n")
            return
        if node.tag == f"{W}noBreakHyphen":
            parts.append("\u2011")
            return
        if node.tag == f"{W}softHyphen":
            parts.append("\u00ad")
            return
        if node.tag in {f"{W}instrText", f"{W}delText"}:
            return
        for child in list(node):
            walk(child, local_deleted)

    walk(paragraph)
    return "".join(parts)


def style_map(styles_xml: bytes) -> dict[str, str]:
    root = ET.fromstring(styles_xml)
    result: dict[str, str] = {}
    for style in root.findall(f".//{W}style"):
        style_id = style.get(f"{W}styleId") or ""
        name = style.find(f"{W}name")
        result[style_id] = (
            name.get(f"{W}val") if name is not None else style_id
        ) or style_id
    return result


def paragraph_style(paragraph: ET.Element, styles: dict[str, str]) -> tuple[str, str]:
    style = paragraph.find(f"./{W}pPr/{W}pStyle")
    style_id = style.get(f"{W}val") if style is not None else "Normal"
    style_id = style_id or "Normal"
    return style_id, styles.get(style_id, style_id)


def heading_level(style_id: str, style_name: str) -> int | None:
    for candidate in (style_name, style_id):
        match = HEADING_RE.fullmatch(candidate.replace("_", " "))
        if match:
            return int(match.group(1))
    return None


def table_rows(table: ET.Element) -> tuple[tuple[str, ...], ...]:
    rows: list[tuple[str, ...]] = []
    for row in table.findall(f"./{W}tr"):
        cells: list[str] = []
        for cell in row.findall(f"./{W}tc"):
            paragraphs = [visible_text(p) for p in cell.findall(f".//{W}p")]
            cells.append("\n".join(text for text in paragraphs if text != ""))
        rows.append(tuple(cells))
    return tuple(rows)


def parse_docx(path: Path) -> list[DocumentBlock]:
    try:
        with ZipFile(path) as package:
            document_xml = package.read("word/document.xml")
            styles_xml = package.read("word/styles.xml")
    except (BadZipFile, KeyError, OSError) as exc:
        raise ExtractionError(f"Unable to read canonical DOCX package: {exc}") from exc

    try:
        document = ET.fromstring(document_xml)
        styles = style_map(styles_xml)
    except ET.ParseError as exc:
        raise ExtractionError(f"Unable to parse canonical WordprocessingML: {exc}") from exc

    body = document.find(f".//{W}body")
    if body is None:
        raise ExtractionError("Canonical DOCX has no WordprocessingML body")

    headings: dict[int, str] = {}
    blocks: list[DocumentBlock] = []
    paragraph_ordinal = 0
    table_ordinal = 0
    for body_block_index, child in enumerate(list(body)):
        if child.tag == f"{W}p":
            style_id, style_name = paragraph_style(child, styles)
            level = heading_level(style_id, style_name)
            text = visible_text(child)
            if level is not None and text.strip():
                headings = {key: value for key, value in headings.items() if key < level}
                headings[level] = text.strip()
            blocks.append(
                ParagraphBlock(
                    body_block_index=body_block_index,
                    paragraph_ordinal=paragraph_ordinal,
                    style_id=style_id,
                    style_name=style_name,
                    heading_path=tuple(headings[key] for key in sorted(headings)),
                    exact_text=text,
                    heading_level=level,
                )
            )
            paragraph_ordinal += 1
        elif child.tag == f"{W}tbl":
            blocks.append(
                TableBlock(
                    body_block_index=body_block_index,
                    table_ordinal=table_ordinal,
                    heading_path=tuple(headings[key] for key in sorted(headings)),
                    rows=table_rows(child),
                )
            )
            table_ordinal += 1
    return blocks


def pointer_for_paragraph(
    block: ParagraphBlock, source_path: str, source_sha256: str
) -> dict[str, Any]:
    return {
        "documentPath": source_path,
        "documentSha256": source_sha256,
        "bodyBlockIndex": block.body_block_index,
        "paragraphOrdinal": block.paragraph_ordinal,
        "styleId": block.style_id,
        "styleName": block.style_name,
        "headingPath": list(block.heading_path),
    }


def pointer_for_table(
    block: TableBlock, source_path: str, source_sha256: str, row_index: int | None = None
) -> dict[str, Any]:
    pointer: dict[str, Any] = {
        "documentPath": source_path,
        "documentSha256": source_sha256,
        "bodyBlockIndex": block.body_block_index,
        "tableOrdinal": block.table_ordinal,
        "headingPath": list(block.heading_path),
    }
    if row_index is not None:
        pointer["rowIndex"] = row_index
    return pointer


def in_phase2_scope(blocks: Iterable[DocumentBlock]) -> list[DocumentBlock]:
    selected: list[DocumentBlock] = []
    started = False
    for block in blocks:
        if isinstance(block, ParagraphBlock):
            if (
                block.heading_level == 1
                and block.exact_text.strip()
                == "14. Phase 2 Authorization, Outcome, and Execution Order"
            ):
                started = True
        if started:
            selected.append(block)
    if not started:
        raise ExtractionError("Phase 2 Section 14 start heading was not found")
    return selected


def extract_area_registry(
    blocks: Iterable[DocumentBlock], source_path: str, source_sha256: str
) -> tuple[list[dict[str, Any]], TableBlock]:
    candidates = [
        block
        for block in blocks
        if isinstance(block, TableBlock)
        and block.heading_path
        and block.heading_path[0] == "Appendix G. Requirement Area Registry Through Phase 2"
    ]
    if len(candidates) != 1:
        raise ExtractionError(
            f"Expected one Appendix G area table, found {len(candidates)}"
        )
    table = candidates[0]
    if not table.rows or tuple(cell.strip() for cell in table.rows[0]) != (
        "Area",
        "Domain",
        "Area",
        "Domain",
    ):
        raise ExtractionError("Appendix G area table header is not canonical")

    areas: list[dict[str, Any]] = []
    for row_index, row in enumerate(table.rows[1:], start=1):
        if len(row) != 4:
            raise ExtractionError(f"Appendix G row {row_index} does not have four cells")
        for offset in (0, 2):
            token = row[offset].strip()
            domain = row[offset + 1].strip()
            if not AREA_TOKEN_RE.fullmatch(token) or not domain:
                raise ExtractionError(
                    f"Appendix G row {row_index} has an invalid area/domain pair"
                )
            areas.append(
                {
                    "token": token,
                    "domain": domain,
                    "sourcePointer": pointer_for_table(
                        table, source_path, source_sha256, row_index
                    ),
                }
            )
    areas.sort(key=lambda item: item["token"])
    return areas, table


def extract_verification_catalog(
    blocks: Iterable[DocumentBlock], source_path: str, source_sha256: str
) -> tuple[list[dict[str, Any]], TableBlock]:
    candidates = [
        block
        for block in blocks
        if isinstance(block, TableBlock)
        and block.heading_path
        and block.heading_path[0] == "Appendix H. Phase 2 Minimum Verification Catalog"
    ]
    if len(candidates) != 1:
        raise ExtractionError(
            f"Expected one Appendix H verification table, found {len(candidates)}"
        )
    table = candidates[0]
    if not table.rows or tuple(cell.strip() for cell in table.rows[0]) != (
        "Verification ID",
        "Minimum proof",
        "Primary requirement areas",
    ):
        raise ExtractionError("Appendix H verification table header is not canonical")

    records: list[dict[str, Any]] = []
    for row_index, row in enumerate(table.rows[1:], start=1):
        if len(row) != 3:
            raise ExtractionError(
                f"Appendix H row {row_index} does not have three cells"
            )
        verification_id = row[0].strip()
        minimum_proof = row[1]
        primary_areas = [part.strip() for part in row[2].split(",") if part.strip()]
        if not VERIFICATION_ID_RE.fullmatch(verification_id):
            raise ExtractionError(
                f"Appendix H row {row_index} has an invalid verification ID"
            )
        records.append(
            {
                "verificationId": verification_id,
                "minimumProof": minimum_proof,
                "primaryRequirementAreas": primary_areas,
                "truthState": "NOT_STARTED",
                "sourcePointer": pointer_for_table(
                    table, source_path, source_sha256, row_index
                ),
                "rowSha256": sha256_bytes("\t".join(row).encode("utf-8")),
            }
        )
    return records, table


def keyword_from_requirement(requirement_body: str) -> str:
    for keyword in (
        "MUST NOT",
        "SHALL NOT",
        "SHOULD NOT",
        "MUST",
        "SHALL",
        "SHOULD",
        "MAY",
    ):
        if requirement_body.startswith(keyword + " ") or requirement_body == keyword:
            return keyword
    return "UNCLASSIFIED"


def normalize_requirement_text(text: str) -> str:
    return " ".join(text.split())


def read_phase1_registry(path: Path) -> tuple[dict[str, dict[str, Any]], str]:
    try:
        raw = path.read_bytes()
        parsed = json.loads(raw.decode("utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ExtractionError(f"Unable to read Phase 1 requirement registry: {exc}") from exc
    records = parsed.get("requirements")
    if not isinstance(records, list):
        raise ExtractionError("Phase 1 requirement registry has no requirements array")
    result: dict[str, dict[str, Any]] = {}
    for record in records:
        if isinstance(record, dict) and isinstance(record.get("id"), str):
            result[record["id"]] = record
    return result, sha256_bytes(raw)


def read_p2_00_evidence(
    path: Path, root: Path, directive_sha256: str
) -> tuple[dict[str, Any] | None, list[dict[str, Any]]]:
    findings: list[dict[str, Any]] = []
    if not path.is_file():
        findings.append(
            {
                "code": "P2_00_EVIDENCE_MISSING",
                "severity": "WARNING",
                "path": project_path(root, path),
            }
        )
        return None, findings
    try:
        raw = path.read_bytes()
        value = json.loads(raw.decode("utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        findings.append(
            {
                "code": "P2_00_EVIDENCE_INVALID",
                "severity": "ERROR",
                "path": project_path(root, path),
                "detail": str(exc),
            }
        )
        return None, findings
    valid = (
        isinstance(value, dict)
        and value.get("gate") == "P2-00"
        and value.get("status") == "PASS"
        and isinstance(value.get("authority"), dict)
        and value["authority"].get("directiveSha256") == directive_sha256
    )
    if not valid:
        findings.append(
            {
                "code": "P2_00_EVIDENCE_CONTRACT_MISMATCH",
                "severity": "ERROR",
                "path": project_path(root, path),
            }
        )
        return None, findings
    return (
        {
            "path": project_path(root, path),
            "sha256": sha256_bytes(raw),
            "gate": "P2-00",
            "recordedStatus": "PASS",
        },
        findings,
    )


def build_outputs(
    *,
    root: Path,
    directive_path: Path,
    phase1_registry_path: Path,
    p2_00_evidence_path: Path,
    expected_directive_sha256: str = EXPECTED_DIRECTIVE_SHA256,
) -> dict[str, dict[str, Any]]:
    root = root.resolve(strict=True)
    directive_path = directive_path.resolve(strict=True)
    observed_directive_sha256 = sha256_file(directive_path)
    if observed_directive_sha256 != expected_directive_sha256.upper():
        raise ExtractionError(
            "Canonical Phase 2 directive SHA-256 mismatch: "
            f"expected {expected_directive_sha256.upper()}, got {observed_directive_sha256}"
        )
    directive_project_path = project_path(root, directive_path)
    blocks = parse_docx(directive_path)
    scope_blocks = in_phase2_scope(blocks)
    phase1_records, phase1_sha256 = read_phase1_registry(phase1_registry_path)
    p2_evidence, p2_evidence_findings = read_p2_00_evidence(
        p2_00_evidence_path, root, observed_directive_sha256
    )

    areas, area_table = extract_area_registry(
        scope_blocks, directive_project_path, observed_directive_sha256
    )
    area_tokens = {item["token"] for item in areas}
    verification_records, verification_table = extract_verification_catalog(
        scope_blocks, directive_project_path, observed_directive_sha256
    )

    definition_occurrences: list[dict[str, Any]] = []
    normative_records: list[dict[str, Any]] = []
    all_scope_text: list[str] = []
    normative_ordinal = 0
    for block in scope_blocks:
        if isinstance(block, ParagraphBlock):
            exact_text = block.exact_text
            all_scope_text.append(exact_text)
            stripped = exact_text.strip()
            if not stripped:
                continue
            match = REQUIREMENT_DEFINITION_RE.fullmatch(stripped)
            if match:
                requirement_id, area, sequence, body = match.groups()
                definition_occurrences.append(
                    {
                        "id": requirement_id,
                        "area": area,
                        "sequence": int(sequence),
                        "exactText": exact_text,
                        "requirementText": body,
                        "normativeKeyword": keyword_from_requirement(body),
                        "sourcePointer": pointer_for_paragraph(
                            block, directive_project_path, observed_directive_sha256
                        ),
                        "textSha256": sha256_bytes(exact_text.encode("utf-8")),
                    }
                )
                continue
            if block.heading_level is not None:
                continue
            normative_ordinal += 1
            if "Code" in block.style_name:
                record_class = "SCHEMA_OR_GRAMMAR"
            elif DEFAULT_RULE_RE.search(stripped):
                record_class = "DEFAULT_OR_NORMATIVE_RULE"
            else:
                record_class = "NORMATIVE_CONTEXT"
            normative_records.append(
                {
                    "recordId": f"P2-NORM-{normative_ordinal:04d}",
                    "recordKind": "PARAGRAPH",
                    "recordClass": record_class,
                    "exactText": exact_text,
                    "textSha256": sha256_bytes(exact_text.encode("utf-8")),
                    "sourcePointer": pointer_for_paragraph(
                        block, directive_project_path, observed_directive_sha256
                    ),
                }
            )
        else:
            table_text = "\n".join("\t".join(row) for row in block.rows)
            all_scope_text.append(table_text)
            normative_ordinal += 1
            normative_records.append(
                {
                    "recordId": f"P2-NORM-{normative_ordinal:04d}",
                    "recordKind": "TABLE",
                    "recordClass": "NORMATIVE_TABLE",
                    "rows": [list(row) for row in block.rows],
                    "exactText": table_text,
                    "textSha256": sha256_bytes(table_text.encode("utf-8")),
                    "sourcePointer": pointer_for_table(
                        block, directive_project_path, observed_directive_sha256
                    ),
                }
            )

    occurrences_by_id: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for record in definition_occurrences:
        occurrences_by_id[record["id"]].append(record)

    requirements: list[dict[str, Any]] = []
    reused_phase1_ids: list[dict[str, Any]] = []
    reused_retired_phase1_ids: list[dict[str, Any]] = []
    for requirement_id in sorted(occurrences_by_id):
        occurrence = occurrences_by_id[requirement_id][0]
        truth_state = "NOT_STARTED"
        evidence: list[dict[str, Any]] = []
        if requirement_id in P2_00_IMPLEMENTED_REQUIREMENTS and p2_evidence is not None:
            truth_state = "IMPLEMENTED_UNVERIFIED"
            evidence = [p2_evidence]
        prior = phase1_records.get(requirement_id)
        prior_summary: dict[str, Any] | None = None
        if prior is not None:
            prior_text = str(prior.get("atomicRequirement", ""))
            lifecycle = prior.get("lifecycle") if isinstance(prior.get("lifecycle"), dict) else {}
            lifecycle_status = lifecycle.get("status")
            same_text = normalize_requirement_text(prior_text) == normalize_requirement_text(
                occurrence["requirementText"]
            )
            prior_summary = {
                "phase1Text": prior_text,
                "phase1TruthState": prior.get("truthState"),
                "phase1LifecycleStatus": lifecycle_status,
                "textDisposition": "EXACT_CARRY_FORWARD" if same_text else "DIFFERENT_TEXT",
            }
            finding = {
                "requirementId": requirement_id,
                "phase1LifecycleStatus": lifecycle_status,
                "phase1Text": prior_text,
                "phase2Text": occurrence["requirementText"],
                "sameNormalizedText": same_text,
                "phase2SourcePointer": occurrence["sourcePointer"],
            }
            reused_phase1_ids.append(finding)
            if lifecycle_status in {"SUPERSEDED", "RETIRED"}:
                reused_retired_phase1_ids.append(finding)

        requirement_record = dict(occurrence)
        requirement_record.update(
            {
                "truthState": truth_state,
                "completionEligible": True,
                "evidence": evidence,
                "priorPhase1Definition": prior_summary,
            }
        )
        requirements.append(requirement_record)

    requirement_ids = {record["id"] for record in requirements}
    all_text = "\n".join(all_scope_text)
    referenced_requirement_ids = set(REQUIREMENT_ID_RE.findall(all_text))
    known_requirement_ids = requirement_ids | set(phase1_records)
    unknown_requirement_references = sorted(
        referenced_requirement_ids - known_requirement_ids
    )

    duplicate_requirement_definitions = [
        {
            "requirementId": requirement_id,
            "occurrenceCount": len(occurrences),
            "sourcePointers": [item["sourcePointer"] for item in occurrences],
            "textSha256Values": [item["textSha256"] for item in occurrences],
        }
        for requirement_id, occurrences in sorted(occurrences_by_id.items())
        if len(occurrences) > 1
    ]
    unknown_area_tokens = sorted(
        {record["area"] for record in requirements} - area_tokens
    )
    unresolved_placeholders: list[dict[str, Any]] = []
    for block in scope_blocks:
        if not isinstance(block, ParagraphBlock):
            continue
        for match in PLACEHOLDER_RE.finditer(block.exact_text):
            unresolved_placeholders.append(
                {
                    "token": match.group(0),
                    "sourcePointer": pointer_for_paragraph(
                        block, directive_project_path, observed_directive_sha256
                    ),
                    "text": block.exact_text,
                }
            )

    verification_ids = [item["verificationId"] for item in verification_records]
    duplicate_verification_records = sorted(
        item for item, count in Counter(verification_ids).items() if count > 1
    )
    unknown_verification_area_tokens = sorted(
        {
            area
            for item in verification_records
            for area in item["primaryRequirementAreas"]
            if area not in area_tokens
        }
    )
    referenced_verification_ids = set(VERIFICATION_ID_RE.findall(all_text))
    catalog_verification_ids = set(verification_ids)
    unknown_verification_references = sorted(
        referenced_verification_ids - catalog_verification_ids
    )

    verification_by_area: dict[str, list[str]] = defaultdict(list)
    for item in verification_records:
        for area in item["primaryRequirementAreas"]:
            verification_by_area[area].append(item["verificationId"])
    for values in verification_by_area.values():
        values.sort()

    mapping_skeleton: list[dict[str, Any]] = []
    requirements_without_candidates: list[str] = []
    for requirement in requirements:
        candidates = verification_by_area.get(requirement["area"], [])
        if not candidates:
            requirements_without_candidates.append(requirement["id"])
        requirement["verificationMappingStatus"] = (
            "AREA_CANDIDATES_UNREVIEWED"
            if candidates
            else "NO_AREA_CANDIDATE_REVIEW_REQUIRED"
        )
        mapping_skeleton.append(
            {
                "requirementId": requirement["id"],
                "requirementTruthState": requirement["truthState"],
                "candidateVerificationIds": list(candidates),
                "mappingStatus": (
                    "AREA_CANDIDATES_UNREVIEWED"
                    if candidates
                    else "NO_AREA_CANDIDATE_REVIEW_REQUIRED"
                ),
                "verified": False,
            }
        )

    requirements_by_area: dict[str, list[str]] = defaultdict(list)
    for requirement in requirements:
        requirements_by_area[requirement["area"]].append(requirement["id"])
    inherited_phase1_by_area: dict[str, list[str]] = defaultdict(list)
    for requirement_id in phase1_records:
        match = REQUIREMENT_ID_RE.fullmatch(requirement_id)
        if match is None or requirement_id in requirement_ids:
            continue
        area = requirement_id.split("-", 2)[1]
        inherited_phase1_by_area[area].append(requirement_id)
    for values in inherited_phase1_by_area.values():
        values.sort()
    for item in verification_records:
        mapped_phase2: set[str] = set()
        mapped_phase1: set[str] = set()
        for area in item["primaryRequirementAreas"]:
            mapped_phase2.update(requirements_by_area.get(area, []))
            mapped_phase1.update(inherited_phase1_by_area.get(area, []))
        item["candidatePhase2RequirementIds"] = sorted(mapped_phase2)
        item["candidateInheritedPhase1RequirementIds"] = sorted(mapped_phase1)
        item["candidateRequirementIds"] = sorted(mapped_phase2 | mapped_phase1)
        item["mappingStatus"] = (
            "AREA_CANDIDATES_UNREVIEWED"
            if mapped_phase2 or mapped_phase1
            else "ORPHAN_REVIEW_REQUIRED"
        )
    orphan_verification_records = sorted(
        item["verificationId"]
        for item in verification_records
        if not item["candidateRequirementIds"]
    )

    hard_finding_count = sum(
        len(items)
        for items in (
            duplicate_requirement_definitions,
            unknown_area_tokens,
            unknown_requirement_references,
            unresolved_placeholders,
            duplicate_verification_records,
            unknown_verification_area_tokens,
            unknown_verification_references,
            orphan_verification_records,
            requirements_without_candidates,
        )
    ) + sum(
        1 for finding in p2_evidence_findings if finding["severity"] == "ERROR"
    )

    generator_path = Path(__file__).resolve(strict=True)
    common_source = {
        "path": directive_project_path,
        "sha256": observed_directive_sha256,
        "bytes": directive_path.stat().st_size,
        "scopeStart": "14. Phase 2 Authorization, Outcome, and Execution Order",
    }
    counts = {
        "requirementDefinitionOccurrences": len(definition_occurrences),
        "uniqueRequirementCount": len(requirements),
        "normativeRecordCount": len(normative_records),
        "normativeTableCount": sum(
            1 for item in normative_records if item["recordKind"] == "TABLE"
        ),
        "areaCount": len(areas),
        "verificationRecordCount": len(verification_records),
        "notStartedRequirementCount": sum(
            1 for item in requirements if item["truthState"] == "NOT_STARTED"
        ),
        "implementedUnverifiedRequirementCount": sum(
            1
            for item in requirements
            if item["truthState"] == "IMPLEMENTED_UNVERIFIED"
        ),
        "verifiedRequirementCount": 0,
    }

    requirements_output = {
        "schemaVersion": 1,
        "registryKind": "PHASE_2_REQUIREMENT_EXTRACTION_BASELINE",
        "generatedBy": project_path(root, generator_path),
        "generatorSha256": sha256_file(generator_path),
        "directive": common_source,
        "phase1Registry": {
            "path": project_path(root, phase1_registry_path),
            "sha256": phase1_sha256,
            "requirementCount": len(phase1_records),
        },
        "truthPolicy": {
            "default": "NOT_STARTED",
            "p2EntryGateException": "IMPLEMENTED_UNVERIFIED",
            "p2EntryGateRequirementIds": sorted(P2_00_IMPLEMENTED_REQUIREMENTS),
            "verifiedIsNeverInferred": True,
        },
        "counts": counts,
        "areas": areas,
        "requirements": requirements,
        "normativeRecords": normative_records,
    }

    verification_output = {
        "schemaVersion": 1,
        "catalogKind": "PHASE_2_APPENDIX_H_MINIMUM_VERIFICATION_CATALOG",
        "generatedBy": project_path(root, generator_path),
        "generatorSha256": sha256_file(generator_path),
        "directive": common_source,
        "catalogSourcePointer": pointer_for_table(
            verification_table, directive_project_path, observed_directive_sha256
        ),
        "mappingPolicy": {
            "status": "SKELETON_ONLY",
            "candidateRule": "Requirement area intersects Appendix H primary areas",
            "candidateDoesNotMeanVerified": True,
            "manualCaseReviewRequired": True,
        },
        "counts": {
            "verificationRecordCount": len(verification_records),
            "mappingSkeletonCount": len(mapping_skeleton),
            "orphanVerificationRecordCount": len(orphan_verification_records),
            "requirementsWithoutCandidateCount": len(requirements_without_candidates),
        },
        "verificationRecords": verification_records,
        "requirementMappingSkeleton": mapping_skeleton,
    }

    audit_output = {
        "schemaVersion": 1,
        "auditKind": "PHASE_2_REQUIREMENT_EXTRACTION_AUDIT",
        "generatedBy": project_path(root, generator_path),
        "generatorSha256": sha256_file(generator_path),
        "directive": common_source,
        "extractionStatus": "PASS" if hard_finding_count == 0 else "REVIEW_REQUIRED",
        "governanceReviewStatus": (
            "REUSED_PHASE1_IDS_DETECTED" if reused_phase1_ids else "NO_REUSED_IDS"
        ),
        "counts": {
            **counts,
            "hardFindingCount": hard_finding_count,
            "duplicateRequirementDefinitionCount": len(
                duplicate_requirement_definitions
            ),
            "unknownAreaTokenCount": len(unknown_area_tokens),
            "unknownRequirementReferenceCount": len(unknown_requirement_references),
            "reusedPhase1IdCount": len(reused_phase1_ids),
            "reusedRetiredPhase1IdCount": len(reused_retired_phase1_ids),
            "unresolvedNormativePlaceholderCount": len(unresolved_placeholders),
            "orphanVerificationRecordCount": len(orphan_verification_records),
        },
        "findings": {
            "duplicateRequirementDefinitions": duplicate_requirement_definitions,
            "unknownAreaTokens": unknown_area_tokens,
            "unknownRequirementReferences": unknown_requirement_references,
            "reusedPhase1Ids": reused_phase1_ids,
            "reusedRetiredPhase1Ids": reused_retired_phase1_ids,
            "unresolvedNormativePlaceholders": unresolved_placeholders,
            "duplicateVerificationRecords": duplicate_verification_records,
            "unknownVerificationAreaTokens": unknown_verification_area_tokens,
            "unknownVerificationReferences": unknown_verification_references,
            "orphanVerificationRecords": orphan_verification_records,
            "requirementsWithoutVerificationCandidates": requirements_without_candidates,
            "p2EntryEvidence": p2_evidence_findings,
        },
        "areaRegistrySourcePointer": pointer_for_table(
            area_table, directive_project_path, observed_directive_sha256
        ),
        "verificationCatalogSourcePointer": pointer_for_table(
            verification_table, directive_project_path, observed_directive_sha256
        ),
        "notes": [
            "Reused Phase 1 IDs are reported without silently renumbering or rewriting either source.",
            "Detected governance findings do not self-promote or verify product requirements.",
            "A passing deterministic extraction check is not a Phase 2 product gate.",
        ],
    }
    return {
        "requirements": requirements_output,
        "verification": verification_output,
        "audit": audit_output,
    }


def write_outputs(root: Path, outputs: dict[str, dict[str, Any]]) -> None:
    for key, relative in OUTPUT_PATHS.items():
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(json_bytes(outputs[key]))
        print(f"WROTE {relative} SHA256={sha256_file(path)}")


def check_outputs(root: Path, outputs: dict[str, dict[str, Any]]) -> bool:
    current = True
    for key, relative in OUTPUT_PATHS.items():
        path = root / relative
        expected = json_bytes(outputs[key])
        if not path.is_file():
            print(f"STALE missing {relative}")
            current = False
            continue
        actual = path.read_bytes()
        if actual != expected:
            print(
                f"STALE {relative} expected={sha256_bytes(expected)} "
                f"actual={sha256_bytes(actual)}"
            )
            current = False
        else:
            print(f"CURRENT {relative} SHA256={sha256_bytes(actual)}")
    return current


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=None)
    parser.add_argument("--source", type=Path, default=Path(DEFAULT_DIRECTIVE_PATH))
    parser.add_argument(
        "--phase1-registry", type=Path, default=Path(DEFAULT_PHASE1_REGISTRY_PATH)
    )
    parser.add_argument(
        "--p2-00-evidence", type=Path, default=Path(DEFAULT_P2_00_EVIDENCE_PATH)
    )
    parser.add_argument("--check", action="store_true")
    return parser.parse_args(argv)


def resolve_under_root(root: Path, value: Path) -> Path:
    return value if value.is_absolute() else root / value


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    root = (
        args.root.resolve(strict=True)
        if args.root is not None
        else Path(__file__).resolve(strict=True).parents[2]
    )
    try:
        outputs = build_outputs(
            root=root,
            directive_path=resolve_under_root(root, args.source),
            phase1_registry_path=resolve_under_root(root, args.phase1_registry),
            p2_00_evidence_path=resolve_under_root(root, args.p2_00_evidence),
        )
    except (ExtractionError, OSError) as exc:
        print(f"ERROR PHASE2-EXTRACTION {exc}", file=sys.stderr)
        return 2

    if args.check:
        if not check_outputs(root, outputs):
            return 1
    else:
        write_outputs(root, outputs)

    audit = outputs["audit"]
    print(
        "PHASE2_EXTRACTION "
        f"requirements={audit['counts']['uniqueRequirementCount']} "
        f"verification={audit['counts']['verificationRecordCount']} "
        f"reusedPhase1Ids={audit['counts']['reusedPhase1IdCount']} "
        f"status={audit['extractionStatus']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
