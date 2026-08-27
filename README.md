# PLC Engineering Simulator

This repository is the **Phase 1 closure candidate** for a professional,
brand-neutral, offline educational PLC engineering simulator. It is awaiting
Scott's acceptance. Phase 2 product implementation has not begun.

The permanent product boundary is:

> `VirtualUniverse` has no adapter to `PhysicalUniverse`.

Phase 1 now contains two deliberately bounded layers:

- A reconciled governance and verification foundation: source authority,
  clean-room controls, safety invariants, requirements, traceability, risks,
  decisions, a trusted Git-object manifest, an adversarial mutation gate, and
  active local/CI configuration.
- A minimal runnable technical foundation: a local React screen sends one typed
  `foundation.health` command through the domain boundary and a Web Worker to a
  real dependency-free Rust/WASM function, validates a structured
  `DomainResult`, and renders the returned schema version, build identity, and
  fixed `HEALTHY` state.

It contains no PLC project model, hardware catalog, tag/type system, LAD/FBD/SCL
editor, compiler, execution runtime, HMI, process simulation, lesson, scenario,
assessment, industrial protocol, device adapter, physical communication path,
packaging, or public branding.

## Controlling sources

Scott's canonical source and evidence documents are preserved under
`References for Codex from Scott/`:

- `Govs PLC project Research Report.md` — SHA-256
  `F05C08323B5CC9483BEB1FEB3C7312CCB9A45EBE3B527E6DAE069C181D3FBF55`.
- `PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx`
  — SHA-256
  `EBF074E2CEAB752F09E6DB63D88E100991729DA13C1EB874290A6B337DA72612`.
- `PLC Engineering Simulator - Phase 1 Corrective Addendum - Closure and Trusted Baseline.docx`
  — SHA-256
  `950C5112C34D0218FD1E59CF6C051ACCD01AB92674CD70C96C08A5F1DA2E5A1C`.

The Phase 1 directive outranks research evidence for product requirements. The
corrective addendum authorizes only bounded Phase 1 closure work and the minimal
technical foundation. None of these files is a legal opinion, trademark
clearance, patent clearance, or freedom-to-operate analysis.

## Repository foundation

- `apps/foundation-shell/` — minimal accessible local UI and worker boundary.
- `packages/foundation-contract/` — strict typed command and `DomainResult`
  validation.
- `crates/foundation-wasm/` — deterministic 247-byte zero-import WASM health
  implementation.
- `tools/foundation/` and `tests/foundation/` — toolchain, build, isolation,
  unit, and real-browser verification.
- `tools/phase1/` and `tests/phase1/` — deterministic extraction, trusted
  baseline verification, and the 12-case mutation gate.

All exact package versions are locked. Third-party packages remain explicitly
`CANDIDATE_UNREVIEWED`; the corrective addendum permits their bounded candidate
use and evaluation, not production release or legal/security approval.

## Commands

Required toolchain: Node `24.19.0`, pnpm `11.19.0`, Python `3.13.12`, and Rust
`1.94.0` with `clippy`, `rustfmt`, and `wasm32-unknown-unknown`.

Install the frozen dependency graph without lifecycle scripts:

```powershell
pnpm install --frozen-lockfile --ignore-scripts
```

When an approved pnpm store is already materialized, the restore can be forced
offline with `--offline` and the selected `--store-dir`.

Run the complete minimal-foundation gate:

```powershell
pnpm gate:foundation
```

Run the full committed closure gate, including exact requirement regeneration,
foundation checks, trusted-baseline verification, and all twelve isolated
mutations:

```powershell
pnpm gate:closure
```

Build and open the standalone local artifact without a development server:

```powershell
pnpm build:foundation
pnpm launch:foundation
```

The artifact is the single ignored file `dist/index.html`. It uses no remote
resource, page request, endpoint, device API, or WASM import.

## Requirement truth

The reconciled schema-v3 record distinguishes quantities that the original
bootstrap conflated:

- 247 source-parent IDs.
- 484 issued IDs after preserving 20 compound parents and issuing 190 atomic
  children.
- 464 atomic records; 463 are completion-eligible.
- 546 source statement units, all mapped, across 789 explicit relationships.
- Zero `VERIFIED` requirements. Only `VERIFIED` can count as complete.

## CI and acceptance boundary

`.github/workflows/phase1-governance.yml` is an active, executable declaration
for push, pull request, and manual dispatch. It runs the same
`pnpm gate:closure` command and does not upload evidence. This local repository
has no configured remote; no hosted run, publication, push, credential use, or
artifact retention is claimed.

The candidate is not accepted or released merely because its automated gate
passes. Scott's review and acceptance remain required before the separate Phase
2 master directive can authorize PLC product implementation.
