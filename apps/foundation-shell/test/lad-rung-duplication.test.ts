import { describe, expect, it } from "vitest";

import {
  createLadProgramPayload,
  recordValue,
  unsignedValue,
} from "../src/canonical-authoring";
import {
  insertMvpLadInstructionBoxOnEdge,
} from "../src/lad-box-authoring";
import type {
  LadBoxAuthoringResult,
} from "../src/lad-box-authoring";
import {
  createSafeLadRungDuplicationPlan,
} from "../src/lad-rung-duplication";
import {
  LAD_STATE_DB_ROLE,
} from "../src/lad-state-storage";
import type {
  MvpLadInstructionKey,
} from "../src/lad-instruction-catalog";
import type {
  ProjectPayload,
  ProjectPayloadValue,
  WorkbenchObjectView,
  WorkbenchSnapshot,
} from "../src/workbench-types";

const ROOT = identity(1);
const CONTROLLER = identity(2);
const PROGRAM = identity(3);
const DB = identity(4);

describe("safe LAD rung duplication planning", () => {
  it("gives a copied edge box a distinct EDGESTATE member before replacing the graph", () => {
    const fixture = statefulFixture(["rising-edge"]);
    const plan = success(createSafeLadRungDuplicationPlan(
      fixture.snapshot,
      fixture.program,
      fixture.graph,
      { idFactory: idFactory(5_000), networkId: fixture.networkId },
    ));

    expect(plan.operations.map((operation) => operation.kind)).toEqual([
      "project.replace-semantic-payload",
      "project.set-semantic-field",
    ]);
    expect(plan.stateMemberIds).toHaveLength(1);
    const replacement = plan.operations[0];
    if (replacement?.kind !== "project.replace-semantic-payload") {
      throw new Error("Expected the private state DB replacement first.");
    }
    const members = records(replacement.semanticPayload.members);
    expect(members).toHaveLength(2);
    expect(members[1]).toMatchObject({
      id: plan.stateMemberIds[0],
      name: "R_TRIG_EdgeState_2",
      type: "EDGESTATE",
    });

    const sourceState = boxStates(networkById(fixture.graph, fixture.networkId))[0];
    const copiedState = boxStates(networkById(plan.graph, plan.createdNetworkId))[0];
    expect(copiedState?.invocationId).not.toBe(sourceState?.invocationId);
    expect(copiedState?.memberId).toBe(plan.stateMemberIds[0]);
    expect(copiedState?.memberId).not.toBe(sourceState?.memberId);
    expect(copiedState?.dataBlockId).toBe(DB);
  });

  it("batches distinct typed members for several copied stateful boxes", () => {
    const fixture = statefulFixture(["rising-edge", "ton", "ctu"]);
    const plan = success(createSafeLadRungDuplicationPlan(
      fixture.snapshot,
      fixture.program,
      fixture.graph,
      { idFactory: idFactory(6_000), networkId: fixture.networkId },
    ));

    expect(plan.operations).toHaveLength(2);
    const replacement = plan.operations[0];
    if (replacement?.kind !== "project.replace-semantic-payload") {
      throw new Error("Expected one batched state DB replacement.");
    }
    const members = records(replacement.semanticPayload.members);
    expect(members).toHaveLength(6);
    expect(members.slice(3).map((member) => member.type)).toEqual([
      "EDGESTATE",
      "TIMERSTATE",
      "COUNTERSTATE",
    ]);
    expect(new Set(plan.stateMemberIds).size).toBe(3);

    const sourceStates = boxStates(networkById(fixture.graph, fixture.networkId));
    const copiedStates = boxStates(networkById(plan.graph, plan.createdNetworkId));
    expect(copiedStates).toHaveLength(3);
    expect(copiedStates.map((state) => state.memberId)).toEqual(plan.stateMemberIds);
    expect(copiedStates.every((state) => state.dataBlockId === DB)).toBe(true);
    expect(copiedStates.every((state, index) =>
      state.memberId !== sourceStates[index]?.memberId &&
      state.invocationId !== sourceStates[index]?.invocationId
    )).toBe(true);
  });

  it("duplicates a stateless rung with only the final graph operation", () => {
    const payload = createLadProgramPayload();
    if (payload.graph === undefined) {
      throw new Error("Expected a starter LAD graph.");
    }
    const sourceNetwork = firstNetwork(payload.graph);
    const program = programObject(payload.graph);
    const plan = success(createSafeLadRungDuplicationPlan(
      snapshot(program),
      program,
      payload.graph,
      { idFactory: idFactory(7_000), networkId: text(sourceNetwork.id) },
    ));

    expect(plan.stateMemberIds).toEqual([]);
    expect(plan.operations).toHaveLength(1);
    expect(plan.operations[0]).toMatchObject({
      key: "graph",
      kind: "project.set-semantic-field",
      objectId: PROGRAM,
      value: plan.graph,
    });
    expect(records(record(plan.graph).networks)).toHaveLength(2);
  });
});

const statefulFixture = (
  instructions: readonly Extract<MvpLadInstructionKey, "ctu" | "rising-edge" | "ton">[],
): Readonly<{
  graph: ProjectPayloadValue;
  networkId: string;
  program: WorkbenchObjectView;
  snapshot: WorkbenchSnapshot;
}> => {
  const payload = createLadProgramPayload();
  if (payload.graph === undefined) {
    throw new Error("Expected a starter LAD graph.");
  }
  let graph = payload.graph;
  const networkId = text(firstNetwork(graph).id);
  const sourceMembers: ProjectPayloadValue[] = [];
  instructions.forEach((instruction, index) => {
    const memberId = identity(100 + index);
    const type = instruction === "rising-edge"
      ? "EDGESTATE"
      : instruction === "ton"
        ? "TIMERSTATE"
        : "COUNTERSTATE";
    const suffix = instruction === "rising-edge"
      ? "R_TRIG_EdgeState"
      : instruction === "ton"
        ? "TON_TimerState"
        : "CTU_CounterState";
    sourceMembers.push(recordValue({
      id: memberId,
      name: `${suffix}_${index + 1}`,
      order: unsignedValue(index),
      requiredOutput: false,
      retentive: false,
      role: "static",
      type,
    }));
    const network = firstNetwork(graph);
    const coil = records(network.nodes).find((node) => node.nodeKind === "coil");
    const coilInputId = text(ports(coil).find((port) => port.direction === "input")?.id);
    const edge = records(network.edges).find((candidate) => candidate.targetPortId === coilInputId);
    if (edge === undefined) {
      throw new Error("Expected the insertion edge before the coil.");
    }
    graph = authoringSuccess(insertMvpLadInstructionBoxOnEdge(graph, {
      edgeId: text(edge.id),
      idFactory: idFactory(1_000 + index * 100),
      instruction,
      networkId,
      stateBinding: {
        storage: {
          dataBlockId: DB,
          kind: "data-block-member",
          memberId,
        },
      },
      valueDataType: "DINT",
    })).graph;
  });
  const program = programObject(graph);
  const stateDb = object(DB, "GlobalDB", CONTROLLER, {
    dbKind: "GlobalDB",
    educationRole: LAD_STATE_DB_ROLE,
    engineeringNumber: unsignedValue(7),
    members: sourceMembers,
    ownerProgramId: PROGRAM,
  });
  return { graph, networkId, program, snapshot: snapshot(program, stateDb) };
};

const boxStates = (network: ProjectPayload): readonly Readonly<{
  dataBlockId: string;
  invocationId: string;
  memberId: string;
}>[] => records(network.nodes).flatMap((node) => {
  const state = optionalRecord(node.state);
  const storage = state === null ? null : optionalRecord(state.storage);
  return state === null || storage === null
    ? []
    : [{
        dataBlockId: text(storage.dataBlockId),
        invocationId: text(state.invocationId),
        memberId: text(storage.memberId),
      }];
});

const networkById = (graph: ProjectPayloadValue, networkId: string): ProjectPayload => {
  const network = records(record(graph).networks).find((candidate) => candidate.id === networkId);
  if (network === undefined) {
    throw new Error(`Expected LAD network ${networkId}.`);
  }
  return network as ProjectPayload;
};

const firstNetwork = (graph: ProjectPayloadValue): Record<string, unknown> => {
  const network = records(record(graph).networks)[0];
  if (network === undefined) {
    throw new Error("Expected one LAD network.");
  }
  return network;
};

const ports = (node: Record<string, unknown> | undefined): readonly Record<string, unknown>[] =>
  records(node?.powerPorts);

const authoringSuccess = (
  result: LadBoxAuthoringResult,
): Extract<LadBoxAuthoringResult, Readonly<{ ok: true }>> => {
  if (result.ok === false) {
    throw new Error(`${result.code}: ${result.message}`);
  }
  return result;
};

const success = (
  result: ReturnType<typeof createSafeLadRungDuplicationPlan>,
): Extract<ReturnType<typeof createSafeLadRungDuplicationPlan>, Readonly<{ ok: true }>> => {
  if (result.ok === false) {
    throw new Error(`${result.code}: ${result.message}`);
  }
  return result;
};

const snapshot = (
  program: WorkbenchObjectView,
  stateDb?: WorkbenchObjectView,
): WorkbenchSnapshot => ({
  buildState: "not-built",
  diagnostics: [],
  dirtyState: "clean",
  documentId: identity(900),
  documentRevision: "1",
  fileGrantId: null,
  lastSavedProjectHash: null,
  objects: Object.fromEntries([
    [ROOT, object(ROOT, "ProjectRoot", null, {})],
    [CONTROLLER, object(CONTROLLER, "Controller", ROOT, {})],
    [PROGRAM, program],
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

const programObject = (graph: ProjectPayloadValue): WorkbenchObjectView => object(
  PROGRAM,
  "OB",
  CONTROLLER,
  {
    blockKind: "OB",
    engineeringNumber: unsignedValue(1),
    graph,
    interface: [],
    language: "LAD",
    obRole: "CyclicMain",
  },
);

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

const record = (value: unknown): Record<string, unknown> => {
  const fields = optionalRecord(value);
  if (fields === null) {
    throw new Error("Expected a canonical record value.");
  }
  return fields;
};

const optionalRecord = (value: unknown): Record<string, unknown> | null => {
  if (
    typeof value !== "object" ||
    value === null ||
    !("$type" in value) ||
    value.$type !== "record" ||
    !("value" in value) ||
    typeof value.value !== "object" ||
    value.value === null ||
    Array.isArray(value.value)
  ) {
    return null;
  }
  return value.value as Record<string, unknown>;
};

const records = (value: unknown): readonly Record<string, unknown>[] => {
  if (!Array.isArray(value)) {
    throw new Error("Expected a canonical record list.");
  }
  return value.map(record);
};

const text = (value: unknown): string => {
  if (typeof value !== "string") {
    throw new Error("Expected canonical text.");
  }
  return value;
};

function identity(ordinal: number): string {
  return `00000000-0000-4000-8000-${ordinal.toString(16).padStart(12, "0")}`;
}

const idFactory = (start: number): (() => string) => {
  let ordinal = start;
  return () => identity(ordinal++);
};
