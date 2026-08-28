const PROJECT_EXTENSION = ".vlabproj";
const PROJECT_MIME_TYPE = "application/vnd.govs.virtual-plc-project";
const MAX_PROJECT_BYTES = 32 * 1024 * 1024;
const MAX_PROJECT_NAME_CODE_UNITS = 255;

const FORBIDDEN_FILE_NAME_CHARACTERS = /[<>:"/\\|?*\u0000-\u001f\u007f]/u;
const FORBIDDEN_FILE_NAME_CHARACTERS_GLOBAL = /[<>:"/\\|?*\u0000-\u001f\u007f]/gu;
const RESERVED_WINDOWS_FILE_STEM = /^(?:AUX|CON|NUL|PRN|COM[1-9¹²³]|LPT[1-9¹²³])$/iu;

type PickerAccept = Readonly<Record<string, readonly string[]>>;

type ProjectFileHandle = Readonly<{
  getFile: () => Promise<File>;
  createWritable: () => Promise<{
    abort: () => Promise<void>;
    close: () => Promise<void>;
    write: (data: Uint8Array<ArrayBuffer>) => Promise<void>;
  }>;
  kind: "file";
  name: string;
}>;

type ProjectFileGrant = Readonly<{
  displayName: string;
  handle: ProjectFileHandle;
}>;

type PickerWindow = Window &
  typeof globalThis &
  Readonly<{
    showOpenFilePicker?: (options: Readonly<{
      excludeAcceptAllOption: boolean;
      multiple: false;
      types: readonly Readonly<{
        accept: PickerAccept;
        description: string;
      }>[];
    }>) => Promise<readonly ProjectFileHandle[]>;
    showSaveFilePicker?: (options: Readonly<{
      excludeAcceptAllOption: boolean;
      suggestedName: string;
      types: readonly Readonly<{
        accept: PickerAccept;
        description: string;
      }>[];
    }>) => Promise<ProjectFileHandle>;
  }>;

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

export class FileAccessError extends Error {
  public readonly code:
    | "ACCESS_CANCELLED"
    | "ACCESS_UNAVAILABLE"
    | "INVALID_EXTENSION"
    | "INVALID_FILE_NAME"
    | "PROJECT_TOO_LARGE"
    | "READ_FAILED"
    | "UNKNOWN_GRANT"
    | "WRITE_FAILED";

  public constructor(code: FileAccessError["code"], message: string) {
    super(message);
    this.name = "FileAccessError";
    this.code = code;
  }
}

const projectPickerType = {
  accept: { [PROJECT_MIME_TYPE]: [PROJECT_EXTENSION] },
  description: "Virtual PLC Lab project",
} as const;

/**
 * The only production boundary allowed to touch project files.
 *
 * Handles remain private to this broker. The domain receives opaque grant IDs
 * and canonical bytes, never a path or a host capability.
 */
export class FileAccessBroker {
  readonly #grants = new Map<string, ProjectFileGrant>();

  public canOpen(): boolean {
    return typeof (window as PickerWindow).showOpenFilePicker === "function";
  }

  public canSave(): boolean {
    return typeof (window as PickerWindow).showSaveFilePicker === "function";
  }

  public async requestOpen(): Promise<OpenedProjectFile> {
    const picker = (window as PickerWindow).showOpenFilePicker;
    if (picker === undefined) {
      throw new FileAccessError(
        "ACCESS_UNAVAILABLE",
        "This browser cannot grant project-file access.",
      );
    }

    let handles: readonly ProjectFileHandle[];
    try {
      handles = await picker({
        excludeAcceptAllOption: true,
        multiple: false,
        types: [projectPickerType],
      });
    } catch (error) {
      throw normalizePickerError(error, "open");
    }
    const grant = inspectGrantedHandle(handles[0], "open");
    const bytes = await readProjectBytes(grant.handle);
    const grantId = crypto.randomUUID();
    this.#grants.set(grantId, grant);
    return { bytes, displayName: grant.displayName, grantId };
  }

  public async requestSaveAs(
    suggestedProjectName: string,
    bytes: Uint8Array<ArrayBuffer>,
  ): Promise<SavedProjectFile> {
    const picker = (window as PickerWindow).showSaveFilePicker;
    if (picker === undefined) {
      throw new FileAccessError(
        "ACCESS_UNAVAILABLE",
        "This browser cannot grant project-file access.",
      );
    }
    assertProjectSize(bytes.byteLength);

    let handle: ProjectFileHandle;
    try {
      handle = await picker({
        excludeAcceptAllOption: true,
        suggestedName: normalizeSuggestedName(suggestedProjectName),
        types: [projectPickerType],
      });
    } catch (error) {
      throw normalizePickerError(error, "save");
    }
    const grant = inspectGrantedHandle(handle, "save");
    const grantId = crypto.randomUUID();
    this.#grants.set(grantId, grant);
    try {
      return await this.writeAndVerify(grantId, bytes);
    } catch (error) {
      this.#grants.delete(grantId);
      throw error;
    }
  }

  public async save(
    grantId: string,
    bytes: Uint8Array<ArrayBuffer>,
  ): Promise<SavedProjectFile> {
    assertProjectSize(bytes.byteLength);
    return this.writeAndVerify(grantId, bytes);
  }

  public revoke(grantId: string): void {
    this.#grants.delete(grantId);
  }

  private async writeAndVerify(
    grantId: string,
    bytes: Uint8Array<ArrayBuffer>,
  ): Promise<SavedProjectFile> {
    const grant = this.#grants.get(grantId);
    if (grant === undefined) {
      throw new FileAccessError("UNKNOWN_GRANT", "The project file grant is no longer active.");
    }

    let writable: Awaited<ReturnType<ProjectFileHandle["createWritable"]>> | undefined;
    try {
      writable = await grant.handle.createWritable();
      await writable.write(bytes);
      await writable.close();
      writable = undefined;

      const reopened = await grant.handle.getFile();
      assertProjectSize(reopened.size);
      const reopenedBytes = new Uint8Array(await reopened.arrayBuffer());
      if (reopenedBytes.byteLength !== reopened.size || !equalBytes(bytes, reopenedBytes)) {
        throw new Error("The reopened bytes differ from the save payload.");
      }
      return {
        displayName: grant.displayName,
        grantId,
        verifiedBytes: reopenedBytes.byteLength,
      };
    } catch {
      if (writable !== undefined) {
        await writable.abort().catch(() => undefined);
      }
      throw new FileAccessError(
        "WRITE_FAILED",
        "The project was not saved because write-and-reopen verification failed.",
      );
    }
  }
}

const normalizePickerError = (
  error: unknown,
  operation: "open" | "save",
): FileAccessError => {
  if (error instanceof DOMException && error.name === "AbortError") {
    return new FileAccessError("ACCESS_CANCELLED", `Project ${operation} was cancelled.`);
  }
  return new FileAccessError(
    operation === "open" ? "READ_FAILED" : "WRITE_FAILED",
    operation === "open"
      ? "The selected project could not be read."
      : "The project file could not be created.",
  );
};

/**
 * Inspect only the two side-effect-free metadata fields exposed by the web
 * File System Access contract before the broker performs selected-byte I/O.
 *
 * This deliberately does not claim that `kind` and `name` attest a fixed,
 * native, non-provider, non-removable backing volume. The browser API exposes
 * no such attestation, so VER-ISO-0004 remains blocked on an approved host
 * architecture even though path-shaped and device-shaped names fail closed.
 */
const inspectGrantedHandle = (
  handle: ProjectFileHandle | undefined,
  operation: "open" | "save",
): ProjectFileGrant => {
  if (handle === undefined || handle === null || typeof handle !== "object") {
    throw fileBoundaryFailure(operation, "No project file was granted.");
  }

  let kind: unknown;
  let name: unknown;
  try {
    kind = handle.kind;
    name = handle.name;
  } catch {
    throw fileBoundaryFailure(operation, "The granted file metadata could not be inspected.");
  }
  if (kind !== "file" || typeof name !== "string") {
    throw fileBoundaryFailure(operation, "The granted object is not a project file.");
  }
  assertProjectName(name);
  return { displayName: name, handle };
};

const readProjectBytes = async (
  handle: ProjectFileHandle,
): Promise<Uint8Array<ArrayBuffer>> => {
  try {
    const file = await handle.getFile();
    assertProjectSize(file.size);
    const bytes = new Uint8Array(await file.arrayBuffer());
    if (bytes.byteLength !== file.size) {
      throw new Error("The selected file changed while it was being read.");
    }
    return bytes;
  } catch (error) {
    if (error instanceof FileAccessError) {
      throw error;
    }
    throw new FileAccessError("READ_FAILED", "The selected project could not be read.");
  }
};

const fileBoundaryFailure = (
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
    name.length > MAX_PROJECT_NAME_CODE_UNITS ||
    name !== name.trim() ||
    name.endsWith(".") ||
    FORBIDDEN_FILE_NAME_CHARACTERS.test(name)
  ) {
    throw new FileAccessError(
      "INVALID_FILE_NAME",
      "Project files must use one bounded file name, not a path, endpoint, device, pipe, or print target.",
    );
  }
  const stem = name.slice(0, -PROJECT_EXTENSION.length);
  if (
    stem.length === 0 ||
    stem !== stem.trim() ||
    stem.endsWith(".") ||
    stem === "." ||
    stem === ".." ||
    isReservedProjectStem(stem)
  ) {
    throw new FileAccessError(
      "INVALID_FILE_NAME",
      "The selected project file name is reserved or empty.",
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
    .trim()
    .replace(/[. ]+$/u, "");
  const admissible = normalized.length > 0 && !isReservedProjectStem(normalized)
    ? normalized
    : "Untitled project";
  const maximumStemLength = MAX_PROJECT_NAME_CODE_UNITS - PROJECT_EXTENSION.length;
  const bounded = admissible
    .slice(0, maximumStemLength)
    .replace(/[\ud800-\udbff]$/u, "")
    .replace(/[. ]+$/u, "");
  return `${bounded || "Untitled project"}${PROJECT_EXTENSION}`;
};

const isReservedProjectStem = (stem: string): boolean => {
  const candidate = stem.split(".", 1)[0]?.replace(/[. ]+$/u, "") ?? "";
  return RESERVED_WINDOWS_FILE_STEM.test(candidate);
};

const equalBytes = (left: Uint8Array, right: Uint8Array): boolean => {
  if (left.byteLength !== right.byteLength) {
    return false;
  }
  for (let index = 0; index < left.byteLength; index += 1) {
    if (left[index] !== right[index]) {
      return false;
    }
  }
  return true;
};
