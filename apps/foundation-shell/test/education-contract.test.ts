import { describe, expect, it } from "vitest";

import {
  ASSIGNMENT_FILE_EXTENSION,
  BUILT_IN_MOTOR_STARTER_ASSIGNMENT,
  EDUCATION_CONTRACT_SCHEMA_VERSION,
  EducationContractValidationError,
  SUBMISSION_FILE_EXTENSION,
  inspectAssignmentDocument,
  inspectSubmissionAgainstAssignment,
  inspectSubmissionDocument,
  parseAssignmentDocument,
  runBehaviorTest,
} from "../src/education-contract";
import type {
  BehaviorTarget,
  BehaviorTestDefinitionV1,
  BehaviorTestDriver,
  EducationScalarValue,
  SubmissionDocumentV1,
} from "../src/education-contract";

describe("offline education contract", () => {
  it("publishes stable extensions and a valid built-in motor assignment", () => {
    expect(ASSIGNMENT_FILE_EXTENSION).toBe(".vlabassign");
    expect(SUBMISSION_FILE_EXTENSION).toBe(".vlabsubmit");
    expect(EDUCATION_CONTRACT_SCHEMA_VERSION).toBe(1);

    const result = inspectAssignmentDocument(BUILT_IN_MOTOR_STARTER_ASSIGNMENT);
    expect(result).toEqual({
      issues: [],
      ok: true,
      value: BUILT_IN_MOTOR_STARTER_ASSIGNMENT,
    });
    expect(BUILT_IN_MOTOR_STARTER_ASSIGNMENT.behaviorTests).toHaveLength(3);
    expect(BUILT_IN_MOTOR_STARTER_ASSIGNMENT.hintPolicy.hints.map((hint) => hint.order)).toEqual([1, 2, 3]);
  });

  it("runs ordered behavior steps with deterministic pass, fail, and driver-error results", () => {
    const definition: BehaviorTestDefinitionV1 = {
      description: "A written value survives a scan.",
      steps: [
        { kind: "reset-runtime", stepId: "reset" },
        {
          kind: "set-value",
          stepId: "write",
          target: { kind: "hmi-control", name: "Enable" },
          value: { type: "BOOL", value: true },
        },
        { count: 2, kind: "run-scans", stepId: "scan" },
        {
          comparison: "equals",
          expected: { type: "BOOL", value: true },
          kind: "expect-value",
          stepId: "check",
          target: { kind: "hmi-control", name: "Enable" },
        },
      ],
      testId: "ef2fc0ca-a5ab-49ba-8d88-8d6f8a8aa001",
      title: "Value remains set",
      visibility: "student",
    };
    const first = statefulDriver();
    const second = statefulDriver();
    const firstResult = runBehaviorTest(definition, first.driver);
    const secondResult = runBehaviorTest(definition, second.driver);

    expect(firstResult).toEqual(secondResult);
    expect(firstResult).toMatchObject({
      completedStepCount: 4,
      errorCode: null,
      status: "passed",
    });
    expect(first.scanCount()).toBe(2);

    const failed = runBehaviorTest({
      ...definition,
      steps: definition.steps.map((step) => step.kind === "expect-value"
        ? { ...step, expected: { type: "BOOL", value: false } as const }
        : step),
    }, statefulDriver().driver);
    expect(failed.status).toBe("failed");
    expect(failed.checks[0]?.passed).toBe(false);

    const errored = runBehaviorTest(definition, {
      ...statefulDriver().driver,
      runScans: () => {
        throw new Error("host-specific detail must not enter portable evidence");
      },
    });
    expect(errored).toMatchObject({
      completedStepCount: 2,
      errorCode: "driver-error",
      status: "error",
    });
    expect(JSON.stringify(errored)).not.toContain("host-specific");
  });

  it("proves the built-in motor behavior tests against a small scan driver", () => {
    const results = BUILT_IN_MOTOR_STARTER_ASSIGNMENT.behaviorTests.map((test) =>
      runBehaviorTest(test, motorStarterDriver())
    );

    expect(results.map((result) => result.status)).toEqual(["passed", "passed", "passed"]);
    expect(results.flatMap((result) => result.checks).every((check) => check.passed)).toBe(true);
  });

  it("validates a privacy-minimal submission and its assignment references", () => {
    const submission = validSubmission();
    const inspected = inspectSubmissionDocument(submission);
    expect(inspected).toEqual({ issues: [], ok: true, value: submission });
    expect(inspectSubmissionAgainstAssignment(submission, BUILT_IN_MOTOR_STARTER_ASSIGNMENT)).toEqual({
      issues: [],
      ok: true,
      value: submission,
    });

    expect(submission).not.toHaveProperty("studentName");
    expect(submission).not.toHaveProperty("email");
    expect(submission).not.toHaveProperty("userId");
  });

  it("rejects stale assignment references, unknown hints, and incomplete test evidence", () => {
    const valid = validSubmission();
    const stale: SubmissionDocumentV1 = {
      ...valid,
      assignmentRevision: valid.assignmentRevision + 1,
      evidence: {
        ...valid.evidence,
        behaviorResults: valid.evidence.behaviorResults.slice(1),
        hintUsage: [{
          hintId: "bc0c3d8c-3975-4c94-a4d0-106ba3b0f999",
          sequence: 1,
        }],
      },
    };

    const result = inspectSubmissionAgainstAssignment(stale, BUILT_IN_MOTOR_STARTER_ASSIGNMENT);
    expect(result.ok).toBe(false);
    if (result.ok) throw new Error("Expected assignment-reference validation to fail.");
    expect(result.issues.map((issue) => issue.path)).toEqual(expect.arrayContaining([
      "$.assignmentRevision",
      "$.evidence.behaviorResults",
      "$.evidence.hintUsage[0].hintId",
    ]));
  });

  it("returns actionable paths for malformed documents and throws at parse boundaries", () => {
    const malformed = {
      ...structuredClone(BUILT_IN_MOTOR_STARTER_ASSIGNMENT),
      schemaVersion: 2,
      title: " ",
      unexpectedCapability: "network",
    };
    const result = inspectAssignmentDocument(malformed);
    expect(result.ok).toBe(false);
    if (result.ok) throw new Error("Expected malformed assignment validation to fail.");
    expect(result.issues).toEqual(expect.arrayContaining([
      expect.objectContaining({ code: "invalid-value", path: "$.schemaVersion" }),
      expect.objectContaining({ code: "invalid-format", path: "$.title" }),
      expect.objectContaining({ code: "unexpected-field", path: "$.unexpectedCapability" }),
    ]));
    expect(() => parseAssignmentDocument(malformed)).toThrow(EducationContractValidationError);

    const invalidArtifact = {
      ...validSubmission(),
      project: {
        fileName: "../student.vlabproj",
        packageBase64: "not base64",
        sha256Hex: "abc",
      },
    };
    const submissionResult = inspectSubmissionDocument(invalidArtifact);
    expect(submissionResult.ok).toBe(false);
    if (submissionResult.ok) throw new Error("Expected project artifact validation to fail.");
    expect(submissionResult.issues.map((issue) => issue.path)).toEqual(expect.arrayContaining([
      "$.project.fileName",
      "$.project.packageBase64",
      "$.project.sha256Hex",
    ]));
  });
});

const statefulDriver = (): Readonly<{
  driver: BehaviorTestDriver;
  scanCount: () => number;
}> => {
  const values = new Map<string, EducationScalarValue>();
  let scans = 0;
  return {
    driver: {
      readValue: (target) => values.get(targetKey(target)) ?? null,
      resetRuntime: () => {
        values.clear();
        scans = 0;
      },
      runScans: (count) => {
        scans += count;
      },
      setValue: (target, value) => {
        values.set(targetKey(target), value);
      },
    },
    scanCount: () => scans,
  };
};

const targetKey = (target: BehaviorTarget): string => `${target.kind}:${target.name}`;

const motorStarterDriver = (): BehaviorTestDriver => {
  let start = false;
  let stop = false;
  let motor = false;
  return {
    readValue: (target) => target.name === "Motor_Run" ? { type: "BOOL", value: motor } : null,
    resetRuntime: () => {
      start = false;
      stop = false;
      motor = false;
    },
    runScans: (count) => {
      for (let scan = 0; scan < count; scan += 1) {
        motor = !stop && (start || motor);
      }
    },
    setValue: (target, value) => {
      if (value.type !== "BOOL") throw new Error("The motor fixture accepts BOOL inputs only.");
      if (target.name === "Start_PB") start = value.value;
      else if (target.name === "Stop_PB") stop = value.value;
      else throw new Error("The motor fixture received an unknown input.");
    },
  };
};

const validSubmission = (): SubmissionDocumentV1 => ({
  assignmentId: BUILT_IN_MOTOR_STARTER_ASSIGNMENT.assignmentId,
  assignmentRevision: BUILT_IN_MOTOR_STARTER_ASSIGNMENT.revision,
  attemptOrdinal: 1,
  documentKind: "vlab-submission",
  evidence: {
    behaviorResults: BUILT_IN_MOTOR_STARTER_ASSIGNMENT.behaviorTests.map((test) =>
      runBehaviorTest(test, motorStarterDriver())
    ),
    compile: {
      attemptCount: 3,
      blockingDiagnosticCodes: [],
      finalStatus: "current",
      warningDiagnosticCodes: [],
    },
    hintUsage: [{
      hintId: BUILT_IN_MOTOR_STARTER_ASSIGNMENT.hintPolicy.hints[0]?.hintId
        ?? "d47fa57d-fd3f-44a4-ac73-bd013db1a101",
      sequence: 1,
    }],
  },
  lifecycle: "submitted",
  project: {
    fileName: "Motor Starter.vlabproj",
    packageBase64: "e30=",
    sha256Hex: "A".repeat(64),
  },
  review: null,
  schemaVersion: 1,
  submissionId: "b5154d33-c55b-4ab7-8dbf-ef5d722bb001",
});
