#!/usr/bin/env python3
"""Verify a Phase 2 implementation claim against exact, current evidence.

This gate is intentionally fail closed.  A passing build, an existing test
name, or an implementation claim is not evidence.  Every one of the 937
requirements, 44 Appendix H obligations, eight acceptance journeys, and 15 G2
gates must be explicitly mapped to current PASS evidence before the candidate
verdict can be emitted.

Evidence may live outside the candidate tree (for example, in a verification
workspace or Git note export) so it can bind to an already immutable candidate
commit without creating a self-referential commit hash.  The default tracked
status ledger is a conservative NOT_STARTED baseline and therefore fails until
real evidence is supplied.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from collections import Counter
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path, PurePosixPath
from typing import Any, Iterable, Mapping, Sequence

import reviewed_requirement_mapping
import source_policy


EXIT_PASS = 0
EXIT_POLICY_FAILURE = 1
EXIT_TOOL_ERROR = 2

EXPECTED_REQUIREMENT_COUNT = 937
EXPECTED_VERIFICATION_COUNT = 44
JOURNEY_IDS = tuple("ABCDEFGH")
G2_IDS = tuple(f"G2-{index:02d}" for index in range(1, 16))
REQUIREMENT_STATES = {"NOT_STARTED", "IMPLEMENTED_UNVERIFIED", "VERIFIED"}
VERIFICATION_STATES = {"NOT_STARTED", "IMPLEMENTED_UNVERIFIED", "VERIFIED"}
OUTCOME_STATES = {"NOT_STARTED", "IMPLEMENTED_UNVERIFIED", "PASS", "FAIL"}
PASS_RESULT = "PASS"
REJECTED_RESULTS = {
    "FAIL",
    "SKIP",
    "SKIPPED",
    "FLAKY",
    "CRASH",
    "CRASHED",
    "UNAVAILABLE",
    "INCONCLUSIVE",
    "CANNED",
    "MOCK",
    "NOT_APPLICABLE",
}
REQUIRED_CASE_KINDS = {"POSITIVE", "NEGATIVE", "INTEGRATION"}
ISOLATION_AREAS = {"ACC", "CI", "COM", "ISO", "NET", "PRJ", "PST", "SEC"}
REQUIRED_BINDING_FIELDS = (
    "candidateCommit",
    "candidateTree",
    "isolationApprovalDecisionId",
    "isolationApprovalSha256",
    "productionSourceSha256",
    "testSourceSha256",
    "requirementsSourceSha256",
    "requirementRegistrySha256",
    "verificationCatalogSha256",
    "reviewedRequirementMappingSha256",
    "directiveSha256",
)
HEX_40 = re.compile(r"^[0-9a-f]{40}$")
HEX_64 = re.compile(r"^[0-9A-F]{64}$")
ISOLATION_EVIDENCE_SCHEMA_VERSION = "2.0"
ISOLATION_APPROVAL_DECISION_ID = "P2-DEC-ISO-NATIVE-001"
ISOLATION_APPROVAL_PATH = "ADR/0005-phase-2-native-isolation-shell.md"
SUPPORTED_ISOLATION_CONFIGURATIONS = {
    "windows-x64-chromium-native-broker-adapters-on": (
        "native-broker",
        "adapters-on-controlled-lan",
        {"microsoft-edge-webview2"},
    ),
    "windows-x64-chromium-packaged-adapters-off": (
        "packaged-browser-disabled",
        "adapters-off",
        {"google-chrome", "microsoft-edge"},
    ),
}
SUPPORTED_CHROMIUM_RUNTIME_PRODUCTS = {
    "google-chrome",
    "microsoft-edge",
    "microsoft-edge-webview2",
}
REQUIRED_ISOLATION_BOUNDARIES = {
    "file-metadata-open",
    "file-metadata-create",
    "file-metadata-replace",
    "project-display-name",
    "saved-project-decode",
    "scl-source-text",
    "semantic-navigation",
    "trace-export-canonical-json",
    "trace-export-csv",
    "virtual-download-target",
}
REQUIRED_EXPORT_SURFACES = {
    "project-native-save",
    "replay-verification-package",
    "trace-canonical-json",
    "trace-csv",
}
REQUIRED_NATIVE_BROKER_OPERATIONS = {"open", "create", "replace"}
ISOLATION_FUZZ_CASE_COUNT = 27
ISOLATION_FUZZ_CORPUS_SHA256 = "C61573EF4B2B686E4DC8E326505B65BFFFC4FFE247D8BE2855612F0D6D3D0F66"
ISOLATION_FUZZ_CASE_IDS_SHA256 = "D7FDF0D3ED6E8BF03772F83F44E7F51E432BE67DE0BD20B247FFE076DFD329F8"
PASS_ENVELOPE_FIELDS = {"complete", "result", "schemaVersion"}
BOUNDARY_COVERAGE_FIELDS = PASS_ENVELOPE_FIELDS | {
    "boundaries", "caseCount", "caseIdsSha256", "corpusSha256"
}
BOUNDARY_ROW_FIELDS = {
    "boundaryId", "caseCount", "caseIdsSha256", "corpusSha256", "externalAttemptCount",
    "productionPathExercised", "result", "sideEffectsObserved",
}
CONFIGURATION_BINDING_FIELDS = {
    "architecture", "browserExecutableSha256", "browserFamily", "browserRuntimeProduct",
    "browserRuntimeVersion", "candidateCommit", "candidateTree", "completeLogs", "configurationId",
    "evidenceManifestSha256", "fileAccessPosture", "hostNetworkPosture", "matchesCandidate",
    "platform", "productionPathExercised", "result", "zeroExternalAttempts",
}
NATIVE_BACKING_FIELDS = PASS_ENVELOPE_FIELDS | {
    "architecture", "candidateCommit", "candidateTree", "decisionId", "evidenceManifestSha256",
    "operations", "platform",
}
NATIVE_OPERATION_FIELDS = {
    "attestationVersion", "fixedLocalBacking", "metadataOnlyBeforeAcceptance", "operationId",
    "productionPathExercised", "providerBacked", "redirected", "remote", "removable", "result",
    "selectedByteIoBeforeAcceptance", "special", "unapprovedHelperEffectObserved", "unsafeTarget",
}
TOPOLOGY_VARIATION_FIELDS = PASS_ENVELOPE_FIELDS | {
    "applicationNetworkCapabilityPresent", "discoveryApiSurfacePresent", "scenarios",
}
LAN_SCENARIO_FIELDS = {
    "architecture", "candidateCommit", "candidateTree", "completeLogs", "configurationId",
    "controlledInputSha256", "deterministicOutputSha256", "evidenceManifestSha256",
    "externalAttemptCount", "platform", "postTopologyFingerprint", "preTopologyFingerprint",
    "productionPathExercised", "result", "scenarioId", "topologyFingerprint",
    "topologyMutationControl", "topologySource",
}
EXPORT_REJECTION_FIELDS = PASS_ENVELOPE_FIELDS | {"surfaces"}
EXPORT_SURFACE_FIELDS = {
    "closedFormatSet", "deployableArtifactAttemptsRejected", "productionPathExercised", "result",
    "sideEffectsObserved", "surfaceId", "vendorArtifactAttemptsRejected",
}
ISOLATION_APPROVAL_FIELDS = {"decisionId", "sha256"}


class GateToolError(RuntimeError):
    """The gate could not establish a trustworthy result."""


@dataclass(frozen=True)
class Failure:
    code: str
    subject: str
    message: str

    def as_json(self) -> dict[str, str]:
        return {"code": self.code, "subject": self.subject, "message": self.message}


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest().upper()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest().upper()


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise GateToolError(f"Unable to read JSON {path}: {exc}") from exc


def load_json_bytes(data: bytes, source: str) -> Any:
    try:
        return json.loads(data.decode("utf-8"))
    except (UnicodeError, json.JSONDecodeError) as exc:
        raise GateToolError(f"Unable to read candidate JSON {source}: {exc}") from exc


def run_git(root: Path, *arguments: str, check: bool = True) -> subprocess.CompletedProcess[bytes]:
    try:
        completed = subprocess.run(
            ["git", *arguments],
            cwd=root,
            check=False,
            capture_output=True,
            timeout=30,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise GateToolError(f"Git command failed to execute: git {' '.join(arguments)}: {exc}") from exc
    if check and completed.returncode != 0:
        detail = (completed.stderr or completed.stdout).decode("utf-8", errors="replace").strip()
        raise GateToolError(
            f"Git command failed ({completed.returncode}): git {' '.join(arguments)}: {detail}"
        )
    return completed


def resolve_commit(root: Path, reference: str) -> str:
    resolved = run_git(root, "rev-parse", "--verify", f"{reference}^{{commit}}")
    commit = resolved.stdout.decode("ascii", errors="strict").strip().lower()
    if not HEX_40.fullmatch(commit):
        raise GateToolError(f"Git reference {reference!r} did not resolve to a full commit")
    return commit


def resolve_tree(root: Path, commit: str) -> str:
    resolved = run_git(root, "rev-parse", "--verify", f"{commit}^{{tree}}")
    tree = resolved.stdout.decode("ascii", errors="strict").strip().lower()
    if not HEX_40.fullmatch(tree):
        raise GateToolError(f"Candidate commit {commit} did not resolve to a full tree")
    return tree


def git_tree_entries(root: Path, commit: str) -> dict[str, tuple[str, str]]:
    completed = run_git(root, "ls-tree", "-r", "-z", "--full-tree", commit)
    entries: dict[str, tuple[str, str]] = {}
    for raw in completed.stdout.split(b"\0"):
        if not raw:
            continue
        try:
            metadata, raw_path = raw.split(b"\t", 1)
            _mode, kind, object_id = metadata.decode("ascii").split(" ", 2)
            path = raw_path.decode("utf-8")
        except (UnicodeDecodeError, ValueError) as exc:
            raise GateToolError(f"Unable to parse git ls-tree output: {exc}") from exc
        if kind == "blob":
            entries[PurePosixPath(path).as_posix()] = (object_id.lower(), kind)
    if not entries:
        raise GateToolError("Candidate Git tree contains no blobs")
    return entries


def git_blob_sources(root: Path, commit: str, paths: Sequence[str]) -> dict[str, bytes]:
    """Read path bytes from the exact candidate using one git cat-file batch."""

    normalized = [PurePosixPath(path.replace("\\", "/")).as_posix() for path in paths]
    if any("\n" in path or "\r" in path for path in normalized):
        raise GateToolError("Candidate path contains a line break and cannot be batch-read safely")
    request = b"".join(f"{commit}:{path}\n".encode("utf-8") for path in normalized)
    try:
        completed = subprocess.run(
            ["git", "cat-file", "--batch"],
            cwd=root,
            input=request,
            check=False,
            capture_output=True,
            timeout=60,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise GateToolError(f"Unable to read exact candidate blobs: {exc}") from exc
    if completed.returncode != 0:
        detail = completed.stderr.decode("utf-8", errors="replace").strip()
        raise GateToolError(f"git cat-file --batch failed: {detail}")

    result: dict[str, bytes] = {}
    offset = 0
    output = completed.stdout
    for path in normalized:
        header_end = output.find(b"\n", offset)
        if header_end < 0:
            raise GateToolError(f"Truncated git cat-file header for {path}")
        header = output[offset:header_end].decode("ascii", errors="replace")
        offset = header_end + 1
        fields = header.split(" ")
        if len(fields) == 2 and fields[1] == "missing":
            raise GateToolError(f"Candidate blob is missing: {path}")
        if len(fields) != 3 or fields[1] != "blob":
            raise GateToolError(f"Unexpected git cat-file response for {path}: {header}")
        try:
            size = int(fields[2])
        except ValueError as exc:
            raise GateToolError(f"Invalid candidate blob size for {path}: {fields[2]}") from exc
        end = offset + size
        if end > len(output):
            raise GateToolError(f"Truncated candidate blob for {path}")
        result[path] = output[offset:end]
        offset = end
        if output[offset : offset + 1] != b"\n":
            raise GateToolError(f"Malformed git cat-file separator after {path}")
        offset += 1
    if offset != len(output):
        raise GateToolError("Unexpected trailing bytes from git cat-file --batch")
    return result


def manifest_digest(entries: Mapping[str, tuple[str, str]], paths: Iterable[str]) -> str:
    normalized = sorted(set(paths))
    payload = "".join(f"{entries[path][0]}  {path}\n" for path in normalized).encode("utf-8")
    return sha256_bytes(payload)


def _is_test_path(path: str) -> bool:
    parts = PurePosixPath(path).parts
    return (
        path.startswith("tests/")
        or path.startswith("tools/phase2/")
        or path.startswith("tools/foundation/")
        or path == "tools/phase1/run_pinned_python.mjs"
        or path.startswith(".github/workflows/")
        or "tests" in parts
        or "test" in parts
    )


def _is_requirement_source(path: str) -> bool:
    return (
        path.startswith("requirements/phase2-")
        or path == "requirements/phase1-requirements.json"
        or path == "evidence/phase2/P2-00_ENTRY_GATE.json"
        or (
            path.startswith("References for Codex from Scott/")
            and "Phase 2 of 4" in path
            and path.endswith(".docx")
        )
    )


def candidate_binding(
    root: Path,
    commit: str,
    tree: str,
    entries: Mapping[str, tuple[str, str]],
    requirement_registry_path: str,
    verification_catalog_path: str,
    reviewed_mapping_path: str,
    directive_path: str,
) -> tuple[dict[str, Any], source_policy.SourcePolicyResult]:
    candidate_fixed = git_blob_sources(
        root,
        commit,
        [
            "Cargo.toml",
            ISOLATION_APPROVAL_PATH,
            requirement_registry_path,
            verification_catalog_path,
            reviewed_mapping_path,
            directive_path,
        ],
    )
    try:
        cargo_toml = candidate_fixed["Cargo.toml"].decode("utf-8")
    except UnicodeDecodeError as exc:
        raise GateToolError(f"Candidate Cargo.toml is not UTF-8: {exc}") from exc
    crates = source_policy.workspace_crates_from_toml(cargo_toml)
    production = source_policy.production_paths(entries, crates)
    missing = [path for path in production if path not in entries]
    if missing:
        raise GateToolError(f"Candidate production manifest has missing paths: {', '.join(missing)}")
    test_paths = tuple(sorted(path for path in entries if _is_test_path(path)))
    requirement_paths = tuple(sorted(path for path in entries if _is_requirement_source(path)))
    if not test_paths:
        raise GateToolError("Candidate test-source manifest is empty")
    if not requirement_paths:
        raise GateToolError("Candidate requirement-source manifest is empty")

    candidate_source_bytes = git_blob_sources(root, commit, production)
    try:
        candidate_sources = {
            path: data.decode("utf-8") for path, data in candidate_source_bytes.items()
        }
    except UnicodeDecodeError as exc:
        raise GateToolError(f"Candidate production source is not UTF-8: {exc}") from exc
    policy_result = source_policy.scan_source_map(candidate_sources, crates)

    binding = {
        "candidateCommit": commit,
        "candidateTree": tree,
        "isolationApprovalDecisionId": ISOLATION_APPROVAL_DECISION_ID,
        "isolationApprovalSha256": sha256_bytes(candidate_fixed[ISOLATION_APPROVAL_PATH]),
        "productionSourceSha256": manifest_digest(entries, production),
        "testSourceSha256": manifest_digest(entries, test_paths),
        "requirementsSourceSha256": manifest_digest(entries, requirement_paths),
        "requirementRegistrySha256": sha256_bytes(candidate_fixed[requirement_registry_path]),
        "verificationCatalogSha256": sha256_bytes(candidate_fixed[verification_catalog_path]),
        "reviewedRequirementMappingSha256": sha256_bytes(
            candidate_fixed[reviewed_mapping_path]
        ),
        "directiveSha256": sha256_bytes(candidate_fixed[directive_path]),
        "productionSourceFileCount": len(production),
        "testSourceFileCount": len(test_paths),
        "requirementSourceFileCount": len(requirement_paths),
        "workspaceCrates": [PurePosixPath(crate).name for crate in crates],
    }
    return binding, policy_result


def accounted_untracked_paths(root: Path, entry_gate: Mapping[str, Any]) -> tuple[set[str], list[Failure]]:
    accounted: set[str] = set()
    failures: list[Failure] = []
    records = entry_gate.get("accountedWorkspaceState", [])
    if not isinstance(records, list):
        return accounted, [Failure("P2-EXACT-0001", "P2-00", "accountedWorkspaceState is not a list")]
    for record in records:
        if not isinstance(record, dict):
            failures.append(Failure("P2-EXACT-0001", "P2-00", "accounted path record is not an object"))
            continue
        path_value = record.get("path")
        expected_hash = record.get("sha256")
        if not isinstance(path_value, str) or not isinstance(expected_hash, str):
            failures.append(Failure("P2-EXACT-0001", "P2-00", "accounted path/hash is malformed"))
            continue
        relative = PurePosixPath(path_value.replace("\\", "/"))
        if relative.is_absolute() or ".." in relative.parts:
            failures.append(Failure("P2-EXACT-0001", path_value, "accounted path escapes repository"))
            continue
        path = root / relative
        if not path.is_file():
            # P2-00 records an allowed/accounted user-workspace condition.  A
            # detached clean candidate must not be forced to contain that
            # untracked reference file.
            continue
        observed = sha256_file(path)
        if observed != expected_hash.upper():
            failures.append(
                Failure("P2-EXACT-0001", path_value, f"accounted file hash mismatch: {observed}")
            )
            continue
        accounted.add(relative.as_posix())
    return accounted, failures


def verify_exact_worktree(
    root: Path, candidate_commit: str, entry_gate: Mapping[str, Any]
) -> tuple[list[Failure], list[str]]:
    failures: list[Failure] = []
    head = resolve_commit(root, "HEAD")
    if head != candidate_commit:
        failures.append(
            Failure("P2-EXACT-0002", "candidate", f"candidate {candidate_commit} is not current HEAD {head}")
        )
    if run_git(root, "diff", "--quiet", "--exit-code", check=False).returncode != 0:
        failures.append(Failure("P2-EXACT-0003", "worktree", "tracked working-tree changes exist"))
    if run_git(root, "diff", "--cached", "--quiet", "--exit-code", check=False).returncode != 0:
        failures.append(Failure("P2-EXACT-0004", "index", "staged changes exist"))
    raw_untracked = run_git(root, "ls-files", "--others", "--exclude-standard", "-z").stdout
    try:
        untracked = sorted(
            PurePosixPath(item.decode("utf-8").replace("\\", "/")).as_posix()
            for item in raw_untracked.split(b"\0")
            if item
        )
    except UnicodeDecodeError as exc:
        raise GateToolError(f"Unable to decode untracked path: {exc}") from exc
    accounted, accounting_failures = accounted_untracked_paths(root, entry_gate)
    failures.extend(accounting_failures)
    unexpected = [path for path in untracked if path not in accounted]
    if unexpected:
        failures.append(
            Failure(
                "P2-EXACT-0005",
                "worktree",
                "unaccounted untracked paths exist: " + ", ".join(unexpected),
            )
        )
    return failures, sorted(path for path in untracked if path in accounted)


def verify_extraction_current(root: Path) -> tuple[bool, str]:
    script = root / "tools" / "phase2" / "extract_phase2_requirements.py"
    try:
        completed = subprocess.run(
            [sys.executable, "-B", str(script), "--root", str(root), "--check"],
            cwd=root,
            check=False,
            capture_output=True,
            text=True,
            timeout=120,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise GateToolError(f"Phase 2 extraction freshness check could not execute: {exc}") from exc
    transcript = (completed.stdout + completed.stderr).strip()
    return completed.returncode == 0, transcript


def _inventory(records: Any, id_key: str, expected_ids: set[str], label: str) -> tuple[dict[str, dict[str, Any]], list[Failure]]:
    failures: list[Failure] = []
    if not isinstance(records, list):
        return {}, [Failure("P2-STATUS-0001", label, "status inventory must be a list")]
    indexed: dict[str, dict[str, Any]] = {}
    for ordinal, record in enumerate(records):
        if not isinstance(record, dict):
            failures.append(Failure("P2-STATUS-0001", f"{label}[{ordinal}]", "entry is not an object"))
            continue
        record_id = record.get(id_key)
        if not isinstance(record_id, str):
            failures.append(Failure("P2-STATUS-0001", f"{label}[{ordinal}]", f"missing {id_key}"))
            continue
        if record_id in indexed:
            failures.append(Failure("P2-STATUS-0002", record_id, f"duplicate {label} entry"))
            continue
        indexed[record_id] = record
    missing = sorted(expected_ids - set(indexed))
    extra = sorted(set(indexed) - expected_ids)
    if missing:
        failures.append(Failure("P2-STATUS-0003", label, "missing entries: " + ", ".join(missing)))
    if extra:
        failures.append(Failure("P2-STATUS-0004", label, "unknown entries: " + ", ".join(extra)))
    return indexed, failures


def _status_counts(records: Iterable[Mapping[str, Any]]) -> dict[str, int]:
    counts = Counter(str(record.get("status", "MISSING")) for record in records)
    return dict(sorted(counts.items()))


def _parse_iso_timestamp(value: Any) -> bool:
    if not isinstance(value, str) or not value.strip():
        return False
    try:
        datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return False
    return True


def _safe_artifact_path(base: Path, value: Any) -> Path | None:
    if not isinstance(value, str) or not value:
        return None
    relative = PurePosixPath(value.replace("\\", "/"))
    if relative.is_absolute() or ".." in relative.parts:
        return None
    candidate = (base / relative).resolve(strict=False)
    try:
        candidate.relative_to(base.resolve(strict=True))
    except ValueError:
        return None
    return candidate


def _string_id_set(records: Any, field: str) -> tuple[set[str], bool]:
    if not isinstance(records, list):
        return set(), False
    values: list[str] = []
    for record in records:
        if not isinstance(record, dict) or not isinstance(record.get(field), str):
            return set(), False
        values.append(record[field])
    return set(values), len(values) == len(set(values))


def _is_sha256(value: Any) -> bool:
    return isinstance(value, str) and HEX_64.fullmatch(value) is not None


def _has_exact_fields(value: Any, expected: set[str]) -> bool:
    return isinstance(value, dict) and set(value) == expected


def _is_bounded_runtime_version(value: Any) -> bool:
    return (
        isinstance(value, str)
        and 1 <= len(value) <= 128
        and re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._+ -]*", value) is not None
    )


def validate_isolation_evidence_fields(
    record: Mapping[str, Any], binding: Mapping[str, Any], subject: str
) -> list[Failure]:
    failures: list[Failure] = []
    if record.get("isolationSchemaVersion") != ISOLATION_EVIDENCE_SCHEMA_VERSION:
        failures.append(Failure("P2-EVID-0025", subject, "isolationSchemaVersion must be 2.0"))

    approval = record.get("isolationApproval")
    if (
        not _has_exact_fields(approval, ISOLATION_APPROVAL_FIELDS)
        or approval.get("decisionId") != ISOLATION_APPROVAL_DECISION_ID
        or approval.get("decisionId") != binding.get("isolationApprovalDecisionId")
        or approval.get("sha256") != binding.get("isolationApprovalSha256")
        or not _is_sha256(approval.get("sha256"))
    ):
        failures.append(
            Failure(
                "P2-EVID-0036",
                subject,
                "isolation approval does not match the exact candidate ADR blob",
            )
        )

    platforms = record.get("platformConfigurations")
    platform_ids, platforms_unique = _string_id_set(platforms, "configurationId")
    if not platforms_unique or platform_ids != set(SUPPORTED_ISOLATION_CONFIGURATIONS):
        failures.append(
            Failure(
                "P2-EVID-0026",
                subject,
                "isolation configurations do not exactly match the approved Windows-first set",
            )
        )
    if isinstance(platforms, list):
        for ordinal, platform_record in enumerate(platforms):
            platform_subject = f"{subject}.platformConfigurations[{ordinal}]"
            if not isinstance(platform_record, dict):
                failures.append(Failure("P2-EVID-0027", platform_subject, "configuration is not an object"))
                continue
            configuration_id = str(platform_record.get("configurationId", ""))
            posture = SUPPORTED_ISOLATION_CONFIGURATIONS.get(configuration_id)
            if (
                not _has_exact_fields(platform_record, CONFIGURATION_BINDING_FIELDS)
                or posture is None
                or platform_record.get("platform") != "windows"
                or platform_record.get("architecture") != "x64"
                or platform_record.get("browserFamily") != "chromium"
                or platform_record.get("browserRuntimeProduct") not in SUPPORTED_CHROMIUM_RUNTIME_PRODUCTS
                or platform_record.get("browserRuntimeProduct") not in posture[2]
                or not _is_bounded_runtime_version(platform_record.get("browserRuntimeVersion"))
                or not _is_sha256(platform_record.get("browserExecutableSha256"))
                or platform_record.get("fileAccessPosture") != posture[0]
                or platform_record.get("hostNetworkPosture") != posture[1]
                or platform_record.get("candidateCommit") != binding.get("candidateCommit")
                or platform_record.get("candidateTree") != binding.get("candidateTree")
                or platform_record.get("completeLogs") is not True
                or platform_record.get("matchesCandidate") is not True
                or platform_record.get("productionPathExercised") is not True
                or platform_record.get("zeroExternalAttempts") is not True
                or platform_record.get("result") != "PASS"
                or not _is_sha256(platform_record.get("evidenceManifestSha256"))
            ):
                failures.append(
                    Failure("P2-EVID-0027", platform_subject, "configuration lacks exact-candidate complete PASS proof")
                )

    boundary = record.get("boundaryFuzzCoverage")
    if not isinstance(boundary, dict):
        failures.append(Failure("P2-EVID-0028", subject, "boundary fuzz coverage is missing"))
    else:
        boundary_ids, boundaries_unique = _string_id_set(boundary.get("boundaries"), "boundaryId")
        if (
            not _has_exact_fields(boundary, BOUNDARY_COVERAGE_FIELDS)
            or boundary.get("schemaVersion") != "1.0"
            or boundary.get("complete") is not True
            or boundary.get("result") != "PASS"
            or boundary.get("caseCount") != ISOLATION_FUZZ_CASE_COUNT
            or boundary.get("corpusSha256") != ISOLATION_FUZZ_CORPUS_SHA256
            or boundary.get("caseIdsSha256") != ISOLATION_FUZZ_CASE_IDS_SHA256
            or not boundaries_unique
            or boundary_ids != REQUIRED_ISOLATION_BOUNDARIES
        ):
            failures.append(Failure("P2-EVID-0028", subject, "boundary fuzz coverage is incomplete"))
        if isinstance(boundary.get("boundaries"), list):
            for item in boundary["boundaries"]:
                if (
                    not _has_exact_fields(item, BOUNDARY_ROW_FIELDS)
                    or item.get("caseCount") != boundary.get("caseCount")
                    or item.get("corpusSha256") != boundary.get("corpusSha256")
                    or item.get("caseIdsSha256") != boundary.get("caseIdsSha256")
                    or item.get("externalAttemptCount") != 0
                    or item.get("productionPathExercised") is not True
                    or item.get("sideEffectsObserved") is not False
                    or item.get("result") != "PASS"
                ):
                    failures.append(Failure("P2-EVID-0029", subject, "one or more boundary corpus rows are non-credit"))
                    break

    topology = record.get("liveLanTopologyVariation")
    scenarios = topology.get("scenarios") if isinstance(topology, dict) else None
    scenario_ids, scenarios_unique = _string_id_set(scenarios, "scenarioId")
    topology_hashes = {
        str(scenario.get("topologyFingerprint", ""))
        for scenario in scenarios
        if isinstance(scenario, dict)
    } if isinstance(scenarios, list) else set()
    input_hashes = {
        str(scenario.get("controlledInputSha256", ""))
        for scenario in scenarios
        if isinstance(scenario, dict)
    } if isinstance(scenarios, list) else set()
    output_hashes = {
        str(scenario.get("deterministicOutputSha256", ""))
        for scenario in scenarios
        if isinstance(scenario, dict)
    } if isinstance(scenarios, list) else set()
    if (
        not _has_exact_fields(topology, TOPOLOGY_VARIATION_FIELDS)
        or topology.get("schemaVersion") != "1.0"
        or topology.get("complete") is not True
        or topology.get("result") != "PASS"
        or topology.get("applicationNetworkCapabilityPresent") is not False
        or topology.get("discoveryApiSurfacePresent") is not False
        or not isinstance(scenarios, list)
        or len(scenarios) < 2
        or not scenarios_unique
        or len(scenario_ids) < 2
        or any(not scenario_id for scenario_id in scenario_ids)
        or len(topology_hashes) < 2
        or len(input_hashes) != 1
        or len(output_hashes) != 1
        or any(not _is_sha256(value) for value in topology_hashes | input_hashes | output_hashes)
    ):
        failures.append(Failure("P2-EVID-0030", subject, "controlled live-LAN invariance proof is incomplete"))
    if isinstance(scenarios, list):
        for scenario in scenarios:
            if (
                not _has_exact_fields(scenario, LAN_SCENARIO_FIELDS)
                or scenario.get("candidateCommit") != binding.get("candidateCommit")
                or scenario.get("candidateTree") != binding.get("candidateTree")
                or scenario.get("configurationId") != "windows-x64-chromium-native-broker-adapters-on"
                or scenario.get("platform") != "windows"
                or scenario.get("architecture") != "x64"
                or scenario.get("topologySource") != "WINDOWS_LIVE_ADAPTER_SNAPSHOT"
                or scenario.get("topologyMutationControl") != "EXTERNAL_LAB_OR_OPERATOR_CONTROLLED"
                or scenario.get("completeLogs") is not True
                or scenario.get("externalAttemptCount") != 0
                or scenario.get("productionPathExercised") is not True
                or scenario.get("result") != "PASS"
                or scenario.get("preTopologyFingerprint") != scenario.get("topologyFingerprint")
                or scenario.get("postTopologyFingerprint") != scenario.get("topologyFingerprint")
                or not _is_sha256(scenario.get("evidenceManifestSha256"))
                or not _is_sha256(scenario.get("preTopologyFingerprint"))
                or not _is_sha256(scenario.get("postTopologyFingerprint"))
            ):
                failures.append(Failure("P2-EVID-0031", subject, "a live-LAN scenario is stale or incomplete"))
                break

    backing = record.get("fixedNativeBackingAttestation")
    operations = backing.get("operations") if isinstance(backing, dict) else None
    operation_ids, operations_unique = _string_id_set(operations, "operationId")
    if (
        not _has_exact_fields(backing, NATIVE_BACKING_FIELDS)
        or backing.get("schemaVersion") != "1.0"
        or backing.get("complete") is not True
        or backing.get("result") != "PASS"
        or backing.get("decisionId") != ISOLATION_APPROVAL_DECISION_ID
        or backing.get("candidateCommit") != binding.get("candidateCommit")
        or backing.get("candidateTree") != binding.get("candidateTree")
        or backing.get("platform") != "windows"
        or backing.get("architecture") != "x64"
        or not _is_sha256(backing.get("evidenceManifestSha256"))
        or not operations_unique
        or operation_ids != REQUIRED_NATIVE_BROKER_OPERATIONS
    ):
        failures.append(Failure("P2-EVID-0032", subject, "fixed-native-backing attestation is incomplete"))
    if isinstance(operations, list):
        for operation in operations:
            if (
                not _has_exact_fields(operation, NATIVE_OPERATION_FIELDS)
                or operation.get("attestationVersion") != 1
                or operation.get("fixedLocalBacking") is not True
                or operation.get("providerBacked") is not False
                or operation.get("remote") is not False
                or operation.get("removable") is not False
                or operation.get("special") is not False
                or operation.get("redirected") is not False
                or operation.get("unsafeTarget") is not False
                or operation.get("metadataOnlyBeforeAcceptance") is not True
                or operation.get("selectedByteIoBeforeAcceptance") is not False
                or operation.get("unapprovedHelperEffectObserved") is not False
                or operation.get("productionPathExercised") is not True
                or operation.get("result") != "PASS"
            ):
                failures.append(Failure("P2-EVID-0033", subject, "a native broker operation attestation is non-credit"))
                break

    exports = record.get("vendorDeployableExportRejection")
    surfaces = exports.get("surfaces") if isinstance(exports, dict) else None
    surface_ids, surfaces_unique = _string_id_set(surfaces, "surfaceId")
    if (
        not _has_exact_fields(exports, EXPORT_REJECTION_FIELDS)
        or exports.get("schemaVersion") != "1.0"
        or exports.get("complete") is not True
        or exports.get("result") != "PASS"
        or not surfaces_unique
        or surface_ids != REQUIRED_EXPORT_SURFACES
    ):
        failures.append(Failure("P2-EVID-0034", subject, "export rejection surface inventory is incomplete"))
    if isinstance(surfaces, list):
        for surface in surfaces:
            if (
                not _has_exact_fields(surface, EXPORT_SURFACE_FIELDS)
                or surface.get("closedFormatSet") is not True
                or surface.get("deployableArtifactAttemptsRejected") is not True
                or surface.get("vendorArtifactAttemptsRejected") is not True
                or surface.get("productionPathExercised") is not True
                or surface.get("sideEffectsObserved") is not False
                or surface.get("result") != "PASS"
            ):
                failures.append(Failure("P2-EVID-0035", subject, "an export surface lacks fail-closed proof"))
                break
    return failures


def validate_evidence_record(
    record: Mapping[str, Any], binding: Mapping[str, Any], evidence_base: Path
) -> list[Failure]:
    failures: list[Failure] = []
    evidence_id = record.get("evidenceId")
    subject = evidence_id if isinstance(evidence_id, str) else "<missing-evidence-id>"
    result = str(record.get("result", "MISSING")).upper()
    if result != PASS_RESULT:
        suffix = " (forbidden non-credit result)" if result in REJECTED_RESULTS else ""
        failures.append(Failure("P2-EVID-0001", subject, f"evidence result is {result}{suffix}"))

    observed_binding = record.get("binding")
    if not isinstance(observed_binding, dict):
        failures.append(Failure("P2-EVID-0002", subject, "evidence binding is missing"))
    else:
        for field in REQUIRED_BINDING_FIELDS:
            if observed_binding.get(field) != binding.get(field):
                failures.append(
                    Failure(
                        "P2-EVID-0003",
                        subject,
                        f"stale binding {field}: expected {binding.get(field)!r}, got {observed_binding.get(field)!r}",
                    )
                )

    execution = record.get("execution")
    if not isinstance(execution, dict):
        failures.append(Failure("P2-EVID-0004", subject, "execution record is missing"))
    else:
        if execution.get("exitCode") != 0:
            failures.append(Failure("P2-EVID-0005", subject, "evidence command did not exit 0"))
        if not isinstance(execution.get("command"), str) or not execution.get("command", "").strip():
            failures.append(Failure("P2-EVID-0006", subject, "exact evidence command is missing"))
        if not isinstance(execution.get("attempts"), int) or execution.get("attempts", 0) < 1:
            failures.append(Failure("P2-EVID-0007", subject, "execution attempt count is invalid"))
        for flag in ("skipped", "flaky", "crashed", "unavailable", "inconclusive", "canned"):
            if execution.get(flag) is not False:
                failures.append(Failure("P2-EVID-0008", subject, f"{flag} must be explicitly false"))
        if execution.get("productionPathExercised") is not True:
            failures.append(Failure("P2-EVID-0009", subject, "productionPathExercised must be true"))
        if not _parse_iso_timestamp(execution.get("startedAt")) or not _parse_iso_timestamp(
            execution.get("finishedAt")
        ):
            failures.append(Failure("P2-EVID-0010", subject, "execution timestamps are missing or invalid"))

    case_kinds = record.get("caseKinds")
    if not isinstance(case_kinds, list) or not case_kinds:
        failures.append(Failure("P2-EVID-0011", subject, "caseKinds is empty"))
    elif any(str(kind).upper() in REJECTED_RESULTS for kind in case_kinds):
        failures.append(Failure("P2-EVID-0012", subject, "caseKinds contains a non-credit state"))

    artifacts = record.get("artifacts")
    valid_log = False
    if not isinstance(artifacts, list) or not artifacts:
        failures.append(Failure("P2-EVID-0013", subject, "complete evidence artifacts are missing"))
    else:
        for ordinal, artifact in enumerate(artifacts):
            artifact_subject = f"{subject}.artifacts[{ordinal}]"
            if not isinstance(artifact, dict):
                failures.append(Failure("P2-EVID-0014", artifact_subject, "artifact is not an object"))
                continue
            path = _safe_artifact_path(evidence_base, artifact.get("path"))
            if path is None or not path.is_file():
                failures.append(Failure("P2-EVID-0015", artifact_subject, "artifact path is unsafe or missing"))
                continue
            observed_bytes = path.stat().st_size
            observed_hash = sha256_file(path)
            if artifact.get("bytes") != observed_bytes:
                failures.append(Failure("P2-EVID-0016", artifact_subject, "artifact byte count is stale"))
            if artifact.get("sha256") != observed_hash:
                failures.append(Failure("P2-EVID-0017", artifact_subject, "artifact SHA-256 is stale"))
            if str(artifact.get("kind", "")).upper() == "LOG" and observed_bytes > 0:
                valid_log = True
        if not valid_log:
            failures.append(Failure("P2-EVID-0018", subject, "at least one non-empty hashed LOG is required"))

    kind = str(record.get("kind", "")).upper()
    if kind == "MUTATION":
        expected = record.get("expectedDetector")
        actual = record.get("actualDetector")
        mutation_id = record.get("mutationId")
        detector_exit = record.get("detectorExitCode")
        if not all(isinstance(value, str) and value.strip() for value in (expected, actual, mutation_id)):
            failures.append(Failure("P2-EVID-0019", subject, "named mutation/detector fields are required"))
        elif expected != actual:
            failures.append(Failure("P2-EVID-0020", subject, "mutation received credit from an unrelated detector"))
        if not isinstance(detector_exit, int) or detector_exit == 0:
            failures.append(Failure("P2-EVID-0021", subject, "mutation detector must reject the mutation without crash credit"))
    if kind == "ISOLATION":
        if record.get("zeroExternalAttempts") is not True:
            failures.append(Failure("P2-EVID-0022", subject, "isolation evidence lacks zero-attempt proof"))
        if record.get("instrumentationStatus") != "COMPLETE":
            failures.append(Failure("P2-EVID-0023", subject, "isolation instrumentation is not COMPLETE"))
        platforms = record.get("platformConfigurations")
        if not isinstance(platforms, list) or not platforms:
            failures.append(Failure("P2-EVID-0024", subject, "isolation platform/configuration coverage is empty"))
        failures.extend(validate_isolation_evidence_fields(record, binding, subject))
    return failures


def status_subjects(record: Mapping[str, Any]) -> dict[str, set[str]]:
    subjects = record.get("subjects")
    if not isinstance(subjects, dict):
        return {key: set() for key in ("requirements", "verifications", "journeys", "gates")}
    result: dict[str, set[str]] = {}
    for key in ("requirements", "verifications", "journeys", "gates"):
        values = subjects.get(key, [])
        result[key] = {str(value) for value in values} if isinstance(values, list) else set()
    return result


def _evidence_refs(entry: Mapping[str, Any], subject: str, failures: list[Failure]) -> list[str]:
    refs = entry.get("evidenceIds")
    if not isinstance(refs, list):
        failures.append(Failure("P2-STATUS-0005", subject, "evidenceIds must be a list"))
        return []
    if any(not isinstance(item, str) or not item for item in refs):
        failures.append(Failure("P2-STATUS-0005", subject, "evidenceIds contains an invalid ID"))
        return []
    if len(refs) != len(set(refs)):
        failures.append(Failure("P2-STATUS-0006", subject, "evidenceIds contains duplicates"))
    return list(refs)


def validate_status_claim(
    ledger: Mapping[str, Any],
    requirement_registry: Mapping[str, Any],
    verification_catalog: Mapping[str, Any],
    reviewed_mapping: Mapping[str, Any],
    binding: Mapping[str, Any],
    evidence_base: Path,
    candidate_paths: set[str],
) -> tuple[dict[str, Any], list[Failure]]:
    failures: list[Failure] = []
    requirement_source = requirement_registry.get("requirements", [])
    verification_source = verification_catalog.get("verificationRecords", [])
    if not isinstance(requirement_source, list) or len(requirement_source) != EXPECTED_REQUIREMENT_COUNT:
        failures.append(
            Failure("P2-CAT-0001", "requirements", f"expected {EXPECTED_REQUIREMENT_COUNT} extracted requirements")
        )
    if not isinstance(verification_source, list) or len(verification_source) != EXPECTED_VERIFICATION_COUNT:
        failures.append(
            Failure("P2-CAT-0002", "verifications", f"expected {EXPECTED_VERIFICATION_COUNT} Appendix H records")
        )
    requirement_ids = {
        str(item.get("id")) for item in requirement_source if isinstance(item, dict) and isinstance(item.get("id"), str)
    }
    verification_ids = {
        str(item.get("verificationId"))
        for item in verification_source
        if isinstance(item, dict) and isinstance(item.get("verificationId"), str)
    }
    requirement_by_id = {
        str(item["id"]): item for item in requirement_source if isinstance(item, dict) and isinstance(item.get("id"), str)
    }
    try:
        reviewed_by_requirement = reviewed_requirement_mapping.validate_reviewed_mapping(
            reviewed_mapping,
            requirement_registry,
            verification_catalog,
            requirement_registry_sha256=str(binding.get("requirementRegistrySha256", "")),
            verification_catalog_sha256=str(binding.get("verificationCatalogSha256", "")),
            directive_sha256=str(binding.get("directiveSha256", "")),
            expected_requirement_count=EXPECTED_REQUIREMENT_COUNT,
            expected_verification_count=EXPECTED_VERIFICATION_COUNT,
        )
    except reviewed_requirement_mapping.ReviewedMappingError as exc:
        failures.append(Failure("P2-MAP-0004", "reviewed-mapping", str(exc)))
        reviewed_by_requirement = {}

    requirements, inventory_failures = _inventory(
        ledger.get("requirements"), "requirementId", requirement_ids, "requirements"
    )
    failures.extend(inventory_failures)
    verifications, inventory_failures = _inventory(
        ledger.get("verifications"), "verificationId", verification_ids, "verifications"
    )
    failures.extend(inventory_failures)
    journeys, inventory_failures = _inventory(
        ledger.get("journeys"), "journeyId", set(JOURNEY_IDS), "journeys"
    )
    failures.extend(inventory_failures)
    gates, inventory_failures = _inventory(ledger.get("gates"), "gateId", set(G2_IDS), "gates")
    failures.extend(inventory_failures)

    evidence_records = ledger.get("evidenceRecords", [])
    evidence_index: dict[str, dict[str, Any]] = {}
    if not isinstance(evidence_records, list):
        failures.append(Failure("P2-EVID-0025", "evidenceRecords", "must be a list"))
        evidence_records = []
    for ordinal, record in enumerate(evidence_records):
        if not isinstance(record, dict) or not isinstance(record.get("evidenceId"), str):
            failures.append(Failure("P2-EVID-0026", f"evidenceRecords[{ordinal}]", "invalid evidence record"))
            continue
        evidence_id = record["evidenceId"]
        if evidence_id in evidence_index:
            failures.append(Failure("P2-EVID-0027", evidence_id, "duplicate evidence ID"))
            continue
        evidence_index[evidence_id] = record
        failures.extend(validate_evidence_record(record, binding, evidence_base))

    valid_subject_ids = {
        "requirements": requirement_ids,
        "verifications": verification_ids,
        "journeys": set(JOURNEY_IDS),
        "gates": set(G2_IDS),
    }
    for evidence_id, record in evidence_index.items():
        subjects = status_subjects(record)
        if not any(subjects.values()):
            failures.append(Failure("P2-EVID-0028", evidence_id, "evidence has no declared subjects"))
        for family, ids in subjects.items():
            unknown = ids - valid_subject_ids[family]
            if unknown:
                failures.append(
                    Failure("P2-EVID-0029", evidence_id, f"unknown {family}: {', '.join(sorted(unknown))}")
                )

    def linked(entry: Mapping[str, Any], family: str, subject_id: str) -> list[dict[str, Any]]:
        refs = _evidence_refs(entry, subject_id, failures)
        records: list[dict[str, Any]] = []
        for evidence_id in refs:
            record = evidence_index.get(evidence_id)
            if record is None:
                failures.append(Failure("P2-STATUS-0007", subject_id, f"missing evidence {evidence_id}"))
                continue
            if subject_id not in status_subjects(record)[family]:
                failures.append(
                    Failure("P2-STATUS-0008", subject_id, f"evidence {evidence_id} does not name this subject")
                )
                continue
            records.append(record)
        return records

    for requirement_id, entry in requirements.items():
        state = str(entry.get("status", "MISSING"))
        if state not in REQUIREMENT_STATES:
            failures.append(Failure("P2-STATUS-0009", requirement_id, f"invalid requirement status {state}"))
            continue
        records = linked(entry, "requirements", requirement_id)
        if state == "NOT_STARTED" and records:
            failures.append(Failure("P2-STATUS-0010", requirement_id, "NOT_STARTED requirement cites evidence"))
        if state == "IMPLEMENTED_UNVERIFIED":
            implementation_paths = entry.get("implementationPaths")
            if not isinstance(implementation_paths, list) or not implementation_paths:
                failures.append(
                    Failure("P2-STATUS-0011", requirement_id, "implementation paths are required for IMPLEMENTED_UNVERIFIED")
                )
            else:
                for path in implementation_paths:
                    normalized = PurePosixPath(str(path).replace("\\", "/")).as_posix()
                    if normalized not in candidate_paths:
                        failures.append(
                            Failure("P2-STATUS-0012", requirement_id, f"implementation path is absent from candidate: {normalized}")
                        )
        reviewed_row = reviewed_by_requirement.get(requirement_id)
        mapped = entry.get("verificationIds")
        if entry.get("mappingStatus") != "REVIEWED":
            failures.append(
                Failure("P2-MAP-0001", requirement_id, "Appendix H mapping remains unreviewed")
            )
        if (
            not isinstance(mapped, list)
            or not mapped
            or any(not isinstance(item, str) or not item for item in mapped)
            or len(mapped) != len(set(mapped))
        ):
            failures.append(
                Failure("P2-MAP-0002", requirement_id, "no exact Appendix H mapping is declared")
            )
        elif reviewed_row is None or mapped != reviewed_row.get("selectedVerificationIds"):
            failures.append(
                Failure(
                    "P2-MAP-0003",
                    requirement_id,
                    "declared Appendix H mapping does not exactly match the reviewed artifact",
                )
            )
        if state == "VERIFIED":
            if not records:
                failures.append(Failure("P2-STATUS-0013", requirement_id, "VERIFIED requirement lacks evidence"))
            observed_case_kinds = {
                str(kind).upper()
                for record in records
                for kind in record.get("caseKinds", [])
                if isinstance(kind, str)
            }
            missing_case_kinds = REQUIRED_CASE_KINDS - observed_case_kinds
            if missing_case_kinds:
                failures.append(
                    Failure(
                        "P2-STATUS-0014",
                        requirement_id,
                        "missing case classes: " + ", ".join(sorted(missing_case_kinds)),
                    )
                )
            area = str(requirement_by_id.get(requirement_id, {}).get("area", ""))
            if area in ISOLATION_AREAS and "ISOLATION" not in observed_case_kinds:
                failures.append(Failure("P2-STATUS-0015", requirement_id, "applicable isolation evidence is missing"))

    for verification_id, entry in verifications.items():
        state = str(entry.get("status", "MISSING"))
        if state not in VERIFICATION_STATES:
            failures.append(Failure("P2-STATUS-0018", verification_id, f"invalid verification status {state}"))
            continue
        records = linked(entry, "verifications", verification_id)
        if state == "NOT_STARTED" and records:
            failures.append(Failure("P2-STATUS-0019", verification_id, "NOT_STARTED verification cites evidence"))
        if state == "VERIFIED" and not records:
            failures.append(Failure("P2-STATUS-0020", verification_id, "VERIFIED obligation lacks evidence"))

    for family, indexed, state_key in (
        ("journeys", journeys, "journeyId"),
        ("gates", gates, "gateId"),
    ):
        for subject_id, entry in indexed.items():
            state = str(entry.get("status", "MISSING"))
            if state not in OUTCOME_STATES:
                failures.append(Failure("P2-STATUS-0021", subject_id, f"invalid outcome status {state}"))
                continue
            records = linked(entry, family, subject_id)
            if state == "NOT_STARTED" and records:
                failures.append(Failure("P2-STATUS-0022", subject_id, "NOT_STARTED outcome cites evidence"))
            if state == "PASS" and not records:
                failures.append(Failure("P2-STATUS-0023", subject_id, "PASS outcome lacks evidence"))
            if state == "PASS" and subject_id in {"H", "G2-11"} and not any(
                str(record.get("kind", "")).upper() == "MUTATION" for record in records
            ):
                failures.append(Failure("P2-STATUS-0024", subject_id, "anti-theater PASS lacks mutation evidence"))
            if state == "PASS" and subject_id in {"G", "G2-12"} and not any(
                str(record.get("kind", "")).upper() == "ISOLATION" for record in records
            ):
                failures.append(Failure("P2-STATUS-0025", subject_id, "isolation PASS lacks isolation evidence"))

    defects = ledger.get("defects", [])
    if not isinstance(defects, list):
        failures.append(Failure("P2-DEF-0001", "defects", "defect ledger must be a list"))
        defects = []
    for ordinal, defect in enumerate(defects):
        if not isinstance(defect, dict):
            failures.append(Failure("P2-DEF-0001", f"defects[{ordinal}]", "defect is not an object"))
            continue
        severity = str(defect.get("severity", "")).upper()
        state = str(defect.get("status", "OPEN")).upper()
        if severity in {"CRITICAL", "HIGH"} and state not in {"CLOSED", "RESOLVED"}:
            failures.append(
                Failure("P2-DEF-0002", str(defect.get("defectId", ordinal)), f"unresolved {severity} defect")
            )

    all_requirements_verified = len(requirements) == EXPECTED_REQUIREMENT_COUNT and all(
        entry.get("status") == "VERIFIED" for entry in requirements.values()
    )
    all_verifications_verified = len(verifications) == EXPECTED_VERIFICATION_COUNT and all(
        entry.get("status") == "VERIFIED" for entry in verifications.values()
    )
    all_journeys_pass = len(journeys) == len(JOURNEY_IDS) and all(
        entry.get("status") == "PASS" for entry in journeys.values()
    )
    all_gates_pass = len(gates) == len(G2_IDS) and all(
        entry.get("status") == "PASS" for entry in gates.values()
    )
    if not all_requirements_verified:
        failures.append(Failure("P2-COMP-0001", "requirements", "all 937 requirements are not VERIFIED"))
    if not all_verifications_verified:
        failures.append(Failure("P2-COMP-0002", "verifications", "all 44 Appendix H obligations are not VERIFIED"))
    if not all_journeys_pass:
        failures.append(Failure("P2-COMP-0003", "journeys", "Journeys A-H are not all PASS"))
    if not all_gates_pass:
        failures.append(Failure("P2-COMP-0004", "gates", "G2-01 through G2-15 are not all PASS"))

    candidate = ledger.get("candidate")
    if all_requirements_verified and all_verifications_verified and all_journeys_pass and all_gates_pass:
        if not isinstance(candidate, dict):
            failures.append(Failure("P2-COMP-0005", "candidate", "candidate record is missing"))
        else:
            if candidate.get("commit") != binding.get("candidateCommit"):
                failures.append(Failure("P2-COMP-0006", "candidate", "candidate commit does not match exact HEAD"))
            tag = candidate.get("tag")
            if not isinstance(tag, str) or not tag.strip():
                failures.append(Failure("P2-COMP-0007", "candidate", "final candidate tag is missing"))

    summary = {
        "requirements": {
            "expected": EXPECTED_REQUIREMENT_COUNT,
            "enumerated": len(requirements),
            "statusCounts": _status_counts(requirements.values()),
            "records": [
                {"requirementId": key, "status": value.get("status", "MISSING")}
                for key, value in sorted(requirements.items())
            ],
        },
        "verifications": {
            "expected": EXPECTED_VERIFICATION_COUNT,
            "enumerated": len(verifications),
            "statusCounts": _status_counts(verifications.values()),
            "records": [
                {"verificationId": key, "status": value.get("status", "MISSING")}
                for key, value in sorted(verifications.items())
            ],
        },
        "journeys": {
            "expected": len(JOURNEY_IDS),
            "enumerated": len(journeys),
            "statusCounts": _status_counts(journeys.values()),
            "records": [
                {"journeyId": key, "status": value.get("status", "MISSING")}
                for key, value in sorted(journeys.items())
            ],
        },
        "gates": {
            "expected": len(G2_IDS),
            "enumerated": len(gates),
            "statusCounts": _status_counts(gates.values()),
            "records": [
                {"gateId": key, "status": value.get("status", "MISSING")}
                for key, value in sorted(gates.items())
            ],
        },
        "evidenceRecordCount": len(evidence_index),
        "defectCount": len(defects),
    }
    return summary, failures


def initial_status_ledger(
    requirement_registry: Mapping[str, Any],
    verification_catalog: Mapping[str, Any],
    reviewed_mapping: Mapping[str, Any],
    *,
    requirement_registry_sha256: str,
    verification_catalog_sha256: str,
    directive_sha256: str,
) -> dict[str, Any]:
    requirements = requirement_registry.get("requirements", [])
    verifications = verification_catalog.get("verificationRecords", [])
    mapping = reviewed_requirement_mapping.validate_reviewed_mapping(
        reviewed_mapping,
        requirement_registry,
        verification_catalog,
        requirement_registry_sha256=requirement_registry_sha256,
        verification_catalog_sha256=verification_catalog_sha256,
        directive_sha256=directive_sha256,
        expected_requirement_count=EXPECTED_REQUIREMENT_COUNT,
        expected_verification_count=EXPECTED_VERIFICATION_COUNT,
    )
    return {
        "schemaVersion": 1,
        "ledgerKind": "PHASE_2_IMPLEMENTATION_EVIDENCE_STATUS",
        "directive": dict(requirement_registry.get("directive", {})),
        "truthPolicy": {
            "default": "NOT_STARTED",
            "verifiedRequiresCurrentExecutableEvidence": True,
            "passNeverInferredFromBuildOrTestName": True,
            "reviewedMappingDoesNotGrantVerificationCredit": True,
            "nonCreditResults": sorted(REJECTED_RESULTS),
        },
        "candidate": {"commit": None, "tag": None},
        "requirements": [
            {
                "requirementId": item["id"],
                "status": "NOT_STARTED",
                "verificationIds": mapping[item["id"]]["selectedVerificationIds"],
                "mappingStatus": "REVIEWED",
                "implementationPaths": [],
                "evidenceIds": [],
            }
            for item in requirements
        ],
        "verifications": [
            {
                "verificationId": item["verificationId"],
                "status": "NOT_STARTED",
                "evidenceIds": [],
            }
            for item in verifications
        ],
        "journeys": [
            {"journeyId": journey_id, "status": "NOT_STARTED", "evidenceIds": []}
            for journey_id in JOURNEY_IDS
        ],
        "gates": [
            {"gateId": gate_id, "status": "NOT_STARTED", "evidenceIds": []}
            for gate_id in G2_IDS
        ],
        "evidenceRecords": [],
        "defects": [],
    }


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--candidate-ref", default="HEAD")
    parser.add_argument(
        "--status", type=Path, default=Path("evidence/phase2/PHASE2_IMPLEMENTATION_STATUS.json")
    )
    parser.add_argument("--report", type=Path)
    parser.add_argument("--binding-only", action="store_true")
    return parser.parse_args(argv)


def _write_report(path: Path, report: Mapping[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        root = args.root.resolve(strict=True)
        requirement_registry_relative = "requirements/phase2-requirements.json"
        verification_catalog_relative = "requirements/phase2-verification-catalog.json"
        reviewed_mapping_relative = reviewed_requirement_mapping.REVIEWED_MAPPING_PATH
        entry_gate_relative = "evidence/phase2/P2-00_ENTRY_GATE.json"
        commit = resolve_commit(root, args.candidate_ref)
        tree = resolve_tree(root, commit)
        entries = git_tree_entries(root, commit)
        candidate_catalog_bytes = git_blob_sources(
            root,
            commit,
            [
                requirement_registry_relative,
                verification_catalog_relative,
                reviewed_mapping_relative,
                entry_gate_relative,
            ],
        )
        requirement_registry = load_json_bytes(
            candidate_catalog_bytes[requirement_registry_relative], requirement_registry_relative
        )
        verification_catalog = load_json_bytes(
            candidate_catalog_bytes[verification_catalog_relative], verification_catalog_relative
        )
        reviewed_mapping = load_json_bytes(
            candidate_catalog_bytes[reviewed_mapping_relative], reviewed_mapping_relative
        )
        entry_gate = load_json_bytes(candidate_catalog_bytes[entry_gate_relative], entry_gate_relative)
        directive_relative = requirement_registry.get("directive", {}).get("path")
        if not isinstance(directive_relative, str):
            raise GateToolError("Requirement registry does not identify the directive path")
        directive_relative = PurePosixPath(directive_relative.replace("\\", "/")).as_posix()
        if directive_relative not in entries:
            raise GateToolError("Canonical Phase 2 directive is absent from the candidate tree")
        exact_failures, accounted_paths = verify_exact_worktree(root, commit, entry_gate)
        binding, policy_result = candidate_binding(
            root,
            commit,
            tree,
            entries,
            requirement_registry_relative,
            verification_catalog_relative,
            reviewed_mapping_relative,
            directive_relative,
        )
        extraction_current, extraction_transcript = verify_extraction_current(root)

        failures = list(exact_failures)
        if entry_gate.get("status") != "PASS":
            failures.append(Failure("P2-ENTRY-0001", "P2-00", "entry gate status is not PASS"))
        authority = entry_gate.get("authority")
        if not isinstance(authority, dict) or authority.get("directiveSha256") != binding.get(
            "directiveSha256"
        ):
            failures.append(
                Failure("P2-ENTRY-0002", "P2-00", "entry-gate directive binding is stale")
            )
        incoming = entry_gate.get("incomingBaseline")
        if not isinstance(incoming, dict):
            failures.append(Failure("P2-ENTRY-0003", "P2-00", "incoming baseline is missing"))
        else:
            for field in ("commit", "tree", "tagTarget"):
                value = incoming.get(field)
                if not isinstance(value, str) or not HEX_40.fullmatch(value.lower()):
                    failures.append(
                        Failure("P2-ENTRY-0004", "P2-00", f"incoming baseline {field} is malformed")
                    )
        if not policy_result.passed:
            failures.extend(
                Failure("P2-ISO-0001", finding.path, f"{finding.rule} at line {finding.line}: {finding.match}")
                for finding in policy_result.findings
            )
        if not extraction_current:
            failures.append(Failure("P2-CAT-0003", "requirements", "extracted Phase 2 registries are stale"))
        try:
            reviewed_requirement_mapping.validate_reviewed_mapping(
                reviewed_mapping,
                requirement_registry,
                verification_catalog,
                requirement_registry_sha256=binding["requirementRegistrySha256"],
                verification_catalog_sha256=binding["verificationCatalogSha256"],
                directive_sha256=binding["directiveSha256"],
                expected_requirement_count=EXPECTED_REQUIREMENT_COUNT,
                expected_verification_count=EXPECTED_VERIFICATION_COUNT,
            )
        except reviewed_requirement_mapping.ReviewedMappingError as exc:
            failures.append(Failure("P2-MAP-0004", "reviewed-mapping", str(exc)))

        base_report: dict[str, Any] = {
            "schemaVersion": 1,
            "gate": "PHASE_2_IMPLEMENTATION_EXIT_GATE",
            "incomingPhase1Baseline": entry_gate.get("incomingBaseline"),
            "candidateBinding": binding,
            "exactWorktree": {
                "required": True,
                "accountedUntrackedPaths": accounted_paths,
            },
            "requirementExtraction": {
                "current": extraction_current,
                "transcript": extraction_transcript,
            },
            "reviewedRequirementMapping": {
                "path": reviewed_mapping_relative,
                "sha256": binding["reviewedRequirementMappingSha256"],
                "rowsSha256": reviewed_mapping.get("binding", {}).get(
                    "reviewedRowsSha256"
                ),
            },
            "sourcePolicy": policy_result.as_json(),
        }
        if args.binding_only:
            base_report.update(
                {
                    "result": "PASS" if not failures else "FAIL",
                    "verdict": "BLOCKED" if failures else "BINDING_READY",
                    "failures": [failure.as_json() for failure in failures],
                }
            )
            print(json.dumps(base_report, indent=2, ensure_ascii=False))
            if args.report:
                _write_report(args.report.resolve(strict=False), base_report)
            return EXIT_PASS if not failures else EXIT_POLICY_FAILURE

        status_path = args.status
        if not status_path.is_absolute():
            status_path = root / status_path
        status_path = status_path.resolve(strict=True)
        ledger = load_json(status_path)
        status_summary, status_failures = validate_status_claim(
            ledger,
            requirement_registry,
            verification_catalog,
            reviewed_mapping,
            binding,
            status_path.parent,
            set(entries),
        )
        failures.extend(status_failures)

        candidate = ledger.get("candidate") if isinstance(ledger, dict) else None
        if isinstance(candidate, dict) and isinstance(candidate.get("tag"), str) and candidate.get("tag"):
            try:
                tag_commit = resolve_commit(root, candidate["tag"])
            except GateToolError as exc:
                failures.append(Failure("P2-COMP-0008", "candidate", str(exc)))
            else:
                if tag_commit != commit:
                    failures.append(Failure("P2-COMP-0009", "candidate", "candidate tag does not resolve to exact HEAD"))

        passed = not failures
        report = {
            **base_report,
            "result": "PASS" if passed else "FAIL",
            "verdict": (
                "PHASE 2 IMPLEMENTATION CANDIDATE - AWAITING SCOTT ACCEPTANCE"
                if passed
                else "BLOCKED"
            ),
            "statusLedger": str(status_path),
            "inventory": status_summary,
            "failureCount": len(failures),
            "failures": [failure.as_json() for failure in failures],
        }
        print(json.dumps(report, indent=2, ensure_ascii=False))
        if args.report:
            _write_report(args.report.resolve(strict=False), report)
        return EXIT_PASS if passed else EXIT_POLICY_FAILURE
    except (GateToolError, OSError, UnicodeError, ValueError) as exc:
        report = {
            "schemaVersion": 1,
            "gate": "PHASE_2_IMPLEMENTATION_EXIT_GATE",
            "result": "TOOL_ERROR",
            "verdict": "BLOCKED",
            "error": str(exc),
        }
        print(json.dumps(report, indent=2), file=sys.stderr)
        if args.report:
            try:
                _write_report(args.report.resolve(strict=False), report)
            except OSError:
                pass
        return EXIT_TOOL_ERROR


if __name__ == "__main__":
    raise SystemExit(main())
