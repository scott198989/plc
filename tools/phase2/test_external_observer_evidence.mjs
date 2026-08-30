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
const DNS = "1C95126E-7EEA-49A9-A3FE-A378B03DDB4D";
const NAME_RESOLUTION = "55404E71-4DB9-4DEB-A5F5-8F86E46DDE56";
const PACKET = "7DD42A49-5329-4832-8DFD-43D979153A88";
const AFD = "E53C6823-7BB8-44BB-90DC-3F86090D48A6";
const NAMES = fixedObserverFileNames();
const WINDOWS_TO_UNIX_EPOCH_100NS = 116444736000000000n;
const EVENT_TIME_BASE = BigInt(fileTimeForUtc("2026-08-28T03:00:01.000Z"));

function fileTimeForUtc(utc) {
  return (BigInt(Date.parse(utc)) * 10_000n + WINDOWS_TO_UNIX_EPOCH_100NS).toString();
}

const jsonBytes = (value) => Buffer.from(`${JSON.stringify(value, null, 2)}\n`, "utf8");
const property = (name, value) => ({ name, value: String(value) });
const afdProcessId = (processId) => {
  const bytes = Buffer.alloc(8);
  bytes.writeUInt32LE(processId);
  return bytes.toString("hex").toUpperCase();
};

function event(sequence, providerId, kind, properties, timestamp = BigInt(fileTimeForUtc("2026-08-28T03:00:01.000Z"))) {
  return {
    eventId: kind === "PROCESS_START" ? 1 : kind === "PROCESS_STOP" ? 2 : providerId === PACKET ? 10 : 1000,
    eventName: kind,
    headerProcessId: Number(properties.find(({ name }) => name === "ObserverProcessId")?.value ?? 0),
    headerThreadId: 7,
    keyword: "0x0000000000000010",
    kind,
    level: 5,
    opcode: kind === "PROCESS_STOP" ? 2 : kind === "PROCESS_START" ? 1 : 0,
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

function afdCreateEvent(processId, token, source = true) {
  const row = event(1, AFD, "OTHER", [
    property("AddressFamily", 23), property("Endpoint", "B0D9B1798C93FFFF"),
    property("ObserverProcessId", processId),
    ...(source ? [property("ObserverProcessIdSource", "afd-create-process-id")] : []),
    property("Process", token), property("ProcessId", afdProcessId(processId)),
    property("Protocol", 17), property("SocketType", 2),
  ]);
  row.eventId = 1000;
  return row;
}

function afdSocketEvent(processId, token, target, source = true) {
  const row = event(1, AFD, "SOCKET", [
    property("Address", target), property("Endpoint", "B0D9B1798C93FFFF"),
    property("ObserverDirection", "outbound"), property("ObserverProcessId", processId),
    ...(source ? [property("ObserverProcessIdSource", "afd-process-map")] : []),
    property("Process", token), property("ObserverTargetAddress", target),
  ]);
  row.eventId = 1018;
  return row;
}

function resequenceEvents(events) {
  return events.map((row, index) => ({
    ...row,
    sequence: index + 1,
    timestampFileTime: String(index === 0
      ? EVENT_TIME_BASE - 10_000n
      : EVENT_TIME_BASE + BigInt(index)),
  }));
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

function descriptorForEvent(row) {
  return {
    eventId: row.eventId,
    eventName: row.eventName,
    keyword: row.keyword,
    level: row.level,
    opcode: row.opcode,
    opcodeName: row.opcodeName,
    task: row.task,
    taskName: row.taskName,
    version: row.version,
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
      property("ObserverProcessId", 50), property("ObserverParentProcessId", 40),
      property("ProcessSequenceNumber", 500), property("ParentProcessSequenceNumber", 400),
    ], BigInt(fileTimeForUtc("2026-08-28T03:00:00.999Z"))),
    event(2, PROCESS, "PROCESS_START", [
      property("ObserverProcessId", 100), property("ObserverParentProcessId", 50),
      property("ProcessSequenceNumber", 1000), property("ParentProcessSequenceNumber", 500),
      property("ObserverImageSha256", candidateImageSha256),
    ]),
    event(3, PROCESS, "PROCESS_START", [
      property("ObserverProcessId", 101), property("ObserverParentProcessId", 100),
      property("ProcessSequenceNumber", 1001), property("ParentProcessSequenceNumber", 1000),
      property("ObserverImageSha256", B),
    ]),
    event(4, PACKET, "PACKET", [
      property("ObserverProcessId", 101), property("PID", 101),
      property("ObserverDirection", "outbound"),
      property("ObserverTargetAddress", "127.0.0.1:43100"),
    ]),
    event(5, PROCESS, "PROCESS_STOP", [
      property("ObserverProcessId", 101), property("ProcessSequenceNumber", 1001),
    ]),
    event(6, PROCESS, "PROCESS_STOP", [
      property("ObserverProcessId", 100), property("ProcessSequenceNumber", 1000),
    ]),
    event(7, PROCESS, "PROCESS_STOP", [
      property("ObserverProcessId", 50), property("ProcessSequenceNumber", 500),
    ]),
  ];
  const counts = new Map(REQUIRED_ETW_PROVIDERS.map(({ providerId }) => [providerId, 0]));
  events.forEach(({ providerId }) => counts.set(providerId, counts.get(providerId) + 1));
  const metadata = {
    evidenceKind: "WINDOWS_PHASE2_ETW_PROVIDER_METADATA",
    providers: REQUIRED_ETW_PROVIDERS.map(({ providerId, role }) => {
      const descriptorRows = new Map(events.filter((row) => row.providerId === providerId)
        .map((row) => [`${row.eventId}:${row.version}`, descriptorForEvent(row)]));
      const eventDescriptors = descriptorRows.size === 0 ? [descriptor()] : [...descriptorRows.values()];
      return {
        eventDescriptors,
        manifestEventCount: eventDescriptors.length,
        observedEventCount: counts.get(providerId),
        providerId,
        providerName: `Synthetic ${role}`,
        role,
      };
    }),
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
      launcherStartedAtFileTime: fileTimeForUtc("2026-08-28T03:00:01.000Z"),
      launcherStartedAtUtc: "2026-08-28T03:00:01.000Z",
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
    const providerEvents = events.filter(({ providerId }) => providerId === provider.providerId);
    provider.observedEventCount = providerEvents.length;
    const descriptorRows = new Map(providerEvents
      .map((row) => [`${row.eventId}:${row.version}`, descriptorForEvent(row)]));
    provider.eventDescriptors = descriptorRows.size === 0 ? [descriptor()] : [...descriptorRows.values()];
    provider.manifestEventCount = provider.eventDescriptors.length;
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

test("accepts bounded Windows provider-manifest templates", () => {
  const input = fixture();
  input.metadata.providers[0].eventDescriptors[0].eventName = "W".repeat(1024);
  input.files.set(NAMES.metadata, jsonBytes(input.metadata));
  input.raw.files.metadata = {
    bytes: input.files.get(NAMES.metadata).byteLength,
    path: NAMES.metadata,
    sha256: hashExternalObserverBytes(input.files.get(NAMES.metadata)),
  };
  resyncRaw(input);
  assert.equal(analyzeExternalObserverEvidence(input).result, "PASS");
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

test("rejects an observed event whose stable descriptor tuple drifts from provider metadata", () => {
  const input = fixture();
  const provider = input.metadata.providers.find(({ providerId }) => providerId === PACKET);
  provider.eventDescriptors.find(({ eventId }) => eventId === 10).opcodeName = "Drifted";
  input.files.set(NAMES.metadata, jsonBytes(input.metadata));
  input.raw.files.metadata = {
    bytes: input.files.get(NAMES.metadata).byteLength,
    path: NAMES.metadata,
    sha256: hashExternalObserverBytes(input.files.get(NAMES.metadata)),
  };
  resyncRaw(input);
  assert.throws(() => analyzeExternalObserverEvidence(input), /fixed provider descriptor/u);
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

test("rejects truncated streams, duplicate process instances, and missing descendant teardown", () => {
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
  assert.throws(() => analyzeExternalObserverEvidence(reused), /duplicate start evidence/u);

  const teardown = fixture();
  replaceEvents(teardown, teardown.events.filter((row) =>
    !(row.kind === "PROCESS_STOP" && row.properties[0].value === "101"))
    .map((row, index) => ({ ...row, sequence: index + 1 })));
  assert.throws(() => analyzeExternalObserverEvidence(teardown), /teardown/u);
});

test("rejects a normalized source that conflicts with authoritative NameResolution header attribution", () => {
  const input = fixture();
  input.events[3] = event(4, NAME_RESOLUTION, "DNS_RESOLVER", [
    property("ObserverProcessId", 101),
    property("ObserverProcessIdSource", "name-resolution-header"),
    property("NodeName", "outside.example"),
  ]);
  input.events[3].headerProcessId = 100;
  replaceEvents(input, input.events);
  assert.throws(() => analyzeExternalObserverEvidence(input), /incompatible normalized/u);
});

test("decodes a real-shape AFD create PID and attributes later rows through its opaque process token", () => {
  const input = fixture();
  const token = "80C0D1888C93FFFF";
  const create = event(1, AFD, "OTHER", [
    property("AddressFamily", 23), property("Endpoint", "B0D9B1798C93FFFF"),
    property("ObserverProcessId", 101), property("ObserverProcessIdSource", "afd-create-process-id"),
    property("Process", token), property("ProcessId", afdProcessId(101)),
    property("Protocol", 17), property("SocketType", 2),
  ]);
  create.eventId = 1000;
  const connect = event(1, AFD, "SOCKET", [
    property("Address", "127.0.0.1:43100"), property("Endpoint", "B0D9B1798C93FFFF"),
    property("ObserverDirection", "outbound"), property("ObserverProcessId", 101),
    property("ObserverProcessIdSource", "afd-process-map"), property("Process", token),
    property("ObserverTargetAddress", "127.0.0.1:43100"),
  ]);
  connect.eventId = 1018;
  replaceEvents(input, resequenceEvents([
    ...input.events.slice(0, 3), create, connect, ...input.events.slice(4),
  ]));
  const result = analyzeExternalObserverEvidence(input);
  assert.equal(result.result, "PASS");
  assert.equal(result.accountedNetworkEventCount, 2);
  assert.equal(result.externalAttemptCount, 0);
});

test("keeps AFD token attribution bound to the exact process lifetime across PID and token reuse", () => {
  const input = fixture();
  const token = "80C0D1888C93FFFF";
  const reusedStart = event(1, PROCESS, "PROCESS_START", [
    property("ObserverProcessId", 101), property("ObserverParentProcessId", 900),
    property("ProcessSequenceNumber", 2001), property("ParentProcessSequenceNumber", 9000),
  ]);
  const reusedStop = event(1, PROCESS, "PROCESS_STOP", [
    property("ObserverProcessId", 101), property("ProcessSequenceNumber", 2001),
  ]);
  replaceEvents(input, resequenceEvents([
    ...input.events.slice(0, 3),
    afdCreateEvent(101, token),
    afdSocketEvent(101, token, "127.0.0.1:43100"),
    input.events[4],
    reusedStart,
    afdCreateEvent(101, token),
    afdSocketEvent(101, token, "192.0.2.10:502"),
    reusedStop,
    ...input.events.slice(5),
  ]));

  const result = analyzeExternalObserverEvidence(input);
  assert.equal(result.result, "PASS");
  assert.equal(result.externalAttemptCount, 0);
  assert.equal(result.accountedNetworkEventCount, 2);
});

test("accounts targetless AFD receive and connect-completion rows as passive", () => {
  const input = fixture();
  const token = "80C0D1888C93FFFF";
  const passiveRows = [
    { eventId: 1006, kind: "NETWORK_PASSIVE" },
    { eventId: 1017, kind: "SOCKET" },
  ].map(({ eventId, kind }) => {
    const row = event(1, AFD, kind, [
      property("Endpoint", "B0D9B1798C93FFFF"),
      property("ObserverDirection", "outbound"),
      property("ObserverProcessId", 101),
      property("ObserverProcessIdSource", "afd-process-map"),
      property("Process", token),
    ]);
    row.eventId = eventId;
    return row;
  });
  replaceEvents(input, resequenceEvents([
    ...input.events.slice(0, 3),
    afdCreateEvent(101, token),
    afdSocketEvent(101, token, "127.0.0.1:43100"),
    afdSocketEvent(101, token, "[::1]:43100"),
    ...passiveRows,
    ...input.events.slice(4),
  ]));

  const result = analyzeExternalObserverEvidence(input);
  assert.equal(result.result, "PASS");
  assert.equal(result.externalAttemptCount, 0);
  assert.equal(result.unknownEventCount, 0);
  assert.equal(result.accountedNetworkEventCount, 5);
});

test("rejects AFD token reuse while its prior process instance is still active", () => {
  const input = fixture();
  const token = "80C0D1888C93FFFF";
  replaceEvents(input, resequenceEvents([
    ...input.events.slice(0, 3),
    afdCreateEvent(101, token),
    afdCreateEvent(100, token),
    ...input.events.slice(4),
  ]));
  assert.throws(() => analyzeExternalObserverEvidence(input), /ambiguously reuses.*process token/u);
});

test("rejects AFD token rebinding when the prior owner lifetime is unavailable", () => {
  const input = fixture();
  const token = "80C0D1888C93FFFF";
  replaceEvents(input, resequenceEvents([
    ...input.events.slice(0, 3),
    afdCreateEvent(900, token),
    afdCreateEvent(101, token),
    ...input.events.slice(4),
  ]));
  assert.throws(() => analyzeExternalObserverEvidence(input), /ambiguously reuses.*process token/u);
});

test("fails closed for malformed AFD tokens, little-endian PIDs, and forged unbound mappings", () => {
  const malformedRows = [
    (() => {
      const row = afdCreateEvent(101, "80C0D1888C93FFFF");
      row.properties.find(({ name }) => name === "Process").value = "NOT-A-TOKEN";
      return row;
    })(),
    (() => {
      const row = afdCreateEvent(101, "80C0D1888C93FFFF");
      row.properties.find(({ name }) => name === "ProcessId").value = "6500000001000000";
      return row;
    })(),
    afdSocketEvent(101, "80C0D1888C93FFFF", "127.0.0.1:43100"),
  ];
  for (const malformed of malformedRows) {
    const input = fixture();
    replaceEvents(input, resequenceEvents([
      ...input.events.slice(0, 3), malformed, ...input.events.slice(4),
    ]));
    assert.throws(() => analyzeExternalObserverEvidence(input), /AFD|incompatible normalized/u);
  }
});

test("uses provider payload attribution for packet and DNS service events", () => {
  const packet = fixture();
  packet.events[3] = event(4, PACKET, "NETWORK_PASSIVE", [
    property("ObserverProcessId", 101), property("ObserverProcessIdSource", "kernel-network-pid"),
    property("PID", 101),
  ]);
  packet.events[3].headerProcessId = 777;
  replaceEvents(packet, packet.events);
  const packetResult = analyzeExternalObserverEvidence(packet);
  assert.equal(packetResult.result, "FAIL");
  assert.equal(packetResult.unknownEventCount, 1);

  const dns = fixture();
  dns.events[3] = event(4, DNS, "DNS_RESOLVER", [
    property("ClientPID", 101), property("ObserverProcessId", 101),
    property("ObserverProcessIdSource", "dns-client-pid"), property("QueryBlob", "50A0918475010000"),
    property("QueryName", "outside.example"),
  ]);
  dns.events[3].eventId = 3009;
  dns.events[3].headerProcessId = 2700;
  replaceEvents(dns, dns.events);
  const dnsResult = analyzeExternalObserverEvidence(dns);
  assert.equal(dnsResult.result, "FAIL");
  assert.equal(dnsResult.externalAttemptCount, 1);
});

test("uses DNS header fallback only when ClientPID is absent and uses NameResolution header attribution", () => {
  const dns = fixture();
  dns.events[3] = event(4, DNS, "DNS_RESOLVER", [
    property("ObserverProcessId", 101),
    property("ObserverProcessIdSource", "dns-client-header-fallback"),
    property("QueryName", "govs-plc.local"),
  ]);
  dns.events[3].eventId = 3006;
  dns.events[3].headerProcessId = 101;
  replaceEvents(dns, dns.events);
  assert.equal(analyzeExternalObserverEvidence(dns).result, "PASS");

  const nameResolution = fixture();
  nameResolution.events[3] = event(4, NAME_RESOLUTION, "DNS_RESOLVER", [
    property("ObserverProcessId", 101),
    property("ObserverProcessIdSource", "name-resolution-header"),
    property("NodeName", "localhost"),
  ]);
  nameResolution.events[3].eventId = 1000;
  nameResolution.events[3].headerProcessId = 101;
  replaceEvents(nameResolution, nameResolution.events);
  assert.equal(analyzeExternalObserverEvidence(nameResolution).result, "PASS");
});

test("rejects malformed provider PIDs instead of falling back to worker-thread headers", () => {
  for (const [providerId, pidName, pidValue, eventId] of [
    [PACKET, "PID", "0x65", 10],
    [DNS, "ClientPID", "not-decimal", 3009],
  ]) {
    const input = fixture();
    input.events[3] = event(4, providerId, providerId === PACKET ? "PACKET" : "DNS_RESOLVER", [
      property(pidName, pidValue),
      ...(providerId === PACKET
        ? [property("ObserverDirection", "outbound"), property("ObserverTargetAddress", "127.0.0.1:43100")]
        : [property("QueryName", "govs-plc.local"), property("QueryBlob", "50A0918475010000")]),
    ]);
    input.events[3].eventId = eventId;
    input.events[3].headerProcessId = 101;
    replaceEvents(input, input.events);
    assert.throws(() => analyzeExternalObserverEvidence(input), /payload PID|client PID/u);
  }
});

test("accounts local resolver and lifecycle rows while deduplicating external resolver observations", () => {
  const input = fixture();
  const rows = [];
  for (const target of ["localhost", "127.0.0.1", "govs-plc.local"]) {
    const local = event(1, NAME_RESOLUTION, "DNS_RESOLVER", [
      property("ObserverProcessId", 101),
      property("ObserverProcessIdSource", "name-resolution-header"),
      property("NodeName", target),
    ]);
    local.eventId = 1000;
    rows.push(local);
  }
  const firstExternal = event(1, NAME_RESOLUTION, "DNS_RESOLVER", [
    property("ObserverProcessId", 101),
    property("ObserverProcessIdSource", "name-resolution-header"),
    property("NodeName", "outside.example"),
  ]);
  firstExternal.eventId = 1000;
  rows.push(firstExternal);
  const duplicateExternal = event(1, NAME_RESOLUTION, "DNS_RESOLVER", [
    property("ObserverProcessId", 101),
    property("ObserverProcessIdSource", "name-resolution-header"),
    property("QueryName", "outside.example"),
  ]);
  duplicateExternal.eventId = 1006;
  rows.push(duplicateExternal);
  const serviceDuplicate = event(1, DNS, "DNS_RESOLVER", [
    property("ClientPID", 101), property("ObserverProcessId", 101),
    property("ObserverProcessIdSource", "dns-client-pid"),
    property("QueryBlob", "50A0918475010000"), property("QueryName", "outside.example"),
  ]);
  serviceDuplicate.eventId = 3009;
  serviceDuplicate.headerProcessId = 2700;
  rows.push(serviceDuplicate);
  const completion = event(1, NAME_RESOLUTION, "NETWORK_PASSIVE", [
    property("ObserverProcessId", 101),
    property("ObserverProcessIdSource", "name-resolution-header"),
    property("NodeName", "outside.example"), property("Status", 0),
  ]);
  completion.eventId = 1001;
  rows.push(completion);
  const startup = event(1, NAME_RESOLUTION, "DNS_RESOLVER", [
    property("ObserverProcessId", 101),
    property("ObserverProcessIdSource", "name-resolution-header"), property("Location", 117),
  ]);
  startup.eventId = 1013;
  rows.push(startup);
  replaceEvents(input, resequenceEvents([
    ...input.events.slice(0, 3), ...rows, ...input.events.slice(4),
  ]));

  const result = analyzeExternalObserverEvidence(input);
  assert.equal(result.result, "FAIL");
  assert.equal(result.externalAttemptCount, 1);
  assert.equal(result.unknownEventCount, 0);
  assert.equal(result.externalAttempts[0].target, "outside.example");
});

test("fails closed for an unsupported resolver event descriptor", () => {
  const input = fixture();
  input.events[3] = event(4, NAME_RESOLUTION, "DNS_RESOLVER", [
    property("ObserverProcessId", 101),
    property("ObserverProcessIdSource", "name-resolution-header"),
    property("NodeName", "outside.example"),
  ]);
  input.events[3].eventId = 65000;
  replaceEvents(input, input.events);
  const result = analyzeExternalObserverEvidence(input);
  assert.equal(result.result, "FAIL");
  assert.equal(result.externalAttemptCount, 0);
  assert.equal(result.unknownEventCount, 1);
  assert.equal(result.unknownEvents[0].reason, "unsupported-resolver-event-schema");
});

test("rejects conflicting normalized and provider network PID attribution", () => {
  for (const [providerId, name, source] of [
    [PACKET, "PID", "kernel-network-pid"],
    [DNS, "ClientPID", "dns-client-pid"],
  ]) {
    const input = fixture();
    input.events[3] = event(4, providerId, "NETWORK_PASSIVE", [
      property("ObserverProcessId", 101), property("ObserverProcessIdSource", source), property(name, 102),
    ]);
    input.events[3].headerProcessId = 777;
    replaceEvents(input, input.events);
    assert.throws(() => analyzeExternalObserverEvidence(input), /incompatible normalized/u);
  }
});

test("reports resolver, non-loopback socket, and unknown candidate network events as non-credit", () => {
  const cases = [
    { eventId: 1000, kind: "DNS_RESOLVER", providerId: NAME_RESOLUTION, properties: [property("ObserverProcessId", 101), property("ObserverProcessIdSource", "name-resolution-header"), property("NodeName", "outside.example")] },
    { eventId: 10, kind: "PACKET", providerId: PACKET, properties: [property("ObserverProcessId", 101), property("ObserverProcessIdSource", "kernel-network-pid"), property("PID", 101), property("ObserverDirection", "outbound"), property("ObserverTargetAddress", "192.0.2.10:502")] },
    { eventId: 999, kind: "OTHER", providerId: PACKET, properties: [property("ObserverProcessId", 101), property("ObserverProcessIdSource", "kernel-network-pid"), property("PID", 101)] },
  ];
  for (const row of cases) {
    const input = fixture();
    input.events[3] = event(4, row.providerId, row.kind, row.properties);
    input.events[3].eventId = row.eventId;
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
  wrongKind.events[3].kind = "PROCESS_START";
  replaceEvents(wrongKind, wrongKind.events);
  assert.throws(() => analyzeExternalObserverEvidence(wrongKind), /provider role/u);
});

test("allows unrelated PID reuse and excludes network events outside the candidate instance lifetime", () => {
  const input = fixture();
  const reusedStart = event(1, PROCESS, "PROCESS_START", [
    property("ObserverProcessId", 101), property("ObserverParentProcessId", 900),
    property("ProcessSequenceNumber", 2001), property("ParentProcessSequenceNumber", 9000),
  ]);
  const unrelatedNetwork = event(1, PACKET, "PACKET", [
    property("ObserverProcessId", 101), property("PID", 101), property("ObserverDirection", "outbound"),
    property("ObserverTargetAddress", "192.0.2.10:502"),
  ]);
  const reusedStop = event(1, PROCESS, "PROCESS_STOP", [
    property("ObserverProcessId", 101), property("ProcessSequenceNumber", 2001),
  ]);
  replaceEvents(input, resequenceEvents([
    ...input.events.slice(0, 5),
    reusedStart,
    unrelatedNetwork,
    reusedStop,
    ...input.events.slice(5),
  ]));

  const result = analyzeExternalObserverEvidence(input);
  assert.equal(result.result, "PASS");
  assert.equal(result.externalAttemptCount, 0);
  assert.equal(result.accountedNetworkEventCount, 1);
  assert.deepEqual(result.processAncestry.map(({ processId }) => processId), [100, 101]);
});

test("rejects overlapping PID instances and conflicting candidate parent sequence evidence", () => {
  const overlapping = fixture();
  const overlappingStart = event(1, PROCESS, "PROCESS_START", [
    property("ObserverProcessId", 101), property("ObserverParentProcessId", 900),
    property("ProcessSequenceNumber", 2001), property("ParentProcessSequenceNumber", 9000),
  ]);
  replaceEvents(overlapping, resequenceEvents([
    ...overlapping.events.slice(0, 4), overlappingStart, ...overlapping.events.slice(4),
  ]));
  assert.throws(() => analyzeExternalObserverEvidence(overlapping), /overlapping process instances/u);

  const conflictingParent = fixture();
  const parentSequence = conflictingParent.events[2].properties
    .find(({ name }) => name === "ParentProcessSequenceNumber");
  parentSequence.value = "9999";
  replaceEvents(conflictingParent, conflictingParent.events);
  assert.throws(() => analyzeExternalObserverEvidence(conflictingParent), /conflicting parent sequence/u);
});

test("rejects missing process-sequence evidence and ambiguous lifetime-bound network attribution", () => {
  const missing = fixture();
  missing.events[1].properties = missing.events[1].properties
    .filter(({ name }) => name !== "ProcessSequenceNumber");
  replaceEvents(missing, missing.events);
  assert.throws(() => analyzeExternalObserverEvidence(missing), /sequence number is missing or malformed/u);

  const ambiguous = fixture();
  const firstStop = ambiguous.events[4];
  const reusedStart = event(1, PROCESS, "PROCESS_START", [
    property("ObserverProcessId", 101), property("ObserverParentProcessId", 100),
    property("ProcessSequenceNumber", 2001), property("ParentProcessSequenceNumber", 1000),
    property("ObserverImageSha256", B),
  ]);
  const boundaryNetwork = event(1, PACKET, "PACKET", [
    property("ObserverProcessId", 101), property("PID", 101), property("ObserverDirection", "outbound"),
    property("ObserverTargetAddress", "127.0.0.1:43100"),
  ]);
  const reusedStop = event(1, PROCESS, "PROCESS_STOP", [
    property("ObserverProcessId", 101), property("ProcessSequenceNumber", 2001),
  ]);
  const rows = resequenceEvents([
    ...ambiguous.events.slice(0, 5), reusedStart, boundaryNetwork, reusedStop, ...ambiguous.events.slice(5),
  ]);
  const sharedBoundary = firstStop.timestampFileTime;
  rows[4].timestampFileTime = sharedBoundary;
  rows[5].timestampFileTime = sharedBoundary;
  rows[6].timestampFileTime = sharedBoundary;
  replaceEvents(ambiguous, rows);
  assert.throws(() => analyzeExternalObserverEvidence(ambiguous), /ambiguous candidate process lifetime/u);
});

test("rejects conflicting stop sequences and ancestry outside the parent process lifetime", () => {
  const conflictingStop = fixture();
  const wrongStop = event(1, PROCESS, "PROCESS_STOP", [
    property("ObserverProcessId", 101), property("ProcessSequenceNumber", 9999),
  ]);
  replaceEvents(conflictingStop, resequenceEvents([
    ...conflictingStop.events.slice(0, 4), wrongStop, ...conflictingStop.events.slice(4),
  ]));
  assert.throws(() => analyzeExternalObserverEvidence(conflictingStop), /stop sequence conflicts/u);

  const outsideParent = fixture();
  replaceEvents(outsideParent, resequenceEvents([
    outsideParent.events[0],
    outsideParent.events[1],
    outsideParent.events[5],
    outsideParent.events[2],
    outsideParent.events[4],
    outsideParent.events[6],
  ]));
  assert.throws(() => analyzeExternalObserverEvidence(outsideParent), /outside its parent process lifetime/u);
});

test("fails closed for unordered or tied process lifecycle evidence", () => {
  const unordered = fixture();
  const deliveryOrder = [
    unordered.events[0],
    unordered.events[5],
    unordered.events[1],
    unordered.events[2],
    unordered.events[3],
    unordered.events[4],
    unordered.events[6],
  ].map((row, index) => ({ ...row, sequence: index + 1 }));
  replaceEvents(unordered, deliveryOrder);
  assert.throws(() => analyzeExternalObserverEvidence(unordered), /started after stop evidence/u);

  const tied = fixture();
  tied.events[4].timestampFileTime = tied.events[2].timestampFileTime;
  replaceEvents(tied, tied.events);
  assert.throws(() => analyzeExternalObserverEvidence(tied), /stopped before it started/u);
});
