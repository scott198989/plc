from __future__ import annotations

import hashlib
import subprocess
import tempfile
import unittest
from pathlib import Path

from verify_phase2 import (
    EXPECTED_REQUIREMENT_COUNT,
    EXPECTED_VERIFICATION_COUNT,
    G2_IDS,
    JOURNEY_IDS,
    accounted_untracked_paths,
    git_blob_sources,
    initial_status_ledger,
    sha256_file,
    validate_evidence_record,
    validate_status_claim,
)


class Phase2GateTests(unittest.TestCase):
    maxDiff = None

    def catalogs(self) -> tuple[dict, dict]:
        verification_ids = [
            f"VER-TST-{index:04d}" for index in range(1, EXPECTED_VERIFICATION_COUNT + 1)
        ]
        requirements = [
            {
                "id": f"PES-TST-{index:04d}",
                "area": "TST",
                "truthState": "NOT_STARTED",
            }
            for index in range(1, EXPECTED_REQUIREMENT_COUNT + 1)
        ]
        registry = {
            "directive": {"path": "directive.docx", "sha256": "D" * 64},
            "requirements": requirements,
        }
        catalog = {
            "verificationRecords": [
                {"verificationId": verification_id} for verification_id in verification_ids
            ],
            "requirementMappingSkeleton": [
                {
                    "requirementId": requirement["id"],
                    "candidateVerificationIds": [verification_ids[0]],
                }
                for requirement in requirements
            ],
        }
        return registry, catalog

    def binding(self) -> dict[str, str]:
        return {
            "candidateCommit": "a" * 40,
            "candidateTree": "b" * 40,
            "productionSourceSha256": "C" * 64,
            "testSourceSha256": "D" * 64,
            "requirementsSourceSha256": "E" * 64,
            "requirementRegistrySha256": "F" * 64,
            "verificationCatalogSha256": "0" * 64,
            "directiveSha256": "1" * 64,
        }

    def evidence(
        self,
        evidence_id: str,
        kind: str,
        subjects: dict[str, list[str]],
        log: Path,
        case_kinds: list[str],
    ) -> dict:
        record = {
            "evidenceId": evidence_id,
            "kind": kind,
            "result": "PASS",
            "binding": self.binding(),
            "subjects": subjects,
            "caseKinds": case_kinds,
            "execution": {
                "command": "run exact production verification",
                "exitCode": 0,
                "attempts": 1,
                "startedAt": "2026-08-28T01:00:00-05:00",
                "finishedAt": "2026-08-28T01:01:00-05:00",
                "skipped": False,
                "flaky": False,
                "crashed": False,
                "unavailable": False,
                "inconclusive": False,
                "canned": False,
                "productionPathExercised": True,
            },
            "artifacts": [
                {
                    "kind": "LOG",
                    "path": log.name,
                    "bytes": log.stat().st_size,
                    "sha256": sha256_file(log),
                }
            ],
        }
        if kind == "MUTATION":
            record.update(
                {
                    "mutationId": "MUT-BYPASS-COMPILER",
                    "expectedDetector": "named-compiler-bypass-check",
                    "actualDetector": "named-compiler-bypass-check",
                    "detectorExitCode": 1,
                }
            )
        if kind == "ISOLATION":
            record.update(
                {
                    "zeroExternalAttempts": True,
                    "instrumentationStatus": "COMPLETE",
                    "platformConfigurations": ["windows-adapters-on", "windows-adapters-off"],
                }
            )
        return record

    def test_initial_ledger_enumerates_every_obligation_without_claiming_completion(self) -> None:
        registry, catalog = self.catalogs()
        ledger = initial_status_ledger(registry, catalog)
        self.assertEqual(len(ledger["requirements"]), 937)
        self.assertEqual(len(ledger["verifications"]), 44)
        self.assertEqual([entry["journeyId"] for entry in ledger["journeys"]], list(JOURNEY_IDS))
        self.assertEqual([entry["gateId"] for entry in ledger["gates"]], list(G2_IDS))
        self.assertTrue(all(entry["status"] == "NOT_STARTED" for entry in ledger["requirements"]))
        self.assertTrue(all(entry["status"] == "NOT_STARTED" for entry in ledger["verifications"]))

    def test_default_ledger_fails_closed_as_incomplete(self) -> None:
        registry, catalog = self.catalogs()
        ledger = initial_status_ledger(registry, catalog)
        with tempfile.TemporaryDirectory() as temporary:
            _summary, failures = validate_status_claim(
                ledger,
                registry,
                catalog,
                self.binding(),
                Path(temporary),
                {"src/implemented.rs"},
            )
        codes = {failure.code for failure in failures}
        self.assertTrue({"P2-COMP-0001", "P2-COMP-0002", "P2-COMP-0003", "P2-COMP-0004"}.issubset(codes))

    def test_stale_flaky_crashed_or_canned_evidence_never_receives_credit(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            log = base / "run.log"
            log.write_text("real output\n", encoding="utf-8")
            record = self.evidence(
                "E-1",
                "TEST",
                {"requirements": ["PES-TST-0001"], "verifications": [], "journeys": [], "gates": []},
                log,
                ["POSITIVE"],
            )
            record["binding"]["candidateTree"] = "f" * 40
            record["execution"]["flaky"] = True
            record["execution"]["crashed"] = True
            record["execution"]["canned"] = True
            failures = validate_evidence_record(record, self.binding(), base)
        codes = [failure.code for failure in failures]
        self.assertIn("P2-EVID-0003", codes)
        self.assertGreaterEqual(codes.count("P2-EVID-0008"), 3)

    def test_complete_synthetic_claim_has_a_reachable_pass_path(self) -> None:
        registry, catalog = self.catalogs()
        ledger = initial_status_ledger(registry, catalog)
        requirement_ids = [entry["requirementId"] for entry in ledger["requirements"]]
        verification_ids = [entry["verificationId"] for entry in ledger["verifications"]]
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            log = base / "run.log"
            log.write_text("complete deterministic verification output\n", encoding="utf-8")
            ordinary = self.evidence(
                "E-ORDINARY",
                "TEST",
                {
                    "requirements": requirement_ids,
                    "verifications": verification_ids,
                    "journeys": list(JOURNEY_IDS),
                    "gates": list(G2_IDS),
                },
                log,
                ["POSITIVE", "NEGATIVE", "INTEGRATION"],
            )
            mutation = self.evidence(
                "E-MUTATION",
                "MUTATION",
                {"requirements": [], "verifications": [], "journeys": ["H"], "gates": ["G2-11"]},
                log,
                ["MUTATION"],
            )
            isolation = self.evidence(
                "E-ISOLATION",
                "ISOLATION",
                {"requirements": [], "verifications": [], "journeys": ["G"], "gates": ["G2-12"]},
                log,
                ["ISOLATION"],
            )
            ledger["evidenceRecords"] = [ordinary, mutation, isolation]
            ledger["candidate"] = {"commit": self.binding()["candidateCommit"], "tag": "phase2-candidate"}
            for entry in ledger["requirements"]:
                entry["status"] = "VERIFIED"
                entry["mappingStatus"] = "REVIEWED"
                entry["evidenceIds"] = ["E-ORDINARY"]
            for entry in ledger["verifications"]:
                entry["status"] = "VERIFIED"
                entry["evidenceIds"] = ["E-ORDINARY"]
            for entry in ledger["journeys"]:
                entry["status"] = "PASS"
                entry["evidenceIds"] = ["E-ORDINARY"]
                if entry["journeyId"] == "H":
                    entry["evidenceIds"].append("E-MUTATION")
                if entry["journeyId"] == "G":
                    entry["evidenceIds"].append("E-ISOLATION")
            for entry in ledger["gates"]:
                entry["status"] = "PASS"
                entry["evidenceIds"] = ["E-ORDINARY"]
                if entry["gateId"] == "G2-11":
                    entry["evidenceIds"].append("E-MUTATION")
                if entry["gateId"] == "G2-12":
                    entry["evidenceIds"].append("E-ISOLATION")
            _summary, failures = validate_status_claim(
                ledger,
                registry,
                catalog,
                self.binding(),
                base,
                {"src/implemented.rs"},
            )
        self.assertEqual(failures, [])

    def test_unrelated_mutation_detector_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            log = base / "run.log"
            log.write_text("mutation output\n", encoding="utf-8")
            record = self.evidence(
                "E-MUTATION",
                "MUTATION",
                {"requirements": [], "verifications": [], "journeys": ["H"], "gates": ["G2-11"]},
                log,
                ["MUTATION"],
            )
            record["actualDetector"] = "unrelated-lint-failure"
            failures = validate_evidence_record(record, self.binding(), base)
        self.assertIn("P2-EVID-0020", {failure.code for failure in failures})

    def test_exact_candidate_blob_scan_does_not_read_later_worktree_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            subprocess.run(["git", "config", "core.autocrlf", "false"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.email", "gate@example.invalid"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "Phase 2 Gate"], cwd=root, check=True)
            source = root / "source.rs"
            source.write_text("pub fn safe() -> bool { true }\n", encoding="utf-8")
            subprocess.run(["git", "add", "source.rs"], cwd=root, check=True)
            subprocess.run(["git", "commit", "-q", "-m", "candidate"], cwd=root, check=True)
            commit = (
                subprocess.run(
                    ["git", "rev-parse", "HEAD"],
                    cwd=root,
                    check=True,
                    capture_output=True,
                    text=True,
                )
                .stdout.strip()
            )
            source.write_text("use std::net::TcpStream;\n", encoding="utf-8")
            candidate = git_blob_sources(root, commit, ["source.rs"])["source.rs"]
        self.assertIn(b"pub fn safe() -> bool { true }", candidate)
        self.assertNotIn(b"std::net", candidate)

    def test_protected_untracked_reference_is_accounted_by_exact_p2_00_hash(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            protected = root / "References for Codex from Scott" / "PHASE_1_ADVERSARIAL_AUDIT.docx"
            protected.parent.mkdir(parents=True)
            protected.write_bytes(b"protected-user-reference")
            digest = hashlib.sha256(protected.read_bytes()).hexdigest().upper()
            gate = {
                "accountedWorkspaceState": [
                    {
                        "path": "References for Codex from Scott/PHASE_1_ADVERSARIAL_AUDIT.docx",
                        "sha256": digest,
                    }
                ]
            }
            accounted, failures = accounted_untracked_paths(root, gate)
        self.assertEqual(failures, [])
        self.assertEqual(
            accounted,
            {"References for Codex from Scott/PHASE_1_ADVERSARIAL_AUDIT.docx"},
        )

    def test_clean_checkout_does_not_require_accounted_user_reference(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            gate = {
                "accountedWorkspaceState": [
                    {
                        "path": "References for Codex from Scott/PHASE_1_ADVERSARIAL_AUDIT.docx",
                        "sha256": "A" * 64,
                    }
                ]
            }
            accounted, failures = accounted_untracked_paths(Path(temporary), gate)
        self.assertEqual(accounted, set())
        self.assertEqual(failures, [])


if __name__ == "__main__":
    unittest.main()
