import {
  FOUNDATION_BUILD_IDENTITY,
  FOUNDATION_HEALTHY_STATE,
  FOUNDATION_RESULT_KIND,
  FOUNDATION_SCHEMA_VERSION,
  FOUNDATION_STATE_HASH,
  createFoundationFailure,
  validateFoundationHealthCommand,
  validateFoundationHealthResult,
} from "@govs/foundation-contract";
import type {
  FoundationHealthCommand,
  FoundationHealthResult,
} from "@govs/foundation-contract";

import {
  FOUNDATION_WASM_BASE64,
  FOUNDATION_WASM_SHA256,
} from "./generated/foundation-wasm";

const MAX_WASM_BYTES = 128 * 1024;
const MAX_PAYLOAD_BYTES = 512;

type FoundationWasmExports = Readonly<{
  foundation_health: () => number;
  foundation_health_len: () => number;
  memory: WebAssembly.Memory;
}>;

type RustHealthPayload = Readonly<{
  buildIdentity: string;
  healthState: string;
  schemaVersion: number;
}>;

export const executeFoundationCommand = async (
  input: unknown,
): Promise<FoundationHealthResult> => {
  let command: FoundationHealthCommand;
  try {
    command = validateFoundationHealthCommand(input);
  } catch {
    return createFoundationFailure(
      "INVALID_COMMAND",
      "The foundation command was rejected.",
    );
  }

  try {
    const payload = await runRustHealthCheck();
    const result = {
      affectedObjectIds: [],
      afterHash: FOUNDATION_STATE_HASH,
      beforeHash: FOUNDATION_STATE_HASH,
      diagnostics: [],
      events: [],
      kind: FOUNDATION_RESULT_KIND,
      requestId: command.requestId,
      schemaVersion: command.schemaVersion,
      success: true,
      value: {
        buildIdentity: payload.buildIdentity,
        healthState: payload.healthState,
        schemaVersion: payload.schemaVersion,
        wasmSha256: FOUNDATION_WASM_SHA256,
      },
    } as const;
    return validateFoundationHealthResult(result);
  } catch {
    return createFoundationFailure(
      "INVALID_WASM",
      "The embedded foundation module did not pass its health check.",
    );
  }
};

const runRustHealthCheck = async (): Promise<RustHealthPayload> => {
  const wasmBytes = decodeBase64(FOUNDATION_WASM_BASE64);
  if (wasmBytes.byteLength < 8 || wasmBytes.byteLength > MAX_WASM_BYTES) {
    throw new Error("WASM byte budget violation.");
  }

  const module = await WebAssembly.compile(wasmBytes.buffer);
  if (WebAssembly.Module.imports(module).length !== 0) {
    throw new Error("WASM module imports are not allowed.");
  }

  const instance = await WebAssembly.instantiate(module, {});
  const exports = readExports(instance.exports);
  const pointer = exports.foundation_health();
  const length = exports.foundation_health_len();
  if (
    !Number.isSafeInteger(pointer) ||
    !Number.isSafeInteger(length) ||
    length < 1 ||
    length > MAX_PAYLOAD_BYTES ||
    pointer < 0 ||
    pointer + length > exports.memory.buffer.byteLength
  ) {
    throw new Error("Invalid WASM payload bounds.");
  }

  const bytes = new Uint8Array(exports.memory.buffer, pointer, length);
  const parsed = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes));
  return validateRustPayload(parsed);
};

const decodeBase64 = (encoded: string): Uint8Array<ArrayBuffer> => {
  const binary = atob(encoded);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
};

const readExports = (exports: WebAssembly.Exports): FoundationWasmExports => {
  const memory = exports.memory;
  const health = exports.foundation_health;
  const healthLength = exports.foundation_health_len;
  if (
    !(memory instanceof WebAssembly.Memory) ||
    typeof health !== "function" ||
    typeof healthLength !== "function"
  ) {
    throw new Error("Required WASM exports are missing.");
  }
  return {
    foundation_health: health as () => number,
    foundation_health_len: healthLength as () => number,
    memory,
  };
};

const validateRustPayload = (input: unknown): RustHealthPayload => {
  if (
    typeof input !== "object" ||
    input === null ||
    Array.isArray(input) ||
    Object.getPrototypeOf(input) !== Object.prototype
  ) {
    throw new Error("Invalid WASM health payload.");
  }
  const record = input as Record<string, unknown>;
  const keys = Object.keys(record).sort();
  if (keys.join(",") !== "buildIdentity,healthState,schemaVersion") {
    throw new Error("Unexpected WASM health fields.");
  }
  if (
    record.buildIdentity !== FOUNDATION_BUILD_IDENTITY ||
    record.healthState !== FOUNDATION_HEALTHY_STATE ||
    record.schemaVersion !== FOUNDATION_SCHEMA_VERSION
  ) {
    throw new Error("Unexpected WASM health value.");
  }
  return {
    buildIdentity: FOUNDATION_BUILD_IDENTITY,
    healthState: FOUNDATION_HEALTHY_STATE,
    schemaVersion: FOUNDATION_SCHEMA_VERSION,
  };
};
