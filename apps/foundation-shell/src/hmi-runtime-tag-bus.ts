import type {
  EngineeringRuntimeView,
  RuntimeOperation,
  RuntimeProbeKind,
  RuntimeProbeView,
  RuntimeSessionView,
} from "./runtime-types";

export type HmiBooleanTruth = "off" | "on" | "unknown";

export type HmiBooleanUnknownReason =
  | "ambiguous-tag"
  | "effective-value-unavailable"
  | "monitoring-degraded"
  | "monitoring-inactive"
  | "monitoring-stale"
  | "quality-bad"
  | "quality-stale"
  | "runtime-session-unavailable"
  | "runtime-unavailable"
  | "tag-not-boolean"
  | "tag-unavailable";

export type HmiBooleanTagRead = Readonly<{
  displayName: string | null;
  momentaryInput: boolean;
  probeKind: RuntimeProbeKind | null;
  quality: RuntimeProbeView["quality"] | "UNKNOWN";
  runtimeAddress: string | null;
  tagId: string;
  truth: HmiBooleanTruth;
  unknownReason: HmiBooleanUnknownReason | null;
  value: boolean | null;
}>;

export type HmiMomentaryPhase = "press" | "release";

export type HmiMomentaryRequestError =
  | "ambiguous-tag"
  | "controller-not-running"
  | "runtime-offline"
  | "runtime-session-unavailable"
  | "runtime-unavailable"
  | "tag-not-boolean"
  | "tag-not-input"
  | "tag-unavailable";

export type HmiMomentaryRequestResult =
  | Readonly<{
      ok: true;
      operations: readonly RuntimeOperation[];
      phase: HmiMomentaryPhase;
      tagId: string;
    }>
  | Readonly<{
      code: HmiMomentaryRequestError;
      message: string;
      ok: false;
      phase: HmiMomentaryPhase;
      tagId: string;
    }>;

export type HmiRuntimeTagBus = Readonly<{
  createMomentaryRequest: (
    tagId: string,
    phase: HmiMomentaryPhase,
  ) => HmiMomentaryRequestResult;
  readBoolean: (tagId: string) => HmiBooleanTagRead;
}>;

type RuntimeTagContext = Readonly<{
  availability: EngineeringRuntimeView["availability"];
  probesById: ReadonlyMap<string, readonly RuntimeProbeView[]>;
  session: RuntimeSessionView | null;
}>;

/**
 * Adapts one immutable runtime snapshot for an educational HMI. Reads always
 * come from authoritative runtime probes, and momentary buttons can only
 * request raw virtual-input changes followed by a controller scan.
 */
export const createHmiRuntimeTagBus = (
  runtime: EngineeringRuntimeView,
): HmiRuntimeTagBus => {
  const context = createRuntimeTagContext(runtime);
  return {
    createMomentaryRequest: (tagId, phase) => createMomentaryRequest(context, tagId, phase),
    readBoolean: (tagId) => readBoolean(context, tagId),
  };
};

const readBoolean = (
  context: RuntimeTagContext,
  tagId: string,
): HmiBooleanTagRead => {
  const unavailable = runtimeUnavailableRead(context, tagId);
  if (unavailable !== null) {
    return unavailable;
  }

  const probes = context.probesById.get(tagId) ?? [];
  if (probes.length === 0) {
    return unknownRead(tagId, "tag-unavailable");
  }
  if (probes.length !== 1) {
    return unknownRead(tagId, "ambiguous-tag");
  }
  const probe = probes[0];
  if (probe === undefined) {
    return unknownRead(tagId, "tag-unavailable");
  }
  const details = probeDetails(probe);
  if (probe.valueType !== "BOOL") {
    return unknownRead(tagId, "tag-not-boolean", details);
  }

  const value = booleanValue(probe.effectiveValue);
  if (value === null) {
    return unknownRead(tagId, "effective-value-unavailable", details);
  }

  const reason = unavailableTruthReason(context.session, probe.quality);
  return {
    ...details,
    tagId,
    truth: reason === null ? (value ? "on" : "off") : "unknown",
    unknownReason: reason,
    value,
  };
};

const createMomentaryRequest = (
  context: RuntimeTagContext,
  tagId: string,
  phase: HmiMomentaryPhase,
): HmiMomentaryRequestResult => {
  if (context.availability !== "READY") {
    return momentaryFailure(tagId, phase, "runtime-unavailable", "The virtual PLC runtime is unavailable.");
  }
  const session = context.session;
  if (session === null) {
    return momentaryFailure(
      tagId,
      phase,
      "runtime-session-unavailable",
      "Build and load the virtual PLC before using an HMI button.",
    );
  }

  const probes = context.probesById.get(tagId) ?? [];
  if (probes.length === 0) {
    return momentaryFailure(tagId, phase, "tag-unavailable", "The HMI tag is not available in the loaded PLC.");
  }
  if (probes.length !== 1) {
    return momentaryFailure(tagId, phase, "ambiguous-tag", "The HMI tag matches more than one runtime probe.");
  }
  const probe = probes[0];
  if (probe === undefined) {
    return momentaryFailure(tagId, phase, "tag-unavailable", "The HMI tag is not available in the loaded PLC.");
  }
  if (probe.valueType !== "BOOL") {
    return momentaryFailure(tagId, phase, "tag-not-boolean", "A momentary HMI button requires a BOOL tag.");
  }
  if (probe.kind !== "input") {
    return momentaryFailure(
      tagId,
      phase,
      "tag-not-input",
      "A momentary HMI button can only operate a virtual PLC input tag.",
    );
  }
  if (!session.online) {
    return momentaryFailure(tagId, phase, "runtime-offline", "Go online with the virtual PLC before using the HMI.");
  }
  if (session.cpuState !== "RUN") {
    return momentaryFailure(
      tagId,
      phase,
      "controller-not-running",
      "Put the virtual PLC in RUN before using the HMI.",
    );
  }

  return {
    ok: true,
    operations: [
      {
        kind: "runtime.set-raw-input",
        targetId: tagId,
        value: { type: "BOOL", value: phase === "press" },
      },
      { kind: "runtime.run-scan" },
    ],
    phase,
    tagId,
  };
};

const createRuntimeTagContext = (
  runtime: EngineeringRuntimeView,
): RuntimeTagContext => {
  const probesById = new Map<string, RuntimeProbeView[]>();
  for (const probe of runtime.session?.probes ?? []) {
    const existing = probesById.get(probe.id);
    if (existing === undefined) {
      probesById.set(probe.id, [probe]);
    } else {
      existing.push(probe);
    }
  }
  return {
    availability: runtime.availability,
    probesById,
    session: runtime.session,
  };
};

const runtimeUnavailableRead = (
  context: RuntimeTagContext,
  tagId: string,
): HmiBooleanTagRead | null => {
  if (context.availability !== "READY") {
    return unknownRead(tagId, "runtime-unavailable");
  }
  if (context.session === null) {
    return unknownRead(tagId, "runtime-session-unavailable");
  }
  return null;
};

const unavailableTruthReason = (
  session: RuntimeSessionView | null,
  quality: RuntimeProbeView["quality"],
): HmiBooleanUnknownReason | null => {
  switch (session?.monitorState) {
    case "ACTIVE": break;
    case "DEGRADED": return "monitoring-degraded";
    case "INACTIVE": return "monitoring-inactive";
    case "STALE": return "monitoring-stale";
    case undefined: return "runtime-session-unavailable";
  }
  switch (quality) {
    case "BAD": return "quality-bad";
    case "STALE": return "quality-stale";
    case "FORCED":
    case "GOOD": return null;
  }
};

const probeDetails = (
  probe: RuntimeProbeView,
): Pick<
  HmiBooleanTagRead,
  "displayName" | "momentaryInput" | "probeKind" | "quality" | "runtimeAddress"
> => ({
  displayName: probe.displayName,
  momentaryInput: probe.kind === "input" && probe.valueType === "BOOL",
  probeKind: probe.kind,
  quality: probe.quality,
  runtimeAddress: probe.runtimeAddress,
});

const unknownRead = (
  tagId: string,
  unknownReason: HmiBooleanUnknownReason,
  details: Partial<Pick<
    HmiBooleanTagRead,
    "displayName" | "momentaryInput" | "probeKind" | "quality" | "runtimeAddress"
  >> = {},
): HmiBooleanTagRead => ({
  displayName: details.displayName ?? null,
  momentaryInput: details.momentaryInput ?? false,
  probeKind: details.probeKind ?? null,
  quality: details.quality ?? "UNKNOWN",
  runtimeAddress: details.runtimeAddress ?? null,
  tagId,
  truth: "unknown",
  unknownReason,
  value: null,
});

const booleanValue = (
  value: RuntimeProbeView["effectiveValue"],
): boolean | null => value?.type === "BOOL" && typeof value.value === "boolean"
  ? value.value
  : null;

const momentaryFailure = (
  tagId: string,
  phase: HmiMomentaryPhase,
  code: HmiMomentaryRequestError,
  message: string,
): HmiMomentaryRequestResult => ({ code, message, ok: false, phase, tagId });
