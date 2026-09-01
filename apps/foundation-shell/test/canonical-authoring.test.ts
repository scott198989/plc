import { describe, expect, it } from "vitest";

import {
  createArrayTypeExpression,
  createDataBlockPayload,
  createFbdProgramPayload,
  createInterfaceMemberPayload,
  createLadProgramPayload,
  createNamedTypeMemberPayload,
  createNamedTypePayload,
  createSclProgramPayload,
  createStarterLadProgramPayload,
  createTracePayload,
  createWatchPayload,
  interfaceMemberIdentity,
  updateGraphNodeFields,
} from "../src/canonical-authoring";

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;

describe("canonical authoring payloads", () => {
  it("creates stable, independent block-interface identities at commit time", () => {
    const first = createSclProgramPayload("cyclic-ob");
    const second = createSclProgramPayload("cyclic-ob");
    expect(first.blockKind).toBe("OB");
    expect(first.sourceText).toBe("");
    const firstMembers = first.interface;
    const secondMembers = second.interface;
    expect(Array.isArray(firstMembers)).toBe(true);
    expect(Array.isArray(secondMembers)).toBe(true);
    if (!Array.isArray(firstMembers) || !Array.isArray(secondMembers)) {
      throw new Error("Expected canonical interface lists.");
    }
    const firstIds = firstMembers.map(memberId);
    const secondIds = secondMembers.map(memberId);
    expect(firstIds).toHaveLength(3);
    expect(new Set(firstIds).size).toBe(3);
    expect(firstIds.every((id) => UUID.test(id))).toBe(true);
    expect(secondIds).not.toEqual(firstIds);
    expect(interfaceMemberIdentity(first, "InputValue")).toBe(firstIds[0]);
    expect(interfaceMemberIdentity(first, "Missing")).toBeNull();
  });

  it("pins DB and trace limits into typed canonical values", () => {
    expect(createDataBlockPayload("GlobalDB")).toMatchObject({
      dbKind: "GlobalDB",
      engineeringNumber: { $type: "u64", value: "1" },
    });
    expect(createTracePayload()).toEqual({
      channels: [],
      everyScans: { $type: "u64", value: "1" },
      maximumDurationMs: { $type: "u64", value: "60000" },
      postSamples: { $type: "u64", value: "32" },
      preSamples: { $type: "u64", value: "0" },
      state: "idle",
      trigger: "immediate",
    });
  });

  it("creates usable named, watch, and trace payloads from canonical tag identities", () => {
    const firstTag = "10000000-0000-4000-8000-000000000001";
    const secondTag = "20000000-0000-4000-8000-000000000001";
    const namedMembers = list(createNamedTypePayload().members).map(record);
    expect(namedMembers).toHaveLength(1);
    expect(namedMembers[0]).toMatchObject({
      declaredOrder: { $type: "u64", value: "0" },
      name: "Ready",
      typeId: "BOOL",
    });
    expect(namedMembers[0]?.id).toMatch(UUID);

    const watchRows = list(createWatchPayload([firstTag, secondTag]).rows).map(record);
    expect(watchRows.map((row) => row.targetTag)).toEqual([firstTag, secondTag]);
    expect(watchRows.map((row) => row.order)).toEqual([
      { $type: "u64", value: "0" },
      { $type: "u64", value: "1" },
    ]);

    const traceChannels = list(createTracePayload([secondTag]).channels).map(record);
    expect(traceChannels).toHaveLength(1);
    expect(traceChannels[0]).toMatchObject({ alias: "Channel 1", layer: "effective", targetTag: secondTag });
    expect(traceChannels[0]?.id).toMatch(UUID);
  });

  it("authors stable scalar and bounded-array member records for the functional type editor", () => {
    const interfaceMember = record(createInterfaceMemberPayload("Velocity", "static", 3, "LREAL"));
    expect(interfaceMember).toMatchObject({
      name: "Velocity",
      order: { $type: "u64", value: "3" },
      requiredOutput: false,
      retentive: false,
      role: "static",
      type: "LREAL",
    });
    expect(interfaceMember.id).toMatch(UUID);

    const array = createArrayTypeExpression("UINT", -2, 12);
    const namedMember = record(createNamedTypeMemberPayload("Samples", 1, array));
    expect(namedMember).toMatchObject({
      declaredOrder: { $type: "u64", value: "1" },
      name: "Samples",
      typeId: {
        $type: "record",
        value: {
          dimensions: [{
            $type: "record",
            value: {
              lower: { $type: "i64", value: "-2" },
              upper: { $type: "i64", value: "12" },
            },
          }],
          elementType: "UINT",
          kind: "array",
        },
      },
    });
    expect(namedMember.id).toMatch(UUID);
  });

  it("authors a coordinate-free LAD rung with stable semantic references", () => {
    const payload = createLadProgramPayload();
    expect(payload).toMatchObject({ blockKind: "OB", language: "LAD", obRole: "CyclicMain" });
    const graph = record(payload.graph);
    expect(graph.schema).toBe("edu.lad-semantic-graph/1");
    expect(JSON.stringify(graph)).not.toMatch(/\b(?:x|y|width|height|route|viewport)\b/u);

    const network = record(list(graph.networks)[0]);
    const nodes = list(network.nodes).map(record);
    const edges = list(network.edges).map(record);
    expect(nodes.map((node) => node.nodeKind)).toEqual(["power-source", "contact", "coil"]);
    expect(edges).toHaveLength(2);
    const ports = new Set(
      nodes.flatMap((node) => list(node.powerPorts).map((port) => String(record(port).id))),
    );
    expect(edges.every((edge) => ports.has(String(edge.sourcePortId)) && ports.has(String(edge.targetPortId)))).toBe(true);
    expect(record(nodes[1]?.operand).memberId).toBe(interfaceMemberIdentity(payload, "InputValue"));
    expect(record(nodes[2]?.operand).memberId).toBe(interfaceMemberIdentity(payload, "OutputValue"));
  });

  it("authors a learner motor-starter rung with three stable BOOL variables", () => {
    const payload = createStarterLadProgramPayload();
    const members = list(payload.interface).map(record);
    expect(members.map((member) => member.name)).toEqual(["Start_PB", "Stop_PB", "Motor_Run"]);
    expect(members.map((member) => member.type)).toEqual(["BOOL", "BOOL", "BOOL"]);
    expect(members.map((member) => member.role)).toEqual(["temp", "temp", "temp"]);
    expect(new Set(members.map((member) => member.id)).size).toBe(3);

    const network = record(list(record(payload.graph).networks)[0]);
    const nodes = list(network.nodes).map(record);
    expect(record(nodes[1]?.operand).memberId).toBe(interfaceMemberIdentity(payload, "Start_PB"));
    expect(record(nodes[2]?.operand).memberId).toBe(interfaceMemberIdentity(payload, "Motor_Run"));
    expect(interfaceMemberIdentity(payload, "Stop_PB")).toMatch(UUID);
  });

  it("authors a typed FBD data-flow graph through the shared NOT instruction", () => {
    const payload = createFbdProgramPayload();
    expect(payload).toMatchObject({ blockKind: "FC", language: "FBD" });
    const graph = record(payload.graph);
    expect(graph.schema).toBe("edu.fbd-semantic-graph/1");
    expect(JSON.stringify(graph)).not.toMatch(/\b(?:x|y|width|height|route|viewport)\b/u);

    const network = record(list(graph.networks)[0]);
    const nodes = list(network.nodes).map(record);
    expect(nodes.map((node) => node.nodeKind)).toEqual([
      "load-member",
      "instruction",
      "store-member",
    ]);
    expect(nodes[1]?.instructionCode).toEqual({ $type: "u64", value: "16" });
    expect(record(list(nodes[1]?.ports)[0]).formalId).toEqual({ $type: "u64", value: "16" });
    expect(record(list(nodes[1]?.ports)[1]).formalId).toEqual({ $type: "u64", value: "17" });
    expect(list(network.connections)).toHaveLength(2);
  });

  it("edits one graphical identity without changing topology identities", () => {
    const payload = createLadProgramPayload();
    const graph = payload.graph;
    if (graph === undefined) {
      throw new Error("Expected a graph payload.");
    }
    const before = record(graph);
    const network = record(list(before.networks)[0]);
    const nodes = list(network.nodes).map(record);
    const contact = nodes.find((node) => node.nodeKind === "contact");
    if (contact === undefined || typeof contact.id !== "string") {
      throw new Error("Expected a stable contact identity.");
    }
    const identitiesBefore = collectIdentities(before);
    const updated = updateGraphNodeFields(graph, contact.id, { mode: "normally-closed" });
    expect(updated).not.toBeNull();
    if (updated === null) {
      throw new Error("Expected a graph update.");
    }
    const after = record(updated);
    const updatedNodes = list(record(list(after.networks)[0]).nodes).map(record);
    expect(updatedNodes.find((node) => node.id === contact.id)?.mode).toBe("normally-closed");
    expect(collectIdentities(after)).toEqual(identitiesBefore);
    expect(updateGraphNodeFields(graph, crypto.randomUUID(), { mode: "normally-closed" })).toBeNull();
  });

  it("authors real LAD calls to compatible FBD and SCL functions", () => {
    const fbd = createFbdProgramPayload();
    const scl = createSclProgramPayload("fc");
    const fbdBlockId = crypto.randomUUID();
    const sclBlockId = crypto.randomUUID();
    const payload = createLadProgramPayload(1, [
      {
        inputFormalId: requiredMember(fbd, "InputValue"),
        outputFormalId: requiredMember(fbd, "Result"),
        resultName: "FbdResult",
        targetBlockId: fbdBlockId,
      },
      {
        inputFormalId: requiredMember(scl, "InputValue"),
        outputFormalId: requiredMember(scl, "Result"),
        resultName: "SclResult",
        targetBlockId: sclBlockId,
      },
    ]);
    const graph = record(payload.graph);
    const network = record(list(graph.networks)[0]);
    const nodes = list(network.nodes).map(record);
    expect(nodes.map((node) => node.nodeKind)).toEqual([
      "power-source",
      "call",
      "call",
      "contact",
      "contact",
      "coil",
    ]);
    const calls = nodes.filter((node) => node.nodeKind === "call");
    expect(calls.map((node) => node.targetBlockId)).toEqual([fbdBlockId, sclBlockId]);
    expect(calls.map((node) => node.instructionCode)).toEqual([
      { $type: "u64", value: "512" },
      { $type: "u64", value: "512" },
    ]);
    expect(calls.every((node) => list(node.pins).length === 2)).toBe(true);
    expect(interfaceMemberIdentity(payload, "FbdResult")).toMatch(UUID);
    expect(interfaceMemberIdentity(payload, "SclResult")).toMatch(UUID);
    expect(list(network.edges)).toHaveLength(5);
  });
});

const requiredMember = (payload: Parameters<typeof interfaceMemberIdentity>[0], name: string): string => {
  const identity = interfaceMemberIdentity(payload, name);
  if (identity === null) {
    throw new Error(`Expected interface member ${name}.`);
  }
  return identity;
};

const collectIdentities = (value: unknown): readonly string[] => {
  const identities: string[] = [];
  const visit = (current: unknown): void => {
    if (Array.isArray(current)) {
      current.forEach(visit);
      return;
    }
    if (typeof current !== "object" || current === null) {
      return;
    }
    for (const [key, child] of Object.entries(current)) {
      if ((key === "id" || key.endsWith("Id")) && typeof child === "string" && UUID.test(child)) {
        identities.push(`${key}:${child}`);
      }
      visit(child);
    }
  };
  visit(value);
  return identities.sort();
};

const record = (value: unknown): Record<string, unknown> => {
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
    throw new Error("Expected a canonical record value.");
  }
  return value.value as Record<string, unknown>;
};

const list = (value: unknown): readonly unknown[] => {
  if (!Array.isArray(value)) {
    throw new Error("Expected a canonical list value.");
  }
  return value;
};

const memberId = (value: unknown): string => {
  if (
    typeof value !== "object" ||
    value === null ||
    !("value" in value) ||
    typeof value.value !== "object" ||
    value.value === null ||
    !("id" in value.value) ||
    typeof value.value.id !== "string"
  ) {
    throw new Error("Expected a canonical interface member record.");
  }
  return value.value.id;
};
