import {
  PLC_CONTRACT_SCHEMA_VERSION,
  PLC_MESSAGE_KIND,
  validateDomainReceipt,
  validatePhase2PlcMessage,
} from "@govs/plc-contract";
import type {
  CommandContext,
  Diagnostic,
  DomainCommand,
  DomainCommandMessage,
  DomainResultMessage,
  PersistenceReceipt,
  ProjectObjectKind,
  ProjectReceipt,
  ProjectSnapshotReceipt,
} from "@govs/plc-contract";

import { encodeCanonicalJson } from "./canonical-json";
import { projectReceiptToWorkbench } from "./project-receipt-projection";
import { WasmKernel, WasmKernelError } from "./wasm-kernel";
import type {
  ProjectPayload,
  ProjectPayloadValue,
  ProjectStorageKind,
  WorkbenchOperation,
  WorkbenchOperationResult,
  WorkbenchSnapshot,
} from "./workbench-types";

const PROFILE_MANIFEST_HASH =
  "9febe00e579c161920610be4d2079621b6255217a623f29ee0f656fcd992ed9a";
const PROFILE_ID = "EDU-21 Core";
const PROFILE_VERSION = "1.0.0";
const MAX_PROJECT_BYTES = 32 * 1024 * 1024;
const MAX_PROJECT_OBJECTS = 16_384;
const ZERO_HASH = "0".repeat(64);
const UUID_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/u;
const DECIMAL_UINT64_PATTERN = /^(?:0|[1-9][0-9]*)$/u;
const HASH_PATTERN = /^[A-Fa-f0-9]{64}$/u;
const SIGNED_DECIMAL_PATTERN = /^(?:0|-[1-9][0-9]*|[1-9][0-9]*)$/u;
const PROJECT_STORAGE_KINDS = new Set<ProjectStorageKind>([
  "folder",
  "controller",
  "rack",
  "module",
  "network",
  "symbol-table",
  "tag",
  "type-definition",
  "program-block",
  "data-block",
  "build-record",
  "snapshot-reference",
  "generic",
]);

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

type EngineeringResponseValue =
  | Readonly<{ coreVersion: string; status: "HEALTHY" }>
  | PreparedSave
  | WorkbenchOperationResult
  | WorkbenchSnapshot
  | null;

export type EngineeringResponse = Readonly<{
  error?: Readonly<{ code: string; message: string }>;
  inReplyTo: string;
  kind: "engineering.response";
  ok: boolean;
  value?: EngineeringResponseValue;
}>;

type PendingSaveState = Readonly<{
  digest: Uint8Array<ArrayBuffer>;
  id: string;
  mode: "save" | "save-as";
  newDocumentId: string | null;
  packageHash: string;
}>;

type RawKernelObject = Readonly<{
  creationOrdinal: string;
  displayName: string;
  id: string;
  kind: string;
  lifecycle: "active" | "tombstoned";
  objectRevision: string;
  parentId: string | null;
  presentationPayload: ProjectPayload;
  payloadSchema: string;
  semanticPayload: ProjectPayload;
  semanticRevision: string;
}>;

type RawKernelReference = Readonly<{
  expectedTargetKind: string;
  kind: string;
  resolution: "resolved" | "unresolved";
  sourceId: string;
  sourceLocation: string;
  targetId: string;
}>;

type RawKernelProject = Readonly<{
  dependencies: readonly Readonly<{
    reason: string;
    sourceId: string;
    targetId: string;
  }>[];
  documentId: string;
  documentRevision: string;
  nextCreationOrdinal: string;
  objects: readonly RawKernelObject[];
  references: readonly RawKernelReference[];
  rootId: string;
  semanticRevision: string;
}>;

type RawKernelStatus = Readonly<{
  canRedo: boolean;
  canUndo: boolean;
  documentDirty: boolean;
  documentHash: string;
  nextUndoToken: string | null;
  savedDocumentHash: string | null;
  savedDocumentRevision: string | null;
  savedSemanticFingerprint: string | null;
  semanticDirty: boolean;
  semanticFingerprint: string;
}>;

type RawKernelQuery = Readonly<{
  project: RawKernelProject;
  status: RawKernelStatus;
}>;

type RawKernelDiagnostic = Readonly<{
  code: string;
  message: string;
  objectIds: readonly string[];
}>;

type RawKernelCommandResult = Readonly<{
  affectedObjectIds: readonly string[];
  afterProjectHash: string | null;
  beforeProjectHash: string;
  diagnostics: readonly RawKernelDiagnostic[];
  outcome: "blocked" | "committed" | "rejected";
  transactionId: string;
  undoToken: string | null;
}>;

type RawSystemDiagnostic = Readonly<{
  blocking: boolean;
  code: string;
  message: string;
  phase: string;
  primaryObjectId: string;
  relatedObjectIds: readonly string[];
}>;

type RawSystemQuery = Readonly<{
  allocationChangeCount: number;
  artifactFingerprint: string | null;
  canBuild: boolean;
  channelBindingCount: number;
  diagnostics: readonly RawSystemDiagnostic[];
  profile: Readonly<{
    id: string;
    manifestHash: string;
    version: string;
  }>;
  sourceDocumentHash: string;
  sourceSemanticFingerprint: string;
}>;

class EngineeringWorkerError extends Error {
  public readonly code: string;

  public constructor(code: string, message: string) {
    super(message);
    this.name = "EngineeringWorkerError";
    this.code = code;
  }
}

class EngineeringWorkerEngine {
  readonly #kernelPromise: Promise<WasmKernel>;
  #currentDiagnostics: readonly Diagnostic[] = [];
  #eventSequence = 0n;
  #fileGrantId: string | null = null;
  #issuedSequence = 0n;
  #lastQuery: RawKernelQuery | null = null;
  #pendingSave: PendingSaveState | null = null;
  #redoTokens: string[] = [];

  public constructor() {
    this.#kernelPromise = WasmKernel.load();
  }

  public async execute(request: EngineeringRequest): Promise<EngineeringResponseValue> {
    switch (request.kind) {
      case "engineering.initialize": {
        const health = (await this.#kernelPromise).health();
        return { coreVersion: health.buildIdentity, status: "HEALTHY" };
      }
      case "engineering.project.create":
        return this.createProject(request);
      case "engineering.project.open":
        return this.openProject(request);
      case "engineering.project.command":
        return this.executeProjectOperation(request.requestId, request.operation);
      case "engineering.persistence.prepare":
        return this.prepareSave(request.mode, request.newDocumentId);
      case "engineering.persistence.commit":
        return this.commitSave(request);
      case "engineering.persistence.abort":
        await this.abortSave(request.pendingSaveId);
        return null;
    }
  }

  private async createProject(
    request: Extract<EngineeringRequest, { kind: "engineering.project.create" }>,
  ): Promise<WorkbenchSnapshot> {
    const transactionId = await deriveUuid(`${request.requestId}:create`);
    const message = this.commandMessage(
      request.requestId,
      transactionId,
      "0",
      [],
      {
        commandKind: "project.create",
        displayName: request.displayName,
        documentId: request.documentId,
        projectRootId: request.projectRootId,
      },
    );
    validateCommandMessage(message);

    const kernel = await this.#kernelPromise;
    const query = parseKernelQuery(
      kernel.create(
        encodeCanonicalJson({
          displayName: request.displayName,
          documentId: request.documentId,
          profile: {
            id: PROFILE_ID,
            manifestHash: PROFILE_MANIFEST_HASH,
            version: PROFILE_VERSION,
          },
          rootId: request.projectRootId,
          schemaVersion: 1,
        }),
      ),
    );
    this.resetSession(query, null);
    await this.validateSyntheticCreateResult(message, query);
    return this.snapshot(query, await deriveUuid(`${request.requestId}:snapshot`));
  }

  private async openProject(
    request: Extract<EngineeringRequest, { kind: "engineering.project.open" }>,
  ): Promise<WorkbenchSnapshot> {
    const transactionId = await deriveUuid(`${request.requestId}:open`);
    const message = this.commandMessage(
      request.requestId,
      transactionId,
      "0",
      [],
      { commandKind: "persistence.open", sourceGrantId: request.fileGrantId },
    );
    validateCommandMessage(message);

    const query = parseKernelQuery(
      (await this.#kernelPromise).open(new Uint8Array(request.bytes)),
    );
    this.resetSession(query, request.fileGrantId);
    const receipt: PersistenceReceipt = {
      action: "open",
      documentId: query.project.documentId,
      documentRevision: query.project.documentRevision,
      domain: "persistence",
      packageHash: (await sha256(new Uint8Array(request.bytes))).hex,
      projectRootId: query.project.rootId,
      recoveryStatus: "not-applicable",
      schemaVersion: "1",
    };
    validateDomainReceipt(receipt);
    return this.snapshot(query, await deriveUuid(`${request.requestId}:snapshot`));
  }

  private async executeProjectOperation(
    requestId: string,
    operation: WorkbenchOperation,
  ): Promise<WorkbenchOperationResult> {
    const before = this.requireQuery();
    const transactionId = await deriveUuid(`${requestId}:transaction`);
    const translation = await this.translateOperation(operation, before, transactionId);
    const message = this.commandMessage(
      requestId,
      transactionId,
      before.project.documentRevision,
      translation.expectedObjectRevisions,
      translation.contractCommand,
    );
    validateCommandMessage(message);

    const kernel = await this.#kernelPromise;
    const rawResult = parseKernelCommandResult(kernel.handle(translation.kernelRequest));
    const after = parseKernelQuery(
      kernel.handle(encodeCanonicalJson({ operation: "query-project", schemaVersion: 1 })),
    );
    const diagnostics = await contractDiagnostics(
      rawResult.diagnostics,
      before,
      requestId,
    );
    await this.validateProjectCommandResult(message, before, after, rawResult, diagnostics);

    if (rawResult.outcome === "committed") {
      if (operation.kind === "project.undo") {
        this.#redoTokens.push(translation.historyToken);
      } else if (operation.kind === "project.redo") {
        this.#redoTokens.pop();
      } else {
        this.#redoTokens = [];
      }
    }
    this.#lastQuery = after;
    this.#currentDiagnostics = diagnostics;
    const snapshot = await this.snapshot(after, await deriveUuid(`${requestId}:snapshot`));
    return { diagnostics: snapshot.diagnostics, outcome: rawResult.outcome, snapshot };
  }

  private async prepareSave(
    mode: "save" | "save-as",
    newDocumentId: string | null,
  ): Promise<PreparedSave> {
    const query = this.requireQuery();
    if (this.#pendingSave !== null) {
      throw new EngineeringWorkerError(
        "SAVE_ALREADY_PENDING",
        "A project save is already awaiting durable verification.",
      );
    }
    if (mode === "save-as" && newDocumentId === null) {
      throw new EngineeringWorkerError(
        "INVALID_SAVE_AS",
        "Save As requires a new document identity.",
      );
    }
    const bytes = (await this.#kernelPromise).prepareSave(mode, newDocumentId);
    const digest = await sha256(bytes);
    const pendingSaveId = crypto.randomUUID();
    this.#pendingSave = {
      digest: digest.bytes,
      id: pendingSaveId,
      mode,
      newDocumentId,
      packageHash: digest.hex,
    };
    return {
      bytes: bytes.slice().buffer,
      packageHash: digest.hex,
      pendingSaveId,
      suggestedName: `${query.project.objects.find((object) => object.id === query.project.rootId)?.displayName ?? "Untitled project"}.vlabproj`,
    };
  }

  private async commitSave(
    request: Extract<EngineeringRequest, { kind: "engineering.persistence.commit" }>,
  ): Promise<WorkbenchSnapshot> {
    const pending = this.#pendingSave;
    const before = this.requireActiveQuery();
    if (pending === null || pending.id !== request.pendingSaveId) {
      throw new EngineeringWorkerError(
        "UNKNOWN_PENDING_SAVE",
        "The durable save receipt does not match the prepared project package.",
      );
    }
    const documentId = pending.mode === "save-as"
      ? requireUuid(pending.newDocumentId, "pending Save As document identity")
      : before.project.documentId;
    const transactionId = await deriveUuid(`${request.requestId}:save`);
    const message = this.commandMessage(
      request.requestId,
      transactionId,
      before.project.documentRevision,
      [],
      {
        commandKind: "persistence.save",
        documentId,
        mode: pending.mode,
        targetGrantId: request.fileGrantId,
      },
    );
    validateCommandMessage(message);

    const after = parseKernelQuery(
      (await this.#kernelPromise).commitSave(request.verifiedBytes, pending.digest),
    );
    const receipt: PersistenceReceipt = {
      action: pending.mode,
      documentId: after.project.documentId,
      documentRevision: after.project.documentRevision,
      domain: "persistence",
      packageHash: pending.packageHash,
      projectRootId: after.project.rootId,
      recoveryStatus: "not-applicable",
      schemaVersion: "1",
    };
    validateDomainReceipt(receipt);
    validateSuccessfulPersistenceResult(message, before, after, receipt);

    this.#pendingSave = null;
    this.#lastQuery = after;
    this.#fileGrantId = request.fileGrantId;
    this.#currentDiagnostics = [];
    return this.snapshot(after, await deriveUuid(`${request.requestId}:snapshot`));
  }

  private async abortSave(pendingSaveId: string): Promise<void> {
    if (this.#pendingSave === null || this.#pendingSave.id !== pendingSaveId) {
      throw new EngineeringWorkerError(
        "UNKNOWN_PENDING_SAVE",
        "The prepared project save is no longer active.",
      );
    }
    (await this.#kernelPromise).abortSave();
    this.#pendingSave = null;
  }

  private async translateOperation(
    operation: WorkbenchOperation,
    query: RawKernelQuery,
    transactionId: string,
  ): Promise<Readonly<{
    contractCommand: DomainCommand;
    expectedObjectRevisions: readonly Readonly<{ objectId: string; revision: string }>[];
    historyToken: string;
    kernelRequest: Uint8Array<ArrayBuffer>;
  }>> {
    const objectById = new Map(query.project.objects.map((object) => [object.id, object]));
    const expected = (ids: readonly string[]) => [...new Set(ids)].sort(ordinalCompare).map((objectId) => {
      const object = objectById.get(objectId);
      if (object === undefined) {
        throw new EngineeringWorkerError("UNKNOWN_OBJECT", `Project object ${objectId} does not exist.`);
      }
      return { objectId, revision: object.objectRevision };
    });
    const envelope = (
      command: Readonly<Record<string, unknown>>,
      revisions: readonly Readonly<{ objectId: string; revision: string }>[],
    ): Uint8Array<ArrayBuffer> => encodeCanonicalJson({
      envelope: {
        command,
        commandId: transactionId,
        context: { actorId: "local-workbench", canMutate: true },
        expectedDocumentRevision: query.project.documentRevision,
        expectedObjectRevisions: revisions.map((revision) => [revision.objectId, revision.revision]),
        transactionId,
      },
      operation: "execute",
      schemaVersion: 1,
    });

    switch (operation.kind) {
      case "project.create-object": {
        const revisions = expected([operation.parentId]);
        return {
          contractCommand: {
            commandKind: "project.create-object",
            displayName: operation.displayName,
            objectId: operation.objectId,
            objectKind: operation.objectKind,
            parentId: operation.parentId,
            payloadSchema: operation.payloadSchema,
          },
          expectedObjectRevisions: revisions,
          historyToken: "",
          kernelRequest: envelope(
            {
              displayName: operation.displayName,
              id: operation.objectId,
              kind: "create",
              objectKind: operation.objectKind,
              parentId: operation.parentId,
              payloadSchema: operation.payloadSchema,
              presentationPayload: operation.presentationPayload,
              semanticPayload: operation.semanticPayload,
            },
            revisions,
          ),
        };
      }
      case "project.rename-object": {
        const revisions = expected([operation.objectId]);
        return {
          contractCommand: {
            commandKind: "project.rename-object",
            displayName: operation.displayName,
            objectId: operation.objectId,
          },
          expectedObjectRevisions: revisions,
          historyToken: "",
          kernelRequest: envelope(
            { displayName: operation.displayName, kind: "rename", objectId: operation.objectId },
            revisions,
          ),
        };
      }
      case "project.set-semantic-field":
      case "project.set-presentation-field": {
        const revisions = expected([operation.objectId]);
        return {
          contractCommand: {
            commandKind: operation.kind,
            key: operation.key,
            objectId: operation.objectId,
            value: operation.value,
          },
          expectedObjectRevisions: revisions,
          historyToken: "",
          kernelRequest: envelope(
            {
              key: operation.key,
              kind: operation.kind === "project.set-semantic-field"
                ? "set-semantic-field"
                : "set-presentation-field",
              objectId: operation.objectId,
              value: operation.value,
            },
            revisions,
          ),
        };
      }
      case "project.delete-object": {
        const revisions = expected([operation.objectId]);
        return {
          contractCommand: {
            commandKind: "project.delete-object",
            objectId: operation.objectId,
          },
          expectedObjectRevisions: revisions,
          historyToken: "",
          kernelRequest: envelope({ kind: "delete", objectId: operation.objectId }, revisions),
        };
      }
      case "project.copy-objects": {
        const closure = collectClosure(operation.sourceObjectIds, objectById);
        const revisions = expected([...closure, operation.targetParentId]);
        const idMap = await Promise.all(
          closure.map(async (sourceId) => [
            sourceId,
            await deriveUuid(`${transactionId}:copy:${sourceId}`),
          ] as const),
        );
        return {
          contractCommand: {
            commandKind: "project.copy-objects",
            sourceObjectIds: operation.sourceObjectIds,
            targetParentId: operation.targetParentId,
          },
          expectedObjectRevisions: revisions,
          historyToken: "",
          kernelRequest: envelope(
            {
              destinationParent: operation.targetParentId,
              idMap,
              kind: "copy-closure",
              roots: operation.sourceObjectIds,
            },
            revisions,
          ),
        };
      }
      case "project.undo": {
        const token = query.status.nextUndoToken;
        if (token === null) {
          throw new EngineeringWorkerError("UNDO_UNAVAILABLE", "No committed change is available to undo.");
        }
        return {
          contractCommand: { commandKind: "project.undo", undoToken: token },
          expectedObjectRevisions: [],
          historyToken: token,
          kernelRequest: encodeCanonicalJson({
            operation: "undo",
            schemaVersion: 1,
            transactionId,
            undoToken: token,
          }),
        };
      }
      case "project.redo": {
        const token = this.#redoTokens.at(-1);
        if (token === undefined || !query.status.canRedo) {
          throw new EngineeringWorkerError("REDO_UNAVAILABLE", "No reverted change is available to redo.");
        }
        return {
          contractCommand: { commandKind: "project.redo", undoToken: token },
          expectedObjectRevisions: [],
          historyToken: token,
          kernelRequest: encodeCanonicalJson({
            operation: "redo",
            schemaVersion: 1,
            transactionId,
          }),
        };
      }
    }
  }

  private commandMessage(
    requestId: string,
    transactionId: string,
    expectedProjectRevision: string,
    expectedObjectRevisions: readonly Readonly<{ objectId: string; revision: string }>[],
    command: DomainCommand,
  ): DomainCommandMessage {
    this.#issuedSequence += 1n;
    const context: CommandContext = {
      commandId: requestId,
      expectedObjectRevisions,
      expectedProjectRevision,
      idempotencyKey: requestId.replaceAll("-", ""),
      issuedSequence: this.#issuedSequence.toString(),
      transactionId,
    };
    return {
      command,
      context,
      kind: PLC_MESSAGE_KIND.command,
      requestId,
      schemaVersion: PLC_CONTRACT_SCHEMA_VERSION,
    };
  }

  private async validateSyntheticCreateResult(
    message: DomainCommandMessage,
    query: RawKernelQuery,
  ): Promise<void> {
    const eventId = await deriveUuid(`${message.context.transactionId}:event`);
    this.#eventSequence += 1n;
    const receipt = projectMutationReceipt(query, [query.project.rootId]);
    const result: DomainResultMessage = {
      inReplyTo: message.requestId,
      kind: PLC_MESSAGE_KIND.result,
      result: {
        affectedObjectIds: [query.project.rootId],
        afterProjectHash: upperHash(query.status.documentHash),
        beforeProjectHash: ZERO_HASH,
        commandId: message.context.commandId,
        commandKind: "project.create",
        diagnostics: [],
        events: [{
          affectedObjectIds: [query.project.rootId],
          documentRevision: query.project.documentRevision,
          eventId,
          eventKind: "project.changed",
          eventSequence: this.#eventSequence.toString(),
          projectHash: upperHash(query.status.documentHash),
          semanticRevision: query.project.semanticRevision,
          transactionId: message.context.transactionId,
        }],
        idempotencyKey: message.context.idempotencyKey,
        outcome: "committed",
        projectRevisionAfter: query.project.documentRevision,
        projectRevisionBefore: "0",
        receipt,
        resultKind: "command",
        transactionId: message.context.transactionId,
        undoToken: null,
      },
      schemaVersion: PLC_CONTRACT_SCHEMA_VERSION,
    };
    validatePhase2PlcMessage(result);
  }

  private async validateProjectCommandResult(
    message: DomainCommandMessage,
    before: RawKernelQuery,
    after: RawKernelQuery,
    raw: RawKernelCommandResult,
    diagnostics: readonly Diagnostic[],
  ): Promise<void> {
    const committed = raw.outcome === "committed";
    const events = [];
    if (committed) {
      this.#eventSequence += 1n;
      events.push({
        affectedObjectIds: raw.affectedObjectIds,
        documentRevision: after.project.documentRevision,
        eventId: await deriveUuid(`${raw.transactionId}:event`),
        eventKind: "project.changed" as const,
        eventSequence: this.#eventSequence.toString(),
        projectHash: upperHash(requireHash(raw.afterProjectHash, "kernel result afterProjectHash")),
        semanticRevision: after.project.semanticRevision,
        transactionId: raw.transactionId,
      });
    }
    const result: DomainResultMessage = {
      inReplyTo: message.requestId,
      kind: PLC_MESSAGE_KIND.result,
      result: {
        affectedObjectIds: committed ? raw.affectedObjectIds : [],
        afterProjectHash: committed
          ? upperHash(requireHash(raw.afterProjectHash, "kernel result afterProjectHash"))
          : null,
        beforeProjectHash: upperHash(raw.beforeProjectHash),
        commandId: message.context.commandId,
        commandKind: message.command.commandKind,
        diagnostics,
        events,
        idempotencyKey: message.context.idempotencyKey,
        outcome: raw.outcome,
        projectRevisionAfter: committed ? after.project.documentRevision : null,
        projectRevisionBefore: before.project.documentRevision,
        receipt: committed ? projectMutationReceipt(after, raw.affectedObjectIds) : null,
        resultKind: "command",
        transactionId: raw.transactionId,
        undoToken: committed ? raw.undoToken : null,
      },
      schemaVersion: PLC_CONTRACT_SCHEMA_VERSION,
    };
    validatePhase2PlcMessage(result);
  }

  private async snapshot(query: RawKernelQuery, queryId: string): Promise<WorkbenchSnapshot> {
    const system = parseSystemQuery((await this.#kernelPromise).systemQuery(), query);
    const systemDiagnostics = await contractSystemDiagnostics(system.diagnostics, query);
    const diagnostics = [...this.#currentDiagnostics, ...systemDiagnostics];
    const receipt = await projectSnapshotReceipt(query, system);
    const queryMessage = {
      context: { consistency: "current", queryId },
      kind: PLC_MESSAGE_KIND.query,
      query: { projectRootId: query.project.rootId, queryKind: "project.get-summary" },
      requestId: queryId,
      schemaVersion: PLC_CONTRACT_SCHEMA_VERSION,
    } as const;
    validatePhase2PlcMessage(queryMessage);
    const result = {
      inReplyTo: queryId,
      kind: PLC_MESSAGE_KIND.result,
      result: {
        diagnostics,
        outcome: "ok",
        queryId,
        queryKind: "project.get-summary",
        receipt,
        resultKind: "query",
        snapshotHash: upperHash(query.status.documentHash),
      },
      schemaVersion: PLC_CONTRACT_SCHEMA_VERSION,
    } as const;
    validatePhase2PlcMessage(result);
    return projectReceiptToWorkbench(receipt, {
      diagnostics,
      fileGrantId: this.#fileGrantId,
      payloads: Object.fromEntries(query.project.objects.map((object) => [
        object.id,
        {
          payloadSchema: object.payloadSchema,
          presentationPayload: object.presentationPayload,
          semanticPayload: object.semanticPayload,
        },
      ])),
      redoLabel: query.status.canRedo ? "Redo last reverted change" : null,
      undoLabel: query.status.canUndo ? "Undo last committed change" : null,
    });
  }

  private requireQuery(): RawKernelQuery {
    const query = this.requireActiveQuery();
    if (this.#pendingSave !== null) {
      throw new EngineeringWorkerError(
        "SAVE_PENDING",
        "Project commands are paused until the prepared save is committed or cancelled.",
      );
    }
    return query;
  }

  private requireActiveQuery(): RawKernelQuery {
    if (this.#lastQuery === null) {
      throw new EngineeringWorkerError("NO_ACTIVE_PROJECT", "Create or open a project first.");
    }
    return this.#lastQuery;
  }

  private resetSession(query: RawKernelQuery, fileGrantId: string | null): void {
    this.#lastQuery = query;
    this.#fileGrantId = fileGrantId;
    this.#pendingSave = null;
    this.#redoTokens = [];
    this.#currentDiagnostics = [];
  }
}

let engine: EngineeringWorkerEngine | null = null;

export const executeEngineeringRequest = async (input: unknown): Promise<EngineeringResponse> => {
  let requestId = "00000000-0000-4000-8000-000000000000";
  try {
    const request = parseEngineeringRequest(input);
    requestId = request.requestId;
    engine ??= new EngineeringWorkerEngine();
    const value = await engine.execute(request);
    return { inReplyTo: requestId, kind: "engineering.response", ok: true, value };
  } catch (error) {
    const normalized = normalizeError(error);
    return {
      error: normalized,
      inReplyTo: requestId,
      kind: "engineering.response",
      ok: false,
    };
  }
};

const validateCommandMessage = (message: DomainCommandMessage): void => {
  const validated = validatePhase2PlcMessage(message);
  if (validated.kind !== PLC_MESSAGE_KIND.command) {
    throw new EngineeringWorkerError("CONTRACT_FAILURE", "The PLC command contract was not preserved.");
  }
};

const validateSuccessfulPersistenceResult = (
  message: DomainCommandMessage,
  before: RawKernelQuery,
  after: RawKernelQuery,
  receipt: PersistenceReceipt,
): void => {
  const result: DomainResultMessage = {
    inReplyTo: message.requestId,
    kind: PLC_MESSAGE_KIND.result,
    result: {
      affectedObjectIds: [],
      afterProjectHash: upperHash(after.status.documentHash),
      beforeProjectHash: upperHash(before.status.documentHash),
      commandId: message.context.commandId,
      commandKind: "persistence.save",
      diagnostics: [],
      events: [],
      idempotencyKey: message.context.idempotencyKey,
      outcome: "committed",
      projectRevisionAfter: after.project.documentRevision,
      projectRevisionBefore: before.project.documentRevision,
      receipt,
      resultKind: "command",
      transactionId: message.context.transactionId,
      undoToken: null,
    },
    schemaVersion: PLC_CONTRACT_SCHEMA_VERSION,
  };
  validatePhase2PlcMessage(result);
};

const projectMutationReceipt = (
  query: RawKernelQuery,
  affectedObjectIds: readonly string[],
): ProjectReceipt => ({
  affectedObjectIds,
  documentId: query.project.documentId,
  documentRevision: query.project.documentRevision,
  domain: "project",
  projectHash: upperHash(query.status.documentHash),
  projectRootId: query.project.rootId,
  semanticRevision: query.project.semanticRevision,
});

const projectSnapshotReceipt = async (
  query: RawKernelQuery,
  system: RawSystemQuery,
): Promise<ProjectSnapshotReceipt> => {
  const objectById = new Map(query.project.objects.map((object) => [object.id, object]));
  const childIds = new Map<string, string[]>();
  for (const object of query.project.objects) {
    if (object.parentId !== null && object.lifecycle === "active") {
      const children = childIds.get(object.parentId) ?? [];
      children.push(object.id);
      childIds.set(object.parentId, children);
    }
  }
  for (const children of childIds.values()) {
    children.sort((left, right) => {
      const leftObject = objectById.get(left);
      const rightObject = objectById.get(right);
      if (leftObject === undefined || rightObject === undefined) {
        return ordinalCompare(left, right);
      }
      const ordinalDifference = BigInt(leftObject.creationOrdinal) - BigInt(rightObject.creationOrdinal);
      return ordinalDifference === 0n ? ordinalCompare(left, right) : ordinalDifference < 0n ? -1 : 1;
    });
  }

  const referencesBySource = new Map<string, RawKernelReference[]>();
  for (const reference of query.project.references) {
    const references = referencesBySource.get(reference.sourceId) ?? [];
    references.push(reference);
    referencesBySource.set(reference.sourceId, references);
  }

  const objects = await Promise.all(query.project.objects.map(async (object) => ({
    creationOrdinal: object.creationOrdinal,
    displayName: object.displayName,
    id: object.id,
    kind: mapProjectKind(object),
    lifecycle: object.lifecycle,
    objectRevision: object.objectRevision,
    orderedChildIds: childIds.get(object.id) ?? [],
    parentId: object.parentId,
    references: await Promise.all((referencesBySource.get(object.id) ?? []).map(async (reference) => ({
      expectedTargetKind: mapRawKind(reference.expectedTargetKind, objectById.get(reference.targetId)),
      referenceId: await deriveUuid([
        reference.sourceId,
        reference.sourceLocation,
        reference.targetId,
        reference.kind,
      ].join(":")),
      resolution: reference.resolution === "resolved"
        ? "resolved" as const
        : objectById.get(reference.targetId)?.lifecycle === "tombstoned"
          ? "tombstoned" as const
          : "unresolved" as const,
      sourceAnchor: {
        anchorKind: "project" as const,
        ownerObjectId: reference.sourceId,
        propertyPath: sourceLocationPath(reference.sourceLocation),
        sourceRevisionHash: upperHash(query.status.semanticFingerprint),
      },
      targetId: reference.targetId,
    }))),
    semanticRevision: object.semanticRevision,
  })));
  const receipt: ProjectSnapshotReceipt = {
    dirtyBuildState: {
      controllerStates: query.project.objects
        .filter((object) => object.kind === "controller")
        .map((object) => ({
          controllerId: object.id,
          hardware: system.canBuild ? "current" as const : "blocked" as const,
          loadedArtifactFingerprint: null,
          software: "not-built" as const,
        })),
      currentDocumentHash: upperHash(query.status.documentHash),
      currentSemanticFingerprint: upperHash(query.status.semanticFingerprint),
      documentDirty: query.status.documentDirty,
      savedDocumentHash: nullableUpperHash(query.status.savedDocumentHash),
      savedDocumentRevision: query.status.savedDocumentRevision,
      savedSemanticFingerprint: nullableUpperHash(query.status.savedSemanticFingerprint),
      semanticDirty: query.status.semanticDirty,
    },
    documentId: query.project.documentId,
    documentRevision: query.project.documentRevision,
    domain: "project-snapshot",
    objects,
    projectRootId: query.project.rootId,
    scope: "summary",
    semanticRevision: query.project.semanticRevision,
  };
  const validated = validateDomainReceipt(receipt);
  if (validated.domain !== "project-snapshot") {
    throw new EngineeringWorkerError("CONTRACT_FAILURE", "The project snapshot contract was not preserved.");
  }
  return receipt;
};

const contractDiagnostics = async (
  diagnostics: readonly RawKernelDiagnostic[],
  query: RawKernelQuery,
  seed: string,
): Promise<readonly Diagnostic[]> => Promise.all(diagnostics.map(async (diagnostic, index) => ({
  blocking: true,
  cause: truncateCharacters(diagnostic.message, 2_048),
  code: "EDU-PRJ-1000",
  diagnosticId: await deriveUuid(`${seed}:diagnostic:${index}:${diagnostic.code}`),
  parameters: [{ kind: "text" as const, name: "kernelCode", value: truncateCharacters(diagnostic.code, 512) }],
  phase: "project",
  primaryAnchor: {
    anchorKind: "project" as const,
    ownerObjectId: diagnostic.objectIds[0] ?? query.project.rootId,
    propertyPath: [],
    sourceRevisionHash: upperHash(query.status.documentHash),
  },
  recoveryHint: "Review the current project state, resolve the reported condition, and retry.",
  relatedAnchors: diagnostic.objectIds.slice(1).map((objectId) => ({
    anchorKind: "project" as const,
    ownerObjectId: objectId,
    propertyPath: [],
    sourceRevisionHash: upperHash(query.status.documentHash),
  })),
  severity: "Error" as const,
})));

const contractSystemDiagnostics = async (
  diagnostics: readonly RawSystemDiagnostic[],
  query: RawKernelQuery,
): Promise<readonly Diagnostic[]> => Promise.all(diagnostics.map(async (diagnostic) => ({
  blocking: diagnostic.blocking,
  cause: truncateCharacters(diagnostic.message, 2_048),
  code: diagnostic.code,
  diagnosticId: await deriveUuid([
    query.status.semanticFingerprint,
    diagnostic.code,
    diagnostic.phase,
    diagnostic.primaryObjectId,
    diagnostic.message,
    ...diagnostic.relatedObjectIds,
  ].join(":")),
  parameters: [],
  phase: diagnostic.phase,
  primaryAnchor: {
    anchorKind: "project" as const,
    ownerObjectId: diagnostic.primaryObjectId,
    propertyPath: [],
    sourceRevisionHash: upperHash(query.status.semanticFingerprint),
  },
  recoveryHint: diagnostic.blocking
    ? "Resolve this canonical project condition before building or loading."
    : "Review this canonical engineering warning before loading.",
  relatedAnchors: diagnostic.relatedObjectIds.map((objectId) => ({
    anchorKind: "project" as const,
    ownerObjectId: objectId,
    propertyPath: [],
    sourceRevisionHash: upperHash(query.status.semanticFingerprint),
  })),
  severity: diagnostic.blocking ? "Error" as const : "Warning" as const,
})));

const collectClosure = (
  roots: readonly string[],
  objects: ReadonlyMap<string, RawKernelObject>,
): readonly string[] => {
  const closure = new Set<string>();
  const queue = [...roots];
  for (let index = 0; index < queue.length; index += 1) {
    const id = queue[index];
    if (id === undefined || closure.has(id)) {
      continue;
    }
    const object = objects.get(id);
    if (object === undefined || object.lifecycle !== "active") {
      throw new EngineeringWorkerError("UNKNOWN_OBJECT", `Project object ${id} is not active.`);
    }
    closure.add(id);
    for (const candidate of objects.values()) {
      if (candidate.parentId === id && candidate.lifecycle === "active") {
        queue.push(candidate.id);
      }
    }
  }
  return [...closure].sort(ordinalCompare);
};

const mapProjectKind = (object: RawKernelObject): ProjectObjectKind =>
  mapRawKind(object.kind, object);

const mapRawKind = (
  kind: string,
  object: RawKernelObject | undefined,
): ProjectObjectKind => {
  switch (kind) {
    case "project": return "ProjectRoot";
    case "folder": return "Folder";
    case "controller": return "Controller";
    case "rack": return "Rack";
    case "module": return "Module";
    case "network": return "VirtualNetwork";
    case "symbol-table": return "SymbolTable";
    case "tag": return "Tag";
    case "type-definition": return "NamedType";
    case "build-record": return "BuildRecord";
    case "snapshot-reference": return "SnapshotReference";
    case "program-block": {
      const blockKind = object?.semanticPayload.blockKind;
      if (blockKind === "OB" || blockKind === "FC" || blockKind === "FB") {
        return blockKind;
      }
      break;
    }
    case "data-block":
      return object?.semanticPayload.dbKind === "InstanceDB" ? "InstanceDB" : "GlobalDB";
    case "generic":
      if (object?.payloadSchema === "edu.watch-table/1") {
        return "WatchTable";
      }
      if (object?.payloadSchema === "edu.trace-configuration/1") {
        return "TraceConfiguration";
      }
      break;
  }
  throw new EngineeringWorkerError(
    "UNSUPPORTED_PROJECT_KIND",
    `The canonical project contains unsupported object kind ${JSON.stringify(kind)}.`,
  );
};

const sourceLocationPath = (sourceLocation: string): readonly string[] => {
  const chunks: string[] = ["references"];
  const characters = [...sourceLocation];
  for (let offset = 0; offset < characters.length && chunks.length < 32; offset += 128) {
    chunks.push(characters.slice(offset, offset + 128).join(""));
  }
  return chunks;
};

const parseEngineeringRequest = (input: unknown): EngineeringRequest => {
  const record = requireRecord(input, "engineering request");
  const kind = requireString(record.kind, "engineering request kind", 80);
  const requestId = requireUuid(record.requestId, "engineering requestId");
  switch (kind) {
    case "engineering.initialize":
      requireExactKeys(record, ["kind", "requestId"], "engineering initialize request");
      return { kind, requestId };
    case "engineering.project.create":
      requireExactKeys(
        record,
        ["displayName", "documentId", "kind", "projectRootId", "requestId"],
        "engineering create request",
      );
      return {
        displayName: requireString(record.displayName, "project displayName", 256),
        documentId: requireUuid(record.documentId, "project documentId"),
        kind,
        projectRootId: requireUuid(record.projectRootId, "project rootId"),
        requestId,
      };
    case "engineering.project.open":
      requireExactKeys(
        record,
        ["bytes", "fileGrantId", "kind", "requestId"],
        "engineering open request",
      );
      if (!(record.bytes instanceof ArrayBuffer) || record.bytes.byteLength < 1 || record.bytes.byteLength > MAX_PROJECT_BYTES) {
        throw new EngineeringWorkerError("INVALID_REQUEST", "The project package violates its byte limit.");
      }
      return {
        bytes: record.bytes,
        fileGrantId: requireUuid(record.fileGrantId, "file grant ID"),
        kind,
        requestId,
      };
    case "engineering.project.command":
      requireExactKeys(record, ["kind", "operation", "requestId"], "engineering command request");
      return { kind, operation: parseWorkbenchOperation(record.operation), requestId };
    case "engineering.persistence.prepare": {
      requireExactKeys(
        record,
        ["kind", "mode", "newDocumentId", "requestId"],
        "engineering prepare-save request",
      );
      const mode = record.mode;
      if (mode !== "save" && mode !== "save-as") {
        throw new EngineeringWorkerError("INVALID_REQUEST", "The save mode is invalid.");
      }
      const newDocumentId = record.newDocumentId === null
        ? null
        : requireUuid(record.newDocumentId, "Save As document ID");
      if ((mode === "save" && newDocumentId !== null) || (mode === "save-as" && newDocumentId === null)) {
        throw new EngineeringWorkerError("INVALID_REQUEST", "The save identity does not match its mode.");
      }
      return { kind, mode, newDocumentId, requestId };
    }
    case "engineering.persistence.commit":
      requireExactKeys(
        record,
        ["fileGrantId", "kind", "pendingSaveId", "requestId", "verifiedBytes"],
        "engineering commit-save request",
      );
      return {
        fileGrantId: requireUuid(record.fileGrantId, "file grant ID"),
        kind,
        pendingSaveId: requireUuid(record.pendingSaveId, "pending save ID"),
        requestId,
        verifiedBytes: requireSafeInteger(record.verifiedBytes, "verified project bytes", 1, MAX_PROJECT_BYTES),
      };
    case "engineering.persistence.abort":
      requireExactKeys(
        record,
        ["kind", "pendingSaveId", "requestId"],
        "engineering abort-save request",
      );
      return {
        kind,
        pendingSaveId: requireUuid(record.pendingSaveId, "pending save ID"),
        requestId,
      };
    default:
      throw new EngineeringWorkerError("INVALID_REQUEST", "The engineering request kind is unsupported.");
  }
};

const parseWorkbenchOperation = (input: unknown): WorkbenchOperation => {
  const record = requireRecord(input, "workbench operation");
  const kind = requireString(record.kind, "workbench operation kind", 80);
  switch (kind) {
    case "project.create-object": {
      requireExactKeys(
        record,
        [
          "displayName",
          "kind",
          "objectId",
          "objectKind",
          "parentId",
          "payloadSchema",
          "presentationPayload",
          "semanticPayload",
        ],
        "create-object operation",
      );
      const objectKind = requireString(record.objectKind, "project object kind", 64);
      if (!isProjectStorageKind(objectKind)) {
        throw new EngineeringWorkerError("INVALID_REQUEST", "The project object kind is unsupported.");
      }
      const payloadBudget = { remaining: 8_192 };
      return {
        displayName: requireString(record.displayName, "object displayName", 256),
        kind,
        objectId: requireUuid(record.objectId, "object ID"),
        objectKind,
        parentId: requireUuid(record.parentId, "parent object ID"),
        payloadSchema: requireString(record.payloadSchema, "payload schema", 128),
        presentationPayload: parseProjectPayload(
          record.presentationPayload,
          "presentation payload",
          payloadBudget,
        ),
        semanticPayload: parseProjectPayload(
          record.semanticPayload,
          "semantic payload",
          payloadBudget,
        ),
      };
    }
    case "project.rename-object":
      requireExactKeys(record, ["displayName", "kind", "objectId"], "rename operation");
      return {
        displayName: requireString(record.displayName, "object displayName", 256),
        kind,
        objectId: requireUuid(record.objectId, "object ID"),
      };
    case "project.set-semantic-field":
    case "project.set-presentation-field": {
      requireExactKeys(record, ["key", "kind", "objectId", "value"], "field operation");
      const key = requireString(record.key, "project payload field key", 128);
      if (!/^[A-Za-z0-9_.-]+$/u.test(key)) {
        throw new EngineeringWorkerError(
          "INVALID_REQUEST",
          "The project payload field key is outside the closed grammar.",
        );
      }
      const payload = parseProjectPayload(
        { value: record.value },
        "project field value",
        { remaining: 8_192 },
      );
      const value = payload.value;
      if (value === undefined) {
        throw new EngineeringWorkerError("INVALID_REQUEST", "The project field value is missing.");
      }
      return {
        key,
        kind,
        objectId: requireUuid(record.objectId, "object ID"),
        value,
      };
    }
    case "project.delete-object":
      requireExactKeys(record, ["kind", "objectId"], "delete operation");
      return { kind, objectId: requireUuid(record.objectId, "object ID") };
    case "project.copy-objects": {
      requireExactKeys(
        record,
        ["kind", "sourceObjectIds", "targetParentId"],
        "copy operation",
      );
      const sourceObjectIds = requireArray(record.sourceObjectIds, "copy source IDs", MAX_PROJECT_OBJECTS, 1)
        .map((value) => requireUuid(value, "copy source ID"));
      if (new Set(sourceObjectIds).size !== sourceObjectIds.length) {
        throw new EngineeringWorkerError("INVALID_REQUEST", "Copy source identities must be unique.");
      }
      return {
        kind,
        sourceObjectIds,
        targetParentId: requireUuid(record.targetParentId, "copy destination ID"),
      };
    }
    case "project.undo":
    case "project.redo":
      requireExactKeys(record, ["kind"], `${kind} operation`);
      return { kind };
    default:
      throw new EngineeringWorkerError("INVALID_REQUEST", "The workbench operation is unsupported.");
  }
};

const parseKernelQuery = (bytes: Uint8Array): RawKernelQuery => {
  const envelope = requireRecord(decodeJson(bytes), "kernel query response");
  requireExactKeys(envelope, ["ok", "project", "schemaVersion", "status"], "kernel query response");
  if (envelope.ok !== true || envelope.schemaVersion !== 1) {
    throw new EngineeringWorkerError("INVALID_CORE_RESPONSE", "The project kernel query was not successful.");
  }
  const projectRecord = requireRecord(envelope.project, "kernel project");
  requireExactKeys(
    projectRecord,
    [
      "dependencies",
      "documentId",
      "documentRevision",
      "nextCreationOrdinal",
      "objects",
      "profile",
      "references",
      "rootId",
      "semanticRevision",
    ],
    "kernel project",
  );
  const profile = requireRecord(projectRecord.profile, "kernel profile");
  requireExactKeys(profile, ["id", "manifestHash", "version"], "kernel profile");
  requireString(profile.id, "kernel profile ID", 128);
  requireHash(profile.manifestHash, "kernel profile hash");
  requireString(profile.version, "kernel profile version", 128);

  const objects = requireArray(projectRecord.objects, "kernel objects", MAX_PROJECT_OBJECTS, 1)
    .map(parseKernelObject);
  const objectIds = new Set(objects.map((object) => object.id));
  if (objectIds.size !== objects.length) {
    throw new EngineeringWorkerError("INVALID_CORE_RESPONSE", "The project kernel returned duplicate objects.");
  }
  const rootId = requireUuid(projectRecord.rootId, "kernel project root ID");
  if (!objectIds.has(rootId)) {
    throw new EngineeringWorkerError("INVALID_CORE_RESPONSE", "The project kernel root is missing.");
  }
  const references = requireArray(projectRecord.references, "kernel references", MAX_PROJECT_OBJECTS)
    .map(parseKernelReference);
  const dependencies = requireArray(projectRecord.dependencies, "kernel dependencies", MAX_PROJECT_OBJECTS)
    .map((input) => {
      const dependency = requireRecord(input, "kernel dependency");
      requireExactKeys(dependency, ["reason", "sourceId", "targetId"], "kernel dependency");
      return {
        reason: requireString(dependency.reason, "kernel dependency reason", 128),
        sourceId: requireUuid(dependency.sourceId, "kernel dependency source"),
        targetId: requireUuid(dependency.targetId, "kernel dependency target"),
      };
    });
  const statusRecord = requireRecord(envelope.status, "kernel project status");
  requireExactKeys(
    statusRecord,
    [
      "canRedo",
      "canUndo",
      "documentDirty",
      "documentHash",
      "nextUndoToken",
      "savedDocumentHash",
      "savedDocumentRevision",
      "savedSemanticFingerprint",
      "semanticDirty",
      "semanticFingerprint",
    ],
    "kernel project status",
  );
  const status: RawKernelStatus = {
    canRedo: requireBoolean(statusRecord.canRedo, "kernel canRedo"),
    canUndo: requireBoolean(statusRecord.canUndo, "kernel canUndo"),
    documentDirty: requireBoolean(statusRecord.documentDirty, "kernel documentDirty"),
    documentHash: requireHash(statusRecord.documentHash, "kernel document hash"),
    nextUndoToken: nullableUuid(statusRecord.nextUndoToken, "kernel next undo token"),
    savedDocumentHash: nullableHash(statusRecord.savedDocumentHash, "kernel saved document hash"),
    savedDocumentRevision: nullableDecimal(statusRecord.savedDocumentRevision, "kernel saved revision"),
    savedSemanticFingerprint: nullableHash(
      statusRecord.savedSemanticFingerprint,
      "kernel saved semantic fingerprint",
    ),
    semanticDirty: requireBoolean(statusRecord.semanticDirty, "kernel semanticDirty"),
    semanticFingerprint: requireHash(statusRecord.semanticFingerprint, "kernel semantic fingerprint"),
  };
  return {
    project: {
      dependencies,
      documentId: requireUuid(projectRecord.documentId, "kernel document ID"),
      documentRevision: requireDecimal(projectRecord.documentRevision, "kernel document revision"),
      nextCreationOrdinal: requireDecimal(
        projectRecord.nextCreationOrdinal,
        "kernel next creation ordinal",
      ),
      objects,
      references,
      rootId,
      semanticRevision: requireDecimal(projectRecord.semanticRevision, "kernel semantic revision"),
    },
    status,
  };
};

const parseSystemQuery = (
  bytes: Uint8Array,
  kernelQuery: RawKernelQuery,
): RawSystemQuery => {
  const record = requireRecord(decodeJson(bytes), "canonical system query");
  requireExactKeys(
    record,
    [
      "allocationChangeCount",
      "artifactFingerprint",
      "canBuild",
      "channelBindingCount",
      "diagnostics",
      "profile",
      "schemaVersion",
      "sourceDocumentHash",
      "sourceSemanticFingerprint",
    ],
    "canonical system query",
  );
  if (record.schemaVersion !== 1) {
    throw new EngineeringWorkerError(
      "INVALID_CORE_RESPONSE",
      "The canonical system query schema is unsupported.",
    );
  }
  const profile = requireRecord(record.profile, "canonical system profile");
  requireExactKeys(profile, ["id", "manifestHash", "version"], "canonical system profile");
  const profileId = requireString(profile.id, "canonical system profile ID", 128);
  const profileVersion = requireString(profile.version, "canonical system profile version", 64);
  const manifestHash = requireHash(
    profile.manifestHash,
    "canonical system profile manifest hash",
  );
  if (
    profileId !== PROFILE_ID ||
    profileVersion !== PROFILE_VERSION ||
    manifestHash.toLowerCase() !== PROFILE_MANIFEST_HASH
  ) {
    throw new EngineeringWorkerError(
      "INVALID_CORE_RESPONSE",
      "The canonical system did not use the shipped EDU-21 profile.",
    );
  }
  const sourceDocumentHash = requireHash(
    record.sourceDocumentHash,
    "canonical system document hash",
  );
  const sourceSemanticFingerprint = requireHash(
    record.sourceSemanticFingerprint,
    "canonical system semantic fingerprint",
  );
  if (
    sourceDocumentHash.toLowerCase() !== kernelQuery.status.documentHash.toLowerCase() ||
    sourceSemanticFingerprint.toLowerCase() !==
      kernelQuery.status.semanticFingerprint.toLowerCase()
  ) {
    throw new EngineeringWorkerError(
      "INVALID_CORE_RESPONSE",
      "The canonical system projection does not match the active project snapshot.",
    );
  }
  const objectIds = new Set(kernelQuery.project.objects.map((object) => object.id));
  const diagnostics = requireArray(
    record.diagnostics,
    "canonical system diagnostics",
    10_000,
  ).map((input) => {
    const diagnostic = requireRecord(input, "canonical system diagnostic");
    requireExactKeys(
      diagnostic,
      [
        "blocking",
        "code",
        "message",
        "phase",
        "primaryObjectId",
        "relatedObjectIds",
      ],
      "canonical system diagnostic",
    );
    const primaryObjectId = requireUuid(
      diagnostic.primaryObjectId,
      "canonical diagnostic primary object",
    );
    const relatedObjectIds = requireArray(
      diagnostic.relatedObjectIds,
      "canonical diagnostic related objects",
      256,
    ).map((input) => requireUuid(input, "canonical diagnostic related object"));
    if (
      !objectIds.has(primaryObjectId) ||
      relatedObjectIds.some((id) => !objectIds.has(id)) ||
      relatedObjectIds.includes(primaryObjectId) ||
      new Set(relatedObjectIds).size !== relatedObjectIds.length
    ) {
      throw new EngineeringWorkerError(
        "INVALID_CORE_RESPONSE",
        "A canonical system diagnostic has invalid project anchors.",
      );
    }
    return {
      blocking: requireBoolean(diagnostic.blocking, "canonical diagnostic blocking flag"),
      code: requireString(diagnostic.code, "canonical diagnostic code", 128),
      message: requireString(diagnostic.message, "canonical diagnostic message", 2_048),
      phase: requireString(diagnostic.phase, "canonical diagnostic phase", 128),
      primaryObjectId,
      relatedObjectIds,
    };
  });
  const artifactFingerprint = nullableHash(
    record.artifactFingerprint,
    "canonical hardware artifact fingerprint",
  );
  const canBuild = requireBoolean(record.canBuild, "canonical hardware canBuild");
  if (
    canBuild !== (artifactFingerprint !== null) ||
    (canBuild && diagnostics.some((diagnostic) => diagnostic.blocking))
  ) {
    throw new EngineeringWorkerError(
      "INVALID_CORE_RESPONSE",
      "The canonical hardware build state is internally inconsistent.",
    );
  }
  return {
    allocationChangeCount: requireSafeInteger(
      record.allocationChangeCount,
      "canonical allocation change count",
      0,
      MAX_PROJECT_OBJECTS,
    ),
    artifactFingerprint,
    canBuild,
    channelBindingCount: requireSafeInteger(
      record.channelBindingCount,
      "canonical channel binding count",
      0,
      1_000_000,
    ),
    diagnostics,
    profile: {
      id: profileId,
      manifestHash,
      version: profileVersion,
    },
    sourceDocumentHash,
    sourceSemanticFingerprint,
  };
};

const parseKernelObject = (input: unknown): RawKernelObject => {
  const object = requireRecord(input, "kernel object");
  requireExactKeys(
    object,
    [
      "creationOrdinal",
      "displayName",
      "id",
      "kind",
      "lifecycle",
      "objectRevision",
      "parentId",
      "payloadSchema",
      "presentationPayload",
      "semanticPayload",
      "semanticRevision",
    ],
    "kernel object",
  );
  if (object.lifecycle !== "active" && object.lifecycle !== "tombstoned") {
    throw new EngineeringWorkerError("INVALID_CORE_RESPONSE", "The project object lifecycle is invalid.");
  }
  return {
    creationOrdinal: requireDecimal(object.creationOrdinal, "kernel object creation ordinal"),
    displayName: requireString(object.displayName, "kernel object displayName", 256),
    id: requireUuid(object.id, "kernel object ID"),
    kind: requireString(object.kind, "kernel object kind", 64),
    lifecycle: object.lifecycle,
    objectRevision: requireDecimal(object.objectRevision, "kernel object revision"),
    parentId: nullableUuid(object.parentId, "kernel object parent"),
    payloadSchema: requireString(object.payloadSchema, "kernel object payload schema", 128),
    presentationPayload: parseProjectPayload(
      object.presentationPayload,
      "kernel presentation payload",
      { remaining: 8_192 },
    ),
    semanticPayload: parseProjectPayload(
      object.semanticPayload,
      "kernel semantic payload",
      { remaining: 8_192 },
    ),
    semanticRevision: requireDecimal(object.semanticRevision, "kernel object semantic revision"),
  };
};

const parseKernelReference = (input: unknown): RawKernelReference => {
  const reference = requireRecord(input, "kernel reference");
  requireExactKeys(
    reference,
    ["expectedTargetKind", "kind", "resolution", "sourceId", "sourceLocation", "targetId"],
    "kernel reference",
  );
  if (reference.resolution !== "resolved" && reference.resolution !== "unresolved") {
    throw new EngineeringWorkerError("INVALID_CORE_RESPONSE", "The project reference state is invalid.");
  }
  return {
    expectedTargetKind: requireString(reference.expectedTargetKind, "reference target kind", 64),
    kind: requireString(reference.kind, "reference kind", 64),
    resolution: reference.resolution,
    sourceId: requireUuid(reference.sourceId, "reference source ID"),
    sourceLocation: requireString(reference.sourceLocation, "reference source location", 1_024),
    targetId: requireUuid(reference.targetId, "reference target ID"),
  };
};

const parseKernelCommandResult = (bytes: Uint8Array): RawKernelCommandResult => {
  const envelope = requireRecord(decodeJson(bytes), "kernel command response");
  requireExactKeys(envelope, ["ok", "result", "schemaVersion"], "kernel command response");
  if (typeof envelope.ok !== "boolean" || envelope.schemaVersion !== 1) {
    throw new EngineeringWorkerError("INVALID_CORE_RESPONSE", "The project kernel result envelope is invalid.");
  }
  const result = requireRecord(envelope.result, "kernel command result");
  requireExactKeys(
    result,
    [
      "affectedObjectIds",
      "afterProjectHash",
      "beforeProjectHash",
      "diagnostics",
      "domainEvents",
      "outcome",
      "transactionId",
      "undoToken",
    ],
    "kernel command result",
  );
  if (result.outcome !== "committed" && result.outcome !== "rejected" && result.outcome !== "blocked") {
    throw new EngineeringWorkerError("INVALID_CORE_RESPONSE", "The project kernel outcome is invalid.");
  }
  const affectedObjectIds = requireArray(result.affectedObjectIds, "affected object IDs", MAX_PROJECT_OBJECTS)
    .map((value) => requireUuid(value, "affected object ID"));
  requireArray(result.domainEvents, "kernel domain events", 2_048);
  const diagnostics = requireArray(result.diagnostics, "kernel diagnostics", 2_048).map((input) => {
    const diagnostic = requireRecord(input, "kernel diagnostic");
    requireExactKeys(diagnostic, ["code", "message", "objectIds"], "kernel diagnostic");
    return {
      code: requireString(diagnostic.code, "kernel diagnostic code", 128),
      message: requireString(diagnostic.message, "kernel diagnostic message", 4_096),
      objectIds: requireArray(diagnostic.objectIds, "kernel diagnostic object IDs", 256)
        .map((value) => requireUuid(value, "kernel diagnostic object ID")),
    };
  });
  return {
    affectedObjectIds,
    afterProjectHash: nullableHash(result.afterProjectHash, "kernel after project hash"),
    beforeProjectHash: requireHash(result.beforeProjectHash, "kernel before project hash"),
    diagnostics,
    outcome: result.outcome,
    transactionId: requireUuid(result.transactionId, "kernel transaction ID"),
    undoToken: nullableUuid(result.undoToken, "kernel undo token"),
  };
};

const decodeJson = (bytes: Uint8Array): unknown => {
  if (bytes.byteLength < 2 || bytes.byteLength > MAX_PROJECT_BYTES) {
    throw new EngineeringWorkerError("INVALID_CORE_RESPONSE", "The project kernel response is out of bounds.");
  }
  try {
    return JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)) as unknown;
  } catch {
    throw new EngineeringWorkerError("INVALID_CORE_RESPONSE", "The project kernel returned malformed JSON.");
  }
};

const sha256 = async (
  bytes: Uint8Array<ArrayBuffer>,
): Promise<Readonly<{ bytes: Uint8Array<ArrayBuffer>; hex: string }>> => {
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  return {
    bytes: digest,
    hex: [...digest].map((byte) => byte.toString(16).padStart(2, "0")).join("").toUpperCase(),
  };
};

const deriveUuid = async (seed: string): Promise<string> => {
  const digest = (await sha256(new TextEncoder().encode(seed))).bytes.slice(0, 16);
  const versionByte = digest[6];
  const variantByte = digest[8];
  if (versionByte === undefined || variantByte === undefined) {
    throw new EngineeringWorkerError("HASH_FAILURE", "A deterministic identity could not be derived.");
  }
  digest[6] = (versionByte & 0x0f) | 0x40;
  digest[8] = (variantByte & 0x3f) | 0x80;
  const hex = [...digest].map((byte) => byte.toString(16).padStart(2, "0")).join("");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
};

const normalizeError = (error: unknown): Readonly<{ code: string; message: string }> => {
  if (error instanceof EngineeringWorkerError) {
    return { code: error.code, message: error.message };
  }
  if (error instanceof WasmKernelError) {
    return { code: "CORE_REJECTED", message: error.message };
  }
  if (error instanceof Error) {
    return { code: "ENGINEERING_FAILURE", message: error.message };
  }
  return { code: "ENGINEERING_FAILURE", message: "The engineering request did not complete." };
};

type PlainRecord = Record<string, unknown>;

const requireRecord = (input: unknown, label: string): PlainRecord => {
  if (
    typeof input !== "object" ||
    input === null ||
    Array.isArray(input) ||
    Object.getPrototypeOf(input) !== Object.prototype
  ) {
    throw new EngineeringWorkerError("INVALID_REQUEST", `${label} must be a plain record.`);
  }
  return input as PlainRecord;
};

const requireExactKeys = (record: PlainRecord, keys: readonly string[], label: string): void => {
  const actual = Object.keys(record).sort(ordinalCompare);
  const expected = [...keys].sort(ordinalCompare);
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) {
    throw new EngineeringWorkerError("INVALID_REQUEST", `${label} has an invalid field set.`);
  }
};

const requireString = (input: unknown, label: string, maximum: number): string => {
  if (typeof input !== "string" || input.length < 1 || [...input].length > maximum || input.includes("\0")) {
    throw new EngineeringWorkerError("INVALID_REQUEST", `${label} is invalid.`);
  }
  return input;
};

const requireUuid = (input: unknown, label: string): string => {
  if (typeof input !== "string" || !UUID_PATTERN.test(input)) {
    throw new EngineeringWorkerError("INVALID_REQUEST", `${label} must be a canonical UUID.`);
  }
  return input;
};

const requireHash = (input: unknown, label: string): string => {
  if (typeof input !== "string" || !HASH_PATTERN.test(input)) {
    throw new EngineeringWorkerError("INVALID_REQUEST", `${label} must be a SHA-256 digest.`);
  }
  return input;
};

const requireDecimal = (input: unknown, label: string): string => {
  if (typeof input !== "string" || !DECIMAL_UINT64_PATTERN.test(input) || BigInt(input) > (1n << 64n) - 1n) {
    throw new EngineeringWorkerError("INVALID_REQUEST", `${label} must be a canonical uint64.`);
  }
  return input;
};

const requireBoolean = (input: unknown, label: string): boolean => {
  if (typeof input !== "boolean") {
    throw new EngineeringWorkerError("INVALID_REQUEST", `${label} must be a boolean.`);
  }
  return input;
};

const requireSafeInteger = (
  input: unknown,
  label: string,
  minimum: number,
  maximum: number,
): number => {
  if (!Number.isSafeInteger(input) || (input as number) < minimum || (input as number) > maximum) {
    throw new EngineeringWorkerError("INVALID_REQUEST", `${label} is out of bounds.`);
  }
  return input as number;
};

const requireArray = (
  input: unknown,
  label: string,
  maximum: number,
  minimum = 0,
): readonly unknown[] => {
  if (!Array.isArray(input) || input.length < minimum || input.length > maximum) {
    throw new EngineeringWorkerError("INVALID_REQUEST", `${label} is out of bounds.`);
  }
  return input;
};

const isProjectStorageKind = (value: string): value is ProjectStorageKind =>
  PROJECT_STORAGE_KINDS.has(value as ProjectStorageKind);

type PayloadBudget = { remaining: number };

const parseProjectPayload = (
  input: unknown,
  label: string,
  budget: PayloadBudget,
  depth = 0,
): ProjectPayload => {
  if (depth > 16) {
    throw new EngineeringWorkerError("INVALID_REQUEST", `${label} exceeds its nesting limit.`);
  }
  const record = requireRecord(input, label);
  if (Object.keys(record).length > 2_048) {
    throw new EngineeringWorkerError("INVALID_REQUEST", `${label} exceeds its field limit.`);
  }
  const parsed: Record<string, ProjectPayloadValue> = {};
  for (const [key, value] of Object.entries(record)) {
    if (key.length < 1 || key.length > 128 || key.includes("\0")) {
      throw new EngineeringWorkerError("INVALID_REQUEST", `${label} contains an invalid field name.`);
    }
    parsed[key] = parseProjectPayloadValue(value, `${label}.${key}`, budget, depth + 1);
  }
  return parsed;
};

const parseProjectPayloadValue = (
  input: unknown,
  label: string,
  budget: PayloadBudget,
  depth: number,
): ProjectPayloadValue => {
  budget.remaining -= 1;
  if (budget.remaining < 0 || depth > 16) {
    throw new EngineeringWorkerError("INVALID_REQUEST", `${label} exceeds its complexity limit.`);
  }
  if (input === null || typeof input === "boolean") {
    return input;
  }
  if (typeof input === "string") {
    if (input.length > 1_048_576 || input.includes("\0")) {
      throw new EngineeringWorkerError("INVALID_REQUEST", `${label} contains an invalid string.`);
    }
    return input;
  }
  if (Array.isArray(input)) {
    if (input.length > 4_096) {
      throw new EngineeringWorkerError("INVALID_REQUEST", `${label} exceeds its list limit.`);
    }
    return input.map((value, index) =>
      parseProjectPayloadValue(value, `${label}[${index}]`, budget, depth + 1),
    );
  }
  const tagged = requireRecord(input, label);
  requireExactKeys(tagged, ["$type", "value"], label);
  const type = requireString(tagged.$type, `${label}.$type`, 16);
  if (type === "record") {
    return {
      $type: "record",
      value: parseProjectPayload(tagged.value, `${label}.value`, budget, depth + 1),
    };
  }
  if (type !== "i64" && type !== "u64") {
    throw new EngineeringWorkerError("INVALID_REQUEST", `${label} has an unsupported value type.`);
  }
  const value = requireString(tagged.value, `${label}.value`, 24);
  if (!SIGNED_DECIMAL_PATTERN.test(value)) {
    throw new EngineeringWorkerError("INVALID_REQUEST", `${label} is not canonical decimal text.`);
  }
  const numeric = BigInt(value);
  const inRange = type === "u64"
    ? numeric >= 0n && numeric <= (1n << 64n) - 1n
    : numeric >= -(1n << 63n) && numeric <= (1n << 63n) - 1n;
  if (!inRange) {
    throw new EngineeringWorkerError("INVALID_REQUEST", `${label} exceeds its numeric range.`);
  }
  return { $type: type, value };
};

const nullableUuid = (input: unknown, label: string): string | null =>
  input === null ? null : requireUuid(input, label);

const nullableHash = (input: unknown, label: string): string | null =>
  input === null ? null : requireHash(input, label);

const nullableDecimal = (input: unknown, label: string): string | null =>
  input === null ? null : requireDecimal(input, label);

const upperHash = (hash: string): string => requireHash(hash, "hash").toUpperCase();

const nullableUpperHash = (hash: string | null): string | null =>
  hash === null ? null : upperHash(hash);

const ordinalCompare = (left: string, right: string): number => left === right ? 0 : left < right ? -1 : 1;

const truncateCharacters = (value: string, maximum: number): string => [...value].slice(0, maximum).join("");
