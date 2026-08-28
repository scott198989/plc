# Phase 2 packaged counterfactual isolation harness

`run_isolation_counterfactual.mjs` exercises the real inline `dist/index.html`
artifact in an admitted local Chromium build. It puts the browser context
offline after the one allowlisted loopback artifact load, removes project-file
picker adapters, denies browser network/device APIs, instruments the packaged
blob worker, and runs a virtual-network/controller/build/load/scan workflow.

The harness does not contact physical devices or deliberately contact an
external network. Endpoint, URL, UNC, pipe, device, print, escape, and malformed
strings are injected only as inert values through project-name and SCL-source
typed boundaries. A reserved `.invalid` name and RFC 5737 documentation
addresses are used where an endpoint-shaped value is required.

## Candidate run

Build the inline artifact, then run from an exact candidate checkout:

```powershell
pnpm build:foundation
node tools/phase2/run_isolation_counterfactual.mjs --candidate-ref HEAD
```

Closure credit additionally requires an external exact-candidate JSON input
that conforms to `tools/phase2/isolation-closure-evidence.schema.json`:

```powershell
node tools/phase2/run_isolation_counterfactual.mjs `
  --candidate-ref HEAD `
  --closure-evidence C:\verification\phase2-isolation-closure.json
```

The runner copies those exact bytes into the evidence directory, hashes the
copy in `evidence-manifest.json`, and independently revalidates the boundary,
native-backing, live-LAN, export, and supported-configuration claims. The input
must bind the current candidate commit and tree. Supplying the file does not
create credit: missing rows, string-only platform labels, duplicate identities,
non-PASS results, stale candidate bindings, one topology presented twice, or
invariant output drift all fail closed.

The final evidence collector can import
`isolationGateProofFieldsFromClosure(value, candidate)` from
`isolation-counterfactual-lib.mjs`. It returns only the seven validated isolation
proof fields expected by `validate_evidence_record`; it throws on non-credit
input and never invents execution, artifact, instrumentation, or zero-attempt
claims.

For a non-JavaScript collector, the dependency-free wrapper writes those same
validated proof fields without an evidence envelope:

```powershell
node tools/phase2/transform_isolation_closure.mjs `
  --input C:\verification\phase2-isolation-closure.json `
  --output C:\verification\phase2-isolation-gate-fields.json `
  --approval-decision-id P2-DEC-ISO-NATIVE-001 `
  --approval-sha256 ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789 `
  --candidate-commit 0123456789abcdef0123456789abcdef01234567 `
  --candidate-tree 89abcdef0123456789abcdef0123456789abcdef
```

The approval digest must be the exact candidate blob SHA-256 for
`ADR/0005-phase-2-native-isolation-shell.md`; the transform preserves the
decision ID and digest in `isolationApproval`. It exits nonzero on malformed,
stale, partial, approval-mismatched, or non-credit input. The caller
must still derive execution, artifact, instrumentation, and zero-attempt fields
from real complete logs.

The strict run fails unless the harness and helper bytes match the candidate,
the candidate is current `HEAD`, and the worktree has no unaccounted changes.
The Phase 1 adversarial-audit duplicate is ignored only when its path and SHA-256
match the accounted record in `evidence/phase2/P2-00_ENTRY_GATE.json`.

The strict run also fails closed while any binding proof remains unresolved:
actual host network adapters disabled before and after the workflow, controlled
live-LAN discovery invariance, fixed native local file backing, the complete
export rejection matrix, or the approved supported-platform/configuration set
with complete exact-candidate evidence. The current browser artifact cannot
truthfully satisfy all of those claims, so a development run is the appropriate
way to collect partial evidence until the blockers are resolved.

The approved Windows-first set is exact, not open-ended:

- `windows-x64-chromium-native-broker-adapters-on` exercises the versioned
  native project-file broker under controlled live-LAN variation;
- `windows-x64-chromium-packaged-adapters-off` exercises the inline packaged
  workbench with browser file/network/device capabilities disabled and Windows
  adapters proved disabled before and after the workflow.

Both rows need complete logs and zero attributable attempts. If either was not
actually exercised, the evidence must remain incomplete.

Each row also binds `browserRuntimeProduct`, `browserRuntimeVersion`, and
`browserExecutableSha256`. The product identity is a closed normalized value;
host paths are not accepted as identity. The packaged runner measures the
launched Chromium executable and requires its row to match all three fields,
so an independently updated browser cannot inherit evidence from an older run.

`tools/phase2/isolation-fuzz-corpus.tsv` is the sole 27-case corpus source for
the Node/TypeScript and Rust boundary suites. Its decoded values and ordered
case IDs produce the fixed `corpusSha256` and `caseIdsSha256`; changing a value
without updating the candidate-bound contract cannot retain evidence credit.

The external operator/lab procedure and canonical topology fingerprint fields
are fixed in `tools/phase2/LIVE_LAN_TOPOLOGY_PROTOCOL.md`. It deliberately
contains no adapter mutation command and grants no credit without two real,
stable, exact-candidate scenario runs.

## Runtime closure assembly

`assemble_isolation_closure.mjs` is the only supported way to produce a
runtime-credit closure from external proof records. It accepts two independently
finalized live-LAN scenario records plus separate raw records for the native
backing matrix, adapters-off packaged run, all ten fuzz boundaries, and all
four vendor/deployable export surfaces. Every raw record has a sibling
`<record>.bundle.json` content bundle. The assembler re-hashes every bounded
regular file listed by that bundle and requires its declared command and log
hashes to be actual listed content; hand-authored hash-shaped JSON is rejected
by the CLI. The adapters-off bundle additionally requires `pre-adapters.json`
and `post-adapters.json` Windows snapshots showing every adapter disabled or
absent.

```powershell
pnpm assemble:phase2:isolation -- `
  --candidate-commit <40-lowercase-hex> --candidate-tree <40-lowercase-hex> `
  --approval-decision-id P2-DEC-ISO-NATIVE-001 --approval-sha256 <64-uppercase-hex> `
  --live-lan-scenario C:\lab\A-scenario.json --live-lan-scenario C:\lab\B-scenario.json `
  --native-backing-raw C:\lab\native-backing.json `
  --adapters-off-raw C:\lab\adapters-off.json `
  --boundary-fuzz-raw C:\lab\boundary-fuzz.json `
  --export-rejection-raw C:\lab\export-rejection.json `
  --output C:\lab\phase2-isolation-closure.json
```

This uses a safe two-pass graph: raw adapters-off evidence is content-addressed
before the closure exists, then the eventual counterfactual aggregate may
consume the finalized closure. The closure never needs (and must not reference)
the aggregate manifest that will itself hash the closure, so no circular hash
is introduced. Unit-test fixtures model field validation only and cannot be
passed to this command without real content bundles.

During implementation, `--development-run` permits an explicitly inconclusive
run. It never turns dirty-worktree output into candidate evidence:

```powershell
node tools/phase2/run_isolation_counterfactual.mjs `
  --development-run `
  --candidate-ref HEAD `
  --output .phase2-verification/P2-ISO-DEV
```

Focused adversarial unit tests are independent of the browser:

```powershell
node --test tests/phase2/isolation-counterfactual.unit.mjs
```

## Evidence and causal accounting

The default output directory is
`.phase2-verification/P2-ISO-WINDOWS-COUNTERFACTUAL`. It contains:

- `counterfactual-isolation.json`: candidate binding, assertions, workflow,
  fuzz cases, static package/WASM scan, normalized browser runtime
  product/version/executable hash, analysis, limitations, and the exact
  harness executable/argv/working-directory tuple under `harness.invocation`;
- `browser-events.json`: Playwright, CDP, main-realm, and worker capability
  observations;
- `windows-process-network.json`: periodic TCP/UDP samples restricted to the
  launched Chromium root process and its descendants;
- `windows-network-adapters.json`: read-only before/after host adapter inventory
  and an explicit active-adapter analysis;
- `chromium-netlog.json`: continuous Chromium network-stack log;
- `artifact-server.json`: the separately allowlisted loopback artifact load;
- `evidence-manifest.json`: byte counts and SHA-256 values for every evidence
  input/log.
- `closure-evidence-input.json`: exact copied closure input when
  `--closure-evidence` is supplied.

Application attribution follows the directive's counterfactual causal rule.
Main-realm/worker capability calls, context requests, routes, WebSockets, and
CDP requests are primary application observations. Chromium NetLog and Windows
endpoint observations are attributed to the application only when their exact
network identity correlates to a primary application attempt. Browser-global
update, account, network-quality, DNS-configuration, or operating-system noise
is preserved in the logs as un-attributed browser-global evidence; it is not
misrepresented as application behavior. The loopback artifact host and browser
control endpoint are separately allowlisted and accounted.

## Scope and honest residuals

The harness materially supports `VER-ISO-0001` through `VER-ISO-0005` and
`VER-NET-0001`, but it is one evidence input rather than a substitute for the
whole Phase 2 gate.

- It currently supplies process-attributable endpoint capture only on Windows
  and one admitted Chromium executable/configuration. It does not claim Linux,
  macOS, another browser engine, or adapters-on coverage.
- Windows endpoint polling is periodic and can miss short-lived sockets;
  Chromium NetLog, CDP, Playwright routing, CSP, and JavaScript instrumentation
  are complementary continuous channels.
- Dynamic `import()` has no replaceable runtime hook. The harness covers it by
  package scanning, CSP, request routing, CDP, and NetLog.
- A browser file handle exposes `kind` and `name`, not a trustworthy attestation
  of backing volume, provider, redirect, remote/removable status, or native
  locality. The [File System Access specification](https://wicg.github.io/file-system-access/)
  further states that storage presented as local may be backed by a cloud
  provider. The adapters-off proof is complete for absence of picker use, but
  it cannot prove the
  fixed-native-local-backing clause of `VER-ISO-0004` for an adapters-on web
  picker. `apps/foundation-shell/test/file-access-broker.test.ts` separately
  proves that path-, endpoint-, pipe-, device-, and print-shaped handle names
  are rejected before selected-byte I/O; this hardening does not manufacture a
  missing backing-volume attestation.
- The current live LAN is not mutated. Zero attributable host access supports
  invariance for this run, while a controlled multi-LAN/multi-platform matrix
  remains separate evidence.
- Browser capability removal and Playwright offline mode are not relabeled as
  proof that Windows network adapters are disabled. The harness records actual
  `Get-NetAdapter` state before and after the workflow and remains incomplete if
  any adapter is active or the read-only capture is unavailable.
- `VER-ISO-0005` uses the approved `P2-DEC-ISO-NATIVE-001` Windows-first set.
  Approval resolves the former configuration-choice question; it does not create
  runtime credit. Both exact configuration rows still need complete PASS evidence.
  Old evidence is not rejected for its date alone; candidate, requirement, test,
  harness, configuration, result, and log bindings control validity.
- Dependency/source-provenance scans and deterministic virtual-network model
  vectors remain the responsibility of the existing Phase 2 source policy and
  Rust verification lanes; their result must be combined with this evidence by
  the exact-candidate exit gate.
