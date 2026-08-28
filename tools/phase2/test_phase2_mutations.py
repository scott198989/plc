from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import run_phase2_mutations as mutations


class Phase2MutationHarnessTests(unittest.TestCase):
    def test_journey_h_inventory_is_exact_and_unique(self) -> None:
        self.assertEqual(len(mutations.MUTATIONS), mutations.EXPECTED_MUTATION_COUNT)
        self.assertEqual(
            [mutation.mutation_id for mutation in mutations.MUTATIONS],
            [f"P2-MUT-{index:02d}" for index in range(1, 9)],
        )
        self.assertEqual(
            len({mutation.name for mutation in mutations.MUTATIONS}),
            mutations.EXPECTED_MUTATION_COUNT,
        )
        self.assertTrue(all(mutation.detector.name for mutation in mutations.MUTATIONS))

    def test_compiler_bypass_mutation_changes_only_reviewed_anchor(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = root / "crates/plc-compiler/src/build.rs"
            path.parent.mkdir(parents=True)
            path.write_text(
                "    fn has_blocking(&self) -> bool {\n"
                "        self.diagnostics.iter().any(BuildDiagnostic::is_blocking)\n"
                "    }\n",
                encoding="utf-8",
            )
            mutations.bypass_compiler(root)
            self.assertIn("        false\n", path.read_text(encoding="utf-8"))

    def test_per_language_runtime_mutation_is_a_new_production_path(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            mutations.add_per_language_runtime(root)
            path = root / "crates/plc-runtime/src/scl_runtime.rs"
            self.assertEqual(path.read_text(encoding="utf-8"), "pub struct SclRuntime;\n")
            with self.assertRaises(mutations.MutationToolError):
                mutations.add_per_language_runtime(root)

    def test_output_path_cannot_escape_verification_root(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            output, transcript = mutations.output_paths(
                root, Path(".phase2-verification/mutations/results.json")
            )
            self.assertEqual(
                output, root / ".phase2-verification/mutations/results.json"
            )
            self.assertEqual(
                transcript,
                root / ".phase2-verification/mutations/mutation-transcript.log",
            )
            with self.assertRaises(mutations.MutationToolError):
                mutations.output_paths(root, Path("evidence/results.json"))
            with self.assertRaises(mutations.MutationToolError):
                mutations.output_paths(root, Path("../escape.json"))


if __name__ == "__main__":
    unittest.main()
