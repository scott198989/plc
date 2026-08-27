# Foundation Shell Visual QA

## Review basis

- Review date: 2026-08-27.
- Reviewed state: the successful `HEALTHY` state after keyboard activation, with the primary action retaining a visible focus indicator.
- Desktop reference: `C:\Users\Scott\.codex\generated_images\01a0445a-cf66-7602-b6ee-0404f0eee0c5\exec-586564ef-99d7-4dc2-9072-f8a06a88f70f.png`; 1586 x 992; SHA-256 `904B4357C847A7EB98911C93098226181FCF358D65A94437D626F0FE1B87BD85`.
- Mobile reference: `C:\Users\Scott\.codex\generated_images\01a0445a-cf66-7602-b6ee-0404f0eee0c5\exec-b5c52f5a-666e-4906-8cd2-1dc070f88bcc.png`; 853 x 1844; SHA-256 `92439BC3F678412E7C5EB3679B2EB933DFE122DFA4E6B95A983688FAB0FC1006`.
- Desktop implementation: `C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\.phase1-verification\foundation\foundation-desktop.png`; 1586 x 920; SHA-256 `8049D03F36F6EC07069057D7B9F3671EC34755F1283771E714466239C165A31F`.
- Mobile implementation: `C:\Users\Scott\OneDrive\Desktop\Codex - GOV's PLC\.phase1-verification\foundation\foundation-mobile.png`; 852 x 1646; SHA-256 `C1A3FA406C0691806CB31EBC422129AC15BC04A3210D61595A4DF62C537C45B6`.

The references include browser chrome while the implementation captures contain only the page viewport. The desktop comparison therefore excludes the reference's 72-pixel browser band and compares its 1586 x 920 page region with a 1586 x 920 DPR 1 implementation capture. The mobile comparison excludes the reference's approximately 203-pixel browser band and compares its approximately 853 x 1641 page region with a 426 x 823 CSS-pixel viewport captured at DPR 2 (852 x 1646 physical pixels). The remaining one-pixel width and five-pixel height differences are bounded to the generated reference crop and browser-density normalization, not an application layout defect.

## Verification method

`pnpm test:e2e` used Playwright Core with the installed Chrome 151.0.7922.174 executable and loaded the built artifact directly through `file://`; no development server was used. The test exercised keyboard focus, Enter-key activation, the loading-to-success transition, a deterministic repeat request, desktop and mobile responsive layouts, horizontal-overflow checks, console/page errors, and page-level remote-request counting. Both viewports completed with zero page remote requests and no page errors.

Playwright supplied the interaction and screenshot evidence. `view_image` supplied independent visual inspection, including a single comparison input containing both references and both post-fix implementation captures. The Codex in-app Browser rejected the `file://` URL under its navigation policy; no bypass was attempted, and the rejection was not treated as application evidence.

## Surface comparison

| Surface | Result | Evidence-bounded assessment |
|---|---|---|
| Layout and spacing | Pass | Desktop header height, content origin, heading and paragraph wrap, primary action, table geometry, health row, and footer align closely with the normalized reference. Mobile gutters, full-width action, stacked rows, health panel, offline note, and footer preserve the reference hierarchy without overflow. |
| Typography | Pass | The local Segoe UI system stack, weights, line heights, and responsive sizes reproduce the reference's utilitarian engineering tone without a remote font dependency. |
| Color and borders | Pass | The navy, teal, pale cyan, muted gray, white, divider, and focus colors are consistently tokenized. Border weights and square-to-lightly-rounded geometry match the reference closely. |
| Copy | Pass | Product name, health-check instruction, button label, subsystem labels, status labels, offline note, and footer wording match the selected visual target. |
| Interaction and states | Pass | Initial, loading, success, and bounded error rendering exist. Keyboard focus remains visible through the success state, and repeated activation returns the same valid `DomainResult`. |
| Responsive behavior | Pass | The desktop information table changes to readable labeled rows at the mobile breakpoint while retaining semantics and the same information. |
| Icons and assets | Accepted policy-constrained deviation | The reference uses geometric icons. The project does not yet have separately approved icon-asset provenance, so the implementation intentionally ships no image, font, SVG, or third-party icon assets. Project-owned code-native `P1`, status, information, and check marks preserve meaning and contrast. Substituting an icon package would contradict the controlling asset policy until its assets have separately approved provenance records. |

## Focused inspection

Focused inspection covered the brand mark, primary action and focus ring, subsystem/status column alignment, healthy status indicator, offline information panel, and footer lockup because those regions contain the smallest high-contrast geometry and reveal spacing or density drift most readily. Text remained sharp at both densities, focus was not clipped, table and row separators were consistent, and the mobile layout did not crop or overlap.

## Comparison history

1. First comparison found P1/P2 scale and hierarchy drift: the desktop content was too compact, an extra header subtitle changed the target, the status grid and footer were vertically misaligned, and the mobile card/gutters/header were oversized. The implementation was retuned to the normalized reference dimensions; the extra subtitle and non-reference card treatment were removed.
2. A governance comparison found a P2 provenance contradiction: an earlier implementation rendered package-provided icons while the approved asset register contained zero assets. The icon dependency, catalog entry, lockfile nodes, and rendered package assets were removed. The current implementation truthfully ships zero external visual assets.
3. A final interaction/visual comparison found P2 focus and density differences. The desktop viewport was normalized to 1586 x 920, loading behavior changed from native `disabled` to guarded `aria-disabled` so keyboard focus remains visible, and desktop/mobile spacing and action alignment were retuned.
4. An earlier evidence set recorded desktop capture hash `D93E42...9F171` at 1584 x 848 and mobile capture hash `527110...AB38C` at 852 x 1646. Those captures are superseded by the full hashes and normalized dimensions listed in Review basis.
5. Post-fix same-input comparison found no remaining actionable P0, P1, or P2 visual defects. The iconography difference is the explicit, traceable asset-policy constraint described above and requires new asset approval authority to change.

## Scope and publication

The screenshots are local, ignored verification evidence under `.phase1-verification`. The active CI workflow does not upload them or any other verification artifact. This report records local inspection only and does not claim remote CI execution, remote publication, release approval, or production readiness.

final result: passed
