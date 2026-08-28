# Phase 2 Windows Native Isolation Shell

- Status: Authorized by Scott for the Phase 2 boundary only
- Decision ID: ADR-0005 / `P2-DEC-ISO-NATIVE-001` Option A
- Date: 2026-08-28
- Authority: Scott's explicit in-thread approval, quoted verbatim below
- Authority text SHA-256 (exact UTF-8 visible text, including the Markdown code delimiters around the decision ID and no trailing newline): `3560A0912241FD04101DF57E8810252D6622C03FC8006BD821C39D7D426ED9DA`
- Scope: Phase 2 Windows product-file open/create/replace only
- Supersedes: No broad decision; narrows one Phase 2 implementation stop while preserving `OQ-0001`

## Recorded authority

> I select Option A for `P2-DEC-ISO-NATIVE-001`. Approve a narrowly scoped Windows-first shell limited to a versioned typed project-file broker and fixed-local-backing attestation. It must fail closed for provider-backed, remote, removable, special, redirected, or unsafe targets and expose no arbitrary filesystem, shell, network, device, PLC, industrial-communication, or deployable-export capability. This decision only unblocks Phase 2 work. Complete all remaining Phase 2 implementation, testing, replay, restoration, isolation, and exact-candidate evidence requirements. Do not declare Phase 2 accepted and do not begin Phase 3 until the full exit gate passes.

## Context

Phase 2 requires a runnable product-file path while the browser-only picker cannot attest the storage backing required by the isolation boundary. Treating an unattestable browser picker or a test-injected global as equivalent to a native product path would be evidence theater. Conversely, exposing a generic native bridge, filesystem path, shell, device, network, or industrial capability would breach the immutable VirtualUniverse boundary.

Scott selected the directive's narrowly scoped Option A. The decision permits only the minimum Windows host and typed project-file broker required to attest a fixed-local backing and perform bounded simulator-native project-file operations.

## Decision

Phase 2 may implement and verify one Windows-first shell with these exact properties:

1. The shell hosts only the packaged local application in a capability-minimized Microsoft WebView2 surface. The reviewed SDK/loader input is pinned to `Microsoft.Web.WebView2` `1.0.4129.50`; no floating, prerelease, or renderer-downloaded shell dependency is admitted.
2. The renderer receives one immutable, versioned `govsProjectFileBrokerV1` surface. Its operations are limited to typed create, host-selected open, conditional replace, and revoke messages for bounded `.vlabproj` documents. It carries opaque grants and bytes, never raw paths.
3. Normal open selection is owned by the host and is limited to an attested, fixed application project root enumerated by the broker. The renderer cannot supply a filename or navigate a generic filesystem dialog. Save As is create-only; replacement requires a live typed grant and fails on stale identity.
4. The root, target, package, helper, and applicable runtime/profile locations fail closed unless authoritative Windows inspection proves a fixed native-local, non-remote, non-removable, non-hotplug, non-provider, non-redirected, non-reparse, non-special backing. Metadata inspection precedes user acceptance; selected bytes are read only after acceptance.
5. The browser-only picker path receives no production or verification credit. When the exact native bridge is absent or malformed, product-file access fails closed and explicitly requires the native broker.
6. The shell and broker expose no arbitrary filesystem traversal, raw path, generic method invocation, shell, process-launch surface, network, endpoint, device, PLC, industrial communication, vendor-tool interoperability, plugin, executable-content, or deployable-export capability.
7. Packaged assets, the helper, the bridge contract, native source/ABI inventory, WebView2 input, renderer build recipe/toolchain, generated renderer artifact, and the observed runtime identity are hash-bound to the exact candidate. Evidence must exercise the real packaged renderer-to-broker path and independently substantiate zero external attempts; mocks and self-asserted manifests are non-credit.

## Supported boundary

The only implementation boundary authorized by this ADR is:

`packaged local workbench -> exact typed broker V1 -> private framed helper channel -> attested fixed-local Projects root`

The helper is not a public service and the private transport is not a general IPC facility. It accepts a fixed protocol/version and bounded message set, is launched only by the manifest-bound shell, and is terminated on channel or authority failure.

## Required fail-closed behavior

- Unknown protocol versions, message kinds, fields, grants, or over-limit frames are rejected.
- Provider-backed, remote, removable, hotplug, virtual/file-backed, redirected, reparse, hard-linked, special, unsafe, or unverifiable paths are rejected before selected-byte I/O.
- Package, helper, asset, bridge-contract, runtime, or manifest identity drift is rejected.
- Browser navigation, downloads, external resources, file ingress/drop, browser pickers, printing, device APIs, and background endpoint attempts are disabled or rejected.
- Grant exhaustion, collision, stale identity, concurrent replacement, failed revoke acknowledgement, broker loss, and shutdown fail closed.
- Missing or inconclusive external isolation evidence is failure, not a waiver.
- Chromium NetLog and periodic endpoint snapshots are diagnostic only. A zero-attempt runtime claim requires a separately captured, gap-free Windows-native event interval covering process ancestry, DNS/resolver, endpoint/socket, and packet observations, hash-bound to the exact candidate and preserved committed project bytes.
- Renderer-visible replay text is diagnostic only. Credit requires an independent verifier to recompute the recorded replay hashes and counts from the preserved committed project artifact; equality with a host or DOM receipt is insufficient.

## Consequences

This decision allows the Phase 2 implementation and evidence work needed to close the typed native project-file boundary on the supported Windows configuration. It does not itself verify that implementation, pass an acceptance row, or make a release claim.

The shell adds a narrowly reviewed native production component and exact WebView2 SDK/loader input. They must appear in the production-source, ABI, dependency, license, package, and exact-candidate inventories. Any new native capability or broader operation requires a separate decision.

## Explicitly unresolved and out of scope

- `OQ-0001` remains open for final supported operating systems, public packaging/distribution, installation model, signing, updater, runtime servicing, and release support. This ADR records a Phase 2 Windows-first verification boundary only.
- Phase 2 is not accepted by this decision. Acceptance remains blocked until every implementation, test, replay, restoration, isolation, exact-candidate, and independent-review exit-gate requirement passes.
- Phase 3 and Phase 4 are not authorized or started.
- Public release, hosted execution, remote publication, telemetry, crash reporting, accounts, cloud storage, network services, physical devices, PLC communication, industrial protocols, vendor-tool integration, and deployable export remain unauthorized.
- No claim is made for macOS, Linux, browser-only/PWA, portable, installer, or updater configurations.

## Verification obligations

The exact candidate must provide, at minimum:

- source, dependency, ABI/import, license, and package-manifest drift rejection;
- focused positive and exhaustive negative broker/adapter/protocol tests;
- a real packaged WebView2 create/open/conditional-replace/revoke run using a valid canonical `.vlabproj` and production workbench path;
- root/target/package/profile/helper/runtime attestation captured before admitted I/O;
- a hash-bound, gap-free independently captured Windows-native process-ancestry, endpoint/socket, DNS/resolver, and packet event interval proving zero external attempts for the supported configuration;
- preserved post-replace project bytes and independent replay recomputation evidence; a renderer or host assertion alone earns no replay credit;
- candidate commit/tree, evidence-manifest, input/output, canonical replay, WebView2 product/version/executable, and artifact hashes;
- fail-closed evidence for unavailable, redirected, provider-backed, remote, removable, hotplug, virtual, special, stale, malformed, and browser-only cases.

Until those obligations and the complete Phase 2 exit gate pass, all results remain implementation or observation evidence only.
