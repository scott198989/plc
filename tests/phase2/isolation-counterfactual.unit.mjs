import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdtempSync,
  readFileSync,
  rmdirSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import {
  DEFAULT_FUZZ_CASES,
  EXPECTED_DIRECTIVE_SHA256,
  FUZZ_CASE_IDS_SHA256,
  FUZZ_CORPUS_SHA256,
  ISOLATION_VERIFICATION_IDS,
  REQUIRED_EXPORT_SURFACE_IDS,
  REQUIRED_FUZZ_BOUNDARY_IDS,
  REQUIRED_NATIVE_BROKER_OPERATION_IDS,
  SUPPORTED_WINDOWS_CONFIGURATION_IDS,
  analyzeCapabilityEvents,
  analyzeHostNetworkAdapters,
  analyzeNetLogTargets,
  analyzeProcessEndpoints,
  assessBoundaryFuzzCoverage,
  assessConfigurationCoverage,
  assessEvidenceCompleteness,
  assessFixedNativeBackingAttestation,
  assessIsolationClosureEvidence,
  assessLiveLanTopologyVariation,
  assessVendorDeployableExportRejection,
  classifyUrl,
  deriveProcessTree,
  isLoopbackHost,
  isolationGateProofFieldsFromClosure,
  parseChromiumNetLog,
  parseGitStatusPorcelainZ,
  partitionCausalObservations,
  scanPackagedHtml,
  sha256,
  splitEndpoint,
  stableJson,
} from "../../tools/phase2/isolation-counterfactual-lib.mjs";

test("authority constants bind all isolation obligations to the issued directive", () => {
  assert.match(EXPECTED_DIRECTIVE_SHA256, /^[0-9A-F]{64}$/u);
  assert.deepEqual(ISOLATION_VERIFICATION_IDS, [
    "VER-ISO-0001",
    "VER-ISO-0002",
    "VER-ISO-0003",
    "VER-ISO-0004",
    "VER-ISO-0005",
    "VER-NET-0001",
  ]);
});

test("fuzz corpus is deterministic and covers endpoint, UNC, device, print, and malformed classes", () => {
  assert.equal(new Set(DEFAULT_FUZZ_CASES.map(({ id }) => id)).size, DEFAULT_FUZZ_CASES.length);
  const categories = new Set(DEFAULT_FUZZ_CASES.map(({ category }) => category));
  for (const expected of [
    "broadcast", "device", "hostname", "ipv6", "loopback", "malformed", "multicast",
    "print", "private", "protocol", "public", "unc", "url",
  ]){
    assert.equal(categories.has(expected), true, `missing ${expected}`);
  }
  assert.ok(DEFAULT_FUZZ_CASES.some(({ value }) => value.includes("\\\\.\\pipe\\")));
  assert.ok(DEFAULT_FUZZ_CASES.some(({ value }) => value.includes("\u0000")));
  assert.equal(FUZZ_CORPUS_SHA256, "C61573EF4B2B686E4DC8E326505B65BFFFC4FFE247D8BE2855612F0D6D3D0F66");
  assert.equal(FUZZ_CASE_IDS_SHA256, "D7FDF0D3ED6E8BF03772F83F44E7F51E432BE67DE0BD20B247FFE076DFD329F8");
  const schema = JSON.parse(readFileSync(
    new URL("../../tools/phase2/isolation-closure-evidence.schema.json", import.meta.url),
    "utf8",
  ));
  assert.equal(schema.$defs.boundaryFuzzCoverage.allOf[1].properties.corpusSha256.const, FUZZ_CORPUS_SHA256);
  assert.equal(schema.$defs.boundaryFuzzCoverage.allOf[1].properties.caseIdsSha256.const, FUZZ_CASE_IDS_SHA256);
});

test("stable evidence serialization and hashing are order independent", () => {
  const left = stableJson({ z: 1, a: { y: 2, b: 3 } });
  const right = stableJson({ a: { b: 3, y: 2 }, z: 1 });
  assert.equal(left, right);
  assert.equal(sha256(left), sha256(right));
});

test("loopback classification is exact and rejects lookalikes", () => {
  for (const value of ["localhost", "127.0.0.1", "127.23.45.67", "::1", "[::1]"]) {
    assert.equal(isLoopbackHost(value), true, value);
  }
  for (const value of ["localhost.invalid", "127.0.0.1.invalid", "192.0.2.1", "::2"]) {
    assert.equal(isLoopbackHost(value), false, value);
  }
});

test("URL allowlist rejects prefix, credential, and malformed bypasses", () => {
  const allowed = new Set(["http://127.0.0.1:42100"]);
  assert.equal(classifyUrl("http://127.0.0.1:42100/index.html", allowed).allowed, true);
  assert.equal(classifyUrl("http://127.0.0.1:42100.invalid/", allowed).allowed, false);
  assert.equal(classifyUrl("http://127.0.0.1:42101/", allowed).allowed, false);
  assert.equal(classifyUrl("http://[::1", allowed).allowed, false);
  assert.equal(classifyUrl("blob:http://127.0.0.1:42100/id", allowed).allowed, true);
});

test("endpoint parser preserves IPv6 and port identity", () => {
  assert.deepEqual(splitEndpoint("[::1]:443"), { host: "::1", port: 443, raw: "[::1]:443" });
  assert.deepEqual(splitEndpoint("127.0.0.1:102"), {
    host: "127.0.0.1",
    port: 102,
    raw: "127.0.0.1:102",
  });
  assert.equal(splitEndpoint("::1").port, null);
});

test("process-tree derivation never attributes sibling or ancestor processes", () => {
  const tree = deriveProcessTree([
    { name: "parent", parentPid: 1, pid: 10 },
    { name: "browser", parentPid: 10, pid: 20 },
    { name: "renderer", parentPid: 20, pid: 21 },
    { name: "gpu", parentPid: 20, pid: 22 },
    { name: "unrelated", parentPid: 10, pid: 30 },
    { name: "unrelated-child", parentPid: 30, pid: 31 },
  ], 20);
  assert.deepEqual(tree.map(({ pid }) => pid), [20, 21, 22]);
});

test("process endpoint analysis separately accounts exact loopback and fails external or UDP sockets", () => {
  const result = analyzeProcessEndpoints([
    {
      endpoints: [
        { localAddress: "127.0.0.1", localPort: 51000, owningProcess: 21, protocol: "TCP", remoteAddress: "127.0.0.1", remotePort: 42100, state: "Established" },
        { localAddress: "10.0.0.7", localPort: 51001, owningProcess: 21, protocol: "TCP", remoteAddress: "192.0.2.1", remotePort: 102, state: "SynSent" },
        { localAddress: "0.0.0.0", localPort: 5353, owningProcess: 22, protocol: "UDP", remoteAddress: "", remotePort: 0, state: "BOUND" },
        { localAddress: "127.0.0.1", localPort: 51002, owningProcess: 21, protocol: "TCP", remoteAddress: "127.0.0.1", remotePort: 43000, state: "Established" },
      ],
    },
  ], new Set(["127.0.0.1:42100"]));
  assert.equal(result.loopbackAccounted.length, 1);
  assert.deepEqual(new Set(result.externalAttempts.map(({ reason }) => reason)), new Set([
    "external-remote",
    "udp-endpoint-opened",
    "unaccounted-loopback-remote",
  ]));
});

test("host adapter analysis requires before-and-after capture and rejects every active adapter", () => {
  const disabled = analyzeHostNetworkAdapters([
    {
      adapters: [{ ifIndex: 7, MediaConnectionState: "Disconnected", Name: "Ethernet", Status: "Disabled" }],
      boundary: "preflight",
      capturedAt: "2020-01-01T00:00:00.000Z",
    },
    {
      adapters: [{ ifIndex: 7, MediaConnectionState: "Disconnected", Name: "Ethernet", Status: "Disabled" }],
      boundary: "postflight",
      capturedAt: "2020-01-01T00:01:00.000Z",
    },
  ]);
  assert.equal(disabled.captureComplete, true);
  assert.equal(disabled.adaptersDisabled, true);

  const active = analyzeHostNetworkAdapters([
    {
      adapters: [{ ifIndex: 7, MediaConnectionState: "Connected", Name: "Wi-Fi", Status: "Up" }],
      boundary: "preflight",
      capturedAt: "2020-01-01T00:00:00.000Z",
    },
    {
      adapters: [{ ifIndex: 7, MediaConnectionState: "Connected", Name: "Wi-Fi", Status: "Up" }],
      boundary: "postflight",
      capturedAt: "2020-01-01T00:01:00.000Z",
    },
  ]);
  assert.equal(active.adaptersDisabled, false);
  assert.equal(active.activeAdapters.length, 2);
  assert.equal(analyzeHostNetworkAdapters([{ adapters: [] }]).captureComplete, false);
});

test("capability analysis allows only the packaged blob worker", () => {
  const result = analyzeCapabilityEvents([
    { api: "Worker", classification: "allowed-internal-blob-worker", outcome: "allowed", target: "blob:http://127.0.0.1/id" },
    { api: "fetch", classification: "denied-capability", outcome: "denied", target: "https://plc.isolation.invalid/" },
  ], new Set(["http://127.0.0.1:42100"]));
  assert.equal(result.accountedInternal.length, 1);
  assert.equal(result.externalAttempts.length, 1);
  assert.equal(result.externalAttempts[0].api, "fetch");
});

test("packaged scan accepts the constrained inline shape", () => {
  const html = `<!doctype html><meta http-equiv="Content-Security-Policy" content="default-src 'none'; base-uri 'none'; connect-src 'none'; form-action 'none'; object-src 'none'; worker-src blob:"><style>body{}</style><script type="module">const core = "AGFzbQEAAAA="; const engineering = "AGFzbQEAAAAAAQA=";</script>`;
  assert.deepEqual(scanPackagedHtml(html).findings, []);
});

test("packaged scan fails each executable capability escape", () => {
  const base = `<!doctype html><meta http-equiv="Content-Security-Policy" content="default-src 'none'; base-uri 'none'; connect-src 'none'; form-action 'none'; object-src 'none'; worker-src blob:">`;
  const mutations = [
    ["external asset", `<script src="https://plc.isolation.invalid/x.js"></script>`],
    ["network constructor", `<script>new WebSocket("wss://plc.isolation.invalid")</script>`],
    ["device API", `<script>navigator.serial.requestPort()</script>`],
    ["dynamic import", `<script>import("https://plc.isolation.invalid/x.js")</script>`],
    ["dynamic execution", `<script>eval("1")</script>`],
    ["local server", `<script>createServer().listen(102)</script>`],
  ];
  for (const [label, mutation] of mutations) {
    assert.equal(scanPackagedHtml(base + mutation).pass, false, label);
  }
});

test("Chromium NetLog parsing finds DNS/socket targets and ignores unrelated metadata", () => {
  const parsed = parseChromiumNetLog({
    constants: {
      logEventTypes: { HOST_RESOLVER_MANAGER_REQUEST: 100, URL_REQUEST_START_JOB: 200, COOKIE_STORE_ALIVE: 300 },
      logSourceType: { HOST_RESOLVER_IMPL_JOB: 10, URL_REQUEST: 20 },
    },
    events: [
      { params: { host: "plc.isolation.invalid" }, phase: 0, source: { id: 1, type: 10 }, type: 100 },
      { params: { url: "http://127.0.0.1:42100/" }, phase: 0, source: { id: 2, type: 20 }, type: 200 },
      { params: { name: "plc.isolation.invalid" }, phase: 0, source: { id: 3, type: 20 }, type: 300 },
    ],
  });
  assert.equal(parsed.relevantEventCount, 2);
  assert.equal(parsed.targetStrings.length, 2);
  const analysis = analyzeNetLogTargets(
    parsed,
    new Set(["http://127.0.0.1:42100"]),
    new Set(["127.0.0.1:42100"]),
  );
  assert.equal(analysis.loopbackAccounted.length, 1);
  assert.equal(analysis.externalAttempts.length, 1);
});

test("counterfactual causal rule never attributes unrelated browser-global traffic to the app", () => {
  const observations = [
    { typeName: "URL_REQUEST_START_JOB", value: "https://updates.browser.invalid/check" },
    { typeName: "URL_REQUEST_START_JOB", value: "https://plc.isolation.invalid/device/1" },
    { remoteAddress: "192.0.2.9", remotePort: 443 },
  ];
  const applicationAttempts = [
    { channel: "javascript-capability", target: "https://plc.isolation.invalid/device/1" },
  ];
  const partition = partitionCausalObservations(observations, applicationAttempts);
  assert.deepEqual(partition.applicationAttributable, [observations[1]]);
  assert.deepEqual(partition.browserGlobalUnattributed, [observations[0], observations[2]]);
  const noApplicationAttempt = partitionCausalObservations(observations, []);
  assert.equal(noApplicationAttempt.applicationAttributable.length, 0);
  assert.equal(noApplicationAttempt.browserGlobalUnattributed.length, 3);
});

test("NUL-delimited Git status preserves spaces and quotes without display quoting", () => {
  const parsed = parseGitStatusPorcelainZ(
    "?? References for Codex from Scott/PHASE_1_ADVERSARIAL_AUDIT.docx\u0000 M tools/phase2/file.mjs\u0000",
  );
  assert.deepEqual(parsed, [
    { path: "References for Codex from Scott/PHASE_1_ADVERSARIAL_AUDIT.docx", state: "??" },
    { path: "tools/phase2/file.mjs", state: " M" },
  ]);
});

test("configuration coverage rejects unresolved, partial, and non-credit evidence", () => {
  const digest = "A".repeat(64);
  const candidate = completeCandidate(digest);
  const baseline = {
    approvalDecisionId: "P2-DEC-ISO-NATIVE-001",
    approvalSha256: digest,
    approvalStatus: "APPROVED",
    evidenceBindings: SUPPORTED_WINDOWS_CONFIGURATION_IDS.map((configurationId, index) => ({
      architecture: "x64",
      browserExecutableSha256: digest,
      browserFamily: "chromium",
      browserRuntimeProduct: index === 0 ? "microsoft-edge-webview2" : "microsoft-edge",
      browserRuntimeVersion: "140.0.0.0",
      candidateCommit: candidate.commit,
      candidateTree: candidate.tree,
      completeLogs: true,
      configurationId,
      evidenceManifestSha256: digest,
      fileAccessPosture: index === 0 ? "native-broker" : "packaged-browser-disabled",
      hostNetworkPosture: index === 0 ? "adapters-on-controlled-lan" : "adapters-off",
      matchesCandidate: true,
      platform: "windows",
      productionPathExercised: true,
      result: "PASS",
      zeroExternalAttempts: true,
    })),
    expectedConfigurationIds: [...SUPPORTED_WINDOWS_CONFIGURATION_IDS],
    status: "COMPLETE",
  };
  assert.equal(assessConfigurationCoverage(baseline, candidate).complete, true);
  assert.equal(assessConfigurationCoverage({ ...baseline, approvalDecisionId: null }).complete, false);
  assert.equal(assessConfigurationCoverage({ ...baseline, approvalDecisionId: "ADR-FAKE" }).complete, false);
  assert.equal(assessConfigurationCoverage({ ...baseline, approvalStatus: "UNRESOLVED" }).complete, false);
  assert.equal(assessConfigurationCoverage({ ...baseline, expectedConfigurationIds: [] }).complete, false);
  assert.equal(assessConfigurationCoverage({
    ...baseline,
    evidenceBindings: [...baseline.evidenceBindings, ...baseline.evidenceBindings],
  }).complete, false);
  for (const result of ["SKIPPED", "FLAKY", "UNAVAILABLE", "STALE", "INCONCLUSIVE", "FAIL"]) {
    assert.equal(assessConfigurationCoverage({
      ...baseline,
      evidenceBindings: baseline.evidenceBindings.map((binding, index) =>
        index === 0 ? { ...binding, result } : binding),
    }).complete, false, result);
  }
  assert.equal(assessConfigurationCoverage({
    ...baseline,
    evidenceBindings: baseline.evidenceBindings.map((binding, index) =>
      index === 0 ? { ...binding, candidateTree: "3".repeat(40) } : binding),
  }, candidate).complete, false);
  assert.equal(assessConfigurationCoverage({
    ...baseline,
    evidenceBindings: baseline.evidenceBindings.map((binding, index) =>
      index === 0 ? { ...binding, browserRuntimeVersion: "" } : binding),
  }, candidate).complete, false);
  assert.equal(assessConfigurationCoverage({
    ...baseline,
    evidenceBindings: baseline.evidenceBindings.map((binding, index) =>
      index === 0 ? { ...binding, browserRuntimeProduct: "microsoft-edge" } : binding),
  }, candidate).complete, false);
  assert.equal(assessConfigurationCoverage({
    ...baseline,
    evidenceBindings: baseline.evidenceBindings.map((binding, index) =>
      index === 1 ? { ...binding, browserRuntimeProduct: "microsoft-edge-webview2" } : binding),
  }, candidate).complete, false);
  assert.equal(assessConfigurationCoverage({
    ...baseline,
    evidenceBindings: baseline.evidenceBindings.map((binding, index) =>
      index === 0 ? { ...binding, theater: false } : binding),
  }, candidate).complete, false);
});

test("typed boundary, topology, and export proof contracts reject omissions and theater", () => {
  const digest = "A".repeat(64);
  const candidate = completeCandidate(digest);
  const boundaryFuzzCoverage = completeBoundaryCoverage();
  assert.equal(assessBoundaryFuzzCoverage(boundaryFuzzCoverage).complete, true);
  assert.equal(assessBoundaryFuzzCoverage({
    ...boundaryFuzzCoverage,
    boundaries: boundaryFuzzCoverage.boundaries.slice(1),
  }).complete, false);
  assert.equal(assessBoundaryFuzzCoverage({
    ...boundaryFuzzCoverage,
    boundaries: boundaryFuzzCoverage.boundaries.map((boundary, index) =>
      index === 0 ? { ...boundary, sideEffectsObserved: true } : boundary),
  }).complete, false);
  assert.equal(assessBoundaryFuzzCoverage({
    ...boundaryFuzzCoverage,
    boundaries: boundaryFuzzCoverage.boundaries.map((boundary, index) =>
      index === 0 ? { ...boundary, theater: false } : boundary),
  }).complete, false);

  const topology = completeTopologyVariation(candidate, digest);
  assert.equal(assessLiveLanTopologyVariation(topology, candidate).complete, true);
  assert.equal(assessLiveLanTopologyVariation({
    ...topology,
    scenarios: topology.scenarios.map((scenario) => ({
      ...scenario,
      topologyFingerprint: digest,
    })),
  }, candidate).complete, false);
  assert.equal(assessLiveLanTopologyVariation({
    ...topology,
    scenarios: topology.scenarios.map((scenario, index) =>
      index === 0 ? { ...scenario, externalAttemptCount: 1 } : scenario),
  }, candidate).complete, false);
  assert.equal(assessLiveLanTopologyVariation({
    ...topology,
    scenarios: topology.scenarios.map((scenario, index) =>
      index === 0 ? { ...scenario, postTopologyFingerprint: "9".repeat(64) } : scenario),
  }, candidate).complete, false);
  assert.equal(assessLiveLanTopologyVariation({ ...topology, theater: false }, candidate).complete, false);

  const exportEvidence = completeExportEvidence();
  assert.equal(assessVendorDeployableExportRejection(exportEvidence).complete, true);
  assert.equal(assessVendorDeployableExportRejection({
    ...exportEvidence,
    surfaces: exportEvidence.surfaces.slice(1),
  }).complete, false);
  assert.equal(assessVendorDeployableExportRejection({
    ...exportEvidence,
    surfaces: exportEvidence.surfaces.map((surface, index) =>
      index === 0 ? { ...surface, theater: false } : surface),
  }).complete, false);
  const backing = completeNativeBacking(candidate, digest);
  assert.equal(assessFixedNativeBackingAttestation(backing, candidate).complete, true);
  assert.equal(assessFixedNativeBackingAttestation({
    ...backing,
    operations: backing.operations.map((operation, index) =>
      index === 0 ? { ...operation, providerBacked: true } : operation),
  }, candidate).complete, false);
  assert.equal(assessFixedNativeBackingAttestation({ ...backing, theater: false }, candidate).complete, false);
});

test("the closure helper requires the exact candidate, exact fields, and complete configuration status", () => {
  const digest = "A".repeat(64);
  const candidate = completeCandidate(digest);
  const configurationCoverage = completeConfigurationCoverage(candidate, digest);
  const closure = {
    boundaryFuzzCoverage: completeBoundaryCoverage(),
    candidateCommit: candidate.commit,
    candidateTree: candidate.tree,
    configurationCoverage,
    evidenceKind: "PHASE2_ISOLATION_CLOSURE_INPUT",
    fixedNativeBackingAttestation: completeNativeBacking(candidate, digest),
    liveLanTopologyVariation: completeTopologyVariation(candidate, digest),
    schemaVersion: "1.0",
    vendorDeployableExportRejection: completeExportEvidence(),
  };
  assert.equal(assessIsolationClosureEvidence(closure, candidate).complete, true);
  const gateFields = isolationGateProofFieldsFromClosure(closure, candidate);
  assert.deepEqual(gateFields.platformConfigurations, configurationCoverage.evidenceBindings);
  assert.deepEqual(gateFields.isolationApproval, {
    decisionId: candidate.isolationApprovalDecisionId,
    sha256: candidate.isolationApprovalSha256,
  });
  assert.equal(gateFields.isolationSchemaVersion, "2.0");
  const temporary = mkdtempSync(path.join(tmpdir(), "phase2-isolation-transform-"));
  const inputPath = path.join(temporary, "closure.json");
  const outputPath = path.join(temporary, "gate-fields.json");
  try {
    writeFileSync(inputPath, JSON.stringify(closure), "utf8");
    const transformed = spawnSync(process.execPath, [
      path.resolve("tools/phase2/transform_isolation_closure.mjs"),
      "--approval-decision-id", candidate.isolationApprovalDecisionId,
      "--approval-sha256", candidate.isolationApprovalSha256,
      "--input", inputPath,
      "--output", outputPath,
      "--candidate-commit", candidate.commit,
      "--candidate-tree", candidate.tree,
    ], { encoding: "utf8" });
    assert.equal(transformed.status, 0, transformed.stderr);
    assert.deepEqual(JSON.parse(readFileSync(outputPath, "utf8")), gateFields);
    unlinkSync(outputPath);
    writeFileSync(inputPath, JSON.stringify({
      ...closure,
      configurationCoverage: { ...configurationCoverage, status: "PARTIAL" },
    }), "utf8");
    const rejected = spawnSync(process.execPath, [
      path.resolve("tools/phase2/transform_isolation_closure.mjs"),
      "--approval-decision-id", candidate.isolationApprovalDecisionId,
      "--approval-sha256", candidate.isolationApprovalSha256,
      "--input", inputPath,
      "--output", outputPath,
      "--candidate-commit", candidate.commit,
      "--candidate-tree", candidate.tree,
    ], { encoding: "utf8" });
    assert.notEqual(rejected.status, 0);
    assert.equal(existsSync(outputPath), false);
  } finally {
    for (const file of [outputPath, inputPath]) {
      if (existsSync(file)) {
        unlinkSync(file);
      }
    }
    rmdirSync(temporary);
  }
  gateFields.platformConfigurations[0].result = "MUTATED_COPY";
  assert.equal(configurationCoverage.evidenceBindings[0].result, "PASS");
  assert.equal(assessIsolationClosureEvidence({ ...closure, theater: true }, candidate).complete, false);
  assert.equal(assessIsolationClosureEvidence({
    ...closure,
    candidateTree: "3".repeat(40),
  }, candidate).complete, false);
  assert.equal(assessIsolationClosureEvidence(
    closure,
    { ...candidate, isolationApprovalSha256: "B".repeat(64) },
  ).complete, false);
  assert.equal(assessIsolationClosureEvidence({
    ...closure,
    configurationCoverage: { ...configurationCoverage, status: "PARTIAL" },
  }, candidate).complete, false);
  assert.throws(() => isolationGateProofFieldsFromClosure({
    ...closure,
    configurationCoverage: { ...configurationCoverage, status: "PARTIAL" },
  }, candidate), /non-credit/u);
});

test("evidence completeness fails closed on stale, partial, or unsupported runs while date alone does not expire a match", () => {
  const digest = "A".repeat(64);
  const candidate = completeCandidate(digest);
  const baseline = {
    assertions: {
      browserCapabilityAdaptersDisabled: true,
      externalAttemptCount: 0,
      fixedNativeLocalBackingProven: true,
      hostNetworkAdaptersDisabled: true,
      liveLanDiscoveryInvarianceProven: true,
      loopbackTrafficAccounted: true,
      vendorDeployableExportRejectionProven: true,
      zeroExternalAttempts: true,
    },
    authority: { directiveSha256Matches: true },
    browser: {
      browserExecutableSha256: digest,
      browserRuntimeProduct: "microsoft-edge",
      browserRuntimeVersion: "140.0.0.0",
      capabilityEvents: [
        { api: "WorkerBlob.instrumentation", outcome: "instrumented" },
        { api: "Worker", classification: "allowed-internal-blob-worker" },
      ],
      cdpEvents: [],
      pageErrors: [],
      playwrightRequests: [],
    },
    boundaryFuzzCoverage: completeBoundaryCoverage(),
    candidate: {
      ...candidate,
      exact: true,
      head: candidate.commit,
      inputBlobBindings: [{
        candidateSha256: digest,
        localSha256: digest,
        matchesCandidate: true,
        path: "tests/phase2/isolation-counterfactual.unit.mjs",
      }],
      workspaceChanges: [],
    },
    chromiumNetLog: { parsed: true },
    completedAt: "2000-01-01T00:01:00.000Z",
    configurationCoverage: {
      approvalDecisionId: "P2-DEC-ISO-NATIVE-001",
      approvalSha256: digest,
      approvalStatus: "APPROVED",
      evidenceBindings: SUPPORTED_WINDOWS_CONFIGURATION_IDS.map((configurationId, index) => ({
        architecture: "x64",
        browserExecutableSha256: digest,
        browserFamily: "chromium",
        browserRuntimeProduct: index === 0 ? "microsoft-edge-webview2" : "microsoft-edge",
        browserRuntimeVersion: "140.0.0.0",
        candidateCommit: candidate.commit,
        candidateTree: candidate.tree,
        completeLogs: true,
        configurationId,
        evidenceManifestSha256: digest,
        fileAccessPosture: index === 0 ? "native-broker" : "packaged-browser-disabled",
        hostNetworkPosture: index === 0 ? "adapters-on-controlled-lan" : "adapters-off",
        matchesCandidate: true,
        platform: "windows",
        productionPathExercised: true,
        result: "PASS",
        zeroExternalAttempts: true,
      })),
      expectedConfigurationIds: [...SUPPORTED_WINDOWS_CONFIGURATION_IDS],
      status: "COMPLETE",
    },
    hostNetworkAdapters: {
      analysis: { adaptersDisabled: true, captureComplete: true },
      snapshots: [{ adapters: [] }, { adapters: [] }],
    },
    fixedNativeBackingAttestation: completeNativeBacking(candidate, digest),
    osNetwork: { samplerComplete: true, samples: [] },
    package: { staticScan: { pass: true } },
    platform: { os: "win32" },
    liveLanTopologyVariation: completeTopologyVariation(candidate, digest),
    startedAt: "2000-01-01T00:00:00.000Z",
    workflow: {
      completed: true,
      fuzzCases: Array.from({ length: DEFAULT_FUZZ_CASES.length * 2 }, () => ({ injected: true })),
    },
    vendorDeployableExportRejection: completeExportEvidence(),
  };
  assert.equal(assessEvidenceCompleteness(baseline).complete, true);
  assert.equal(assessEvidenceCompleteness({
    ...baseline,
    candidate: { ...baseline.candidate, exact: false },
  }).complete, false);
  assert.equal(assessEvidenceCompleteness({
    ...baseline,
    candidate: {
      ...baseline.candidate,
      inputBlobBindings: [{ ...baseline.candidate.inputBlobBindings[0], matchesCandidate: false }],
    },
  }).complete, false);
  assert.equal(assessEvidenceCompleteness({ ...baseline, platform: { os: "linux" } }).complete, false);
  assert.equal(assessEvidenceCompleteness({ ...baseline, chromiumNetLog: { parsed: false } }).complete, false);
  assert.equal(assessEvidenceCompleteness({
    ...baseline,
    assertions: { ...baseline.assertions, externalAttemptCount: 1 },
  }).complete, false);
  for (const flag of [
    "browserCapabilityAdaptersDisabled",
    "fixedNativeLocalBackingProven",
    "hostNetworkAdaptersDisabled",
    "liveLanDiscoveryInvarianceProven",
    "vendorDeployableExportRejectionProven",
  ]) {
    assert.equal(assessEvidenceCompleteness({
      ...baseline,
      assertions: { ...baseline.assertions, [flag]: false },
    }).complete, false, flag);
  }
  assert.equal(assessEvidenceCompleteness({
    ...baseline,
    configurationCoverage: { ...baseline.configurationCoverage, approvalDecisionId: null },
  }).complete, false);
  assert.equal(assessEvidenceCompleteness({
    ...baseline,
    browser: { ...baseline.browser, browserRuntimeVersion: "141.0.0.0" },
  }).complete, false);
});

function completeBoundaryCoverage() {
  return {
    boundaries: REQUIRED_FUZZ_BOUNDARY_IDS.map((boundaryId) => ({
      boundaryId,
      caseCount: DEFAULT_FUZZ_CASES.length,
      caseIdsSha256: FUZZ_CASE_IDS_SHA256,
      corpusSha256: FUZZ_CORPUS_SHA256,
      externalAttemptCount: 0,
      productionPathExercised: true,
      result: "PASS",
      sideEffectsObserved: false,
    })),
    caseCount: DEFAULT_FUZZ_CASES.length,
    caseIdsSha256: FUZZ_CASE_IDS_SHA256,
    complete: true,
    corpusSha256: FUZZ_CORPUS_SHA256,
    result: "PASS",
    schemaVersion: "1.0",
  };
}

function completeCandidate(digest) {
  return {
    commit: "1".repeat(40),
    isolationApprovalDecisionId: "P2-DEC-ISO-NATIVE-001",
    isolationApprovalSha256: digest,
    tree: "2".repeat(40),
  };
}

function completeTopologyVariation(candidate, digest) {
  return {
    applicationNetworkCapabilityPresent: false,
    complete: true,
    discoveryApiSurfacePresent: false,
    result: "PASS",
    scenarios: ["A", "B"].map((scenarioId, index) => ({
      architecture: "x64",
      candidateCommit: candidate.commit,
      candidateTree: candidate.tree,
      completeLogs: true,
      configurationId: SUPPORTED_WINDOWS_CONFIGURATION_IDS[0],
      controlledInputSha256: "C".repeat(64),
      deterministicOutputSha256: "D".repeat(64),
      evidenceManifestSha256: digest,
      externalAttemptCount: 0,
      platform: "windows",
      postTopologyFingerprint: (index === 0 ? "E" : "F").repeat(64),
      preTopologyFingerprint: (index === 0 ? "E" : "F").repeat(64),
      productionPathExercised: true,
      result: "PASS",
      scenarioId,
      topologyFingerprint: (index === 0 ? "E" : "F").repeat(64),
      topologyMutationControl: "EXTERNAL_LAB_OR_OPERATOR_CONTROLLED",
      topologySource: "WINDOWS_LIVE_ADAPTER_SNAPSHOT",
    })),
    schemaVersion: "1.0",
  };
}

function completeConfigurationCoverage(candidate, digest) {
  return {
    approvalDecisionId: "P2-DEC-ISO-NATIVE-001",
    approvalSha256: digest,
    approvalStatus: "APPROVED",
    evidenceBindings: SUPPORTED_WINDOWS_CONFIGURATION_IDS.map((configurationId, index) => ({
      architecture: "x64",
      browserExecutableSha256: digest,
      browserFamily: "chromium",
      browserRuntimeProduct: index === 0 ? "microsoft-edge-webview2" : "microsoft-edge",
      browserRuntimeVersion: "140.0.0.0",
      candidateCommit: candidate.commit,
      candidateTree: candidate.tree,
      completeLogs: true,
      configurationId,
      evidenceManifestSha256: digest,
      fileAccessPosture: index === 0 ? "native-broker" : "packaged-browser-disabled",
      hostNetworkPosture: index === 0 ? "adapters-on-controlled-lan" : "adapters-off",
      matchesCandidate: true,
      platform: "windows",
      productionPathExercised: true,
      result: "PASS",
      zeroExternalAttempts: true,
    })),
    expectedConfigurationIds: [...SUPPORTED_WINDOWS_CONFIGURATION_IDS],
    status: "COMPLETE",
  };
}

function completeExportEvidence() {
  return {
    complete: true,
    result: "PASS",
    schemaVersion: "1.0",
    surfaces: REQUIRED_EXPORT_SURFACE_IDS.map((surfaceId) => ({
      closedFormatSet: true,
      deployableArtifactAttemptsRejected: true,
      productionPathExercised: true,
      result: "PASS",
      sideEffectsObserved: false,
      surfaceId,
      vendorArtifactAttemptsRejected: true,
    })),
  };
}

function completeNativeBacking(candidate, digest) {
  return {
    architecture: "x64",
    candidateCommit: candidate.commit,
    candidateTree: candidate.tree,
    complete: true,
    decisionId: "P2-DEC-ISO-NATIVE-001",
    evidenceManifestSha256: digest,
    operations: REQUIRED_NATIVE_BROKER_OPERATION_IDS.map((operationId) => ({
      attestationVersion: 1,
      fixedLocalBacking: true,
      metadataOnlyBeforeAcceptance: true,
      operationId,
      productionPathExercised: true,
      providerBacked: false,
      redirected: false,
      remote: false,
      removable: false,
      result: "PASS",
      selectedByteIoBeforeAcceptance: false,
      special: false,
      unapprovedHelperEffectObserved: false,
      unsafeTarget: false,
    })),
    platform: "windows",
    result: "PASS",
    schemaVersion: "1.0",
  };
}
