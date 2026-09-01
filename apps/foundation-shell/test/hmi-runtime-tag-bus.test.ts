import { describe, expect, it } from "vitest";

import { createHmiRuntimeTagBus } from "../src/hmi-runtime-tag-bus";
import type {
  EngineeringRuntimeView,
  RuntimeCpuState,
  RuntimeProbeView,
} from "../src/runtime-types";

const INPUT_ID = "10000000-0000-4000-8000-000000000001";
const OUTPUT_ID = "10000000-0000-4000-8000-000000000002";
const MEMORY_ID = "10000000-0000-4000-8000-000000000003";
const NUMBER_ID = "10000000-0000-4000-8000-000000000004";

describe("HMI runtime tag bus", () => {
  it("reads authoritative BOOL tag values with their runtime quality", () => {
    const bus = createHmiRuntimeTagBus(runtime([
      booleanProbe(INPUT_ID, "input", false, "GOOD"),
      booleanProbe(OUTPUT_ID, "output", true, "FORCED"),
      booleanProbe(MEMORY_ID, "memory", false, "GOOD"),
    ]));

    expect(bus.readBoolean(INPUT_ID)).toEqual({
      displayName: "Start_PB",
      momentaryInput: true,
      probeKind: "input",
      quality: "GOOD",
      runtimeAddress: "%I0.0",
      tagId: INPUT_ID,
      truth: "off",
      unknownReason: null,
      value: false,
    });
    expect(bus.readBoolean(OUTPUT_ID)).toMatchObject({
      momentaryInput: false,
      probeKind: "output",
      quality: "FORCED",
      truth: "on",
      unknownReason: null,
      value: true,
    });
    expect(bus.readBoolean(MEMORY_ID)).toMatchObject({
      probeKind: "memory",
      truth: "off",
      value: false,
    });
  });

  it("keeps stale, bad, and unavailable values visibly unknown", () => {
    const staleMonitoring = createHmiRuntimeTagBus(runtime([
      booleanProbe(INPUT_ID, "input", true, "GOOD"),
    ], { monitorState: "STALE" })).readBoolean(INPUT_ID);
    expect(staleMonitoring).toMatchObject({
      quality: "GOOD",
      truth: "unknown",
      unknownReason: "monitoring-stale",
      value: true,
    });

    const badQuality = createHmiRuntimeTagBus(runtime([
      booleanProbe(INPUT_ID, "input", false, "BAD"),
    ])).readBoolean(INPUT_ID);
    expect(badQuality).toMatchObject({
      quality: "BAD",
      truth: "unknown",
      unknownReason: "quality-bad",
      value: false,
    });

    const unavailable = createHmiRuntimeTagBus({
      ...runtime([]),
      availability: "UNAVAILABLE",
      reason: "No virtual controller",
      session: null,
    }).readBoolean(INPUT_ID);
    expect(unavailable).toEqual({
      displayName: null,
      momentaryInput: false,
      probeKind: null,
      quality: "UNKNOWN",
      runtimeAddress: null,
      tagId: INPUT_ID,
      truth: "unknown",
      unknownReason: "runtime-unavailable",
      value: null,
    });
  });

  it("fails closed for missing, ambiguous, non-BOOL, and valueless probes", () => {
    const valueless = {
      ...booleanProbe(MEMORY_ID, "memory", false, "GOOD"),
      effectiveValue: null,
    } satisfies RuntimeProbeView;
    const bus = createHmiRuntimeTagBus(runtime([
      booleanProbe(INPUT_ID, "input", true, "GOOD"),
      booleanProbe(INPUT_ID, "input", false, "GOOD"),
      numericProbe(NUMBER_ID),
      valueless,
    ]));

    expect(bus.readBoolean("missing")).toMatchObject({
      truth: "unknown",
      unknownReason: "tag-unavailable",
    });
    expect(bus.readBoolean(INPUT_ID)).toMatchObject({
      truth: "unknown",
      unknownReason: "ambiguous-tag",
    });
    expect(bus.readBoolean(NUMBER_ID)).toMatchObject({
      quality: "GOOD",
      truth: "unknown",
      unknownReason: "tag-not-boolean",
    });
    expect(bus.readBoolean(MEMORY_ID)).toMatchObject({
      truth: "unknown",
      unknownReason: "effective-value-unavailable",
    });
  });

  it("creates separate press and release requests using only virtual input operations", () => {
    const bus = createHmiRuntimeTagBus(runtime([
      booleanProbe(INPUT_ID, "input", false, "GOOD"),
    ]));

    expect(bus.createMomentaryRequest(INPUT_ID, "press")).toEqual({
      ok: true,
      operations: [
        {
          kind: "runtime.set-raw-input",
          targetId: INPUT_ID,
          value: { type: "BOOL", value: true },
        },
        { kind: "runtime.run-scan" },
      ],
      phase: "press",
      tagId: INPUT_ID,
    });
    expect(bus.createMomentaryRequest(INPUT_ID, "release")).toEqual({
      ok: true,
      operations: [
        {
          kind: "runtime.set-raw-input",
          targetId: INPUT_ID,
          value: { type: "BOOL", value: false },
        },
        { kind: "runtime.run-scan" },
      ],
      phase: "release",
      tagId: INPUT_ID,
    });
    expect(JSON.stringify(bus.createMomentaryRequest(INPUT_ID, "press"))).not.toContain("modify-once");
    expect(JSON.stringify(bus.createMomentaryRequest(INPUT_ID, "press"))).not.toContain("force");
  });

  it("never creates a momentary request for outputs, memory, non-BOOL tags, or an unready PLC", () => {
    const probes = [
      booleanProbe(INPUT_ID, "input", false, "GOOD"),
      booleanProbe(OUTPUT_ID, "output", false, "GOOD"),
      booleanProbe(MEMORY_ID, "memory", false, "GOOD"),
      numericProbe(NUMBER_ID),
    ];
    const ready = createHmiRuntimeTagBus(runtime(probes));
    expect(ready.createMomentaryRequest(OUTPUT_ID, "press")).toMatchObject({
      code: "tag-not-input",
      ok: false,
    });
    expect(ready.createMomentaryRequest(MEMORY_ID, "press")).toMatchObject({
      code: "tag-not-input",
      ok: false,
    });
    expect(ready.createMomentaryRequest(NUMBER_ID, "press")).toMatchObject({
      code: "tag-not-boolean",
      ok: false,
    });

    const offline = createHmiRuntimeTagBus(runtime(probes, { online: false }));
    expect(offline.createMomentaryRequest(INPUT_ID, "press")).toMatchObject({
      code: "runtime-offline",
      ok: false,
    });
    const stopped = createHmiRuntimeTagBus(runtime(probes, { cpuState: "STOP" }));
    expect(stopped.createMomentaryRequest(INPUT_ID, "press")).toMatchObject({
      code: "controller-not-running",
      ok: false,
    });
  });
});

const runtime = (
  probes: readonly RuntimeProbeView[],
  overrides: Readonly<{
    cpuState?: RuntimeCpuState;
    monitorState?: "ACTIVE" | "DEGRADED" | "INACTIVE" | "STALE";
    online?: boolean;
  }> = {},
): EngineeringRuntimeView => ({
  availability: "READY",
  canBuild: true,
  diagnostics: [],
  reason: null,
  schemaVersion: 1,
  session: {
    buildCurrent: true,
    buildFingerprint: "build",
    controllerEpoch: "1",
    controllerObjectId: "20000000-0000-4000-8000-000000000001",
    cpuState: overrides.cpuState ?? "RUN",
    diagnosticReplayHash: "diagnostics",
    diagnostics: [],
    documentDirty: false,
    forceCount: 0,
    forceRegistryVersion: "0",
    forces: [],
    hardwareToLoaded: "MATCH",
    hashes: null,
    loadPreview: null,
    loaded: true,
    loadedArtifactFingerprint: "loaded",
    monitorState: overrides.monitorState ?? "ACTIVE",
    online: overrides.online ?? true,
    probes,
    runtimeControllerId: "20000000-0000-4000-8000-000000000002",
    runtimeReplayHash: "runtime-replay",
    scanSequence: "12",
    snapshotAvailable: false,
    softwareToLoaded: "MATCH",
    traces: [],
    universeEpoch: "1",
    universeId: "20000000-0000-4000-8000-000000000003",
    virtualTimeMilliseconds: "120",
    watches: [],
  },
  sourceDocumentHash: "document",
  sourceSemanticFingerprint: "semantic",
});

const booleanProbe = (
  id: string,
  kind: RuntimeProbeView["kind"],
  value: boolean,
  quality: RuntimeProbeView["quality"],
): RuntimeProbeView => ({
  committedOutputValue: kind === "output" ? { type: "BOOL", value } : null,
  deliveredOutputValue: kind === "output" ? { type: "BOOL", value } : null,
  displayName: id === INPUT_ID ? "Start_PB" : id === OUTPUT_ID ? "Motor_Run" : "Memory_Bit",
  effectiveValue: { type: "BOOL", value },
  forcedValue: quality === "FORCED" ? { type: "BOOL", value } : null,
  id,
  kind,
  naturalValue: { type: "BOOL", value },
  quality,
  rawInputValue: kind === "input" ? { type: "BOOL", value } : null,
  runtimeAddress: kind === "input" ? "%I0.0" : kind === "output" ? "%Q0.0" : "%M0.0",
  valueType: "BOOL",
});

const numericProbe = (id: string): RuntimeProbeView => ({
  committedOutputValue: null,
  deliveredOutputValue: null,
  displayName: "Batch_Count",
  effectiveValue: { type: "I32", value: "4" },
  forcedValue: null,
  id,
  kind: "memory",
  naturalValue: { type: "I32", value: "4" },
  quality: "GOOD",
  rawInputValue: null,
  runtimeAddress: "%MD4",
  valueType: "I32",
});
