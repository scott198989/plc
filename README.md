# PLC Engineering Simulator

This repository is the Phase 1 foundation for a professional, brand-neutral,
offline educational PLC engineering simulator. The governing product boundary
is simple and permanent:

> `VirtualUniverse` has no adapter to `PhysicalUniverse`.

Phase 1 establishes the product constitution, clean-room rules, security wall,
architecture decisions, evidence and provenance systems, decision/risk records,
and policy verification. It does **not** implement PLC editors, compilation,
runtime execution, HMI behavior, process simulation, lessons, packaging, public
branding, or any physical communication capability.

## Controlling sources

- `PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx`
  is the supplied Phase 1 directive. Its living-document filename remains an
  open decision; see `OPEN_DECISIONS.md`.
- `Govs PLC project Research Report.md` is the frozen research baseline. Its
  SHA-256 is
  `F05C08323B5CC9483BEB1FEB3C7312CCB9A45EBE3B527E6DAE069C181D3FBF55`.

The directive outranks research evidence for product requirements. Neither file
is a legal opinion, trademark clearance, patent clearance, or freedom-to-operate
analysis.

## Phase 1 verification

The local verifier uses only the Node.js standard library and performs no
network access. It deliberately fails unless `node --version` is exactly
`v24.19.0`; the extractor and local launcher likewise require Python `3.13.12`.
The launcher checks `PHASE1_NODE`, the current `PATH`, and the bundled Codex
runtime location, then selects only an exact match:

```powershell
pnpm verify:phase1
```

The verifier first proves that the committed 247-record registry and matrix are
the deterministic output of the current hash-bound extractor, then checks
source hashes, required governance artifacts, ADR invariants,
machine-readable registers, requirement-ID uniqueness, truth-state integrity,
and the absence of forbidden feature/package scaffolding, then writes a report
under `.phase1-verification/`. These checks evaluate the current Phase 1
repository foundation only; they do not represent reviewer acceptance, product
verification, or release-isolation proof.

## Status

- Phase 1 governance foundation: in progress. Automated local checks pass only
  for the current hash-bound snapshot. DOCX structure is current and a complete
  rendered visual observation found no defect, but the rendering/inspection
  toolchain is unapproved, so the visual-QA gate remains unmet. Document
  identity, remote CI, tool admission, contributor attestation, and reviewer
  acceptance remain unresolved or unverified.
- Product implementation: not started.
- Phases 2-4: not authorized by this phase.
- Packaging and public branding: blocked by `OQ-0001` and `OQ-0002`.
- Remote CI and report upload: blocked by `DEC-0002`; the checked-in workflow is
  a disabled proposal, not evidence of a hosted run.

Only `VERIFIED` means complete. File count, package count, a successful build,
or visible scaffolding never counts as implementation progress.
