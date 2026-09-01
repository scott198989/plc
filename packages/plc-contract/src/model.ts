/** Wire contract only. This package contains no PLC execution semantics. */
export const PLC_CONTRACT_SCHEMA_VERSION = 2 as const;
export const PLC_MESSAGE_KIND = {
  command: "plc.command",
  event: "plc.event",
  query: "plc.query",
  result: "plc.result",
} as const;

export const FOUNDATION_COMPATIBILITY = {
  buildIdentity: "foundation-core@0.1.0",
  commandKind: "foundation.health",
  healthyState: "HEALTHY",
  requestId: "phase1-foundation-health",
  resultKind: "domain.result",
  schemaVersion: 1,
  stateHash:
    "64E21C28C534606DD9C9AA27A56C928DC09574CD70B56B6D468FE3F96C2F5A94",
} as const;

export const PLC_CONTRACT_LIMITS = {
  diagnosticCount: 2_048,
  diagnosticMessageCharacters: 4_096,
  eventCount: 2_048,
  graphEdges: 16_384,
  graphNetworks: 1_024,
  graphNodes: 8_192,
  graphPortsPerNode: 256,
  identifierCharacters: 128,
  interfaceMembersPerRole: 4_096,
  messageBytes: 1_048_576,
  objectIds: 16_384,
  projectNameCharacters: 256,
  relatedAnchors: 256,
  sourceCharacters: 524_288,
  sourceRangeBytes: 524_288,
  stringCodeUnits: 65_535,
  typedValueDepth: 32,
  typedValueElements: 16_384,
} as const;

export type Uuid = string;
export type Sha256 = string;
export type DecimalUInt64 = string;
export type DecimalInt64 = string;
export type IdempotencyKey = string;
export type UndoToken = string;
export type CanonicalTypeId = string;

export type FoundationHealthCompatibilityCommand = Readonly<{
  kind: typeof FOUNDATION_COMPATIBILITY.commandKind;
  requestId: typeof FOUNDATION_COMPATIBILITY.requestId;
  schemaVersion: typeof FOUNDATION_COMPATIBILITY.schemaVersion;
}>;

export type FoundationHealthCompatibilityDiagnostic = Readonly<{
  code: "INVALID_COMMAND" | "INVALID_WASM" | "WORKER_FAILURE";
  message: string;
  severity: "error";
}>;

type FoundationHealthCompatibilityEnvelope = Readonly<{
  affectedObjectIds: readonly [];
  afterHash: typeof FOUNDATION_COMPATIBILITY.stateHash;
  beforeHash: typeof FOUNDATION_COMPATIBILITY.stateHash;
  events: readonly [];
  kind: typeof FOUNDATION_COMPATIBILITY.resultKind;
  requestId: typeof FOUNDATION_COMPATIBILITY.requestId;
  schemaVersion: typeof FOUNDATION_COMPATIBILITY.schemaVersion;
  undoToken?: UndoToken;
}>;

export type FoundationHealthCompatibilitySuccess =
  FoundationHealthCompatibilityEnvelope &
    Readonly<{
      diagnostics: readonly [];
      success: true;
      value: Readonly<{
        buildIdentity: typeof FOUNDATION_COMPATIBILITY.buildIdentity;
        healthState: typeof FOUNDATION_COMPATIBILITY.healthyState;
        schemaVersion: typeof FOUNDATION_COMPATIBILITY.schemaVersion;
        wasmSha256: Sha256;
      }>;
    }>;

export type FoundationHealthCompatibilityFailure =
  FoundationHealthCompatibilityEnvelope &
    Readonly<{
      diagnostics: readonly [FoundationHealthCompatibilityDiagnostic];
      success: false;
      value?: never;
    }>;

export type FoundationHealthCompatibilityResult =
  | FoundationHealthCompatibilityFailure
  | FoundationHealthCompatibilitySuccess;

export type SignedIntegerTypeId = "SINT" | "INT" | "DINT" | "LINT";
export type UnsignedIntegerTypeId = "USINT" | "UINT" | "UDINT" | "ULINT";
export type BitStringTypeId = "BYTE" | "WORD" | "DWORD" | "LWORD";
export type FloatingTypeId = "REAL" | "LREAL";
export type InstructionStateTypeId =
  | "EdgeState"
  | "TimerState"
  | "CounterState";

export type CanonicalBooleanValue = Readonly<{
  kind: "bool";
  typeId: "BOOL";
  value: boolean;
}>;

export type CanonicalSignedIntegerValue = Readonly<{
  kind: "signed-integer";
  typeId: SignedIntegerTypeId;
  value: DecimalInt64;
}>;

export type CanonicalUnsignedIntegerValue = Readonly<{
  kind: "unsigned-integer";
  typeId: UnsignedIntegerTypeId;
  value: DecimalUInt64;
}>;

export type CanonicalBitStringValue = Readonly<{
  bitsHex: string;
  kind: "bit-string";
  typeId: BitStringTypeId;
}>;

export type CanonicalFloatingValue = Readonly<{
  ieeeHex: string;
  kind: "floating";
  typeId: FloatingTypeId;
}>;

export type CanonicalCharValue = Readonly<{
  codeUnit: number;
  kind: "char";
  typeId: "CHAR";
}>;

export type CanonicalStringValue = Readonly<{
  capacity: number;
  codeUnits: readonly number[];
  kind: "string";
  typeId: "STRING";
}>;

export type CanonicalTimeValue = Readonly<{
  kind: "time";
  milliseconds: DecimalInt64;
  typeId: "TIME";
}>;

export type CanonicalArrayBound = Readonly<{
  lower: number;
  upper: number;
}>;

export type CanonicalArrayValue = Readonly<{
  bounds: readonly CanonicalArrayBound[];
  elements: readonly CanonicalTypedValue[];
  kind: "array";
  typeId: CanonicalTypeId;
}>;

export type CanonicalStructMemberValue = Readonly<{
  memberId: Uuid;
  value: CanonicalTypedValue;
}>;

export type CanonicalStructValue = Readonly<{
  kind: "struct";
  members: readonly CanonicalStructMemberValue[];
  typeId: CanonicalTypeId;
}>;

/** Instruction state is intentionally opaque at this boundary. */
export type CanonicalInstructionStateValue = Readonly<{
  kind: "instruction-state";
  stateFingerprint: Sha256;
  typeId: InstructionStateTypeId;
}>;

export type CanonicalTypedValue =
  | CanonicalArrayValue
  | CanonicalBitStringValue
  | CanonicalBooleanValue
  | CanonicalCharValue
  | CanonicalFloatingValue
  | CanonicalInstructionStateValue
  | CanonicalSignedIntegerValue
  | CanonicalStringValue
  | CanonicalStructValue
  | CanonicalTimeValue
  | CanonicalUnsignedIntegerValue;

export type Utf8ByteRange = Readonly<{
  endExclusive: number;
  start: number;
}>;

export type ProjectSourceAnchor = Readonly<{
  anchorKind: "project";
  ownerObjectId: Uuid;
  propertyPath: readonly string[];
  sourceRevisionHash: Sha256;
}>;

export type TextSourceAnchor = Readonly<{
  anchorKind: "text";
  language: "SCL";
  ownerObjectId: Uuid;
  range: Utf8ByteRange;
  semanticNodeId?: Uuid;
  sourceRevisionHash: Sha256;
}>;

export type GraphSourceAnchor = Readonly<{
  anchorKind: "graph";
  edgeId?: Uuid;
  language: "LAD" | "FBD";
  networkId: Uuid;
  nodeId?: Uuid;
  ownerObjectId: Uuid;
  portId?: Uuid;
  semanticNodeId?: Uuid;
  sourceRevisionHash: Sha256;
}>;

export type GeneratedSourceAnchor = Readonly<{
  anchorKind: "generated";
  causalAnchor: ProjectSourceAnchor | TextSourceAnchor | GraphSourceAnchor;
  ownerObjectId: Uuid;
  sourceRevisionHash: Sha256;
}>;

export type SourceAnchor =
  | GeneratedSourceAnchor
  | GraphSourceAnchor
  | ProjectSourceAnchor
  | TextSourceAnchor;

export type DiagnosticParameter =
  | Readonly<{ kind: "boolean"; name: string; value: boolean }>
  | Readonly<{ kind: "decimal"; name: string; value: DecimalInt64 }>
  | Readonly<{ kind: "hash"; name: string; value: Sha256 }>
  | Readonly<{ kind: "identity"; name: string; value: Uuid }>
  | Readonly<{ kind: "text"; name: string; value: string }>;

export type DiagnosticSeverity = "Info" | "Warning" | "Error" | "Internal";

export type Diagnostic = Readonly<{
  blocking: boolean;
  buildAttemptId?: Uuid;
  cause: string;
  code: string;
  diagnosticId: Uuid;
  parameters: readonly DiagnosticParameter[];
  phase: string;
  primaryAnchor: SourceAnchor;
  recoveryHint: string;
  relatedAnchors: readonly SourceAnchor[];
  severity: DiagnosticSeverity;
  snapshotHash?: Sha256;
}>;

export type ExpectedObjectRevision = Readonly<{
  objectId: Uuid;
  revision: DecimalUInt64;
}>;

export type CommandContext = Readonly<{
  commandId: Uuid;
  expectedObjectRevisions: readonly ExpectedObjectRevision[];
  expectedProjectRevision: DecimalUInt64;
  idempotencyKey: IdempotencyKey;
  issuedSequence: DecimalUInt64;
  transactionId: Uuid;
}>;

export type QueryContext =
  | Readonly<{
      atProjectRevision?: never;
      consistency: "current";
      queryId: Uuid;
    }>
  | Readonly<{
      atProjectRevision: DecimalUInt64;
      consistency: "captured";
      queryId: Uuid;
    }>;

export type InterfaceInput = Readonly<{
  comment: string;
  declaredOrder: DecimalUInt64;
  defaultValue?: CanonicalTypedValue;
  id: Uuid;
  name: string;
  role: "Input";
  typeId: CanonicalTypeId;
}>;

export type InterfaceOutput = Readonly<{
  comment: string;
  declaredOrder: DecimalUInt64;
  id: Uuid;
  name: string;
  requiredOutputBinding: boolean;
  role: "Output";
  startValue?: CanonicalTypedValue;
  typeId: CanonicalTypeId;
}>;

export type InterfaceInOut = Readonly<{
  comment: string;
  declaredOrder: DecimalUInt64;
  id: Uuid;
  name: string;
  role: "InOut";
  typeId: CanonicalTypeId;
}>;

export type InterfaceStatic = Readonly<{
  comment: string;
  declaredOrder: DecimalUInt64;
  id: Uuid;
  name: string;
  retainPolicy?: "Retentive" | "NonRetentive";
  role: "Static";
  startValue?: CanonicalTypedValue;
  typeId: CanonicalTypeId;
}>;

export type InterfaceTemp = Readonly<{
  comment: string;
  declaredOrder: DecimalUInt64;
  id: Uuid;
  name: string;
  role: "Temp";
  typeId: CanonicalTypeId;
}>;

export type InterfaceConstant = Readonly<{
  comment: string;
  declaredOrder: DecimalUInt64;
  id: Uuid;
  name: string;
  role: "Constant";
  typeId: CanonicalTypeId;
  value: CanonicalTypedValue;
}>;

export type InterfaceReturn = Readonly<{
  comment: string;
  id: Uuid;
  name: string;
  role: "Return";
  typeId: CanonicalTypeId;
}>;

export type BlockInterfaceContract = Readonly<{
  constants: readonly InterfaceConstant[];
  inOuts: readonly InterfaceInOut[];
  inputs: readonly InterfaceInput[];
  outputs: readonly InterfaceOutput[];
  return: InterfaceReturn | null;
  statics: readonly InterfaceStatic[];
  temps: readonly InterfaceTemp[];
}>;

export type GraphPortContract = Readonly<{
  direction: "input" | "output";
  id: Uuid;
  required: boolean;
  role: "power" | "data" | "execution" | "activation" | "status";
  typeId: CanonicalTypeId | null;
}>;

export type GraphNodeContract = Readonly<{
  definitionId: string;
  id: Uuid;
  ports: readonly GraphPortContract[];
  semanticOrder: DecimalUInt64;
  stateBindingId: Uuid | null;
}>;

export type GraphEdgeContract = Readonly<{
  edgeKind: "power" | "data" | "execution";
  id: Uuid;
  sourcePortId: Uuid;
  targetPortId: Uuid;
}>;

export type GraphNetworkContract = Readonly<{
  edges: readonly GraphEdgeContract[];
  id: Uuid;
  nodes: readonly GraphNodeContract[];
  semanticOrder: DecimalUInt64;
}>;

export type SemanticGraphContract = Readonly<{
  language: "LAD" | "FBD";
  networks: readonly GraphNetworkContract[];
  sourceRevisionHash: Sha256;
}>;

export type ProjectPayloadValue =
  | null
  | boolean
  | string
  | Readonly<{ $type: "i64" | "u64"; value: string }>
  | readonly ProjectPayloadValue[]
  | Readonly<{
      $type: "record";
      value: Readonly<Record<string, ProjectPayloadValue>>;
    }>;

export type ProjectPayload = Readonly<Record<string, ProjectPayloadValue>>;

export type ProjectCommand =
  | Readonly<{
      commandKind: "project.create";
      displayName: string;
      documentId: Uuid;
      projectRootId: Uuid;
    }>
  | Readonly<{
      commandKind: "project.create-object";
      displayName: string;
      objectId: Uuid;
      objectKind:
        | "folder"
        | "controller"
        | "rack"
        | "module"
        | "network"
        | "symbol-table"
        | "tag"
        | "type-definition"
        | "program-block"
        | "data-block"
        | "build-record"
        | "snapshot-reference"
        | "generic";
      parentId: Uuid;
      payloadSchema: string;
    }>
  | Readonly<{
      commandKind: "project.rename-object";
      displayName: string;
      objectId: Uuid;
    }>
  | Readonly<{
      commandKind: "project.set-semantic-field" | "project.set-presentation-field";
      key: string;
      objectId: Uuid;
      value: ProjectPayloadValue;
    }>
  | Readonly<{
      commandKind: "project.replace-semantic-payload";
      objectId: Uuid;
      semanticPayload: ProjectPayload;
    }>
  | Readonly<{
      commandKind: "project.move-object";
      objectId: Uuid;
      orderKey: DecimalUInt64;
      parentId: Uuid;
    }>
  | Readonly<{
      commandKind: "project.delete-object";
      objectId: Uuid;
    }>
  | Readonly<{
      commandKind: "project.copy-objects";
      sourceObjectIds: readonly Uuid[];
      targetParentId: Uuid;
    }>
  | Readonly<{
      commandKind: "project.undo" | "project.redo";
      undoToken: UndoToken;
    }>;

export type PersistenceCommand =
  | Readonly<{
      commandKind: "persistence.open";
      sourceGrantId: Uuid;
    }>
  | Readonly<{
      commandKind: "persistence.save";
      documentId: Uuid;
      mode: "save" | "save-as";
      targetGrantId: Uuid;
    }>
  | Readonly<{
      commandKind: "persistence.recover";
      recoveryJournalId: Uuid;
    }>;

export type HardwareCommand =
  | Readonly<{
      commandKind: "hardware.configure-controller";
      controllerId: Uuid;
      profileId: string;
      profileVersion: string;
    }>
  | Readonly<{
      catalogId: string;
      commandKind: "hardware.upsert-device";
      controllerId: Uuid;
      deviceId: Uuid;
      parentId: Uuid | null;
      slot: number | null;
    }>
  | Readonly<{
      area: "I" | "Q" | "M";
      bitOffset: number | null;
      byteOffset: number;
      commandKind: "hardware.assign-address";
      controllerId: Uuid;
      objectId: Uuid;
      widthBits: number;
    }>
  | Readonly<{
      commandKind: "hardware.assign-virtual-network";
      controllerId: Uuid;
      interfaceId: Uuid;
      subnetId: Uuid;
      virtualAddress: string;
    }>;

export type ProgramCommand =
  | Readonly<{
      blockId: Uuid;
      blockKind: "OB" | "FC" | "FB";
      commandKind: "program.create-block";
      controllerId: Uuid;
      displayName: string;
      engineeringNumber: number | null;
      language: "LAD" | "FBD" | "SCL";
      obRole: "CyclicMain" | "Startup" | "TimedCyclic" | null;
      offsetMilliseconds: number | null;
      periodMilliseconds: number | null;
    }>
  | Readonly<{
      blockId: Uuid;
      commandKind: "program.replace-interface";
      controllerId: Uuid;
      interface: BlockInterfaceContract;
    }>
  | Readonly<{
      blockId: Uuid;
      commandKind: "program.replace-scl-body";
      controllerId: Uuid;
      sourceHash: Sha256;
      sourceText: string;
    }>
  | Readonly<{
      blockId: Uuid;
      commandKind: "program.replace-graph-body";
      controllerId: Uuid;
      graph: SemanticGraphContract;
    }>;

export type CompileScope =
  | "CurrentObject"
  | "SoftwareChanges"
  | "RebuildAllSoftware"
  | "VirtualHardware"
  | "ControllerBuild";

export type BuildCommand =
  | Readonly<{
      commandKind: "build.compile";
      controllerId: Uuid;
      selectedObjectId: Uuid | null;
      scope: CompileScope;
    }>
  | Readonly<{
      attemptId: Uuid;
      commandKind: "build.cancel";
    }>;

export type RuntimeCommand =
  | Readonly<{
      commandKind: "runtime.set-mode";
      controllerId: Uuid;
      expectedControllerEpoch: DecimalUInt64;
      mode: "RUN" | "STOP";
    }>
  | Readonly<{
      commandKind: "runtime.advance-virtual-time";
      deltaMilliseconds: DecimalUInt64;
      universeId: Uuid;
    }>
  | Readonly<{
      commandKind: "runtime.set-virtual-input";
      controllerId: Uuid;
      expectedControllerEpoch: DecimalUInt64;
      targetId: Uuid;
      value: CanonicalTypedValue;
    }>;

export type CommissioningCommand =
  | Readonly<{
      artifactPackageFingerprint: Sha256;
      commandKind: "commissioning.preview-load";
      controllerId: Uuid;
      expectedControllerEpoch: DecimalUInt64;
    }>
  | Readonly<{
      commandKind: "commissioning.commit-load";
      controllerId: Uuid;
      expectedControllerEpoch: DecimalUInt64;
      previewFingerprint: Sha256;
      previewId: Uuid;
    }>
  | Readonly<{
      commandKind: "commissioning.cancel-preview";
      previewId: Uuid;
    }>;

export type MonitoringCommand =
  | Readonly<{
      commandKind: "monitoring.subscribe";
      controllerId: Uuid;
      expectedControllerEpoch: DecimalUInt64;
      targetIds: readonly Uuid[];
    }>
  | Readonly<{
      commandKind: "monitoring.unsubscribe";
      subscriptionId: Uuid;
    }>
  | Readonly<{
      commandKind: "monitoring.modify";
      controllerId: Uuid;
      expectedControllerEpoch: DecimalUInt64;
      targetId: Uuid;
      value: CanonicalTypedValue;
    }>
  | Readonly<{
      commandKind: "monitoring.create-force";
      controllerId: Uuid;
      expectedControllerEpoch: DecimalUInt64;
      expectedForceRevision: DecimalUInt64;
      targetId: Uuid;
      value: CanonicalTypedValue;
    }>
  | Readonly<{
      commandKind: "monitoring.remove-force";
      controllerId: Uuid;
      expectedControllerEpoch: DecimalUInt64;
      forceId: Uuid;
    }>
  | Readonly<{
      commandKind: "monitoring.trace-control";
      controllerId: Uuid;
      expectedControllerEpoch: DecimalUInt64;
      operation: "arm" | "start" | "stop" | "abort";
      traceId: Uuid;
    }>;

export type DomainCommand =
  | BuildCommand
  | CommissioningCommand
  | HardwareCommand
  | MonitoringCommand
  | PersistenceCommand
  | ProgramCommand
  | ProjectCommand
  | RuntimeCommand;

export type DomainQuery =
  | Readonly<{ queryKind: "system.health" }>
  | Readonly<{ projectRootId: Uuid; queryKind: "project.get-summary" }>
  | Readonly<{ objectId: Uuid; queryKind: "project.get-object" }>
  | Readonly<{ documentId: Uuid; queryKind: "persistence.get-status" }>
  | Readonly<{
      controllerId: Uuid;
      queryKind: "hardware.get-configuration";
    }>
  | Readonly<{ blockId: Uuid; queryKind: "program.get-block" }>
  | Readonly<{ attemptId: Uuid; queryKind: "build.get-report" }>
  | Readonly<{
      controllerId: Uuid;
      expectedControllerEpoch: DecimalUInt64;
      queryKind: "runtime.get-status";
    }>
  | Readonly<{
      previewId: Uuid;
      queryKind: "commissioning.get-preview";
    }>
  | Readonly<{
      queryKind: "monitoring.get-snapshot";
      subscriptionId: Uuid;
    }>
  | Readonly<{
      blocking: boolean | null;
      phase: string | null;
      queryKind: "diagnostics.list";
    }>;

export type HealthReceipt = Readonly<{
  contractSchemaVersion: typeof PLC_CONTRACT_SCHEMA_VERSION;
  coreVersion: string;
  domain: "health";
  status: "HEALTHY";
  supportedMessageSchemaVersions: readonly [1, 2];
  wasmSha256: Sha256;
}>;

export type ProjectReceipt = Readonly<{
  affectedObjectIds: readonly Uuid[];
  documentId: Uuid;
  documentRevision: DecimalUInt64;
  domain: "project";
  projectHash: Sha256;
  projectRootId: Uuid;
  semanticRevision: DecimalUInt64;
}>;

export type ProjectObjectKind =
  | "ProjectRoot"
  | "Folder"
  | "Controller"
  | "Device"
  | "Rack"
  | "Module"
  | "VirtualNetwork"
  | "VirtualInterface"
  | "SymbolTable"
  | "Tag"
  | "Constant"
  | "NamedType"
  | "OB"
  | "FC"
  | "FB"
  | "GlobalDB"
  | "InstanceDB"
  | "HmiScreen"
  | "WatchTable"
  | "TraceConfiguration"
  | "BuildRecord"
  | "SnapshotReference";

export type ProjectReferenceSnapshot = Readonly<{
  expectedTargetKind: ProjectObjectKind;
  referenceId: Uuid;
  resolution: "resolved" | "unresolved" | "tombstoned";
  sourceAnchor: SourceAnchor;
  targetId: Uuid;
}>;

export type ProjectObjectSnapshot = Readonly<{
  creationOrdinal: DecimalUInt64;
  displayName: string;
  id: Uuid;
  kind: ProjectObjectKind;
  lifecycle: "active" | "tombstoned";
  objectRevision: DecimalUInt64;
  orderedChildIds: readonly Uuid[];
  parentId: Uuid | null;
  references: readonly ProjectReferenceSnapshot[];
  semanticRevision: DecimalUInt64;
}>;

export type ControllerBuildStateSnapshot = Readonly<{
  controllerId: Uuid;
  hardware: "not-built" | "current" | "stale" | "blocked";
  loadedArtifactFingerprint: Sha256 | null;
  software: "not-built" | "current" | "stale" | "blocked";
}>;

export type DirtyBuildStateSnapshot = Readonly<{
  controllerStates: readonly ControllerBuildStateSnapshot[];
  currentDocumentHash: Sha256;
  currentSemanticFingerprint: Sha256;
  documentDirty: boolean;
  savedDocumentHash: Sha256 | null;
  savedDocumentRevision: DecimalUInt64 | null;
  savedSemanticFingerprint: Sha256 | null;
  semanticDirty: boolean;
}>;

export type ProjectSnapshotReceipt = Readonly<{
  dirtyBuildState: DirtyBuildStateSnapshot;
  documentId: Uuid;
  documentRevision: DecimalUInt64;
  domain: "project-snapshot";
  objects: readonly ProjectObjectSnapshot[];
  projectRootId: Uuid;
  scope: "summary" | "object";
  semanticRevision: DecimalUInt64;
}>;

export type PersistenceReceipt = Readonly<{
  action: "open" | "save" | "save-as" | "recover";
  documentId: Uuid;
  documentRevision: DecimalUInt64;
  domain: "persistence";
  packageHash: Sha256;
  projectRootId: Uuid;
  recoveryStatus: "not-applicable" | "recovered" | "discarded";
  schemaVersion: string;
}>;

export type HardwareReceipt = Readonly<{
  affectedObjectIds: readonly Uuid[];
  configurationFingerprint: Sha256;
  controllerId: Uuid;
  domain: "hardware";
}>;

export type HardwareObjectSnapshot = Readonly<{
  address: Readonly<{
    area: "I" | "Q" | "M";
    bitOffset: number | null;
    byteOffset: number;
    widthBits: number;
  }> | null;
  catalogId: string;
  creationOrdinal: DecimalUInt64;
  displayName: string;
  id: Uuid;
  kind: "Controller" | "Device" | "Module" | "Channel" | "VirtualInterface";
  lifecycle: "active" | "tombstoned";
  objectRevision: DecimalUInt64;
  orderedChildIds: readonly Uuid[];
  parentId: Uuid | null;
  references: readonly ProjectReferenceSnapshot[];
  semanticRevision: DecimalUInt64;
  slot: number | null;
  virtualAddress: string | null;
  virtualSubnetId: Uuid | null;
}>;

export type HardwareSnapshotReceipt = Readonly<{
  configurationFingerprint: Sha256;
  controllerId: Uuid;
  domain: "hardware-snapshot";
  objects: readonly HardwareObjectSnapshot[];
  profileId: string;
  profileVersion: string;
}>;

export type ProgramReceipt = Readonly<{
  affectedObjectIds: readonly Uuid[];
  blockId: Uuid | null;
  controllerId: Uuid;
  domain: "program";
  invalidatedObjectIds: readonly Uuid[];
  publicSignatureFingerprint: Sha256 | null;
}>;

export type ProgramBodySnapshot =
  | Readonly<{
      bodyKind: "scl";
      sourceHash: Sha256;
      sourceText: string;
    }>
  | Readonly<{
      bodyKind: "graph";
      graph: SemanticGraphContract;
    }>;

export type ProgramBlockSnapshot = Readonly<{
  blockId: Uuid;
  blockKind: "OB" | "FC" | "FB";
  body: ProgramBodySnapshot;
  bodyLanguage: "LAD" | "FBD" | "SCL";
  creationOrdinal: DecimalUInt64;
  displayName: string;
  engineeringNumber: number | null;
  interface: BlockInterfaceContract;
  lifecycle: "active" | "tombstoned";
  objectRevision: DecimalUInt64;
  obRole: "CyclicMain" | "Startup" | "TimedCyclic" | null;
  offsetMilliseconds: number | null;
  orderedChildIds: readonly Uuid[];
  parentId: Uuid;
  periodMilliseconds: number | null;
  publicSignatureFingerprint: Sha256;
  references: readonly ProjectReferenceSnapshot[];
  semanticFingerprint: Sha256;
  semanticRevision: DecimalUInt64;
  sourceRevisionHash: Sha256;
}>;

export type ProgramSnapshotReceipt = Readonly<{
  blocks: readonly ProgramBlockSnapshot[];
  controllerId: Uuid;
  domain: "program-snapshot";
}>;

export type BuildAttemptRecord = Readonly<{
  attemptId: Uuid;
  compilerSemanticVersion: string;
  instructionRegistryHash: Sha256;
  requestedScope: CompileScope;
  snapshotHash: Sha256;
  trainingProfileHash: Sha256;
  typeSystemVersion: string;
}>;

export type BuildOutcome =
  | "Success"
  | "Blocked"
  | "Cancelled"
  | "Stale"
  | "ResourceLimit"
  | "InternalFailure";

export type BuildReportRecord = Readonly<{
  artifactFingerprint: Sha256 | null;
  attemptId: Uuid;
  diagnostics: readonly Diagnostic[];
  expandedClosure: readonly Uuid[];
  outcome: BuildOutcome;
  requestedScope: CompileScope;
  semanticFingerprint: Sha256 | null;
  snapshotHash: Sha256;
  stale: boolean;
}>;

export type BuildArtifactIdentity = Readonly<{
  artifactPackageFingerprint: Sha256;
  artifactSchema: string;
  irVersion: string;
  memorySchemaHash: Sha256;
  probeSchemaVersion: string;
  profileHash: Sha256;
  runtimeContractVersion: string;
  semanticBuildFingerprint: Sha256;
  sourceMapHash: Sha256;
}>;

export type BuildReceipt = Readonly<{
  artifact: BuildArtifactIdentity | null;
  attempt: BuildAttemptRecord;
  domain: "build";
  report: BuildReportRecord;
}>;

export type CpuState =
  | "POWERED_OFF"
  | "STARTUP"
  | "STOP"
  | "RUN"
  | "PAUSED"
  | "FAULTED";

export type RuntimeReceipt = Readonly<{
  affectedTargetIds: readonly Uuid[];
  controllerEpoch: DecimalUInt64;
  controllerId: Uuid;
  cpuState: CpuState;
  domain: "runtime";
  loadedArtifactFingerprint: Sha256 | null;
  scanSequence: DecimalUInt64;
  universeEpoch: DecimalUInt64;
  virtualTimestampMilliseconds: DecimalUInt64;
}>;

export type MemoryAction = Readonly<{
  action: "initialize" | "preserve" | "reset" | "reject";
  regionId: Uuid;
  reason: string;
}>;

export type CommissioningReceipt = Readonly<{
  action: "preview" | "commit" | "cancel" | "rollback";
  candidateArtifactFingerprint: Sha256;
  controllerEpoch: DecimalUInt64;
  controllerId: Uuid;
  domain: "commissioning";
  loadedArtifactFingerprint: Sha256 | null;
  memoryActions: readonly MemoryAction[];
  previewFingerprint: Sha256;
  previewId: Uuid;
}>;

export type ObservedValue = Readonly<{
  forceId: Uuid | null;
  freshness: "CURRENT" | "STALE" | "UNKNOWN";
  probeId: Uuid;
  quality: "GOOD" | "UNCERTAIN" | "BAD" | "NOT_PRESENT";
  scanSequence: DecimalUInt64;
  targetId: Uuid;
  value: CanonicalTypedValue;
  virtualTimestampMilliseconds: DecimalUInt64;
}>;

export type MonitoringReceipt = Readonly<{
  action:
    | "subscribe"
    | "unsubscribe"
    | "snapshot"
    | "modify"
    | "force-create"
    | "force-remove"
    | "trace-control";
  commandReceiptId: Uuid | null;
  controllerEpoch: DecimalUInt64;
  controllerId: Uuid;
  domain: "monitoring";
  samples: readonly ObservedValue[];
  subscriptionId: Uuid | null;
}>;

export type DiagnosticsReceipt = Readonly<{
  diagnosticRevision: DecimalUInt64;
  diagnostics: readonly Diagnostic[];
  domain: "diagnostics";
}>;

export type DomainReceipt =
  | BuildReceipt
  | CommissioningReceipt
  | DiagnosticsReceipt
  | HardwareReceipt
  | HardwareSnapshotReceipt
  | HealthReceipt
  | MonitoringReceipt
  | PersistenceReceipt
  | ProgramReceipt
  | ProgramSnapshotReceipt
  | ProjectReceipt
  | ProjectSnapshotReceipt
  | RuntimeReceipt;

type EventMetadata = Readonly<{
  eventId: Uuid;
  eventSequence: DecimalUInt64;
  transactionId: Uuid | null;
}>;

export type DomainEventRecord = EventMetadata &
  (
    | Readonly<{
        affectedObjectIds: readonly Uuid[];
        documentRevision: DecimalUInt64;
        eventKind: "project.changed";
        projectHash: Sha256;
        semanticRevision: DecimalUInt64;
      }>
    | Readonly<{
        artifactFingerprint: Sha256 | null;
        attemptId: Uuid;
        eventKind: "build.completed";
        outcome: BuildOutcome;
      }>
    | Readonly<{
        controllerEpoch: DecimalUInt64;
        controllerId: Uuid;
        cpuState: CpuState;
        eventKind: "runtime.state-changed";
        virtualTimestampMilliseconds: DecimalUInt64;
      }>
    | Readonly<{
        action: "preview" | "commit" | "cancel" | "rollback";
        controllerId: Uuid;
        eventKind: "commissioning.changed";
        previewId: Uuid;
      }>
    | Readonly<{
        eventKind: "monitoring.samples";
        samples: readonly ObservedValue[];
        subscriptionId: Uuid;
      }>
    | Readonly<{
        diagnosticIds: readonly Uuid[];
        eventKind: "diagnostics.changed";
      }>
  );

export type CommandResultBody = Readonly<{
  affectedObjectIds: readonly Uuid[];
  afterProjectHash: Sha256 | null;
  beforeProjectHash: Sha256;
  commandId: Uuid;
  commandKind: DomainCommand["commandKind"];
  diagnostics: readonly Diagnostic[];
  events: readonly DomainEventRecord[];
  idempotencyKey: IdempotencyKey;
  outcome: "committed" | "rejected" | "blocked";
  projectRevisionAfter: DecimalUInt64 | null;
  projectRevisionBefore: DecimalUInt64;
  receipt: DomainReceipt | null;
  resultKind: "command";
  transactionId: Uuid;
  undoToken: UndoToken | null;
}>;

type QueryResultCommon = Readonly<{
  diagnostics: readonly Diagnostic[];
  queryId: Uuid;
  resultKind: "query";
}>;

type SuccessfulQueryResult =
  | Readonly<{
      outcome: "ok";
      queryKind: "system.health";
      receipt: HealthReceipt;
      snapshotHash: Sha256;
    }>
  | Readonly<{
      outcome: "ok";
      queryKind: "project.get-summary" | "project.get-object";
      receipt: ProjectSnapshotReceipt;
      snapshotHash: Sha256;
    }>
  | Readonly<{
      outcome: "ok";
      queryKind: "persistence.get-status";
      receipt: PersistenceReceipt;
      snapshotHash: Sha256;
    }>
  | Readonly<{
      outcome: "ok";
      queryKind: "hardware.get-configuration";
      receipt: HardwareSnapshotReceipt;
      snapshotHash: Sha256;
    }>
  | Readonly<{
      outcome: "ok";
      queryKind: "program.get-block";
      receipt: ProgramSnapshotReceipt;
      snapshotHash: Sha256;
    }>
  | Readonly<{
      outcome: "ok";
      queryKind: "build.get-report";
      receipt: BuildReceipt;
      snapshotHash: Sha256;
    }>
  | Readonly<{
      outcome: "ok";
      queryKind: "runtime.get-status";
      receipt: RuntimeReceipt;
      snapshotHash: Sha256;
    }>
  | Readonly<{
      outcome: "ok";
      queryKind: "commissioning.get-preview";
      receipt: CommissioningReceipt;
      snapshotHash: Sha256;
    }>
  | Readonly<{
      outcome: "ok";
      queryKind: "monitoring.get-snapshot";
      receipt: MonitoringReceipt;
      snapshotHash: Sha256;
    }>
  | Readonly<{
      outcome: "ok";
      queryKind: "diagnostics.list";
      receipt: DiagnosticsReceipt;
      snapshotHash: Sha256;
    }>;

type RejectedQueryResult = Readonly<{
  outcome: "rejected";
  queryKind: DomainQuery["queryKind"];
  receipt: null;
  snapshotHash: null;
}>;

export type QueryResultBody = QueryResultCommon &
  (RejectedQueryResult | SuccessfulQueryResult);

export type DomainResultBody = CommandResultBody | QueryResultBody;

export type DomainCommandMessage = Readonly<{
  command: DomainCommand;
  context: CommandContext;
  kind: typeof PLC_MESSAGE_KIND.command;
  requestId: Uuid;
  schemaVersion: typeof PLC_CONTRACT_SCHEMA_VERSION;
}>;

export type DomainQueryMessage = Readonly<{
  context: QueryContext;
  kind: typeof PLC_MESSAGE_KIND.query;
  query: DomainQuery;
  requestId: Uuid;
  schemaVersion: typeof PLC_CONTRACT_SCHEMA_VERSION;
}>;

export type DomainResultMessage = Readonly<{
  inReplyTo: Uuid;
  kind: typeof PLC_MESSAGE_KIND.result;
  result: DomainResultBody;
  schemaVersion: typeof PLC_CONTRACT_SCHEMA_VERSION;
}>;

export type DomainEventMessage = Readonly<{
  event: DomainEventRecord;
  kind: typeof PLC_MESSAGE_KIND.event;
  schemaVersion: typeof PLC_CONTRACT_SCHEMA_VERSION;
  subscriptionId: Uuid | null;
}>;

export type Phase2PlcMessage =
  | DomainCommandMessage
  | DomainEventMessage
  | DomainQueryMessage
  | DomainResultMessage;

export type PlcWireMessage =
  | FoundationHealthCompatibilityCommand
  | FoundationHealthCompatibilityResult
  | Phase2PlcMessage;
