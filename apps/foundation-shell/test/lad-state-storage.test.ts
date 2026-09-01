import { describe, expect, it } from "vitest";

import { recordValue, unsignedValue } from "../src/canonical-authoring";
import {
  createLadStateStoragePlan,
  LAD_STATE_DB_ROLE,
} from "../src/lad-state-storage";
import type {
  WorkbenchObjectView,
  WorkbenchSnapshot,
} from "../src/workbench-types";

const ROOT = identity(1);
const CONTROLLER = identity(2);
const PROGRAM = identity(3);
const DB = identity(4);

describe("LAD instruction state storage planning", () => {
  it("creates the first private state DB and typed timer member", () => {
    const ids = idFactory(100);
    const plan = createLadStateStoragePlan(snapshot(), program(), "ton", ids);
    expect(plan.memberName).toBe("TON_TimerState_1");
    expect(plan.operations).toHaveLength(1);
    expect(plan.operations[0]).toMatchObject({
      displayName: "MainCycle instances",
      kind: "project.create-object",
      objectKind: "data-block",
      parentId: CONTROLLER,
      semanticPayload: {
        dbKind: "GlobalDB",
        educationRole: LAD_STATE_DB_ROLE,
        engineeringNumber: unsignedValue(1),
        ownerProgramId: PROGRAM,
      },
    });
    expect(plan.stateBinding.storage).toEqual({
      dataBlockId: plan.dataBlockId,
      kind: "data-block-member",
      memberId: plan.memberId,
    });
    const operation = plan.operations[0];
    if (operation?.kind !== "project.create-object") throw new Error("create operation expected");
    expect(operation.semanticPayload.members).toEqual([
      recordValue({
        id: plan.memberId,
        name: "TON_TimerState_1",
        order: unsignedValue(0),
        requiredOutput: false,
        retentive: false,
        role: "static",
        type: "TIMERSTATE",
      }),
    ]);
  });

  it("reuses the program state DB and appends a uniquely named member atomically", () => {
    const stateDb = object(DB, "GlobalDB", CONTROLLER, {
      dbKind: "GlobalDB",
      educationRole: LAD_STATE_DB_ROLE,
      engineeringNumber: unsignedValue(7),
      members: [recordValue({
        id: identity(20),
        name: "CTU_CounterState_1",
        order: unsignedValue(0),
        requiredOutput: false,
        retentive: false,
        role: "static",
        type: "COUNTERSTATE",
      })],
      ownerProgramId: PROGRAM,
    });
    const plan = createLadStateStoragePlan(snapshot(stateDb), program(), "ctu", idFactory(200));
    expect(plan.dataBlockId).toBe(DB);
    expect(plan.memberName).toBe("CTU_CounterState_2");
    expect(plan.operations).toHaveLength(1);
    expect(plan.operations[0]).toMatchObject({
      kind: "project.replace-semantic-payload",
      objectId: DB,
    });
    const operation = plan.operations[0];
    if (operation?.kind !== "project.replace-semantic-payload") throw new Error("replace expected");
    expect(operation.semanticPayload.members).toHaveLength(2);
  });

  it("rejects stateless instructions and programs outside a controller", () => {
    expect(() => createLadStateStoragePlan(snapshot(), program(), "add", idFactory(300)))
      .toThrow(/does not require persistent state/u);
    expect(() => createLadStateStoragePlan(snapshot(), { ...program(), parentId: ROOT }, "ton", idFactory(400)))
      .toThrow(/belong directly/u);
  });
});

const snapshot = (stateDb?: WorkbenchObjectView): WorkbenchSnapshot => ({
  buildState: "not-built",
  diagnostics: [],
  dirtyState: "clean",
  documentId: identity(90),
  documentRevision: "1",
  fileGrantId: null,
  lastSavedProjectHash: null,
  objects: Object.fromEntries([
    [ROOT, object(ROOT, "ProjectRoot", null, {})],
    [CONTROLLER, object(CONTROLLER, "Controller", ROOT, {})],
    [PROGRAM, program()],
    ...(stateDb === undefined ? [] : [[stateDb.id, stateDb] as const]),
  ]),
  projectHash: "A".repeat(64),
  projectName: "Lab",
  projectRootId: ROOT,
  runtime: {
    availability: "UNAVAILABLE",
    canBuild: false,
    diagnostics: [],
    reason: "fixture",
    schemaVersion: 1,
    session: null,
    sourceDocumentHash: "hash",
    sourceSemanticFingerprint: "fingerprint",
  },
  semanticRevision: "1",
  undo: { canRedo: false, canUndo: false, redoLabel: null, undoLabel: null },
});

const program = (): WorkbenchObjectView => object(PROGRAM, "OB", CONTROLLER, {
  blockKind: "OB",
  engineeringNumber: unsignedValue(1),
  interface: [],
  language: "LAD",
  obRole: "CyclicMain",
});

const object = (
  id: string,
  kind: WorkbenchObjectView["kind"],
  parentId: string | null,
  semanticPayload: WorkbenchObjectView["semanticPayload"],
): WorkbenchObjectView => ({
  children: [],
  creationOrdinal: "1",
  displayName: kind === "OB" ? "MainCycle" : kind,
  id,
  kind,
  lifecycle: "active",
  objectRevision: "1",
  parentId,
  payloadSchema: kind === "GlobalDB" ? "edu.data-block/1" : "edu.program-block/1",
  presentationPayload: {},
  semanticPayload,
  semanticRevision: "1",
});

function identity(ordinal: number): string {
  return `00000000-0000-4000-8000-${ordinal.toString(16).padStart(12, "0")}`;
}

const idFactory = (start: number): (() => string) => {
  let ordinal = start;
  return () => identity(ordinal++);
};
