from __future__ import annotations

import tempfile
import unittest
from json import dumps
from pathlib import Path
from unittest import mock

import collect_phase2_evidence as collector
import verify_phase2


class Phase2EvidenceCollectorTests(unittest.TestCase):
    @staticmethod
    def manifest_proof(digest: str) -> dict[str, object]:
        return {
            "fixedNativeBackingAttestation": {"evidenceManifestSha256": digest},
            "liveLanTopologyVariation": {
                "scenarios": [
                    {"evidenceManifestSha256": digest, "scenarioId": "lan-a"},
                    {"evidenceManifestSha256": digest, "scenarioId": "lan-b"},
                ]
            },
            "platformConfigurations": [
                {
                    "configurationId": "windows-x64-chromium-native-broker-adapters-on",
                    "evidenceManifestSha256": digest,
                },
                {
                    "configurationId": "windows-x64-chromium-packaged-adapters-off",
                    "evidenceManifestSha256": digest,
                },
            ],
        }

    @staticmethod
    def write_manifest(directory: Path, *, include_log: bool) -> tuple[Path, str]:
        directory.mkdir(parents=True)
        log = directory / "native-run.log"
        if include_log:
            log.write_text("complete native run\n", encoding="utf-8")
        manifest = directory / "native-platform-evidence-manifest.json"
        value = {
            "evidenceFiles": [
                {
                    "bytes": log.stat().st_size if include_log else 20,
                    "path": log.name,
                    "sha256": collector.sha256_file(log) if include_log else "A" * 64,
                }
            ],
            "evidenceKind": "WINDOWS_NATIVE_PRODUCT_PATH_MANIFEST",
            "schemaVersion": "1.0",
        }
        manifest.write_text(
            dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        return manifest, collector.sha256_file(manifest)

    @staticmethod
    def binding() -> dict[str, object]:
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
            "workspaceCrates": ["plc-core"],
        }

    @staticmethod
    def execution() -> collector.Execution:
        return collector.Execution(
            command="pnpm complete-suite",
            exit_code=0,
            started_at="2026-08-28T00:00:00Z",
            finished_at="2026-08-28T00:01:00Z",
        )

    def test_core_record_covers_exactly_the_non_isolation_catalog(self) -> None:
        catalog = {
            "verificationRecords": [
                {"verificationId": f"VER-CORE-{ordinal:04d}"}
                for ordinal in range(
                    1,
                    verify_phase2.EXPECTED_VERIFICATION_COUNT
                    - len(collector.ISOLATION_VERIFICATIONS)
                    + 1,
                )
            ]
            + [
                {"verificationId": verification}
                for verification in collector.ISOLATION_VERIFICATIONS
            ]
        }
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            log = output / "logs" / "core.log"
            log.parent.mkdir()
            log.write_text("real suite transcript\n", encoding="utf-8")
            record = collector.core_record(
                binding=self.binding(),
                catalog=catalog,
                execution=self.execution(),
                output=output,
                log_path=log,
            )
        observed = set(record["subjects"]["verifications"])
        self.assertEqual(len(observed), 39)
        self.assertTrue(observed.isdisjoint(collector.ISOLATION_VERIFICATIONS))
        self.assertEqual(record["subjects"]["journeys"], list("ABCDEF"))
        self.assertNotIn("G2-11", record["subjects"]["gates"])
        self.assertNotIn("G2-12", record["subjects"]["gates"])

    def test_mutation_records_require_every_intended_named_detector(self) -> None:
        mutations = []
        for ordinal in range(1, collector.EXPECTED_MUTATIONS + 1):
            mutations.append(
                {
                    "actualDetector": f"detector-{ordinal}",
                    "baselineExitCode": 0,
                    "crashed": False,
                    "detected": True,
                    "detectorExitCode": 1,
                    "expectedDetector": f"detector-{ordinal}",
                    "mutationApplied": True,
                    "mutationId": f"P2-MUT-{ordinal:02d}",
                    "unrelatedFailure": False,
                }
            )
        report = {
            "crashCredit": False,
            "intendedMutationDetections": collector.EXPECTED_MUTATIONS,
            "mutations": mutations,
            "overallPassed": True,
            "prescribedMutationCount": collector.EXPECTED_MUTATIONS,
            "schemaVersion": collector.MUTATION_SCHEMA,
            "scratchRemoved": True,
            "unrelatedFailureCredit": False,
        }
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            log = output / "mutation.log"
            log.write_text("mutation transcript\n", encoding="utf-8")
            report_path = output / "mutation-results.json"
            report_path.write_text("{}\n", encoding="utf-8")
            transcript_path = output / "mutation-transcript.log"
            transcript_path.write_text("detector transcript\n", encoding="utf-8")
            records = collector.build_mutation_records(
                report,
                self.execution(),
                output,
                log,
                report_path=report_path,
                transcript_path=transcript_path,
            )
            collector.bind_and_validate_mutations(records, self.binding(), output)
            mutations[4]["actualDetector"] = "unrelated"
            with self.assertRaisesRegex(collector.CollectionError, "intended-detector"):
                collector.build_mutation_records(
                    report,
                    self.execution(),
                    output,
                    log,
                    report_path=report_path,
                    transcript_path=transcript_path,
                )
        self.assertEqual(len(records), collector.EXPECTED_MUTATIONS)
        self.assertTrue(all(record["subjects"]["journeys"] == ["H"] for record in records))
        self.assertTrue(
            all(
                {artifact["kind"] for artifact in record["artifacts"]}
                == {"LOG", "REPORT"}
                for record in records
            )
        )
        self.assertTrue(all(len(record["artifacts"]) == 3 for record in records))

    def test_output_directory_cannot_escape_verification_root(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            accepted = collector.resolve_inside_verification_root(root, None, "a" * 40)
            self.assertEqual(accepted.parent, root / ".phase2-verification")
            with self.assertRaisesRegex(collector.CollectionError, "must remain"):
                collector.resolve_inside_verification_root(
                    root,
                    root / "outside",
                    "a" * 40,
                )

    def test_manifest_materialization_copies_and_hashes_an_intentionally_shared_bundle(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            verification = base / "verification"
            manifest, digest = self.write_manifest(verification / "native-e2e", include_log=True)
            output = base / "output"
            copied = output / "strict-run"
            copied.mkdir(parents=True)

            files = collector.materialize_referenced_manifests(
                verification,
                copied,
                self.manifest_proof(digest),
                output,
            )

        observed = {path.name for path in files}
        self.assertIn(manifest.name, observed)
        self.assertIn("native-run.log", observed)

    def test_manifest_materialization_rejects_an_invented_digest(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            verification = base / "verification"
            verification.mkdir()
            output = base / "output"
            copied = output / "strict-run"
            copied.mkdir(parents=True)
            with self.assertRaisesRegex(collector.CollectionError, "referenced evidence manifest"):
                collector.materialize_referenced_manifests(
                    verification,
                    copied,
                    self.manifest_proof("A" * 64),
                    output,
                )

    def test_manifest_materialization_rejects_an_omitted_nested_log(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            verification = base / "verification"
            _manifest, digest = self.write_manifest(verification / "native-e2e", include_log=False)
            output = base / "output"
            copied = output / "strict-run"
            copied.mkdir(parents=True)
            with self.assertRaisesRegex(collector.CollectionError, "omits listed file"):
                collector.materialize_referenced_manifests(
                    verification,
                    copied,
                    self.manifest_proof(digest),
                    output,
                )

    def test_manifest_materialization_rejects_an_omitted_nested_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            verification = base / "verification"
            source = verification / "native-e2e"
            source.mkdir(parents=True)
            manifest = source / "native-platform-evidence-manifest.json"
            manifest.write_text(
                dumps(
                    {
                        "evidenceFiles": [
                            {
                                "bytes": 100,
                                "path": "nested-evidence-manifest.json",
                                "sha256": "B" * 64,
                            }
                        ],
                        "evidenceKind": "WINDOWS_NATIVE_PRODUCT_PATH_MANIFEST",
                        "schemaVersion": "1.0",
                    },
                    ensure_ascii=False,
                    indent=2,
                    sort_keys=True,
                )
                + "\n",
                encoding="utf-8",
            )
            digest = collector.sha256_file(manifest)
            output = base / "output"
            copied = output / "strict-run"
            copied.mkdir(parents=True)
            with self.assertRaisesRegex(collector.CollectionError, "omits listed file"):
                collector.materialize_referenced_manifests(
                    verification,
                    copied,
                    self.manifest_proof(digest),
                    output,
                )

    def test_manifest_materialization_rejects_duplicate_relative_paths(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            source = base / "verification" / "native-e2e"
            source.mkdir(parents=True)
            log = source / "native-run.log"
            log.write_text("complete native run\n", encoding="utf-8")
            row = {
                "bytes": log.stat().st_size,
                "path": log.name,
                "sha256": collector.sha256_file(log),
            }
            manifest = source / "native-platform-evidence-manifest.json"
            manifest.write_text(
                dumps(
                    {
                        "evidenceFiles": [row, dict(row)],
                        "evidenceKind": "WINDOWS_NATIVE_PRODUCT_PATH_MANIFEST",
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n",
                encoding="utf-8",
            )
            digest = collector.sha256_file(manifest)
            output = base / "output"
            copied = output / "strict-run"
            copied.mkdir(parents=True)
            with self.assertRaisesRegex(collector.CollectionError, "duplicate path"):
                collector.materialize_referenced_manifests(
                    base / "verification",
                    copied,
                    self.manifest_proof(digest),
                    output,
                )

    def test_manifest_materialization_rejects_relative_path_escape(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            verification = base / "verification"
            source = verification / "native-e2e"
            source.mkdir(parents=True)
            outside = verification / "outside.log"
            outside.write_text("must not be reached\n", encoding="utf-8")
            manifest = source / "native-platform-evidence-manifest.json"
            manifest.write_text(
                dumps(
                    {
                        "evidenceFiles": [
                            {
                                "bytes": outside.stat().st_size,
                                "path": "../outside.log",
                                "sha256": collector.sha256_file(outside),
                            }
                        ],
                        "evidenceKind": "WINDOWS_NATIVE_PRODUCT_PATH_MANIFEST",
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n",
                encoding="utf-8",
            )
            digest = collector.sha256_file(manifest)
            output = base / "output"
            copied = output / "strict-run"
            copied.mkdir(parents=True)
            with self.assertRaisesRegex(collector.CollectionError, "escaping relative path"):
                collector.materialize_referenced_manifests(
                    verification,
                    copied,
                    self.manifest_proof(digest),
                    output,
                )

    def test_manifest_bundle_rejects_recursive_cycle(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary)
            first = source / "a-manifest.json"
            second = source / "b-manifest.json"

            def write_cycle() -> None:
                first.write_text(
                    dumps(
                        {
                            "evidenceFiles": [
                                {
                                    "bytes": second.stat().st_size if second.exists() else 1,
                                    "path": second.name,
                                    "sha256": "B" * 64,
                                }
                            ],
                            "evidenceKind": "TEST_A_MANIFEST",
                        },
                        indent=2,
                        sort_keys=True,
                    )
                    + "\n",
                    encoding="utf-8",
                )
                second.write_text(
                    dumps(
                        {
                            "evidenceFiles": [
                                {
                                    "bytes": first.stat().st_size,
                                    "path": first.name,
                                    "sha256": "A" * 64,
                                }
                            ],
                            "evidenceKind": "TEST_B_MANIFEST",
                        },
                        indent=2,
                        sort_keys=True,
                    )
                    + "\n",
                    encoding="utf-8",
                )

            for _ in range(8):
                before = (first.stat().st_size if first.exists() else 0, second.stat().st_size if second.exists() else 0)
                write_cycle()
                after = (first.stat().st_size, second.stat().st_size)
                if before == after:
                    break
            self.assertEqual(
                (first.stat().st_size, second.stat().st_size),
                (second.stat().st_size, first.stat().st_size),
            )

            def content_digest(path: Path) -> str:
                return "A" * 64 if path.name == first.name else "B" * 64

            with mock.patch.object(collector, "sha256_file", side_effect=content_digest):
                with self.assertRaisesRegex(collector.CollectionError, "cycle detected"):
                    collector.collect_manifest_bundle(first)

    def test_bounded_tree_rejects_symlink_or_reparse_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            evidence = root / "linked.log"
            evidence.write_text("not admissible through a reparse boundary\n", encoding="utf-8")
            original = collector._is_reparse_or_symlink

            def classify(path: Path, metadata=None) -> bool:
                return path == evidence or original(path, metadata)

            with mock.patch.object(collector, "_is_reparse_or_symlink", side_effect=classify):
                with self.assertRaisesRegex(collector.CollectionError, "symlink or reparse"):
                    collector.bounded_regular_tree(root)


if __name__ == "__main__":
    unittest.main()
