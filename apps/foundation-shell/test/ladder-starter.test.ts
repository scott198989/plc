import { describe, expect, it } from "vitest";

import { canonicalRecordFields, interfaceMemberIdentity } from "../src/canonical-authoring";
import { createLadderStarterPlan } from "../src/ladder-starter";

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;

describe("learner ladder starter", () => {
  it("provisions a complete virtual motor lab with real tag bindings", () => {
    const projectRootId = crypto.randomUUID();
    const plan = createLadderStarterPlan(projectRootId);
    const creates = plan.operations.flatMap((operation) =>
      operation.kind === "project.create-object" ? [operation] : []
    );

    expect(creates).toHaveLength(10);
    expect(new Set(creates.map((operation) => operation.objectId)).size).toBe(10);
    expect(creates.every((operation) => UUID.test(operation.objectId))).toBe(true);
    expect(creates[0]?.parentId).toBe(projectRootId);

    const program = creates.find((operation) => operation.objectId === plan.programId);
    expect(program).toBeDefined();
    expect(program?.semanticPayload).toMatchObject({
      blockKind: "OB",
      language: "LAD",
      obRole: "CyclicMain",
    });
    if (program === undefined) {
      throw new Error("Expected the starter MainCycle block.");
    }

    const memberIds = new Map(
      ["Start_PB", "Stop_PB", "Motor_Run"].map((name) => [
        name,
        interfaceMemberIdentity(program.semanticPayload, name),
      ] as const),
    );
    expect([...memberIds.values()].every((value) => typeof value === "string" && UUID.test(value))).toBe(true);

    for (const tagName of ["Start_PB", "Stop_PB", "Motor_Run"] as const) {
      const tag = creates.find((operation) => operation.displayName === tagName && operation.objectKind === "tag");
      expect(tag?.semanticPayload.blockId).toBe(plan.programId);
      expect(tag?.semanticPayload.memberId).toBe(memberIds.get(tagName));
    }

    const graph = canonicalRecordFields(program.semanticPayload.graph);
    const networks = graph !== null && Array.isArray(graph.networks) ? graph.networks : [];
    const firstNetwork = canonicalRecordFields(networks[0]);
    const nodes = firstNetwork !== null && Array.isArray(firstNetwork.nodes)
      ? firstNetwork.nodes.map(canonicalRecordFields).filter((node) => node !== null)
      : [];
    const contact = nodes.find((node) => node.nodeKind === "contact");
    const coil = nodes.find((node) => node.nodeKind === "coil");
    expect(canonicalRecordFields(contact?.operand)?.memberId).toBe(memberIds.get("Start_PB"));
    expect(canonicalRecordFields(coil?.operand)?.memberId).toBe(memberIds.get("Motor_Run"));
  });
});
