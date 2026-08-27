from __future__ import annotations

from collections import Counter
import hashlib
import re
from pathlib import Path
from xml.etree import ElementTree as ET
from zipfile import ZipFile

import pdfplumber


REPO = Path(r"C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC")
DOCX = REPO / "PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx"
PDF = REPO / ".phase1-verification/docx-visual/phase1-directive-render.pdf"
W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
ID_RE = re.compile(r"\[(PES-[A-Z]+-\d{4})\]")
MODAL_RE = re.compile(
    r"(?i)(\bshall\b|\bmust\b|\bnever\b|"
    r"\bis\s+(?:prohibited|forbidden|required)\b|"
    r"\bmay\b(?:\s+\S+){0,8}\s+only\b)"
)


def paragraph_text(paragraph: ET.Element) -> str:
    parts: list[str] = []
    for element in paragraph.iter():
        if element.tag == W + "t":
            parts.append(element.text or "")
        elif element.tag == W + "tab":
            parts.append("\t")
        elif element.tag in (W + "br", W + "cr"):
            parts.append("\n")
    return "".join(parts).strip()


def is_numbered(paragraph: ET.Element) -> bool:
    properties = paragraph.find(W + "pPr")
    return bool(properties is not None and properties.find(W + "numPr") is not None)


def load_blocks() -> list[dict[str, object]]:
    with ZipFile(DOCX) as archive:
        root = ET.fromstring(archive.read("word/document.xml"))
    body = root.find(f".//{W}body")
    assert body is not None
    blocks: list[dict[str, object]] = []
    for child in list(body):
        if child.tag == W + "p":
            text = paragraph_text(child)
            if text:
                blocks.append({"kind": "p", "text": text, "numbered": is_numbered(child)})
        elif child.tag == W + "tbl":
            rows: list[list[str]] = []
            for table_row in child.findall(f"./{W}tr"):
                cells: list[str] = []
                for table_cell in table_row.findall(f"./{W}tc"):
                    texts = [paragraph_text(p) for p in table_cell.findall(f".//{W}p")]
                    cells.append(" ".join(text for text in texts if text))
                if any(cells):
                    rows.append(cells)
            blocks.append(
                {
                    "kind": "table",
                    "rows": rows,
                    "text": " | ".join(" | ".join(row) for row in rows),
                    "numbered": False,
                }
            )
    return blocks


def normalize(text: str) -> str:
    text = text.replace("\u00ad", "").replace("\n", " ")
    text = re.sub(r"([A-Za-z])-\s+([A-Za-z])", r"\1\2", text)
    return re.sub(r"[^a-z0-9]+", " ", text.lower()).strip()


def load_pages() -> tuple[list[str], list[str]]:
    with pdfplumber.open(PDF) as pdf:
        texts = [page.extract_text(x_tolerance=2, y_tolerance=2) or "" for page in pdf.pages]
    assert len(texts) == 40
    return texts, [normalize(text) for text in texts]


def locate_page(text: str, pages: list[str], normalized_pages: list[str], fallback: int) -> int:
    requirement = re.search(r"PES-[A-Z]+-\d{4}", text)
    if requirement:
        for number, page in enumerate(pages, 1):
            if requirement.group(0) in page:
                return number
    words = normalize(text).split()
    probes: list[str]
    if len(words) >= 5:
        probes = [
            " ".join(words[: min(12, len(words))]),
            " ".join(words[-min(10, len(words)) :]),
        ]
    else:
        probes = [" ".join(words)]
    for probe in probes:
        hits = [
            page_number
            for page_number, page in enumerate(normalized_pages, 1)
            if probe and probe in page
        ]
        if hits:
            return min(hits, key=lambda page: (page < fallback, abs(page - fallback)))
    return fallback


def split_sentences(text: str) -> list[str]:
    flattened = text.replace("\n", " ")
    return [
        sentence.strip()
        for sentence in re.split(
            r"(?<=[.!?])\s+(?=(?:\[PES-|[A-Z\"“]))", flattened
        )
        if sentence.strip()
    ]


def nearest_section(blocks: list[dict[str, object]]) -> dict[int, str]:
    section = "Front matter"
    result: dict[int, str] = {}
    for index, block in enumerate(blocks):
        if block["kind"] == "p":
            text = str(block["text"]).replace("\n", " ")
            if not bool(block["numbered"]) and (
                re.match(r"^\d+\.\d+\s", text)
                or re.match(r"^\d+\.\s", text)
                or re.match(r"^Appendix [A-F]\.", text)
                or text in {"Normative Keywords", "Document Control", "How to Use This Directive"}
            ):
                section = text
        result[index] = section
    return result


def inherited_children(blocks: list[dict[str, object]]) -> dict[tuple[str, int, int], tuple[int, str | None]]:
    inherited: dict[tuple[str, int, int], tuple[int, str | None]] = {}
    for index, block in enumerate(blocks):
        if (
            block["kind"] != "p"
            or not str(block["text"]).rstrip().endswith(":")
            or not MODAL_RE.search(str(block["text"]))
        ):
            continue
        match = ID_RE.search(str(block["text"]))
        parent_id = match.group(1) if match else None
        child_index = index + 1
        if child_index >= len(blocks):
            continue
        child = blocks[child_index]
        if child["kind"] == "table":
            for row_index in range(1, len(child["rows"])):
                inherited[("row", child_index, row_index)] = (index, parent_id)
        elif child["kind"] == "p" and "\n" in str(child["text"]):
            for line_index, line in enumerate(str(child["text"]).splitlines()):
                if line.strip():
                    inherited[("line", child_index, line_index)] = (index, parent_id)
        else:
            while (
                child_index < len(blocks)
                and blocks[child_index]["kind"] == "p"
                and bool(blocks[child_index]["numbered"])
            ):
                inherited[("block", child_index, 0)] = (index, parent_id)
                child_index += 1
    return inherited


def collect_records() -> list[dict[str, object]]:
    blocks = load_blocks()
    pages, normalized_pages = load_pages()
    sections = nearest_section(blocks)
    inherited = inherited_children(blocks)
    records: list[dict[str, object]] = []
    last_page = 1
    for index, block in enumerate(blocks):
        section = sections[index]
        if block["kind"] == "p":
            text = str(block["text"])
            direct_match = ID_RE.search(text)
            direct_id = direct_match.group(1) if direct_match else None
            inherited_block = inherited.get(("block", index, 0))
            if inherited_block:
                lead_in = str(blocks[inherited_block[0]]["text"]).replace("\n", " ")
                page = locate_page(text, pages, normalized_pages, last_page)
                last_page = max(last_page, page)
                records.append(
                    {
                        "page": page,
                        "section": section,
                        "text": text.replace("\n", " "),
                        "maps": [inherited_block[1]] if inherited_block[1] else [],
                        "kind": "inherited bullet",
                        "block": index,
                        "lead_in": lead_in,
                    }
                )
                continue
            line_keys = [key for key in inherited if key[0] == "line" and key[1] == index]
            if line_keys:
                parent_index, parent_id = inherited[line_keys[0]]
                lead_in = str(blocks[parent_index]["text"]).replace("\n", " ")
                for line in text.splitlines():
                    if not line.strip():
                        continue
                    page = locate_page(line, pages, normalized_pages, last_page)
                    last_page = max(last_page, page)
                    records.append(
                        {
                            "page": page,
                            "section": section,
                            "text": line.strip(),
                            "maps": [parent_id] if parent_id else [],
                            "kind": "inherited line",
                            "block": index,
                            "lead_in": lead_in,
                        }
                    )
                continue
            for sentence in split_sentences(text):
                if MODAL_RE.search(sentence):
                    page = locate_page(sentence, pages, normalized_pages, last_page)
                    last_page = max(last_page, page)
                    records.append(
                        {
                            "page": page,
                            "section": section,
                            "text": sentence,
                            "maps": [direct_id] if direct_id else [],
                        "kind": "explicit",
                        "block": index,
                        "lead_in": None,
                        }
                    )
        else:
            rows = block["rows"]
            for row_index, row in enumerate(rows):
                text = " | ".join(row)
                inherited_row = inherited.get(("row", index, row_index))
                if inherited_row:
                    lead_in = str(blocks[inherited_row[0]]["text"]).replace("\n", " ")
                    page = locate_page(" ".join(row), pages, normalized_pages, last_page)
                    last_page = max(last_page, page)
                    records.append(
                        {
                            "page": page,
                            "section": section,
                            "text": text,
                            "maps": [inherited_row[1]] if inherited_row[1] else [],
                            "kind": "inherited table row",
                            "block": index,
                            "lead_in": lead_in,
                        }
                    )
                elif MODAL_RE.search(text):
                    page = locate_page(" ".join(row), pages, normalized_pages, last_page)
                    last_page = max(last_page, page)
                    records.append(
                        {
                            "page": page,
                            "section": section,
                            "text": text,
                            "maps": [],
                            "kind": "explicit table row",
                            "block": index,
                            "lead_in": None,
                        }
                    )
    for record in records:
        if record["text"] == "author;" and record["maps"] == ["PES-CRM-0017"]:
            record["page"] = 18
    return records


ARTIFACTS = {
    "DomainResult {",
    "}",
    "Binding MUST/MUST NOT rules | Final Codex marching orders",
}


SOURCE_CROSS_MAPS: dict[str, list[str]] = {
    "The product shall simulate engineering decisions and consequences with high training-transfer fidelity while remaining permanently incapable of communicating with or operating physical industrial equipment.": ["PES-MSN-0003", "PES-SCP-0002"],
    "It shall provide high causal, behavioral, workflow, and training-transfer fidelity inside a wholly fictional VirtualUniverse.": ["PES-MSN-0003", "PES-FID-0002"],
    "It shall never communicate with, discover, configure, commission, download to, or operate physical industrial equipment.": ["PES-SCP-0002", "PES-ISO-0001", "PES-ISO-0002"],
    "Unless Scott separately orders otherwise, Codex shall not begin product implementation from this incomplete directive.": ["PES-ACC-0007"],
    "Reserved headings are not implementation requirements and shall not be inferred.": ["PES-GOV-0019", "PES-ACC-0007"],
    "Never renumber or reuse one.": ["PES-REQ-0004"],
    "Never trade away the safety wall, clean-room rules, or causal-fidelity doctrine for speed, convenience, visual similarity, or a demo.": ["PES-ISO-0001", "PES-CRM-0001", "PES-FID-0002"],
    "Every externally inspired requirement shall be classified before implementation:": ["PES-CRM-0007"],
    "1 | Functional behavior | Independently implement": ["PES-CRM-0001"],
    "2 | Industry or IEC convention | Implement from lawfully licensed standards or public behavior": ["PES-CRM-0008"],
    "3 | Workflow behavior | Preserve useful workflow logic; redesign visuals and expression": ["PES-CRM-0003", "PES-CRM-0004", "PES-CRM-0005"],
    "4 | Vendor-specific expression | Redesign": ["PES-CRM-0004", "PES-CRM-0005"],
    "5 | Branding or trademark | Replace or exclude": ["PES-CRM-0012", "PES-CRM-0013"],
    "6 | Proprietary technology | Create an original simulated equivalent": ["PES-CRM-0001", "PES-CRM-0004", "PES-CRM-0005"],
    "7 | Patent or licensing concern | BLOCKED pending focused review": ["PES-SCP-0010"],
    "8 | Uncertain or high-risk | BLOCKED pending professional legal review": ["PES-CRM-0006", "PES-SCP-0010"],
    "9 | Physical industrial communication | Permanently EXCLUDED": ["PES-SCP-0002", "PES-ISO-0001", "PES-ISO-0002"],
    "Untrusted content | imported projects, archives, CSV/JSON, images, future libraries/scenarios/scripts | Validate, limit, never execute": ["PES-SEC-0012", "PES-SEC-0013", "PES-SEC-0014"],
    "Development environment | package managers, compilers, test servers, CI tools | May use development capabilities but shall not enter production": ["PES-SEC-0004", "PES-SEC-0005"],
    "Every meaningful mutation shall be a domain command.": ["PES-ARC-0012"],
    "CLEAN_ROOM_POLICY.md": ["PES-CRM-0016"],
    "SECURITY_INVARIANTS.md": ["PES-SEC-0017"],
    "CONTRIBUTOR_CLEAN_ROOM_ATTESTATION.md": ["PES-CRM-0020"],
    "THREAT_MODEL.md": ["PES-SEC-0025"],
    "EVIDENCE_REGISTER.*": ["PES-CRM-0017"],
    "ASSET_PROVENANCE.*": ["PES-CRM-0021"],
    "CHANGELOG_DIRECTIVE.md": ["PES-GOV-0014", "PES-GOV-0015", "PES-GOV-0016"],
    "ADR/": ["PES-DOC-0001", "PES-DOC-0003"],
    "0001-no-physical-industrial-communication.md": ["PES-DOC-0001"],
    "0002-original-project-format.md": ["PES-DOC-0003"],
    "0003-unified-plc-ir.md": ["PES-DOC-0003"],
    "0004-deterministic-virtual-time.md": ["PES-DOC-0003"],
    "Meaningful autonomous decisions shall still be recorded in an ADR or implementation note.": ["PES-DEC-0001"],
    "Engineering timestamp | Human-facing wall-clock metadata; never authoritative simulation time": ["PES-DET-0002", "PES-DET-0005"],
    "The product may become broad, realistic, polished, and deeply functional only inside these boundaries.": ["PES-SCP-0001", "PES-ISO-0001", "PES-CRM-0001", "PES-DET-0001", "PES-FID-0002"],
}


DEFECT_BUCKETS: list[tuple[str, set[str]]] = [
    (
        "Unnumbered normative-keyword semantics",
        {
            "MUST / SHALL | Required. Violation blocks merge, release, or acceptance.",
            "MUST NOT / SHALL NOT | Prohibited. Presence blocks merge, release, or acceptance.",
            "MAY | Optional and permitted only inside the approved scope.",
        },
    ),
    (
        "Unnumbered DomainResult minimum-field schema",
        {"success", "value?", "events[]", "diagnostics[]", "affectedObjectIds[]", "undoToken?", "beforeHash", "afterHash"},
    ),
    (
        "Top-level governance-file obligations lacking a materially equivalent numbered owner",
        {
            "Before feature implementation, the repository shall contain:",
            "LEGAL_REVIEW_CHECKLIST.md",
            "REQUIREMENTS.md",
            "IMPLEMENTATION_MATRIX.*",
            "DEPENDENCY_POLICY.*",
            "OPEN_DECISIONS.md",
            "RISK_REGISTER.md",
        },
    ),
    (
        "Unnumbered atomic requirement-record schema",
        {
            "Every requirement record shall contain:", "stable ID;", "short title;", "normative keyword;",
            "one atomic, testable statement;", "rationale;", "scope/component;",
            "source pointer and research classification;", "IP class and disposition;", "dependencies;",
            "target software release or milestone;", "current truth state;", "positive acceptance condition;",
            "negative acceptance condition;", "verification IDs;", "ADR/decision/change links;", "owner and reviewer.",
        },
    ),
    (
        "Unnumbered BLOCKED-decision record schema",
        {
            "Every blocked decision request shall contain:", "Decision ID:", "Affected requirement IDs:",
            "Known facts:", "Unknown or conflicting point:", "Why Codex cannot decide safely:",
            "Option A and impact:", "Option B and impact:", "Option C and impact, if useful:",
            "Recommended option:", "Exact approval or evidence needed:", "Work that can continue:",
        },
    ),
    (
        "Open-question acceptance-budget obligation without a PES requirement owner",
        {"OQ-0008 | Accessibility conformance target and performance/capacity budgets | Must be objective before experience acceptance | Phase 3"},
    ),
]


def source_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest().upper()


def markdown_code(text: str) -> str:
    if "``" not in text:
        return f"``{text}``"
    return "<code>" + text.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;") + "</code>"


def adjudicate() -> list[dict[str, object]]:
    records = [record for record in collect_records() if record["text"] not in ARTIFACTS]
    for sequence, record in enumerate(records, 1):
        record["audit_id"] = f"T2-{sequence:04d}"
        if record["maps"]:
            record["mapping_basis"] = "owning source requirement ID"
        elif record["text"] in SOURCE_CROSS_MAPS:
            record["maps"] = SOURCE_CROSS_MAPS[record["text"]]
            record["mapping_basis"] = "materially equivalent numbered statement elsewhere in the source"
        else:
            record["mapping_basis"] = "UNMAPPED"
    assert len(records) == 546, len(records)
    assert sum(bool(record["maps"]) for record in records) == 498
    assert sum(not record["maps"] for record in records) == 48
    assert all(records[index]["page"] <= records[index + 1]["page"] for index in range(len(records) - 1))
    defect_texts = set().union(*(texts for _, texts in DEFECT_BUCKETS))
    unmapped_texts = {str(record["text"]) for record in records if not record["maps"]}
    assert defect_texts == unmapped_texts, (defect_texts - unmapped_texts, unmapped_texts - defect_texts)
    return records


def render_entry(record: dict[str, object], *, include_basis: bool = True) -> list[str]:
    maps = record["maps"]
    mapping = ", ".join(markdown_code(str(item)) for item in maps) if maps else "**UNMAPPED**"
    header = (
        f"- **{record['audit_id']}** — p. {record['page']}; § {record['section']}; "
        f"{record['kind']}; mapping: {mapping}"
    )
    lines = [header, f"  - Verbatim: {markdown_code(str(record['text']))}"]
    if record.get("lead_in"):
        lines.append(f"  - Modal lead-in: {markdown_code(str(record['lead_in']))}")
    if include_basis:
        lines.append(f"  - Basis: {record['mapping_basis']}.")
    return lines


def render_report() -> str:
    records = adjudicate()
    mapped = [record for record in records if record["maps"]]
    unmapped = [record for record in records if not record["maps"]]
    page_counts = Counter(int(record["page"]) for record in records)
    kind_counts = Counter(str(record["kind"]) for record in records)
    lines: list[str] = [
        "# Phase 1 Directive — Adversarial Normative-Statement Recall Audit",
        "",
        "## Audit result",
        "",
        f"- In-scope normative statements: **{len(records)}**",
        f"- Mapped to one or more Phase 1 requirement IDs: **{len(mapped)}**",
        f"- UNMAPPED: **{len(unmapped)}**",
        "- Source-page coverage: **40/40 pages walked**; pages 37 and 39 contain no in-scope statement.",
        "",
        "The 48 unmapped statements are substantive recall defects: they are normative source obligations but have no owning or materially equivalent numbered Phase 1 requirement statement. The complete 546-statement ledger follows below; repository registries, matrices, reports, and verifier output were not used as source truth.",
        "",
        "## Source evidence and method",
        "",
        f"- Source DOCX: {markdown_code(str(DOCX))}",
        f"- DOCX SHA-256: {markdown_code(source_sha256(DOCX))}",
        f"- Full-page rendered PDF used only to anchor source page numbers: {markdown_code(str(PDF))}",
        f"- Rendered PDF SHA-256: {markdown_code(source_sha256(PDF))}",
        "- Extraction: the DOCX body was walked in document order directly from `word/document.xml`; every one of the 40 rendered pages was independently extracted with `pdfplumber` to assign page anchors.",
        "- Trigger scope: case-insensitive `shall`, `must` (including `must not`), `never`, `is prohibited`, `is forbidden`, `is required`, and limited-permission `may … only` forms. A modal list/table/line lead-in propagates to its individual children. The DomainResult minimum-field list was conservatively included because its two-sentence introductory paragraph contains `shall` and ends with the field-list lead-in.",
        "- Unit of count: a modal lead-in is one statement; each separately testable child bullet, table row, or schema line governed by that lead-in is another statement. Separate modal sentences in one paragraph are separate statements.",
        "- Exclusions: structural braces (`DomainResult {` and `}`) and Appendix E's descriptive crosswalk row `Binding MUST/MUST NOT rules | Final Codex marching orders` are not normative statements.",
        "- Mapping rule: an embedded/owning `PES-*` ID controls its sentence and modal children. An unnumbered statement is cross-mapped only where a numbered statement elsewhere in the same source materially restates the obligation. Topic similarity, a repository implementation, or a verifier assertion is insufficient.",
        f"- Statement forms counted: {', '.join(f'{name}={count}' for name, count in sorted(kind_counts.items()))}.",
        "",
        "## Major defects",
        "",
    ]
    for title, texts in DEFECT_BUCKETS:
        matched = [record for record in unmapped if record["text"] in texts]
        pages = sorted({int(record["page"]) for record in matched})
        lines.append(f"- **{title}: {len(matched)} statements** (p. {', '.join(map(str, pages))}).")
    lines.extend([
        "",
        "## Every unmapped source statement",
        "",
    ])
    for record in unmapped:
        lines.extend(render_entry(record))
    lines.extend([
        "",
        "## Complete source recall ledger",
        "",
        "<!-- LEDGER_START -->",
    ])
    by_page: dict[int, list[dict[str, object]]] = {page: [] for page in range(1, 41)}
    for record in records:
        by_page[int(record["page"])].append(record)
    for page in range(1, 41):
        lines.extend(["", f"### Page {page} — {page_counts.get(page, 0)} statement(s)", ""])
        if not by_page[page]:
            lines.append("No in-scope modal statement on this page.")
        else:
            for record in by_page[page]:
                lines.extend(render_entry(record))
    lines.extend([
        "",
        "<!-- LEDGER_END -->",
        "",
        "## Coverage conclusion",
        "",
        "The ledger accounts for all 40 pages and all 546 in-scope source statements under the stated trigger and inheritance rules. Of those, 498 have a direct, inherited, or materially equivalent source requirement-ID mapping; 48 remain UNMAPPED. No repository file was treated as authoritative evidence for either recall or mapping.",
        "",
    ])
    return "\n".join(lines)


if __name__ == "__main__":
    print(render_report())
