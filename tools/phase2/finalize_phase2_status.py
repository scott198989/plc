"""Build a fail-closed, exact-candidate Phase 2 evidence ledger.

The tracked status ledger is intentionally conservative.  This tool creates an
external final ledger only after an independently generated execution index
contains current PASS evidence for every Appendix H verification, Journey A-H,
and G2-01 through G2-15, and the static coverage audit has no remaining gap.

Requirement evidence is derived from the reviewed Appendix H candidate mapping;
the tool never upgrades a requirement from a test name, file count, or build
result alone.  The ordinary, isolation, and mutation evidence records remain
subject to ``verify_phase2.validate_status_claim`` before any output is written.
"""

from __future__ import annotations

import argparse
import copy
import json
import subprocess
import sys
from pathlib import Path, PurePosixPath
from typing import Any, Mapping, Sequence

import reviewed_requirement_mapping
import verify_phase2


EXIT_PASS = 0
EXIT_INCOMPLETE = 1
EXIT_TOOL_ERROR = 2
READY = "IMPLEMENTED_EVIDENCE_READY"
ISOLATION_CASE = "ISOLATION"
REQUIRED_CASE_KINDS = frozenset({"POSITIVE", "NEGATIVE", "INTEGRATION"})


class FinalizationError(RuntimeError):
    """The supplied candidate or evidence cannot support a final ledger."""


def load_object(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise FinalizationError(f"cannot read {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise FinalizationError(f"{path} must contain one JSON object")
    return value


def candidate_context(
    root: Path,
    candidate_ref: str,
) -> tuple[
    str,
    dict[str, Any],
    dict[str, Any],
    dict[str, Any],
    dict[str, Any],
    set[str],
]:
    requirement_path = "requirements/phase2-requirements.json"
    catalog_path = "requirements/phase2-verification-catalog.json"
    reviewed_mapping_path = reviewed_requirement_mapping.REVIEWED_MAPPING_PATH
    entry_gate_path = "evidence/phase2/P2-00_ENTRY_GATE.json"
    commit = verify_phase2.resolve_commit(root, candidate_ref)
    tree = verify_phase2.resolve_tree(root, commit)
    entries = verify_phase2.git_tree_entries(root, commit)
    blobs = verify_phase2.git_blob_sources(
        root,
        commit,
        [requirement_path, catalog_path, reviewed_mapping_path, entry_gate_path],
    )
    requirements = verify_phase2.load_json_bytes(blobs[requirement_path], requirement_path)
    catalog = verify_phase2.load_json_bytes(blobs[catalog_path], catalog_path)
    reviewed_mapping = verify_phase2.load_json_bytes(
        blobs[reviewed_mapping_path], reviewed_mapping_path
    )
    entry_gate = verify_phase2.load_json_bytes(blobs[entry_gate_path], entry_gate_path)
    directive = requirements.get("directive", {}).get("path")
    if not isinstance(directive, str):
        raise FinalizationError("candidate requirement registry has no directive path")
    directive = PurePosixPath(directive.replace("\\", "/")).as_posix()
    exact_failures, _ = verify_phase2.verify_exact_worktree(root, commit, entry_gate)
    if exact_failures:
        detail = "; ".join(
            f"{failure.code} {failure.subject}: {failure.message}"
            for failure in exact_failures
        )
        raise FinalizationError(f"candidate worktree is not exact: {detail}")
    binding, policy = verify_phase2.candidate_binding(
        root,
        commit,
        tree,
        entries,
        requirement_path,
        catalog_path,
        reviewed_mapping_path,
        directive,
    )
    if not policy.passed:
        detail = "; ".join(
            f"{finding.rule} {finding.path}:{finding.line}"
            for finding in policy.findings
        )
        raise FinalizationError(f"candidate source policy failed: {detail}")
    try:
        reviewed_requirement_mapping.validate_reviewed_mapping(
            reviewed_mapping,
            requirements,
            catalog,
            requirement_registry_sha256=binding["requirementRegistrySha256"],
            verification_catalog_sha256=binding["verificationCatalogSha256"],
            directive_sha256=binding["directiveSha256"],
            expected_requirement_count=verify_phase2.EXPECTED_REQUIREMENT_COUNT,
            expected_verification_count=verify_phase2.EXPECTED_VERIFICATION_COUNT,
        )
    except reviewed_requirement_mapping.ReviewedMappingError as exc:
        raise FinalizationError(f"reviewed requirement mapping is invalid: {exc}") from exc
    return commit, requirements, catalog, reviewed_mapping, binding, set(entries)


def require_gapless_static_audit(
    audit: Mapping[str, Any],
    requirements: Mapping[str, Any],
    catalog: Mapping[str, Any],
    reviewed_mapping: Mapping[str, Any],
    candidate_binding: Mapping[str, Any],
) -> None:
    summary = audit.get("summary")
    if not isinstance(summary, dict):
        raise FinalizationError("static coverage audit has no summary")
    counts = summary.get("verificationClassificationCounts")
    expected_requirements = len(requirements.get("requirements", []))
    expected_verifications = len(catalog.get("verificationRecords", []))
    if not isinstance(counts, dict):
        raise FinalizationError("static coverage audit has no classification counts")
    failures: list[str] = []
    if summary.get("requirementsEnumerated") != expected_requirements:
        failures.append("requirement inventory is incomplete")
    if summary.get("verificationsEnumerated") != expected_verifications:
        failures.append("verification inventory is incomplete")
    if counts.get(READY) != expected_verifications:
        failures.append(f"all {expected_verifications} verifications are not evidence-ready")
    if counts.get("PARTIAL", 0) != 0 or counts.get("MISSING", 0) != 0:
        failures.append("static audit still contains PARTIAL or MISSING rows")
    if summary.get("uncoveredProofClauseCount") != 0:
        failures.append("static audit still contains uncovered proof clauses")
    assessments = audit.get("verificationAssessments")
    if not isinstance(assessments, list) or any(
        not isinstance(record, dict)
        or record.get("classification") != READY
        or record.get("uncoveredProofClauses") != []
        for record in assessments
    ):
        failures.append("one or more static assessment records remain incomplete")
    binding = audit.get("binding")
    if not isinstance(binding, dict):
        failures.append("static audit binding is missing")
    else:
        if binding.get("requirementRegistrySha256") != candidate_binding.get(
            "requirementRegistrySha256"
        ):
            failures.append("static audit requirement-registry binding is stale")
        if binding.get("verificationCatalogSha256") != candidate_binding.get(
            "verificationCatalogSha256"
        ):
            failures.append("static audit verification-catalog binding is stale")
        if binding.get("directiveSha256") != candidate_binding.get("directiveSha256"):
            failures.append("static audit directive binding is stale")
        if binding.get("reviewedRequirementMappingSha256") != candidate_binding.get(
            "reviewedRequirementMappingSha256"
        ):
            failures.append("static audit reviewed-mapping binding is stale")
        if binding.get("reviewedMappingRowsSha256") != reviewed_mapping.get(
            "binding", {}
        ).get("reviewedRowsSha256"):
            failures.append("static audit reviewed-row inventory binding is stale")
    if failures:
        raise FinalizationError("; ".join(failures))


def normalized_subjects(record: Mapping[str, Any]) -> dict[str, list[str]]:
    source = record.get("subjects")
    if not isinstance(source, dict):
        raise FinalizationError(f"evidence {record.get('evidenceId')} has no subjects")
    result: dict[str, list[str]] = {}
    for family in ("requirements", "verifications", "journeys", "gates"):
        values = source.get(family, [])
        if not isinstance(values, list) or any(not isinstance(value, str) for value in values):
            raise FinalizationError(
                f"evidence {record.get('evidenceId')} has invalid {family} subjects"
            )
        result[family] = sorted(set(values))
    return result


def record_case_kinds(record: Mapping[str, Any]) -> set[str]:
    values = record.get("caseKinds", [])
    return {
        value.upper()
        for value in values
        if isinstance(value, str) and value.strip()
    }


def build_ledger(
    *,
    commit: str,
    candidate_tag: str,
    requirements: Mapping[str, Any],
    catalog: Mapping[str, Any],
    reviewed_mapping: Mapping[str, Any],
    binding: Mapping[str, Any],
    audit: Mapping[str, Any],
    execution_index: Mapping[str, Any],
    evidence_base: Path,
    candidate_paths: set[str],
) -> dict[str, Any]:
    require_gapless_static_audit(
        audit, requirements, catalog, reviewed_mapping, binding
    )
    if execution_index.get("schemaVersion") != 1:
        raise FinalizationError("execution index schemaVersion must be 1")
    if execution_index.get("candidateBinding") != binding:
        raise FinalizationError("execution index is not bound to the exact candidate")
    source_records = execution_index.get("evidenceRecords")
    if not isinstance(source_records, list) or not source_records:
        raise FinalizationError("execution index contains no evidence records")
    records: list[dict[str, Any]] = []
    record_by_id: dict[str, dict[str, Any]] = {}
    for ordinal, source in enumerate(source_records):
        if not isinstance(source, dict):
            raise FinalizationError(f"evidenceRecords[{ordinal}] is not an object")
        record = copy.deepcopy(source)
        evidence_id = record.get("evidenceId")
        if not isinstance(evidence_id, str) or not evidence_id:
            raise FinalizationError(f"evidenceRecords[{ordinal}] has no evidenceId")
        if evidence_id in record_by_id:
            raise FinalizationError(f"duplicate evidence ID {evidence_id}")
        subjects = normalized_subjects(record)
        # Requirement claims are always derived below; an execution index may
        # not smuggle pre-credited requirement IDs into the final ledger.
        subjects["requirements"] = []
        record["subjects"] = subjects
        evidence_failures = verify_phase2.validate_evidence_record(record, binding, evidence_base)
        if evidence_failures:
            detail = "; ".join(
                f"{failure.code}: {failure.message}" for failure in evidence_failures
            )
            raise FinalizationError(f"evidence {evidence_id} is invalid: {detail}")
        records.append(record)
        record_by_id[evidence_id] = record

    verification_records = catalog.get("verificationRecords", [])
    verification_ids = {
        str(record["verificationId"])
        for record in verification_records
        if isinstance(record, dict) and isinstance(record.get("verificationId"), str)
    }
    try:
        reviewed_by_requirement = reviewed_requirement_mapping.validate_reviewed_mapping(
            reviewed_mapping,
            requirements,
            catalog,
            requirement_registry_sha256=str(binding.get("requirementRegistrySha256", "")),
            verification_catalog_sha256=str(binding.get("verificationCatalogSha256", "")),
            directive_sha256=str(binding.get("directiveSha256", "")),
            expected_requirement_count=len(requirements.get("requirements", [])),
            expected_verification_count=len(verification_records),
        )
    except reviewed_requirement_mapping.ReviewedMappingError as exc:
        raise FinalizationError(f"reviewed requirement mapping is invalid: {exc}") from exc
    evidence_for: dict[str, dict[str, list[str]]] = {
        family: {} for family in ("verifications", "journeys", "gates")
    }
    for record in records:
        evidence_id = str(record["evidenceId"])
        for family in evidence_for:
            for subject_id in record["subjects"][family]:
                evidence_for[family].setdefault(subject_id, []).append(evidence_id)

    missing_verifications = sorted(verification_ids - set(evidence_for["verifications"]))
    missing_journeys = sorted(set(verify_phase2.JOURNEY_IDS) - set(evidence_for["journeys"]))
    missing_gates = sorted(set(verify_phase2.G2_IDS) - set(evidence_for["gates"]))
    if missing_verifications or missing_journeys or missing_gates:
        raise FinalizationError(
            "execution evidence inventory is incomplete: "
            f"verifications={missing_verifications}, journeys={missing_journeys}, gates={missing_gates}"
        )

    isolation_record_ids = {
        str(record["evidenceId"])
        for record in records
        if str(record.get("kind", "")).upper() == "ISOLATION"
        and ISOLATION_CASE in record_case_kinds(record)
    }
    if not isolation_record_ids:
        raise FinalizationError("no complete isolation evidence record exists")

    requirement_entries: list[dict[str, Any]] = []
    for requirement in requirements.get("requirements", []):
        if not isinstance(requirement, dict) or not isinstance(requirement.get("id"), str):
            raise FinalizationError("candidate requirement inventory contains an invalid record")
        requirement_id = requirement["id"]
        reviewed_row = reviewed_by_requirement.get(requirement_id)
        if reviewed_row is None:
            raise FinalizationError(f"requirement {requirement_id} has no reviewed mapping")
        candidate_verifications = set(reviewed_row["selectedVerificationIds"])
        covered_verifications = sorted(
            candidate_verifications & set(evidence_for["verifications"])
        )
        if not covered_verifications:
            raise FinalizationError(f"requirement {requirement_id} has no executable Appendix H mapping")
        linked_ids = {
            evidence_id
            for verification_id in covered_verifications
            for evidence_id in evidence_for["verifications"][verification_id]
        }
        if str(requirement.get("area", "")) in verify_phase2.ISOLATION_AREAS:
            linked_ids.update(isolation_record_ids)
        observed_cases = {
            case
            for evidence_id in linked_ids
            for case in record_case_kinds(record_by_id[evidence_id])
        }
        missing_cases = REQUIRED_CASE_KINDS - observed_cases
        if missing_cases:
            raise FinalizationError(
                f"requirement {requirement_id} lacks case classes {sorted(missing_cases)}"
            )
        for evidence_id in linked_ids:
            record_by_id[evidence_id]["subjects"]["requirements"].append(requirement_id)
        requirement_entries.append(
            {
                "requirementId": requirement_id,
                "status": "VERIFIED",
                "verificationIds": covered_verifications,
                "mappingStatus": "REVIEWED",
                "mappingDisposition": reviewed_row["disposition"],
                "mappingReviewerRationale": reviewed_row["reviewerRationale"],
                "implementationPaths": [],
                "evidenceIds": sorted(linked_ids),
            }
        )

    for record in records:
        record["subjects"]["requirements"] = sorted(
            set(record["subjects"]["requirements"])
        )

    ledger = {
        "schemaVersion": 1,
        "ledgerKind": "PHASE_2_IMPLEMENTATION_EVIDENCE_STATUS",
        "directive": copy.deepcopy(requirements.get("directive", {})),
        "truthPolicy": {
            "default": "NOT_STARTED",
            "verifiedRequiresCurrentExecutableEvidence": True,
            "passNeverInferredFromBuildOrTestName": True,
            "reviewedMappingDoesNotGrantVerificationCredit": True,
            "nonCreditResults": sorted(verify_phase2.REJECTED_RESULTS),
        },
        "candidate": {"commit": commit, "tag": candidate_tag},
        "requirements": requirement_entries,
        "verifications": [
            {
                "verificationId": verification_id,
                "status": "VERIFIED",
                "evidenceIds": sorted(evidence_for["verifications"][verification_id]),
            }
            for verification_id in sorted(verification_ids)
        ],
        "journeys": [
            {
                "journeyId": journey_id,
                "status": "PASS",
                "evidenceIds": sorted(evidence_for["journeys"][journey_id]),
            }
            for journey_id in verify_phase2.JOURNEY_IDS
        ],
        "gates": [
            {
                "gateId": gate_id,
                "status": "PASS",
                "evidenceIds": sorted(evidence_for["gates"][gate_id]),
            }
            for gate_id in verify_phase2.G2_IDS
        ],
        "evidenceRecords": records,
        "defects": copy.deepcopy(execution_index.get("defects", [])),
    }
    _summary, failures = verify_phase2.validate_status_claim(
        ledger,
        requirements,
        catalog,
        reviewed_mapping,
        binding,
        evidence_base,
        candidate_paths,
    )
    if failures:
        detail = "; ".join(
            f"{failure.code} {failure.subject}: {failure.message}"
            for failure in failures
        )
        raise FinalizationError(f"generated ledger failed the exit-gate validator: {detail}")
    return ledger


def tag_target(root: Path, tag: str) -> str:
    completed = subprocess.run(
        ["git", "rev-parse", f"{tag}^{{commit}}"],
        cwd=root,
        check=False,
        capture_output=True,
        text=True,
        timeout=30,
    )
    if completed.returncode != 0:
        raise FinalizationError(f"candidate tag {tag!r} does not resolve")
    return completed.stdout.strip().lower()


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--candidate-ref", default="HEAD")
    parser.add_argument("--candidate-tag", required=True)
    parser.add_argument("--execution-index", type=Path, required=True)
    parser.add_argument(
        "--static-audit",
        type=Path,
        default=Path("evidence/phase2/PHASE2_COVERAGE_AUDIT.json"),
    )
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def resolve_from(root: Path, value: Path) -> Path:
    return value.resolve(strict=False) if value.is_absolute() else (root / value).resolve(strict=False)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        root = args.root.resolve(strict=True)
        execution_path = resolve_from(root, args.execution_index)
        audit_path = resolve_from(root, args.static_audit)
        output_path = resolve_from(root, args.output)
        if execution_path.parent != output_path.parent:
            raise FinalizationError(
                "execution index and output ledger must share one evidence directory"
            )
        (
            commit,
            requirements,
            catalog,
            reviewed_mapping,
            binding,
            candidate_paths,
        ) = candidate_context(root, args.candidate_ref)
        if tag_target(root, args.candidate_tag) != commit:
            raise FinalizationError("candidate tag does not point to the exact candidate commit")
        ledger = build_ledger(
            commit=commit,
            candidate_tag=args.candidate_tag,
            requirements=requirements,
            catalog=catalog,
            reviewed_mapping=reviewed_mapping,
            binding=binding,
            audit=load_object(audit_path),
            execution_index=load_object(execution_path),
            evidence_base=execution_path.parent,
            candidate_paths=candidate_paths,
        )
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(
            json.dumps(ledger, indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8",
            newline="",
        )
        print(
            "PHASE2_FINAL_STATUS "
            f"candidate={commit} requirements={len(ledger['requirements'])} "
            f"verifications={len(ledger['verifications'])} journeys={len(ledger['journeys'])} "
            f"gates={len(ledger['gates'])} evidence={len(ledger['evidenceRecords'])}"
        )
        print(f"STATUS_LEDGER={output_path}")
        return EXIT_PASS
    except FinalizationError as exc:
        print(f"PHASE2_FINAL_STATUS_BLOCKED {exc}", file=sys.stderr)
        return EXIT_INCOMPLETE
    except (OSError, subprocess.SubprocessError, ValueError, KeyError, TypeError) as exc:
        print(f"PHASE2_FINAL_STATUS_TOOL_ERROR {exc}", file=sys.stderr)
        return EXIT_TOOL_ERROR


if __name__ == "__main__":
    raise SystemExit(main())
