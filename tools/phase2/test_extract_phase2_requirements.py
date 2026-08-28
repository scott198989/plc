from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile


MODULE_PATH = Path(__file__).with_name("extract_phase2_requirements.py")
SPEC = importlib.util.spec_from_file_location("phase2_extractor", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
extractor = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = extractor
SPEC.loader.exec_module(extractor)


class Phase2ExtractorTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        fixture_dir = Path(__file__).with_name("fixtures")
        self.directive = self.root / "directive.docx"
        with ZipFile(self.directive, "w", compression=ZIP_DEFLATED) as package:
            package.writestr(
                "word/document.xml",
                (fixture_dir / "document.xml").read_bytes(),
            )
            package.writestr(
                "word/styles.xml",
                (fixture_dir / "styles.xml").read_bytes(),
            )
        self.phase1 = self.root / "phase1.json"
        self.phase1.write_bytes((fixture_dir / "phase1-requirements.json").read_bytes())
        self.evidence = self.root / "p2-entry.json"
        self.evidence.write_text(
            json.dumps(
                {
                    "gate": "P2-00",
                    "status": "PASS",
                    "authority": {
                        "directiveSha256": extractor.sha256_file(self.directive)
                    },
                }
            ),
            encoding="utf-8",
        )

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def build(self):
        return extractor.build_outputs(
            root=self.root,
            directive_path=self.directive,
            phase1_registry_path=self.phase1,
            p2_00_evidence_path=self.evidence,
            expected_directive_sha256=extractor.sha256_file(self.directive),
        )

    def test_extracts_unique_requirements_and_honest_truth(self) -> None:
        outputs = self.build()
        requirements = outputs["requirements"]["requirements"]
        self.assertEqual(
            [item["id"] for item in requirements],
            ["PES-ACC-0001", "PES-GOV-0040", "PES-ZZZ-0001"],
        )
        entry = next(item for item in requirements if item["id"] == "PES-GOV-0040")
        self.assertEqual(entry["truthState"], "IMPLEMENTED_UNVERIFIED")
        self.assertEqual(entry["evidence"][0]["path"], "p2-entry.json")
        self.assertFalse(any(item["truthState"] == "VERIFIED" for item in requirements))
        acc = next(item for item in requirements if item["id"] == "PES-ACC-0001")
        self.assertEqual(acc["exactText"], "[PES-ACC-0001] MUST provide a real journey.")
        self.assertEqual(acc["sourcePointer"]["paragraphOrdinal"], 2)

    def test_audit_detects_required_governance_fault_classes(self) -> None:
        findings = self.build()["audit"]["findings"]
        self.assertEqual(
            findings["duplicateRequirementDefinitions"][0]["requirementId"],
            "PES-ACC-0001",
        )
        self.assertEqual(findings["unknownAreaTokens"], ["ZZZ"])
        self.assertEqual(findings["unknownRequirementReferences"], ["PES-GOV-9999"])
        self.assertEqual(findings["unknownVerificationReferences"], ["VER-GOV-9999"])
        self.assertEqual(findings["orphanVerificationRecords"], ["VER-NAV-0001"])
        self.assertEqual(
            findings["reusedPhase1Ids"][0]["requirementId"], "PES-ACC-0001"
        )
        self.assertEqual(
            findings["unresolvedNormativePlaceholders"][0]["token"], "TBD"
        )

    def test_area_mapping_is_a_non_verified_skeleton(self) -> None:
        output = self.build()["verification"]
        mapping = {
            item["requirementId"]: item
            for item in output["requirementMappingSkeleton"]
        }
        self.assertEqual(
            mapping["PES-GOV-0040"]["candidateVerificationIds"],
            ["VER-GOV-0001"],
        )
        self.assertFalse(mapping["PES-GOV-0040"]["verified"])
        self.assertEqual(
            mapping["PES-ZZZ-0001"]["mappingStatus"],
            "NO_AREA_CANDIDATE_REVIEW_REQUIRED",
        )

    def test_write_and_check_are_byte_deterministic(self) -> None:
        first = self.build()
        second = self.build()
        self.assertEqual(
            extractor.json_bytes(first["requirements"]),
            extractor.json_bytes(second["requirements"]),
        )
        extractor.write_outputs(self.root, first)
        self.assertTrue(extractor.check_outputs(self.root, second))
        stale = self.root / extractor.OUTPUT_PATHS["audit"]
        stale.write_text("{}\n", encoding="utf-8")
        self.assertFalse(extractor.check_outputs(self.root, second))

    def test_rejects_a_source_hash_mismatch(self) -> None:
        with self.assertRaises(extractor.ExtractionError):
            extractor.build_outputs(
                root=self.root,
                directive_path=self.directive,
                phase1_registry_path=self.phase1,
                p2_00_evidence_path=self.evidence,
                expected_directive_sha256="0" * 64,
            )


if __name__ == "__main__":
    unittest.main()
