import { describe, expect, it } from "vitest";

import { projectMotorStarterGuide } from "../src/motor-starter-guide";
import type { LadNetworkTopology, LadTopologyElement, LadTopologyParallel } from "../src/lad-topology";

const START = "10000000-0000-4000-8000-000000000001";
const STOP = "10000000-0000-4000-8000-000000000002";
const MOTOR = "10000000-0000-4000-8000-000000000003";
const members = [
  { id: START, name: "Start_PB" },
  { id: STOP, name: "Stop_PB" },
  { id: MOTOR, name: "Motor_Run" },
];

describe("motor-starter learning guide", () => {
  it("recognizes the incomplete starter and exposes the next safe edit targets", () => {
    const projection = projectMotorStarterGuide(incompleteTopology(), members);
    expect(projection).toMatchObject({
      available: true,
      complete: false,
      hasMotorCoil: true,
      hasSealInBranch: false,
      hasStopContact: false,
      motorMemberId: MOTOR,
      startContactNodeId: "start",
      stopInsertionEdgeId: "source-start",
      stopMemberId: STOP,
    });
  });

  it("recognizes a canonical Stop and seal-in branch as complete", () => {
    const projection = projectMotorStarterGuide(completeTopology(), members);
    expect(projection).toMatchObject({
      available: true,
      complete: true,
      hasMotorCoil: true,
      hasSealInBranch: true,
      hasStopContact: true,
    });
  });

  it("does not mistake Start and Motor contacts in the same path for a seal-in branch", () => {
    const topology = completeTopology();
    const parallel = topology.items[2];
    if (parallel?.kind !== "parallel") {
      throw new Error("Expected the fixture branch.");
    }
    const startPath = parallel.paths[0];
    const otherPath = parallel.paths[1];
    if (startPath === undefined || otherPath === undefined) {
      throw new Error("Expected two fixture paths.");
    }
    const invalidParallel: LadTopologyParallel = {
      ...parallel,
      paths: [
        {
          ...startPath,
          items: [
            ...startPath.items,
            element("motor", "contact", "start-motor", "motor-exit", MOTOR, "normally-open"),
          ],
        },
        {
          ...otherPath,
          items: [element("other", "contact", "other-entry", "other-exit", STOP, "normally-open")],
        },
      ],
    };
    const invalid: LadNetworkTopology = {
      ...topology,
      items: [
        ...topology.items.slice(0, 2),
        invalidParallel,
        ...topology.items.slice(3),
      ],
    };
    expect(projectMotorStarterGuide(invalid, members).hasSealInBranch).toBe(false);
  });
});

const incompleteTopology = (): LadNetworkTopology => ({
  items: [
    element("source", "power-source", null, "source-start"),
    element("start", "contact", "source-start", "start-coil", START, "normally-open"),
    element("coil", "coil", "start-coil", null, MOTOR, "normal"),
  ],
  kind: "network",
  networkId: "network",
  semanticOrder: 0,
  sourceNodeId: "source",
  terminalNodeId: "coil",
});

const completeTopology = (): LadNetworkTopology => ({
  items: [
    element("source", "power-source", null, "source-stop"),
    element("stop", "contact", "source-stop", "stop-branch", STOP, "normally-closed"),
    {
      afterEdgeId: "branch-coil",
      beforeEdgeId: "stop-branch",
      branchId: "branch",
      joinNode: {},
      joinNodeId: "join",
      kind: "parallel",
      paths: [
        {
          entryEdgeId: "start-entry",
          exitEdgeId: "start-exit",
          items: [element("start", "contact", "start-entry", "start-exit", START, "normally-open")],
          kind: "path",
          pathId: "start-path",
        },
        {
          entryEdgeId: "motor-entry",
          exitEdgeId: "motor-exit",
          items: [element("motor", "contact", "motor-entry", "motor-exit", MOTOR, "normally-open")],
          kind: "path",
          pathId: "motor-path",
        },
      ],
      splitNode: {},
      splitNodeId: "split",
    },
    element("coil", "coil", "branch-coil", null, MOTOR, "normal"),
  ],
  kind: "network",
  networkId: "network",
  semanticOrder: 0,
  sourceNodeId: "source",
  terminalNodeId: "coil",
});

const element = (
  nodeId: string,
  nodeKind: LadTopologyElement["nodeKind"],
  beforeEdgeId: string | null,
  afterEdgeId: string | null,
  memberId?: string,
  mode?: string,
): LadTopologyElement => ({
  afterEdgeId,
  beforeEdgeId,
  kind: "element",
  node: {
    ...(mode === undefined ? {} : { mode }),
    ...(memberId === undefined ? {} : {
      operand: { $type: "record", value: { memberId } },
    }),
  },
  nodeId,
  nodeKind,
  semanticOrder: 0,
});
