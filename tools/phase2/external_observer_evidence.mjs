import { createHash } from "node:crypto";
import { isIP } from "node:net";

export const EXTERNAL_OBSERVER_VERSION = "govs-p2-windows-etw-observer-v1";
export const EXTERNAL_OBSERVER_SESSION = "GovsPLC-Phase2-External-Observer-v1";

export const REQUIRED_ETW_PROVIDERS = Object.freeze([
  Object.freeze({
    providerId: "1C95126E-7EEA-49A9-A3FE-A378B03DDB4D",
    role: "dns-client",
  }),
  Object.freeze({
    providerId: "22FB2CD6-0E7B-422B-A0C7-2FAD1FD0E716",
    role: "process-ancestry",
  }),
  Object.freeze({
    providerId: "55404E71-4DB9-4DEB-A5F5-8F86E46DDE56",
    role: "resolver-api",
  }),
  Object.freeze({
    providerId: "7DD42A49-5329-4832-8DFD-43D979153A88",
    role: "packet",
  }),
  Object.freeze({
    providerId: "E53C6823-7BB8-44BB-90DC-3F86090D48A6",
    role: "endpoint-socket",
  }),
]);

const SHA256 = /^[A-F0-9]{64}$/u;
const GIT_OBJECT = /^[a-f0-9]{40}$/u;
const GUID = /^[A-F0-9]{8}-[A-F0-9]{4}-[A-F0-9]{4}-[A-F0-9]{4}-[A-F0-9]{12}$/u;
const DECIMAL_FILETIME = /^[1-9][0-9]{16,19}$/u;
const ISO_UTC = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/u;
const WINDOWS_TO_UNIX_EPOCH_100NS = 116444736000000000n;
const SAFE_NAME = /^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/u;
// Windows provider manifests contain descriptive event templates, not merely
// short identifiers. Keep them bounded without rejecting valid OS metadata.
const MAX_MANIFEST_DESCRIPTOR_TEXT = 4096;
const NETWORK_PROVIDER_IDS = new Set(REQUIRED_ETW_PROVIDERS
  .filter(({ role }) => role !== "process-ancestry")
  .map(({ providerId }) => providerId));
const PROCESS_PROVIDER_ID = REQUIRED_ETW_PROVIDERS
  .find(({ role }) => role === "process-ancestry").providerId;
const DNS_CLIENT_PROVIDER_ID = REQUIRED_ETW_PROVIDERS
  .find(({ role }) => role === "dns-client").providerId;
const NAME_RESOLUTION_PROVIDER_ID = REQUIRED_ETW_PROVIDERS
  .find(({ role }) => role === "resolver-api").providerId;
const PACKET_PROVIDER_ID = REQUIRED_ETW_PROVIDERS
  .find(({ role }) => role === "packet").providerId;
const AFD_PROVIDER_ID = REQUIRED_ETW_PROVIDERS
  .find(({ role }) => role === "endpoint-socket").providerId;
const PROVIDER_ROLES = new Map(REQUIRED_ETW_PROVIDERS.map(({ providerId, role }) => [providerId, role]));
const EVENT_KINDS = new Set([
  "DNS_RESOLVER",
  "NETWORK_PASSIVE",
  "OTHER",
  "PACKET",
  "PROCESS_START",
  "PROCESS_STOP",
  "SOCKET",
]);
const DNS_CLIENT_INITIATION_EVENT_IDS = new Set([3006, 3009, 3010, 3012, 3019]);
const DNS_CLIENT_PASSIVE_EVENT_IDS = new Set([
  1001, 1015, 1016, 3008, 3011, 3013, 3014, 3016, 3018, 3020,
]);
const NAME_RESOLUTION_INITIATION_EVENT_IDS = new Set([1000, 1002, 1006]);
const NAME_RESOLUTION_PASSIVE_EVENT_IDS = new Set([
  1001, 1003, 1004, 1007, 1008, 1009, 1010, 1011, 1012, 1013, 1014,
]);
const PACKET_OUTBOUND_EVENT_IDS = new Set([10, 12, 42, 58]);
const PACKET_PASSIVE_EVENT_IDS = new Set([11, 13, 15, 18, 43, 59]);
const AFD_LIFECYCLE_EVENT_IDS = new Set([1001, 1002, 1032, 1035, 3006]);
const AFD_OUTBOUND_EVENT_IDS = new Set([1003, 1007, 1013, 1018, 1021]);
const AFD_DIRECT_REMOTE_EVENT_IDS = new Set([1007, 1013, 1018, 1021]);
const AFD_PASSIVE_EVENT_IDS = new Set([
  1004, 1006, 1009, 1012, 1015, 1017, 1020, 1023, 1024, 1026,
  1027, 1036, 3001, 3003, 3004,
]);
const AFD_BIND_EVENT_ID = 1030;
const FIXED_FILES = Object.freeze({
  etl: "native-gap-free-external-events.etl",
  events: "native-gap-free-external-events.jsonl",
  metadata: "native-gap-free-external-provider-metadata.json",
  transcript: "native-gap-free-external-observer-transcript.log",
});

const fail = (message) => { throw new Error(message); };
const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex").toUpperCase();
const exactKeys = (value, keys) => value !== null && typeof value === "object" &&
  !Array.isArray(value) && Object.keys(value).sort().join("\0") === [...keys].sort().join("\0");
const requireCondition = (condition, message) => { if (!condition) fail(message); };
const stableSort = (values, selector) => [...values].sort((left, right) =>
  selector(left).localeCompare(selector(right), "en"));

function requireFileRow(row, expectedPath, files, label) {
  requireCondition(
    exactKeys(row, ["bytes", "path", "sha256"]) && row.path === expectedPath &&
    Number.isSafeInteger(row.bytes) && row.bytes > 0 && SHA256.test(row.sha256),
    `${label} file row is malformed.`,
  );
  const content = files.get(expectedPath);
  const actual = Buffer.isBuffer(content)
    ? { bytes: content.byteLength, sha256: sha256(content) }
    : content;
  requireCondition(exactKeys(actual, ["bytes", "sha256"]) && actual.bytes === row.bytes &&
    actual.sha256 === row.sha256, `${label} bytes are missing or hash-drifted.`);
}

function contentSha256(files, name) {
  const content = files.get(name);
  return Buffer.isBuffer(content) ? sha256(content) : content?.sha256;
}

function requireCandidateIdentity(candidateManifest, candidateManifestBytes, candidateImageBytes, raw, sourceBytes) {
  requireCondition(
    candidateManifest !== null && typeof candidateManifest === "object" &&
    candidateManifest.schemaVersion === "1.0" &&
    candidateManifest.evidenceKind === "WINDOWS_NATIVE_CANDIDATE_PACKAGE_MANIFEST" &&
    candidateManifest.gitCommit === raw.candidateCommit && candidateManifest.gitTree === raw.candidateTree &&
    candidateManifest.developmentDirty === false && Array.isArray(candidateManifest.packageFiles) &&
    Array.isArray(candidateManifest.sourceInputs) &&
    sha256(candidateManifestBytes) === raw.candidateManifestSha256,
    "The ETW run is not bound to a clean exact candidate manifest.",
  );
  const shellRows = candidateManifest.packageFiles.filter((row) => row?.path === "GovsPLC.exe");
  requireCondition(shellRows.length === 1 && exactKeys(shellRows[0], ["bytes", "path", "sha256"]) &&
    shellRows[0].bytes === candidateImageBytes.byteLength &&
    shellRows[0].sha256 === raw.candidateImageSha256 &&
    sha256(candidateImageBytes) === raw.candidateImageSha256,
  "The ETW run candidate executable identity is incomplete or drifted.");
  const expectedSources = new Map([
    ["tools/phase2/build_external_observer.mjs", sourceBytes.observerBuildScriptBytes],
    ["tools/phase2/external_observer_evidence.mjs", sourceBytes.observerAnalyzerSourceBytes],
    ["tools/phase2/finalize_external_observer_evidence.mjs", sourceBytes.observerFinalizerSourceBytes],
    ["tools/phase2/verify_external_observer_source.mjs", sourceBytes.observerSourceVerifierBytes],
    ["tools/phase2/windows_external_observer.cpp", sourceBytes.observerSourceBytes],
  ]);
  for (const [sourcePath, bytes] of expectedSources) {
    const rows = candidateManifest.sourceInputs.filter((row) => row?.path === sourcePath);
    requireCondition(Buffer.isBuffer(bytes) && rows.length === 1 &&
      exactKeys(rows[0], ["bytes", "path", "sha256"]) && rows[0].bytes === bytes.byteLength &&
      rows[0].sha256 === sha256(bytes), `Exact candidate manifest does not bind observer source ${sourcePath}.`);
  }
}

function validateRawManifest(raw, files, identities) {
  requireCondition(exactKeys(raw, [
    "candidateCommit", "candidateImageSha256", "candidateManifestSha256", "candidateTree",
    "clockType", "evidenceKind", "files", "interval", "launcher", "launcherSha256",
    "observerBuildScriptSha256", "observerExecutableSha256", "observerSourceSha256",
    "observerVersion", "providers", "result", "schemaVersion", "sessionId", "sessionName",
    "traceStatistics",
  ]), "The raw ETW observer manifest contains missing or unrecognized fields.");
  requireCondition(
    raw.schemaVersion === "1.0" && raw.evidenceKind === "WINDOWS_PHASE2_ETW_RAW_OBSERVER_RUN" &&
    raw.observerVersion === EXTERNAL_OBSERVER_VERSION && raw.sessionName === EXTERNAL_OBSERVER_SESSION &&
    raw.result === "RAW_CAPTURE_COMPLETE" && raw.clockType === "SYSTEM_TIME" &&
    GIT_OBJECT.test(raw.candidateCommit) && GIT_OBJECT.test(raw.candidateTree) &&
    [raw.candidateImageSha256, raw.candidateManifestSha256, raw.launcherSha256,
      raw.observerBuildScriptSha256, raw.observerExecutableSha256, raw.observerSourceSha256]
      .every((value) => SHA256.test(value)) && GUID.test(raw.sessionId),
    "The raw ETW observer identity is malformed or incomplete.",
  );
  requireCondition(exactKeys(raw.interval, [
    "launcherExitedAtFileTime", "launcherExitedAtUtc", "launcherStartedAtFileTime",
    "launcherStartedAtUtc", "providersEnabledAtFileTime", "providersEnabledAtUtc",
    "startedAtFileTime", "startedAtUtc", "stoppedAtFileTime", "stoppedAtUtc",
  ]), "The ETW interval schema is malformed.");
  const timePairs = ["started", "providersEnabled", "launcherStarted", "launcherExited", "stopped"];
  const fileTimes = [];
  for (const prefix of timePairs) {
    const fileTime = raw.interval[`${prefix}AtFileTime`];
    const utc = raw.interval[`${prefix}AtUtc`];
    requireCondition(DECIMAL_FILETIME.test(fileTime) && ISO_UTC.test(utc),
      `The ${prefix} ETW boundary is malformed.`);
    const milliseconds = (BigInt(fileTime) - WINDOWS_TO_UNIX_EPOCH_100NS) / 10_000n;
    requireCondition(milliseconds >= 0n && new Date(Number(milliseconds)).toISOString() === utc,
      `The ${prefix} ETW FILETIME and UTC boundary disagree.`);
    fileTimes.push(BigInt(fileTime));
  }
  requireCondition(fileTimes.every((value, index) => index === 0 || value > fileTimes[index - 1]),
    "The ETW observation boundaries are not strictly ordered.");
  requireCondition(exactKeys(raw.launcher, ["exitCode", "processId"]) &&
    raw.launcher.exitCode === 0 && Number.isSafeInteger(raw.launcher.processId) &&
    raw.launcher.processId > 0, "The fixed native launcher did not complete successfully.");
  requireCondition(exactKeys(raw.traceStatistics, [
    "buffersWritten", "eventsLost", "logBuffersLost", "realTimeBuffersLost",
  ]) && Number.isSafeInteger(raw.traceStatistics.buffersWritten) &&
    raw.traceStatistics.buffersWritten > 0 && raw.traceStatistics.eventsLost === 0 &&
    raw.traceStatistics.logBuffersLost === 0 && raw.traceStatistics.realTimeBuffersLost === 0,
  "The ETW capture is not gap-free: a loss counter is nonzero or missing.");
  requireCondition(exactKeys(raw.files, ["etl", "events", "metadata", "transcript"]),
    "The ETW raw-file inventory is malformed.");
  for (const [key, expectedPath] of Object.entries(FIXED_FILES)) {
    requireFileRow(raw.files[key], expectedPath, files, `ETW ${key}`);
  }
  requireCondition(Buffer.isBuffer(identities.launcherBytes) && sha256(identities.launcherBytes) === raw.launcherSha256,
    "The fixed launcher executable hash drifted.");
  requireCondition(Buffer.isBuffer(identities.observerBytes) && sha256(identities.observerBytes) === raw.observerExecutableSha256,
    "The ETW observer executable hash drifted.");
  requireCondition(Buffer.isBuffer(identities.observerSourceBytes) && sha256(identities.observerSourceBytes) === raw.observerSourceSha256,
    "The ETW observer source hash drifted.");
  requireCondition(Buffer.isBuffer(identities.observerBuildScriptBytes) &&
    sha256(identities.observerBuildScriptBytes) === raw.observerBuildScriptSha256,
  "The ETW observer build-script hash drifted.");
  const expectedProviders = stableSort(REQUIRED_ETW_PROVIDERS, ({ providerId }) => providerId);
  requireCondition(Array.isArray(raw.providers) && raw.providers.length === expectedProviders.length,
    "The ETW provider-enable inventory is incomplete.");
  const observedProviders = stableSort(raw.providers, ({ providerId }) => String(providerId));
  observedProviders.forEach((row, index) => {
    const expected = expectedProviders[index];
    requireCondition(exactKeys(row, [
      "enableStatus", "level", "matchAllKeyword", "matchAnyKeyword", "providerId", "registered", "role",
    ]) && row.providerId === expected.providerId && row.role === expected.role &&
      row.registered === true && row.enableStatus === 0 && row.level === 5 &&
      /^0x[A-F0-9]{16}$/u.test(row.matchAnyKeyword) && /^0x[A-F0-9]{16}$/u.test(row.matchAllKeyword),
    `Required ETW provider ${expected.providerId} was not registered and enabled exactly.`);
  });
  return { ended: fileTimes[4], started: fileTimes[0] };
}

function validateProviderMetadata(metadata, events) {
  requireCondition(exactKeys(metadata, ["evidenceKind", "providers", "schemaVersion"]) &&
    metadata.schemaVersion === "1.0" &&
    metadata.evidenceKind === "WINDOWS_PHASE2_ETW_PROVIDER_METADATA" &&
    Array.isArray(metadata.providers), "ETW provider metadata is malformed.");
  const expected = stableSort(REQUIRED_ETW_PROVIDERS, ({ providerId }) => providerId);
  const observed = stableSort(metadata.providers, ({ providerId }) => String(providerId));
  requireCondition(observed.length === expected.length, "ETW provider metadata is incomplete.");
  const eventCounts = new Map(REQUIRED_ETW_PROVIDERS.map(({ providerId }) => [providerId, 0]));
  const descriptorInventory = new Map();
  for (const event of events) eventCounts.set(event.providerId, (eventCounts.get(event.providerId) ?? 0) + 1);
  observed.forEach((row, index) => {
    const expectedRow = expected[index];
    requireCondition(exactKeys(row, [
      "eventDescriptors", "manifestEventCount", "observedEventCount", "providerId", "providerName", "role",
    ]) && row.providerId === expectedRow.providerId && row.role === expectedRow.role &&
      typeof row.providerName === "string" && row.providerName.length > 0 && row.providerName.length <= 256 &&
      Number.isSafeInteger(row.manifestEventCount) && row.manifestEventCount > 0 &&
      Number.isSafeInteger(row.observedEventCount) && row.observedEventCount === eventCounts.get(row.providerId) &&
      Array.isArray(row.eventDescriptors) && row.eventDescriptors.length === row.manifestEventCount,
    `ETW provider metadata for ${expectedRow.providerId} is incomplete or inconsistent.`);
    const descriptorKeys = new Set();
    const descriptors = new Map();
    for (const descriptor of row.eventDescriptors) {
      requireCondition(exactKeys(descriptor, [
        "eventId", "eventName", "keyword", "level", "opcode", "opcodeName", "task", "taskName", "version",
      ]) && Number.isSafeInteger(descriptor.eventId) && descriptor.eventId >= 0 && descriptor.eventId <= 65_535 &&
        Number.isSafeInteger(descriptor.version) && descriptor.version >= 0 && descriptor.version <= 255 &&
        Number.isSafeInteger(descriptor.level) && descriptor.level >= 0 && descriptor.level <= 255 &&
        Number.isSafeInteger(descriptor.opcode) && descriptor.opcode >= 0 && descriptor.opcode <= 255 &&
        Number.isSafeInteger(descriptor.task) && descriptor.task >= 0 && descriptor.task <= 65_535 &&
        /^0x[A-F0-9]{16}$/u.test(descriptor.keyword) &&
        [descriptor.eventName, descriptor.opcodeName, descriptor.taskName].every((name) =>
          typeof name === "string" && name.length <= MAX_MANIFEST_DESCRIPTOR_TEXT),
      `ETW provider ${row.providerId} contains malformed event metadata.`);
      const key = `${descriptor.eventId}:${descriptor.version}`;
      requireCondition(!descriptorKeys.has(key), `ETW provider ${row.providerId} repeats event metadata ${key}.`);
      descriptorKeys.add(key);
      descriptors.set(key, descriptor);
    }
    descriptorInventory.set(row.providerId, descriptors);
  });
  for (const event of events) {
    const descriptor = descriptorInventory.get(event.providerId)?.get(`${event.eventId}:${event.version}`);
    // Winsock-AFD adds runtime keyword bits and changes level/opcode for the
    // enter/exit or status phase of an otherwise identical manifest event.
    // Those runtime fields are schema-validated in parseEvents. The AFD stable
    // identity is provider + ID/version/task and manifest names; providers with
    // stable runtime descriptors must match level/opcode/opcodeName as well.
    const afdRuntimeDescriptor = event.providerId === AFD_PROVIDER_ID;
    requireCondition(descriptor !== undefined &&
      descriptor.eventName === event.eventName &&
      descriptor.task === event.task &&
      descriptor.taskName === event.taskName,
    `ETW event ${event.sequence} does not match its fixed provider descriptor.`);
    requireCondition(afdRuntimeDescriptor ||
      descriptor.level === event.level && descriptor.opcode === event.opcode &&
      descriptor.opcodeName === event.opcodeName,
    `ETW event ${event.sequence} does not match its fixed provider descriptor.`);
  }
  return observed.map(({ eventDescriptors: _eventDescriptors, ...row }) => row);
}

function parseEvents(bytes, interval) {
  let text;
  try { text = new TextDecoder("utf-8", { fatal: true }).decode(bytes); } catch {
    fail("The normalized ETW event stream is not valid UTF-8.");
  }
  requireCondition(text.endsWith("\n"), "The normalized ETW event stream is truncated.");
  const lines = text.slice(0, -1).split("\n");
  requireCondition(lines.length > 0 && lines.every((line) => line.length > 0 && line.length <= 256 * 1024),
    "The normalized ETW event stream is empty or contains an invalid line.");
  const events = lines.map((line, index) => {
    let row;
    try { row = JSON.parse(line); } catch { fail(`Normalized ETW event ${index + 1} is malformed JSON.`); }
    requireCondition(exactKeys(row, [
      "eventId", "eventName", "headerProcessId", "headerThreadId", "keyword", "kind", "level",
      "opcode", "opcodeName", "properties", "providerId", "sequence", "task", "taskName",
      "timestampFileTime", "version",
    ]), `Normalized ETW event ${index + 1} contains missing or unrecognized fields.`);
    requireCondition(row.sequence === index + 1 && GUID.test(row.providerId) &&
      REQUIRED_ETW_PROVIDERS.some(({ providerId }) => providerId === row.providerId) &&
      DECIMAL_FILETIME.test(row.timestampFileTime) && BigInt(row.timestampFileTime) >= interval.started &&
      BigInt(row.timestampFileTime) <= interval.ended && EVENT_KINDS.has(row.kind) &&
      Number.isSafeInteger(row.eventId) && row.eventId >= 0 && row.eventId <= 65_535 &&
      Number.isSafeInteger(row.version) && row.version >= 0 && row.version <= 255 &&
      Number.isSafeInteger(row.level) && row.level >= 0 && row.level <= 255 &&
      Number.isSafeInteger(row.opcode) && row.opcode >= 0 && row.opcode <= 255 &&
      Number.isSafeInteger(row.task) && row.task >= 0 && row.task <= 65_535 &&
      Number.isSafeInteger(row.headerProcessId) && row.headerProcessId >= 0 &&
      Number.isSafeInteger(row.headerThreadId) && row.headerThreadId >= 0 &&
      /^0x[A-F0-9]{16}$/u.test(row.keyword) &&
      [row.eventName, row.opcodeName, row.taskName].every((name) =>
        typeof name === "string" && name.length <= 256) && Array.isArray(row.properties),
    `Normalized ETW event ${index + 1} is outside the fixed schema or capture interval.`);
    const role = PROVIDER_ROLES.get(row.providerId);
    const kindAllowed = role === "process-ancestry"
      ? new Set(["OTHER", "PROCESS_START", "PROCESS_STOP"]).has(row.kind)
      : role === "packet"
        ? new Set(["NETWORK_PASSIVE", "OTHER", "PACKET"]).has(row.kind)
        : role === "endpoint-socket"
          ? new Set(["NETWORK_PASSIVE", "OTHER", "SOCKET"]).has(row.kind)
          : new Set(["DNS_RESOLVER", "NETWORK_PASSIVE", "OTHER"]).has(row.kind);
    requireCondition(kindAllowed,
      `Normalized ETW event ${index + 1} has a kind outside its provider role.`);
    const propertyNames = new Set();
    for (const property of row.properties) {
      requireCondition(exactKeys(property, ["name", "value"]) &&
        typeof property.name === "string" && property.name.length > 0 && property.name.length <= 128 &&
        typeof property.value === "string" && property.value.length <= 8192 &&
        !propertyNames.has(property.name.toLocaleLowerCase("en-US")),
      `Normalized ETW event ${index + 1} has malformed or duplicate properties.`);
      propertyNames.add(property.name.toLocaleLowerCase("en-US"));
    }
    return row;
  });
  return events;
}

function properties(row) {
  return new Map(row.properties.map(({ name, value }) => [name.toLocaleLowerCase("en-US"), value]));
}

function integerProperty(row, names, required, label) {
  const map = properties(row);
  const values = names.map((name) => map.get(name.toLocaleLowerCase("en-US"))).filter((value) => value !== undefined);
  if (values.length === 0) {
    if (required) fail(`${label} is missing.`);
    return null;
  }
  const parsed = values.map((value) => /^[0-9]{1,10}$/u.test(value) ? Number(value) : Number.NaN);
  requireCondition(parsed.every((value) => Number.isSafeInteger(value) && value >= 0) &&
    parsed.every((value) => value === parsed[0]), `${label} is malformed or conflicting.`);
  return parsed[0];
}

function processSequenceProperty(row, name, label) {
  const value = properties(row).get(name.toLocaleLowerCase("en-US"));
  requireCondition(typeof value === "string" && /^[1-9][0-9]{0,19}$/u.test(value),
    `${label} is missing or malformed.`);
  const parsed = BigInt(value);
  requireCondition(parsed <= 18_446_744_073_709_551_615n, `${label} exceeds unsigned 64-bit range.`);
  return value;
}

function stringProperty(row, names, required, label) {
  const map = properties(row);
  const values = names.map((name) => map.get(name.toLocaleLowerCase("en-US"))).filter((value) => value !== undefined);
  if (values.length === 0) {
    if (required) fail(`${label} is missing.`);
    return null;
  }
  requireCondition(values.every((value) => value === values[0] && value.length > 0),
    `${label} is malformed or conflicting.`);
  return values[0];
}

function analyzeProcesses(events, raw) {
  const starts = new Map();
  const activeByPid = new Map();
  const stopSequences = new Set();
  for (const row of events) {
    if (row.providerId !== PROCESS_PROVIDER_ID || !["PROCESS_START", "PROCESS_STOP"].includes(row.kind)) continue;
    const pid = integerProperty(row, ["ObserverProcessId", "ProcessId", "PID"], true, "Process event PID");
    requireCondition(pid > 0, "Process event PID must be positive.");
    const processSequenceNumber = processSequenceProperty(
      row, "ProcessSequenceNumber", `Process ${pid} sequence number`);
    if (row.kind === "PROCESS_START") {
      requireCondition(!starts.has(processSequenceNumber),
        `ETW process sequence ${processSequenceNumber} has duplicate start evidence.`);
      requireCondition(!stopSequences.has(processSequenceNumber),
        `ETW process sequence ${processSequenceNumber} started after stop evidence.`);
      requireCondition(!activeByPid.has(pid),
        `ETW process PID ${pid} has overlapping process instances.`);
      const parentProcessId = integerProperty(row,
        ["ObserverParentProcessId", "ParentProcessId", "ParentId", "PPID"], true,
        `Process ${pid} parent PID`);
      const parentProcessSequenceNumber = processSequenceProperty(
        row, "ParentProcessSequenceNumber", `Process ${pid} parent sequence number`);
      // System-wide ETW includes unrelated processes. They may have no readable image
      // hash, but the exact candidate root must still carry the hash below.
      const imageSha256 = stringProperty(row, ["ObserverImageSha256"], false, `Process ${pid} image hash`);
      requireCondition(imageSha256 === null || SHA256.test(imageSha256), `Process ${pid} image hash is malformed.`);
      const instance = {
        imageSha256,
        parentProcessId,
        parentProcessSequenceNumber,
        processId: pid,
        processSequenceNumber,
        startedAt: BigInt(row.timestampFileTime),
        stoppedAt: null,
      };
      starts.set(processSequenceNumber, instance);
      activeByPid.set(pid, processSequenceNumber);
    } else {
      requireCondition(!stopSequences.has(processSequenceNumber),
        `ETW process sequence ${processSequenceNumber} has duplicate stop evidence.`);
      stopSequences.add(processSequenceNumber);
      const instance = starts.get(processSequenceNumber);
      if (instance === undefined) {
        requireCondition(!activeByPid.has(pid),
          `ETW process PID ${pid} stop sequence conflicts with its active instance.`);
        continue;
      }
      requireCondition(instance.processId === pid,
        `ETW process sequence ${processSequenceNumber} changed PID at teardown.`);
      requireCondition(activeByPid.get(pid) === processSequenceNumber,
        `ETW process PID ${pid} teardown is ambiguous.`);
      const stoppedAt = BigInt(row.timestampFileTime);
      requireCondition(stoppedAt > instance.startedAt,
        `ETW process sequence ${processSequenceNumber} stopped before it started.`);
      instance.stoppedAt = stoppedAt;
      activeByPid.delete(pid);
    }
  }

  const providersEnabled = BigInt(raw.interval.providersEnabledAtFileTime);
  const launcherStarted = BigInt(raw.interval.launcherStartedAtFileTime);
  const launcherExited = BigInt(raw.interval.launcherExitedAtFileTime);
  const launcherInstances = [...starts.values()].filter((row) =>
    row.processId === raw.launcher.processId && row.startedAt >= providersEnabled &&
    row.startedAt <= launcherStarted);
  requireCondition(launcherInstances.length === 1,
    "ETW process evidence did not identify exactly one fixed launcher instance.");
  const launcher = launcherInstances[0];
  requireCondition(launcher.stoppedAt !== null && launcher.stoppedAt <= launcherExited,
    "The fixed launcher instance lacks a covered teardown event.");
  const roots = [...starts.values()].filter((row) =>
    row.imageSha256 === raw.candidateImageSha256 && row.parentProcessId === raw.launcher.processId &&
    row.parentProcessSequenceNumber === launcher.processSequenceNumber);
  requireCondition(roots.length === 1, "ETW ancestry did not identify exactly one exact-hash candidate root process.");
  const root = roots[0];
  const descendants = new Map([[root.processSequenceNumber, root]]);
  let changed = true;
  while (changed) {
    changed = false;
    for (const row of starts.values()) {
      if (descendants.has(row.processSequenceNumber)) continue;
      const parent = descendants.get(row.parentProcessSequenceNumber);
      if (parent !== undefined) {
        requireCondition(row.parentProcessId === parent.processId,
          `Candidate process ${row.processId} has conflicting parent PID and process sequence evidence.`);
        requireCondition(parent.startedAt <= row.startedAt && parent.stoppedAt !== null &&
          row.startedAt <= parent.stoppedAt,
        `Candidate process ${row.processId} started outside its parent process lifetime.`);
        descendants.set(row.processSequenceNumber, row);
        changed = true;
      }
    }
  }
  const stopped = BigInt(raw.interval.stoppedAtFileTime);
  requireCondition(root.startedAt >= launcherStarted && root.startedAt < launcherExited,
    "The exact candidate process did not start inside the observed launcher interval.");
  for (const row of descendants.values()) {
    requireCondition(row.stoppedAt !== null && row.stoppedAt <= stopped,
      `Candidate process ${row.processId} lacks a covered teardown event.`);
  }

  for (const row of starts.values()) {
    if (descendants.has(row.processSequenceNumber)) continue;
    const conflictingParents = [...descendants.values()].filter((parent) =>
      parent.processId === row.parentProcessId && parent.startedAt <= row.startedAt &&
      parent.stoppedAt !== null && row.startedAt <= parent.stoppedAt);
    requireCondition(conflictingParents.length === 0,
      `Process ${row.processId} has a parent PID inside candidate lifetime but a conflicting parent sequence.`);
  }

  const candidateInstances = [...descendants.values()];
  return {
    candidateInstances,
    processInstances: [...starts.values()],
    processAncestry: [...candidateInstances]
      .sort((left, right) => left.processId - right.processId ||
        (BigInt(left.processSequenceNumber) < BigInt(right.processSequenceNumber) ? -1 : 1))
      .map(({ imageSha256, parentProcessId, processId }) => ({ imageSha256, parentProcessId, processId })),
    rootProcessId: root.processId,
  };
}

function optionalProperty(row, name) {
  return properties(row).get(name.toLocaleLowerCase("en-US"));
}

function normalizedProcessAttribution(row, authoritativeProcessId, expectedSource) {
  const processIdValue = optionalProperty(row, "ObserverProcessId");
  const source = optionalProperty(row, "ObserverProcessIdSource");
  let normalizedProcessId = null;
  if (processIdValue !== undefined) {
    requireCondition(/^[1-9][0-9]{0,9}$/u.test(processIdValue),
      `Network event ${row.sequence} normalized process attribution is malformed.`);
    normalizedProcessId = Number(processIdValue);
    requireCondition(Number.isSafeInteger(normalizedProcessId) && normalizedProcessId <= 4_294_967_295,
      `Network event ${row.sequence} normalized process attribution is malformed.`);
  }
  if (source === undefined) {
    // Captures made by observer v1 before attribution-source tagging remain
    // replayable. Their normalized PID is syntax-checked but never trusted;
    // provider-specific data below remains authoritative.
    return;
  }
  requireCondition(typeof expectedSource === "string" && authoritativeProcessId !== null &&
    source === expectedSource && normalizedProcessId === authoritativeProcessId,
  `Network event ${row.sequence} has incompatible normalized process attribution.`);
}

function positiveProviderPid(row, name, label) {
  const processId = integerProperty(row, [name], true, label);
  requireCondition(processId > 0 && processId <= 4_294_967_295, `${label} is malformed.`);
  return processId;
}

function afdProcessToken(row) {
  const token = stringProperty(row, ["Process"], true,
    `AFD event ${row.sequence} process token`);
  requireCondition(/^[A-Fa-f0-9]{16}$/u.test(token) && !/^0{16}$/u.test(token),
    `AFD event ${row.sequence} process token is malformed.`);
  return token.toLocaleUpperCase("en-US");
}

function afdCreateProcessId(row) {
  const value = stringProperty(row, ["ProcessId"], true,
    `AFD create event ${row.sequence} process ID`);
  requireCondition(/^[A-Fa-f0-9]{16}$/u.test(value),
    `AFD create event ${row.sequence} process ID is malformed.`);
  const bytes = [];
  for (let index = 0; index < value.length; index += 2) {
    bytes.push(Number.parseInt(value.slice(index, index + 2), 16));
  }
  requireCondition(bytes.slice(4).every((byte) => byte === 0),
    `AFD create event ${row.sequence} process ID exceeds DWORD range.`);
  const processId = bytes[0] + bytes[1] * 0x100 + bytes[2] * 0x1_0000 + bytes[3] * 0x100_0000;
  requireCondition(Number.isSafeInteger(processId) && processId > 0 && processId <= 4_294_967_295,
    `AFD create event ${row.sequence} process ID is malformed.`);
  return processId;
}

function activeProcessInstance(processInstances, processId, timestamp, label) {
  const matches = processInstances.filter((instance) => instance.processId === processId &&
    instance.startedAt <= timestamp && (instance.stoppedAt === null || timestamp <= instance.stoppedAt));
  requireCondition(matches.length <= 1, `${label} has ambiguous process-lifetime attribution.`);
  return matches[0] ?? null;
}

function afdAttributions(events, processInstances) {
  const bindings = new Map();
  const attributed = new Map();
  for (const row of events) {
    if (row.providerId !== AFD_PROVIDER_ID) continue;
    const timestamp = BigInt(row.timestampFileTime);
    const token = afdProcessToken(row);
    const previous = bindings.get(token) ?? null;
    if (previous !== null) {
      requireCondition(timestamp >= previous.boundAt,
        `AFD event ${row.sequence} is temporally before its process-token binding.`);
    }
    if (row.eventId === 1000) {
      const processId = afdCreateProcessId(row);
      const instance = activeProcessInstance(processInstances, processId, timestamp,
        `AFD create event ${row.sequence}`);
      if (previous !== null && previous.processId !== processId) {
        requireCondition(previous.instance !== null && previous.instance.stoppedAt !== null &&
          previous.instance.stoppedAt < timestamp,
          `AFD create event ${row.sequence} ambiguously reuses a process token without a completed prior lifetime.`);
      }
      if (previous !== null && previous.boundAt === timestamp) {
        requireCondition(previous.processId === processId &&
          previous.instance?.processSequenceNumber === instance?.processSequenceNumber,
        `AFD create event ${row.sequence} has conflicting simultaneous process-token bindings.`);
      }
      const binding = { boundAt: timestamp, instance, processId };
      bindings.set(token, binding);
      normalizedProcessAttribution(row, processId, "afd-create-process-id");
      attributed.set(row.sequence, binding);
      continue;
    }
    const bindingExpired = previous !== null && previous.instance !== null &&
      previous.instance.stoppedAt !== null && timestamp > previous.instance.stoppedAt;
    if (previous === null || bindingExpired) {
      normalizedProcessAttribution(row, null, null);
      attributed.set(row.sequence, null);
      continue;
    }
    normalizedProcessAttribution(row, previous.processId, "afd-process-map");
    attributed.set(row.sequence, previous);
  }
  return attributed;
}

function attributedProcess(row, afdRows) {
  if (row.providerId === AFD_PROVIDER_ID) return afdRows.get(row.sequence) ?? null;
  let processId;
  let source;
  if (row.providerId === PACKET_PROVIDER_ID) {
    processId = positiveProviderPid(row, "PID", `Packet event ${row.sequence} payload PID`);
    source = "kernel-network-pid";
  } else if (row.providerId === DNS_CLIENT_PROVIDER_ID) {
    if (optionalProperty(row, "ClientPID") !== undefined) {
      processId = positiveProviderPid(row, "ClientPID", `DNS event ${row.sequence} client PID`);
      source = "dns-client-pid";
    } else {
      requireCondition(row.headerProcessId > 0,
        `DNS event ${row.sequence} header-fallback PID is malformed.`);
      processId = row.headerProcessId;
      source = "dns-client-header-fallback";
    }
  } else if (row.providerId === NAME_RESOLUTION_PROVIDER_ID) {
    requireCondition(row.headerProcessId > 0,
      `Name-resolution event ${row.sequence} header PID is malformed.`);
    processId = row.headerProcessId;
    source = "name-resolution-header";
  } else {
    fail(`Network event ${row.sequence} has an unsupported provider attribution rule.`);
  }
  normalizedProcessAttribution(row, processId, source);
  return { boundAt: BigInt(row.timestampFileTime), instance: null, processId };
}

function normalizeIpTarget(value) {
  let target = String(value ?? "").trim();
  if (target.startsWith("[") && target.includes("]")) target = target.slice(1, target.indexOf("]"));
  const lastColon = target.lastIndexOf(":");
  if (isIP(target) === 0 && lastColon > 0 && /^[0-9]{1,5}$/u.test(target.slice(lastColon + 1))) {
    const host = target.slice(0, lastColon);
    if (isIP(host) !== 0) target = host;
  }
  const zone = target.indexOf("%");
  if (zone > 0) target = target.slice(0, zone);
  return target.toLocaleLowerCase("en-US");
}

function loopback(target) {
  const value = normalizeIpTarget(target);
  if (value === "localhost") return true;
  if (isIP(value) === 4) {
    const first = Number(value.split(".")[0]);
    return first === 127;
  }
  return isIP(value) === 6 && (value === "::1" || value === "0:0:0:0:0:0:0:1");
}

function unspecified(target) {
  const value = normalizeIpTarget(target);
  return value === "0.0.0.0" || value === "::" || value === "0:0:0:0:0:0:0:0";
}

function resolverTarget(row) {
  const value = stringProperty(row, ["QueryName", "NodeName"], false,
    `Resolver event ${row.sequence} target`);
  if (value === null) return null;
  const target = value.trim().replace(/\.$/u, "").toLocaleLowerCase("en-US");
  requireCondition(target.length > 0 && !/[\u0000-\u001F\u007F\s]/u.test(target),
    `Resolver event ${row.sequence} target is malformed.`);
  return target;
}

function approvedResolverTarget(target) {
  if (target === "null") return true;
  if (target === "localhost" || target === "govs-plc.local") return true;
  return loopback(target);
}

function resolverDisposition(row) {
  const initiating = row.providerId === DNS_CLIENT_PROVIDER_ID
    ? DNS_CLIENT_INITIATION_EVENT_IDS
    : NAME_RESOLUTION_INITIATION_EVENT_IDS;
  const passive = row.providerId === DNS_CLIENT_PROVIDER_ID
    ? DNS_CLIENT_PASSIVE_EVENT_IDS
    : NAME_RESOLUTION_PASSIVE_EVENT_IDS;
  if (passive.has(row.eventId)) {
    return { disposition: "accounted", reason: "resolver-completion-or-lifecycle", target: null };
  }
  if (!initiating.has(row.eventId) || row.kind !== "DNS_RESOLVER") {
    return { disposition: "unknown", reason: "unsupported-resolver-event-schema", target: null };
  }
  const target = resolverTarget(row);
  if (target === null) {
    return { disposition: "unknown", reason: "resolver-invocation-missing-target", target: null };
  }
  return approvedResolverTarget(target)
    ? { disposition: "accounted", reason: "approved-local-resolver-target", target }
    : { disposition: "external", reason: "resolver-invocation", target };
}

function afdEndpointToken(row) {
  const endpoint = stringProperty(row, ["Endpoint"], false,
    `AFD event ${row.sequence} endpoint token`);
  if (endpoint === null) return null;
  requireCondition(/^[A-Fa-f0-9]{16}$/u.test(endpoint) && !/^0{16}$/u.test(endpoint),
    `AFD event ${row.sequence} endpoint token is malformed.`);
  return endpoint.toLocaleUpperCase("en-US");
}

function eventIpTarget(row) {
  return stringProperty(row,
    ["ObserverTargetAddress", "RemoteAddress", "DestinationAddress", "DestAddress", "daddr", "Address"],
    false, `Network event ${row.sequence} target`);
}

function packetIpTarget(row) {
  // KernelNetwork's UDP receive templates place the remote peer in saddr;
  // the other fixed packet templates expose the remote peer in daddr.
  return [43, 59].includes(row.eventId)
    ? stringProperty(row, ["saddr"], false, `Packet event ${row.sequence} remote target`)
    : eventIpTarget(row);
}

function afdEndpointContexts(events, afdRows) {
  const contexts = new Map();
  const rows = new Map();
  for (const row of events) {
    if (row.providerId !== AFD_PROVIDER_ID) continue;
    const attribution = afdRows.get(row.sequence) ?? null;
    if (attribution?.instance === null || attribution === null) continue;
    const endpoint = afdEndpointToken(row);
    if (endpoint === null) continue;
    const key = `${attribution.instance.processSequenceNumber}\0${endpoint}`;
    const context = contexts.get(key) ?? { endpoint, targets: new Map() };
    const target = AFD_DIRECT_REMOTE_EVENT_IDS.has(row.eventId) ? eventIpTarget(row) : null;
    if (target !== null) {
      const normalized = normalizeIpTarget(target);
      requireCondition(isIP(normalized) !== 0,
        `AFD event ${row.sequence} endpoint target is malformed.`);
      context.targets.set(normalized, target);
    }
    contexts.set(key, context);
    rows.set(row.sequence, { context, directTarget: target });
  }
  for (const value of rows.values()) {
    if (value.directTarget !== null) continue;
    const routable = [...value.context.targets].filter(([target]) => !unspecified(target));
    const available = routable.length > 0 ? routable : [...value.context.targets];
    value.inferredTarget = available.length === 1 ? available[0][1] : null;
  }
  return rows;
}

function analyzeNetwork(events, candidateInstances, processInstances) {
  const externalAttempts = [];
  const accountedEvents = [];
  const unknownEvents = [];
  const resolverAttemptKeys = new Set();
  const ipAttemptKeys = new Set();
  const afdRows = afdAttributions(events, processInstances);
  const afdEndpoints = afdEndpointContexts(events, afdRows);
  const recordIpObservation = (summary, target, direction, attemptKey) => {
    if (target === null) {
      unknownEvents.push({ ...summary, reason: "network-target-unavailable" });
      return;
    }
    const normalized = normalizeIpTarget(target);
    if (!new Set(["listen", "outbound", "passive"]).has(direction) || isIP(normalized) === 0) {
      unknownEvents.push({ ...summary, direction, reason: "unparseable-network-target", target });
    } else if (loopback(target) || (direction === "listen" && unspecified(target))) {
      accountedEvents.push({ ...summary, direction, reason: "loopback-or-unspecified-listener", target });
    } else if (ipAttemptKeys.has(attemptKey)) {
      accountedEvents.push({ ...summary, direction, reason: "duplicate-network-observation", target });
    } else {
      ipAttemptKeys.add(attemptKey);
      externalAttempts.push({
        ...summary,
        direction,
        reason: direction === "outbound" ? "non-loopback-network-attempt" : "non-loopback-network-activity",
        target,
      });
    }
  };
  for (const row of events) {
    if (!NETWORK_PROVIDER_IDS.has(row.providerId)) continue;
    const attribution = attributedProcess(row, afdRows);
    if (attribution === null) continue;
    const processId = attribution.processId;
    const timestamp = BigInt(row.timestampFileTime);
    const matchingInstances = row.providerId === AFD_PROVIDER_ID
      ? attribution.instance === null
        ? []
        : candidateInstances.filter((instance) =>
          instance.processSequenceNumber === attribution.instance.processSequenceNumber)
      : candidateInstances.filter((instance) => instance.processId === processId &&
        instance.startedAt <= timestamp && instance.stoppedAt !== null && timestamp <= instance.stoppedAt);
    requireCondition(matchingInstances.length <= 1,
      `Network event ${row.sequence} has ambiguous candidate process lifetime attribution.`);
    if (matchingInstances.length === 0) continue;
    const summary = {
      eventId: row.eventId,
      eventName: row.eventName,
      kind: row.kind,
      processId,
      providerId: row.providerId,
      sequence: row.sequence,
    };
    if ([DNS_CLIENT_PROVIDER_ID, NAME_RESOLUTION_PROVIDER_ID].includes(row.providerId)) {
      const resolver = resolverDisposition(row);
      const resolverSummary = resolver.target === null ? summary : { ...summary, target: resolver.target };
      if (resolver.disposition === "unknown") {
        unknownEvents.push({ ...resolverSummary, reason: resolver.reason });
      } else if (resolver.disposition === "accounted") {
        accountedEvents.push({ ...resolverSummary, reason: resolver.reason });
      } else {
        const key = `${matchingInstances[0].processSequenceNumber}\0${resolver.target}`;
        if (resolverAttemptKeys.has(key)) {
          accountedEvents.push({ ...resolverSummary, reason: "duplicate-resolver-observation" });
        } else {
          resolverAttemptKeys.add(key);
          externalAttempts.push({ ...resolverSummary, reason: resolver.reason });
        }
      }
      continue;
    }
    if (row.providerId === AFD_PROVIDER_ID && row.eventId === 1000) {
      accountedEvents.push({ ...summary, reason: "afd-process-token-binding" });
      continue;
    }
    if (row.providerId === AFD_PROVIDER_ID) {
      if (AFD_LIFECYCLE_EVENT_IDS.has(row.eventId)) {
        accountedEvents.push({ ...summary, reason: "afd-lifecycle-or-socket-configuration" });
        continue;
      }
      if (AFD_PASSIVE_EVENT_IDS.has(row.eventId)) {
        accountedEvents.push({ ...summary, reason: "afd-passive-completion-or-receive" });
        continue;
      }
      if (!AFD_OUTBOUND_EVENT_IDS.has(row.eventId) && row.eventId !== AFD_BIND_EVENT_ID) {
        unknownEvents.push({ ...summary, reason: "unsupported-afd-event-schema" });
        continue;
      }
      const endpoint = afdEndpoints.get(row.sequence) ?? null;
      const target = eventIpTarget(row) ?? endpoint?.inferredTarget ?? null;
      const direction = row.eventId === AFD_BIND_EVENT_ID ? "listen" : "outbound";
      const endpointKey = endpoint?.context.endpoint ?? `sequence-${row.sequence}`;
      recordIpObservation(summary, target, direction,
        `${matchingInstances[0].processSequenceNumber}\0AFD\0${endpointKey}\0${normalizeIpTarget(target)}`);
      continue;
    }
    if (row.providerId === PACKET_PROVIDER_ID) {
      const outbound = PACKET_OUTBOUND_EVENT_IDS.has(row.eventId);
      if (!outbound && !PACKET_PASSIVE_EVENT_IDS.has(row.eventId)) {
        unknownEvents.push({ ...summary, reason: "unsupported-kernel-network-event-schema" });
        continue;
      }
      const target = packetIpTarget(row);
      const sourcePort = stringProperty(row, ["sport"], false,
        `Packet event ${row.sequence} source port`) ?? "";
      const destinationPort = stringProperty(row, ["dport"], false,
        `Packet event ${row.sequence} destination port`) ?? "";
      recordIpObservation(summary, target, outbound ? "outbound" : "passive",
        `${matchingInstances[0].processSequenceNumber}\0PACKET\0${normalizeIpTarget(target)}\0${sourcePort}\0${destinationPort}`);
      continue;
    }
    fail(`Network event ${row.sequence} has no provider-specific classification rule.`);
  }
  return { accountedEvents, externalAttempts, unknownEvents };
}

export function analyzeExternalObserverEvidence({
  candidateImageBytes,
  candidateManifest,
  candidateManifestBytes,
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
  rawBytes,
}) {
  requireCondition(files instanceof Map && Buffer.isBuffer(rawBytes), "External observer inputs are incomplete.");
  let parsedRaw;
  let parsedCandidateManifest;
  let parsedMetadata;
  try {
    parsedRaw = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(rawBytes));
    parsedCandidateManifest = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(candidateManifestBytes));
    const metadataBytes = files.get(FIXED_FILES.metadata);
    requireCondition(Buffer.isBuffer(metadataBytes), "The ETW provider metadata content is unavailable.");
    parsedMetadata = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(metadataBytes));
  } catch {
    fail("An external observer JSON input is malformed or is not valid UTF-8.");
  }
  requireCondition(
    JSON.stringify(parsedRaw) === JSON.stringify(raw) &&
    JSON.stringify(parsedCandidateManifest) === JSON.stringify(candidateManifest) &&
    JSON.stringify(parsedMetadata) === JSON.stringify(metadata),
    "Parsed external observer inputs are not bound to their supplied bytes.",
  );
  const identities = { launcherBytes, observerBuildScriptBytes, observerBytes, observerSourceBytes };
  const interval = validateRawManifest(raw, files, identities);
  requireCandidateIdentity(candidateManifest, candidateManifestBytes, candidateImageBytes, raw, {
    observerAnalyzerSourceBytes,
    observerBuildScriptBytes,
    observerFinalizerSourceBytes,
    observerSourceBytes,
    observerSourceVerifierBytes,
  });
  const events = parseEvents(files.get(FIXED_FILES.events), interval);
  const providerCoverage = validateProviderMetadata(metadata, events);
  const processes = analyzeProcesses(events, raw);
  const network = analyzeNetwork(events, processes.candidateInstances, processes.processInstances);
  const zeroExternalAttempts = network.externalAttempts.length === 0 && network.unknownEvents.length === 0;
  return {
    accountedNetworkEventCount: network.accountedEvents.length,
    candidateCommit: raw.candidateCommit,
    candidateManifestSha256: raw.candidateManifestSha256,
    candidateTree: raw.candidateTree,
    coverage: {
      dnsClient: true,
      endpointSocket: true,
      gapFree: true,
      packet: true,
      processAncestry: true,
      resolverApi: true,
    },
    evidenceKind: "WINDOWS_PHASE2_ETW_EXTERNAL_OBSERVER_ANALYSIS",
    eventInterval: {
      endedAtUtc: raw.interval.stoppedAtUtc,
      eventCount: events.length,
      eventStreamSha256: contentSha256(files, FIXED_FILES.events),
      startedAtUtc: raw.interval.startedAtUtc,
    },
    externalAttemptCount: network.externalAttempts.length,
    externalAttempts: network.externalAttempts,
    analyzerSourceSha256: sha256(observerAnalyzerSourceBytes),
    finalizerSourceSha256: sha256(observerFinalizerSourceBytes),
    observerExecutableSha256: raw.observerExecutableSha256,
    observerVersion: raw.observerVersion,
    processAncestry: processes.processAncestry,
    providerCoverage,
    rawEtlSha256: contentSha256(files, FIXED_FILES.etl),
    rawObserverManifestSha256: sha256(rawBytes),
    result: zeroExternalAttempts ? "PASS" : "FAIL",
    rootProcessId: processes.rootProcessId,
    schemaVersion: "1.0",
    sourceVerifierSha256: sha256(observerSourceVerifierBytes),
    traceStatistics: structuredClone(raw.traceStatistics),
    unknownEventCount: network.unknownEvents.length,
    unknownEvents: network.unknownEvents,
    zeroExternalAttempts,
  };
}

export function fixedObserverFileNames() { return structuredClone(FIXED_FILES); }
export function hashExternalObserverBytes(bytes) { return sha256(bytes); }
