# Legal Review Checklist

Status: Controlled Phase 1 checklist; all approval fields are initially `NOT REVIEWED`  
Purpose: identify mandatory legal and product-decision gates; this document is not legal advice or an approval  
Governing requirements: PES-GOV-0005; PES-CRM-0001 through PES-CRM-0025; PES-PRJ-0001 through PES-PRJ-0007; PES-DEC-0002 through PES-DEC-0006; PES-CI-0001 through PES-CI-0003

## 1. Use and status rules

Each review item must receive exactly one status:

- `PASS`: the named reviewer approved the precise scope and linked evidence.
- `NOT_APPLICABLE`: the item is outside the reviewed release, with a written reason.
- `BLOCKED`: approval or evidence is required before affected work continues.
- `EXCLUDED`: the product constitution permanently forbids the capability.
- `NOT_REVIEWED`: no conclusion has been reached; this is not a pass.

Every `PASS` or `NOT_APPLICABLE` entry requires reviewer identity, review date, reviewed artifact hash or version, jurisdiction and distribution assumptions, and an evidence/decision link. An unchecked box, silence, elapsed time, or educational purpose is not approval. Related questions should be bundled into the smallest coherent decision request while unrelated work continues, as required by PES-DEC-0004 through PES-DEC-0006.

## 2. Current mandatory dispositions

| Topic | Current status | Controlling reason |
|---|---|---|
| Physical PLC/HMI/drive/I/O communication or deployment | `EXCLUDED` | Class 9 and immutable VirtualUniverse wall; not a legal-review path |
| Vendor project/load/protocol formats | `EXCLUDED` | PES-CRM-0009; PES-PRJ-0006 |
| Installed TIA Portal observation for implementation verification | `BLOCKED` | Counsel-approved license review and written protocol required by PES-CRM-0010 |
| Public Siemens/TIA comparative wording | `BLOCKED` | Exact wording and notices require trademark counsel under PES-CRM-0014 |
| Final public product name, logo, and compatibility language | `BLOCKED` | OQ-0002; working title is descriptive only |
| Class 7 patent/licensing item | `BLOCKED` | Focused review required |
| Class 8 uncertain/high-risk item | `BLOCKED` | Professional legal review required; uncertainty defaults here |
| Patent freedom-to-operate conclusion | `NOT REVIEWED` | The research report expressly is not an FTO search |
| Non-U.S. distribution | `NOT REVIEWED` | Jurisdiction-specific review has not been performed |

## 3. Pre-implementation legal gates

### 3.1 Source and clean-room gate

- [ ] Exact research input hash matches `F05C08323B5CC9483BEB1FEB3C7312CCB9A45EBE3B527E6DAE069C181D3FBF55`.
- [ ] Every relied-upon source has title, publisher, version/date, durable location, access date, and claim mapping in `EVIDENCE_REGISTER.json`.
- [ ] Fragile `turn...search...` tokens have not been represented as stable citations.
- [ ] Research labels remain `DOCUMENTED`, `INFERENCE`, `PROPOSED`, `LEGAL_INTERPRETATION`, or `ENGINEERING_RECOMMENDATION` without silent upgrading.
- [ ] Every externally inspired requirement has an IP class and disposition; uncertain items are Class 8 `BLOCKED`.
- [ ] No leaked, confidential, partner-only, decompiled, disassembled, scraped, extracted, pirated, bypassed, hooked, cracked, or protocol-capture-derived material was used.
- [ ] Required contributor attestations are complete, signed, reviewed, and consistent with the contribution.
- [ ] Any contamination incident has a documented quarantine, scope analysis, purge, and clean-rewrite disposition.

Controlling requirements: PES-GOV-0003 through PES-GOV-0013; PES-CRM-0006 through PES-CRM-0011; PES-CRM-0016 through PES-CRM-0020.

### 3.2 Installed-product observation gate

Before any installed TIA Portal product is observed for implementation verification, counsel must approve all of the following:

- [ ] the exact software edition, version, acquisition channel, license holder, and applicable license/EULA terms;
- [ ] the purpose and limits of observation;
- [ ] the people authorized to observe and the people permitted to implement;
- [ ] prohibited activities, including decompilation, disassembly, memory scraping, API hooking, resource extraction, project-format cracking, protocol capture, and access-control circumvention;
- [ ] what may be recorded, how it will be paraphrased, and where it will be quarantined;
- [ ] retention, access, deletion, incident, and audit procedures;
- [ ] confirmation that no screenshot, icon, diagnostic prose, event ID, asset, or proprietary structure will enter implementation materials or prompts.

Until every item is approved in writing, the status remains `BLOCKED` under PES-CRM-0010 and PES-DEC-0002.

## 4. Copyright, UI expression, and documentation review

- [ ] Functional behavior has been separated from vendor-specific expression.
- [ ] UI composition, visual hierarchy, spacing, typography, colors, icons, illustrations, and interaction details are demonstrably original.
- [ ] No Siemens screen, screenshot, help page, manual diagram, table, artwork, device illustration, icon, diagnostic text/number, or completion database is included or traced.
- [ ] No vendor source, binary, compiler component, firmware implementation, project layout, or proprietary algorithm has been copied or mechanically translated.
- [ ] Diagnostic codes, messages, help, samples, lessons, and documentation are original.
- [ ] Documentation quotations, if any, are minimal, necessary, attributed, licensed or legally approved, and not acting as implementation assets.
- [ ] Automated or generated content was not derived from forbidden prompt/reference material and has required provenance disclosure.
- [ ] A UI-similarity audit found no copied trade dress or unusually close expressive composition.

Controlling requirements: PES-CRM-0001 through PES-CRM-0005; PES-CRM-0011; PES-CRM-0013; PES-CRM-0021 through PES-CRM-0023.

## 5. Trademark, product identity, and public claims

- [ ] Final public product name has a documented clearance decision for the intended jurisdictions and goods/services.
- [ ] Logo, installer identity, repository identity, splash screen, catalog, domain name, and store listing are independently designed and cleared.
- [ ] Siemens, SIMATIC, TIA Portal, S7, WinCC, and PLCSIM are not used as product or catalog identities and do not imply affiliation, endorsement, sponsorship, or support.
- [ ] No Siemens logo, SIMATIC logo, vendor model number, or vendor device identity appears as active product content.
- [ ] Exact public comparative wording and required trademark notices have written trademark-counsel approval.
- [ ] Marketing and documentation do not claim certification, equivalence, full compatibility, endorsement, production readiness, or suitability for real machine control.
- [ ] “Fully functional” is qualified as functionality inside VirtualUniverse only.

Controlling requirements: PES-MSN-0004; PES-MSN-0005; PES-MSN-0009; PES-CRM-0012 through PES-CRM-0015.

## 6. Patent and specialized-technology review

- [ ] Each advanced motion trajectory, auto-tuning method, specialized drive model, unusual commissioning workflow, advanced digital-twin/diagnostic algorithm, and specialized interaction mechanism has an IP class.
- [ ] Each Class 7 item has focused patent/licensing review covering the exact independently designed implementation.
- [ ] Each Class 8 item has professional legal review; no uncertainty was reclassified merely to avoid review.
- [ ] Any FTO search states its jurisdictions, search date, claims reviewed, searcher qualifications, assumptions, and limitations.
- [ ] Generic PID, motion, drive, and SFC work is not represented as vendor-exact and is not implemented before its later-phase authorization.
- [ ] Safety-related educational content does not simulate a safety-rated engineering product or make safety certification claims.

Controlling requirements: PES-CRM-0006; PES-SCP-0005; PES-SCP-0010; PES-DEC-0002; PES-DEC-0003.

## 7. Project formats and interoperability

- [ ] Every project/archive format is simulator-native, documented, versioned, non-executable, and brand-neutral.
- [ ] No `.apXX`, `.zapXX`, Siemens library, vendor source export, vendor load artifact, PLCopen XML, firmware, device package, or protocol payload is imported or exported.
- [ ] `.vlabproj`, `.vlabarchive`, CSV, and JSON output is clearly labeled simulator-only and cannot be directly accepted by a physical industrial tool.
- [ ] File-format names, schemas, manifests, migration history, and source representations are original.
- [ ] Any future non-physical interoperability proposal has separate research, legal approval, change control, and requirements; none is implied by this checklist.

Controlling requirements: PES-CRM-0004; PES-PRJ-0001 through PES-PRJ-0007.

## 8. Asset and dependency review

- [ ] Every shipped image, icon, font, sound, animation, template, sample, translation, and generated asset is registered and approved in `ASSET_PROVENANCE.json`.
- [ ] No vendor or external asset is present without a verified license, source, hash, derivative chain, and approval.
- [ ] Screenshot/icon tracing, vendor-art recoloring, and vendor-brand sampling are absent.
- [ ] Every direct, transitive, optional, native, WebAssembly, font, asset, build, development, and test dependency is covered by license and capability review.
- [ ] Unknown, custom, source-available, noncommercial, field-of-use, copyleft, patent-sensitive, or distribution-incompatible terms received the required focused review.
- [ ] Release SBOM, license notices, source-offer obligations, attribution, and redistribution obligations are complete for the intended distribution.
- [ ] Production bundles contain no remote font, CDN asset, telemetry, updater, vendor SDK, protocol library, networking/device/process capability, or unreviewed native code.

Controlling requirements: PES-CRM-0021 through PES-CRM-0025; PES-SEC-0004 through PES-SEC-0008; PES-CI-0001 through PES-CI-0003.

## 9. Privacy, education, and safety claims

- [ ] Student identity collection is minimal, local, and pseudonymous by default.
- [ ] Teacher answer keys, hidden faults, checkpoints, and scoring rules are separated from student-visible state without claiming impossible local secrecy.
- [ ] Retention, audit export, and deletion behavior is approved before Teacher Mode release.
- [ ] No grades, logs, identifiers, projects, or telemetry leave the local product.
- [ ] Educational disclaimers clearly state that the product is not industrial-control software, vendor software, safety-rated software, certification training, or a deployment tool.
- [ ] No claim implies that a simulator artifact can be loaded into or used to operate physical equipment.

Controlling requirements: PES-TCH-0001 through PES-TCH-0005; PES-MSN-0004 through PES-MSN-0009; PES-SCP-0005.

## 10. Physical isolation and communications

- [ ] Review confirms that physical industrial communication remains permanently `EXCLUDED`, not approved through a license or disclaimer.
- [ ] No adapter, transport provider, protocol plugin, vendor SDK, generic connector, endpoint API, network updater, local server, device API, native bridge, or executable plugin exists in production.
- [ ] Production dependency and packaged-artifact audits cover indirect, optional, aliased, dynamic, native, and WebAssembly capabilities.
- [ ] Simulator exports contain no vendor project, load binary, deployable industrial payload, protocol frame, or executable.
- [ ] Required zero-egress, zero-attempt, inert-address, InternalTagBus, and VirtualControllerId isolation evidence passed without skips or waivers.

This section records constitutional compliance rather than legal permission. Physical capability cannot be approved inside this product. Controlling requirements: PES-ISO-0001 through PES-ISO-0022; PES-CI-0001 through PES-CI-0003.

## 11. Release sign-off record

This record is intentionally incomplete until an identified release candidate is reviewed. It must not be represented as approval while any field is `NOT REVIEWED` or `BLOCKED`.

| Field | Required value |
|---|---|
| Release candidate identifier and hash | `NOT REVIEWED` |
| Distribution model and jurisdictions | `NOT REVIEWED` |
| Product owner decision | `NOT REVIEWED` |
| Clean-room reviewer and date | `NOT REVIEWED` |
| Trademark counsel and date | `NOT REVIEWED` |
| Patent/licensing reviewer and date, if applicable | `NOT REVIEWED` |
| Dependency/license reviewer and date | `NOT REVIEWED` |
| Privacy/safety-claims reviewer and date | `NOT REVIEWED` |
| Linked evidence package | `NOT REVIEWED` |
| Final disposition | `BLOCKED` |
