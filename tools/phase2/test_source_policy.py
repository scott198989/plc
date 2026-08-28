from __future__ import annotations

import unittest

import source_policy


class SourcePolicyTests(unittest.TestCase):
    def fixture(self) -> tuple[dict[str, str], tuple[str, ...]]:
        cargo = """[workspace]
members = ["crates/plc-core", "crates/plc-system"]
resolver = "3"
"""
        sources = {
            "Cargo.toml": cargo,
            "Cargo.lock": "version = 4\n",
            "package.json": '{"dependencies": {}}',
            "pnpm-lock.yaml": "lockfileVersion: '9.0'\n",
            "pnpm-workspace.yaml": "packages: []\n",
            "crates/plc-core/Cargo.toml": "[package]\nname='plc-core'\nversion='0.0.0'\n",
            "crates/plc-core/src/lib.rs": "pub fn core() -> bool { true }\n",
            "crates/plc-system/Cargo.toml": "[package]\nname='plc-system'\nversion='0.0.0'\n",
            "crates/plc-system/src/lib.rs": "pub fn system() -> bool { true }\n",
            "apps/foundation-shell/package.json": '{"dependencies": {}}',
            "apps/foundation-shell/src/main.ts": "export const ready = true;\n",
        }
        return sources, source_policy.workspace_crates_from_toml(cargo)

    def test_discovers_and_scans_every_workspace_crate(self) -> None:
        sources, crates = self.fixture()
        result = source_policy.scan_source_map(sources, crates)
        self.assertTrue(result.passed, result.findings)
        self.assertEqual(
            result.workspace_crates,
            ("crates/plc-core", "crates/plc-system"),
        )
        self.assertIn("crates/plc-system/src/lib.rs", result.scanned_files)

    def test_new_crate_cannot_hide_network_capability(self) -> None:
        sources, crates = self.fixture()
        sources["crates/plc-system/src/lib.rs"] = (
            "use std::net::TcpStream;\npub fn connect() {}\n"
        )
        result = source_policy.scan_source_map(sources, crates)
        self.assertFalse(result.passed)
        self.assertTrue(
            any(
                finding.path == "crates/plc-system/src/lib.rs"
                and finding.rule == "host network/process capability"
                for finding in result.findings
            )
        )

    def test_production_dependency_capability_is_rejected(self) -> None:
        sources, crates = self.fixture()
        sources["crates/plc-core/Cargo.toml"] += "[dependencies]\nreqwest='1'\n"
        result = source_policy.scan_source_map(sources, crates)
        self.assertFalse(result.passed)
        self.assertTrue(any(finding.match == "reqwest" for finding in result.findings))

    def test_locked_node_capability_is_rejected_even_if_manifest_hides_it(self) -> None:
        sources, crates = self.fixture()
        sources["pnpm-lock.yaml"] = "packages:\n  ws@9.0.0:\n    resolution: {}\n"
        result = source_policy.scan_source_map(sources, crates)
        self.assertFalse(result.passed)
        self.assertTrue(
            any(
                finding.rule == "forbidden locked Node capability" and finding.match == "ws"
                for finding in result.findings
            )
        )

    def test_workspace_member_outside_crates_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "outside crates"):
            source_policy.workspace_crates_from_toml(
                '[workspace]\nmembers=["../external"]\n'
            )

    def test_unlisted_crate_cannot_evade_scan(self) -> None:
        sources, crates = self.fixture()
        sources["crates/rogue/Cargo.toml"] = "[package]\nname='rogue'\nversion='0.0.0'\n"
        sources["crates/rogue/src/lib.rs"] = "pub fn hidden() {}\n"
        result = source_policy.scan_source_map(sources, crates)
        self.assertFalse(result.passed)
        self.assertTrue(
            any(
                finding.path == "crates/rogue"
                and finding.rule == "crate is not declared in workspace.members"
                for finding in result.findings
            )
        )


if __name__ == "__main__":
    unittest.main()
