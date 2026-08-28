#!/usr/bin/env python3
"""Fail-closed production capability scan for the complete Phase 2 workspace.

The Phase 1 source-policy scan used a fixed crate list.  Phase 2 must discover
the Rust workspace from Cargo.toml so newly added crates cannot silently fall
outside the isolation boundary.  This module is also imported by the Phase 2
candidate verifier, which applies the same rules to blobs from the exact Git
candidate rather than trusting the mutable working tree.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Iterable, Mapping


EXIT_PASS = 0
EXIT_POLICY_FAILURE = 1
EXIT_TOOL_ERROR = 2

TYPESCRIPT_ROOTS = (
    "apps/foundation-shell/src",
    "packages/foundation-contract/src",
    "packages/plc-contract/src",
)
TYPESCRIPT_SUFFIXES = {".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"}
RUST_SUFFIXES = {".rs"}
WINDOWS_SHELL_ROOT = "apps/windows-shell/src"
WINDOWS_SHELL_SOURCES = {
    f"{WINDOWS_SHELL_ROOT}/main.cpp",
    f"{WINDOWS_SHELL_ROOT}/broker_client.cpp",
    f"{WINDOWS_SHELL_ROOT}/broker_client.h",
    f"{WINDOWS_SHELL_ROOT}/bridge_protocol.cpp",
    f"{WINDOWS_SHELL_ROOT}/bridge_protocol.h",
}
WINDOWS_SHELL_BUILD_PATH = "tools/phase2/build_windows_shell.mjs"
WINDOWS_SHELL_TEXT_PATHS = WINDOWS_SHELL_SOURCES | {WINDOWS_SHELL_BUILD_PATH}
# External evidence collectors are deliberately not production sources.  They
# may read completed logs on an operator-controlled Windows host, but cannot
# become an application capability simply by being added to the repository.
EXTERNAL_EVIDENCE_TOOL_PATHS = {
    "tools/phase2/assemble_isolation_closure.mjs",
    "tools/phase2/collect_live_lan_topology.mjs",
    "tools/phase2/finalize_external_isolation_proofs.mjs",
}

# These are capability-bearing production dependencies, not merely strings
# which happen to resemble a virtual address.  Address-like strings are valid
# negative-corpus inputs and therefore are not prohibited in Rust source.
BANNED_CARGO_PACKAGES = {
    "async-std",
    "attohttpc",
    "btleplug",
    "curl",
    "hidapi",
    "hyper",
    "hyper-util",
    "isahc",
    "libusb",
    "mio",
    "nix",
    "pnet",
    "reqwest",
    "rusb",
    "serialport",
    "smol",
    "socket2",
    "surf",
    "tokio",
    "tokio-tungstenite",
    "tungstenite",
    "ureq",
    "wasi",
    "web-sys",
    "windows-sys",
    "windows",
}
BANNED_NODE_PACKAGES = {
    "@abandonware/noble",
    "@serialport/bindings-cpp",
    "axios",
    "bluetooth-serial-port",
    "electron",
    "eventsource",
    "http-proxy",
    "node-fetch",
    "node-hid",
    "serialport",
    "socket.io",
    "socket.io-client",
    "tough-cookie",
    "undici",
    "usb",
    "websocket",
    "ws",
}

TS_PATTERNS = (
    (
        "forbidden production import",
        re.compile(
            r"\bfrom\s+[\"'](?:node:)?(?:child_process|cluster|dgram|dns|http|https|net|tls|worker_threads)[\"']",
        ),
    ),
    (
        "forbidden CommonJS production import",
        re.compile(
            r"\brequire\s*\(\s*[\"'](?:node:)?(?:child_process|cluster|dgram|dns|http|https|net|tls|worker_threads)[\"']\s*\)",
        ),
    ),
    (
        "forbidden production capability",
        re.compile(
            r"\b(?:new\s+)?(?:EventSource|RTCPeerConnection|WebSocket|WebTransport|XMLHttpRequest|fetch|importScripts)\s*\(",
        ),
    ),
    (
        "forbidden browser device capability",
        re.compile(
            r"\bnavigator\.(?:bluetooth|hid|mediaDevices|midi|nfc|serial|serviceWorker|usb)\b",
        ),
    ),
    (
        "endpoint-shaped production string",
        re.compile(
            r"\b(?:ftp|https?|wss?)://|\blocalhost\b|\b127\.0\.0\.1\b|\b0\.0\.0\.0\b|\[::1\]",
            re.IGNORECASE,
        ),
    ),
    ("runtime dynamic import", re.compile(r"\bimport\s*\(")),
    ("dynamic execution", re.compile(r"\beval\s*\(|\bnew\s+Function\s*\(")),
)

RUST_PATTERNS = (
    (
        "host network/process capability",
        re.compile(
            r"\bstd::(?:net|process)\b|\b(?:TcpListener|TcpStream|UdpSocket)\b|\bCommand::new\b",
            re.IGNORECASE,
        ),
    ),
    (
        "network/device/runtime capability",
        re.compile(
            r"\b(?:reqwest|hyper|socket2|serialport|rusb|hidapi|btleplug|libusb|tokio::net|mio::net|pnet)::",
            re.IGNORECASE,
        ),
    ),
    (
        "native system ABI capability",
        re.compile(r"\bextern\s+\"system\"", re.IGNORECASE),
    ),
    (
        "foreign-function import block",
        re.compile(r"\bunsafe\s+extern\s+\"C\"\s*\{|\bextern\s+\"C\"\s*\{", re.IGNORECASE),
    ),
    (
        "WASI capability",
        re.compile(r"\b(?:wasi|wasi_snapshot_preview1|wasip1|wasip2)\b", re.IGNORECASE),
    ),
)

WINDOWS_PROJECT_BROKER = "crates/windows-project-broker"
WINDOWS_PROJECT_BROKER_ABI_PATH = f"{WINDOWS_PROJECT_BROKER}/src/windows.rs"
WINDOWS_PROJECT_BROKER_SOURCES = {
    f"{WINDOWS_PROJECT_BROKER}/src/lib.rs",
    f"{WINDOWS_PROJECT_BROKER}/src/main.rs",
    f"{WINDOWS_PROJECT_BROKER}/src/protocol.rs",
    f"{WINDOWS_PROJECT_BROKER}/src/sha256.rs",
    WINDOWS_PROJECT_BROKER_ABI_PATH,
}
WINDOWS_PROJECT_BROKER_ABI = {
    "kernel32": {
        "GetDriveTypeW",
        "GetWindowsDirectoryW",
        "DeviceIoControl",
        "GetFileInformationByHandle",
        "GetFileInformationByHandleEx",
        "GetFinalPathNameByHandleW",
        "GetVolumeInformationW",
        "MoveFileExW",
        "ReplaceFileW",
        "SetFileInformationByHandle",
    },
    "ole32": {"CoTaskMemFree"},
    "shell32": {"SHGetKnownFolderPath"},
}
WINDOWS_PROJECT_BROKER_STD_MODULES = {
    "env",
    "ffi",
    "fs",
    "io",
    "iter",
    "mem",
    "os",
    "path",
    "ptr",
    "slice",
    "sync",
}
WINDOWS_PROJECT_BROKER_FORBIDDEN_APIS = re.compile(
    r"\b(?:CreateFileW|DnsQuery|GetAddrInfo|GetProcAddress|"
    r"InternetOpen|LoadLibrary|OpenPrinter|ReadFile|RegOpenKey|ShellExecute|"
    r"WSAStartup|WriteFile|WritePrinter)\b",
    re.IGNORECASE,
)
WINDOWS_SHELL_FORBIDDEN_APIS = re.compile(
    r"\b(?:WinHttpOpen|InternetOpenW|InternetConnectW|URLDownloadToFileW|"
    r"WSAStartup|DnsQuery_W|"
    r"ShellExecuteW|CreateServiceW|RegOpenKeyExW|LoadLibraryW|GetProcAddress|"
    r"SetupDiGetClassDevsW|BluetoothFindFirstRadio|HidD_GetAttributes|"
    r"WinUsb_Initialize)\s*\(",
)
WINDOWS_SHELL_BUILD_LIBRARIES = (
    "WebView2LoaderStatic.lib",
    "user32.lib",
    "gdi32.lib",
    "ole32.lib",
    "shell32.lib",
    "bcrypt.lib",
    "runtimeobject.lib",
    "advapi32.lib",
)
WINDOWS_SHELL_BUILD_FLAGS = (
    "/GS",
    "/sdl",
    "/guard:cf",
    "/Brepro",
    "/DYNAMICBASE",
    "/NXCOMPAT",
    "/HIGHENTROPYVA",
    "/CETCOMPAT",
)
PER_LANGUAGE_RUNTIME_PATH = re.compile(
    r"(?:^|/)(?:lad|fbd|scl)(?:[-_](?:runtime|engine|executor|interpreter))?(?:/|\.rs$)",
    re.IGNORECASE,
)
PER_LANGUAGE_RUNTIME_IDENTIFIER = re.compile(
    r"\b(?:Lad|Fbd|Scl)(?:Runtime|Engine|Executor|Interpreter)\b|"
    r"\b(?:lad|fbd|scl)_(?:runtime|engine|executor|interpreter)\b",
)
VIRTUAL_DOWNLOAD_ENDPOINT_IDENTIFIER = re.compile(r"\bendpoint\b", re.IGNORECASE)


@dataclass(frozen=True)
class Finding:
    path: str
    line: int
    rule: str
    match: str

    def as_json(self) -> dict[str, object]:
        return {
            "path": self.path,
            "line": self.line,
            "rule": self.rule,
            "match": self.match,
        }


@dataclass(frozen=True)
class SourcePolicyResult:
    workspace_crates: tuple[str, ...]
    scanned_files: tuple[str, ...]
    findings: tuple[Finding, ...]

    @property
    def passed(self) -> bool:
        return not self.findings

    def as_json(self) -> dict[str, object]:
        return {
            "schemaVersion": 1,
            "policy": "PHASE_2_COMPLETE_PRODUCTION_CAPABILITY_SCAN",
            "result": "PASS" if self.passed else "FAIL",
            "workspaceCrates": list(self.workspace_crates),
            "workspaceCrateCount": len(self.workspace_crates),
            "scannedFiles": list(self.scanned_files),
            "scannedFileCount": len(self.scanned_files),
            "findings": [finding.as_json() for finding in self.findings],
        }


def _line_number(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def workspace_crates_from_toml(cargo_toml: str) -> tuple[str, ...]:
    try:
        parsed = tomllib.loads(cargo_toml)
    except (tomllib.TOMLDecodeError, TypeError) as exc:
        raise ValueError(f"Cargo.toml is not valid TOML: {exc}") from exc
    workspace = parsed.get("workspace")
    members = workspace.get("members") if isinstance(workspace, dict) else None
    if not isinstance(members, list) or not members:
        raise ValueError("Cargo.toml must declare a non-empty workspace.members list")
    normalized: list[str] = []
    for member in members:
        if not isinstance(member, str):
            raise ValueError("Cargo workspace member must be a string")
        candidate = PurePosixPath(member.replace("\\", "/"))
        if (
            candidate.is_absolute()
            or ".." in candidate.parts
            or len(candidate.parts) != 2
            or candidate.parts[0] != "crates"
        ):
            raise ValueError(f"Phase 2 workspace member is outside crates/<name>: {member!r}")
        normalized.append(candidate.as_posix())
    if len(normalized) != len(set(normalized)):
        raise ValueError("Cargo workspace contains duplicate member paths")
    return tuple(sorted(normalized))


def production_paths(
    available_paths: Iterable[str], workspace_crates: Iterable[str]
) -> tuple[str, ...]:
    paths = {PurePosixPath(path.replace("\\", "/")).as_posix() for path in available_paths}
    selected: set[str] = set()
    for path in paths:
        suffix = PurePosixPath(path).suffix.lower()
        if suffix in TYPESCRIPT_SUFFIXES and any(
            path == root or path.startswith(root + "/") for root in TYPESCRIPT_ROOTS
        ):
            selected.add(path)
        if path.startswith(WINDOWS_SHELL_ROOT + "/") and suffix in {".cpp", ".h"}:
            selected.add(path)
    if WINDOWS_SHELL_BUILD_PATH in paths:
        selected.add(WINDOWS_SHELL_BUILD_PATH)
    discovered_crate_roots = {
        "/".join(PurePosixPath(path).parts[:2])
        for path in paths
        if len(PurePosixPath(path).parts) >= 3 and PurePosixPath(path).parts[0] == "crates"
    }
    for crate in set(workspace_crates) | discovered_crate_roots:
        manifest = f"{crate}/Cargo.toml"
        selected.add(manifest)
        for path in paths:
            if path == f"{crate}/build.rs" or (
                path.startswith(f"{crate}/src/")
                and PurePosixPath(path).suffix.lower() in RUST_SUFFIXES
            ):
                selected.add(path)
    for fixed in (
        "Cargo.toml",
        "Cargo.lock",
        "package.json",
        "pnpm-lock.yaml",
        "pnpm-workspace.yaml",
    ):
        if fixed in paths:
            selected.add(fixed)
    for path in paths:
        if path.endswith("/package.json") and any(
            path.startswith(prefix + "/") for prefix in ("apps", "packages")
        ):
            selected.add(path)
    # Keep an explicit invariant alongside the normal root selection.  These
    # programs are verification-only even though they use host inspection APIs.
    selected.difference_update(EXTERNAL_EVIDENCE_TOOL_PATHS)
    return tuple(sorted(selected))


def _scan_package_json(path: str, source: str, findings: list[Finding]) -> None:
    try:
        package = json.loads(source)
    except json.JSONDecodeError as exc:
        findings.append(Finding(path, exc.lineno, "invalid package.json", exc.msg))
        return
    for section in ("dependencies", "optionalDependencies"):
        dependencies = package.get(section, {})
        if not isinstance(dependencies, dict):
            findings.append(Finding(path, 1, f"invalid {section}", type(dependencies).__name__))
            continue
        for name in dependencies:
            if name.casefold() in BANNED_NODE_PACKAGES:
                findings.append(Finding(path, 1, f"forbidden {section} capability", name))


def _cargo_dependency_sections(parsed: Mapping[str, object]) -> Iterable[tuple[str, Mapping[str, object]]]:
    for section in ("dependencies", "build-dependencies"):
        value = parsed.get(section)
        if isinstance(value, dict):
            yield section, value
    targets = parsed.get("target")
    if isinstance(targets, dict):
        for target_name, target_value in targets.items():
            if not isinstance(target_value, dict):
                continue
            for section in ("dependencies", "build-dependencies"):
                value = target_value.get(section)
                if isinstance(value, dict):
                    yield f"target.{target_name}.{section}", value


def _scan_cargo_manifest(path: str, source: str, findings: list[Finding]) -> None:
    try:
        parsed = tomllib.loads(source)
    except tomllib.TOMLDecodeError as exc:
        findings.append(Finding(path, 1, "invalid Cargo manifest", str(exc)))
        return
    for section, dependencies in _cargo_dependency_sections(parsed):
        for name in dependencies:
            if name.casefold() in BANNED_CARGO_PACKAGES:
                findings.append(Finding(path, 1, f"forbidden {section} capability", name))
    if path == f"{WINDOWS_PROJECT_BROKER}/Cargo.toml":
        for section, dependencies in _cargo_dependency_sections(parsed):
            for name in dependencies:
                findings.append(
                    Finding(
                        path,
                        1,
                        "native project broker must be dependency-free",
                        f"{section}:{name}",
                    )
                )


def _scan_windows_project_broker_abi(
    path: str, source: str, findings: list[Finding]
) -> None:
    link_matches = list(
        re.finditer(
            r'#\s*\[\s*link\s*\(\s*name\s*=\s*"([^"]+)"\s*\)\s*\]',
            source,
        )
    )
    observed_links = [match.group(1) for match in link_matches]
    if observed_links != ["kernel32", "shell32", "ole32"]:
        findings.append(
            Finding(
                path,
                1,
                "native project broker ABI link order changed",
                ",".join(observed_links) or "missing",
            )
        )

    blocks = list(
        re.finditer(r'unsafe\s+extern\s+"system"\s*\{(?P<body>.*?)\}', source, re.DOTALL)
    )
    if len(blocks) != len(link_matches):
        findings.append(
            Finding(
                path,
                1,
                "native project broker ABI link/block count changed",
                str(len(blocks)),
            )
        )
    else:
        covered_ranges: list[tuple[int, int]] = []
        for link, block in zip(link_matches, blocks, strict=True):
            library = link.group(1)
            covered_ranges.append((link.start(), block.end()))
            expected_imports = WINDOWS_PROJECT_BROKER_ABI.get(library, set())
            imports = {
                match.group(1)
                for match in re.finditer(
                    r"(?m)^\s*fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(",
                    block.group("body"),
                )
            }
            missing = sorted(expected_imports - imports)
            extra = sorted(imports - expected_imports)
            if missing or extra:
                findings.append(
                    Finding(
                        path,
                        _line_number(source, block.start()),
                        "native project broker ABI import set changed",
                        f"library={library};missing={missing};extra={extra}",
                    )
                )
        outside_parts: list[str] = []
        offset = 0
        for start, end in covered_ranges:
            outside_parts.append(source[offset:start])
            offset = end
        outside_parts.append(source[offset:])
        outside = "".join(outside_parts)
        match = re.search(r'\bextern\s+"system"', outside)
        if match:
            findings.append(
                Finding(
                    path,
                    1,
                    "unreviewed native project broker ABI declaration",
                    match.group(0),
                )
            )

    for match in re.finditer(r"\bstd::([A-Za-z_][A-Za-z0-9_]*)", source):
        if match.group(1) not in WINDOWS_PROJECT_BROKER_STD_MODULES:
            findings.append(
                Finding(
                    path,
                    _line_number(source, match.start()),
                    "native project broker std module is not allowlisted",
                    match.group(0),
                )
            )
    for match in WINDOWS_PROJECT_BROKER_FORBIDDEN_APIS.finditer(source):
        findings.append(
            Finding(
                path,
                _line_number(source, match.start()),
                "native project broker forbidden host capability",
                match.group(0),
            )
        )


def _scan_windows_shell_source(path: str, source: str, findings: list[Finding]) -> None:
    for match in WINDOWS_SHELL_FORBIDDEN_APIS.finditer(source):
        findings.append(
            Finding(
                path,
                _line_number(source, match.start()),
                "Windows shell forbidden host capability",
                match.group(0),
            )
        )
    if "CreateProcessW(" in source and path != f"{WINDOWS_SHELL_ROOT}/broker_client.cpp":
        findings.append(
            Finding(path, 1, "Windows shell helper launch escaped fixed broker client", path)
        )
    if path == f"{WINDOWS_SHELL_ROOT}/broker_client.cpp":
        required = (
            "CREATE_SUSPENDED",
            "JOB_OBJECT_LIMIT_ACTIVE_PROCESS",
            "JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE",
            "ResumeThread(process.hThread)",
            "GOVS_BROKER_SHA256",
            "GOVS_APP_SHA256",
            "GOVS_PACKAGE_CONTRACT_SHA256",
            "BusTypeNvme",
            "IOCTL_STORAGE_GET_HOTPLUG_INFO",
            "authoritative_known_folder(FOLDERID_LocalAppData)",
            "authoritative_known_folder(FOLDERID_Profile)",
        )
        for token in required:
            if token not in source:
                findings.append(
                    Finding(path, 1, "Windows shell fail-closed invariant missing", token)
                )
        if source.count("CreateProcessW(") != 1 or source.count("DeviceIoControl(") != 2:
            findings.append(
                Finding(
                    path,
                    1,
                    "Windows shell reviewed process/storage ABI count changed",
                    f"CreateProcessW={source.count('CreateProcessW(')};DeviceIoControl={source.count('DeviceIoControl(')}",
                )
            )
    if path == f"{WINDOWS_SHELL_ROOT}/main.cpp":
        for token in (
            "put_AllowExternalDrop(FALSE)",
            "request->get_Method(&raw_method)",
            "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
            "SetEnvironmentVariableW(",
            "add_FrameNavigationStarting",
            "runtimeReplaySha256",
            "verifiedReplayEventCount",
            "verifiedReplayBoundaryCount",
            'message == L"P2REVKF1"',
        ):
            if token not in source:
                findings.append(Finding(path, 1, "Windows shell browser denial missing", token))
    if path == f"{WINDOWS_SHELL_ROOT}/bridge_protocol.cpp":
        for token in (
            'if (window !== window.top) return;',
            '"showOpenFilePicker"',
            '"showSaveFilePicker"',
            '"showDirectoryPicker"',
            '"print"',
            'buttonWithText("Verify replay")',
            'aria-label="Replay verification receipt"',
            "P2VEFY1|${runtimeReplayHash}|${verifiedReplay.fingerprint}",
            'channel.postMessage("P2REVKF1")',
            'request("revoke", [grantField(grantId)], 5000)',
        ):
            if token not in source:
                findings.append(Finding(path, 1, "Windows shell renderer denial missing", token))


def _scan_windows_shell_build(path: str, source: str, findings: list[Finding]) -> None:
    source_match = re.search(r"const sources = \[(?P<body>.*?)\];", source, re.DOTALL)
    observed_sources = (
        re.findall(r'"([A-Za-z0-9_.-]+\.cpp)"', source_match.group("body"))
        if source_match
        else []
    )
    if observed_sources != ["main.cpp", "broker_client.cpp", "bridge_protocol.cpp"]:
        findings.append(
            Finding(path, 1, "Windows shell compilation unit inventory changed", str(observed_sources))
        )
    library_match = re.search(r"const libraries = \[(?P<body>.*?)\];", source, re.DOTALL)
    observed_libraries = (
        tuple(re.findall(r'"([A-Za-z0-9_.-]+\.lib)"', library_match.group("body")))
        if library_match
        else ()
    )
    if observed_libraries != WINDOWS_SHELL_BUILD_LIBRARIES:
        findings.append(
            Finding(path, 1, "Windows shell link library inventory changed", str(observed_libraries))
        )
    for flag in WINDOWS_SHELL_BUILD_FLAGS:
        if flag not in source:
            findings.append(Finding(path, 1, "Windows shell hardening flag missing", flag))
    for token in (
        '"1.0.4129.50"',
        "D3934F482D484B89FB4825DF720C710664E1143A1E90F7B3A60794EF33F473D2",
        "482F24196B20E784C4D29B752EA760946CB54E22C2532A29699EF538D2D5C28C",
        "assertSortedUniqueRows(sourceInputs",
        "verifyRows(packageRows, packageRoot)",
        "reviewedRequirementMappingSha256",
        "requirements/phase2-reviewed-requirement-mapping.json",
        "...candidateSourceFiles, ...vendorFiles",
    ):
        if token not in source:
            findings.append(Finding(path, 1, "Windows shell exact package binding missing", token))


def _scan_cargo_lock(path: str, source: str, findings: list[Finding]) -> None:
    try:
        parsed = tomllib.loads(source)
    except tomllib.TOMLDecodeError as exc:
        findings.append(Finding(path, 1, "invalid Cargo.lock", str(exc)))
        return
    packages = parsed.get("package", [])
    if not isinstance(packages, list):
        findings.append(Finding(path, 1, "invalid Cargo.lock package list", type(packages).__name__))
        return
    for package in packages:
        if not isinstance(package, dict):
            continue
        name = package.get("name")
        if isinstance(name, str) and name.casefold() in BANNED_CARGO_PACKAGES:
            findings.append(Finding(path, 1, "forbidden locked Cargo capability", name))


def _scan_pnpm_lock(path: str, source: str, findings: list[Finding]) -> None:
    for name in sorted(BANNED_NODE_PACKAGES):
        pattern = re.compile(
            rf"(?m)^\s+['\"]?{re.escape(name)}(?:@[^:\s'\"]+)?['\"]?:",
            re.IGNORECASE,
        )
        match = pattern.search(source)
        if match:
            findings.append(
                Finding(path, _line_number(source, match.start()), "forbidden locked Node capability", name)
            )


def scan_source_map(
    sources: Mapping[str, str], workspace_crates: Iterable[str]
) -> SourcePolicyResult:
    normalized_sources = {
        PurePosixPath(path.replace("\\", "/")).as_posix(): text
        for path, text in sources.items()
    }
    crates = tuple(sorted(workspace_crates))
    findings: list[Finding] = []
    selected = production_paths(normalized_sources, crates)

    discovered_crates = {
        "/".join(PurePosixPath(path).parts[:2])
        for path in normalized_sources
        if len(PurePosixPath(path).parts) >= 3 and PurePosixPath(path).parts[0] == "crates"
    }
    for crate in sorted(discovered_crates - set(crates)):
        findings.append(Finding(crate, 1, "crate is not declared in workspace.members", crate))

    for crate in sorted(set(crates) | discovered_crates):
        manifest = f"{crate}/Cargo.toml"
        crate_sources = [
            path
            for path in selected
            if path.startswith(f"{crate}/src/") and path.endswith(".rs")
        ]
        if manifest not in normalized_sources:
            findings.append(Finding(manifest, 1, "workspace crate manifest missing", crate))
        if not crate_sources:
            findings.append(Finding(crate, 1, "workspace crate production source missing", crate))

    if WINDOWS_PROJECT_BROKER in set(crates) | discovered_crates:
        observed_broker_sources = {
            path
            for path in selected
            if path.startswith(f"{WINDOWS_PROJECT_BROKER}/src/") and path.endswith(".rs")
        }
        missing = sorted(WINDOWS_PROJECT_BROKER_SOURCES - observed_broker_sources)
        extra = sorted(observed_broker_sources - WINDOWS_PROJECT_BROKER_SOURCES)
        if missing or extra:
            findings.append(
                Finding(
                    WINDOWS_PROJECT_BROKER,
                    1,
                    "native project broker production source inventory changed",
                    f"missing={missing};extra={extra}",
                )
            )

    observed_windows_shell = {
        path for path in selected if path.startswith(WINDOWS_SHELL_ROOT + "/")
    }
    if observed_windows_shell or WINDOWS_SHELL_BUILD_PATH in selected:
        expected = WINDOWS_SHELL_TEXT_PATHS
        observed = observed_windows_shell | (
            {WINDOWS_SHELL_BUILD_PATH} if WINDOWS_SHELL_BUILD_PATH in selected else set()
        )
        missing = sorted(expected - observed)
        extra = sorted(observed - expected)
        if missing or extra:
            findings.append(
                Finding(
                    WINDOWS_SHELL_ROOT,
                    1,
                    "Windows shell production text-source inventory changed",
                    f"missing={missing};extra={extra}",
                )
            )

    for path in selected:
        source = normalized_sources.get(path)
        if source is None:
            findings.append(Finding(path, 1, "declared production source missing", path))
            continue
        suffix = PurePosixPath(path).suffix.lower()
        patterns = (
            TS_PATTERNS
            if suffix in TYPESCRIPT_SUFFIXES and any(
                path == root or path.startswith(root + "/") for root in TYPESCRIPT_ROOTS
            )
            else RUST_PATTERNS if suffix in RUST_SUFFIXES else ()
        )
        if path == WINDOWS_PROJECT_BROKER_ABI_PATH:
            patterns = tuple(
                (rule, pattern)
                for rule, pattern in patterns
                if rule not in {"native system ABI capability", "foreign-function import block"}
            )
        for rule, pattern in patterns:
            for match in pattern.finditer(source):
                findings.append(
                    Finding(path, _line_number(source, match.start()), rule, match.group(0)[:160])
                )
        if path == WINDOWS_PROJECT_BROKER_ABI_PATH:
            _scan_windows_project_broker_abi(path, source, findings)
        if path.startswith(WINDOWS_SHELL_ROOT + "/"):
            _scan_windows_shell_source(path, source, findings)
        elif path == WINDOWS_SHELL_BUILD_PATH:
            _scan_windows_shell_build(path, source, findings)
        if path.startswith("crates/plc-runtime/src/"):
            if PER_LANGUAGE_RUNTIME_PATH.search(path):
                findings.append(Finding(path, 1, "per-language production runtime path", path))
            for match in PER_LANGUAGE_RUNTIME_IDENTIFIER.finditer(source):
                findings.append(
                    Finding(
                        path,
                        _line_number(source, match.start()),
                        "per-language production runtime identifier",
                        match.group(0),
                    )
                )
        if path == "apps/foundation-shell/src/runtime-wire.ts":
            for match in VIRTUAL_DOWNLOAD_ENDPOINT_IDENTIFIER.finditer(source):
                findings.append(
                    Finding(
                        path,
                        _line_number(source, match.start()),
                        "endpoint-like Virtual Download target capability",
                        match.group(0),
                    )
                )
        if path.endswith("package.json"):
            _scan_package_json(path, source, findings)
        if path == "Cargo.lock":
            _scan_cargo_lock(path, source, findings)
        elif path == "pnpm-lock.yaml":
            _scan_pnpm_lock(path, source, findings)
        elif path.endswith("Cargo.toml"):
            _scan_cargo_manifest(path, source, findings)

    findings.sort(key=lambda item: (item.path, item.line, item.rule, item.match))
    return SourcePolicyResult(crates, selected, tuple(findings))


def load_worktree_sources(root: Path) -> tuple[dict[str, str], tuple[str, ...]]:
    cargo_path = root / "Cargo.toml"
    cargo_text = cargo_path.read_text(encoding="utf-8")
    crates = workspace_crates_from_toml(cargo_text)
    candidates: set[Path] = {
        root / "Cargo.toml",
        root / "Cargo.lock",
        root / "package.json",
        root / "pnpm-lock.yaml",
        root / "pnpm-workspace.yaml",
    }
    for relative_root in TYPESCRIPT_ROOTS:
        directory = root / PurePosixPath(relative_root)
        if directory.is_dir():
            candidates.update(path for path in directory.rglob("*") if path.is_file())
    windows_shell = root / PurePosixPath(WINDOWS_SHELL_ROOT)
    if windows_shell.is_dir():
        candidates.update(
            path for path in windows_shell.rglob("*")
            if path.is_file() and path.suffix.lower() in {".cpp", ".h"}
        )
        candidates.add(root / PurePosixPath(WINDOWS_SHELL_BUILD_PATH))
    for family in ("apps", "packages"):
        directory = root / family
        if directory.is_dir():
            candidates.update(directory.glob("*/package.json"))
    crate_roots = {root / PurePosixPath(crate) for crate in crates}
    crates_directory = root / "crates"
    if crates_directory.is_dir():
        crate_roots.update(path for path in crates_directory.iterdir() if path.is_dir())
    for crate_root in crate_roots:
        candidates.add(crate_root / "Cargo.toml")
        candidates.add(crate_root / "build.rs")
        source_root = crate_root / "src"
        if source_root.is_dir():
            candidates.update(path for path in source_root.rglob("*.rs") if path.is_file())
    available = [
        path.relative_to(root).as_posix()
        for path in candidates
        if path.is_file()
    ]
    selected = production_paths(available, crates)
    sources: dict[str, str] = {}
    for relative in selected:
        path = root / PurePosixPath(relative)
        if path.is_file():
            sources[relative] = path.read_text(encoding="utf-8")
    return sources, crates


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        root = args.root.resolve(strict=True)
        sources, crates = load_worktree_sources(root)
        result = scan_source_map(sources, crates)
    except (OSError, UnicodeError, ValueError) as exc:
        print(json.dumps({"result": "TOOL_ERROR", "error": str(exc)}, indent=2), file=sys.stderr)
        return EXIT_TOOL_ERROR
    print(json.dumps(result.as_json(), indent=2))
    return EXIT_PASS if result.passed else EXIT_POLICY_FAILURE


if __name__ == "__main__":
    raise SystemExit(main())
