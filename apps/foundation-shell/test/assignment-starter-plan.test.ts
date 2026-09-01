import { describe, expect, it } from "vitest";

import {
  AssignmentStarterPlanError,
  createAssignmentStarterPlan,
} from "../src/assignment-starter-plan";
import { canonicalRecordFields } from "../src/canonical-authoring";
import {
  BUILT_IN_MOTOR_STARTER_ASSIGNMENT,
  type AssignmentDocumentV1,
} from "../src/education-contract";
import type { WorkbenchObjectView, WorkbenchSnapshot } from "../src/workbench-types";

describe("assignment starter planning", () => {
  it("creates required hardware and real tag-to-MainCycle bindings from the first supported PLC", () => {
    const source = assignment({
      plcCatalogIds: ["future-controller", "vctrl-m1", "vctrl-c1"],
      requiredModules: [
        { catalogId: "vdi16", quantity: 2 },
        { catalogId: "vdo16", quantity: 1 },
      ],
      starterTags: [
        { address: "%I0.0", dataType: "BOOL", description: "Start", name: "Start_PB" },
        { address: "%Q1.7", dataType: "BOOL", description: "Run", name: "Motor_Run" },
        { address: null, dataType: "DINT", description: "Count", name: "Part_Count" },
        { address: null, dataType: "TIME", description: "Delay", name: "Start_Delay" },
        { address: null, dataType: "BOOL", description: "Latch", name: "Cycle_Latched" },
      ],
    });
    const plan = createAssignmentStarterPlan(emptySnapshot(), source, sequentialIds());
    const creates = createOperations(plan.operations);

    expect(plan.catalogId).toBe("vctrl-m1");
    expect(creates.find((operation) => operation.objectId === plan.controllerId)?.semanticPayload.catalogId)
      .toBe("vctrl-m1");
    expect(creates.find((operation) => operation.semanticPayload.catalogId === "vpwr1")?.semanticPayload.slot)
      .toEqual({ $type: "u64", value: "0" });

    const lessonModules = creates.filter((operation) =>
      operation.semanticPayload.catalogId === "vdi16" || operation.semanticPayload.catalogId === "vdo16"
    );
    expect(lessonModules.map((operation) => operation.semanticPayload.slot)).toEqual([
      { $type: "u64", value: "2" },
      { $type: "u64", value: "3" },
      { $type: "u64", value: "4" },
    ]);
    expect(lessonModules.every((operation) => operation.parentId === plan.rackId)).toBe(true);

    const program = creates.find((operation) => operation.objectId === plan.programId);
    const tags = creates.filter((operation) => operation.objectKind === "tag");
    const members = Array.isArray(program?.semanticPayload.interface)
      ? program.semanticPayload.interface.map(canonicalRecordFields).filter((member) => member !== null)
      : [];
    for (const tag of tags) {
      const member = members.find((candidate) => candidate.id === tag.semanticPayload.memberId);
      expect(member).toMatchObject({ name: tag.displayName, role: "temp", type: tag.semanticPayload.dataType });
      expect(tag.semanticPayload.blockId).toBe(plan.programId);
      expect(tag.parentId).toBe(plan.symbolTableId);
    }

    expect(tags.find((tag) => tag.displayName === "Start_PB")?.semanticPayload).toMatchObject({
      addressArea: "I",
      addressIntent: "explicit",
      bitOffset: { $type: "u64", value: "0" },
      byteOffset: { $type: "u64", value: "0" },
      tagKind: "Input",
    });
    expect(tags.find((tag) => tag.displayName === "Motor_Run")?.semanticPayload).toMatchObject({
      addressArea: "Q",
      addressIntent: "explicit",
      bitOffset: { $type: "u64", value: "7" },
      byteOffset: { $type: "u64", value: "1" },
      tagKind: "Output",
    });
    for (const name of ["Part_Count", "Start_Delay", "Cycle_Latched"]) {
      expect(tags.find((tag) => tag.displayName === name)?.semanticPayload).toMatchObject({
        addressArea: "M",
        addressIntent: "auto",
        tagKind: "Memory",
      });
    }

    const graphOwnedIds = [
      ...creates.map((operation) => operation.objectId),
      ...tags.map((tag) => String(tag.semanticPayload.memberId)),
    ];
    expect(new Set(graphOwnedIds).size).toBe(graphOwnedIds.length);
  });

  it("supports a blank assignment starter and selects the first supported catalog in assignment order", () => {
    const source = {
      ...assignment({
        plcCatalogIds: ["vctrl-p1", "vctrl-c1"],
        requiredModules: [{ catalogId: "vdi16", quantity: 1 }],
        starterTags: [],
      }),
      starterProject: { kind: "blank" as const },
    };
    const plan = createAssignmentStarterPlan(emptySnapshot(), source, sequentialIds());

    expect(plan.catalogId).toBe("vctrl-p1");
    expect(createOperations(plan.operations).some((operation) => operation.objectId === plan.programId)).toBe(true);
    expect(createOperations(plan.operations).some((operation) => operation.objectId === plan.symbolTableId)).toBe(true);
  });

  it.each([
    {
      code: "unsupported-template",
      source: {
        ...assignment(),
        starterProject: { kind: "built-in-template" as const, templateId: "builtin.future/1" },
      },
    },
    {
      code: "embedded-project",
      source: {
        ...assignment(),
        starterProject: {
          artifact: { fileName: "starter.vlabproj", packageBase64: "e30=", sha256Hex: "A".repeat(64) },
          kind: "embedded-project" as const,
        },
      },
    },
    {
      code: "unsupported-controller",
      source: assignment({ plcCatalogIds: ["future-controller"] }),
    },
    {
      code: "unsupported-module",
      source: assignment({ requiredModules: [{ catalogId: "future-module", quantity: 1 }] }),
    },
    {
      code: "module-capacity",
      source: assignment({ requiredModules: [{ catalogId: "vdi16", quantity: 9 }] }),
    },
  ])("rejects unsupported starter configuration with $code", ({ code, source }) => {
    expectPlanError(() => createAssignmentStarterPlan(emptySnapshot(), source, sequentialIds()), code);
  });

  it.each([
    {
      tag: { address: "%I0.0", dataType: "DINT" as const, description: "Bad I/O type", name: "Counter" },
      text: "Only BOOL",
    },
    {
      tag: { address: "%I1024.0", dataType: "BOOL" as const, description: "Out of range", name: "Far_Input" },
      text: "outside",
    },
    {
      tag: { address: "%M0.0", dataType: "BOOL" as const, description: "Explicit memory", name: "Memory" },
      text: "input or output bits",
    },
  ])("rejects a starter tag that cannot be represented canonically: $text", ({ tag, text }) => {
    const source = assignment({ starterTags: [tag] });
    expect(() => createAssignmentStarterPlan(emptySnapshot(), source, sequentialIds())).toThrow(text);
    expectPlanError(
      () => createAssignmentStarterPlan(emptySnapshot(), source, sequentialIds()),
      "invalid-starter-tag",
    );
  });

  it("rejects a non-empty project and a reused identity before returning partial work", () => {
    expectPlanError(
      () => createAssignmentStarterPlan(nonEmptySnapshot(), assignment(), sequentialIds()),
      "project-not-empty",
    );
    const repeated = () => "00000000-0000-4000-8000-000000000010";
    expectPlanError(
      () => createAssignmentStarterPlan(emptySnapshot(), assignment(), repeated),
      "duplicate-identity",
    );
  });
});

type Requirements = AssignmentDocumentV1["requirements"];

const assignment = (requirements: Partial<Requirements> = {}): AssignmentDocumentV1 => ({
  ...BUILT_IN_MOTOR_STARTER_ASSIGNMENT,
  requirements: {
    ...BUILT_IN_MOTOR_STARTER_ASSIGNMENT.requirements,
    ...requirements,
  },
});

const ROOT_ID = "00000000-0000-4000-8000-000000000001";

const emptySnapshot = (): WorkbenchSnapshot => snapshot([object(ROOT_ID, "ProjectRoot", null)]);
const nonEmptySnapshot = (): WorkbenchSnapshot => snapshot([
  object(ROOT_ID, "ProjectRoot", null),
  object("00000000-0000-4000-8000-000000000002", "VirtualNetwork", ROOT_ID),
]);

const snapshot = (items: readonly WorkbenchObjectView[]): WorkbenchSnapshot => ({
  buildState: "not-built",
  diagnostics: [],
  dirtyState: "clean",
  documentId: "00000000-0000-4000-8000-000000000003",
  documentRevision: "0",
  fileGrantId: null,
  lastSavedProjectHash: null,
  objects: Object.fromEntries(items.map((item) => [item.id, item])),
  projectHash: "hash",
  projectName: "Assignment project",
  projectRootId: ROOT_ID,
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
  semanticRevision: "0",
  undo: { canRedo: false, canUndo: false, redoLabel: null, undoLabel: null },
});

const object = (
  id: string,
  kind: WorkbenchObjectView["kind"],
  parentId: string | null,
): WorkbenchObjectView => ({
  children: [],
  creationOrdinal: "1",
  displayName: kind,
  id,
  kind,
  lifecycle: "active",
  objectRevision: "0",
  parentId,
  payloadSchema: "fixture/1",
  presentationPayload: {},
  semanticPayload: {},
  semanticRevision: "0",
});

const sequentialIds = (): (() => string) => {
  let next = 10;
  return () => `00000000-0000-4000-8000-${String(next++).padStart(12, "0")}`;
};

const createOperations = (operations: readonly import("../src/workbench-types").WorkbenchOperation[]) =>
  operations.flatMap((operation) => operation.kind === "project.create-object" ? [operation] : []);

const expectPlanError = (action: () => unknown, code: string): void => {
  try {
    action();
    throw new Error("Expected starter planning to fail.");
  } catch (error) {
    expect(error).toBeInstanceOf(AssignmentStarterPlanError);
    expect((error as AssignmentStarterPlanError).code).toBe(code);
  }
};
