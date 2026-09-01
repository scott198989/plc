import { describe, expect, it } from "vitest";

import { createPlcSetupPlan, virtualPlcCatalog } from "../src/plc-setup";
import type { WorkbenchObjectView, WorkbenchSnapshot } from "../src/workbench-types";

describe("learner PLC setup", () => {
  it("offers a small brand-neutral catalog", () => {
    expect(virtualPlcCatalog.map((option) => option.catalogId)).toEqual([
      "vctrl-c1",
      "vctrl-m1",
      "vctrl-p1",
    ]);
    expect(virtualPlcCatalog.every((option) => option.label.endsWith("PLC"))).toBe(true);
  });

  it("creates a compact PLC workspace without hiding the I/O lesson", () => {
    const ids = idFactory();
    const plan = createPlcSetupPlan(snapshot(), "vctrl-c1", ids.next);
    const creates = plan.operations.flatMap((operation) =>
      operation.kind === "project.create-object" ? [operation] : []
    );

    expect(creates.map((operation) => operation.objectKind)).toEqual([
      "network",
      "controller",
      "rack",
      "program-block",
      "symbol-table",
    ]);
    expect(creates.some((operation) => operation.objectKind === "module")).toBe(false);
    expect(creates.find((operation) => operation.objectId === plan.controllerId)?.semanticPayload.catalogId)
      .toBe("vctrl-c1");
    expect(creates.find((operation) => operation.objectId === plan.programId)?.semanticPayload.language)
      .toBe("LAD");
  });

  it("adds the required power supply for a modular rack", () => {
    const ids = idFactory();
    const plan = createPlcSetupPlan(snapshot(true), "vctrl-m1", ids.next);
    const creates = plan.operations.flatMap((operation) =>
      operation.kind === "project.create-object" ? [operation] : []
    );
    const power = creates.find((operation) => operation.semanticPayload.catalogId === "vpwr1");

    expect(creates.some((operation) => operation.objectKind === "network")).toBe(false);
    expect(power).toMatchObject({ objectKind: "module", parentId: plan.rackId });
    expect(power?.semanticPayload.slot).toEqual({ $type: "u64", value: "0" });
  });
});

const ROOT_ID = "00000000-0000-4000-8000-000000000001";

const snapshot = (withNetwork = false): WorkbenchSnapshot => {
  const root = object(ROOT_ID, "ProjectRoot", null);
  const network = object("00000000-0000-4000-8000-000000000002", "VirtualNetwork", ROOT_ID);
  const objects = withNetwork ? [root, network] : [root];
  return {
    buildState: "not-built",
    diagnostics: [],
    dirtyState: "clean",
    documentId: "00000000-0000-4000-8000-000000000003",
    documentRevision: "0",
    fileGrantId: null,
    lastSavedProjectHash: null,
    objects: Object.fromEntries(objects.map((candidate) => [candidate.id, candidate])),
    projectHash: "hash",
    projectName: "Training project",
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
  };
};

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

const idFactory = (): Readonly<{ next: () => string }> => {
  let value = 10;
  return {
    next: () => `00000000-0000-4000-8000-${String(value++).padStart(12, "0")}`,
  };
};
