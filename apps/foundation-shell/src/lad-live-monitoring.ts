import type {
  RuntimeCpuState,
  RuntimeProbeKind,
  RuntimeProbeView,
} from "./runtime-types";
import type {
  ProjectPayload,
  ProjectPayloadValue,
  WorkbenchObjectView,
  WorkbenchSnapshot,
} from "./workbench-types";

export type LadLiveTruth = "off" | "on" | "unknown";
type RuntimeQuality = RuntimeProbeView["quality"];

export type LadLiveUnknownReason =
  | "ambiguous-active-tag-bindings"
  | "ambiguous-runtime-probes"
  | "effective-value-unavailable"
  | "monitoring-degraded"
  | "monitoring-inactive"
  | "monitoring-stale"
  | "no-active-tag-binding"
  | "probe-not-boolean"
  | "quality-bad"
  | "quality-stale"
  | "runtime-probe-unavailable"
  | "runtime-session-unavailable"
  | "runtime-unavailable"
  | "tag-not-boolean";

export type LadBooleanMemberLiveState = Readonly<{
  forced: boolean;
  forcedValue: boolean | null;
  memberId: string;
  memberName: string;
  observedValue: boolean | null;
  probeId: string | null;
  probeKind: RuntimeProbeKind | null;
  quality: RuntimeQuality | "UNKNOWN";
  role: string;
  runtimeAddress: string | null;
  tagId: string | null;
  tagName: string | null;
  truth: LadLiveTruth;
  unknownReason: LadLiveUnknownReason | null;
}>;

export type LadLiveMonitoringProjection = Readonly<{
  cpuState: RuntimeCpuState | null;
  members: readonly LadBooleanMemberLiveState[];
  monitorState: "ACTIVE" | "DEGRADED" | "INACTIVE" | "STALE" | "UNAVAILABLE";
  online: boolean;
  programBlockId: string;
  scanSequence: string | null;
}>;

export type LadLiveMonitoringErrorCode =
  | "duplicate-interface-member"
  | "invalid-interface"
  | "not-lad-program"
  | "program-not-found";

export type LadLiveMonitoringResult =
  | Readonly<{
      code: LadLiveMonitoringErrorCode;
      message: string;
      ok: false;
    }>
  | Readonly<{
      ok: true;
      projection: LadLiveMonitoringProjection;
    }>;

type BooleanMember = Readonly<{
  id: string;
  name: string;
  role: string;
}>;

type BoundTag = Readonly<{
  dataType: ProjectPayloadValue | undefined;
  id: string;
  name: string;
}>;

type RuntimeContext = Readonly<{
  monitorState: LadLiveMonitoringProjection["monitorState"];
  probesById: ReadonlyMap<string, readonly RuntimeProbeView[]>;
  unavailableReason: LadLiveUnknownReason | null;
}>;

/**
 * Joins one selected LAD block to authoritative runtime values without
 * evaluating the ladder in the UI. The canonical contract makes a tag object
 * ID the runtime probe ID, while the tag's blockId/memberId fields identify the
 * interface member it publishes. Missing and ambiguous joins stay unknown.
 */
export const projectLadLiveMonitoring = (
  snapshot: Pick<WorkbenchSnapshot, "objects" | "runtime">,
  programBlockId: string,
): LadLiveMonitoringResult => {
  const program = snapshot.objects[programBlockId];
  if (program === undefined || program.lifecycle !== "active") {
    return failure("program-not-found", "The selected program block is unavailable.");
  }
  if (!isProgramBlock(program) || program.semanticPayload.language !== "LAD") {
    return failure("not-lad-program", "Live LAD monitoring requires an active LAD program block.");
  }

  const parsedMembers = readBooleanMembers(program.semanticPayload.interface);
  if (!parsedMembers.ok) {
    return parsedMembers;
  }

  const tagsByMember = activeTagsByMember(snapshot.objects, programBlockId);
  const runtime = runtimeContext(snapshot.runtime);
  return {
    ok: true,
    projection: {
      cpuState: snapshot.runtime.session?.cpuState ?? null,
      members: parsedMembers.members.map((member) => projectMember(
        member,
        tagsByMember.get(member.id) ?? [],
        runtime,
      )),
      monitorState: runtime.monitorState,
      online: snapshot.runtime.session?.online ?? false,
      programBlockId,
      scanSequence: snapshot.runtime.session?.scanSequence ?? null,
    },
  };
};

const projectMember = (
  member: BooleanMember,
  tags: readonly BoundTag[],
  runtime: RuntimeContext,
): LadBooleanMemberLiveState => {
  if (tags.length === 0) {
    return unknownMember(member, "no-active-tag-binding");
  }
  if (tags.length !== 1) {
    return unknownMember(member, "ambiguous-active-tag-bindings");
  }

  const tag = tags[0];
  if (tag === undefined) {
    return unknownMember(member, "no-active-tag-binding");
  }
  const binding = {
    tagId: tag.id,
    tagName: tag.name,
  };
  if (tag.dataType !== "BOOL") {
    return unknownMember(member, "tag-not-boolean", binding);
  }
  if (runtime.unavailableReason !== null) {
    return unknownMember(member, runtime.unavailableReason, binding);
  }

  const probes = runtime.probesById.get(tag.id) ?? [];
  if (probes.length === 0) {
    return unknownMember(member, "runtime-probe-unavailable", binding);
  }
  if (probes.length !== 1) {
    return unknownMember(member, "ambiguous-runtime-probes", binding);
  }
  const probe = probes[0];
  if (probe === undefined) {
    return unknownMember(member, "runtime-probe-unavailable", binding);
  }
  const probeBinding = {
    ...binding,
    probeId: probe.id,
    probeKind: probe.kind,
    quality: probe.quality,
    runtimeAddress: probe.runtimeAddress,
  };
  if (probe.valueType !== "BOOL") {
    return unknownMember(member, "probe-not-boolean", probeBinding);
  }

  const observedValue = booleanValue(probe.effectiveValue);
  const forcedValue = booleanValue(probe.forcedValue);
  const forced = probe.quality === "FORCED" || forcedValue !== null;
  if (observedValue === null) {
    return unknownMember(member, "effective-value-unavailable", {
      ...probeBinding,
      forced,
      forcedValue,
    });
  }

  const unavailable = liveAvailabilityReason(runtime.monitorState, probe.quality);
  return {
    forced,
    forcedValue,
    memberId: member.id,
    memberName: member.name,
    observedValue,
    probeId: probe.id,
    probeKind: probe.kind,
    quality: probe.quality,
    role: member.role,
    runtimeAddress: probe.runtimeAddress,
    tagId: tag.id,
    tagName: tag.name,
    truth: unavailable === null ? (observedValue ? "on" : "off") : "unknown",
    unknownReason: unavailable,
  };
};

const liveAvailabilityReason = (
  monitorState: LadLiveMonitoringProjection["monitorState"],
  quality: RuntimeQuality,
): LadLiveUnknownReason | null => {
  switch (monitorState) {
    case "INACTIVE": return "monitoring-inactive";
    case "DEGRADED": return "monitoring-degraded";
    case "STALE": return "monitoring-stale";
    case "UNAVAILABLE": return "runtime-session-unavailable";
    case "ACTIVE": break;
  }
  switch (quality) {
    case "BAD": return "quality-bad";
    case "STALE": return "quality-stale";
    case "FORCED":
    case "GOOD": return null;
  }
};

const unknownMember = (
  member: BooleanMember,
  reason: LadLiveUnknownReason,
  details: Partial<Pick<
    LadBooleanMemberLiveState,
    "forced" | "forcedValue" | "probeId" | "probeKind" | "quality" | "runtimeAddress" | "tagId" | "tagName"
  >> = {},
): LadBooleanMemberLiveState => ({
  forced: details.forced ?? false,
  forcedValue: details.forcedValue ?? null,
  memberId: member.id,
  memberName: member.name,
  observedValue: null,
  probeId: details.probeId ?? null,
  probeKind: details.probeKind ?? null,
  quality: details.quality ?? "UNKNOWN",
  role: member.role,
  runtimeAddress: details.runtimeAddress ?? null,
  tagId: details.tagId ?? null,
  tagName: details.tagName ?? null,
  truth: "unknown",
  unknownReason: reason,
});

const runtimeContext = (runtime: WorkbenchSnapshot["runtime"]): RuntimeContext => {
  if (runtime.availability !== "READY") {
    return {
      monitorState: "UNAVAILABLE",
      probesById: new Map(),
      unavailableReason: "runtime-unavailable",
    };
  }
  if (runtime.session === null) {
    return {
      monitorState: "UNAVAILABLE",
      probesById: new Map(),
      unavailableReason: "runtime-session-unavailable",
    };
  }
  const probesById = new Map<string, RuntimeProbeView[]>();
  for (const probe of runtime.session.probes) {
    const existing = probesById.get(probe.id);
    if (existing === undefined) {
      probesById.set(probe.id, [probe]);
    } else {
      existing.push(probe);
    }
  }
  return {
    monitorState: runtime.session.monitorState,
    probesById,
    unavailableReason: null,
  };
};

const activeTagsByMember = (
  objects: WorkbenchSnapshot["objects"],
  programBlockId: string,
): ReadonlyMap<string, readonly BoundTag[]> => {
  const tags = new Map<string, BoundTag[]>();
  for (const object of Object.values(objects)) {
    if (
      object.kind !== "Tag" ||
      object.lifecycle !== "active" ||
      object.semanticPayload.blockId !== programBlockId ||
      typeof object.semanticPayload.memberId !== "string"
    ) {
      continue;
    }
    const memberId = object.semanticPayload.memberId;
    const binding: BoundTag = {
      dataType: object.semanticPayload.dataType,
      id: object.id,
      name: object.displayName,
    };
    const existing = tags.get(memberId);
    if (existing === undefined) {
      tags.set(memberId, [binding]);
    } else {
      existing.push(binding);
    }
  }
  return tags;
};

const readBooleanMembers = (
  value: ProjectPayloadValue | undefined,
):
  | Readonly<{ code: LadLiveMonitoringErrorCode; message: string; ok: false }>
  | Readonly<{ members: readonly BooleanMember[]; ok: true }> => {
  if (!Array.isArray(value)) {
    return failure("invalid-interface", "The selected LAD block has no canonical interface list.");
  }
  const members: BooleanMember[] = [];
  const identities = new Set<string>();
  for (const item of value) {
    const fields = recordFields(item);
    if (fields === null) {
      return failure("invalid-interface", "The selected LAD block contains an invalid interface member.");
    }
    if (fields.type !== "BOOL") {
      continue;
    }
    if (
      typeof fields.id !== "string" ||
      typeof fields.name !== "string" ||
      typeof fields.role !== "string"
    ) {
      return failure("invalid-interface", "A BOOL interface member is missing its canonical identity or label.");
    }
    if (identities.has(fields.id)) {
      return failure("duplicate-interface-member", "The selected LAD block repeats a BOOL interface identity.");
    }
    identities.add(fields.id);
    members.push({ id: fields.id, name: fields.name, role: fields.role });
  }
  return { members, ok: true };
};

const recordFields = (value: ProjectPayloadValue): ProjectPayload | null => {
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

const booleanValue = (
  value: RuntimeProbeView["effectiveValue"],
): boolean | null => value?.type === "BOOL" && typeof value.value === "boolean"
  ? value.value
  : null;

const isProgramBlock = (object: WorkbenchObjectView): boolean =>
  object.kind === "OB" || object.kind === "FC" || object.kind === "FB";

const failure = <Code extends LadLiveMonitoringErrorCode>(
  code: Code,
  message: string,
): Readonly<{ code: Code; message: string; ok: false }> => ({ code, message, ok: false });
