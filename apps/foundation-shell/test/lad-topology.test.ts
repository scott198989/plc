import { describe, expect, it } from "vitest";

import {
  createLadProgramPayload,
  interfaceMemberIdentity,
  recordValue,
  unsignedValue,
} from "../src/canonical-authoring";
import {
  insertSeriesContact,
  wrapContactWithParallelContact,
} from "../src/lad-authoring";
import type { LadAuthoringResult } from "../src/lad-authoring";
import {
  LAD_TOPOLOGY_LIMITS,
  projectLadNetworkTopology,
} from "../src/lad-topology";
import type {
  LadNetworkTopology,
  LadTopologyParallel,
  LadTopologyResult,
} from "../src/lad-topology";
import type { ProjectPayload, ProjectPayloadValue } from "../src/workbench-types";

describe("LAD network topology projection", () => {
  it("derives real series order from edges and exposes insertion-edge identities", () => {
    const fixture = linearFixture();
    const edgeBeforeContact = fixture.edges.find((edge) =>
      edge.targetPortId === inputPort(fixture.contact).id
    );
    if (edgeBeforeContact === undefined) {
      throw new Error("Expected the edge before the fixture contact.");
    }

    const inserted = authoringSuccess(insertSeriesContact(fixture.graph, {
      edgeId: text(edgeBeforeContact.id),
      idFactory: deterministicIds(100),
      memberId: fixture.inputMemberId,
      networkId: fixture.networkId,
    }));
    const network = firstNetwork(inserted.graph);
    const networkBefore = JSON.stringify(network);
    const topology = topologySuccess(projectLadNetworkTopology(network));
    const elements = topology.items.map((item) => {
      if (item.kind !== "element") {
        throw new Error("Expected a linear element.");
      }
      return item;
    });

    expect(elements.map((item) => item.nodeKind)).toEqual([
      "power-source",
      "contact",
      "contact",
      "coil",
    ]);
    expect(elements.map((item) => item.nodeId)).toEqual(
      records(network.nodes).map((node) => text(node.id)),
    );
    expect(elements[0]?.beforeEdgeId).toBeNull();
    expect(elements.at(-1)?.afterEdgeId).toBeNull();
    for (let index = 0; index < elements.length - 1; index += 1) {
      expect(elements[index]?.afterEdgeId).toBe(elements[index + 1]?.beforeEdgeId);
    }

    const edgeArrayOrder = records(network.edges).map((edge) => edge.id);
    const traversalOrder = elements.slice(0, -1).map((item) => item.afterEdgeId);
    expect(traversalOrder).not.toEqual(edgeArrayOrder);
    expect(JSON.stringify(network)).toBe(networkBefore);
  });

  it("preserves a canonical parallel branch instead of flattening its paths", () => {
    const fixture = linearFixture();
    const wrapped = authoringSuccess(wrapContactWithParallelContact(fixture.graph, {
      contactNodeId: text(fixture.contact.id),
      idFactory: deterministicIds(200),
      memberId: fixture.outputMemberId,
      networkId: fixture.networkId,
    }));
    const network = firstNetwork(wrapped.graph);
    const branch = records(network.branches)[0];
    if (branch === undefined) {
      throw new Error("Expected one canonical parallel branch.");
    }
    const branchPaths = records(branch.paths);
    const topology = topologySuccess(projectLadNetworkTopology(network));

    expect(topology.items.map((item) => item.kind)).toEqual([
      "element",
      "parallel",
      "element",
    ]);
    const parallel = topology.items[1];
    if (parallel?.kind !== "parallel") {
      throw new Error("Expected a parallel topology item.");
    }
    expect(parallel).toMatchObject({
      branchId: branch.id,
      joinNodeId: branch.joinNodeId,
      splitNodeId: branch.splitNodeId,
    });
    expect(parallel.paths.map((path) => ({
      entryEdgeId: path.entryEdgeId,
      exitEdgeId: path.exitEdgeId,
      pathId: path.pathId,
    }))).toEqual(branchPaths.map((path) => ({
      entryEdgeId: path.entryEdgeId,
      exitEdgeId: path.exitEdgeId,
      pathId: path.id,
    })));
    expect(parallel.paths.every((path) =>
      path.items.length === 1 &&
      path.items[0]?.beforeEdgeId === path.entryEdgeId &&
      path.items[0]?.afterEdgeId === path.exitEdgeId
    )).toBe(true);
    expect(topology.items[0]?.afterEdgeId).toBe(parallel.beforeEdgeId);
    expect(parallel.afterEdgeId).toBe(topology.items[2]?.beforeEdgeId);
  });

  it("retains nested canonical parallel branches recursively", () => {
    const fixture = linearFixture();
    const outerGraph = authoringSuccess(wrapContactWithParallelContact(fixture.graph, {
      contactNodeId: text(fixture.contact.id),
      idFactory: deterministicIds(300),
      memberId: fixture.outputMemberId,
      networkId: fixture.networkId,
    })).graph;
    const nestedGraph = authoringSuccess(wrapContactWithParallelContact(outerGraph, {
      contactNodeId: text(fixture.contact.id),
      idFactory: deterministicIds(400),
      memberId: fixture.inputMemberId,
      networkId: fixture.networkId,
    })).graph;
    const network = firstNetwork(nestedGraph);
    const branches = records(network.branches);
    const topology = topologySuccess(projectLadNetworkTopology(network));
    const outer = topology.items.find((item): item is LadTopologyParallel => item.kind === "parallel");
    const nested = outer?.paths
      .flatMap((path) => path.items)
      .find((item): item is LadTopologyParallel => item.kind === "parallel");

    expect(outer?.branchId).toBe(branches[0]?.id);
    expect(nested?.branchId).toBe(branches[1]?.id);
    expect(outer?.paths.flatMap((path) => path.items).map((item) => item.kind)).toContain("parallel");
  });

  it("returns a cycle error for a bounded, port-complete cyclic fragment", () => {
    const fixture = linearFixture();
    const ids = deterministicIds(500);
    const first = cyclicContact(ids, records(fixture.network.nodes).length);
    const second = cyclicContact(ids, records(fixture.network.nodes).length + 1);
    const cyclic: ProjectPayload = {
      ...fixture.network,
      edges: [
        ...values(fixture.network.edges),
        edge(first.outputPortId, second.inputPortId, ids()),
        edge(second.outputPortId, first.inputPortId, ids()),
      ],
      nodes: [...values(fixture.network.nodes), first.value, second.value],
    };

    expect(() => projectLadNetworkTopology(cyclic)).not.toThrow();
    expect(projectLadNetworkTopology(cyclic)).toMatchObject({ code: "cycle", ok: false });
  });

  it("returns a dangling-port error without throwing", () => {
    const fixture = linearFixture();
    const nodeValues = values(fixture.network.nodes);
    const nodes = records(fixture.network.nodes);
    const contactIndex = nodes.findIndex((node) => node.nodeKind === "contact");
    const contact = nodes[contactIndex];
    if (contact === undefined || contactIndex < 0) {
      throw new Error("Expected one fixture contact.");
    }
    const danglingPortId = deterministicIds(600)();
    const malformed: ProjectPayload = {
      ...fixture.network,
      nodes: nodeValues.map((node, index) => index === contactIndex
        ? recordValue({
          ...contact,
          powerPorts: [
            ...values(contact.powerPorts),
            recordValue({ direction: "output", id: danglingPortId }),
          ],
        })
        : node),
    };

    expect(() => projectLadNetworkTopology(malformed)).not.toThrow();
    expect(projectLadNetworkTopology(malformed)).toMatchObject({
      code: "dangling-port",
      ok: false,
    });
  });

  it("rejects malformed branch boundaries without flattening or throwing", () => {
    const fixture = linearFixture();
    const wrapped = authoringSuccess(wrapContactWithParallelContact(fixture.graph, {
      contactNodeId: text(fixture.contact.id),
      idFactory: deterministicIds(700),
      memberId: fixture.outputMemberId,
      networkId: fixture.networkId,
    }));
    const network = firstNetwork(wrapped.graph);
    const branch = records(network.branches)[0];
    const branchPaths = records(branch?.paths);
    const firstPath = branchPaths[0];
    if (branch === undefined || firstPath === undefined) {
      throw new Error("Expected canonical branch metadata.");
    }
    const malformed: ProjectPayload = {
      ...network,
      branches: [recordValue({
        ...branch,
        paths: [
          recordValue({ ...firstPath, entryEdgeId: text(firstPath.exitEdgeId) }),
          ...values(branch.paths).slice(1),
        ],
      })],
    };

    expect(() => projectLadNetworkTopology(malformed)).not.toThrow();
    expect(projectLadNetworkTopology(malformed)).toMatchObject({
      code: "invalid-branch",
      ok: false,
    });
  });

  it("fails closed before iterating a network beyond the Rust node bound", () => {
    const fixture = linearFixture();
    const firstNode = values(fixture.network.nodes)[0];
    if (firstNode === undefined) {
      throw new Error("Expected a fixture node.");
    }
    const oversized: ProjectPayload = {
      ...fixture.network,
      nodes: Array.from(
        { length: LAD_TOPOLOGY_LIMITS.maxNodesPerNetwork + 1 },
        () => firstNode,
      ),
    };
    expect(projectLadNetworkTopology(oversized)).toMatchObject({
      code: "resource-limit",
      ok: false,
    });
  });

  it("converts malformed runtime input into a typed error", () => {
    const fixture = linearFixture();
    const malformed = { ...fixture.network, nodes: [null] } as ProjectPayload;
    expect(() => projectLadNetworkTopology(malformed)).not.toThrow();
    expect(projectLadNetworkTopology(malformed)).toMatchObject({
      code: "invalid-network",
      ok: false,
    });
  });
});

const linearFixture = (): Readonly<{
  contact: ProjectPayload;
  edges: readonly ProjectPayload[];
  graph: ProjectPayloadValue;
  inputMemberId: string;
  network: ProjectPayload;
  networkId: string;
  outputMemberId: string;
}> => {
  const payload = createLadProgramPayload();
  const graph = payload.graph;
  if (graph === undefined) {
    throw new Error("Expected a canonical LAD graph.");
  }
  const network = firstNetwork(graph);
  const contact = records(network.nodes).find((node) => node.nodeKind === "contact");
  const inputMemberId = interfaceMemberIdentity(payload, "InputValue");
  const outputMemberId = interfaceMemberIdentity(payload, "OutputValue");
  if (contact === undefined || inputMemberId === null || outputMemberId === null) {
    throw new Error("Expected a complete linear LAD fixture.");
  }
  return {
    contact,
    edges: records(network.edges),
    graph,
    inputMemberId,
    network,
    networkId: text(network.id),
    outputMemberId,
  };
};

const cyclicContact = (
  nextId: () => string,
  semanticOrder: number,
): Readonly<{
  inputPortId: string;
  outputPortId: string;
  value: ProjectPayloadValue;
}> => {
  const inputPortId = nextId();
  const outputPortId = nextId();
  return {
    inputPortId,
    outputPortId,
    value: recordValue({
      id: nextId(),
      mode: "normally-open",
      nodeKind: "contact",
      operand: null,
      powerPorts: [
        recordValue({ direction: "input", id: inputPortId }),
        recordValue({ direction: "output", id: outputPortId }),
      ],
      semanticOrder: unsignedValue(semanticOrder),
    }),
  };
};

const edge = (
  sourcePortId: string,
  targetPortId: string,
  id: string,
): ProjectPayloadValue => recordValue({ id, sourcePortId, targetPortId });

const inputPort = (node: ProjectPayload): ProjectPayload => {
  const port = records(node.powerPorts).find((candidate) => candidate.direction === "input");
  if (port === undefined) {
    throw new Error("Expected one input port.");
  }
  return port;
};

const authoringSuccess = (
  result: LadAuthoringResult,
): Extract<LadAuthoringResult, Readonly<{ ok: true }>> => {
  if (result.ok === false) {
    throw new Error(`${result.code}: ${result.message}`);
  }
  return result;
};

const topologySuccess = (result: LadTopologyResult): LadNetworkTopology => {
  if (result.ok === false) {
    throw new Error(`${result.code}: ${result.message}`);
  }
  return result.topology;
};

const deterministicIds = (start: number): (() => string) => {
  let value = start;
  return () => {
    const suffix = value.toString(16).padStart(12, "0");
    value += 1;
    return `e0000000-0000-4000-8000-${suffix}`;
  };
};

const firstNetwork = (graph: ProjectPayloadValue): ProjectPayload => {
  const network = records(record(graph).networks)[0];
  if (network === undefined) {
    throw new Error("Expected one canonical LAD network.");
  }
  return network;
};

const record = (value: unknown): ProjectPayload => {
  if (
    typeof value !== "object" ||
    value === null ||
    Array.isArray(value) ||
    !("$type" in value) ||
    value.$type !== "record" ||
    !("value" in value) ||
    typeof value.value !== "object" ||
    value.value === null ||
    Array.isArray(value.value)
  ) {
    throw new Error("Expected a canonical record value.");
  }
  return value.value as ProjectPayload;
};

const records = (value: unknown): readonly ProjectPayload[] => values(value).map(record);

const values = (value: unknown): readonly ProjectPayloadValue[] => {
  if (!Array.isArray(value)) {
    throw new Error("Expected a canonical value list.");
  }
  return value as readonly ProjectPayloadValue[];
};

const text = (value: unknown): string => {
  if (typeof value !== "string") {
    throw new Error("Expected canonical text.");
  }
  return value;
};
