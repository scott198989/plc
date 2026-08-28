import { describe, expect, it } from "vitest";
import type { ProjectSnapshotReceipt } from "@govs/plc-contract";

import { projectReceiptToWorkbench } from "../src/project-receipt-projection";

const hash = (digit: string): string => digit.repeat(64);
const rootId = "00000000-0000-4000-8000-000000000001";
const controllerId = "00000000-0000-4000-8000-000000000002";
const payloads = {
  [controllerId]: {
    payloadSchema: "edu.controller/1",
    presentationPayload: {},
    semanticPayload: { catalogId: "vctrl-c1" },
  },
  [rootId]: {
    payloadSchema: "edu.project/1",
    presentationPayload: {},
    semanticPayload: {},
  },
} as const;

const receipt = (): ProjectSnapshotReceipt => ({
  dirtyBuildState: {
    controllerStates: [{
      controllerId,
      hardware: "current",
      loadedArtifactFingerprint: null,
      software: "stale",
    }],
    currentDocumentHash: hash("A"),
    currentSemanticFingerprint: hash("B"),
    documentDirty: true,
    savedDocumentHash: hash("C"),
    savedDocumentRevision: "2",
    savedSemanticFingerprint: hash("D"),
    semanticDirty: true,
  },
  documentId: "00000000-0000-4000-8000-000000000003",
  documentRevision: "3",
  domain: "project-snapshot",
  objects: [
    {
      creationOrdinal: "1",
      displayName: "Packaging Cell",
      id: rootId,
      kind: "ProjectRoot",
      lifecycle: "active",
      objectRevision: "3",
      orderedChildIds: [controllerId],
      parentId: null,
      references: [],
      semanticRevision: "2",
    },
    {
      creationOrdinal: "2",
      displayName: "VC-1",
      id: controllerId,
      kind: "Controller",
      lifecycle: "active",
      objectRevision: "1",
      orderedChildIds: [],
      parentId: rootId,
      references: [],
      semanticRevision: "1",
    },
  ],
  projectRootId: rootId,
  scope: "summary",
  semanticRevision: "2",
});

describe("projectReceiptToWorkbench", () => {
  it("projects canonical identity, dirty state, build state, and exact order", () => {
    const projected = projectReceiptToWorkbench(receipt(), {
      diagnostics: [],
      fileGrantId: "grant-1",
      payloads,
      redoLabel: null,
      undoLabel: "Undo rename",
    });

    expect(projected.projectName).toBe("Packaging Cell");
    expect(projected.dirtyState).toBe("semantic-dirty");
    expect(projected.buildState).toBe("stale");
    expect(projected.objects[rootId]?.children).toEqual([controllerId]);
    expect(projected.objects[controllerId]?.parentId).toBe(rootId);
    expect(projected.undo.canUndo).toBe(true);
    expect(projected.lastSavedProjectHash).toBe(hash("C"));
  });

  it("fails closed when the receipt has an inconsistent child edge", () => {
    const invalid = receipt();
    const broken: ProjectSnapshotReceipt = {
      ...invalid,
      objects: invalid.objects.map((object) =>
        object.id === controllerId ? { ...object, parentId: null } : object,
      ),
    };

    expect(() => projectReceiptToWorkbench(broken, {
      diagnostics: [],
      fileGrantId: null,
      payloads,
      redoLabel: null,
      undoLabel: null,
    })).toThrow(/invalid child edge/u);
  });
});
