#!/usr/bin/env python3
"""Run the eight exact Journey H anti-theater mutations fail closed.

Each mutation is applied to a disposable archive of the exact Git candidate.
Credit is awarded only when the named intended detector passes on the baseline,
rejects the mutation, appears in the rejection transcript, and the rejection is
neither a compiler failure nor a process crash.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path, PurePosixPath
from typing import Callable, Sequence


EXIT_PASS = 0
EXIT_POLICY_FAILURE = 1
EXIT_TOOL_ERROR = 2
EXPECTED_MUTATION_COUNT = 8
HEX_40 = re.compile(r"^[0-9a-f]{40}$")
COMPILER_FAILURE = re.compile(
    r"(?im)(?:error\[E\d{4}\]|could not compile|failed to resolve|unresolved import|syntax error)"
)
PROCESS_CRASH = re.compile(
    r"(?im)(?:access violation|stack overflow|segmentation fault|STATUS_[A-Z_]+|terminated by signal|core dumped)"
)


class MutationToolError(RuntimeError):
    """The harness could not establish a trustworthy mutation result."""


@dataclass(frozen=True)
class Detector:
    name: str
    command: tuple[str, ...]
    expected_pattern: re.Pattern[str]


@dataclass(frozen=True)
class Mutation:
    mutation_id: str
    name: str
    target_paths: tuple[str, ...]
    detector: Detector
    apply: Callable[[Path], None]


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest().upper()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def replace_once(root: Path, relative: str, old: str, new: str) -> None:
    path = root / PurePosixPath(relative)
    text = path.read_text(encoding="utf-8")
    if text.count(old) != 1:
        raise MutationToolError(
            f"{relative}: expected exactly one mutation anchor, found {text.count(old)}"
        )
    path.write_text(text.replace(old, new, 1), encoding="utf-8", newline="")


def append_text(root: Path, relative: str, value: str) -> None:
    path = root / PurePosixPath(relative)
    text = path.read_text(encoding="utf-8")
    path.write_text(text + value, encoding="utf-8", newline="")


def add_file(root: Path, relative: str, value: str) -> None:
    path = root / PurePosixPath(relative)
    if path.exists():
        raise MutationToolError(f"mutation target already exists: {relative}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(value, encoding="utf-8", newline="")


def bypass_compiler(root: Path) -> None:
    replace_once(
        root,
        "crates/plc-compiler/src/build.rs",
        "    fn has_blocking(&self) -> bool {\n"
        "        self.diagnostics.iter().any(BuildDiagnostic::is_blocking)\n"
        "    }",
        "    fn has_blocking(&self) -> bool {\n"
        "        false\n"
        "    }",
    )


def canned_diagnostics(root: Path) -> None:
    replace_once(
        root,
        "crates/plc-compiler/src/diagnostic.rs",
        'pub const MALFORMED_TOKEN: Self = Self("EDU-SYN-0001");',
        'pub const MALFORMED_TOKEN: Self = Self("CANNED");',
    )


def scripted_runtime_output(root: Path) -> None:
    replace_once(
        root,
        "crates/plc-runtime/src/controller.rs",
        "outputs.insert(FORMAL_OUTPUT, instruction_input(inputs, FORMAL_INPUT)?);",
        "outputs.insert(FORMAL_OUTPUT, CanonicalValue::I32(41));",
    )


def hide_active_force(root: Path) -> None:
    replace_once(
        root,
        "crates/plc-engineering-wasm/src/system_bridge.rs",
        'write!(output, "{}", read.forces.count).expect("write to String");',
        'write!(output, "{}", 0).expect("write to String");',
    )


def execute_lad_by_coordinate_proxy(root: Path) -> None:
    replace_once(
        root,
        "crates/plc-lad/src/lowering.rs",
        "        for node_id in &analysis.execution_order {\n",
        "        let mut coordinate_order: Vec<_> = network.nodes.keys().copied().collect();\n"
        "        coordinate_order.sort_by_key(|node_id| node_id.get());\n"
        "        for node_id in &coordinate_order {\n",
    )


def add_per_language_runtime(root: Path) -> None:
    add_file(
        root,
        "crates/plc-runtime/src/scl_runtime.rs",
        "pub struct SclRuntime;\n",
    )


def accept_virtual_download_endpoint(root: Path) -> None:
    replace_once(
        root,
        "apps/foundation-shell/src/runtime-wire.ts",
        '    case "runtime.commit-load":\n    case "runtime.go-online":',
        '    case "runtime.commit-load":\n'
        '      exactKeys(value, ["endpoint", "kind"], "runtime operation");\n'
        "      return { kind };\n"
        '    case "runtime.go-online":',
    )


def add_production_network_api(root: Path) -> None:
    append_text(
        root,
        "crates/plc-runtime/src/lib.rs",
        "\n#[allow(dead_code)]\n"
        "fn prohibited_network_api() {\n"
        '    let _ = std::net::TcpStream::connect("127.0.0.1:1");\n'
        "}\n",
    )


def cargo_detector(package: str, arguments: Sequence[str], name: str) -> Detector:
    return Detector(
        name=name,
        command=("{cargo}", "test", "--locked", "-p", package, *arguments),
        expected_pattern=re.compile(rf"(?m)test .*{re.escape(name)} .*FAILED"),
    )


SOURCE_POLICY = Detector(
    name="phase2 complete production capability scan",
    command=("{python}", "-B", "tools/phase2/source_policy.py", "--root", "."),
    expected_pattern=re.compile(r"this pattern is replaced per mutation"),
)


MUTATIONS = (
    Mutation(
        "P2-MUT-01",
        "Bypass compiler blocking-diagnostic admission",
        ("crates/plc-compiler/src/build.rs",),
        cargo_detector(
            "plc-compiler",
            (
                "--test",
                "compiler_pipeline",
                "malformed_and_unsupported_source_never_produce_artifacts",
                "--",
                "--exact",
            ),
            "malformed_and_unsupported_source_never_produce_artifacts",
        ),
        bypass_compiler,
    ),
    Mutation(
        "P2-MUT-02",
        "Return a canned compiler diagnostic code",
        ("crates/plc-compiler/src/diagnostic.rs",),
        cargo_detector(
            "plc-compiler",
            (
                "--test",
                "compiler_pipeline",
                "diagnostic_registry_is_complete_original_and_separates_blocking_from_severity",
                "--",
                "--exact",
            ),
            "diagnostic_registry_is_complete_original_and_separates_blocking_from_severity",
        ),
        canned_diagnostics,
    ),
    Mutation(
        "P2-MUT-03",
        "Script a constant runtime instruction output",
        ("crates/plc-runtime/src/controller.rs",),
        cargo_detector(
            "plc-runtime",
            (
                "--test",
                "invocation_calls",
                "disabled_move_publishes_false_eno_and_suppresses_downstream_effects",
                "--",
                "--exact",
            ),
            "disabled_move_publishes_false_eno_and_suppresses_downstream_effects",
        ),
        scripted_runtime_output,
    ),
    Mutation(
        "P2-MUT-04",
        "Hide an active force in the production read model",
        ("crates/plc-engineering-wasm/src/system_bridge.rs",),
        cargo_detector(
            "plc-engineering-wasm",
            (
                "kernel_bridge::tests::journey_a_runs_through_the_native_kernel_system_bridge",
                "--",
                "--exact",
            ),
            "journey_a_runs_through_the_native_kernel_system_bridge",
        ),
        hide_active_force,
    ),
    Mutation(
        "P2-MUT-05",
        "Execute LAD by a rendered-coordinate proxy instead of authored graph paths",
        ("crates/plc-lad/src/lowering.rs",),
        cargo_detector(
            "plc-lad",
            (
                "--test",
                "lad_contract",
                "authored_branch_path_order_overrides_incidental_edge_and_node_identity_order",
                "--",
                "--exact",
            ),
            "authored_branch_path_order_overrides_incidental_edge_and_node_identity_order",
        ),
        execute_lad_by_coordinate_proxy,
    ),
    Mutation(
        "P2-MUT-06",
        "Create a per-language SCL runtime path",
        ("crates/plc-runtime/src/scl_runtime.rs",),
        Detector(
            SOURCE_POLICY.name,
            SOURCE_POLICY.command,
            re.compile(r'"rule": "per-language production runtime (?:path|identifier)"'),
        ),
        add_per_language_runtime,
    ),
    Mutation(
        "P2-MUT-07",
        "Accept an endpoint-like Virtual Download target",
        ("apps/foundation-shell/src/runtime-wire.ts",),
        Detector(
            SOURCE_POLICY.name,
            SOURCE_POLICY.command,
            re.compile(r'"rule": "endpoint-like Virtual Download target capability"'),
        ),
        accept_virtual_download_endpoint,
    ),
    Mutation(
        "P2-MUT-08",
        "Add a production host-network API",
        ("crates/plc-runtime/src/lib.rs",),
        Detector(
            SOURCE_POLICY.name,
            SOURCE_POLICY.command,
            re.compile(r'"rule": "host network/process capability"'),
        ),
        add_production_network_api,
    ),
)


def run_process(
    command: Sequence[str], cwd: Path, environment: dict[str, str], timeout: int = 600
) -> dict[str, object]:
    started = datetime.now(UTC)
    try:
        completed = subprocess.run(
            list(command),
            cwd=cwd,
            env=environment,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise MutationToolError(f"detector could not execute: {' '.join(command)}: {exc}") from exc
    finished = datetime.now(UTC)
    output = (completed.stdout + completed.stderr).strip()
    return {
        "command": " ".join(command),
        "exitCode": completed.returncode,
        "startedAt": started.isoformat().replace("+00:00", "Z"),
        "finishedAt": finished.isoformat().replace("+00:00", "Z"),
        "output": output,
    }


def git(root: Path, *arguments: str) -> str:
    result = run_process(("git", *arguments), root, os.environ.copy(), timeout=60)
    if result["exitCode"] != 0:
        raise MutationToolError(f"git {' '.join(arguments)} failed: {result['output']}")
    return str(result["output"]).strip()


def resolve_command(
    command: Sequence[str], python: str, cargo: str
) -> tuple[str, ...]:
    return tuple(part.replace("{python}", python).replace("{cargo}", cargo) for part in command)


def safe_extract(archive: Path, destination: Path) -> None:
    with zipfile.ZipFile(archive) as package:
        destination_resolved = destination.resolve(strict=True)
        for member in package.infolist():
            relative = PurePosixPath(member.filename)
            if relative.is_absolute() or ".." in relative.parts:
                raise MutationToolError(f"candidate archive contains unsafe path: {member.filename}")
            target = (destination / relative).resolve(strict=False)
            try:
                target.relative_to(destination_resolved)
            except ValueError as exc:
                raise MutationToolError(
                    f"candidate archive path escapes scratch: {member.filename}"
                ) from exc
        package.extractall(destination)


def restore_targets(
    scratch: Path, originals: dict[str, bytes | None], targets: Sequence[str]
) -> None:
    for relative in targets:
        path = scratch / PurePosixPath(relative)
        original = originals[relative]
        if original is None:
            if path.is_file():
                path.unlink()
            continue
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(original)


def run_mutations(root: Path, candidate_ref: str) -> tuple[dict[str, object], str]:
    if len(MUTATIONS) != EXPECTED_MUTATION_COUNT:
        raise MutationToolError(
            f"Journey H mutation inventory drift: {len(MUTATIONS)}/{EXPECTED_MUTATION_COUNT}"
        )
    commit = git(root, "rev-parse", "--verify", f"{candidate_ref}^{{commit}}").lower()
    tree = git(root, "rev-parse", "--verify", f"{commit}^{{tree}}").lower()
    if not HEX_40.fullmatch(commit) or not HEX_40.fullmatch(tree):
        raise MutationToolError("candidate ref did not resolve to exact commit/tree identities")
    cargo = shutil.which("cargo")
    if cargo is None:
        raise MutationToolError("cargo is unavailable")
    python = sys.executable
    transcript: list[str] = []
    records: list[dict[str, object]] = []
    scratch_name = ""

    with tempfile.TemporaryDirectory(prefix="govs-plc-phase2-mutations-") as temporary:
        temp_root = Path(temporary).resolve(strict=True)
        scratch_name = temp_root.name
        if not scratch_name.startswith("govs-plc-phase2-mutations-"):
            raise MutationToolError(f"refusing unexpected scratch root: {temp_root}")
        archive = temp_root / "candidate.zip"
        scratch = temp_root / "candidate"
        scratch.mkdir()
        archive_result = run_process(
            ("git", "archive", "--format=zip", f"--output={archive}", commit),
            root,
            os.environ.copy(),
            timeout=120,
        )
        if archive_result["exitCode"] != 0 or not archive.is_file():
            raise MutationToolError(f"candidate archive failed: {archive_result['output']}")
        archive_sha256 = sha256_file(archive)
        safe_extract(archive, scratch)

        all_targets = sorted({path for mutation in MUTATIONS for path in mutation.target_paths})
        originals: dict[str, bytes | None] = {}
        for relative in all_targets:
            path = scratch / PurePosixPath(relative)
            originals[relative] = path.read_bytes() if path.is_file() else None

        environment = os.environ.copy()
        environment["CARGO_TARGET_DIR"] = str(root / "target" / "phase2-mutation-target")
        environment["CARGO_NET_OFFLINE"] = "true"
        baseline_cache: dict[tuple[str, ...], dict[str, object]] = {}

        for mutation in MUTATIONS:
            restore_targets(scratch, originals, mutation.target_paths)
            command = resolve_command(mutation.detector.command, python, cargo)
            baseline = baseline_cache.get(command)
            if baseline is None:
                baseline = run_process(command, scratch, environment)
                baseline_cache[command] = baseline
            if baseline["exitCode"] != 0:
                raise MutationToolError(
                    f"baseline detector does not pass for {mutation.mutation_id}: "
                    f"{mutation.detector.name}\n{baseline['output']}"
                )

            before = {
                relative: sha256_file(scratch / PurePosixPath(relative))
                if (scratch / PurePosixPath(relative)).is_file()
                else None
                for relative in mutation.target_paths
            }
            mutation.apply(scratch)
            after = {
                relative: sha256_file(scratch / PurePosixPath(relative))
                if (scratch / PurePosixPath(relative)).is_file()
                else None
                for relative in mutation.target_paths
            }
            mutation_applied = before != after
            mutated = run_process(command, scratch, environment)
            output = str(mutated["output"])
            intended_detected = mutation.detector.expected_pattern.search(output) is not None
            unrelated_failure = COMPILER_FAILURE.search(output) is not None
            crashed = PROCESS_CRASH.search(output) is not None
            detected = (
                mutation_applied
                and mutated["exitCode"] != 0
                and intended_detected
                and not unrelated_failure
                and not crashed
            )
            records.append(
                {
                    "mutationId": mutation.mutation_id,
                    "name": mutation.name,
                    "targetPaths": list(mutation.target_paths),
                    "beforeSha256": before,
                    "afterSha256": after,
                    "mutationApplied": mutation_applied,
                    "expectedDetector": mutation.detector.name,
                    "actualDetector": mutation.detector.name if intended_detected else None,
                    "baselineExitCode": baseline["exitCode"],
                    "detectorExitCode": mutated["exitCode"],
                    "detectorCommand": mutated["command"],
                    "intendedDetectorMatched": intended_detected,
                    "unrelatedFailure": unrelated_failure,
                    "crashed": crashed,
                    "detected": detected,
                    "startedAt": mutated["startedAt"],
                    "finishedAt": mutated["finishedAt"],
                }
            )
            transcript.extend(
                [
                    f"===== {mutation.mutation_id} {mutation.name} =====",
                    f"EXPECTED_DETECTOR={mutation.detector.name}",
                    f"COMMAND={mutated['command']}",
                    f"EXIT_CODE={mutated['exitCode']}",
                    output,
                    "",
                ]
            )
            restore_targets(scratch, originals, mutation.target_paths)
            restored = {
                relative: sha256_file(scratch / PurePosixPath(relative))
                if (scratch / PurePosixPath(relative)).is_file()
                else None
                for relative in mutation.target_paths
            }
            if restored != before:
                raise MutationToolError(
                    f"scratch restoration failed after {mutation.mutation_id}: {restored} != {before}"
                )

    detected_count = sum(record["detected"] is True for record in records)
    overall = detected_count == EXPECTED_MUTATION_COUNT
    result = {
        "schemaVersion": "P2-JOURNEY-H-1",
        "candidateCommit": commit,
        "candidateTree": tree,
        "candidateArchiveSha256": archive_sha256,
        "prescribedMutationCount": EXPECTED_MUTATION_COUNT,
        "intendedMutationDetections": detected_count,
        "overallPassed": overall,
        "unrelatedFailureCredit": False,
        "crashCredit": False,
        "scratchDirectoryName": scratch_name,
        "scratchRemoved": True,
        "mutations": records,
    }
    return result, "\n".join(transcript)


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--candidate-ref", default="HEAD")
    parser.add_argument(
        "--output",
        type=Path,
        default=Path(".phase2-verification/mutations/mutation-results.json"),
    )
    return parser.parse_args(argv)


def output_paths(root: Path, requested: Path) -> tuple[Path, Path]:
    verification_root = (root / ".phase2-verification").resolve(strict=False)
    output = requested if requested.is_absolute() else root / requested
    output = output.resolve(strict=False)
    try:
        output.relative_to(verification_root)
    except ValueError as exc:
        raise MutationToolError(
            "mutation output must remain under .phase2-verification"
        ) from exc
    if output.suffix.lower() != ".json":
        raise MutationToolError("mutation output must be a JSON file")
    return output, output.with_name("mutation-transcript.log")


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        root = args.root.resolve(strict=True)
        output, transcript_path = output_paths(root, args.output)
        result, transcript = run_mutations(root, args.candidate_ref)
        output.parent.mkdir(parents=True, exist_ok=True)
        transcript_path.write_text(transcript, encoding="utf-8", newline="")
        result["transcript"] = {
            "path": transcript_path.relative_to(root).as_posix(),
            "sha256": sha256_file(transcript_path),
        }
        temporary = output.with_suffix(output.suffix + ".tmp")
        temporary.write_text(
            json.dumps(result, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
            newline="",
        )
        temporary.replace(output)
    except (MutationToolError, OSError, UnicodeError, zipfile.BadZipFile) as exc:
        print(json.dumps({"result": "TOOL_ERROR", "error": str(exc)}, indent=2), file=sys.stderr)
        return EXIT_TOOL_ERROR
    print(json.dumps(result, indent=2, sort_keys=True))
    return EXIT_PASS if result["overallPassed"] is True else EXIT_POLICY_FAILURE


if __name__ == "__main__":
    raise SystemExit(main())
