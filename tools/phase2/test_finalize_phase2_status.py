from __future__ import annotations

import hashlib
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import finalize_phase2_status as finalizer


class FinalizePhase2StatusTests(unittest.TestCase):
    def binding(self) -> dict[str, object]:
        return {
            "candidateCommit": "1" * 40,
            "candidateTree": "2" * 40,
            "isolationApprovalDecisionId": "P2-DEC-ISO-NATIVE-001",
            "isolationApprovalSha256": "9" * 64,
            "productionSourceSha256": "A" * 64,
            "testSourceSha256": "B" * 64,
            "requirementsSourceSha256": "C" * 64,
            "requirementRegistrySha256": "D" * 64,
            "verificationCatalogSha256": "E" * 64,
            "reviewedRequirementMappingSha256": "8" * 64,
            "directiveSha256": "F" * 64,
            "productionSourceFileCount": 1,
            "testSourceFileCount": 1,
            "requirementSourceFileCount": 1,
            "workspaceCrates": ["test"],
        }

    def audit(self, count: int = 2) -> dict[str, object]:
        return {
            "binding": {
                "directiveSha256": "F" * 64,
                "requirementRegistrySha256": "D" * 64,
                "verificationCatalogSha256": "E" * 64,
                "reviewedRequirementMappingSha256": "8" * 64,
                "reviewedMappingRowsSha256": self.reviewed_mapping()["binding"][
                    "reviewedRowsSha256"
                ],
            },
            "summary": {
                "requirementsEnumerated": 2,
                "verificationsEnumerated": count,
                "verificationClassificationCounts": {
                    finalizer.READY: count,
                    "PARTIAL": 0,
                    "MISSING": 0,
                },
                "uncoveredProofClauseCount": 0,
            },
            "verificationAssessments": [
                {
                    "classification": finalizer.READY,
                    "uncoveredProofClauses": [],
                }
                for _ in range(count)
            ],
        }

    def requirements(self) -> dict[str, object]:
        iso_text = "[PES-ISO-TEST] MUST remain exact."
        typ_text = "[PES-TYP-TEST] MUST remain exact."
        return {
            "directive": {"path": "directive.docx", "sha256": "F" * 64},
            "requirements": [
                {
                    "id": "PES-ISO-TEST",
                    "area": "ISO",
                    "exactText": iso_text,
                    "textSha256": hashlib.sha256(iso_text.encode("utf-8"))
                    .hexdigest()
                    .upper(),
                },
                {
                    "id": "PES-TYP-TEST",
                    "area": "TYP",
                    "exactText": typ_text,
                    "textSha256": hashlib.sha256(typ_text.encode("utf-8"))
                    .hexdigest()
                    .upper(),
                },
            ],
        }

    def reviewed_mapping(self) -> dict[str, object]:
        requirements = self.requirements()
        catalog = self.catalog()
        rows = [
            {
                "requirementId": "PES-ISO-TEST",
                "requirementTextSha256": requirements["requirements"][0]["textSha256"],
                "selectedVerificationIds": ["VER-ISO-TEST"],
                "disposition": finalizer.reviewed_requirement_mapping.DISPOSITION,
                "reviewerRationale": "Reviewed exact ISO requirement and selected its direct proof.",
            },
            {
                "requirementId": "PES-TYP-TEST",
                "requirementTextSha256": requirements["requirements"][1]["textSha256"],
                "selectedVerificationIds": ["VER-TYP-TEST"],
                "disposition": finalizer.reviewed_requirement_mapping.DISPOSITION,
                "reviewerRationale": "Reviewed exact TYP requirement and selected its direct proof.",
            },
        ]
        return {
            "schemaVersion": 1,
            "artifactKind": finalizer.reviewed_requirement_mapping.ARTIFACT_KIND,
            "reviewAuthority": {
                "reviewerId": "phase2-finalizer-test",
                "reviewedOn": "2026-08-28",
                "dispositionPolicy": (
                    finalizer.reviewed_requirement_mapping.DISPOSITION_POLICY
                ),
                "rationaleMaxCharacters": (
                    finalizer.reviewed_requirement_mapping.RATIONALE_MAX_CHARACTERS
                ),
            },
            "binding": {
                "directivePath": "directive.docx",
                "directiveSha256": "F" * 64,
                "requirementRegistryPath": (
                    finalizer.reviewed_requirement_mapping.REQUIREMENT_REGISTRY_PATH
                ),
                "requirementRegistrySha256": "D" * 64,
                "verificationCatalogPath": (
                    finalizer.reviewed_requirement_mapping.VERIFICATION_CATALOG_PATH
                ),
                "verificationCatalogSha256": "E" * 64,
                "requirementCount": 2,
                "verificationCount": 2,
                "requirementInventorySha256": (
                    finalizer.reviewed_requirement_mapping.requirement_inventory_sha256(
                        requirements["requirements"]
                    )
                ),
                "verificationInventorySha256": (
                    finalizer.reviewed_requirement_mapping.verification_inventory_sha256(
                        catalog["verificationRecords"]
                    )
                ),
                "reviewedRowsSha256": (
                    finalizer.reviewed_requirement_mapping.reviewed_rows_sha256(rows)
                ),
            },
            "mappingRows": rows,
        }

    def catalog(self) -> dict[str, object]:
        return {
            "verificationRecords": [
                {"verificationId": "VER-ISO-TEST"},
                {"verificationId": "VER-TYP-TEST"},
            ],
            "requirementMappingSkeleton": [
                {
                    "requirementId": "PES-ISO-TEST",
                    "candidateVerificationIds": ["VER-ISO-TEST"],
                },
                {
                    "requirementId": "PES-TYP-TEST",
                    "candidateVerificationIds": ["VER-TYP-TEST"],
                },
            ],
        }

    def record(
        self,
        evidence_id: str,
        *,
        kind: str,
        verifications: list[str],
        journeys: list[str],
        gates: list[str],
        isolation: bool = False,
    ) -> dict[str, object]:
        case_kinds = ["POSITIVE", "NEGATIVE", "INTEGRATION"]
        if isolation:
            case_kinds.append("ISOLATION")
        return {
            "evidenceId": evidence_id,
            "kind": kind,
            "result": "PASS",
            "binding": self.binding(),
            "execution": {},
            "caseKinds": case_kinds,
            "artifacts": [],
            "subjects": {
                "requirements": ["PES-SMUGGLED-TEST"],
                "verifications": verifications,
                "journeys": journeys,
                "gates": gates,
            },
        }

    def test_gapless_audit_rejects_a_partial_row(self) -> None:
        audit = self.audit()
        audit["summary"]["verificationClassificationCounts"] = {  # type: ignore[index]
            finalizer.READY: 1,
            "PARTIAL": 1,
            "MISSING": 0,
        }
        with self.assertRaisesRegex(finalizer.FinalizationError, "evidence-ready"):
            finalizer.require_gapless_static_audit(
                audit,
                self.requirements(),
                self.catalog(),
                self.reviewed_mapping(),
                self.binding(),
            )

    def test_build_ledger_derives_requirement_subjects_and_adds_isolation(self) -> None:
        ordinary = self.record(
            "EVID-ORDINARY",
            kind="TEST",
            verifications=["VER-ISO-TEST", "VER-TYP-TEST"],
            journeys=list(finalizer.verify_phase2.JOURNEY_IDS),
            gates=list(finalizer.verify_phase2.G2_IDS),
        )
        isolation = self.record(
            "EVID-ISOLATION",
            kind="ISOLATION",
            verifications=["VER-ISO-TEST"],
            journeys=["G"],
            gates=["G2-12"],
            isolation=True,
        )
        mutation = self.record(
            "EVID-MUTATION",
            kind="MUTATION",
            verifications=[],
            journeys=["H"],
            gates=["G2-11"],
        )
        execution_index = {
            "schemaVersion": 1,
            "candidateBinding": self.binding(),
            "evidenceRecords": [ordinary, isolation, mutation],
            "defects": [],
        }
        with tempfile.TemporaryDirectory() as directory:
            with (
                patch.object(finalizer.verify_phase2, "validate_evidence_record", return_value=[]),
                patch.object(
                    finalizer.verify_phase2,
                    "validate_status_claim",
                    return_value=({}, []),
                ),
            ):
                ledger = finalizer.build_ledger(
                    commit="1" * 40,
                    candidate_tag="phase2-test",
                    requirements=self.requirements(),
                    catalog=self.catalog(),
                    reviewed_mapping=self.reviewed_mapping(),
                    binding=self.binding(),
                    audit=self.audit(),
                    execution_index=execution_index,
                    evidence_base=Path(directory),
                    candidate_paths=set(),
                )

        by_requirement = {
            record["requirementId"]: record for record in ledger["requirements"]
        }
        self.assertEqual(
            by_requirement["PES-ISO-TEST"]["evidenceIds"],
            ["EVID-ISOLATION", "EVID-ORDINARY"],
        )
        self.assertEqual(
            by_requirement["PES-TYP-TEST"]["evidenceIds"],
            ["EVID-ORDINARY"],
        )
        subjects = {
            record["evidenceId"]: record["subjects"]["requirements"]
            for record in ledger["evidenceRecords"]
        }
        self.assertNotIn("PES-SMUGGLED-TEST", subjects["EVID-ORDINARY"])
        self.assertEqual(subjects["EVID-ISOLATION"], ["PES-ISO-TEST"])


if __name__ == "__main__":
    unittest.main()
