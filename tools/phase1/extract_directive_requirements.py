#!/usr/bin/env python3
"""Extract the Phase 1 directive into deterministic requirement registers.

This is development-only governance tooling. It reads the supplied DOCX and
writes canonical JSON snapshots; it is never part of the classroom product.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable
from zipfile import ZipFile

DIRECTIVE_NAME = (
    "PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx"
)
RESEARCH_NAME = "Govs PLC project Research Report.md"
DIRECTIVE_SHA256 = "EBF074E2CEAB752F09E6DB63D88E100991729DA13C1EB874290A6B337DA72612"
RESEARCH_SHA256 = "F05C08323B5CC9483BEB1FEB3C7312CCB9A45EBE3B527E6DAE069C181D3FBF55"
REQUIREMENT_PATTERN = re.compile(r"\[(PES-[A-Z]+-\d{4})\]\s*(.*)")
ID_PATTERN = re.compile(r"^PES-[A-Z]+-\d{4}$")
KEYWORD_PATTERN = re.compile(r"^(MUST NOT|SHALL NOT|SHOULD NOT|MUST|SHALL|SHOULD|MAY)\b")
WORD_NAMESPACE = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
W = f"{{{WORD_NAMESPACE}}}"


AREA_COMPONENTS = {
    "ACC": "acceptance and product claims",
    "ARC": "constitutional architecture",
    "CI": "continuous integration and release evidence",
    "CRM": "clean-room, intellectual property, and provenance",
    "DEC": "decision and mandatory-stop governance",
    "DET": "determinism, scheduling, and replay",
    "DEV": "development stack and repository boundaries",
    "DIA": "diagnostics and causal faults",
    "DOC": "governance documentation",
    "EDU": "Engineering Mode, Learning Lens, and Teacher Mode",
    "FID": "fidelity doctrine",
    "GOV": "authority, change, and directive governance",
    "IR": "unified typed PLC intermediate representation",
    "ISO": "VirtualUniverse physical-isolation wall",
    "MSN": "product mission and intended environment",
    "PRJ": "simulator-native project and archive boundary",
    "PROF": "TrainingProfile and version claims",
    "QLT": "quality and anti-placeholder policy",
    "REQ": "requirement and traceability system",
    "SCP": "scope, exclusions, and deferrals",
    "SEC": "security and trust boundaries",
    "TCH": "teacher/student data boundary",
    "TYP": "canonical type system",
    "VOC": "canonical vocabulary",
}


FOUNDATION_VERIFICATION = {
    "PES-CRM-0016": ["VER-CRM-0001", "VER-QLT-0001"],
    "PES-SEC-0017": ["VER-ISO-0001"],
    "PES-DOC-0001": ["VER-DOC-0002"],
    "PES-DOC-0002": ["VER-DOC-0002"],
    "PES-DOC-0003": ["VER-DOC-0001", "VER-QLT-0001"],
    "PES-DOC-0004": ["VER-CRM-0001", "VER-QLT-0001"],
    "PES-REQ-0001": ["VER-REQ-0001"],
    "PES-REQ-0002": ["VER-REQ-0001"],
    "PES-REQ-0003": ["VER-REQ-0001", "VER-CRM-0001", "VER-DEC-0001"],
    "PES-REQ-0004": ["VER-REQ-0001"],
    "PES-REQ-0008": ["VER-REQ-0002"],
    "PES-REQ-0009": ["VER-REQ-0002"],
    "PES-ACC-0006": ["VER-QLT-0001"],
    "PES-ACC-0007": ["VER-QLT-0001"],
    "PES-DEV-0010": ["VER-QLT-0001"],
    "PES-DEV-0012": ["VER-ISO-0001"],
    "PES-ARC-0030": ["VER-QLT-0001"],
    "PES-QLT-0001": ["VER-QLT-0001"],
    "PES-QLT-0004": ["VER-QLT-0001"],
    "PES-QLT-0005": ["VER-QLT-0001", "VER-ISO-0001"],
}


FOUNDATION_COMPONENTS = {
    "PES-CRM-0016": ["CLEAN_ROOM_POLICY.md"],
    "PES-CRM-0017": ["EVIDENCE_REGISTER.json"],
    "PES-CRM-0021": ["ASSET_PROVENANCE.json"],
    "PES-CRM-0022": ["ASSET_PROVENANCE.json", "tools/phase1/verify-phase1.mjs"],
    "PES-CI-0001": [".github/workflows/phase1-governance.yml", "tools/phase1/verify-phase1.mjs"],
    "PES-SEC-0017": ["SECURITY_INVARIANTS.md", "THREAT_MODEL.md"],
    "PES-ARC-0030": ["IMPLEMENTATION_MATRIX.json", "tools/phase1/verify-phase1.mjs"],
    "PES-DOC-0001": ["ADR/0001-no-physical-industrial-communication.md"],
    "PES-DOC-0002": ["ADR/0001-no-physical-industrial-communication.md"],
    "PES-DOC-0003": [
        "ADR/0002-original-project-format.md",
        "ADR/0003-unified-plc-ir.md",
        "ADR/0004-deterministic-virtual-time.md",
    ],
    "PES-DOC-0004": ["EVIDENCE_REGISTER.json", "ASSET_PROVENANCE.json"],
    "PES-DEV-0010": ["pnpm-workspace.yaml", "Cargo.toml", "IMPLEMENTATION_MATRIX.json"],
    "PES-DEV-0006": ["package.json", "pnpm-workspace.yaml", "Cargo.toml", "rust-toolchain.toml"],
    "PES-DEV-0012": ["tools/phase1/verify-phase1.mjs"],
    "PES-REQ-0001": ["REQUIREMENTS.md", "requirements/phase1-requirements.json"],
    "PES-REQ-0002": ["REQUIREMENTS.md", "requirements/phase1-requirements.json"],
    "PES-REQ-0003": ["EVIDENCE_REGISTER.json", "OPEN_DECISIONS.md", "RISK_REGISTER.md"],
    "PES-REQ-0004": ["REQUIREMENTS.md", "tools/phase1/extract_directive_requirements.py"],
    "PES-REQ-0008": ["IMPLEMENTATION_MATRIX.json", "tools/phase1/verify-phase1.mjs"],
    "PES-REQ-0009": ["IMPLEMENTATION_MATRIX.json", "REQUIREMENTS.md"],
    "PES-QLT-0001": ["IMPLEMENTATION_MATRIX.json", "tools/phase1/verify-phase1.mjs"],
    "PES-QLT-0004": ["IMPLEMENTATION_MATRIX.json", "pnpm-workspace.yaml", "Cargo.toml"],
    "PES-QLT-0005": ["tools/phase1/verify-phase1.mjs"],
    "PES-ACC-0006": ["docs/governance/PHASE_1_SCOPE_AUDIT.md"],
    "PES-ACC-0007": ["docs/governance/PHASE_1_SCOPE_AUDIT.md"],
    "PES-GOV-0010": ["OPEN_DECISIONS.md", "EVIDENCE_REGISTER.json"],
    "PES-GOV-0017": ["OPEN_DECISIONS.md"],
    "PES-GOV-0018": ["OPEN_DECISIONS.md"],
    "PES-DEV-0009": ["OPEN_DECISIONS.md"],
    "PES-ACC-0005": ["OPEN_DECISIONS.md"],
}


FOUNDATION_ACCEPTANCE = {
    "PES-CRM-0016": {
        "positive": "CLEAN_ROOM_POLICY.md exists before every product-feature directory and defines permitted, forbidden, quarantine, evidence, and contributor controls.",
        "negative": "The file is missing/empty, permits a forbidden source, or any product-feature directory predates the policy.",
        "dependencies": ["PES-GOV-0001", "PES-CRM-0006", "PES-CRM-0007"],
    },
    "PES-SEC-0017": {
        "positive": "SECURITY_INVARIANTS.md defines trust zones and ownership, THREAT_MODEL.md traces the crossings, and no existing package contradicts either document.",
        "negative": "A trust crossing is undocumented, package ownership conflicts with the boundary, or a production-capable package bypasses the domain controls.",
        "dependencies": ["PES-ARC-0001", "PES-DOC-0001"],
    },
    "PES-ARC-0030": {
        "positive": "Reserved product roots are absent and the repository contains no empty feature UI, no-op product object, transport placeholder, or generic forbidden-capability seam.",
        "negative": "Any reserved feature surface exists only as scaffolding or exposes a placeholder transport/plugin/network capability.",
        "dependencies": ["PES-QLT-0001", "PES-QLT-0005"],
    },
    "PES-DOC-0001": {
        "positive": "ADR-0001 exists with the exact mandated title and exact Project Safety Invariant status.",
        "negative": "The ADR is absent, renamed, or its title/status differs from the mandated text.",
        "dependencies": ["PES-GOV-0009"],
    },
    "PES-DOC-0002": {
        "positive": "ADR-0001 states that amendment cannot add physical capability and requires a separately authorized repository, legal analysis, threat model, and governance.",
        "negative": "ADR-0001 leaves an amendment, adapter, branch, edition, or same-repository path to physical capability.",
        "dependencies": ["PES-DOC-0001"],
    },
    "PES-DOC-0003": {
        "positive": "ADR-0002, ADR-0003, and ADR-0004 substantively document the original project format, unified typed IR/runtime, and deterministic virtual-time boundaries before product code exists.",
        "negative": "Any boundary ADR is absent/empty, claims unrecorded acceptance, or product implementation exists before the relevant decision is documented.",
        "dependencies": ["PES-ARC-0017", "PES-IR-0001", "PES-DET-0001"],
    },
    "PES-DOC-0004": {
        "positive": "Evidence/research files are explicitly classified outside assets/original and every production feature/asset root is absent in the Phase 1 repository.",
        "negative": "Research, evidence, quarantine, citation-cache, manual, or screenshot material appears under a production asset root or packaged allowlist.",
        "dependencies": ["PES-CRM-0017", "PES-CRM-0021"],
    },
    "PES-DEV-0010": {
        "positive": "Workspace manifests reserve responsibility-map patterns while apps, packages, profiles, scenarios, and assets/original remain absent and earn no completion credit.",
        "negative": "An empty product package/directory is created or counted as implementation progress.",
        "dependencies": ["PES-QLT-0001", "PES-QLT-0004"],
    },
    "PES-DEV-0012": {
        "positive": "No top-level or nested product package is named or functions as a network, transport, connector, vendor adapter, protocol, external-HMI, collaboration, or plugin host.",
        "negative": "A forbidden package, alias, generic provider, hidden test seam, or reserved connector boundary exists anywhere in product scope.",
        "dependencies": ["PES-DOC-0001", "PES-QLT-0005"],
    },
    "PES-REQ-0001": {
        "positive": "Exactly 247 unique normative IDs extracted from the hash-bound directive match PES-AREA-NNNN and are represented once in the registry.",
        "negative": "Any normative ID is missing, duplicated, malformed, or uses an unstable/non-domain area.",
        "dependencies": ["PES-REQ-0002"],
    },
    "PES-REQ-0002": {
        "positive": "Every product requirement ID contains only its stable area and non-semantic four-digit number.",
        "negative": "Any requirement ID encodes phase, release, priority, status, or document section.",
        "dependencies": ["PES-REQ-0001"],
    },
    "PES-REQ-0003": {
        "positive": "All supporting records use their prescribed namespaces and every source/evidence record uses one unique SRC-NNNN identifier.",
        "negative": "A supporting record uses an unapproved namespace, collides with another ID, or is represented as a PES requirement.",
        "dependencies": ["PES-REQ-0001"],
    },
    "PES-REQ-0004": {
        "positive": "The registry generation rule rejects duplicate IDs and REQUIREMENTS.md requires any future retired ID to remain as a non-recycled tombstone with disposition.",
        "negative": "A duplicate/recycled ID is accepted or a retired ID can disappear without a supersession/rejection record.",
        "dependencies": ["PES-REQ-0001"],
    },
    "PES-REQ-0008": {
        "positive": "The policy contract and matrix accept the controlled truth-state vocabulary and define only VERIFIED as complete.",
        "negative": "Any other state is treated as complete or a VERIFIED row lacks its executable verification/component mapping.",
        "dependencies": ["PES-REQ-0006", "PES-REQ-0007"],
    },
    "PES-REQ-0009": {
        "positive": "The matrix contains exact state counts and no completion percentage; SCAFFOLDED/PARTIAL/IMPLEMENTED_UNVERIFIED records earn no completion credit.",
        "negative": "File/package/control counts, compilation, or any non-VERIFIED state contributes to a completion percentage or claim.",
        "dependencies": ["PES-REQ-0008"],
    },
    "PES-QLT-0001": {
        "positive": "The matrix gives no implementation credit to absent/reserved UI, type, package, schema, sample, animation, or mocked product paths.",
        "negative": "A placeholder surface or structural artifact is represented as an implemented product feature.",
        "dependencies": ["PES-REQ-0008", "PES-REQ-0009"],
    },
    "PES-QLT-0004": {
        "positive": "The only scaffolded item is the non-user-visible empty workspace foundation; it is labeled SCAFFOLDED, has owner/target metadata, no forbidden capability, and zero completion credit.",
        "negative": "Scaffolding becomes user/release reachable, fails open, lacks ownership/target metadata, or earns completion credit.",
        "dependencies": ["PES-QLT-0001", "PES-REQ-0008"],
    },
    "PES-QLT-0005": {
        "positive": "Repository/package scans find no physical connection abstraction, generic transport, executable plugin host, network-capable HMI provider, or arbitrary scripting engine.",
        "negative": "Any such interface, provider, dependency, test seam, or placeholder exists even if disabled or unused.",
        "dependencies": ["PES-DOC-0001", "PES-DEV-0012"],
    },
    "PES-ACC-0006": {
        "positive": "The scope audit, README, matrix, and changelog explicitly state that Phase 1 work does not complete the four-phase master directive.",
        "negative": "Any repository claim equates Phase 1 authoring/foundation work with completion of the master directive or product.",
        "dependencies": ["PES-GOV-0017", "PES-GOV-0018"],
    },
    "PES-ACC-0007": {
        "positive": "No product feature root/code exists; only the explicitly authorized Phase 1 governance foundation was created.",
        "negative": "Any Phase 2-4 product implementation, user-visible placeholder, or release artifact is created under Phase 1 authorization.",
        "dependencies": ["PES-ACC-0006", "PES-GOV-0018"],
    },
}


BLOCKED_REQUIREMENTS = {
    "PES-GOV-0010": "DEC-0001: research hash matches, but the controlled filename differs.",
    "PES-GOV-0017": "DEC-0001: living-directive filename and phase-document model require approval.",
    "PES-GOV-0018": "DEC-0001: separate phase prompts may conflict with the one-document rule.",
    "PES-DEV-0009": "OQ-0001: initial OS and packaging model are intentionally undecided.",
    "PES-ACC-0005": "DEC-0001: supplied status says 'Phase 1 authored; Phases 2-4 reserved' instead of the mandated exact wording.",
}


PHASE_1_FOUNDATION_IDS = set(FOUNDATION_VERIFICATION) | {
    "PES-CRM-0017",
    "PES-CRM-0021",
    "PES-CRM-0022",
    "PES-DEV-0006",
    "PES-CI-0001",
    *BLOCKED_REQUIREMENTS.keys(),
}


CURATED_CLASS_9_IDS = {
    "PES-DOC-0001",
    "PES-DOC-0002",
    "PES-DEV-0012",
    "PES-QLT-0005",
}


LATER_RELEASE_VERIFICATION_IDS = {
    *(f"PES-ISO-{number:04d}" for number in range(11, 23)),
    *(f"PES-SEC-{number:04d}" for number in range(9, 12)),
    *(f"PES-SEC-{number:04d}" for number in range(21, 25)),
    "PES-CI-0002",
    "PES-CI-0003",
}


@dataclass
class ExtractedRequirement:
    requirement_id: str
    heading_path: list[str]
    body_block: int
    parts: list[str] = field(default_factory=list)
    continuation_blocks: int = 0
    table_rows: int = 0

    @property
    def text(self) -> str:
        return "\n".join(part for part in self.parts if part).strip()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest().upper()


def paragraph_text(element: ET.Element) -> str:
    parts: list[str] = []
    for node in element.iter():
        if node.tag == f"{W}t":
            parts.append(node.text or "")
        elif node.tag == f"{W}tab":
            parts.append("\t")
        elif node.tag in {f"{W}br", f"{W}cr"}:
            parts.append("\n")
    return "".join(parts)


def load_docx_parts(document_path: Path) -> tuple[ET.Element, dict[str, str]]:
    with ZipFile(document_path) as archive:
        document = ET.fromstring(archive.read("word/document.xml"))
        styles_root = ET.fromstring(archive.read("word/styles.xml"))
    style_names: dict[str, str] = {}
    for style in styles_root.findall(f".//{W}style"):
        style_id = style.get(f"{W}styleId")
        name_element = style.find(f"{W}name")
        if style_id and name_element is not None:
            style_names[style_id] = name_element.get(f"{W}val", style_id)
    body = document.find(f".//{W}body")
    if body is None:
        raise SystemExit("DOCX has no word/document.xml body")
    return body, style_names


def paragraph_style(element: ET.Element, style_names: dict[str, str]) -> str:
    style_element = element.find(f"./{W}pPr/{W}pStyle")
    if style_element is None:
        return "Normal"
    style_id = style_element.get(f"{W}val", "Normal")
    return style_names.get(style_id, style_id)


def heading_level(style: str) -> int | None:
    """Return a Word heading level without assuming style-name capitalization."""

    match = re.fullmatch(r"heading\s*([1-9][0-9]*)", style.strip(), re.IGNORECASE)
    return int(match.group(1)) if match else None


def table_rows(element: ET.Element) -> Iterable[list[str]]:
    for row in element.findall(f"./{W}tr"):
        values: list[str] = []
        for cell in row.findall(f"./{W}tc"):
            paragraphs = [paragraph_text(item).strip() for item in cell.findall(f".//{W}p")]
            values.append(" / ".join(item for item in paragraphs if item))
        yield values


def extract(document_path: Path) -> list[ExtractedRequirement]:
    body, style_names = load_docx_parts(document_path)
    requirements: list[ExtractedRequirement] = []
    current: ExtractedRequirement | None = None
    headings: dict[int, str] = {}

    def flush() -> None:
        nonlocal current
        if current is not None:
            requirements.append(current)
            current = None

    for block_number, item in enumerate(list(body), start=1):
        tag = item.tag.rsplit("}", 1)[-1]
        if tag == "p":
            text = paragraph_text(item).strip()
            style = paragraph_style(item, style_names)
            level = heading_level(style)
            if level is not None:
                flush()
                headings[level] = text
                for deeper in tuple(key for key in headings if key > level):
                    del headings[deeper]
                continue

            match = REQUIREMENT_PATTERN.match(text)
            if match:
                flush()
                current = ExtractedRequirement(
                    requirement_id=match.group(1),
                    heading_path=[headings[key] for key in sorted(headings)],
                    body_block=block_number,
                    parts=[match.group(2).strip()],
                )
                continue

            if current is not None and text:
                current.parts.append(text)
                current.continuation_blocks += 1
        elif tag == "tbl" and current is not None:
            for values in table_rows(item):
                current.parts.append("Table row: " + " | ".join(values))
                current.table_rows += 1

    flush()
    return requirements


def normative_keyword(text: str) -> str:
    match = KEYWORD_PATTERN.match(text)
    return match.group(1) if match else "MUST"


def short_title(req: ExtractedRequirement) -> str:
    statement = re.sub(
        r"^(MUST NOT|SHALL NOT|SHOULD NOT|MUST|SHALL|SHOULD|MAY)\s+",
        "",
        req.text.splitlines()[0],
    ).strip()
    statement = re.split(r"[.;:]", statement, maxsplit=1)[0].strip()
    if len(statement) > 88:
        statement = statement[:85].rstrip() + "..."
    return statement or req.requirement_id


def candidate_ip_flags(text: str) -> dict[str, object]:
    value = text.lower()
    classes: set[int] = set()
    dispositions: list[str] = []
    bases: list[str] = []
    physical_terms = (
        "physical industrial",
        "physical plc",
        "physical hmi",
        "industrial protocol",
        "s7comm",
        "profinet",
        "profibus",
        "ethernet/ip",
        "modbus",
        "external opc",
        "host nic",
        "raw socket",
        "device discovery",
    )
    if any(term in value for term in physical_terms):
        classes.add(9)
        dispositions.append("EXCLUDE physical capability; implement only negative isolation controls.")
        bases.append("Physical industrial communication category.")
    if any(term in value for term in ("trademark", "branding", "siemens", "simatic", "wincc", "plcsim", "model number")):
        classes.add(5)
        dispositions.append("REPLACE or EXCLUDE vendor identity; public comparative language remains BLOCKED.")
        bases.append("Branding, mark, or vendor identity concern.")
    if any(term in value for term in ("screenshot", "icon", "artwork", "trade dress", "diagnostic prose", "screen composition")):
        classes.add(4)
        dispositions.append("REDESIGN with original expression; vendor expression is excluded.")
        bases.append("Vendor-specific expression concern.")
    if any(term in value for term in ("patent", "class 7", "freedom-to-operate")):
        classes.add(7)
        dispositions.append("BLOCKED pending focused professional review.")
        bases.append("Patent or licensing concern.")
    if any(term in value for term in ("class 8", "professional legal review", "uncertain or high-risk")):
        classes.add(8)
        dispositions.append("BLOCKED pending professional legal review.")
        bases.append("Uncertain or high-risk item.")
    if any(term in value for term in ("firmware", "proprietary", ".apxx", ".zapxx", "vendor project")):
        classes.add(6)
        dispositions.append("Use an original simulated equivalent or EXCLUDE; never reproduce proprietary technology.")
        bases.append("Proprietary technology or format concern.")
    if any(term in value for term in ("workflow", "v21-era", "tia-oriented", "online/offline")):
        classes.add(3)
        dispositions.append("Preserve useful functional workflow while independently redesigning expression.")
        bases.append("Workflow behavior.")
    if any(term in value for term in ("iec 61131", "industry convention", "structured text")):
        classes.add(2)
        dispositions.append("Implement independently from lawfully usable standards or public functional behavior.")
        bases.append("Industry or IEC convention.")
    return {
        "classes": sorted(classes),
        "candidateDispositions": list(dict.fromkeys(dispositions)),
        "matchedConcerns": list(dict.fromkeys(bases)),
        "purpose": "NON_NORMATIVE_TRIAGE_ONLY; keyword matches must never be copied into the reviewed IP classification",
    }


def reviewed_ip_classification(requirement_id: str) -> dict[str, object]:
    if requirement_id in CURATED_CLASS_9_IDS:
        return {
            "classes": [9],
            "disposition": "EXCLUDE physical capability; the Phase 1 work is a negative governance or isolation control only.",
            "basis": "Curated requirement-ID review: this record prohibits a PhysicalUniverse capability rather than implementing one.",
            "classificationMethod": "CURATED_PHASE_1_REQUIREMENT_ID_REVIEW",
            "reviewStatus": "REVIEWED_FOR_PHASE_1_GOVERNANCE_SCOPE; not legal advice or product implementation approval",
        }
    if requirement_id in PHASE_1_FOUNDATION_IDS:
        return {
            "classes": [1],
            "disposition": "Implement only the original project-governance or repository-foundation control described by this requirement.",
            "basis": "Curated requirement-ID review limited to the current Phase 1 governance artifact, not any later product behavior.",
            "classificationMethod": "CURATED_PHASE_1_REQUIREMENT_ID_REVIEW",
            "reviewStatus": "REVIEWED_FOR_PHASE_1_GOVERNANCE_SCOPE; not legal advice or product implementation approval",
        }
    return {
        "classes": [8],
        "disposition": "BLOCKED_PENDING_SUBJECT_AWARE_PROFESSIONAL_REVIEW",
        "basis": "No curated subject-aware IP classification has been completed for later product implementation.",
        "classificationMethod": "UNRESOLVED_DEFAULT_CLASS_8",
        "reviewStatus": "BLOCKED_PENDING_REQUIRED_REVIEW",
    }


def target_for(requirement_id: str, area: str) -> tuple[str, str]:
    if requirement_id in PHASE_1_FOUNDATION_IDS:
        return "Phase 1 governance foundation", "FOUNDATION_WORK_ONLY"
    if requirement_id in LATER_RELEASE_VERIFICATION_IDS:
        return (
            "Later release-verification milestone after authorized product implementation",
            "RESERVED_LATER_PHASE_NO_PRODUCT_AUTHORIZATION",
        )
    if area in {"EDU", "TCH"}:
        return (
            "Reserved for a later authored education/teacher phase",
            "RESERVED_LATER_PHASE_NO_PRODUCT_AUTHORIZATION",
        )
    if area in {"ARC", "DET", "DEV", "DIA", "IR", "ISO", "PRJ", "PROF", "SEC", "TYP"}:
        return (
            "Reserved for a later authored product specification/implementation phase",
            "RESERVED_LATER_PHASE_NO_PRODUCT_AUTHORIZATION",
        )
    if area in {"FID", "MSN", "SCP", "VOC"}:
        return (
            "Phase 1 constitution only; observable product implementation requires a later authored phase",
            "POLICY_ONLY_NO_PRODUCT_AUTHORIZATION",
        )
    return (
        "Continuing governance; any product behavior requires a later authored phase",
        "POLICY_ONLY_NO_PRODUCT_AUTHORIZATION",
    )


def state_for(requirement_id: str) -> tuple[str, str, list[str]]:
    if requirement_id in BLOCKED_REQUIREMENTS:
        return "BLOCKED", BLOCKED_REQUIREMENTS[requirement_id], []
    if requirement_id == "PES-DEV-0006":
        return (
            "SCAFFOLDED",
            "Root pnpm/Cargo workspace manifests exist, but product packages are intentionally absent.",
            ["VER-CI-0001"],
        )
    if requirement_id == "PES-CI-0001":
        return (
            "PARTIAL",
            "Local Phase 1 governance checks and a disabled workflow proposal exist; DEC-0002 blocks remote CI, while product/WASM/packaged-artifact gates require later artifacts.",
            ["VER-CI-0001"],
        )
    if requirement_id == "PES-CRM-0017":
        return (
            "IMPLEMENTED_UNVERIFIED",
            "The normalized evidence-register structure exists, but its records remain unreviewed and lack verification mappings.",
            ["VER-CRM-0001"],
        )
    if requirement_id == "PES-CRM-0021":
        return (
            "IMPLEMENTED_UNVERIFIED",
            "The asset-provenance schema exists and truthfully records zero assets, but no reviewed production-asset record exists yet.",
            ["VER-CRM-0001"],
        )
    if requirement_id == "PES-CRM-0022":
        return (
            "PARTIAL",
            "The Phase 1 verifier checks the registry; production asset-pipeline enforcement awaits product assets.",
            ["VER-CRM-0001"],
        )
    if requirement_id in FOUNDATION_VERIFICATION:
        return (
            "IMPLEMENTED_UNVERIFIED",
            "A current-scope automated check is defined, but the generated registry does not self-certify verification or reviewer acceptance.",
            FOUNDATION_VERIFICATION[requirement_id],
        )
    return (
        "NOT_STARTED",
        "No product implementation is authorized by this Phase 1 foundation.",
        [],
    )


def acceptance_for(
    requirement_id: str, keyword: str
) -> tuple[str, str, list[str], str]:
    curated = FOUNDATION_ACCEPTANCE.get(requirement_id)
    if curated is not None:
        return (
            curated["positive"],
            curated["negative"],
            curated["dependencies"],
            "CURATED_PHASE_1_CURRENT_SCOPE",
        )
    if "NOT" in keyword:
        return (
            "Verification demonstrates the prohibited capability or state is absent across the supported scope.",
            "Any reachable counterexample, dependency, artifact, state, or behavior prohibited by the requirement fails acceptance.",
            [],
            "UNRESOLVED_BASELINE_REQUIRES_LATER_REQUIREMENT_REVIEW",
        )
    return (
        "Verification demonstrates the required statement for every supported condition in its declared scope.",
        "A supported condition that violates the required statement, or missing objective evidence, fails acceptance.",
        [],
        "UNRESOLVED_BASELINE_REQUIRES_LATER_REQUIREMENT_REVIEW",
    )


def build_records(requirements: list[ExtractedRequirement]) -> tuple[list[dict], list[dict]]:
    records: list[dict] = []
    matrix: list[dict] = []
    for req in requirements:
        area = req.requirement_id.split("-")[1]
        keyword = normative_keyword(req.text)
        positive, negative, related_requirements, acceptance_maturity = acceptance_for(
            req.requirement_id, keyword
        )
        state, status_note, verification_ids = state_for(req.requirement_id)
        atomicity = (
            "COMPOUND_SOURCE_REQUIRES_REVIEW"
            if req.continuation_blocks > 1 or req.table_rows > 0
            else "BASELINE_ATOMIC"
        )
        decisions = []
        if "DEC-0001" in status_note:
            decisions.append("DEC-0001")
        if "DEC-0002" in status_note:
            decisions.append("DEC-0002")
        if "OQ-0001" in status_note:
            decisions.append("OQ-0001")
        ip_classification = reviewed_ip_classification(req.requirement_id)
        candidate_flags = candidate_ip_flags(req.text)
        has_defined_automated_check = bool(verification_ids)
        implementation_components = FOUNDATION_COMPONENTS.get(req.requirement_id, [])
        target_milestone, phase1_disposition = target_for(req.requirement_id, area)
        record = {
            "id": req.requirement_id,
            "title": short_title(req),
            "normativeKeyword": keyword,
            "atomicRequirement": req.text,
            "atomicity": atomicity,
            "rationale": "Controlled by the directive section identified in sourcePointer; a separate rationale is unresolved where the directive does not state one.",
            "scopeComponent": AREA_COMPONENTS.get(area, f"directive area {area}"),
            "sourcePointer": {
                "sourceId": "SRC-0002",
                "file": DIRECTIVE_NAME,
                "sha256": DIRECTIVE_SHA256,
                "headingPath": req.heading_path,
                "bodyBlock": req.body_block,
                "researchClassification": "DIRECTIVE_ADOPTED_NORMATIVE; underlying research label requires per-source evidence resolution",
            },
            "ipClassification": ip_classification,
            "candidateIpFlags": candidate_flags,
            # Empty means no blocking prerequisite was established by the
            # current review. It does not mean the requirement has no related
            # obligations; those are recorded separately below.
            "dependencies": [],
            "relatedRequirements": related_requirements,
            "dependencyMaturity": (
                "CURATED_PHASE_1_RELATIONSHIPS; no blocking prerequisite asserted"
                if req.requirement_id in FOUNDATION_ACCEPTANCE
                else "UNRESOLVED_BASELINE; empty dependencies are not a no-dependency assertion"
            ),
            "targetMilestone": target_milestone,
            "phase1Disposition": phase1_disposition,
            "truthState": state,
            "statusNote": status_note,
            "positiveAcceptance": positive,
            "negativeAcceptance": negative,
            "acceptanceMaturity": acceptance_maturity,
            "verificationIds": verification_ids,
            "adrDecisionChangeLinks": decisions,
            "implementationComponents": implementation_components,
            "owner": "Scott",
            "reviewer": "UNASSIGNED",
            "reviewStatus": (
                "AUTOMATED_CHECK_DEFINED; execution evidence is external to this snapshot and reviewer acceptance is not recorded"
                if has_defined_automated_check
                else "UNREVIEWED"
            ),
            "reviewDate": None,
        }
        records.append(record)
        matrix.append(
            {
                "requirementId": req.requirement_id,
                "component": record["scopeComponent"],
                "targetMilestone": record["targetMilestone"],
                "phase1Disposition": phase1_disposition,
                "truthState": state,
                "verificationIds": verification_ids,
                "implementationComponents": implementation_components,
                "decisionLinks": decisions,
                "notes": status_note,
            }
        )
    return records, matrix


def validate(requirements: list[ExtractedRequirement]) -> None:
    identifiers = [item.requirement_id for item in requirements]
    duplicates = sorted({item for item in identifiers if identifiers.count(item) > 1})
    invalid = sorted(item for item in identifiers if not ID_PATTERN.match(item))
    if duplicates:
        raise SystemExit(f"Duplicate requirement IDs: {duplicates}")
    if invalid:
        raise SystemExit(f"Invalid requirement IDs: {invalid}")
    if len(requirements) != 247:
        raise SystemExit(f"Expected 247 requirements, found {len(requirements)}")
    missing_heading_paths = [
        item.requirement_id for item in requirements if not item.heading_path
    ]
    if missing_heading_paths:
        raise SystemExit(
            "Requirements missing source heading paths: "
            + ", ".join(missing_heading_paths)
        )
    embedded_requirement_ids = [
        item.requirement_id
        for item in requirements
        if re.search(r"\[PES-[A-Z]+-\d{4}\]", item.text)
    ]
    if embedded_requirement_ids:
        raise SystemExit(
            "Requirements contain a later requirement marker: "
            + ", ".join(embedded_requirement_ids)
        )
    known_ids = set(identifiers)
    if set(FOUNDATION_VERIFICATION) != set(FOUNDATION_ACCEPTANCE):
        raise SystemExit(
            "Every Phase 1 automated-check mapping must have curated acceptance criteria"
        )
    if not set(FOUNDATION_VERIFICATION).issubset(FOUNDATION_COMPONENTS):
        raise SystemExit(
            "Every Phase 1 automated-check mapping must have implementation components"
        )
    unknown_related = sorted(
        related_id
        for acceptance in FOUNDATION_ACCEPTANCE.values()
        for related_id in acceptance["dependencies"]
        if related_id not in known_ids
    )
    if unknown_related:
        raise SystemExit(
            "Curated acceptance criteria reference unknown requirement IDs: "
            + ", ".join(unknown_related)
        )


def json_text(value: object) -> str:
    return json.dumps(value, indent=2, ensure_ascii=True) + "\n"


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(json_text(value).encode("utf-8"))


def main() -> None:
    expected_python = (3, 13, 12)
    if sys.version_info[:3] != expected_python:
        actual = ".".join(str(part) for part in sys.version_info[:3])
        expected = ".".join(str(part) for part in expected_python)
        raise SystemExit(
            f"Python runtime mismatch: expected {expected}, got {actual}"
        )
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument(
        "--check",
        action="store_true",
        help="Fail if committed JSON snapshots differ from deterministic output.",
    )
    args = parser.parse_args()
    root = args.root.resolve()
    directive = root / DIRECTIVE_NAME
    research = root / RESEARCH_NAME
    generator_hash = sha256(Path(__file__).resolve())

    actual_directive_hash = sha256(directive)
    actual_research_hash = sha256(research)
    if actual_directive_hash != DIRECTIVE_SHA256:
        raise SystemExit(
            f"Directive hash mismatch: expected {DIRECTIVE_SHA256}, got {actual_directive_hash}"
        )
    if actual_research_hash != RESEARCH_SHA256:
        raise SystemExit(
            f"Research hash mismatch: expected {RESEARCH_SHA256}, got {actual_research_hash}"
        )

    requirements = extract(directive)
    validate(requirements)
    records, matrix_entries = build_records(requirements)
    state_counts: dict[str, int] = {}
    for entry in matrix_entries:
        state_counts[entry["truthState"]] = state_counts.get(entry["truthState"], 0) + 1

    registry = {
        "schemaVersion": 2,
        "snapshotDate": "2026-08-27",
        "generatedBy": "tools/phase1/extract_directive_requirements.py",
        "generatorSha256": generator_hash,
        "directive": {
            "path": DIRECTIVE_NAME,
            "sha256": DIRECTIVE_SHA256,
            "status": "Phase 1 supplied; living-document filename/status reconciliation is BLOCKED by DEC-0001",
        },
        "researchBaseline": {"path": RESEARCH_NAME, "sha256": RESEARCH_SHA256},
        "requirementCount": len(records),
        "requirements": records,
    }
    matrix = {
        "schemaVersion": 2,
        "snapshotDate": "2026-08-27",
        "generatedBy": "tools/phase1/extract_directive_requirements.py",
        "generatorSha256": generator_hash,
        "completionRule": "Only VERIFIED means complete. No completion percentage is calculated.",
        "scope": "Phase 1 repository foundation; product implementation is not started.",
        "requirementCount": len(matrix_entries),
        "stateCounts": dict(sorted(state_counts.items())),
        "entries": matrix_entries,
    }
    outputs = {
        root / "requirements" / "phase1-requirements.json": registry,
        root / "IMPLEMENTATION_MATRIX.json": matrix,
    }
    if args.check:
        stale = [
            str(path.relative_to(root))
            for path, value in outputs.items()
            if not path.exists() or path.read_bytes() != json_text(value).encode("utf-8")
        ]
        if stale:
            raise SystemExit(
                "Generated Phase 1 snapshots are stale: " + ", ".join(stale)
            )
        print(
            f"Verified {len(records)} requirement records and implementation-matrix entries are current."
        )
        return

    for path, value in outputs.items():
        write_json(path, value)
    print(f"Wrote {len(records)} requirement records and implementation-matrix entries.")


if __name__ == "__main__":
    main()
