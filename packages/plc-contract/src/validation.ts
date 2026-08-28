import {
  FOUNDATION_COMPATIBILITY,
  PLC_CONTRACT_LIMITS,
  PLC_CONTRACT_SCHEMA_VERSION,
  PLC_MESSAGE_KIND,
} from "./model";
import type {
  BlockInterfaceContract,
  BuildArtifactIdentity,
  BuildAttemptRecord,
  BuildReceipt,
  BuildReportRecord,
  CanonicalTypedValue,
  CommandContext,
  CommandResultBody,
  CommissioningReceipt,
  Diagnostic,
  DiagnosticParameter,
  DomainCommand,
  DomainCommandMessage,
  DomainEventMessage,
  DomainEventRecord,
  DomainQuery,
  DomainQueryMessage,
  DomainReceipt,
  DomainResultMessage,
  FoundationHealthCompatibilityCommand,
  FoundationHealthCompatibilityResult,
  GraphSourceAnchor,
  HardwareReceipt,
  HealthReceipt,
  MonitoringReceipt,
  ObservedValue,
  PersistenceReceipt,
  Phase2PlcMessage,
  PlcWireMessage,
  ProgramReceipt,
  ProjectReceipt,
  QueryContext,
  QueryResultBody,
  RuntimeReceipt,
  SemanticGraphContract,
  SourceAnchor,
} from "./model";

const UUID_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/u;
const SHA256_PATTERN = /^[A-F0-9]{64}$/u;
const DECIMAL_UNSIGNED_PATTERN = /^(?:0|[1-9][0-9]*)$/u;
const DECIMAL_SIGNED_PATTERN = /^(?:0|-?[1-9][0-9]*)$/u;
const IDEMPOTENCY_KEY_PATTERN = /^[A-Za-z0-9_-]{1,96}$/u;
const UNDO_TOKEN_PATTERN = /^[A-Za-z0-9_-]{1,192}$/u;
const CANONICAL_TYPE_ID_PATTERN = /^[A-Za-z][A-Za-z0-9_.:\-\[\]]{0,127}$/u;
const REGISTRY_ID_PATTERN = /^[A-Za-z][A-Za-z0-9_.:\-]{0,127}$/u;
const DIAGNOSTIC_CODE_PATTERN = /^EDU-[A-Z]{2,8}-[0-9]{4}$/u;
const VERSION_PATTERN = /^[A-Za-z0-9][A-Za-z0-9.+_-]{0,63}$/u;
const UINT64_MAX = (1n << 64n) - 1n;
const INT64_MIN = -(1n << 63n);
const INT64_MAX = (1n << 63n) - 1n;

const SIGNED_LIMITS = {
  DINT: [-(1n << 31n), (1n << 31n) - 1n],
  INT: [-(1n << 15n), (1n << 15n) - 1n],
  LINT: [INT64_MIN, INT64_MAX],
  SINT: [-(1n << 7n), (1n << 7n) - 1n],
} as const;

const UNSIGNED_LIMITS = {
  UDINT: [0n, (1n << 32n) - 1n],
  UINT: [0n, (1n << 16n) - 1n],
  ULINT: [0n, UINT64_MAX],
  USINT: [0n, (1n << 8n) - 1n],
} as const;

const BIT_HEX_LENGTHS = {
  BYTE: 2,
  DWORD: 8,
  LWORD: 16,
  WORD: 4,
} as const;

export class PlcContractValidationError extends Error {
  public readonly path: string;

  public constructor(path: string, message: string) {
    super(`${path}: ${message}`);
    this.name = "PlcContractValidationError";
    this.path = path;
  }
}

type PlainRecord = Record<string, unknown>;

function fail(path: string, message: string): never {
  throw new PlcContractValidationError(path, message);
}

const hasOwn = (record: PlainRecord, key: string): boolean =>
  Object.hasOwn(record, key);

const requireRecord = (input: unknown, path: string): PlainRecord => {
  if (
    typeof input !== "object" ||
    input === null ||
    Array.isArray(input) ||
    Object.getPrototypeOf(input) !== Object.prototype
  ) {
    fail(path, "expected a plain object");
  }
  return input as PlainRecord;
};

const requireExactKeys = (
  record: PlainRecord,
  required: readonly string[],
  path: string,
  optional: readonly string[] = [],
): void => {
  const allowed = new Set([...required, ...optional]);
  for (const key of required) {
    if (!hasOwn(record, key)) {
      fail(path, `missing required key ${JSON.stringify(key)}`);
    }
  }
  for (const key of Object.keys(record)) {
    if (!allowed.has(key)) {
      fail(path, `unknown key ${JSON.stringify(key)}`);
    }
  }
};

const requireString = (
  input: unknown,
  path: string,
  maximumCharacters: number,
  allowEmpty = false,
): string => {
  if (typeof input !== "string") {
    fail(path, "expected a string");
  }
  const characters = [...input].length;
  if ((!allowEmpty && characters === 0) || characters > maximumCharacters) {
    fail(path, `string length must be ${allowEmpty ? "0" : "1"}..${maximumCharacters}`);
  }
  if (input.includes("\0")) {
    fail(path, "NUL is not allowed");
  }
  return input;
};

const requireEnum = <T extends string>(
  input: unknown,
  allowed: readonly T[],
  path: string,
): T => {
  if (typeof input !== "string" || !allowed.includes(input as T)) {
    fail(path, `expected one of ${allowed.join(", ")}`);
  }
  return input as T;
};

const requireBoolean = (input: unknown, path: string): boolean => {
  if (typeof input !== "boolean") {
    fail(path, "expected a boolean");
  }
  return input;
};

const requireNull = (input: unknown, path: string): null => {
  if (input !== null) {
    fail(path, "expected null");
  }
  return null;
};

const requireSafeInteger = (
  input: unknown,
  path: string,
  minimum: number,
  maximum: number,
): number => {
  if (
    typeof input !== "number" ||
    !Number.isSafeInteger(input) ||
    input < minimum ||
    input > maximum
  ) {
    fail(path, `expected a safe integer in ${minimum}..${maximum}`);
  }
  return input;
};

const requireArray = (
  input: unknown,
  path: string,
  maximumLength: number,
  minimumLength = 0,
): readonly unknown[] => {
  if (!Array.isArray(input)) {
    fail(path, "expected an array");
  }
  if (input.length < minimumLength || input.length > maximumLength) {
    fail(path, `array length must be ${minimumLength}..${maximumLength}`);
  }
  return input;
};

const requireUuid = (input: unknown, path: string): string => {
  if (typeof input !== "string" || !UUID_PATTERN.test(input)) {
    fail(path, "expected a canonical lowercase UUID");
  }
  return input;
};

const requireSha256 = (input: unknown, path: string): string => {
  if (typeof input !== "string" || !SHA256_PATTERN.test(input)) {
    fail(path, "expected an uppercase SHA-256 digest");
  }
  return input;
};

const requireDecimal = (
  input: unknown,
  path: string,
  minimum: bigint,
  maximum: bigint,
): string => {
  if (
    typeof input !== "string" ||
    !(minimum < 0n ? DECIMAL_SIGNED_PATTERN : DECIMAL_UNSIGNED_PATTERN).test(input)
  ) {
    fail(path, "expected a canonical decimal integer string");
  }
  const value = BigInt(input);
  if (value < minimum || value > maximum) {
    fail(path, `decimal value is outside ${minimum}..${maximum}`);
  }
  return input;
};

const requireUInt64 = (input: unknown, path: string): string =>
  requireDecimal(input, path, 0n, UINT64_MAX);

const requireInt64 = (input: unknown, path: string): string =>
  requireDecimal(input, path, INT64_MIN, INT64_MAX);

const requireIdempotencyKey = (input: unknown, path: string): string => {
  if (typeof input !== "string" || !IDEMPOTENCY_KEY_PATTERN.test(input)) {
    fail(path, "invalid idempotency key");
  }
  return input;
};

const requireUndoToken = (input: unknown, path: string): string => {
  if (typeof input !== "string" || !UNDO_TOKEN_PATTERN.test(input)) {
    fail(path, "invalid undo token");
  }
  return input;
};

const requireCanonicalTypeId = (input: unknown, path: string): string => {
  if (typeof input !== "string" || !CANONICAL_TYPE_ID_PATTERN.test(input)) {
    fail(path, "invalid canonical type identity");
  }
  return input;
};

const requireRegistryId = (input: unknown, path: string): string => {
  if (typeof input !== "string" || !REGISTRY_ID_PATTERN.test(input)) {
    fail(path, "invalid stable registry identity");
  }
  return input;
};

const requireVersion = (input: unknown, path: string): string => {
  if (typeof input !== "string" || !VERSION_PATTERN.test(input)) {
    fail(path, "invalid version token");
  }
  return input;
};

const requireUniqueStrings = (
  values: readonly string[],
  path: string,
): void => {
  const seen = new Set<string>();
  for (const value of values) {
    if (seen.has(value)) {
      fail(path, `duplicate identity ${value}`);
    }
    seen.add(value);
  }
};

const validateUuidArray = (
  input: unknown,
  path: string,
  maximumLength: number = PLC_CONTRACT_LIMITS.objectIds,
): readonly string[] => {
  const values = requireArray(input, path, maximumLength).map((value, index) =>
    requireUuid(value, `${path}[${index}]`),
  );
  requireUniqueStrings(values, path);
  return values;
};

const validateNullable = <T>(
  input: unknown,
  path: string,
  validator: (value: unknown, valuePath: string) => T,
): T | null => (input === null ? null : validator(input, path));

const validateCanonicalFloat = (
  typeId: "REAL" | "LREAL",
  input: unknown,
  path: string,
): void => {
  const digits = typeId === "REAL" ? 8 : 16;
  if (typeof input !== "string" || !new RegExp(`^[A-F0-9]{${digits}}$`, "u").test(input)) {
    fail(path, `expected ${digits} uppercase hexadecimal digits`);
  }
  const bits = BigInt(`0x${input}`);
  if (typeId === "REAL") {
    const exponent = (bits >> 23n) & 0xffn;
    const fraction = bits & 0x7fffffn;
    if (exponent === 0xffn && fraction !== 0n && input !== "7FC00000") {
      fail(path, "REAL NaN must use the canonical 7FC00000 encoding");
    }
  } else {
    const exponent = (bits >> 52n) & 0x7ffn;
    const fraction = bits & 0xfffffffffffffn;
    if (
      exponent === 0x7ffn &&
      fraction !== 0n &&
      input !== "7FF8000000000000"
    ) {
      fail(path, "LREAL NaN must use the canonical 7FF8000000000000 encoding");
    }
  }
};

export const validateCanonicalTypedValue = (
  input: unknown,
  path = "$value",
  depth = 0,
): CanonicalTypedValue => {
  if (depth > PLC_CONTRACT_LIMITS.typedValueDepth) {
    fail(path, "typed value nesting limit exceeded");
  }
  const record = requireRecord(input, path);
  const kind = requireString(record.kind, `${path}.kind`, 32);
  switch (kind) {
    case "bool": {
      requireExactKeys(record, ["kind", "typeId", "value"], path);
      requireEnum(record.typeId, ["BOOL"], `${path}.typeId`);
      requireBoolean(record.value, `${path}.value`);
      break;
    }
    case "signed-integer": {
      requireExactKeys(record, ["kind", "typeId", "value"], path);
      const typeId = requireEnum(
        record.typeId,
        ["SINT", "INT", "DINT", "LINT"],
        `${path}.typeId`,
      );
      const [minimum, maximum] = SIGNED_LIMITS[typeId];
      requireDecimal(record.value, `${path}.value`, minimum, maximum);
      break;
    }
    case "unsigned-integer": {
      requireExactKeys(record, ["kind", "typeId", "value"], path);
      const typeId = requireEnum(
        record.typeId,
        ["USINT", "UINT", "UDINT", "ULINT"],
        `${path}.typeId`,
      );
      const [minimum, maximum] = UNSIGNED_LIMITS[typeId];
      requireDecimal(record.value, `${path}.value`, minimum, maximum);
      break;
    }
    case "bit-string": {
      requireExactKeys(record, ["bitsHex", "kind", "typeId"], path);
      const typeId = requireEnum(
        record.typeId,
        ["BYTE", "WORD", "DWORD", "LWORD"],
        `${path}.typeId`,
      );
      const digits = BIT_HEX_LENGTHS[typeId];
      if (
        typeof record.bitsHex !== "string" ||
        !new RegExp(`^[A-F0-9]{${digits}}$`, "u").test(record.bitsHex)
      ) {
        fail(`${path}.bitsHex`, `expected ${digits} uppercase hexadecimal digits`);
      }
      break;
    }
    case "floating": {
      requireExactKeys(record, ["ieeeHex", "kind", "typeId"], path);
      const typeId = requireEnum(record.typeId, ["REAL", "LREAL"], `${path}.typeId`);
      validateCanonicalFloat(typeId, record.ieeeHex, `${path}.ieeeHex`);
      break;
    }
    case "char": {
      requireExactKeys(record, ["codeUnit", "kind", "typeId"], path);
      requireEnum(record.typeId, ["CHAR"], `${path}.typeId`);
      requireSafeInteger(record.codeUnit, `${path}.codeUnit`, 0, 255);
      break;
    }
    case "string": {
      requireExactKeys(record, ["capacity", "codeUnits", "kind", "typeId"], path);
      requireEnum(record.typeId, ["STRING"], `${path}.typeId`);
      const capacity = requireSafeInteger(
        record.capacity,
        `${path}.capacity`,
        0,
        PLC_CONTRACT_LIMITS.stringCodeUnits,
      );
      const codeUnits = requireArray(
        record.codeUnits,
        `${path}.codeUnits`,
        Math.min(capacity, PLC_CONTRACT_LIMITS.stringCodeUnits),
      );
      for (const [index, codeUnit] of codeUnits.entries()) {
        requireSafeInteger(codeUnit, `${path}.codeUnits[${index}]`, 0, 255);
      }
      break;
    }
    case "time": {
      requireExactKeys(record, ["kind", "milliseconds", "typeId"], path);
      requireEnum(record.typeId, ["TIME"], `${path}.typeId`);
      requireInt64(record.milliseconds, `${path}.milliseconds`);
      break;
    }
    case "array": {
      requireExactKeys(record, ["bounds", "elements", "kind", "typeId"], path);
      requireCanonicalTypeId(record.typeId, `${path}.typeId`);
      const bounds = requireArray(record.bounds, `${path}.bounds`, 8, 1);
      let elementCount = 1;
      for (const [index, boundInput] of bounds.entries()) {
        const bound = requireRecord(boundInput, `${path}.bounds[${index}]`);
        requireExactKeys(bound, ["lower", "upper"], `${path}.bounds[${index}]`);
        const lower = requireSafeInteger(
          bound.lower,
          `${path}.bounds[${index}].lower`,
          -2_147_483_648,
          2_147_483_647,
        );
        const upper = requireSafeInteger(
          bound.upper,
          `${path}.bounds[${index}].upper`,
          -2_147_483_648,
          2_147_483_647,
        );
        if (upper < lower) {
          fail(`${path}.bounds[${index}]`, "upper must not be less than lower");
        }
        elementCount *= upper - lower + 1;
        if (elementCount > PLC_CONTRACT_LIMITS.typedValueElements) {
          fail(`${path}.bounds`, "declared array shape exceeds the element limit");
        }
      }
      const elements = requireArray(
        record.elements,
        `${path}.elements`,
        PLC_CONTRACT_LIMITS.typedValueElements,
      );
      if (elements.length !== elementCount) {
        fail(`${path}.elements`, `expected exactly ${elementCount} elements`);
      }
      for (const [index, element] of elements.entries()) {
        validateCanonicalTypedValue(element, `${path}.elements[${index}]`, depth + 1);
      }
      break;
    }
    case "struct": {
      requireExactKeys(record, ["kind", "members", "typeId"], path);
      requireCanonicalTypeId(record.typeId, `${path}.typeId`);
      const members = requireArray(
        record.members,
        `${path}.members`,
        PLC_CONTRACT_LIMITS.typedValueElements,
      );
      const memberIds: string[] = [];
      for (const [index, memberInput] of members.entries()) {
        const memberPath = `${path}.members[${index}]`;
        const member = requireRecord(memberInput, memberPath);
        requireExactKeys(member, ["memberId", "value"], memberPath);
        memberIds.push(requireUuid(member.memberId, `${memberPath}.memberId`));
        validateCanonicalTypedValue(member.value, `${memberPath}.value`, depth + 1);
      }
      requireUniqueStrings(memberIds, `${path}.members`);
      break;
    }
    case "instruction-state": {
      requireExactKeys(record, ["kind", "stateFingerprint", "typeId"], path);
      requireEnum(
        record.typeId,
        ["EdgeState", "TimerState", "CounterState"],
        `${path}.typeId`,
      );
      requireSha256(record.stateFingerprint, `${path}.stateFingerprint`);
      break;
    }
    default:
      fail(`${path}.kind`, `unknown canonical value kind ${JSON.stringify(kind)}`);
  }
  return input as CanonicalTypedValue;
};

const validateUtf8ByteRange = (input: unknown, path: string): void => {
  const range = requireRecord(input, path);
  requireExactKeys(range, ["endExclusive", "start"], path);
  const start = requireSafeInteger(
    range.start,
    `${path}.start`,
    0,
    PLC_CONTRACT_LIMITS.sourceRangeBytes,
  );
  const end = requireSafeInteger(
    range.endExclusive,
    `${path}.endExclusive`,
    0,
    PLC_CONTRACT_LIMITS.sourceRangeBytes,
  );
  if (end < start) {
    fail(path, "endExclusive must not precede start");
  }
};

export const validateSourceAnchor = (
  input: unknown,
  path = "$anchor",
  allowGenerated = true,
): SourceAnchor => {
  const record = requireRecord(input, path);
  const kind = requireEnum(
    record.anchorKind,
    ["project", "text", "graph", "generated"],
    `${path}.anchorKind`,
  );
  switch (kind) {
    case "project": {
      requireExactKeys(
        record,
        ["anchorKind", "ownerObjectId", "propertyPath", "sourceRevisionHash"],
        path,
      );
      requireUuid(record.ownerObjectId, `${path}.ownerObjectId`);
      requireSha256(record.sourceRevisionHash, `${path}.sourceRevisionHash`);
      const propertyPath = requireArray(record.propertyPath, `${path}.propertyPath`, 32);
      for (const [index, segment] of propertyPath.entries()) {
        requireString(segment, `${path}.propertyPath[${index}]`, 128);
      }
      break;
    }
    case "text": {
      requireExactKeys(
        record,
        ["anchorKind", "language", "ownerObjectId", "range", "sourceRevisionHash"],
        path,
        ["semanticNodeId"],
      );
      requireEnum(record.language, ["SCL"], `${path}.language`);
      requireUuid(record.ownerObjectId, `${path}.ownerObjectId`);
      requireSha256(record.sourceRevisionHash, `${path}.sourceRevisionHash`);
      validateUtf8ByteRange(record.range, `${path}.range`);
      if (hasOwn(record, "semanticNodeId")) {
        requireUuid(record.semanticNodeId, `${path}.semanticNodeId`);
      }
      break;
    }
    case "graph": {
      requireExactKeys(
        record,
        [
          "anchorKind",
          "language",
          "networkId",
          "ownerObjectId",
          "sourceRevisionHash",
        ],
        path,
        ["edgeId", "nodeId", "portId", "semanticNodeId"],
      );
      requireEnum(record.language, ["LAD", "FBD"], `${path}.language`);
      requireUuid(record.networkId, `${path}.networkId`);
      requireUuid(record.ownerObjectId, `${path}.ownerObjectId`);
      requireSha256(record.sourceRevisionHash, `${path}.sourceRevisionHash`);
      for (const key of ["edgeId", "nodeId", "portId", "semanticNodeId"] as const) {
        if (hasOwn(record, key)) {
          requireUuid(record[key], `${path}.${key}`);
        }
      }
      break;
    }
    case "generated": {
      if (!allowGenerated) {
        fail(path, "generated anchors cannot recursively cause generated anchors");
      }
      requireExactKeys(
        record,
        ["anchorKind", "causalAnchor", "ownerObjectId", "sourceRevisionHash"],
        path,
      );
      requireUuid(record.ownerObjectId, `${path}.ownerObjectId`);
      requireSha256(record.sourceRevisionHash, `${path}.sourceRevisionHash`);
      validateSourceAnchor(record.causalAnchor, `${path}.causalAnchor`, false);
      break;
    }
  }
  return input as SourceAnchor;
};

const validateDiagnosticParameter = (
  input: unknown,
  path: string,
): DiagnosticParameter => {
  const record = requireRecord(input, path);
  requireExactKeys(record, ["kind", "name", "value"], path);
  const kind = requireEnum(
    record.kind,
    ["boolean", "decimal", "hash", "identity", "text"],
    `${path}.kind`,
  );
  requireRegistryId(record.name, `${path}.name`);
  switch (kind) {
    case "boolean":
      requireBoolean(record.value, `${path}.value`);
      break;
    case "decimal":
      requireInt64(record.value, `${path}.value`);
      break;
    case "hash":
      requireSha256(record.value, `${path}.value`);
      break;
    case "identity":
      requireUuid(record.value, `${path}.value`);
      break;
    case "text":
      requireString(record.value, `${path}.value`, 512, true);
      break;
  }
  return input as DiagnosticParameter;
};

export const validateDiagnostic = (
  input: unknown,
  path = "$diagnostic",
): Diagnostic => {
  const record = requireRecord(input, path);
  requireExactKeys(
    record,
    [
      "blocking",
      "cause",
      "code",
      "diagnosticId",
      "parameters",
      "phase",
      "primaryAnchor",
      "recoveryHint",
      "relatedAnchors",
      "severity",
    ],
    path,
    ["buildAttemptId", "snapshotHash"],
  );
  requireBoolean(record.blocking, `${path}.blocking`);
  requireString(record.cause, `${path}.cause`, 2_048);
  if (typeof record.code !== "string" || !DIAGNOSTIC_CODE_PATTERN.test(record.code)) {
    fail(`${path}.code`, "invalid original EDU diagnostic code");
  }
  requireUuid(record.diagnosticId, `${path}.diagnosticId`);
  requireRegistryId(record.phase, `${path}.phase`);
  requireString(record.recoveryHint, `${path}.recoveryHint`, 2_048);
  requireEnum(
    record.severity,
    ["Info", "Warning", "Error", "Internal"],
    `${path}.severity`,
  );
  if (hasOwn(record, "buildAttemptId")) {
    requireUuid(record.buildAttemptId, `${path}.buildAttemptId`);
  }
  if (hasOwn(record, "snapshotHash")) {
    requireSha256(record.snapshotHash, `${path}.snapshotHash`);
  }
  validateSourceAnchor(record.primaryAnchor, `${path}.primaryAnchor`);
  const parameters = requireArray(record.parameters, `${path}.parameters`, 256);
  const parameterNames: string[] = [];
  for (const [index, parameter] of parameters.entries()) {
    validateDiagnosticParameter(parameter, `${path}.parameters[${index}]`);
    parameterNames.push((parameter as DiagnosticParameter).name);
  }
  requireUniqueStrings(parameterNames, `${path}.parameters`);
  const related = requireArray(
    record.relatedAnchors,
    `${path}.relatedAnchors`,
    PLC_CONTRACT_LIMITS.relatedAnchors,
  );
  for (const [index, anchor] of related.entries()) {
    validateSourceAnchor(anchor, `${path}.relatedAnchors[${index}]`);
  }
  return input as Diagnostic;
};

const validateDiagnosticArray = (input: unknown, path: string): readonly Diagnostic[] => {
  const diagnostics = requireArray(
    input,
    path,
    PLC_CONTRACT_LIMITS.diagnosticCount,
  );
  const ids: string[] = [];
  for (const [index, diagnostic] of diagnostics.entries()) {
    const validated = validateDiagnostic(diagnostic, `${path}[${index}]`);
    ids.push(validated.diagnosticId);
  }
  requireUniqueStrings(ids, path);
  return diagnostics as readonly Diagnostic[];
};

const validateCommandContext = (input: unknown, path: string): CommandContext => {
  const record = requireRecord(input, path);
  requireExactKeys(
    record,
    [
      "commandId",
      "expectedObjectRevisions",
      "expectedProjectRevision",
      "idempotencyKey",
      "issuedSequence",
      "transactionId",
    ],
    path,
  );
  requireUuid(record.commandId, `${path}.commandId`);
  requireUuid(record.transactionId, `${path}.transactionId`);
  requireIdempotencyKey(record.idempotencyKey, `${path}.idempotencyKey`);
  requireUInt64(record.issuedSequence, `${path}.issuedSequence`);
  requireUInt64(record.expectedProjectRevision, `${path}.expectedProjectRevision`);
  const revisions = requireArray(
    record.expectedObjectRevisions,
    `${path}.expectedObjectRevisions`,
    PLC_CONTRACT_LIMITS.objectIds,
  );
  const ids: string[] = [];
  for (const [index, revisionInput] of revisions.entries()) {
    const revisionPath = `${path}.expectedObjectRevisions[${index}]`;
    const revision = requireRecord(revisionInput, revisionPath);
    requireExactKeys(revision, ["objectId", "revision"], revisionPath);
    ids.push(requireUuid(revision.objectId, `${revisionPath}.objectId`));
    requireUInt64(revision.revision, `${revisionPath}.revision`);
  }
  requireUniqueStrings(ids, `${path}.expectedObjectRevisions`);
  return input as CommandContext;
};

const validateQueryContext = (input: unknown, path: string): QueryContext => {
  const record = requireRecord(input, path);
  requireExactKeys(record, ["consistency", "queryId"], path, ["atProjectRevision"]);
  requireEnum(record.consistency, ["current", "captured"], `${path}.consistency`);
  requireUuid(record.queryId, `${path}.queryId`);
  if (hasOwn(record, "atProjectRevision")) {
    requireUInt64(record.atProjectRevision, `${path}.atProjectRevision`);
  }
  if (record.consistency === "captured" && !hasOwn(record, "atProjectRevision")) {
    fail(path, "captured consistency requires atProjectRevision");
  }
  if (record.consistency === "current" && hasOwn(record, "atProjectRevision")) {
    fail(path, "current consistency cannot select a captured project revision");
  }
  return input as QueryContext;
};

const validateInterfaceMemberBase = (
  record: PlainRecord,
  path: string,
): void => {
  requireUuid(record.id, `${path}.id`);
  requireUInt64(record.declaredOrder, `${path}.declaredOrder`);
  requireString(record.name, `${path}.name`, PLC_CONTRACT_LIMITS.identifierCharacters);
  requireCanonicalTypeId(record.typeId, `${path}.typeId`);
  requireString(record.comment, `${path}.comment`, 4_096, true);
};

const validateInterfaceMemberArray = (
  input: unknown,
  path: string,
  role:
    | "Input"
    | "Output"
    | "InOut"
    | "Static"
    | "Temp"
    | "Constant",
): readonly unknown[] => {
  const members = requireArray(
    input,
    path,
    PLC_CONTRACT_LIMITS.interfaceMembersPerRole,
  );
  const ids: string[] = [];
  for (const [index, memberInput] of members.entries()) {
    const memberPath = `${path}[${index}]`;
    const member = requireRecord(memberInput, memberPath);
    switch (role) {
      case "Input":
        requireExactKeys(
          member,
          ["comment", "declaredOrder", "id", "name", "role", "typeId"],
          memberPath,
          ["defaultValue"],
        );
        if (hasOwn(member, "defaultValue")) {
          validateCanonicalTypedValue(member.defaultValue, `${memberPath}.defaultValue`);
        }
        break;
      case "Output":
        requireExactKeys(
          member,
          [
            "comment",
            "declaredOrder",
            "id",
            "name",
            "requiredOutputBinding",
            "role",
            "typeId",
          ],
          memberPath,
          ["startValue"],
        );
        requireBoolean(
          member.requiredOutputBinding,
          `${memberPath}.requiredOutputBinding`,
        );
        if (hasOwn(member, "startValue")) {
          validateCanonicalTypedValue(member.startValue, `${memberPath}.startValue`);
        }
        break;
      case "InOut":
      case "Temp":
        requireExactKeys(
          member,
          ["comment", "declaredOrder", "id", "name", "role", "typeId"],
          memberPath,
        );
        break;
      case "Static":
        requireExactKeys(
          member,
          ["comment", "declaredOrder", "id", "name", "role", "typeId"],
          memberPath,
          ["retainPolicy", "startValue"],
        );
        if (hasOwn(member, "retainPolicy")) {
          requireEnum(
            member.retainPolicy,
            ["Retentive", "NonRetentive"],
            `${memberPath}.retainPolicy`,
          );
        }
        if (hasOwn(member, "startValue")) {
          validateCanonicalTypedValue(member.startValue, `${memberPath}.startValue`);
        }
        break;
      case "Constant":
        requireExactKeys(
          member,
          ["comment", "declaredOrder", "id", "name", "role", "typeId", "value"],
          memberPath,
        );
        validateCanonicalTypedValue(member.value, `${memberPath}.value`);
        break;
    }
    requireEnum(member.role, [role], `${memberPath}.role`);
    validateInterfaceMemberBase(member, memberPath);
    ids.push(member.id as string);
  }
  requireUniqueStrings(ids, path);
  return members;
};

const validateBlockInterface = (
  input: unknown,
  path: string,
): BlockInterfaceContract => {
  const record = requireRecord(input, path);
  requireExactKeys(
    record,
    ["constants", "inOuts", "inputs", "outputs", "return", "statics", "temps"],
    path,
  );
  const memberGroups = [
    validateInterfaceMemberArray(record.inputs, `${path}.inputs`, "Input"),
    validateInterfaceMemberArray(record.outputs, `${path}.outputs`, "Output"),
    validateInterfaceMemberArray(record.inOuts, `${path}.inOuts`, "InOut"),
    validateInterfaceMemberArray(record.statics, `${path}.statics`, "Static"),
    validateInterfaceMemberArray(record.temps, `${path}.temps`, "Temp"),
    validateInterfaceMemberArray(record.constants, `${path}.constants`, "Constant"),
  ];
  const allIds = memberGroups.flatMap((group) =>
    group.map((member) => (member as PlainRecord).id as string),
  );
  if (record.return !== null) {
    const returnRecord = requireRecord(record.return, `${path}.return`);
    requireExactKeys(
      returnRecord,
      ["comment", "id", "name", "role", "typeId"],
      `${path}.return`,
    );
    requireUuid(returnRecord.id, `${path}.return.id`);
    requireString(
      returnRecord.name,
      `${path}.return.name`,
      PLC_CONTRACT_LIMITS.identifierCharacters,
    );
    requireEnum(returnRecord.role, ["Return"], `${path}.return.role`);
    requireCanonicalTypeId(returnRecord.typeId, `${path}.return.typeId`);
    requireString(returnRecord.comment, `${path}.return.comment`, 4_096, true);
    allIds.push(returnRecord.id as string);
  }
  requireUniqueStrings(allIds, path);
  return input as BlockInterfaceContract;
};

const validateSemanticGraph = (
  input: unknown,
  path: string,
): SemanticGraphContract => {
  const graph = requireRecord(input, path);
  requireExactKeys(graph, ["language", "networks", "sourceRevisionHash"], path);
  requireEnum(graph.language, ["LAD", "FBD"], `${path}.language`);
  requireSha256(graph.sourceRevisionHash, `${path}.sourceRevisionHash`);
  const networks = requireArray(
    graph.networks,
    `${path}.networks`,
    PLC_CONTRACT_LIMITS.graphNetworks,
  );
  const networkIds: string[] = [];
  const allNodeIds: string[] = [];
  const allPortIds: string[] = [];
  const allEdgeIds: string[] = [];
  const knownPortIds = new Set<string>();
  const pendingEdges: Array<Readonly<{ path: string; source: string; target: string }>> = [];
  for (const [networkIndex, networkInput] of networks.entries()) {
    const networkPath = `${path}.networks[${networkIndex}]`;
    const network = requireRecord(networkInput, networkPath);
    requireExactKeys(network, ["edges", "id", "nodes", "semanticOrder"], networkPath);
    networkIds.push(requireUuid(network.id, `${networkPath}.id`));
    requireUInt64(network.semanticOrder, `${networkPath}.semanticOrder`);
    const nodes = requireArray(
      network.nodes,
      `${networkPath}.nodes`,
      PLC_CONTRACT_LIMITS.graphNodes,
    );
    for (const [nodeIndex, nodeInput] of nodes.entries()) {
      const nodePath = `${networkPath}.nodes[${nodeIndex}]`;
      const node = requireRecord(nodeInput, nodePath);
      requireExactKeys(
        node,
        ["definitionId", "id", "ports", "semanticOrder", "stateBindingId"],
        nodePath,
      );
      allNodeIds.push(requireUuid(node.id, `${nodePath}.id`));
      requireRegistryId(node.definitionId, `${nodePath}.definitionId`);
      requireUInt64(node.semanticOrder, `${nodePath}.semanticOrder`);
      validateNullable(node.stateBindingId, `${nodePath}.stateBindingId`, requireUuid);
      const ports = requireArray(
        node.ports,
        `${nodePath}.ports`,
        PLC_CONTRACT_LIMITS.graphPortsPerNode,
      );
      for (const [portIndex, portInput] of ports.entries()) {
        const portPath = `${nodePath}.ports[${portIndex}]`;
        const port = requireRecord(portInput, portPath);
        requireExactKeys(
          port,
          ["direction", "id", "required", "role", "typeId"],
          portPath,
        );
        requireEnum(port.direction, ["input", "output"], `${portPath}.direction`);
        const portId = requireUuid(port.id, `${portPath}.id`);
        allPortIds.push(portId);
        knownPortIds.add(portId);
        requireBoolean(port.required, `${portPath}.required`);
        requireEnum(
          port.role,
          ["power", "data", "execution", "activation", "status"],
          `${portPath}.role`,
        );
        validateNullable(port.typeId, `${portPath}.typeId`, requireCanonicalTypeId);
      }
    }
    const edges = requireArray(
      network.edges,
      `${networkPath}.edges`,
      PLC_CONTRACT_LIMITS.graphEdges,
    );
    for (const [edgeIndex, edgeInput] of edges.entries()) {
      const edgePath = `${networkPath}.edges[${edgeIndex}]`;
      const edge = requireRecord(edgeInput, edgePath);
      requireExactKeys(
        edge,
        ["edgeKind", "id", "sourcePortId", "targetPortId"],
        edgePath,
      );
      requireEnum(edge.edgeKind, ["power", "data", "execution"], `${edgePath}.edgeKind`);
      allEdgeIds.push(requireUuid(edge.id, `${edgePath}.id`));
      pendingEdges.push({
        path: edgePath,
        source: requireUuid(edge.sourcePortId, `${edgePath}.sourcePortId`),
        target: requireUuid(edge.targetPortId, `${edgePath}.targetPortId`),
      });
    }
  }
  requireUniqueStrings(networkIds, `${path}.networks`);
  requireUniqueStrings(allNodeIds, `${path}.nodes`);
  requireUniqueStrings(allPortIds, `${path}.ports`);
  requireUniqueStrings(allEdgeIds, `${path}.edges`);
  for (const edge of pendingEdges) {
    if (!knownPortIds.has(edge.source) || !knownPortIds.has(edge.target)) {
      fail(edge.path, "edge endpoint does not reference a declared graph port");
    }
  }
  return input as SemanticGraphContract;
};

const validateProjectCommand = (record: PlainRecord, path: string): void => {
  switch (record.commandKind) {
    case "project.create":
      requireExactKeys(
        record,
        ["commandKind", "displayName", "documentId", "projectRootId"],
        path,
      );
      requireString(
        record.displayName,
        `${path}.displayName`,
        PLC_CONTRACT_LIMITS.projectNameCharacters,
      );
      requireUuid(record.documentId, `${path}.documentId`);
      requireUuid(record.projectRootId, `${path}.projectRootId`);
      break;
    case "project.rename-object":
      requireExactKeys(record, ["commandKind", "displayName", "objectId"], path);
      requireString(
        record.displayName,
        `${path}.displayName`,
        PLC_CONTRACT_LIMITS.projectNameCharacters,
      );
      requireUuid(record.objectId, `${path}.objectId`);
      break;
    case "project.move-object":
      requireExactKeys(
        record,
        ["commandKind", "objectId", "orderKey", "parentId"],
        path,
      );
      requireUuid(record.objectId, `${path}.objectId`);
      requireUInt64(record.orderKey, `${path}.orderKey`);
      requireUuid(record.parentId, `${path}.parentId`);
      break;
    case "project.delete-object":
      requireExactKeys(record, ["commandKind", "objectId"], path);
      requireUuid(record.objectId, `${path}.objectId`);
      break;
    case "project.copy-objects":
      requireExactKeys(record, ["commandKind", "sourceObjectIds", "targetParentId"], path);
      validateUuidArray(record.sourceObjectIds, `${path}.sourceObjectIds`);
      requireUuid(record.targetParentId, `${path}.targetParentId`);
      break;
    case "project.undo":
    case "project.redo":
      requireExactKeys(record, ["commandKind", "undoToken"], path);
      requireUndoToken(record.undoToken, `${path}.undoToken`);
      break;
    default:
      fail(`${path}.commandKind`, "unknown project command");
  }
};

const validatePersistenceCommand = (record: PlainRecord, path: string): void => {
  switch (record.commandKind) {
    case "persistence.open":
      requireExactKeys(record, ["commandKind", "sourceGrantId"], path);
      requireUuid(record.sourceGrantId, `${path}.sourceGrantId`);
      break;
    case "persistence.save":
      requireExactKeys(
        record,
        ["commandKind", "documentId", "mode", "targetGrantId"],
        path,
      );
      requireUuid(record.documentId, `${path}.documentId`);
      requireEnum(record.mode, ["save", "save-as"], `${path}.mode`);
      requireUuid(record.targetGrantId, `${path}.targetGrantId`);
      break;
    case "persistence.recover":
      requireExactKeys(record, ["commandKind", "recoveryJournalId"], path);
      requireUuid(record.recoveryJournalId, `${path}.recoveryJournalId`);
      break;
    default:
      fail(`${path}.commandKind`, "unknown persistence command");
  }
};

const validateHardwareCommand = (record: PlainRecord, path: string): void => {
  switch (record.commandKind) {
    case "hardware.configure-controller":
      requireExactKeys(
        record,
        ["commandKind", "controllerId", "profileId", "profileVersion"],
        path,
      );
      requireUuid(record.controllerId, `${path}.controllerId`);
      requireRegistryId(record.profileId, `${path}.profileId`);
      requireVersion(record.profileVersion, `${path}.profileVersion`);
      break;
    case "hardware.upsert-device":
      requireExactKeys(
        record,
        ["catalogId", "commandKind", "controllerId", "deviceId", "parentId", "slot"],
        path,
      );
      requireRegistryId(record.catalogId, `${path}.catalogId`);
      requireUuid(record.controllerId, `${path}.controllerId`);
      requireUuid(record.deviceId, `${path}.deviceId`);
      validateNullable(record.parentId, `${path}.parentId`, requireUuid);
      validateNullable(record.slot, `${path}.slot`, (value, valuePath) =>
        requireSafeInteger(value, valuePath, 0, 65_535),
      );
      break;
    case "hardware.assign-address":
      requireExactKeys(
        record,
        [
          "area",
          "bitOffset",
          "byteOffset",
          "commandKind",
          "controllerId",
          "objectId",
          "widthBits",
        ],
        path,
      );
      requireEnum(record.area, ["I", "Q", "M"], `${path}.area`);
      validateNullable(record.bitOffset, `${path}.bitOffset`, (value, valuePath) =>
        requireSafeInteger(value, valuePath, 0, 7),
      );
      requireSafeInteger(record.byteOffset, `${path}.byteOffset`, 0, 16_777_215);
      requireUuid(record.controllerId, `${path}.controllerId`);
      requireUuid(record.objectId, `${path}.objectId`);
      requireSafeInteger(record.widthBits, `${path}.widthBits`, 1, 65_536);
      break;
    case "hardware.assign-virtual-network":
      requireExactKeys(
        record,
        [
          "commandKind",
          "controllerId",
          "interfaceId",
          "subnetId",
          "virtualAddress",
        ],
        path,
      );
      requireUuid(record.controllerId, `${path}.controllerId`);
      requireUuid(record.interfaceId, `${path}.interfaceId`);
      requireUuid(record.subnetId, `${path}.subnetId`);
      requireString(record.virtualAddress, `${path}.virtualAddress`, 128);
      break;
    default:
      fail(`${path}.commandKind`, "unknown hardware command");
  }
};

const validateProgramCommand = (record: PlainRecord, path: string): void => {
  switch (record.commandKind) {
    case "program.create-block":
      requireExactKeys(
        record,
        [
          "blockId",
          "blockKind",
          "commandKind",
          "controllerId",
          "displayName",
          "engineeringNumber",
          "language",
          "obRole",
          "offsetMilliseconds",
          "periodMilliseconds",
        ],
        path,
      );
      requireUuid(record.blockId, `${path}.blockId`);
      requireEnum(record.blockKind, ["OB", "FC", "FB"], `${path}.blockKind`);
      requireUuid(record.controllerId, `${path}.controllerId`);
      requireString(
        record.displayName,
        `${path}.displayName`,
        PLC_CONTRACT_LIMITS.projectNameCharacters,
      );
      validateNullable(
        record.engineeringNumber,
        `${path}.engineeringNumber`,
        (value, valuePath) => requireSafeInteger(value, valuePath, 1, 65_535),
      );
      requireEnum(record.language, ["LAD", "FBD", "SCL"], `${path}.language`);
      validateNullable(record.obRole, `${path}.obRole`, (value, valuePath) =>
        requireEnum(value, ["CyclicMain", "Startup", "TimedCyclic"], valuePath),
      );
      validateNullable(
        record.offsetMilliseconds,
        `${path}.offsetMilliseconds`,
        (value, valuePath) => requireSafeInteger(value, valuePath, 0, 60_000),
      );
      validateNullable(
        record.periodMilliseconds,
        `${path}.periodMilliseconds`,
        (value, valuePath) => requireSafeInteger(value, valuePath, 10, 60_000),
      );
      break;
    case "program.replace-interface":
      requireExactKeys(
        record,
        ["blockId", "commandKind", "controllerId", "interface"],
        path,
      );
      requireUuid(record.blockId, `${path}.blockId`);
      requireUuid(record.controllerId, `${path}.controllerId`);
      validateBlockInterface(record.interface, `${path}.interface`);
      break;
    case "program.replace-scl-body":
      requireExactKeys(
        record,
        ["blockId", "commandKind", "controllerId", "sourceHash", "sourceText"],
        path,
      );
      requireUuid(record.blockId, `${path}.blockId`);
      requireUuid(record.controllerId, `${path}.controllerId`);
      requireSha256(record.sourceHash, `${path}.sourceHash`);
      requireString(
        record.sourceText,
        `${path}.sourceText`,
        PLC_CONTRACT_LIMITS.sourceCharacters,
        true,
      );
      break;
    case "program.replace-graph-body":
      requireExactKeys(
        record,
        ["blockId", "commandKind", "controllerId", "graph"],
        path,
      );
      requireUuid(record.blockId, `${path}.blockId`);
      requireUuid(record.controllerId, `${path}.controllerId`);
      validateSemanticGraph(record.graph, `${path}.graph`);
      break;
    default:
      fail(`${path}.commandKind`, "unknown program command");
  }
};

const COMPILE_SCOPES = [
  "CurrentObject",
  "SoftwareChanges",
  "RebuildAllSoftware",
  "VirtualHardware",
  "ControllerBuild",
] as const;

const BUILD_OUTCOMES = [
  "Success",
  "Blocked",
  "Cancelled",
  "Stale",
  "ResourceLimit",
  "InternalFailure",
] as const;

const CPU_STATES = [
  "POWERED_OFF",
  "STARTUP",
  "STOP",
  "RUN",
  "PAUSED",
  "FAULTED",
] as const;

const validateBuildCommand = (record: PlainRecord, path: string): void => {
  switch (record.commandKind) {
    case "build.compile":
      requireExactKeys(
        record,
        ["commandKind", "controllerId", "scope", "selectedObjectId"],
        path,
      );
      requireUuid(record.controllerId, `${path}.controllerId`);
      requireEnum(record.scope, COMPILE_SCOPES, `${path}.scope`);
      validateNullable(record.selectedObjectId, `${path}.selectedObjectId`, requireUuid);
      if (record.scope === "CurrentObject" && record.selectedObjectId === null) {
        fail(path, "CurrentObject scope requires selectedObjectId");
      }
      if (record.scope !== "CurrentObject" && record.selectedObjectId !== null) {
        fail(path, "selectedObjectId is legal only for CurrentObject scope");
      }
      break;
    case "build.cancel":
      requireExactKeys(record, ["attemptId", "commandKind"], path);
      requireUuid(record.attemptId, `${path}.attemptId`);
      break;
    default:
      fail(`${path}.commandKind`, "unknown build command");
  }
};

const validateRuntimeCommand = (record: PlainRecord, path: string): void => {
  switch (record.commandKind) {
    case "runtime.set-mode":
      requireExactKeys(
        record,
        ["commandKind", "controllerId", "expectedControllerEpoch", "mode"],
        path,
      );
      requireUuid(record.controllerId, `${path}.controllerId`);
      requireUInt64(record.expectedControllerEpoch, `${path}.expectedControllerEpoch`);
      requireEnum(record.mode, ["RUN", "STOP"], `${path}.mode`);
      break;
    case "runtime.advance-virtual-time":
      requireExactKeys(
        record,
        ["commandKind", "deltaMilliseconds", "universeId"],
        path,
      );
      requireUInt64(record.deltaMilliseconds, `${path}.deltaMilliseconds`);
      requireUuid(record.universeId, `${path}.universeId`);
      break;
    case "runtime.set-virtual-input":
      requireExactKeys(
        record,
        [
          "commandKind",
          "controllerId",
          "expectedControllerEpoch",
          "targetId",
          "value",
        ],
        path,
      );
      requireUuid(record.controllerId, `${path}.controllerId`);
      requireUInt64(record.expectedControllerEpoch, `${path}.expectedControllerEpoch`);
      requireUuid(record.targetId, `${path}.targetId`);
      validateCanonicalTypedValue(record.value, `${path}.value`);
      break;
    default:
      fail(`${path}.commandKind`, "unknown runtime command");
  }
};

const validateCommissioningCommand = (record: PlainRecord, path: string): void => {
  switch (record.commandKind) {
    case "commissioning.preview-load":
      requireExactKeys(
        record,
        [
          "artifactPackageFingerprint",
          "commandKind",
          "controllerId",
          "expectedControllerEpoch",
        ],
        path,
      );
      requireSha256(
        record.artifactPackageFingerprint,
        `${path}.artifactPackageFingerprint`,
      );
      requireUuid(record.controllerId, `${path}.controllerId`);
      requireUInt64(record.expectedControllerEpoch, `${path}.expectedControllerEpoch`);
      break;
    case "commissioning.commit-load":
      requireExactKeys(
        record,
        [
          "commandKind",
          "controllerId",
          "expectedControllerEpoch",
          "previewFingerprint",
          "previewId",
        ],
        path,
      );
      requireUuid(record.controllerId, `${path}.controllerId`);
      requireUInt64(record.expectedControllerEpoch, `${path}.expectedControllerEpoch`);
      requireSha256(record.previewFingerprint, `${path}.previewFingerprint`);
      requireUuid(record.previewId, `${path}.previewId`);
      break;
    case "commissioning.cancel-preview":
      requireExactKeys(record, ["commandKind", "previewId"], path);
      requireUuid(record.previewId, `${path}.previewId`);
      break;
    default:
      fail(`${path}.commandKind`, "unknown commissioning command");
  }
};

const validateMonitoringCommand = (record: PlainRecord, path: string): void => {
  switch (record.commandKind) {
    case "monitoring.subscribe":
      requireExactKeys(
        record,
        ["commandKind", "controllerId", "expectedControllerEpoch", "targetIds"],
        path,
      );
      requireUuid(record.controllerId, `${path}.controllerId`);
      requireUInt64(record.expectedControllerEpoch, `${path}.expectedControllerEpoch`);
      validateUuidArray(record.targetIds, `${path}.targetIds`, 4_096);
      break;
    case "monitoring.unsubscribe":
      requireExactKeys(record, ["commandKind", "subscriptionId"], path);
      requireUuid(record.subscriptionId, `${path}.subscriptionId`);
      break;
    case "monitoring.modify":
      requireExactKeys(
        record,
        [
          "commandKind",
          "controllerId",
          "expectedControllerEpoch",
          "targetId",
          "value",
        ],
        path,
      );
      requireUuid(record.controllerId, `${path}.controllerId`);
      requireUInt64(record.expectedControllerEpoch, `${path}.expectedControllerEpoch`);
      requireUuid(record.targetId, `${path}.targetId`);
      validateCanonicalTypedValue(record.value, `${path}.value`);
      break;
    case "monitoring.create-force":
      requireExactKeys(
        record,
        [
          "commandKind",
          "controllerId",
          "expectedControllerEpoch",
          "expectedForceRevision",
          "targetId",
          "value",
        ],
        path,
      );
      requireUuid(record.controllerId, `${path}.controllerId`);
      requireUInt64(record.expectedControllerEpoch, `${path}.expectedControllerEpoch`);
      requireUInt64(record.expectedForceRevision, `${path}.expectedForceRevision`);
      requireUuid(record.targetId, `${path}.targetId`);
      validateCanonicalTypedValue(record.value, `${path}.value`);
      break;
    case "monitoring.remove-force":
      requireExactKeys(
        record,
        ["commandKind", "controllerId", "expectedControllerEpoch", "forceId"],
        path,
      );
      requireUuid(record.controllerId, `${path}.controllerId`);
      requireUInt64(record.expectedControllerEpoch, `${path}.expectedControllerEpoch`);
      requireUuid(record.forceId, `${path}.forceId`);
      break;
    case "monitoring.trace-control":
      requireExactKeys(
        record,
        [
          "commandKind",
          "controllerId",
          "expectedControllerEpoch",
          "operation",
          "traceId",
        ],
        path,
      );
      requireUuid(record.controllerId, `${path}.controllerId`);
      requireUInt64(record.expectedControllerEpoch, `${path}.expectedControllerEpoch`);
      requireEnum(record.operation, ["arm", "start", "stop", "abort"], `${path}.operation`);
      requireUuid(record.traceId, `${path}.traceId`);
      break;
    default:
      fail(`${path}.commandKind`, "unknown monitoring command");
  }
};

const DOMAIN_COMMAND_KINDS = [
  "project.create",
  "project.rename-object",
  "project.move-object",
  "project.delete-object",
  "project.copy-objects",
  "project.undo",
  "project.redo",
  "persistence.open",
  "persistence.save",
  "persistence.recover",
  "hardware.configure-controller",
  "hardware.upsert-device",
  "hardware.assign-address",
  "hardware.assign-virtual-network",
  "program.create-block",
  "program.replace-interface",
  "program.replace-scl-body",
  "program.replace-graph-body",
  "build.compile",
  "build.cancel",
  "runtime.set-mode",
  "runtime.advance-virtual-time",
  "runtime.set-virtual-input",
  "commissioning.preview-load",
  "commissioning.commit-load",
  "commissioning.cancel-preview",
  "monitoring.subscribe",
  "monitoring.unsubscribe",
  "monitoring.modify",
  "monitoring.create-force",
  "monitoring.remove-force",
  "monitoring.trace-control",
] as const;

const commandDomain = (commandKind: string): string =>
  commandKind.slice(0, Math.max(0, commandKind.indexOf(".")));

export const validateDomainCommand = (
  input: unknown,
  path = "$command",
): DomainCommand => {
  const record = requireRecord(input, path);
  const commandKind = requireString(record.commandKind, `${path}.commandKind`, 64);
  switch (commandDomain(commandKind)) {
    case "project":
      validateProjectCommand(record, path);
      break;
    case "persistence":
      validatePersistenceCommand(record, path);
      break;
    case "hardware":
      validateHardwareCommand(record, path);
      break;
    case "program":
      validateProgramCommand(record, path);
      break;
    case "build":
      validateBuildCommand(record, path);
      break;
    case "runtime":
      validateRuntimeCommand(record, path);
      break;
    case "commissioning":
      validateCommissioningCommand(record, path);
      break;
    case "monitoring":
      validateMonitoringCommand(record, path);
      break;
    default:
      fail(`${path}.commandKind`, `unknown command kind ${JSON.stringify(commandKind)}`);
  }
  return input as DomainCommand;
};

const DOMAIN_QUERY_KINDS = [
  "system.health",
  "project.get-summary",
  "project.get-object",
  "persistence.get-status",
  "hardware.get-configuration",
  "program.get-block",
  "build.get-report",
  "runtime.get-status",
  "commissioning.get-preview",
  "monitoring.get-snapshot",
  "diagnostics.list",
] as const;

export const validateDomainQuery = (
  input: unknown,
  path = "$query",
): DomainQuery => {
  const record = requireRecord(input, path);
  const queryKind = requireString(record.queryKind, `${path}.queryKind`, 64);
  switch (queryKind) {
    case "system.health":
      requireExactKeys(record, ["queryKind"], path);
      break;
    case "project.get-summary":
      requireExactKeys(record, ["projectRootId", "queryKind"], path);
      requireUuid(record.projectRootId, `${path}.projectRootId`);
      break;
    case "project.get-object":
      requireExactKeys(record, ["objectId", "queryKind"], path);
      requireUuid(record.objectId, `${path}.objectId`);
      break;
    case "persistence.get-status":
      requireExactKeys(record, ["documentId", "queryKind"], path);
      requireUuid(record.documentId, `${path}.documentId`);
      break;
    case "hardware.get-configuration":
      requireExactKeys(record, ["controllerId", "queryKind"], path);
      requireUuid(record.controllerId, `${path}.controllerId`);
      break;
    case "program.get-block":
      requireExactKeys(record, ["blockId", "queryKind"], path);
      requireUuid(record.blockId, `${path}.blockId`);
      break;
    case "build.get-report":
      requireExactKeys(record, ["attemptId", "queryKind"], path);
      requireUuid(record.attemptId, `${path}.attemptId`);
      break;
    case "runtime.get-status":
      requireExactKeys(
        record,
        ["controllerId", "expectedControllerEpoch", "queryKind"],
        path,
      );
      requireUuid(record.controllerId, `${path}.controllerId`);
      requireUInt64(record.expectedControllerEpoch, `${path}.expectedControllerEpoch`);
      break;
    case "commissioning.get-preview":
      requireExactKeys(record, ["previewId", "queryKind"], path);
      requireUuid(record.previewId, `${path}.previewId`);
      break;
    case "monitoring.get-snapshot":
      requireExactKeys(record, ["queryKind", "subscriptionId"], path);
      requireUuid(record.subscriptionId, `${path}.subscriptionId`);
      break;
    case "diagnostics.list":
      requireExactKeys(record, ["blocking", "phase", "queryKind"], path);
      validateNullable(record.blocking, `${path}.blocking`, requireBoolean);
      validateNullable(record.phase, `${path}.phase`, (value, valuePath) =>
        requireRegistryId(value, valuePath),
      );
      break;
    default:
      fail(`${path}.queryKind`, `unknown query kind ${JSON.stringify(queryKind)}`);
  }
  return input as DomainQuery;
};

const PROJECT_OBJECT_KINDS = [
  "ProjectRoot",
  "Folder",
  "Controller",
  "Device",
  "Module",
  "VirtualNetwork",
  "VirtualInterface",
  "SymbolTable",
  "Tag",
  "Constant",
  "NamedType",
  "OB",
  "FC",
  "FB",
  "GlobalDB",
  "InstanceDB",
  "WatchTable",
  "TraceConfiguration",
  "BuildRecord",
  "SnapshotReference",
] as const;

const validateProjectReferenceSnapshot = (
  input: unknown,
  path: string,
): string => {
  const reference = requireRecord(input, path);
  requireExactKeys(
    reference,
    [
      "expectedTargetKind",
      "referenceId",
      "resolution",
      "sourceAnchor",
      "targetId",
    ],
    path,
  );
  requireEnum(
    reference.expectedTargetKind,
    PROJECT_OBJECT_KINDS,
    `${path}.expectedTargetKind`,
  );
  const referenceId = requireUuid(reference.referenceId, `${path}.referenceId`);
  requireEnum(
    reference.resolution,
    ["resolved", "unresolved", "tombstoned"],
    `${path}.resolution`,
  );
  validateSourceAnchor(reference.sourceAnchor, `${path}.sourceAnchor`);
  requireUuid(reference.targetId, `${path}.targetId`);
  return referenceId;
};

const validateProjectReferenceArray = (input: unknown, path: string): void => {
  const references = requireArray(input, path, PLC_CONTRACT_LIMITS.objectIds);
  const referenceIds = references.map((reference, index) =>
    validateProjectReferenceSnapshot(reference, `${path}[${index}]`),
  );
  requireUniqueStrings(referenceIds, path);
};

const validateProjectObjectSnapshot = (input: unknown, path: string): string => {
  const record = requireRecord(input, path);
  requireExactKeys(
    record,
    [
      "creationOrdinal",
      "displayName",
      "id",
      "kind",
      "lifecycle",
      "objectRevision",
      "orderedChildIds",
      "parentId",
      "references",
      "semanticRevision",
    ],
    path,
  );
  requireUInt64(record.creationOrdinal, `${path}.creationOrdinal`);
  requireString(
    record.displayName,
    `${path}.displayName`,
    PLC_CONTRACT_LIMITS.projectNameCharacters,
  );
  const objectId = requireUuid(record.id, `${path}.id`);
  requireEnum(record.kind, PROJECT_OBJECT_KINDS, `${path}.kind`);
  requireEnum(record.lifecycle, ["active", "tombstoned"], `${path}.lifecycle`);
  requireUInt64(record.objectRevision, `${path}.objectRevision`);
  validateUuidArray(record.orderedChildIds, `${path}.orderedChildIds`);
  validateNullable(record.parentId, `${path}.parentId`, requireUuid);
  requireUInt64(record.semanticRevision, `${path}.semanticRevision`);
  validateProjectReferenceArray(record.references, `${path}.references`);
  return objectId;
};

const validateDirtyBuildState = (input: unknown, path: string): void => {
  const record = requireRecord(input, path);
  requireExactKeys(
    record,
    [
      "controllerStates",
      "currentDocumentHash",
      "currentSemanticFingerprint",
      "documentDirty",
      "savedDocumentHash",
      "savedDocumentRevision",
      "savedSemanticFingerprint",
      "semanticDirty",
    ],
    path,
  );
  requireSha256(record.currentDocumentHash, `${path}.currentDocumentHash`);
  requireSha256(
    record.currentSemanticFingerprint,
    `${path}.currentSemanticFingerprint`,
  );
  requireBoolean(record.documentDirty, `${path}.documentDirty`);
  requireBoolean(record.semanticDirty, `${path}.semanticDirty`);
  validateNullable(record.savedDocumentHash, `${path}.savedDocumentHash`, requireSha256);
  validateNullable(
    record.savedDocumentRevision,
    `${path}.savedDocumentRevision`,
    requireUInt64,
  );
  validateNullable(
    record.savedSemanticFingerprint,
    `${path}.savedSemanticFingerprint`,
    requireSha256,
  );
  const states = requireArray(
    record.controllerStates,
    `${path}.controllerStates`,
    PLC_CONTRACT_LIMITS.objectIds,
  );
  const controllerIds: string[] = [];
  for (const [index, stateInput] of states.entries()) {
    const statePath = `${path}.controllerStates[${index}]`;
    const state = requireRecord(stateInput, statePath);
    requireExactKeys(
      state,
      ["controllerId", "hardware", "loadedArtifactFingerprint", "software"],
      statePath,
    );
    controllerIds.push(requireUuid(state.controllerId, `${statePath}.controllerId`));
    requireEnum(
      state.hardware,
      ["not-built", "current", "stale", "blocked"],
      `${statePath}.hardware`,
    );
    requireEnum(
      state.software,
      ["not-built", "current", "stale", "blocked"],
      `${statePath}.software`,
    );
    validateNullable(
      state.loadedArtifactFingerprint,
      `${statePath}.loadedArtifactFingerprint`,
      requireSha256,
    );
  }
  requireUniqueStrings(controllerIds, `${path}.controllerStates`);
};

const validateProjectReceipt = (record: PlainRecord, path: string): ProjectReceipt => {
  requireExactKeys(
    record,
    [
      "affectedObjectIds",
      "documentId",
      "documentRevision",
      "domain",
      "projectHash",
      "projectRootId",
      "semanticRevision",
    ],
    path,
  );
  validateUuidArray(record.affectedObjectIds, `${path}.affectedObjectIds`);
  requireUuid(record.documentId, `${path}.documentId`);
  requireUInt64(record.documentRevision, `${path}.documentRevision`);
  requireSha256(record.projectHash, `${path}.projectHash`);
  requireUuid(record.projectRootId, `${path}.projectRootId`);
  requireUInt64(record.semanticRevision, `${path}.semanticRevision`);
  return record as ProjectReceipt;
};

const validateProjectSnapshotReceipt = (record: PlainRecord, path: string): void => {
  requireExactKeys(
    record,
    [
      "dirtyBuildState",
      "documentId",
      "documentRevision",
      "domain",
      "objects",
      "projectRootId",
      "scope",
      "semanticRevision",
    ],
    path,
  );
  validateDirtyBuildState(record.dirtyBuildState, `${path}.dirtyBuildState`);
  requireUuid(record.documentId, `${path}.documentId`);
  requireUInt64(record.documentRevision, `${path}.documentRevision`);
  const scope = requireEnum(record.scope, ["summary", "object"], `${path}.scope`);
  requireUuid(record.projectRootId, `${path}.projectRootId`);
  requireUInt64(record.semanticRevision, `${path}.semanticRevision`);
  const objects = requireArray(
    record.objects,
    `${path}.objects`,
    PLC_CONTRACT_LIMITS.objectIds,
    1,
  );
  if (scope === "object" && objects.length !== 1) {
    fail(`${path}.objects`, "object scope must contain exactly one object snapshot");
  }
  const objectIds = objects.map((object, index) =>
    validateProjectObjectSnapshot(object, `${path}.objects[${index}]`),
  );
  requireUniqueStrings(objectIds, `${path}.objects`);
};

const validatePersistenceReceipt = (
  record: PlainRecord,
  path: string,
): PersistenceReceipt => {
  requireExactKeys(
    record,
    [
      "action",
      "documentId",
      "documentRevision",
      "domain",
      "packageHash",
      "projectRootId",
      "recoveryStatus",
      "schemaVersion",
    ],
    path,
  );
  requireEnum(record.action, ["open", "save", "save-as", "recover"], `${path}.action`);
  requireUuid(record.documentId, `${path}.documentId`);
  requireUInt64(record.documentRevision, `${path}.documentRevision`);
  requireSha256(record.packageHash, `${path}.packageHash`);
  requireUuid(record.projectRootId, `${path}.projectRootId`);
  requireEnum(
    record.recoveryStatus,
    ["not-applicable", "recovered", "discarded"],
    `${path}.recoveryStatus`,
  );
  requireVersion(record.schemaVersion, `${path}.schemaVersion`);
  return record as PersistenceReceipt;
};

const validateHardwareReceipt = (record: PlainRecord, path: string): HardwareReceipt => {
  requireExactKeys(
    record,
    ["affectedObjectIds", "configurationFingerprint", "controllerId", "domain"],
    path,
  );
  validateUuidArray(record.affectedObjectIds, `${path}.affectedObjectIds`);
  requireSha256(record.configurationFingerprint, `${path}.configurationFingerprint`);
  requireUuid(record.controllerId, `${path}.controllerId`);
  return record as HardwareReceipt;
};

const validateAddressSnapshot = (input: unknown, path: string): void => {
  const record = requireRecord(input, path);
  requireExactKeys(record, ["area", "bitOffset", "byteOffset", "widthBits"], path);
  requireEnum(record.area, ["I", "Q", "M"], `${path}.area`);
  validateNullable(record.bitOffset, `${path}.bitOffset`, (value, valuePath) =>
    requireSafeInteger(value, valuePath, 0, 7),
  );
  requireSafeInteger(record.byteOffset, `${path}.byteOffset`, 0, 16_777_215);
  requireSafeInteger(record.widthBits, `${path}.widthBits`, 1, 65_536);
};

const validateHardwareSnapshotReceipt = (record: PlainRecord, path: string): void => {
  requireExactKeys(
    record,
    [
      "configurationFingerprint",
      "controllerId",
      "domain",
      "objects",
      "profileId",
      "profileVersion",
    ],
    path,
  );
  requireSha256(record.configurationFingerprint, `${path}.configurationFingerprint`);
  requireUuid(record.controllerId, `${path}.controllerId`);
  requireRegistryId(record.profileId, `${path}.profileId`);
  requireVersion(record.profileVersion, `${path}.profileVersion`);
  const objects = requireArray(
    record.objects,
    `${path}.objects`,
    PLC_CONTRACT_LIMITS.objectIds,
  );
  const objectIds: string[] = [];
  for (const [index, objectInput] of objects.entries()) {
    const objectPath = `${path}.objects[${index}]`;
    const object = requireRecord(objectInput, objectPath);
    requireExactKeys(
      object,
      [
        "address",
        "catalogId",
        "creationOrdinal",
        "displayName",
        "id",
        "kind",
        "lifecycle",
        "objectRevision",
        "orderedChildIds",
        "parentId",
        "references",
        "semanticRevision",
        "slot",
        "virtualAddress",
        "virtualSubnetId",
      ],
      objectPath,
    );
    if (object.address !== null) {
      validateAddressSnapshot(object.address, `${objectPath}.address`);
    }
    requireRegistryId(object.catalogId, `${objectPath}.catalogId`);
    requireUInt64(object.creationOrdinal, `${objectPath}.creationOrdinal`);
    requireString(
      object.displayName,
      `${objectPath}.displayName`,
      PLC_CONTRACT_LIMITS.projectNameCharacters,
    );
    objectIds.push(requireUuid(object.id, `${objectPath}.id`));
    requireEnum(
      object.kind,
      ["Controller", "Device", "Module", "Channel", "VirtualInterface"],
      `${objectPath}.kind`,
    );
    requireEnum(object.lifecycle, ["active", "tombstoned"], `${objectPath}.lifecycle`);
    requireUInt64(object.objectRevision, `${objectPath}.objectRevision`);
    validateUuidArray(object.orderedChildIds, `${objectPath}.orderedChildIds`);
    validateNullable(object.parentId, `${objectPath}.parentId`, requireUuid);
    validateProjectReferenceArray(object.references, `${objectPath}.references`);
    requireUInt64(object.semanticRevision, `${objectPath}.semanticRevision`);
    validateNullable(object.slot, `${objectPath}.slot`, (value, valuePath) =>
      requireSafeInteger(value, valuePath, 0, 65_535),
    );
    validateNullable(object.virtualAddress, `${objectPath}.virtualAddress`, (value, valuePath) =>
      requireString(value, valuePath, 128),
    );
    validateNullable(object.virtualSubnetId, `${objectPath}.virtualSubnetId`, requireUuid);
  }
  requireUniqueStrings(objectIds, `${path}.objects`);
};

const validateProgramReceipt = (record: PlainRecord, path: string): ProgramReceipt => {
  requireExactKeys(
    record,
    [
      "affectedObjectIds",
      "blockId",
      "controllerId",
      "domain",
      "invalidatedObjectIds",
      "publicSignatureFingerprint",
    ],
    path,
  );
  validateUuidArray(record.affectedObjectIds, `${path}.affectedObjectIds`);
  validateNullable(record.blockId, `${path}.blockId`, requireUuid);
  requireUuid(record.controllerId, `${path}.controllerId`);
  validateUuidArray(record.invalidatedObjectIds, `${path}.invalidatedObjectIds`);
  validateNullable(
    record.publicSignatureFingerprint,
    `${path}.publicSignatureFingerprint`,
    requireSha256,
  );
  return record as ProgramReceipt;
};

const validateProgramSnapshotReceipt = (record: PlainRecord, path: string): void => {
  requireExactKeys(record, ["blocks", "controllerId", "domain"], path);
  requireUuid(record.controllerId, `${path}.controllerId`);
  const blocks = requireArray(
    record.blocks,
    `${path}.blocks`,
    PLC_CONTRACT_LIMITS.objectIds,
  );
  const blockIds: string[] = [];
  for (const [index, blockInput] of blocks.entries()) {
    const blockPath = `${path}.blocks[${index}]`;
    const block = requireRecord(blockInput, blockPath);
    requireExactKeys(
      block,
      [
        "blockId",
        "blockKind",
        "body",
        "bodyLanguage",
        "creationOrdinal",
        "displayName",
        "engineeringNumber",
        "interface",
        "lifecycle",
        "objectRevision",
        "obRole",
        "offsetMilliseconds",
        "orderedChildIds",
        "parentId",
        "periodMilliseconds",
        "publicSignatureFingerprint",
        "references",
        "semanticFingerprint",
        "semanticRevision",
        "sourceRevisionHash",
      ],
      blockPath,
    );
    blockIds.push(requireUuid(block.blockId, `${blockPath}.blockId`));
    requireEnum(block.blockKind, ["OB", "FC", "FB"], `${blockPath}.blockKind`);
    const bodyLanguage = requireEnum(
      block.bodyLanguage,
      ["LAD", "FBD", "SCL"],
      `${blockPath}.bodyLanguage`,
    );
    const body = requireRecord(block.body, `${blockPath}.body`);
    const bodyKind = requireEnum(
      body.bodyKind,
      ["scl", "graph"],
      `${blockPath}.body.bodyKind`,
    );
    if (bodyKind === "scl") {
      requireExactKeys(
        body,
        ["bodyKind", "sourceHash", "sourceText"],
        `${blockPath}.body`,
      );
      requireSha256(body.sourceHash, `${blockPath}.body.sourceHash`);
      requireString(
        body.sourceText,
        `${blockPath}.body.sourceText`,
        PLC_CONTRACT_LIMITS.sourceCharacters,
        true,
      );
      if (bodyLanguage !== "SCL") {
        fail(`${blockPath}.bodyLanguage`, "SCL body snapshot requires SCL language");
      }
    } else {
      requireExactKeys(body, ["bodyKind", "graph"], `${blockPath}.body`);
      const graph = validateSemanticGraph(body.graph, `${blockPath}.body.graph`);
      if (bodyLanguage === "SCL" || graph.language !== bodyLanguage) {
        fail(`${blockPath}.bodyLanguage`, "graph body language does not reconcile");
      }
    }
    requireUInt64(block.creationOrdinal, `${blockPath}.creationOrdinal`);
    requireString(
      block.displayName,
      `${blockPath}.displayName`,
      PLC_CONTRACT_LIMITS.projectNameCharacters,
    );
    validateNullable(
      block.engineeringNumber,
      `${blockPath}.engineeringNumber`,
      (value, valuePath) => requireSafeInteger(value, valuePath, 1, 65_535),
    );
    validateBlockInterface(block.interface, `${blockPath}.interface`);
    requireEnum(block.lifecycle, ["active", "tombstoned"], `${blockPath}.lifecycle`);
    requireUInt64(block.objectRevision, `${blockPath}.objectRevision`);
    validateNullable(block.obRole, `${blockPath}.obRole`, (value, valuePath) =>
      requireEnum(value, ["CyclicMain", "Startup", "TimedCyclic"], valuePath),
    );
    validateNullable(
      block.offsetMilliseconds,
      `${blockPath}.offsetMilliseconds`,
      (value, valuePath) => requireSafeInteger(value, valuePath, 0, 60_000),
    );
    validateUuidArray(block.orderedChildIds, `${blockPath}.orderedChildIds`);
    requireUuid(block.parentId, `${blockPath}.parentId`);
    validateNullable(
      block.periodMilliseconds,
      `${blockPath}.periodMilliseconds`,
      (value, valuePath) => requireSafeInteger(value, valuePath, 10, 60_000),
    );
    requireSha256(
      block.publicSignatureFingerprint,
      `${blockPath}.publicSignatureFingerprint`,
    );
    validateProjectReferenceArray(block.references, `${blockPath}.references`);
    requireSha256(block.semanticFingerprint, `${blockPath}.semanticFingerprint`);
    requireUInt64(block.semanticRevision, `${blockPath}.semanticRevision`);
    requireSha256(block.sourceRevisionHash, `${blockPath}.sourceRevisionHash`);
    if (
      (bodyKind === "scl" && body.sourceHash !== block.sourceRevisionHash) ||
      (bodyKind === "graph" &&
        (body.graph as PlainRecord).sourceRevisionHash !== block.sourceRevisionHash)
    ) {
      fail(`${blockPath}.sourceRevisionHash`, "body and block source revisions do not reconcile");
    }
  }
  requireUniqueStrings(blockIds, `${path}.blocks`);
};

const validateBuildAttempt = (
  input: unknown,
  path: string,
): BuildAttemptRecord => {
  const record = requireRecord(input, path);
  requireExactKeys(
    record,
    [
      "attemptId",
      "compilerSemanticVersion",
      "instructionRegistryHash",
      "requestedScope",
      "snapshotHash",
      "trainingProfileHash",
      "typeSystemVersion",
    ],
    path,
  );
  requireUuid(record.attemptId, `${path}.attemptId`);
  requireVersion(record.compilerSemanticVersion, `${path}.compilerSemanticVersion`);
  requireSha256(record.instructionRegistryHash, `${path}.instructionRegistryHash`);
  requireEnum(record.requestedScope, COMPILE_SCOPES, `${path}.requestedScope`);
  requireSha256(record.snapshotHash, `${path}.snapshotHash`);
  requireSha256(record.trainingProfileHash, `${path}.trainingProfileHash`);
  requireVersion(record.typeSystemVersion, `${path}.typeSystemVersion`);
  return input as BuildAttemptRecord;
};

const validateBuildReport = (input: unknown, path: string): BuildReportRecord => {
  const record = requireRecord(input, path);
  requireExactKeys(
    record,
    [
      "artifactFingerprint",
      "attemptId",
      "diagnostics",
      "expandedClosure",
      "outcome",
      "requestedScope",
      "semanticFingerprint",
      "snapshotHash",
      "stale",
    ],
    path,
  );
  requireUuid(record.attemptId, `${path}.attemptId`);
  validateNullable(
    record.artifactFingerprint,
    `${path}.artifactFingerprint`,
    requireSha256,
  );
  validateDiagnosticArray(record.diagnostics, `${path}.diagnostics`);
  validateUuidArray(record.expandedClosure, `${path}.expandedClosure`);
  const outcome = requireEnum(record.outcome, BUILD_OUTCOMES, `${path}.outcome`);
  requireEnum(record.requestedScope, COMPILE_SCOPES, `${path}.requestedScope`);
  validateNullable(
    record.semanticFingerprint,
    `${path}.semanticFingerprint`,
    requireSha256,
  );
  requireSha256(record.snapshotHash, `${path}.snapshotHash`);
  requireBoolean(record.stale, `${path}.stale`);
  if (outcome === "Success") {
    if (record.artifactFingerprint === null || record.semanticFingerprint === null) {
      fail(path, "successful build report requires both fingerprints");
    }
  } else if (record.artifactFingerprint !== null) {
    fail(path, "non-successful build report cannot identify an artifact");
  }
  return input as BuildReportRecord;
};

const validateBuildArtifactIdentity = (
  input: unknown,
  path: string,
): BuildArtifactIdentity => {
  const record = requireRecord(input, path);
  requireExactKeys(
    record,
    [
      "artifactPackageFingerprint",
      "artifactSchema",
      "irVersion",
      "memorySchemaHash",
      "probeSchemaVersion",
      "profileHash",
      "runtimeContractVersion",
      "semanticBuildFingerprint",
      "sourceMapHash",
    ],
    path,
  );
  requireSha256(
    record.artifactPackageFingerprint,
    `${path}.artifactPackageFingerprint`,
  );
  requireVersion(record.artifactSchema, `${path}.artifactSchema`);
  requireVersion(record.irVersion, `${path}.irVersion`);
  requireSha256(record.memorySchemaHash, `${path}.memorySchemaHash`);
  requireVersion(record.probeSchemaVersion, `${path}.probeSchemaVersion`);
  requireSha256(record.profileHash, `${path}.profileHash`);
  requireVersion(record.runtimeContractVersion, `${path}.runtimeContractVersion`);
  requireSha256(
    record.semanticBuildFingerprint,
    `${path}.semanticBuildFingerprint`,
  );
  requireSha256(record.sourceMapHash, `${path}.sourceMapHash`);
  return input as BuildArtifactIdentity;
};

const validateBuildReceipt = (record: PlainRecord, path: string): BuildReceipt => {
  requireExactKeys(record, ["artifact", "attempt", "domain", "report"], path);
  const attempt = validateBuildAttempt(record.attempt, `${path}.attempt`);
  const report = validateBuildReport(record.report, `${path}.report`);
  const artifact = validateNullable(
    record.artifact,
    `${path}.artifact`,
    validateBuildArtifactIdentity,
  );
  if (
    attempt.attemptId !== report.attemptId ||
    attempt.snapshotHash !== report.snapshotHash ||
    attempt.requestedScope !== report.requestedScope
  ) {
    fail(path, "attempt and report identities do not reconcile");
  }
  if (report.outcome === "Success") {
    if (
      artifact === null ||
      report.artifactFingerprint !== artifact.artifactPackageFingerprint ||
      report.semanticFingerprint !== artifact.semanticBuildFingerprint
    ) {
      fail(path, "successful report and artifact fingerprints do not reconcile");
    }
  } else if (artifact !== null) {
    fail(path, "failed build receipt cannot contain an artifact");
  }
  return record as BuildReceipt;
};

const validateRuntimeReceipt = (record: PlainRecord, path: string): RuntimeReceipt => {
  requireExactKeys(
    record,
    [
      "affectedTargetIds",
      "controllerEpoch",
      "controllerId",
      "cpuState",
      "domain",
      "loadedArtifactFingerprint",
      "scanSequence",
      "universeEpoch",
      "virtualTimestampMilliseconds",
    ],
    path,
  );
  validateUuidArray(record.affectedTargetIds, `${path}.affectedTargetIds`);
  requireUInt64(record.controllerEpoch, `${path}.controllerEpoch`);
  requireUuid(record.controllerId, `${path}.controllerId`);
  requireEnum(record.cpuState, CPU_STATES, `${path}.cpuState`);
  validateNullable(
    record.loadedArtifactFingerprint,
    `${path}.loadedArtifactFingerprint`,
    requireSha256,
  );
  requireUInt64(record.scanSequence, `${path}.scanSequence`);
  requireUInt64(record.universeEpoch, `${path}.universeEpoch`);
  requireUInt64(
    record.virtualTimestampMilliseconds,
    `${path}.virtualTimestampMilliseconds`,
  );
  return record as RuntimeReceipt;
};

const validateCommissioningReceipt = (
  record: PlainRecord,
  path: string,
): CommissioningReceipt => {
  requireExactKeys(
    record,
    [
      "action",
      "candidateArtifactFingerprint",
      "controllerEpoch",
      "controllerId",
      "domain",
      "loadedArtifactFingerprint",
      "memoryActions",
      "previewFingerprint",
      "previewId",
    ],
    path,
  );
  requireEnum(record.action, ["preview", "commit", "cancel", "rollback"], `${path}.action`);
  requireSha256(
    record.candidateArtifactFingerprint,
    `${path}.candidateArtifactFingerprint`,
  );
  requireUInt64(record.controllerEpoch, `${path}.controllerEpoch`);
  requireUuid(record.controllerId, `${path}.controllerId`);
  validateNullable(
    record.loadedArtifactFingerprint,
    `${path}.loadedArtifactFingerprint`,
    requireSha256,
  );
  requireSha256(record.previewFingerprint, `${path}.previewFingerprint`);
  requireUuid(record.previewId, `${path}.previewId`);
  const actions = requireArray(
    record.memoryActions,
    `${path}.memoryActions`,
    PLC_CONTRACT_LIMITS.objectIds,
  );
  const regions: string[] = [];
  for (const [index, actionInput] of actions.entries()) {
    const actionPath = `${path}.memoryActions[${index}]`;
    const action = requireRecord(actionInput, actionPath);
    requireExactKeys(action, ["action", "reason", "regionId"], actionPath);
    requireEnum(
      action.action,
      ["initialize", "preserve", "reset", "reject"],
      `${actionPath}.action`,
    );
    requireString(action.reason, `${actionPath}.reason`, 1_024);
    regions.push(requireUuid(action.regionId, `${actionPath}.regionId`));
  }
  requireUniqueStrings(regions, `${path}.memoryActions`);
  return record as CommissioningReceipt;
};

const validateObservedValue = (input: unknown, path: string): ObservedValue => {
  const record = requireRecord(input, path);
  requireExactKeys(
    record,
    [
      "forceId",
      "freshness",
      "probeId",
      "quality",
      "scanSequence",
      "targetId",
      "value",
      "virtualTimestampMilliseconds",
    ],
    path,
  );
  validateNullable(record.forceId, `${path}.forceId`, requireUuid);
  requireEnum(record.freshness, ["CURRENT", "STALE", "UNKNOWN"], `${path}.freshness`);
  requireUuid(record.probeId, `${path}.probeId`);
  requireEnum(
    record.quality,
    ["GOOD", "UNCERTAIN", "BAD", "NOT_PRESENT"],
    `${path}.quality`,
  );
  requireUInt64(record.scanSequence, `${path}.scanSequence`);
  requireUuid(record.targetId, `${path}.targetId`);
  validateCanonicalTypedValue(record.value, `${path}.value`);
  requireUInt64(
    record.virtualTimestampMilliseconds,
    `${path}.virtualTimestampMilliseconds`,
  );
  return input as ObservedValue;
};

const validateObservedValues = (input: unknown, path: string): readonly ObservedValue[] => {
  const samples = requireArray(input, path, PLC_CONTRACT_LIMITS.typedValueElements);
  const probeAndTarget = new Set<string>();
  for (const [index, sampleInput] of samples.entries()) {
    const sample = validateObservedValue(sampleInput, `${path}[${index}]`);
    const key = `${sample.probeId}:${sample.targetId}`;
    if (probeAndTarget.has(key)) {
      fail(path, `duplicate observation for ${key}`);
    }
    probeAndTarget.add(key);
  }
  return samples as readonly ObservedValue[];
};

const validateMonitoringReceipt = (
  record: PlainRecord,
  path: string,
): MonitoringReceipt => {
  requireExactKeys(
    record,
    [
      "action",
      "commandReceiptId",
      "controllerEpoch",
      "controllerId",
      "domain",
      "samples",
      "subscriptionId",
    ],
    path,
  );
  requireEnum(
    record.action,
    [
      "subscribe",
      "unsubscribe",
      "snapshot",
      "modify",
      "force-create",
      "force-remove",
      "trace-control",
    ],
    `${path}.action`,
  );
  validateNullable(record.commandReceiptId, `${path}.commandReceiptId`, requireUuid);
  requireUInt64(record.controllerEpoch, `${path}.controllerEpoch`);
  requireUuid(record.controllerId, `${path}.controllerId`);
  validateObservedValues(record.samples, `${path}.samples`);
  validateNullable(record.subscriptionId, `${path}.subscriptionId`, requireUuid);
  return record as MonitoringReceipt;
};

const validateDiagnosticsReceipt = (record: PlainRecord, path: string): void => {
  requireExactKeys(
    record,
    ["diagnosticRevision", "diagnostics", "domain"],
    path,
  );
  requireUInt64(record.diagnosticRevision, `${path}.diagnosticRevision`);
  validateDiagnosticArray(record.diagnostics, `${path}.diagnostics`);
};

const validateHealthReceipt = (record: PlainRecord, path: string): HealthReceipt => {
  requireExactKeys(
    record,
    [
      "contractSchemaVersion",
      "coreVersion",
      "domain",
      "status",
      "supportedMessageSchemaVersions",
      "wasmSha256",
    ],
    path,
  );
  if (record.contractSchemaVersion !== PLC_CONTRACT_SCHEMA_VERSION) {
    fail(`${path}.contractSchemaVersion`, "unsupported contract schema version");
  }
  requireVersion(record.coreVersion, `${path}.coreVersion`);
  requireEnum(record.status, ["HEALTHY"], `${path}.status`);
  const versions = requireArray(
    record.supportedMessageSchemaVersions,
    `${path}.supportedMessageSchemaVersions`,
    2,
    2,
  );
  if (versions[0] !== 1 || versions[1] !== 2) {
    fail(`${path}.supportedMessageSchemaVersions`, "expected exact compatibility tuple [1, 2]");
  }
  requireSha256(record.wasmSha256, `${path}.wasmSha256`);
  return record as HealthReceipt;
};

export const validateDomainReceipt = (
  input: unknown,
  path = "$receipt",
): DomainReceipt => {
  const record = requireRecord(input, path);
  const domain = requireString(record.domain, `${path}.domain`, 32);
  switch (domain) {
    case "health":
      return validateHealthReceipt(record, path);
    case "project":
      return validateProjectReceipt(record, path);
    case "project-snapshot":
      validateProjectSnapshotReceipt(record, path);
      break;
    case "persistence":
      return validatePersistenceReceipt(record, path);
    case "hardware":
      return validateHardwareReceipt(record, path);
    case "hardware-snapshot":
      validateHardwareSnapshotReceipt(record, path);
      break;
    case "program":
      return validateProgramReceipt(record, path);
    case "program-snapshot":
      validateProgramSnapshotReceipt(record, path);
      break;
    case "build":
      return validateBuildReceipt(record, path);
    case "runtime":
      return validateRuntimeReceipt(record, path);
    case "commissioning":
      return validateCommissioningReceipt(record, path);
    case "monitoring":
      return validateMonitoringReceipt(record, path);
    case "diagnostics":
      validateDiagnosticsReceipt(record, path);
      break;
    default:
      fail(`${path}.domain`, `unknown receipt domain ${JSON.stringify(domain)}`);
  }
  return input as DomainReceipt;
};

export const validateDomainEvent = (
  input: unknown,
  path = "$event",
): DomainEventRecord => {
  const record = requireRecord(input, path);
  const eventKind = requireString(record.eventKind, `${path}.eventKind`, 64);
  const commonKeys = ["eventId", "eventKind", "eventSequence", "transactionId"];
  switch (eventKind) {
    case "project.changed":
      requireExactKeys(
        record,
        [
          ...commonKeys,
          "affectedObjectIds",
          "documentRevision",
          "projectHash",
          "semanticRevision",
        ],
        path,
      );
      validateUuidArray(record.affectedObjectIds, `${path}.affectedObjectIds`);
      requireUInt64(record.documentRevision, `${path}.documentRevision`);
      requireSha256(record.projectHash, `${path}.projectHash`);
      requireUInt64(record.semanticRevision, `${path}.semanticRevision`);
      break;
    case "build.completed":
      requireExactKeys(
        record,
        [...commonKeys, "artifactFingerprint", "attemptId", "outcome"],
        path,
      );
      validateNullable(
        record.artifactFingerprint,
        `${path}.artifactFingerprint`,
        requireSha256,
      );
      requireUuid(record.attemptId, `${path}.attemptId`);
      requireEnum(record.outcome, BUILD_OUTCOMES, `${path}.outcome`);
      if (record.outcome === "Success" && record.artifactFingerprint === null) {
        fail(path, "successful build event requires artifactFingerprint");
      }
      if (record.outcome !== "Success" && record.artifactFingerprint !== null) {
        fail(path, "failed build event cannot identify an artifact");
      }
      break;
    case "runtime.state-changed":
      requireExactKeys(
        record,
        [
          ...commonKeys,
          "controllerEpoch",
          "controllerId",
          "cpuState",
          "virtualTimestampMilliseconds",
        ],
        path,
      );
      requireUInt64(record.controllerEpoch, `${path}.controllerEpoch`);
      requireUuid(record.controllerId, `${path}.controllerId`);
      requireEnum(record.cpuState, CPU_STATES, `${path}.cpuState`);
      requireUInt64(
        record.virtualTimestampMilliseconds,
        `${path}.virtualTimestampMilliseconds`,
      );
      break;
    case "commissioning.changed":
      requireExactKeys(
        record,
        [...commonKeys, "action", "controllerId", "previewId"],
        path,
      );
      requireEnum(record.action, ["preview", "commit", "cancel", "rollback"], `${path}.action`);
      requireUuid(record.controllerId, `${path}.controllerId`);
      requireUuid(record.previewId, `${path}.previewId`);
      break;
    case "monitoring.samples":
      requireExactKeys(
        record,
        [...commonKeys, "samples", "subscriptionId"],
        path,
      );
      validateObservedValues(record.samples, `${path}.samples`);
      requireUuid(record.subscriptionId, `${path}.subscriptionId`);
      break;
    case "diagnostics.changed":
      requireExactKeys(record, [...commonKeys, "diagnosticIds"], path);
      validateUuidArray(record.diagnosticIds, `${path}.diagnosticIds`);
      break;
    default:
      fail(`${path}.eventKind`, `unknown event kind ${JSON.stringify(eventKind)}`);
  }
  requireUuid(record.eventId, `${path}.eventId`);
  requireUInt64(record.eventSequence, `${path}.eventSequence`);
  validateNullable(record.transactionId, `${path}.transactionId`, requireUuid);
  return input as DomainEventRecord;
};

const validateDomainEventArray = (
  input: unknown,
  path: string,
): readonly DomainEventRecord[] => {
  const events = requireArray(input, path, PLC_CONTRACT_LIMITS.eventCount);
  const eventIds: string[] = [];
  let priorSequence: bigint | null = null;
  for (const [index, eventInput] of events.entries()) {
    const event = validateDomainEvent(eventInput, `${path}[${index}]`);
    eventIds.push(event.eventId);
    const sequence = BigInt(event.eventSequence);
    if (priorSequence !== null && sequence <= priorSequence) {
      fail(path, "eventSequence values must be strictly increasing");
    }
    priorSequence = sequence;
  }
  requireUniqueStrings(eventIds, path);
  return events as readonly DomainEventRecord[];
};

const QUERY_RECEIPT_DOMAINS = {
  "build.get-report": "build",
  "commissioning.get-preview": "commissioning",
  "diagnostics.list": "diagnostics",
  "hardware.get-configuration": "hardware-snapshot",
  "monitoring.get-snapshot": "monitoring",
  "persistence.get-status": "persistence",
  "program.get-block": "program-snapshot",
  "project.get-object": "project-snapshot",
  "project.get-summary": "project-snapshot",
  "runtime.get-status": "runtime",
  "system.health": "health",
} as const;

const validateCommandResultBody = (
  record: PlainRecord,
  path: string,
): CommandResultBody => {
  requireExactKeys(
    record,
    [
      "affectedObjectIds",
      "afterProjectHash",
      "beforeProjectHash",
      "commandId",
      "commandKind",
      "diagnostics",
      "events",
      "idempotencyKey",
      "outcome",
      "projectRevisionAfter",
      "projectRevisionBefore",
      "receipt",
      "resultKind",
      "transactionId",
      "undoToken",
    ],
    path,
  );
  const affected = validateUuidArray(
    record.affectedObjectIds,
    `${path}.affectedObjectIds`,
  );
  const afterHash = validateNullable(
    record.afterProjectHash,
    `${path}.afterProjectHash`,
    requireSha256,
  );
  requireSha256(record.beforeProjectHash, `${path}.beforeProjectHash`);
  requireUuid(record.commandId, `${path}.commandId`);
  const commandKind = requireEnum(
    record.commandKind,
    DOMAIN_COMMAND_KINDS,
    `${path}.commandKind`,
  );
  validateDiagnosticArray(record.diagnostics, `${path}.diagnostics`);
  const events = validateDomainEventArray(record.events, `${path}.events`);
  requireIdempotencyKey(record.idempotencyKey, `${path}.idempotencyKey`);
  const outcome = requireEnum(
    record.outcome,
    ["committed", "rejected", "blocked"],
    `${path}.outcome`,
  );
  const afterRevision = validateNullable(
    record.projectRevisionAfter,
    `${path}.projectRevisionAfter`,
    requireUInt64,
  );
  requireUInt64(record.projectRevisionBefore, `${path}.projectRevisionBefore`);
  const receipt = validateNullable(record.receipt, `${path}.receipt`, validateDomainReceipt);
  requireEnum(record.resultKind, ["command"], `${path}.resultKind`);
  const transactionId = requireUuid(record.transactionId, `${path}.transactionId`);
  const undoToken = validateNullable(record.undoToken, `${path}.undoToken`, requireUndoToken);
  if (outcome === "committed") {
    if (afterHash === null || afterRevision === null || receipt === null) {
      fail(path, "committed command requires after hash, revision, and receipt");
    }
    if (receipt.domain !== commandDomain(commandKind)) {
      fail(
        `${path}.receipt.domain`,
        `command ${commandKind} requires a ${commandDomain(commandKind)} receipt`,
      );
    }
    for (const event of events) {
      if (event.transactionId !== transactionId) {
        fail(`${path}.events`, "committed event transactionId does not match result");
      }
    }
  } else if (
    afterHash !== null ||
    afterRevision !== null ||
    affected.length !== 0 ||
    events.length !== 0 ||
    undoToken !== null ||
    receipt !== null
  ) {
    fail(path, "rejected or blocked command must not claim mutation or a receipt");
  }
  return record as CommandResultBody;
};

const validateQueryResultBody = (
  record: PlainRecord,
  path: string,
): QueryResultBody => {
  requireExactKeys(
    record,
    [
      "diagnostics",
      "outcome",
      "queryId",
      "queryKind",
      "receipt",
      "resultKind",
      "snapshotHash",
    ],
    path,
  );
  validateDiagnosticArray(record.diagnostics, `${path}.diagnostics`);
  const outcome = requireEnum(record.outcome, ["ok", "rejected"], `${path}.outcome`);
  requireUuid(record.queryId, `${path}.queryId`);
  const queryKind = requireEnum(
    record.queryKind,
    DOMAIN_QUERY_KINDS,
    `${path}.queryKind`,
  );
  const receipt = validateNullable(record.receipt, `${path}.receipt`, validateDomainReceipt);
  requireEnum(record.resultKind, ["query"], `${path}.resultKind`);
  const snapshotHash = validateNullable(
    record.snapshotHash,
    `${path}.snapshotHash`,
    requireSha256,
  );
  if (outcome === "ok" && (receipt === null || snapshotHash === null)) {
    fail(path, "successful query requires receipt and snapshotHash");
  }
  if (outcome === "ok" && receipt !== null) {
    const expectedDomain = QUERY_RECEIPT_DOMAINS[queryKind];
    if (receipt.domain !== expectedDomain) {
      fail(
        `${path}.receipt.domain`,
        `query ${queryKind} requires a ${expectedDomain} receipt`,
      );
    }
    if (
      receipt.domain === "project-snapshot" &&
      ((queryKind === "project.get-summary" && receipt.scope !== "summary") ||
        (queryKind === "project.get-object" && receipt.scope !== "object"))
    ) {
      fail(`${path}.receipt.scope`, `query ${queryKind} has an incompatible scope`);
    }
    if (
      queryKind === "program.get-block" &&
      receipt.domain === "program-snapshot" &&
      receipt.blocks.length !== 1
    ) {
      fail(`${path}.receipt.blocks`, "program.get-block requires exactly one block snapshot");
    }
    if (
      queryKind === "monitoring.get-snapshot" &&
      receipt.domain === "monitoring" &&
      receipt.action !== "snapshot"
    ) {
      fail(`${path}.receipt.action`, "monitoring snapshot query requires snapshot action");
    }
  }
  if (outcome === "rejected" && (receipt !== null || snapshotHash !== null)) {
    fail(path, "rejected query cannot contain receipt or snapshotHash");
  }
  return record as QueryResultBody;
};

const validateDomainCommandMessage = (
  record: PlainRecord,
  path: string,
): DomainCommandMessage => {
  requireExactKeys(
    record,
    ["command", "context", "kind", "requestId", "schemaVersion"],
    path,
  );
  validateDomainCommand(record.command, `${path}.command`);
  validateCommandContext(record.context, `${path}.context`);
  requireEnum(record.kind, [PLC_MESSAGE_KIND.command], `${path}.kind`);
  requireUuid(record.requestId, `${path}.requestId`);
  if (record.schemaVersion !== PLC_CONTRACT_SCHEMA_VERSION) {
    fail(`${path}.schemaVersion`, "unsupported schema version");
  }
  return record as DomainCommandMessage;
};

const validateDomainQueryMessage = (
  record: PlainRecord,
  path: string,
): DomainQueryMessage => {
  requireExactKeys(record, ["context", "kind", "query", "requestId", "schemaVersion"], path);
  validateQueryContext(record.context, `${path}.context`);
  validateDomainQuery(record.query, `${path}.query`);
  requireEnum(record.kind, [PLC_MESSAGE_KIND.query], `${path}.kind`);
  requireUuid(record.requestId, `${path}.requestId`);
  if (record.schemaVersion !== PLC_CONTRACT_SCHEMA_VERSION) {
    fail(`${path}.schemaVersion`, "unsupported schema version");
  }
  return record as DomainQueryMessage;
};

const validateDomainResultMessage = (
  record: PlainRecord,
  path: string,
): DomainResultMessage => {
  requireExactKeys(record, ["inReplyTo", "kind", "result", "schemaVersion"], path);
  requireUuid(record.inReplyTo, `${path}.inReplyTo`);
  requireEnum(record.kind, [PLC_MESSAGE_KIND.result], `${path}.kind`);
  const result = requireRecord(record.result, `${path}.result`);
  if (result.resultKind === "command") {
    validateCommandResultBody(result, `${path}.result`);
  } else if (result.resultKind === "query") {
    validateQueryResultBody(result, `${path}.result`);
  } else {
    fail(`${path}.result.resultKind`, "unknown result kind");
  }
  if (record.schemaVersion !== PLC_CONTRACT_SCHEMA_VERSION) {
    fail(`${path}.schemaVersion`, "unsupported schema version");
  }
  return record as DomainResultMessage;
};

const validateDomainEventMessage = (
  record: PlainRecord,
  path: string,
): DomainEventMessage => {
  requireExactKeys(record, ["event", "kind", "schemaVersion", "subscriptionId"], path);
  const event = validateDomainEvent(record.event, `${path}.event`);
  requireEnum(record.kind, [PLC_MESSAGE_KIND.event], `${path}.kind`);
  const subscriptionId = validateNullable(
    record.subscriptionId,
    `${path}.subscriptionId`,
    requireUuid,
  );
  if (event.eventKind === "monitoring.samples" && subscriptionId === null) {
    fail(path, "monitoring sample event requires subscriptionId");
  }
  if (
    event.eventKind === "monitoring.samples" &&
    subscriptionId !== event.subscriptionId
  ) {
    fail(`${path}.subscriptionId`, "envelope and event subscription IDs must match");
  }
  if (record.schemaVersion !== PLC_CONTRACT_SCHEMA_VERSION) {
    fail(`${path}.schemaVersion`, "unsupported schema version");
  }
  return record as DomainEventMessage;
};

export const validatePhase2PlcMessage = (
  input: unknown,
  path = "$",
): Phase2PlcMessage => {
  const record = requireRecord(input, path);
  switch (record.kind) {
    case PLC_MESSAGE_KIND.command:
      return validateDomainCommandMessage(record, path);
    case PLC_MESSAGE_KIND.query:
      return validateDomainQueryMessage(record, path);
    case PLC_MESSAGE_KIND.result:
      return validateDomainResultMessage(record, path);
    case PLC_MESSAGE_KIND.event:
      return validateDomainEventMessage(record, path);
    default:
      fail(`${path}.kind`, "unknown Phase 2 message kind");
  }
};

const validateFoundationHealthCommand = (
  record: PlainRecord,
  path: string,
): FoundationHealthCompatibilityCommand => {
  requireExactKeys(record, ["kind", "requestId", "schemaVersion"], path);
  if (
    record.kind !== FOUNDATION_COMPATIBILITY.commandKind ||
    record.requestId !== FOUNDATION_COMPATIBILITY.requestId ||
    record.schemaVersion !== FOUNDATION_COMPATIBILITY.schemaVersion
  ) {
    fail(path, "unsupported Phase 1 health command");
  }
  return record as FoundationHealthCompatibilityCommand;
};

const validateFoundationDiagnostic = (input: unknown, path: string): void => {
  const record = requireRecord(input, path);
  requireExactKeys(record, ["code", "message", "severity"], path);
  requireEnum(
    record.code,
    ["INVALID_COMMAND", "INVALID_WASM", "WORKER_FAILURE"],
    `${path}.code`,
  );
  requireString(record.message, `${path}.message`, 160);
  requireEnum(record.severity, ["error"], `${path}.severity`);
};

const validateFoundationHealthResult = (
  record: PlainRecord,
  path: string,
): FoundationHealthCompatibilityResult => {
  const required = [
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
    required.push("value");
  } else if (record.success !== false) {
    fail(`${path}.success`, "expected a boolean success discriminator");
  }
  requireExactKeys(record, required, path, ["undoToken"]);
  if (
    record.afterHash !== FOUNDATION_COMPATIBILITY.stateHash ||
    record.beforeHash !== FOUNDATION_COMPATIBILITY.stateHash ||
    record.kind !== FOUNDATION_COMPATIBILITY.resultKind ||
    record.requestId !== FOUNDATION_COMPATIBILITY.requestId ||
    record.schemaVersion !== FOUNDATION_COMPATIBILITY.schemaVersion
  ) {
    fail(path, "invalid Phase 1 health result envelope");
  }
  if (requireArray(record.affectedObjectIds, `${path}.affectedObjectIds`, 0).length !== 0) {
    fail(`${path}.affectedObjectIds`, "foundation health result must be empty");
  }
  if (requireArray(record.events, `${path}.events`, 0).length !== 0) {
    fail(`${path}.events`, "foundation health result must be empty");
  }
  if (hasOwn(record, "undoToken")) {
    requireUndoToken(record.undoToken, `${path}.undoToken`);
  }
  if (record.success) {
    if (requireArray(record.diagnostics, `${path}.diagnostics`, 0).length !== 0) {
      fail(`${path}.diagnostics`, "successful foundation result must be empty");
    }
    const value = requireRecord(record.value, `${path}.value`);
    requireExactKeys(
      value,
      ["buildIdentity", "healthState", "schemaVersion", "wasmSha256"],
      `${path}.value`,
    );
    if (
      value.buildIdentity !== FOUNDATION_COMPATIBILITY.buildIdentity ||
      value.healthState !== FOUNDATION_COMPATIBILITY.healthyState ||
      value.schemaVersion !== FOUNDATION_COMPATIBILITY.schemaVersion
    ) {
      fail(`${path}.value`, "invalid Phase 1 health value");
    }
    requireSha256(value.wasmSha256, `${path}.value.wasmSha256`);
  } else {
    const diagnostics = requireArray(record.diagnostics, `${path}.diagnostics`, 1, 1);
    validateFoundationDiagnostic(diagnostics[0], `${path}.diagnostics[0]`);
  }
  return record as FoundationHealthCompatibilityResult;
};

export const validatePlcMessage = (
  input: unknown,
  path = "$",
): PlcWireMessage => {
  const record = requireRecord(input, path);
  if (record.schemaVersion === FOUNDATION_COMPATIBILITY.schemaVersion) {
    if (record.kind === FOUNDATION_COMPATIBILITY.commandKind) {
      return validateFoundationHealthCommand(record, path);
    }
    if (record.kind === FOUNDATION_COMPATIBILITY.resultKind) {
      return validateFoundationHealthResult(record, path);
    }
    fail(`${path}.kind`, "unknown Phase 1 compatibility message");
  }
  if (record.schemaVersion === PLC_CONTRACT_SCHEMA_VERSION) {
    return validatePhase2PlcMessage(record, path);
  }
  fail(`${path}.schemaVersion`, "unsupported message schema version");
};

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

export const decodePlcMessage = (serialized: string): PlcWireMessage => {
  if (typeof serialized !== "string") {
    fail("$serialized", "expected a serialized JSON string");
  }
  const bytes = utf8ByteLength(serialized);
  if (bytes < 2 || bytes > PLC_CONTRACT_LIMITS.messageBytes) {
    fail(
      "$serialized",
      `serialized message must be 2..${PLC_CONTRACT_LIMITS.messageBytes} UTF-8 bytes`,
    );
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(serialized) as unknown;
  } catch {
    fail("$serialized", "malformed JSON");
  }
  return validatePlcMessage(parsed);
};

export const encodePlcMessage = (message: PlcWireMessage): string => {
  validatePlcMessage(message);
  const serialized = JSON.stringify(message);
  if (utf8ByteLength(serialized) > PLC_CONTRACT_LIMITS.messageBytes) {
    fail("$", "serialized message exceeds the wire byte limit");
  }
  return serialized;
};
