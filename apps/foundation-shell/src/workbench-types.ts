import type { ProjectObjectKind } from "@govs/plc-contract";

/** Hardware-only children join canonical project objects in the navigator projection. */
export type WorkbenchObjectKind = ProjectObjectKind | "Rack" | "Channel";

export type ProjectStorageKind =
  | "folder"
  | "controller"
  | "rack"
  | "module"
  | "network"
  | "symbol-table"
  | "tag"
  | "type-definition"
  | "program-block"
  | "data-block"
  | "build-record"
  | "snapshot-reference"
  | "generic";

export type ProjectPayloadValue =
  | null
  | boolean
  | string
  | Readonly<{ $type: "i64" | "u64"; value: string }>
  | readonly ProjectPayloadValue[]
  | Readonly<{
      $type: "record";
      value: Readonly<Record<string, ProjectPayloadValue>>;
    }>;

export type ProjectPayload = Readonly<Record<string, ProjectPayloadValue>>;

export type WorkbenchObjectView = Readonly<{
  children: readonly string[];
  creationOrdinal: string;
  displayName: string;
  id: string;
  kind: WorkbenchObjectKind;
  lifecycle: "active" | "tombstoned";
  objectRevision: string;
  parentId: string | null;
  payloadSchema: string;
  presentationPayload: ProjectPayload;
  semanticPayload: ProjectPayload;
  semanticRevision: string;
}>;

export type WorkbenchDiagnosticView = Readonly<{
  blocking: boolean;
  code: string;
  diagnosticId: string;
  message: string;
  objectId: string | null;
  phase: string;
  severity: "Info" | "Warning" | "Error" | "Internal";
}>;

export type WorkbenchSnapshot = Readonly<{
  buildState: "not-built" | "current" | "stale" | "blocked";
  diagnostics: readonly WorkbenchDiagnosticView[];
  dirtyState: "clean" | "presentation-dirty" | "semantic-dirty";
  documentId: string;
  documentRevision: string;
  fileGrantId: string | null;
  lastSavedProjectHash: string | null;
  objects: Readonly<Record<string, WorkbenchObjectView>>;
  projectHash: string;
  projectName: string;
  projectRootId: string;
  semanticRevision: string;
  undo: Readonly<{
    canRedo: boolean;
    canUndo: boolean;
    redoLabel: string | null;
    undoLabel: string | null;
  }>;
}>;

export type WorkbenchOperation =
  | Readonly<{
      displayName: string;
      kind: "project.create-object";
      objectId: string;
      objectKind: ProjectStorageKind;
      parentId: string;
      payloadSchema: string;
      presentationPayload: ProjectPayload;
      semanticPayload: ProjectPayload;
    }>
  | Readonly<{ kind: "project.rename-object"; displayName: string; objectId: string }>
  | Readonly<{ kind: "project.delete-object"; objectId: string }>
  | Readonly<{
      kind: "project.copy-objects";
      sourceObjectIds: readonly string[];
      targetParentId: string;
    }>
  | Readonly<{
      key: string;
      kind: "project.set-semantic-field" | "project.set-presentation-field";
      objectId: string;
      value: ProjectPayloadValue;
    }>
  | Readonly<{ kind: "project.undo" }>
  | Readonly<{ kind: "project.redo" }>;

export type WorkbenchOperationResult = Readonly<{
  diagnostics: readonly WorkbenchDiagnosticView[];
  outcome: "committed" | "rejected" | "blocked";
  snapshot: WorkbenchSnapshot;
}>;
