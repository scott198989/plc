import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { isIP } from "node:net";

export const EVIDENCE_SCHEMA_VERSION = "2.0";
export const ISOLATION_EVIDENCE_RECORD_SCHEMA_VERSION = "2.0";
export const ISOLATION_APPROVAL_DECISION_ID = "P2-DEC-ISO-NATIVE-001";
export const ISOLATION_APPROVAL_PATH = "ADR/0005-phase-2-native-isolation-shell.md";
export const ISOLATION_VERIFICATION_IDS = Object.freeze([
  "VER-ISO-0001",
  "VER-ISO-0002",
  "VER-ISO-0003",
  "VER-ISO-0004",
  "VER-ISO-0005",
  "VER-NET-0001",
]);

export const EXPECTED_DIRECTIVE_SHA256 =
  "938A0958F0CF15739A2DC8ED674F7C9F25D531DCE32CCA6A4CEEE5D638E68536";

export const DEFAULT_FUZZ_CASES = Object.freeze(parseFuzzCorpus(
  readFileSync(new URL("./isolation-fuzz-corpus.tsv", import.meta.url), "utf8"),
));

export const REQUIRED_FUZZ_BOUNDARY_IDS = Object.freeze([
  "file-metadata-open",
  "file-metadata-create",
  "file-metadata-replace",
  "project-display-name",
  "saved-project-decode",
  "scl-source-text",
  "semantic-navigation",
  "trace-export-canonical-json",
  "trace-export-csv",
  "virtual-download-target",
]);

export const REQUIRED_EXPORT_SURFACE_IDS = Object.freeze([
  "project-native-save",
  "replay-verification-package",
  "trace-canonical-json",
  "trace-csv",
]);

export const REQUIRED_NATIVE_BROKER_OPERATION_IDS = Object.freeze([
  "open",
  "create",
  "replace",
]);

export const SUPPORTED_WINDOWS_CONFIGURATION_IDS = Object.freeze([
  "windows-x64-chromium-native-broker-adapters-on",
  "windows-x64-chromium-packaged-adapters-off",
]);

export const SUPPORTED_CHROMIUM_RUNTIME_PRODUCTS = Object.freeze([
  "google-chrome",
  "microsoft-edge",
  "microsoft-edge-webview2",
]);

export const ISOLATION_CLOSURE_REQUIRED_FIELDS = Object.freeze([
  "boundaryFuzzCoverage",
  "candidateCommit",
  "candidateTree",
  "configurationCoverage",
  "evidenceKind",
  "fixedNativeBackingAttestation",
  "liveLanTopologyVariation",
  "schemaVersion",
  "vendorDeployableExportRejection",
]);

const LOOPBACK_HOSTS = new Set(["localhost", "localhost.localdomain", "127.0.0.1", "::1", "0:0:0:0:0:0:0:1"]);
const UNSPECIFIED_HOSTS = new Set(["", "0.0.0.0", "::", "0:0:0:0:0:0:0:0", "*"]);
const SHA256_PATTERN = /^[A-F0-9]{64}$/u;
const PASS_ENVELOPE_FIELDS = ["complete", "result", "schemaVersion"];
const BOUNDARY_COVERAGE_FIELDS = [
  ...PASS_ENVELOPE_FIELDS, "boundaries", "caseCount", "caseIdsSha256", "corpusSha256",
];
const BOUNDARY_ROW_FIELDS = [
  "boundaryId", "caseCount", "caseIdsSha256", "corpusSha256", "externalAttemptCount",
  "productionPathExercised", "result", "sideEffectsObserved",
];
const CONFIGURATION_COVERAGE_FIELDS = [
  "approvalDecisionId", "approvalSha256", "approvalStatus", "evidenceBindings",
  "expectedConfigurationIds", "status",
];
const CONFIGURATION_BINDING_FIELDS = [
  "architecture", "browserExecutableSha256", "browserFamily", "browserRuntimeProduct",
  "browserRuntimeVersion", "candidateCommit", "candidateTree", "completeLogs", "configurationId",
  "evidenceManifestSha256", "fileAccessPosture", "hostNetworkPosture", "matchesCandidate",
  "platform", "productionPathExercised", "result", "zeroExternalAttempts",
];
const NATIVE_BACKING_FIELDS = [
  ...PASS_ENVELOPE_FIELDS, "architecture", "candidateCommit", "candidateTree", "decisionId",
  "evidenceManifestSha256", "operations", "platform",
];
const NATIVE_OPERATION_FIELDS = [
  "attestationVersion", "fixedLocalBacking", "metadataOnlyBeforeAcceptance", "operationId",
  "productionPathExercised", "providerBacked", "redirected", "remote", "removable", "result",
  "selectedByteIoBeforeAcceptance", "special", "unapprovedHelperEffectObserved", "unsafeTarget",
];
const TOPOLOGY_VARIATION_FIELDS = [
  ...PASS_ENVELOPE_FIELDS, "applicationNetworkCapabilityPresent", "discoveryApiSurfacePresent",
  "scenarios",
];
const LAN_SCENARIO_FIELDS = [
  "architecture", "candidateCommit", "candidateTree", "completeLogs", "configurationId",
  "controlledInputSha256", "deterministicOutputSha256", "evidenceManifestSha256",
  "externalAttemptCount", "platform", "postTopologyFingerprint", "preTopologyFingerprint",
  "productionPathExercised", "result", "scenarioId", "topologyFingerprint",
  "topologyMutationControl", "topologySource",
];
const EXPORT_REJECTION_FIELDS = [...PASS_ENVELOPE_FIELDS, "surfaces"];
const EXPORT_SURFACE_FIELDS = [
  "closedFormatSet", "deployableArtifactAttemptsRejected", "productionPathExercised", "result",
  "sideEffectsObserved", "surfaceId", "vendorArtifactAttemptsRejected",
];

export const sha256 = (value) =>
  createHash("sha256").update(value).digest("hex").toUpperCase();

export const FUZZ_CORPUS_SHA256 = sha256(stableJson(DEFAULT_FUZZ_CASES));
export const FUZZ_CASE_IDS_SHA256 = sha256(stableJson(DEFAULT_FUZZ_CASES.map(({ id }) => id)));

export function stableJson(value) {
  return JSON.stringify(sortJson(value), null, 2).concat("\n");
}

function sortJson(value) {
  if (Array.isArray(value)) {
    return value.map(sortJson);
  }
  if (value !== null && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value)
        .sort(([left], [right]) => left.localeCompare(right, "en-US"))
        .map(([key, item]) => [key, sortJson(item)]),
    );
  }
  return value;
}

function hasExactOwnFields(value, expectedFields) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    return false;
  }
  const observed = Object.keys(value).sort((left, right) => left.localeCompare(right, "en-US"));
  const expected = [...expectedFields].sort((left, right) => left.localeCompare(right, "en-US"));
  return observed.length === expected.length && observed.every((field, index) => field === expected[index]);
}

function parseFuzzCorpus(source) {
  const cases = source
    .split(/\r?\n/u)
    .filter((line) => line.length > 0)
    .map((line) => {
      const [category, id, encodedValue, ...extra] = line.split("\t");
      if (
        extra.length !== 0 ||
        !/^[a-z][a-z0-9-]*$/u.test(category ?? "") ||
        !/^[a-z0-9][a-z0-9-]*$/u.test(id ?? "")
      ) {
        throw new Error(`Malformed isolation fuzz corpus row: ${line}`);
      }
      let value;
      try {
        value = JSON.parse(encodedValue);
      } catch {
        throw new Error(`Malformed isolation fuzz JSON value for ${id}`);
      }
      if (typeof value !== "string") {
        throw new Error(`Isolation fuzz value for ${id} is not a string`);
      }
      return Object.freeze({ category, id, value });
    });
  if (cases.length !== 27 || new Set(cases.map(({ id }) => id)).size !== cases.length) {
    throw new Error("Isolation fuzz corpus must contain exactly 27 unique case IDs");
  }
  return cases;
}

export function normalizeHost(value) {
  let host = String(value ?? "").trim().toLocaleLowerCase("en-US");
  if (host.startsWith("[") && host.endsWith("]")) {
    host = host.slice(1, -1);
  }
  const percent = host.indexOf("%");
  if (percent >= 0) {
    host = host.slice(0, percent);
  }
  return host;
}

export function isLoopbackHost(value) {
  const host = normalizeHost(value);
  if (LOOPBACK_HOSTS.has(host)) {
    return true;
  }
  if (isIP(host) === 4) {
    return host.split(".")[0] === "127";
  }
  return false;
}

export function isUnspecifiedHost(value) {
  return UNSPECIFIED_HOSTS.has(normalizeHost(value));
}

export function splitEndpoint(value) {
  const text = String(value ?? "").trim();
  if (text.length === 0) {
    return { host: "", port: null, raw: text };
  }
  if (text.startsWith("[")) {
    const closing = text.indexOf("]");
    if (closing >= 0) {
      const portText = text.slice(closing + 1).replace(/^:/u, "");
      return {
        host: text.slice(1, closing),
        port: /^\d+$/u.test(portText) ? Number(portText) : null,
        raw: text,
      };
    }
  }
  const lastColon = text.lastIndexOf(":");
  if (lastColon > 0 && text.indexOf(":") === lastColon) {
    const portText = text.slice(lastColon + 1);
    if (/^\d+$/u.test(portText)) {
      return { host: text.slice(0, lastColon), port: Number(portText), raw: text };
    }
  }
  return { host: text, port: null, raw: text };
}

export function classifyUrl(value, allowedOrigins = new Set()) {
  const text = String(value ?? "");
  if (/^(?:blob|data|about):/iu.test(text)) {
    return { allowed: true, category: "inert-or-internal", target: text };
  }
  try {
    const parsed = new URL(text);
    const allowed = allowedOrigins.has(parsed.origin);
    return {
      allowed,
      category: allowed ? "allowlisted-loopback-origin" : "external-or-unapproved-origin",
      host: parsed.hostname,
      origin: parsed.origin,
      protocol: parsed.protocol,
      target: parsed.href,
    };
  } catch {
    return { allowed: false, category: "malformed-target", target: text };
  }
}

export function deriveProcessTree(processes, rootPid) {
  const normalized = processes
    .map((process) => ({
      executablePath: process.executablePath ?? process.ExecutablePath ?? null,
      name: process.name ?? process.Name ?? null,
      parentPid: Number(process.parentPid ?? process.ParentProcessId),
      pid: Number(process.pid ?? process.ProcessId),
    }))
    .filter((process) => Number.isSafeInteger(process.pid) && process.pid > 0);
  const selected = new Set([Number(rootPid)]);
  let changed = true;
  while (changed) {
    changed = false;
    for (const process of normalized) {
      if (!selected.has(process.pid) && selected.has(process.parentPid)) {
        selected.add(process.pid);
        changed = true;
      }
    }
  }
  return normalized.filter((process) => selected.has(process.pid)).sort((left, right) => left.pid - right.pid);
}

export function analyzeProcessEndpoints(samples, allowlistedRemoteEndpoints = new Set()) {
  const observations = [];
  const externalAttempts = [];
  const loopbackAccounted = [];
  for (const sample of samples) {
    for (const endpoint of sample.endpoints ?? []) {
      const protocol = String(endpoint.protocol ?? endpoint.Protocol ?? "").toLocaleUpperCase("en-US");
      const state = String(endpoint.state ?? endpoint.State ?? "").toLocaleUpperCase("en-US");
      const remoteAddress = String(endpoint.remoteAddress ?? endpoint.RemoteAddress ?? "");
      const remotePort = Number(endpoint.remotePort ?? endpoint.RemotePort ?? 0);
      const localAddress = String(endpoint.localAddress ?? endpoint.LocalAddress ?? "");
      const localPort = Number(endpoint.localPort ?? endpoint.LocalPort ?? 0);
      const normalized = {
        localAddress,
        localPort,
        owningProcess: Number(endpoint.owningProcess ?? endpoint.OwningProcess ?? 0),
        protocol,
        remoteAddress,
        remotePort,
        state,
      };
      observations.push(normalized);
      if (protocol === "TCP" && ["LISTEN", "LISTENING", "BOUND"].includes(state)) {
        if (!isLoopbackHost(localAddress) && !isUnspecifiedHost(localAddress)) {
          externalAttempts.push({ ...normalized, reason: "non-loopback-listener" });
        }
        continue;
      }
      if (protocol === "UDP") {
        if (isLoopbackHost(localAddress)) {
          loopbackAccounted.push({ ...normalized, reason: "loopback-udp-endpoint" });
        } else {
          externalAttempts.push({ ...normalized, reason: "udp-endpoint-opened" });
        }
        continue;
      }
      const remoteKey = `${normalizeHost(remoteAddress)}:${remotePort}`;
      if (allowlistedRemoteEndpoints.has(remoteKey)) {
        loopbackAccounted.push({ ...normalized, reason: "allowlisted-loopback-remote" });
      } else if (isLoopbackHost(remoteAddress)) {
        externalAttempts.push({ ...normalized, reason: "unaccounted-loopback-remote" });
      } else if (!isUnspecifiedHost(remoteAddress)) {
        externalAttempts.push({ ...normalized, reason: "external-remote" });
      }
    }
  }
  return {
    externalAttempts: deduplicate(externalAttempts),
    loopbackAccounted: deduplicate(loopbackAccounted),
    observations: deduplicate(observations),
  };
}

export function analyzeHostNetworkAdapters(snapshots) {
  const normalizedSnapshots = [];
  const activeAdapters = [];
  for (const snapshot of Array.isArray(snapshots) ? snapshots : []) {
    if (!Array.isArray(snapshot?.adapters)) {
      continue;
    }
    const adapters = snapshot.adapters.map((adapter) => {
      const normalized = {
        interfaceDescription: String(adapter.interfaceDescription ?? adapter.InterfaceDescription ?? ""),
        interfaceIndex: Number(adapter.interfaceIndex ?? adapter.ifIndex ?? adapter.InterfaceIndex ?? 0),
        mediaConnectionState: String(adapter.mediaConnectionState ?? adapter.MediaConnectionState ?? ""),
        name: String(adapter.name ?? adapter.Name ?? ""),
        status: String(adapter.status ?? adapter.Status ?? ""),
      };
      const status = normalized.status.toLocaleLowerCase("en-US");
      const mediaState = normalized.mediaConnectionState.toLocaleLowerCase("en-US");
      if (status === "up" || mediaState === "connected" || mediaState === "1") {
        activeAdapters.push({ boundary: snapshot.boundary ?? null, capturedAt: snapshot.capturedAt ?? null, ...normalized });
      }
      return normalized;
    });
    normalizedSnapshots.push({
      adapters,
      boundary: snapshot.boundary ?? null,
      capturedAt: snapshot.capturedAt ?? null,
    });
  }
  const boundaries = new Set(normalizedSnapshots.map(({ boundary }) => String(boundary ?? "")));
  const captureComplete =
    boundaries.has("preflight") &&
    [...boundaries].some((boundary) => boundary.startsWith("postflight"));
  return {
    activeAdapters: deduplicate(activeAdapters),
    adaptersDisabled: captureComplete && activeAdapters.length === 0,
    captureComplete,
    snapshots: normalizedSnapshots,
  };
}

export function analyzeCapabilityEvents(events, allowedOrigins = new Set()) {
  const externalAttempts = [];
  const accountedInternal = [];
  for (const event of events) {
    if (event.api === "Worker" && event.classification === "allowed-internal-blob-worker") {
      accountedInternal.push(event);
      continue;
    }
    if (event.classification === "observed-safe-metadata") {
      accountedInternal.push(event);
      continue;
    }
    if (typeof event.target === "string") {
      const classification = classifyUrl(event.target, allowedOrigins);
      if (classification.allowed && event.outcome !== "denied") {
        accountedInternal.push({ ...event, targetClassification: classification.category });
        continue;
      }
    }
    externalAttempts.push(event);
  }
  return {
    accountedInternal: deduplicate(accountedInternal),
    externalAttempts: deduplicate(externalAttempts),
  };
}

export function scanPackagedHtml(html) {
  const findings = [];
  const requiredCsp = [
    "default-src 'none'",
    "base-uri 'none'",
    "connect-src 'none'",
    "form-action 'none'",
    "object-src 'none'",
    "worker-src blob:",
  ];
  for (const directive of requiredCsp) {
    if (!html.includes(directive)) {
      findings.push({ code: "ISO-PKG-CSP", message: `Missing CSP directive: ${directive}` });
    }
  }
  const forbidden = [
    ["ISO-PKG-EXTERNAL-ASSET", /<(?:script|link|img|iframe|object|embed|source)[^>]+(?:src|href)\s*=\s*["'][^"']+["']/iu],
    ["ISO-PKG-ACTIVE-ELEMENT", /<(?:iframe|object|embed|form)\b/iu],
    ["ISO-PKG-NETWORK-API", /\b(?:XMLHttpRequest|WebSocket|EventSource|WebTransport|RTCPeerConnection)\b/u],
    ["ISO-PKG-DEVICE-API", /navigator\.(?:serial|usb|bluetooth|hid|nfc|midi|mediaDevices|serviceWorker)\b/u],
    ["ISO-PKG-DYNAMIC-IMPORT", /\bimport\s*\(/u],
    ["ISO-PKG-DYNAMIC-EXECUTION", /\beval\s*\(|\bnew\s+Function\s*\(/u],
    ["ISO-PKG-UPDATER", /\b(?:autoUpdater|checkForUpdates|update-electron-app)\b/u],
    ["ISO-PKG-LOCAL-SERVER", /\b(?:createServer|listen)\s*\(/u],
  ];
  for (const [code, pattern] of forbidden) {
    if (pattern.test(html)) {
      findings.push({ code, message: `Packaged HTML matched forbidden pattern ${pattern.source}` });
    }
  }
  const wasmImports = [];
  const wasmModules = [];
  for (const match of html.matchAll(/["'`]([A-Za-z0-9+/]{8,}={0,2})["'`]/gu)) {
    try {
      const bytes = Buffer.from(match[1], "base64");
      if (
        bytes.byteLength < 8 ||
        bytes[0] !== 0 ||
        bytes[1] !== 0x61 ||
        bytes[2] !== 0x73 ||
        bytes[3] !== 0x6d
      ) {
        continue;
      }
      const module = new WebAssembly.Module(bytes);
      const imports = WebAssembly.Module.imports(module);
      wasmImports.push(...imports);
      wasmModules.push({
        bytes: bytes.byteLength,
        exports: WebAssembly.Module.exports(module).map(({ name }) => name).sort(),
        imports,
        sha256: sha256(bytes),
      });
    } catch {
      // A base64-shaped non-WASM string is inert for this check.
    }
  }
  const distinctWasmModules = new Set(wasmModules.map(({ sha256: digest }) => digest)).size;
  if (wasmModules.length < 2 || distinctWasmModules < 2) {
    findings.push({ code: "ISO-PKG-WASM-MISSING", message: `Expected two distinct embedded WASM modules; found ${wasmModules.length} occurrences and ${distinctWasmModules} distinct modules` });
  }
  if (wasmImports.length > 0) {
    findings.push({ code: "ISO-PKG-WASM-IMPORT", message: `Packaged WASM imports ${JSON.stringify(wasmImports)}` });
  }
  return {
    cspDirectives: requiredCsp.map((directive) => ({ directive, present: html.includes(directive) })),
    findings,
    pass: findings.length === 0,
    wasmImports,
    wasmModules,
  };
}

export function parseGitStatusPorcelainZ(output) {
  const changes = [];
  const entries = String(output ?? "").split("\u0000").filter(Boolean);
  for (let index = 0; index < entries.length; index += 1) {
    const entry = entries[index];
    if (entry.length < 4) {
      continue;
    }
    const state = entry.slice(0, 2);
    const path = entry.slice(3).replaceAll("\\", "/");
    changes.push({ path, state });
    if (state.includes("R") || state.includes("C")) {
      index += 1;
    }
  }
  return changes;
}

export function parseChromiumNetLog(netLog) {
  const typeNames = reverseNumericMap(netLog?.constants?.logEventTypes ?? {});
  const sourceNames = reverseNumericMap(netLog?.constants?.logSourceType ?? {});
  const relevant = [];
  const targetStrings = [];
  for (const event of netLog?.events ?? []) {
    const typeName = typeNames.get(event.type) ?? String(event.type);
    const sourceName = sourceNames.get(event.source?.type) ?? String(event.source?.type ?? "");
    if (!/(?:DNS|HOST_RESOLVER|URL_REQUEST|SOCKET|TCP|UDP|QUIC|WEBSOCKET|WEBTRANSPORT|HTTP_STREAM|PROXY|CONNECT)/iu.test(typeName)) {
      continue;
    }
    const strings = collectStrings(event.params ?? {});
    for (const value of strings) {
      if (looksLikeTarget(value)) {
        targetStrings.push({ sourceName, typeName, value });
      }
    }
    relevant.push({ phase: event.phase, sourceId: event.source?.id ?? null, sourceName, typeName });
  }
  return { relevantEventCount: relevant.length, relevantEvents: relevant, targetStrings: deduplicate(targetStrings) };
}

export function analyzeNetLogTargets(parsed, allowedOrigins = new Set(), allowedEndpoints = new Set()) {
  const externalAttempts = [];
  const loopbackAccounted = [];
  const configurationObservations = [];
  const syntheticBlockedTargets = [];
  for (const record of parsed.targetStrings ?? []) {
    if (/(?:DNS_CONFIG_CHANGED|NETWORK_CHANGED)/iu.test(record.typeName)) {
      configurationObservations.push(record);
      continue;
    }
    const targets = extractTargets(record.value);
    for (const value of targets.length > 0 ? targets : [record.value]) {
      if (/^[a-z][a-z0-9+.-]*:\/\//iu.test(value)) {
        const classification = classifyUrl(value, allowedOrigins);
        if (classification.host === "~notfound") {
          syntheticBlockedTargets.push({ ...record, classification: "host-resolver-rule-synthetic-block", value });
        } else {
          (classification.allowed ? loopbackAccounted : externalAttempts).push({ ...record, classification, value });
        }
        continue;
      }
      const endpoint = splitEndpoint(value);
      const endpointKey = `${normalizeHost(endpoint.host)}:${endpoint.port ?? 0}`;
      if (allowedEndpoints.has(endpointKey)) {
        loopbackAccounted.push({ ...record, classification: "allowlisted-loopback-endpoint", value });
      } else if (isLoopbackHost(endpoint.host)) {
        externalAttempts.push({ ...record, classification: "unaccounted-loopback-endpoint", value });
      } else if (normalizeHost(endpoint.host) === "~notfound") {
        syntheticBlockedTargets.push({ ...record, classification: "host-resolver-rule-synthetic-block", value });
      } else if (!isUnspecifiedHost(endpoint.host)) {
        externalAttempts.push({ ...record, classification: "external-or-name-resolution-target", value });
      }
    }
  }
  return {
    configurationObservations: deduplicate(configurationObservations),
    externalAttempts: deduplicate(externalAttempts),
    loopbackAccounted: deduplicate(loopbackAccounted),
    syntheticBlockedTargets: deduplicate(syntheticBlockedTargets),
  };
}

export function assessBoundaryFuzzCoverage(coverage) {
  const failures = [];
  if (!hasExactOwnFields(coverage, BOUNDARY_COVERAGE_FIELDS)) {
    failures.push("Boundary fuzz coverage contains missing or unrecognized fields");
  }
  if (coverage?.schemaVersion !== "1.0" || coverage?.result !== "PASS" || coverage?.complete !== true) {
    failures.push("Boundary fuzz coverage is not a complete PASS under schema 1.0");
  }
  if (
    coverage?.corpusSha256 !== FUZZ_CORPUS_SHA256 ||
    coverage?.caseIdsSha256 !== FUZZ_CASE_IDS_SHA256 ||
    coverage?.caseCount !== DEFAULT_FUZZ_CASES.length
  ) {
    failures.push("Boundary fuzz coverage is not bound to the complete issued corpus");
  }
  const boundaries = Array.isArray(coverage?.boundaries) ? coverage.boundaries : [];
  const boundaryIds = boundaries.map(({ boundaryId }) => String(boundaryId ?? ""));
  if (new Set(boundaryIds).size !== boundaryIds.length) {
    failures.push("Boundary fuzz coverage contains duplicate boundary identities");
  }
  const expected = new Set(REQUIRED_FUZZ_BOUNDARY_IDS);
  const observed = new Set(boundaryIds);
  if (
    expected.size !== observed.size ||
    [...expected].some((boundaryId) => !observed.has(boundaryId))
  ) {
    failures.push("Boundary fuzz coverage does not exactly enumerate every applicable typed boundary");
  }
  for (const boundary of boundaries) {
    const boundaryId = String(boundary?.boundaryId ?? "<missing>");
    if (
      !hasExactOwnFields(boundary, BOUNDARY_ROW_FIELDS) ||
      boundary?.caseCount !== DEFAULT_FUZZ_CASES.length ||
      boundary?.caseIdsSha256 !== FUZZ_CASE_IDS_SHA256 ||
      boundary?.corpusSha256 !== FUZZ_CORPUS_SHA256 ||
      boundary?.externalAttemptCount !== 0 ||
      boundary?.productionPathExercised !== true ||
      boundary?.result !== "PASS" ||
      boundary?.sideEffectsObserved !== false
    ) {
      failures.push(`Boundary ${boundaryId} lacks complete inert exact-corpus PASS evidence`);
    }
  }
  return { complete: failures.length === 0, failures };
}

export function assessLiveLanTopologyVariation(evidence, candidate) {
  const failures = [];
  if (!hasExactOwnFields(evidence, TOPOLOGY_VARIATION_FIELDS)) {
    failures.push("Live-LAN variation contains missing or unrecognized fields");
  }
  if (
    evidence?.schemaVersion !== "1.0" ||
    evidence?.result !== "PASS" ||
    evidence?.complete !== true ||
    evidence?.applicationNetworkCapabilityPresent !== false ||
    evidence?.discoveryApiSurfacePresent !== false
  ) {
    failures.push("Live-LAN variation is not a complete capability-free PASS under schema 1.0");
  }
  const scenarios = Array.isArray(evidence?.scenarios) ? evidence.scenarios : [];
  if (scenarios.length < 2) {
    failures.push("Live-LAN variation requires at least two controlled topology scenarios");
  }
  const scenarioIds = scenarios.map(({ scenarioId }) => String(scenarioId ?? ""));
  const topologyFingerprints = scenarios.map(({ topologyFingerprint }) => String(topologyFingerprint ?? ""));
  if (new Set(scenarioIds).size !== scenarios.length || scenarioIds.some((value) => value.length === 0)) {
    failures.push("Live-LAN scenarios have missing or duplicate identities");
  }
  if (
    new Set(topologyFingerprints).size < 2 ||
    topologyFingerprints.some((digest) => !SHA256_PATTERN.test(digest))
  ) {
    failures.push("Live-LAN scenarios do not prove at least two distinct topology fingerprints");
  }
  const inputHashes = new Set();
  const outputHashes = new Set();
  for (const scenario of scenarios) {
    const scenarioId = String(scenario?.scenarioId ?? "<missing>");
    inputHashes.add(String(scenario?.controlledInputSha256 ?? ""));
    outputHashes.add(String(scenario?.deterministicOutputSha256 ?? ""));
    if (
      !hasExactOwnFields(scenario, LAN_SCENARIO_FIELDS) ||
      scenario?.candidateCommit !== candidate?.commit ||
      scenario?.candidateTree !== candidate?.tree ||
      scenario?.configurationId !== SUPPORTED_WINDOWS_CONFIGURATION_IDS[0] ||
      scenario?.platform !== "windows" ||
      scenario?.architecture !== "x64" ||
      scenario?.topologySource !== "WINDOWS_LIVE_ADAPTER_SNAPSHOT" ||
      scenario?.topologyMutationControl !== "EXTERNAL_LAB_OR_OPERATOR_CONTROLLED" ||
      scenario?.completeLogs !== true ||
      scenario?.externalAttemptCount !== 0 ||
      scenario?.productionPathExercised !== true ||
      scenario?.result !== "PASS" ||
      scenario?.preTopologyFingerprint !== scenario?.topologyFingerprint ||
      scenario?.postTopologyFingerprint !== scenario?.topologyFingerprint ||
      !SHA256_PATTERN.test(String(scenario?.evidenceManifestSha256 ?? "")) ||
      !SHA256_PATTERN.test(String(scenario?.preTopologyFingerprint ?? "")) ||
      !SHA256_PATTERN.test(String(scenario?.postTopologyFingerprint ?? "")) ||
      !SHA256_PATTERN.test(String(scenario?.controlledInputSha256 ?? "")) ||
      !SHA256_PATTERN.test(String(scenario?.deterministicOutputSha256 ?? ""))
    ) {
      failures.push(`Live-LAN scenario ${scenarioId} lacks complete exact-candidate PASS evidence`);
    }
  }
  if (inputHashes.size !== 1 || outputHashes.size !== 1) {
    failures.push("Live-LAN topology variation changed the controlled input or deterministic product output");
  }
  return { complete: failures.length === 0, failures };
}

export function assessVendorDeployableExportRejection(evidence) {
  const failures = [];
  if (!hasExactOwnFields(evidence, EXPORT_REJECTION_FIELDS)) {
    failures.push("Export rejection evidence contains missing or unrecognized fields");
  }
  if (evidence?.schemaVersion !== "1.0" || evidence?.result !== "PASS" || evidence?.complete !== true) {
    failures.push("Export rejection evidence is not a complete PASS under schema 1.0");
  }
  const surfaces = Array.isArray(evidence?.surfaces) ? evidence.surfaces : [];
  const surfaceIds = surfaces.map(({ surfaceId }) => String(surfaceId ?? ""));
  if (new Set(surfaceIds).size !== surfaceIds.length) {
    failures.push("Export rejection evidence contains duplicate surface identities");
  }
  const expected = new Set(REQUIRED_EXPORT_SURFACE_IDS);
  const observed = new Set(surfaceIds);
  if (
    expected.size !== observed.size ||
    [...expected].some((surfaceId) => !observed.has(surfaceId))
  ) {
    failures.push("Export rejection evidence does not exactly enumerate every Phase 2 export surface");
  }
  for (const surface of surfaces) {
    const surfaceId = String(surface?.surfaceId ?? "<missing>");
    if (
      !hasExactOwnFields(surface, EXPORT_SURFACE_FIELDS) ||
      surface?.closedFormatSet !== true ||
      surface?.deployableArtifactAttemptsRejected !== true ||
      surface?.vendorArtifactAttemptsRejected !== true ||
      surface?.productionPathExercised !== true ||
      surface?.result !== "PASS" ||
      surface?.sideEffectsObserved !== false
    ) {
      failures.push(`Export surface ${surfaceId} lacks vendor/deployable fail-closed proof`);
    }
  }
  return { complete: failures.length === 0, failures };
}

export function assessFixedNativeBackingAttestation(evidence, candidate) {
  const failures = [];
  if (!hasExactOwnFields(evidence, NATIVE_BACKING_FIELDS)) {
    failures.push("Fixed-native-backing evidence contains missing or unrecognized fields");
  }
  if (
    evidence?.schemaVersion !== "1.0" ||
    evidence?.result !== "PASS" ||
    evidence?.complete !== true ||
    evidence?.decisionId !== ISOLATION_APPROVAL_DECISION_ID ||
    evidence?.candidateCommit !== candidate?.commit ||
    evidence?.candidateTree !== candidate?.tree ||
    evidence?.platform !== "windows" ||
    evidence?.architecture !== "x64" ||
    !SHA256_PATTERN.test(String(evidence?.evidenceManifestSha256 ?? ""))
  ) {
    failures.push("Fixed-native-backing evidence is not a complete exact-candidate Windows PASS");
  }
  const operations = Array.isArray(evidence?.operations) ? evidence.operations : [];
  const operationIds = operations.map(({ operationId }) => String(operationId ?? ""));
  const expected = new Set(REQUIRED_NATIVE_BROKER_OPERATION_IDS);
  const observed = new Set(operationIds);
  if (
    operationIds.length !== observed.size ||
    expected.size !== observed.size ||
    [...expected].some((operationId) => !observed.has(operationId))
  ) {
    failures.push("Fixed-native-backing evidence does not exactly enumerate open/create/replace");
  }
  for (const operation of operations) {
    const operationId = String(operation?.operationId ?? "<missing>");
    if (
      !hasExactOwnFields(operation, NATIVE_OPERATION_FIELDS) ||
      operation?.attestationVersion !== 1 ||
      operation?.fixedLocalBacking !== true ||
      operation?.providerBacked !== false ||
      operation?.remote !== false ||
      operation?.removable !== false ||
      operation?.special !== false ||
      operation?.redirected !== false ||
      operation?.unsafeTarget !== false ||
      operation?.metadataOnlyBeforeAcceptance !== true ||
      operation?.selectedByteIoBeforeAcceptance !== false ||
      operation?.unapprovedHelperEffectObserved !== false ||
      operation?.productionPathExercised !== true ||
      operation?.result !== "PASS"
    ) {
      failures.push(`Native broker operation ${operationId} lacks fail-closed backing attestation`);
    }
  }
  return { complete: failures.length === 0, failures };
}

export function assessConfigurationCoverage(coverage, candidate = undefined) {
  const failures = [];
  if (!hasExactOwnFields(coverage, CONFIGURATION_COVERAGE_FIELDS)) {
    failures.push("Configuration coverage contains missing or unrecognized fields");
  }
  const approvalDecisionId = String(coverage?.approvalDecisionId ?? "");
  if (
    approvalDecisionId !== ISOLATION_APPROVAL_DECISION_ID ||
    coverage?.approvalStatus !== "APPROVED" ||
    !SHA256_PATTERN.test(String(coverage?.approvalSha256 ?? ""))
  ) {
    failures.push("The supported platform/configuration set has no approved decision binding");
  }
  if (
    candidate !== undefined &&
    (
      approvalDecisionId !== candidate?.isolationApprovalDecisionId ||
      coverage?.approvalSha256 !== candidate?.isolationApprovalSha256
    )
  ) {
    failures.push("The supported configuration approval does not match the exact candidate ADR blob");
  }
  if (coverage?.status !== "COMPLETE") {
    failures.push("Supported configuration evidence is not marked COMPLETE");
  }
  const expected = Array.isArray(coverage?.expectedConfigurationIds)
    ? coverage.expectedConfigurationIds.map(String)
    : [];
  if (
    expected.length !== SUPPORTED_WINDOWS_CONFIGURATION_IDS.length ||
    new Set(expected).size !== expected.length ||
    SUPPORTED_WINDOWS_CONFIGURATION_IDS.some((configurationId) => !expected.includes(configurationId))
  ) {
    failures.push("The supported configuration set is not the exact approved Windows-first set");
  }
  const bindings = Array.isArray(coverage?.evidenceBindings) ? coverage.evidenceBindings : [];
  const bindingConfigurationIds = bindings.map((binding) => String(binding.configurationId ?? ""));
  if (new Set(bindingConfigurationIds).size !== bindingConfigurationIds.length) {
    failures.push("Configuration evidence bindings contain duplicate identities");
  }
  if (
    bindingConfigurationIds.length !== expected.length ||
    bindingConfigurationIds.some((configurationId) => !expected.includes(configurationId))
  ) {
    failures.push("Configuration evidence bindings do not exactly match the approved set");
  }
  const byConfiguration = new Map(bindings.map((binding) => [String(binding.configurationId ?? ""), binding]));
  for (const configurationId of expected) {
    const binding = byConfiguration.get(configurationId);
    if (
      binding === undefined ||
      !hasExactOwnFields(binding, CONFIGURATION_BINDING_FIELDS) ||
      binding.completeLogs !== true ||
      binding.matchesCandidate !== true ||
      binding.result !== "PASS" ||
      binding.platform !== "windows" ||
      binding.architecture !== "x64" ||
      binding.browserFamily !== "chromium" ||
      !SUPPORTED_CHROMIUM_RUNTIME_PRODUCTS.includes(binding.browserRuntimeProduct) ||
      !isBoundedRuntimeVersion(binding.browserRuntimeVersion) ||
      !SHA256_PATTERN.test(String(binding.browserExecutableSha256 ?? "")) ||
      binding.productionPathExercised !== true ||
      binding.zeroExternalAttempts !== true ||
      (candidate !== undefined && (
        binding.candidateCommit !== candidate?.commit ||
        binding.candidateTree !== candidate?.tree
      )) ||
      !SHA256_PATTERN.test(String(binding.evidenceManifestSha256 ?? ""))
    ) {
      failures.push(`Configuration ${configurationId} lacks complete exact-candidate PASS evidence`);
    }
  }
  const native = byConfiguration.get(SUPPORTED_WINDOWS_CONFIGURATION_IDS[0]);
  if (
    native?.fileAccessPosture !== "native-broker" ||
    native?.hostNetworkPosture !== "adapters-on-controlled-lan" ||
    native?.browserRuntimeProduct !== "microsoft-edge-webview2"
  ) {
    failures.push("Native-broker configuration posture is incomplete");
  }
  const packaged = byConfiguration.get(SUPPORTED_WINDOWS_CONFIGURATION_IDS[1]);
  if (
    packaged?.fileAccessPosture !== "packaged-browser-disabled" ||
    packaged?.hostNetworkPosture !== "adapters-off" ||
    !["google-chrome", "microsoft-edge"].includes(packaged?.browserRuntimeProduct)
  ) {
    failures.push("Packaged adapters-off configuration posture is incomplete");
  }
  return { complete: failures.length === 0, failures };
}

function isBoundedRuntimeVersion(value) {
  return typeof value === "string" && /^[A-Za-z0-9][A-Za-z0-9._+ -]{0,127}$/u.test(value);
}

export function assessIsolationClosureEvidence(value, candidate) {
  const failures = [];
  const isRecord = value !== null && typeof value === "object" && !Array.isArray(value);
  if (
    !isRecord ||
    value.schemaVersion !== "1.0" ||
    value.evidenceKind !== "PHASE2_ISOLATION_CLOSURE_INPUT" ||
    value.candidateCommit !== candidate?.commit ||
    value.candidateTree !== candidate?.tree
  ) {
    failures.push("Closure evidence is not an exact-candidate schema 1.0 isolation input");
  }
  const observedFields = isRecord ? Object.keys(value).sort((left, right) => left.localeCompare(right, "en-US")) : [];
  if (
    observedFields.length !== ISOLATION_CLOSURE_REQUIRED_FIELDS.length ||
    ISOLATION_CLOSURE_REQUIRED_FIELDS.some((field) => !observedFields.includes(field))
  ) {
    failures.push("Closure evidence fields do not exactly match the machine-readable contract");
  }
  const assessments = {
    backing: assessFixedNativeBackingAttestation(value?.fixedNativeBackingAttestation, candidate),
    boundary: assessBoundaryFuzzCoverage(value?.boundaryFuzzCoverage),
    configuration: assessConfigurationCoverage(value?.configurationCoverage, candidate),
    export: assessVendorDeployableExportRejection(value?.vendorDeployableExportRejection),
    topology: assessLiveLanTopologyVariation(value?.liveLanTopologyVariation, candidate),
  };
  for (const [name, assessment] of Object.entries(assessments)) {
    failures.push(...assessment.failures.map((failure) => `${name}: ${failure}`));
  }
  return { assessments, complete: failures.length === 0, failures };
}

export function isolationGateProofFieldsFromClosure(value, candidate) {
  const assessment = assessIsolationClosureEvidence(value, candidate);
  if (!assessment.complete) {
    throw new Error(`Isolation closure is non-credit: ${assessment.failures.join("; ")}`);
  }
  return {
    boundaryFuzzCoverage: structuredClone(value.boundaryFuzzCoverage),
    fixedNativeBackingAttestation: structuredClone(value.fixedNativeBackingAttestation),
    isolationApproval: {
      decisionId: value.configurationCoverage.approvalDecisionId,
      sha256: value.configurationCoverage.approvalSha256,
    },
    isolationSchemaVersion: ISOLATION_EVIDENCE_RECORD_SCHEMA_VERSION,
    liveLanTopologyVariation: structuredClone(value.liveLanTopologyVariation),
    platformConfigurations: structuredClone(value.configurationCoverage.evidenceBindings),
    vendorDeployableExportRejection: structuredClone(value.vendorDeployableExportRejection),
  };
}

export function assessEvidenceCompleteness(report) {
  const failures = [];
  const requiredArrays = [
    ["browser.cdpEvents", report.browser?.cdpEvents],
    ["browser.playwrightRequests", report.browser?.playwrightRequests],
    ["browser.capabilityEvents", report.browser?.capabilityEvents],
    ["osNetwork.samples", report.osNetwork?.samples],
    ["workflow.fuzzCases", report.workflow?.fuzzCases],
  ];
  for (const [label, value] of requiredArrays) {
    if (!Array.isArray(value)) {
      failures.push(`${label} is missing`);
    }
  }
  if (report.platform?.os !== "win32") {
    failures.push("This harness currently provides process-attributable endpoint capture only on Windows");
  }
  if (report.candidate?.exact !== true) {
    failures.push("The run is not bound to an exact clean candidate");
  }
  const candidateBindings = report.candidate?.inputBlobBindings;
  if (
    report.candidate?.head !== report.candidate?.commit ||
    !Array.isArray(report.candidate?.workspaceChanges) ||
    report.candidate.workspaceChanges.length !== 0 ||
    !Array.isArray(candidateBindings) ||
    candidateBindings.length === 0 ||
    candidateBindings.some((binding) =>
      binding.matchesCandidate !== true ||
      !SHA256_PATTERN.test(String(binding.candidateSha256 ?? "")) ||
      !SHA256_PATTERN.test(String(binding.localSha256 ?? "")))
  ) {
    failures.push("Candidate commit, workspace, requirement, test, or harness byte bindings are incomplete");
  }
  if (report.authority?.directiveSha256Matches !== true) {
    failures.push("The issued Phase 2 directive hash does not match");
  }
  if (report.assertions?.browserCapabilityAdaptersDisabled !== true) {
    failures.push("Browser file/device/network capability adapters were not proven disabled");
  }
  if (
    report.assertions?.hostNetworkAdaptersDisabled !== true ||
    report.hostNetworkAdapters?.analysis?.captureComplete !== true ||
    report.hostNetworkAdapters?.analysis?.adaptersDisabled !== true
  ) {
    failures.push("Host network adapters were not proven disabled before and after the workflow");
  }
  if (report.assertions?.liveLanDiscoveryInvarianceProven !== true) {
    failures.push("Controlled live-LAN discovery invariance was not proven");
  }
  if (report.assertions?.fixedNativeLocalBackingProven !== true) {
    failures.push("Fixed native local non-provider non-removable file backing was not proven");
  }
  const fixedNativeBacking = assessFixedNativeBackingAttestation(
    report.fixedNativeBackingAttestation,
    report.candidate,
  );
  failures.push(...fixedNativeBacking.failures);
  if (report.assertions?.vendorDeployableExportRejectionProven !== true) {
    failures.push("Every export surface was not proven to reject vendor or deployable artifacts");
  }
  const boundaryFuzzCoverage = assessBoundaryFuzzCoverage(report.boundaryFuzzCoverage);
  failures.push(...boundaryFuzzCoverage.failures);
  const liveLanTopologyVariation = assessLiveLanTopologyVariation(
    report.liveLanTopologyVariation,
    report.candidate,
  );
  failures.push(...liveLanTopologyVariation.failures);
  const exportRejection = assessVendorDeployableExportRejection(
    report.vendorDeployableExportRejection,
  );
  failures.push(...exportRejection.failures);
  if (report.assertions?.loopbackTrafficAccounted !== true) {
    failures.push("Loopback test/control traffic was not separately accounted");
  }
  if (report.assertions?.zeroExternalAttempts !== true) {
    failures.push("The explicit zero-application-attempt assertion is absent or false");
  }
  if (report.package?.staticScan?.pass !== true) {
    failures.push("Packaged static isolation scan did not pass");
  }
  if ((report.assertions?.externalAttemptCount ?? -1) !== 0) {
    failures.push("One or more external capability/network attempts were observed");
  }
  if (report.workflow?.completed !== true) {
    failures.push("The packaged workflow did not complete");
  }
  if (
    !Array.isArray(report.workflow?.fuzzCases) ||
    report.workflow.fuzzCases.length < DEFAULT_FUZZ_CASES.length * 2 ||
    report.workflow.fuzzCases.some((fuzzCase) => fuzzCase.injected !== true)
  ) {
    failures.push("The complete two-boundary adversarial fuzz corpus was not injected");
  }
  if (!Array.isArray(report.browser?.pageErrors) || report.browser.pageErrors.length !== 0) {
    failures.push("The browser reported one or more application page errors");
  }
  const capabilityEvents = report.browser?.capabilityEvents ?? [];
  if (
    !Array.isArray(capabilityEvents) ||
    !capabilityEvents.some(({ api, outcome }) => api === "WorkerBlob.instrumentation" && outcome === "instrumented") ||
    !capabilityEvents.some(({ api, classification }) => api === "Worker" && classification === "allowed-internal-blob-worker")
  ) {
    failures.push("Main-realm and packaged-worker capability instrumentation was not installed");
  }
  if (report.osNetwork?.samplerComplete !== true) {
    failures.push("Windows process endpoint sampling was unavailable or incomplete");
  }
  if (report.chromiumNetLog?.parsed !== true) {
    failures.push("Chromium NetLog was unavailable or malformed");
  }
  const configurationCoverage = assessConfigurationCoverage(report.configurationCoverage, report.candidate);
  failures.push(...configurationCoverage.failures);
  const packagedBinding = Array.isArray(report.configurationCoverage?.evidenceBindings)
    ? report.configurationCoverage.evidenceBindings.find(({ configurationId }) =>
      configurationId === SUPPORTED_WINDOWS_CONFIGURATION_IDS[1])
    : undefined;
  if (
    packagedBinding === undefined ||
    !SUPPORTED_CHROMIUM_RUNTIME_PRODUCTS.includes(report.browser?.browserRuntimeProduct) ||
    !isBoundedRuntimeVersion(report.browser?.browserRuntimeVersion) ||
    !SHA256_PATTERN.test(String(report.browser?.browserExecutableSha256 ?? "")) ||
    packagedBinding.browserRuntimeProduct !== report.browser?.browserRuntimeProduct ||
    packagedBinding.browserRuntimeVersion !== report.browser?.browserRuntimeVersion ||
    packagedBinding.browserExecutableSha256 !== report.browser?.browserExecutableSha256
  ) {
    failures.push("The packaged configuration row does not match the measured Chromium runtime identity");
  }
  return { complete: failures.length === 0, failures };
}

export function partitionCausalObservations(observations, applicationAttempts) {
  const applicationKeys = new Set(applicationAttempts.flatMap(networkIdentityKeys));
  const applicationAttributable = [];
  const browserGlobalUnattributed = [];
  for (const observation of observations) {
    const observationKeys = networkIdentityKeys(observation);
    if (observationKeys.some((key) => applicationKeys.has(key))) {
      applicationAttributable.push(observation);
    } else {
      browserGlobalUnattributed.push(observation);
    }
  }
  return { applicationAttributable, browserGlobalUnattributed };
}

export function networkIdentityKeys(record) {
  const values = [];
  for (const key of ["target", "value", "remoteAddress"]) {
    if (typeof record?.[key] === "string") {
      values.push(record[key]);
    }
  }
  if (typeof record?.remoteAddress === "string" && Number(record?.remotePort) > 0) {
    values.push(`${record.remoteAddress}:${record.remotePort}`);
  }
  for (const key of ["target", "host", "origin"]) {
    if (typeof record?.classification?.[key] === "string") {
      values.push(record.classification[key]);
    }
  }
  const keys = new Set();
  for (const value of values) {
    const normalized = value.trim().toLocaleLowerCase("en-US");
    if (normalized.length === 0) {
      continue;
    }
    keys.add(`raw:${normalized}`);
    try {
      const parsed = new URL(value);
      keys.add(`origin:${parsed.origin.toLocaleLowerCase("en-US")}`);
      keys.add(`host:${parsed.hostname.toLocaleLowerCase("en-US")}`);
      if (parsed.port !== "") {
        keys.add(`endpoint:${parsed.hostname.toLocaleLowerCase("en-US")}:${parsed.port}`);
      }
    } catch {
      const endpoint = /^\[?([^\]]+)\]?:([0-9]+)$/u.exec(normalized);
      if (endpoint !== null) {
        keys.add(`endpoint:${endpoint[1]}:${endpoint[2]}`);
        keys.add(`host:${endpoint[1]}`);
      } else if (/^[a-z0-9.-]+$/u.test(normalized)) {
        keys.add(`host:${normalized}`);
      }
    }
  }
  return [...keys];
}

function collectStrings(value, seen = new Set()) {
  if (typeof value === "string") {
    return [value];
  }
  if (value === null || typeof value !== "object" || seen.has(value)) {
    return [];
  }
  seen.add(value);
  const result = [];
  for (const item of Array.isArray(value) ? value : Object.values(value)) {
    result.push(...collectStrings(item, seen));
  }
  return result;
}

function looksLikeTarget(value) {
  return (
    /^[a-z][a-z0-9+.-]*:\/\//iu.test(value) ||
    /^\[[0-9a-f:%]+\]:\d+$/iu.test(value) ||
    /^(?:\d{1,3}\.){3}\d{1,3}:\d+$/u.test(value) ||
    /^(?:localhost|[a-z0-9.-]+\.invalid)(?::\d+)?$/iu.test(value)
  );
}

function extractTargets(value) {
  const text = String(value ?? "");
  const urls = text.match(/[a-z][a-z0-9+.-]*:\/\/[^\s<>]+/giu) ?? [];
  if (urls.length > 0) {
    return urls;
  }
  const endpoints = text.match(/\[[0-9a-f:%]+\]:\d+|(?:\d{1,3}\.){3}\d{1,3}:\d+/giu) ?? [];
  const names = text.match(/(?:~notfound|localhost|[a-z0-9.-]+\.invalid)(?::\d+)?/giu) ?? [];
  return [...urls, ...endpoints, ...names];
}

function reverseNumericMap(record) {
  const result = new Map();
  for (const [name, value] of Object.entries(record)) {
    if (typeof value === "number") {
      result.set(value, name);
    }
  }
  return result;
}

function deduplicate(values) {
  const seen = new Set();
  const result = [];
  for (const value of values) {
    const key = JSON.stringify(sortJson(value));
    if (!seen.has(key)) {
      seen.add(key);
      result.push(value);
    }
  }
  return result;
}
