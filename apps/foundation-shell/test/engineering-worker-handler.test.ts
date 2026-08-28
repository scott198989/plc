import { describe, expect, it } from "vitest";

import {
  executeEngineeringRequest,
  type EngineeringResponse,
} from "../src/engineering-worker-handler";

const IDS = {
  create: "10000000-0000-4000-8000-000000000001",
  document: "20000000-0000-4000-8000-000000000001",
  root: "30000000-0000-4000-8000-000000000001",
  createController: "31000000-0000-4000-8000-000000000001",
  controller: "32000000-0000-4000-8000-000000000001",
  createFolder: "33000000-0000-4000-8000-000000000001",
  folder: "34000000-0000-4000-8000-000000000001",
  copyFolder: "35000000-0000-4000-8000-000000000001",
  deleteFolder: "36000000-0000-4000-8000-000000000001",
  undoDelete: "37000000-0000-4000-8000-000000000001",
  redoDelete: "38000000-0000-4000-8000-000000000001",
  rename: "40000000-0000-4000-8000-000000000001",
  undo: "50000000-0000-4000-8000-000000000001",
  redo: "60000000-0000-4000-8000-000000000001",
  prepareSaveAs: "70000000-0000-4000-8000-000000000001",
  saveAsDocument: "80000000-0000-4000-8000-000000000001",
  commitSaveAs: "90000000-0000-4000-8000-000000000001",
  firstGrant: "a0000000-0000-4000-8000-000000000001",
  open: "b0000000-0000-4000-8000-000000000001",
  openGrant: "c0000000-0000-4000-8000-000000000001",
  prepareSave: "d0000000-0000-4000-8000-000000000001",
  commitSave: "e0000000-0000-4000-8000-000000000001",
  corruptOpen: "f0000000-0000-4000-8000-000000000001",
  corruptGrant: "01000000-0000-4000-8000-000000000001",
  renameAfterCorruption: "02000000-0000-4000-8000-000000000001",
} as const;

describe("real engineering worker and WASM kernel", () => {
  it("creates, edits, reverses, saves, reopens, and rejects corruption atomically", async () => {
    const created = successValue(await executeEngineeringRequest({
      displayName: "Training Cell",
      documentId: IDS.document,
      kind: "engineering.project.create",
      projectRootId: IDS.root,
      requestId: IDS.create,
    }));
    expect(snapshot(created).projectRootId).toBe(IDS.root);
    expect(snapshot(created).dirtyState).toBe("semantic-dirty");

    const controllerCreated = operationValue(await executeEngineeringRequest({
      kind: "engineering.project.command",
      operation: {
        displayName: "Controller",
        kind: "project.create-object",
        objectId: IDS.controller,
        objectKind: "controller",
        parentId: IDS.root,
        payloadSchema: "edu.controller/1",
        presentationPayload: {},
        semanticPayload: {
          catalogId: "vctrl-c1",
          profileId: "EDU-21 Core",
          profileVersion: "1.0.0",
        },
      },
      requestId: IDS.createController,
    }));
    expect(controllerCreated.outcome).toBe("committed");
    expect(controllerCreated.snapshot.objects[IDS.controller]?.kind).toBe("Controller");

    const folderCreated = operationValue(await executeEngineeringRequest({
      kind: "engineering.project.command",
      operation: {
        displayName: "Engineering folder",
        kind: "project.create-object",
        objectId: IDS.folder,
        objectKind: "folder",
        parentId: IDS.root,
        payloadSchema: "edu.folder/1",
        presentationPayload: {},
        semanticPayload: {},
      },
      requestId: IDS.createFolder,
    }));
    expect(folderCreated.outcome).toBe("committed");

    const copied = operationValue(await executeEngineeringRequest({
      kind: "engineering.project.command",
      operation: {
        kind: "project.copy-objects",
        sourceObjectIds: [IDS.folder],
        targetParentId: IDS.root,
      },
      requestId: IDS.copyFolder,
    }));
    expect(copied.outcome).toBe("committed");
    const copiedFolders = Object.values(copied.snapshot.objects).filter((object) => object.kind === "Folder");
    expect(copiedFolders).toHaveLength(2);
    expect(copiedFolders.map((object) => object.displayName).sort()).toEqual([
      "Engineering folder",
      "Engineering folder copy",
    ]);

    const deleted = operationValue(await executeEngineeringRequest({
      kind: "engineering.project.command",
      operation: { kind: "project.delete-object", objectId: IDS.folder },
      requestId: IDS.deleteFolder,
    }));
    expect(deleted.outcome).toBe("committed");
    expect(deleted.snapshot.objects[IDS.folder]?.lifecycle).toBe("tombstoned");

    const deleteUndone = operationValue(await executeEngineeringRequest({
      kind: "engineering.project.command",
      operation: { kind: "project.undo" },
      requestId: IDS.undoDelete,
    }));
    expect(deleteUndone.snapshot.objects[IDS.folder]?.lifecycle).toBe("active");

    const deleteRedone = operationValue(await executeEngineeringRequest({
      kind: "engineering.project.command",
      operation: { kind: "project.redo" },
      requestId: IDS.redoDelete,
    }));
    expect(deleteRedone.snapshot.objects[IDS.folder]?.lifecycle).toBe("tombstoned");

    const renamed = operationValue(await executeEngineeringRequest({
      kind: "engineering.project.command",
      operation: {
        displayName: "Packaging Cell",
        kind: "project.rename-object",
        objectId: IDS.root,
      },
      requestId: IDS.rename,
    }));
    expect(renamed.outcome).toBe("committed");
    expect(renamed.snapshot.projectName).toBe("Packaging Cell");
    expect(renamed.snapshot.projectRootId).toBe(IDS.root);

    const undone = operationValue(await executeEngineeringRequest({
      kind: "engineering.project.command",
      operation: { kind: "project.undo" },
      requestId: IDS.undo,
    }));
    expect(undone.outcome).toBe("committed");
    expect(undone.snapshot.projectName).toBe("Training Cell");
    expect(undone.snapshot.projectRootId).toBe(IDS.root);

    const redone = operationValue(await executeEngineeringRequest({
      kind: "engineering.project.command",
      operation: { kind: "project.redo" },
      requestId: IDS.redo,
    }));
    expect(redone.outcome).toBe("committed");
    expect(redone.snapshot.projectName).toBe("Packaging Cell");

    const firstPrepared = preparedValue(await executeEngineeringRequest({
      kind: "engineering.persistence.prepare",
      mode: "save-as",
      newDocumentId: IDS.saveAsDocument,
      requestId: IDS.prepareSaveAs,
    }));
    const firstPackage = new Uint8Array(firstPrepared.bytes).slice();
    expect(firstPackage.byteLength).toBeGreaterThan(100);
    const firstCommitted = snapshot(successValue(await executeEngineeringRequest({
      fileGrantId: IDS.firstGrant,
      kind: "engineering.persistence.commit",
      pendingSaveId: firstPrepared.pendingSaveId,
      requestId: IDS.commitSaveAs,
      verifiedBytes: firstPackage.byteLength,
    })));
    expect(firstCommitted.documentId).toBe(IDS.saveAsDocument);
    expect(firstCommitted.projectRootId).toBe(IDS.root);
    expect(firstCommitted.dirtyState).toBe("clean");

    const reopened = snapshot(successValue(await executeEngineeringRequest({
      bytes: firstPackage.slice().buffer,
      fileGrantId: IDS.openGrant,
      kind: "engineering.project.open",
      requestId: IDS.open,
    })));
    expect(reopened.documentId).toBe(IDS.saveAsDocument);
    expect(reopened.projectRootId).toBe(IDS.root);
    expect(reopened.projectName).toBe("Packaging Cell");

    const secondPrepared = preparedValue(await executeEngineeringRequest({
      kind: "engineering.persistence.prepare",
      mode: "save",
      newDocumentId: null,
      requestId: IDS.prepareSave,
    }));
    expect(new Uint8Array(secondPrepared.bytes)).toEqual(firstPackage);
    const secondCommitted = snapshot(successValue(await executeEngineeringRequest({
      fileGrantId: IDS.openGrant,
      kind: "engineering.persistence.commit",
      pendingSaveId: secondPrepared.pendingSaveId,
      requestId: IDS.commitSave,
      verifiedBytes: secondPrepared.bytes.byteLength,
    })));
    expect(secondCommitted.projectHash).toBe(reopened.projectHash);

    const corrupt = firstPackage.slice();
    const lastIndex = corrupt.byteLength - 1;
    corrupt[lastIndex] = (corrupt[lastIndex] ?? 0) ^ 0xff;
    const rejectedOpen = await executeEngineeringRequest({
      bytes: corrupt.buffer,
      fileGrantId: IDS.corruptGrant,
      kind: "engineering.project.open",
      requestId: IDS.corruptOpen,
    });
    expect(rejectedOpen.ok).toBe(false);

    const preserved = operationValue(await executeEngineeringRequest({
      kind: "engineering.project.command",
      operation: {
        displayName: "Preserved after rejection",
        kind: "project.rename-object",
        objectId: IDS.root,
      },
      requestId: IDS.renameAfterCorruption,
    }));
    expect(preserved.outcome).toBe("committed");
    expect(preserved.snapshot.documentId).toBe(IDS.saveAsDocument);
    expect(preserved.snapshot.projectName).toBe("Preserved after rejection");
  });
});

const successValue = (response: EngineeringResponse): unknown => {
  expect(response.ok, response.error?.message).toBe(true);
  if (!response.ok || response.value === undefined) {
    throw new Error(response.error?.message ?? "Expected a successful engineering response.");
  }
  return response.value;
};

const snapshot = (value: unknown): Readonly<Record<string, unknown>> & {
  dirtyState: string;
  documentId: string;
  projectHash: string;
  projectName: string;
  projectRootId: string;
  objects: Readonly<Record<string, Readonly<{
    displayName: string;
    kind: string;
    lifecycle: string;
  }>>>;
} => {
  if (!isRecord(value) || typeof value.projectRootId !== "string") {
    throw new Error("Expected a workbench snapshot.");
  }
  return value as ReturnType<typeof snapshot>;
};

const operationValue = (response: EngineeringResponse): {
  outcome: string;
  snapshot: ReturnType<typeof snapshot>;
} => {
  const value = successValue(response);
  if (!isRecord(value) || typeof value.outcome !== "string" || !isRecord(value.snapshot)) {
    throw new Error("Expected a workbench operation result.");
  }
  return value as ReturnType<typeof operationValue>;
};

const preparedValue = (response: EngineeringResponse): {
  bytes: ArrayBuffer;
  packageHash: string;
  pendingSaveId: string;
} => {
  const value = successValue(response);
  if (
    !isRecord(value) ||
    !(value.bytes instanceof ArrayBuffer) ||
    typeof value.packageHash !== "string" ||
    typeof value.pendingSaveId !== "string"
  ) {
    throw new Error("Expected a prepared save.");
  }
  return value as ReturnType<typeof preparedValue>;
};

const isRecord = (value: unknown): value is Readonly<Record<string, unknown>> =>
  typeof value === "object" && value !== null && !Array.isArray(value);
