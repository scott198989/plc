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
): ProjectPayloadValue => recordValue({
  id: crypto.randomUUID(),
  name,
  order: unsignedValue(order),
  requiredOutput: role === "output",
  retentive: false,
  role,
  type: "DINT",
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
          interfaceMember("InputValue", "temp", 0),
          interfaceMember("OutputValue", "temp", 1),
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
      members: [interfaceMember("MemoryValue", "static", 0)],
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
