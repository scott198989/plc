import { describe, expect, it } from "vitest";

import {
  createDataBlockPayload,
  createSclProgramPayload,
  createTracePayload,
} from "../src/canonical-authoring";

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;

describe("canonical authoring payloads", () => {
  it("creates stable, independent block-interface identities at commit time", () => {
    const first = createSclProgramPayload("cyclic-ob");
    const second = createSclProgramPayload("cyclic-ob");
    expect(first.blockKind).toBe("OB");
    expect(first.sourceText).toBe("");
    const firstMembers = first.interface;
    const secondMembers = second.interface;
    expect(Array.isArray(firstMembers)).toBe(true);
    expect(Array.isArray(secondMembers)).toBe(true);
    if (!Array.isArray(firstMembers) || !Array.isArray(secondMembers)) {
      throw new Error("Expected canonical interface lists.");
    }
    const firstIds = firstMembers.map(memberId);
    const secondIds = secondMembers.map(memberId);
    expect(firstIds).toHaveLength(2);
    expect(new Set(firstIds).size).toBe(2);
    expect(firstIds.every((id) => UUID.test(id))).toBe(true);
    expect(secondIds).not.toEqual(firstIds);
  });

  it("pins DB and trace limits into typed canonical values", () => {
    expect(createDataBlockPayload("GlobalDB")).toMatchObject({
      dbKind: "GlobalDB",
      engineeringNumber: { $type: "u64", value: "1" },
    });
    expect(createTracePayload()).toEqual({
      channels: [],
      everyScans: { $type: "u64", value: "1" },
      maximumDurationMs: { $type: "u64", value: "60000" },
      postSamples: { $type: "u64", value: "32" },
      preSamples: { $type: "u64", value: "0" },
      state: "idle",
      trigger: "immediate",
    });
  });
});

const memberId = (value: unknown): string => {
  if (
    typeof value !== "object" ||
    value === null ||
    !("value" in value) ||
    typeof value.value !== "object" ||
    value.value === null ||
    !("id" in value.value) ||
    typeof value.value.id !== "string"
  ) {
    throw new Error("Expected a canonical interface member record.");
  }
  return value.value.id;
};
