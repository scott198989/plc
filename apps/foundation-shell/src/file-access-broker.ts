const PROJECT_EXTENSION = ".vlabproj";
const PROJECT_MIME_TYPE = "application/vnd.govs.virtual-plc-project";
const MAX_PROJECT_BYTES = 32 * 1024 * 1024;

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
  readonly #grants = new Map<string, ProjectFileHandle>();

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
        "This browser cannot grant local project-file access.",
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
    const handle = handles[0];
    if (handle === undefined || handle.kind !== "file") {
      throw new FileAccessError("READ_FAILED", "No project file was granted.");
    }
    assertProjectName(handle.name);

    let file: File;
    try {
      file = await handle.getFile();
    } catch {
      throw new FileAccessError("READ_FAILED", "The selected project could not be read.");
    }
    assertProjectSize(file.size);
    const bytes = new Uint8Array(await file.arrayBuffer());
    const grantId = crypto.randomUUID();
    this.#grants.set(grantId, handle);
    return { bytes, displayName: handle.name, grantId };
  }

  public async requestSaveAs(
    suggestedProjectName: string,
    bytes: Uint8Array<ArrayBuffer>,
  ): Promise<SavedProjectFile> {
    const picker = (window as PickerWindow).showSaveFilePicker;
    if (picker === undefined) {
      throw new FileAccessError(
        "ACCESS_UNAVAILABLE",
        "This browser cannot grant local project-file access.",
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
    assertProjectName(handle.name);
    const grantId = crypto.randomUUID();
    this.#grants.set(grantId, handle);
    return this.writeAndVerify(grantId, bytes);
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
    const handle = this.#grants.get(grantId);
    if (handle === undefined) {
      throw new FileAccessError("UNKNOWN_GRANT", "The project file grant is no longer active.");
    }

    let writable: Awaited<ReturnType<ProjectFileHandle["createWritable"]>> | undefined;
    try {
      writable = await handle.createWritable();
      await writable.write(bytes);
      await writable.close();
      writable = undefined;

      const reopened = await handle.getFile();
      assertProjectSize(reopened.size);
      const reopenedBytes = new Uint8Array(await reopened.arrayBuffer());
      if (!equalBytes(bytes, reopenedBytes)) {
        throw new Error("The reopened bytes differ from the save payload.");
      }
      return {
        displayName: handle.name,
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

const assertProjectName = (name: string): void => {
  if (!name.toLocaleLowerCase("en-US").endsWith(PROJECT_EXTENSION)) {
    throw new FileAccessError(
      "INVALID_EXTENSION",
      `Project files must use the ${PROJECT_EXTENSION} extension.`,
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
  const safeName = withoutExtension.replaceAll(/[<>:"/\\|?*\u0000-\u001f]/gu, " ").trim();
  return `${safeName || "Untitled project"}${PROJECT_EXTENSION}`;
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
