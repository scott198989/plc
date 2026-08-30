import {
  canonicalRecordFields,
  recordValue,
  unsignedValue,
} from "./canonical-authoring";
import type {
  ProjectPayload,
  ProjectPayloadValue,
} from "./workbench-types";

export type LadContactMode = "normally-open" | "normally-closed";
export type LadCoilMode = "normal" | "negated" | "set" | "reset";
export type LadIdFactory = () => string;

export type LadAuthoringErrorCode =
  | "ambiguous-connection"
  | "edge-not-found"
  | "id-exhausted"
  | "invalid-binding"
  | "invalid-graph"
  | "invalid-request"
  | "last-network"
  | "network-not-found"
  | "node-not-found"
  | "not-a-coil"
  | "not-a-contact"
  | "would-empty-branch-path";

export type LadAuthoringResult =
  | Readonly<{
      createdIds: readonly string[];
      graph: ProjectPayloadValue;
      ok: true;
    }>
  | Readonly<{
      code: LadAuthoringErrorCode;
      message: string;
      ok: false;
    }>;

export type InsertSeriesContactRequest = Readonly<{
  edgeId: string;
  idFactory?: LadIdFactory;
  memberId: string;
  mode?: LadContactMode;
  networkId: string;
}>;

export type WrapContactWithParallelContactRequest = Readonly<{
  contactNodeId: string;
  idFactory?: LadIdFactory;
  memberId: string;
  mode?: LadContactMode;
  networkId: string;
}>;

export type RemoveContactRequest = Readonly<{
  contactNodeId: string;
  idFactory?: LadIdFactory;
  networkId: string;
}>;

export type AddLadNetworkRequest = Readonly<{
  coilMemberId: string;
  coilMode?: LadCoilMode;
  idFactory?: LadIdFactory;
}>;

export type RemoveLadNetworkRequest = Readonly<{
  networkId: string;
}>;

export type UpdateLadContactRequest = Readonly<{
  contactNodeId: string;
  idFactory?: LadIdFactory;
  memberId?: string;
  mode?: LadContactMode;
  networkId: string;
}>;

export type UpdateLadCoilRequest = Readonly<{
  coilNodeId: string;
  idFactory?: LadIdFactory;
  memberId?: string;
  mode?: LadCoilMode;
  networkId: string;
}>;

type ParsedGraph = Readonly<{
  fields: ProjectPayload;
  networks: readonly ParsedNetwork[];
}>;

type ParsedNetwork = Readonly<{
  branches: readonly ParsedBranch[];
  edges: readonly ParsedEdge[];
  fields: ProjectPayload;
  id: string;
  nodes: readonly ParsedNode[];
}>;

type ParsedNode = Readonly<{
  fields: ProjectPayload;
  id: string;
  kind: string;
  ports: readonly ParsedPort[];
}>;

type ParsedPort = Readonly<{
  direction: "input" | "output";
  fields: ProjectPayload;
  id: string;
}>;

type ParsedEdge = Readonly<{
  fields: ProjectPayload;
  id: string;
  sourcePortId: string;
  targetPortId: string;
}>;

type ParsedBranch = Readonly<{
  fields: ProjectPayload;
  id: string;
  paths: readonly ParsedBranchPath[];
}>;

type ParsedBranchPath = Readonly<{
  entryEdgeId: string;
  exitEdgeId: string;
  fields: ProjectPayload;
  id: string;
}>;

const LAD_GRAPH_SCHEMA = "edu.lad-semantic-graph/1";
const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/iu;
const MAX_ID_ATTEMPTS = 32;
const MAX_UNSIGNED_64 = BigInt("18446744073709551615");

type SixIds = readonly [string, string, string, string, string, string];
type SevenIds = readonly [string, string, string, string, string, string, string];
type TwentyOneIds = readonly [
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
  string,
];

export const insertSeriesContact = (
  graph: ProjectPayloadValue,
  request: InsertSeriesContactRequest,
): LadAuthoringResult => {
  if (!validIdentity(request.memberId) || !validContactMode(request.mode ?? "normally-open")) {
    return failure("invalid-binding", "A series contact requires a valid member identity and contact mode.");
  }
  const parsed = parseGraph(graph);
  if (parsed === null) {
    return invalidGraph();
  }
  const located = locateNetwork(parsed, request.networkId);
  if (located.ok === false) {
    return located.result;
  }
  const edge = uniqueById(located.network.edges, request.edgeId);
  if (edge === null) {
    return failure("edge-not-found", "The selected LAD power edge does not exist exactly once.");
  }
  const targetNodeIndex = located.network.nodes.findIndex((node) =>
    node.ports.some((port) => port.id === edge.targetPortId && port.direction === "input")
  );
  if (targetNodeIndex < 0) {
    return invalidGraph("The selected LAD edge does not terminate at one canonical input port.");
  }

  const allocator = createAllocator(graph, request.idFactory);
  const created = allocator.takeMany(6);
  if (created === null) {
    return idExhausted();
  }
  const [nodeId, operandId, inputPortId, outputPortId, upstreamEdgeId, downstreamEdgeId] =
    created as SixIds;
  const contact = contactNode(
    nodeId,
    operandId,
    inputPortId,
    outputPortId,
    request.memberId,
    request.mode ?? "normally-open",
  );
  const replacementEdges = [
    powerEdge(upstreamEdgeId, edge.sourcePortId, inputPortId),
    powerEdge(downstreamEdgeId, outputPortId, edge.targetPortId),
  ];
  const nodes = [
    ...located.network.nodes.slice(0, targetNodeIndex).map((node) => recordValue(node.fields)),
    contact,
    ...located.network.nodes.slice(targetNodeIndex).map((node) => recordValue(node.fields)),
  ];
  const edges = [
    ...located.network.edges
      .filter((candidate) => candidate.id !== edge.id)
      .map((candidate) => recordValue(candidate.fields)),
    ...replacementEdges,
  ];
  const branches = rewriteBranchEdgeReferences(
    located.network.branches,
    new Map([[edge.id, { entry: upstreamEdgeId, exit: downstreamEdgeId }]]),
  );
  if (branches === null) {
    return invalidGraph("The selected LAD edge is referenced by malformed branch metadata.");
  }

  return replaceNetwork(
    parsed,
    located.index,
    located.network.fields,
    nodes,
    edges,
    branches,
    created,
  );
};

export const wrapContactWithParallelContact = (
  graph: ProjectPayloadValue,
  request: WrapContactWithParallelContactRequest,
): LadAuthoringResult => {
  if (!validIdentity(request.memberId) || !validContactMode(request.mode ?? "normally-open")) {
    return failure("invalid-binding", "A parallel contact requires a valid member identity and contact mode.");
  }
  const parsed = parseGraph(graph);
  if (parsed === null) {
    return invalidGraph();
  }
  const located = locateNetwork(parsed, request.networkId);
  if (located.ok === false) {
    return located.result;
  }
  const contactIndex = located.network.nodes.findIndex((node) => node.id === request.contactNodeId);
  if (contactIndex < 0) {
    return failure("node-not-found", "The selected LAD contact does not exist.");
  }
  const selected = located.network.nodes[contactIndex];
  if (selected === undefined) {
    return invalidGraph("The selected LAD contact index could not be resolved.");
  }
  if (selected.kind !== "contact") {
    return failure("not-a-contact", "Only a LAD contact can be wrapped with a parallel contact.");
  }
  const connection = singleNodeConnection(located.network, selected);
  if (connection === null) {
    return failure(
      "ambiguous-connection",
      "The selected contact must have exactly one incoming and one outgoing power edge.",
    );
  }

  const allocator = createAllocator(graph, request.idFactory);
  const created = allocator.takeMany(21);
  if (created === null) {
    return idExhausted();
  }
  const [
    branchId,
    splitNodeId,
    splitInputPortId,
    splitFirstOutputPortId,
    splitSecondOutputPortId,
    parallelNodeId,
    parallelOperandId,
    parallelInputPortId,
    parallelOutputPortId,
    joinNodeId,
    joinFirstInputPortId,
    joinSecondInputPortId,
    joinOutputPortId,
    pathOneId,
    pathTwoId,
    beforeSplitEdgeId,
    firstEntryEdgeId,
    firstExitEdgeId,
    secondEntryEdgeId,
    secondExitEdgeId,
    afterJoinEdgeId,
  ] = created as TwentyOneIds;

  const split = recordValue({
    branchId,
    id: splitNodeId,
    nodeKind: "branch-split",
    powerPorts: [
      powerPort(splitInputPortId, "input"),
      powerPort(splitFirstOutputPortId, "output"),
      powerPort(splitSecondOutputPortId, "output"),
    ],
    semanticOrder: unsignedValue(0),
  });
  const parallel = contactNode(
    parallelNodeId,
    parallelOperandId,
    parallelInputPortId,
    parallelOutputPortId,
    request.memberId,
    request.mode ?? "normally-open",
  );
  const join = recordValue({
    branchId,
    id: joinNodeId,
    nodeKind: "branch-join",
    powerPorts: [
      powerPort(joinFirstInputPortId, "input"),
      powerPort(joinSecondInputPortId, "input"),
      powerPort(joinOutputPortId, "output"),
    ],
    semanticOrder: unsignedValue(0),
  });
  const nodes = [
    ...located.network.nodes.slice(0, contactIndex).map((node) => recordValue(node.fields)),
    split,
    recordValue(selected.fields),
    parallel,
    join,
    ...located.network.nodes.slice(contactIndex + 1).map((node) => recordValue(node.fields)),
  ];
  const edges = [
    ...located.network.edges
      .filter((edge) => edge.id !== connection.incoming.id && edge.id !== connection.outgoing.id)
      .map((edge) => recordValue(edge.fields)),
    powerEdge(beforeSplitEdgeId, connection.incoming.sourcePortId, splitInputPortId),
    powerEdge(firstEntryEdgeId, splitFirstOutputPortId, connection.inputPortId),
    powerEdge(firstExitEdgeId, connection.outputPortId, joinFirstInputPortId),
    powerEdge(secondEntryEdgeId, splitSecondOutputPortId, parallelInputPortId),
    powerEdge(secondExitEdgeId, parallelOutputPortId, joinSecondInputPortId),
    powerEdge(afterJoinEdgeId, joinOutputPortId, connection.outgoing.targetPortId),
  ];
  const rewrittenBranches = rewriteBranchEdgeReferences(
    located.network.branches,
    new Map([
      [connection.incoming.id, { entry: beforeSplitEdgeId, exit: beforeSplitEdgeId }],
      [connection.outgoing.id, { entry: afterJoinEdgeId, exit: afterJoinEdgeId }],
    ]),
  );
  if (rewrittenBranches === null) {
    return invalidGraph("The selected LAD contact is referenced by malformed branch metadata.");
  }
  const branches = [
    ...rewrittenBranches,
    recordValue({
      id: branchId,
      joinNodeId,
      paths: [
        recordValue({
          entryEdgeId: firstEntryEdgeId,
          exitEdgeId: firstExitEdgeId,
          id: pathOneId,
        }),
        recordValue({
          entryEdgeId: secondEntryEdgeId,
          exitEdgeId: secondExitEdgeId,
          id: pathTwoId,
        }),
      ],
      splitNodeId,
    }),
  ];

  return replaceNetwork(
    parsed,
    located.index,
    located.network.fields,
    nodes,
    edges,
    branches,
    created,
  );
};

export const removeContactAndReconnect = (
  graph: ProjectPayloadValue,
  request: RemoveContactRequest,
): LadAuthoringResult => {
  const parsed = parseGraph(graph);
  if (parsed === null) {
    return invalidGraph();
  }
  const located = locateNetwork(parsed, request.networkId);
  if (located.ok === false) {
    return located.result;
  }
  const selected = uniqueById(located.network.nodes, request.contactNodeId);
  if (selected === null) {
    return failure("node-not-found", "The selected LAD contact does not exist exactly once.");
  }
  if (selected.kind !== "contact") {
    return failure("not-a-contact", "Only a LAD contact can be removed by this operation.");
  }
  const connection = singleNodeConnection(located.network, selected);
  if (connection === null) {
    return failure(
      "ambiguous-connection",
      "The selected contact must have exactly one incoming and one outgoing power edge.",
    );
  }
  const emptiesPath = located.network.branches.some((branch) =>
    branch.paths.some((path) =>
      path.entryEdgeId === connection.incoming.id && path.exitEdgeId === connection.outgoing.id
    )
  );
  if (emptiesPath) {
    return failure(
      "would-empty-branch-path",
      "Removing this contact would leave a zero-element parallel path; remove or collapse the branch instead.",
    );
  }

  const allocator = createAllocator(graph, request.idFactory);
  const replacementEdgeId = allocator.take();
  if (replacementEdgeId === null) {
    return idExhausted();
  }
  const branches = rewriteBranchEdgeReferences(
    located.network.branches,
    new Map([
      [connection.incoming.id, { entry: replacementEdgeId, exit: replacementEdgeId }],
      [connection.outgoing.id, { entry: replacementEdgeId, exit: replacementEdgeId }],
    ]),
  );
  if (branches === null) {
    return invalidGraph("The selected LAD contact is referenced by malformed branch metadata.");
  }
  const nodes = located.network.nodes
    .filter((node) => node.id !== selected.id)
    .map((node) => recordValue(node.fields));
  const edges = [
    ...located.network.edges
      .filter((edge) => edge.id !== connection.incoming.id && edge.id !== connection.outgoing.id)
      .map((edge) => recordValue(edge.fields)),
    powerEdge(replacementEdgeId, connection.incoming.sourcePortId, connection.outgoing.targetPortId),
  ];

  return replaceNetwork(
    parsed,
    located.index,
    located.network.fields,
    nodes,
    edges,
    branches,
    [replacementEdgeId],
  );
};

export const addLadNetwork = (
  graph: ProjectPayloadValue,
  request: AddLadNetworkRequest,
): LadAuthoringResult => {
  if (!validIdentity(request.coilMemberId) || !validCoilMode(request.coilMode ?? "normal")) {
    return failure("invalid-binding", "A new LAD network requires a valid coil member and mode.");
  }
  const parsed = parseGraph(graph);
  if (parsed === null) {
    return invalidGraph();
  }
  const allocator = createAllocator(graph, request.idFactory);
  const created = allocator.takeMany(7);
  if (created === null) {
    return idExhausted();
  }
  const [
    networkId,
    sourceNodeId,
    sourceOutputPortId,
    coilNodeId,
    coilOperandId,
    coilInputPortId,
    edgeId,
  ] = created as SevenIds;
  const network = recordValue({
    branches: [],
    edges: [powerEdge(edgeId, sourceOutputPortId, coilInputPortId)],
    id: networkId,
    nodes: [
      recordValue({
        id: sourceNodeId,
        nodeKind: "power-source",
        powerPorts: [powerPort(sourceOutputPortId, "output")],
        semanticOrder: unsignedValue(0),
      }),
      coilNode(
        coilNodeId,
        coilOperandId,
        coilInputPortId,
        request.coilMemberId,
        request.coilMode ?? "normal",
      ),
    ],
    semanticOrder: unsignedValue(parsed.networks.length),
  });
  return replaceNetworks(
    parsed,
    [...parsed.networks.map((value) => recordValue(value.fields)), network],
    created,
  );
};

export const removeLadNetwork = (
  graph: ProjectPayloadValue,
  request: RemoveLadNetworkRequest,
): LadAuthoringResult => {
  const parsed = parseGraph(graph);
  if (parsed === null) {
    return invalidGraph();
  }
  const matches = parsed.networks.filter((network) => network.id === request.networkId);
  if (matches.length !== 1) {
    return failure("network-not-found", "The selected LAD network does not exist exactly once.");
  }
  if (parsed.networks.length === 1) {
    return failure("last-network", "A LAD document must retain at least one network.");
  }
  return replaceNetworks(
    parsed,
    parsed.networks
      .filter((network) => network.id !== request.networkId)
      .map((network) => recordValue(network.fields)),
    [],
  );
};

export const updateLadContact = (
  graph: ProjectPayloadValue,
  request: UpdateLadContactRequest,
): LadAuthoringResult => updateBoundNode(graph, {
  idFactory: request.idFactory,
  memberId: request.memberId,
  mode: request.mode,
  networkId: request.networkId,
  nodeId: request.contactNodeId,
  nodeKind: "contact",
});

export const updateLadCoil = (
  graph: ProjectPayloadValue,
  request: UpdateLadCoilRequest,
): LadAuthoringResult => updateBoundNode(graph, {
  idFactory: request.idFactory,
  memberId: request.memberId,
  mode: request.mode,
  networkId: request.networkId,
  nodeId: request.coilNodeId,
  nodeKind: "coil",
});

const updateBoundNode = (
  graph: ProjectPayloadValue,
  request: Readonly<{
    idFactory: LadIdFactory | undefined;
    memberId: string | undefined;
    mode: LadContactMode | LadCoilMode | undefined;
    networkId: string;
    nodeId: string;
    nodeKind: "coil" | "contact";
  }>,
): LadAuthoringResult => {
  if (request.memberId === undefined && request.mode === undefined) {
    return failure("invalid-request", "A LAD node update must change its binding, mode, or both.");
  }
  if (request.memberId !== undefined && !validIdentity(request.memberId)) {
    return failure("invalid-binding", "The LAD operand member identity is invalid.");
  }
  if (
    request.mode !== undefined &&
    (request.nodeKind === "contact"
      ? !validContactMode(request.mode)
      : !validCoilMode(request.mode))
  ) {
    return failure("invalid-request", `The requested LAD ${request.nodeKind} mode is invalid.`);
  }
  const parsed = parseGraph(graph);
  if (parsed === null) {
    return invalidGraph();
  }
  const located = locateNetwork(parsed, request.networkId);
  if (located.ok === false) {
    return located.result;
  }
  const selectedIndex = located.network.nodes.findIndex((node) => node.id === request.nodeId);
  if (selectedIndex < 0) {
    return failure("node-not-found", `The selected LAD ${request.nodeKind} does not exist.`);
  }
  const selected = located.network.nodes[selectedIndex];
  if (selected === undefined) {
    return invalidGraph(`The selected LAD ${request.nodeKind} index could not be resolved.`);
  }
  if (selected.kind !== request.nodeKind) {
    return failure(
      request.nodeKind === "contact" ? "not-a-contact" : "not-a-coil",
      `The selected LAD node is not a ${request.nodeKind}.`,
    );
  }

  let operand = selected.fields.operand;
  const createdIds: string[] = [];
  if (request.memberId !== undefined) {
    const existing = canonicalRecordFields(operand);
    if (existing === null && operand !== undefined && operand !== null) {
      return invalidGraph("The selected LAD operand is not a canonical record.");
    }
    if (existing !== null && existing.kind !== "caller-member") {
      return failure("invalid-binding", "This editor only updates caller-member LAD operands.");
    }
    let operandId = existing === null ? null : identityField(existing, "id");
    if (existing !== null && operandId === null) {
      return invalidGraph("The selected LAD operand identity is malformed.");
    }
    if (operandId === null) {
      const allocator = createAllocator(graph, request.idFactory);
      operandId = allocator.take();
      if (operandId === null) {
        return idExhausted();
      }
      createdIds.push(operandId);
    }
    operand = recordValue({
      ...(existing ?? {}),
      id: operandId,
      kind: "caller-member",
      memberId: request.memberId,
    });
  }

  const updatedNode = recordValue({
    ...selected.fields,
    ...(request.mode === undefined ? {} : { mode: request.mode }),
    ...(request.memberId === undefined ? {} : { operand: operand ?? null }),
  });
  const nodes = located.network.nodes.map((node, index) =>
    index === selectedIndex ? updatedNode : recordValue(node.fields)
  );
  return replaceNetwork(
    parsed,
    located.index,
    located.network.fields,
    nodes,
    located.network.edges.map((edge) => recordValue(edge.fields)),
    located.network.branches.map((branch) => recordValue(branch.fields)),
    createdIds,
  );
};

const parseGraph = (graph: ProjectPayloadValue): ParsedGraph | null => {
  const fields = canonicalRecordFields(graph);
  if (fields === null || fields.schema !== LAD_GRAPH_SCHEMA || !Array.isArray(fields.networks)) {
    return null;
  }
  const networks = fields.networks.map(parseNetwork);
  if (networks.some((network) => network === null)) {
    return null;
  }
  const values = networks.filter((network): network is ParsedNetwork => network !== null);
  return unique(values.map((network) => network.id)) ? { fields, networks: values } : null;
};

const parseNetwork = (value: ProjectPayloadValue): ParsedNetwork | null => {
  const fields = canonicalRecordFields(value);
  const id = fields === null ? null : identityField(fields, "id");
  if (
    fields === null ||
    id === null ||
    !Array.isArray(fields.nodes) ||
    !Array.isArray(fields.edges) ||
    (fields.branches !== undefined && !Array.isArray(fields.branches))
  ) {
    return null;
  }
  const nodes = fields.nodes.map(parseNode);
  const edges = fields.edges.map(parseEdge);
  const branches = (Array.isArray(fields.branches) ? fields.branches : []).map(parseBranch);
  if (
    nodes.some((node) => node === null) ||
    edges.some((edge) => edge === null) ||
    branches.some((branch) => branch === null)
  ) {
    return null;
  }
  const parsedNodes = nodes.filter((node): node is ParsedNode => node !== null);
  const parsedEdges = edges.filter((edge): edge is ParsedEdge => edge !== null);
  const parsedBranches = branches.filter((branch): branch is ParsedBranch => branch !== null);
  const ports = parsedNodes.flatMap((node) => node.ports);
  if (
    !unique(parsedNodes.map((node) => node.id)) ||
    !unique(parsedEdges.map((edge) => edge.id)) ||
    !unique(parsedBranches.map((branch) => branch.id)) ||
    !unique(ports.map((port) => port.id))
  ) {
    return null;
  }
  const portById = new Map(ports.map((port) => [port.id, port] as const));
  if (parsedEdges.some((edge) =>
    portById.get(edge.sourcePortId)?.direction !== "output" ||
    portById.get(edge.targetPortId)?.direction !== "input"
  )) {
    return null;
  }
  const edgeIds = new Set(parsedEdges.map((edge) => edge.id));
  if (parsedBranches.some((branch) => branch.paths.some((path) =>
    !edgeIds.has(path.entryEdgeId) || !edgeIds.has(path.exitEdgeId)
  ))) {
    return null;
  }
  return { branches: parsedBranches, edges: parsedEdges, fields, id, nodes: parsedNodes };
};

const parseNode = (value: ProjectPayloadValue): ParsedNode | null => {
  const fields = canonicalRecordFields(value);
  const id = fields === null ? null : identityField(fields, "id");
  if (
    fields === null ||
    id === null ||
    typeof fields.nodeKind !== "string" ||
    !Array.isArray(fields.powerPorts)
  ) {
    return null;
  }
  const ports = fields.powerPorts.map(parsePort);
  if (ports.some((port) => port === null)) {
    return null;
  }
  const parsedPorts = ports.filter((port): port is ParsedPort => port !== null);
  return unique(parsedPorts.map((port) => port.id))
    ? { fields, id, kind: fields.nodeKind, ports: parsedPorts }
    : null;
};

const parsePort = (value: ProjectPayloadValue): ParsedPort | null => {
  const fields = canonicalRecordFields(value);
  const id = fields === null ? null : identityField(fields, "id");
  const direction = fields?.direction;
  return fields !== null && id !== null && (direction === "input" || direction === "output")
    ? { direction, fields, id }
    : null;
};

const parseEdge = (value: ProjectPayloadValue): ParsedEdge | null => {
  const fields = canonicalRecordFields(value);
  const id = fields === null ? null : identityField(fields, "id");
  const sourcePortId = fields === null ? null : identityField(fields, "sourcePortId");
  const targetPortId = fields === null ? null : identityField(fields, "targetPortId");
  return fields !== null && id !== null && sourcePortId !== null && targetPortId !== null
    ? { fields, id, sourcePortId, targetPortId }
    : null;
};

const parseBranch = (value: ProjectPayloadValue): ParsedBranch | null => {
  const fields = canonicalRecordFields(value);
  const id = fields === null ? null : identityField(fields, "id");
  if (
    fields === null ||
    id === null ||
    identityField(fields, "splitNodeId") === null ||
    identityField(fields, "joinNodeId") === null ||
    !Array.isArray(fields.paths)
  ) {
    return null;
  }
  const paths = fields.paths.map(parseBranchPath);
  if (paths.some((path) => path === null)) {
    return null;
  }
  const parsedPaths = paths.filter((path): path is ParsedBranchPath => path !== null);
  return unique(parsedPaths.map((path) => path.id)) ? { fields, id, paths: parsedPaths } : null;
};

const parseBranchPath = (value: ProjectPayloadValue): ParsedBranchPath | null => {
  const fields = canonicalRecordFields(value);
  const id = fields === null ? null : identityField(fields, "id");
  const entryEdgeId = fields === null ? null : identityField(fields, "entryEdgeId");
  const exitEdgeId = fields === null ? null : identityField(fields, "exitEdgeId");
  return fields !== null && id !== null && entryEdgeId !== null && exitEdgeId !== null
    ? { entryEdgeId, exitEdgeId, fields, id }
    : null;
};

const locateNetwork = (
  graph: ParsedGraph,
  networkId: string,
): Readonly<{ index: number; network: ParsedNetwork; ok: true }> | Readonly<{
  ok: false;
  result: LadAuthoringResult;
}> => {
  const matches = graph.networks.flatMap((network, index) =>
    network.id === networkId ? [{ index, network }] : []
  );
  const match = matches[0];
  return matches.length === 1 && match !== undefined
    ? { ...match, ok: true }
    : {
        ok: false,
        result: failure("network-not-found", "The selected LAD network does not exist exactly once."),
      };
};

const singleNodeConnection = (
  network: ParsedNetwork,
  node: ParsedNode,
): Readonly<{
  incoming: ParsedEdge;
  inputPortId: string;
  outgoing: ParsedEdge;
  outputPortId: string;
}> | null => {
  const inputs = node.ports.filter((port) => port.direction === "input");
  const outputs = node.ports.filter((port) => port.direction === "output");
  if (inputs.length !== 1 || outputs.length !== 1) {
    return null;
  }
  const input = inputs[0];
  const output = outputs[0];
  if (input === undefined || output === undefined) {
    return null;
  }
  const incoming = network.edges.filter((edge) => edge.targetPortId === input.id);
  const outgoing = network.edges.filter((edge) => edge.sourcePortId === output.id);
  const incomingEdge = incoming[0];
  const outgoingEdge = outgoing[0];
  return incoming.length === 1 && outgoing.length === 1 &&
    incomingEdge !== undefined && outgoingEdge !== undefined
    ? {
        incoming: incomingEdge,
        inputPortId: input.id,
        outgoing: outgoingEdge,
        outputPortId: output.id,
      }
    : null;
};

const rewriteBranchEdgeReferences = (
  branches: readonly ParsedBranch[],
  replacements: ReadonlyMap<string, Readonly<{ entry: string; exit: string }>>,
): ProjectPayloadValue[] | null => {
  const rewritten: ProjectPayloadValue[] = [];
  for (const branch of branches) {
    const paths: ProjectPayloadValue[] = [];
    for (const path of branch.paths) {
      const entry = replacements.get(path.entryEdgeId)?.entry ?? path.entryEdgeId;
      const exit = replacements.get(path.exitEdgeId)?.exit ?? path.exitEdgeId;
      if (!validIdentity(entry) || !validIdentity(exit)) {
        return null;
      }
      paths.push(recordValue({ ...path.fields, entryEdgeId: entry, exitEdgeId: exit }));
    }
    rewritten.push(recordValue({ ...branch.fields, paths }));
  }
  return rewritten;
};

const replaceNetwork = (
  parsed: ParsedGraph,
  networkIndex: number,
  networkFields: ProjectPayload,
  nodes: readonly ProjectPayloadValue[],
  edges: readonly ProjectPayloadValue[],
  branches: readonly ProjectPayloadValue[],
  createdIds: readonly string[],
): LadAuthoringResult => {
  const normalizedNodes = nodes.map((node, index) => {
    const fields = canonicalRecordFields(node);
    return fields === null ? null : recordValue({ ...fields, semanticOrder: unsignedValue(index) });
  });
  if (normalizedNodes.some((node) => node === null)) {
    return invalidGraph("A LAD mutation produced a malformed node record.");
  }
  const updatedNetwork = recordValue({
    ...networkFields,
    branches,
    edges,
    nodes: normalizedNodes.filter((node) => node !== null),
  });
  const networks = parsed.networks.map((network, index) =>
    index === networkIndex ? updatedNetwork : recordValue(network.fields)
  );
  return replaceNetworks(parsed, networks, createdIds);
};

const replaceNetworks = (
  parsed: ParsedGraph,
  networks: readonly ProjectPayloadValue[],
  createdIds: readonly string[],
): LadAuthoringResult => {
  const normalized = networks.map((network, index) => {
    const fields = canonicalRecordFields(network);
    return fields === null ? null : recordValue({ ...fields, semanticOrder: unsignedValue(index) });
  });
  if (normalized.some((network) => network === null)) {
    return invalidGraph("A LAD mutation produced a malformed network record.");
  }
  const semanticRevision = incrementSemanticRevision(parsed.fields.semanticRevision);
  if (semanticRevision === null) {
    return invalidGraph("The LAD semantic revision is malformed or cannot be incremented.");
  }
  return {
    createdIds: [...createdIds],
    graph: recordValue({
      ...parsed.fields,
      networks: normalized.filter((network) => network !== null),
      semanticRevision,
    }),
    ok: true,
  };
};

const contactNode = (
  nodeId: string,
  operandId: string,
  inputPortId: string,
  outputPortId: string,
  memberId: string,
  mode: LadContactMode,
): ProjectPayloadValue => recordValue({
  id: nodeId,
  mode,
  nodeKind: "contact",
  operand: callerMemberOperand(operandId, memberId),
  powerPorts: [powerPort(inputPortId, "input"), powerPort(outputPortId, "output")],
  semanticOrder: unsignedValue(0),
});

const coilNode = (
  nodeId: string,
  operandId: string,
  inputPortId: string,
  memberId: string,
  mode: LadCoilMode,
): ProjectPayloadValue => recordValue({
  id: nodeId,
  mode,
  nodeKind: "coil",
  operand: callerMemberOperand(operandId, memberId),
  powerPorts: [powerPort(inputPortId, "input")],
  semanticOrder: unsignedValue(0),
});

const callerMemberOperand = (id: string, memberId: string): ProjectPayloadValue => recordValue({
  id,
  kind: "caller-member",
  memberId,
});

const powerPort = (
  id: string,
  direction: "input" | "output",
): ProjectPayloadValue => recordValue({ direction, id });

const powerEdge = (
  id: string,
  sourcePortId: string,
  targetPortId: string,
): ProjectPayloadValue => recordValue({ id, sourcePortId, targetPortId });

const createAllocator = (
  graph: ProjectPayloadValue,
  factory: LadIdFactory = () => crypto.randomUUID(),
): Readonly<{
  take: () => string | null;
  takeMany: (count: number) => readonly string[] | null;
}> => {
  const used = new Set<string>();
  collectIdentities(graph, used);
  const take = (): string | null => {
    for (let attempt = 0; attempt < MAX_ID_ATTEMPTS; attempt += 1) {
      let candidate: string;
      try {
        candidate = factory();
      } catch {
        return null;
      }
      if (validIdentity(candidate) && !used.has(candidate.toLocaleLowerCase("en-US"))) {
        used.add(candidate.toLocaleLowerCase("en-US"));
        return candidate;
      }
    }
    return null;
  };
  const takeMany = (count: number): readonly string[] | null => {
    const values: string[] = [];
    for (let index = 0; index < count; index += 1) {
      const value = take();
      if (value === null) {
        return null;
      }
      values.push(value);
    }
    return values;
  };
  return { take, takeMany };
};

const collectIdentities = (value: ProjectPayloadValue, output: Set<string>): void => {
  if (typeof value === "string") {
    if (validIdentity(value)) {
      output.add(value.toLocaleLowerCase("en-US"));
    }
    return;
  }
  if (value === null || typeof value === "boolean") {
    return;
  }
  if (Array.isArray(value)) {
    (value as readonly ProjectPayloadValue[]).forEach((entry) => collectIdentities(entry, output));
    return;
  }
  if ("$type" in value && value.$type === "record") {
    Object.values(value.value).forEach((entry) => collectIdentities(entry, output));
  }
};

const incrementSemanticRevision = (
  value: ProjectPayloadValue | undefined,
): Readonly<{ $type: "u64"; value: string }> | null => {
  if (
    typeof value !== "object" ||
    value === null ||
    Array.isArray(value) ||
    !("$type" in value) ||
    value.$type !== "u64" ||
    !/^(?:0|[1-9][0-9]*)$/u.test(value.value)
  ) {
    return null;
  }
  const parsed = BigInt(value.value);
  if (parsed >= MAX_UNSIGNED_64) {
    return null;
  }
  return { $type: "u64", value: (parsed + BigInt(1)).toString(10) };
};

const identityField = (fields: ProjectPayload, key: string): string | null => {
  const value = fields[key];
  return typeof value === "string" && validIdentity(value) ? value : null;
};

const validIdentity = (value: string): boolean => UUID_PATTERN.test(value);

const validContactMode = (value: string): value is LadContactMode =>
  value === "normally-open" || value === "normally-closed";

const validCoilMode = (value: string): value is LadCoilMode =>
  value === "normal" || value === "negated" || value === "set" || value === "reset";

const unique = (values: readonly string[]): boolean =>
  new Set(values.map((value) => value.toLocaleLowerCase("en-US"))).size === values.length;

const uniqueById = <T extends Readonly<{ id: string }>>(
  values: readonly T[],
  id: string,
): T | null => {
  const matches = values.filter((value) => value.id === id);
  return matches.length === 1 ? (matches[0] ?? null) : null;
};

const invalidGraph = (message = "The LAD graph is not a supported canonical graph."): LadAuthoringResult =>
  failure("invalid-graph", message);

const idExhausted = (): LadAuthoringResult =>
  failure("id-exhausted", "A unique canonical LAD identity could not be allocated.");

const failure = (code: LadAuthoringErrorCode, message: string): LadAuthoringResult => ({
  code,
  message,
  ok: false,
});
