import { canonicalRecordFields } from "./canonical-authoring";
import type { LadBooleanMemberLiveState } from "./lad-live-monitoring";
import type {
  ProjectPayload,
} from "./workbench-types";
import type {
  LadNetworkTopology,
  LadTopologyItem,
  LadTopologyParallel,
} from "./lad-topology";

export type LadPowerState = "off" | "on" | "unknown";

export type LadNodePowerState = Readonly<{
  condition: LadPowerState | null;
  incoming: LadPowerState;
  outgoing: LadPowerState;
}>;

export type LadPowerFlowProjection = Readonly<{
  edgeStates: ReadonlyMap<string, LadPowerState>;
  nodeStates: ReadonlyMap<string, LadNodePowerState>;
  pathStates: ReadonlyMap<string, LadPowerState>;
  rungState: LadPowerState;
}>;

/**
 * Builds a presentation-only power-flow read model from canonical topology and
 * authoritative monitored operand values. It never predicts or mutates PLC
 * state: missing runtime evidence remains unknown, and the simulator remains
 * the only authority for output values.
 */
export const projectLadPowerFlow = (
  topology: LadNetworkTopology,
  liveMembers: ReadonlyMap<string, LadBooleanMemberLiveState>,
): LadPowerFlowProjection => {
  const edgeStates = new Map<string, LadPowerState>();
  const nodeStates = new Map<string, LadNodePowerState>();
  const pathStates = new Map<string, LadPowerState>();
  const rungState = projectSeries(
    topology.items,
    "on",
    liveMembers,
    edgeStates,
    nodeStates,
    pathStates,
  );
  return { edgeStates, nodeStates, pathStates, rungState };
};

const projectSeries = (
  items: readonly LadTopologyItem[],
  initialState: LadPowerState,
  liveMembers: ReadonlyMap<string, LadBooleanMemberLiveState>,
  edgeStates: Map<string, LadPowerState>,
  nodeStates: Map<string, LadNodePowerState>,
  pathStates: Map<string, LadPowerState>,
): LadPowerState => {
  let current = initialState;
  for (const item of items) {
    if (item.beforeEdgeId !== null) {
      edgeStates.set(item.beforeEdgeId, current);
    }
    if (item.kind === "parallel") {
      current = projectParallel(
        item,
        current,
        liveMembers,
        edgeStates,
        nodeStates,
        pathStates,
      );
      continue;
    }

    const condition = item.nodeKind === "contact"
      ? contactCondition(item.node, liveMembers)
      : null;
    const outgoing = item.nodeKind === "power-source"
      ? "on"
      : item.nodeKind === "contact"
        ? seriesState(current, condition ?? "unknown")
        : passesPower(item.nodeKind)
          ? current
          : current === "off" ? "off" : "unknown";
    nodeStates.set(item.nodeId, {
      condition,
      incoming: item.nodeKind === "power-source" ? "on" : current,
      outgoing,
    });
    if (item.afterEdgeId !== null) {
      edgeStates.set(item.afterEdgeId, outgoing);
    }
    current = outgoing;
  }
  return current;
};

const projectParallel = (
  parallel: LadTopologyParallel,
  incoming: LadPowerState,
  liveMembers: ReadonlyMap<string, LadBooleanMemberLiveState>,
  edgeStates: Map<string, LadPowerState>,
  nodeStates: Map<string, LadNodePowerState>,
  pathStates: Map<string, LadPowerState>,
): LadPowerState => {
  nodeStates.set(parallel.splitNodeId, {
    condition: null,
    incoming,
    outgoing: incoming,
  });
  const results = parallel.paths.map((path) => {
    edgeStates.set(path.entryEdgeId, incoming);
    const result = projectSeries(
      path.items,
      incoming,
      liveMembers,
      edgeStates,
      nodeStates,
      pathStates,
    );
    edgeStates.set(path.exitEdgeId, result);
    pathStates.set(path.pathId, result);
    return result;
  });
  const outgoing = parallelState(results);
  nodeStates.set(parallel.joinNodeId, {
    condition: null,
    incoming: outgoing,
    outgoing,
  });
  edgeStates.set(parallel.afterEdgeId, outgoing);
  return outgoing;
};

const contactCondition = (
  node: ProjectPayload,
  liveMembers: ReadonlyMap<string, LadBooleanMemberLiveState>,
): LadPowerState => {
  const operand = canonicalRecordFields(node.operand);
  const memberId = typeof operand?.memberId === "string" ? operand.memberId : null;
  const truth = memberId === null ? "unknown" : liveMembers.get(memberId)?.truth ?? "unknown";
  if (truth === "unknown") {
    return "unknown";
  }
  const active = truth === "on";
  const conducts = node.mode === "normally-closed" ? !active : active;
  return conducts ? "on" : "off";
};

const passesPower = (nodeKind: string): boolean =>
  nodeKind === "box" ||
  nodeKind === "coil" ||
  nodeKind === "return";

const seriesState = (left: LadPowerState, right: LadPowerState): LadPowerState => {
  if (left === "off" || right === "off") {
    return "off";
  }
  return left === "on" && right === "on" ? "on" : "unknown";
};

const parallelState = (states: readonly LadPowerState[]): LadPowerState => {
  if (states.some((state) => state === "on")) {
    return "on";
  }
  return states.every((state) => state === "off") ? "off" : "unknown";
};
