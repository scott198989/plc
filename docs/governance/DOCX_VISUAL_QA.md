# Phase 1 Directive DOCX Visual-QA Observation

Status: **OBSERVATION PASS FOR THE CURRENT SOURCE HASH; NOT ADMISSIBLE AS PHASE 1 GATE EVIDENCE**  
Observation date: 2026-08-27  
Source: `References for Codex from Scott/PLC Engineering Simulator - Codex Master Implementation Directive Phase 1.docx`
Source SHA-256: `EBF074E2CEAB752F09E6DB63D88E100991729DA13C1EB874290A6B337DA72612`

## Method

1. Microsoft Word opened the source read-only and exported an ignored local PDF. The source DOCX was not saved, edited, renamed, or replaced.
2. Poppler `pdfinfo` reported a 40-page, US Letter, unencrypted, tagged PDF with no form fields or JavaScript.
3. Poppler `pdftoppm` rendered all 40 pages to PNG at 120 DPI.
4. Four ten-page contact sheets provided a preliminary page-order and gross-layout sweep.
5. All 40 rendered pages were then inspected individually at full-page resolution for margins, content flow, clipping, overlap, table breakage, typography, unexpected blank areas, headers, footers, and page numbering.
6. Pages 2, 4, 12, 18, 22, and 31-40 received additional isolated review after the preview layer intermittently masked margins or list rows in multi-image displays. Those apparent defects were not present in the stored PNGs.
7. A raw-pixel comparison confirmed that pages 2-40 have byte-identical header regions and byte-identical left-footer regions. This resolved the preview-layer masking false positives without relying on text extraction.
8. An offline `pdfplumber` geometry/text pass separately confirmed, on every page: non-empty extractable text, zero out-of-bounds words, zero Unicode replacement glyphs, the expected header text, and the sequential page footer, with the cover treated separately.

## Observation result

**OBSERVATION PASS for the rendered current source hash.** The full-page review found:

- no clipped, overlapping, or off-page text;
- no broken, truncated, or unreadable tables;
- no black squares, replacement glyphs, or visibly corrupt typography;
- consistent headers, footers, page numbering, margins, section hierarchy, and table styling; and
- complete final-page content with the closing rule and page 40 footer visible.

Structural review separately found 247 unique normative requirement IDs and no tracked changes or comments. This observation does not resolve the document-control wording conflicts in `DEC-0001` and does not approve the directive, its renderer, or any tool.

## Gate and authorization boundary

Word, Poppler, and the PDF-QA Python stack remain `UNASSIGNED` / `NOT_REVIEWED` in `TOOLCHAIN_ADMISSION_REGISTER.md`. Their use was outside the standard-library-only local bootstrap exception in `DEPENDENCY_POLICY.md` Section 5.1. The output is therefore retained only as an **unapproved observation**:

- it does not retroactively authorize these tools or expand the bootstrap exception;
- it does not satisfy the directive's visual-QA acceptance gate;
- it is not reviewer acceptance or tool admission;
- it must not be reused as release evidence; and
- an admissible complete rerun is required after the exact rendering/inspection toolchain and reviewer are approved.

## Reproducibility evidence

| Item | Observed identity or SHA-256 |
|---|---|
| Read-only renderer | Microsoft Word 2021 executable version `16.0.20326.20100`; SHA-256 `AB39524C857C0F48D75FD52486597E87D7560C3185258562A08E56028EB00DA5` |
| PDF inspection/rendering | Poppler `pdfinfo`/`pdftoppm` `26.05.0`; executable hashes recorded in `TOOLCHAIN_ADMISSION_REGISTER.md` |
| Geometry/text and pixel analysis | Bundled Python `3.12.13`, `pdfplumber 0.11.9`, `Pillow 12.3.0`, and `pypdf 6.10.0` |
| Derived PDF | `35DD6EB0A4CD87487B4597CDDD1285D7F47101DD2EE39DC2746BCD99A8589B79` |
| Forty-page PNG manifest | `DE64F05C5150C2A204173418B884448D6E70D5EA15782A2254A290A8E35A69A9` over sorted lines of `filename=SHA-256` |
| Machine analysis JSON | `E375D621054A4C0F584379EBB70AFDD521A21F11E50E39C389E6D72677FF5A63` |

The derived PDF, PNGs, contact sheets, helper scripts, and analysis JSON live only under the ignored `.phase1-verification/` directory. They are reproducible local observation material, not controlled production assets or portable review evidence. When present, the local verifier validates the derived PDF hash, the 40-page PNG count/manifest, and the machine-analysis JSON hash; it records when that contracted set is absent and never treats absence or presence as gate acceptance. Contact sheets and helper scripts are ignored convenience material and are not covered by that hash validation. Exact renderer/package provenance, licenses, signatures, maintenance, production exclusion, authorized use, and independent reviewer acceptance remain `UNREVIEWED`.

## Closure-audit DOCX production QA

Status: **OUTPUT QA PASS FOR THE IDENTIFIED CLOSURE-CANDIDATE BYTES; NOT SCOTT ACCEPTANCE OR TOOL ADMISSION**

QA date: 2026-08-27

Source Markdown: `docs/governance/PHASE_1_ADVERSARIAL_AUDIT.md`

Delivered DOCX: `docs/governance/PHASE_1_ADVERSARIAL_AUDIT.docx`

The closure-audit deliverable received a separate full-document production-QA pass after its Markdown audit record was finalized. Microsoft Word opened the generated DOCX read-only and exported an ignored local PDF; the DOCX SHA-256 was identical before and after rendering. The PDF was rasterized at 96 DPI into 133 page images. Automated geometry, text, and raster checks examined every page, and nine contact sheets were inspected across the complete page sequence. Full-resolution inspection additionally covered pages 1, 3, 42, 118, 126, 127, 132, and 133 to sample the cover, defect and reconciliation content, dense audit tables, the mutation section, the interpretation ledger, the pre-remediation conclusions, and the final closure verdict.

The resulting QA observations were:

- exactly 133 nonblank US-Letter landscape pages and 133 rendered PNGs, each 1056 x 816 pixels;
- no blank pages, Unicode replacement glyphs, out-of-bounds words, rendered edge-touch pages, clipped text, visible overlaps, corrupt typography, or unreadable tables;
- consistent page headers, footers, numbering, margins, hierarchy, color, and table styling; and
- complete final-page `Not verifiable` and `Phase 2 verdict` content with no truncation.

| Item | Bytes or SHA-256 |
|---|---|
| Source audit Markdown | `390341` bytes; `E087CB1B5C769D300C07CD50CC19CB6617CFFFE03CF7B3B0F24B50F1949D9A9B` |
| Delivered audit DOCX, before and after rendering | `186939` bytes; `A20C5C208CC33FC5187F39883D4792BBA8E52C62B999BC56B2ABC99FCB6D45F3` |
| Derived 133-page PDF | `2780536` bytes; `158278E8A8327205BA3EF3591FE115275D713A454581BBCCAC8F3D5861B3C9CE` |
| Machine analysis JSON | `140DB167DE53BAC4E50B2A6C96DF23844357354FCF829F365103425C962FCB0E` |

The ignored render material is located under `.phase1-verification/audit-docx/`; it is reproducible QA support and is not shipped as a production asset. This QA result establishes the observed integrity of the identified deliverable bytes. It does not admit Microsoft Word, Poppler, or the PDF-QA Python stack; approve any dependency; convert any requirement to `VERIFIED`; constitute Scott's acceptance; or authorize Phase 2.
