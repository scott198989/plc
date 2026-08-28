#!/usr/bin/env python3
"""Collect exact-candidate Phase 2 execution evidence without inferring PASS.

The collector has two deliberately separate stages. ``core`` executes the
complete local regression and Journey H mutation suites and saves reusable,
candidate-bound records under the ignored verification directory. ``assemble``
requires those current records plus a strict PASS isolation run, validates the
isolation closure through the JavaScript counterfactual validator, and emits
the execution index consumed by ``finalize_phase2_status.py``.

Nothing in this tool upgrades requirement truth. The finalizer still derives
requirement credit only from the reviewed Appendix H mapping, and
``verify_phase2.py`` remains the terminal fail-closed validator.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import re
import shutil
import stat
import subprocess
import sys
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path, PurePosixPath
from typing import Any, Mapping, Sequence

import finalize_phase2_status
import verify_phase2


EXIT_PASS = 0
EXIT_INCOMPLETE = 1
EXIT_TOOL_ERROR = 2

CORE_COMMANDS: tuple[tuple[str, ...], ...] = (
    ("pnpm", "check:toolchain"),
    ("pnpm", "requirements:phase2:check"),
    ("pnpm", "audit:phase2:governance"),
    ("pnpm", "coverage:phase2:check"),
    ("pnpm", "test:phase2:gate"),
    ("pnpm", "lint"),
    ("pnpm", "typecheck"),
    ("pnpm", "test:unit"),
    ("pnpm", "build:foundation"),
    ("pnpm", "verify:isolation"),
    ("pnpm", "test:e2e:phase2"),
    ("pnpm", "gate:closure"),
)

CORE_JOURNEYS = tuple("ABCDEF")
CORE_GATES = tuple(
    gate for gate in verify_phase2.G2_IDS if gate not in {"G2-11", "G2-12"}
)
ISOLATION_VERIFICATIONS = tuple(f"VER-ISO-{ordinal:04d}" for ordinal in range(1, 6))
MUTATION_SCHEMA = "P2-JOURNEY-H-1"
EXPECTED_MUTATIONS = 8
MAX_STRICT_RUN_FILES = 256
MAX_STRICT_RUN_FILE_BYTES = 256 * 1024 * 1024
MAX_STRICT_RUN_TOTAL_BYTES = 512 * 1024 * 1024
MAX_MANIFEST_DEPTH = 8
MAX_MANIFEST_SEARCH_FILES = 4096


class CollectionError(RuntimeError):
    """The requested evidence cannot truthfully receive credit."""


@dataclass(frozen=True)
class Execution:
    command: str
    exit_code: int
    started_at: str
    finished_at: str

    def envelope(self, *, production_path: bool) -> dict[str, Any]:
        return {
            "attempts": 1,
            "canned": False,
            "command": self.command,
            "crashed": False,
            "exitCode": self.exit_code,
            "finishedAt": self.finished_at,
            "flaky": False,
            "inconclusive": False,
            "productionPathExercised": production_path,
            "skipped": False,
            "startedAt": self.started_at,
            "unavailable": False,
        }


def iso_now() -> str:
    return datetime.now(UTC).isoformat().replace("+00:00", "Z")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest().upper()


def load_object(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise CollectionError(f"cannot read {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise CollectionError(f"{path} must contain one JSON object")
    return value


def write_json(path: Path, value: Mapping[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(
        json.dumps(value, indent=2, ensure_ascii=False, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="",
    )
    temporary.replace(path)


def resolve_executable(name: str) -> str:
    candidates = (f"{name}.cmd", f"{name}.exe", name) if os.name == "nt" else (name,)
    for candidate in candidates:
        resolved = shutil.which(candidate)
        if resolved is not None:
            return resolved
    raise CollectionError(f"required executable is unavailable: {name}")


def render_command(command: Sequence[str]) -> str:
    return subprocess.list2cmdline(list(command)) if os.name == "nt" else " ".join(command)


def run_logged_commands(
    root: Path,
    commands: Sequence[Sequence[str]],
    log_path: Path,
    *,
    timeout_seconds: int,
) -> Execution:
    started_at = iso_now()
    log_path.parent.mkdir(parents=True, exist_ok=True)
    environment = os.environ.copy()
    environment.update({"CARGO_NET_OFFLINE": "true", "CI": "1", "NO_COLOR": "1"})
    rendered: list[str] = []
    exit_code = 0
    with log_path.open("w", encoding="utf-8", newline="") as log:
        log.write(f"PHASE2_EVIDENCE_STARTED={started_at}\n")
        for raw in commands:
            if not raw:
                raise CollectionError("empty evidence command")
            executable = resolve_executable(raw[0])
            command = (executable, *raw[1:])
            command_text = render_command(command)
            rendered.append(command_text)
            command_started = iso_now()
            log.write(f"\n===== COMMAND {len(rendered):02d} =====\n")
            log.write(f"STARTED_AT={command_started}\nCOMMAND={command_text}\n")
            log.flush()
            print(f"PHASE2_EVIDENCE_COMMAND_START {len(rendered):02d} {render_command(raw)}", flush=True)
            try:
                completed = subprocess.run(
                    command,
                    cwd=root,
                    env=environment,
                    check=False,
                    stdout=log,
                    stderr=subprocess.STDOUT,
                    text=True,
                    encoding="utf-8",
                    errors="replace",
                    timeout=timeout_seconds,
                )
                exit_code = completed.returncode
            except subprocess.TimeoutExpired:
                exit_code = 124
                log.write(f"TIMEOUT_SECONDS={timeout_seconds}\n")
            command_finished = iso_now()
            log.write(f"FINISHED_AT={command_finished}\nEXIT_CODE={exit_code}\n")
            log.flush()
            print(
                f"PHASE2_EVIDENCE_COMMAND_END {len(rendered):02d} exit={exit_code}",
                flush=True,
            )
            if exit_code != 0:
                break
        finished_at = iso_now()
        log.write(f"PHASE2_EVIDENCE_FINISHED={finished_at}\nOVERALL_EXIT_CODE={exit_code}\n")
    return Execution(
        command=" ; ".join(rendered),
        exit_code=exit_code,
        started_at=started_at,
        finished_at=finished_at,
    )


def artifact(base: Path, path: Path, *, kind: str = "LOG") -> dict[str, Any]:
    path = path.resolve(strict=True)
    try:
        relative = path.relative_to(base.resolve(strict=True))
    except ValueError as exc:
        raise CollectionError(f"evidence artifact escapes output directory: {path}") from exc
    return {
        "bytes": path.stat().st_size,
        "kind": kind,
        "path": PurePosixPath(relative).as_posix(),
        "sha256": sha256_file(path),
    }


def _is_reparse_or_symlink(path: Path, metadata: os.stat_result | None = None) -> bool:
    metadata = path.lstat() if metadata is None else metadata
    reparse_flag = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
    return path.is_symlink() or bool(getattr(metadata, "st_file_attributes", 0) & reparse_flag)


def bounded_regular_tree(root: Path) -> list[Path]:
    root = root.absolute()
    try:
        root_metadata = root.lstat()
    except OSError as exc:
        raise CollectionError(f"strict-run evidence root is unavailable: {root}: {exc}") from exc
    if _is_reparse_or_symlink(root, root_metadata) or not stat.S_ISDIR(root_metadata.st_mode):
        raise CollectionError(f"strict-run evidence root is a symlink, reparse point, or non-directory: {root}")

    files: list[Path] = []
    total_bytes = 0

    def visit(directory: Path, depth: int) -> None:
        nonlocal total_bytes
        if depth > MAX_MANIFEST_DEPTH * 4:
            raise CollectionError(f"strict-run evidence directory nesting is excessive: {directory}")
        try:
            entries = sorted(os.scandir(directory), key=lambda entry: entry.name.casefold())
        except OSError as exc:
            raise CollectionError(f"cannot enumerate strict-run evidence directory {directory}: {exc}") from exc
        for entry in entries:
            path = Path(entry.path)
            try:
                metadata = entry.stat(follow_symlinks=False)
            except OSError as exc:
                raise CollectionError(f"cannot inspect strict-run evidence entry {path}: {exc}") from exc
            if entry.is_symlink() or _is_reparse_or_symlink(path, metadata):
                raise CollectionError(f"strict-run evidence contains a symlink or reparse entry: {path}")
            if stat.S_ISDIR(metadata.st_mode):
                visit(path, depth + 1)
                continue
            if not stat.S_ISREG(metadata.st_mode):
                raise CollectionError(f"strict-run evidence contains a non-regular entry: {path}")
            if metadata.st_size <= 0 or metadata.st_size > MAX_STRICT_RUN_FILE_BYTES:
                raise CollectionError(f"strict-run evidence file size is non-credit: {path}")
            files.append(path)
            total_bytes += metadata.st_size
            if len(files) > MAX_STRICT_RUN_FILES or total_bytes > MAX_STRICT_RUN_TOTAL_BYTES:
                raise CollectionError("strict-run evidence exceeds the bounded file-count or byte budget")

    visit(root, 0)
    return files


def referenced_manifest_digests(proof: Mapping[str, Any]) -> dict[str, list[str]]:
    references: dict[str, list[str]] = {}

    def add(value: Any, subject: str) -> None:
        if not isinstance(value, str) or re.fullmatch(r"[A-F0-9]{64}", value) is None:
            raise CollectionError(f"{subject} has no valid evidence manifest SHA-256")
        references.setdefault(value, []).append(subject)

    platforms = proof.get("platformConfigurations")
    topology = proof.get("liveLanTopologyVariation")
    backing = proof.get("fixedNativeBackingAttestation")
    if not isinstance(platforms, list) or not isinstance(topology, dict) or not isinstance(backing, dict):
        raise CollectionError("isolation proof has no structured manifest references")
    for record in platforms:
        if not isinstance(record, dict):
            raise CollectionError("isolation platform manifest reference is malformed")
        add(record.get("evidenceManifestSha256"), f"configuration:{record.get('configurationId')}")
    scenarios = topology.get("scenarios")
    if not isinstance(scenarios, list):
        raise CollectionError("isolation topology manifest references are malformed")
    for record in scenarios:
        if not isinstance(record, dict):
            raise CollectionError("isolation topology scenario is malformed")
        add(record.get("evidenceManifestSha256"), f"topology:{record.get('scenarioId')}")
    add(backing.get("evidenceManifestSha256"), "fixed-native-backing")
    if not references:
        raise CollectionError("isolation proof references no evidence manifests")
    return references


def _safe_manifest_target(manifest: Path, value: Any) -> Path:
    if (
        not isinstance(value, str)
        or not value
        or len(value.encode("utf-8")) > 4096
        or "\\" in value
    ):
        raise CollectionError(f"evidence manifest {manifest} contains an invalid relative path")
    relative = PurePosixPath(value)
    if (
        relative.is_absolute()
        or relative.as_posix() != value
        or any(part in {"", ".", ".."} for part in relative.parts)
    ):
        raise CollectionError(f"evidence manifest {manifest} contains an escaping relative path: {value}")
    base = manifest.parent.absolute()
    target = base.joinpath(*relative.parts)
    current = base
    for part in relative.parts:
        current = current / part
        try:
            metadata = current.lstat()
        except OSError as exc:
            raise CollectionError(f"evidence manifest {manifest} omits listed file {value}") from exc
        if _is_reparse_or_symlink(current, metadata):
            raise CollectionError(f"evidence manifest {manifest} lists a symlink or reparse entry: {value}")
    if not stat.S_ISREG(target.lstat().st_mode):
        raise CollectionError(f"evidence manifest {manifest} lists a non-regular file: {value}")
    return target


def collect_manifest_bundle(manifest: Path) -> set[Path]:
    collected: set[Path] = set()
    validated: set[Path] = set()

    def visit(current: Path, stack: tuple[Path, ...]) -> None:
        current = current.absolute()
        if current in stack:
            raise CollectionError(f"evidence manifest cycle detected at {current}")
        if current in validated:
            return
        if len(stack) >= MAX_MANIFEST_DEPTH:
            raise CollectionError(f"evidence manifest recursion exceeds {MAX_MANIFEST_DEPTH}: {current}")
        try:
            current_metadata = current.lstat()
        except OSError as exc:
            raise CollectionError(f"referenced evidence manifest is unavailable: {current}") from exc
        if (
            _is_reparse_or_symlink(current, current_metadata)
            or not stat.S_ISREG(current_metadata.st_mode)
            or current_metadata.st_size <= 0
            or current_metadata.st_size > MAX_STRICT_RUN_FILE_BYTES
        ):
            raise CollectionError(f"referenced evidence manifest is not a bounded regular file: {current}")
        manifest_object = load_object(current)
        evidence_kind = manifest_object.get("evidenceKind")
        records = manifest_object.get("evidenceFiles")
        if (
            not isinstance(evidence_kind, str)
            or not evidence_kind.endswith("_MANIFEST")
            or not isinstance(records, list)
            or not records
            or len(records) > MAX_STRICT_RUN_FILES
        ):
            raise CollectionError(f"referenced evidence manifest is malformed or empty: {current}")
        relative_paths: set[str] = set()
        collected.add(current)
        for ordinal, record in enumerate(records):
            if not isinstance(record, dict) or set(record) != {"bytes", "path", "sha256"}:
                raise CollectionError(f"evidence manifest {current} row {ordinal} is not exact")
            relative_path = record.get("path")
            canonical_key = relative_path.casefold() if isinstance(relative_path, str) else ""
            if not isinstance(relative_path, str) or canonical_key in relative_paths:
                raise CollectionError(f"evidence manifest {current} contains a duplicate path")
            relative_paths.add(canonical_key)
            target = _safe_manifest_target(current, relative_path)
            observed_bytes = target.stat().st_size
            if (
                type(record.get("bytes")) is not int
                or record.get("bytes") != observed_bytes
                or observed_bytes <= 0
                or observed_bytes > MAX_STRICT_RUN_FILE_BYTES
                or not isinstance(record.get("sha256"), str)
                or record.get("sha256") != sha256_file(target)
            ):
                raise CollectionError(f"evidence manifest {current} has stale bytes or SHA-256 for {relative_path}")
            collected.add(target)
            if target.suffix.casefold() == ".json":
                try:
                    nested = load_object(target)
                except CollectionError:
                    if "manifest" in target.name.casefold():
                        raise
                    nested = {}
                if "manifest" in target.name.casefold() or "evidenceFiles" in nested:
                    visit(target, (*stack, current))
        validated.add(current)

    visit(manifest, ())
    total_bytes = sum(path.stat().st_size for path in collected)
    if len(collected) > MAX_STRICT_RUN_FILES or total_bytes > MAX_STRICT_RUN_TOTAL_BYTES:
        raise CollectionError("referenced manifest bundle exceeds the bounded file-count or byte budget")
    return collected


def _manifest_candidates(root: Path, excluded: tuple[Path, ...]) -> list[Path]:
    candidates: list[Path] = []
    visited_files = 0
    root = root.absolute()

    def visit(directory: Path) -> None:
        nonlocal visited_files
        if any(directory == item or item in directory.parents for item in excluded):
            return
        try:
            entries = sorted(os.scandir(directory), key=lambda entry: entry.name.casefold())
        except OSError:
            return
        for entry in entries:
            path = Path(entry.path)
            try:
                metadata = entry.stat(follow_symlinks=False)
            except OSError:
                continue
            if entry.is_symlink() or _is_reparse_or_symlink(path, metadata):
                continue
            if stat.S_ISDIR(metadata.st_mode):
                visit(path)
                continue
            if not stat.S_ISREG(metadata.st_mode):
                continue
            visited_files += 1
            if visited_files > MAX_MANIFEST_SEARCH_FILES:
                raise CollectionError("verification manifest search exceeds its bounded file budget")
            if path.suffix.casefold() == ".json" and 0 < metadata.st_size <= 16 * 1024 * 1024:
                candidates.append(path)

    if root.is_dir():
        visit(root)
    return candidates


def materialize_referenced_manifests(
    verification_root: Path,
    copied: Path,
    proof: Mapping[str, Any],
    output: Path,
) -> list[Path]:
    references = referenced_manifest_digests(proof)

    def copied_matches() -> dict[str, list[Path]]:
        matches: dict[str, list[Path]] = {digest: [] for digest in references}
        for candidate in bounded_regular_tree(copied):
            if candidate.suffix.casefold() != ".json":
                continue
            digest = sha256_file(candidate)
            if digest in matches:
                matches[digest].append(candidate)
        return matches

    matches = copied_matches()
    missing = {digest for digest, paths in matches.items() if not paths}
    if missing:
        sources: dict[str, list[Path]] = {digest: [] for digest in missing}
        for candidate in _manifest_candidates(verification_root, (output.absolute(), copied.absolute())):
            digest = sha256_file(candidate)
            if digest in sources:
                sources[digest].append(candidate)
        for digest in sorted(missing):
            candidates = sources[digest]
            if len(candidates) != 1:
                detail = "missing" if not candidates else "ambiguous"
                raise CollectionError(f"referenced evidence manifest {digest} is {detail}")
            source = candidates[0]
            bundle_files = collect_manifest_bundle(source)
            destination_root = copied / "referenced-manifests" / digest.casefold()
            if destination_root.exists():
                raise CollectionError(f"referenced manifest destination already exists: {destination_root}")
            for source_file in sorted(bundle_files):
                try:
                    relative = source_file.relative_to(source.parent)
                except ValueError as exc:
                    raise CollectionError(f"referenced manifest bundle escapes its root: {source_file}") from exc
                destination = destination_root / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(source_file, destination)
            collect_manifest_bundle(destination_root / source.name)
        matches = copied_matches()

    for digest, paths in matches.items():
        if len(paths) != 1:
            raise CollectionError(f"referenced evidence manifest {digest} is not present exactly once")
        collect_manifest_bundle(paths[0])
    copied_files = bounded_regular_tree(copied)
    for candidate in copied_files:
        if candidate.suffix.casefold() != ".json":
            continue
        try:
            manifest_object = load_object(candidate)
        except CollectionError:
            if "manifest" in candidate.name.casefold():
                raise
            continue
        if isinstance(manifest_object.get("evidenceFiles"), list):
            collect_manifest_bundle(candidate)
    return bounded_regular_tree(copied)


def strict_run_artifact_kind(path: Path) -> str:
    name = path.name.casefold()
    if "manifest" in name:
        return "MANIFEST"
    if name in {"closure-evidence-input.json", "counterfactual-isolation.json"}:
        return "REPORT"
    return "LOG"


def validate_record(record: Mapping[str, Any], binding: Mapping[str, Any], base: Path) -> None:
    failures = verify_phase2.validate_evidence_record(record, binding, base)
    if failures:
        details = "; ".join(f"{item.code}: {item.message}" for item in failures)
        raise CollectionError(f"evidence {record.get('evidenceId')} is invalid: {details}")


def core_record(
    *,
    binding: Mapping[str, Any],
    catalog: Mapping[str, Any],
    execution: Execution,
    output: Path,
    log_path: Path,
) -> dict[str, Any]:
    if execution.exit_code != 0:
        raise CollectionError(f"core Phase 2 evidence suite failed with exit {execution.exit_code}")
    verification_ids = sorted(
        str(item["verificationId"])
        for item in catalog.get("verificationRecords", [])
        if isinstance(item, dict)
        and isinstance(item.get("verificationId"), str)
        and item["verificationId"] not in ISOLATION_VERIFICATIONS
    )
    expected = verify_phase2.EXPECTED_VERIFICATION_COUNT - len(ISOLATION_VERIFICATIONS)
    if len(verification_ids) != expected:
        raise CollectionError(
            f"core verification inventory drift: expected {expected}, got {len(verification_ids)}"
        )
    record = {
        "artifacts": [artifact(output, log_path)],
        "binding": copy.deepcopy(binding),
        "caseKinds": ["INTEGRATION", "NEGATIVE", "POSITIVE"],
        "evidenceId": "P2-EXEC-CORE-001",
        "execution": execution.envelope(production_path=True),
        "kind": "EXECUTION",
        "result": "PASS",
        "subjects": {
            "gates": list(CORE_GATES),
            "journeys": list(CORE_JOURNEYS),
            "requirements": [],
            "verifications": verification_ids,
        },
    }
    validate_record(record, binding, output)
    return record


def run_mutation_suite(root: Path, output: Path) -> tuple[list[dict[str, Any]], Execution]:
    report_path = output / "mutations" / "mutation-results.json"
    log_path = output / "logs" / "mutation-suite.log"
    python = resolve_executable("node")
    command = (
        python,
        "tools/phase2/run_pinned_python.mjs",
        "-B",
        "tools/phase2/run_phase2_mutations.py",
        "--root",
        ".",
        "--candidate-ref",
        "HEAD",
        "--output",
        report_path.relative_to(root).as_posix(),
    )
    execution = run_logged_commands(root, (command,), log_path, timeout_seconds=1800)
    if execution.exit_code != 0:
        raise CollectionError(f"Journey H mutation suite failed with exit {execution.exit_code}")
    report = load_object(report_path)
    transcript_path = report_path.with_name("mutation-transcript.log")
    transcript = report.get("transcript")
    if (
        not transcript_path.is_file()
        or not isinstance(transcript, dict)
        or transcript.get("path") != transcript_path.relative_to(root).as_posix()
        or transcript.get("sha256") != sha256_file(transcript_path)
    ):
        raise CollectionError("Journey H mutation transcript binding is missing or stale")
    return (
        build_mutation_records(
            report,
            execution,
            output,
            log_path,
            report_path=report_path,
            transcript_path=transcript_path,
        ),
        execution,
    )


def build_mutation_records(
    report: Mapping[str, Any],
    execution: Execution,
    output: Path,
    log_path: Path,
    *,
    report_path: Path,
    transcript_path: Path,
) -> list[dict[str, Any]]:
    if (
        report.get("schemaVersion") != MUTATION_SCHEMA
        or report.get("overallPassed") is not True
        or report.get("prescribedMutationCount") != EXPECTED_MUTATIONS
        or report.get("intendedMutationDetections") != EXPECTED_MUTATIONS
        or report.get("unrelatedFailureCredit") is not False
        or report.get("crashCredit") is not False
        or report.get("scratchRemoved") is not True
    ):
        raise CollectionError("Journey H mutation report is incomplete or non-credit")
    mutations = report.get("mutations")
    if not isinstance(mutations, list) or len(mutations) != EXPECTED_MUTATIONS:
        raise CollectionError("Journey H mutation inventory is incomplete")
    identifiers = {item.get("mutationId") for item in mutations if isinstance(item, dict)}
    expected_ids = {f"P2-MUT-{ordinal:02d}" for ordinal in range(1, EXPECTED_MUTATIONS + 1)}
    if identifiers != expected_ids:
        raise CollectionError(f"Journey H mutation IDs drifted: {sorted(str(item) for item in identifiers)}")
    records: list[dict[str, Any]] = []
    shared_artifacts = [
        artifact(output, log_path),
        artifact(output, transcript_path),
        artifact(output, report_path, kind="REPORT"),
    ]
    for item in sorted(mutations, key=lambda value: str(value.get("mutationId"))):
        if (
            not isinstance(item, dict)
            or item.get("detected") is not True
            or item.get("mutationApplied") is not True
            or item.get("baselineExitCode") != 0
            or not isinstance(item.get("detectorExitCode"), int)
            or item.get("detectorExitCode") == 0
            or item.get("expectedDetector") != item.get("actualDetector")
            or item.get("unrelatedFailure") is not False
            or item.get("crashed") is not False
        ):
            raise CollectionError(f"mutation {item.get('mutationId')} lacks intended-detector proof")
        record = {
            "actualDetector": item["actualDetector"],
            "artifacts": copy.deepcopy(shared_artifacts),
            "binding": {},  # Bound by the caller after exact candidate resolution.
            "caseKinds": ["INTEGRATION", "NEGATIVE"],
            "detectorExitCode": item["detectorExitCode"],
            "evidenceId": f"P2-EXEC-{item['mutationId']}",
            "execution": execution.envelope(production_path=True),
            "expectedDetector": item["expectedDetector"],
            "kind": "MUTATION",
            "mutationId": item["mutationId"],
            "result": "PASS",
            "subjects": {
                "gates": ["G2-11"],
                "journeys": ["H"],
                "requirements": [],
                "verifications": [],
            },
        }
        records.append(record)
    return records


def bind_and_validate_mutations(
    records: list[dict[str, Any]], binding: Mapping[str, Any], output: Path
) -> list[dict[str, Any]]:
    for record in records:
        record["binding"] = copy.deepcopy(binding)
        validate_record(record, binding, output)
    return records


def candidate_identity(candidate: Mapping[str, Any], field: str) -> str | None:
    value = candidate.get(field)
    if isinstance(value, str):
        return value.lower()
    aliases = {"commit": "candidateCommit", "tree": "candidateTree"}
    value = candidate.get(aliases[field])
    return value.lower() if isinstance(value, str) else None


def isolation_record(
    *,
    root: Path,
    output: Path,
    isolation_directory: Path,
    binding: Mapping[str, Any],
) -> dict[str, Any]:
    isolation_directory = isolation_directory.absolute()
    bounded_regular_tree(isolation_directory)
    isolation_directory = isolation_directory.resolve(strict=True)
    report_path = isolation_directory / "counterfactual-isolation.json"
    manifest_path = isolation_directory / "evidence-manifest.json"
    closure_path = isolation_directory / "closure-evidence-input.json"
    report = load_object(report_path)
    manifest = load_object(manifest_path)
    candidate = report.get("candidate")
    if not isinstance(candidate, dict):
        raise CollectionError("isolation report has no exact candidate binding")
    manifest_candidate = manifest.get("candidate")
    completeness = report.get("completeness")
    workflow = report.get("workflow")
    assertions = report.get("assertions")
    if (
        report.get("result") != "PASS"
        or not isinstance(completeness, dict)
        or completeness.get("complete") is not True
        or not isinstance(workflow, dict)
        or workflow.get("completed") is not True
        or not isinstance(assertions, dict)
        or assertions.get("zeroExternalAttempts") is not True
        or candidate_identity(candidate, "commit") != binding.get("candidateCommit")
        or candidate_identity(candidate, "tree") != binding.get("candidateTree")
        or candidate.get("isolationApprovalDecisionId") != binding.get("isolationApprovalDecisionId")
        or candidate.get("isolationApprovalSha256") != binding.get("isolationApprovalSha256")
        or candidate.get("exact") is not True
        or not isinstance(manifest_candidate, dict)
        or manifest_candidate != candidate
        or manifest.get("result") != "PASS"
        or manifest.get("complete") is not True
    ):
        raise CollectionError("strict isolation run is incomplete, stale, or non-credit")
    harness = report.get("harness")
    invocation = harness.get("invocation") if isinstance(harness, dict) else None
    if not isinstance(invocation, dict):
        raise CollectionError("strict isolation report does not preserve its exact invocation")
    executable = invocation.get("executable")
    argv = invocation.get("argv")
    working_directory = invocation.get("workingDirectory")
    if (
        not isinstance(executable, str)
        or not executable.strip()
        or not isinstance(argv, list)
        or not argv
        or any(not isinstance(value, str) or not value for value in argv)
        or not isinstance(working_directory, str)
        or not working_directory.strip()
    ):
        raise CollectionError("strict isolation invocation tuple is malformed")
    command = f"cwd={working_directory} {render_command((executable, *argv))}"

    proof_path = output / "isolation" / "isolation-gate-fields.json"
    transform_log = output / "logs" / "isolation-transform.log"
    transform = (
        resolve_executable("node"),
        "tools/phase2/transform_isolation_closure.mjs",
        "--approval-decision-id",
        str(binding["isolationApprovalDecisionId"]),
        "--approval-sha256",
        str(binding["isolationApprovalSha256"]),
        "--input",
        str(closure_path),
        "--output",
        str(proof_path),
        "--candidate-commit",
        str(binding["candidateCommit"]),
        "--candidate-tree",
        str(binding["candidateTree"]),
    )
    transformed = run_logged_commands(root, (transform,), transform_log, timeout_seconds=120)
    if transformed.exit_code != 0:
        raise CollectionError("strict isolation closure transform failed")
    proof = load_object(proof_path)

    copied = output / "isolation" / "strict-run"
    try:
        copied.resolve(strict=False).relative_to(output.resolve(strict=True))
    except ValueError as exc:
        raise CollectionError("refusing unsafe isolation evidence replacement target") from exc
    if copied.exists():
        shutil.rmtree(copied)
    shutil.copytree(isolation_directory, copied, symlinks=True)
    copied_files = materialize_referenced_manifests(
        root / ".phase2-verification",
        copied,
        proof,
        output,
    )
    artifacts = [
        artifact(
            output,
            file,
            kind=strict_run_artifact_kind(file),
        )
        for file in copied_files
    ]
    artifacts.append(artifact(output, proof_path, kind="REPORT"))
    artifacts.append(artifact(output, transform_log))

    execution = Execution(
        command=command,
        exit_code=0,
        started_at=str(report.get("startedAt", "")),
        finished_at=str(report.get("completedAt", "")),
    )
    record: dict[str, Any] = {
        "artifacts": artifacts,
        "binding": copy.deepcopy(binding),
        "caseKinds": ["INTEGRATION", "ISOLATION", "NEGATIVE", "POSITIVE"],
        "evidenceId": "P2-EXEC-ISOLATION-001",
        "execution": execution.envelope(production_path=True),
        "instrumentationStatus": "COMPLETE",
        "kind": "ISOLATION",
        "result": "PASS",
        "subjects": {
            "gates": ["G2-12"],
            "journeys": ["G"],
            "requirements": [],
            "verifications": list(ISOLATION_VERIFICATIONS),
        },
        "zeroExternalAttempts": True,
        **proof,
    }
    validate_record(record, binding, output)
    return record


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--candidate-ref", default="HEAD")
    parser.add_argument("--mode", choices=("all", "assemble", "core"), default="all")
    parser.add_argument("--output-dir", type=Path)
    parser.add_argument("--isolation-dir", type=Path)
    parser.add_argument("--reuse", action="store_true")
    return parser.parse_args(argv)


def resolve_inside_verification_root(root: Path, value: Path | None, commit: str) -> Path:
    verification_root = (root / ".phase2-verification").resolve(strict=False)
    output = (
        verification_root / f"P2-FINAL-{commit}"
        if value is None
        else (value if value.is_absolute() else root / value).resolve(strict=False)
    )
    try:
        output.relative_to(verification_root)
    except ValueError as exc:
        raise CollectionError("evidence output must remain under .phase2-verification") from exc
    return output


def load_reusable_records(
    output: Path, binding: Mapping[str, Any]
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    core = load_object(output / "core-record.json")
    mutation_container = load_object(output / "mutation-records.json")
    mutations = mutation_container.get("evidenceRecords")
    if not isinstance(mutations, list) or any(not isinstance(item, dict) for item in mutations):
        raise CollectionError("reusable mutation record inventory is invalid")
    validate_record(core, binding, output)
    for record in mutations:
        validate_record(record, binding, output)
    return core, mutations


def collect_core(
    root: Path,
    output: Path,
    binding: Mapping[str, Any],
    catalog: Mapping[str, Any],
    *,
    reuse: bool,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    if reuse:
        return load_reusable_records(output, binding)
    core_log = output / "logs" / "core-suite.log"
    execution = run_logged_commands(root, CORE_COMMANDS, core_log, timeout_seconds=3600)
    core = core_record(
        binding=binding,
        catalog=catalog,
        execution=execution,
        output=output,
        log_path=core_log,
    )
    mutations, _mutation_execution = run_mutation_suite(root, output)
    mutations = bind_and_validate_mutations(mutations, binding, output)
    write_json(output / "core-record.json", core)
    write_json(output / "mutation-records.json", {"evidenceRecords": mutations})
    return core, mutations


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        root = args.root.resolve(strict=True)
        commit, _requirements, catalog, binding, _candidate_paths = (
            finalize_phase2_status.candidate_context(root, args.candidate_ref)
        )
        output = resolve_inside_verification_root(root, args.output_dir, commit)
        output.mkdir(parents=True, exist_ok=True)
        core, mutations = collect_core(
            root,
            output,
            binding,
            catalog,
            reuse=args.reuse or args.mode == "assemble",
        )
        if args.mode == "core":
            print(f"PHASE2_CORE_EVIDENCE_PASS candidate={commit} output={output}")
            return EXIT_PASS
        if args.isolation_dir is None:
            raise CollectionError("assemble/all mode requires --isolation-dir with a strict PASS run")
        isolation_dir = (
            args.isolation_dir
            if args.isolation_dir.is_absolute()
            else (root / args.isolation_dir).resolve(strict=False)
        )
        isolation = isolation_record(
            root=root,
            output=output,
            isolation_directory=isolation_dir,
            binding=binding,
        )
        records = [core, *mutations, isolation]
        index = {
            "candidateBinding": binding,
            "defects": [],
            "evidenceRecords": records,
            "schemaVersion": 1,
        }
        for record in records:
            validate_record(record, binding, output)
        index_path = output / "execution-index.json"
        write_json(index_path, index)
        print(
            f"PHASE2_EXECUTION_INDEX_PASS candidate={commit} "
            f"records={len(records)} output={index_path}"
        )
        return EXIT_PASS
    except CollectionError as exc:
        print(f"PHASE2_EVIDENCE_BLOCKED {exc}", file=sys.stderr)
        return EXIT_INCOMPLETE
    except (
        OSError,
        subprocess.SubprocessError,
        ValueError,
        KeyError,
        TypeError,
    ) as exc:
        print(f"PHASE2_EVIDENCE_TOOL_ERROR {exc}", file=sys.stderr)
        return EXIT_TOOL_ERROR


if __name__ == "__main__":
    raise SystemExit(main())
