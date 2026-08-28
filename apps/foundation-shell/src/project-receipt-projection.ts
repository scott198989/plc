import type {
  Diagnostic,
  ProjectObjectSnapshot,
  ProjectSnapshotReceipt,
  SourceAnchor,
} from "@govs/plc-contract";

import type {
  ProjectPayload,
  WorkbenchDiagnosticView,
  WorkbenchObjectView,
  WorkbenchSnapshot,
} from "./workbench-types";

export type WorkbenchSessionProjection = Readonly<{
  diagnostics: readonly Diagnostic[];
  fileGrantId: string | null;
  payloads: Readonly<Record<string, Readonly<{
    payloadSchema: string;
    presentationPayload: ProjectPayload;
    semanticPayload: ProjectPayload;
  }>>>;
  redoLabel: string | null;
  undoLabel: string | null;
}>;

export type ProjectWorkbenchSnapshot = Omit<WorkbenchSnapshot, "runtime">;

/**
 * Adapts an already validated domain receipt for display. It derives no PLC
 * semantics and rejects graph inconsistencies instead of repairing them.
 */
export const projectReceiptToWorkbench = (
  receipt: ProjectSnapshotReceipt,
  session: WorkbenchSessionProjection,
): ProjectWorkbenchSnapshot => {
  const sourceObjects = new Map(receipt.objects.map((object) => [object.id, object]));
  const root = sourceObjects.get(receipt.projectRootId);
  if (root === undefined || root.kind !== "ProjectRoot" || root.parentId !== null) {
    throw new Error("The canonical project receipt has no valid project root.");
  }

  const objects: Record<string, WorkbenchObjectView> = Object.create(null) as Record<
    string,
    WorkbenchObjectView
  >;
  for (const object of receipt.objects) {
    assertContainmentLinks(object, sourceObjects);
    const payload = session.payloads[object.id];
    if (payload === undefined) {
      throw new Error(`Canonical payload for object ${object.id} is missing.`);
    }
    objects[object.id] = {
      children: object.orderedChildIds,
      creationOrdinal: object.creationOrdinal,
      displayName: object.displayName,
      id: object.id,
      kind: object.kind,
      lifecycle: object.lifecycle,
      objectRevision: object.objectRevision,
      parentId: object.parentId,
      payloadSchema: payload.payloadSchema,
      presentationPayload: payload.presentationPayload,
      semanticPayload: payload.semanticPayload,
      semanticRevision: object.semanticRevision,
    };
  }

  return {
    buildState: aggregateBuildState(receipt),
    diagnostics: session.diagnostics.map(projectDiagnostic),
    dirtyState: receipt.dirtyBuildState.semanticDirty
      ? "semantic-dirty"
      : receipt.dirtyBuildState.documentDirty
        ? "presentation-dirty"
        : "clean",
    documentId: receipt.documentId,
    documentRevision: receipt.documentRevision,
    fileGrantId: session.fileGrantId,
    lastSavedProjectHash: receipt.dirtyBuildState.savedDocumentHash,
    objects,
    projectHash: receipt.dirtyBuildState.currentDocumentHash,
    projectName: root.displayName,
    projectRootId: receipt.projectRootId,
    semanticRevision: receipt.semanticRevision,
    undo: {
      canRedo: session.redoLabel !== null,
      canUndo: session.undoLabel !== null,
      redoLabel: session.redoLabel,
      undoLabel: session.undoLabel,
    },
  };
};

const assertContainmentLinks = (
  object: ProjectObjectSnapshot,
  objects: ReadonlyMap<string, ProjectObjectSnapshot>,
): void => {
  if (object.parentId !== null && !objects.has(object.parentId)) {
    throw new Error(`Object ${object.id} has an unknown parent.`);
  }
  const seen = new Set<string>();
  for (const childId of object.orderedChildIds) {
    if (seen.has(childId)) {
      throw new Error(`Object ${object.id} contains a duplicate child.`);
    }
    seen.add(childId);
    const child = objects.get(childId);
    if (child === undefined || child.parentId !== object.id) {
      throw new Error(`Object ${object.id} has an invalid child edge.`);
    }
  }
};

const aggregateBuildState = (
  receipt: ProjectSnapshotReceipt,
): ProjectWorkbenchSnapshot["buildState"] => {
  const states = receipt.dirtyBuildState.controllerStates.flatMap((state) => [
    state.hardware,
    state.software,
  ]);
  if (states.includes("blocked")) {
    return "blocked";
  }
  if (states.includes("stale")) {
    return "stale";
  }
  if (states.length > 0 && states.every((state) => state === "current")) {
    return "current";
  }
  return "not-built";
};

const projectDiagnostic = (diagnostic: Diagnostic): WorkbenchDiagnosticView => ({
  blocking: diagnostic.blocking,
  code: diagnostic.code,
  diagnosticId: diagnostic.diagnosticId,
  message: diagnostic.cause,
  objectId: anchorOwner(diagnostic.primaryAnchor),
  phase: diagnostic.phase,
  severity: diagnostic.severity,
});

const anchorOwner = (anchor: SourceAnchor): string => {
  switch (anchor.anchorKind) {
    case "generated":
    case "graph":
    case "project":
    case "text":
      return anchor.ownerObjectId;
  }
};
