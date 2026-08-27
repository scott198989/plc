# Clean-Room Policy

Status: Phase 1 controlled policy; effective for all project work  
Research baseline: `References for Codex from Scott/Govs PLC project Research Report.md`
Research SHA-256: `F05C08323B5CC9483BEB1FEB3C7312CCB9A45EBE3B527E6DAE069C181D3FBF55`  
Governing requirements: PES-GOV-0001 through PES-GOV-0013; PES-CRM-0001 through PES-CRM-0025; PES-DOC-0004; PES-CI-0001 through PES-CI-0003

## 1. Purpose and controlling rule

This policy protects the PLC Engineering Simulator as an independent, original, brand-neutral implementation. Educational purpose is the product mission; it is not permission to copy and is not a substitute for legal review. Functional ideas may be studied and independently implemented, but proprietary expression, protected assets, confidential material, reverse-engineering output, vendor project formats, vendor communication technology, and vendor branding do not enter the product. This implements PES-CRM-0001 through PES-CRM-0005.

This policy is a project control, not legal advice or a freedom-to-operate opinion. A contributor must stop affected work when the Phase 1 directive requires counsel, additional verified research, or Scott's decision. No ADR, deadline, prototype, educational rationale, or technical convenience may waive the immutable safety wall or this clean-room boundary.

## 2. Scope

This policy applies to every contributor and to all:

- requirements, research notes, designs, code, tests, schemas, prompts, generated output, documentation, examples, scenarios, lessons, sample projects, translations, and release materials;
- UI layouts, icons, fonts, illustrations, sounds, animations, diagnostic wording, identifiers, device names, and other assets;
- direct, transitive, optional, native, WebAssembly, font, build, development, and test dependencies;
- source material viewed, downloaded, copied, summarized, observed, or supplied to a human or automated tool.

Evidence and research remain separate from production assets and shipped bundles, as required by PES-DOC-0004. A file's location does not make it admissible; its source, license, classification, and approval determine admissibility.

## 3. Source admission

### 3.1 Permitted evidence

The following may be used only as evidence and only after registration under PES-CRM-0008 and PES-CRM-0017:

- public Siemens documentation, Siemens SCE material, and public Siemens product or support pages;
- IEC descriptions or standards lawfully licensed to the project team;
- public statutes and published judicial opinions;
- independent textbooks and tutorials used for corroboration;
- independently created observations made only under a written observation protocol approved by counsel.

Permitted evidence does not become a production asset. Contributors paraphrase behavior, retain the source's actual evidentiary classification, and create simulator-owned requirements and original expression.

### 3.2 Forbidden material

The following must not be acquired for, introduced into, or used to derive project work, per PES-CRM-0009 through PES-CRM-0011:

- Siemens or other vendor source code, leaked code, leaked manuals, partner-only material, confidential training material, or confidential support material;
- decompiled or disassembled output, memory scraping, extracted executable resources, extracted icons, resource packages, or API-hooking output;
- protocol captures intended to reproduce vendor communications, encrypted project-format cracking, or undocumented interoperability obtained through circumvention;
- pirated software, license bypasses, access-control circumvention, or material obtained in violation of applicable terms;
- screenshots, copied manual diagrams, copied tables, copied device illustrations, copied diagnostic text or identifiers, copied help prose, or copied UI compositions used as implementation assets;
- vendor binaries, firmware, engineering DLLs, SDKs, project files, load artifacts, device packages, or proprietary completion databases.

An installed TIA Portal product must not be observed for implementation verification unless counsel has reviewed the applicable license terms and approved a written observation procedure. Public documentation remains evidence, not an asset library. This is an affirmative stop gate under PES-CRM-0010.

### 3.3 Fragile and unresolved citations

The frozen research report contains conversation-scoped citation tokens such as `turn11search2`. Those tokens do not preserve a source title, publisher, version or date, durable URL, or access date. They are inventoried in `docs/research/UNRESOLVED_SOURCE_TOKENS.md` and are not stable evidence records.

No contributor may invent bibliographic details, infer a URL from surrounding prose, or represent an unresolved token as verified. A claim relying only on one of these tokens remains `UNRESOLVED` for evidence purposes until the underlying source is recovered and registered. This implements PES-GOV-0013. The report itself remains the frozen research baseline under PES-GOV-0010; unresolved citations limit the strength of individual claims rather than changing the report hash.

## 4. Required evidence record before implementation

No research-derived behavior may enter implementation until its record in `EVIDENCE_REGISTER.json` contains all fields required by PES-CRM-0017:

1. stable requirement ID;
2. paraphrased observed behavior;
3. source title, publisher, version or date, durable location, and access date, with unknown values recorded honestly as `null` or `UNRESOLVED`;
4. research classification retained from the source: `DOCUMENTED`, `INFERENCE`, `PROPOSED`, `LEGAL_INTERPRETATION`, or `ENGINEERING_RECOMMENDATION`;
5. IP class and disposition;
6. simulator-owned implementation requirement;
7. forbidden implementation shortcut;
8. author, reviewer, review status, and review date;
9. implementation component;
10. verification IDs.

The record must be atomic enough that a reviewer can approve or block the behavior without approving unrelated behavior. A report label may not be silently upgraded. Adoption by the directive makes the requirement normative, but does not make its source evidence stronger.

## 5. IP classification gate

Every externally inspired item must receive one of these dispositions before implementation, per PES-CRM-0006 and PES-CRM-0007:

| Class | Meaning | Required disposition |
|---|---|---|
| 1 | Functional behavior | Independently implement |
| 2 | Industry or lawfully licensed IEC convention | Implement from licensed standard or public behavior |
| 3 | Workflow behavior | Preserve useful workflow logic; redesign expression and visuals |
| 4 | Vendor-specific expression | Redesign; do not copy |
| 5 | Branding or trademark | Replace or exclude |
| 6 | Proprietary technology | Create an original simulated equivalent |
| 7 | Patent or licensing concern | `BLOCKED` pending focused review |
| 8 | Uncertain or high-risk | `BLOCKED` pending professional legal review |
| 9 | Physical industrial communication | Permanently `EXCLUDED` |

An unclassified, mixed, ambiguous, or uncertain item defaults to Class 8 `BLOCKED`; it never defaults to "probably permitted." Class 9 is not a legal-review backlog item and cannot be approved inside this product.

## 6. Independent implementation and original expression

### 6.1 Behavior and code

Implementers receive simulator-owned requirements, not vendor source or extracted structures. The implementation must use original algorithms, data structures, schemas, identifiers, event codes, diagnostics, project formats, source representations, and documentation. The following are forbidden shortcuts:

- translating, porting, or mechanically rewriting vendor code or decompiled behavior;
- using vendor binary layouts, optimized data-block layouts, firmware behavior, compiler components, load artifacts, protocol payloads, or project formats;
- reproducing diagnostic numbers, exact prose, undocumented built-ins, or vendor-specific edge behavior without independently specified and approved requirements;
- treating a screenshot, captured interaction, or visual similarity as a specification.

Functional concepts such as compilation, dependency analysis, scan execution, tags, stateful blocks, online/offline comparison, watch/force semantics, and diagnostic navigation may be implemented only through original simulator semantics. This implements PES-CRM-0003 through PES-CRM-0005.

### 6.2 UI, naming, and documentation

The product must use an original visual system, information architecture, interaction details, typography, icon family, spacing system, fictional device identities, sample data, and written language. It must not copy or sample Siemens colors, icon silhouettes, device illustration style, screen composition, trade dress, or branding. Siemens, SIMATIC, TIA Portal, S7, WinCC, and PLCSIM marks may appear only in quarantined research provenance or counsel-approved factual comparative text, never as product, installer, repository, catalog, domain, or splash-screen identity. These rules implement PES-CRM-0012 through PES-CRM-0015.

All public comparative language mentioning Siemens or TIA Portal is `BLOCKED` until trademark counsel approves the exact wording and required notices. The working product title is descriptive and is not evidence of name clearance.

### 6.3 Assets and generated material

Only assets registered and approved in `ASSET_PROVENANCE.json` may ship. Screenshot tracing, icon tracing, redrawing vendor artwork, recoloring vendor assets, and sampling vendor branding are not original creation. Generated assets require disclosure of the generator or process, human direction, creation date, original hash, derivative chain, license basis, and reviewer approval under PES-CRM-0021 through PES-CRM-0023.

Vendor screenshots, manuals, icons, illustrations, and other quarantined evidence must not be placed in prompts, fine-tuning data, reference boards, mockups, asset-generation inputs, or production repositories. An automated tool does not cleanse a forbidden source.

## 7. Role and review controls

One person may hold more than one role for ordinary low-risk public evidence, but the evidence and implementation records must remain distinct:

- **Research author:** registers permitted sources and writes a behavior-only paraphrase.
- **Requirement owner:** converts evidence into an atomic simulator-owned requirement and identifies forbidden shortcuts.
- **Implementer:** works from the approved requirement and allowed references only.
- **Reviewer:** checks evidence sufficiency, IP class, originality, provenance, and verification links.
- **Legal reviewer:** decides only matters expressly escalated for professional legal review.

Class 7, Class 8, public comparative claims, installed-product observation, suspected contamination, and unusual vendor resemblance require a reviewer independent of the affected implementation and the approval specified by the directive.

## 8. Contributor attestation

Before a contribution is accepted, its contributor must complete `CONTRIBUTOR_CLEAN_ROOM_ATTESTATION.md` for the contribution or an approved batch. The attestation must identify sources and tools actually used and disclose uncertainty. An unsigned, incomplete, or qualified attestation is not approval. This implements PES-CRM-0020.

## 9. Contamination response

Anyone who encounters suspected forbidden or contaminated material must immediately:

1. stop only the affected work;
2. avoid copying, summarizing, forwarding, uploading, or placing the material in a prompt;
3. record the minimum necessary incident metadata without reproducing the material;
4. quarantine affected commits, branches, artifacts, prompts, caches, and generated outputs from builds and derivative work;
5. notify the project owner and clean-room reviewer;
6. identify every potentially derived file and contributor;
7. await a documented disposition.

Suspected contamination may not enter builds, prompts, generated assets, or derived work under PES-CRM-0018. Confirmed contamination requires a clean rewrite by a person who has not relied on the tainted expression, without reuse of tainted code, prose, assets, names, layouts, or extracted structure, as required by PES-CRM-0019. Deleting the obvious source file alone is not remediation. The reviewer records the scope, purge evidence, clean-rewrite basis, and decision.

## 10. Dependency clean-room controls

Every dependency follows `DEPENDENCY_POLICY.md`. A package may be technically convenient and still be rejected for capability, provenance, license, native-code, or contamination reasons. Production dependencies must not introduce networking, device access, native FFI, process execution, executable plugins, vendor formats, vendor SDKs, or remote assets. Development-only tooling may use ordinary developer capabilities only when it cannot enter a production graph, bundle, permission, or user-reachable path. Releases require an SBOM and documented license review under PES-CRM-0024 and PES-CRM-0025.

## 11. Merge and release gates

A merge or release fails when any of the following is true:

- a research-derived requirement lacks an approved IP class and disposition;
- a source record lacks stable metadata and the implementation relies on it as verified evidence;
- a contributor attestation is missing, incomplete, or inconsistent with the contribution;
- suspected contamination is unresolved;
- a vendor screenshot, logo, icon, illustration, copied prose, diagnostic, project format, SDK, or protocol artifact enters production;
- an asset is absent from or unapproved in `ASSET_PROVENANCE.json`;
- a dependency lacks provenance, capability review, license disposition, exact locking, or SBOM coverage;
- a Class 7 or Class 8 item proceeds without the required review;
- public comparative wording lacks exact counsel approval;
- evidence or research material is included in a production bundle.

These are mandatory gates under PES-CRM-0007, PES-CRM-0016 through PES-CRM-0025, PES-DOC-0004, and PES-CI-0001 through PES-CI-0003. A skipped, unavailable, inconclusive, or manually waived required gate is a failure, not a pass.

## 12. Current Phase 1 baseline

- The frozen report is admitted as a project research baseline at the exact hash stated above.
- Its 99 unique fragile citation tokens are unresolved and may not substitute for stable evidence records.
- No external or vendor asset is approved for shipment by this policy.
- No dependency is approved merely by appearing in a package manifest or lockfile.
- Physical industrial communication and vendor deployment artifacts are permanently excluded, not candidates for clean-room implementation.
