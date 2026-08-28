import {
  ENGINEERING_WASM_BASE64,
  ENGINEERING_WASM_SHA256,
} from "./generated/plc-engineering-wasm";

const MAX_WASM_BYTES = 8 * 1024 * 1024;
const MAX_BRIDGE_BYTES = 32 * 1024 * 1024;

type KernelWasmExports = Readonly<{
  foundation_health: () => number;
  foundation_health_len: () => number;
  memory: WebAssembly.Memory;
  plc_input_prepare: (length: number) => number;
  plc_output_length: () => number;
  plc_output_pointer: () => number;
  plc_session_abort_save: () => number;
  plc_session_commit_save: (verifiedBytes: number, digestLength: number) => number;
  plc_session_create: (length: number) => number;
  plc_session_export_replay_baseline: () => number;
  plc_session_handle: (length: number) => number;
  plc_session_open: (length: number) => number;
  plc_session_prepare_save: (mode: number, length: number) => number;
  plc_session_system_query: () => number;
  plc_session_system_command: (length: number) => number;
  plc_session_verify_replay_package: (length: number) => number;
}>;

export type WasmHealth = Readonly<{
  buildIdentity: string;
  healthState: string;
  schemaVersion: number;
  wasmSha256: string;
}>;

export class WasmKernelError extends Error {
  public constructor(message: string) {
    super(message);
    this.name = "WasmKernelError";
  }
}

export class WasmKernel {
  readonly #exports: KernelWasmExports;

  private constructor(exports: KernelWasmExports) {
    this.#exports = exports;
  }

  public static async load(): Promise<WasmKernel> {
    const wasmBytes = decodeBase64(ENGINEERING_WASM_BASE64);
    if (wasmBytes.byteLength < 8 || wasmBytes.byteLength > MAX_WASM_BYTES) {
      throw new WasmKernelError("The embedded engineering core violates its byte budget.");
    }
    const module = await WebAssembly.compile(wasmBytes.buffer);
    if (WebAssembly.Module.imports(module).length !== 0) {
      throw new WasmKernelError("The embedded engineering core has forbidden imports.");
    }
    const instance = await WebAssembly.instantiate(module, {});
    return new WasmKernel(readExports(instance.exports));
  }

  public health(): WasmHealth {
    const pointer = this.#exports.foundation_health();
    const length = this.#exports.foundation_health_len();
    const value = parseJson(this.read(pointer, length));
    if (
      !isRecord(value) ||
      typeof value.buildIdentity !== "string" ||
      value.healthState !== "HEALTHY" ||
      value.schemaVersion !== 1
    ) {
      throw new WasmKernelError("The engineering core health receipt is invalid.");
    }
    return {
      buildIdentity: value.buildIdentity,
      healthState: "HEALTHY",
      schemaVersion: 1,
      wasmSha256: ENGINEERING_WASM_SHA256,
    };
  }

  public create(request: Uint8Array<ArrayBuffer>): Uint8Array<ArrayBuffer> {
    return this.callWithInput(request, (length) => this.#exports.plc_session_create(length));
  }

  public open(packageBytes: Uint8Array<ArrayBuffer>): Uint8Array<ArrayBuffer> {
    return this.callWithInput(packageBytes, (length) => this.#exports.plc_session_open(length));
  }

  public handle(request: Uint8Array<ArrayBuffer>): Uint8Array<ArrayBuffer> {
    return this.callWithInput(request, (length) => this.#exports.plc_session_handle(length));
  }

  public systemQuery(): Uint8Array<ArrayBuffer> {
    return this.call(() => this.#exports.plc_session_system_query());
  }

  public systemHandle(request: Uint8Array<ArrayBuffer>): Uint8Array<ArrayBuffer> {
    return this.callWithInput(
      request,
      (length) => this.#exports.plc_session_system_command(length),
    );
  }

  public exportReplayBaseline(): Uint8Array<ArrayBuffer> {
    return this.call(() => this.#exports.plc_session_export_replay_baseline());
  }

  public verifyReplayPackage(
    packageBytes: Uint8Array<ArrayBuffer>,
  ): Uint8Array<ArrayBuffer> {
    return this.callWithInput(
      packageBytes,
      (length) => this.#exports.plc_session_verify_replay_package(length),
    );
  }

  public prepareSave(
    mode: "save" | "save-as",
    saveAsDocumentId: string | null,
  ): Uint8Array<ArrayBuffer> {
    if (mode === "save") {
      return this.call(() => this.#exports.plc_session_prepare_save(0, 0));
    }
    if (saveAsDocumentId === null) {
      throw new WasmKernelError("Save As requires a new document identity.");
    }
    return this.callWithInput(
      new TextEncoder().encode(saveAsDocumentId),
      (length) => this.#exports.plc_session_prepare_save(1, length),
    );
  }

  public commitSave(
    verifiedBytes: number,
    preparedDigest: Uint8Array<ArrayBuffer>,
  ): Uint8Array<ArrayBuffer> {
    if (!Number.isSafeInteger(verifiedBytes) || verifiedBytes < 1 || verifiedBytes > MAX_BRIDGE_BYTES) {
      throw new WasmKernelError("The verified save length is invalid.");
    }
    if (preparedDigest.byteLength !== 32) {
      throw new WasmKernelError("The prepared save digest is invalid.");
    }
    return this.callWithInput(
      preparedDigest,
      (length) => this.#exports.plc_session_commit_save(verifiedBytes, length),
    );
  }

  public abortSave(): void {
    this.call(() => this.#exports.plc_session_abort_save());
  }

  private callWithInput(
    input: Uint8Array<ArrayBuffer>,
    invoke: (length: number) => number,
  ): Uint8Array<ArrayBuffer> {
    if (input.byteLength < 1 || input.byteLength > MAX_BRIDGE_BYTES) {
      throw new WasmKernelError("The engineering command violates its byte budget.");
    }
    const pointer = this.#exports.plc_input_prepare(input.byteLength);
    const memory = this.#exports.memory.buffer;
    if (
      !Number.isSafeInteger(pointer) ||
      pointer < 1 ||
      pointer + input.byteLength > memory.byteLength
    ) {
      throw new WasmKernelError("The engineering core rejected its input buffer.");
    }
    new Uint8Array(memory, pointer, input.byteLength).set(input);
    return this.call(() => invoke(input.byteLength));
  }

  private call(invoke: () => number): Uint8Array<ArrayBuffer> {
    const status = invoke();
    const output = this.read(
      this.#exports.plc_output_pointer(),
      this.#exports.plc_output_length(),
    );
    if (status !== 0) {
      const parsed = parseJson(output);
      const message = isRecord(parsed) && typeof parsed.error === "string"
        ? parsed.error
        : "The engineering core rejected the request.";
      throw new WasmKernelError(message);
    }
    return output;
  }

  private read(pointer: number, length: number): Uint8Array<ArrayBuffer> {
    const memory = this.#exports.memory.buffer;
    if (
      !Number.isSafeInteger(pointer) ||
      !Number.isSafeInteger(length) ||
      pointer < 0 ||
      length < 1 ||
      length > MAX_BRIDGE_BYTES ||
      pointer + length > memory.byteLength
    ) {
      throw new WasmKernelError("The engineering core returned invalid output bounds.");
    }
    return new Uint8Array(memory, pointer, length).slice();
  }
}

const decodeBase64 = (encoded: string): Uint8Array<ArrayBuffer> => {
  const binary = atob(encoded);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
};

const parseJson = (bytes: Uint8Array): unknown => {
  try {
    return JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)) as unknown;
  } catch {
    throw new WasmKernelError("The engineering core returned malformed UTF-8 JSON.");
  }
};

const isRecord = (value: unknown): value is Readonly<Record<string, unknown>> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const readExports = (exports: WebAssembly.Exports): KernelWasmExports => {
  const names = [
    "foundation_health",
    "foundation_health_len",
    "plc_input_prepare",
    "plc_output_length",
    "plc_output_pointer",
    "plc_session_abort_save",
    "plc_session_commit_save",
    "plc_session_create",
    "plc_session_export_replay_baseline",
    "plc_session_handle",
    "plc_session_open",
    "plc_session_prepare_save",
    "plc_session_system_query",
    "plc_session_system_command",
    "plc_session_verify_replay_package",
  ] as const;
  if (!(exports.memory instanceof WebAssembly.Memory)) {
    throw new WasmKernelError("The engineering core memory export is missing.");
  }
  for (const name of names) {
    if (typeof exports[name] !== "function") {
      throw new WasmKernelError(`The engineering core export ${name} is missing.`);
    }
  }
  return exports as unknown as KernelWasmExports;
};
