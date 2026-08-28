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

The strict run fails unless the harness and helper bytes match the candidate,
the candidate is current `HEAD`, and the worktree has no unaccounted changes.
The Phase 1 adversarial-audit duplicate is ignored only when its path and SHA-256
match the accounted record in `evidence/phase2/P2-00_ENTRY_GATE.json`.

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
`.phase2-verification/P2-ISO-WINDOWS-ADAPTERS-OFF`. It contains:

- `counterfactual-isolation.json`: candidate binding, assertions, workflow,
  fuzz cases, static package/WASM scan, analysis, and limitations;
- `browser-events.json`: Playwright, CDP, main-realm, and worker capability
  observations;
- `windows-process-network.json`: periodic TCP/UDP samples restricted to the
  launched Chromium root process and its descendants;
- `chromium-netlog.json`: continuous Chromium network-stack log;
- `artifact-server.json`: the separately allowlisted loopback artifact load;
- `evidence-manifest.json`: byte counts and SHA-256 values for every evidence
  input/log.

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
  locality. The adapters-off proof is complete for absence of picker use, but
  it cannot prove the fixed-native-local-backing clause of `VER-ISO-0004` for an
  adapters-on web picker.
- The current live LAN is not mutated. Zero attributable host access supports
  invariance for this run, while a controlled multi-LAN/multi-platform matrix
  remains separate evidence.
- Dependency/source-provenance scans and deterministic virtual-network model
  vectors remain the responsibility of the existing Phase 2 source policy and
  Rust verification lanes; their result must be combined with this evidence by
  the exact-candidate exit gate.
