import type { ProjectPayload, ProjectPayloadValue } from "./workbench-types";

export type SclProgramTemplate = "cyclic-ob" | "fc" | "fb";

export const unsignedValue = (value: number): Readonly<{ $type: "u64"; value: string }> => ({
  $type: "u64",
  value: value.toString(10),
});

export const recordValue = (
  value: Readonly<Record<string, ProjectPayloadValue>>,
): Readonly<{
  $type: "record";
  value: Readonly<Record<string, ProjectPayloadValue>>;
}> => ({ $type: "record", value });

const interfaceMember = (
  name: string,
  role: "input" | "output" | "static" | "temp",
  order: number,
  dataType: "BOOL" | "DINT" = "DINT",
): ProjectPayloadValue => recordValue({
  id: crypto.randomUUID(),
  name,
  order: unsignedValue(order),
  requiredOutput: role === "output",
  retentive: false,
  role,
  type: dataType,
});

const graphNetwork = (
  value: Readonly<Record<string, ProjectPayloadValue>>,
): ProjectPayloadValue => recordValue(value);

const ladderPowerPort = (
  id: string,
  direction: "input" | "output",
): ProjectPayloadValue => recordValue({ direction, id });

const ladderOperand = (memberId: string): ProjectPayloadValue => recordValue({
  id: crypto.randomUUID(),
  kind: "caller-member",
  memberId,
});

const fbdPort = (
  name: string,
  direction: "input" | "output",
): ProjectPayloadValue => recordValue({
  activation: "none",
  dataType: "BOOL",
  direction,
  effectRole: "value",
  id: crypto.randomUUID(),
  multiplicity: direction === "output" ? "many" : "one",
  name,
  required: direction === "input",
  status: "active",
});

export type LadFcCallTarget = Readonly<{
  inputFormalId: string;
  outputFormalId: string;
  resultName: string;
  targetBlockId: string;
}>;

/**
 * Creates a coordinate-free semantic LAD graph. When compatible FC targets
 * are supplied, the rung calls each frontend through real block-member pins
 * and consumes both results in power flow. The visible editor is free to lay
 * the rung out differently without changing this executable state.
 */
export const createLadProgramPayload = (
  engineeringNumber = 1,
  callTargets: readonly LadFcCallTarget[] = [],
): ProjectPayload => {
  const input = interfaceMember("InputValue", "temp", 0, "BOOL");
  const output = interfaceMember("OutputValue", "temp", 1, "BOOL");
  const inputMemberId = recordMemberId(input);
  const outputMemberId = recordMemberId(output);
  const callResults = callTargets.map((target, index) => ({
    member: interfaceMember(target.resultName, "temp", index + 2, "BOOL"),
    target,
  }));
  const sourceOutput = crypto.randomUUID();
  const coilInput = crypto.randomUUID();
  const nodes: ProjectPayloadValue[] = [recordValue({
    id: crypto.randomUUID(),
    nodeKind: "power-source",
    powerPorts: [ladderPowerPort(sourceOutput, "output")],
    semanticOrder: unsignedValue(0),
  })];
  const edges: ProjectPayloadValue[] = [];
  let previousOutput = sourceOutput;
  let semanticOrder = 1;

  if (callResults.length === 0) {
    const contactInput = crypto.randomUUID();
    const contactOutput = crypto.randomUUID();
    nodes.push(recordValue({
      id: crypto.randomUUID(),
      mode: "normally-open",
      nodeKind: "contact",
      operand: ladderOperand(inputMemberId),
      powerPorts: [
        ladderPowerPort(contactInput, "input"),
        ladderPowerPort(contactOutput, "output"),
      ],
      semanticOrder: unsignedValue(semanticOrder),
    }));
    edges.push(ladderEdge(previousOutput, contactInput));
    previousOutput = contactOutput;
    semanticOrder += 1;
  } else {
    for (const { member, target } of callResults) {
      const callInput = crypto.randomUUID();
      const callOutput = crypto.randomUUID();
      nodes.push(recordValue({
        callSiteId: crypto.randomUUID(),
        id: crypto.randomUUID(),
        instance: null,
        instructionCode: unsignedValue(0x0200),
        nodeKind: "call",
        pins: [
          ladderCallPin(
            "InputValue",
            "input",
            target.inputFormalId,
            inputMemberId,
          ),
          ladderCallPin(
            "Result",
            "output",
            target.outputFormalId,
            recordMemberId(member),
          ),
        ],
        powerPorts: [
          ladderPowerPort(callInput, "input"),
          ladderPowerPort(callOutput, "output"),
        ],
        semanticOrder: unsignedValue(semanticOrder),
        targetBlockId: target.targetBlockId,
      }));
      edges.push(ladderEdge(previousOutput, callInput));
      previousOutput = callOutput;
      semanticOrder += 1;
    }
    for (const { member } of callResults) {
      const contactInput = crypto.randomUUID();
      const contactOutput = crypto.randomUUID();
      nodes.push(recordValue({
        id: crypto.randomUUID(),
        mode: "normally-open",
        nodeKind: "contact",
        operand: ladderOperand(recordMemberId(member)),
        powerPorts: [
          ladderPowerPort(contactInput, "input"),
          ladderPowerPort(contactOutput, "output"),
        ],
        semanticOrder: unsignedValue(semanticOrder),
      }));
      edges.push(ladderEdge(previousOutput, contactInput));
      previousOutput = contactOutput;
      semanticOrder += 1;
    }
  }

  nodes.push(recordValue({
    id: crypto.randomUUID(),
    mode: "normal",
    nodeKind: "coil",
    operand: ladderOperand(outputMemberId),
    powerPorts: [ladderPowerPort(coilInput, "input")],
    semanticOrder: unsignedValue(semanticOrder),
  }));
  edges.push(ladderEdge(previousOutput, coilInput));

  return {
    blockKind: "OB",
    engineeringNumber: unsignedValue(engineeringNumber),
    graph: recordValue({
      documentId: crypto.randomUUID(),
      networks: [
        graphNetwork({
          branches: [],
          edges,
          id: crypto.randomUUID(),
          nodes,
          semanticOrder: unsignedValue(0),
        }),
      ],
      schema: "edu.lad-semantic-graph/1",
      semanticRevision: unsignedValue(0),
    }),
    interface: [input, output, ...callResults.map(({ member }) => member)],
    language: "LAD",
    obRole: "CyclicMain",
  };
};

const ladderEdge = (sourcePortId: string, targetPortId: string): ProjectPayloadValue =>
  recordValue({ id: crypto.randomUUID(), sourcePortId, targetPortId });

const ladderCallPin = (
  name: string,
  direction: "input" | "output",
  formalId: string,
  callerMemberId: string,
): ProjectPayloadValue => recordValue({
  binding: ladderOperand(callerMemberId),
  dataType: "BOOL",
  direction,
  formalId,
  formalKind: "block-member",
  id: crypto.randomUUID(),
  name,
  required: true,
  status: "active",
});

/** Creates a typed, coordinate-free FBD function with one real NOT node. */
export const createFbdProgramPayload = (engineeringNumber = 1): ProjectPayload => {
  const input = interfaceMember("InputValue", "input", 0, "BOOL");
  const output = interfaceMember("Result", "output", 1, "BOOL");
  const inputMemberId = recordMemberId(input);
  const outputMemberId = recordMemberId(output);
  const loadOutput = fbdPort("OUT", "output");
  const invertInput = recordValue({
    ...recordFields(fbdPort("IN", "input")),
    formalId: unsignedValue(0x0010),
    formalKind: "instruction",
  });
  const invertOutput = recordValue({
    ...recordFields(fbdPort("OUT", "output")),
    formalId: unsignedValue(0x0011),
    formalKind: "instruction",
  });
  const storeInput = fbdPort("IN", "input");

  return {
    blockKind: "FC",
    engineeringNumber: unsignedValue(engineeringNumber),
    graph: recordValue({
      documentId: crypto.randomUUID(),
      networks: [
        graphNetwork({
          connections: [
            recordValue({
              id: crypto.randomUUID(),
              kind: "data",
              sourcePortId: recordId(loadOutput),
              targetPortId: recordId(invertInput),
            }),
            recordValue({
              id: crypto.randomUUID(),
              kind: "data",
              sourcePortId: recordId(invertOutput),
              targetPortId: recordId(storeInput),
            }),
          ],
          id: crypto.randomUUID(),
          nodes: [
            recordValue({
              id: crypto.randomUUID(),
              memberId: inputMemberId,
              nodeKind: "load-member",
              ports: [loadOutput],
              semanticOrder: unsignedValue(0),
            }),
            recordValue({
              id: crypto.randomUUID(),
              instructionCode: unsignedValue(0x0010),
              nodeKind: "instruction",
              ports: [invertInput, invertOutput],
              semanticOrder: unsignedValue(1),
              stateInstanceId: null,
            }),
            recordValue({
              id: crypto.randomUUID(),
              memberId: outputMemberId,
              nodeKind: "store-member",
              ports: [storeInput],
              semanticOrder: unsignedValue(2),
            }),
          ],
          semanticOrder: unsignedValue(0),
        }),
      ],
      schema: "edu.fbd-semantic-graph/1",
    }),
    interface: [input, output],
    language: "FBD",
  };
};

const recordFields = (
  value: ProjectPayloadValue,
): Readonly<Record<string, ProjectPayloadValue>> => {
  if (
    typeof value !== "object" ||
    value === null ||
    Array.isArray(value) ||
    !("$type" in value) ||
    value.$type !== "record"
  ) {
    throw new Error("Canonical authoring expected a record value.");
  }
  return value.value;
};

const recordId = (value: ProjectPayloadValue): string => {
  const id = recordFields(value).id;
  if (typeof id !== "string") {
    throw new Error("Canonical authoring expected a record identity.");
  }
  return id;
};

const recordMemberId = (value: ProjectPayloadValue): string => recordId(value);

/**
 * Creates the semantic payload for a new SCL block at the moment the user
 * commits creation. Interface identities are canonical project identities,
 * not renderer-local row keys.
 */
export const createSclProgramPayload = (
  template: SclProgramTemplate,
  engineeringNumber = 1,
): ProjectPayload => {
  switch (template) {
    case "cyclic-ob":
      return {
        blockKind: "OB",
        engineeringNumber: unsignedValue(engineeringNumber),
        interface: [
          interfaceMember("InputValue", "temp", 0, "BOOL"),
          interfaceMember("OutputValue", "temp", 1, "BOOL"),
          interfaceMember("WorkingValue", "temp", 2),
        ],
        language: "SCL",
        obRole: "CyclicMain",
        sourceText: "",
      };
    case "fc":
      return {
        blockKind: "FC",
        engineeringNumber: unsignedValue(engineeringNumber),
        interface: [
          interfaceMember("InputValue", "input", 0, "BOOL"),
          interfaceMember("Result", "output", 1, "BOOL"),
        ],
        language: "SCL",
        sourceText: "",
      };
    case "fb":
      return {
        blockKind: "FB",
        engineeringNumber: unsignedValue(engineeringNumber),
        interface: [
          interfaceMember("InputValue", "input", 0),
          interfaceMember("Accumulator", "static", 1),
          interfaceMember("Result", "output", 2),
        ],
        language: "SCL",
        sourceText: "",
      };
  }
};

export const createDataBlockPayload = (
  kind: "GlobalDB" | "InstanceDB",
  instanceOf: string | null = null,
  engineeringNumber = 1,
): ProjectPayload => kind === "GlobalDB"
  ? {
      dbKind: kind,
      engineeringNumber: unsignedValue(engineeringNumber),
      members: [interfaceMember("MemoryValue", "static", 0, "BOOL")],
    }
  : {
      dbKind: kind,
      engineeringNumber: unsignedValue(engineeringNumber),
      instanceOf,
      members: [],
    };

export const createWatchPayload = (): ProjectPayload => ({ rows: [] });

export const createTracePayload = (): ProjectPayload => ({
  channels: [],
  everyScans: unsignedValue(1),
  maximumDurationMs: unsignedValue(60_000),
  postSamples: unsignedValue(32),
  preSamples: unsignedValue(0),
  state: "idle",
  trigger: "immediate",
});

export const interfaceMemberIdentity = (
  payload: ProjectPayload,
  memberName: string,
): string | null => {
  const members = payload.interface ?? payload.members;
  if (!Array.isArray(members)) {
    return null;
  }
  for (const member of members) {
    if (
      typeof member !== "object" ||
      member === null ||
      Array.isArray(member) ||
      !("$type" in member) ||
      member.$type !== "record" ||
      !("value" in member) ||
      typeof member.value !== "object" ||
      member.value === null ||
      Array.isArray(member.value)
    ) {
      continue;
    }
    const name = member.value.name;
    const id = member.value.id;
    if (name === memberName && typeof id === "string") {
      return id;
    }
  }
  return null;
};

/**
 * Replaces fields on exactly one stable graphical node. The operation returns
 * null for malformed graphs, missing identities, or duplicate identities so a
 * UI edit can never silently target an ambiguous semantic object.
 */
export const updateGraphNodeFields = (
  graph: ProjectPayloadValue,
  nodeId: string,
  fields: ProjectPayload,
): ProjectPayloadValue | null => {
  const graphRecord = recordFieldsOrNull(graph);
  if (graphRecord === null || !Array.isArray(graphRecord.networks)) {
    return null;
  }
  let replacements = 0;
  const networks = graphRecord.networks.map((networkValue) => {
    const network = recordFieldsOrNull(networkValue);
    if (network === null || !Array.isArray(network.nodes)) {
      return networkValue;
    }
    const nodes = network.nodes.map((nodeValue) => {
      const node = recordFieldsOrNull(nodeValue);
      if (node === null || node.id !== nodeId) {
        return nodeValue;
      }
      replacements += 1;
      return recordValue({ ...node, ...fields });
    });
    return recordValue({ ...network, nodes });
  });
  return replacements === 1 ? recordValue({ ...graphRecord, networks }) : null;
};

export const canonicalRecordFields = (
  value: ProjectPayloadValue | undefined,
): ProjectPayload | null => value === undefined ? null : recordFieldsOrNull(value);

const recordFieldsOrNull = (value: ProjectPayloadValue): ProjectPayload | null => {
  if (
    typeof value !== "object" ||
    value === null ||
    Array.isArray(value) ||
    !("$type" in value) ||
    value.$type !== "record"
  ) {
    return null;
  }
  return value.value;
};
