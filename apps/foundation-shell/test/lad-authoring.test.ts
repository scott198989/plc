import { describe, expect, it } from "vitest";

import {
  createLadProgramPayload,
  interfaceMemberIdentity,
} from "../src/canonical-authoring";
import {
  addLadNetwork,
  insertSeriesContact,
  removeContactAndReconnect,
  removeLadNetwork,
  updateLadCoil,
  updateLadContact,
  wrapContactWithParallelContact,
} from "../src/lad-authoring";
import type { LadAuthoringResult } from "../src/lad-authoring";
import type { ProjectPayloadValue } from "../src/workbench-types";

describe("LAD authoring graph transforms", () => {
  it("inserts and removes a series contact without mutating the source graph", () => {
    const fixture = linearFixture();
    const sourceBefore = JSON.stringify(fixture.graph);
    const coilInput = portByDirection(fixture.coil, "input");
    const edgeBeforeCoil = fixture.edges.find((edge) => edge.targetPortId === coilInput.id);
    if (edgeBeforeCoil === undefined) {
      throw new Error("Expected the linear edge before the coil.");
    }

    const inserted = success(insertSeriesContact(fixture.graph, {
      edgeId: text(edgeBeforeCoil.id),
      idFactory: deterministicIds(1),
      memberId: fixture.inputMemberId,
      mode: "normally-closed",
      networkId: fixture.networkId,
    }));
    expect(JSON.stringify(fixture.graph)).toBe(sourceBefore);
    expect(inserted.createdIds).toHaveLength(6);
    expect(semanticRevision(inserted.graph)).toBe("1");

    const insertedNetwork = firstNetwork(inserted.graph);
    const insertedNodes = records(insertedNetwork.nodes);
    const insertedEdges = records(insertedNetwork.edges);
    expect(insertedNodes.map((node) => node.nodeKind)).toEqual([
      "power-source",
      "contact",
      "contact",
      "coil",
    ]);
    expect(insertedNodes.map((node) => node.semanticOrder)).toEqual([
      unsigned(0),
      unsigned(1),
      unsigned(2),
      unsigned(3),
    ]);
    expect(insertedEdges).toHaveLength(3);
    expect(insertedEdges.some((edge) => edge.id === edgeBeforeCoil.id)).toBe(false);

    const newContactId = inserted.createdIds[0];
    const newContact = insertedNodes.find((node) => node.id === newContactId);
    expect(newContact).toMatchObject({ mode: "normally-closed", nodeKind: "contact" });
    expect(record(newContact?.operand).memberId).toBe(fixture.inputMemberId);

    const removed = success(removeContactAndReconnect(inserted.graph, {
      contactNodeId: newContactId ?? "",
      idFactory: deterministicIds(100),
      networkId: fixture.networkId,
    }));
    expect(semanticRevision(removed.graph)).toBe("2");
    const removedNetwork = firstNetwork(removed.graph);
    expect(records(removedNetwork.nodes).map((node) => node.nodeKind)).toEqual([
      "power-source",
      "contact",
      "coil",
    ]);
    expect(records(removedNetwork.edges)).toHaveLength(2);
    expect(records(removedNetwork.branches)).toEqual([]);
  });

  it("wraps one contact in a canonical two-path parallel branch", () => {
    const fixture = linearFixture();
    const selectedContactId = text(fixture.contact.id);
    const wrapped = success(wrapContactWithParallelContact(fixture.graph, {
      contactNodeId: selectedContactId,
      idFactory: deterministicIds(200),
      memberId: fixture.outputMemberId,
      networkId: fixture.networkId,
    }));
    expect(semanticRevision(wrapped.graph)).toBe("1");
    const network = firstNetwork(wrapped.graph);
    const nodes = records(network.nodes);
    const edges = records(network.edges);
    const branches = records(network.branches);

    expect(nodes.map((node) => node.nodeKind)).toEqual([
      "power-source",
      "branch-split",
      "contact",
      "contact",
      "branch-join",
      "coil",
    ]);
    expect(nodes.filter((node) => node.id === selectedContactId)).toHaveLength(1);
    expect(edges).toHaveLength(6);
    expect(branches).toHaveLength(1);

    const branch = branches[0];
    const paths = records(branch?.paths);
    expect(paths).toHaveLength(2);
    expect(new Set(paths.flatMap((path) => [path.entryEdgeId, path.exitEdgeId])).size).toBe(4);
    const edgeById = new Map(edges.map((edge) => [edge.id, edge] as const));
    const split = nodes.find((node) => node.nodeKind === "branch-split");
    const join = nodes.find((node) => node.nodeKind === "branch-join");
    const splitOutputs = ports(split).filter((port) => port.direction === "output");
    const joinInputs = ports(join).filter((port) => port.direction === "input");
    expect(paths.every((path) =>
      splitOutputs.some((port) => edgeById.get(path.entryEdgeId)?.sourcePortId === port.id) &&
      joinInputs.some((port) => edgeById.get(path.exitEdgeId)?.targetPortId === port.id)
    )).toBe(true);

    const parallelContact = nodes.find((node) =>
      node.nodeKind === "contact" && node.id !== selectedContactId
    );
    expect(record(parallelContact?.operand).memberId).toBe(fixture.outputMemberId);
    const unsafeRemoval = removeContactAndReconnect(wrapped.graph, {
      contactNodeId: text(parallelContact?.id),
      idFactory: deterministicIds(300),
      networkId: fixture.networkId,
    });
    expect(unsafeRemoval).toMatchObject({ code: "would-empty-branch-path", ok: false });
  });

  it("adds and removes a valid source-to-coil LAD network", () => {
    const fixture = linearFixture();
    const added = success(addLadNetwork(fixture.graph, {
      coilMemberId: fixture.outputMemberId,
      idFactory: deterministicIds(400),
    }));
    expect(semanticRevision(added.graph)).toBe("1");
    const graph = record(added.graph);
    const networks = records(graph.networks);
    expect(networks).toHaveLength(2);
    expect(networks.map((network) => network.semanticOrder)).toEqual([unsigned(0), unsigned(1)]);
    expect(records(networks[1]?.nodes).map((node) => node.nodeKind)).toEqual([
      "power-source",
      "coil",
    ]);
    expect(records(networks[1]?.edges)).toHaveLength(1);
    expect(records(networks[1]?.branches)).toEqual([]);

    const addedNetworkId = added.createdIds[0];
    const removed = success(removeLadNetwork(added.graph, { networkId: addedNetworkId ?? "" }));
    expect(semanticRevision(removed.graph)).toBe("2");
    expect(records(record(removed.graph).networks)).toHaveLength(1);
    expect(removeLadNetwork(removed.graph, { networkId: fixture.networkId })).toMatchObject({
      code: "last-network",
      ok: false,
    });
  });

  it("updates contact and coil bindings immutably while preserving operand identities", () => {
    const fixture = linearFixture();
    const sourceBefore = JSON.stringify(fixture.graph);
    const contactOperandId = text(record(fixture.contact.operand).id);
    const coilOperandId = text(record(fixture.coil.operand).id);

    const updatedContact = success(updateLadContact(fixture.graph, {
      contactNodeId: text(fixture.contact.id),
      memberId: fixture.outputMemberId,
      mode: "normally-closed",
      networkId: fixture.networkId,
    }));
    expect(semanticRevision(updatedContact.graph)).toBe("1");
    const contact = records(firstNetwork(updatedContact.graph).nodes)
      .find((node) => node.id === fixture.contact.id);
    expect(contact?.mode).toBe("normally-closed");
    expect(record(contact?.operand)).toMatchObject({
      id: contactOperandId,
      kind: "caller-member",
      memberId: fixture.outputMemberId,
    });

    const updatedCoil = success(updateLadCoil(updatedContact.graph, {
      coilNodeId: text(fixture.coil.id),
      memberId: fixture.inputMemberId,
      mode: "set",
      networkId: fixture.networkId,
    }));
    expect(semanticRevision(updatedCoil.graph)).toBe("2");
    const coil = records(firstNetwork(updatedCoil.graph).nodes)
      .find((node) => node.id === fixture.coil.id);
    expect(coil?.mode).toBe("set");
    expect(record(coil?.operand)).toMatchObject({
      id: coilOperandId,
      kind: "caller-member",
      memberId: fixture.inputMemberId,
    });
    expect(JSON.stringify(fixture.graph)).toBe(sourceBefore);
  });

  it("fails closed when a canonical identity factory cannot provide a usable identity", () => {
    const fixture = linearFixture();
    const edge = fixture.edges[0];
    if (edge === undefined) {
      throw new Error("Expected a linear LAD edge.");
    }
    const result = insertSeriesContact(fixture.graph, {
      edgeId: text(edge.id),
      idFactory: () => "not-a-uuid",
      memberId: fixture.inputMemberId,
      networkId: fixture.networkId,
    });
    expect(result).toMatchObject({ code: "id-exhausted", ok: false });
    expect(semanticRevision(fixture.graph)).toBe("0");
  });

  it("increments semanticRevision exactly once for every successful mutation", () => {
    const fixture = linearFixture();
    expect(semanticRevision(fixture.graph)).toBe("0");

    const updatedContact = success(updateLadContact(fixture.graph, {
      contactNodeId: text(fixture.contact.id),
      mode: "normally-closed",
      networkId: fixture.networkId,
    }));
    expect(semanticRevision(updatedContact.graph)).toBe("1");

    const updatedCoil = success(updateLadCoil(updatedContact.graph, {
      coilNodeId: text(fixture.coil.id),
      mode: "negated",
      networkId: fixture.networkId,
    }));
    expect(semanticRevision(updatedCoil.graph)).toBe("2");

    const edgeBeforeCoil = records(firstNetwork(updatedCoil.graph).edges)
      .find((edge) => edge.targetPortId === portByDirection(fixture.coil, "input").id);
    if (edgeBeforeCoil === undefined) {
      throw new Error("Expected the linear edge before the coil.");
    }
    const inserted = success(insertSeriesContact(updatedCoil.graph, {
      edgeId: text(edgeBeforeCoil.id),
      idFactory: deterministicIds(500),
      memberId: fixture.inputMemberId,
      networkId: fixture.networkId,
    }));
    expect(semanticRevision(inserted.graph)).toBe("3");

    const insertedContactId = inserted.createdIds[0];
    if (insertedContactId === undefined) {
      throw new Error("Expected the inserted contact identity.");
    }
    const removedContact = success(removeContactAndReconnect(inserted.graph, {
      contactNodeId: insertedContactId,
      idFactory: deterministicIds(600),
      networkId: fixture.networkId,
    }));
    expect(semanticRevision(removedContact.graph)).toBe("4");

    const addedNetwork = success(addLadNetwork(removedContact.graph, {
      coilMemberId: fixture.outputMemberId,
      idFactory: deterministicIds(700),
    }));
    expect(semanticRevision(addedNetwork.graph)).toBe("5");

    const addedNetworkId = addedNetwork.createdIds[0];
    if (addedNetworkId === undefined) {
      throw new Error("Expected the added network identity.");
    }
    const removedNetwork = success(removeLadNetwork(addedNetwork.graph, {
      networkId: addedNetworkId,
    }));
    expect(semanticRevision(removedNetwork.graph)).toBe("6");
  });
});

const linearFixture = (): Readonly<{
  coil: Record<string, unknown>;
  contact: Record<string, unknown>;
  edges: readonly Record<string, unknown>[];
  graph: ProjectPayloadValue;
  inputMemberId: string;
  networkId: string;
  outputMemberId: string;
}> => {
  const payload = createLadProgramPayload();
  if (payload.graph === undefined) {
    throw new Error("Expected a canonical LAD graph.");
  }
  const network = firstNetwork(payload.graph);
  const nodes = records(network.nodes);
  const contact = nodes.find((node) => node.nodeKind === "contact");
  const coil = nodes.find((node) => node.nodeKind === "coil");
  const inputMemberId = interfaceMemberIdentity(payload, "InputValue");
  const outputMemberId = interfaceMemberIdentity(payload, "OutputValue");
  if (
    contact === undefined ||
    coil === undefined ||
    inputMemberId === null ||
    outputMemberId === null
  ) {
    throw new Error("Expected a complete linear LAD fixture.");
  }
  return {
    coil,
    contact,
    edges: records(network.edges),
    graph: payload.graph,
    inputMemberId,
    networkId: text(network.id),
    outputMemberId,
  };
};

const success = (
  result: LadAuthoringResult,
): Extract<LadAuthoringResult, Readonly<{ ok: true }>> => {
  if (result.ok === false) {
    throw new Error(`${result.code}: ${result.message}`);
  }
  return result;
};

const deterministicIds = (start: number): (() => string) => {
  let value = start;
  return () => {
    const suffix = value.toString(16).padStart(12, "0");
    value += 1;
    return `f0000000-0000-4000-8000-${suffix}`;
  };
};

const firstNetwork = (graph: ProjectPayloadValue): Record<string, unknown> => {
  const networks = records(record(graph).networks);
  const network = networks[0];
  if (network === undefined) {
    throw new Error("Expected one canonical LAD network.");
  }
  return network;
};

const ports = (node: Record<string, unknown> | undefined): readonly Record<string, unknown>[] =>
  records(node?.powerPorts);

const portByDirection = (
  node: Record<string, unknown>,
  direction: "input" | "output",
): Record<string, unknown> => {
  const port = ports(node).find((candidate) => candidate.direction === direction);
  if (port === undefined) {
    throw new Error(`Expected one ${direction} LAD port.`);
  }
  return port;
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

const unsigned = (value: number): Readonly<{ $type: "u64"; value: string }> => ({
  $type: "u64",
  value: value.toString(10),
});

const semanticRevision = (graph: ProjectPayloadValue): string => {
  const value = record(graph).semanticRevision;
  if (
    typeof value !== "object" ||
    value === null ||
    !("$type" in value) ||
    value.$type !== "u64" ||
    !("value" in value) ||
    typeof value.value !== "string"
  ) {
    throw new Error("Expected a canonical unsigned semantic revision.");
  }
  return value.value;
};
