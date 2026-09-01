import { describe, expect, it } from "vitest";

import type { LadBooleanMemberLiveState } from "../src/lad-live-monitoring";
import { projectLadPowerFlow } from "../src/lad-power-flow";
import type { LadNetworkTopology, LadTopologyElement } from "../src/lad-topology";

const START = "10000000-0000-4000-8000-000000000001";
const STOP = "10000000-0000-4000-8000-000000000002";
const MOTOR = "10000000-0000-4000-8000-000000000003";

describe("LAD learner power-flow projection", () => {
  it("shows a Start contact energizing the motor rung", () => {
    const result = projectLadPowerFlow(topology(), live({ start: true, stop: false, motor: false }));
    expect(result.rungState).toBe("on");
    expect(result.nodeStates.get("stop")?.condition).toBe("on");
    expect(result.pathStates.get("start-path")).toBe("on");
    expect(result.nodeStates.get("coil")?.incoming).toBe("on");
  });

  it("shows the motor contact maintaining the parallel seal-in path", () => {
    const result = projectLadPowerFlow(topology(), live({ start: false, stop: false, motor: true }));
    expect(result.rungState).toBe("on");
    expect(result.pathStates.get("start-path")).toBe("off");
    expect(result.pathStates.get("motor-path")).toBe("on");
  });

  it("shows the normally-closed Stop contact interrupting every branch", () => {
    const result = projectLadPowerFlow(topology(), live({ start: true, stop: true, motor: true }));
    expect(result.rungState).toBe("off");
    expect(result.nodeStates.get("stop")?.condition).toBe("off");
    expect(result.edgeStates.get("after-branch")).toBe("off");
  });

  it("fails closed to unknown when monitored evidence is unavailable", () => {
    const result = projectLadPowerFlow(topology(), new Map());
    expect(result.rungState).toBe("unknown");
    expect(result.nodeStates.get("coil")?.incoming).toBe("unknown");
  });

  it("carries incoming rung power through an instruction box ENO path", () => {
    const result = projectLadPowerFlow({
      ...topology(),
      items: [
        element("source", "power-source", null, "source-box"),
        element("add", "box", "source-box", "box-coil"),
        element("coil", "coil", "box-coil", null, MOTOR, "normal"),
      ],
    }, new Map());
    expect(result.nodeStates.get("add")).toEqual({
      condition: null,
      incoming: "on",
      outgoing: "on",
    });
    expect(result.rungState).toBe("on");
  });
});

const topology = (): LadNetworkTopology => ({
  items: [
    element("source", "power-source", null, "source-stop"),
    element("stop", "contact", "source-stop", "stop-branch", STOP, "normally-closed"),
    {
      afterEdgeId: "after-branch",
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
    element("coil", "coil", "after-branch", null, MOTOR, "normal"),
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

const live = (values: Readonly<{ motor: boolean; start: boolean; stop: boolean }>): ReadonlyMap<string, LadBooleanMemberLiveState> =>
  new Map([
    state(START, "Start_PB", values.start),
    state(STOP, "Stop_PB", values.stop),
    state(MOTOR, "Motor_Run", values.motor),
  ]);

const state = (
  memberId: string,
  memberName: string,
  value: boolean,
): readonly [string, LadBooleanMemberLiveState] => [memberId, {
  forced: false,
  forcedValue: null,
  memberId,
  memberName,
  observedValue: value,
  probeId: memberId,
  probeKind: memberName === "Motor_Run" ? "output" : "input",
  quality: "GOOD",
  role: "temp",
  runtimeAddress: memberName === "Motor_Run" ? "%Q0.0" : "%I0.0",
  tagId: memberId,
  tagName: memberName,
  truth: value ? "on" : "off",
  unknownReason: null,
}];
