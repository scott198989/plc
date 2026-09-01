import { describe, expect, it } from "vitest";

import {
  createLadProgramPayload,
  interfaceMemberIdentity,
  recordValue,
} from "../src/canonical-authoring";
import {
  insertLadBoxOnEdge,
  insertMvpLadInstructionBoxOnEdge,
  removeLadBoxAndReconnect,
  updateLadBoxPinBinding,
} from "../src/lad-box-authoring";
import type {
  LadBoxAuthoringResult,
  LadBoxNodeFactory,
} from "../src/lad-box-authoring";
import { wrapContactWithParallelContact } from "../src/lad-authoring";
import { buildCanonicalLadBoxNode } from "../src/lad-instruction-catalog";
import { projectLadNetworkTopology } from "../src/lad-topology";
import type { ProjectPayloadValue } from "../src/workbench-types";

describe("LAD instruction-box authoring", () => {
  it("inserts a catalog box immutably and allocates only graph-owned identities", () => {
    const fixture = linearFixture();
    const sourceBefore = JSON.stringify(fixture.graph);
    const edge = fixture.edges.find((candidate) =>
      candidate.targetPortId === portByDirection(fixture.coil, "input").id
    );
    if (edge === undefined) {
      throw new Error("Expected the linear edge before the coil.");
    }
    const dataBlockId = id(900);
    const dataBlockMemberId = id(901);
    const inserted = success(insertLadBoxOnEdge(fixture.graph, {
      boxNodeFactory: moveBoxFactory(fixture.inputMemberId, dataBlockId, dataBlockMemberId),
      edgeId: text(edge.id),
      idFactory: deterministicIds(100),
      networkId: fixture.networkId,
    }));

    expect(JSON.stringify(fixture.graph)).toBe(sourceBefore);
    expect(semanticRevision(inserted.graph)).toBe("1");
    expect(new Set(inserted.createdIds).size).toBe(inserted.createdIds.length);
    expect(inserted.createdIds).toHaveLength(9);
    expect(inserted.createdIds).not.toContain(fixture.inputMemberId);
    expect(inserted.createdIds).not.toContain(dataBlockId);
    expect(inserted.createdIds).not.toContain(dataBlockMemberId);

    const network = firstNetwork(inserted.graph);
    const nodes = records(network.nodes);
    expect(nodes.map((node) => node.nodeKind)).toEqual([
      "power-source",
      "contact",
      "box",
      "coil",
    ]);
    expect(nodes.map((node) => node.semanticOrder)).toEqual([
      unsigned(0),
      unsigned(1),
      unsigned(2),
      unsigned(3),
    ]);
    expect(records(network.edges)).toHaveLength(3);
    expect(records(network.edges).some((candidate) => candidate.id === edge.id)).toBe(false);

    const box = nodes.find((node) => node.nodeKind === "box");
    const pins = records(box?.pins);
    expect(record(pins.find((pin) => pin.name === "IN")?.binding)).toMatchObject({
      kind: "caller-member",
      memberId: fixture.inputMemberId,
    });
    expect(record(pins.find((pin) => pin.name === "OUT")?.binding)).toMatchObject({
      dataBlockId,
      kind: "data-block-member",
      memberId: dataBlockMemberId,
    });
    expect(topology(network)).toBe(true);
  });

  it("updates one pin while preserving the pin and existing operand identities", () => {
    const fixture = insertedFixture();
    const box = fixture.box;
    const inputPin = records(box.pins).find((pin) => pin.name === "IN");
    if (inputPin === undefined) {
      throw new Error("Expected the MOVE input pin.");
    }
    const pinId = text(inputPin.id);
    const operandId = text(record(inputPin.binding).id);
    const replacementDataBlockId = id(910);
    const replacementMemberId = id(911);
    const updated = success(updateLadBoxPinBinding(fixture.graph, {
      binding: recordValue({
        dataBlockId: replacementDataBlockId,
        id: id(999),
        kind: "data-block-member",
        memberId: replacementMemberId,
      }),
      boxNodeId: text(box.id),
      networkId: fixture.networkId,
      pinId,
    }));

    expect(updated.createdIds).toEqual([]);
    expect(semanticRevision(updated.graph)).toBe("2");
    const updatedBox = records(firstNetwork(updated.graph).nodes)
      .find((node) => node.id === box.id);
    const updatedPin = records(updatedBox?.pins).find((pin) => pin.id === pinId);
    expect(updatedPin?.id).toBe(pinId);
    expect(record(updatedPin?.binding)).toEqual({
      dataBlockId: replacementDataBlockId,
      id: operandId,
      kind: "data-block-member",
      memberId: replacementMemberId,
    });
    expect(topology(firstNetwork(updated.graph))).toBe(true);

    const outputPin = records(updatedBox?.pins).find((pin) => pin.name === "OUT");
    if (outputPin === undefined) {
      throw new Error("Expected the MOVE output pin.");
    }
    const cleared = success(updateLadBoxPinBinding(updated.graph, {
      binding: null,
      boxNodeId: text(box.id),
      networkId: fixture.networkId,
      pinId: text(outputPin.id),
    }));
    const clearedBox = records(firstNetwork(cleared.graph).nodes)
      .find((node) => node.id === box.id);
    expect(records(clearedBox?.pins).find((pin) => pin.id === outputPin.id)?.binding).toBeNull();
  });

  it("allocates a binding identity when a previously unbound pin is connected", () => {
    const fixture = linearFixture();
    const edge = fixture.edges.find((candidate) =>
      candidate.targetPortId === portByDirection(fixture.coil, "input").id
    );
    if (edge === undefined) {
      throw new Error("Expected the linear edge before the coil.");
    }
    const inserted = success(insertMvpLadInstructionBoxOnEdge(fixture.graph, {
      edgeId: text(edge.id),
      idFactory: deterministicIds(300),
      instruction: "move",
      networkId: fixture.networkId,
    }));
    const box = records(firstNetwork(inserted.graph).nodes).find((node) => node.nodeKind === "box");
    const inputPin = records(box?.pins).find((pin) => pin.name === "IN");
    if (box === undefined || inputPin === undefined) {
      throw new Error("Expected the inserted MOVE box and input pin.");
    }
    expect(inputPin.binding).toBeNull();

    const updated = success(updateLadBoxPinBinding(inserted.graph, {
      binding: recordValue({ kind: "caller-member", memberId: fixture.inputMemberId }),
      boxNodeId: text(box.id),
      idFactory: deterministicIds(400),
      networkId: fixture.networkId,
      pinId: text(inputPin.id),
    }));
    expect(updated.createdIds).toHaveLength(1);
    expect(record(records(
      records(firstNetwork(updated.graph).nodes).find((node) => node.id === box.id)?.pins,
    ).find((pin) => pin.id === inputPin.id)?.binding)).toEqual({
      id: updated.createdIds[0],
      kind: "caller-member",
      memberId: fixture.inputMemberId,
    });
  });

  it("rewrites branch entry metadata on insert and reconnects it exactly on removal", () => {
    const fixture = linearFixture();
    const selectedContactId = text(fixture.contact.id);
    const wrapped = authoringSuccess(wrapContactWithParallelContact(fixture.graph, {
      contactNodeId: selectedContactId,
      idFactory: deterministicIds(500),
      memberId: fixture.outputMemberId,
      networkId: fixture.networkId,
    }));
    const wrappedNetwork = firstNetwork(wrapped.graph);
    const branchBefore = records(wrappedNetwork.branches)[0];
    const pathBefore = records(branchBefore?.paths).find((path) => {
      const entry = records(wrappedNetwork.edges).find((edge) => edge.id === path.entryEdgeId);
      return entry?.targetPortId === portByDirection(
        records(wrappedNetwork.nodes).find((node) => node.id === selectedContactId) ?? {},
        "input",
      ).id;
    });
    if (branchBefore === undefined || pathBefore === undefined) {
      throw new Error("Expected the original contact's canonical branch path.");
    }

    const inserted = success(insertLadBoxOnEdge(wrapped.graph, {
      boxNodeFactory: moveBoxFactory(fixture.inputMemberId, id(920), id(921)),
      edgeId: text(pathBefore.entryEdgeId),
      idFactory: deterministicIds(600),
      networkId: fixture.networkId,
    }));
    const insertedNetwork = firstNetwork(inserted.graph);
    const box = records(insertedNetwork.nodes).find((node) => node.nodeKind === "box");
    if (box === undefined) {
      throw new Error("Expected an inserted box on the branch path.");
    }
    const inputPort = portByDirection(box, "input");
    const outputPort = portByDirection(box, "output");
    const incoming = records(insertedNetwork.edges).find((edge) => edge.targetPortId === inputPort.id);
    const outgoing = records(insertedNetwork.edges).find((edge) => edge.sourcePortId === outputPort.id);
    const branchInserted = records(insertedNetwork.branches)[0];
    const pathInserted = records(branchInserted?.paths).find((path) => path.id === pathBefore.id);
    expect(pathInserted).toEqual({
      ...pathBefore,
      entryEdgeId: incoming?.id,
    });
    expect(pathInserted?.exitEdgeId).toBe(pathBefore.exitEdgeId);
    expect(topology(insertedNetwork)).toBe(true);

    const removed = success(removeLadBoxAndReconnect(inserted.graph, {
      boxNodeId: text(box.id),
      idFactory: deterministicIds(800),
      networkId: fixture.networkId,
    }));
    const removedNetwork = firstNetwork(removed.graph);
    const branchRemoved = records(removedNetwork.branches)[0];
    const pathRemoved = records(branchRemoved?.paths).find((path) => path.id === pathBefore.id);
    expect(pathRemoved).toEqual({
      ...pathBefore,
      entryEdgeId: removed.createdIds[0],
    });
    expect(pathRemoved?.exitEdgeId).toBe(pathBefore.exitEdgeId);
    expect(records(removedNetwork.nodes).some((node) => node.id === box.id)).toBe(false);
    expect(records(removedNetwork.edges).some((edge) => edge.id === incoming?.id)).toBe(false);
    expect(records(removedNetwork.edges).some((edge) => edge.id === outgoing?.id)).toBe(false);
    expect(topology(removedNetwork)).toBe(true);
  });

  it("fails closed for malformed boxes and non-box removal targets", () => {
    const fixture = linearFixture();
    const edge = fixture.edges[0];
    if (edge === undefined) {
      throw new Error("Expected a linear edge.");
    }
    expect(insertLadBoxOnEdge(fixture.graph, {
      boxNode: recordValue({ id: id(1), nodeKind: "box", pins: [], powerPorts: [] }),
      edgeId: text(edge.id),
      idFactory: deterministicIds(900),
      networkId: fixture.networkId,
    })).toMatchObject({ code: "invalid-box", ok: false });
    expect(removeLadBoxAndReconnect(fixture.graph, {
      boxNodeId: text(fixture.contact.id),
      networkId: fixture.networkId,
    })).toMatchObject({ code: "not-a-box", ok: false });
    expect(semanticRevision(fixture.graph)).toBe("0");
  });
});

const moveBoxFactory = (
  inputMemberId: string,
  dataBlockId: string,
  dataBlockMemberId: string,
): LadBoxNodeFactory => ({ idFactory, semanticOrder }) => buildCanonicalLadBoxNode({
  bindings: {
    IN: { kind: "caller-member", memberId: inputMemberId },
    OUT: { dataBlockId, kind: "data-block-member", memberId: dataBlockMemberId },
  },
  idFactory,
  instruction: "move",
  semanticOrder,
});

const insertedFixture = (): Readonly<{
  box: Record<string, unknown>;
  graph: ProjectPayloadValue;
  networkId: string;
}> => {
  const fixture = linearFixture();
  const edge = fixture.edges.find((candidate) =>
    candidate.targetPortId === portByDirection(fixture.coil, "input").id
  );
  if (edge === undefined) {
    throw new Error("Expected the linear edge before the coil.");
  }
  const inserted = success(insertLadBoxOnEdge(fixture.graph, {
    boxNodeFactory: moveBoxFactory(fixture.inputMemberId, id(930), id(931)),
    edgeId: text(edge.id),
    idFactory: deterministicIds(10),
    networkId: fixture.networkId,
  }));
  const box = records(firstNetwork(inserted.graph).nodes).find((node) => node.nodeKind === "box");
  if (box === undefined) {
    throw new Error("Expected the inserted MOVE box.");
  }
  return { box, graph: inserted.graph, networkId: fixture.networkId };
};

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
  result: LadBoxAuthoringResult,
): Extract<LadBoxAuthoringResult, Readonly<{ ok: true }>> => {
  if (result.ok === false) {
    throw new Error(`${result.code}: ${result.message}`);
  }
  return result;
};

const authoringSuccess = <T extends Readonly<{ ok: boolean }>>(
  result: T,
): Extract<T, Readonly<{ ok: true }>> => {
  if (result.ok === false) {
    throw new Error("Expected the LAD authoring operation to succeed.");
  }
  return result as Extract<T, Readonly<{ ok: true }>>;
};

const deterministicIds = (start: number): (() => string) => {
  let value = start;
  return () => {
    const suffix = value.toString(16).padStart(12, "0");
    value += 1;
    return `f0000000-0000-4000-8000-${suffix}`;
  };
};

const id = (value: number): string =>
  `a0000000-0000-4000-8000-${value.toString(16).padStart(12, "0")}`;

const firstNetwork = (graph: ProjectPayloadValue): Record<string, unknown> => {
  const network = records(record(graph).networks)[0];
  if (network === undefined) {
    throw new Error("Expected one canonical LAD network.");
  }
  return network;
};

const topology = (network: Record<string, unknown>): boolean =>
  projectLadNetworkTopology(network as never).ok;

const portByDirection = (
  node: Record<string, unknown>,
  direction: "input" | "output",
): Record<string, unknown> => {
  const port = records(node.powerPorts).find((candidate) => candidate.direction === direction);
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
