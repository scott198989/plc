import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import {
  existsSync,
  lstatSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const errors = [];
const checks = [];
const ignoredDirectories = new Set([
  ".git",
  ".idea",
  ".phase1-verification",
  ".pnpm-store",
  ".vscode",
  "__pycache__",
  "coverage",
  "node_modules",
  "playwright-report",
  "target",
  "test-results",
]);

function isIgnoredLocalFile(projectPath) {
  const name = projectPath.split("/").at(-1);
  return (
    name === ".DS_Store" ||
    name === "Thumbs.db" ||
    name === "desktop.ini" ||
    name.startsWith("~$") ||
    (/^\.env(?:\..*)?$/.test(name) && name !== ".env.example") ||
    /\.(?:local|log|py[co]|tmp)$/i.test(name)
  );
}

function record(id, passed, detail) {
  checks.push({ id, passed, detail });
  if (!passed) errors.push(`${id}: ${detail}`);
}

function readText(path) {
  return readFileSync(join(root, path), "utf8");
}

function readJson(path) {
  try {
    return JSON.parse(readText(path));
  } catch (error) {
    record("VER-JSON-0001", false, `${path} is not valid JSON: ${error.message}`);
    return null;
  }
}

function fileSha256(path) {
  return createHash("sha256").update(readFileSync(join(root, path))).digest("hex").toUpperCase();
}

function pageImageManifest(directory) {
  const names = readdirSync(join(root, directory))
    .filter((name) => /^page-\d{2}\.png$/.test(name))
    .sort();
  const manifest = names.map((name) => `${name}=${fileSha256(`${directory}/${name}`)}\n`).join("");
  return {
    count: names.length,
    sha256: createHash("sha256").update(manifest, "utf8").digest("hex").toUpperCase(),
  };
}

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function missingFields(value, fields) {
  if (!isObject(value)) return fields;
  return fields.filter((field) => !(field in value));
}

function countStates(entries, field) {
  const counts = {};
  for (const entry of entries) counts[entry[field]] = (counts[entry[field]] ?? 0) + 1;
  return Object.fromEntries(Object.entries(counts).sort(([left], [right]) => left.localeCompare(right)));
}

function allFiles(start) {
  const absolute = join(root, start);
  if (!existsSync(absolute)) return [];
  const result = [];
  const stack = [absolute];
  while (stack.length > 0) {
    const current = stack.pop();
    for (const name of readdirSync(current)) {
      const child = join(current, name);
      const projectPath = relative(root, child).replaceAll("\\", "/");
      const status = lstatSync(child);
      if (status.isSymbolicLink()) {
        result.push({ path: projectPath, symlink: true });
      } else if (status.isDirectory()) {
        const firstSegment = projectPath.split("/")[0];
        if (!ignoredDirectories.has(firstSegment) && !ignoredDirectories.has(name)) stack.push(child);
      } else if (!isIgnoredLocalFile(projectPath)) {
        result.push({ path: projectPath, symlink: false });
      }
    }
  }
  return result.sort((left, right) => left.path.localeCompare(right.path));
}

function includesEvery(text, needles) {
  const normalized = text.toLowerCase();
  return needles.every((needle) => normalized.includes(needle.toLowerCase()));
}

function normalizedText(text) {
  return text.replaceAll("\r\n", "\n");
}

const contract = readJson("tests/phase1/policy-contract.json");
if (!contract) process.exit(1);
record(
  "VER-CI-0001",
  process.version === `v${contract.expectedToolchain.node}`,
  `Verifier runtime is ${process.version}; required runtime is v${contract.expectedToolchain.node}`,
);
const extractorCheck = spawnSync(
  process.platform === "win32" ? "python" : "python3",
  ["-B", "tools/phase1/extract_directive_requirements.py", "--check", "--root", "."],
  { cwd: root, encoding: "utf8", shell: false },
);
record(
  "VER-REQ-0001",
  extractorCheck.status === 0,
  extractorCheck.status === 0
    ? "Fresh deterministic extraction exactly matches both committed requirement snapshots"
    : `Deterministic extraction check failed with status ${extractorCheck.status ?? "unavailable"}`,
);

for (const source of contract.sourceFiles) {
  const present = existsSync(join(root, source.path));
  record("VER-GOV-0001", present, present ? `${source.path} exists` : `${source.path} is missing`);
  if (present) {
    const actual = fileSha256(source.path);
    record(
      "VER-GOV-0001",
      actual === source.sha256,
      actual === source.sha256
        ? `${source.path} matches ${actual}`
        : `${source.path} expected ${source.sha256}, got ${actual}`,
    );
  }
}

for (const path of contract.requiredFiles) {
  const absolute = join(root, path);
  const valid = existsSync(absolute) && statSync(absolute).isFile() && statSync(absolute).size > 0;
  record("VER-DOC-0001", valid, valid ? `${path} exists and is non-empty` : `${path} is absent or empty`);
}

const adr1Path = "ADR/0001-no-physical-industrial-communication.md";
if (existsSync(join(root, adr1Path))) {
  const adr1 = readText(adr1Path);
  record(
    "VER-DOC-0002",
    adr1.includes("# Physical Industrial Communication Is Permanently Out of Scope"),
    "ADR-0001 contains the exact mandated title",
  );
  record(
    "VER-DOC-0002",
    /Status:\s*Project Safety Invariant\b/.test(adr1),
    "ADR-0001 contains the exact mandated status",
  );
  record(
    "VER-DOC-0002",
    includesEvery(adr1, ["cannot be amended", "separate repository", "authorization", "legal", "threat model", "governance"]),
    "ADR-0001 makes a physical-capable product a separately authorized and governed repository",
  );
  record(
    "VER-DOC-0002",
    includesEvery(adr1, ["applicability is not discretionary", "missing required behavior", "host network adapters"]),
    "ADR-0001 closes isolation-gate applicability and host-adapter ambiguity",
  );
}

const architectureAdrs = [
  {
    path: "ADR/0002-original-project-format.md",
    terms: ["status: proposed", "approval: not yet recorded", ".vlabproj", ".vlabarchive", "non-executable", "oq-0005"],
  },
  {
    path: "ADR/0003-unified-plc-ir.md",
    terms: ["status: proposed", "approval: not yet recorded", "one versioned, typed, serializable plc ir", "one virtual controller runtime", "never passed to `eval`"],
  },
  {
    path: "ADR/0004-deterministic-virtual-time.md",
    terms: ["status: proposed", "approval: not yet recorded", "simulator-controlled monotonic virtual time", "cannot be authoritative plc/process time", "replay identity"],
  },
];
for (const adr of architectureAdrs) {
  if (!existsSync(join(root, adr.path))) continue;
  const text = readText(adr.path);
  record(
    "VER-DOC-0001",
    includesEvery(text, adr.terms) && !/Status:\s*Accepted/i.test(text),
    `${adr.path} documents its mandated boundary without claiming unrecorded approval`,
  );
}

const docxVisualQa = existsSync(join(root, "docs/governance/DOCX_VISUAL_QA.md"))
  ? readText("docs/governance/DOCX_VISUAL_QA.md")
  : "";
const docxVisualContract = contract.docxVisualQaObservation;
record(
  "VER-DOC-0001",
  includesEvery(docxVisualQa, [
    contract.sourceFiles[1].sha256,
    "all 40 rendered pages were then inspected individually at full-page resolution",
    "zero out-of-bounds words",
    "zero unicode replacement glyphs",
    "observation pass for the rendered current source hash",
    "not admissible as phase 1 gate evidence",
    "outside the standard-library-only local bootstrap exception",
    "does not satisfy the directive's visual-qa acceptance gate",
    "preview layer intermittently masked",
    "raw-pixel comparison",
    "source docx was not saved, edited, renamed, or replaced",
  ]),
  "DOCX observation record binds all-page inspection to the current source hash and denies gate, tool, or reviewer acceptance",
);

const visualEvidenceFiles = [
  docxVisualContract.derivedPdf,
  docxVisualContract.machineAnalysis,
];
const visualEvidencePresence = visualEvidenceFiles.map((entry) => existsSync(join(root, entry.path)));
const visualPagesPresent = existsSync(join(root, docxVisualContract.pagesDirectory));
const visualEvidencePresentCount = visualEvidencePresence.filter(Boolean).length + (visualPagesPresent ? 1 : 0);
let docxVisualQaLocalEvidenceStatus = "ABSENT_IGNORED_LOCAL_EVIDENCE";

if (visualEvidencePresentCount === 0) {
  record(
    "VER-DOC-0001",
    true,
    "Ignored DOCX observation binaries are absent; the Markdown record remains an unapproved observation, not portable gate evidence",
  );
} else if (visualEvidencePresentCount !== visualEvidenceFiles.length + 1) {
  docxVisualQaLocalEvidenceStatus = "PARTIAL_LOCAL_EVIDENCE_INVALID";
  record(
    "VER-DOC-0001",
    false,
    "Ignored DOCX observation material is only partially present; remove it or reproduce the complete hash-bound set",
  );
} else {
  const pdfMatches = fileSha256(docxVisualContract.derivedPdf.path) === docxVisualContract.derivedPdf.sha256;
  const analysisMatches =
    fileSha256(docxVisualContract.machineAnalysis.path) === docxVisualContract.machineAnalysis.sha256;
  const pages = pageImageManifest(docxVisualContract.pagesDirectory);
  const pageCountMatches = pages.count === docxVisualContract.expectedPageCount;
  const pageManifestMatches = pages.sha256 === docxVisualContract.pagesManifestSha256;
  const localEvidenceMatches =
    pdfMatches && analysisMatches && pageCountMatches && pageManifestMatches;
  docxVisualQaLocalEvidenceStatus = localEvidenceMatches
    ? "AVAILABLE_HASH_VALIDATED_UNAPPROVED"
    : "AVAILABLE_HASH_MISMATCH";
  record(
    "VER-DOC-0001",
    pdfMatches,
    `Ignored rendered PDF ${pdfMatches ? "matches" : "does not match"} its observation hash`,
  );
  record(
    "VER-DOC-0001",
    analysisMatches,
    `Ignored machine-analysis JSON ${analysisMatches ? "matches" : "does not match"} its observation hash`,
  );
  record(
    "VER-DOC-0001",
    pageCountMatches,
    `Ignored rendered page set contains ${pages.count}/${docxVisualContract.expectedPageCount} PNGs`,
  );
  record(
    "VER-DOC-0001",
    pageManifestMatches,
    `Ignored rendered page manifest ${pageManifestMatches ? "matches" : "does not match"} ${pages.sha256}`,
  );
}

const securityInvariants = existsSync(join(root, "SECURITY_INVARIANTS.md"))
  ? readText("SECURITY_INVARIANTS.md")
  : "";
const threatModel = existsSync(join(root, "THREAT_MODEL.md")) ? readText("THREAT_MODEL.md") : "";
record(
  "VER-ISO-0001",
  includesEvery(securityInvariants, [
    "gate applicability is not discretionary",
    "absence never turns a gate into `n/a`",
    "production-bundle evidence exclusion",
    "pes-crm-0010",
  ]),
  "Security invariants define non-discretionary gates, evidence exclusion, and the vendor-observation stop",
);
record(
  "VER-ISO-0001",
  includesEvery(threatModel, [
    "simulator-owned hostile fixtures",
    "physical equipment is never an acceptance-test target",
    "host network adapters absent",
    "research notes, evidence records, quarantined material",
  ]),
  "Threat model constrains export tests, host adapters, and production evidence separation",
);

const registry = readJson("requirements/phase1-requirements.json");
const matrix = readJson("IMPLEMENTATION_MATRIX.json");
let requirementIds = new Set();
let requirementsById = new Map();
if (registry && matrix) {
  const requirements = Array.isArray(registry.requirements) ? registry.requirements : [];
  const ids = requirements.map((item) => item.id);
  requirementIds = new Set(ids);
  requirementsById = new Map(requirements.map((item) => [item.id, item]));
  record(
    "VER-REQ-0001",
    registry.requirementCount === contract.expectedRequirementCount && requirements.length === contract.expectedRequirementCount,
    `Requirement registry contains ${requirements.length}/${contract.expectedRequirementCount} records`,
  );
  record("VER-REQ-0001", requirementIds.size === ids.length, `Unique requirement IDs: ${requirementIds.size}/${ids.length}`);
  record(
    "VER-REQ-0001",
    ids.every((id) => /^PES-[A-Z]+-\d{4}$/.test(id)),
    "All requirement IDs match PES-AREA-NNNN",
  );

  const fields = [
    "id",
    "title",
    "normativeKeyword",
    "atomicRequirement",
    "atomicity",
    "rationale",
    "scopeComponent",
    "sourcePointer",
    "ipClassification",
    "candidateIpFlags",
    "dependencies",
    "relatedRequirements",
    "dependencyMaturity",
    "targetMilestone",
    "phase1Disposition",
    "truthState",
    "statusNote",
    "positiveAcceptance",
    "negativeAcceptance",
    "acceptanceMaturity",
    "verificationIds",
    "adrDecisionChangeLinks",
    "implementationComponents",
    "owner",
    "reviewer",
    "reviewStatus",
    "reviewDate",
  ];
  const missing = requirements.flatMap((item) => missingFields(item, fields).map((field) => `${item.id}.${field}`));
  record(
    "VER-REQ-0002",
    missing.length === 0,
    missing.length === 0 ? "All requirement records satisfy the Phase 1 schema" : `Missing: ${missing.join(", ")}`,
  );
  record(
    "VER-REQ-0001",
    requirements.every(
      (item) =>
        isObject(item.sourcePointer) &&
        item.sourcePointer.file === contract.sourceFiles[1].path &&
        item.sourcePointer.sha256 === contract.sourceFiles[1].sha256 &&
        Number.isInteger(item.sourcePointer.bodyBlock) &&
        item.sourcePointer.bodyBlock > 0 &&
        Array.isArray(item.sourcePointer.headingPath) &&
        item.sourcePointer.headingPath.length > 0,
    ),
    "Every requirement has a non-empty heading path and hash-bound directive pointer",
  );
  const structuralSpillPattern = /^(?:Phase \d(?: of \d)?:.*|Appendix [A-Z](?:\.|:).*|Open Questions|Risk Register|Construction-Phase Ledger|Phase 1 Contents)$/im;
  record(
    "VER-REQ-0001",
    requirements.every(
      (item) =>
        typeof item.atomicRequirement === "string" &&
        item.atomicRequirement.length > 0 &&
        item.atomicRequirement.length < 5000 &&
        !/\[PES-[A-Z]+-\d{4}\]/.test(item.atomicRequirement) &&
        !structuralSpillPattern.test(item.atomicRequirement),
    ),
    "No requirement contains a later requirement marker or document-structure spillover",
  );
  record(
    "VER-REQ-0002",
    requirements.every((item) => contract.validTruthStates.includes(item.truthState)),
    "Every requirement uses a valid truth state",
  );
  const extractorHash = fileSha256("tools/phase1/extract_directive_requirements.py");
  record(
    "VER-REQ-0001",
    registry.schemaVersion === 2 &&
      matrix.schemaVersion === 2 &&
      registry.generatorSha256 === extractorHash &&
      matrix.generatorSha256 === extractorHash,
    "Both generated snapshots are schema v2 and are bound to the current extractor SHA-256",
  );
  record(
    "VER-REQ-0002",
    requirements.every(
      (item) =>
        isObject(item.ipClassification) &&
        Array.isArray(item.ipClassification.classes) &&
        item.ipClassification.classes.length > 0 &&
        item.ipClassification.classes.every((value) => Number.isInteger(value) && value >= 1 && value <= 9) &&
        typeof item.ipClassification.disposition === "string" &&
        typeof item.ipClassification.basis === "string" &&
        ["CURATED_PHASE_1_REQUIREMENT_ID_REVIEW", "UNRESOLVED_DEFAULT_CLASS_8"].includes(
          item.ipClassification.classificationMethod,
        ) &&
        isObject(item.candidateIpFlags) &&
        Array.isArray(item.candidateIpFlags.classes) &&
        item.candidateIpFlags.purpose.includes("NON_NORMATIVE_TRIAGE_ONLY"),
    ),
    "Every requirement separates reviewed/default IP disposition from non-normative keyword triage",
  );
  record(
    "VER-REQ-0002",
    requirements
      .filter((item) => item.ipClassification.classes.some((value) => value === 7 || value === 8))
      .every((item) => ["BLOCKED", "NOT_STARTED", "DEFERRED"].includes(item.truthState)),
    "Class 7/8 requirements are not represented as implemented or complete",
  );
  const laterExecutableIds = ["PES-ISO-0015", "PES-ISO-0016", "PES-SEC-0021", "PES-DEV-0004"];
  record(
    "VER-REQ-0002",
    laterExecutableIds.every((id) => {
      const item = requirementsById.get(id);
      return (
        item?.phase1Disposition === "RESERVED_LATER_PHASE_NO_PRODUCT_AUTHORIZATION" &&
        /later|reserved/i.test(item.targetMilestone) &&
        item.truthState === "NOT_STARTED"
      );
    }),
    "Representative executable/runtime/release obligations are reserved for later phases, not Phase 1",
  );
  const verifiedIds = requirements.filter((item) => item.truthState === "VERIFIED").map((item) => item.id).sort();
  const expectedVerifiedIds = [...contract.expectedVerifiedRequirementIds].sort();
  record(
    "VER-REQ-0002",
    JSON.stringify(verifiedIds) === JSON.stringify(expectedVerifiedIds),
    "The generated baseline contains no self-certified VERIFIED requirements",
  );

  const expectedFoundationMappings = contract.expectedFoundationMappings;
  const expectedFoundationIds = Object.keys(expectedFoundationMappings).sort();
  const curatedFoundationIds = requirements
    .filter((item) => item.acceptanceMaturity === "CURATED_PHASE_1_CURRENT_SCOPE")
    .map((item) => item.id)
    .sort();
  record(
    "VER-REQ-0002",
    JSON.stringify(curatedFoundationIds) === JSON.stringify(expectedFoundationIds),
    "The curated Phase 1 acceptance set exactly matches the independent policy-contract allowlist",
  );
  for (const [id, expected] of Object.entries(expectedFoundationMappings)) {
    const item = requirementsById.get(id);
    const criteriaAreCurated =
      typeof item?.positiveAcceptance === "string" &&
      item.positiveAcceptance.length >= 80 &&
      typeof item?.negativeAcceptance === "string" &&
      item.negativeAcceptance.length >= 60 &&
      !item.positiveAcceptance.startsWith("Verification demonstrates");
    record(
      "VER-REQ-0002",
      item?.truthState === "IMPLEMENTED_UNVERIFIED" &&
        JSON.stringify(item.verificationIds) === JSON.stringify(expected.verificationIds) &&
        JSON.stringify(item.implementationComponents) === JSON.stringify(expected.implementationComponents) &&
        item.implementationComponents.every((path) => existsSync(join(root, path))) &&
        Array.isArray(item.dependencies) &&
        item.dependencies.length === 0 &&
        Array.isArray(item.relatedRequirements) &&
        item.relatedRequirements.every((relatedId) => requirementIds.has(relatedId)) &&
        item.dependencyMaturity.startsWith("CURATED_PHASE_1_RELATIONSHIPS") &&
        criteriaAreCurated &&
        item.reviewer === "UNASSIGNED" &&
        item.reviewStatus.includes("reviewer acceptance is not recorded"),
      `${id} has exact check/component mappings and remains IMPLEMENTED_UNVERIFIED pending acceptance`,
    );
  }
  record(
    "VER-REQ-0002",
    requirements
      .filter((item) => !expectedFoundationMappings[item.id])
      .every(
        (item) =>
          Array.isArray(item.dependencies) &&
          Array.isArray(item.relatedRequirements) &&
          item.dependencyMaturity.startsWith("UNRESOLVED_BASELINE"),
      ),
    "Unreviewed later requirements label empty dependency fields as unresolved rather than no-dependency assertions",
  );

  const entries = Array.isArray(matrix.entries) ? matrix.entries : [];
  const matrixIds = entries.map((item) => item.requirementId);
  const entriesById = new Map(entries.map((item) => [item.requirementId, item]));
  record(
    "VER-REQ-0002",
    matrix.requirementCount === requirements.length && entries.length === requirements.length,
    `Implementation matrix contains ${entries.length}/${requirements.length} entries`,
  );
  record(
    "VER-REQ-0002",
    new Set(matrixIds).size === matrixIds.length &&
      ids.every((id) => entriesById.has(id)) &&
      entries.every((item) => requirementIds.has(item.requirementId)),
    "Requirement registry and implementation matrix have identical, unique ID coverage",
  );
  record(
    "VER-REQ-0002",
    entries.every(
      (entry) =>
        contract.validTruthStates.includes(entry.truthState) &&
        requirementsById.get(entry.requirementId)?.truthState === entry.truthState &&
        requirementsById.get(entry.requirementId)?.phase1Disposition === entry.phase1Disposition &&
        JSON.stringify(requirementsById.get(entry.requirementId)?.verificationIds) === JSON.stringify(entry.verificationIds) &&
        JSON.stringify(requirementsById.get(entry.requirementId)?.implementationComponents) ===
          JSON.stringify(entry.implementationComponents),
    ),
    "Every matrix entry matches registry state, verification IDs, and implementation components",
  );
  const actualStateCounts = countStates(entries, "truthState");
  record(
    "VER-REQ-0002",
    JSON.stringify(actualStateCounts) === JSON.stringify(matrix.stateCounts),
    `Matrix state counts are exact: ${JSON.stringify(actualStateCounts)}`,
  );
  record(
    "VER-REQ-0002",
    matrix.completionRule === "Only VERIFIED means complete. No completion percentage is calculated." &&
      !Object.keys(matrix).some((key) => /percent|percentage/i.test(key)),
    "Matrix defines VERIFIED as the sole completion state and contains no completion percentage",
  );
  const scaffolded = requirements.filter((item) => item.truthState === "SCAFFOLDED");
  record(
    "VER-QLT-0001",
    scaffolded.length === 1 &&
      scaffolded[0].id === "PES-DEV-0006" &&
      scaffolded[0].owner === "Scott" &&
      scaffolded[0].targetMilestone === "Phase 1 governance foundation" &&
      scaffolded[0].phase1Disposition === "FOUNDATION_WORK_ONLY",
    "Exactly one non-product workspace foundation is SCAFFOLDED with owner, target, and zero completion credit",
  );
}

const verificationPlan = existsSync(join(root, "docs/governance/PHASE_1_VERIFICATION_PLAN.md"))
  ? readText("docs/governance/PHASE_1_VERIFICATION_PLAN.md")
  : "";
record(
  "VER-REQ-0002",
  contract.verificationIds.every((id) => verificationPlan.includes(`\`${id}\``)),
  "Every automated verification ID is documented in the verification plan",
);

const evidence = readJson("EVIDENCE_REGISTER.json");
const assets = readJson("ASSET_PROVENANCE.json");
if (evidence) {
  const sources = Array.isArray(evidence.sources) ? evidence.sources : [];
  const rows = Array.isArray(evidence.requirementEvidence) ? evidence.requirementEvidence : [];
  const sourceFields = [
    "sourceId",
    "title",
    "publisher",
    "version",
    "publicationDate",
    "durableLocation",
    "accessDate",
    "sha256",
    "sourceType",
    "reviewStatus",
    "limitations",
  ];
  const evidenceFields = [
    "evidenceRecordId",
    "requirementId",
    "title",
    "paraphrasedObservedBehavior",
    "sourceId",
    "sourcePointer",
    "researchClassification",
    "subjectIpClass",
    "subjectIpDisposition",
    "simulatorOwnedImplementationRequirement",
    "forbiddenImplementationShortcut",
    "author",
    "reviewer",
    "reviewStatus",
    "reviewDate",
    "recordStatus",
    "implementationComponent",
    "verificationIds",
  ];
  const sourceIds = new Set(sources.map((item) => item.sourceId));
  const evidenceIds = new Set(rows.map((item) => item.evidenceRecordId));
  const combinedIds = [...sourceIds, ...evidenceIds];
  record(
    "VER-CRM-0001",
    sources.length >= 3 &&
      sources.every((item) => missingFields(item, sourceFields).length === 0) &&
      rows.length > 0 &&
      rows.every((item) => missingFields(item, evidenceFields).length === 0),
    `Evidence register has ${sources.length} normalized sources and ${rows.length} schema-complete requirement-evidence rows`,
  );
  record(
    "VER-CRM-0001",
    combinedIds.length === new Set(combinedIds).size && combinedIds.every((id) => /^SRC-\d{4}$/.test(id)),
    "Source and evidence records share one unique SRC-NNNN namespace",
  );
  record(
    "VER-CRM-0001",
    rows.every(
      (item) =>
        requirementIds.has(item.requirementId) &&
        sourceIds.has(item.sourceId) &&
        Number.isInteger(item.subjectIpClass) &&
        item.subjectIpClass >= 1 &&
        item.subjectIpClass <= 9 &&
        typeof item.subjectIpDisposition === "string" &&
        item.subjectIpDisposition.length > 0 &&
        contract.validTruthStates.includes(item.recordStatus) &&
        Array.isArray(item.verificationIds) &&
        (item.reviewStatus !== "APPROVED" ||
          (typeof item.reviewer === "string" && item.reviewer.length > 0 && item.reviewDate && item.verificationIds.length > 0)),
    ),
    "Evidence rows resolve requirements/sources and cannot claim approval without reviewer, date, and verification",
  );
  const hashBoundSources = sources.filter((item) => item.durableLocation.startsWith("project://"));
  record(
    "VER-CRM-0001",
    hashBoundSources.every((item) => {
      const path = item.durableLocation.slice("project://".length);
      return existsSync(join(root, path)) && fileSha256(path) === item.sha256;
    }),
    `All ${hashBoundSources.length} project-local evidence sources match their SHA-256 records`,
  );
  record(
    "VER-CRM-0001",
    evidence.status === "PHASE_1_DRAFT_UNREVIEWED" &&
      rows.every((item) => item.recordStatus !== "VERIFIED" && item.reviewStatus !== "APPROVED") &&
      JSON.stringify(evidence).includes("UNRESOLVED") &&
      evidence.unresolvedCitationState?.truthState === "BLOCKED",
    "Evidence register truthfully remains unreviewed and blocks unresolved citation metadata",
  );
}

if (assets) {
  const assetEntries = Array.isArray(assets.assets) ? assets.assets : [];
  const requiredAssetFields = Array.isArray(assets.requiredAssetFields) ? assets.requiredAssetFields : [];
  const productionRoot = assets.policy?.productionAssetRoot;
  const evidenceOnlyFiles = Array.isArray(assets.evidenceOnlyFiles) ? assets.evidenceOnlyFiles : [];
  record(
    "VER-CRM-0001",
    requiredAssetFields.length > 0 &&
      assetEntries.every(
        (item) =>
          missingFields(item, requiredAssetFields).length === 0 &&
          item.reviewStatus === assets.policy.requiredApprovalStatus &&
          typeof item.filePath === "string" &&
          item.filePath.startsWith(productionRoot) &&
          existsSync(join(root, item.filePath)) &&
          fileSha256(item.filePath) === item.currentHash,
      ),
    assetEntries.length === 0
      ? "Asset schema is defined and no production asset is registered or shipped"
      : `All ${assetEntries.length} production assets are complete, approved, and hash-bound`,
  );
  record(
    "VER-CRM-0001",
    assets.inventorySummary?.registeredAssetCount === assetEntries.length &&
      assets.inventorySummary?.approvedAssetCount ===
        assetEntries.filter((item) => item.reviewStatus === assets.policy.requiredApprovalStatus).length &&
      (assetEntries.length > 0 || assets.inventorySummary?.releaseEligible === false),
    "Asset inventory counts are exact and an empty inventory is not release-eligible",
  );
  record(
    "VER-CRM-0001",
    evidenceOnlyFiles.every(
      (item) =>
        !item.path.startsWith(productionRoot) &&
        existsSync(join(root, item.path)) &&
        fileSha256(item.path) === item.sha256,
    ),
    "Every evidence-only file is outside the production asset root and hash-bound",
  );
}

const attestation = existsSync(join(root, "CONTRIBUTOR_CLEAN_ROOM_ATTESTATION.md"))
  ? readText("CONTRIBUTOR_CLEAN_ROOM_ATTESTATION.md")
  : "";
const cleanRoomPolicy = existsSync(join(root, "CLEAN_ROOM_POLICY.md")) ? readText("CLEAN_ROOM_POLICY.md") : "";
record(
  "VER-CRM-0001",
  includesEvery(cleanRoomPolicy, [
    "permitted evidence",
    "forbidden material",
    "required evidence record before implementation",
    "ip classification gate",
    "independent implementation and original expression",
    "contributor attestation",
    "contamination response",
    "merge and release gates",
  ]),
  "Clean-room policy defines source admission, forbidden material, evidence, originality, contributor, quarantine, and merge controls",
);
record(
  "VER-CRM-0001",
  includesEvery(attestation, [
    "not an attestation until completed, signed, and reviewed",
    "pes-crm-0024",
    "transitive",
    "optional",
    "native",
    "webassembly",
    "font",
    "packaging",
  ]) && attestation.includes("`UNCOMPLETED`"),
  "Contributor form covers the full dependency graph and does not impersonate a completed attestation",
);

const toolchainRegister = existsSync(join(root, "docs/governance/TOOLCHAIN_ADMISSION_REGISTER.md"))
  ? readText("docs/governance/TOOLCHAIN_ADMISSION_REGISTER.md")
  : "";
record(
  "VER-CI-0001",
  includesEvery(toolchainRegister, [
    "local bootstrap contained; remote ci blocked; all tools unapproved",
    "blocked_dec_0002_and_admission",
    "tools/phase1/run_phase1_verification.py",
    "node.js",
    contract.expectedToolchain.node,
    "pnpm",
    contract.expectedToolchain.pnpm,
    "python",
    contract.expectedToolchain.python,
    "rust compiler",
    contract.expectedToolchain.rust,
    contract.expectedToolchain.runner,
    ...Object.values(contract.expectedActionPins),
  ]),
  "Toolchain admission register inventories every declared Phase 1 tool without claiming approval",
);
const toolchainRecords = toolchainRegister
  .split(/^### TC-\d{4}[^\n]*$/m)
  .slice(1)
  .map((section) => section.split(/^## \d/m)[0]);
record(
  "VER-CI-0001",
  toolchainRecords.length === contract.expectedToolchainRecordCount &&
    toolchainRecords.every((section) => {
      const reviewerRows = section.match(/^\| Reviewer\/decision\/date \|.*$/gm) ?? [];
      return (
        reviewerRows.length === 1 &&
        reviewerRows[0] === "| Reviewer/decision/date | `UNASSIGNED` / `NOT_REVIEWED` / `null` |" &&
        !section.includes("APPROVED")
      );
    }),
  `All ${contract.expectedToolchainRecordCount} toolchain records have exactly one unassigned/not-reviewed disposition and no approval claim`,
);

const openDecisions = existsSync(join(root, "OPEN_DECISIONS.md")) ? readText("OPEN_DECISIONS.md") : "";
const risks = existsSync(join(root, "RISK_REGISTER.md")) ? readText("RISK_REGISTER.md") : "";
for (const id of new Set([...contract.openQuestionIds, ...contract.blockedDecisionIds])) {
  record("VER-DEC-0001", openDecisions.includes(id), `${id} is recorded in OPEN_DECISIONS.md`);
}
record(
  "VER-DEC-0001",
  /DEC-0001[\s\S]{0,500}BLOCKED/i.test(openDecisions),
  "DEC-0001 is explicitly BLOCKED rather than silently resolved",
);
record(
  "VER-DEC-0001",
  /DEC-0002[\s\S]{0,500}BLOCKED/i.test(openDecisions),
  "DEC-0002 is explicitly BLOCKED rather than silently authorizing remote services",
);
for (const id of contract.riskIds) {
  record("VER-DEC-0001", risks.includes(id), `${id} is recorded in RISK_REGISTER.md`);
}

const reservedRoots = [
  "apps",
  "packages",
  "profiles",
  "scenarios",
  "assets/original",
  "artifacts",
  "build",
  "dist",
];
for (const path of reservedRoots) {
  const absent = !existsSync(join(root, path));
  record(
    "VER-QLT-0001",
    absent,
    absent ? `${path} is intentionally absent` : `${path} exists before its feature scope is authorized`,
  );
}

const readme = existsSync(join(root, "README.md")) ? readText("README.md") : "";
const scopeAudit = existsSync(join(root, "docs/governance/PHASE_1_SCOPE_AUDIT.md"))
  ? readText("docs/governance/PHASE_1_SCOPE_AUDIT.md")
  : "";
const directiveLog = existsSync(join(root, "CHANGELOG_DIRECTIVE.md")) ? readText("CHANGELOG_DIRECTIVE.md") : "";
record(
  "VER-QLT-0001",
  includesEvery(readme, ["product implementation: not started", "phases 2-4: not authorized", "only `verified` means complete"]) &&
    includesEvery(scopeAudit, ["phase 1 exit not passed", "no phase 2-4 product feature work", "does not mark phase 1 complete"]) &&
    includesEvery(directiveLog, ["phase 1 exit gate is not claimed as passed", "no phase 2-4 product feature work"]),
  "README, scope audit, and directive log reject Phase 1/product/master-directive completion claims",
);

for (const base of ["apps", "packages"]) {
  const absolute = join(root, base);
  if (!existsSync(absolute)) continue;
  for (const name of readdirSync(absolute)) {
    record(
      "VER-ISO-0001",
      !contract.forbiddenTopLevelPackageNames.includes(name.toLowerCase()),
      `${base}/${name} is not a forbidden package boundary`,
    );
  }
}

const projectFiles = allFiles(".");
const controlledInputPaths = new Set([
  ...contract.sourceFiles.map((item) => item.path),
  ...contract.requiredFiles,
]);
const uncontrolledFiles = projectFiles.filter((item) => !item.symlink && !controlledInputPaths.has(item.path));
const controlledProjectFiles = projectFiles.filter((item) => !item.symlink && controlledInputPaths.has(item.path));
record(
  "VER-QLT-0001",
  uncontrolledFiles.length === 0,
  uncontrolledFiles.length === 0
    ? "Every non-local project file is registered in the Phase 1 policy contract"
    : `${uncontrolledFiles.length} unregistered project file(s) exist; inspect locally without publishing their paths or hashes`,
);
record(
  "VER-CI-0001",
  controlledProjectFiles.length === controlledInputPaths.size,
  `Controlled report inputs present: ${controlledProjectFiles.length}/${controlledInputPaths.size}`,
);
record(
  "VER-ISO-0001",
  projectFiles.every((item) => !item.symlink),
  "Phase 1 project files contain no symbolic links that could escape the repository",
);

if (requirementIds.size > 0) {
  const textFiles = controlledProjectFiles.filter((item) => /\.(?:md|json|mjs|py|toml|ya?ml)$/i.test(item.path));
  const unknownReferences = [];
  for (const file of textFiles) {
    const references = readText(file.path).match(/\bPES-[A-Z]+-\d{4}\b/g) ?? [];
    for (const id of new Set(references)) {
      if (!requirementIds.has(id)) unknownReferences.push(`${file.path}:${id}`);
    }
  }
  record(
    "VER-REQ-0001",
    unknownReferences.length === 0,
    unknownReferences.length === 0
      ? `All PES references across ${textFiles.length} text artifacts resolve to the registry`
      : `Unknown requirement references: ${unknownReferences.join(", ")}`,
  );
}

const packageJson = readJson("package.json");
if (packageJson) {
  const expectedPackageJson = {
    name: "plc-engineering-simulator-governance",
    version: "0.0.0-phase1",
    private: true,
    description: "Phase 1 governance and verification foundation for an offline educational PLC engineering simulator.",
    type: "module",
    packageManager: `pnpm@${contract.expectedToolchain.pnpm}`,
    engines: {
      node: contract.expectedToolchain.node,
      pnpm: contract.expectedToolchain.pnpm,
    },
    scripts: {
      test: "pnpm verify:phase1",
      "verify:phase1": "python -B tools/phase1/run_phase1_verification.py",
      "requirements:extract": "python -B tools/phase1/extract_directive_requirements.py --root .",
      "requirements:check": "python -B tools/phase1/extract_directive_requirements.py --check --root .",
    },
  };
  const dependencyFields = ["dependencies", "devDependencies", "optionalDependencies", "peerDependencies"];
  const dependencyCount = dependencyFields.reduce(
    (total, field) => total + Object.keys(packageJson[field] ?? {}).length,
    0,
  );
  record("VER-CI-0001", dependencyCount === 0, `Root production/development dependency count is ${dependencyCount}`);
  record(
    "VER-CI-0001",
    JSON.stringify(packageJson) === JSON.stringify(expectedPackageJson),
    "Root package manifest exactly matches the dependency-free Phase 1 contract and has no lifecycle hooks",
  );
  record(
    "VER-CI-0001",
    packageJson.packageManager === `pnpm@${contract.expectedToolchain.pnpm}` &&
      packageJson.engines?.node === contract.expectedToolchain.node &&
      packageJson.engines?.pnpm === contract.expectedToolchain.pnpm,
    "Node and pnpm declarations are exact and match the Phase 1 policy contract",
  );
}

record(
  "VER-CI-0001",
  readText(".python-version").trim() === contract.expectedToolchain.python,
  `Python extractor version is exactly ${contract.expectedToolchain.python}`,
);
const rustToolchain = readText("rust-toolchain.toml");
record(
  "VER-CI-0001",
  normalizedText(rustToolchain) ===
    `[toolchain]\nchannel = "${contract.expectedToolchain.rust}"\nprofile = "minimal"\n`,
  `Rust toolchain is exactly ${contract.expectedToolchain.rust}, minimal, with no extra targets or components`,
);
const gitAttributes = readText(".gitattributes");
record(
  "VER-CI-0001",
  includesEvery(gitAttributes, [
    "*.json text eol=lf",
    "*.md text eol=lf",
    "*.mjs text eol=lf",
    "*.py text eol=lf",
    '"Govs PLC project Research Report.md" -text',
    "*.docx -text",
  ]),
  "Git attributes preserve byte-hashed source inputs and normalize generated text deterministically",
);
const cargoManifest = readText("Cargo.toml");
record(
  "VER-CI-0001",
  normalizedText(cargoManifest) ===
    `[workspace]\nmembers = []\nresolver = "3"\n\n[workspace.package]\nversion = "0.0.0"\nedition = "2024"\nrust-version = "${contract.expectedToolchain.rust}"\n`,
  "Cargo manifest is the exact empty workspace and contains no dependencies, patches, profiles, or product crates",
);
record(
  "VER-CI-0001",
  normalizedText(readText("Cargo.lock")) ===
    "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 4\n",
  "Cargo.lock is the exact empty version-4 lockfile",
);
record(
  "VER-CI-0001",
  normalizedText(readText("pnpm-workspace.yaml")) ===
    'packages:\n  - "apps/*"\n  - "packages/*"\n\ncatalogMode: strict\ncleanupUnusedCatalogs: true\nlinkWorkspacePackages: false\npreferWorkspacePackages: false\n',
  "pnpm workspace contains only reserved absent roots and no catalog, dependency, injection, or link surface",
);
record(
  "VER-CI-0001",
  normalizedText(readText("pnpm-lock.yaml")) ===
    "lockfileVersion: '9.0'\n\nsettings:\n  autoInstallPeers: true\n  excludeLinksFromLockfile: false\n\nimporters:\n\n  .: {}\n",
  "pnpm lockfile has exactly one empty root importer and no packages or snapshots",
);

const workflowPath = ".github/workflows/phase1-governance.yml";
if (existsSync(join(root, workflowPath))) {
  const workflow = readText(workflowPath);
  const actionUses = [...workflow.matchAll(/uses:\s*([^@\s]+)@([^\s]+)/g)];
  const jobsSection = workflow.split(/^jobs:\s*$/m)[1] ?? "";
  const jobDeclarationLines = jobsSection
    .split(/\r?\n/)
    .filter((line) => /^  \S/.test(line) && !/^  #/.test(line));
  record(
    "VER-CI-0001",
    actionUses.length === Object.keys(contract.expectedActionPins).length &&
      actionUses.every((match) => contract.expectedActionPins[match[1]] === match[2] && /^[0-9a-f]{40}$/.test(match[2])),
    "Every GitHub Action dependency is allowlisted and pinned to its expected full commit SHA",
  );
  record(
    "VER-CI-0001",
    includesEvery(workflow, [
      `runs-on: ${contract.expectedToolchain.runner}`,
      `node-version: ${contract.expectedToolchain.node}`,
      `python-version: ${contract.expectedToolchain.python}`,
      "package-manager-cache: false",
      "persist-credentials: false",
      "contents: read",
      "ImageVersion",
      "extract_directive_requirements.py --check --root .",
      "node --check tools/phase1/verify-phase1.mjs",
      ".phase1-verification/phase1-report.json",
    ]) && !workflow.includes("windows-latest"),
    "Proposed disabled workflow declares fixed runtimes, snapshot/syntax checks, credential/cache limits, and report retention",
  );
  record(
    "VER-DEC-0001",
    /on:\s*\r?\n\s+workflow_dispatch:\s*(?:\r?\n|$)/.test(workflow) &&
      !/^\s*(?:push|pull_request):/m.test(workflow) &&
      jobDeclarationLines.length === 1 &&
      jobDeclarationLines[0] === "  verify-foundation:" &&
      (workflow.match(/^\s{4}if:\s*\$\{\{\s*false\s*\}\}\s*$/gm) ?? []).length === 1 &&
      workflow.includes("DEC-0002"),
    "Remote CI is workflow-dispatch-only, has exactly one job, and that job is literally disabled pending DEC-0002",
  );
}

const implementedCheckIds = new Set(checks.map((item) => item.id));
record(
  "VER-REQ-0002",
  contract.verificationIds.every((id) => implementedCheckIds.has(id)),
  "Every declared Phase 1 automated verification ID executes in this suite",
);

const artifactHashes = Object.fromEntries(
  [...controlledInputPaths].sort().map((path) => [path, fileSha256(path)]),
);
const report = {
  schemaVersion: 1,
  verificationSuite: "Phase 1 governance foundation",
  suiteVersion: "1.2.0",
  date: new Date().toISOString(),
  platform: `${process.platform}-${process.arch}`,
  nodeVersion: process.version,
  repositoryRoot: root,
  result: errors.length === 0 ? "PASS" : "FAIL",
  artifactManifestScope: "Every sourceFiles and requiredFiles path in tests/phase1/policy-contract.json",
  artifactHashes,
  docxVisualQaObservation: {
    recordedResult: docxVisualContract.recordedResult,
    admissionStatus: docxVisualContract.admissionStatus,
    localEvidenceStatus: docxVisualQaLocalEvidenceStatus,
  },
  checks,
  limitations: [
    "No product, WASM, packaged classroom artifact, or release candidate exists.",
    "Release isolation, offline-course, zero-attempt, export, and InternalTagBus product proofs remain NOT_STARTED.",
    "The evidence register and toolchain admission register remain explicitly unreviewed; this suite validates their structure and truthfulness, not legal approval.",
    "A completed contributor clean-room attestation and reviewer acceptance do not yet exist.",
    "The GitHub-hosted windows-2025 runner family is fixed, but each image revision remains externally mutable and must be captured per run.",
    "The current Word/Poppler all-page visual observation found no stored-render defect, but the tools were outside the standard-library bootstrap exception and remain unapproved; the visual-QA gate is unmet.",
    "The ignored rendered PDF, 40-page PNG set, and machine-analysis JSON are not portable controlled evidence; when present that contracted set is count/hash-validated, and when absent the report records that state without converting the Markdown observation into acceptance. Contact sheets and helper scripts are outside this validation.",
  ],
};
const reportPath = join(root, ".phase1-verification", "phase1-report.json");
mkdirSync(dirname(reportPath), { recursive: true });
writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");

for (const check of checks) {
  console.log(`${check.passed ? "PASS" : "FAIL"} ${check.id} ${check.detail}`);
}
console.log(`\nPhase 1 governance verification: ${report.result}`);
console.log(`Evidence: ${relative(root, reportPath)}`);
if (errors.length > 0) process.exit(1);
