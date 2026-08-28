# Controlled live-LAN topology variation protocol

This protocol closes the live-LAN portion of `VER-ISO-0003` without adding any
network, discovery, device, PLC, or industrial-communication capability to the
product. Topology mutation and observation belong to an operator-controlled
Windows verification environment outside the renderer and native project-file
broker.

## Safety and admission

- Run only on the supported `windows-x64-chromium-native-broker-adapters-on`
  configuration from an exact clean candidate.
- Do not change adapters on a machine whose remote-control session depends on
  them. Use a local operator or a lab/CI host with an out-of-band control path.
- The operator changes the LAN attachment. Product code must not enumerate,
  enable, disable, configure, probe, resolve, or connect through an adapter.
- Admit a scenario only when the versioned native broker and the production
  packaged workbench both ran, instrumentation logs are complete, and the
  application-attributable external-attempt count is exactly zero.

## Immutable controlled input and output

Before scenario A, bind one immutable simulator-native project/workflow input
as `controlledInputSha256`. Reuse those exact bytes for every scenario. After
each run, export the same canonical simulator-native replay verification result,
excluding evidence timestamps and host topology metadata, as
`deterministicOutputSha256`.

The input hash must be identical across scenarios. The canonical product output
hash must also be identical. A changed product output is a failed invariance
test, not a new baseline.

## Topology fingerprint

Immediately before and after each product run, an external read-only harness
captures Windows adapter/IP configuration. Canonicalize only these fields,
sorted by interface identity:

- interface index, GUID, name, description, operational status, media state,
  physical/virtual classification, MAC address, and link speed;
- unicast address and prefix, address family, default gateway, DNS server, and
  connection-profile identity.

Exclude counters, capture timestamps, DHCP lease times, and other volatile
telemetry. Hash the canonical UTF-8 JSON as `topologyFingerprint`. Pre/post
fingerprints within one scenario must agree; otherwise the scenario is
unstable and receives no credit.

## Required scenarios

Collect at least two independently executed scenarios with distinct non-empty
scenario IDs and distinct topology fingerprints. The mutation must be genuine
operator/lab control (for example, two isolated LAN fixtures); editing a saved
snapshot or relabeling one run is non-credit.

Each scenario object must conform to `lanScenario` in
`isolation-closure-evidence.schema.json`, including exact candidate commit/tree,
the admitted configuration ID, complete log-manifest hash, immutable input and
output hashes, and zero external attempts. The closure validator rejects a
duplicate topology, changed input, changed output, partial log set, stale
candidate, or non-PASS run.

## External collector procedure

The collector is verification tooling, never product code. It invokes only
read-only Windows adapter/IP/profile cmdlets and must not be used to enable,
disable, configure, discover, probe, resolve, or connect through an adapter.
It also never builds or launches the candidate. For each independently run
fixture, use the fixed collector bytes from the exact candidate:

```powershell
pnpm collect:phase2:live-lan -- snapshot --boundary pre --output C:\lab\A-pre.json
# A local lab operator manually runs and finalizes one native E2E bundle here.
pnpm collect:phase2:live-lan -- snapshot --boundary post --output C:\lab\A-post.json
pnpm collect:phase2:live-lan -- assemble-scenario --scenario-id A `
  --pre C:\lab\A-pre.json --post C:\lab\A-post.json `
  --native-bundle C:\lab\native-A --output C:\lab\A-scenario.json
```

Repeat the three capture steps for B after a genuine operator-controlled LAN
change and a second native E2E run. The scenario assembler recomputes the
canonical fingerprint from both snapshots, verifies the collector's actual
source SHA-256, and re-hashes the seven finalized native-bundle files. It
rejects changed pre/post topology, a reused final evidence manifest or raw host
receipt across A/B, missing files, symlinks, or mismatched bytes. It does not
copy, delete, or modify a finalized bundle.

## Separate adapters-off configuration

The second supported row,
`windows-x64-chromium-packaged-adapters-off`, is a distinct execution. Capture
actual adapter state before and after the packaged workflow and admit it only
when every adapter is disabled or absent. A browser offline flag is not a
substitute for host adapter state.

The two configuration rows and all scenario objects are runtime evidence. This
protocol, the JSON Schema, and passing validator tests create no verification
credit by themselves.
