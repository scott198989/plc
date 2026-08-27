# Original Simulator-Native Project and Archive Format

- Status: Proposed Phase 1 Architecture Invariant
- Decision ID: ADR-0002
- Date: 2026-08-27
- Approval: Not yet recorded; directive-mandated boundary documented for review before implementation depends on it
- Scope: Project persistence, archives, import/export, migration, and recovery
- Supersedes: None

## Context

The simulator needs durable projects, archives, classroom exchange, recovery, migrations, and reproducible builds. A vendor format or a format intended for physical deployment would create compatibility, intellectual-property, and safety risks. An opaque executable container would also prevent deterministic validation, safe migration, and clean-room evidence.

Phase 1 establishes the format boundary but deliberately does not invent the complete Phase 2 object schema, migration/downgrade policy, resource limits, or compatibility rules. Those observable details remain open under OQ-0005 and the mandatory stop rules in `PES-DEC-0002`.

This decision implements `PES-PRJ-0001`-`PES-PRJ-0007`, `PES-PROF-0003`-`PES-PROF-0005`, `PES-SEC-0002`, `PES-SEC-0012`-`PES-SEC-0016`, `PES-SEC-0021`-`PES-SEC-0023`, `PES-ARC-0004`-`PES-ARC-0016`, `PES-DOC-0003`, and `PES-CRM-0004`-`PES-CRM-0005`.

## Decision

The product uses only original, brand-neutral, simulator-native project and archive formats.

- Project package extension: `.vlabproj` (provisional until an approved product-name decision replaces it).
- Archive extension: `.vlabarchive` (provisional on the same basis).
- Both are non-executable document formats.
- Canonical semantic content is versioned UTF-8 data using explicit schemas.
- Binary-neutral, simulator-owned assets may be included only when a normative requirement permits the asset type and the manifest inventories and hashes it.
- A ZIP container is permitted only with the archive defenses and deterministic limits below. ZIP is a storage envelope, not the semantic format.

The complete internal layout and domain schema are not decided by this ADR. They must be specified in Phase 2 before implementation depends on them.

## Required package manifest

Every project and archive contains a manifest with at least:

| Field | Binding meaning |
|---|---|
| Schema version | Exact project representation version, independent from TrainingProfile version |
| TrainingProfile ID and version | Pinned semantic capability manifest selected by the project |
| Object-index version | Version of the stable-ID/object index contract |
| Required capabilities | Declarative simulator capabilities needed to interpret the package; never host capabilities |
| File inventory | Canonical list of every package entry and its declared role |
| SHA-256 hashes | Integrity hash for every inventoried entry |
| Creation application version | Simulator version that created the package |
| Migration history | Ordered, explicit record of applied schema migrations |

The selected TrainingProfile and capability-manifest version are pinned. Opening a project does not silently change semantics (`PES-PROF-0004`). Project schema version and TrainingProfile version remain independent (`PES-PROF-0003`).

## Semantic identity and references

- Every semantically referenceable object uses an immutable UUID under `PES-ARC-0004`-`PES-ARC-0007`.
- Names, addresses, paths, block numbers, array positions, and source coordinates are never identity.
- Deletion retains tombstones while live references, undo, migration, diagnostics, audit, or snapshots need them.
- Unresolved references persist explicitly with target UUID and reference kind; loaders cannot erase or retarget them silently.
- Import detects UUID collisions. Ambiguous merge is rejected. Explicit remapping is allowed only for a defined “create independent objects” operation and must be complete, deterministic, and traceable.
- Persistence cannot construct a trusted-valid state behind the domain model. Deserialized content enters through validation and typed domain restoration/migration contracts.

## Canonicalization and deterministic persistence

The later schema specification must define canonical ordering, encoding, number/string representation, normalization, and hash scope sufficiently for deterministic comparison and reproducible verification. Until those rules are approved, no implementation may claim byte-for-byte canonical persistence.

Saving is an atomic bounded document operation. A failed save or migration cannot leave a package that appears valid but is partially updated. Recovery journals and temporary material remain application-controlled and cannot introduce an executable or general filesystem surface.

Unknown, corrupt, oversized, ambiguous, or hash-mismatched content is visible and fails closed. It is never silently discarded, rewritten as a valid default, or reported as success (`PES-PRJ-0005`, `PES-SEC-0022`).

## Untrusted-input controls

Every project, archive, CSV/JSON interchange file, image, library, lesson, and scenario is untrusted. Import must enforce:

1. explicit supported schema and version validation;
2. canonical path validation and rejection of `..`, absolute paths, UNC/device paths, traversal, and platform ambiguity;
3. duplicate-entry and case/normalization-collision detection;
4. compression-ratio, compressed-size, uncompressed-size, file-count, and nesting limits;
5. string, array, object, token, and image-dimension limits;
6. deterministic parser and migration budgets;
7. manifest inventory and SHA-256 validation;
8. stable-ID collision and reference-integrity validation;
9. structured recoverable failures with no catch-and-return-success;
10. rejection of executable content.

Exact numeric limits are not set in Phase 1. They must be objective, tested, and approved before the corresponding importer is implemented; unbounded defaults are prohibited.

## Non-executable boundary

Projects and archives cannot contain or invoke:

- JavaScript, general-purpose WebAssembly, macros, native libraries, binaries, shell commands, process definitions, or executable embedded content;
- dynamic imports, host method names, URLs interpreted as resources, transport descriptors, network endpoints, device handles, or credentials;
- vendor firmware, load binaries, protocol frames, project packages, engineering APIs, or physical deployment artifacts.

A later capability-limited declarative DSL is not authorized by this ADR. If later approved, it must be original, deterministic, explicitly bounded, interpreted as data, and unable to reach host objects, network, filesystem, process, dynamic imports, or general-purpose code (`PES-SEC-0016`). No executable plugin or scripting seam is reserved now.

## Import and export boundary

The product does not read or write `.apXX`, `.zapXX`, Siemens library formats, vendor source exports, PLCopen XML, real-tool load artifacts, firmware, protocol payloads, or other files intended for physical/vendor tools (`PES-PRJ-0006`).

Simulator-native CSV/JSON interchange may be added only by later explicit requirements. It must be documented as simulator-only, contain no executable code, preserve defined identity/reference semantics, and remain distinct from vendor or physical deployment (`PES-PRJ-0007`).

## Migration and compatibility rules

The following are fixed now:

- Migrations are explicit, versioned, deterministic, and auditable.
- Migration preserves stable identity and data unless an approved change record explicitly defines an intentional transformation.
- Unknown/newer versions fail visibly; they are not opened by silently dropping fields.
- Profile semantics cannot change silently during schema migration.
- A failed migration is recoverable and cannot overwrite the only valid source package.
- Every migration has positive, negative, identity, data-preservation, determinism, and rollback/recovery verification.

The complete schema, supported-version window, downgrade behavior, compatibility policy, and migration graph remain BLOCKED under OQ-0005. Choosing them changes observable file compatibility and requires Scott's decision under `PES-DEC-0002`.

## Decision consequences

### Benefits

- Project files are original, inspectable, testable, and clearly simulator-only.
- Schema and profile evolution can be traced independently.
- Stable identity, unresolved references, build fingerprints, and replay inputs can survive persistence.
- Importers can apply deterministic security budgets and fail closed.
- No project file can serve as a physical controller/vendor deployment artifact.

### Costs

- The product cannot open or save vendor projects or use vendor tooling as a persistence layer.
- A documented schema, canonicalization contract, migration suite, and archive-security corpus must be maintained.
- Import/export convenience features require explicit requirements rather than arbitrary file access.
- Unsupported/newer/corrupt content must be rejected visibly instead of best-effort silent recovery.

## Alternatives rejected

| Alternative | Reason rejected |
|---|---|
| Use a Siemens/vendor project or archive format | Violates original-format, clean-room, safety, and compatibility boundaries. |
| Use PLCopen XML now | No Phase 1 approval or legal/interoperability specification exists; `PES-PRJ-0006` forbids it. |
| Store executable scripts/plugins in the project | Converts untrusted documents into a host escape surface. |
| Use an opaque proprietary binary without documented schema | Prevents reliable validation, migration, diffing, traceability, and clean-room review. |
| Use arbitrary ZIP extraction | Enables traversal, collision, bomb, and resource attacks. |
| Key objects by name/path/address | Breaks rename, undo, unresolved references, and deterministic migrations. |
| Silently ignore unknown/corrupt fields | Causes data loss and can manufacture an apparently valid state. |

## Verification obligations

Before a format implementation can be marked VERIFIED, tests must cover:

- manifest presence, schema/profile independence, inventory, hashes, application version, and migration history;
- deterministic round-trip of all supported semantic objects;
- UUID preservation through save/open/migrate/rename/move/readdress/undo;
- tombstone and unresolved-reference persistence;
- collision rejection and explicitly traced independent-object remapping;
- corrupt, unknown, truncated, oversized, hash-mismatched, and ambiguous packages;
- traversal, absolute/UNC/device paths, duplicates, normalization/case collisions, extreme compression, excessive nesting/counts/sizes, invalid encoding, and malicious images;
- executable/script/native/WASM/URL/transport/credential content rejection;
- atomic save, migration failure recovery, and no overwrite of the last valid source;
- no vendor/physical format signature, deployable artifact, protocol payload, or executable in exports;
- requirement-to-test traceability and machine-readable evidence.

Skipped, flaky, unavailable, waived, or inconclusive tests prevent `VERIFIED` status (`PES-REQ-0008`, `PES-QLT-0008`).

## Follow-up decision gate

Phase 2 must resolve OQ-0005—the exact project object schema and migration/downgrade policy—before persistence implementation depends on those details. Any proposal for real-tool interoperability, executable content, cloud storage, remote collaboration, or physical deployment is outside this ADR and must stop under `PES-DEC-0002`.
