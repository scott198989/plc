import { createHash } from "node:crypto";
import { isIP } from "node:net";
import { lstat, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  analyzeNetLogTargets,
  isLoopbackHost,
  isUnspecifiedHost,
  parseChromiumNetLog,
  splitEndpoint,
} from "./isolation-counterfactual-lib.mjs";

const scriptPath = fileURLToPath(import.meta.url);
const root = path.resolve(path.dirname(scriptPath), "..", "..");
const analysisLibraryPath = path.join(root, "tools", "phase2", "isolation-counterfactual-lib.mjs");
const evidenceRoot = path.join(root, ".phase2-verification", "native-e2e");
const observerPath = path.join(evidenceRoot, "native-platform-observer-manifest.json");
const finalPath = path.join(evidenceRoot, "native-platform-evidence-manifest.json");
const networkAnalysisPath = path.join(evidenceRoot, "native-network-analysis.json");
const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex").toUpperCase();
const stableJson = (value) => `${JSON.stringify(value, null, 2)}\n`;
const SHA256 = /^[A-F0-9]{64}$/u;
const GIT_OBJECT = /^[a-f0-9]{40}$/u;
const RELEVANT_EVENT = /(?:DNS|HOST_RESOLVER|URL_REQUEST|SOCKET|TCP|UDP|QUIC|WEBSOCKET|WEBTRANSPORT|HTTP_STREAM|PROXY|CONNECT)/iu;
const CONFIGURATION_EVENT = /(?:DNS_CONFIG_CHANGED|NETWORK_CHANGED|NETWORK_IP_ADDRESSES_CHANGED|PROXY_CONFIG_CHANGED)/iu;
const TARGET_KEY = /(?:^|_)(?:url|uri|host|hostname|address|endpoint|destination|origin|proxy|socket)(?:$|_)/iu;
const URL_FRAGMENT = /[a-z][a-z0-9+.-]*:\/\/[^\s<>"']+/giu;
const IP_ENDPOINT_FRAGMENT = /\[[0-9a-f:%]+\]:\d+|(?:\d{1,3}\.){3}\d{1,3}:\d+/giu;

function requireCondition(condition, message) {
  if (!condition) throw new Error(message);
}

async function readBoundedRegularFile(file, maximum) {
  const status = await lstat(file);
  requireCondition(status.isFile() && !status.isSymbolicLink(), `Evidence input is not a regular file: ${path.basename(file)}`);
  requireCondition(status.size > 0 && status.size <= maximum, `Evidence input has an invalid bounded size: ${path.basename(file)}`);
  return readFile(file);
}

export function validateEvidenceRows(rows, filesByPath) {
  requireCondition(Array.isArray(rows) && rows.length > 0, "Observer evidenceFiles is empty");
  const names = rows.map((row) => row?.path);
  const sorted = [...names].sort((left, right) => left.localeCompare(right, "en"));
  requireCondition(
    new Set(names).size === names.length && names.every((name, index) => name === sorted[index]),
    "Observer evidenceFiles must be sorted and unique",
  );
  for (const row of rows) {
    requireCondition(
      row !== null && typeof row === "object" && !Array.isArray(row) &&
      Object.keys(row).sort().join("\0") === "bytes\0path\0sha256" &&
      typeof row.path === "string" && !row.path.includes("/") && !row.path.includes("\\") &&
      Number.isSafeInteger(row.bytes) && row.bytes > 0 && SHA256.test(row.sha256),
      "Observer evidenceFiles contains an invalid row",
    );
    const bytes = filesByPath.get(row.path);
    requireCondition(Buffer.isBuffer(bytes), `Observer evidence file is missing: ${row.path}`);
    requireCondition(
      bytes.byteLength === row.bytes && sha256(bytes) === row.sha256,
      `Observer evidence hash drifted: ${row.path}`,
    );
  }
  return true;
}

export function validateRawHostManifest(raw) {
  requireCondition(
    raw !== null && typeof raw === "object" && !Array.isArray(raw) &&
    raw.schemaVersion === "1.0" &&
    raw.evidenceKind === "WINDOWS_NATIVE_BRIDGE_RAW_RUN" && raw.result === "PASS" &&
    raw.fixedLocalBacking === true && raw.providerBacked === false && raw.remote === false &&
    raw.removable === false && raw.special === false && raw.redirected === false &&
    raw.metadataOnlyBeforeAcceptance === true &&
    raw.selectedByteIoBeforeAcceptance === false && raw.verificationStage === 4 &&
    JSON.stringify(raw.operations) === JSON.stringify(["create", "open", "replace"]) &&
    raw.instrumentationStatus === "REQUIRES_EXTERNAL_HARNESS" &&
    SHA256.test(raw.controlledInputSha256) && SHA256.test(raw.deterministicOutputSha256) &&
    SHA256.test(raw.runtimeReplaySha256) && SHA256.test(raw.canonicalReplaySha256) &&
    Number.isSafeInteger(raw.verifiedReplayEventCount) &&
    raw.verifiedReplayEventCount > 0 && raw.verifiedReplayEventCount <= 1_000_000 &&
    Number.isSafeInteger(raw.verifiedReplayBoundaryCount) &&
    raw.verifiedReplayBoundaryCount > 0 && raw.verifiedReplayBoundaryCount <= 1_000_000,
    "The raw host manifest did not complete the native bridge and verified replay journey",
  );
  return {
    controlledInputSha256: raw.controlledInputSha256,
    deterministicOutputSha256: raw.deterministicOutputSha256,
    runtimeReplaySha256: raw.runtimeReplaySha256,
    canonicalReplaySha256: raw.canonicalReplaySha256,
    verifiedReplayEventCount: raw.verifiedReplayEventCount,
    verifiedReplayBoundaryCount: raw.verifiedReplayBoundaryCount,
  };
}

function reverseNumericMap(record) {
  const result = new Map();
  if (record === null || typeof record !== "object" || Array.isArray(record)) return result;
  for (const [name, value] of Object.entries(record)) {
    if (Number.isSafeInteger(value)) result.set(value, name);
  }
  return result;
}

function collectStringLeaves(value, keyPath = [], output = []) {
  if (typeof value === "string") {
    output.push({ keyPath, value });
    return output;
  }
  if (value === null || typeof value !== "object") return output;
  if (Array.isArray(value)) {
    for (const item of value) collectStringLeaves(item, keyPath, output);
    return output;
  }
  for (const [key, item] of Object.entries(value)) {
    collectStringLeaves(item, [...keyPath, key], output);
  }
  return output;
}

function normalizeHost(value) {
  let host = String(value ?? "").trim().toLocaleLowerCase("en-US");
  if (host.startsWith("[") && host.endsWith("]")) host = host.slice(1, -1);
  const zone = host.indexOf("%");
  if (zone >= 0) host = host.slice(0, zone);
  return host;
}

function classifyTarget(target) {
  const value = String(target ?? "").trim();
  if (value.length === 0) return { classification: "ignored-empty", value };
  if (/^(?:blob|data|about|chrome|chrome-extension):/iu.test(value)) {
    return { classification: "allowed-internal-scheme", value };
  }
  if (/^[a-z][a-z0-9+.-]*:\/\//iu.test(value)) {
    try {
      const parsed = new URL(value);
      if (parsed.origin === "https://govs-plc.local") {
        return { classification: "allowed-fixed-virtual-host", value: parsed.href };
      }
      return { classification: "external-or-unapproved-url", value: parsed.href };
    } catch {
      return { classification: "unknown-malformed-url", value };
    }
  }
  const endpoint = splitEndpoint(value);
  const host = normalizeHost(endpoint.host);
  if (host === "govs-plc.local") {
    return { classification: "allowed-fixed-virtual-host", value };
  }
  if (isIP(host) !== 0 || host === "localhost" || host.includes(".") || host === "~notfound") {
    return { classification: "external-or-unapproved-host-address", value };
  }
  return { classification: "unknown-network-target", value };
}

function candidateTargets(leaf) {
  const candidates = new Set([
    ...(leaf.value.match(URL_FRAGMENT) ?? []),
    ...(leaf.value.match(IP_ENDPOINT_FRAGMENT) ?? []),
  ]);
  const key = leaf.keyPath.at(-1) ?? "";
  if (TARGET_KEY.test(key) && leaf.value.trim() !== "") candidates.add(leaf.value.trim());
  return [...candidates];
}

export function analyzeBoundNetLogObject(netLog) {
  requireCondition(netLog !== null && typeof netLog === "object" && !Array.isArray(netLog), "Chromium NetLog root is malformed");
  requireCondition(netLog.constants?.logCaptureMode === "Everything", "Chromium NetLog was not captured in Everything mode");
  requireCondition(Array.isArray(netLog.events) && netLog.events.length > 0, "Chromium NetLog event stream is empty");
  const parsed = parseChromiumNetLog(netLog);
  requireCondition(parsed.relevantEventCount > 0, "Chromium NetLog contains no relevant network events");
  const baseline = analyzeNetLogTargets(
    parsed,
    new Set(["https://govs-plc.local"]),
    new Set(),
  );
  const typeNames = reverseNumericMap(netLog.constants.logEventTypes);
  const externalTargets = baseline.externalAttempts.map((record) => ({
    classification: "audited-netlog-external-target",
    typeName: record.typeName,
    value: record.value,
  }));
  const unknownTargets = [];
  const allowedTargets = baseline.loopbackAccounted.map((record) => ({
    classification: "audited-netlog-accounted-target",
    typeName: record.typeName,
    value: record.value,
  }));
  const configurationObservations = [...baseline.configurationObservations];
  let actionableEventCount = 0;
  for (const event of netLog.events) {
    const typeName = typeNames.get(event?.type) ?? String(event?.type ?? "");
    if (!RELEVANT_EVENT.test(typeName)) continue;
    if (CONFIGURATION_EVENT.test(typeName)) {
      configurationObservations.push({ typeName });
      continue;
    }
    ++actionableEventCount;
    for (const leaf of collectStringLeaves(event?.params ?? {})) {
      for (const target of candidateTargets(leaf)) {
        const classified = { ...classifyTarget(target), keyPath: leaf.keyPath.join("."), typeName };
        if (classified.classification.startsWith("allowed-")) {
          allowedTargets.push(classified);
        } else if (classified.classification.startsWith("unknown-")) {
          unknownTargets.push(classified);
        } else {
          externalTargets.push(classified);
        }
      }
    }
  }
  requireCondition(actionableEventCount > 0, "Chromium NetLog contains only configuration observations");
  const deduplicate = (records) => {
    const seen = new Set();
    return records.filter((record) => {
      const key = JSON.stringify(record);
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
  };
  return {
    actionableEventCount,
    allowedTargets: deduplicate(allowedTargets),
    configurationObservationCount: configurationObservations.length,
    externalTargets: deduplicate(externalTargets),
    relevantEventCount: parsed.relevantEventCount,
    unknownTargets: deduplicate(unknownTargets),
  };
}

export function parseBoundNetLogText(text) {
  let value;
  try {
    value = JSON.parse(text);
  } catch (error) {
    throw new Error(`Chromium NetLog JSON is malformed or truncated: ${error.message}`);
  }
  return analyzeBoundNetLogObject(value);
}

function externalEndpointReason(endpoint) {
  const protocol = String(endpoint?.protocol ?? "").toLocaleLowerCase("en-US");
  const localAddress = String(endpoint?.localAddress ?? "");
  const remoteAddress = String(endpoint?.remoteAddress ?? "");
  const localPort = Number(endpoint?.localPort ?? -1);
  const remotePort = Number(endpoint?.remotePort ?? -1);
  const state = Number(endpoint?.state ?? -1);
  if (!Number.isInteger(localPort) || localPort < 0 || localPort > 65_535 ||
      !Number.isInteger(remotePort) || remotePort < 0 || remotePort > 65_535) {
    return "malformed-endpoint-port";
  }
  if (protocol === "udp") {
    return isLoopbackHost(localAddress) ? null : "non-loopback-or-wildcard-udp-ownership-observation";
  }
  if (protocol !== "tcp") return "unknown-endpoint-protocol";
  if (state === 2) {
    return isLoopbackHost(localAddress) || isUnspecifiedHost(localAddress)
      ? null
      : "non-loopback-tcp-listener";
  }
  if (remotePort === 0 && isUnspecifiedHost(remoteAddress)) return null;
  return "unapproved-tcp-remote";
}

export function analyzeProcessEvidence(processEvidence, browserExecutableSha256) {
  requireCondition(
    processEvidence?.schemaVersion === "1.0" &&
    processEvidence?.evidenceKind === "WINDOWS_NATIVE_EXTERNAL_PROCESS_ENDPOINT_CAPTURE" &&
    processEvidence?.captureComplete === true &&
    processEvidence?.snapshotIntervalMilliseconds === 50 &&
    Number.isSafeInteger(processEvidence?.snapshotCount) && processEvidence.snapshotCount > 0 &&
    Array.isArray(processEvidence?.processes) && Array.isArray(processEvidence?.endpoints),
    "External process/endpoint capture is incomplete",
  );
  const runtimeProcesses = processEvidence.processes.filter((process) =>
    process?.imageName === "msedgewebview2.exe" &&
    process?.executableSha256 === browserExecutableSha256);
  requireCondition(runtimeProcesses.length > 0, "Process capture is not bound to the attested WebView2 runtime");
  const externalEndpoints = [];
  const accountedEndpoints = [];
  for (const endpoint of processEvidence.endpoints) {
    const reason = externalEndpointReason(endpoint);
    const normalized = {
      family: endpoint?.family,
      localAddress: endpoint?.localAddress,
      localPort: endpoint?.localPort,
      processId: endpoint?.processId,
      protocol: endpoint?.protocol,
      remoteAddress: endpoint?.remoteAddress,
      remotePort: endpoint?.remotePort,
      state: endpoint?.state,
    };
    if (reason === null && endpoint?.external === false) {
      accountedEndpoints.push(normalized);
    } else {
      externalEndpoints.push({ ...normalized, reason: reason ?? "launcher-classified-external" });
    }
  }
  return { accountedEndpoints, externalEndpoints, runtimeProcessCount: runtimeProcesses.length };
}

async function realEvidenceFiles(observer) {
  const files = new Map();
  for (const row of observer.evidenceFiles ?? []) {
    const file = path.join(evidenceRoot, String(row?.path ?? ""));
    const bytes = await readBoundedRegularFile(file, 256 * 1024 * 1024);
    files.set(row.path, bytes);
  }
  validateEvidenceRows(observer.evidenceFiles, files);
  return files;
}

async function main() {
  requireCondition(process.argv.slice(2).length === 0, "The native evidence finalizer accepts zero arguments");
  const observerBytes = await readBoundedRegularFile(observerPath, 2 * 1024 * 1024);
  const observer = JSON.parse(observerBytes.toString("utf8"));
  requireCondition(
    observer?.schemaVersion === "1.0" &&
    observer?.evidenceKind === "WINDOWS_NATIVE_PRODUCT_PATH_OBSERVER_MANIFEST" &&
    GIT_OBJECT.test(observer?.candidateCommit) && GIT_OBJECT.test(observer?.candidateTree) &&
    SHA256.test(observer?.candidateManifestSha256) &&
    SHA256.test(observer?.reviewedRequirementMappingSha256) &&
    SHA256.test(observer?.controlledInputSha256) &&
    SHA256.test(observer?.deterministicOutputSha256) &&
    SHA256.test(observer?.runtimeReplaySha256) &&
    SHA256.test(observer?.canonicalReplaySha256) &&
    Number.isSafeInteger(observer?.verifiedReplayEventCount) &&
    observer.verifiedReplayEventCount > 0 &&
    Number.isSafeInteger(observer?.verifiedReplayBoundaryCount) &&
    observer.verifiedReplayBoundaryCount > 0 &&
    observer?.productionPathExercised === true && observer?.shellExitCode === 0 &&
    observer?.runtimeBackingAttested === true &&
    observer?.instrumentationStatus === "REQUIRES_BOUND_NETLOG_ANALYSIS" &&
    observer?.instrumentationComplete === false && observer?.zeroExternalAttempts === false,
    "The immutable native observer manifest is not complete raw product-path evidence",
  );
  const scriptBytes = await readBoundedRegularFile(scriptPath, 2 * 1024 * 1024);
  const libraryBytes = await readBoundedRegularFile(analysisLibraryPath, 4 * 1024 * 1024);
  requireCondition(
    sha256(scriptBytes) === observer.nativeEvidenceFinalizerSha256 &&
    sha256(libraryBytes) === observer.isolationAnalysisLibrarySha256,
    "The bound native evidence analyzer source drifted",
  );
  const files = await realEvidenceFiles(observer);
  const candidate = JSON.parse(files.get("candidate-package-manifest.json").toString("utf8"));
  requireCondition(
    sha256(files.get("candidate-package-manifest.json")) === observer.candidateManifestSha256 &&
    candidate?.gitCommit === observer.candidateCommit &&
    candidate?.gitTree === observer.candidateTree &&
    candidate?.developmentDirty === observer.candidateDevelopmentDirty &&
    candidate?.packageContractSha256 === observer.candidatePackageContractSha256 &&
    candidate?.reviewedRequirementMappingSha256 === observer.reviewedRequirementMappingSha256,
    "The immutable observer is not bound to its exact candidate manifest",
  );
  const raw = JSON.parse(files.get("native-run-raw.json").toString("utf8"));
  const replay = validateRawHostManifest(raw);
  requireCondition(
    sha256(files.get("native-run-raw.json")) === observer.rawHostManifestSha256 &&
    sha256(files.get("native-netlog.json")) === observer.chromiumNetLogSha256 &&
    sha256(files.get("native-process-endpoints.json")) ===
      observer.externalProcessEvidenceSha256 &&
    replay.runtimeReplaySha256 === observer.runtimeReplaySha256 &&
    replay.canonicalReplaySha256 === observer.canonicalReplaySha256 &&
    replay.controlledInputSha256 === observer.controlledInputSha256 &&
    replay.deterministicOutputSha256 === observer.deterministicOutputSha256 &&
    replay.verifiedReplayEventCount === observer.verifiedReplayEventCount &&
    replay.verifiedReplayBoundaryCount === observer.verifiedReplayBoundaryCount,
    "The immutable observer is not bound to its raw host and verified replay evidence",
  );
  const netLogAnalysis = parseBoundNetLogText(files.get("native-netlog.json").toString("utf8"));
  const processEvidence = JSON.parse(files.get("native-process-endpoints.json").toString("utf8"));
  const processAnalysis = analyzeProcessEvidence(processEvidence, observer.browserExecutableSha256);
  const externalAttempts = [
    ...netLogAnalysis.externalTargets.map((target) => ({ channel: "chromium-netlog", ...target })),
    ...processAnalysis.externalEndpoints.map((target) => ({ channel: "windows-process-endpoint", ...target })),
  ];
  const unknownTargets = netLogAnalysis.unknownTargets;
  const instrumentationComplete =
    netLogAnalysis.relevantEventCount > 0 && netLogAnalysis.actionableEventCount > 0 &&
    processEvidence.captureComplete === true && processEvidence.snapshotCount > 0 &&
    observer.runtimeBackingAttested === true;
  const zeroExternalAttempts =
    instrumentationComplete && externalAttempts.length === 0 && unknownTargets.length === 0;
  const networkAnalysis = {
    schemaVersion: "1.0",
    evidenceKind: "WINDOWS_NATIVE_BOUND_NETWORK_ANALYSIS",
    candidateCommit: observer.candidateCommit,
    candidateTree: observer.candidateTree,
    candidateManifestSha256: observer.candidateManifestSha256,
    observerManifestSha256: sha256(observerBytes),
    chromiumNetLogSha256: sha256(files.get("native-netlog.json")),
    processEndpointEvidenceSha256: sha256(files.get("native-process-endpoints.json")),
    runtimeReplaySha256: replay.runtimeReplaySha256,
    canonicalReplaySha256: replay.canonicalReplaySha256,
    controlledInputSha256: replay.controlledInputSha256,
    deterministicOutputSha256: replay.deterministicOutputSha256,
    verifiedReplayEventCount: replay.verifiedReplayEventCount,
    verifiedReplayBoundaryCount: replay.verifiedReplayBoundaryCount,
    analyzerSourceSha256: sha256(scriptBytes),
    isolationAnalysisLibrarySha256: sha256(libraryBytes),
    instrumentationComplete,
    instrumentationStatus: instrumentationComplete
      ? "COMPLETE_BOUND_NETLOG_AND_PROCESS_ENDPOINT_ANALYSIS"
      : "INCOMPLETE_REQUIRES_EXTERNAL_HARNESS",
    netLog: netLogAnalysis,
    processEndpoints: processAnalysis,
    externalAttemptCount: externalAttempts.length,
    externalAttempts,
    unknownTargetCount: unknownTargets.length,
    zeroExternalAttempts,
    result: zeroExternalAttempts ? "PASS" : "FAIL",
  };
  await writeFile(networkAnalysisPath, stableJson(networkAnalysis), { encoding: "utf8", flag: "wx" });
  const evidenceNames = [
    "candidate-package-manifest.json",
    "native-launcher-transcript.log",
    "native-netlog.json",
    "native-network-analysis.json",
    "native-platform-observer-manifest.json",
    "native-process-endpoints.json",
    "native-run-raw.json",
  ];
  const evidenceFiles = [];
  for (const name of evidenceNames) {
    const bytes = name === "native-platform-observer-manifest.json"
      ? observerBytes
      : name === "native-network-analysis.json"
        ? await readBoundedRegularFile(networkAnalysisPath, 32 * 1024 * 1024)
        : files.get(name);
    requireCondition(Buffer.isBuffer(bytes), `Final evidence input is missing: ${name}`);
    evidenceFiles.push({ bytes: bytes.byteLength, path: name, sha256: sha256(bytes) });
  }
  const result = observer.candidateDevelopmentDirty
    ? "INCONCLUSIVE_DEVELOPMENT"
    : zeroExternalAttempts ? "PASS" : "FAIL";
  const finalManifest = {
    ...observer,
    evidenceKind: "WINDOWS_NATIVE_PRODUCT_PATH_MANIFEST",
    result,
    observerManifestSha256: sha256(observerBytes),
    networkAnalysisSha256: evidenceFiles.find(({ path: name }) =>
      name === "native-network-analysis.json").sha256,
    instrumentationStatus: networkAnalysis.instrumentationStatus,
    instrumentationComplete,
    netLogRelevantEventCount: netLogAnalysis.relevantEventCount,
    netLogExternalTargetCount: netLogAnalysis.externalTargets.length,
    netLogUnknownTargetCount: unknownTargets.length,
    runtimeReplaySha256: replay.runtimeReplaySha256,
    canonicalReplaySha256: replay.canonicalReplaySha256,
    controlledInputSha256: replay.controlledInputSha256,
    deterministicOutputSha256: replay.deterministicOutputSha256,
    verifiedReplayEventCount: replay.verifiedReplayEventCount,
    verifiedReplayBoundaryCount: replay.verifiedReplayBoundaryCount,
    externalAttemptCount: externalAttempts.length,
    zeroExternalAttempts,
    evidenceFiles,
  };
  await writeFile(finalPath, stableJson(finalManifest), { encoding: "utf8", flag: "wx" });
  console.log(stableJson({
    evidenceManifestSha256: sha256(await readFile(finalPath)),
    externalAttemptCount: externalAttempts.length,
    instrumentationComplete,
    result,
  }).trim());
  if (result !== "PASS") process.exitCode = result === "INCONCLUSIVE_DEVELOPMENT" ? 2 : 1;
}

const invokedDirectly = process.argv[1] !== undefined &&
  path.resolve(process.argv[1]) === path.resolve(scriptPath);
if (invokedDirectly) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
