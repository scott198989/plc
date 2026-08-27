#!/usr/bin/env python3
"""Extract the Phase 1 directive into deterministic requirement registers.

This is development-only governance tooling. It reads the supplied DOCX and
writes canonical JSON snapshots; it is never part of the classroom product.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
import sys
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable
from zipfile import ZipFile

REFERENCE_DIRECTORY = "References for Codex from Scott"
DIRECTIVE_NAME = (
    f"{REFERENCE_DIRECTORY}/"
    "PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx"
)
RESEARCH_NAME = f"{REFERENCE_DIRECTORY}/Govs PLC project Research Report.md"
DIRECTIVE_SHA256 = "EBF074E2CEAB752F09E6DB63D88E100991729DA13C1EB874290A6B337DA72612"
RESEARCH_SHA256 = "F05C08323B5CC9483BEB1FEB3C7312CCB9A45EBE3B527E6DAE069C181D3FBF55"
CORRECTIVE_ADDENDUM_NAME = (
    f"{REFERENCE_DIRECTORY}/"
    "PLC Engineering Simulator - Phase 1 Corrective Addendum - Closure and Trusted Baseline.docx"
)
CORRECTIVE_ADDENDUM_SHA256 = (
    "950C5112C34D0218FD1E59CF6C051ACCD01AB92674CD70C96C08A5F1DA2E5A1C"
)
REQUIREMENT_PATTERN = re.compile(r"\[(PES-[A-Z]+-\d{4})\]\s*(.*)")
ID_PATTERN = re.compile(r"^PES-[A-Z]+-\d{4}$")
KEYWORD_PATTERN = re.compile(r"^(MUST NOT|SHALL NOT|SHOULD NOT|MUST|SHALL|SHOULD|MAY)\b")
WORD_NAMESPACE = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
W = f"{{{WORD_NAMESPACE}}}"


# This allocation is the approved WP-0C corrective ledger.  It is deliberately
# explicit: once issued, a child ID is permanent even if a later change revises
# its wording or disposition.  Children are ordered by source body order and
# clause order and use the next previously-unused number in the parent's area.
COMPOUND_CHILD_STARTS = {
    "PES-GOV-0001": 21,
    "PES-GOV-0003": 27,
    "PES-MSN-0002": 10,
    "PES-FID-0001": 9,
    "PES-ISO-0008": 23,
    "PES-ISO-0009": 34,
    "PES-SEC-0001": 26,
    "PES-CRM-0008": 26,
    "PES-CRM-0009": 31,
    "PES-CRM-0017": 38,
    "PES-CRM-0021": 50,
    "PES-ARC-0022": 39,
    "PES-CI-0001": 4,
    "PES-REQ-0003": 13,
    "PES-DEC-0001": 7,
    "PES-DEC-0002": 15,
    "PES-QLT-0002": 9,
    "PES-QLT-0004": 23,
    "PES-QLT-0006": 29,
    "PES-GOV-0017": 32,
}

EXPECTED_COMPOUND_CHILD_COUNTS = {
    "PES-GOV-0001": 6,
    "PES-GOV-0003": 5,
    "PES-MSN-0002": 15,
    "PES-FID-0001": 6,
    "PES-ISO-0008": 11,
    "PES-ISO-0009": 5,
    "PES-SEC-0001": 7,
    "PES-CRM-0008": 5,
    "PES-CRM-0009": 7,
    "PES-CRM-0017": 12,
    "PES-CRM-0021": 8,
    "PES-ARC-0022": 18,
    "PES-CI-0001": 14,
    "PES-REQ-0003": 7,
    "PES-DEC-0001": 8,
    "PES-DEC-0002": 12,
    "PES-QLT-0002": 14,
    "PES-QLT-0004": 6,
    "PES-QLT-0006": 16,
    "PES-GOV-0017": 8,
}

PAGE_STATEMENT_COUNTS = [
    1, 4, 2, 17, 13, 21, 8, 12, 17, 8,
    12, 21, 17, 14, 12, 18, 20, 22, 11, 20,
    8, 22, 13, 22, 11, 10, 27, 6, 26, 23,
    15, 27, 24, 23, 13, 4, 0, 1, 0, 1,
]

MODAL_RE = re.compile(
    r"(?i)(\bshall\b|\bmust\b|\bnever\b|"
    r"\bis\s+(?:prohibited|forbidden|required)\b|"
    r"\bmay\b(?:\s+\S+){0,8}\s+only\b)"
)


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


DOMAIN_RESULT_FIELD_BY_ID = {
    "PES-ARC-0031": "success",
    "PES-ARC-0032": "value?",
    "PES-ARC-0033": "events[]",
    "PES-ARC-0034": "diagnostics[]",
    "PES-ARC-0035": "affectedObjectIds[]",
    "PES-ARC-0036": "undoToken?",
    "PES-ARC-0037": "beforeHash",
    "PES-ARC-0038": "afterHash",
}

DOMAIN_RESULT_COMPONENTS = [
    "packages/foundation-contract/src/index.ts",
    "apps/foundation-shell/src/foundation.worker.ts",
    "apps/foundation-shell/src/worker-handler.ts",
    "apps/foundation-shell/src/foundation-client.ts",
    "apps/foundation-shell/src/App.tsx",
    "crates/foundation-wasm/src/lib.rs",
    "packages/foundation-contract/test/contract.test.ts",
    "apps/foundation-shell/test/ui-model.test.ts",
    "tests/foundation/foundation-browser.e2e.mjs",
]

FOUNDATION_WORKSPACE_COMPONENTS = [
    "package.json",
    "pnpm-workspace.yaml",
    "pnpm-lock.yaml",
    "apps/foundation-shell/package.json",
    "packages/foundation-contract/package.json",
    "Cargo.toml",
    "Cargo.lock",
    "crates/foundation-wasm/Cargo.toml",
    "rust-toolchain.toml",
]

CI_COMMON_COMPONENTS = [
    ".github/workflows/phase1-governance.yml",
    "package.json",
]

# The Phase 1 gate is executable, but most source obligations describe later
# PLC-product surfaces that do not exist yet.  Each atomic child therefore
# records the exact current-scope controls and the remaining non-vacuous proof;
# none receives completion credit merely because the future surface is absent.
CI_CHILD_CONTROLS = {
    "PES-CI-0004": {
        "verificationIds": ["VER-CI-0001", "VER-DEP-0001", "VER-ISO-0001"],
        "components": [
            *CI_COMMON_COMPONENTS,
            "DEPENDENCY_POLICY.md",
            "tests/phase1/policy-contract.json",
            "tools/phase1/verify-phase1.mjs",
            "tools/foundation/lint-source-policy.mjs",
            "tools/foundation/verify-isolation.mjs",
        ],
        "evidence": "exact dependency admission plus source and bundle capability scans",
        "remaining": "the full future production dependency and capability surface does not yet exist",
    },
    "PES-CI-0005": {
        "verificationIds": ["VER-CI-0001", "VER-ISO-0001"],
        "components": [
            *CI_COMMON_COMPONENTS,
            "tests/phase1/policy-contract.json",
            "tools/phase1/verify-phase1.mjs",
            "tools/foundation/lint-source-policy.mjs",
            "tools/foundation/verify-isolation.mjs",
            "apps/foundation-shell/src/worker-handler.ts",
        ],
        "evidence": "prohibited source-API scans and semantic zero-import inspection of the embedded WASM module",
        "remaining": "later product modules and packaged artifacts are not present to exercise the complete obligation",
    },
    "PES-CI-0006": {
        "verificationIds": ["VER-CI-0001", "VER-OFF-0001", "VER-DEP-0001", "VER-ISO-0001"],
        "components": [
            *CI_COMMON_COMPONENTS,
            "tests/phase1/policy-contract.json",
            "tools/phase1/verify-phase1.mjs",
            "tools/foundation/lint-source-policy.mjs",
            "tools/foundation/verify-isolation.mjs",
            "tests/foundation/foundation-browser.e2e.mjs",
        ],
        "evidence": "restricted URL and dependency scans, bundle isolation checks, and an offline browser run with zero remote requests",
        "remaining": "later course, runtime, and packaged-product surfaces do not yet exist",
    },
    "PES-CI-0007": {
        "verificationIds": ["VER-CI-0001", "VER-CRM-0001"],
        "components": [
            *CI_COMMON_COMPONENTS,
            "ASSET_PROVENANCE.json",
            "tests/phase1/policy-contract.json",
            "tools/phase1/verify-phase1.mjs",
        ],
        "evidence": "asset-registry schema, provenance, and exact zero-shipped-asset checks",
        "remaining": "no later production asset corpus exists to exercise the complete approval workflow",
    },
    "PES-CI-0008": {
        "verificationIds": ["VER-CI-0001", "VER-CRM-0001", "VER-BRN-0001"],
        "components": [
            *CI_COMMON_COMPONENTS,
            "ASSET_PROVENANCE.json",
            "CLEAN_ROOM_POLICY.md",
            "tests/phase1/policy-contract.json",
            "tools/phase1/verify-phase1.mjs",
            "apps/foundation-shell/src/App.tsx",
        ],
        "evidence": "clean-room, provenance, and user-facing vendor-mark scans over the admitted foundation",
        "remaining": "the later production illustration and prose corpus does not yet exist",
    },
    "PES-CI-0009": {
        "verificationIds": ["VER-CI-0001", "VER-REQ-0002", "VER-CRM-0001"],
        "components": [
            *CI_COMMON_COMPONENTS,
            "requirements/phase1-requirements.json",
            "EVIDENCE_REGISTER.json",
            "CLEAN_ROOM_POLICY.md",
            "tools/phase1/verify-phase1.mjs",
        ],
        "evidence": "requirement IP-disposition fields, evidence classification, and matrix truth-state controls",
        "remaining": "subject-aware review of later research-derived product requirements remains blocked",
    },
    "PES-CI-0010": {
        "verificationIds": ["VER-CI-0001"],
        "components": [
            *CI_COMMON_COMPONENTS,
            "packages/foundation-contract/test/contract.test.ts",
            "apps/foundation-shell/test/ui-model.test.ts",
            "tests/foundation/foundation-browser.e2e.mjs",
            "crates/foundation-wasm/src/lib.rs",
            "tools/phase1/verify-phase1.mjs",
            "tools/phase1/run_phase1_mutations.mjs",
        ],
        "evidence": "required lint, type, unit, build, isolation, browser, governance, and mutation checks",
        "remaining": "cross-run hosted repetition has not established a general flakiness history",
    },
    "PES-CI-0011": {
        "verificationIds": ["VER-CI-0001"],
        "components": [
            *CI_COMMON_COMPONENTS,
            "packages/foundation-contract/test/contract.test.ts",
            "apps/foundation-shell/test/ui-model.test.ts",
            "tests/foundation/foundation-browser.e2e.mjs",
            "apps/foundation-shell/src/worker-handler.ts",
            "crates/foundation-wasm/src/lib.rs",
        ],
        "evidence": "deterministic contract/unit checks and repeated offline browser health results for the current foundation",
        "remaining": "PLC scan, replay, and product-runtime determinism are later features and have no current proof corpus",
    },
    "PES-CI-0012": {
        "verificationIds": ["VER-CI-0001", "VER-REQ-0002"],
        "components": [
            *CI_COMMON_COMPONENTS,
            "requirements/phase1-requirements.json",
            "IMPLEMENTATION_MATRIX.json",
            "tools/phase1/verify-phase1.mjs",
        ],
        "evidence": "an active required-test entrypoint and an explicit NOT_STARTED migration truth state",
        "remaining": "no migration implementation or identity-and-data preservation corpus exists",
    },
    "PES-CI-0013": {
        "verificationIds": ["VER-CI-0001", "VER-SCP-0001", "VER-QLT-0001"],
        "components": [
            *CI_COMMON_COMPONENTS,
            "packages/foundation-contract/src/index.ts",
            "apps/foundation-shell/src/worker-handler.ts",
            "apps/foundation-shell/src/App.tsx",
            "tests/foundation/foundation-browser.e2e.mjs",
            "tools/phase1/verify-phase1.mjs",
        ],
        "evidence": "the foundation UI's typed command path through the isolated worker and WASM health boundary",
        "remaining": "lessons and ordinary PLC-domain diagnostic behavior are not implemented",
    },
    "PES-CI-0014": {
        "verificationIds": ["VER-CI-0001", "VER-OFF-0002", "VER-ISO-0001"],
        "components": [
            *CI_COMMON_COMPONENTS,
            "tests/phase1/policy-contract.json",
            "tools/phase1/verify-phase1.mjs",
            "tools/foundation/lint-source-policy.mjs",
            "tools/foundation/verify-isolation.mjs",
        ],
        "evidence": "loopback, endpoint-string, network-API, and browser-device-API rejection scans",
        "remaining": "Virtual Download does not exist, so its behavioral value-rejection proof is not implemented",
    },
    "PES-CI-0015": {
        "verificationIds": ["VER-CI-0001", "VER-ISO-0001", "VER-SCP-0001"],
        "components": [
            *CI_COMMON_COMPONENTS,
            "tests/phase1/policy-contract.json",
            "tools/phase1/verify-phase1.mjs",
            "tools/foundation/lint-source-policy.mjs",
            "tools/foundation/verify-isolation.mjs",
        ],
        "evidence": "forbidden transport, connector, network, and package-boundary scans",
        "remaining": "HMI and InternalTagBus are not implemented, so the product transport invariant has no behavioral proof",
    },
    "PES-CI-0016": {
        "verificationIds": ["VER-CI-0001", "VER-QLT-0001", "VER-BRN-0001", "VER-ISO-0001"],
        "components": [
            *CI_COMMON_COMPONENTS,
            "apps/foundation-shell/src/App.tsx",
            "docs/governance/PHASE_1_SCOPE_AUDIT.md",
            "tests/phase1/policy-contract.json",
            "tools/phase1/verify-phase1.mjs",
            "tools/foundation/verify-isolation.mjs",
        ],
        "evidence": "a non-PLC foundation identity plus bundle scans that reject deployment, network, and device interfaces",
        "remaining": "no simulator export pipeline or industrial-deployment negative corpus exists",
    },
    "PES-CI-0017": {
        "verificationIds": ["VER-CI-0001", "VER-REQ-0002"],
        "components": [
            *CI_COMMON_COMPONENTS,
            "requirements/phase1-requirements.json",
            "requirements/phase1-reconciliation.json",
            "IMPLEMENTATION_MATRIX.json",
            "tools/phase1/verify-phase1.mjs",
        ],
        "evidence": "registry/matrix reciprocity, exact check mappings, and the rule that only VERIFIED may count as complete",
        "remaining": "the baseline intentionally has zero VERIFIED requirements, so a non-vacuous missing-test-link case remains unexercised",
    },
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
    "PES-ACC-0007": ["VER-QLT-0001", "VER-SCP-0001", "VER-ISO-0001"],
    "PES-DEV-0006": ["VER-CI-0001", "VER-DEP-0001", "VER-SCP-0001"],
    "PES-DEV-0010": ["VER-QLT-0001", "VER-SCP-0001"],
    "PES-DEV-0012": ["VER-ISO-0001"],
    "PES-ARC-0030": ["VER-QLT-0001", "VER-SCP-0001"],
    "PES-QLT-0001": ["VER-QLT-0001"],
    "PES-QLT-0004": ["VER-QLT-0001", "VER-SCP-0001"],
    "PES-QLT-0005": ["VER-QLT-0001", "VER-ISO-0001"],
}


FOUNDATION_COMPONENTS = {
    "PES-CRM-0016": ["CLEAN_ROOM_POLICY.md"],
    "PES-CRM-0017": ["EVIDENCE_REGISTER.json"],
    "PES-CRM-0021": ["ASSET_PROVENANCE.json"],
    "PES-CRM-0022": ["ASSET_PROVENANCE.json", "tools/phase1/verify-phase1.mjs"],
    "PES-CI-0001": [".github/workflows/phase1-governance.yml", "tools/phase1/verify-phase1.mjs"],
    "PES-SEC-0017": ["SECURITY_INVARIANTS.md", "THREAT_MODEL.md"],
    "PES-ARC-0030": [
        "IMPLEMENTATION_MATRIX.json",
        "tests/phase1/policy-contract.json",
        "tools/phase1/verify-phase1.mjs",
        "apps/foundation-shell/src/App.tsx",
        "tests/foundation/foundation-browser.e2e.mjs",
    ],
    "PES-DOC-0001": ["ADR/0001-no-physical-industrial-communication.md"],
    "PES-DOC-0002": ["ADR/0001-no-physical-industrial-communication.md"],
    "PES-DOC-0003": [
        "ADR/0002-original-project-format.md",
        "ADR/0003-unified-plc-ir.md",
        "ADR/0004-deterministic-virtual-time.md",
    ],
    "PES-DOC-0004": ["EVIDENCE_REGISTER.json", "ASSET_PROVENANCE.json"],
    "PES-DEV-0010": [
        "pnpm-workspace.yaml",
        "Cargo.toml",
        "apps/foundation-shell/package.json",
        "packages/foundation-contract/package.json",
        "crates/foundation-wasm/Cargo.toml",
        "IMPLEMENTATION_MATRIX.json",
    ],
    "PES-DEV-0006": FOUNDATION_WORKSPACE_COMPONENTS,
    "PES-DEV-0012": ["tools/phase1/verify-phase1.mjs"],
    "PES-REQ-0001": ["REQUIREMENTS.md", "requirements/phase1-requirements.json"],
    "PES-REQ-0002": ["REQUIREMENTS.md", "requirements/phase1-requirements.json"],
    "PES-REQ-0003": ["EVIDENCE_REGISTER.json", "OPEN_DECISIONS.md", "RISK_REGISTER.md"],
    "PES-REQ-0004": ["REQUIREMENTS.md", "tools/phase1/extract_directive_requirements.py"],
    "PES-REQ-0008": ["IMPLEMENTATION_MATRIX.json", "tools/phase1/verify-phase1.mjs"],
    "PES-REQ-0009": ["IMPLEMENTATION_MATRIX.json", "REQUIREMENTS.md"],
    "PES-QLT-0001": ["IMPLEMENTATION_MATRIX.json", "tools/phase1/verify-phase1.mjs"],
    "PES-QLT-0004": [
        "IMPLEMENTATION_MATRIX.json",
        "tests/phase1/policy-contract.json",
        "tools/phase1/verify-phase1.mjs",
        "apps/foundation-shell/package.json",
        "packages/foundation-contract/package.json",
        "crates/foundation-wasm/Cargo.toml",
    ],
    "PES-QLT-0005": ["tools/phase1/verify-phase1.mjs"],
    "PES-ACC-0006": ["docs/governance/PHASE_1_SCOPE_AUDIT.md"],
    "PES-ACC-0007": [
        "docs/governance/PHASE_1_SCOPE_AUDIT.md",
        "apps/foundation-shell/src/App.tsx",
        "apps/foundation-shell/src/worker-handler.ts",
        "packages/foundation-contract/src/index.ts",
        "crates/foundation-wasm/src/lib.rs",
        "tests/foundation/foundation-browser.e2e.mjs",
    ],
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
        "positive": "The admitted user-visible Phase 1 foundation action performs a real typed worker/WASM health round trip, while reserved PLC-product surfaces remain absent and receive no implementation credit.",
        "negative": "Any no-op foundation control, empty later-feature UI, coming-soon product panel, placeholder transport, or generic forbidden-capability seam fails acceptance.",
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
        "positive": "The admitted app, TypeScript contract package, and Rust crate implement the bounded non-PLC health path; each has a concrete responsibility and none is counted as later PLC-product completion.",
        "negative": "An empty package, reserved later-feature directory, responsibility-free crate, or structural-only artifact is created or counted as product progress.",
        "dependencies": ["PES-QLT-0001", "PES-QLT-0004"],
    },
    "PES-DEV-0006": {
        "positive": "The exact pnpm and Cargo workspaces contain the admitted foundation app, contract package, and Rust/WASM crate; package, dependency, lock, and Rust toolchain records are pinned and checked by the active foundation gate.",
        "negative": "A workspace member, direct declaration, lockfile, package-manager version, Rust version, component, or WASM target is absent, mutable, unadmitted, or outside the active gate.",
        "dependencies": ["PES-DEV-0010", "PES-CI-0001"],
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
        "positive": "The admitted app, package, and crate contain tested non-PLC foundation behavior and are not labeled SCAFFOLDED; any future SCAFFOLDED record must remain non-reachable, fail closed, carry owner/target metadata, and earn zero completion credit.",
        "negative": "A structural-only artifact is treated as implemented, or any scaffolding becomes user/release reachable, fails open, lacks ownership/target metadata, contains forbidden capability, or earns completion credit.",
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
        "positive": "Only the explicitly authorized minimal non-PLC technical foundation exists: a local health UI, typed command/result contract, isolated worker, and deterministic zero-import Rust/WASM health path; no PLC-domain feature was begun.",
        "negative": "Any PLC compiler, typed PLC IR, controller runtime, scan cycle, engineering editor, HMI, process physics, lesson, scenario, assessment, packaging, physical communication, or Phase 2-4 placeholder is added under Phase 1 authority.",
        "dependencies": ["PES-ACC-0006", "PES-GOV-0018"],
    },
}


BLOCKED_REQUIREMENTS = {
    "PES-DEV-0009": "OQ-0001: initial OS and packaging model are intentionally undecided.",
    "PES-ACC-0005": "CR-0001 preserves the historical source; its supplied status still differs from the mandated exact wording.",
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
                # Preserve the table text exactly.  The former synthetic
                # "Table row:" prefix caused the PES-REQ-0003 fidelity defect.
                current.parts.append(" | ".join(values))
                current.table_rows += 1

    flush()
    return requirements


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

RECALL_ARTIFACTS = {
    "DomainResult {",
    "}",
    "Binding MUST/MUST NOT rules | Final Codex marching orders",
}


def normalized_source_text(text: str) -> str:
    text = text.replace("\u00ad", "").replace("\n", " ")
    text = re.sub(r"([A-Za-z])-\s+([A-Za-z])", r"\1\2", text)
    return re.sub(r"[^a-z0-9]+", " ", text.lower()).strip()


def split_source_sentences(text: str) -> list[str]:
    flattened = text.replace("\n", " ")
    return [
        sentence.strip()
        for sentence in re.split(
            r"(?<=[.!?])\s+(?=(?:\[PES-|[A-Z\"“]))", flattened
        )
        if sentence.strip()
    ]


def paragraph_is_numbered(paragraph: ET.Element) -> bool:
    properties = paragraph.find(f"{W}pPr")
    return bool(properties is not None and properties.find(f"{W}numPr") is not None)


def recall_blocks(document_path: Path) -> list[dict[str, object]]:
    body, _ = load_docx_parts(document_path)
    blocks: list[dict[str, object]] = []
    for body_block, child in enumerate(list(body), start=1):
        if child.tag == f"{W}p":
            text = paragraph_text(child).strip()
            if text:
                blocks.append(
                    {
                        "kind": "p",
                        "text": text,
                        "numbered": paragraph_is_numbered(child),
                        "bodyBlock": body_block,
                    }
                )
        elif child.tag == f"{W}tbl":
            rows = [row for row in table_rows(child) if any(row)]
            blocks.append(
                {
                    "kind": "table",
                    "rows": rows,
                    "text": " | ".join(" | ".join(row) for row in rows),
                    "numbered": False,
                    "bodyBlock": body_block,
                }
            )
    return blocks


def recall_sections(blocks: list[dict[str, object]]) -> dict[int, str]:
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


def inherited_recall_children(
    blocks: list[dict[str, object]],
) -> dict[tuple[str, int, int], tuple[int, str | None]]:
    inherited: dict[tuple[str, int, int], tuple[int, str | None]] = {}
    for index, block in enumerate(blocks):
        if (
            block["kind"] != "p"
            or not str(block["text"]).rstrip().endswith(":")
            or not MODAL_RE.search(str(block["text"]))
        ):
            continue
        match = re.search(r"\[(PES-[A-Z]+-\d{4})\]", str(block["text"]))
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


def collect_recall_units(document_path: Path) -> list[dict[str, object]]:
    blocks = recall_blocks(document_path)
    sections = recall_sections(blocks)
    inherited = inherited_recall_children(blocks)
    units: list[dict[str, object]] = []
    for index, block in enumerate(blocks):
        section = sections[index]
        if block["kind"] == "p":
            text = str(block["text"])
            direct_match = re.search(r"\[(PES-[A-Z]+-\d{4})\]", text)
            direct_id = direct_match.group(1) if direct_match else None
            inherited_block = inherited.get(("block", index, 0))
            if inherited_block:
                units.append(
                    {
                        "section": section,
                        "text": text.replace("\n", " "),
                        "requirementIds": [inherited_block[1]] if inherited_block[1] else [],
                        "kind": "inherited bullet",
                        "modalLeadIn": str(blocks[inherited_block[0]]["text"]).replace("\n", " "),
                        "bodyBlock": int(block["bodyBlock"]),
                    }
                )
                continue
            line_keys = [key for key in inherited if key[0] == "line" and key[1] == index]
            if line_keys:
                parent_index, parent_id = inherited[line_keys[0]]
                lead_in = str(blocks[parent_index]["text"]).replace("\n", " ")
                for line in text.splitlines():
                    if line.strip():
                        units.append(
                            {
                                "section": section,
                                "text": line.strip(),
                                "requirementIds": [parent_id] if parent_id else [],
                                "kind": "inherited line",
                                "modalLeadIn": lead_in,
                                "bodyBlock": int(block["bodyBlock"]),
                            }
                        )
                continue
            for sentence in split_source_sentences(text):
                if MODAL_RE.search(sentence):
                    units.append(
                        {
                            "section": section,
                            "text": sentence,
                            "requirementIds": [direct_id] if direct_id else [],
                            "kind": "explicit",
                            "modalLeadIn": None,
                            "bodyBlock": int(block["bodyBlock"]),
                        }
                    )
        else:
            for row_index, row in enumerate(block["rows"]):
                text = " | ".join(row)
                inherited_row = inherited.get(("row", index, row_index))
                if inherited_row:
                    units.append(
                        {
                            "section": section,
                            "text": text,
                            "requirementIds": [inherited_row[1]] if inherited_row[1] else [],
                            "kind": "inherited table row",
                            "modalLeadIn": str(blocks[inherited_row[0]]["text"]).replace("\n", " "),
                            "bodyBlock": int(block["bodyBlock"]),
                        }
                    )
                elif MODAL_RE.search(text):
                    units.append(
                        {
                            "section": section,
                            "text": text,
                            "requirementIds": [],
                            "kind": "explicit table row",
                            "modalLeadIn": None,
                            "bodyBlock": int(block["bodyBlock"]),
                        }
                    )

    units = [unit for unit in units if unit["text"] not in RECALL_ARTIFACTS]
    page_numbers: list[int] = []
    for page, count in enumerate(PAGE_STATEMENT_COUNTS, start=1):
        page_numbers.extend([page] * count)
    if len(units) != 546 or len(page_numbers) != 546:
        raise SystemExit(
            f"Recall population changed: units={len(units)}, page anchors={len(page_numbers)}"
        )
    for sequence, (unit, page) in enumerate(zip(units, page_numbers, strict=True), start=1):
        unit["id"] = f"T2-{sequence:04d}"
        unit["page"] = page
        if unit["text"] == "author;" and unit["requirementIds"] == ["PES-CRM-0017"]:
            unit["page"] = 18
        if not unit["requirementIds"] and unit["text"] in SOURCE_CROSS_MAPS:
            unit["requirementIds"] = list(SOURCE_CROSS_MAPS[unit["text"]])
    if sum(bool(unit["requirementIds"]) for unit in units) != 498:
        raise SystemExit("Pre-remediation mapped recall count changed from 498")
    return units


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
            "IMPLEMENTED_UNVERIFIED",
            "The real minimal foundation app, contract package, and Rust/WASM crate are wired through exact pnpm/Cargo workspaces, lockfiles, and pinned toolchains; automated evidence and reviewer acceptance remain external to the generated registry.",
            FOUNDATION_VERIFICATION[requirement_id],
        )
    if requirement_id == "PES-CI-0001":
        return (
            "PARTIAL",
            "The complete local closure gate and active checked-in CI configuration exist. CR-0001 authorizes configuration only; remote creation, push, hosted execution, credentials, service terms, and report upload remain outside this work, while later PLC-product obligations require later artifacts.",
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


def build_base_records(requirements: list[ExtractedRequirement]) -> tuple[list[dict], list[dict]]:
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


def compound_child_ids(parent_id: str, count: int) -> list[str]:
    area = parent_id.split("-")[1]
    start = COMPOUND_CHILD_STARTS[parent_id]
    return [f"PES-{area}-{number:04d}" for number in range(start, start + count)]


def compound_clauses(requirement: ExtractedRequirement) -> list[str]:
    clauses = requirement.text.splitlines()[1:]
    if requirement.requirement_id == "PES-REQ-0003":
        clauses = [clause for clause in clauses if clause != "Record | Identifier"]
    return clauses


def concise_title(text: str) -> str:
    title = re.sub(r"^(?:MUST NOT|SHALL NOT|MUST|SHALL|MAY)\s+", "", text).strip()
    title = title.rstrip(".;:")
    return title if len(title) <= 88 else title[:85].rstrip() + "..."


def lifecycle(
    status: str,
    *,
    parent_id: str | None = None,
    child_ids: list[str] | None = None,
    clause_ordinal: int | None = None,
    reason: str | None = None,
) -> dict[str, object]:
    return {
        "status": status,
        "parentId": parent_id,
        "childIds": child_ids or [],
        "clauseOrdinal": clause_ordinal,
        "changeRecordIds": ["CR-0001"],
        "supersededBy": child_ids or [],
        "reason": reason,
    }


def child_record(
    parent: dict[str, object],
    parent_requirement: ExtractedRequirement,
    child_id: str,
    clause: str,
    ordinal: int,
) -> dict[str, object]:
    result = copy.deepcopy(parent)
    modal_lead = parent_requirement.text.splitlines()[0]
    if parent_requirement.requirement_id == "PES-DEC-0001" and clause.startswith(
        "Meaningful autonomous decisions shall"
    ):
        atomic_text = clause
        keyword = "SHALL"
    else:
        atomic_text = f"{modal_lead}\n{clause}"
        keyword = str(parent["normativeKeyword"])
    result.update(
        {
            "id": child_id,
            "title": concise_title(clause),
            "normativeKeyword": keyword,
            "atomicRequirement": atomic_text,
            "atomicity": "ATOMIC_CHILD",
            "completionEligible": child_id != "PES-GOV-0032",
            "lifecycle": lifecycle(
                "SUPERSEDED" if child_id == "PES-GOV-0032" else "ACTIVE",
                parent_id=parent_requirement.requirement_id,
                clause_ordinal=ordinal,
                reason=(
                    "The corrective addendum supersedes the one-cumulative-DOCX exact-filename rule."
                    if child_id == "PES-GOV-0032"
                    else None
                ),
            ),
            "relatedRequirements": list(
                dict.fromkeys(
                    [*list(result.get("relatedRequirements", [])), parent_requirement.requirement_id]
                )
            ),
            "adrDecisionChangeLinks": list(
                dict.fromkeys([*list(result.get("adrDecisionChangeLinks", [])), "CR-0001"])
            ),
        }
    )
    source_pointer = copy.deepcopy(result["sourcePointer"])
    source_pointer.update(
        {
            "sourceUnitIds": [],
            "modalLeadIn": modal_lead,
            "clauseVerbatim": clause,
            "sourceVerbatim": parent_requirement.text,
        }
    )
    result["sourcePointer"] = source_pointer
    control = CI_CHILD_CONTROLS.get(child_id)
    if control is not None:
        current_evidence = str(control["evidence"])
        remaining_boundary = str(control["remaining"])
        result.update(
            {
                "truthState": "PARTIAL",
                "statusNote": (
                    "The active local/CI closure gate executes "
                    f"{current_evidence}; this remains PARTIAL because "
                    f"{remaining_boundary}."
                ),
                "positiveAcceptance": (
                    "For the admitted Phase 1 foundation scope, the active local/CI "
                    f"closure gate executes {current_evidence}. The requirement remains "
                    f"PARTIAL because {remaining_boundary}; complete acceptance requires "
                    "non-vacuous proof when that product surface exists."
                ),
                "negativeAcceptance": (
                    "Acceptance fails if the active gate stops executing its named "
                    f"current-scope controls for {current_evidence}, or if absence of a "
                    "later feature is presented as proof of that feature's behavior."
                ),
                "acceptanceMaturity": "CURATED_PHASE_1_CURRENT_SCOPE",
                "verificationIds": list(control["verificationIds"]),
                "implementationComponents": list(control["components"]),
                "reviewStatus": (
                    "AUTOMATED_CHECK_DEFINED; execution evidence is external to this "
                    "snapshot and later-scope acceptance is not recorded"
                ),
            }
        )
    return result


def gap_specifications(units_by_id: dict[str, dict[str, object]]) -> list[dict[str, object]]:
    specs: list[dict[str, object]] = []

    def add(
        requirement_id: str,
        unit_ids: list[str],
        atomic_requirement: str,
        acceptance: str,
        *,
        negative_acceptance: str | None = None,
        status_note: str | None = None,
        truth_state: str = "IMPLEMENTED_UNVERIFIED",
        verification_ids: list[str] | None = None,
        components: list[str] | None = None,
        milestone: str = "Phase 1 governance foundation",
        disposition: str = "FOUNDATION_WORK_ONLY",
    ) -> None:
        specs.append(
            {
                "id": requirement_id,
                "unitIds": unit_ids,
                "atomicRequirement": atomic_requirement,
                "acceptance": acceptance,
                "negativeAcceptance": negative_acceptance,
                "statusNote": status_note,
                "truthState": truth_state,
                "verificationIds": verification_ids or [],
                "components": components or [],
                "targetMilestone": milestone,
                "phase1Disposition": disposition,
            }
        )

    add(
        "PES-REQ-0010",
        ["T2-0008"],
        str(units_by_id["T2-0008"]["text"]),
        "The gate treats MUST and SHALL as required and blocks a violated obligation.",
        verification_ids=["VER-REQ-0001"],
        components=["REQUIREMENTS.md", "tools/phase1/verify-phase1.mjs"],
    )
    add(
        "PES-REQ-0011",
        ["T2-0009"],
        str(units_by_id["T2-0009"]["text"]),
        "The gate treats MUST NOT and SHALL NOT as prohibited and blocks their presence.",
        verification_ids=["VER-REQ-0001"],
        components=["REQUIREMENTS.md", "tools/phase1/verify-phase1.mjs"],
    )
    add(
        "PES-REQ-0012",
        ["T2-0010"],
        str(units_by_id["T2-0010"]["text"]),
        "A MAY record is accepted only when its behavior remains inside approved scope.",
        verification_ids=["VER-REQ-0001"],
        components=["REQUIREMENTS.md", "tools/phase1/verify-phase1.mjs"],
    )

    for offset, unit_number in enumerate(range(287, 295), start=31):
        unit_id = f"T2-{unit_number:04d}"
        unit = units_by_id[unit_id]
        requirement_id = f"PES-ARC-{offset:04d}"
        field_name = DOMAIN_RESULT_FIELD_BY_ID[requirement_id]
        add(
            requirement_id,
            [unit_id],
            f"{unit['modalLeadIn']}\n{unit['text']}",
            (
                f"The typed foundation DomainResult declares and strictly validates the {field_name} member, "
                "the worker constructs and validates the envelope around the deterministic zero-import Rust/WASM "
                "health result, and the local UI receives that validated result through the isolated worker path."
            ),
            negative_acceptance=(
                f"A missing, extra, wrongly optional, malformed, or inconsistently propagated {field_name} member; "
                "an unvalidated worker result; or a UI path that bypasses the typed contract fails current-scope acceptance."
            ),
            status_note=(
                f"The real Phase 1 health round trip implements and tests the {field_name} DomainResult member "
                "across the typed contract, worker/WASM boundary, and UI integration. This is current-scope "
                "foundation evidence only, not a claim that later PLC-domain mutations exist or are verified."
            ),
            truth_state="IMPLEMENTED_UNVERIFIED",
            verification_ids=["VER-FND-0001", "VER-REQ-0002", "VER-CI-0001"],
            components=DOMAIN_RESULT_COMPONENTS,
            milestone="Phase 1 minimal technical foundation",
            disposition="FOUNDATION_WORK_ONLY",
        )

    governance_files = [
        ("PES-DOC-0005", "T2-0366", "LEGAL_REVIEW_CHECKLIST.md"),
        ("PES-DOC-0006", "T2-0369", "REQUIREMENTS.md"),
        ("PES-DOC-0007", "T2-0370", "IMPLEMENTATION_MATRIX.json"),
        ("PES-DOC-0008", "T2-0373", "DEPENDENCY_POLICY.md"),
        ("PES-DOC-0009", "T2-0374", "OPEN_DECISIONS.md"),
        ("PES-DOC-0010", "T2-0375", "RISK_REGISTER.md"),
    ]
    for requirement_id, unit_id, path in governance_files:
        unit = units_by_id[unit_id]
        add(
            requirement_id,
            ["T2-0363", unit_id],
            f"{unit['modalLeadIn']}\n{unit['text']}",
            f"{path} exists at the exact top-level path, is nonempty, and is controlled by the trusted baseline.",
            verification_ids=["VER-DOC-0001"],
            components=[path],
        )

    for offset, unit_number in enumerate(range(419, 435), start=20):
        unit_id = f"T2-{unit_number:04d}"
        unit = units_by_id[unit_id]
        add(
            f"PES-REQ-{offset:04d}",
            ["T2-0418", unit_id],
            f"{unit['modalLeadIn']}\n{unit['text']}",
            f"Schema validation rejects a requirement record that omits or invalidates {unit['text']}",
            verification_ids=["VER-REQ-0002"],
            components=["requirements/phase1-requirements.json", "tools/phase1/verify-phase1.mjs"],
        )

    for offset, unit_number in enumerate(range(469, 480), start=27):
        unit_id = f"T2-{unit_number:04d}"
        unit = units_by_id[unit_id]
        add(
            f"PES-DEC-{offset:04d}",
            ["T2-0468", unit_id],
            f"{unit['modalLeadIn']}\n{unit['text']}",
            f"Decision-record validation rejects a blocked request that omits {unit['text']}",
            verification_ids=["VER-DEC-0001"],
            components=["OPEN_DECISIONS.md", "tools/phase1/verify-phase1.mjs"],
        )

    for requirement_id, noun in [
        ("PES-ACC-0008", "accessibility conformance target"),
        ("PES-ACC-0009", "performance budget"),
        ("PES-ACC-0010", "capacity budget"),
    ]:
        add(
            requirement_id,
            ["T2-0544"],
            f"MUST define an objective {noun} before experience acceptance.",
            f"Experience acceptance remains blocked until an objective {noun} is approved and recorded.",
            truth_state="DEFERRED",
            components=["OPEN_DECISIONS.md"],
            milestone="Phase 3 experience acceptance",
            disposition="RESERVED_LATER_PHASE_NO_PRODUCT_AUTHORIZATION",
        )

    if len(specs) != 47:
        raise SystemExit(f"Expected 47 recall-gap records, found {len(specs)}")
    return specs


def gap_record(spec: dict[str, object], units_by_id: dict[str, dict[str, object]]) -> dict[str, object]:
    requirement_id = str(spec["id"])
    area = requirement_id.split("-")[1]
    units = [units_by_id[unit_id] for unit_id in spec["unitIds"]]
    principal = units[-1]
    keyword = normative_keyword(str(spec["atomicRequirement"]))
    classes = (
        [1]
        if area in {"REQ", "DOC", "DEC"} or requirement_id in DOMAIN_RESULT_FIELD_BY_ID
        else [8]
    )
    return {
        "id": requirement_id,
        "title": concise_title(str(spec["atomicRequirement"])),
        "normativeKeyword": keyword,
        "atomicRequirement": spec["atomicRequirement"],
        "atomicity": "BASELINE_ATOMIC",
        "completionEligible": True,
        "lifecycle": lifecycle("ACTIVE"),
        "rationale": "Issued by CR-0001 to close an independently reproduced normative-source recall gap.",
        "scopeComponent": AREA_COMPONENTS[area],
        "sourcePointer": {
            "sourceId": "SRC-0002",
            "file": DIRECTIVE_NAME,
            "sha256": DIRECTIVE_SHA256,
            "page": principal["page"],
            "headingPath": [principal["section"]],
            "bodyBlock": principal["bodyBlock"],
            "sourceUnitIds": list(spec["unitIds"]),
            "modalLeadIn": principal.get("modalLeadIn"),
            "clauseVerbatim": principal["text"],
            "sourceVerbatim": "\n".join(str(unit["text"]) for unit in units),
            "researchClassification": "DIRECTIVE_ADOPTED_NORMATIVE; WP-0C recall reconciliation",
        },
        "ipClassification": {
            "classes": classes,
            "disposition": (
                "Implement only the original Phase 1 governance control."
                if classes == [1]
                else "BLOCKED_PENDING_SUBJECT_AWARE_PROFESSIONAL_REVIEW"
            ),
            "basis": "CR-0001 source-unit reconciliation.",
            "classificationMethod": (
                "CURATED_PHASE_1_REQUIREMENT_ID_REVIEW"
                if classes == [1]
                else "UNRESOLVED_DEFAULT_CLASS_8"
            ),
            "reviewStatus": "WP-0C_RECONCILED; not product acceptance or legal advice",
        },
        "candidateIpFlags": candidate_ip_flags(str(spec["atomicRequirement"])),
        "dependencies": [],
        "relatedRequirements": [],
        "dependencyMaturity": "CURATED_PHASE_1_RELATIONSHIPS; no blocking prerequisite asserted",
        "targetMilestone": spec["targetMilestone"],
        "phase1Disposition": spec["phase1Disposition"],
        "truthState": spec["truthState"],
        "statusNote": spec.get("statusNote") or "CR-0001 issued this atomic record from an unmapped directive source unit.",
        "positiveAcceptance": spec["acceptance"],
        "negativeAcceptance": spec.get("negativeAcceptance") or f"Missing objective evidence for this obligation fails acceptance: {spec['acceptance']}",
        "acceptanceMaturity": "CURATED_PHASE_1_CURRENT_SCOPE",
        "verificationIds": spec["verificationIds"],
        "adrDecisionChangeLinks": ["CR-0001"],
        "implementationComponents": spec["components"],
        "owner": "Scott",
        "reviewer": "UNASSIGNED",
        "reviewStatus": (
            "AUTOMATED_CHECK_DEFINED; execution evidence is external to this snapshot and reviewer acceptance is not recorded"
            if spec["verificationIds"]
            else "IMPLEMENTED_UNVERIFIED where an artifact exists; reviewer acceptance is not recorded"
        ),
        "reviewDate": None,
    }


def matrix_entry(record: dict[str, object]) -> dict[str, object]:
    return {
        "requirementId": record["id"],
        "component": record["scopeComponent"],
        "targetMilestone": record["targetMilestone"],
        "phase1Disposition": record["phase1Disposition"],
        "truthState": record["truthState"],
        "lifecycleStatus": record["lifecycle"]["status"],
        "parentId": record["lifecycle"]["parentId"],
        "completionEligible": record["completionEligible"],
        "verificationIds": record["verificationIds"],
        "implementationComponents": record["implementationComponents"],
        "decisionLinks": record["adrDecisionChangeLinks"],
        "notes": record["statusNote"],
    }


def build_records(
    requirements: list[ExtractedRequirement], units: list[dict[str, object]]
) -> tuple[list[dict[str, object]], list[dict[str, object]], list[dict[str, object]]]:
    base_records, _ = build_base_records(requirements)
    extracted_by_id = {item.requirement_id: item for item in requirements}
    records_by_id = {str(item["id"]): item for item in base_records}
    records: list[dict[str, object]] = []
    split_ledger: list[dict[str, object]] = []
    for requirement in requirements:
        parent = records_by_id[requirement.requirement_id]
        parent["sourcePointer"].update(
            {
                "sourceUnitIds": [],
                "modalLeadIn": requirement.text.splitlines()[0],
                "clauseVerbatim": None,
                "sourceVerbatim": requirement.text,
            }
        )
        if requirement.requirement_id not in COMPOUND_CHILD_STARTS:
            parent["atomicity"] = "BASELINE_ATOMIC"
            if requirement.requirement_id == "PES-GOV-0018":
                parent["completionEligible"] = True
                parent["lifecycle"] = lifecycle(
                    "PARTIALLY_SUPERSEDED",
                    reason=(
                        "CR-0001 authorizes four separate phase directives, while the "
                        "narrowed prohibition on unauthorized competing directives remains."
                    ),
                )
                parent["statusNote"] = (
                    "CR-0001 partially supersedes this requirement for the four authorized "
                    "phase directives; the continuing rule prohibits unauthorized competing "
                    "authorities."
                )
                parent["adrDecisionChangeLinks"] = list(
                    dict.fromkeys([*list(parent["adrDecisionChangeLinks"]), "CR-0001"])
                )
            else:
                parent["completionEligible"] = True
                parent["lifecycle"] = lifecycle("ACTIVE")
            records.append(parent)
            continue

        clauses = compound_clauses(requirement)
        expected = EXPECTED_COMPOUND_CHILD_COUNTS[requirement.requirement_id]
        if len(clauses) != expected:
            raise SystemExit(
                f"{requirement.requirement_id} child count changed: expected {expected}, got {len(clauses)}"
            )
        child_ids = compound_child_ids(requirement.requirement_id, len(clauses))
        # Children inherit the parent's reviewed implementation/evidence data as
        # it existed before the historical parent is retired from completion.
        # The parent itself is then stripped of completion-bearing mappings so
        # it cannot double count the same obligation.
        parent_for_children = copy.deepcopy(parent)
        parent["atomicity"] = "SUPERSEDED_COMPOUND_PARENT"
        parent["completionEligible"] = False
        parent["lifecycle"] = lifecycle(
            "SUPERSEDED_PARENT",
            child_ids=child_ids,
            reason="CR-0001 split the source compound without deleting or reusing its issued ID.",
        )
        parent["verificationIds"] = []
        parent["implementationComponents"] = []
        parent["statusNote"] = (
            "Historical compound parent; the active local closure gate and checked-in "
            "CI configuration are real, while exact current-scope controls and remaining "
            "later-product proof obligations attach to the atomic children."
            if requirement.requirement_id == "PES-CI-0001"
            else "Historical compound parent; current acceptance and evidence attach to atomic children."
        )
        parent["adrDecisionChangeLinks"] = list(
            dict.fromkeys([*list(parent["adrDecisionChangeLinks"]), "CR-0001"])
        )
        records.append(parent)
        children = [
            child_record(parent_for_children, requirement, child_id, clause, ordinal)
            for ordinal, (child_id, clause) in enumerate(zip(child_ids, clauses, strict=True), start=1)
        ]
        records.extend(children)
        split_ledger.append(
            {
                "parentId": requirement.requirement_id,
                "sourceVerbatim": requirement.text,
                "childIds": child_ids,
                "clauses": [
                    {"ordinal": index, "childId": child_id, "verbatim": clause}
                    for index, (child_id, clause) in enumerate(zip(child_ids, clauses, strict=True), start=1)
                ],
                "changeRecordId": "CR-0001",
            }
        )

    units_by_id = {str(unit["id"]): unit for unit in units}
    for spec in gap_specifications(units_by_id):
        records.append(gap_record(spec, units_by_id))

    ids = [str(record["id"]) for record in records]
    if len(records) != 484 or len(set(ids)) != 484:
        raise SystemExit(f"Expected 484 unique issued records, got {len(records)}/{len(set(ids))}")
    if sum(record["atomicity"] == "SUPERSEDED_COMPOUND_PARENT" for record in records) != 20:
        raise SystemExit("Expected exactly 20 superseded compound parents")
    if sum(record["atomicity"] != "SUPERSEDED_COMPOUND_PARENT" for record in records) != 464:
        raise SystemExit("Expected exactly 464 atomic records")
    return records, [matrix_entry(record) for record in records], split_ledger


def build_reconciliation(
    requirements: list[ExtractedRequirement],
    units: list[dict[str, object]],
    records: list[dict[str, object]],
    split_ledger: list[dict[str, object]],
) -> dict[str, object]:
    extracted_by_id = {item.requirement_id: item for item in requirements}
    records_by_id = {str(record["id"]): record for record in records}
    child_ids_by_parent: dict[str, list[str]] = {}
    child_by_clause: dict[str, dict[str, str]] = {}
    for split in split_ledger:
        parent_id = str(split["parentId"])
        child_ids_by_parent[parent_id] = list(split["childIds"])
        child_by_clause[parent_id] = {
            normalized_source_text(str(item["verbatim"])): str(item["childId"])
            for item in split["clauses"]
        }

    units_by_id = {str(unit["id"]): unit for unit in units}
    gap_specs = gap_specifications(units_by_id)
    gap_ids_by_unit: dict[str, list[str]] = {}
    for spec in gap_specs:
        for unit_id in spec["unitIds"]:
            gap_ids_by_unit.setdefault(str(unit_id), []).append(str(spec["id"]))

    pre_unmapped = [str(unit["id"]) for unit in units if not unit["requirementIds"]]
    expected_unmapped = [
        "T2-0008", "T2-0009", "T2-0010",
        *[f"T2-{number:04d}" for number in range(287, 295)],
        "T2-0363", "T2-0366", "T2-0369", "T2-0370", "T2-0373", "T2-0374", "T2-0375",
        *[f"T2-{number:04d}" for number in range(418, 435)],
        *[f"T2-{number:04d}" for number in range(468, 480)],
        "T2-0544",
    ]
    if pre_unmapped != expected_unmapped:
        raise SystemExit(
            "The independently reproduced 48-unit gap population changed: "
            + ", ".join(pre_unmapped)
        )

    reconciled_units: list[dict[str, object]] = []
    gap_dispositions: list[dict[str, object]] = []
    for unit in units:
        unit_id = str(unit["id"])
        old_ids = [str(value) for value in unit["requirementIds"]]
        historical_parents: list[str] = []
        active_ids: list[str] = []
        for old_id in old_ids:
            if old_id not in child_ids_by_parent:
                active_ids.append(old_id)
                continue
            historical_parents.append(old_id)
            parent = extracted_by_id[old_id]
            normalized_unit = normalized_source_text(str(unit["text"]))
            normalized_lead = normalized_source_text(
                f"[{old_id}] {parent.text.splitlines()[0]}"
            )
            if normalized_unit == normalized_lead:
                active_ids.extend(child_ids_by_parent[old_id])
            elif normalized_unit in child_by_clause[old_id]:
                active_ids.append(child_by_clause[old_id][normalized_unit])
            elif (
                old_id == "PES-CRM-0008"
                and str(unit["text"])
                == "2 | Industry or IEC convention | Implement from lawfully licensed standards or public behavior"
            ):
                active_ids.append("PES-CRM-0027")
            elif (
                old_id == "PES-CRM-0017"
                and str(unit["text"]) == "EVIDENCE_REGISTER.*"
            ) or (
                old_id == "PES-CRM-0021"
                and str(unit["text"]) == "ASSET_PROVENANCE.*"
            ):
                # The directive's top-level governance-file inventory points to
                # each full register contract, so the file source unit fans out
                # to every atomic child of that contract.
                active_ids.extend(child_ids_by_parent[old_id])
            else:
                raise SystemExit(
                    f"Cannot reconcile compound mapping {unit_id} -> {old_id}: {unit['text']}"
                )

        if not old_ids:
            active_ids.extend(gap_ids_by_unit.get(unit_id, []))
        active_ids = list(dict.fromkeys(active_ids))
        if not active_ids:
            raise SystemExit(f"Unmapped source unit remains after WP-0C: {unit_id}")
        reconciled = {
            "id": unit_id,
            "page": unit["page"],
            "section": unit["section"],
            "kind": unit["kind"],
            "text": unit["text"],
            "modalLeadIn": unit.get("modalLeadIn"),
            "bodyBlock": unit["bodyBlock"],
            "disposition": "MAPPED",
            "requirementIds": active_ids,
            "historicalParentIds": list(dict.fromkeys(historical_parents)),
        }
        reconciled_units.append(reconciled)
        if unit_id in expected_unmapped:
            gap_dispositions.append(
                {
                    "sourceUnitId": unit_id,
                    "preRemediationDisposition": "UNMAPPED",
                    "finalDisposition": "MAPPED",
                    "requirementIds": active_ids,
                    "acceptanceMethods": [
                        str(records_by_id[requirement_id]["positiveAcceptance"])
                        for requirement_id in active_ids
                    ],
                }
            )

    relationship_count = sum(len(unit["requirementIds"]) for unit in reconciled_units)
    if len(reconciled_units) != 546:
        raise SystemExit(f"Expected 546 reconciled source units, got {len(reconciled_units)}")
    if len(gap_dispositions) != 48:
        raise SystemExit(f"Expected 48 final gap dispositions, got {len(gap_dispositions)}")
    if relationship_count != 789:
        raise SystemExit(
            f"Expected 789 source-unit-to-issued-ID relationships, got {relationship_count}"
        )

    source_units_by_requirement: dict[str, list[str]] = {}
    historical_units_by_parent: dict[str, list[str]] = {}
    for unit in reconciled_units:
        for requirement_id in unit["requirementIds"]:
            source_units_by_requirement.setdefault(str(requirement_id), []).append(str(unit["id"]))
        for parent_id in unit["historicalParentIds"]:
            historical_units_by_parent.setdefault(str(parent_id), []).append(str(unit["id"]))
    for record in records:
        requirement_id = str(record["id"])
        if record["atomicity"] == "SUPERSEDED_COMPOUND_PARENT":
            record["sourcePointer"]["sourceUnitIds"] = historical_units_by_parent.get(
                requirement_id, []
            )
        else:
            record["sourcePointer"]["sourceUnitIds"] = source_units_by_requirement.get(
                requirement_id, record["sourcePointer"].get("sourceUnitIds", [])
            )

    return {
        "schemaVersion": 3,
        "snapshotDate": "2026-08-27",
        "generatedBy": "tools/phase1/extract_directive_requirements.py",
        "directive": {"path": DIRECTIVE_NAME, "sha256": DIRECTIVE_SHA256},
        "correctiveAuthority": {
            "file": CORRECTIVE_ADDENDUM_NAME,
            "sha256": CORRECTIVE_ADDENDUM_SHA256,
            "changeRecordId": "CR-0001",
        },
        "method": {
            "triggerScope": "shall; must; never; is prohibited/forbidden/required; limited-permission may ... only; inherited modal children",
            "unitRule": "A modal lead-in and each separately testable governed child are distinct source units.",
            "pageAnchors": "Forty-page Word render distribution independently reproduced by the adversarial audit.",
            "mappingRule": "Every atomic child links to its modal lead-in and exact clause. Historical compound parents remain lineage only. File-inventory aliases that name a complete register fan out to every atomic child of that register contract.",
        },
        "counts": {
            "sourceParentIdCount": 247,
            "issuedIdCount": 484,
            "supersededCompoundParentCount": 20,
            "atomicRecordCount": 464,
            "completionEligibleAtomicRecordCount": sum(
                bool(record["completionEligible"]) for record in records
            ),
            "sourceStatementUnitCount": 546,
            "mappedStatementUnitCount": 546,
            "unmappedStatementUnitCount": 0,
            "sourceUnitRelationshipCount": relationship_count,
        },
        "gapDispositions": gap_dispositions,
        "compoundSplits": split_ledger,
        "sourceUnits": reconciled_units,
    }


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
    units = collect_recall_units(directive)
    records, matrix_entries, split_ledger = build_records(requirements, units)
    reconciliation = build_reconciliation(
        requirements, units, records, split_ledger
    )
    state_counts: dict[str, int] = {}
    completion_eligible_state_counts: dict[str, int] = {}
    for entry in matrix_entries:
        state_counts[entry["truthState"]] = state_counts.get(entry["truthState"], 0) + 1
        if entry["completionEligible"]:
            completion_eligible_state_counts[entry["truthState"]] = (
                completion_eligible_state_counts.get(entry["truthState"], 0) + 1
            )

    registry = {
        "schemaVersion": 3,
        "snapshotDate": "2026-08-27",
        "generatedBy": "tools/phase1/extract_directive_requirements.py",
        "generatorSha256": generator_hash,
        "directive": {
            "path": DIRECTIVE_NAME,
            "sha256": DIRECTIVE_SHA256,
            "status": "Phase 1 supplied; CR-0001 resolves the document-control conflict and preserves this file as the canonical Phase 1 directive",
        },
        "researchBaseline": {"path": RESEARCH_NAME, "sha256": RESEARCH_SHA256},
        "correctiveAuthority": {
            "path": CORRECTIVE_ADDENDUM_NAME,
            "sha256": CORRECTIVE_ADDENDUM_SHA256,
            "changeRecordId": "CR-0001",
        },
        "counts": {
            "sourceParentIdCount": 247,
            "issuedIdCount": 484,
            "supersededCompoundParentCount": 20,
            "atomicRecordCount": 464,
            "completionEligibleAtomicRecordCount": sum(
                bool(record["completionEligible"]) for record in records
            ),
            "sourceStatementUnitCount": 546,
            "mappedStatementUnitCount": 546,
            "unmappedStatementUnitCount": 0,
            "sourceUnitRelationshipCount": 789,
        },
        "requirementCount": len(records),
        "requirements": records,
    }
    matrix = {
        "schemaVersion": 3,
        "snapshotDate": "2026-08-27",
        "generatedBy": "tools/phase1/extract_directive_requirements.py",
        "generatorSha256": generator_hash,
        "completionRule": "Only completion-eligible atomic records may be counted, and only VERIFIED means complete. Historical compound parents and superseded records are never completion-bearing. No completion percentage is calculated.",
        "scope": "Phase 1 governance plus the minimal non-PLC technical foundation; PLC-domain and Phase 2 product implementation are not started.",
        "counts": {
            "sourceParentIdCount": 247,
            "issuedIdCount": 484,
            "supersededCompoundParentCount": 20,
            "atomicRecordCount": 464,
            "completionEligibleAtomicRecordCount": sum(
                bool(record["completionEligible"]) for record in records
            ),
            "sourceStatementUnitCount": 546,
            "mappedStatementUnitCount": 546,
            "unmappedStatementUnitCount": 0,
            "sourceUnitRelationshipCount": 789,
        },
        "requirementCount": len(matrix_entries),
        "stateCounts": dict(sorted(state_counts.items())),
        "completionEligibleStateCounts": dict(
            sorted(completion_eligible_state_counts.items())
        ),
        "entries": matrix_entries,
    }
    outputs = {
        root / "requirements" / "phase1-requirements.json": registry,
        root / "IMPLEMENTATION_MATRIX.json": matrix,
        root / "requirements" / "phase1-reconciliation.json": reconciliation,
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
            f"Verified {len(records)} issued records, {len(matrix_entries)} matrix entries, and {len(units)} reconciled source units are current."
        )
        return

    for path, value in outputs.items():
        write_json(path, value)
    print(
        f"Wrote {len(records)} issued records, {len(matrix_entries)} matrix entries, and {len(units)} reconciled source units."
    )


if __name__ == "__main__":
    main()
