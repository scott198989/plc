import {
  canonicalRecordFields,
  recordValue,
  unsignedValue,
} from "./canonical-authoring";
import {
  buildCanonicalLadBoxNode,
} from "./lad-instruction-catalog";
import type {
  BuildCanonicalLadBoxNodeRequest,
} from "./lad-instruction-catalog";
import type {
  ProjectPayload,
  ProjectPayloadValue,
} from "./workbench-types";

export type LadBoxIdFactory = () => string;

export type LadBoxAuthoringErrorCode =
  | "ambiguous-connection"
  | "box-factory-failed"
  | "edge-not-found"
  | "id-exhausted"
  | "invalid-binding"
  | "invalid-box"
  | "invalid-graph"
  | "invalid-request"
  | "network-not-found"
  | "node-not-found"
  | "not-a-box"
  | "pin-not-found"
  | "would-empty-branch-path";

export type LadBoxAuthoringResult =
  | Readonly<{
      createdIds: readonly string[];
      graph: ProjectPayloadValue;
      ok: true;
    }>
  | Readonly<{
      code: LadBoxAuthoringErrorCode;
      message: string;
      ok: false;
    }>;

export type LadBoxNodeFactoryContext = Readonly<{
  /**
   * Produces valid template identities. The transform replaces every returned
   * graph-owned identity with one allocated from the request's idFactory.
   */
  idFactory: LadBoxIdFactory;
  semanticOrder: number;
}>;

export type LadBoxNodeFactory = (
  context: LadBoxNodeFactoryContext,
) => ProjectPayloadValue | Readonly<{ node: ProjectPayloadValue }>;

type InsertLadBoxBaseRequest = Readonly<{
  edgeId: string;
  idFactory?: LadBoxIdFactory;
  networkId: string;
}>;

/** The catalog fields a learner-facing insert control needs to provide. */
export type InsertMvpLadInstructionBoxRequest = InsertLadBoxBaseRequest &
  Omit<BuildCanonicalLadBoxNodeRequest, "idFactory" | "semanticOrder">;

export type InsertLadBoxRequest = InsertLadBoxBaseRequest & (
  | Readonly<{
      /** A canonical box template whose graph-owned identities will be replaced. */
      boxNode: ProjectPayloadValue;
      boxNodeFactory?: never;
    }>
  | Readonly<{
      /**
       * A catalog-backed factory. Its template IDs are deliberately replaced,
       * so external member and data-block identities remain the only borrowed IDs.
       */
      boxNode?: never;
      boxNodeFactory: LadBoxNodeFactory;
    }>
);

export type UpdateLadBoxPinBindingRequest = Readonly<{
  /**
   * Canonical operand fields. A supplied operand `id` is ignored: an existing
   * binding identity is retained, or a new graph-owned identity is allocated.
   */
  binding: ProjectPayloadValue | null;
  boxNodeId: string;
  idFactory?: LadBoxIdFactory;
  networkId: string;
  pinId: string;
}>;

export type RemoveLadBoxRequest = Readonly<{
  boxNodeId: string;
  idFactory?: LadBoxIdFactory;
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
const GRAPH_OWNED_ID_FIELDS = new Set(["callSiteId", "id", "invocationId"]);

/**
 * Learner-facing convenience API: selects an MVP instruction and supplies its
 * pin/state bindings without requiring UI code to construct a canonical node.
 */
export const insertMvpLadInstructionBoxOnEdge = (
  graph: ProjectPayloadValue,
  request: InsertMvpLadInstructionBoxRequest,
): LadBoxAuthoringResult => {
  const { edgeId, idFactory, networkId, ...boxRequest } = request;
  const insertRequest: InsertLadBoxRequest = {
    boxNodeFactory: ({ idFactory: templateFactory, semanticOrder }) =>
      buildCanonicalLadBoxNode({
        ...boxRequest,
        idFactory: templateFactory,
        semanticOrder,
      }),
    edgeId,
    ...(idFactory === undefined ? {} : { idFactory }),
    networkId,
  };
  return insertLadBoxOnEdge(graph, insertRequest);
};

/** Inserts one canonical instruction box in series on a selected power edge. */
export const insertLadBoxOnEdge = (
  graph: ProjectPayloadValue,
  request: InsertLadBoxRequest,
): LadBoxAuthoringResult => {
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

  const templateResult = boxTemplate(request, targetNodeIndex);
  if (templateResult.ok === false) {
    return templateResult.result;
  }
  const ownedTemplateIds = collectGraphOwnedIdentities(templateResult.node);
  if (ownedTemplateIds === null || ownedTemplateIds.length === 0) {
    return failure(
      "invalid-box",
      "The provided LAD box must contain unique canonical graph-owned identities.",
    );
  }

  const allocator = createAllocator(graph, request.idFactory);
  const replacements = new Map<string, string>();
  for (const templateId of ownedTemplateIds) {
    const allocated = allocator.take();
    if (allocated === null) {
      return idExhausted();
    }
    replacements.set(templateId.toLocaleLowerCase("en-US"), allocated);
  }
  const remapped = remapGraphOwnedIdentities(templateResult.node, replacements);
  const box = remapped === null ? null : parseCanonicalBox(remapped);
  if (remapped === null || box === null) {
    return failure("invalid-box", "The provided LAD box is not a supported canonical box node.");
  }

  const upstreamEdgeId = allocator.take();
  const downstreamEdgeId = allocator.take();
  if (upstreamEdgeId === null || downstreamEdgeId === null) {
    return idExhausted();
  }
  const nodes = [
    ...located.network.nodes.slice(0, targetNodeIndex).map((node) => recordValue(node.fields)),
    remapped,
    ...located.network.nodes.slice(targetNodeIndex).map((node) => recordValue(node.fields)),
  ];
  const edges = [
    ...located.network.edges
      .filter((candidate) => candidate.id !== edge.id)
      .map((candidate) => recordValue(candidate.fields)),
    powerEdge(upstreamEdgeId, edge.sourcePortId, box.inputPortId),
    powerEdge(downstreamEdgeId, box.outputPortId, edge.targetPortId),
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
    allocator.createdIds(),
  );
};

/** Updates exactly one pin binding without changing the pin or existing operand identity. */
export const updateLadBoxPinBinding = (
  graph: ProjectPayloadValue,
  request: UpdateLadBoxPinBindingRequest,
): LadBoxAuthoringResult => {
  const parsed = parseGraph(graph);
  if (parsed === null) {
    return invalidGraph();
  }
  const located = locateNetwork(parsed, request.networkId);
  if (located.ok === false) {
    return located.result;
  }
  const selectedIndex = located.network.nodes.findIndex((node) => node.id === request.boxNodeId);
  if (selectedIndex < 0) {
    return failure("node-not-found", "The selected LAD box does not exist.");
  }
  const selected = located.network.nodes[selectedIndex];
  if (selected === undefined) {
    return invalidGraph("The selected LAD box index could not be resolved.");
  }
  if (selected.kind !== "box") {
    return failure("not-a-box", "Only a LAD instruction box has editable pin bindings.");
  }
  if (!Array.isArray(selected.fields.pins)) {
    return invalidGraph("The selected LAD box does not contain canonical pins.");
  }
  const pinFields = selected.fields.pins.map(canonicalRecordFields);
  if (pinFields.some((pin) => pin === null)) {
    return invalidGraph("The selected LAD box contains a malformed pin.");
  }
  const pins = pinFields.filter((pin): pin is ProjectPayload => pin !== null);
  const matchingIndexes = pins.flatMap((pin, index) =>
    identityField(pin, "id") === request.pinId ? [index] : []
  );
  const pinIndex = matchingIndexes[0];
  if (matchingIndexes.length !== 1 || pinIndex === undefined) {
    return failure("pin-not-found", "The selected LAD box pin does not exist exactly once.");
  }

  const allocator = createAllocator(graph, request.idFactory);
  let binding: ProjectPayloadValue | null = null;
  if (request.binding !== null) {
    const requestedFields = canonicalRecordFields(request.binding);
    if (requestedFields === null || !validBindingFields(requestedFields)) {
      return failure("invalid-binding", "The requested LAD box pin binding is not canonical.");
    }
    const currentFields = canonicalRecordFields(pins[pinIndex]?.binding);
    const currentId = currentFields === null ? null : identityField(currentFields, "id");
    const bindingId = currentId ?? allocator.take();
    if (bindingId === null) {
      return idExhausted();
    }
    binding = recordValue({ ...requestedFields, id: bindingId });
  }

  const updatedPins = pins.map((pin, index) =>
    recordValue(index === pinIndex ? { ...pin, binding } : pin)
  );
  const updatedNode = recordValue({ ...selected.fields, pins: updatedPins });
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
    allocator.createdIds(),
  );
};

/** Removes one series box and reconnects its incoming and outgoing power edges. */
export const removeLadBoxAndReconnect = (
  graph: ProjectPayloadValue,
  request: RemoveLadBoxRequest,
): LadBoxAuthoringResult => {
  const parsed = parseGraph(graph);
  if (parsed === null) {
    return invalidGraph();
  }
  const located = locateNetwork(parsed, request.networkId);
  if (located.ok === false) {
    return located.result;
  }
  const selected = uniqueById(located.network.nodes, request.boxNodeId);
  if (selected === null) {
    return failure("node-not-found", "The selected LAD box does not exist exactly once.");
  }
  if (selected.kind !== "box") {
    return failure("not-a-box", "Only a LAD instruction box can be removed by this operation.");
  }
  const connection = singleNodeConnection(located.network, selected);
  if (connection === null) {
    return failure(
      "ambiguous-connection",
      "The selected LAD box must have exactly one incoming and one outgoing power edge.",
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
      "Removing this box would leave a zero-element parallel path; remove or collapse the branch instead.",
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
    return invalidGraph("The selected LAD box is referenced by malformed branch metadata.");
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
    allocator.createdIds(),
  );
};

const boxTemplate = (
  request: InsertLadBoxRequest,
  semanticOrder: number,
): Readonly<{ node: ProjectPayloadValue; ok: true }> | Readonly<{
  ok: false;
  result: LadBoxAuthoringResult;
}> => {
  if (request.boxNode !== undefined) {
    return { node: request.boxNode, ok: true };
  }
  let result: ReturnType<LadBoxNodeFactory>;
  try {
    result = request.boxNodeFactory({
      idFactory: templateIdFactory(),
      semanticOrder,
    });
  } catch {
    return {
      ok: false,
      result: failure("box-factory-failed", "The LAD instruction catalog could not build this box."),
    };
  }
  const node = isFactoryEnvelope(result) ? result.node : result;
  return { node, ok: true };
};

const isFactoryEnvelope = (
  value: ReturnType<LadBoxNodeFactory>,
): value is Readonly<{ node: ProjectPayloadValue }> =>
  typeof value === "object" && value !== null && !Array.isArray(value) && "node" in value;

const templateIdFactory = (): LadBoxIdFactory => {
  let value = 1;
  return () => {
    const suffix = value.toString(16).padStart(12, "0");
    value += 1;
    return `e0000000-0000-4000-8000-${suffix}`;
  };
};

const parseCanonicalBox = (
  value: ProjectPayloadValue,
): Readonly<{ inputPortId: string; outputPortId: string }> | null => {
  const fields = canonicalRecordFields(value);
  if (
    fields === null ||
    fields.nodeKind !== "box" ||
    identityField(fields, "id") === null ||
    canonicalUnsigned(fields.instructionCode) === null ||
    !Array.isArray(fields.powerPorts) ||
    !Array.isArray(fields.pins)
  ) {
    return null;
  }
  const ports = fields.powerPorts.map(parsePort);
  const pins = fields.pins.map(canonicalRecordFields);
  if (ports.some((port) => port === null) || pins.some((pin) => pin === null)) {
    return null;
  }
  const parsedPorts = ports.filter((port): port is ParsedPort => port !== null);
  const input = parsedPorts.filter((port) => port.direction === "input");
  const output = parsedPorts.filter((port) => port.direction === "output");
  const parsedPins = pins.filter((pin): pin is ProjectPayload => pin !== null);
  if (
    input.length !== 1 ||
    output.length !== 1 ||
    parsedPins.length === 0 ||
    !unique(parsedPorts.map((port) => port.id)) ||
    !unique(parsedPins.map((pin) => identityField(pin, "id") ?? "")) ||
    parsedPins.some((pin) => !validPinFields(pin)) ||
    !validState(fields.state)
  ) {
    return null;
  }
  const inputPortId = input[0]?.id;
  const outputPortId = output[0]?.id;
  return inputPortId === undefined || outputPortId === undefined
    ? null
    : { inputPortId, outputPortId };
};

const validPinFields = (fields: ProjectPayload): boolean => {
  const binding = fields.binding;
  return identityField(fields, "id") !== null &&
    typeof fields.name === "string" &&
    (fields.direction === "input" ||
      fields.direction === "output" ||
      fields.direction === "inout" ||
      fields.direction === "activation" ||
      fields.direction === "status") &&
    typeof fields.dataType === "string" &&
    typeof fields.required === "boolean" &&
    (fields.status === "active" || fields.status === "stale" || fields.status === "orphan") &&
    (fields.formalKind === undefined || fields.formalKind === "instruction") &&
    (fields.formalId === undefined || canonicalUnsigned(fields.formalId) !== null) &&
    (binding === undefined || binding === null || validCanonicalBinding(binding));
};

const validCanonicalBinding = (value: ProjectPayloadValue): boolean => {
  const fields = canonicalRecordFields(value);
  return fields !== null && identityField(fields, "id") !== null && validBindingFields(fields);
};

const validBindingFields = (fields: ProjectPayload): boolean => {
  switch (fields.kind) {
    case "caller-member":
      return identityField(fields, "memberId") !== null;
    case "data-block-member":
      return identityField(fields, "dataBlockId") !== null && identityField(fields, "memberId") !== null;
    case "constant":
      return typeof fields.dataType === "string" && fields.value !== undefined;
    case "expression":
      return typeof fields.source === "string";
    case "unresolved":
      return typeof fields.spelling === "string";
    default:
      return false;
  }
};

const validState = (value: ProjectPayloadValue | undefined): boolean => {
  if (value === undefined || value === null) {
    return true;
  }
  const fields = canonicalRecordFields(value);
  const storage = fields === null ? null : canonicalRecordFields(fields.storage);
  return fields !== null &&
    identityField(fields, "invocationId") !== null &&
    (fields.stateKind === "edge" || fields.stateKind === "timer" || fields.stateKind === "counter") &&
    storage !== null &&
    (storage.kind === "caller-member"
      ? identityField(storage, "memberId") !== null
      : storage.kind === "data-block-member" &&
        identityField(storage, "dataBlockId") !== null &&
        identityField(storage, "memberId") !== null);
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
  const parsed = networks.filter((network): network is ParsedNetwork => network !== null);
  return unique(parsed.map((network) => network.id)) ? { fields, networks: parsed } : null;
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
  result: LadBoxAuthoringResult;
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
  outgoing: ParsedEdge;
}> | null => {
  const inputs = node.ports.filter((port) => port.direction === "input");
  const outputs = node.ports.filter((port) => port.direction === "output");
  const input = inputs[0];
  const output = outputs[0];
  if (inputs.length !== 1 || outputs.length !== 1 || input === undefined || output === undefined) {
    return null;
  }
  const incoming = network.edges.filter((edge) => edge.targetPortId === input.id);
  const outgoing = network.edges.filter((edge) => edge.sourcePortId === output.id);
  return incoming.length === 1 && outgoing.length === 1 &&
    incoming[0] !== undefined && outgoing[0] !== undefined
    ? { incoming: incoming[0], outgoing: outgoing[0] }
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

const collectGraphOwnedIdentities = (
  value: ProjectPayloadValue,
): readonly string[] | null => {
  const identities: string[] = [];
  const seen = new Set<string>();
  const visit = (candidate: ProjectPayloadValue, fieldName?: string): boolean => {
    if (typeof candidate === "string") {
      if (fieldName === undefined || !GRAPH_OWNED_ID_FIELDS.has(fieldName)) {
        return true;
      }
      if (!validIdentity(candidate)) {
        return false;
      }
      const canonical = candidate.toLocaleLowerCase("en-US");
      if (seen.has(canonical)) {
        return false;
      }
      seen.add(canonical);
      identities.push(candidate);
      return true;
    }
    if (candidate === null || typeof candidate === "boolean") {
      return true;
    }
    if (Array.isArray(candidate)) {
      return candidate.every((entry) => visit(entry));
    }
    if (!("$type" in candidate) || candidate.$type !== "record") {
      return true;
    }
    return Object.entries(candidate.value).every(([key, entry]) => visit(entry, key));
  };
  return visit(value) ? identities : null;
};

const remapGraphOwnedIdentities = (
  value: ProjectPayloadValue,
  replacements: ReadonlyMap<string, string>,
  fieldName?: string,
): ProjectPayloadValue | null => {
  if (typeof value === "string") {
    return fieldName !== undefined && GRAPH_OWNED_ID_FIELDS.has(fieldName)
      ? replacements.get(value.toLocaleLowerCase("en-US")) ?? null
      : value;
  }
  if (value === null || typeof value === "boolean") {
    return value;
  }
  if (Array.isArray(value)) {
    const remapped: ProjectPayloadValue[] = [];
    for (const entry of value) {
      const result = remapGraphOwnedIdentities(entry, replacements);
      if (result === null && entry !== null) {
        return null;
      }
      remapped.push(result);
    }
    return remapped;
  }
  if (!("$type" in value) || value.$type !== "record") {
    return value;
  }
  const fields: Record<string, ProjectPayloadValue> = {};
  for (const [key, entry] of Object.entries(value.value)) {
    const remapped = remapGraphOwnedIdentities(entry, replacements, key);
    if (remapped === null && entry !== null) {
      return null;
    }
    fields[key] = remapped;
  }
  return recordValue(fields);
};

const replaceNetwork = (
  parsed: ParsedGraph,
  networkIndex: number,
  networkFields: ProjectPayload,
  nodes: readonly ProjectPayloadValue[],
  edges: readonly ProjectPayloadValue[],
  branches: readonly ProjectPayloadValue[],
  createdIds: readonly string[],
): LadBoxAuthoringResult => {
  const normalizedNodes = nodes.map((node, index) => {
    const fields = canonicalRecordFields(node);
    return fields === null ? null : recordValue({ ...fields, semanticOrder: unsignedValue(index) });
  });
  if (normalizedNodes.some((node) => node === null)) {
    return invalidGraph("A LAD box mutation produced a malformed node record.");
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
  const semanticRevision = incrementSemanticRevision(parsed.fields.semanticRevision);
  if (semanticRevision === null) {
    return invalidGraph("The LAD semantic revision is malformed or cannot be incremented.");
  }
  return {
    createdIds: [...createdIds],
    graph: recordValue({
      ...parsed.fields,
      networks: networks.map((network, index) => {
        const fields = canonicalRecordFields(network);
        return fields === null ? network : recordValue({ ...fields, semanticOrder: unsignedValue(index) });
      }),
      semanticRevision,
    }),
    ok: true,
  };
};

const createAllocator = (
  graph: ProjectPayloadValue,
  factory: LadBoxIdFactory = () => crypto.randomUUID(),
): Readonly<{
  createdIds: () => readonly string[];
  take: () => string | null;
}> => {
  const used = new Set<string>();
  collectAllIdentities(graph, used);
  const created: string[] = [];
  const take = (): string | null => {
    for (let attempt = 0; attempt < MAX_ID_ATTEMPTS; attempt += 1) {
      let candidate: string;
      try {
        candidate = factory();
      } catch {
        return null;
      }
      const canonical = candidate.toLocaleLowerCase("en-US");
      if (validIdentity(candidate) && !used.has(canonical)) {
        used.add(canonical);
        created.push(candidate);
        return candidate;
      }
    }
    return null;
  };
  return { createdIds: () => [...created], take };
};

const collectAllIdentities = (value: ProjectPayloadValue, output: Set<string>): void => {
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
    value.forEach((entry) => collectAllIdentities(entry, output));
    return;
  }
  if ("$type" in value && value.$type === "record") {
    Object.values(value.value).forEach((entry) => collectAllIdentities(entry, output));
  }
};

const incrementSemanticRevision = (
  value: ProjectPayloadValue | undefined,
): Readonly<{ $type: "u64"; value: string }> | null => {
  const parsed = canonicalUnsigned(value);
  return parsed === null || parsed >= MAX_UNSIGNED_64
    ? null
    : { $type: "u64", value: (parsed + BigInt(1)).toString(10) };
};

const canonicalUnsigned = (value: ProjectPayloadValue | undefined): bigint | null => {
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
  return BigInt(value.value);
};

const powerEdge = (
  id: string,
  sourcePortId: string,
  targetPortId: string,
): ProjectPayloadValue => recordValue({ id, sourcePortId, targetPortId });

const identityField = (fields: ProjectPayload, key: string): string | null => {
  const value = fields[key];
  return typeof value === "string" && validIdentity(value) ? value : null;
};

const validIdentity = (value: string): boolean => UUID_PATTERN.test(value);

const unique = (values: readonly string[]): boolean =>
  new Set(values.map((value) => value.toLocaleLowerCase("en-US"))).size === values.length;

const uniqueById = <T extends Readonly<{ id: string }>>(
  values: readonly T[],
  id: string,
): T | null => {
  const matches = values.filter((value) => value.id === id);
  return matches.length === 1 ? (matches[0] ?? null) : null;
};

const invalidGraph = (
  message = "The LAD graph is not a supported canonical graph.",
): LadBoxAuthoringResult => failure("invalid-graph", message);

const idExhausted = (): LadBoxAuthoringResult =>
  failure("id-exhausted", "A unique canonical LAD identity could not be allocated.");

const failure = (
  code: LadBoxAuthoringErrorCode,
  message: string,
): LadBoxAuthoringResult => ({ code, message, ok: false });
