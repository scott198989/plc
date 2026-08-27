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
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const EXIT_POLICY_FAILURE = 1;
const EXIT_TOOL_ERROR = 2;

process.on("uncaughtException", (error) => {
  console.error(`ERROR VER-RUN-0001 Internal verifier error: ${error.message}`);
  process.exitCode = EXIT_TOOL_ERROR;
});
process.on("unhandledRejection", (error) => {
  const detail = error instanceof Error ? error.message : String(error);
  console.error(`ERROR VER-RUN-0001 Internal verifier rejection: ${detail}`);
  process.exitCode = EXIT_TOOL_ERROR;
});

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
  "dist",
  "node_modules",
  "playwright-report",
  "target",
  "test-results",
]);
const ignoredProjectPaths = new Set(["apps/foundation-shell/src/generated"]);

function record(id, passed, detail) {
  checks.push({ id, passed, detail });
  if (!passed) errors.push(`${id}: ${detail}`);
}

function readText(path) {
  const absolute = join(root, path);
  if (!existsSync(absolute) || !statSync(absolute).isFile()) return "";
  return readFileSync(absolute, "utf8");
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
  const absolute = join(root, path);
  if (!existsSync(absolute) || !statSync(absolute).isFile()) return null;
  return createHash("sha256").update(readFileSync(absolute)).digest("hex").toUpperCase();
}

function sha256Bytes(bytes) {
  return createHash("sha256").update(bytes).digest("hex").toUpperCase();
}

function normalizedProjectPath(path) {
  return path.replaceAll("\\", "/");
}

function isSafeProjectPath(path) {
  return (
    typeof path === "string" &&
    path.length > 0 &&
    !isAbsolute(path) &&
    normalizedProjectPath(path) === path &&
    !path.split("/").includes("..") &&
    !path.startsWith("./")
  );
}

function argumentValue(name) {
  const index = process.argv.indexOf(name);
  if (index < 0 || index + 1 >= process.argv.length) return null;
  return process.argv[index + 1];
}

function fatalBaseline(message) {
  console.error(`ERROR VER-INT-0001 ${message}`);
  process.exit(EXIT_TOOL_ERROR);
}

function fatalTool(message) {
  console.error(`ERROR VER-RUN-0001 ${message}`);
  process.exit(EXIT_TOOL_ERROR);
}

function loadTrustedBaseline() {
  const manifestArgument = argumentValue("--baseline-manifest");
  const expectedSha256 = argumentValue("--baseline-manifest-sha256");
  const baselineCommit = argumentValue("--baseline-commit");
  if (!manifestArgument || !expectedSha256 || !baselineCommit) {
    fatalBaseline(
      "Required arguments: --baseline-manifest, --baseline-manifest-sha256, and --baseline-commit",
    );
  }
  if (!/^[0-9A-F]{64}$/i.test(expectedSha256)) {
    fatalBaseline("Trusted baseline manifest SHA-256 must contain exactly 64 hexadecimal characters");
  }
  if (!/^[0-9A-F]{40}$/i.test(baselineCommit)) {
    fatalBaseline("Trusted baseline commit must contain exactly 40 hexadecimal characters");
  }

  const absolute = resolve(manifestArgument);
  const fromRoot = relative(root, absolute);
  if (fromRoot === "" || (!fromRoot.startsWith("..") && !isAbsolute(fromRoot))) {
    fatalBaseline("Trusted baseline manifest must be supplied from outside the subject repository");
  }
  if (!existsSync(absolute) || !statSync(absolute).isFile()) {
    fatalBaseline(`Trusted baseline manifest is missing: ${absolute}`);
  }
  const bytes = readFileSync(absolute);
  const actualSha256 = sha256Bytes(bytes);
  if (actualSha256 !== expectedSha256.toUpperCase()) {
    fatalBaseline(
      `Trusted baseline manifest SHA-256 mismatch: expected ${expectedSha256.toUpperCase()}, got ${actualSha256}`,
    );
  }

  let parsed;
  try {
    parsed = JSON.parse(bytes.toString("utf8"));
  } catch (error) {
    fatalBaseline(`Trusted baseline manifest is not valid JSON: ${error.message}`);
  }
  if (
    !isObject(parsed) ||
    parsed.schemaVersion !== 1 ||
    typeof parsed.baselineId !== "string" ||
    parsed.baselineId.length === 0 ||
    parsed.hashAlgorithm !== "SHA-256" ||
    parsed.manifestPath !== "tests/phase1/trusted-baseline.json" ||
    parsed.scope !== "exact-project-file-set" ||
    !Array.isArray(parsed.excludedRoots) ||
    !Array.isArray(parsed.excludedPaths) ||
    !Array.isArray(parsed.files)
  ) {
    fatalBaseline("Trusted baseline manifest has an unsupported or incomplete schema");
  }

  const expectedExcludedRoots = [...ignoredDirectories].sort();
  if (JSON.stringify(parsed.excludedRoots) !== JSON.stringify(expectedExcludedRoots)) {
    fatalBaseline("Trusted baseline manifest excludedRoots do not match the verifier's fixed exclusion policy");
  }
  const expectedExcludedPaths = [...ignoredProjectPaths].sort();
  if (JSON.stringify(parsed.excludedPaths) !== JSON.stringify(expectedExcludedPaths)) {
    fatalBaseline("Trusted baseline manifest excludedPaths do not match the verifier's fixed generated-path policy");
  }

  const files = new Map();
  for (const entry of parsed.files) {
    if (
      !isObject(entry) ||
      !isSafeProjectPath(entry.path) ||
      entry.path === parsed.manifestPath ||
      !Number.isSafeInteger(entry.bytes) ||
      entry.bytes < 0 ||
      !/^[0-9A-F]{64}$/.test(entry.sha256)
    ) {
      fatalBaseline("Trusted baseline manifest contains an invalid file entry");
    }
    if (files.has(entry.path)) fatalBaseline(`Trusted baseline manifest repeats ${entry.path}`);
    files.set(entry.path, entry);
  }
  const sortedPaths = [...files.keys()].sort();
  if (JSON.stringify([...files.keys()]) !== JSON.stringify(sortedPaths)) {
    fatalBaseline("Trusted baseline manifest file entries are not sorted by project path");
  }
  return {
    bytes,
    manifest: parsed,
    files,
    sha256: actualSha256,
    commit: baselineCommit.toLowerCase(),
  };
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

function countsMatchExpected(actual, expected) {
  if (!isObject(expected)) return true;
  return (
    Object.entries(expected).every(([state, count]) => (actual[state] ?? 0) === count) &&
    Object.entries(actual).every(([state, count]) => state in expected || count === 0)
  );
}

function isExcludedProjectDirectory(projectPath, name) {
  if (projectPath === ".git") return true;
  const firstSegment = projectPath.split("/")[0];
  return (
    ignoredDirectories.has(firstSegment) ||
    (name !== ".git" && ignoredDirectories.has(name)) ||
    ignoredProjectPaths.has(projectPath)
  );
}

function isExcludedProjectFile(projectPath) {
  return projectPath === ".git";
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
        if (!isExcludedProjectDirectory(projectPath, name)) {
          stack.push(child);
        }
      } else if (!isExcludedProjectFile(projectPath)) {
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

const trustedBaseline = loadTrustedBaseline();
const contract = readJson("tests/phase1/policy-contract.json");
if (!contract) fatalTool("Policy contract is missing or invalid JSON");

const subjectManifestPath = trustedBaseline.manifest.manifestPath;
const subjectManifestAbsolute = join(root, subjectManifestPath);
const subjectManifestPresent =
  existsSync(subjectManifestAbsolute) && statSync(subjectManifestAbsolute).isFile();
const subjectManifestMatches =
  subjectManifestPresent && readFileSync(subjectManifestAbsolute).equals(trustedBaseline.bytes);
record(
  "VER-INT-0001",
  subjectManifestMatches,
  subjectManifestMatches
    ? `Subject manifest exactly matches the Git-object baseline ${trustedBaseline.commit}`
    : subjectManifestPresent
      ? `Subject manifest differs from the externally supplied Git-object manifest for ${trustedBaseline.commit}`
      : `Subject manifest is missing: ${subjectManifestPath}`,
);
record(
  "VER-INT-0001",
  contract.trustedBaseline?.manifestPath === subjectManifestPath &&
    contract.trustedBaseline?.schemaVersion === trustedBaseline.manifest.schemaVersion &&
    contract.trustedBaseline?.hashAlgorithm === trustedBaseline.manifest.hashAlgorithm &&
    contract.trustedBaseline?.scope === trustedBaseline.manifest.scope &&
    JSON.stringify(contract.trustedBaseline?.excludedRoots) ===
      JSON.stringify(trustedBaseline.manifest.excludedRoots) &&
    JSON.stringify(contract.trustedBaseline?.excludedPaths) ===
      JSON.stringify(trustedBaseline.manifest.excludedPaths),
  "Policy contract identifies the externally trusted manifest format without supplying expected artifact hashes",
);
const gitControlPolicy = contract.trustedBaseline?.gitControlMetadata;
record(
  "VER-INT-0001",
  gitControlPolicy?.rootPath === ".git" &&
    gitControlPolicy?.excludeRootFile === true &&
    gitControlPolicy?.excludeRootDirectory === true &&
    gitControlPolicy?.nestedGitPaths === "BASELINED_AS_ORDINARY_PROJECT_PATHS" &&
    isExcludedProjectFile(".git") &&
    !isExcludedProjectFile("nested/.git") &&
    isExcludedProjectDirectory(".git", ".git") &&
    !isExcludedProjectDirectory("nested/.git", ".git"),
  "Only the repository-root .git control entry is excluded as Git metadata; nested .git paths remain baseline-controlled",
);
record(
  "VER-CI-0001",
  process.version === `v${contract.expectedToolchain.node}`,
  `Verifier runtime is ${process.version}; required runtime is v${contract.expectedToolchain.node}`,
);
const extractorCheck = spawnSync(
  process.env.PHASE1_PYTHON ?? (process.platform === "win32" ? "python" : "python3"),
  ["-B", "tools/phase1/extract_directive_requirements.py", "--check", "--root", "."],
  { cwd: root, encoding: "utf8", shell: false, timeout: 120_000 },
);
if (extractorCheck.error || extractorCheck.status === null) {
  fatalTool(
    `Deterministic extractor could not execute: ${extractorCheck.error?.message ?? extractorCheck.signal ?? "no exit status"}`,
  );
}
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

const unresolvedPlaceholderPattern =
  /\b(?:TBD|TODO|FIXME|REPLACE_ME|INSERT_HERE)\b|\[\s*(?:INSERT|TBD|TODO)[^\]]*\]|<(?:INSERT|TBD|TODO)[^>]*>|(?<!\$)\{\{\s*(?:INSERT|TBD|TODO|REPLACE)[^{}\r\n]*\}\}|\b(?:COMMIT|TAG|HASH)_PLACEHOLDER\b/gi;
const placeholderViolations = [];
for (const path of contract.placeholderFreeFiles ?? []) {
  const text = readText(path);
  const tokens = text.match(unresolvedPlaceholderPattern) ?? [];
  if (tokens.length > 0) placeholderViolations.push(`${path}: ${[...new Set(tokens)].join(", ")}`);
}
record(
  "VER-DOC-0001",
  placeholderViolations.length === 0,
  placeholderViolations.length === 0
    ? `Final audit and closure report contain no unresolved placeholder tokens`
    : `Unresolved placeholder token(s): ${placeholderViolations.join("; ")}`,
);

const requiredAdrPaths = [
  "ADR/0001-no-physical-industrial-communication.md",
  "ADR/0002-original-project-format.md",
  "ADR/0003-unified-plc-ir.md",
  "ADR/0004-deterministic-virtual-time.md",
];
for (const path of requiredAdrPaths) {
  const absolute = join(root, path);
  const valid = existsSync(absolute) && statSync(absolute).isFile() && statSync(absolute).size > 0;
  record(
    "VER-ADR-0001",
    valid,
    valid ? `Required ADR is present: ${path}` : `Missing required ADR: ${path}`,
  );
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
const reconciliation = readJson("requirements/phase1-reconciliation.json");
let requirementIds = new Set();
let requirementsById = new Map();
if (registry && matrix && reconciliation) {
  const requirements = Array.isArray(registry.requirements) ? registry.requirements : [];
  const ids = requirements.map((item) => item.id);
  requirementIds = new Set(ids);
  requirementsById = new Map(requirements.map((item) => [item.id, item]));
  const snapshotPolicy = contract.requirementsSnapshot;
  const expectedCounts = snapshotPolicy.counts;
  const countsMatch = (actual, expected) =>
    Object.keys(expected).every((key) => actual?.[key] === expected[key]) &&
    Object.keys(actual ?? {}).every((key) => key in expected);
  record(
    "VER-REQ-0001",
    registry.requirementCount === expectedCounts.issuedIdCount &&
      requirements.length === expectedCounts.issuedIdCount,
    `Requirement registry contains ${requirements.length}/${expectedCounts.issuedIdCount} issued records`,
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
    "completionEligible",
    "lifecycle",
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
        item.sourcePointer.headingPath.length > 0 &&
        Array.isArray(item.sourcePointer.sourceUnitIds) &&
        (item.sourcePointer.sourceUnitIds.length > 0 ||
          (["SHOULD", "MAY"].includes(item.normativeKeyword) &&
            typeof item.sourcePointer.sourceVerbatim === "string" &&
            item.sourcePointer.sourceVerbatim.length > 0)),
    ),
    "Every requirement has a non-empty heading and hash-bound directive pointer; mandatory units have reconciliation lineage and advisory records retain verbatim source",
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
    registry.schemaVersion === snapshotPolicy.schemaVersion &&
      matrix.schemaVersion === snapshotPolicy.schemaVersion &&
      reconciliation.schemaVersion === snapshotPolicy.schemaVersion &&
      registry.generatorSha256 === extractorHash &&
      matrix.generatorSha256 === extractorHash &&
      reconciliation.generatedBy === "tools/phase1/extract_directive_requirements.py",
    `Registry, matrix, and reconciliation are schema v${snapshotPolicy.schemaVersion} and bound to the current extractor`,
  );
  record(
    "VER-REQ-0001",
    countsMatch(registry.counts, expectedCounts) &&
      countsMatch(matrix.counts, expectedCounts) &&
      countsMatch(reconciliation.counts, expectedCounts),
    `All schema-v${snapshotPolicy.schemaVersion} snapshots carry the independently admitted count contract ${JSON.stringify(expectedCounts)}`,
  );

  const sourceUnits = Array.isArray(reconciliation.sourceUnits) ? reconciliation.sourceUnits : [];
  const sourceUnitIds = sourceUnits.map((item) => item.id);
  const sourceUnitsById = new Map(sourceUnits.map((item) => [item.id, item]));
  const supersededParents = requirements.filter(
    (item) => item.atomicity === "SUPERSEDED_COMPOUND_PARENT",
  );
  const atomicRecords = requirements.filter(
    (item) => item.atomicity !== "SUPERSEDED_COMPOUND_PARENT",
  );
  const completionEligibleAtomicRecords = atomicRecords.filter((item) => item.completionEligible === true);
  const mappedSourceUnits = sourceUnits.filter((item) => item.disposition === "MAPPED");
  const relationshipCount = sourceUnits.reduce(
    (total, item) => total + (Array.isArray(item.requirementIds) ? item.requirementIds.length : 0),
    0,
  );
  record(
    "VER-REQ-0001",
    supersededParents.length === expectedCounts.supersededCompoundParentCount &&
      atomicRecords.length === expectedCounts.atomicRecordCount &&
      completionEligibleAtomicRecords.length === expectedCounts.completionEligibleAtomicRecordCount &&
      sourceUnits.length === expectedCounts.sourceStatementUnitCount &&
      mappedSourceUnits.length === expectedCounts.mappedStatementUnitCount &&
      sourceUnits.length - mappedSourceUnits.length === expectedCounts.unmappedStatementUnitCount &&
      relationshipCount === expectedCounts.sourceUnitRelationshipCount,
    `Derived reconciliation counts are ${requirements.length} issued, ${supersededParents.length} historical parents, ${atomicRecords.length} atomic, ${completionEligibleAtomicRecords.length} completion-eligible, ${mappedSourceUnits.length}/${sourceUnits.length} mapped, and ${relationshipCount} relationships`,
  );
  record(
    "VER-REQ-0001",
    new Set(sourceUnitIds).size === sourceUnitIds.length &&
      sourceUnits.every(
        (unit) =>
          /^T2-\d{4}$/.test(unit.id) &&
          unit.disposition === "MAPPED" &&
          Array.isArray(unit.requirementIds) &&
          unit.requirementIds.length > 0 &&
          new Set(unit.requirementIds).size === unit.requirementIds.length &&
          unit.requirementIds.every(
            (id) =>
              requirementIds.has(id) &&
              requirementsById.get(id).sourcePointer.sourceUnitIds.includes(unit.id),
          ) &&
          Array.isArray(unit.historicalParentIds) &&
          unit.historicalParentIds.every(
            (id) => requirementIds.has(id) && requirementsById.get(id).atomicity === "SUPERSEDED_COMPOUND_PARENT",
          ),
      ) &&
      requirements.every((item) =>
        item.sourcePointer.sourceUnitIds.every(
          (unitId) =>
            sourceUnitsById.has(unitId) &&
            (sourceUnitsById.get(unitId).requirementIds.includes(item.id) ||
              sourceUnitsById.get(unitId).historicalParentIds.includes(item.id)),
        ),
      ),
    "All 546 source units are uniquely identified, mapped to issued requirements, and reciprocal with requirement pointers",
  );
  record(
    "VER-REQ-0002",
    requirements.every((item) =>
      ["BASELINE_ATOMIC", "ATOMIC_CHILD", "SUPERSEDED_COMPOUND_PARENT"].includes(item.atomicity),
    ) &&
      supersededParents.every(
        (parent) =>
          parent.completionEligible === false &&
          parent.lifecycle?.status === "SUPERSEDED_PARENT" &&
          Array.isArray(parent.lifecycle.childIds) &&
          parent.lifecycle.childIds.length > 0 &&
          JSON.stringify(parent.lifecycle.childIds) === JSON.stringify(parent.lifecycle.supersededBy) &&
          parent.lifecycle.childIds.every((childId) => {
            const child = requirementsById.get(childId);
            return child?.atomicity === "ATOMIC_CHILD" && child.lifecycle?.parentId === parent.id;
          }),
      ) &&
      atomicRecords
        .filter((item) => item.completionEligible === false)
        .every((item) => item.lifecycle?.status === "SUPERSEDED"),
    "Historical compound parents and superseded atomic records are non-completion-bearing with explicit child lineage",
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
      item?.truthState === (expected.truthState ?? "IMPLEMENTED_UNVERIFIED") &&
        item?.completionEligible === (expected.completionEligible ?? true) &&
        JSON.stringify(item.verificationIds) === JSON.stringify(expected.verificationIds) &&
        JSON.stringify(item.implementationComponents) === JSON.stringify(expected.implementationComponents) &&
        item.implementationComponents.every((path) => existsSync(join(root, path))) &&
        Array.isArray(item.relatedRequirements) &&
        item.relatedRequirements.every((relatedId) => requirementIds.has(relatedId)) &&
        criteriaAreCurated &&
        item.truthState !== "VERIFIED",
      `${id} has its exact admitted truth state, eligibility, check mapping, and foundation components without claiming VERIFIED`,
    );
  }

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
        requirementsById.get(entry.requirementId)?.completionEligible === entry.completionEligible &&
        requirementsById.get(entry.requirementId)?.lifecycle?.status === entry.lifecycleStatus &&
        requirementsById.get(entry.requirementId)?.lifecycle?.parentId === entry.parentId &&
        JSON.stringify(requirementsById.get(entry.requirementId)?.verificationIds) === JSON.stringify(entry.verificationIds) &&
        JSON.stringify(requirementsById.get(entry.requirementId)?.implementationComponents) ===
          JSON.stringify(entry.implementationComponents) &&
        JSON.stringify(requirementsById.get(entry.requirementId)?.adrDecisionChangeLinks) ===
          JSON.stringify(entry.decisionLinks),
    ),
    "Every matrix entry matches registry state, verification IDs, and implementation components",
  );
  const actualStateCounts = countStates(entries, "truthState");
  const actualCompletionEligibleStateCounts = countStates(
    entries.filter((entry) => entry.completionEligible === true),
    "truthState",
  );
  record(
    "VER-REQ-0002",
    JSON.stringify(actualStateCounts) === JSON.stringify(matrix.stateCounts) &&
      JSON.stringify(actualCompletionEligibleStateCounts) ===
        JSON.stringify(matrix.completionEligibleStateCounts) &&
      countsMatchExpected(actualStateCounts, snapshotPolicy.stateCounts) &&
      countsMatchExpected(
        actualCompletionEligibleStateCounts,
        snapshotPolicy.completionEligibleStateCounts,
      ),
    `Matrix state counts are exact for ${entries.length} issued and ${entries.filter((entry) => entry.completionEligible).length} completion-eligible records: ${JSON.stringify(actualStateCounts)}`,
  );
  record(
    "VER-REQ-0002",
    includesEvery(matrix.completionRule ?? "", [
      "only completion-eligible atomic records may be counted",
      "only verified means complete",
      "historical compound parents and superseded records are never completion-bearing",
      "no completion percentage is calculated",
    ]) &&
      !Object.keys(matrix).some((key) => /percent|percentage/i.test(key)),
    "Matrix limits completion to eligible atomic VERIFIED records, excludes superseded lineage, and contains no percentage",
  );
  const scaffolded = requirements.filter((item) => item.truthState === "SCAFFOLDED");
  record(
    "VER-QLT-0001",
    scaffolded.every(
      (item) =>
        typeof item.owner === "string" &&
        item.owner.length > 0 &&
        typeof item.targetMilestone === "string" &&
        item.targetMilestone.length > 0 &&
        item.phase1Disposition === "FOUNDATION_WORK_ONLY" &&
        Array.isArray(item.implementationComponents) &&
        item.implementationComponents.length > 0,
    ),
    `${scaffolded.length} SCAFFOLDED record(s) have an owner, target, foundation-only disposition, components, and zero completion credit`,
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
    "all tools unapproved",
    "production reachability: prohibited",
    "tools/phase1/run_phase1_verification.py",
    contract.expectedToolchain.node,
    contract.expectedToolchain.pnpm,
    contract.expectedToolchain.python,
    contract.expectedToolchain.rust,
    contract.expectedToolchain.runner,
    ...contract.expectedToolchainNames,
    ...Object.values(contract.expectedActionPins),
  ]),
  "Toolchain admission register inventories every declared Phase 1 tool without claiming approval",
);
const toolchainRecords = toolchainRegister
  .split(/^### TC-\d{4}[^\n]*$/m)
  .slice(1)
  .map((section) => section.split(/^## \d/m)[0]);
const admittedToolchainReviewerRows = new Set([
  "| Reviewer/decision/date | `UNASSIGNED` / `NOT_REVIEWED` / `null` |",
  "| Reviewer/decision/date | `UNASSIGNED` / `CANDIDATE_UNREVIEWED` / `null` |",
]);
record(
  "VER-CI-0001",
  toolchainRecords.length === contract.expectedToolchainRecordCount &&
    toolchainRecords.every((section) => {
      const reviewerRows = section.match(/^\| Reviewer\/decision\/date \|.*$/gm) ?? [];
      return reviewerRows.length === 1 && admittedToolchainReviewerRows.has(reviewerRows[0]);
    }),
  `All ${contract.expectedToolchainRecordCount} toolchain records have exactly one unassigned NOT_REVIEWED or CANDIDATE_UNREVIEWED disposition and no approval claim`,
);

const openDecisions = existsSync(join(root, "OPEN_DECISIONS.md")) ? readText("OPEN_DECISIONS.md") : "";
const risks = existsSync(join(root, "RISK_REGISTER.md")) ? readText("RISK_REGISTER.md") : "";
for (const id of new Set([...contract.openQuestionIds, ...contract.blockedDecisionIds])) {
  record("VER-DEC-0001", openDecisions.includes(id), `${id} is recorded in OPEN_DECISIONS.md`);
}
for (const [id, expected] of Object.entries(contract.decisionDispositions ?? {})) {
  const sectionPattern = new RegExp(`^### ${id}\\b[\\s\\S]*?(?=^### |(?![\\s\\S]))`, "im");
  const section = openDecisions.match(sectionPattern)?.[0] ?? "";
  const expectedStatus = String(expected.status ?? "").toUpperCase();
  const statusMatches =
    typeof expected.exactStatusLine === "string" && expected.exactStatusLine.length > 0
      ? section.split(/\r?\n/).includes(expected.exactStatusLine)
      : section.toUpperCase().includes(`**STATUS:** ${expectedStatus}`);
  const changeRecordMatches =
    typeof expected.changeRecordId !== "string" ||
    (expected.changeRecordId.length > 0 && section.includes(expected.changeRecordId));
  const blockedBoundaryMatches = (expected.requiredBlockedBoundaryPhrases ?? []).every((phrase) =>
    section.includes(phrase),
  );
  record(
    "VER-DEC-0001",
    statusMatches && changeRecordMatches && blockedBoundaryMatches,
    statusMatches && changeRecordMatches && blockedBoundaryMatches
      ? `${id} has its exact admitted ${expectedStatus} disposition${expected.changeRecordId ? ` under ${expected.changeRecordId}` : ""} and preserves every blocked external-operation boundary`
      : `${id} must have the exact ${expectedStatus} status${expected.changeRecordId ? `, link ${expected.changeRecordId}` : ""}, and preserve every declared blocked boundary`,
  );
}
for (const id of contract.riskIds) {
  record("VER-DEC-0001", risks.includes(id), `${id} is recorded in RISK_REGISTER.md`);
}

const riskStatuses = new Map();
for (const line of risks.split(/\r?\n/)) {
  const cells = line.split("|").slice(1, -1).map((cell) => cell.trim());
  if (cells.length !== 4) continue;
  const id = cells[0].match(/RSK-\d{4}/)?.[0];
  const status = cells[3].match(/\b(OPEN|CLOSED|ACCEPTED)\b/i)?.[1]?.toUpperCase();
  if (id && status) riskStatuses.set(id, status);
}
const riskClosureEvidence = Array.isArray(evidence?.riskClosureEvidence)
  ? evidence.riskClosureEvidence
  : [];
const closureFields = [
  "riskId",
  "disposition",
  "evidenceRecordIds",
  "verificationIds",
  "decisionOrAdrIds",
  "changeRecordId",
  "reviewer",
  "reviewDate",
  "closureRationale",
  "residualRisk",
];
record(
  "VER-RSK-0001",
  Array.isArray(evidence?.riskClosureEvidence) &&
    JSON.stringify(evidence?.riskClosureEvidenceSchema?.requiredFields) ===
      JSON.stringify(closureFields) &&
    Array.isArray(evidence?.riskClosureEvidenceSchema?.rules) &&
    evidence.riskClosureEvidenceSchema.rules.length >= 5,
  "Evidence register defines the exact machine-readable risk-closure schema and fail-closed rules",
);
record(
  "VER-RSK-0001",
  riskClosureEvidence.every((closure) => contract.riskIds.includes(closure?.riskId)),
  "Every risk-closure record targets a declared Phase 1 risk",
);
const evidenceRowsById = new Map(
  (Array.isArray(evidence?.requirementEvidence) ? evidence.requirementEvidence : []).map((item) => [
    item.evidenceRecordId,
    item,
  ]),
);
const pendingRiskClosureChecks = [];
const knownDecisionOrAdrIds = new Set([
  ...Object.keys(contract.decisionDispositions ?? {}),
  ...contract.openQuestionIds,
  "ADR-0001",
  "ADR-0002",
  "ADR-0003",
  "ADR-0004",
]);
const changeLog = readText("CHANGELOG_DIRECTIVE.md");
for (const id of contract.riskIds) {
  const status = riskStatuses.get(id);
  const closures = riskClosureEvidence.filter((item) => item?.riskId === id);
  if (!status) {
    record("VER-RSK-0001", false, `${id} has no parseable OPEN, CLOSED, or ACCEPTED status`);
    continue;
  }
  if (status === "OPEN") {
    record(
      "VER-RSK-0001",
      closures.length === 0,
      closures.length === 0
        ? `${id} remains OPEN and has no active closure claim`
        : `${id} is OPEN but has ${closures.length} active closure-evidence record(s)`,
    );
    continue;
  }

  if (closures.length !== 1) {
    record(
      "VER-RSK-0001",
      false,
      `${id} is ${status} but has no approved closureEvidence record`,
    );
    continue;
  }
  const closure = closures[0];
  const fieldsPresent =
    missingFields(closure, closureFields).length === 0 &&
    closureFields
      .filter((field) => !["evidenceRecordIds", "verificationIds", "decisionOrAdrIds"].includes(field))
      .every((field) => typeof closure[field] === "string" && closure[field].trim().length > 0) &&
    ["evidenceRecordIds", "verificationIds", "decisionOrAdrIds"].every(
      (field) => Array.isArray(closure[field]) && closure[field].length > 0,
    ) &&
    closure.disposition === status &&
    /^\d{4}-\d{2}-\d{2}$/.test(closure.reviewDate) &&
    ["evidenceRecordIds", "verificationIds", "decisionOrAdrIds"].every(
      (field) => new Set(closure[field]).size === closure[field].length,
    ) &&
    closure.decisionOrAdrIds.every((referenceId) => knownDecisionOrAdrIds.has(referenceId)) &&
    /^CR-\d{4}$/.test(closure.changeRecordId) &&
    changeLog.includes(closure.changeRecordId);
  const evidenceApproved =
    fieldsPresent &&
    closure.evidenceRecordIds.every((recordId) => {
      const row = evidenceRowsById.get(recordId);
      return (
        row &&
        row.reviewStatus === "APPROVED" &&
        row.recordStatus === "VERIFIED" &&
        typeof row.reviewer === "string" &&
        row.reviewer.trim().length > 0 &&
        /^\d{4}-\d{2}-\d{2}$/.test(row.reviewDate ?? "")
      );
    });
  const verificationDeclared =
    fieldsPresent &&
    closure.verificationIds.every((verificationId) => contract.verificationIds.includes(verificationId));
  pendingRiskClosureChecks.push(
    { id, status, closure, fieldsPresent, evidenceApproved, verificationDeclared },
  );
}

const reservedRoots = [
  "profiles",
  "scenarios",
  "assets/original",
  "artifacts",
  "build",
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
  includesEvery(readme, ["phase 2 product implementation has not begun", "contains no plc project model", "only `verified` can count as complete"]) &&
    includesEvery(scopeAudit, ["no phase 2-4 plc product feature work was performed", "does not mark the four-phase master directive", "does not promote any requirement to `verified`"]) &&
    includesEvery(directiveLog, ["no phase 2-4 product feature work was performed", "does not pass the phase 1 closure gate", "or authorize phase 2 product implementation"]),
  "README, scope audit, and directive log reject Phase 1/product/master-directive completion claims",
);

for (const base of ["apps", "packages", "crates"]) {
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
const projectFileMap = new Map(projectFiles.map((item) => [item.path, item]));
const controlledInputPaths = new Set([...trustedBaseline.files.keys(), subjectManifestPath]);
const expectedProjectPaths = [...controlledInputPaths].sort();
const actualProjectPaths = projectFiles
  .filter((item) => !item.symlink)
  .map((item) => item.path)
  .sort();
const missingBaselinePaths = expectedProjectPaths.filter((path) => !projectFileMap.has(path));
const extraBaselinePaths = actualProjectPaths.filter((path) => !controlledInputPaths.has(path));
record(
  "VER-INT-0002",
  missingBaselinePaths.length === 0 && extraBaselinePaths.length === 0,
  missingBaselinePaths.length === 0 && extraBaselinePaths.length === 0
    ? `Project path set exactly matches ${expectedProjectPaths.length} trusted baseline paths`
    : [
        missingBaselinePaths.length > 0 ? `Missing baseline path(s): ${missingBaselinePaths.join(", ")}` : "",
        extraBaselinePaths.length > 0 ? `Unexpected project path(s): ${extraBaselinePaths.join(", ")}` : "",
      ]
        .filter(Boolean)
        .join("; "),
);

const baselineByteMismatches = [];
for (const [path, expected] of trustedBaseline.files) {
  const absolute = join(root, path);
  if (!existsSync(absolute) || !statSync(absolute).isFile()) continue;
  const actualBytes = statSync(absolute).size;
  const actualSha256 = fileSha256(path);
  if (actualBytes !== expected.bytes) {
    baselineByteMismatches.push(
      `Byte-length mismatch: ${path} expected ${expected.bytes}, got ${actualBytes}`,
    );
  }
  if (actualSha256 !== expected.sha256) {
    baselineByteMismatches.push(
      `SHA-256 mismatch: ${path} expected ${expected.sha256}, got ${actualSha256}`,
    );
  }
}
record(
  "VER-INT-0002",
  baselineByteMismatches.length === 0,
  baselineByteMismatches.length === 0
    ? `All ${trustedBaseline.files.size} externally baselined files match byte length and SHA-256`
    : baselineByteMismatches.join("; "),
);

const contentPolicy = contract.scopedContentPolicy ?? {};
const restrictedTextExtensions = new Set(
  (contentPolicy.textExtensions ?? []).map((extension) => extension.toLowerCase()),
);
function isScopedTextPath(path, exactFiles, roots) {
  if (exactFiles.includes(path)) return true;
  const extension = path.includes(".") ? `.${path.split(".").at(-1).toLowerCase()}` : "";
  return restrictedTextExtensions.has(extension) && roots.some((prefix) => path.startsWith(`${prefix}/`));
}
function isAllowedOccurrence(kind, path, literal) {
  return (contentPolicy.allowlist ?? []).some(
    (entry) =>
      entry.kind === kind &&
      entry.path === path &&
      entry.literal === literal &&
      typeof entry.authorizationId === "string" &&
      entry.authorizationId.trim().length > 0,
  );
}
const restrictedTextFiles = projectFiles.filter(
  (item) =>
    !item.symlink &&
    isScopedTextPath(
      item.path,
      contentPolicy.restrictedTextFiles ?? [],
      contentPolicy.restrictedTextRoots ?? [],
    ),
);
const userFacingTextFiles = projectFiles.filter(
  (item) =>
    !item.symlink &&
    isScopedTextPath(
      item.path,
      contentPolicy.userFacingTextFiles ?? [],
      contentPolicy.userFacingTextRoots ?? [],
    ),
);

const externalUrlViolations = [];
const loopbackViolations = [];
for (const file of restrictedTextFiles) {
  const text = readText(file.path);
  for (const match of text.matchAll(/\b(?:ftp|https?|wss?):\/\/[^\s"'<>]+/gi)) {
    const literal = match[0].replace(/[),.;:]+$/, "");
    if (!isAllowedOccurrence("EXTERNAL_URL", file.path, literal)) {
      externalUrlViolations.push(`${file.path}: ${literal}`);
    }
  }
  for (const match of text.matchAll(/\blocalhost(?::\d{1,5})?\b|\b127(?:\.\d{1,3}){3}(?::\d{1,5})?\b|\b0\.0\.0\.0(?::\d{1,5})?\b|\[::1\](?::\d{1,5})?/gi)) {
    const literal = match[0];
    if (!isAllowedOccurrence("LOOPBACK_ENDPOINT", file.path, literal)) {
      loopbackViolations.push(`${file.path}: ${literal}`);
    }
  }
}
record(
  "VER-OFF-0001",
  externalUrlViolations.length === 0,
  externalUrlViolations.length === 0
    ? `No unauthorized external URL exists across ${restrictedTextFiles.length} restricted text files`
    : `Unauthorized external URL in ${externalUrlViolations.join("; ")}`,
);
record(
  "VER-OFF-0002",
  loopbackViolations.length === 0,
  loopbackViolations.length === 0
    ? `No unauthorized loopback endpoint exists across ${restrictedTextFiles.length} restricted text files`
    : `Unauthorized loopback endpoint in ${loopbackViolations.join("; ")}`,
);

const vendorPatterns = (contentPolicy.vendorMarks ?? []).map(
  (literal) => new RegExp(`\\b${literal.replace(/[.*+?^${}()|[\]\\]/g, "\\$&").replaceAll(" ", "\\s+")}\\b`, "gi"),
);
const vendorViolations = [];
for (const file of userFacingTextFiles) {
  const text = readText(file.path);
  const lines = text.split(/\r?\n/);
  for (const line of lines) {
    if (!vendorPatterns.some((pattern) => pattern.test(line))) continue;
    for (const pattern of vendorPatterns) pattern.lastIndex = 0;
    const literal = line.trim();
    if (literal && !isAllowedOccurrence("VENDOR_TEXT", file.path, literal)) {
      vendorViolations.push(`${file.path}: ${literal}`);
    }
  }
}
record(
  "VER-BRN-0001",
  vendorViolations.length === 0,
  vendorViolations.length === 0
    ? `No unauthorized vendor-facing text exists across ${userFacingTextFiles.length} user-facing files`
    : `Unauthorized vendor-facing text in ${vendorViolations.join("; ")}`,
);

const approvedFoundationFiles = new Set(contract.phase1FoundationFileAllowlist ?? []);
const productRootFiles = projectFiles.filter(
  (item) =>
    !item.symlink &&
    ["apps/", "packages/", "crates/"].some((prefix) => item.path.startsWith(prefix)),
);
const unauthorizedProductFiles = productRootFiles
  .map((item) => item.path)
  .filter((path) => !approvedFoundationFiles.has(path));
const productRootPathSet = new Set(productRootFiles.map((item) => item.path));
const missingApprovedProductFiles = [...approvedFoundationFiles].filter(
  (path) => !productRootPathSet.has(path),
);
record(
  "VER-SCP-0001",
  unauthorizedProductFiles.length === 0 && missingApprovedProductFiles.length === 0,
  unauthorizedProductFiles.length === 0 && missingApprovedProductFiles.length === 0
    ? `All ${productRootFiles.length} product-root files are explicitly authorized for the Phase 1 foundation`
    : [
        unauthorizedProductFiles.length > 0
          ? `Unauthorized Phase 1 product-root file: ${unauthorizedProductFiles.join(", ")}`
          : "",
        missingApprovedProductFiles.length > 0
          ? `Approved foundation file is missing: ${missingApprovedProductFiles.join(", ")}`
          : "",
      ]
        .filter(Boolean)
        .join("; "),
);

const foundationContract = contract.foundationContract;
const foundationContractSource = readText(foundationContract.contractPath);
const foundationAppSource = readText(foundationContract.appPath);
const foundationClientSource = readText(foundationContract.clientPath);
const foundationWorkerSource = readText(foundationContract.workerPath);
const foundationHandlerSource = readText(foundationContract.workerHandlerPath);
const foundationRustSource = readText(foundationContract.rustPath);
const foundationBrowserTest = readText(foundationContract.browserTestPath);
const foundationContractTest = readText(foundationContract.contractTestPath);
record(
  "VER-FND-0001",
  includesEvery(foundationContractSource, [
    `FOUNDATION_SCHEMA_VERSION = ${foundationContract.schemaVersion}`,
    `FOUNDATION_COMMAND_KIND = "${foundationContract.commandKind}"`,
    `FOUNDATION_RESULT_KIND = "${foundationContract.resultKind}"`,
    `FOUNDATION_REQUEST_ID = "${foundationContract.requestId}"`,
    `FOUNDATION_BUILD_IDENTITY = "${foundationContract.buildIdentity}"`,
    `FOUNDATION_HEALTHY_STATE = "${foundationContract.healthState}"`,
    foundationContract.stateHash,
    "validateFoundationHealthCommand",
    "validateFoundationHealthResult",
    "requireExactKeys",
    "record.beforeHash !== record.afterHash",
  ]),
  "Typed foundation contract fixes the only command, exact DomainResult envelope, deterministic state hash, and closed record shapes",
);
record(
  "VER-FND-0001",
  includesEvery(foundationAppSource, [
    "verifyLocalFoundation",
    "successValue?.schemaVersion",
    "successValue?.buildIdentity",
    "successValue?.healthState",
    "No PLC features are active in this phase",
  ]) &&
    includesEvery(foundationClientSource, [
      "new FoundationWorker",
      "validateFoundationHealthResult",
      `kind: "${foundationContract.commandKind}"`,
      `requestId: "${foundationContract.requestId}"`,
      `schemaVersion: ${foundationContract.schemaVersion}`,
    ]) &&
    includesEvery(foundationWorkerSource, ["executeFoundationCommand", "workerScope.postMessage(result)"]) &&
    includesEvery(foundationHandlerSource, [
      "validateFoundationHealthCommand",
      "runRustHealthCheck",
      "WebAssembly.Module.imports(module).length !== 0",
      "validateFoundationHealthResult(result)",
      "FOUNDATION_WASM_SHA256",
    ]),
  "UI sends the typed command through an isolated worker and real WASM handler, validates DomainResult, and renders returned values",
);
record(
  "VER-FND-0001",
  includesEvery(foundationRustSource, [
    `"schemaVersion":${foundationContract.schemaVersion}`,
    `"buildIdentity":"${foundationContract.buildIdentity}"`,
    `"healthState":"${foundationContract.healthState}"`,
    "foundation_health",
    "foundation_health_len",
    "health_payload_is_exact_and_deterministic",
  ]) &&
    includesEvery(foundationContractTest, [
      "accepts the reconciled deterministic DomainResult envelope",
      "rejects malformed or mutated DomainResult envelopes",
      "rejects invalid or capability-shaped commands",
    ]) &&
    includesEvery(foundationBrowserTest, [
      "offline: true",
      "remoteRequests.length > 0",
      "firstResult !== repeatedResult",
      'getByText("HEALTHY", { exact: true })',
    ]),
  "Rust/WASM payload and unit/browser tests prove exact deterministic output, negative validation, repeated UI round trips, and offline execution",
);
const forbiddenFoundationCapability =
  /\b(?:fetch|XMLHttpRequest|WebSocket|EventSource|WebTransport|RTCPeerConnection)\s*(?:\.|\()|navigator\.(?:serial|usb|bluetooth|hid|nfc|midi|mediaDevices|serviceWorker)\b|\b(?:TcpStream|UdpSocket|serialport|rusb)\b/;
const foundationCapabilityViolations = productRootFiles
  .filter((item) => /\.(?:js|jsx|mjs|rs|ts|tsx)$/i.test(item.path))
  .filter((item) => forbiddenFoundationCapability.test(readText(item.path)))
  .map((item) => item.path);
record(
  "VER-FND-0001",
  foundationCapabilityViolations.length === 0,
  foundationCapabilityViolations.length === 0
    ? "Authorized foundation product sources contain no host-network, browser-network, discovery, or physical-device API"
    : `Forbidden foundation capability API in ${foundationCapabilityViolations.join(", ")}`,
);

const authorizedCandidateNpmEntries = contract.authorizedCandidateDependencies?.npm ?? [];
const authorizedCandidateCargoEntries = contract.authorizedCandidateDependencies?.cargo ?? [];
const authorizedCandidateNpmDependencies = new Map(
  authorizedCandidateNpmEntries.map((entry) => [
    `${entry.manifestPath}|${entry.field}|${entry.name}|${entry.versionSpec}`,
    entry,
  ]),
);
const authorizedCandidateCargoDependencies = new Map(
  authorizedCandidateCargoEntries.map((entry) => [
    `${entry.manifestPath}|${entry.section}|${entry.name}|${entry.declaration}`,
    entry,
  ]),
);
const networkDependencyNames = new Set(
  (contract.networkCapableDependencyNames ?? []).map((name) => name.toLowerCase()),
);
const unauthorizedDependencies = [];
const invalidDependencyAdmissions = [
  ...authorizedCandidateNpmEntries,
  ...authorizedCandidateCargoEntries,
].filter(
  (entry) =>
    typeof entry.authorizationId !== "string" ||
    entry.authorizationId.trim().length === 0 ||
    entry.admissionStatus !== "CANDIDATE_UNREVIEWED" ||
    !["BUILD_TEST_ONLY", "FOUNDATION_RUNTIME"].includes(entry.phase1Use) ||
    entry.networkRuntimeAllowed !== false ||
    (networkDependencyNames.has(entry.name.toLowerCase()) && entry.phase1Use !== "BUILD_TEST_ONLY"),
);
const observedNpmDependencyKeys = new Set();
const observedCargoDependencyKeys = new Set();
for (const file of projectFiles.filter((item) => !item.symlink && item.path.endsWith("package.json"))) {
  let manifest;
  try {
    manifest = JSON.parse(readText(file.path));
  } catch {
    continue;
  }
  for (const field of ["dependencies", "devDependencies", "optionalDependencies", "peerDependencies"]) {
    for (const [name, versionSpec] of Object.entries(manifest[field] ?? {})) {
      const key = `${file.path}|${field}|${name}|${versionSpec}`;
      observedNpmDependencyKeys.add(key);
      if (!authorizedCandidateNpmDependencies.has(key)) {
        unauthorizedDependencies.push({ ecosystem: "npm", path: file.path, field, name, versionSpec });
      }
    }
  }
}
for (const file of projectFiles.filter((item) => !item.symlink && item.path.endsWith("Cargo.toml"))) {
  let section = "";
  for (const rawLine of readText(file.path).split(/\r?\n/)) {
    const line = rawLine.replace(/\s+#.*$/, "").trim();
    const sectionMatch = line.match(/^\[([^\]]+)\]$/);
    if (sectionMatch) {
      section = sectionMatch[1];
      continue;
    }
    if (!/(?:^|\.)dependencies$/.test(section)) continue;
    const dependencyMatch = line.match(/^(?:"([^"]+)"|'([^']+)'|([A-Za-z0-9_.-]+))\s*=\s*(.+)$/);
    if (!dependencyMatch) continue;
    const name = dependencyMatch[1] ?? dependencyMatch[2] ?? dependencyMatch[3];
    const declaration = dependencyMatch[4].trim();
    const key = `${file.path}|${section}|${name}|${declaration}`;
    observedCargoDependencyKeys.add(key);
    if (!authorizedCandidateCargoDependencies.has(key)) {
      unauthorizedDependencies.push({ ecosystem: "cargo", path: file.path, field: section, name, versionSpec: declaration });
    }
  }
}
const missingAuthorizedCandidates = [
  ...[...authorizedCandidateNpmDependencies.keys()]
    .filter((key) => !observedNpmDependencyKeys.has(key))
    .map((key) => `Missing authorized npm candidate declaration: ${key}`),
  ...[...authorizedCandidateCargoDependencies.keys()]
    .filter((key) => !observedCargoDependencyKeys.has(key))
    .map((key) => `Missing authorized Cargo candidate declaration: ${key}`),
];
const duplicateCandidateEntries =
  authorizedCandidateNpmDependencies.size !== authorizedCandidateNpmEntries.length ||
  authorizedCandidateCargoDependencies.size !== authorizedCandidateCargoEntries.length;
const dependencyDetails = unauthorizedDependencies.map((item) => {
  const capability = networkDependencyNames.has(item.name.toLowerCase())
    ? "Unauthorized network-capable dependency"
    : "Unauthorized dependency";
  return `${capability}: ${item.path} ${item.field}.${item.name}@${item.versionSpec}`;
});
record(
  "VER-DEP-0001",
  unauthorizedDependencies.length === 0 &&
    invalidDependencyAdmissions.length === 0 &&
    missingAuthorizedCandidates.length === 0 &&
    !duplicateCandidateEntries,
  unauthorizedDependencies.length === 0 &&
    invalidDependencyAdmissions.length === 0 &&
    missingAuthorizedCandidates.length === 0 &&
    !duplicateCandidateEntries
    ? `All declared direct dependencies are present in the bounded Phase 1 candidate allowlist; every admission remains CANDIDATE_UNREVIEWED`
    : [
        ...dependencyDetails,
        ...missingAuthorizedCandidates,
        ...(duplicateCandidateEntries ? ["Candidate dependency allowlist contains duplicate exact declarations"] : []),
        ...invalidDependencyAdmissions.map(
          (entry) =>
            `Invalid dependency admission metadata: ${entry.manifestPath} ${entry.name}`,
        ),
      ].join("; "),
);

const uncontrolledFiles = projectFiles.filter((item) => !item.symlink && !controlledInputPaths.has(item.path));
const controlledProjectFiles = projectFiles.filter((item) => !item.symlink && controlledInputPaths.has(item.path));
record(
  "VER-QLT-0001",
  uncontrolledFiles.length === 0,
  uncontrolledFiles.length === 0
    ? "Every non-local project file is registered in the externally trusted baseline"
    : `${uncontrolledFiles.length} project file(s) are outside the externally trusted baseline`,
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
  const dependencyFields = ["dependencies", "devDependencies", "optionalDependencies", "peerDependencies"];
  const dependencyCount = dependencyFields.reduce(
    (total, field) => total + Object.keys(packageJson[field] ?? {}).length,
    0,
  );
  record(
    "VER-CI-0001",
    packageJson.name === contract.rootPackageIdentity?.name &&
      packageJson.private === true &&
      packageJson.type === "module" &&
      Object.entries(contract.requiredRootScripts ?? {}).every(
        ([name, command]) => packageJson.scripts?.[name] === command,
      ) &&
      ["preinstall", "install", "postinstall"].every((name) => !(name in (packageJson.scripts ?? {}))),
    `Root package preserves its private identity, required gate scripts, and has no install lifecycle hooks; declared dependency count is ${dependencyCount}`,
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
const tomlStringArray = (values) => `[${values.map((value) => JSON.stringify(value)).join(", ")}]`;
const expectedRustToolchain = [
  "[toolchain]",
  `channel = "${contract.expectedToolchain.rust}"`,
  ...(contract.expectedToolchain.rustComponents?.length
    ? [`components = ${tomlStringArray(contract.expectedToolchain.rustComponents)}`]
    : []),
  'profile = "minimal"',
  ...(contract.expectedToolchain.rustTargets?.length
    ? [`targets = ${tomlStringArray(contract.expectedToolchain.rustTargets)}`]
    : []),
  "",
].join("\n");
record(
  "VER-CI-0001",
  normalizedText(rustToolchain) === expectedRustToolchain,
  `Rust toolchain is exactly ${contract.expectedToolchain.rust} with the admitted components and targets`,
);
const gitAttributes = readText(".gitattributes");
record(
  "VER-CI-0001",
  includesEvery(gitAttributes, [
    "*.json text eol=lf",
    "*.md text eol=lf",
    "*.mjs text eol=lf",
    "*.py text eol=lf",
    '"References for Codex from Scott/Govs PLC project Research Report.md" -text',
    "*.docx -text",
  ]),
  "Git attributes preserve byte-hashed source inputs and normalize generated text deterministically",
);
const cargoManifest = readText("Cargo.toml");
record(
  "VER-CI-0001",
  /\[workspace\]/.test(cargoManifest) &&
    /resolver\s*=\s*"3"/.test(cargoManifest) &&
    new RegExp(`rust-version\\s*=\\s*"${contract.expectedToolchain.rust.replaceAll(".", "\\.")}"`).test(cargoManifest),
  "Cargo workspace uses resolver 3 and the pinned minimum Rust version; exact files and dependency declarations are controlled separately",
);
record(
  "VER-CI-0001",
  /^version\s*=\s*4$/m.test(readText("Cargo.lock")),
  "Cargo.lock uses lockfile version 4; exact lock bytes are controlled by the trusted baseline",
);
record(
  "VER-CI-0001",
  includesEvery(readText("pnpm-workspace.yaml"), [
    '"apps/*"',
    '"packages/*"',
    "catalogMode: strict",
    "linkWorkspacePackages: false",
    "preferWorkspacePackages: false",
  ]),
  "pnpm workspace is limited to approved app/package roots with strict catalog and disabled implicit workspace linking",
);
record(
  "VER-CI-0001",
  /^lockfileVersion:\s*['"]?9\.0['"]?$/m.test(readText("pnpm-lock.yaml")) &&
    /^importers:\s*$/m.test(readText("pnpm-lock.yaml")),
  "pnpm lockfile uses format 9 with explicit importers; exact lock bytes are controlled by the trusted baseline",
);

const workflowPath = contract.ciWorkflow.path;
if (existsSync(join(root, workflowPath))) {
  const workflow = readText(workflowPath);
  const actionUses = [...workflow.matchAll(/uses:\s*([^@\s]+)@([^\s]+)/g)];
  const runCommands = [...workflow.matchAll(/^\s+run:\s*([^|>].*)$/gm)].map((match) =>
    match[1].trim(),
  );
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
      ...contract.ciWorkflow.requiredCommands,
    ]) &&
      contract.ciWorkflow.requiredCommands.length === 1 &&
      runCommands.filter((command) => command === contract.ciWorkflow.requiredCommands[0]).length === 1 &&
      !workflow.includes("windows-latest") &&
      !workflow.includes("actions/upload-artifact") &&
      !/^\s*run:\s*(?:git\s+push|gh\s+|curl\s+|Invoke-WebRequest\b)/gim.test(workflow) &&
      !/\$\{\{\s*false\s*\}\}|\bif:\s*false\b/i.test(workflow),
    "Active workflow declares fixed runtimes, exactly one shared full closure command, credential/cache limits, and no publication or remote artifact upload step",
  );
  record(
    "VER-CI-0001",
    /^  workflow_dispatch:\s*(?:\r?\n|$)/m.test(workflow) &&
      /^  push:\s*(?:\r?\n|$)/m.test(workflow) &&
      /^  pull_request:\s*(?:\r?\n|$)/m.test(workflow) &&
      jobDeclarationLines.length === 1 &&
      jobDeclarationLines[0] === `  ${contract.ciWorkflow.jobId}:`,
    `CI is active for push, pull_request, and workflow_dispatch with the single admitted ${contract.ciWorkflow.jobId} closure job`,
  );
} else {
  record("VER-CI-0001", false, `Missing active CI workflow: ${workflowPath}`);
}

for (const pending of pendingRiskClosureChecks) {
  const verificationPassed =
    pending.verificationDeclared &&
    pending.closure.verificationIds.every((verificationId) =>
      checks.some((check) => check.id === verificationId && check.passed),
    );
  const passed = pending.fieldsPresent && pending.evidenceApproved && verificationPassed;
  record(
    "VER-RSK-0001",
    passed,
    passed
      ? `${pending.id} ${pending.status} disposition is linked to approved evidence, passing checks, review, and change control`
      : `${pending.id} is ${pending.status} but its closureEvidence record is incomplete, unapproved, or not backed by a passing check`,
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
  suiteVersion: "2.0.0",
  date: new Date().toISOString(),
  platform: `${process.platform}-${process.arch}`,
  nodeVersion: process.version,
  repositoryRoot: root,
  result: errors.length === 0 ? "PASS" : "FAIL",
  artifactManifestScope: "Exact non-local project path set from the externally supplied Git-object manifest",
  trustedBaseline: {
    commit: trustedBaseline.commit,
    manifestPath: subjectManifestPath,
    manifestSha256: trustedBaseline.sha256,
    expectedFileCount: trustedBaseline.files.size + 1,
  },
  artifactHashesAreObservationsNotExpectedValues: true,
  artifactHashes,
  docxVisualQaObservation: {
    recordedResult: docxVisualContract.recordedResult,
    admissionStatus: docxVisualContract.admissionStatus,
    localEvidenceStatus: docxVisualQaLocalEvidenceStatus,
  },
  checks,
  limitations: [
    "The authorized Phase 1 shell and deterministic Rust/WASM health round trip are a technical foundation only; no PLC-domain editor, compiler, runtime, HMI, process, lesson, packaging, or physical capability is implemented.",
    "Later-phase release isolation, offline-course, zero-attempt, export, and InternalTagBus product proofs remain outside this Phase 1 foundation.",
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
if (errors.length > 0) process.exit(EXIT_POLICY_FAILURE);
