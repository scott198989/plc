export type RuntimeCpuState =
  | "POWERED_OFF"
  | "STARTUP"
  | "STOP"
  | "RUN"
  | "PAUSED_EDUCATIONAL"
  | "FAULTED";

export type RuntimeValueType = "BOOL" | "I32" | "I64" | "U32" | "TIME_MS";

export type RuntimeValue = Readonly<{
  type: RuntimeValueType;
  value: boolean | string;
}>;

export type RuntimeProbeKind = "memory" | "input" | "output";

export type RuntimeProbeView = Readonly<{
  committedOutputValue: RuntimeValue | null;
  deliveredOutputValue: RuntimeValue | null;
  displayName: string;
  effectiveValue: RuntimeValue | null;
  forcedValue: RuntimeValue | null;
  id: string;
  kind: RuntimeProbeKind;
  naturalValue: RuntimeValue | null;
  quality: "GOOD" | "STALE" | "BAD" | "FORCED";
  rawInputValue: RuntimeValue | null;
  runtimeAddress: string;
  valueType: RuntimeValueType;
}>;

export type RuntimeForceView = Readonly<{
  forceId: string;
  reason: string;
  targetId: string;
  value: RuntimeValue;
}>;

export type RuntimeWatchRowView = Readonly<{
  displayBase: string;
  latestValue: RuntimeValue | null;
  quality: string | null;
  rowId: string;
  targetId: string;
}>;

export type RuntimeWatchTableView = Readonly<{
  id: string;
  name: string;
  rows: readonly RuntimeWatchRowView[];
}>;

export type RuntimeTraceView = Readonly<{
  captureCount: number;
  id: string;
  name: string;
  state: "IDLE" | "ARMED" | "CAPTURING" | "COMPLETE" | "ABORTED";
}>;

export type RuntimeDiagnosticView = Readonly<{
  active: boolean;
  code: string;
  message: string;
  navigationObjectId: string | null;
  occurrenceId: string;
  severity: "INFO" | "WARNING" | "ERROR" | "FATAL";
}>;

export type RuntimeHashView = Readonly<{
  controllerState: string;
  diagnosticReplay: string;
  runtimeReplay: string;
  universeState: string;
}>;

export type VirtualLoadPreviewView = Readonly<{
  blockerCount: number;
  candidateFingerprint: string;
  compatibility: string;
  initializationCount: number;
  previewFingerprint: string;
  previewId: string;
  removalCount: number;
  requiresStop: boolean;
  warningCount: number;
}>;

export type RuntimeSessionView = Readonly<{
  buildCurrent: boolean;
  buildFingerprint: string | null;
  controllerEpoch: string;
  controllerObjectId: string;
  cpuState: RuntimeCpuState;
  diagnosticReplayHash: string;
  diagnostics: readonly RuntimeDiagnosticView[];
  documentDirty: boolean;
  forceCount: number;
  forceRegistryVersion: string;
  forces: readonly RuntimeForceView[];
  hashes: RuntimeHashView | null;
  hardwareToLoaded: string | null;
  loadPreview: VirtualLoadPreviewView | null;
  loaded: boolean;
  loadedArtifactFingerprint: string | null;
  monitorState: "INACTIVE" | "ACTIVE" | "DEGRADED" | "STALE";
  online: boolean;
  probes: readonly RuntimeProbeView[];
  runtimeControllerId: string;
  runtimeReplayHash: string;
  scanSequence: string;
  snapshotAvailable: boolean;
  softwareToLoaded: string | null;
  traces: readonly RuntimeTraceView[];
  universeEpoch: string;
  universeId: string;
  virtualTimeMilliseconds: string;
  watches: readonly RuntimeWatchTableView[];
}>;

export type EngineeringRuntimeView = Readonly<{
  availability: "READY" | "UNAVAILABLE";
  canBuild: boolean;
  diagnostics: readonly Readonly<{
    blocking: boolean;
    code: string;
    message: string;
    objectId: string | null;
  }>[];
  reason: string | null;
  schemaVersion: 1;
  session: RuntimeSessionView | null;
  sourceDocumentHash: string;
  sourceSemanticFingerprint: string;
}>;

export type RuntimeOperation =
  | Readonly<{ kind: "runtime.build" }>
  | Readonly<{ kind: "runtime.power-on" }>
  | Readonly<{ kind: "runtime.power-off" }>
  | Readonly<{ kind: "runtime.preview-load"; postLoadMode: "STOP" | "RUN" }>
  | Readonly<{ kind: "runtime.commit-load" }>
  | Readonly<{ kind: "runtime.go-online" }>
  | Readonly<{ kind: "runtime.request-run" }>
  | Readonly<{ kind: "runtime.request-stop" }>
  | Readonly<{ kind: "runtime.run-scan" }>
  | Readonly<{ kind: "runtime.start-monitoring" }>
  | Readonly<{ kind: "runtime.set-raw-input"; targetId: string; value: RuntimeValue }>
  | Readonly<{ kind: "runtime.modify-once"; targetId: string; value: RuntimeValue }>
  | Readonly<{
      forceId: string;
      kind: "runtime.create-force";
      reason: string;
      targetId: string;
      value: RuntimeValue;
    }>
  | Readonly<{ forceId: string; kind: "runtime.remove-force"; reason: string }>
  | Readonly<{ kind: "runtime.arm-trace"; traceId: string }>
  | Readonly<{ kind: "runtime.capture-snapshot" }>
  | Readonly<{ kind: "runtime.restore-snapshot" }>;

