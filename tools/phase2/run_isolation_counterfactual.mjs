import { execFile } from "node:child_process";
import { createServer } from "node:http";
import { once } from "node:events";
import { access, mkdir, readFile, rename, stat, writeFile } from "node:fs/promises";
import { platform, release, arch } from "node:os";
import path from "node:path";
import { promisify } from "node:util";

import { chromium } from "playwright-core";

import {
  DEFAULT_FUZZ_CASES,
  EVIDENCE_SCHEMA_VERSION,
  EXPECTED_DIRECTIVE_SHA256,
  ISOLATION_APPROVAL_DECISION_ID,
  ISOLATION_APPROVAL_PATH,
  ISOLATION_VERIFICATION_IDS,
  SUPPORTED_WINDOWS_CONFIGURATION_IDS,
  analyzeCapabilityEvents,
  analyzeHostNetworkAdapters,
  analyzeNetLogTargets,
  analyzeProcessEndpoints,
  assessEvidenceCompleteness,
  assessIsolationClosureEvidence,
  deriveProcessTree,
  parseChromiumNetLog,
  parseGitStatusPorcelainZ,
  partitionCausalObservations,
  scanPackagedHtml,
  sha256,
  stableJson,
} from "./isolation-counterfactual-lib.mjs";

const execFileAsync = promisify(execFile);
const projectRootDefault = path.resolve(import.meta.dirname, "../..");
const options = parseArguments(process.argv.slice(2));
const projectRoot = path.resolve(options.root ?? projectRootDefault);
const artifactPath = path.resolve(projectRoot, options.artifact ?? "dist/index.html");
const outputDirectory = path.resolve(
  projectRoot,
  options.output ?? ".phase2-verification/P2-ISO-WINDOWS-COUNTERFACTUAL",
);
const candidateRef = options.candidateRef ?? "HEAD";
const developmentRun = options.developmentRun === true;
const directivePath = path.join(
  projectRoot,
  "References for Codex from Scott",
  "PLC Engineering Simulator - Codex Master Implementation Directive - Phase 2 of 4 - Runnable PLC Engineering Core.docx",
);
const entryGatePath = path.join(projectRoot, "evidence", "phase2", "P2-00_ENTRY_GATE.json");
const harnessPath = path.join(projectRoot, "tools", "phase2", "run_isolation_counterfactual.mjs");
const libraryPath = path.join(projectRoot, "tools", "phase2", "isolation-counterfactual-lib.mjs");
const browserCandidates = [
  "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
  "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
  "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
];

await mkdir(outputDirectory, { recursive: true });
const netLogPath = path.join(outputDirectory, "chromium-netlog.json");
const reportPath = path.join(outputDirectory, "counterfactual-isolation.json");
const browserLogPath = path.join(outputDirectory, "browser-events.json");
const processLogPath = path.join(outputDirectory, "windows-process-network.json");
const hostAdapterLogPath = path.join(outputDirectory, "windows-network-adapters.json");
const serverLogPath = path.join(outputDirectory, "artifact-server.json");
const manifestPath = path.join(outputDirectory, "evidence-manifest.json");
const closureEvidenceInputPath = path.join(outputDirectory, "closure-evidence-input.json");

const startedAt = new Date().toISOString();
const candidate = await captureCandidateBinding(projectRoot, candidateRef, entryGatePath, [
  ISOLATION_APPROVAL_PATH,
  "tools/phase2/run_isolation_counterfactual.mjs",
  "tools/phase2/isolation-counterfactual-lib.mjs",
  "tools/phase2/isolation-counterfactual-lib.d.mts",
  "tools/phase2/isolation-fuzz-corpus.tsv",
  "tools/phase2/transform_isolation_closure.mjs",
  "tools/phase2/assemble_isolation_closure.mjs",
  "tools/phase2/collect_live_lan_topology.mjs",
  "tools/phase2/finalize_external_isolation_proofs.mjs",
  "tools/phase2/isolation-closure-evidence.schema.json",
  "tools/phase2/ISOLATION_COUNTERFACTUAL.md",
  "tools/phase2/LIVE_LAN_TOPOLOGY_PROTOCOL.md",
  "tests/phase2/isolation-counterfactual.unit.mjs",
  "tests/support/isolation_fuzz.rs",
  "apps/foundation-shell/test/isolation-boundary-fuzz.test.ts",
  "crates/plc-compiler/tests/isolation_boundary_fuzz.rs",
  "crates/plc-core/tests/isolation_boundary_fuzz.rs",
  "crates/plc-observability/tests/isolation_boundary_fuzz.rs",
  "crates/windows-project-broker/tests/isolation_boundary_fuzz.rs",
  "tools/phase2/source_policy.py",
  "requirements/phase2-requirements.json",
  "requirements/phase2-verification-catalog.json",
  "References for Codex from Scott/PLC Engineering Simulator - Codex Master Implementation Directive - Phase 2 of 4 - Runnable PLC Engineering Core.docx",
  "Cargo.lock",
  "pnpm-lock.yaml",
]);
const [artifactBytes, directiveBytes, harnessBytes, libraryBytes] = await Promise.all([
  readFile(artifactPath),
  readFile(directivePath),
  readFile(harnessPath),
  readFile(libraryPath),
]);
const artifactHtml = artifactBytes.toString("utf8");
const packageStaticScan = scanPackagedHtml(artifactHtml);
const directiveSha256 = sha256(directiveBytes);
const browserPath = await findBrowser();
const browserExecutableSha256 = sha256(await readFile(browserPath));
const browserRuntimeProduct = normalizedChromiumRuntimeProduct(browserPath);

const report = {
  assertions: {
    browserCapabilityAdaptersDisabled: false,
    externalAttemptCount: -1,
    fixedNativeLocalBackingProven: false,
    hostNetworkAdaptersDisabled: false,
    liveLanDiscoveryInvarianceProven: false,
    loopbackTrafficAccounted: false,
    packageStaticScan: packageStaticScan.pass,
    processAttributionBoundedToBrowserTree: platform() === "win32",
    vendorDeployableExportRejectionProven: false,
    zeroExternalAttempts: false,
  },
  authority: {
    directivePath: path.relative(projectRoot, directivePath).replaceAll("\\", "/"),
    directiveSha256,
    directiveSha256Expected: EXPECTED_DIRECTIVE_SHA256,
    directiveSha256Matches: directiveSha256 === EXPECTED_DIRECTIVE_SHA256,
    verificationIds: ISOLATION_VERIFICATION_IDS,
  },
  browser: {
    browserExecutableSha256,
    browserRuntimeProduct,
    browserRuntimeVersion: null,
    capabilityAnalysis: null,
    capabilityEvents: [],
    cdpEvents: [],
    pageErrors: [],
    playwrightRequests: [],
    rejectedRequests: [],
    webSockets: [],
    workerCapabilityEvents: [],
    workers: [],
  },
  boundaryFuzzCoverage: {
    boundaries: [],
    complete: false,
    result: "UNAVAILABLE",
    schemaVersion: "1.0",
  },
  candidate,
  chromiumNetLog: {
    analysis: null,
    parsed: false,
    path: path.basename(netLogPath),
    relevantEventCount: 0,
  },
  completedAt: null,
  configuration: "windows-chromium-browser-capabilities-disabled-host-network-state-observed",
  configurationCoverage: {
    approvalDecisionId: ISOLATION_APPROVAL_DECISION_ID,
    approvalSha256: candidate.isolationApprovalSha256,
    approvalStatus: candidate.isolationApprovalSha256 === null ? "UNRESOLVED" : "APPROVED",
    currentConfigurationId: SUPPORTED_WINDOWS_CONFIGURATION_IDS[1],
    evidenceBindings: [],
    expectedConfigurationIds: [...SUPPORTED_WINDOWS_CONFIGURATION_IDS],
    status: candidate.isolationApprovalSha256 === null
      ? "BLOCKED_APPROVAL_BINDING_MISSING"
      : "PARTIAL_RUNTIME_EVIDENCE_MISSING",
  },
  closureEvidenceInput: null,
  evidenceKind: "PHASE2_PACKAGED_COUNTERFACTUAL_ISOLATION",
  harness: {
    harnessSha256: sha256(harnessBytes),
    invocation: {
      argv: process.argv.slice(1),
      executable: process.execPath,
      workingDirectory: process.cwd(),
    },
    librarySha256: sha256(libraryBytes),
    samplingIntervalMilliseconds: 350,
  },
  limitations: [
    "This run covers one admitted Chromium executable on one Windows host; it does not claim Linux, macOS, another browser engine, or adapters-on coverage.",
    "Windows Get-NetTCPConnection/Get-NetUDPEndpoint sampling is periodic and can miss very short-lived OS endpoints; Chromium NetLog and browser/CDP instrumentation provide complementary continuous browser-stack evidence.",
    "This packaged adapters-off harness does not exercise the separately versioned native project-file broker. Fixed-local-backing credit must come from an exact-candidate native-broker run and attestation.",
    "Dynamic import has no replaceable JavaScript hook. It is covered by packaged static scanning, CSP connect-src none, Playwright routing, CDP request capture, and Chromium NetLog rather than by a direct import() wrapper.",
    "Current live-LAN topology is not mutated. Browser-offline execution and zero attributable access are partial evidence only; controlled live-LAN discovery invariance still requires a separate approved lab/CI matrix.",
  ],
  hostNetworkAdapters: {
    analysis: null,
    errors: [],
    snapshots: [],
  },
  fixedNativeBackingAttestation: {
    complete: false,
    operations: [],
    result: "UNAVAILABLE",
    schemaVersion: "1.0",
  },
  liveLanTopologyVariation: {
    complete: false,
    result: "UNAVAILABLE",
    scenarios: [],
    schemaVersion: "1.0",
  },
  osNetwork: {
    analysis: null,
    browserRootPid: null,
    errors: [],
    samplerComplete: false,
    samples: [],
  },
  package: {
    bytes: artifactBytes.byteLength,
    path: path.relative(projectRoot, artifactPath).replaceAll("\\", "/"),
    sha256: sha256(artifactBytes),
    staticScan: packageStaticScan,
  },
  platform: {
    architecture: arch(),
    node: process.version,
    os: platform(),
    osRelease: release(),
  },
  result: "ERROR",
  schemaVersion: EVIDENCE_SCHEMA_VERSION,
  startedAt,
  unresolvedProofs: [
    {
      clause: "controlled live-LAN discovery invariance",
      reason: "The harness does not mutate LAN topology and no approved controlled LAN matrix is available.",
      verificationId: "VER-ISO-0003",
    },
    {
      clause: "fixed native local non-provider non-removable backing before selected-byte I/O",
      reason: "The current packaged adapters-off run does not exercise the approved versioned native project-file broker.",
      verificationId: "VER-ISO-0004",
    },
    {
      clause: "every export rejects vendor and deployable artifacts",
      reason: "No complete executable negative matrix covers every export surface.",
      verificationId: "VER-ISO-0004",
    },
    {
      clause: "every supported platform and configuration has complete exact-candidate evidence",
      reason: "The approved Windows-first configuration set still requires complete exact-candidate PASS evidence for both rows.",
      verificationId: "VER-ISO-0005",
    },
  ],
  vendorDeployableExportRejection: {
    complete: false,
    result: "UNAVAILABLE",
    schemaVersion: "1.0",
    surfaces: [],
  },
  workflow: {
    completed: false,
    commands: [],
    fuzzCases: [],
    observations: {},
  },
};

if (options.closureEvidence !== undefined) {
  const closurePath = path.resolve(projectRoot, options.closureEvidence);
  const closureBytes = await readFile(closurePath);
  const closure = parseClosureEvidence(closureBytes, candidate);
  await writeFile(closureEvidenceInputPath, closureBytes, { flag: "w" });
  report.closureEvidenceInput = {
    bytes: closureBytes.byteLength,
    copiedPath: path.basename(closureEvidenceInputPath),
    sourcePath: path.relative(projectRoot, closurePath).replaceAll("\\", "/"),
    sha256: sha256(closureBytes),
  };
  report.boundaryFuzzCoverage = closure.boundaryFuzzCoverage;
  report.configurationCoverage = closure.configurationCoverage;
  report.fixedNativeBackingAttestation = closure.fixedNativeBackingAttestation;
  report.liveLanTopologyVariation = closure.liveLanTopologyVariation;
  report.vendorDeployableExportRejection = closure.vendorDeployableExportRejection;

  const closureAssessment = assessIsolationClosureEvidence(closure, candidate);
  const closureAssessments = closureAssessment.assessments;
  report.assertions.fixedNativeLocalBackingProven = closureAssessments.backing.complete;
  report.assertions.liveLanDiscoveryInvarianceProven = closureAssessments.topology.complete;
  report.assertions.vendorDeployableExportRejectionProven = closureAssessments.export.complete;
  report.closureEvidenceInput.assessments = closureAssessments;
  report.closureEvidenceInput.complete = closureAssessment.complete;
  report.closureEvidenceInput.failures = closureAssessment.failures;
  report.unresolvedProofs = report.unresolvedProofs.filter(({ clause }) =>
    !(clause === "controlled live-LAN discovery invariance" && closureAssessments.topology.complete) &&
    !(clause.startsWith("fixed native local") && closureAssessments.backing.complete) &&
    !(clause.startsWith("every export rejects") && closureAssessments.export.complete) &&
    !(clause.startsWith("every supported platform") && closureAssessments.configuration.complete));
}

const serverRequests = [];
let artifactServer;
let browserServer;
let browser;
let context;
let processSampler;
let artifactUrl;
let workflowError = null;

await captureHostNetworkAdapterSnapshot(report, "preflight");

try {
  if (directiveSha256 !== EXPECTED_DIRECTIVE_SHA256) {
    throw new Error(`Directive SHA-256 mismatch: ${directiveSha256}`);
  }
  if (!packageStaticScan.pass) {
    throw new Error(`Packaged static isolation scan failed: ${JSON.stringify(packageStaticScan.findings)}`);
  }

  ({ server: artifactServer, url: artifactUrl } = await startArtifactServer(artifactBytes, serverRequests));
  const artifactOrigin = new URL(artifactUrl).origin;
  const artifactPort = Number(new URL(artifactUrl).port);

  browserServer = await chromium.launchServer({
    args: chromiumIsolationArguments(netLogPath),
    executablePath: browserPath,
    headless: true,
  });
  const browserProcess = browserServer.process();
  if (browserProcess === null || !Number.isSafeInteger(browserProcess.pid)) {
    throw new Error("Playwright did not expose the admitted Chromium root process.");
  }
  report.osNetwork.browserRootPid = browserProcess.pid;
  const controlEndpoint = new URL(browserServer.wsEndpoint());
  const controlPort = Number(controlEndpoint.port);
  const allowedOrigins = new Set([
    artifactOrigin,
    `${controlEndpoint.protocol}//${controlEndpoint.host}`,
    `http://${controlEndpoint.host}`,
  ]);
  const allowedEndpoints = new Set([
    `127.0.0.1:${artifactPort}`,
    `::1:${artifactPort}`,
    `127.0.0.1:${controlPort}`,
    `::1:${controlPort}`,
  ]);

  processSampler = startWindowsProcessEndpointSampler(browserProcess.pid, 350);
  browser = await chromium.connect(browserServer.wsEndpoint());
  report.browser.browserRuntimeVersion = browser.version();
  context = await browser.newContext({
    javaScriptEnabled: true,
    locale: "en-US",
    serviceWorkers: "block",
    viewport: { height: 920, width: 1586 },
  });
  await installCapabilityBoundary(context);
  await context.route("**/*", async (route) => {
    const target = route.request().url();
    let allowed = false;
    try {
      allowed = new URL(target).origin === artifactOrigin;
    } catch {
      allowed = false;
    }
    if (allowed) {
      await route.continue();
    } else {
      report.browser.rejectedRequests.push({ method: route.request().method(), target });
      await route.abort("blockedbyclient");
    }
  });
  context.on("request", (request) => {
    report.browser.playwrightRequests.push({
      frameUrl: request.frame()?.url() ?? null,
      method: request.method(),
      resourceType: request.resourceType(),
      target: request.url(),
    });
  });
  context.on("serviceworker", (worker) => {
    report.browser.workers.push({ kind: "service-worker", target: worker.url() });
  });

  const page = await context.newPage();
  page.on("console", (message) => {
    const prefix = "__PHASE2_ISOLATION_CAPABILITY__";
    if (!message.text().startsWith(prefix)) {
      return;
    }
    try {
      report.browser.workerCapabilityEvents.push(JSON.parse(message.text().slice(prefix.length)));
    } catch {
      report.browser.workerCapabilityEvents.push({ api: "worker.instrumentation", outcome: "malformed-log", target: null });
    }
  });
  page.on("pageerror", (error) => report.browser.pageErrors.push(error.message));
  page.on("websocket", (socket) => report.browser.webSockets.push({ target: socket.url() }));
  page.on("worker", (worker) => report.browser.workers.push({ kind: "dedicated-worker", target: worker.url() }));
  const cdp = await context.newCDPSession(page);
  await cdp.send("Network.enable");
  for (const eventName of [
    "Network.requestWillBeSent",
    "Network.webSocketCreated",
    "Network.webTransportCreated",
    "Network.directTCPSocketCreated",
  ]) {
    cdp.on(eventName, (event) => {
      report.browser.cdpEvents.push(compactCdpEvent(eventName, event));
    });
  }

  await page.goto(artifactUrl, { waitUntil: "load" });
  await page.getByText("Core plc-engineering-core@0.2.0", { exact: true }).waitFor();
  await context.setOffline(true);
  report.assertions.browserCapabilityAdaptersDisabled = await page.evaluate(() => {
    const snapshot = window.__phase2IsolationSnapshot();
    return snapshot.adaptersDisabled === true;
  });
  if (!report.assertions.browserCapabilityAdaptersDisabled) {
    throw new Error("Browser capability adapters were not disabled before the workflow.");
  }

  await executePackagedWorkflow(page, report.workflow);
  report.workflow.completed = true;
  report.browser.capabilityEvents = [
    ...await page.evaluate(() => window.__phase2IsolationSnapshot().events),
    ...report.browser.workerCapabilityEvents,
  ];
  report.browser.capabilityAnalysis = analyzeCapabilityEvents(
    report.browser.capabilityEvents,
    allowedOrigins,
  );

  await context.setOffline(false);
  await cdp.detach();
  await context.close();
  context = null;
  await browser.close();
  browser = null;
  await browserServer.close();
  browserServer = null;

  report.osNetwork = {
    ...report.osNetwork,
    ...(await processSampler.stop()),
  };
  processSampler = null;
  report.osNetwork.analysis = analyzeProcessEndpoints(report.osNetwork.samples, allowedEndpoints);

  const parsedNetLog = JSON.parse(await readFile(netLogPath, "utf8"));
  const netLogSummary = parseChromiumNetLog(parsedNetLog);
  report.chromiumNetLog.parsed = true;
  report.chromiumNetLog.relevantEventCount = netLogSummary.relevantEventCount;
  report.chromiumNetLog.analysis = {
    ...analyzeNetLogTargets(netLogSummary, allowedOrigins, allowedEndpoints),
    targetStrings: netLogSummary.targetStrings,
  };

  const browserRequestAnalysis = analyzeBrowserRequests(
    report.browser.playwrightRequests,
    report.browser.cdpEvents,
    report.browser.webSockets,
    allowedOrigins,
  );
  report.browser.requestAnalysis = browserRequestAnalysis;

  const primaryApplicationAttempts = [
    ...report.browser.rejectedRequests.map((attempt) => ({ channel: "playwright-route", ...attempt })),
    ...report.browser.capabilityAnalysis.externalAttempts.map((attempt) => ({ channel: "javascript-capability", ...attempt })),
    ...browserRequestAnalysis.externalAttempts.map((attempt) => ({ channel: "browser-request", ...attempt })),
  ];
  const processCausality = partitionCausalObservations(
    report.osNetwork.analysis.externalAttempts,
    primaryApplicationAttempts,
  );
  const netLogCausality = partitionCausalObservations(
    report.chromiumNetLog.analysis.externalAttempts,
    primaryApplicationAttempts,
  );
  report.osNetwork.causalAttribution = processCausality;
  report.chromiumNetLog.causalAttribution = netLogCausality;
  const externalAttempts = [
    ...primaryApplicationAttempts,
    ...processCausality.applicationAttributable.map((attempt) => ({ channel: "windows-process-endpoint", ...attempt })),
    ...netLogCausality.applicationAttributable.map((attempt) => ({ channel: "chromium-netlog", ...attempt })),
  ];
  report.assertions.browserGlobalExternalObservationCount =
    processCausality.browserGlobalUnattributed.length +
    netLogCausality.browserGlobalUnattributed.length;
  report.assertions.browserGlobalExternalObservations = [
    ...processCausality.browserGlobalUnattributed.map((attempt) => ({ channel: "windows-process-endpoint", ...attempt })),
    ...netLogCausality.browserGlobalUnattributed.map((attempt) => ({ channel: "chromium-netlog", ...attempt })),
  ];
  report.assertions.externalAttemptCount = externalAttempts.length;
  report.assertions.externalAttempts = externalAttempts;
  report.assertions.zeroExternalAttempts = externalAttempts.length === 0;
  report.assertions.loopbackTrafficAccounted =
    serverRequests.length > 0 &&
    browserRequestAnalysis.externalAttempts.length === 0 &&
    processCausality.applicationAttributable.length === 0 &&
    netLogCausality.applicationAttributable.length === 0;

  await captureHostNetworkAdapterSnapshot(report, "postflight");
  report.hostNetworkAdapters.analysis = analyzeHostNetworkAdapters(
    report.hostNetworkAdapters.snapshots,
  );
  report.assertions.hostNetworkAdaptersDisabled =
    report.hostNetworkAdapters.errors.length === 0 &&
    report.hostNetworkAdapters.analysis.adaptersDisabled === true;

  const completeness = assessEvidenceCompleteness(report);
  report.completeness = completeness;
  report.result = completeness.complete ? "PASS" : developmentRun ? "INCONCLUSIVE_DEVELOPMENT" : "FAIL";
} catch (error) {
  workflowError = error;
  report.result = developmentRun ? "INCONCLUSIVE_DEVELOPMENT" : "FAIL";
  report.error = serializeError(error);
} finally {
  if (context !== undefined && context !== null) {
    await context.close().catch(() => undefined);
  }
  if (browser !== undefined && browser !== null) {
    await browser.close().catch(() => undefined);
  }
  if (browserServer !== undefined && browserServer !== null) {
    await browserServer.close().catch(() => undefined);
  }
  if (processSampler !== undefined && processSampler !== null) {
    const stopped = await processSampler.stop();
    report.osNetwork = { ...report.osNetwork, ...stopped };
  }
  if (artifactServer !== undefined) {
    artifactServer.close();
    await once(artifactServer, "close").catch(() => undefined);
  }
  if (report.hostNetworkAdapters.snapshots.length < 2) {
    await captureHostNetworkAdapterSnapshot(report, "postflight-finally");
  }
  report.hostNetworkAdapters.analysis = analyzeHostNetworkAdapters(
    report.hostNetworkAdapters.snapshots,
  );
  report.assertions.hostNetworkAdaptersDisabled =
    report.hostNetworkAdapters.errors.length === 0 &&
    report.hostNetworkAdapters.analysis.adaptersDisabled === true;
  report.completedAt = new Date().toISOString();

  await Promise.all([
    writeJson(browserLogPath, report.browser),
    writeJson(hostAdapterLogPath, report.hostNetworkAdapters),
    writeJson(processLogPath, report.osNetwork),
    writeJson(serverLogPath, {
      allowlistedOrigin: artifactUrl === undefined ? null : new URL(artifactUrl).origin,
      requests: serverRequests,
    }),
  ]);
  await writeJson(reportPath, report);
  await writeEvidenceManifest(manifestPath, outputDirectory, [
    reportPath,
    browserLogPath,
    hostAdapterLogPath,
    processLogPath,
    serverLogPath,
    ...(await fileExists(netLogPath) ? [netLogPath] : []),
    ...(await fileExists(closureEvidenceInputPath) ? [closureEvidenceInputPath] : []),
  ], report);
}

console.log(stableJson({
  candidateExact: report.candidate.exact,
  evidenceDirectory: outputDirectory,
  externalAttemptCount: report.assertions.externalAttemptCount,
  result: report.result,
  workflowCompleted: report.workflow.completed,
}));

if (report.result !== "PASS" && !(developmentRun && workflowError === null)) {
  process.exitCode = 1;
}

function parseArguments(arguments_) {
  const parsed = {};
  for (let index = 0; index < arguments_.length; index += 1) {
    const argument = arguments_[index];
    if (argument === "--development-run") {
      parsed.developmentRun = true;
      continue;
    }
    const key = {
      "--artifact": "artifact",
      "--candidate-ref": "candidateRef",
      "--closure-evidence": "closureEvidence",
      "--output": "output",
      "--root": "root",
    }[argument];
    if (key === undefined || arguments_[index + 1] === undefined) {
      throw new Error(`Unknown or incomplete argument: ${argument}`);
    }
    parsed[key] = arguments_[index + 1];
    index += 1;
  }
  return parsed;
}

function parseClosureEvidence(bytes, candidate_) {
  let value;
  try {
    value = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes));
  } catch (error) {
    throw new Error(`Closure evidence is not strict UTF-8 JSON: ${serializeError(error).message}`);
  }
  if (
    value === null ||
    typeof value !== "object" ||
    Array.isArray(value) ||
    value.schemaVersion !== "1.0" ||
    value.evidenceKind !== "PHASE2_ISOLATION_CLOSURE_INPUT" ||
    value.candidateCommit !== candidate_.commit ||
    value.candidateTree !== candidate_.tree
  ) {
    throw new Error("Closure evidence is not bound to this exact candidate under schema 1.0.");
  }
  for (const field of [
    "boundaryFuzzCoverage",
    "configurationCoverage",
    "fixedNativeBackingAttestation",
    "liveLanTopologyVariation",
    "vendorDeployableExportRejection",
  ]) {
    if (value[field] === null || typeof value[field] !== "object" || Array.isArray(value[field])) {
      throw new Error(`Closure evidence is missing ${field}.`);
    }
  }
  const assessment = assessIsolationClosureEvidence(value, candidate_);
  if (assessment.failures.some((failure) => failure.startsWith("Closure evidence"))) {
    throw new Error(assessment.failures.join("; "));
  }
  return value;
}

async function captureCandidateBinding(root, reference, p2EntryGatePath, harnessFiles) {
  const commit = (await git(root, "rev-parse", "--verify", `${reference}^{commit}`)).trim().toLocaleLowerCase("en-US");
  const tree = (await git(root, "rev-parse", "--verify", `${commit}^{tree}`)).trim().toLocaleLowerCase("en-US");
  const head = (await git(root, "rev-parse", "--verify", "HEAD^{commit}")).trim().toLocaleLowerCase("en-US");
  const porcelain = await git(root, "status", "--porcelain=v1", "-z", "--untracked-files=all");
  const accounted = await accountedUntracked(root, p2EntryGatePath);
  const workspaceChanges = parseGitStatusPorcelainZ(porcelain)
    .filter((change) => !accounted.has(change.path));
  const inputBlobBindings = [];
  for (const file of harnessFiles) {
    const localBytes = await readFile(path.join(root, file));
    let candidateSha256 = null;
    try {
      candidateSha256 = sha256(Buffer.from(await git(root, "show", `${commit}:${file}`, { encoding: "buffer" })));
    } catch {
      candidateSha256 = null;
    }
    inputBlobBindings.push({
      candidateSha256,
      localSha256: sha256(localBytes),
      matchesCandidate: candidateSha256 === sha256(localBytes),
      path: file,
    });
  }
  const exact =
    head === commit &&
    workspaceChanges.length === 0 &&
    inputBlobBindings.every((binding) => binding.matchesCandidate);
  const approvalBinding = inputBlobBindings.find(({ path: inputPath }) => inputPath === ISOLATION_APPROVAL_PATH);
  return {
    commit,
    exact,
    head,
    inputBlobBindings,
    isolationApprovalDecisionId: ISOLATION_APPROVAL_DECISION_ID,
    isolationApprovalSha256: approvalBinding?.candidateSha256 ?? null,
    ref: reference,
    tree,
    workspaceChanges,
  };
}

async function accountedUntracked(root, p2EntryGatePath) {
  const accounted = new Set();
  try {
    const gate = JSON.parse(await readFile(p2EntryGatePath, "utf8"));
    for (const record of gate.accountedWorkspaceState ?? []) {
      const relative = String(record.path ?? "").replaceAll("\\", "/");
      const absolute = path.join(root, relative);
      if (relative !== "" && await fileExists(absolute)) {
        const bytes = await readFile(absolute);
        if (sha256(bytes) === String(record.sha256 ?? "").toLocaleUpperCase("en-US")) {
          accounted.add(relative);
        }
      }
    }
  } catch {
    return accounted;
  }
  return accounted;
}

async function git(root, ...arguments_) {
  let options_ = {};
  if (typeof arguments_.at(-1) === "object") {
    options_ = arguments_.pop();
  }
  const { stdout } = await execFileAsync("git", arguments_, {
    cwd: root,
    encoding: options_.encoding === "buffer" ? "buffer" : "utf8",
    maxBuffer: 16 * 1024 * 1024,
    windowsHide: true,
  });
  return stdout;
}

async function startArtifactServer(artifact, requests) {
  const server = createServer((request, response) => {
    const requestUrl = new URL(request.url ?? "/", "http://127.0.0.1");
    requests.push({
      method: request.method ?? null,
      path: requestUrl.pathname,
      remoteAddress: request.socket.remoteAddress ?? null,
      remotePort: request.socket.remotePort ?? null,
    });
    if (requestUrl.pathname !== "/" && requestUrl.pathname !== "/index.html") {
      response.writeHead(404, { "Content-Type": "text/plain; charset=utf-8" });
      response.end("Not found");
      return;
    }
    response.writeHead(200, {
      "Cache-Control": "no-store",
      "Content-Length": String(artifact.byteLength),
      "Content-Type": "text/html; charset=utf-8",
    });
    response.end(artifact);
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const address = server.address();
  if (address === null || typeof address === "string") {
    throw new Error("Loopback artifact server did not expose a TCP port.");
  }
  return { server, url: `http://127.0.0.1:${address.port}/` };
}

function chromiumIsolationArguments(logPath) {
  return [
    "--disable-background-networking",
    "--disable-breakpad",
    "--disable-client-side-phishing-detection",
    "--disable-component-update",
    "--disable-default-apps",
    "--disable-domain-reliability",
    "--disable-extensions",
    "--disable-features=AutofillServerCommunication,CertificateTransparencyComponentUpdater,DialMediaRouteProvider,DnsOverHttps,InterestFeedContentSuggestions,MediaRouter,NetworkTimeServiceQuerying,OptimizationHints,Translate",
    "--disable-search-engine-choice-screen",
    "--disable-sync",
    "--force-webrtc-ip-handling-policy=disable_non_proxied_udp",
    "--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE 127.0.0.1, EXCLUDE ::1",
    `--log-net-log=${logPath}`,
    "--metrics-recording-only",
    "--net-log-capture-mode=Everything",
    "--no-default-browser-check",
    "--no-first-run",
    "--no-pings",
    "--password-store=basic",
    "--use-mock-keychain",
    "--webrtc-ip-handling-policy=disable_non_proxied_udp",
  ];
}

async function installCapabilityBoundary(browserContext) {
  await browserContext.addInitScript(() => {
    const events = [];
    const record = (api, outcome, target = null, details = null, classification = "denied-capability") => {
      events.push({ api, classification, details, outcome, target, timestamp: performance.now() });
    };
    const targetText = (value) => {
      try {
        return String(value);
      } catch {
        return "<unstringifiable>";
      }
    };
    const denyFunction = (api) => function deniedCapability(...arguments_) {
      record(api, "denied", arguments_.length > 0 ? targetText(arguments_[0]) : null);
      throw new DOMException(`${api} is disabled by the Phase 2 isolation boundary.`, "SecurityError");
    };
    const denyConstructor = (api) => new Proxy(function deniedConstructor() {}, {
      apply(_target, _thisValue, arguments_) {
        record(api, "denied", arguments_.length > 0 ? targetText(arguments_[0]) : null);
        throw new DOMException(`${api} is disabled by the Phase 2 isolation boundary.`, "SecurityError");
      },
      construct(_target, arguments_) {
        record(api, "denied", arguments_.length > 0 ? targetText(arguments_[0]) : null);
        throw new DOMException(`${api} is disabled by the Phase 2 isolation boundary.`, "SecurityError");
      },
    });
    const defineValue = (target, key, value) => {
      try {
        Object.defineProperty(target, key, { configurable: false, enumerable: false, value, writable: false });
      } catch {
        try {
          target[key] = value;
        } catch {
          // A failed patch is exposed by adaptersDisabled below.
        }
      }
    };

    for (const api of ["XMLHttpRequest", "WebSocket", "EventSource", "WebTransport", "RTCPeerConnection", "RTCDataChannel", "SharedWorker"]) {
      defineValue(window, api, denyConstructor(api));
    }
    defineValue(window, "fetch", denyFunction("fetch"));
    defineValue(window, "open", denyFunction("window.open"));
    defineValue(navigator, "sendBeacon", denyFunction("navigator.sendBeacon"));
    defineValue(navigator, "registerProtocolHandler", denyFunction("navigator.registerProtocolHandler"));

    const capabilityObjects = {
      bluetooth: ["getAvailability", "requestDevice"],
      hid: ["getDevices", "requestDevice"],
      mediaDevices: ["enumerateDevices", "getDisplayMedia", "getUserMedia"],
      serial: ["getPorts", "requestPort"],
      serviceWorker: ["getRegistration", "getRegistrations", "register"],
      usb: ["getDevices", "requestDevice"],
    };
    for (const [capability, methods] of Object.entries(capabilityObjects)) {
      const blocked = {};
      for (const method of methods) {
        defineValue(blocked, method, denyFunction(`navigator.${capability}.${method}`));
      }
      if (capability === "serviceWorker") {
        defineValue(blocked, "ready", Promise.reject(new DOMException("Service workers are disabled.", "SecurityError")));
        blocked.ready.catch(() => undefined);
      }
      defineValue(navigator, capability, blocked);
    }
    defineValue(navigator, "requestMIDIAccess", denyFunction("navigator.requestMIDIAccess"));
    defineValue(navigator, "connection", Object.freeze({ downlink: 0, effectiveType: "none", rtt: 0, saveData: true, type: "none" }));

    const workerPrelude = `(() => {
      const marker = "__PHASE2_ISOLATION_CAPABILITY__";
      const text = (value) => { try { return String(value); } catch { return "<unstringifiable>"; } };
      const record = (api, target = null) => console.error(marker + JSON.stringify({ api, classification: "denied-worker-capability", outcome: "denied", target }));
      const deny = (api) => function (...args) { record(api, args.length > 0 ? text(args[0]) : null); throw new DOMException(api + " is disabled.", "SecurityError"); };
      const denyConstructor = (api) => new Proxy(function () {}, {
        apply(_target, _thisValue, args) { record(api, args.length > 0 ? text(args[0]) : null); throw new DOMException(api + " is disabled.", "SecurityError"); },
        construct(_target, args) { record(api, args.length > 0 ? text(args[0]) : null); throw new DOMException(api + " is disabled.", "SecurityError"); },
      });
      const set = (target, key, value) => { try { Object.defineProperty(target, key, { configurable: false, value, writable: false }); } catch {} };
      set(self, "fetch", deny("worker.fetch"));
      set(self, "importScripts", deny("worker.importScripts"));
      for (const api of ["XMLHttpRequest", "WebSocket", "EventSource", "WebTransport", "RTCPeerConnection", "Worker", "SharedWorker"]) set(self, api, denyConstructor("worker." + api));
      if (typeof navigator === "object" && navigator !== null) {
        for (const capability of ["serial", "usb", "bluetooth", "hid", "serviceWorker"]) {
          set(navigator, capability, new Proxy({}, { get(_target, property) { return deny("worker.navigator." + capability + "." + String(property)); } }));
        }
        set(navigator, "requestMIDIAccess", deny("worker.navigator.requestMIDIAccess"));
        set(navigator, "sendBeacon", deny("worker.navigator.sendBeacon"));
      }
    })();\n`;
    const NativeBlob = window.Blob;
    const BlobBoundary = new Proxy(NativeBlob, {
      construct(target, arguments_, newTarget) {
        const parts = Array.isArray(arguments_[0]) ? arguments_[0] : [];
        const options = arguments_[1] ?? {};
        const type = String(options.type ?? "").toLocaleLowerCase("en-US");
        if (type.includes("javascript") || type.includes("ecmascript")) {
          record(
            "WorkerBlob.instrumentation",
            "instrumented",
            null,
            { partCount: parts.length, type },
            "observed-safe-metadata",
          );
          return Reflect.construct(target, [[workerPrelude, ...parts], options], newTarget);
        }
        return Reflect.construct(target, arguments_, newTarget);
      },
    });
    defineValue(window, "Blob", BlobBoundary);

    const NativeWorker = window.Worker;
    const WorkerBoundary = new Proxy(NativeWorker, {
      construct(target, arguments_, newTarget) {
        const workerTarget = targetText(arguments_[0]);
        if (!workerTarget.startsWith("blob:")) {
          record("Worker", "denied", workerTarget);
          throw new DOMException("Only the packaged internal blob worker is allowed.", "SecurityError");
        }
        record("Worker", "allowed", workerTarget, { name: arguments_[1]?.name ?? null }, "allowed-internal-blob-worker");
        return Reflect.construct(target, arguments_, newTarget);
      },
    });
    defineValue(window, "Worker", WorkerBoundary);

    defineValue(window, "showOpenFilePicker", undefined);
    defineValue(window, "showSaveFilePicker", undefined);

    defineValue(window, "__phase2IsolationSnapshot", () => ({
      adaptersDisabled:
        window.showOpenFilePicker === undefined &&
        window.showSaveFilePicker === undefined &&
        window.Worker === WorkerBoundary &&
        typeof navigator.serial?.requestPort === "function",
      events: structuredClone(events),
    }));
  });
}

async function executePackagedWorkflow(page, workflow) {
  await page.getByLabel("Project name").fill("Isolation counterfactual cell");
  await page.getByRole("button", { name: /^Create/u }).click();
  await page.getByRole("heading", { level: 1, name: "Isolation counterfactual cell" }).waitFor();
  workflow.commands.push("create-project");

  for (const fuzzCase of DEFAULT_FUZZ_CASES) {
    const observation = await exerciseTextBoundary(page, fuzzCase, "project-display-name");
    workflow.fuzzCases.push(observation);
  }
  await renameSelectedObject(page, "Isolation counterfactual cell");

  await addObject(page, "Virtual network");
  await page.getByRole("heading", { level: 1, name: "Virtual network" }).waitFor();
  await treeItem(page, "Isolation counterfactual cell").click();
  await addObject(page, "Controller");
  await page.getByRole("heading", { level: 1, name: "Controller" }).waitFor();
  await addObject(page, "Rack");
  await page.getByRole("heading", { level: 1, name: "Local rack" }).waitFor();
  await addObject(page, "Digital input module");
  await page.getByRole("heading", { level: 1, name: "VDI16" }).waitFor();
  await treeItem(page, "Local rack").click();
  await addObject(page, "Digital output module");
  await page.getByRole("heading", { level: 1, name: "VDO16" }).waitFor();
  workflow.commands.push("create-virtual-network-controller-rack-and-io");

  await treeItem(page, "Controller").click();
  await addObject(page, "Reusable SCL function");
  await page.getByRole("heading", { level: 1, name: "Function" }).waitFor();
  for (const fuzzCase of DEFAULT_FUZZ_CASES) {
    const source = `Result := NOT InputValue;\n// isolation-fuzz ${fuzzCase.value}`;
    const editor = page.getByLabel("SCL source");
    await editor.fill(source);
    await page.getByRole("button", { name: "Apply SCL source" }).click();
    await waitForOperationSettled(page);
    workflow.fuzzCases.push({
      boundary: "scl-source-text",
      category: fuzzCase.category,
      id: fuzzCase.id,
      injected: true,
      observedAlert: await visibleAlertText(page),
      valueSha256: sha256(Buffer.from(fuzzCase.value, "utf8")),
    });
  }
  await page.getByLabel("SCL source").fill("Result := NOT InputValue;");
  await page.getByRole("button", { name: "Apply SCL source" }).click();
  await waitForOperationSettled(page);

  await treeItem(page, "Controller").click();
  await addObject(page, "Ladder organization block");
  await page.getByRole("heading", { level: 1, name: "MainCycle" }).waitFor();
  await treeItem(page, "Controller").click();
  await addObject(page, "Tag table");
  await page.getByRole("heading", { level: 1, name: "PLC tags" }).waitFor();
  await addObject(page, "Input tag");
  await page.getByRole("heading", { level: 1, name: "Input" }).waitFor();
  await treeItem(page, "PLC tags").click();
  await addObject(page, "Output tag");
  await page.getByRole("heading", { level: 1, name: "Output" }).waitFor();
  workflow.commands.push("author-mixed-language-program-and-bound-io-tags");

  await page.getByRole("tab", { name: "Runtime & commissioning" }).click();
  await page.getByRole("button", { name: "Build", exact: true }).click();
  await waitForBuildCurrent(page);
  await page.getByRole("button", { name: "Power on", exact: true }).click();
  await waitForText(page.locator(".runtime-toolbar__identity strong"), "Stop");
  await page.getByRole("button", { name: "Preview load", exact: true }).click();
  await page.getByRole("region", { name: "Virtual Download preview" }).waitFor();
  if (await page.locator('input[type="url"], input[placeholder*="address" i], input[placeholder*="endpoint" i]').count() !== 0) {
    throw new Error("Virtual Download exposed a host endpoint input instead of a virtual-controller-only target.");
  }
  await page.getByRole("button", { name: "Commit load", exact: true }).click();
  await page.getByRole("button", { name: "Go online", exact: true }).click();
  await page.getByText("Online session active", { exact: true }).waitFor();
  await page.getByRole("button", { name: "RUN", exact: true }).click();
  await waitForText(page.locator(".runtime-toolbar__identity strong"), "Run");
  await page.getByRole("button", { name: "Scan +1", exact: true }).click();
  await waitForCondition(async () => {
    const row = page.locator(".runtime-summary dl > div").filter({ hasText: "Scan sequence" });
    return (await row.locator("dd").innerText()).trim() === "1";
  }, "virtual controller did not complete an offline scan");
  workflow.commands.push("build-load-online-run-and-scan-virtual-controller");
  workflow.observations = {
    exportButtons: await page.getByRole("button", { name: /(?:export|deploy|download to device)/iu }).count(),
    filePickersAvailable: await page.evaluate(() => ({
      open: typeof window.showOpenFilePicker === "function",
      save: typeof window.showSaveFilePicker === "function",
    })),
    runtimeState: (await page.locator(".runtime-toolbar__identity strong").innerText()).trim(),
    scanSequence: "1",
    virtualOnlyVisible: (await page.locator(".status-segment--safe").innerText()).includes("Virtual only"),
  };
  if (workflow.observations.exportButtons !== 0) {
    throw new Error("An unverified export/deploy surface is visible in the Phase 2 workbench.");
  }
}

async function exerciseTextBoundary(page, fuzzCase, boundary) {
  const input = page.getByLabel("Name");
  let injected = false;
  try {
    await input.fill(fuzzCase.value);
    injected = true;
  } catch {
    await input.evaluate((element, value) => {
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
      setter?.call(element, value);
      element.dispatchEvent(new Event("input", { bubbles: true }));
    }, fuzzCase.value);
    injected = true;
  }
  await page.getByRole("button", { name: "Apply name" }).click();
  await waitForOperationSettled(page);
  const heading = page.locator(".editor-title h1");
  return {
    boundary,
    category: fuzzCase.category,
    id: fuzzCase.id,
    injected,
    observedHeading: await heading.count() > 0 ? (await heading.first().innerText()).trim() : null,
    observedAlert: await visibleAlertText(page),
    valueSha256: sha256(Buffer.from(fuzzCase.value, "utf8")),
  };
}

async function renameSelectedObject(page, value) {
  await page.getByLabel("Name").fill(value);
  await page.getByRole("button", { name: "Apply name" }).click();
  await waitForOperationSettled(page);
  await page.getByRole("heading", { level: 1, name: value }).waitFor();
}

async function visibleAlertText(page) {
  const alert = page.getByRole("alert");
  if (await alert.count() === 0 || !(await alert.first().isVisible())) {
    return null;
  }
  return (await alert.first().innerText()).trim();
}

async function waitForOperationSettled(page) {
  await page.waitForTimeout(20);
  await waitForCondition(
    async () => await page.locator(".status-segment--busy").count() === 0,
    "workbench remained busy",
  );
}

async function waitForBuildCurrent(page) {
  await waitForCondition(async () => {
    const current = page.getByText("Build current", { exact: true });
    if (await current.count() > 0 && await current.isVisible()) {
      return true;
    }
    const alert = page.getByRole("alert");
    if (await alert.count() > 0 && await alert.isVisible()) {
      throw new Error(`runtime build failed: ${await alert.innerText()}`);
    }
    return false;
  }, "runtime build did not become current");
}

async function waitForText(locator, expected) {
  await waitForCondition(async () => (await locator.innerText()).trim() === expected, `text did not become ${expected}`);
}

async function waitForCondition(condition, message, timeoutMilliseconds = 30_000) {
  const deadline = Date.now() + timeoutMilliseconds;
  let lastError = null;
  while (Date.now() < deadline) {
    try {
      if (await condition()) {
        return;
      }
    } catch (error) {
      lastError = error;
    }
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  throw new Error(`${message}${lastError instanceof Error ? `: ${lastError.message}` : ""}`);
}

async function addObject(page, menuItemName) {
  await page.getByRole("button", { name: "Add engineering object" }).click();
  await page.getByRole("menuitem", { name: new RegExp(menuItemName, "u") }).click();
}

function treeItem(page, text) {
  return page.getByRole("treeitem", { exact: true, name: text });
}

function compactCdpEvent(eventName, event) {
  return {
    event: eventName,
    initiatorType: event.initiator?.type ?? null,
    method: event.request?.method ?? null,
    requestId: event.requestId ?? null,
    target: event.request?.url ?? event.url ?? null,
    type: event.type ?? null,
  };
}

function analyzeBrowserRequests(requests, cdpEvents, sockets, allowedOrigins) {
  const all = [
    ...requests.map((request) => ({ source: "playwright", target: request.target })),
    ...cdpEvents.filter((event) => event.target !== null).map((event) => ({ source: `cdp:${event.event}`, target: event.target })),
    ...sockets.map((socket) => ({ source: "playwright-websocket", target: socket.target })),
  ];
  const externalAttempts = [];
  const loopbackAccounted = [];
  for (const record of all) {
    if (/^(?:blob|data|about):/iu.test(record.target)) {
      loopbackAccounted.push({ ...record, classification: "internal-or-inert" });
      continue;
    }
    try {
      const target = new URL(record.target);
      if (allowedOrigins.has(target.origin)) {
        loopbackAccounted.push({ ...record, classification: "allowlisted-loopback" });
      } else {
        externalAttempts.push({ ...record, classification: "unapproved-origin" });
      }
    } catch {
      externalAttempts.push({ ...record, classification: "malformed-target" });
    }
  }
  return { externalAttempts, loopbackAccounted };
}

function startWindowsProcessEndpointSampler(rootPid, intervalMilliseconds) {
  const samples = [];
  const errors = [];
  let stopped = false;
  let inFlight = null;
  let timer = null;
  const sample = async () => {
    if (stopped) {
      return;
    }
    try {
      const observed = await captureWindowsProcessEndpoints(rootPid);
      samples.push(observed);
    } catch (error) {
      errors.push(serializeError(error));
    } finally {
      if (!stopped) {
        timer = setTimeout(() => {
          inFlight = sample();
        }, intervalMilliseconds);
      }
    }
  };
  inFlight = sample();
  return {
    async stop() {
      stopped = true;
      if (timer !== null) {
        clearTimeout(timer);
      }
      await inFlight?.catch(() => undefined);
      return {
        errors,
        samplerComplete: platform() === "win32" && samples.length > 0 && errors.length === 0,
        samples,
      };
    },
  };
}

async function captureWindowsProcessEndpoints(rootPid) {
  if (platform() !== "win32") {
    throw new Error("Process-attributable endpoint capture is implemented only for Windows.");
  }
  const script = [
    "$ErrorActionPreference = 'Stop'",
    `$rootPid = ${Number(rootPid)}`,
    "$all = @(Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId,Name,ExecutablePath)",
    "$ids = [System.Collections.Generic.HashSet[int]]::new()",
    "[void]$ids.Add([int]$rootPid)",
    "$changed = $true",
    "while ($changed) { $changed = $false; foreach ($p in $all) { if ($ids.Contains([int]$p.ParentProcessId) -and -not $ids.Contains([int]$p.ProcessId)) { [void]$ids.Add([int]$p.ProcessId); $changed = $true } } }",
    "$procs = @($all | Where-Object { $ids.Contains([int]$_.ProcessId) })",
    "$tcp = @(Get-NetTCPConnection -ErrorAction Stop | Where-Object { $ids.Contains([int]$_.OwningProcess) } | ForEach-Object { [pscustomobject]@{ protocol='TCP'; localAddress=$_.LocalAddress; localPort=$_.LocalPort; remoteAddress=$_.RemoteAddress; remotePort=$_.RemotePort; state=[string]$_.State; owningProcess=$_.OwningProcess } })",
    "$udp = @(Get-NetUDPEndpoint -ErrorAction Stop | Where-Object { $ids.Contains([int]$_.OwningProcess) } | ForEach-Object { [pscustomobject]@{ protocol='UDP'; localAddress=$_.LocalAddress; localPort=$_.LocalPort; remoteAddress=''; remotePort=0; state='BOUND'; owningProcess=$_.OwningProcess } })",
    "[pscustomobject]@{ processes=$procs; endpoints=@($tcp + $udp) } | ConvertTo-Json -Compress -Depth 6",
  ].join("; ");
  const { stdout } = await execFileAsync(
    "powershell.exe",
    ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", script],
    { encoding: "utf8", maxBuffer: 8 * 1024 * 1024, windowsHide: true },
  );
  const parsed = JSON.parse(stdout.trim());
  const processes = asArray(parsed.processes);
  const tree = deriveProcessTree(processes, rootPid);
  const processIds = new Set(tree.map((process) => process.pid));
  return {
    capturedAt: new Date().toISOString(),
    endpoints: asArray(parsed.endpoints).filter((endpoint) => processIds.has(Number(endpoint.owningProcess))),
    processes: tree,
    rootPid,
  };
}

async function captureHostNetworkAdapterSnapshot(report_, boundary) {
  try {
    report_.hostNetworkAdapters.snapshots.push({
      ...(await captureWindowsNetworkAdapters()),
      boundary,
    });
  } catch (error) {
    report_.hostNetworkAdapters.errors.push({ boundary, ...serializeError(error) });
  }
}

async function captureWindowsNetworkAdapters() {
  if (platform() !== "win32") {
    throw new Error("Host network-adapter capture is implemented only for Windows.");
  }
  const script = [
    "$ErrorActionPreference = 'Stop'",
    "$adapters = @(Get-NetAdapter -IncludeHidden -ErrorAction Stop | Select-Object Name,InterfaceDescription,ifIndex,Status,MediaConnectionState)",
    "[pscustomobject]@{ adapters=$adapters } | ConvertTo-Json -Compress -Depth 5",
  ].join("; ");
  const { stdout } = await execFileAsync(
    "powershell.exe",
    ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", script],
    { encoding: "utf8", maxBuffer: 4 * 1024 * 1024, windowsHide: true },
  );
  const parsed = JSON.parse(stdout.trim());
  return {
    adapters: asArray(parsed.adapters),
    capturedAt: new Date().toISOString(),
  };
}

function asArray(value) {
  if (value === null || value === undefined) {
    return [];
  }
  return Array.isArray(value) ? value : [value];
}

async function findBrowser() {
  for (const candidate_ of browserCandidates) {
    try {
      await access(candidate_);
      return candidate_;
    } catch {
      // Continue to the next admitted local Chromium browser.
    }
  }
  throw new Error("No admitted system Chromium browser was found.");
}

function normalizedChromiumRuntimeProduct(executablePath) {
  const executable = path.basename(executablePath).toLocaleLowerCase("en-US");
  if (executable === "chrome.exe") {
    return "google-chrome";
  }
  if (executable === "msedge.exe") {
    return "microsoft-edge";
  }
  throw new Error("The admitted Chromium executable has no normalized product identity.");
}

async function writeJson(filePath, value) {
  const temporary = `${filePath}.tmp`;
  await writeFile(temporary, stableJson(value), { encoding: "utf8", flag: "w" });
  await rename(temporary, filePath);
}

async function writeEvidenceManifest(filePath, directory, files, report_) {
  const records = [];
  for (const file of files) {
    const bytes = await readFile(file);
    records.push({
      bytes: bytes.byteLength,
      path: path.relative(directory, file).replaceAll("\\", "/"),
      sha256: sha256(bytes),
    });
  }
  await writeJson(filePath, {
    candidate: report_.candidate,
    complete: report_.completeness?.complete === true,
    configuration: report_.configuration,
    evidenceFiles: records,
    evidenceKind: "PHASE2_ISOLATION_EVIDENCE_MANIFEST",
    result: report_.result,
    schemaVersion: EVIDENCE_SCHEMA_VERSION,
    verificationIds: ISOLATION_VERIFICATION_IDS,
  });
}

async function fileExists(filePath) {
  try {
    await stat(filePath);
    return true;
  } catch {
    return false;
  }
}

function serializeError(error) {
  return error instanceof Error
    ? { message: error.message, name: error.name, stack: error.stack ?? null }
    : { message: String(error), name: "UnknownError", stack: null };
}
