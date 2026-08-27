export const FOUNDATION_SCHEMA_VERSION = 1 as const;
export const FOUNDATION_COMMAND_KIND = "foundation.health" as const;
export const FOUNDATION_RESULT_KIND = "domain.result" as const;
export const FOUNDATION_REQUEST_ID = "phase1-foundation-health" as const;
export const FOUNDATION_HEALTHY_STATE = "HEALTHY" as const;
export const FOUNDATION_BUILD_IDENTITY = "foundation-core@0.1.0" as const;
export const FOUNDATION_STATE_HASH =
  "64E21C28C534606DD9C9AA27A56C928DC09574CD70B56B6D468FE3F96C2F5A94" as const;

const MAX_COMMAND_BYTES = 256;
const SHA256_PATTERN = /^[A-F0-9]{64}$/u;
const UNDO_TOKEN_PATTERN = /^[A-Za-z0-9_-]{1,96}$/u;

export type FoundationHealthCommand = Readonly<{
  kind: typeof FOUNDATION_COMMAND_KIND;
  requestId: typeof FOUNDATION_REQUEST_ID;
  schemaVersion: typeof FOUNDATION_SCHEMA_VERSION;
}>;

export type FoundationHealthValue = Readonly<{
  buildIdentity: typeof FOUNDATION_BUILD_IDENTITY;
  healthState: typeof FOUNDATION_HEALTHY_STATE;
  schemaVersion: typeof FOUNDATION_SCHEMA_VERSION;
  wasmSha256: string;
}>;

export type FoundationDiagnostic = Readonly<{
  code: "INVALID_COMMAND" | "INVALID_WASM" | "WORKER_FAILURE";
  message: string;
  severity: "error";
}>;

type FoundationDomainEnvelope = Readonly<{
  affectedObjectIds: readonly [];
  afterHash: typeof FOUNDATION_STATE_HASH;
  beforeHash: typeof FOUNDATION_STATE_HASH;
  events: readonly [];
  kind: typeof FOUNDATION_RESULT_KIND;
  requestId: typeof FOUNDATION_REQUEST_ID;
  schemaVersion: typeof FOUNDATION_SCHEMA_VERSION;
  undoToken?: string;
}>;

export type FoundationHealthSuccess = FoundationDomainEnvelope &
  Readonly<{
    diagnostics: readonly [];
    success: true;
    value: FoundationHealthValue;
  }>;

export type FoundationHealthFailure = FoundationDomainEnvelope &
  Readonly<{
    diagnostics: readonly [FoundationDiagnostic];
    success: false;
    value?: never;
  }>;

export type FoundationHealthResult =
  | FoundationHealthFailure
  | FoundationHealthSuccess;

export const createFoundationHealthCommand = (): FoundationHealthCommand => ({
  kind: FOUNDATION_COMMAND_KIND,
  requestId: FOUNDATION_REQUEST_ID,
  schemaVersion: FOUNDATION_SCHEMA_VERSION,
});

export const createFoundationFailure = (
  code: FoundationDiagnostic["code"],
  message: string,
): FoundationHealthFailure =>
  validateFoundationHealthResult({
    affectedObjectIds: [],
    afterHash: FOUNDATION_STATE_HASH,
    beforeHash: FOUNDATION_STATE_HASH,
    diagnostics: [{ code, message, severity: "error" }],
    events: [],
    kind: FOUNDATION_RESULT_KIND,
    requestId: FOUNDATION_REQUEST_ID,
    schemaVersion: FOUNDATION_SCHEMA_VERSION,
    success: false,
  }) as FoundationHealthFailure;

export const validateFoundationHealthCommand = (
  input: unknown,
): FoundationHealthCommand => {
  const record = requireExactRecord(input, ["kind", "requestId", "schemaVersion"]);
  const serializedBytes = utf8ByteLength(JSON.stringify(record));
  if (serializedBytes > MAX_COMMAND_BYTES) {
    throw new ContractValidationError("Foundation command exceeds its byte budget.");
  }
  if (
    record.kind !== FOUNDATION_COMMAND_KIND ||
    record.requestId !== FOUNDATION_REQUEST_ID ||
    record.schemaVersion !== FOUNDATION_SCHEMA_VERSION
  ) {
    throw new ContractValidationError("Unsupported foundation command.");
  }
  return createFoundationHealthCommand();
};

export const validateFoundationHealthResult = (
  input: unknown,
): FoundationHealthResult => {
  const record = requireRecord(input);
  const requiredKeys = [
    "affectedObjectIds",
    "afterHash",
    "beforeHash",
    "diagnostics",
    "events",
    "kind",
    "requestId",
    "schemaVersion",
    "success",
  ];
  if (record.success === true) {
    requiredKeys.push("value");
  } else if (record.success !== false) {
    throw new ContractValidationError("DomainResult must declare success.");
  }
  requireExactKeys(record, requiredKeys, ["undoToken"]);
  requireEnvelope(record);

  if (!isEmptyArray(record.events) || !isEmptyArray(record.affectedObjectIds)) {
    throw new ContractValidationError("Foundation DomainResult arrays must be empty.");
  }
  if (
    record.undoToken !== undefined &&
    (typeof record.undoToken !== "string" ||
      !UNDO_TOKEN_PATTERN.test(record.undoToken))
  ) {
    throw new ContractValidationError("Invalid DomainResult undo token.");
  }

  if (record.success) {
    if (!isEmptyArray(record.diagnostics)) {
      throw new ContractValidationError("Healthy DomainResult diagnostics must be empty.");
    }
    const value = requireExactRecord(record.value, [
      "buildIdentity",
      "healthState",
      "schemaVersion",
      "wasmSha256",
    ]);
    if (
      value.buildIdentity !== FOUNDATION_BUILD_IDENTITY ||
      value.healthState !== FOUNDATION_HEALTHY_STATE ||
      value.schemaVersion !== FOUNDATION_SCHEMA_VERSION ||
      typeof value.wasmSha256 !== "string" ||
      !SHA256_PATTERN.test(value.wasmSha256)
    ) {
      throw new ContractValidationError("Invalid foundation health value.");
    }
    return input as FoundationHealthSuccess;
  }

  if (!Array.isArray(record.diagnostics) || record.diagnostics.length !== 1) {
    throw new ContractValidationError("Failed DomainResult requires one diagnostic.");
  }
  validateDiagnostic(record.diagnostics[0]);
  return input as FoundationHealthFailure;
};

export class ContractValidationError extends Error {
  public constructor(message: string) {
    super(message);
    this.name = "ContractValidationError";
  }
}

const requireEnvelope = (record: Record<string, unknown>): void => {
  if (
    record.kind !== FOUNDATION_RESULT_KIND ||
    record.requestId !== FOUNDATION_REQUEST_ID ||
    record.schemaVersion !== FOUNDATION_SCHEMA_VERSION ||
    record.beforeHash !== FOUNDATION_STATE_HASH ||
    record.afterHash !== FOUNDATION_STATE_HASH ||
    record.beforeHash !== record.afterHash
  ) {
    throw new ContractValidationError("Invalid foundation DomainResult envelope.");
  }
};

const validateDiagnostic = (input: unknown): void => {
  const diagnostic = requireExactRecord(input, ["code", "message", "severity"]);
  if (
    !["INVALID_COMMAND", "INVALID_WASM", "WORKER_FAILURE"].includes(
      String(diagnostic.code),
    ) ||
    typeof diagnostic.message !== "string" ||
    diagnostic.message.length < 1 ||
    diagnostic.message.length > 160 ||
    diagnostic.severity !== "error"
  ) {
    throw new ContractValidationError("Invalid foundation diagnostic.");
  }
};

const isEmptyArray = (input: unknown): input is readonly [] =>
  Array.isArray(input) && input.length === 0;

const utf8ByteLength = (input: string): number => {
  let bytes = 0;
  for (const character of input) {
    const codePoint = character.codePointAt(0);
    if (codePoint === undefined) {
      continue;
    }
    bytes +=
      codePoint <= 0x7f
        ? 1
        : codePoint <= 0x7ff
          ? 2
          : codePoint <= 0xffff
            ? 3
            : 4;
  }
  return bytes;
};

const requireExactRecord = (
  input: unknown,
  keys: readonly string[],
): Record<string, unknown> => {
  const record = requireRecord(input);
  requireExactKeys(record, keys);
  return record;
};

const requireRecord = (input: unknown): Record<string, unknown> => {
  if (
    typeof input !== "object" ||
    input === null ||
    Array.isArray(input) ||
    Object.getPrototypeOf(input) !== Object.prototype
  ) {
    throw new ContractValidationError("Expected a plain object.");
  }
  return input as Record<string, unknown>;
};

const requireExactKeys = (
  record: Record<string, unknown>,
  requiredKeys: readonly string[],
  optionalKeys: readonly string[] = [],
): void => {
  const actual = Object.keys(record);
  const allowed = new Set([...requiredKeys, ...optionalKeys]);
  if (
    requiredKeys.some((key) => !Object.hasOwn(record, key)) ||
    actual.some((key) => !allowed.has(key))
  ) {
    throw new ContractValidationError("Object contains missing or unknown fields.");
  }
};
