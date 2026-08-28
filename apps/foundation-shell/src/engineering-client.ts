import EngineeringWorker from "./foundation.worker?worker&inline";
import type {
  WorkbenchOperation,
  WorkbenchOperationResult,
  WorkbenchSnapshot,
} from "./workbench-types";

const RESPONSE_TIMEOUT_MILLISECONDS = 20_000;

type EngineeringRequest =
  | Readonly<{ kind: "engineering.initialize"; requestId: string }>
  | Readonly<{
      displayName: string;
      documentId: string;
      kind: "engineering.project.create";
      projectRootId: string;
      requestId: string;
    }>
  | Readonly<{
      bytes: ArrayBuffer;
      fileGrantId: string;
      kind: "engineering.project.open";
      requestId: string;
    }>
  | Readonly<{
      kind: "engineering.project.command";
      operation: WorkbenchOperation;
      requestId: string;
    }>
  | Readonly<{
      kind: "engineering.persistence.prepare";
      mode: "save" | "save-as";
      newDocumentId: string | null;
      requestId: string;
    }>
  | Readonly<{
      fileGrantId: string;
      kind: "engineering.persistence.commit";
      pendingSaveId: string;
      requestId: string;
      verifiedBytes: number;
    }>
  | Readonly<{
      kind: "engineering.persistence.abort";
      pendingSaveId: string;
      requestId: string;
    }>;

type PreparedSave = Readonly<{
  bytes: ArrayBuffer;
  packageHash: string;
  pendingSaveId: string;
  suggestedName: string;
}>;

type ResponseValue =
  | Readonly<{ coreVersion: string; status: "HEALTHY" }>
  | PreparedSave
  | WorkbenchOperationResult
  | WorkbenchSnapshot
  | null;

type EngineeringResponse = Readonly<{
  error?: Readonly<{ code: string; message: string }>;
  inReplyTo: string;
  kind: "engineering.response";
  ok: boolean;
  value?: ResponseValue;
}>;

type PendingRequest = Readonly<{
  reject: (error: Error) => void;
  resolve: (value: ResponseValue) => void;
  timeout: ReturnType<typeof setTimeout>;
}>;

export class EngineeringClientError extends Error {
  public readonly code: string;

  public constructor(code: string, message: string) {
    super(message);
    this.name = "EngineeringClientError";
    this.code = code;
  }
}

export class EngineeringClient {
  readonly #pending = new Map<string, PendingRequest>();
  readonly #worker: Worker;
  #disposed = false;

  public constructor() {
    this.#worker = new EngineeringWorker({ name: "plc-engineering-core" });
    this.#worker.addEventListener("message", this.onMessage);
    this.#worker.addEventListener("messageerror", this.onWorkerFailure);
    this.#worker.addEventListener("error", this.onWorkerFailure);
  }

  public async initialize(): Promise<Readonly<{ coreVersion: string; status: "HEALTHY" }>> {
    return this.request(
      { kind: "engineering.initialize", requestId: crypto.randomUUID() },
      isHealthValue,
    );
  }

  public async createProject(displayName: string): Promise<WorkbenchSnapshot> {
    return this.request(
      {
        displayName,
        documentId: crypto.randomUUID(),
        kind: "engineering.project.create",
        projectRootId: crypto.randomUUID(),
        requestId: crypto.randomUUID(),
      },
      isWorkbenchSnapshot,
    );
  }

  public async openProject(
    bytes: Uint8Array<ArrayBuffer>,
    fileGrantId: string,
  ): Promise<WorkbenchSnapshot> {
    const transferable = bytes.slice().buffer;
    return this.request(
      {
        bytes: transferable,
        fileGrantId,
        kind: "engineering.project.open",
        requestId: crypto.randomUUID(),
      },
      isWorkbenchSnapshot,
      [transferable],
    );
  }

  public async execute(operation: WorkbenchOperation): Promise<WorkbenchOperationResult> {
    return this.request(
      {
        kind: "engineering.project.command",
        operation,
        requestId: crypto.randomUUID(),
      },
      isOperationResult,
    );
  }

  public async prepareSave(mode: "save" | "save-as"): Promise<PreparedSave> {
    return this.request(
      {
        kind: "engineering.persistence.prepare",
        mode,
        newDocumentId: mode === "save-as" ? crypto.randomUUID() : null,
        requestId: crypto.randomUUID(),
      },
      isPreparedSave,
    );
  }

  public async commitSave(
    pendingSaveId: string,
    fileGrantId: string,
    verifiedBytes: number,
  ): Promise<WorkbenchSnapshot> {
    return this.request(
      {
        fileGrantId,
        kind: "engineering.persistence.commit",
        pendingSaveId,
        requestId: crypto.randomUUID(),
        verifiedBytes,
      },
      isWorkbenchSnapshot,
    );
  }

  public async abortSave(pendingSaveId: string): Promise<void> {
    await this.request(
      {
        kind: "engineering.persistence.abort",
        pendingSaveId,
        requestId: crypto.randomUUID(),
      },
      (value): value is null => value === null,
    );
  }

  public dispose(): void {
    if (this.#disposed) {
      return;
    }
    this.#disposed = true;
    this.#worker.terminate();
    for (const pending of this.#pending.values()) {
      clearTimeout(pending.timeout);
      pending.reject(new EngineeringClientError("CLIENT_DISPOSED", "The engineering core closed."));
    }
    this.#pending.clear();
  }

  private readonly onMessage = (event: MessageEvent<unknown>): void => {
    if (!isEngineeringResponse(event.data)) {
      this.rejectAll("INVALID_RESPONSE", "The engineering core returned an invalid response.");
      return;
    }
    const pending = this.#pending.get(event.data.inReplyTo);
    if (pending === undefined) {
      return;
    }
    clearTimeout(pending.timeout);
    this.#pending.delete(event.data.inReplyTo);
    if (!event.data.ok) {
      pending.reject(
        new EngineeringClientError(
          event.data.error?.code ?? "COMMAND_FAILED",
          event.data.error?.message ?? "The engineering command did not complete.",
        ),
      );
      return;
    }
    pending.resolve(event.data.value ?? null);
  };

  private readonly onWorkerFailure = (): void => {
    this.rejectAll("WORKER_FAILURE", "The isolated engineering core stopped unexpectedly.");
  };

  private rejectAll(code: string, message: string): void {
    for (const pending of this.#pending.values()) {
      clearTimeout(pending.timeout);
      pending.reject(new EngineeringClientError(code, message));
    }
    this.#pending.clear();
  }

  private async request<T extends ResponseValue>(
    request: EngineeringRequest,
    validate: (value: unknown) => value is T,
    transfer: Transferable[] = [],
  ): Promise<T> {
    if (this.#disposed) {
      throw new EngineeringClientError("CLIENT_DISPOSED", "The engineering core is closed.");
    }
    const value = await new Promise<ResponseValue>((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.#pending.delete(request.requestId);
        reject(
          new EngineeringClientError(
            "RESPONSE_TIMEOUT",
            "The engineering command exceeded its deterministic response budget.",
          ),
        );
      }, RESPONSE_TIMEOUT_MILLISECONDS);
      this.#pending.set(request.requestId, { reject, resolve, timeout });
      this.#worker.postMessage(request, transfer);
    });
    if (!validate(value)) {
      throw new EngineeringClientError(
        "INVALID_RESPONSE",
        "The engineering core returned a response with the wrong shape.",
      );
    }
    return value;
  }
}

const isRecord = (value: unknown): value is Readonly<Record<string, unknown>> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const isEngineeringResponse = (value: unknown): value is EngineeringResponse =>
  isRecord(value) &&
  value.kind === "engineering.response" &&
  typeof value.inReplyTo === "string" &&
  typeof value.ok === "boolean" &&
  (value.error === undefined ||
    (isRecord(value.error) &&
      typeof value.error.code === "string" &&
      typeof value.error.message === "string"));

const isHealthValue = (
  value: unknown,
): value is Readonly<{ coreVersion: string; status: "HEALTHY" }> =>
  isRecord(value) && value.status === "HEALTHY" && typeof value.coreVersion === "string";

const isPreparedSave = (value: unknown): value is PreparedSave =>
  isRecord(value) &&
  value.bytes instanceof ArrayBuffer &&
  typeof value.packageHash === "string" &&
  typeof value.pendingSaveId === "string" &&
  typeof value.suggestedName === "string";

const isWorkbenchSnapshot = (value: unknown): value is WorkbenchSnapshot =>
  isRecord(value) &&
  typeof value.documentId === "string" &&
  typeof value.projectRootId === "string" &&
  typeof value.projectName === "string" &&
  typeof value.projectHash === "string" &&
  isRecord(value.objects) &&
  Array.isArray(value.diagnostics) &&
  isRecord(value.undo);

const isOperationResult = (value: unknown): value is WorkbenchOperationResult =>
  isRecord(value) &&
  (value.outcome === "committed" || value.outcome === "rejected" || value.outcome === "blocked") &&
  Array.isArray(value.diagnostics) &&
  isWorkbenchSnapshot(value.snapshot);
