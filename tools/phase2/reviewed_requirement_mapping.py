#!/usr/bin/env python3
"""Validate the explicit, source-bound Phase 2 requirement review artifact.

The extraction catalog contains candidate Appendix-H mappings only.  This
module never promotes those candidates.  It accepts a mapping as reviewed only
when a separate checked-in artifact explicitly selects a nonempty candidate
subset for every exact requirement text and binds the complete source
inventories and source-file hashes.
"""

from __future__ import annotations

import hashlib
import json
import re
from datetime import date
from typing import Any, Mapping, Sequence


REVIEWED_MAPPING_PATH = "requirements/phase2-reviewed-requirement-mapping.json"
REQUIREMENT_REGISTRY_PATH = "requirements/phase2-requirements.json"
VERIFICATION_CATALOG_PATH = "requirements/phase2-verification-catalog.json"
ARTIFACT_KIND = "PHASE_2_REVIEWED_REQUIREMENT_MAPPING"
DISPOSITION = "REVIEWED_CONSERVATIVE"
DISPOSITION_POLICY = "REVIEWED_CONSERVATIVE_CANDIDATE_SELECTION"
RATIONALE_MAX_CHARACTERS = 240
HEX_64 = re.compile(r"^[0-9A-F]{64}$")

TOP_LEVEL_FIELDS = {
    "schemaVersion",
    "artifactKind",
    "reviewAuthority",
    "binding",
    "mappingRows",
}
REVIEW_AUTHORITY_FIELDS = {
    "reviewerId",
    "reviewedOn",
    "dispositionPolicy",
    "rationaleMaxCharacters",
}
BINDING_FIELDS = {
    "directivePath",
    "directiveSha256",
    "requirementRegistryPath",
    "requirementRegistrySha256",
    "verificationCatalogPath",
    "verificationCatalogSha256",
    "requirementCount",
    "verificationCount",
    "requirementInventorySha256",
    "verificationInventorySha256",
    "reviewedRowsSha256",
}
ROW_FIELDS = {
    "requirementId",
    "requirementTextSha256",
    "selectedVerificationIds",
    "disposition",
    "reviewerRationale",
}


class ReviewedMappingError(RuntimeError):
    """The explicit review artifact is missing, stale, or malformed."""


def _canonical_sha256(domain: str, value: Any) -> str:
    payload = json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")
    return hashlib.sha256(domain.encode("ascii") + b"\0" + payload).hexdigest().upper()


def requirement_inventory_sha256(records: Sequence[Mapping[str, Any]]) -> str:
    inventory = sorted(
        (str(record.get("id", "")), str(record.get("textSha256", "")))
        for record in records
    )
    return _canonical_sha256("PES-P2-REQUIREMENT-INVENTORY-1", inventory)


def verification_inventory_sha256(records: Sequence[Mapping[str, Any]]) -> str:
    inventory = sorted(str(record.get("verificationId", "")) for record in records)
    return _canonical_sha256("PES-P2-VERIFICATION-INVENTORY-1", inventory)


def reviewed_rows_sha256(records: Sequence[Mapping[str, Any]]) -> str:
    return _canonical_sha256("PES-P2-REVIEWED-MAPPING-ROWS-1", list(records))


def _require_exact_fields(value: Mapping[str, Any], expected: set[str], subject: str) -> None:
    actual = set(value)
    if actual != expected:
        raise ReviewedMappingError(
            f"{subject} fields drift: missing={sorted(expected - actual)}, "
            f"extra={sorted(actual - expected)}"
        )


def _normalized_hash(value: Any, subject: str) -> str:
    if not isinstance(value, str) or not HEX_64.fullmatch(value.upper()):
        raise ReviewedMappingError(f"{subject} must be a 64-character SHA-256")
    if value != value.upper():
        raise ReviewedMappingError(f"{subject} must use canonical uppercase hex")
    return value


def _unique_requirement_records(
    registry: Mapping[str, Any], expected_count: int
) -> tuple[list[Mapping[str, Any]], dict[str, Mapping[str, Any]]]:
    source = registry.get("requirements")
    if not isinstance(source, list) or len(source) != expected_count:
        raise ReviewedMappingError(
            f"requirement registry must contain exactly {expected_count} records"
        )
    records: list[Mapping[str, Any]] = []
    by_id: dict[str, Mapping[str, Any]] = {}
    for ordinal, value in enumerate(source):
        if not isinstance(value, dict):
            raise ReviewedMappingError(f"requirements[{ordinal}] is not an object")
        requirement_id = value.get("id")
        text_sha256 = value.get("textSha256")
        if not isinstance(requirement_id, str) or not requirement_id:
            raise ReviewedMappingError(f"requirements[{ordinal}] has no requirement ID")
        exact_text = value.get("exactText")
        if not isinstance(exact_text, str) or not exact_text:
            raise ReviewedMappingError(f"requirement {requirement_id} has no exact text")
        normalized_text_sha = _normalized_hash(
            text_sha256, f"requirement {requirement_id} textSha256"
        )
        calculated_text_sha = hashlib.sha256(exact_text.encode("utf-8")).hexdigest().upper()
        if normalized_text_sha != calculated_text_sha:
            raise ReviewedMappingError(f"requirement {requirement_id} exact text hash is stale")
        if requirement_id in by_id:
            raise ReviewedMappingError(f"duplicate extracted requirement {requirement_id}")
        records.append(value)
        by_id[requirement_id] = value
    return records, by_id


def _catalog_candidates(
    catalog: Mapping[str, Any],
    requirement_ids: set[str],
    expected_verification_count: int,
) -> tuple[list[Mapping[str, Any]], dict[str, set[str]]]:
    source = catalog.get("verificationRecords")
    if not isinstance(source, list) or len(source) != expected_verification_count:
        raise ReviewedMappingError(
            f"verification catalog must contain exactly {expected_verification_count} records"
        )
    verification_records: list[Mapping[str, Any]] = []
    verification_ids: set[str] = set()
    for ordinal, value in enumerate(source):
        if not isinstance(value, dict):
            raise ReviewedMappingError(f"verificationRecords[{ordinal}] is not an object")
        verification_id = value.get("verificationId")
        if not isinstance(verification_id, str) or not verification_id:
            raise ReviewedMappingError(
                f"verificationRecords[{ordinal}] has no verification ID"
            )
        if verification_id in verification_ids:
            raise ReviewedMappingError(f"duplicate Appendix-H verification {verification_id}")
        verification_ids.add(verification_id)
        verification_records.append(value)

    skeleton = catalog.get("requirementMappingSkeleton")
    if not isinstance(skeleton, list) or len(skeleton) != len(requirement_ids):
        raise ReviewedMappingError(
            "candidate mapping skeleton does not enumerate the exact requirement inventory"
        )
    candidates: dict[str, set[str]] = {}
    for ordinal, value in enumerate(skeleton):
        if not isinstance(value, dict):
            raise ReviewedMappingError(f"requirementMappingSkeleton[{ordinal}] is not an object")
        requirement_id = value.get("requirementId")
        selected = value.get("candidateVerificationIds")
        if not isinstance(requirement_id, str) or requirement_id not in requirement_ids:
            raise ReviewedMappingError(
                f"requirementMappingSkeleton[{ordinal}] names an unknown requirement"
            )
        if requirement_id in candidates:
            raise ReviewedMappingError(f"duplicate candidate mapping {requirement_id}")
        if (
            not isinstance(selected, list)
            or not selected
            or any(not isinstance(item, str) or not item for item in selected)
            or len(selected) != len(set(selected))
        ):
            raise ReviewedMappingError(
                f"candidate mapping {requirement_id} must contain unique candidate IDs"
            )
        unknown = set(selected) - verification_ids
        if unknown:
            raise ReviewedMappingError(
                f"candidate mapping {requirement_id} names unknown verifications {sorted(unknown)}"
            )
        candidates[requirement_id] = set(selected)
    if set(candidates) != requirement_ids:
        raise ReviewedMappingError("candidate mapping requirement inventory drift")
    return verification_records, candidates


def validate_reviewed_mapping(
    artifact: Mapping[str, Any],
    requirement_registry: Mapping[str, Any],
    verification_catalog: Mapping[str, Any],
    *,
    requirement_registry_sha256: str,
    verification_catalog_sha256: str,
    directive_sha256: str,
    expected_requirement_count: int,
    expected_verification_count: int,
) -> dict[str, dict[str, Any]]:
    """Return exact reviewed rows or raise on any source or inventory drift."""

    if not isinstance(artifact, dict):
        raise ReviewedMappingError("reviewed requirement mapping must be one JSON object")
    _require_exact_fields(artifact, TOP_LEVEL_FIELDS, "reviewed mapping")
    if artifact.get("schemaVersion") != 1 or artifact.get("artifactKind") != ARTIFACT_KIND:
        raise ReviewedMappingError("reviewed mapping schema or artifact kind is unsupported")

    authority = artifact.get("reviewAuthority")
    if not isinstance(authority, dict):
        raise ReviewedMappingError("reviewAuthority must be an object")
    _require_exact_fields(authority, REVIEW_AUTHORITY_FIELDS, "reviewAuthority")
    reviewer = authority.get("reviewerId")
    if not isinstance(reviewer, str) or reviewer != reviewer.strip() or not (3 <= len(reviewer) <= 80):
        raise ReviewedMappingError("reviewerId must be a bounded nonblank identity")
    reviewed_on = authority.get("reviewedOn")
    try:
        date.fromisoformat(reviewed_on)
    except (TypeError, ValueError) as exc:
        raise ReviewedMappingError("reviewedOn must be an ISO calendar date") from exc
    if authority.get("dispositionPolicy") != DISPOSITION_POLICY:
        raise ReviewedMappingError("review disposition policy is unsupported")
    if authority.get("rationaleMaxCharacters") != RATIONALE_MAX_CHARACTERS:
        raise ReviewedMappingError("review rationale bound has drifted")

    requirement_records, requirement_by_id = _unique_requirement_records(
        requirement_registry, expected_requirement_count
    )
    verification_records, candidates = _catalog_candidates(
        verification_catalog,
        set(requirement_by_id),
        expected_verification_count,
    )

    directive = requirement_registry.get("directive")
    if not isinstance(directive, dict) or not isinstance(directive.get("path"), str):
        raise ReviewedMappingError("requirement registry directive binding is missing")
    registry_directive_sha = _normalized_hash(
        directive.get("sha256"), "requirement registry directive sha256"
    )
    expected_directive_sha = _normalized_hash(directive_sha256, "candidate directive sha256")
    if registry_directive_sha != expected_directive_sha:
        raise ReviewedMappingError("directive bytes drift from the requirement registry")

    binding = artifact.get("binding")
    if not isinstance(binding, dict):
        raise ReviewedMappingError("reviewed mapping binding must be an object")
    _require_exact_fields(binding, BINDING_FIELDS, "reviewed mapping binding")
    expected_binding = {
        "directivePath": directive["path"],
        "directiveSha256": expected_directive_sha,
        "requirementRegistryPath": REQUIREMENT_REGISTRY_PATH,
        "requirementRegistrySha256": _normalized_hash(
            requirement_registry_sha256, "candidate requirement registry sha256"
        ),
        "verificationCatalogPath": VERIFICATION_CATALOG_PATH,
        "verificationCatalogSha256": _normalized_hash(
            verification_catalog_sha256, "candidate verification catalog sha256"
        ),
        "requirementCount": expected_requirement_count,
        "verificationCount": expected_verification_count,
        "requirementInventorySha256": requirement_inventory_sha256(requirement_records),
        "verificationInventorySha256": verification_inventory_sha256(verification_records),
    }
    for field, expected in expected_binding.items():
        if binding.get(field) != expected:
            raise ReviewedMappingError(f"reviewed mapping {field} is stale")

    source_rows = artifact.get("mappingRows")
    if not isinstance(source_rows, list) or len(source_rows) != expected_requirement_count:
        raise ReviewedMappingError(
            f"reviewed mapping must contain exactly {expected_requirement_count} rows"
        )
    rows: dict[str, dict[str, Any]] = {}
    observed_order: list[str] = []
    for ordinal, value in enumerate(source_rows):
        if not isinstance(value, dict):
            raise ReviewedMappingError(f"mappingRows[{ordinal}] is not an object")
        _require_exact_fields(value, ROW_FIELDS, f"mappingRows[{ordinal}]")
        requirement_id = value.get("requirementId")
        if not isinstance(requirement_id, str) or requirement_id not in requirement_by_id:
            raise ReviewedMappingError(f"mappingRows[{ordinal}] names an unknown requirement")
        if requirement_id in rows:
            raise ReviewedMappingError(f"duplicate reviewed mapping {requirement_id}")
        if value.get("requirementTextSha256") != requirement_by_id[requirement_id].get(
            "textSha256"
        ):
            raise ReviewedMappingError(f"reviewed mapping text drift for {requirement_id}")
        selected = value.get("selectedVerificationIds")
        if (
            not isinstance(selected, list)
            or not selected
            or any(not isinstance(item, str) or not item for item in selected)
            or len(selected) != len(set(selected))
        ):
            raise ReviewedMappingError(
                f"reviewed mapping {requirement_id} must select unique verification IDs"
            )
        if selected != sorted(selected):
            raise ReviewedMappingError(
                f"reviewed mapping {requirement_id} verification IDs are not canonical"
            )
        noncandidates = set(selected) - candidates[requirement_id]
        if noncandidates:
            raise ReviewedMappingError(
                f"reviewed mapping {requirement_id} selects noncandidates {sorted(noncandidates)}"
            )
        if value.get("disposition") != DISPOSITION:
            raise ReviewedMappingError(f"reviewed mapping {requirement_id} has no disposition")
        rationale = value.get("reviewerRationale")
        if (
            not isinstance(rationale, str)
            or rationale != rationale.strip()
            or not (20 <= len(rationale) <= RATIONALE_MAX_CHARACTERS)
            or any(character in rationale for character in "\r\n\0")
        ):
            raise ReviewedMappingError(
                f"reviewed mapping {requirement_id} has an invalid bounded rationale"
            )
        row = dict(value)
        rows[requirement_id] = row
        observed_order.append(requirement_id)

    if set(rows) != set(requirement_by_id):
        missing = sorted(set(requirement_by_id) - set(rows))
        extra = sorted(set(rows) - set(requirement_by_id))
        raise ReviewedMappingError(
            f"reviewed mapping inventory drift: missing={missing}, extra={extra}"
        )
    if observed_order != sorted(observed_order):
        raise ReviewedMappingError("reviewed mapping rows are not in canonical requirement order")
    expected_rows_sha = reviewed_rows_sha256(source_rows)
    if binding.get("reviewedRowsSha256") != expected_rows_sha:
        raise ReviewedMappingError("reviewed mapping row hash is stale")
    return rows
