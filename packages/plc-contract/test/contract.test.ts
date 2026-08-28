import {
  FOUNDATION_COMPATIBILITY,
  PLC_CONTRACT_LIMITS,
  PlcContractValidationError,
  decodePlcMessage,
  encodePlcMessage,
  validateCanonicalTypedValue,
  validateDiagnostic,
  validateDomainCommand,
  validateDomainEvent,
  validateDomainQuery,
  validateDomainReceipt,
  validatePhase2PlcMessage,
  validatePlcMessage,
  validateSourceAnchor,
} from "../src/index";

const uuid = (value: number): string =>
  `00000000-0000-4000-8000-${value.toString().padStart(12, "0")}`;

const HASH_A = "A".repeat(64);
const HASH_B = "B".repeat(64);
const HASH_C = "C".repeat(64);

const BOOL_VALUE = {
  kind: "bool",
  typeId: "BOOL",
  value: true,
} as const;

const PROJECT_ANCHOR = {
  anchorKind: "project",
  ownerObjectId: uuid(1),
  propertyPath: ["displayName"],
  sourceRevisionHash: HASH_A,
} as const;

const TEXT_ANCHOR = {
  anchorKind: "text",
  language: "SCL",
  ownerObjectId: uuid(2),
  range: { endExclusive: 11, start: 4 },
  semanticNodeId: uuid(3),
  sourceRevisionHash: HASH_B,
} as const;

const GRAPH_ANCHOR = {
  anchorKind: "graph",
  edgeId: uuid(4),
  language: "LAD",
  networkId: uuid(5),
  nodeId: uuid(6),
  ownerObjectId: uuid(2),
  portId: uuid(7),
  semanticNodeId: uuid(8),
  sourceRevisionHash: HASH_C,
} as const;

const GENERATED_ANCHOR = {
  anchorKind: "generated",
  causalAnchor: TEXT_ANCHOR,
  ownerObjectId: uuid(2),
  sourceRevisionHash: HASH_C,
} as const;

const DIAGNOSTIC = {
  blocking: true,
  buildAttemptId: uuid(20),
  cause: "The requested binding is not legal for the selected instruction.",
  code: "EDU-CALL-0001",
  diagnosticId: uuid(21),
  parameters: [
    { kind: "boolean", name: "isRequired", value: true },
    { kind: "decimal", name: "actualCount", value: "1" },
    { kind: "hash", name: "snapshotHash", value: HASH_A },
    { kind: "identity", name: "memberId", value: uuid(22) },
    { kind: "text", name: "parameterName", value: "Done" },
  ],
  phase: "InterfaceValidation",
  primaryAnchor: GRAPH_ANCHOR,
  recoveryHint: "Bind the required output or change the instruction.",
  relatedAnchors: [PROJECT_ANCHOR, TEXT_ANCHOR, GENERATED_ANCHOR],
  severity: "Error",
  snapshotHash: HASH_B,
} as const;

const BLOCK_INTERFACE = {
  constants: [
    {
      comment: "constant",
      declaredOrder: "5",
      id: uuid(35),
      name: "Limit",
      role: "Constant",
      typeId: "INT",
      value: { kind: "signed-integer", typeId: "INT", value: "10" },
    },
  ],
  inOuts: [
    {
      comment: "in/out",
      declaredOrder: "2",
      id: uuid(32),
      name: "State",
      role: "InOut",
      typeId: "BOOL",
    },
  ],
  inputs: [
    {
      comment: "input",
      declaredOrder: "0",
      defaultValue: BOOL_VALUE,
      id: uuid(30),
      name: "Enable",
      role: "Input",
      typeId: "BOOL",
    },
  ],
  outputs: [
    {
      comment: "output",
      declaredOrder: "1",
      id: uuid(31),
      name: "Done",
      requiredOutputBinding: true,
      role: "Output",
      startValue: BOOL_VALUE,
      typeId: "BOOL",
    },
  ],
  return: {
    comment: "return",
    id: uuid(36),
    name: "Result",
    role: "Return",
    typeId: "BOOL",
  },
  statics: [
    {
      comment: "static",
      declaredOrder: "3",
      id: uuid(33),
      name: "Prior",
      retainPolicy: "NonRetentive",
      role: "Static",
      startValue: BOOL_VALUE,
      typeId: "BOOL",
    },
  ],
  temps: [
    {
      comment: "temporary",
      declaredOrder: "4",
      id: uuid(34),
      name: "Scratch",
      role: "Temp",
      typeId: "BOOL",
    },
  ],
} as const;

const GRAPH = {
  language: "LAD",
  networks: [
    {
      edges: [
        {
          edgeKind: "power",
          id: uuid(45),
          sourcePortId: uuid(42),
          targetPortId: uuid(44),
        },
      ],
      id: uuid(40),
      nodes: [
        {
          definitionId: "lad.contact.no",
          id: uuid(41),
          ports: [
            {
              direction: "output",
              id: uuid(42),
              required: true,
              role: "power",
              typeId: "BOOL",
            },
          ],
          semanticOrder: "0",
          stateBindingId: null,
        },
        {
          definitionId: "lad.coil.normal",
          id: uuid(43),
          ports: [
            {
              direction: "input",
              id: uuid(44),
              required: true,
              role: "power",
              typeId: "BOOL",
            },
          ],
          semanticOrder: "1",
          stateBindingId: null,
        },
      ],
      semanticOrder: "0",
    },
  ],
  sourceRevisionHash: HASH_A,
} as const;

const COMMAND_CONTEXT = {
  commandId: uuid(100),
  expectedObjectRevisions: [{ objectId: uuid(1), revision: "7" }],
  expectedProjectRevision: "12",
  idempotencyKey: "command-100",
  issuedSequence: "25",
  transactionId: uuid(101),
} as const;

const QUERY_CONTEXT = {
  atProjectRevision: "12",
  consistency: "captured",
  queryId: uuid(102),
} as const;

const commandMessage = (command: unknown) => ({
  command,
  context: COMMAND_CONTEXT,
  kind: "plc.command",
  requestId: uuid(103),
  schemaVersion: 2,
});

const queryMessage = (query: unknown) => ({
  context: QUERY_CONTEXT,
  kind: "plc.query",
  query,
  requestId: uuid(104),
  schemaVersion: 2,
});

const OBSERVED_VALUE = {
  forceId: null,
  freshness: "CURRENT",
  probeId: uuid(70),
  quality: "GOOD",
  scanSequence: "8",
  targetId: uuid(71),
  value: BOOL_VALUE,
  virtualTimestampMilliseconds: "1000",
} as const;

const PROJECT_EVENT = {
  affectedObjectIds: [uuid(1)],
  documentRevision: "13",
  eventId: uuid(110),
  eventKind: "project.changed",
  eventSequence: "30",
  projectHash: HASH_B,
  semanticRevision: "9",
  transactionId: COMMAND_CONTEXT.transactionId,
} as const;

const EMPTY_INTERFACE = {
  constants: [],
  inOuts: [],
  inputs: [],
  outputs: [],
  return: null,
  statics: [],
  temps: [],
} as const;

const PROJECT_SNAPSHOT = {
  dirtyBuildState: {
    controllerStates: [
      {
        controllerId: uuid(10),
        hardware: "current",
        loadedArtifactFingerprint: HASH_C,
        software: "stale",
      },
    ],
    currentDocumentHash: HASH_A,
    currentSemanticFingerprint: HASH_B,
    documentDirty: true,
    savedDocumentHash: HASH_C,
    savedDocumentRevision: "11",
    savedSemanticFingerprint: HASH_A,
    semanticDirty: true,
  },
  documentId: uuid(11),
  documentRevision: "12",
  domain: "project-snapshot",
  objects: [
    {
      creationOrdinal: "0",
      displayName: "Main Project",
      id: uuid(1),
      kind: "ProjectRoot",
      lifecycle: "active",
      objectRevision: "4",
      orderedChildIds: [uuid(10)],
      parentId: null,
      references: [
        {
          expectedTargetKind: "Controller",
          referenceId: uuid(12),
          resolution: "resolved",
          sourceAnchor: PROJECT_ANCHOR,
          targetId: uuid(10),
        },
      ],
      semanticRevision: "3",
    },
    {
      creationOrdinal: "1",
      displayName: "PLC_1",
      id: uuid(10),
      kind: "Controller",
      lifecycle: "active",
      objectRevision: "5",
      orderedChildIds: [],
      parentId: uuid(1),
      references: [],
      semanticRevision: "4",
    },
  ],
  projectRootId: uuid(1),
  scope: "summary",
  semanticRevision: "8",
} as const;

const HARDWARE_SNAPSHOT = {
  configurationFingerprint: HASH_A,
  controllerId: uuid(10),
  domain: "hardware-snapshot",
  objects: [
    {
      address: null,
      catalogId: "sim.cpu.1516",
      creationOrdinal: "0",
      displayName: "PLC_1",
      id: uuid(10),
      kind: "Controller",
      lifecycle: "active",
      objectRevision: "5",
      orderedChildIds: [uuid(72)],
      parentId: null,
      references: [],
      semanticRevision: "4",
      slot: null,
      virtualAddress: "192.0.2.10",
      virtualSubnetId: uuid(74),
    },
    {
      address: { area: "I", bitOffset: 0, byteOffset: 0, widthBits: 1 },
      catalogId: "sim.di.channel",
      creationOrdinal: "1",
      displayName: "Start Button",
      id: uuid(72),
      kind: "Channel",
      lifecycle: "active",
      objectRevision: "2",
      orderedChildIds: [],
      parentId: uuid(10),
      references: [],
      semanticRevision: "2",
      slot: 1,
      virtualAddress: null,
      virtualSubnetId: null,
    },
  ],
  profileId: "training.cpu.standard",
  profileVersion: "2.0.0",
} as const;

const PROGRAM_SNAPSHOT = {
  blocks: [
    {
      blockId: uuid(2),
      blockKind: "OB",
      body: {
        bodyKind: "scl",
        sourceHash: HASH_C,
        sourceText: "#Done := #Enable;",
      },
      bodyLanguage: "SCL",
      creationOrdinal: "2",
      displayName: "Main",
      engineeringNumber: 1,
      interface: EMPTY_INTERFACE,
      lifecycle: "active",
      objectRevision: "6",
      obRole: "CyclicMain",
      offsetMilliseconds: null,
      orderedChildIds: [],
      parentId: uuid(10),
      periodMilliseconds: null,
      publicSignatureFingerprint: HASH_A,
      references: [],
      semanticFingerprint: HASH_B,
      semanticRevision: "5",
      sourceRevisionHash: HASH_C,
    },
  ],
  controllerId: uuid(10),
  domain: "program-snapshot",
} as const;

const BUILD_RECEIPT = {
  artifact: {
    artifactPackageFingerprint: HASH_A,
    artifactSchema: "2.0.0",
    irVersion: "2.0.0",
    memorySchemaHash: HASH_B,
    probeSchemaVersion: "2.0.0",
    profileHash: HASH_C,
    runtimeContractVersion: "2.0.0",
    semanticBuildFingerprint: HASH_B,
    sourceMapHash: HASH_C,
  },
  attempt: {
    attemptId: uuid(20),
    compilerSemanticVersion: "2.0.0",
    instructionRegistryHash: HASH_A,
    requestedScope: "ControllerBuild",
    snapshotHash: HASH_C,
    trainingProfileHash: HASH_B,
    typeSystemVersion: "2.0.0",
  },
  domain: "build",
  report: {
    artifactFingerprint: HASH_A,
    attemptId: uuid(20),
    diagnostics: [],
    expandedClosure: [uuid(2)],
    outcome: "Success",
    requestedScope: "ControllerBuild",
    semanticFingerprint: HASH_B,
    snapshotHash: HASH_C,
    stale: false,
  },
} as const;

const RECEIPTS = [
  {
    contractSchemaVersion: 2,
    coreVersion: "2.0.0",
    domain: "health",
    status: "HEALTHY",
    supportedMessageSchemaVersions: [1, 2],
    wasmSha256: HASH_A,
  },
  {
    affectedObjectIds: [uuid(1)],
    documentId: uuid(11),
    documentRevision: "13",
    domain: "project",
    projectHash: HASH_B,
    projectRootId: uuid(1),
    semanticRevision: "9",
  },
  PROJECT_SNAPSHOT,
  {
    action: "save",
    documentId: uuid(11),
    documentRevision: "13",
    domain: "persistence",
    packageHash: HASH_A,
    projectRootId: uuid(1),
    recoveryStatus: "not-applicable",
    schemaVersion: "2.0.0",
  },
  {
    affectedObjectIds: [uuid(72)],
    configurationFingerprint: HASH_A,
    controllerId: uuid(10),
    domain: "hardware",
  },
  HARDWARE_SNAPSHOT,
  {
    affectedObjectIds: [uuid(2)],
    blockId: uuid(2),
    controllerId: uuid(10),
    domain: "program",
    invalidatedObjectIds: [uuid(3)],
    publicSignatureFingerprint: HASH_A,
  },
  PROGRAM_SNAPSHOT,
  BUILD_RECEIPT,
  {
    affectedTargetIds: [uuid(71)],
    controllerEpoch: "4",
    controllerId: uuid(10),
    cpuState: "RUN",
    domain: "runtime",
    loadedArtifactFingerprint: HASH_A,
    scanSequence: "8",
    universeEpoch: "2",
    virtualTimestampMilliseconds: "1000",
  },
  {
    action: "preview",
    candidateArtifactFingerprint: HASH_A,
    controllerEpoch: "4",
    controllerId: uuid(10),
    domain: "commissioning",
    loadedArtifactFingerprint: HASH_B,
    memoryActions: [
      { action: "preserve", reason: "Compatible retained region", regionId: uuid(80) },
    ],
    previewFingerprint: HASH_C,
    previewId: uuid(81),
  },
  {
    action: "snapshot",
    commandReceiptId: null,
    controllerEpoch: "4",
    controllerId: uuid(10),
    domain: "monitoring",
    samples: [OBSERVED_VALUE],
    subscriptionId: uuid(82),
  },
  {
    diagnosticRevision: "3",
    diagnostics: [DIAGNOSTIC],
    domain: "diagnostics",
  },
] as const;

const DOMAIN_COMMANDS = [
  {
    commandKind: "project.create",
    displayName: "Training Project",
    documentId: uuid(11),
    projectRootId: uuid(1),
  },
  { commandKind: "project.rename-object", displayName: "PLC_1", objectId: uuid(10) },
  { commandKind: "project.move-object", objectId: uuid(2), orderKey: "2", parentId: uuid(1) },
  { commandKind: "project.delete-object", objectId: uuid(2) },
  { commandKind: "project.copy-objects", sourceObjectIds: [uuid(2)], targetParentId: uuid(1) },
  { commandKind: "project.undo", undoToken: "undo-1" },
  { commandKind: "project.redo", undoToken: "redo-1" },
  { commandKind: "persistence.open", sourceGrantId: uuid(90) },
  { commandKind: "persistence.save", documentId: uuid(11), mode: "save", targetGrantId: uuid(91) },
  { commandKind: "persistence.recover", recoveryJournalId: uuid(92) },
  {
    commandKind: "hardware.configure-controller",
    controllerId: uuid(10),
    profileId: "training.cpu.standard",
    profileVersion: "2.0.0",
  },
  {
    catalogId: "sim.di.module",
    commandKind: "hardware.upsert-device",
    controllerId: uuid(10),
    deviceId: uuid(72),
    parentId: uuid(10),
    slot: 1,
  },
  {
    area: "I",
    bitOffset: 0,
    byteOffset: 0,
    commandKind: "hardware.assign-address",
    controllerId: uuid(10),
    objectId: uuid(72),
    widthBits: 1,
  },
  {
    commandKind: "hardware.assign-virtual-network",
    controllerId: uuid(10),
    interfaceId: uuid(73),
    subnetId: uuid(74),
    virtualAddress: "192.0.2.10",
  },
  {
    blockId: uuid(2),
    blockKind: "OB",
    commandKind: "program.create-block",
    controllerId: uuid(10),
    displayName: "Main",
    engineeringNumber: 1,
    language: "SCL",
    obRole: "CyclicMain",
    offsetMilliseconds: null,
    periodMilliseconds: null,
  },
  {
    blockId: uuid(2),
    commandKind: "program.replace-interface",
    controllerId: uuid(10),
    interface: BLOCK_INTERFACE,
  },
  {
    blockId: uuid(2),
    commandKind: "program.replace-scl-body",
    controllerId: uuid(10),
    sourceHash: HASH_A,
    sourceText: "#Done := #Enable;",
  },
  {
    blockId: uuid(2),
    commandKind: "program.replace-graph-body",
    controllerId: uuid(10),
    graph: GRAPH,
  },
  { commandKind: "build.compile", controllerId: uuid(10), scope: "CurrentObject", selectedObjectId: uuid(2) },
  { attemptId: uuid(20), commandKind: "build.cancel" },
  { commandKind: "runtime.set-mode", controllerId: uuid(10), expectedControllerEpoch: "4", mode: "RUN" },
  { commandKind: "runtime.advance-virtual-time", deltaMilliseconds: "10", universeId: uuid(75) },
  { commandKind: "runtime.set-virtual-input", controllerId: uuid(10), expectedControllerEpoch: "4", targetId: uuid(71), value: BOOL_VALUE },
  { artifactPackageFingerprint: HASH_A, commandKind: "commissioning.preview-load", controllerId: uuid(10), expectedControllerEpoch: "4" },
  { commandKind: "commissioning.commit-load", controllerId: uuid(10), expectedControllerEpoch: "4", previewFingerprint: HASH_B, previewId: uuid(81) },
  { commandKind: "commissioning.cancel-preview", previewId: uuid(81) },
  { commandKind: "monitoring.subscribe", controllerId: uuid(10), expectedControllerEpoch: "4", targetIds: [uuid(71)] },
  { commandKind: "monitoring.unsubscribe", subscriptionId: uuid(82) },
  { commandKind: "monitoring.modify", controllerId: uuid(10), expectedControllerEpoch: "4", targetId: uuid(71), value: BOOL_VALUE },
  { commandKind: "monitoring.create-force", controllerId: uuid(10), expectedControllerEpoch: "4", expectedForceRevision: "2", targetId: uuid(71), value: BOOL_VALUE },
  { commandKind: "monitoring.remove-force", controllerId: uuid(10), expectedControllerEpoch: "4", forceId: uuid(83) },
  { commandKind: "monitoring.trace-control", controllerId: uuid(10), expectedControllerEpoch: "4", operation: "arm", traceId: uuid(84) },
] as const;

const DOMAIN_QUERIES = [
  { queryKind: "system.health" },
  { projectRootId: uuid(1), queryKind: "project.get-summary" },
  { objectId: uuid(2), queryKind: "project.get-object" },
  { documentId: uuid(11), queryKind: "persistence.get-status" },
  { controllerId: uuid(10), queryKind: "hardware.get-configuration" },
  { blockId: uuid(2), queryKind: "program.get-block" },
  { attemptId: uuid(20), queryKind: "build.get-report" },
  { controllerId: uuid(10), expectedControllerEpoch: "4", queryKind: "runtime.get-status" },
  { previewId: uuid(81), queryKind: "commissioning.get-preview" },
  { queryKind: "monitoring.get-snapshot", subscriptionId: uuid(82) },
  { blocking: null, phase: null, queryKind: "diagnostics.list" },
] as const;

const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T;

const expectInvalid = (operation: () => unknown): void => {
  expect(operation).toThrow(PlcContractValidationError);
};

describe("Phase 1 health compatibility", () => {
  it("accepts the frozen command and both exact result forms", () => {
    const command = {
      kind: FOUNDATION_COMPATIBILITY.commandKind,
      requestId: FOUNDATION_COMPATIBILITY.requestId,
      schemaVersion: FOUNDATION_COMPATIBILITY.schemaVersion,
    };
    const success = {
      affectedObjectIds: [],
      afterHash: FOUNDATION_COMPATIBILITY.stateHash,
      beforeHash: FOUNDATION_COMPATIBILITY.stateHash,
      diagnostics: [],
      events: [],
      kind: FOUNDATION_COMPATIBILITY.resultKind,
      requestId: FOUNDATION_COMPATIBILITY.requestId,
      schemaVersion: FOUNDATION_COMPATIBILITY.schemaVersion,
      success: true,
      value: {
        buildIdentity: FOUNDATION_COMPATIBILITY.buildIdentity,
        healthState: FOUNDATION_COMPATIBILITY.healthyState,
        schemaVersion: FOUNDATION_COMPATIBILITY.schemaVersion,
        wasmSha256: HASH_A,
      },
    };
    const failure = {
      affectedObjectIds: [],
      afterHash: FOUNDATION_COMPATIBILITY.stateHash,
      beforeHash: FOUNDATION_COMPATIBILITY.stateHash,
      diagnostics: [{ code: "WORKER_FAILURE", message: "Worker unavailable", severity: "error" }],
      events: [],
      kind: FOUNDATION_COMPATIBILITY.resultKind,
      requestId: FOUNDATION_COMPATIBILITY.requestId,
      schemaVersion: FOUNDATION_COMPATIBILITY.schemaVersion,
      success: false,
    };

    expect(validatePlcMessage(command)).toBe(command);
    expect(validatePlcMessage(success)).toBe(success);
    expect(validatePlcMessage(failure)).toBe(failure);
  });

  it("rejects drift and extension of the frozen Phase 1 envelope", () => {
    expectInvalid(() => validatePlcMessage({ kind: "foundation.health", requestId: "other", schemaVersion: 1 }));
    expectInvalid(() => validatePlcMessage({
      kind: "foundation.health",
      requestId: FOUNDATION_COMPATIBILITY.requestId,
      schemaVersion: 1,
      surprise: true,
    }));
  });
});

describe("canonical typed values", () => {
  const values = [
    BOOL_VALUE,
    { kind: "signed-integer", typeId: "SINT", value: "-128" },
    { kind: "signed-integer", typeId: "LINT", value: "9223372036854775807" },
    { kind: "unsigned-integer", typeId: "ULINT", value: "18446744073709551615" },
    { bitsHex: "00FF", kind: "bit-string", typeId: "WORD" },
    { ieeeHex: "7FC00000", kind: "floating", typeId: "REAL" },
    { ieeeHex: "7FF8000000000000", kind: "floating", typeId: "LREAL" },
    { codeUnit: 255, kind: "char", typeId: "CHAR" },
    { capacity: 4, codeUnits: [65, 66], kind: "string", typeId: "STRING" },
    { kind: "time", milliseconds: "-1", typeId: "TIME" },
    {
      bounds: [{ lower: -1, upper: 0 }],
      elements: [BOOL_VALUE, BOOL_VALUE],
      kind: "array",
      typeId: "ARRAY[-1..0].BOOL",
    },
    {
      kind: "struct",
      members: [{ memberId: uuid(22), value: BOOL_VALUE }],
      typeId: "UDT.Motor",
    },
    { kind: "instruction-state", stateFingerprint: HASH_A, typeId: "TimerState" },
  ] as const;

  it.each(values.map((value) => [value.kind, value]))("accepts %s", (_kind, value) => {
    expect(validateCanonicalTypedValue(value)).toBe(value);
  });

  it("rejects noncanonical, overflowing, malformed, duplicate, and extra-key values", () => {
    const invalid = [
      { kind: "signed-integer", typeId: "SINT", value: "128" },
      { kind: "signed-integer", typeId: "INT", value: "01" },
      { kind: "unsigned-integer", typeId: "UINT", value: "-1" },
      { bitsHex: "00ff", kind: "bit-string", typeId: "WORD" },
      { ieeeHex: "7F800001", kind: "floating", typeId: "REAL" },
      { capacity: 1, codeUnits: [65, 66], kind: "string", typeId: "STRING" },
      { bounds: [{ lower: 0, upper: 1 }], elements: [BOOL_VALUE], kind: "array", typeId: "Array.BOOL" },
      { kind: "struct", members: [
        { memberId: uuid(22), value: BOOL_VALUE },
        { memberId: uuid(22), value: BOOL_VALUE },
      ], typeId: "UDT.Motor" },
      { extra: true, kind: "bool", typeId: "BOOL", value: true },
      { kind: "future-value", typeId: "BOOL", value: true },
    ];
    for (const value of invalid) {
      expectInvalid(() => validateCanonicalTypedValue(value));
    }
  });

  it("enforces the recursive depth and declared element bounds", () => {
    let nested: unknown = BOOL_VALUE;
    for (let index = 0; index < PLC_CONTRACT_LIMITS.typedValueDepth + 2; index += 1) {
      nested = {
        kind: "struct",
        members: [{ memberId: uuid(200 + index), value: nested }],
        typeId: "UDT.Nested",
      };
    }
    expectInvalid(() => validateCanonicalTypedValue(nested));
    expectInvalid(() => validateCanonicalTypedValue({
      bounds: [{ lower: 0, upper: PLC_CONTRACT_LIMITS.typedValueElements }],
      elements: [],
      kind: "array",
      typeId: "Array.BOOL",
    }));
  });
});

describe("source anchors and diagnostics", () => {
  it.each([
    ["project", PROJECT_ANCHOR],
    ["text", TEXT_ANCHOR],
    ["graph", GRAPH_ANCHOR],
    ["generated", GENERATED_ANCHOR],
  ])("accepts %s anchors", (_kind, anchor) => {
    expect(validateSourceAnchor(anchor)).toBe(anchor);
  });

  it("accepts structured diagnostics without relying on rendered prose", () => {
    expect(validateDiagnostic(DIAGNOSTIC)).toBe(DIAGNOSTIC);
  });

  it("rejects invalid ranges, recursive generated anchors, duplicate parameters, and invented codes", () => {
    const badRange = clone(TEXT_ANCHOR) as Record<string, any>;
    badRange.range = { endExclusive: 2, start: 3 };
    expectInvalid(() => validateSourceAnchor(badRange));

    const recursive = clone(GENERATED_ANCHOR) as Record<string, any>;
    recursive.causalAnchor = clone(GENERATED_ANCHOR);
    expectInvalid(() => validateSourceAnchor(recursive));

    const duplicate = clone(DIAGNOSTIC) as Record<string, any>;
    duplicate.parameters.push({ kind: "text", name: "parameterName", value: "again" });
    expectInvalid(() => validateDiagnostic(duplicate));

    const invented = { ...DIAGNOSTIC, code: "TYPE_ERROR" };
    expectInvalid(() => validateDiagnostic(invented));
  });
});

describe("domain command and query vocabulary", () => {
  it.each(DOMAIN_COMMANDS.map((command) => [command.commandKind, command]))(
    "accepts command %s",
    (_kind, command) => {
      expect(validateDomainCommand(command)).toBe(command);
      expect(validatePhase2PlcMessage(commandMessage(command))).toEqual(commandMessage(command));
    },
  );

  it.each(DOMAIN_QUERIES.map((query) => [query.queryKind, query]))(
    "accepts query %s",
    (_kind, query) => {
      expect(validateDomainQuery(query)).toBe(query);
      expect(validatePhase2PlcMessage(queryMessage(query))).toEqual(queryMessage(query));
    },
  );

  it("rejects unknown commands and queries and exact-key violations", () => {
    expectInvalid(() => validateDomainCommand({ commandKind: "runtime.teleport" }));
    expectInvalid(() => validateDomainQuery({ queryKind: "program.get-everything" }));
    expectInvalid(() => validateDomainCommand({ ...DOMAIN_COMMANDS[0], uiSelection: true }));
    expectInvalid(() => validateDomainQuery({ ...DOMAIN_QUERIES[0], optionalFutureField: null }));
  });

  it("enforces compile scope, revision, idempotency, and captured-query rules", () => {
    expectInvalid(() => validateDomainCommand({
      commandKind: "build.compile",
      controllerId: uuid(10),
      scope: "CurrentObject",
      selectedObjectId: null,
    }));
    expectInvalid(() => validateDomainCommand({
      commandKind: "build.compile",
      controllerId: uuid(10),
      scope: "ControllerBuild",
      selectedObjectId: uuid(2),
    }));

    const duplicateRevision = clone(commandMessage(DOMAIN_COMMANDS[0])) as Record<string, any>;
    duplicateRevision.context.expectedObjectRevisions.push(
      duplicateRevision.context.expectedObjectRevisions[0],
    );
    expectInvalid(() => validatePhase2PlcMessage(duplicateRevision));

    const invalidIdempotency = clone(commandMessage(DOMAIN_COMMANDS[0])) as Record<string, any>;
    invalidIdempotency.context.idempotencyKey = "contains spaces";
    expectInvalid(() => validatePhase2PlcMessage(invalidIdempotency));

    const missingCapturedRevision = clone(queryMessage(DOMAIN_QUERIES[0])) as Record<string, any>;
    delete missingCapturedRevision.context.atProjectRevision;
    expectInvalid(() => validatePhase2PlcMessage(missingCapturedRevision));

    const current = clone(queryMessage(DOMAIN_QUERIES[0])) as Record<string, any>;
    current.context = { consistency: "current", queryId: uuid(105) };
    expect(validatePhase2PlcMessage(current)).toBe(current);
    current.context.atProjectRevision = "12";
    expectInvalid(() => validatePhase2PlcMessage(current));
  });

  it("rejects UI layout in the semantic graph and dangling or duplicate graph identities", () => {
    const layout = clone(DOMAIN_COMMANDS.find((entry) => entry.commandKind === "program.replace-graph-body")) as Record<string, any>;
    layout.graph.networks[0].nodes[0].x = 40;
    expectInvalid(() => validateDomainCommand(layout));

    const dangling = clone(layout) as Record<string, any>;
    delete dangling.graph.networks[0].nodes[0].x;
    dangling.graph.networks[0].edges[0].sourcePortId = uuid(999);
    expectInvalid(() => validateDomainCommand(dangling));

    const duplicate = clone(dangling) as Record<string, any>;
    duplicate.graph.networks[0].edges[0].sourcePortId = uuid(42);
    duplicate.graph.networks[0].nodes[1].ports[0].id = uuid(42);
    expectInvalid(() => validateDomainCommand(duplicate));
  });

  it("enforces role-specific interface metadata and globally unique member IDs", () => {
    const extra = clone(DOMAIN_COMMANDS.find((entry) => entry.commandKind === "program.replace-interface")) as Record<string, any>;
    extra.interface.temps[0].startValue = BOOL_VALUE;
    expectInvalid(() => validateDomainCommand(extra));

    const duplicate = clone(extra) as Record<string, any>;
    delete duplicate.interface.temps[0].startValue;
    duplicate.interface.temps[0].id = duplicate.interface.inputs[0].id;
    expectInvalid(() => validateDomainCommand(duplicate));
  });
});

describe("canonical UI-readable receipts", () => {
  it.each(RECEIPTS.map((receipt) => [receipt.domain, receipt]))(
    "accepts %s receipts",
    (_domain, receipt) => {
      expect(validateDomainReceipt(receipt)).toBe(receipt);
    },
  );

  it("returns ordered project identity, lifecycle, references, revisions, and dirty/build state", () => {
    const message = {
      inReplyTo: uuid(104),
      kind: "plc.result",
      result: {
        diagnostics: [],
        outcome: "ok",
        queryId: QUERY_CONTEXT.queryId,
        queryKind: "project.get-summary",
        receipt: PROJECT_SNAPSHOT,
        resultKind: "query",
        snapshotHash: HASH_A,
      },
      schemaVersion: 2,
    };
    expect(validatePhase2PlcMessage(message)).toBe(message);
  });

  it("supports exact single-object project snapshots", () => {
    const receipt = clone(PROJECT_SNAPSHOT) as Record<string, any>;
    receipt.scope = "object";
    receipt.objects = [receipt.objects[1]];
    expect(validateDomainReceipt(receipt)).toBe(receipt);
  });

  it("rejects duplicate identities, UI state, incorrect object scope, and unknown domains", () => {
    const duplicateObjects = clone(PROJECT_SNAPSHOT) as Record<string, any>;
    duplicateObjects.objects.push(duplicateObjects.objects[0]);
    expectInvalid(() => validateDomainReceipt(duplicateObjects));

    const uiTruth = clone(PROJECT_SNAPSHOT) as Record<string, any>;
    uiTruth.objects[0].expanded = true;
    expectInvalid(() => validateDomainReceipt(uiTruth));

    const objectScope = clone(PROJECT_SNAPSHOT) as Record<string, any>;
    objectScope.scope = "object";
    expectInvalid(() => validateDomainReceipt(objectScope));

    const duplicateHardware = clone(HARDWARE_SNAPSHOT) as Record<string, any>;
    duplicateHardware.objects[1].id = duplicateHardware.objects[0].id;
    expectInvalid(() => validateDomainReceipt(duplicateHardware));

    expectInvalid(() => validateDomainReceipt({ domain: "ui-owned-cache" }));
  });

  it("reconciles build attempt, report, and artifact identities", () => {
    const attemptMismatch = clone(BUILD_RECEIPT) as Record<string, any>;
    attemptMismatch.report.attemptId = uuid(999);
    expectInvalid(() => validateDomainReceipt(attemptMismatch));

    const artifactMismatch = clone(BUILD_RECEIPT) as Record<string, any>;
    artifactMismatch.report.artifactFingerprint = HASH_C;
    expectInvalid(() => validateDomainReceipt(artifactMismatch));

    const failedWithArtifact = clone(BUILD_RECEIPT) as Record<string, any>;
    failedWithArtifact.report.outcome = "Blocked";
    failedWithArtifact.report.artifactFingerprint = null;
    failedWithArtifact.report.semanticFingerprint = null;
    expectInvalid(() => validateDomainReceipt(failedWithArtifact));
  });

  it("carries canonical program bodies and reconciles their language and source revision", () => {
    const graphReceipt = clone(PROGRAM_SNAPSHOT) as Record<string, any>;
    graphReceipt.blocks[0].bodyLanguage = "LAD";
    graphReceipt.blocks[0].body = { bodyKind: "graph", graph: clone(GRAPH) };
    graphReceipt.blocks[0].sourceRevisionHash = GRAPH.sourceRevisionHash;
    expect(validateDomainReceipt(graphReceipt)).toBe(graphReceipt);

    const languageMismatch = clone(graphReceipt) as Record<string, any>;
    languageMismatch.blocks[0].bodyLanguage = "FBD";
    expectInvalid(() => validateDomainReceipt(languageMismatch));

    const revisionMismatch = clone(PROGRAM_SNAPSHOT) as Record<string, any>;
    revisionMismatch.blocks[0].body.sourceHash = HASH_A;
    expectInvalid(() => validateDomainReceipt(revisionMismatch));
  });
});

describe("events and transactionally coherent results", () => {
  it("accepts every event family", () => {
    const events = [
      PROJECT_EVENT,
      {
        artifactFingerprint: HASH_A,
        attemptId: uuid(20),
        eventId: uuid(111),
        eventKind: "build.completed",
        eventSequence: "31",
        outcome: "Success",
        transactionId: null,
      },
      {
        controllerEpoch: "4",
        controllerId: uuid(10),
        cpuState: "RUN",
        eventId: uuid(112),
        eventKind: "runtime.state-changed",
        eventSequence: "32",
        transactionId: null,
        virtualTimestampMilliseconds: "1000",
      },
      {
        action: "preview",
        controllerId: uuid(10),
        eventId: uuid(113),
        eventKind: "commissioning.changed",
        eventSequence: "33",
        previewId: uuid(81),
        transactionId: null,
      },
      {
        eventId: uuid(114),
        eventKind: "monitoring.samples",
        eventSequence: "34",
        samples: [OBSERVED_VALUE],
        subscriptionId: uuid(82),
        transactionId: null,
      },
      {
        diagnosticIds: [uuid(21)],
        eventId: uuid(115),
        eventKind: "diagnostics.changed",
        eventSequence: "35",
        transactionId: null,
      },
    ];
    for (const event of events) {
      expect(validateDomainEvent(event)).toBe(event);
    }
  });

  it("accepts a committed command receipt whose event is in the same transaction", () => {
    const result = {
      inReplyTo: uuid(103),
      kind: "plc.result",
      result: {
        affectedObjectIds: [uuid(1)],
        afterProjectHash: HASH_B,
        beforeProjectHash: HASH_A,
        commandId: COMMAND_CONTEXT.commandId,
        commandKind: "project.rename-object",
        diagnostics: [],
        events: [PROJECT_EVENT],
        idempotencyKey: COMMAND_CONTEXT.idempotencyKey,
        outcome: "committed",
        projectRevisionAfter: "13",
        projectRevisionBefore: "12",
        receipt: RECEIPTS[1],
        resultKind: "command",
        transactionId: COMMAND_CONTEXT.transactionId,
        undoToken: "undo-100",
      },
      schemaVersion: 2,
    };
    expect(validatePhase2PlcMessage(result)).toBe(result);

    const missingReceipt = clone(result) as Record<string, any>;
    missingReceipt.result.receipt = null;
    expectInvalid(() => validatePhase2PlcMessage(missingReceipt));

    const wrongDomain = clone(result) as Record<string, any>;
    wrongDomain.result.receipt = RECEIPTS[4];
    expectInvalid(() => validatePhase2PlcMessage(wrongDomain));
  });

  it("accepts rejected commands only when they claim no mutation", () => {
    const result = {
      inReplyTo: uuid(103),
      kind: "plc.result",
      result: {
        affectedObjectIds: [],
        afterProjectHash: null,
        beforeProjectHash: HASH_A,
        commandId: COMMAND_CONTEXT.commandId,
        commandKind: "project.rename-object",
        diagnostics: [DIAGNOSTIC],
        events: [],
        idempotencyKey: COMMAND_CONTEXT.idempotencyKey,
        outcome: "rejected",
        projectRevisionAfter: null,
        projectRevisionBefore: "12",
        receipt: null,
        resultKind: "command",
        transactionId: COMMAND_CONTEXT.transactionId,
        undoToken: null,
      },
      schemaVersion: 2,
    };
    expect(validatePhase2PlcMessage(result)).toBe(result);

    const mutationClaim = clone(result) as Record<string, any>;
    mutationClaim.result.afterProjectHash = HASH_B;
    expectInvalid(() => validatePhase2PlcMessage(mutationClaim));
  });

  it("rejects mismatched transaction IDs and non-increasing event sequence", () => {
    const base = {
      inReplyTo: uuid(103),
      kind: "plc.result",
      result: {
        affectedObjectIds: [uuid(1)],
        afterProjectHash: HASH_B,
        beforeProjectHash: HASH_A,
        commandId: COMMAND_CONTEXT.commandId,
        commandKind: "project.rename-object",
        diagnostics: [],
        events: [PROJECT_EVENT],
        idempotencyKey: COMMAND_CONTEXT.idempotencyKey,
        outcome: "committed",
        projectRevisionAfter: "13",
        projectRevisionBefore: "12",
        receipt: RECEIPTS[1],
        resultKind: "command",
        transactionId: COMMAND_CONTEXT.transactionId,
        undoToken: "undo-100",
      },
      schemaVersion: 2,
    };
    const mismatch = clone(base) as Record<string, any>;
    mismatch.result.events[0].transactionId = uuid(999);
    expectInvalid(() => validatePhase2PlcMessage(mismatch));

    const order = clone(base) as Record<string, any>;
    order.result.events.push({ ...order.result.events[0], eventId: uuid(998) });
    expectInvalid(() => validatePhase2PlcMessage(order));
  });

  it("requires successful query snapshots and strips them from rejections", () => {
    const ok = {
      inReplyTo: uuid(104),
      kind: "plc.result",
      result: {
        diagnostics: [],
        outcome: "ok",
        queryId: QUERY_CONTEXT.queryId,
        queryKind: "hardware.get-configuration",
        receipt: HARDWARE_SNAPSHOT,
        resultKind: "query",
        snapshotHash: HASH_A,
      },
      schemaVersion: 2,
    };
    expect(validatePhase2PlcMessage(ok)).toBe(ok);

    const missingReceipt = clone(ok) as Record<string, any>;
    missingReceipt.result.receipt = null;
    expectInvalid(() => validatePhase2PlcMessage(missingReceipt));

    const rejectedWithSnapshot = clone(ok) as Record<string, any>;
    rejectedWithSnapshot.result.outcome = "rejected";
    expectInvalid(() => validatePhase2PlcMessage(rejectedWithSnapshot));

    const wrongDomain = clone(ok) as Record<string, any>;
    wrongDomain.result.queryKind = "project.get-summary";
    expectInvalid(() => validatePhase2PlcMessage(wrongDomain));

    const diagnostics = clone(ok) as Record<string, any>;
    diagnostics.result.queryKind = "diagnostics.list";
    diagnostics.result.receipt = RECEIPTS.at(-1);
    expect(validatePhase2PlcMessage(diagnostics)).toBe(diagnostics);

    const projectObject = clone(ok) as Record<string, any>;
    const objectReceipt = clone(PROJECT_SNAPSHOT) as Record<string, any>;
    objectReceipt.scope = "object";
    objectReceipt.objects = [objectReceipt.objects[1]];
    projectObject.result.queryKind = "project.get-object";
    projectObject.result.receipt = objectReceipt;
    expect(validatePhase2PlcMessage(projectObject)).toBe(projectObject);

    projectObject.result.receipt.scope = "summary";
    expectInvalid(() => validatePhase2PlcMessage(projectObject));
  });

  it("requires subscription identity for asynchronous monitoring sample messages", () => {
    const message = {
      event: {
        eventId: uuid(114),
        eventKind: "monitoring.samples",
        eventSequence: "34",
        samples: [OBSERVED_VALUE],
        subscriptionId: uuid(82),
        transactionId: null,
      },
      kind: "plc.event",
      schemaVersion: 2,
      subscriptionId: uuid(82),
    };
    expect(validatePhase2PlcMessage(message)).toBe(message);
    expectInvalid(() => validatePhase2PlcMessage({ ...message, subscriptionId: null }));
    expectInvalid(() => validatePhase2PlcMessage({ ...message, subscriptionId: uuid(999) }));
  });
});

describe("strict and bounded wire serialization", () => {
  it("round-trips a validated message", () => {
    const message = queryMessage({ projectRootId: uuid(1), queryKind: "project.get-summary" });
    expect(decodePlcMessage(encodePlcMessage(message))).toEqual(message);
  });

  it("rejects malformed JSON, non-object JSON, unknown versions and unknown message kinds", () => {
    expectInvalid(() => decodePlcMessage("{"));
    expectInvalid(() => decodePlcMessage("[]"));
    expectInvalid(() => validatePlcMessage({ kind: "plc.query", schemaVersion: 99 }));
    expectInvalid(() => validatePlcMessage({ kind: "plc.future", schemaVersion: 2 }));
  });

  it("rejects extra keys at every envelope level", () => {
    const top = { ...queryMessage(DOMAIN_QUERIES[0]), extra: true };
    expectInvalid(() => validatePlcMessage(top));

    const nested = clone(queryMessage(DOMAIN_QUERIES[0])) as Record<string, any>;
    nested.context.extra = true;
    expectInvalid(() => validatePlcMessage(nested));
  });

  it("rejects serialized messages over the UTF-8 byte limit", () => {
    const large = clone(commandMessage(DOMAIN_COMMANDS.find((entry) => entry.commandKind === "program.replace-scl-body"))) as Record<string, any>;
    large.command.sourceText = "é".repeat(PLC_CONTRACT_LIMITS.sourceCharacters);
    expectInvalid(() => encodePlcMessage(large));
    expectInvalid(() => decodePlcMessage(`"${"é".repeat(PLC_CONTRACT_LIMITS.messageBytes)}"`));
  });

  it("does not accept class instances as transport records", () => {
    class MessageLike {
      public kind = "plc.query";
      public schemaVersion = 2;
    }
    expectInvalid(() => validatePlcMessage(new MessageLike()));
  });
});
