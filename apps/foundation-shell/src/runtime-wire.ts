import type {
  EngineeringRuntimeView,
  RuntimeDiagnosticView,
  RuntimeForceView,
  RuntimeHashView,
  RuntimeOperation,
  RuntimeProbeKind,
  RuntimeProbeView,
  RuntimeSessionView,
  RuntimeTraceView,
  RuntimeValue,
  RuntimeValueType,
  RuntimeWatchRowView,
  RuntimeWatchTableView,
  VirtualLoadPreviewView,
} from "./runtime-types";

const WIRE_MAGIC = "PES-SYSTEM-COMMAND-1";
const MAX_RESPONSE_BYTES = 8 * 1024 * 1024;
const MAX_COLLECTION_ITEMS = 16_384;
const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/iu;
const HASH_PATTERN = /^[0-9a-f]{64}$/iu;
const UINT64_PATTERN = /^(?:0|[1-9][0-9]*)$/u;

export type RuntimeWireIdentity = Readonly<{
  authorId: string;
  commandId: string;
  idempotencyKey: string;
}>;

export class RuntimeWireError extends Error {
  public constructor(message: string) {
    super(message);
    this.name = "RuntimeWireError";
  }
}

/**
 * Encodes the already validated worker command into a deliberately tiny,
 * capability-free WASM boundary. Every field is scalar, bounded, and parsed
 * again by Rust before a typed system command is constructed.
 */
export const encodeRuntimeOperation = (
  operation: RuntimeOperation,
  identity: RuntimeWireIdentity,
): Uint8Array<ArrayBuffer> => {
  const fields = [
    WIRE_MAGIC,
    operationToken(operation),
    requireUuid(identity.commandId, "command identity"),
    requireUuid(identity.idempotencyKey, "idempotency identity"),
    requireUuid(identity.authorId, "author identity"),
    ...operationFields(operation),
  ];
  const encoded = new TextEncoder().encode(fields.join("\n"));
  if (encoded.byteLength < 1 || encoded.byteLength > 4_096) {
    throw new RuntimeWireError("The runtime command exceeds its wire budget.");
  }
  return encoded;
};

export const parseEngineeringRuntimeView = (
  bytes: Uint8Array,
): EngineeringRuntimeView => {
  if (bytes.byteLength < 2 || bytes.byteLength > MAX_RESPONSE_BYTES) {
    throw new RuntimeWireError("The runtime response is outside its byte budget.");
  }
  let decoded: unknown;
  try {
    decoded = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)) as unknown;
  } catch {
    throw new RuntimeWireError("The runtime core returned malformed UTF-8 JSON.");
  }
  const root = record(decoded, "runtime response");
  exactKeys(root, [
    "availability",
    "canBuild",
    "diagnostics",
    "reason",
    "schemaVersion",
    "session",
    "sourceDocumentHash",
    "sourceSemanticFingerprint",
  ], "runtime response");
  const availability = oneOf(root.availability, ["READY", "UNAVAILABLE"] as const, "runtime availability");
  const session = root.session === null ? null : parseSession(root.session);
  if ((availability === "READY") !== (session !== null)) {
    throw new RuntimeWireError("Runtime availability and session state disagree.");
  }
  return {
    availability,
    canBuild: boolean(root.canBuild, "runtime canBuild"),
    diagnostics: array(root.diagnostics, "runtime diagnostics", 2_048).map((value, index) => {
      const diagnostic = record(value, `runtime diagnostics[${index}]`);
      exactKeys(diagnostic, ["blocking", "code", "message", "objectId"], `runtime diagnostics[${index}]`);
      return {
        blocking: boolean(diagnostic.blocking, "runtime diagnostic blocking"),
        code: string(diagnostic.code, "runtime diagnostic code", 128),
        message: string(diagnostic.message, "runtime diagnostic message", 4_096),
        objectId: nullableUuid(diagnostic.objectId, "runtime diagnostic object identity"),
      };
    }),
    reason: nullableString(root.reason, "runtime unavailable reason", 1_024),
    schemaVersion: exactNumber(root.schemaVersion, 1, "runtime schema version"),
    session,
    sourceDocumentHash: hash(root.sourceDocumentHash, "runtime source document hash"),
    sourceSemanticFingerprint: hash(root.sourceSemanticFingerprint, "runtime semantic fingerprint"),
  };
};

const operationToken = (operation: RuntimeOperation): string => {
  switch (operation.kind) {
    case "runtime.build": return "BUILD";
    case "runtime.power-on": return "POWER_ON";
    case "runtime.power-off": return "POWER_OFF";
    case "runtime.preview-load": return "PREVIEW_LOAD";
    case "runtime.commit-load": return "COMMIT_LOAD";
    case "runtime.go-online": return "GO_ONLINE";
    case "runtime.request-run": return "REQUEST_RUN";
    case "runtime.request-stop": return "REQUEST_STOP";
    case "runtime.run-scan": return "RUN_SCAN";
    case "runtime.start-monitoring": return "START_MONITORING";
    case "runtime.set-raw-input": return "SET_RAW_INPUT";
    case "runtime.modify-once": return "MODIFY_ONCE";
    case "runtime.create-force": return "CREATE_FORCE";
    case "runtime.remove-force": return "REMOVE_FORCE";
    case "runtime.arm-trace": return "ARM_TRACE";
    case "runtime.capture-snapshot": return "CAPTURE_SNAPSHOT";
    case "runtime.restore-snapshot": return "RESTORE_SNAPSHOT";
  }
};

const operationFields = (operation: RuntimeOperation): readonly string[] => {
  switch (operation.kind) {
    case "runtime.preview-load":
      return [operation.postLoadMode];
    case "runtime.set-raw-input":
    case "runtime.modify-once":
      return [requireUuid(operation.targetId, "runtime target identity"), ...valueFields(operation.value)];
    case "runtime.create-force":
      return [
        requireUuid(operation.forceId, "force identity"),
        requireUuid(operation.targetId, "runtime target identity"),
        ...valueFields(operation.value),
        wireText(operation.reason, "force reason", 128),
      ];
    case "runtime.remove-force":
      return [
        requireUuid(operation.forceId, "force identity"),
        wireText(operation.reason, "force reason", 128),
      ];
    case "runtime.arm-trace":
      return [requireUuid(operation.traceId, "trace identity")];
    default:
      return [];
  }
};

const valueFields = (value: RuntimeValue): readonly [RuntimeValueType, string] => {
  if (value.type === "BOOL") {
    if (typeof value.value !== "boolean") {
      throw new RuntimeWireError("A BOOL runtime value must be boolean.");
    }
    return [value.type, value.value ? "true" : "false"];
  }
  if (typeof value.value !== "string" || !/^-?(?:0|[1-9][0-9]*)$/u.test(value.value)) {
    throw new RuntimeWireError(`${value.type} requires canonical decimal text.`);
  }
  const numeric = BigInt(value.value);
  const valid = value.type === "I32"
    ? numeric >= -(1n << 31n) && numeric <= (1n << 31n) - 1n
    : value.type === "I64"
      ? numeric >= -(1n << 63n) && numeric <= (1n << 63n) - 1n
      : value.type === "U32"
        ? numeric >= 0n && numeric <= (1n << 32n) - 1n
        : numeric >= 0n && numeric <= (1n << 64n) - 1n;
  if (!valid) {
    throw new RuntimeWireError(`${value.type} is outside its canonical range.`);
  }
  return [value.type, value.value];
};

const parseSession = (input: unknown): RuntimeSessionView => {
  const value = record(input, "runtime session");
  exactKeys(value, [
    "buildCurrent", "buildFingerprint", "controllerEpoch", "controllerObjectId", "cpuState",
    "diagnosticReplayHash", "diagnostics", "documentDirty", "forceCount", "forceRegistryVersion",
    "forces", "hardwareToLoaded", "hashes", "loadPreview", "loaded", "loadedArtifactFingerprint",
    "monitorState", "online", "probes", "runtimeControllerId", "runtimeReplayHash", "scanSequence",
    "snapshotAvailable", "softwareToLoaded", "traces", "universeEpoch", "universeId",
    "virtualTimeMilliseconds", "watches",
  ], "runtime session");
  const forces = array(value.forces, "runtime forces", 4_096).map(parseForce);
  const declaredForceCount = safeInteger(value.forceCount, "runtime force count", 0, 4_096);
  if (declaredForceCount !== forces.length) {
    throw new RuntimeWireError("Runtime force count does not match its entries.");
  }
  return {
    buildCurrent: boolean(value.buildCurrent, "runtime build current"),
    buildFingerprint: nullableHash(value.buildFingerprint, "runtime build fingerprint"),
    controllerEpoch: uint64(value.controllerEpoch, "runtime controller epoch"),
    controllerObjectId: requireUuid(value.controllerObjectId, "controller object identity"),
    cpuState: oneOf(value.cpuState, ["POWERED_OFF", "STARTUP", "STOP", "RUN", "PAUSED_EDUCATIONAL", "FAULTED"] as const, "CPU state"),
    diagnosticReplayHash: hash(value.diagnosticReplayHash, "diagnostic replay hash"),
    diagnostics: array(value.diagnostics, "runtime diagnostic ledger", 4_096).map(parseRuntimeDiagnostic),
    documentDirty: boolean(value.documentDirty, "runtime document dirty"),
    forceCount: declaredForceCount,
    forceRegistryVersion: uint64(value.forceRegistryVersion, "force registry version"),
    forces,
    hashes: value.hashes === null ? null : parseHashes(value.hashes),
    hardwareToLoaded: nullableString(value.hardwareToLoaded, "hardware comparison", 64),
    loadPreview: value.loadPreview === null ? null : parsePreview(value.loadPreview),
    loaded: boolean(value.loaded, "runtime loaded state"),
    loadedArtifactFingerprint: nullableHash(value.loadedArtifactFingerprint, "loaded artifact fingerprint"),
    monitorState: oneOf(value.monitorState, ["INACTIVE", "ACTIVE", "DEGRADED", "STALE"] as const, "monitor state"),
    online: boolean(value.online, "online state"),
    probes: array(value.probes, "runtime probes", MAX_COLLECTION_ITEMS).map(parseProbe),
    runtimeControllerId: requireUuid(value.runtimeControllerId, "runtime controller identity"),
    runtimeReplayHash: hash(value.runtimeReplayHash, "runtime replay hash"),
    scanSequence: uint64(value.scanSequence, "scan sequence"),
    snapshotAvailable: boolean(value.snapshotAvailable, "snapshot availability"),
    softwareToLoaded: nullableString(value.softwareToLoaded, "software comparison", 64),
    traces: array(value.traces, "runtime traces", 1_024).map(parseTrace),
    universeEpoch: uint64(value.universeEpoch, "universe epoch"),
    universeId: requireUuid(value.universeId, "universe identity"),
    virtualTimeMilliseconds: uint64(value.virtualTimeMilliseconds, "virtual time"),
    watches: array(value.watches, "runtime watch tables", 1_024).map(parseWatchTable),
  };
};

const parseProbe = (input: unknown, index: number): RuntimeProbeView => {
  const value = record(input, `runtime probe[${index}]`);
  exactKeys(value, [
    "committedOutputValue", "deliveredOutputValue", "displayName", "effectiveValue", "forcedValue",
    "id", "kind", "naturalValue", "quality", "rawInputValue", "runtimeAddress", "valueType",
  ], `runtime probe[${index}]`);
  const valueType = parseValueType(value.valueType, "runtime probe value type");
  return {
    committedOutputValue: nullableRuntimeValue(value.committedOutputValue, valueType, "committed output value"),
    deliveredOutputValue: nullableRuntimeValue(value.deliveredOutputValue, valueType, "delivered output value"),
    displayName: string(value.displayName, "runtime probe name", 256),
    effectiveValue: nullableRuntimeValue(value.effectiveValue, valueType, "effective runtime value"),
    forcedValue: nullableRuntimeValue(value.forcedValue, valueType, "forced runtime value"),
    id: requireUuid(value.id, "runtime probe identity"),
    kind: oneOf(value.kind, ["memory", "input", "output"] as const satisfies readonly RuntimeProbeKind[], "runtime probe kind"),
    naturalValue: nullableRuntimeValue(value.naturalValue, valueType, "natural runtime value"),
    quality: oneOf(value.quality, ["GOOD", "STALE", "BAD", "FORCED"] as const, "runtime probe quality"),
    rawInputValue: nullableRuntimeValue(value.rawInputValue, valueType, "raw input value"),
    runtimeAddress: string(value.runtimeAddress, "runtime address", 128),
    valueType,
  };
};

const parseForce = (input: unknown, index: number): RuntimeForceView => {
  const value = record(input, `runtime force[${index}]`);
  exactKeys(value, ["forceId", "reason", "targetId", "value"], `runtime force[${index}]`);
  return {
    forceId: requireUuid(value.forceId, "force identity"),
    reason: string(value.reason, "force reason", 256),
    targetId: requireUuid(value.targetId, "forced target identity"),
    value: parseRuntimeValue(value.value, undefined, "forced value"),
  };
};

const parseTrace = (input: unknown, index: number): RuntimeTraceView => {
  const value = record(input, `runtime trace[${index}]`);
  exactKeys(value, ["captureCount", "id", "name", "state"], `runtime trace[${index}]`);
  return {
    captureCount: safeInteger(value.captureCount, "trace capture count", 0, 1_000_000),
    id: requireUuid(value.id, "trace identity"),
    name: string(value.name, "trace name", 256),
    state: oneOf(value.state, ["IDLE", "ARMED", "CAPTURING", "COMPLETE", "ABORTED"] as const, "trace state"),
  };
};

const parseWatchTable = (input: unknown, index: number): RuntimeWatchTableView => {
  const value = record(input, `runtime watch table[${index}]`);
  exactKeys(value, ["id", "name", "rows"], `runtime watch table[${index}]`);
  return {
    id: requireUuid(value.id, "watch table identity"),
    name: string(value.name, "watch table name", 256),
    rows: array(value.rows, "watch rows", 4_096).map(parseWatchRow),
  };
};

const parseWatchRow = (input: unknown, index: number): RuntimeWatchRowView => {
  const value = record(input, `watch row[${index}]`);
  exactKeys(value, ["displayBase", "latestValue", "quality", "rowId", "targetId"], `watch row[${index}]`);
  return {
    displayBase: string(value.displayBase, "watch display base", 32),
    latestValue: value.latestValue === null ? null : parseRuntimeValue(value.latestValue, undefined, "watch value"),
    quality: nullableString(value.quality, "watch quality", 32),
    rowId: requireUuid(value.rowId, "watch row identity"),
    targetId: requireUuid(value.targetId, "watch target identity"),
  };
};

const parseRuntimeDiagnostic = (input: unknown, index: number): RuntimeDiagnosticView => {
  const value = record(input, `runtime diagnostic event[${index}]`);
  exactKeys(value, ["active", "code", "message", "navigationObjectId", "occurrenceId", "severity"], `runtime diagnostic event[${index}]`);
  return {
    active: boolean(value.active, "runtime diagnostic active state"),
    code: string(value.code, "runtime diagnostic code", 128),
    message: string(value.message, "runtime diagnostic message", 4_096),
    navigationObjectId: nullableUuid(value.navigationObjectId, "runtime diagnostic navigation identity"),
    occurrenceId: requireUuid(value.occurrenceId, "runtime diagnostic occurrence identity"),
    severity: oneOf(value.severity, ["INFO", "WARNING", "ERROR", "FATAL"] as const, "runtime diagnostic severity"),
  };
};

const parseHashes = (input: unknown): RuntimeHashView => {
  const value = record(input, "runtime hashes");
  exactKeys(value, ["controllerState", "diagnosticReplay", "runtimeReplay", "universeState"], "runtime hashes");
  return {
    controllerState: hash(value.controllerState, "controller state hash"),
    diagnosticReplay: hash(value.diagnosticReplay, "diagnostic replay hash"),
    runtimeReplay: hash(value.runtimeReplay, "runtime replay hash"),
    universeState: hash(value.universeState, "universe state hash"),
  };
};

const parsePreview = (input: unknown): VirtualLoadPreviewView => {
  const value = record(input, "load preview");
  exactKeys(value, [
    "blockerCount", "candidateFingerprint", "compatibility", "initializationCount",
    "previewFingerprint", "previewId", "removalCount", "requiresStop", "warningCount",
  ], "load preview");
  return {
    blockerCount: safeInteger(value.blockerCount, "load blocker count", 0, 1_000_000),
    candidateFingerprint: hash(value.candidateFingerprint, "load candidate fingerprint"),
    compatibility: string(value.compatibility, "load compatibility", 128),
    initializationCount: safeInteger(value.initializationCount, "load initialization count", 0, 1_000_000),
    previewFingerprint: hash(value.previewFingerprint, "load preview fingerprint"),
    previewId: requireUuid(value.previewId, "load preview identity"),
    removalCount: safeInteger(value.removalCount, "load removal count", 0, 1_000_000),
    requiresStop: boolean(value.requiresStop, "load requires STOP"),
    warningCount: safeInteger(value.warningCount, "load warning count", 0, 1_000_000),
  };
};

const parseRuntimeValue = (input: unknown, expected: RuntimeValueType | undefined, label: string): RuntimeValue => {
  const value = record(input, label);
  exactKeys(value, ["type", "value"], label);
  const type = parseValueType(value.type, `${label} type`);
  if (expected !== undefined && type !== expected) {
    throw new RuntimeWireError(`${label} does not match its declared target type.`);
  }
  if (type === "BOOL") {
    return { type, value: boolean(value.value, label) };
  }
  const canonical = string(value.value, label, 24);
  valueFields({ type, value: canonical });
  return { type, value: canonical };
};

const nullableRuntimeValue = (input: unknown, expected: RuntimeValueType, label: string): RuntimeValue | null =>
  input === null ? null : parseRuntimeValue(input, expected, label);

const parseValueType = (input: unknown, label: string): RuntimeValueType =>
  oneOf(input, ["BOOL", "I32", "I64", "U32", "TIME_MS"] as const, label);

type PlainRecord = Record<string, unknown>;

const record = (input: unknown, label: string): PlainRecord => {
  if (typeof input !== "object" || input === null || Array.isArray(input) || Object.getPrototypeOf(input) !== Object.prototype) {
    throw new RuntimeWireError(`${label} must be a plain record.`);
  }
  return input as PlainRecord;
};

const exactKeys = (value: PlainRecord, expected: readonly string[], label: string): void => {
  const actual = Object.keys(value).sort();
  const sortedExpected = [...expected].sort();
  if (actual.length !== sortedExpected.length || actual.some((key, index) => key !== sortedExpected[index])) {
    throw new RuntimeWireError(`${label} has an invalid field set.`);
  }
};

const array = (input: unknown, label: string, maximum: number): readonly unknown[] => {
  if (!Array.isArray(input) || input.length > maximum) {
    throw new RuntimeWireError(`${label} is outside its item budget.`);
  }
  return input;
};

const string = (input: unknown, label: string, maximum: number): string => {
  if (typeof input !== "string" || input.length < 1 || [...input].length > maximum || input.includes("\0")) {
    throw new RuntimeWireError(`${label} is invalid.`);
  }
  return input;
};

const nullableString = (input: unknown, label: string, maximum: number): string | null =>
  input === null ? null : string(input, label, maximum);

const boolean = (input: unknown, label: string): boolean => {
  if (typeof input !== "boolean") {
    throw new RuntimeWireError(`${label} must be boolean.`);
  }
  return input;
};

const oneOf = <T extends string>(input: unknown, values: readonly T[], label: string): T => {
  if (typeof input !== "string" || !values.includes(input as T)) {
    throw new RuntimeWireError(`${label} is unsupported.`);
  }
  return input as T;
};

const exactNumber = <T extends number>(input: unknown, expected: T, label: string): T => {
  if (input !== expected) {
    throw new RuntimeWireError(`${label} must equal ${expected}.`);
  }
  return expected;
};

const safeInteger = (input: unknown, label: string, minimum: number, maximum: number): number => {
  if (!Number.isSafeInteger(input) || (input as number) < minimum || (input as number) > maximum) {
    throw new RuntimeWireError(`${label} is outside its range.`);
  }
  return input as number;
};

const uint64 = (input: unknown, label: string): string => {
  if (typeof input !== "string" || !UINT64_PATTERN.test(input) || BigInt(input) > (1n << 64n) - 1n) {
    throw new RuntimeWireError(`${label} is not canonical uint64 text.`);
  }
  return input;
};

const requireUuid = (input: unknown, label: string): string => {
  if (typeof input !== "string" || !UUID_PATTERN.test(input)) {
    throw new RuntimeWireError(`${label} must be a canonical UUID.`);
  }
  return input.toLocaleLowerCase("en-US");
};

const nullableUuid = (input: unknown, label: string): string | null =>
  input === null ? null : requireUuid(input, label);

const hash = (input: unknown, label: string): string => {
  if (typeof input !== "string" || !HASH_PATTERN.test(input)) {
    throw new RuntimeWireError(`${label} must be a SHA-256 digest.`);
  }
  return input.toLocaleUpperCase("en-US");
};

const nullableHash = (input: unknown, label: string): string | null =>
  input === null ? null : hash(input, label);

const wireText = (input: unknown, label: string, maximum: number): string => {
  const value = string(input, label, maximum);
  if (/[\0\r\n\t]/u.test(value)) {
    throw new RuntimeWireError(`${label} contains a forbidden wire character.`);
  }
  return value;
};

