const PROJECT_EXTENSION = ".vlabproj";
const MAX_PROJECT_BYTES = 32 * 1024 * 1024;
const MAX_PROJECT_NAME_CODE_UNITS = 255;

const FORBIDDEN_FILE_NAME_CHARACTERS = /[<>:"/\\|?*\u0000-\u001f\u007f]/u;
const FORBIDDEN_FILE_NAME_CHARACTERS_GLOBAL = /[<>:"/\\|?*\u0000-\u001f\u007f]/gu;
const RESERVED_WINDOWS_FILE_STEM = /^(?:AUX|CON|NUL|PRN|COM[1-9]|LPT[1-9])$/iu;
const NATIVE_GRANT_ID = /^p2-native-v1:[0-9a-f]{16}$/u;
const ATTESTATION_ID = /^fixed-local-v1:[0-9A-F]{8}:[0-9A-F]{16}$/u;

export const NATIVE_PROJECT_BROKER_CONTRACT = "govs.project-file-broker" as const;
export const NATIVE_PROJECT_BROKER_VERSION = 1 as const;
export const NATIVE_PROJECT_BROKER_GLOBAL = "govsProjectFileBrokerV1" as const;

export type NativeFixedLocalAttestationV1 = Readonly<{
  attestationId: string;
  fixedDrive: true;
  kind: "fixed-native-local-v1";
  nativeLocal: true;
  platform: "windows";
  providerBacked: false;
  redirected: false;
  removable: false;
  special: false;
}>;

export type NativeOpenedProjectV1 = Readonly<{
  attestationId: string;
  bytes: Uint8Array<ArrayBuffer>;
  displayName: string;
  grantId: string;
  protocolVersion: 1;
}>;

export type NativeSavedProjectV1 = Readonly<{
  attestationId: string;
  displayName: string;
  grantId: string;
  protocolVersion: 1;
  verifiedBytes: number;
}>;

export type NativeProjectFileBrokerV1 = Readonly<{
  attestation: NativeFixedLocalAttestationV1;
  contract: typeof NATIVE_PROJECT_BROKER_CONTRACT;
  open: () => Promise<NativeOpenedProjectV1>;
  protocolVersion: typeof NATIVE_PROJECT_BROKER_VERSION;
  revoke: (grantId: string) => void;
  save: (request: Readonly<{
    bytes: Uint8Array<ArrayBuffer>;
    grantId: string;
    protocolVersion: 1;
  }>) => Promise<NativeSavedProjectV1>;
  saveAs: (request: Readonly<{
    bytes: Uint8Array<ArrayBuffer>;
    projectName: string;
    protocolVersion: 1;
  }>) => Promise<NativeSavedProjectV1>;
}>;

declare global {
  interface Window {
    readonly govsProjectFileBrokerV1?: NativeProjectFileBrokerV1;
  }
}

export type OpenedProjectFile = Readonly<{
  bytes: Uint8Array<ArrayBuffer>;
  displayName: string;
  grantId: string;
}>;

export type SavedProjectFile = Readonly<{
  displayName: string;
  grantId: string;
  verifiedBytes: number;
}>;

type ActiveGrant = Readonly<{
  attestationId: string;
  displayName: string;
}>;

export class FileAccessError extends Error {
  public readonly code:
    | "ACCESS_CANCELLED"
    | "ACCESS_UNAVAILABLE"
    | "ATTESTATION_FAILED"
    | "INVALID_EXTENSION"
    | "INVALID_FILE_NAME"
    | "PROJECT_TOO_LARGE"
    | "PROTOCOL_MISMATCH"
    | "READ_FAILED"
    | "UNKNOWN_GRANT"
    | "WRITE_FAILED";

  public constructor(code: FileAccessError["code"], message: string) {
    super(message);
    this.name = "FileAccessError";
    this.code = code;
  }
}

/**
 * Sole production adapter for project-file access.
 *
 * Ordinary web picker handles are intentionally ignored: their metadata cannot
 * attest fixed native local backing. Only a pre-document, non-replaceable
 * Windows shell bridge with the exact V1 surface can activate this boundary.
 * The renderer receives typed bytes and opaque grants, never a path, generic
 * host invocation method, native method name, or filesystem capability.
 */
export class FileAccessBroker {
  readonly #grants = new Map<string, ActiveGrant>();

  public canOpen(): boolean {
    return inspectNativeBridge() !== null;
  }

  public canSave(): boolean {
    return inspectNativeBridge() !== null;
  }

  public async requestOpen(): Promise<OpenedProjectFile> {
    const bridge = requireNativeBridge();
    let opened: NativeOpenedProjectV1;
    try {
      opened = await bridge.open();
    } catch (error) {
      throw normalizeNativeError(error, "open");
    }
    try {
      assertExactKeys(opened, [
        "attestationId",
        "bytes",
        "displayName",
        "grantId",
        "protocolVersion",
      ], "READ_FAILED");
      assertNativeResultIdentity(opened, bridge.attestation, "open");
      assertProjectName(opened.displayName);
      assertNativeGrantId(opened.grantId, "open");
      if (!(opened.bytes instanceof Uint8Array)) {
        throw boundaryFailure("open", "The native broker returned an invalid project payload.");
      }
      assertProjectSize(opened.bytes.byteLength);
    } catch (error) {
      revokeReturnedGrant(bridge, opened);
      throw error;
    }
    const bytes = opened.bytes.slice();
    this.#grants.set(opened.grantId, {
      attestationId: opened.attestationId,
      displayName: opened.displayName,
    });
    return {
      bytes,
      displayName: opened.displayName,
      grantId: opened.grantId,
    };
  }

  public async requestSaveAs(
    suggestedProjectName: string,
    bytes: Uint8Array<ArrayBuffer>,
  ): Promise<SavedProjectFile> {
    const bridge = requireNativeBridge();
    assertProjectSize(bytes.byteLength);
    let saved: NativeSavedProjectV1;
    try {
      saved = await bridge.saveAs(Object.freeze({
        bytes: bytes.slice(),
        projectName: normalizeSuggestedName(suggestedProjectName),
        protocolVersion: NATIVE_PROJECT_BROKER_VERSION,
      }));
    } catch (error) {
      throw normalizeNativeError(error, "save");
    }
    let result: SavedProjectFile;
    try {
      result = inspectSavedResult(saved, bridge.attestation, "save", bytes.byteLength);
    } catch (error) {
      revokeReturnedGrant(bridge, saved);
      throw error;
    }
    this.#grants.set(result.grantId, {
      attestationId: saved.attestationId,
      displayName: result.displayName,
    });
    return result;
  }

  public async save(
    grantId: string,
    bytes: Uint8Array<ArrayBuffer>,
  ): Promise<SavedProjectFile> {
    const bridge = requireNativeBridge();
    assertProjectSize(bytes.byteLength);
    const active = this.#grants.get(grantId);
    if (active === undefined || active.attestationId !== bridge.attestation.attestationId) {
      throw new FileAccessError("UNKNOWN_GRANT", "The native project file grant is no longer active.");
    }
    let saved: NativeSavedProjectV1;
    try {
      saved = await bridge.save(Object.freeze({
        bytes: bytes.slice(),
        grantId,
        protocolVersion: NATIVE_PROJECT_BROKER_VERSION,
      }));
    } catch (error) {
      const normalized = normalizeNativeError(error, "save");
      if (normalized.code === "UNKNOWN_GRANT") {
        this.#grants.delete(grantId);
      }
      throw normalized;
    }
    let result: SavedProjectFile;
    try {
      result = inspectSavedResult(saved, bridge.attestation, "save", bytes.byteLength);
    } catch (error) {
      revokeReturnedGrant(bridge, saved);
      this.#grants.delete(grantId);
      throw error;
    }
    if (result.grantId !== grantId || result.displayName !== active.displayName) {
      this.#grants.delete(grantId);
      throw boundaryFailure("save", "The native broker changed the active file grant identity.");
    }
    return result;
  }

  public revoke(grantId: string): void {
    const active = this.#grants.get(grantId);
    this.#grants.delete(grantId);
    if (active === undefined) {
      return;
    }
    const bridge = inspectNativeBridge();
    if (bridge !== null && bridge.attestation.attestationId === active.attestationId) {
      bridge.revoke(grantId);
    }
  }
}

const inspectNativeBridge = (): NativeProjectFileBrokerV1 | null => {
  if (typeof window !== "object" || window === null) {
    return null;
  }
  const descriptor = Object.getOwnPropertyDescriptor(window, NATIVE_PROJECT_BROKER_GLOBAL);
  if (
    descriptor === undefined ||
    descriptor.configurable !== false ||
    descriptor.enumerable !== false ||
    "get" in descriptor ||
    descriptor.writable !== false ||
    descriptor.value === null ||
    typeof descriptor.value !== "object"
  ) {
    return null;
  }
  const candidate = descriptor.value as Partial<NativeProjectFileBrokerV1>;
  try {
    if (!isPlainFrozenObject(candidate)) {
      return null;
    }
    assertExactKeys(candidate, [
      "attestation",
      "contract",
      "open",
      "protocolVersion",
      "revoke",
      "save",
      "saveAs",
    ], "ACCESS_UNAVAILABLE");
    if (
      candidate.contract !== NATIVE_PROJECT_BROKER_CONTRACT ||
      candidate.protocolVersion !== NATIVE_PROJECT_BROKER_VERSION ||
      typeof candidate.open !== "function" ||
      typeof candidate.saveAs !== "function" ||
      typeof candidate.save !== "function" ||
      typeof candidate.revoke !== "function" ||
      !isFixedLocalAttestation(candidate.attestation)
    ) {
      return null;
    }
    return candidate as NativeProjectFileBrokerV1;
  } catch {
    return null;
  }
};

const requireNativeBridge = (): NativeProjectFileBrokerV1 => {
  const bridge = inspectNativeBridge();
  if (bridge === null) {
    throw new FileAccessError(
      "ACCESS_UNAVAILABLE",
      "Project files require the approved Windows native fixed-local broker; browser file pickers are not accepted.",
    );
  }
  return bridge;
};

const isFixedLocalAttestation = (
  value: NativeFixedLocalAttestationV1 | undefined,
): value is NativeFixedLocalAttestationV1 => {
  if (value === undefined || value === null || typeof value !== "object") {
    return false;
  }
  try {
    if (!isPlainFrozenObject(value)) {
      return false;
    }
    assertExactKeys(value, [
      "attestationId",
      "fixedDrive",
      "kind",
      "nativeLocal",
      "platform",
      "providerBacked",
      "redirected",
      "removable",
      "special",
    ], "ATTESTATION_FAILED");
  } catch {
    return false;
  }
  return value.kind === "fixed-native-local-v1"
    && value.platform === "windows"
    && value.fixedDrive === true
    && value.nativeLocal === true
    && value.providerBacked === false
    && value.redirected === false
    && value.removable === false
    && value.special === false
    && ATTESTATION_ID.test(value.attestationId);
};

const assertNativeResultIdentity = (
  result: Readonly<{ attestationId: string; protocolVersion: number }>,
  attestation: NativeFixedLocalAttestationV1,
  operation: "open" | "save",
): void => {
  if (!isPlainFrozenObject(result)) {
    throw boundaryFailure(operation, "The native broker result was not an immutable plain record.");
  }
  if (
    result.protocolVersion !== NATIVE_PROJECT_BROKER_VERSION ||
    result.attestationId !== attestation.attestationId
  ) {
    throw boundaryFailure(operation, "The native broker result does not match its fixed-local attestation.");
  }
};

const inspectSavedResult = (
  saved: NativeSavedProjectV1,
  attestation: NativeFixedLocalAttestationV1,
  operation: "save",
  expectedBytes: number,
): SavedProjectFile => {
  assertExactKeys(saved, [
    "attestationId",
    "displayName",
    "grantId",
    "protocolVersion",
    "verifiedBytes",
  ], "WRITE_FAILED");
  assertNativeResultIdentity(saved, attestation, operation);
  assertProjectName(saved.displayName);
  assertNativeGrantId(saved.grantId, operation);
  assertProjectSize(saved.verifiedBytes);
  if (saved.verifiedBytes !== expectedBytes) {
    throw boundaryFailure(operation, "The native broker did not verify the complete save payload.");
  }
  return {
    displayName: saved.displayName,
    grantId: saved.grantId,
    verifiedBytes: saved.verifiedBytes,
  };
};

const assertNativeGrantId = (grantId: string, operation: "open" | "save"): void => {
  if (!NATIVE_GRANT_ID.test(grantId)) {
    throw boundaryFailure(operation, "The native broker returned an invalid opaque grant.");
  }
};

const normalizeNativeError = (
  error: unknown,
  operation: "open" | "save",
): FileAccessError => {
  if (error instanceof FileAccessError) {
    return error;
  }
  if (error !== null && typeof error === "object") {
    const descriptor = Object.getOwnPropertyDescriptor(error, "code");
    const code = descriptor !== undefined && "value" in descriptor
      ? String(descriptor.value)
      : "";
    if (code === "ACCESS_CANCELLED") {
      return new FileAccessError("ACCESS_CANCELLED", `Project ${operation} was cancelled.`);
    }
    if (code === "ATTESTATION_FAILED") {
      return new FileAccessError(
        "ATTESTATION_FAILED",
        "The selected target was rejected because fixed native local backing was not proven.",
      );
    }
    if (code === "UNKNOWN_GRANT" || code === "STALE_GRANT") {
      return new FileAccessError("UNKNOWN_GRANT", "The native project file grant is no longer active.");
    }
  }
  return boundaryFailure(
    operation,
    operation === "open"
      ? "The native broker did not open the project."
      : "The native broker did not save and reverify the project.",
  );
};

const boundaryFailure = (
  operation: "open" | "save",
  message: string,
): FileAccessError => new FileAccessError(
  operation === "open" ? "READ_FAILED" : "WRITE_FAILED",
  message,
);

const assertProjectName = (name: string): void => {
  if (!name.toLocaleLowerCase("en-US").endsWith(PROJECT_EXTENSION)) {
    throw new FileAccessError(
      "INVALID_EXTENSION",
      `Project files must use the ${PROJECT_EXTENSION} extension.`,
    );
  }
  if (
    !/^[\x20-\x7e]+$/u.test(name) ||
    name.length > MAX_PROJECT_NAME_CODE_UNITS ||
    name !== name.trim() ||
    name.endsWith(".") ||
    FORBIDDEN_FILE_NAME_CHARACTERS.test(name)
  ) {
    throw new FileAccessError(
      "INVALID_FILE_NAME",
      "Project files must use one bounded ASCII file name, not a path, endpoint, device, pipe, or print target.",
    );
  }
  const stem = name.slice(0, -PROJECT_EXTENSION.length);
  if (
    stem.length === 0 ||
    stem !== stem.trim() ||
    stem.endsWith(".") ||
    stem === "." ||
    stem === ".." ||
    !/^[A-Za-z0-9 _().-]+$/u.test(stem) ||
    isReservedProjectStem(stem)
  ) {
    throw new FileAccessError(
      "INVALID_FILE_NAME",
      "The selected project file name is reserved, unsafe, or empty.",
    );
  }
};

const assertProjectSize = (size: number): void => {
  if (!Number.isSafeInteger(size) || size < 1 || size > MAX_PROJECT_BYTES) {
    throw new FileAccessError(
      "PROJECT_TOO_LARGE",
      `Project files must be between 1 byte and ${MAX_PROJECT_BYTES} bytes.`,
    );
  }
};

const normalizeSuggestedName = (name: string): string => {
  const withoutExtension = name.toLocaleLowerCase("en-US").endsWith(PROJECT_EXTENSION)
    ? name.slice(0, -PROJECT_EXTENSION.length)
    : name;
  const normalized = withoutExtension
    .replaceAll(FORBIDDEN_FILE_NAME_CHARACTERS_GLOBAL, " ")
    .replace(/[^A-Za-z0-9 _().-]/gu, " ")
    .replace(/\s+/gu, " ")
    .trim()
    .replace(/[. ]+$/u, "");
  const admissible = normalized.length > 0 && !isReservedProjectStem(normalized)
    ? normalized
    : "Untitled project";
  const maximumStemLength = MAX_PROJECT_NAME_CODE_UNITS - PROJECT_EXTENSION.length;
  const bounded = admissible.slice(0, maximumStemLength).replace(/[. ]+$/u, "");
  return `${bounded || "Untitled project"}${PROJECT_EXTENSION}`;
};

const isReservedProjectStem = (stem: string): boolean => {
  const candidate = stem.split(".", 1)[0]?.replace(/[. ]+$/u, "") ?? "";
  return RESERVED_WINDOWS_FILE_STEM.test(candidate);
};

const assertExactKeys = (
  value: object,
  expected: readonly string[],
  code: "ACCESS_UNAVAILABLE" | "ATTESTATION_FAILED" | "READ_FAILED" | "WRITE_FAILED",
): void => {
  const ownKeys = Reflect.ownKeys(value);
  if (ownKeys.some((key) => typeof key !== "string")) {
    throw new FileAccessError(code, "The native project broker exposed a symbol capability.");
  }
  const actual = (ownKeys as string[]).sort();
  const required = [...expected].sort();
  if (actual.length !== required.length || actual.some((key, index) => key !== required[index])) {
    throw new FileAccessError(code, "The native project broker exposed an unexpected capability surface.");
  }
};

const isPlainFrozenObject = (value: object): boolean =>
  Object.getPrototypeOf(value) === Object.prototype && Object.isFrozen(value);

const revokeReturnedGrant = (
  bridge: NativeProjectFileBrokerV1,
  result: unknown,
): void => {
  if (result === null || typeof result !== "object") {
    return;
  }
  const descriptor = Object.getOwnPropertyDescriptor(result, "grantId");
  if (
    descriptor === undefined ||
    !("value" in descriptor) ||
    typeof descriptor.value !== "string" ||
    !NATIVE_GRANT_ID.test(descriptor.value)
  ) {
    return;
  }
  try {
    bridge.revoke(descriptor.value);
  } catch {
    // The native shell terminates its broker if revoke is not acknowledged.
    // Preserve the original result-validation failure at this boundary.
  }
};
