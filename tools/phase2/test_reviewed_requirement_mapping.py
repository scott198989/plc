from __future__ import annotations

import copy
import hashlib
import unittest

import reviewed_requirement_mapping as reviewed


class ReviewedRequirementMappingTests(unittest.TestCase):
    def fixture(self) -> tuple[dict, dict, dict, dict[str, str]]:
        first_text = "[PES-TST-0001] MUST remain exact."
        second_text = "[PES-TST-0002] MUST remain exact."
        hashes = {
            "directive": "D" * 64,
            "registry": "E" * 64,
            "catalog": "F" * 64,
        }
        registry = {
            "directive": {
                "path": "References for Codex from Scott/phase2.docx",
                "sha256": hashes["directive"],
            },
            "requirements": [
                {
                    "id": "PES-TST-0001",
                    "area": "TST",
                    "exactText": first_text,
                    "textSha256": hashlib.sha256(first_text.encode("utf-8"))
                    .hexdigest()
                    .upper(),
                },
                {
                    "id": "PES-TST-0002",
                    "area": "TST",
                    "exactText": second_text,
                    "textSha256": hashlib.sha256(second_text.encode("utf-8"))
                    .hexdigest()
                    .upper(),
                },
            ],
        }
        catalog = {
            "verificationRecords": [
                {"verificationId": "VER-TST-0001"},
                {"verificationId": "VER-TST-0002"},
            ],
            "requirementMappingSkeleton": [
                {
                    "requirementId": "PES-TST-0001",
                    "candidateVerificationIds": ["VER-TST-0001", "VER-TST-0002"],
                },
                {
                    "requirementId": "PES-TST-0002",
                    "candidateVerificationIds": ["VER-TST-0002"],
                },
            ],
        }
        rows = [
            {
                "requirementId": "PES-TST-0001",
                "requirementTextSha256": registry["requirements"][0]["textSha256"],
                "selectedVerificationIds": ["VER-TST-0001"],
                "disposition": reviewed.DISPOSITION,
                "reviewerRationale": (
                    "Reviewed the exact requirement text and selected its direct Appendix-H proof."
                ),
            },
            {
                "requirementId": "PES-TST-0002",
                "requirementTextSha256": registry["requirements"][1]["textSha256"],
                "selectedVerificationIds": ["VER-TST-0002"],
                "disposition": reviewed.DISPOSITION,
                "reviewerRationale": (
                    "Reviewed the exact requirement text and selected its direct Appendix-H proof."
                ),
            },
        ]
        artifact = {
            "schemaVersion": 1,
            "artifactKind": reviewed.ARTIFACT_KIND,
            "reviewAuthority": {
                "reviewerId": "phase2-test-reviewer",
                "reviewedOn": "2026-08-28",
                "dispositionPolicy": reviewed.DISPOSITION_POLICY,
                "rationaleMaxCharacters": reviewed.RATIONALE_MAX_CHARACTERS,
            },
            "binding": {
                "directivePath": registry["directive"]["path"],
                "directiveSha256": hashes["directive"],
                "requirementRegistryPath": reviewed.REQUIREMENT_REGISTRY_PATH,
                "requirementRegistrySha256": hashes["registry"],
                "verificationCatalogPath": reviewed.VERIFICATION_CATALOG_PATH,
                "verificationCatalogSha256": hashes["catalog"],
                "requirementCount": 2,
                "verificationCount": 2,
                "requirementInventorySha256": reviewed.requirement_inventory_sha256(
                    registry["requirements"]
                ),
                "verificationInventorySha256": reviewed.verification_inventory_sha256(
                    catalog["verificationRecords"]
                ),
                "reviewedRowsSha256": reviewed.reviewed_rows_sha256(rows),
            },
            "mappingRows": rows,
        }
        return registry, catalog, artifact, hashes

    def validate(self, registry: dict, catalog: dict, artifact: dict, hashes: dict) -> dict:
        return reviewed.validate_reviewed_mapping(
            artifact,
            registry,
            catalog,
            requirement_registry_sha256=hashes["registry"],
            verification_catalog_sha256=hashes["catalog"],
            directive_sha256=hashes["directive"],
            expected_requirement_count=2,
            expected_verification_count=2,
        )

    def test_valid_review_is_exact_and_source_bound(self) -> None:
        registry, catalog, artifact, hashes = self.fixture()
        rows = self.validate(registry, catalog, artifact, hashes)
        self.assertEqual(set(rows), {"PES-TST-0001", "PES-TST-0002"})
        self.assertEqual(rows["PES-TST-0001"]["selectedVerificationIds"], ["VER-TST-0001"])

    def test_missing_extra_duplicate_drift_empty_and_noncandidate_rows_fail_closed(self) -> None:
        mutations = {
            "missing": lambda artifact: artifact["mappingRows"].pop(),
            "extra": lambda artifact: artifact["mappingRows"].__setitem__(
                1,
                {
                    **artifact["mappingRows"][1],
                    "requirementId": "PES-TST-EXTRA",
                },
            ),
            "duplicate": lambda artifact: artifact["mappingRows"].__setitem__(
                1, copy.deepcopy(artifact["mappingRows"][0])
            ),
            "text drift": lambda artifact: artifact["mappingRows"][0].__setitem__(
                "requirementTextSha256", "0" * 64
            ),
            "empty": lambda artifact: artifact["mappingRows"][0].__setitem__(
                "selectedVerificationIds", []
            ),
            "noncandidate": lambda artifact: artifact["mappingRows"][1].__setitem__(
                "selectedVerificationIds", ["VER-TST-0001"]
            ),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                registry, catalog, artifact, hashes = self.fixture()
                mutate(artifact)
                with self.assertRaises(reviewed.ReviewedMappingError):
                    self.validate(registry, catalog, artifact, hashes)

    def test_source_hash_inventory_row_hash_and_unknown_fields_fail_closed(self) -> None:
        mutations = {
            "registry hash": lambda artifact: artifact["binding"].__setitem__(
                "requirementRegistrySha256", "0" * 64
            ),
            "inventory hash": lambda artifact: artifact["binding"].__setitem__(
                "requirementInventorySha256", "0" * 64
            ),
            "row hash": lambda artifact: artifact["binding"].__setitem__(
                "reviewedRowsSha256", "0" * 64
            ),
            "unknown field": lambda artifact: artifact["mappingRows"][0].__setitem__(
                "unreviewed", False
            ),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                registry, catalog, artifact, hashes = self.fixture()
                mutate(artifact)
                with self.assertRaises(reviewed.ReviewedMappingError):
                    self.validate(registry, catalog, artifact, hashes)

    def test_extracted_exact_text_hash_drift_fails_closed(self) -> None:
        registry, catalog, artifact, hashes = self.fixture()
        registry["requirements"][0]["exactText"] += " drift"
        with self.assertRaisesRegex(reviewed.ReviewedMappingError, "exact text hash is stale"):
            self.validate(registry, catalog, artifact, hashes)


if __name__ == "__main__":
    unittest.main()
