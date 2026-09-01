import type {
  BehaviorTarget,
  BehaviorTestDefinitionV1,
  BehaviorTestResultV1,
  EducationScalarValue,
} from "./education-contract";
import type {
  EngineeringRuntimeView,
  RuntimeOperation,
  RuntimeProbeView,
  RuntimeValue,
} from "./runtime-types";

export type EducationRuntimeBridge = Readonly<{
  current: () => EngineeringRuntimeView;
  execute: (operation: RuntimeOperation) => Promise<EngineeringRuntimeView>;
  hmiControlTagIds?: Readonly<Record<string, string>>;
  reset: () => Promise<EngineeringRuntimeView>;
}>;

export type EducationRuntimeReadiness = Readonly<{
  ready: boolean;
  reason: string;
}>;

/**
 * Grades one behavior definition through the same runtime command boundary as
 * the trainer. No expected value is manufactured in the education layer.
 */
export const runBehaviorTestAgainstRuntime = async (
  definition: BehaviorTestDefinitionV1,
  bridge: EducationRuntimeBridge,
): Promise<BehaviorTestResultV1> => {
  const checks: BehaviorTestResultV1["checks"][number][] = [];
  let completedStepCount = 0;
  let runtime = bridge.current();

  for (const step of definition.steps) {
    try {
      switch (step.kind) {
        case "reset-runtime":
          runtime = await bridge.reset();
          assertRuntimeReady(runtime);
          break;
        case "set-value": {
          const probe = resolveTarget(runtime, step.target, bridge.hmiControlTagIds);
          if (probe.kind !== "input") {
            throw new Error("Behavior actions can write only virtual input probes.");
          }
          runtime = await bridge.execute({
            kind: "runtime.set-raw-input",
            targetId: probe.id,
            value: toRuntimeValue(step.value),
          });
          break;
        }
        case "run-scans":
          for (let scan = 0; scan < step.count; scan += 1) {
            runtime = await bridge.execute({ kind: "runtime.run-scan" });
          }
          break;
        case "expect-value": {
          const probe = resolveTarget(runtime, step.target, bridge.hmiControlTagIds);
          const actual = readProbe(probe);
          checks.push({
            actual,
            comparison: step.comparison,
            expected: step.expected,
            passed: educationValuesEqual(actual, step.expected),
            stepId: step.stepId,
            target: step.target,
          });
          break;
        }
      }
      completedStepCount += 1;
    } catch {
      return {
        checks,
        completedStepCount,
        errorCode: "driver-error",
        status: "error",
        testId: definition.testId,
      };
    }
  }

  return {
    checks,
    completedStepCount,
    errorCode: null,
    status: checks.every((check) => check.passed) ? "passed" : "failed",
    testId: definition.testId,
  };
};

export const inspectEducationRuntimeReadiness = (
  runtime: EngineeringRuntimeView,
): EducationRuntimeReadiness => {
  if (runtime.availability !== "READY") {
    return { ready: false, reason: runtime.reason ?? "The virtual PLC runtime is unavailable." };
  }
  const session = runtime.session;
  if (session === null || !session.loaded) {
    return { ready: false, reason: "Build and load the project before running behavior checks." };
  }
  if (!session.online) {
    return { ready: false, reason: "Go online with the virtual PLC before running behavior checks." };
  }
  if (session.cpuState !== "RUN") {
    return { ready: false, reason: "Put the virtual PLC in RUN before running behavior checks." };
  }
  if (session.monitorState !== "ACTIVE") {
    return { ready: false, reason: "Start live monitoring before running behavior checks." };
  }
  if (!session.snapshotAvailable) {
    return { ready: false, reason: "Start the simulation once to capture a clean reset point." };
  }
  return { ready: true, reason: "The virtual PLC is ready for behavior checks." };
};

const assertRuntimeReady = (runtime: EngineeringRuntimeView): void => {
  const readiness = inspectEducationRuntimeReadiness(runtime);
  if (!readiness.ready) throw new Error(readiness.reason);
};

const resolveTarget = (
  runtime: EngineeringRuntimeView,
  target: BehaviorTarget,
  hmiControlTagIds: Readonly<Record<string, string>> | undefined,
): RuntimeProbeView => {
  assertRuntimeReady(runtime);
  const probes = runtime.session?.probes ?? [];
  const matches = target.kind === "plc-tag"
    ? probes.filter((probe) => probe.displayName === target.name)
    : probes.filter((probe) => probe.id === hmiControlTagIds?.[target.name]);
  if (matches.length !== 1 || matches[0] === undefined) {
    throw new Error(`Behavior target ${target.name} is unavailable or ambiguous.`);
  }
  const probe = matches[0];
  if (probe.quality !== "GOOD") {
    throw new Error(`Behavior target ${target.name} does not have unforced good-quality data.`);
  }
  return probe;
};

const toRuntimeValue = (value: EducationScalarValue): RuntimeValue => {
  switch (value.type) {
    case "BOOL": return { type: "BOOL", value: value.value };
    case "DINT": return { type: "I32", value: String(value.value) };
    case "TIME_MS": return { type: "TIME_MS", value: String(value.value) };
  }
};

const readProbe = (probe: RuntimeProbeView): EducationScalarValue => {
  const value = probe.kind === "output"
    ? probe.deliveredOutputValue ?? probe.committedOutputValue ?? probe.effectiveValue
    : probe.effectiveValue;
  if (value === null) throw new Error("Runtime value is unavailable.");
  switch (value.type) {
    case "BOOL":
      if (typeof value.value !== "boolean") throw new Error("Malformed BOOL runtime value.");
      return { type: "BOOL", value: value.value };
    case "I32": {
      if (typeof value.value !== "string") throw new Error("Malformed DINT runtime value.");
      const parsed = Number(value.value);
      if (!Number.isSafeInteger(parsed) || parsed < -2_147_483_648 || parsed > 2_147_483_647) {
        throw new Error("DINT runtime value is outside the education contract.");
      }
      return { type: "DINT", value: parsed };
    }
    case "TIME_MS": {
      if (typeof value.value !== "string") throw new Error("Malformed TIME runtime value.");
      const parsed = Number(value.value);
      if (!Number.isSafeInteger(parsed) || parsed < 0 || parsed > 2_147_483_647) {
        throw new Error("TIME runtime value is outside the education contract.");
      }
      return { type: "TIME_MS", value: parsed };
    }
    case "I64":
    case "U32":
      throw new Error(`${value.type} is not part of the MVP behavior-test value set.`);
  }
};

const educationValuesEqual = (
  actual: EducationScalarValue,
  expected: EducationScalarValue,
): boolean => actual.type === expected.type && actual.value === expected.value;
