import type { ProjectPayload } from "./workbench-types";

/**
 * The canonical Rust LAD boundary accepts at most 10,000 nodes and 20,000
 * edges in one network. A valid graph can therefore own at most two ports per
 * edge. The additional nesting bound protects the recursive render model from
 * pathologically deep, but otherwise finite, branch structures.
 */
export const LAD_TOPOLOGY_LIMITS = Object.freeze({
  maxBranchesPerNetwork: 10_000,
  maxBranchPathsPerNetwork: 20_000,
  maxEdgesPerNetwork: 20_000,
  maxNestingDepth: 256,
  maxNodesPerNetwork: 10_000,
  maxPowerPortsPerNetwork: 40_000,
});

export type LadTopologyElementNodeKind =
  | "power-source"
  | "contact"
  | "coil"
  | "box"
  | "call"
  | "return"
  | "unsupported-control"
  | "unresolved";

export type LadTopologyElement = Readonly<{
  /** The edge immediately entering this element, or null at the power source. */
  beforeEdgeId: string | null;
  /** The edge immediately leaving this element, or null at a terminal. */
  afterEdgeId: string | null;
  kind: "element";
  node: ProjectPayload;
  nodeId: string;
  nodeKind: LadTopologyElementNodeKind;
  semanticOrder: number;
}>;

export type LadTopologyPath = Readonly<{
  /** The canonical split-to-path edge. It is also the first item's before edge. */
  entryEdgeId: string;
  /** The canonical path-to-join edge. It is also the last item's after edge. */
  exitEdgeId: string;
  items: readonly LadTopologyItem[];
  kind: "path";
  pathId: string;
}>;

export type LadTopologyParallel = Readonly<{
  /** The edge entering the branch split. */
  beforeEdgeId: string;
  /** The edge leaving the branch join. */
  afterEdgeId: string;
  branchId: string;
  joinNode: ProjectPayload;
  joinNodeId: string;
  kind: "parallel";
  paths: readonly LadTopologyPath[];
  splitNode: ProjectPayload;
  splitNodeId: string;
}>;

export type LadTopologyItem = LadTopologyElement | LadTopologyParallel;

export type LadNetworkTopology = Readonly<{
  items: readonly LadTopologyItem[];
  kind: "network";
  networkId: string;
  semanticOrder: number;
  sourceNodeId: string;
  terminalNodeId: string;
}>;

export type LadTopologyErrorCode =
  | "cycle"
  | "dangling-port"
  | "duplicate-identity"
  | "invalid-branch"
  | "invalid-edge-direction"
  | "invalid-network"
  | "invalid-node"
  | "invalid-order"
  | "open-graph"
  | "orphan-edge"
  | "resource-limit"
  | "unreachable";

export type LadTopologyResult =
  | Readonly<{ ok: true; topology: LadNetworkTopology }>
  | Readonly<{ code: LadTopologyErrorCode; message: string; ok: false }>;

type PowerPortDirection = "input" | "output";
type ParsedNodeKind = LadTopologyElementNodeKind | "branch-split" | "branch-join";

type ParsedPort = Readonly<{
  direction: PowerPortDirection;
  id: string;
}>;

type ParsedNode = Readonly<{
  branchId: string | null;
  fields: ProjectPayload;
  id: string;
  kind: ParsedNodeKind;
  ports: readonly ParsedPort[];
  semanticOrder: number;
}>;

type ParsedEdge = Readonly<{
  id: string;
  sourcePortId: string;
  targetPortId: string;
}>;

type IndexedEdge = ParsedEdge & Readonly<{
  sourceNodeId: string;
  targetNodeId: string;
}>;

type ParsedBranchPath = Readonly<{
  entryEdgeId: string;
  exitEdgeId: string;
  id: string;
}>;

type ParsedBranch = Readonly<{
  id: string;
  joinNodeId: string;
  paths: readonly ParsedBranchPath[];
  splitNodeId: string;
}>;

type ParsedNetwork = Readonly<{
  branches: readonly ParsedBranch[];
  branchById: ReadonlyMap<string, ParsedBranch>;
  edges: readonly ParsedEdge[];
  edgeById: ReadonlyMap<string, ParsedEdge>;
  fields: ProjectPayload;
  id: string;
  nodeById: ReadonlyMap<string, ParsedNode>;
  nodes: readonly ParsedNode[];
  semanticOrder: number;
}>;

type GraphIndex = Readonly<{
  edgeById: ReadonlyMap<string, IndexedEdge>;
  incoming: ReadonlyMap<string, readonly IndexedEdge[]>;
  outgoing: ReadonlyMap<string, readonly IndexedEdge[]>;
}>;

type ProjectionState = {
  readonly edges: Set<string>;
  readonly nodes: Set<string>;
};

type ProjectedSeries = Readonly<{
  arrivalEdgeId: string | null;
  items: readonly LadTopologyItem[];
  terminalNodeId: string | null;
}>;

const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/iu;
const MAX_U16 = 65_535;
const MAX_U32 = 4_294_967_295;

class ProjectionAbort extends Error {
  readonly code: LadTopologyErrorCode;

  constructor(code: LadTopologyErrorCode, message: string) {
    super(message);
    this.code = code;
  }
}

/**
 * Projects one unwrapped canonical LAD network record into a topology-safe UI
 * model. The function never repairs or flattens the graph: invalid input is a
 * discriminated error, and canonical branch/path order is retained verbatim.
 */
export const projectLadNetworkTopology = (
  network: ProjectPayload,
): LadTopologyResult => {
  try {
    const parsed = parseNetwork(network);
    const index = indexGraph(parsed);
    validateGraph(parsed, index);
    return { ok: true, topology: projectTopology(parsed, index) };
  } catch (error: unknown) {
    if (error instanceof ProjectionAbort) {
      return { code: error.code, message: error.message, ok: false };
    }
    return {
      code: "invalid-network",
      message: "The LAD network could not be projected safely.",
      ok: false,
    };
  }
};

const parseNetwork = (value: ProjectPayload): ParsedNetwork => {
  const fields = payloadFields(value, "network");
  const id = identityField(fields, "id", "network");
  const semanticOrder = unsignedField(fields, "semanticOrder", MAX_U32, "network");
  const nodeValues = boundedArray(
    fields.nodes,
    LAD_TOPOLOGY_LIMITS.maxNodesPerNetwork,
    "network.nodes",
    "nodes",
  );
  const edgeValues = boundedArray(
    fields.edges,
    LAD_TOPOLOGY_LIMITS.maxEdgesPerNetwork,
    "network.edges",
    "edges",
  );
  const branchValues = fields.branches === undefined
    ? []
    : boundedArray(
      fields.branches,
      LAD_TOPOLOGY_LIMITS.maxBranchesPerNetwork,
      "network.branches",
      "branches",
    );

  const nodes: ParsedNode[] = [];
  const nodeById = new Map<string, ParsedNode>();
  const portIds = new Set<string>();
  let portCount = 0;
  for (let index = 0; index < nodeValues.length; index += 1) {
    const path = `network.nodes[${index}]`;
    const node = parseNode(recordFields(nodeValues[index], path), path);
    if (node.semanticOrder !== index) {
      abort("invalid-order", `${path}.semanticOrder does not match canonical node-array order.`);
    }
    if (nodeById.has(node.id)) {
      abort("duplicate-identity", `${path}.id duplicates another LAD node identity.`);
    }
    if (portCount > LAD_TOPOLOGY_LIMITS.maxPowerPortsPerNetwork - node.ports.length) {
      abort(
        "resource-limit",
        `network power ports exceed the limit of ${LAD_TOPOLOGY_LIMITS.maxPowerPortsPerNetwork}.`,
      );
    }
    portCount += node.ports.length;
    for (const port of node.ports) {
      if (portIds.has(port.id)) {
        abort("duplicate-identity", `${path} contains a duplicate LAD power-port identity.`);
      }
      portIds.add(port.id);
    }
    nodes.push(node);
    nodeById.set(node.id, node);
  }

  const edges: ParsedEdge[] = [];
  const edgeById = new Map<string, ParsedEdge>();
  for (let index = 0; index < edgeValues.length; index += 1) {
    const path = `network.edges[${index}]`;
    const edge = parseEdge(recordFields(edgeValues[index], path), path);
    if (edgeById.has(edge.id)) {
      abort("duplicate-identity", `${path}.id duplicates another LAD power-edge identity.`);
    }
    edges.push(edge);
    edgeById.set(edge.id, edge);
  }

  const branches: ParsedBranch[] = [];
  const branchById = new Map<string, ParsedBranch>();
  const pathIds = new Set<string>();
  let branchPathCount = 0;
  for (let index = 0; index < branchValues.length; index += 1) {
    const path = `network.branches[${index}]`;
    const branch = parseBranch(recordFields(branchValues[index], path), path);
    if (branchById.has(branch.id)) {
      abort("duplicate-identity", `${path}.id duplicates another LAD branch identity.`);
    }
    if (branchPathCount > LAD_TOPOLOGY_LIMITS.maxBranchPathsPerNetwork - branch.paths.length) {
      abort(
        "resource-limit",
        `network branch paths exceed the limit of ${LAD_TOPOLOGY_LIMITS.maxBranchPathsPerNetwork}.`,
      );
    }
    branchPathCount += branch.paths.length;
    for (const branchPath of branch.paths) {
      if (pathIds.has(branchPath.id)) {
        abort("duplicate-identity", `${path}.paths contains a duplicate LAD path identity.`);
      }
      pathIds.add(branchPath.id);
    }
    branches.push(branch);
    branchById.set(branch.id, branch);
  }

  return {
    branches,
    branchById,
    edges,
    edgeById,
    fields,
    id,
    nodeById,
    nodes,
    semanticOrder,
  };
};

const parseNode = (fields: ProjectPayload, path: string): ParsedNode => {
  const id = identityField(fields, "id", path);
  const kind = nodeKindField(fields, path);
  const semanticOrder = unsignedField(fields, "semanticOrder", MAX_U32, path);
  const portValues = boundedArray(
    fields.powerPorts,
    LAD_TOPOLOGY_LIMITS.maxPowerPortsPerNetwork,
    `${path}.powerPorts`,
    "power ports",
  );
  const ports = portValues.map((port, index): ParsedPort => {
    const portPath = `${path}.powerPorts[${index}]`;
    const portFields = recordFields(port, portPath);
    const direction = portFields.direction;
    if (direction !== "input" && direction !== "output") {
      abort("invalid-node", `${portPath}.direction must be input or output.`);
    }
    return {
      direction,
      id: identityField(portFields, "id", portPath),
    };
  });

  let branchId: string | null = null;
  switch (kind) {
    case "branch-join":
    case "branch-split":
      branchId = identityField(fields, "branchId", path);
      break;
    case "contact":
      if (fields.mode !== "normally-open" && fields.mode !== "normally-closed") {
        abort("invalid-node", `${path}.mode is not a supported canonical contact mode.`);
      }
      break;
    case "coil":
      if (
        fields.mode !== "normal" &&
        fields.mode !== "negated" &&
        fields.mode !== "set" &&
        fields.mode !== "reset"
      ) {
        abort("invalid-node", `${path}.mode is not a supported canonical coil mode.`);
      }
      break;
    case "box":
      unsignedField(fields, "instructionCode", MAX_U16, path);
      requireArrayField(fields, "pins", path);
      break;
    case "call":
      identityField(fields, "callSiteId", path);
      identityField(fields, "targetBlockId", path);
      unsignedField(fields, "instructionCode", MAX_U16, path);
      requireArrayField(fields, "pins", path);
      break;
    case "unsupported-control":
      textField(fields, "capability", path);
      break;
    case "unresolved":
      textField(fields, "requestedName", path);
      break;
    case "power-source":
    case "return":
      break;
  }

  return { branchId, fields, id, kind, ports, semanticOrder };
};

const parseEdge = (fields: ProjectPayload, path: string): ParsedEdge => ({
  id: identityField(fields, "id", path),
  sourcePortId: identityField(fields, "sourcePortId", path),
  targetPortId: identityField(fields, "targetPortId", path),
});

const parseBranch = (fields: ProjectPayload, path: string): ParsedBranch => {
  const pathValues = boundedArray(
    fields.paths,
    LAD_TOPOLOGY_LIMITS.maxBranchPathsPerNetwork,
    `${path}.paths`,
    "branch paths",
  );
  const paths = pathValues.map((value, index) => {
    const itemPath = `${path}.paths[${index}]`;
    const item = recordFields(value, itemPath);
    return {
      entryEdgeId: identityField(item, "entryEdgeId", itemPath),
      exitEdgeId: identityField(item, "exitEdgeId", itemPath),
      id: identityField(item, "id", itemPath),
    };
  });
  return {
    id: identityField(fields, "id", path),
    joinNodeId: identityField(fields, "joinNodeId", path),
    paths,
    splitNodeId: identityField(fields, "splitNodeId", path),
  };
};

const indexGraph = (network: ParsedNetwork): GraphIndex => {
  const portOwner = new Map<string, Readonly<{ direction: PowerPortDirection; nodeId: string }>>();
  const portUse = new Map<string, number>();
  for (const node of network.nodes) {
    for (const port of node.ports) {
      portOwner.set(port.id, { direction: port.direction, nodeId: node.id });
      portUse.set(port.id, 0);
    }
  }

  const incoming = new Map<string, IndexedEdge[]>();
  const outgoing = new Map<string, IndexedEdge[]>();
  const edgeById = new Map<string, IndexedEdge>();
  for (const edge of network.edges) {
    const source = portOwner.get(edge.sourcePortId);
    const target = portOwner.get(edge.targetPortId);
    if (source === undefined) {
      abort("orphan-edge", `LAD edge ${edge.id} references an unknown power port.`);
    }
    if (target === undefined) {
      abort("orphan-edge", `LAD edge ${edge.id} references an unknown power port.`);
    }
    if (source.direction !== "output" || target.direction !== "input") {
      abort("invalid-edge-direction", `LAD edge ${edge.id} does not flow from output to input.`);
    }
    const indexed: IndexedEdge = {
      ...edge,
      sourceNodeId: source.nodeId,
      targetNodeId: target.nodeId,
    };
    edgeById.set(edge.id, indexed);
    pushIndex(outgoing, source.nodeId, indexed);
    pushIndex(incoming, target.nodeId, indexed);
    portUse.set(edge.sourcePortId, (portUse.get(edge.sourcePortId) ?? 0) + 1);
    portUse.set(edge.targetPortId, (portUse.get(edge.targetPortId) ?? 0) + 1);
  }
  for (const [portId, uses] of portUse) {
    if (uses !== 1) {
      abort(
        "dangling-port",
        `LAD power port ${portId} must be referenced by exactly one edge; found ${uses}.`,
      );
    }
  }
  return { edgeById, incoming, outgoing };
};

const validateGraph = (network: ParsedNetwork, index: GraphIndex): void => {
  if (hasCycle(network, index)) {
    abort("cycle", "The LAD network contains a control-flow cycle.");
  }
  validateBranches(network, index);
  validateNodeArities(network, index);

  const sources = network.nodes.filter((node) => node.kind === "power-source");
  const source = sources[0];
  if (sources.length !== 1 || source === undefined) {
    abort("invalid-network", "A LAD network must contain exactly one power source.");
  }
  if (!network.nodes.some((node) => node.kind === "coil" || node.kind === "return")) {
    abort("invalid-network", "A LAD network must contain at least one coil or return terminal.");
  }

  const reachable = reachableNodes(source.id, index);
  if (reachable.size !== network.nodes.length) {
    const missing = network.nodes.find((node) => !reachable.has(node.id));
    abort(
      "unreachable",
      `LAD node ${missing?.id ?? "unknown"} is not reachable from the power source.`,
    );
  }
};

const validateBranches = (network: ParsedNetwork, index: GraphIndex): void => {
  for (const branch of network.branches) {
    const split = network.nodeById.get(branch.splitNodeId);
    const join = network.nodeById.get(branch.joinNodeId);
    if (split === undefined || split.kind !== "branch-split" || split.branchId !== branch.id) {
      abort("invalid-branch", `LAD branch ${branch.id} does not own a matching split and join.`);
    }
    if (
      join === undefined ||
      join.kind !== "branch-join" ||
      join.branchId !== branch.id ||
      split.id === join.id
    ) {
      abort("invalid-branch", `LAD branch ${branch.id} does not own a matching split and join.`);
    }
    if (branch.paths.length < 2) {
      abort("invalid-branch", `LAD branch ${branch.id} must contain at least two paths.`);
    }
    const entryIds = new Set<string>();
    const exitIds = new Set<string>();
    for (const path of branch.paths) {
      const entry = index.edgeById.get(path.entryEdgeId);
      const exit = index.edgeById.get(path.exitEdgeId);
      if (entry === undefined || entry.sourceNodeId !== split.id) {
        abort("invalid-branch", `LAD path ${path.id} is not bounded by its canonical split and join.`);
      }
      if (exit === undefined || exit.targetNodeId !== join.id) {
        abort("invalid-branch", `LAD path ${path.id} is not bounded by its canonical split and join.`);
      }
      if (
        path.entryEdgeId === path.exitEdgeId ||
        entry.targetNodeId === join.id ||
        entryIds.has(path.entryEdgeId) ||
        exitIds.has(path.exitEdgeId)
      ) {
        abort("invalid-branch", `LAD path ${path.id} is empty or reuses a branch boundary edge.`);
      }
      entryIds.add(path.entryEdgeId);
      exitIds.add(path.exitEdgeId);
    }
  }

  for (const node of network.nodes) {
    if (
      (node.kind === "branch-split" || node.kind === "branch-join") &&
      (node.branchId === null || !network.branchById.has(node.branchId))
    ) {
      abort("invalid-branch", `LAD ${node.kind} node ${node.id} has no canonical branch metadata.`);
    }
  }
};

const validateNodeArities = (network: ParsedNetwork, index: GraphIndex): void => {
  for (const node of network.nodes) {
    const incoming = index.incoming.get(node.id)?.length ?? 0;
    const outgoing = index.outgoing.get(node.id)?.length ?? 0;
    const inputPorts = node.ports.filter((port) => port.direction === "input").length;
    const outputPorts = node.ports.length - inputPorts;
    const branchPathCount = node.branchId === null
      ? 0
      : network.branchById.get(node.branchId)?.paths.length ?? 0;
    let valid = false;
    switch (node.kind) {
      case "power-source":
        valid = incoming === 0 && outgoing === 1;
        break;
      case "branch-split":
        valid = incoming === 1 && branchPathCount >= 2 && outgoing === branchPathCount;
        break;
      case "branch-join":
        valid = outgoing === 1 && branchPathCount >= 2 && incoming === branchPathCount;
        break;
      case "coil":
      case "return":
        valid = incoming === 1 && outgoing === 0;
        break;
      case "box":
      case "call":
      case "contact":
      case "unsupported-control":
      case "unresolved":
        valid = incoming === 1 && outgoing === 1;
        break;
    }
    if (!valid || inputPorts !== incoming || outputPorts !== outgoing) {
      abort("invalid-node", `LAD node ${node.id} has an illegal power-flow arity.`);
    }
  }
};

const projectTopology = (network: ParsedNetwork, index: GraphIndex): LadNetworkTopology => {
  const source = network.nodes.find((node) => node.kind === "power-source");
  if (source === undefined) {
    abort("invalid-network", "The LAD network has no power source to project.");
  }
  const state: ProjectionState = { edges: new Set(), nodes: new Set() };
  const projected = projectSeries(network, index, source.id, null, null, 0, state);
  const terminalNodeId = projected.terminalNodeId;
  if (terminalNodeId === null) {
    abort("open-graph", "The LAD network does not end at a terminal node.");
  }
  if (state.nodes.size !== network.nodes.length || state.edges.size !== network.edges.length) {
    abort("unreachable", "The LAD topology does not cover every canonical node and edge exactly once.");
  }
  return {
    items: projected.items,
    kind: "network",
    networkId: network.id,
    semanticOrder: network.semanticOrder,
    sourceNodeId: source.id,
    terminalNodeId,
  };
};

const projectSeries = (
  network: ParsedNetwork,
  index: GraphIndex,
  startNodeId: string,
  stopNodeId: string | null,
  initialEdgeId: string | null,
  depth: number,
  state: ProjectionState,
): ProjectedSeries => {
  if (depth > LAD_TOPOLOGY_LIMITS.maxNestingDepth) {
    abort(
      "resource-limit",
      `LAD branch nesting exceeds the limit of ${LAD_TOPOLOGY_LIMITS.maxNestingDepth}.`,
    );
  }
  const items: LadTopologyItem[] = [];
  let currentNodeId = startNodeId;
  let incomingEdgeId = initialEdgeId;
  let steps = 0;

  while (currentNodeId !== stopNodeId) {
    steps += 1;
    if (steps > network.nodes.length + 1) {
      abort("cycle", "The LAD topology traversal exceeded the finite node bound.");
    }
    const node = network.nodeById.get(currentNodeId);
    if (node === undefined) {
      abort("open-graph", `The LAD topology reaches an unknown node ${currentNodeId}.`);
    }
    if (state.nodes.has(node.id)) {
      abort("invalid-branch", `LAD node ${node.id} is represented by more than one topology path.`);
    }
    const canonicalIncoming = onlyEdge(index.incoming.get(node.id));
    if ((canonicalIncoming?.id ?? null) !== incomingEdgeId) {
      abort("invalid-branch", `LAD node ${node.id} is entered through a non-canonical path edge.`);
    }

    if (node.kind === "branch-join") {
      abort("invalid-branch", `LAD branch join ${node.id} was reached outside its owning branch.`);
    }
    if (node.kind === "branch-split") {
      if (incomingEdgeId === null || node.branchId === null) {
        abort("invalid-branch", `LAD branch split ${node.id} has no canonical incoming edge.`);
      }
      const beforeBranchEdgeId = incomingEdgeId;
      const branchId = node.branchId;
      const branch = network.branchById.get(branchId);
      if (branch === undefined) {
        abort("invalid-branch", `LAD branch split ${node.id} has no branch metadata.`);
      }
      const join = network.nodeById.get(branch.joinNodeId);
      if (join === undefined || join.kind !== "branch-join" || state.nodes.has(join.id)) {
        abort("invalid-branch", `LAD branch ${branch.id} has an unavailable join node.`);
      }

      state.nodes.add(node.id);
      const paths: LadTopologyPath[] = [];
      for (const path of branch.paths) {
        const entry = requiredIndexedEdge(index, path.entryEdgeId, path.id);
        takeEdge(state, entry.id);
        const pathProjection = projectSeries(
          network,
          index,
          entry.targetNodeId,
          join.id,
          entry.id,
          depth + 1,
          state,
        );
        if (
          pathProjection.items.length === 0 ||
          pathProjection.arrivalEdgeId !== path.exitEdgeId ||
          pathProjection.terminalNodeId !== null
        ) {
          abort("invalid-branch", `LAD path ${path.id} does not terminate at its canonical exit edge.`);
        }
        paths.push({
          entryEdgeId: path.entryEdgeId,
          exitEdgeId: path.exitEdgeId,
          items: pathProjection.items,
          kind: "path",
          pathId: path.id,
        });
      }

      state.nodes.add(join.id);
      const afterJoin = onlyEdge(index.outgoing.get(join.id));
      if (afterJoin === null) {
        abort("open-graph", `LAD branch join ${join.id} has no outgoing edge.`);
      }
      takeEdge(state, afterJoin.id);
      items.push({
        afterEdgeId: afterJoin.id,
        beforeEdgeId: beforeBranchEdgeId,
        branchId: branch.id,
        joinNode: join.fields,
        joinNodeId: join.id,
        kind: "parallel",
        paths,
        splitNode: node.fields,
        splitNodeId: node.id,
      });
      currentNodeId = afterJoin.targetNodeId;
      incomingEdgeId = afterJoin.id;
      continue;
    }

    const elementKind = node.kind;
    state.nodes.add(node.id);
    const outgoing = onlyEdge(index.outgoing.get(node.id));
    if (outgoing !== null) {
      takeEdge(state, outgoing.id);
    }
    items.push({
      afterEdgeId: outgoing?.id ?? null,
      beforeEdgeId: incomingEdgeId,
      kind: "element",
      node: node.fields,
      nodeId: node.id,
      nodeKind: elementKind,
      semanticOrder: node.semanticOrder,
    });

    if (outgoing === null) {
      if (stopNodeId !== null) {
        abort("open-graph", `LAD path terminates before branch join ${stopNodeId}.`);
      }
      if (node.kind !== "coil" && node.kind !== "return") {
        abort("open-graph", `LAD node ${node.id} ends the network without terminal semantics.`);
      }
      return { arrivalEdgeId: null, items, terminalNodeId: node.id };
    }
    currentNodeId = outgoing.targetNodeId;
    incomingEdgeId = outgoing.id;
  }

  return { arrivalEdgeId: incomingEdgeId, items, terminalNodeId: null };
};

const hasCycle = (network: ParsedNetwork, index: GraphIndex): boolean => {
  const indegree = new Map<string, number>();
  const pending: string[] = [];
  for (const node of network.nodes) {
    const count = index.incoming.get(node.id)?.length ?? 0;
    indegree.set(node.id, count);
    if (count === 0) {
      pending.push(node.id);
    }
  }
  let cursor = 0;
  let visited = 0;
  while (cursor < pending.length) {
    const nodeId = pending[cursor];
    cursor += 1;
    if (nodeId === undefined) {
      abort("invalid-network", "The LAD cycle index became inconsistent.");
    }
    visited += 1;
    for (const edge of index.outgoing.get(nodeId) ?? []) {
      const next = (indegree.get(edge.targetNodeId) ?? 0) - 1;
      indegree.set(edge.targetNodeId, next);
      if (next === 0) {
        pending.push(edge.targetNodeId);
      }
    }
  }
  return visited !== network.nodes.length;
};

const reachableNodes = (sourceNodeId: string, index: GraphIndex): ReadonlySet<string> => {
  const reached = new Set<string>();
  const pending = [sourceNodeId];
  while (pending.length > 0) {
    const nodeId = pending.pop();
    if (nodeId === undefined || reached.has(nodeId)) {
      continue;
    }
    reached.add(nodeId);
    for (const edge of index.outgoing.get(nodeId) ?? []) {
      pending.push(edge.targetNodeId);
    }
  }
  return reached;
};

const requiredIndexedEdge = (
  index: GraphIndex,
  edgeId: string,
  pathId: string,
): IndexedEdge => {
  const edge = index.edgeById.get(edgeId);
  if (edge === undefined) {
    abort("invalid-branch", `LAD path ${pathId} references unknown edge ${edgeId}.`);
  }
  return edge;
};

const takeEdge = (state: ProjectionState, edgeId: string): void => {
  if (state.edges.has(edgeId)) {
    abort("invalid-branch", `LAD edge ${edgeId} is represented by more than one topology path.`);
  }
  state.edges.add(edgeId);
};

const onlyEdge = (edges: readonly IndexedEdge[] | undefined): IndexedEdge | null =>
  edges?.[0] ?? null;

const pushIndex = (
  index: Map<string, IndexedEdge[]>,
  nodeId: string,
  edge: IndexedEdge,
): void => {
  const values = index.get(nodeId);
  if (values === undefined) {
    index.set(nodeId, [edge]);
  } else {
    values.push(edge);
  }
};

const payloadFields = (value: unknown, path: string): ProjectPayload => {
  if (!isRecordObject(value)) {
    abort("invalid-network", `${path} must be a canonical payload record.`);
  }
  return value as ProjectPayload;
};

const recordFields = (value: unknown, path: string): ProjectPayload => {
  if (!isRecordObject(value) || value.$type !== "record") {
    abort("invalid-network", `${path} must be a canonical typed record.`);
  }
  const fields = value.value;
  if (!isRecordObject(fields)) {
    abort("invalid-network", `${path} must contain canonical record fields.`);
  }
  return fields as ProjectPayload;
};

const boundedArray = (
  value: unknown,
  maximum: number,
  path: string,
  label: string,
): readonly unknown[] => {
  if (!Array.isArray(value)) {
    abort("invalid-network", `${path} must be a canonical array.`);
  }
  const values: readonly unknown[] = value;
  if (values.length > maximum) {
    abort("resource-limit", `${label} exceed the limit of ${maximum}.`);
  }
  return values;
};

const identityField = (fields: ProjectPayload, key: string, path: string): string => {
  const value = fields[key];
  if (typeof value === "string" && UUID_PATTERN.test(value)) {
    return value;
  }
  return abort("invalid-network", `${path}.${key} must be a canonical UUID identity.`);
};

const unsignedField = (
  fields: ProjectPayload,
  key: string,
  maximum: number,
  path: string,
): number => {
  const value = fields[key];
  if (!isRecordObject(value) || value.$type !== "u64") {
    abort("invalid-network", `${path}.${key} must be a canonical unsigned integer.`);
  }
  const encoded = value.value;
  if (typeof encoded !== "string" || !/^[0-9]+$/u.test(encoded) || encoded.length > 10) {
    abort("invalid-network", `${path}.${key} must be a canonical unsigned integer.`);
  }
  const parsed = Number(encoded);
  if (!Number.isSafeInteger(parsed) || parsed < 0 || parsed > maximum) {
    abort("invalid-network", `${path}.${key} is outside the supported unsigned range.`);
  }
  return parsed;
};

const textField = (fields: ProjectPayload, key: string, path: string): string => {
  const value = fields[key];
  if (typeof value === "string") {
    return value;
  }
  return abort("invalid-node", `${path}.${key} must be canonical text.`);
};

const requireArrayField = (fields: ProjectPayload, key: string, path: string): void => {
  if (!Array.isArray(fields[key])) {
    abort("invalid-node", `${path}.${key} must be a canonical array.`);
  }
};

const nodeKindField = (fields: ProjectPayload, path: string): ParsedNodeKind => {
  const value = fields.nodeKind;
  switch (value) {
    case "power-source":
    case "contact":
    case "coil":
    case "box":
    case "call":
    case "branch-split":
    case "branch-join":
    case "return":
    case "unsupported-control":
    case "unresolved":
      return value;
    default:
      return abort("invalid-node", `${path}.nodeKind is not supported by the canonical LAD schema.`);
  }
};

const isRecordObject = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

function abort(code: LadTopologyErrorCode, message: string): never {
  throw new ProjectionAbort(code, message);
}
