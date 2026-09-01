import { describe, expect, it } from "vitest";

import { BUILT_IN_MOTOR_STARTER_ASSIGNMENT } from "../src/education-contract";
import {
  inspectEducationRuntimeReadiness,
  runBehaviorTestAgainstRuntime,
} from "../src/education-runtime-adapter";
import type { EducationRuntimeBridge } from "../src/education-runtime-adapter";
import type {
  EngineeringRuntimeView,
  RuntimeOperation,
  RuntimeProbeView,
} from "../src/runtime-types";

describe("authoritative education runtime adapter", () => {
  it("drives real virtual-input operations and grades observed motor output probes", async () => {
    const harness = motorRuntimeHarness();
    const results = [];
    for (const test of BUILT_IN_MOTOR_STARTER_ASSIGNMENT.behaviorTests) {
      results.push(await runBehaviorTestAgainstRuntime(test, harness.bridge));
    }

    expect(results.map((result) => result.status)).toEqual(["passed", "passed", "passed"]);
    expect(results.flatMap((result) => result.checks).every((check) => check.passed)).toBe(true);
    expect(harness.operations).toContainEqual({
      kind: "runtime.set-raw-input",
      targetId: "start-tag",
      value: { type: "BOOL", value: true },
    });
    expect(harness.operations).toContainEqual({ kind: "runtime.run-scan" });
    expect(harness.resetCount()).toBe(3);
  });

  it("fails closed for ambiguous or forced probes instead of manufacturing a grade", async () => {
    const ambiguous = motorRuntimeHarness({ duplicateStart: true });
    const ambiguousResult = await runBehaviorTestAgainstRuntime(
      BUILT_IN_MOTOR_STARTER_ASSIGNMENT.behaviorTests[0]!,
      ambiguous.bridge,
    );
    expect(ambiguousResult).toMatchObject({ errorCode: "driver-error", status: "error" });

    const forced = motorRuntimeHarness({ forcedMotor: true });
    const forcedResult = await runBehaviorTestAgainstRuntime(
      BUILT_IN_MOTOR_STARTER_ASSIGNMENT.behaviorTests[0]!,
      forced.bridge,
    );
    expect(forcedResult).toMatchObject({ errorCode: "driver-error", status: "error" });
    expect(forcedResult.checks).toEqual([]);
  });

  it("explains each runtime prerequisite used by the UI", () => {
    const ready = motorRuntimeHarness().bridge.current();
    expect(inspectEducationRuntimeReadiness(ready)).toEqual({
      ready: true,
      reason: "The virtual PLC is ready for behavior checks.",
    });
    expect(inspectEducationRuntimeReadiness({
      ...ready,
      session: ready.session === null ? null : { ...ready.session, snapshotAvailable: false },
    })).toEqual({
      ready: false,
      reason: "Start the simulation once to capture a clean reset point.",
    });
  });
});

const motorRuntimeHarness = (options: Readonly<{
  duplicateStart?: boolean;
  forcedMotor?: boolean;
}> = {}): Readonly<{
  bridge: EducationRuntimeBridge;
  operations: RuntimeOperation[];
  resetCount: () => number;
}> => {
  let start = false;
  let stop = false;
  let motor = false;
  let scans = 0;
  let resets = 0;
  const operations: RuntimeOperation[] = [];
  const current = (): EngineeringRuntimeView => runtime([
    booleanProbe("start-tag", "Start_PB", "input", start),
    ...(options.duplicateStart ? [booleanProbe("duplicate-start", "Start_PB", "input", start)] : []),
    booleanProbe("stop-tag", "Stop_PB", "input", stop),
    booleanProbe("motor-tag", "Motor_Run", "output", motor, options.forcedMotor ? "FORCED" : "GOOD"),
  ], scans);
  const execute = async (operation: RuntimeOperation): Promise<EngineeringRuntimeView> => {
    operations.push(operation);
    if (operation.kind === "runtime.set-raw-input") {
      if (operation.value.type !== "BOOL" || typeof operation.value.value !== "boolean") {
        throw new Error("Motor fixture expects BOOL input writes.");
      }
      if (operation.targetId === "start-tag") start = operation.value.value;
      else if (operation.targetId === "stop-tag") stop = operation.value.value;
      else throw new Error("Unknown input target.");
    }
    if (operation.kind === "runtime.run-scan") {
      motor = !stop && (start || motor);
      scans += 1;
    }
    return current();
  };
  return {
    bridge: {
      current,
      execute,
      reset: async () => {
        start = false;
        stop = false;
        motor = false;
        scans += 1;
        resets += 1;
        return current();
      },
    },
    operations,
    resetCount: () => resets,
  };
};

const runtime = (probes: readonly RuntimeProbeView[], scans: number): EngineeringRuntimeView => ({
  availability: "READY",
  canBuild: true,
  diagnostics: [],
  reason: null,
  schemaVersion: 1,
  session: {
    buildCurrent: true,
    buildFingerprint: "build",
    controllerEpoch: "1",
    controllerObjectId: "controller",
    cpuState: "RUN",
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
    loadedArtifactFingerprint: "build",
    monitorState: "ACTIVE",
    online: true,
    probes,
    runtimeControllerId: "runtime-controller",
    runtimeReplayHash: "runtime-replay",
    scanSequence: String(scans),
    snapshotAvailable: true,
    softwareToLoaded: "MATCH",
    traces: [],
    universeEpoch: "1",
    universeId: "universe",
    virtualTimeMilliseconds: String(scans * 10),
    watches: [],
  },
  sourceDocumentHash: "document",
  sourceSemanticFingerprint: "semantic",
});

const booleanProbe = (
  id: string,
  displayName: string,
  kind: "input" | "output",
  value: boolean,
  quality: RuntimeProbeView["quality"] = "GOOD",
): RuntimeProbeView => ({
  committedOutputValue: kind === "output" ? { type: "BOOL", value } : null,
  deliveredOutputValue: kind === "output" ? { type: "BOOL", value } : null,
  displayName,
  effectiveValue: { type: "BOOL", value },
  forcedValue: quality === "FORCED" ? { type: "BOOL", value } : null,
  id,
  kind,
  naturalValue: { type: "BOOL", value },
  quality,
  rawInputValue: kind === "input" ? { type: "BOOL", value } : null,
  runtimeAddress: kind === "input" ? "%I0.0" : "%Q0.0",
  valueType: "BOOL",
});
