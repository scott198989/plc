#!/usr/bin/env python3
"""Independent adversarial tests for the Phase 2 governance audit."""

from __future__ import annotations

import copy
import unittest
from dataclasses import replace
from pathlib import Path

import extract_phase2_requirements as extractor
import governance_audit as governance


ROOT = Path(__file__).resolve().parents[2]
DIRECTIVE = ROOT / extractor.DEFAULT_DIRECTIVE_PATH
PHASE1_REGISTRY = ROOT / extractor.DEFAULT_PHASE1_REGISTRY_PATH
P2_00_EVIDENCE = ROOT / extractor.DEFAULT_P2_00_EVIDENCE_PATH


class GovernanceAuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.blocks = extractor.parse_docx(DIRECTIVE)
        cls.outputs = extractor.build_outputs(
            root=ROOT,
            directive_path=DIRECTIVE,
            phase1_registry_path=PHASE1_REGISTRY,
            p2_00_evidence_path=P2_00_EVIDENCE,
        )
        cls.known_ids = governance._load_phase1_ids(PHASE1_REGISTRY)
        cls.known_ids.update(
            record["id"]
            for record in cls.outputs["requirements"]["requirements"]
        )

    def audit(self, *, blocks=None, outputs=None, source_sha256=None):
        return governance.audit_contract(
            blocks=self.blocks if blocks is None else blocks,
            outputs=self.outputs if outputs is None else outputs,
            source_path=extractor.DEFAULT_DIRECTIVE_PATH,
            source_sha256=(
                governance.EXPECTED_DIRECTIVE_SHA256
                if source_sha256 is None
                else source_sha256
            ),
            known_requirement_ids=self.known_ids,
        )

    @staticmethod
    def codes(report) -> set[str]:
        return {finding["code"] for finding in report["findings"]}

    @staticmethod
    def replace_table_rows(blocks, heading, mutate):
        result = list(blocks)
        matches = [
            index
            for index, block in enumerate(result)
            if isinstance(block, extractor.TableBlock)
            and block.heading_path == heading
        ]
        if len(matches) != 1:
            raise AssertionError(f"fixture expected one table, found {len(matches)}")
        index = matches[0]
        table = result[index]
        result[index] = replace(table, rows=tuple(mutate(list(table.rows))))
        return result

    def test_canonical_audit_is_complete_but_grants_no_credit(self) -> None:
        report = governance.build_audit(
            root=ROOT,
            directive_path=DIRECTIVE,
            phase1_registry_path=PHASE1_REGISTRY,
            p2_00_evidence_path=P2_00_EVIDENCE,
        )
        self.assertEqual(report["auditStatus"], "PASS")
        self.assertEqual(report["findings"], [])
        self.assertEqual(
            report["counts"],
            {
                "findingCount": 0,
                "clarificationCount": 14,
                "resolvedOpenQuestionCount": 4,
                "openDecisionCount": 7,
                "exclusionParagraphCount": 24,
            },
        )
        self.assertFalse(report["truthPolicy"]["verificationCreditGranted"])
        self.assertFalse(report["truthPolicy"]["phase2ExitVerdictIssued"])
        self.assertFalse(report["truthPolicy"]["scottAcceptanceInferred"])
        self.assertEqual(
            report["inventory"]["terminalVerdict"]["allowedExitVerdicts"],
            list(governance.ALLOWED_EXIT_VERDICTS),
        )
        self.assertEqual(
            report["reportedNotices"][1]["requirementIds"],
            ["PES-GOV-0032"],
        )

    def test_source_authority_fails_closed_on_hash_drift(self) -> None:
        report = self.audit(source_sha256="0" * 64)
        self.assertEqual(report["auditStatus"], "BLOCKED")
        self.assertIn("GOV-SOURCE-HASH", self.codes(report))

    def test_normative_table_omission_duplicate_and_drift_are_detected(self) -> None:
        table = next(
            block
            for block in self.blocks
            if isinstance(block, extractor.TableBlock)
            and block.heading_path == governance.NORMATIVE_HEADING
        )
        omitted = [block for block in self.blocks if block is not table]
        duplicated = [*self.blocks, table]

        def drift(rows):
            row = list(rows[4])
            row[1] = "Optional even outside approved scope."
            rows[4] = tuple(row)
            return rows

        drifted = self.replace_table_rows(
            self.blocks, governance.NORMATIVE_HEADING, drift
        )
        self.assertIn("GOV-NORMATIVE-TABLE-COUNT", self.codes(self.audit(blocks=omitted)))
        self.assertIn("GOV-NORMATIVE-TABLE-COUNT", self.codes(self.audit(blocks=duplicated)))
        self.assertIn("GOV-NORMATIVE-ROW-DRIFT", self.codes(self.audit(blocks=drifted)))

    def test_requirement_keyword_omission_and_metadata_drift_are_detected(self) -> None:
        outputs = copy.deepcopy(self.outputs)
        record = outputs["requirements"]["requirements"][0]
        record["normativeKeyword"] = "RECOMMENDED"
        report = self.audit(outputs=outputs)
        codes = self.codes(report)
        self.assertIn("GOV-NORMATIVE-UNDEFINED", codes)
        self.assertIn("GOV-NORMATIVE-CLASSIFICATION", codes)
        self.assertIn("GOV-NORMATIVE-DISTRIBUTION", codes)

    def test_clarification_ledger_omission_duplicate_drift_and_unknown_ref(self) -> None:
        def omit(rows):
            return rows[:-1]

        def duplicate(rows):
            rows.append(rows[1])
            return rows

        def drift(rows):
            row = list(rows[1])
            row[2] += " silently changed"
            rows[1] = tuple(row)
            return rows

        def unknown_ref(rows):
            row = list(rows[1])
            row[1] = "PES-GOV-9999"
            rows[1] = tuple(row)
            return rows

        omitted = self.replace_table_rows(
            self.blocks, governance.CLARIFICATION_HEADING, omit
        )
        duplicated = self.replace_table_rows(
            self.blocks, governance.CLARIFICATION_HEADING, duplicate
        )
        drifted = self.replace_table_rows(
            self.blocks, governance.CLARIFICATION_HEADING, drift
        )
        unknown = self.replace_table_rows(
            self.blocks, governance.CLARIFICATION_HEADING, unknown_ref
        )
        self.assertIn("GOV-CLARIFICATION-ROW-KEYS", self.codes(self.audit(blocks=omitted)))
        self.assertIn(
            "GOV-CLARIFICATION-DUPLICATE-KEY",
            self.codes(self.audit(blocks=duplicated)),
        )
        self.assertIn("GOV-CLARIFICATION-ROW-DRIFT", self.codes(self.audit(blocks=drifted)))
        self.assertIn(
            "GOV-CLARIFICATION-UNKNOWN-REFERENCE",
            self.codes(self.audit(blocks=unknown)),
        )

    def test_phase_reservation_and_exclusion_omissions_are_detected(self) -> None:
        reservation = next(
            block
            for block in self.blocks
            if isinstance(block, extractor.ParagraphBlock)
            and block.heading_path == governance.PHASE_RESERVATION_HEADING
            and block.exact_text.strip()
            and block.exact_text != governance.PHASE_RESERVATION_HEADING[-1]
        )
        exclusion = next(
            block
            for block in self.blocks
            if isinstance(block, extractor.ParagraphBlock)
            and block.heading_path == governance.EXCLUSION_HEADING
            and block.exact_text == "The following are EXCLUDED, not backlog items:"
        )
        report = self.audit(
            blocks=[
                block
                for block in self.blocks
                if block is not reservation and block is not exclusion
            ]
        )
        codes = self.codes(report)
        self.assertIn("GOV-PHASE-RESERVATION-COUNT", codes)
        self.assertIn("GOV-EXCLUSION-COUNT", codes)

    def test_decision_template_and_stop_requirement_drift_are_detected(self) -> None:
        decision = next(
            block
            for block in self.blocks
            if isinstance(block, extractor.ParagraphBlock)
            and block.heading_path == governance.DECISION_TEMPLATE_HEADING
            and block.exact_text.startswith("Decision ID:")
        )
        blocks = [block for block in self.blocks if block is not decision]
        outputs = copy.deepcopy(self.outputs)
        stop = next(
            record
            for record in outputs["requirements"]["requirements"]
            if record["id"] == "PES-GOV-0038"
        )
        stop["textSha256"] = "0" * 64
        report = self.audit(blocks=blocks, outputs=outputs)
        codes = self.codes(report)
        self.assertIn("GOV-DECISION-TEMPLATE-DRIFT", codes)
        self.assertIn("GOV-STOP-RULE-DRIFT", codes)

    def test_terminal_verdict_omission_duplicate_and_drift_are_detected(self) -> None:
        def omit(rows):
            return [row for row in rows if not row or row[0] != "Final verdict"]

        def duplicate(rows):
            final = next(row for row in rows if row and row[0] == "Final verdict")
            rows.append(final)
            return rows

        def drift(rows):
            index = next(
                index
                for index, row in enumerate(rows)
                if row and row[0] == "Final verdict"
            )
            rows[index] = ("Final verdict", "PASS")
            return rows

        omitted = self.replace_table_rows(self.blocks, governance.EXIT_HEADING, omit)
        duplicated = self.replace_table_rows(
            self.blocks, governance.EXIT_HEADING, duplicate
        )
        drifted = self.replace_table_rows(self.blocks, governance.EXIT_HEADING, drift)
        self.assertIn("GOV-EXIT-ROW-KEYS", self.codes(self.audit(blocks=omitted)))
        self.assertIn(
            "GOV-EXIT-DUPLICATE-KEY", self.codes(self.audit(blocks=duplicated))
        )
        drift_codes = self.codes(self.audit(blocks=drifted))
        self.assertIn("GOV-EXIT-ROW-DRIFT", drift_codes)
        self.assertIn("GOV-EXIT-VERDICT-LANGUAGE", drift_codes)

    def test_registry_array_omission_duplicate_and_truth_promotion_are_detected(self) -> None:
        outputs = copy.deepcopy(self.outputs)
        requirements = outputs["requirements"]["requirements"]
        requirements.append(copy.deepcopy(requirements[0]))
        verifications = outputs["verification"]["verificationRecords"]
        verifications.pop()
        verifications[0]["truthState"] = "VERIFIED"
        report = self.audit(outputs=outputs)
        codes = self.codes(report)
        self.assertIn("GOV-EXTRACTION-ARRAY-COUNTS", codes)
        self.assertIn("GOV-REQUIREMENT-DUPLICATE", codes)
        self.assertIn("GOV-VERIFICATION-TRUTH-DRIFT", codes)
        self.assertFalse(report["truthPolicy"]["verificationCreditGranted"])

    def test_reused_id_inventory_is_reported_and_drift_is_blocked(self) -> None:
        outputs = copy.deepcopy(self.outputs)
        outputs["audit"]["findings"]["reusedRetiredPhase1Ids"] = []
        report = self.audit(outputs=outputs)
        self.assertIn("GOV-REUSED-ID-INVENTORY-DRIFT", self.codes(report))
        self.assertFalse(report["truthPolicy"]["verificationCreditGranted"])


if __name__ == "__main__":
    unittest.main()
