import assert from "node:assert/strict";
import { Buffer } from "node:buffer";
import test from "node:test";

import {
  EXTERNAL_OBSERVER_SESSION,
  EXTERNAL_OBSERVER_VERSION,
  REQUIRED_ETW_PROVIDERS,
  analyzeExternalObserverEvidence,
  fixedObserverFileNames,
  hashExternalObserverBytes,
} from "./external_observer_evidence.mjs";

const A = "A".repeat(64);
const B = "B".repeat(64);
const COMMIT = "a".repeat(40);
const TREE = "b".repeat(40);
const PROCESS = "22FB2CD6-0E7B-422B-A0C7-2FAD1FD0E716";
const AFD = "E53C6823-7BB8-44BB-90DC-3F86090D48A6";
const NAMES = fixedObserverFileNames();
const WINDOWS_TO_UNIX_EPOCH_100NS = 116444736000000000n;

function fileTimeForUtc(utc) {
  return (BigInt(Date.parse(utc)) * 10_000n + WINDOWS_TO_UNIX_EPOCH_100NS).toString();
}

const jsonBytes = (value) => Buffer.from(`${JSON.stringify(value, null, 2)}\n`, "utf8");
const property = (name, value) => ({ name, value: String(value) });

function event(sequence, providerId, kind, properties, timestamp = BigInt(fileTimeForUtc("2026-08-28T03:00:01.000Z"))) {
  return {
    eventId: kind === "PROCESS_START" ? 1 : kind === "PROCESS_STOP" ? 2 : 1000,
    eventName: kind,
    headerProcessId: Number(properties.find(({ name }) => name === "ObserverProcessId")?.value ?? 0),
    headerThreadId: 7,
    keyword: "0x0000000000000010",
    kind,
    level: 5,
    opcode: 1,
    opcodeName: kind,
    properties,
    providerId,
    sequence,
    task: 1,
    taskName: kind,
    timestampFileTime: String(timestamp + BigInt(sequence)),
    version: 0,
  };
}

function descriptor() {
  return {
    eventId: 1,
    eventName: "Synthetic",
    keyword: "0x0000000000000000",
    level: 5,
    opcode: 1,
    opcodeName: "Synthetic",
    task: 1,
    taskName: "Synthetic",
    version: 0,
  };
}

function fixture() {
  const candidateImageBytes = Buffer.from("candidate-image", "utf8");
  const launcherBytes = Buffer.from("fixed-launcher", "utf8");
  const observerBytes = Buffer.from("fixed-observer", "utf8");
  const observerSourceBytes = Buffer.from("observer-source", "utf8");
  const observerBuildScriptBytes = Buffer.from("observer-build", "utf8");
  const observerAnalyzerSourceBytes = Buffer.from("observer-analyzer", "utf8");
  const observerFinalizerSourceBytes = Buffer.from("observer-finalizer", "utf8");
  const observerSourceVerifierBytes = Buffer.from("observer-source-verifier", "utf8");
  const candidateImageSha256 = hashExternalObserverBytes(candidateImageBytes);
  const candidateManifest = {
    developmentDirty: false,
    evidenceKind: "WINDOWS_NATIVE_CANDIDATE_PACKAGE_MANIFEST",
    gitCommit: COMMIT,
    gitTree: TREE,
    packageFiles: [{ bytes: candidateImageBytes.byteLength, path: "GovsPLC.exe", sha256: candidateImageSha256 }],
    schemaVersion: "1.0",
    sourceInputs: [
      ["tools/phase2/build_external_observer.mjs", observerBuildScriptBytes],
      ["tools/phase2/external_observer_evidence.mjs", observerAnalyzerSourceBytes],
      ["tools/phase2/finalize_external_observer_evidence.mjs", observerFinalizerSourceBytes],
      ["tools/phase2/verify_external_observer_source.mjs", observerSourceVerifierBytes],
      ["tools/phase2/windows_external_observer.cpp", observerSourceBytes],
    ].map(([path, bytes]) => ({ bytes: bytes.byteLength, path, sha256: hashExternalObserverBytes(bytes) })),
  };
  const candidateManifestBytes = jsonBytes(candidateManifest);
  const events = [
    event(1, PROCESS, "PROCESS_START", [
      property("ObserverProcessId", 100), property("ObserverParentProcessId", 50),
      property("ObserverImageSha256", candidateImageSha256),
    ]),
    event(2, PROCESS, "PROCESS_START", [
      property("ObserverProcessId", 101), property("ObserverParentProcessId", 100),
      property("ObserverImageSha256", B),
    ]),
    event(3, AFD, "SOCKET", [
      property("ObserverProcessId", 101), property("ObserverDirection", "outbound"),
      property("ObserverTargetAddress", "127.0.0.1:43100"),
    ]),
    event(4, PROCESS, "PROCESS_STOP", [property("ObserverProcessId", 101)]),
    event(5, PROCESS, "PROCESS_STOP", [property("ObserverProcessId", 100)]),
  ];
  const counts = new Map(REQUIRED_ETW_PROVIDERS.map(({ providerId }) => [providerId, 0]));
  events.forEach(({ providerId }) => counts.set(providerId, counts.get(providerId) + 1));
  const metadata = {
    evidenceKind: "WINDOWS_PHASE2_ETW_PROVIDER_METADATA",
    providers: REQUIRED_ETW_PROVIDERS.map(({ providerId, role }) => ({
      eventDescriptors: [descriptor()],
      manifestEventCount: 1,
      observedEventCount: counts.get(providerId),
      providerId,
      providerName: `Synthetic ${role}`,
      role,
    })),
    schemaVersion: "1.0",
  };
  const files = new Map([
    [NAMES.etl, Buffer.from("synthetic-etl", "utf8")],
    [NAMES.events, Buffer.from(`${events.map((row) => JSON.stringify(row)).join("\n")}\n`, "utf8")],
    [NAMES.metadata, jsonBytes(metadata)],
    [NAMES.transcript, Buffer.from("observer ready\nlauncher completed\ntrace stopped\n", "utf8")],
  ]);
  const fileRow = (path) => ({ bytes: files.get(path).byteLength, path, sha256: hashExternalObserverBytes(files.get(path)) });
  const raw = {
    candidateCommit: COMMIT,
    candidateImageSha256,
    candidateManifestSha256: hashExternalObserverBytes(candidateManifestBytes),
    candidateTree: TREE,
    clockType: "SYSTEM_TIME",
    evidenceKind: "WINDOWS_PHASE2_ETW_RAW_OBSERVER_RUN",
    files: {
      etl: fileRow(NAMES.etl),
      events: fileRow(NAMES.events),
      metadata: fileRow(NAMES.metadata),
      transcript: fileRow(NAMES.transcript),
    },
    interval: {
      launcherExitedAtFileTime: fileTimeForUtc("2026-08-28T03:00:08.000Z"),
      launcherExitedAtUtc: "2026-08-28T03:00:08.000Z",
      launcherStartedAtFileTime: fileTimeForUtc("2026-08-28T03:00:00.900Z"),
      launcherStartedAtUtc: "2026-08-28T03:00:00.900Z",
      providersEnabledAtFileTime: fileTimeForUtc("2026-08-28T03:00:00.500Z"),
      providersEnabledAtUtc: "2026-08-28T03:00:00.500Z",
      startedAtFileTime: fileTimeForUtc("2026-08-28T03:00:00.000Z"),
      startedAtUtc: "2026-08-28T03:00:00.000Z",
      stoppedAtFileTime: fileTimeForUtc("2026-08-28T03:00:10.000Z"),
      stoppedAtUtc: "2026-08-28T03:00:10.000Z",
    },
    launcher: { exitCode: 0, processId: 50 },
    launcherSha256: hashExternalObserverBytes(launcherBytes),
    observerBuildScriptSha256: hashExternalObserverBytes(observerBuildScriptBytes),
    observerExecutableSha256: hashExternalObserverBytes(observerBytes),
    observerSourceSha256: hashExternalObserverBytes(observerSourceBytes),
    observerVersion: EXTERNAL_OBSERVER_VERSION,
    providers: REQUIRED_ETW_PROVIDERS.map(({ providerId, role }) => ({
      enableStatus: 0,
      level: 5,
      matchAllKeyword: "0x0000000000000000",
      matchAnyKeyword: "0xFFFFFFFFFFFFFFFF",
      providerId,
      registered: true,
      role,
    })),
    result: "RAW_CAPTURE_COMPLETE",
    schemaVersion: "1.0",
    sessionId: "20C18B26-8C38-4CC8-BB04-5D6832FC0F01",
    sessionName: EXTERNAL_OBSERVER_SESSION,
    traceStatistics: { buffersWritten: 8, eventsLost: 0, logBuffersLost: 0, realTimeBuffersLost: 0 },
  };
  return {
    candidateImageBytes,
    candidateManifest,
    candidateManifestBytes,
    events,
    files,
    launcherBytes,
    metadata,
    observerAnalyzerSourceBytes,
    observerBuildScriptBytes,
    observerBytes,
    observerFinalizerSourceBytes,
    observerSourceBytes,
    observerSourceVerifierBytes,
    raw,
    rawBytes: jsonBytes(raw),
  };
}

function resyncRaw(input) { input.rawBytes = jsonBytes(input.raw); }
function replaceEvents(input, events) {
  input.events = events;
  input.files.set(NAMES.events, Buffer.from(`${events.map((row) => JSON.stringify(row)).join("\n")}\n`, "utf8"));
  input.raw.files.events = {
    bytes: input.files.get(NAMES.events).byteLength,
    path: NAMES.events,
    sha256: hashExternalObserverBytes(input.files.get(NAMES.events)),
  };
  for (const provider of input.metadata.providers) {
    provider.observedEventCount = events.filter(({ providerId }) => providerId === provider.providerId).length;
  }
  input.files.set(NAMES.metadata, jsonBytes(input.metadata));
  input.raw.files.metadata = {
    bytes: input.files.get(NAMES.metadata).byteLength,
    path: NAMES.metadata,
    sha256: hashExternalObserverBytes(input.files.get(NAMES.metadata)),
  };
  resyncRaw(input);
}

test("accepts exact lossless ETW evidence with complete ancestry and loopback-only activity", () => {
  const input = fixture();
  const result = analyzeExternalObserverEvidence(input);
  assert.equal(result.result, "PASS");
  assert.equal(result.zeroExternalAttempts, true);
  assert.equal(result.externalAttemptCount, 0);
  assert.equal(result.unknownEventCount, 0);
  assert.deepEqual(result.processAncestry.map(({ processId }) => processId), [100, 101]);
  assert.deepEqual(Object.values(result.coverage), [true, true, true, true, true, true]);
});

test("rejects nonzero ETW loss counters", () => {
  const input = fixture();
  input.raw.traceStatistics.eventsLost = 1;
  resyncRaw(input);
  assert.throws(() => analyzeExternalObserverEvidence(input), /not gap-free/u);
});

test("rejects provider enablement after the fixed launcher boundary", () => {
  const input = fixture();
  input.raw.interval.providersEnabledAtFileTime = fileTimeForUtc("2026-08-28T03:00:01.000Z");
  input.raw.interval.providersEnabledAtUtc = "2026-08-28T03:00:01.000Z";
  resyncRaw(input);
  assert.throws(() => analyzeExternalObserverEvidence(input), /not strictly ordered/u);
});

test("rejects missing provider registration, metadata, and event descriptors", () => {
  for (const mutate of [
    (input) => { input.raw.providers.pop(); resyncRaw(input); },
    (input) => {
      input.metadata.providers.pop();
      input.files.set(NAMES.metadata, jsonBytes(input.metadata));
      input.raw.files.metadata = { bytes: input.files.get(NAMES.metadata).byteLength, path: NAMES.metadata, sha256: hashExternalObserverBytes(input.files.get(NAMES.metadata)) };
      resyncRaw(input);
    },
    (input) => {
      input.metadata.providers[0].eventDescriptors = [];
      input.metadata.providers[0].manifestEventCount = 0;
      input.files.set(NAMES.metadata, jsonBytes(input.metadata));
      input.raw.files.metadata = { bytes: input.files.get(NAMES.metadata).byteLength, path: NAMES.metadata, sha256: hashExternalObserverBytes(input.files.get(NAMES.metadata)) };
      resyncRaw(input);
    },
  ]) {
    const input = fixture();
    mutate(input);
    assert.throws(() => analyzeExternalObserverEvidence(input), /provider/u);
  }
});

test("rejects raw ETL, source, executable, and manifest identity drift", () => {
  const mutations = [
    (input) => input.files.set(NAMES.etl, Buffer.from("tampered-etl", "utf8")),
    (input) => { input.observerSourceBytes = Buffer.from("tampered-source", "utf8"); },
    (input) => { input.observerBytes = Buffer.from("tampered-observer", "utf8"); },
    (input) => { input.candidateManifest.gitTree = "c".repeat(40); },
  ];
  for (const mutate of mutations) {
    const input = fixture();
    mutate(input);
    assert.throws(() => analyzeExternalObserverEvidence(input), /drift|bound|identity|bytes/u);
  }
});

test("rejects truncated event streams, PID reuse, and missing descendant teardown", () => {
  const truncated = fixture();
  truncated.files.set(NAMES.events, Buffer.from("{}", "utf8"));
  truncated.raw.files.events = { bytes: 2, path: NAMES.events, sha256: hashExternalObserverBytes(truncated.files.get(NAMES.events)) };
  resyncRaw(truncated);
  assert.throws(() => analyzeExternalObserverEvidence(truncated), /truncated/u);

  const reused = fixture();
  const duplicate = structuredClone(reused.events[0]);
  duplicate.sequence = 5;
  duplicate.timestampFileTime = (BigInt(reused.events[0].timestampFileTime) + 100n).toString();
  const rows = [...reused.events.slice(0, 4), duplicate, ...reused.events.slice(4)]
    .map((row, index) => ({ ...row, sequence: index + 1 }));
  replaceEvents(reused, rows);
  assert.throws(() => analyzeExternalObserverEvidence(reused), /reused/u);

  const teardown = fixture();
  replaceEvents(teardown, teardown.events.filter((row) =>
    !(row.kind === "PROCESS_STOP" && row.properties[0].value === "101"))
    .map((row, index) => ({ ...row, sequence: index + 1 })));
  assert.throws(() => analyzeExternalObserverEvidence(teardown), /teardown/u);
});

test("rejects conflicting network PID attribution", () => {
  const input = fixture();
  input.events[2].headerProcessId = 100;
  replaceEvents(input, input.events);
  assert.throws(() => analyzeExternalObserverEvidence(input), /conflicting/u);
});

test("reports resolver, non-loopback socket, and unknown candidate network events as non-credit", () => {
  const cases = [
    { kind: "DNS_RESOLVER", providerId: "55404E71-4DB9-4DEB-A5F5-8F86E46DDE56", properties: [property("ObserverProcessId", 101)] },
    { kind: "SOCKET", providerId: AFD, properties: [property("ObserverProcessId", 101), property("ObserverDirection", "outbound"), property("ObserverTargetAddress", "192.0.2.10:502")] },
    { kind: "OTHER", providerId: AFD, properties: [property("ObserverProcessId", 101)] },
  ];
  for (const row of cases) {
    const input = fixture();
    input.events[2] = event(3, row.providerId, row.kind, row.properties);
    replaceEvents(input, input.events);
    const result = analyzeExternalObserverEvidence(input);
    assert.equal(result.result, "FAIL");
    assert.equal(result.zeroExternalAttempts, false);
    assert.equal(result.externalAttemptCount + result.unknownEventCount, 1);
  }
});

test("rejects an extra raw field and a normalized kind outside the provider role", () => {
  const extra = fixture();
  extra.raw.operatorClaim = "PASS";
  resyncRaw(extra);
  assert.throws(() => analyzeExternalObserverEvidence(extra), /unrecognized fields/u);

  const wrongKind = fixture();
  wrongKind.events[2].kind = "PROCESS_START";
  replaceEvents(wrongKind, wrongKind.events);
  assert.throws(() => analyzeExternalObserverEvidence(wrongKind), /provider role/u);
});
