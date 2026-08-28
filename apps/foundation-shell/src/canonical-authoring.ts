import type { ProjectPayload, ProjectPayloadValue } from "./workbench-types";

export type SclProgramTemplate = "cyclic-ob" | "fc" | "fb";

export const unsignedValue = (value: number): Readonly<{ $type: "u64"; value: string }> => ({
  $type: "u64",
  value: value.toString(10),
});

const recordValue = (
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
          interfaceMember("InputValue", "input", 0),
          interfaceMember("Result", "output", 1),
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
