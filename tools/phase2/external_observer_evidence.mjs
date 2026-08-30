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
    }
  });
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
    processAncestry: [...candidateInstances]
      .sort((left, right) => left.processId - right.processId ||
        (BigInt(left.processSequenceNumber) < BigInt(right.processSequenceNumber) ? -1 : 1))
      .map(({ imageSha256, parentProcessId, processId }) => ({ imageSha256, parentProcessId, processId })),
    rootProcessId: root.processId,
  };
}

function attributedProcessId(row) {
  const payload = integerProperty(row, ["ObserverProcessId", "ProcessId", "PID"], false,
    `Network event ${row.sequence} process attribution`);
  if (payload !== null && row.headerProcessId !== 0 && row.headerProcessId !== 4) {
    requireCondition(payload === row.headerProcessId,
      `Network event ${row.sequence} has conflicting header and payload process attribution.`);
  }
  return payload ?? row.headerProcessId;
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

function analyzeNetwork(events, candidateInstances) {
  const externalAttempts = [];
  const accountedEvents = [];
  const unknownEvents = [];
  for (const row of events) {
    if (!NETWORK_PROVIDER_IDS.has(row.providerId)) continue;
    const processId = attributedProcessId(row);
    const timestamp = BigInt(row.timestampFileTime);
    const matchingInstances = candidateInstances.filter((instance) =>
      instance.processId === processId && instance.startedAt <= timestamp &&
      instance.stoppedAt !== null && timestamp <= instance.stoppedAt);
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
    if (row.kind === "NETWORK_PASSIVE") {
      // A passive receive/accept/completion is not affirmative evidence that no
      // network attempt occurred. Without a causal, fixed-schema counterpart it
      // remains non-credit rather than being silently accepted.
      unknownEvents.push({ ...summary, reason: "unpaired-passive-network-observation" });
      continue;
    }
    if (row.kind === "DNS_RESOLVER") {
      externalAttempts.push({ ...summary, reason: "resolver-api-invocation" });
      continue;
    }
    if (!["SOCKET", "PACKET"].includes(row.kind)) {
      unknownEvents.push({ ...summary, reason: "unclassified-candidate-network-event" });
      continue;
    }
    const direction = stringProperty(row, ["ObserverDirection"], true,
      `Network event ${row.sequence} direction`).toLocaleLowerCase("en-US");
    const target = stringProperty(row,
      ["ObserverTargetAddress", "RemoteAddress", "DestinationAddress", "DestAddress", "daddr"],
      true, `Network event ${row.sequence} target`);
    if (!new Set(["listen", "outbound"]).has(direction) || isIP(normalizeIpTarget(target)) === 0) {
      unknownEvents.push({ ...summary, direction, reason: "unparseable-network-target", target });
    } else if (loopback(target) || (direction === "listen" && unspecified(target))) {
      accountedEvents.push({ ...summary, direction, reason: "loopback-or-unspecified-listener", target });
    } else {
      externalAttempts.push({ ...summary, direction, reason: "non-loopback-network-attempt", target });
    }
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
  const network = analyzeNetwork(events, processes.candidateInstances);
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
