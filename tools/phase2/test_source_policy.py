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

    def test_per_language_runtime_path_and_identifier_are_rejected(self) -> None:
        sources, _ = self.fixture()
        cargo = sources["Cargo.toml"].replace(
            '"crates/plc-system"', '"crates/plc-system", "crates/plc-runtime"'
        )
        sources["Cargo.toml"] = cargo
        sources["crates/plc-runtime/Cargo.toml"] = (
            "[package]\nname='plc-runtime'\nversion='0.0.0'\n"
        )
        sources["crates/plc-runtime/src/lib.rs"] = "pub struct SclRuntime;\n"
        sources["crates/plc-runtime/src/lad_runtime.rs"] = "pub fn execute() {}\n"
        crates = source_policy.workspace_crates_from_toml(cargo)
        result = source_policy.scan_source_map(sources, crates)
        self.assertFalse(result.passed)
        self.assertTrue(
            any(
                finding.rule == "per-language production runtime identifier"
                and finding.match == "SclRuntime"
                for finding in result.findings
            )
        )
        self.assertTrue(
            any(
                finding.rule == "per-language production runtime path"
                and finding.path.endswith("lad_runtime.rs")
                for finding in result.findings
            )
        )

    def test_native_broker_abi_exception_is_exact_and_dependency_free(self) -> None:
        sources, _ = self.fixture()
        cargo = sources["Cargo.toml"].replace(
            '"crates/plc-system"',
            '"crates/plc-system", "crates/windows-project-broker"',
        )
        sources["Cargo.toml"] = cargo
        sources["crates/windows-project-broker/Cargo.toml"] = (
            "[package]\nname='windows-project-broker'\nversion='0.0.0'\n"
            "[dependencies]\n"
        )
        sources["crates/windows-project-broker/src/lib.rs"] = "pub mod protocol;\n"
        sources["crates/windows-project-broker/src/main.rs"] = "fn main() {}\n"
        sources["crates/windows-project-broker/src/protocol.rs"] = (
            "pub const VERSION: u8 = 1;\n"
        )
        sources["crates/windows-project-broker/src/sha256.rs"] = (
            "pub fn sha256(_: &[u8]) -> [u8; 32] { [0; 32] }\n"
        )
        sources["crates/windows-project-broker/src/windows.rs"] = '''
use std::ffi::c_void;
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetDriveTypeW(value: *const u16) -> u32;
    fn GetWindowsDirectoryW(value: *mut u16, size: u32) -> u32;
    fn DeviceIoControl(device: *mut c_void, code: u32, input: *mut c_void, input_size: u32, output: *mut c_void, output_size: u32, returned: *mut u32, overlapped: *mut c_void) -> i32;
    fn GetFileInformationByHandle(value: *mut c_void, out: *mut c_void) -> i32;
    fn GetFileInformationByHandleEx(value: *mut c_void, kind: i32, out: *mut c_void, size: u32) -> i32;
    fn GetFinalPathNameByHandleW(value: *mut c_void, out: *mut u16, size: u32, flags: u32) -> u32;
    fn GetVolumeInformationW(root: *const u16, name: *mut u16, size: u32, serial: *mut u32, maximum: *mut u32, flags: *mut u32, fs: *mut u16, fs_size: u32) -> i32;
    fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
    fn ReplaceFileW(target: *const u16, replacement: *const u16, backup: *const u16, flags: u32, exclude: *mut c_void, reserved: *mut c_void) -> i32;
    fn SetFileInformationByHandle(file: *mut c_void, kind: i32, info: *mut c_void, size: u32) -> i32;
}
#[link(name = "shell32")]
unsafe extern "system" {
    fn SHGetKnownFolderPath(id: *const c_void, flags: u32, token: *mut c_void, path: *mut *mut u16) -> i32;
}
#[link(name = "ole32")]
unsafe extern "system" {
    fn CoTaskMemFree(memory: *mut c_void);
}
'''
        crates = source_policy.workspace_crates_from_toml(cargo)
        result = source_policy.scan_source_map(sources, crates)
        self.assertTrue(result.passed, result.findings)

        sources["crates/windows-project-broker/src/windows.rs"] += (
            '\nunsafe extern "system" { fn CreateFileW() -> i32; }\n'
        )
        sources["crates/windows-project-broker/Cargo.toml"] += "serde='1'\n"
        result = source_policy.scan_source_map(sources, crates)
        self.assertFalse(result.passed)
        self.assertTrue(
            any(
                finding.rule == "native project broker forbidden host capability"
                for finding in result.findings
            )
        )
        self.assertTrue(
            any(
                finding.rule == "native project broker must be dependency-free"
                for finding in result.findings
            )
        )

    def test_windows_shell_inventory_and_build_boundary_are_exact(self) -> None:
        sources, crates = self.fixture()
        sources.update({
            "apps/windows-shell/src/main.cpp": """
put_AllowExternalDrop(FALSE);
request->get_Method(&raw_method);
WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS;
SetEnvironmentVariableW();
add_FrameNavigationStarting();
"runtimeReplaySha256";
"verifiedReplayEventCount";
"verifiedReplayBoundaryCount";
message == L"P2REVKF1";
""",
            "apps/windows-shell/src/broker_client.cpp": """
CREATE_SUSPENDED;
JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
ResumeThread(process.hThread);
GOVS_BROKER_SHA256;
GOVS_APP_SHA256;
GOVS_PACKAGE_CONTRACT_SHA256;
BusTypeNvme;
IOCTL_STORAGE_GET_HOTPLUG_INFO;
authoritative_known_folder(FOLDERID_LocalAppData);
authoritative_known_folder(FOLDERID_Profile);
CreateProcessW();
DeviceIoControl();
DeviceIoControl();
""",
            "apps/windows-shell/src/broker_client.h": "#pragma once\n",
            "apps/windows-shell/src/bridge_protocol.cpp": """
if (window !== window.top) return;
"showOpenFilePicker";
"showSaveFilePicker";
"showDirectoryPicker";
"print";
buttonWithText("Verify replay");
'aria-label="Replay verification receipt"';
`P2VEFY1|${runtimeReplayHash}|${verifiedReplay.fingerprint}`;
channel.postMessage("P2REVKF1");
request("revoke", [grantField(grantId)], 5000);
""",
            "apps/windows-shell/src/bridge_protocol.h": "#pragma once\n",
            "tools/phase2/build_windows_shell.mjs": """
const sources = ["main.cpp", "broker_client.cpp", "bridge_protocol.cpp"];
const libraries = [
  "WebView2LoaderStatic.lib", "user32.lib", "gdi32.lib", "ole32.lib",
  "shell32.lib", "bcrypt.lib", "runtimeobject.lib", "advapi32.lib",
];
const flags = "/GS /sdl /guard:cf /Brepro /DYNAMICBASE /NXCOMPAT /HIGHENTROPYVA /CETCOMPAT";
"1.0.4129.50";
"D3934F482D484B89FB4825DF720C710664E1143A1E90F7B3A60794EF33F473D2";
"482F24196B20E784C4D29B752EA760946CB54E22C2532A29699EF538D2D5C28C";
assertSortedUniqueRows(sourceInputs);
verifyRows(packageRows, packageRoot);
const trackedProductionInputs = [...candidateSourceFiles, ...vendorFiles];
const reviewedMapping = "requirements/phase2-reviewed-requirement-mapping.json";
const reviewedRequirementMappingSha256 = hash(reviewedMapping);
""",
        })
        result = source_policy.scan_source_map(sources, crates)
        self.assertTrue(result.passed, result.findings)
        self.assertTrue(source_policy.WINDOWS_SHELL_SOURCES <= set(result.scanned_files))

        sources["apps/windows-shell/src/rogue.cpp"] = "WinHttpOpen();\n"
        sources["tools/phase2/build_windows_shell.mjs"] = sources[
            "tools/phase2/build_windows_shell.mjs"
        ].replace('"advapi32.lib",', '"advapi32.lib", "ws2_32.lib",')
        result = source_policy.scan_source_map(sources, crates)
        self.assertFalse(result.passed)
        self.assertTrue(any(
            finding.rule == "Windows shell production text-source inventory changed"
            for finding in result.findings
        ))
        self.assertTrue(any(
            finding.rule == "Windows shell forbidden host capability"
            for finding in result.findings
        ))
        self.assertTrue(any(
            finding.rule == "Windows shell link library inventory changed"
            for finding in result.findings
        ))

        for path, token in (
            ("apps/windows-shell/src/main.cpp", 'message == L"P2REVKF1"'),
            ("apps/windows-shell/src/bridge_protocol.cpp", 'channel.postMessage("P2REVKF1")'),
            (
                "apps/windows-shell/src/bridge_protocol.cpp",
                'request("revoke", [grantField(grantId)], 5000)',
            ),
        ):
            revoke_sources, revoke_crates = self.fixture()
            revoke_sources.update({
                key: value
                for key, value in sources.items()
                if key in source_policy.WINDOWS_SHELL_SOURCES
                or key == "tools/phase2/build_windows_shell.mjs"
            })
            revoke_sources[path] = revoke_sources[path].replace(token, "")
            revoke_result = source_policy.scan_source_map(revoke_sources, revoke_crates)
            self.assertFalse(revoke_result.passed)
            self.assertTrue(any(
                finding.rule in {
                    "Windows shell browser denial missing",
                    "Windows shell renderer denial missing",
                }
                and finding.match == token
                for finding in revoke_result.findings
            ))

    def test_virtual_download_endpoint_identifier_is_rejected(self) -> None:
        sources, crates = self.fixture()
        sources["apps/foundation-shell/src/runtime-wire.ts"] = (
            'exactKeys(value, ["endpoint", "kind"], "runtime commit-load");\n'
        )
        result = source_policy.scan_source_map(sources, crates)
        self.assertFalse(result.passed)
        self.assertTrue(
            any(
                finding.rule == "endpoint-like Virtual Download target capability"
                for finding in result.findings
            )
        )

    def test_external_evidence_collectors_are_never_production_paths(self) -> None:
        sources, crates = self.fixture()
        sources.update({
            "tools/phase2/assemble_isolation_closure.mjs": 'import "node:child_process";\n',
            "tools/phase2/collect_live_lan_topology.mjs": 'import "node:child_process";\n',
            "tools/phase2/finalize_external_isolation_proofs.mjs": 'import "node:child_process";\n',
        })
        selected = source_policy.production_paths(sources, crates)
        self.assertFalse(
            source_policy.EXTERNAL_EVIDENCE_TOOL_PATHS & set(selected), selected
        )


if __name__ == "__main__":
    unittest.main()
