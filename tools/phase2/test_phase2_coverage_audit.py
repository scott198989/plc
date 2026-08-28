from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path


TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parents[1]
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

from generate_phase2_coverage_audit import (  # noqa: E402
    ALLOWED_CLASSIFICATIONS,
    EXPECTED_REQUIREMENTS,
    EXPECTED_VERIFICATIONS,
    MISSING,
    PARTIAL,
    READY,
    build_audit,
    check_or_write,
    render_json,
    render_report,
)
import reviewed_requirement_mapping  # noqa: E402


class Phase2CoverageAuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.audit = build_audit(ROOT)

    def test_enumerates_all_requirements_and_appendix_h_without_credit(self) -> None:
        self.assertEqual(len(self.audit["verificationAssessments"]), EXPECTED_VERIFICATIONS)
        self.assertEqual(len(self.audit["requirementCoverage"]), EXPECTED_REQUIREMENTS)
        self.assertEqual(self.audit["summary"]["verificationCreditGranted"], 0)
        self.assertTrue(
            all(record["verificationCredit"] == "NONE" for record in self.audit["verificationAssessments"])
        )
        self.assertTrue(
            all(record["verificationCredit"] == "NONE" for record in self.audit["requirementCoverage"])
        )
        self.assertTrue(all(record["executionEvidenceIds"] == [] for record in self.audit["requirementCoverage"]))
        self.assertNotIn("VERIFIED", self.audit["summary"]["requirementTruthStateCounts"])
        self.assertEqual(
            {record["mappingStatus"] for record in self.audit["requirementCoverage"]},
            {"REVIEWED"},
        )
        self.assertEqual(
            self.audit["summary"]["reviewedRequirementMappingCount"],
            EXPECTED_REQUIREMENTS,
        )
        self.assertTrue(
            all(
                record["selectedVerificationIds"]
                and set(record["selectedVerificationIds"]).issubset(
                    record["candidateVerificationIds"]
                )
                and 20 <= len(record["mappingReviewerRationale"]) <= 240
                for record in self.audit["requirementCoverage"]
            )
        )
        self.assertFalse(
            any(
                "UNREVIEWED" in record["coverageSignal"]
                for record in self.audit["requirementCoverage"]
            )
        )

    def test_reviewed_mapping_binding_is_exact(self) -> None:
        path = ROOT / reviewed_requirement_mapping.REVIEWED_MAPPING_PATH
        expected = hashlib.sha256(path.read_bytes()).hexdigest().upper()
        self.assertEqual(
            self.audit["binding"]["reviewedRequirementMappingSha256"], expected
        )
        artifact = json.loads(path.read_text(encoding="utf-8"))
        self.assertEqual(
            self.audit["binding"]["reviewedMappingRowsSha256"],
            artifact["binding"]["reviewedRowsSha256"],
        )

    def test_classifications_are_conservative_and_gap_complete(self) -> None:
        observed = {record["classification"] for record in self.audit["verificationAssessments"]}
        self.assertTrue(observed)
        self.assertLessEqual(observed, ALLOWED_CLASSIFICATIONS)
        for record in self.audit["verificationAssessments"]:
            if record["classification"] == READY:
                self.assertEqual(record["uncoveredProofClauses"], [])
                self.assertIsNone(record["gapLaneId"])
            elif record["classification"] in {PARTIAL, MISSING}:
                self.assertTrue(record["uncoveredProofClauses"])
                self.assertIsInstance(record["gapLaneId"], str)

    def test_cited_paths_exist_and_lanes_do_not_overlap(self) -> None:
        lane_members: set[str] = set()
        for record in self.audit["verificationAssessments"]:
            for relative in record["implementationPaths"] + record["testPaths"]:
                self.assertTrue((ROOT / relative).is_file(), relative)
        for lane in self.audit["independentGapLanes"]:
            members = set(lane["verificationIds"])
            self.assertTrue(members)
            self.assertFalse(lane_members & members)
            lane_members |= members
        incomplete = {
            record["verificationId"]
            for record in self.audit["verificationAssessments"]
            if record["classification"] != READY
        }
        self.assertEqual(lane_members, incomplete)

    def test_rendering_and_write_check_are_byte_deterministic(self) -> None:
        second = build_audit(ROOT)
        self.assertEqual(render_json(self.audit), render_json(second))
        self.assertEqual(render_report(self.audit), render_report(second))
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            json_path = directory / "audit.json"
            report_path = directory / "audit.md"
            json_text = render_json(self.audit)
            report_text = render_report(self.audit)
            self.assertTrue(check_or_write(json_path, json_text, False))
            self.assertTrue(check_or_write(report_path, report_text, False))
            self.assertTrue(check_or_write(json_path, json_text, True))
            self.assertTrue(check_or_write(report_path, report_text, True))
            self.assertEqual(json.loads(json_path.read_text(encoding="utf-8"))["schemaVersion"], 1)
            json_path.write_text(json_text + " ", encoding="utf-8")
            self.assertFalse(check_or_write(json_path, json_text, True))

    def test_checked_in_outputs_are_current(self) -> None:
        self.assertEqual(
            (ROOT / "evidence/phase2/PHASE2_COVERAGE_AUDIT.json").read_text(encoding="utf-8"),
            render_json(self.audit),
        )
        self.assertEqual(
            (ROOT / "evidence/phase2/PHASE2_COVERAGE_AUDIT.md").read_text(encoding="utf-8"),
            render_report(self.audit),
        )


if __name__ == "__main__":
    unittest.main()
