# plc-core

`plc-core` is the capability-free Phase 2 project kernel. It owns canonical
project identity, revisions, graph transactions, dependency/reference state,
undo/redo, persistence bytes, autosave replay, and sequential migrations. It
uses no third-party crate and compiles for `wasm32-unknown-unknown`.

## Host boundary

The crate never opens a path, reads a clock, obtains entropy, resolves a URL,
starts a process, or touches a network/device API. UUIDs are supplied by the
caller or derived deterministically from an explicit seed. `KernelSession`
provides canonical byte requests/responses for a thin Worker/WASM wrapper;
all UInt64 values on that JSON boundary are lossless decimal strings.

The `.vlabproj` codec produces a sorted, bounded, uncompressed logical
container with member and whole-container SHA-256 verification. Because this
crate deliberately has no filesystem capability, the application shell is
responsible for durable temporary writes, atomic replacement, recovery-file
retention, and immutable backup storage. `save_package` performs construction
and reopen verification before updating its in-session checkpoint, without
incrementing any engineering revision.

The decoder exposes independent package, entry, total-entry, expansion-ratio,
path, image, JSON-depth, string, collection, value, and project-object limits.
The physical format has no compression or link member type, so admitted
members have expansion ratio one and cannot carry symlink behavior. Approved
`edu.*` simulator extensions are closed-schema structured records: they are
bounded, hashed, preserved across open/save/Save As/archive/retrieve, excluded
from executable dispatch, and unavailable to ordinary engineering mutation.

## Explicitly staged outcomes

- Vendor/native project import is not admitted by P2-01;
  `preview_native_import` returns a typed deterministic unsupported outcome.
- HMI references/dependencies are represented as reserved enum cases and are
  rejected by mutation commands in Phase 2.
- Migration mechanics and reports are implemented, but schema 1 is the only
  shipped document schema in this crate, so no historical migration callback
  is registered here. The generic runner creates immutable in-memory source
  evidence before callbacks, verifies each adjacent callback twice for
  deterministic model/report output, requires exact changed-object reporting,
  proves idempotence, and returns no partial candidate on failure.
- Compression is intentionally absent. Expansion-ratio and link/symlink risks
  therefore do not exist in this physical codec; byte/member/string/nesting/
  collection/path/object limits are still enforced before interpretation.
- `ProjectArchive` is a verified in-memory archive abstraction. Durable archive
  placement remains a host responsibility, and archive APIs explicitly report
  when unsaved edits were excluded.

## Verification

From the repository root:

```text
cargo fmt --package plc-core -- --check
cargo clippy -p plc-core --all-targets -- -D warnings
cargo test -p plc-core
cargo check -p plc-core --target wasm32-unknown-unknown
```

The integration suite includes Journey D: rename identity, unresolved
tombstone references, exact identity restoration through undo, copy-closure
UUID remapping with external-reference preservation, canonical save/open/save,
Save As identity, and corruption rejection without partial session mutation.
`persistence_adversarial` adds checked-in migration goldens plus crash-tail,
journal-chain, archive-limit, path/device/case, unknown-schema, extension,
downgrade, corruption, identity, and inert hostile-text vectors.
