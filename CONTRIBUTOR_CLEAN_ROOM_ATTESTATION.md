# Contributor Clean-Room Attestation

Status: Blank controlled form; **not an attestation until completed, signed, and reviewed**  
Governing requirements: PES-CRM-0001 through PES-CRM-0025, especially PES-CRM-0020 and PES-CRM-0024; PES-DOC-0004; PES-DEC-0002

## 1. Instructions

Complete this form for one contribution or a clearly identified batch. Answer from actual knowledge; do not sign on behalf of another contributor. Disclose uncertainty, accidental exposure, generated-tool use, and all source material. A qualification or disclosure is not automatically disqualifying, but it blocks the affected contribution until reviewed. An incomplete form earns no approval.

Attach the completed form to the contribution record outside production bundles. Do not attach or reproduce confidential, leaked, reverse-engineered, or otherwise forbidden material.

## 2. Contributor and contribution record

| Field | Contributor-supplied value |
|---|---|
| Contributor legal or approved project identity | `UNCOMPLETED` |
| Project role | `UNCOMPLETED` |
| Contribution identifier (commit/PR/artifact IDs) | `UNCOMPLETED` |
| Contribution title and scope | `UNCOMPLETED` |
| Files or components covered | `UNCOMPLETED` |
| Work start and completion dates | `UNCOMPLETED` |
| Related requirement IDs | `UNCOMPLETED` |
| Related evidence/source IDs | `UNCOMPLETED` |
| Related asset IDs | `UNCOMPLETED` |
| Related dependency review IDs | `UNCOMPLETED` |

## 3. Source disclosure

List every external or project research source consulted for this contribution, including public documentation, standards, statutes, cases, textbooks, tutorials, examples, code, packages, images, design references, prompts, and observations. Use stable source IDs from `EVIDENCE_REGISTER.json` where available.

| Source ID or exact description | Durable location | Access date | How it influenced the contribution | IP class/disposition |
|---|---|---|---|---|
| `UNCOMPLETED` | `UNCOMPLETED` | `UNCOMPLETED` | `UNCOMPLETED` | `UNCOMPLETED` |

If no external source was consulted, replace the row with an explicit signed statement: “No external source was consulted for this contribution.” Do not leave the table blank and infer that answer.

## 4. Contributor affirmations

Initial every statement that is true. Strike none silently; explain every qualification in Section 7.

- [ ] I used only sources permitted by `CLEAN_ROOM_POLICY.md` and disclosed every source in Section 3.
- [ ] I did not use Siemens or other vendor source code, leaked code, leaked manuals, partner-only material, confidential training/support material, or other confidential information.
- [ ] I did not decompile, disassemble, scrape memory, hook APIs, extract executable resources/icons, crack encrypted formats, capture protocols to reproduce vendor communications, bypass licensing, or circumvent access controls.
- [ ] I did not use pirated software or material obtained contrary to applicable access or license restrictions.
- [ ] I did not observe an installed TIA Portal product for implementation verification unless the exact observation was covered by a written counsel-approved protocol identified in Section 7.
- [ ] I did not copy, trace, redraw, recolor, or mechanically transform vendor screenshots, screens, icons, illustrations, color systems, typography, layouts, help text, tables, diagnostic messages/numbers, project structures, or trade dress.
- [ ] I did not use vendor screenshots, assets, manuals, or other forbidden expression in a prompt, reference board, training corpus, generator input, mockup, or asset pipeline.
- [ ] My code, schemas, algorithms, event codes, diagnostics, project/source formats, samples, documentation, visual language, and names are original or are derived only from approved, registered, and properly licensed inputs.
- [ ] Any functional or workflow behavior I implemented has an approved requirement evidence record and IP disposition; I did not treat a fragile citation token as stable evidence.
- [ ] I classified every uncertain or high-risk item as Class 8 `BLOCKED` rather than assuming permission.
- [ ] I introduced no physical-industrial communication, vendor protocol, vendor SDK, endpoint-bearing adapter, generic transport, device API, native bridge, executable plugin, vendor project/load artifact, or hidden future connector.
- [ ] I registered every shipped non-code asset and disclosed every direct, transitive, optional, native, WebAssembly, font, asset, build, development, test, and packaging dependency or toolchain change, including changes to resolved graphs, lockfiles, generated output, and bundled content.
- [ ] I preserved research/evidence separation from production assets and did not place evidence archives in a shipped bundle.
- [ ] I have disclosed all automated coding, generative, translation, image, audio, or design tools that materially influenced this contribution.
- [ ] I am not aware of undisclosed contamination or a reason this contribution should be quarantined.

## 5. Automated and generative tool disclosure

| Tool/service and version | Local or external | Inputs/reference material supplied | Output used and where | Human review performed |
|---|---|---|---|---|
| `UNCOMPLETED` | `UNCOMPLETED` | `UNCOMPLETED` | `UNCOMPLETED` | `UNCOMPLETED` |

If no such tool influenced the contribution, replace the row with an explicit signed statement. Use of an automated tool does not remove source, license, originality, asset-provenance, security, or review obligations.

## 6. Asset and dependency disclosure

### Assets

| Asset ID | Origin and license | Original SHA-256 | Derivative/generated chain | Approval status |
|---|---|---|---|---|
| `UNCOMPLETED` | `UNCOMPLETED` | `UNCOMPLETED` | `UNCOMPLETED` | `UNCOMPLETED` |

### Dependencies

Disclose the complete changed dependency surface, not only packages named directly in a manifest. Include added, removed, upgraded, downgraded, newly enabled, or newly reachable direct, transitive, optional, native, WebAssembly, font, asset, build, development, test, packaging, and installer components; toolchain or CI-action changes; lockfile or resolved-graph changes; install/build scripts; generated code or executable payloads; and bundled assets. A development-only label does not remove the disclosure obligation.

| Package/crate/tool/asset and exact version or commit | Relationship and change | Production/development classification and reachability | Source, integrity, and license evidence | Capability review | Admission/review record |
|---|---|---|---|---|---|
| `UNCOMPLETED` | `UNCOMPLETED` | `UNCOMPLETED` | `UNCOMPLETED` | `UNCOMPLETED` | `UNCOMPLETED` |

If no asset, dependency, resolved dependency graph, toolchain, build/test/packaging input, lockfile, or generated/bundled component was added, removed, or changed, replace the relevant row with an explicit statement. Do not treat an empty row as evidence. Link each disclosed item to its admission record under PES-CRM-0024; disclosure alone is not approval.

## 7. Qualifications, exposure, and incident disclosure

Describe any uncertainty, accidental exposure, similarity concern, unavailable source metadata, license question, generated-output concern, or suspected contamination. Identify affected files and the minimum facts needed for review without attaching forbidden material.

`UNCOMPLETED — absence of text is not a “none” response.`

If none exists, write and initial: “I have no qualification, exposure, or incident to disclose.”

## 8. Contributor certification

I certify that the statements above are complete and accurate to the best of my knowledge. I understand that educational purpose is not permission to copy; that uncertainty must be disclosed; that suspected contamination must be quarantined; and that a false or incomplete attestation may require rejection, purge, and clean rewrite of the affected contribution.

| Field | Required value |
|---|---|
| Contributor signature | `UNCOMPLETED` |
| Printed/approved identity | `UNCOMPLETED` |
| Date | `UNCOMPLETED` |
| Contribution hash at signing | `UNCOMPLETED` |

## 9. Reviewer disposition

The reviewer must compare this form with the contribution, evidence register, asset register, dependency changes, and automated-tool disclosures. The reviewer does not approve matters reserved to counsel.

| Field | Required value |
|---|---|
| Reviewer identity | `UNCOMPLETED` |
| Review date | `UNCOMPLETED` |
| Disposition (`ACCEPT`, `REJECT`, `QUARANTINE`, `LEGAL_REVIEW_REQUIRED`) | `UNCOMPLETED` |
| Scope of disposition | `UNCOMPLETED` |
| Evidence/decision links | `UNCOMPLETED` |
| Reviewer signature | `UNCOMPLETED` |

No contribution covered by this form is clean-room approved until the disposition is `ACCEPT` and all required legal or specialist reviews are linked.
