# Phase 2 native product-path evidence workflow

This workflow has no path, package, runtime, shell, or network override input.
It intentionally separates the user-confirmed native launch from evidence
finalization so an automated task cannot silently substitute for action-time
approval under the interactive Windows identity.

1. Run the fixed strict build with `node tools/phase2/build_phase2_native.mjs`.
   This builds the exact candidate shell first and then compiles the launcher
   with the candidate manifest, package rows, reviewed requirement mapping,
   finalizer, and isolation-analysis library SHA-256 values embedded.
2. From Windows Explorer, run the fixed, no-argument
   `.phase2-verification/native-build/Run-Native-E2E.exe`. The launcher must run
   as the interactive user. It fails closed under a redirected or constrained
   identity and writes only the immutable, non-credit
   `native-platform-observer-manifest.json` plus its raw evidence bundle.
3. Run `node tools/phase2/finalize_native_e2e_evidence.mjs` with no arguments.
   The finalizer validates every observer input hash before parsing, performs
   complete bounded Chromium NetLog and conservative process/endpoint
   analysis, writes `native-network-analysis.json`, and emits the distinct
   `native-platform-evidence-manifest.json`. The raw, observer, network, and
   final manifests are bound to the exact runtime replay SHA-256, verified
   canonical replay content SHA-256, positive event count, and positive
   boundary count. Only the final manifest can carry
   `instrumentationComplete=true`, `zeroExternalAttempts=true`, and `PASS`.

For package-command wiring, use:

- `build:phase2:native`: `node tools/phase2/build_phase2_native.mjs`
- `finalize:phase2:native`: `node tools/phase2/finalize_native_e2e_evidence.mjs`
- `test:e2e:phase2:native`: the same fixed finalizer command, after the
  separately confirmed Explorer launch

The finalizer is intentionally not coupled to a task-owned process launch.
Missing observer evidence, a constrained-identity failure, stale/tampered
bytes, malformed or truncated NetLog, an unclassified target, or any external
DNS/URL/socket/UDP observation fails closed.
