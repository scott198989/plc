import { describe, expect, it } from "vitest";

import {
  BUILT_IN_MOTOR_STARTER_ASSIGNMENT,
  EducationContractValidationError,
  inspectAssignmentDocument,
  inspectSubmissionAgainstAssignment,
} from "../src/education-contract";
import type { BehaviorTestDefinitionV1, BehaviorTestResultV1 } from "../src/education-contract";
import { recordValue, unsignedValue } from "../src/canonical-authoring";
import {
  acceptReviewedSubmissionForStudent,
  assignmentCompileAttemptDelta,
  appendHintUsage,
  cloneAssignmentAsDraft,
  createBehaviorStepDraft,
  createBehaviorTestDraft,
  createBlankAssignmentDraft,
  createEducationSubmission,
  createProgressiveHintDraft,
  createSubmissionReviewComment,
  inspectAssignmentAuthoring,
  inspectProjectRequirements,
  inspectSubmissionReadiness,
  projectNextProgressiveHint,
  publishAssignmentDraft,
  recordSubmissionReview,
  reviseAssignmentAsDraft,
} from "../src/education-workflow";
import type { ProjectPayload, WorkbenchObjectView } from "../src/workbench-types";

describe("education workflow helpers", () => {
  it("creates a strict, editable assignment with a real ordered behavior test", () => {
    const draft = createBlankAssignmentDraft(sequentialIds());

    expect(inspectAssignmentDocument(draft)).toMatchObject({ ok: true });
    expect(draft).toMatchObject({
      documentKind: "vlab-assignment",
      lifecycle: "draft",
      revision: 1,
      schemaVersion: 1,
    });
    expect(draft.behaviorTests[0]?.steps.map((step) => step.kind)).toEqual([
      "reset-runtime",
      "set-value",
      "run-scans",
      "expect-value",
    ]);
  });

  it("gives clones a new assignment graph while revisions preserve stable authoring IDs", () => {
    const source = BUILT_IN_MOTOR_STARTER_ASSIGNMENT;
    const clone = cloneAssignmentAsDraft(source, sequentialIds(100));

    expect(inspectAssignmentDocument(clone)).toMatchObject({ ok: true });
    expect(clone.assignmentId).not.toBe(source.assignmentId);
    expect(clone.behaviorTests.map((test) => test.testId)).not.toEqual(source.behaviorTests.map((test) => test.testId));
    expect(clone.hintPolicy.hints.map((hint) => hint.hintId)).not.toEqual(source.hintPolicy.hints.map((hint) => hint.hintId));
    expect(clone.behaviorTests.map((test) => test.steps.map((step) => step.stepId)))
      .toEqual(source.behaviorTests.map((test) => test.steps.map((step) => step.stepId)));
    const linkedHint = clone.hintPolicy.hints.find((hint) => hint.unlock.kind === "after-behavior-failures");
    const linkedTestId = linkedHint?.unlock.kind === "after-behavior-failures" ? linkedHint.unlock.testId : null;
    expect(linkedTestId === null || clone.behaviorTests.some((test) => test.testId === linkedTestId)).toBe(true);

    const revised = reviseAssignmentAsDraft(source);
    expect(revised.assignmentId).toBe(source.assignmentId);
    expect(revised.revision).toBe(source.revision + 1);
    expect(revised.behaviorTests).toBe(source.behaviorTests);
    expect(revised.hintPolicy).toBe(source.hintPolicy);
  });

  it("publishes only strict drafts and preserves their managed IDs", () => {
    const draft = createBlankAssignmentDraft(sequentialIds());
    const published = publishAssignmentDraft(draft);
    expect(published.lifecycle).toBe("published");
    expect(published.assignmentId).toBe(draft.assignmentId);
    expect(published.behaviorTests[0]?.testId).toBe(draft.behaviorTests[0]?.testId);
    expect(() => publishAssignmentDraft({ ...draft, title: "" }))
      .toThrow(EducationContractValidationError);
  });

  it("validates behavior targets semantically before a teacher can publish", () => {
    const draft = createBlankAssignmentDraft(sequentialIds());
    const test = draft.behaviorTests[0]!;
    const missingTarget = {
      ...draft,
      behaviorTests: [{
        ...test,
        steps: test.steps.map((step) => step.kind === "set-value"
          ? { ...step, target: { kind: "plc-tag" as const, name: "Missing_Tag" } }
          : step),
      }],
    };
    const missing = inspectAssignmentAuthoring(missingTarget);
    expect(missing.ok).toBe(false);
    expect(missing.issues).toEqual(expect.arrayContaining([
      expect.objectContaining({ code: "invalid-reference", message: expect.stringContaining("Missing_Tag") }),
    ]));

    const wrongType = {
      ...draft,
      behaviorTests: [{
        ...test,
        steps: test.steps.map((step) => step.kind === "set-value"
          ? { ...step, value: { type: "DINT" as const, value: 1 } }
          : step),
      }],
    };
    expect(inspectAssignmentAuthoring(wrongType).issues).toEqual(expect.arrayContaining([
      expect.objectContaining({ code: "invalid-type", message: expect.stringContaining("DINT") }),
    ]));

    const hmiTarget = {
      ...draft,
      behaviorTests: [{
        ...test,
        steps: test.steps.map((step) => step.kind === "set-value"
          ? { ...step, target: { kind: "hmi-control" as const, name: "Start button" } }
          : step),
      }],
    };
    expect(() => publishAssignmentDraft(hmiTarget)).toThrow(EducationContractValidationError);
    expect(inspectAssignmentAuthoring({
      ...hmiTarget,
      starterProject: {
        artifact: {
          fileName: "Starter.vlabproj",
          packageBase64: "e30=",
          sha256Hex: "A".repeat(64),
        },
        kind: "embedded-project",
      },
    }).ok).toBe(true);
  });

  it("scopes compile attempts to assignment activation", () => {
    expect(assignmentCompileAttemptDelta(8, 5)).toBe(3);
    expect(assignmentCompileAttemptDelta(4, 5)).toBe(0);
  });

  it("checks canonical hardware, tags, addresses, and LAD instructions", () => {
    const assignment = BUILT_IN_MOTOR_STARTER_ASSIGNMENT;
    const snapshot = { objects: requirementObjects() };
    expect(inspectProjectRequirements(assignment, snapshot)).toEqual({ issues: [], ready: true });

    const changed = {
      ...snapshot.objects,
      start: object("start", "Tag", "Start_PB", {
        addressArea: "I",
        addressIntent: "explicit",
        bitOffset: unsignedValue(7),
        byteOffset: unsignedValue(0),
        dataType: "BOOL",
      }),
      program: object("program", "OB", "MainCycle", ladPayload([
        { mode: "normally-open", nodeKind: "contact" },
        { instructionCode: unsignedValue(0x0110), nodeKind: "box" },
        { mode: "normal", nodeKind: "coil" },
      ])),
    };
    const inspected = inspectProjectRequirements(assignment, { objects: changed });
    expect(inspected.ready).toBe(false);
    expect(inspected.issues).toEqual(expect.arrayContaining([
      expect.objectContaining({ code: "tag-address-mismatch" }),
      expect.objectContaining({ code: "prohibited-instruction", message: expect.stringContaining("ton") }),
    ]));

    const unresolvedAndMultiple = inspectProjectRequirements(assignment, {
      objects: {
        ...snapshot.objects,
        controller2: object("controller2", "Controller", "Second controller", { catalogId: "vctrl-m1" }),
        start: object("start", "Tag", "Start_PB", {
          addressArea: "I",
          addressIntent: "auto",
          dataType: "BOOL",
        }),
      },
    });
    expect(unresolvedAndMultiple.issues).toEqual(expect.arrayContaining([
      expect.objectContaining({ code: "multiple-controllers" }),
      expect.objectContaining({
        code: "tag-address-mismatch",
        message: expect.stringContaining("automatic or unresolved"),
      }),
    ]));
  });

  it("builds exact step unions and appends hints after imported non-contiguous orders", () => {
    const ids = sequentialIds();
    expect(createBehaviorTestDraft(ids).steps).toHaveLength(4);
    expect(createBehaviorStepDraft("reset-runtime", ids)).toEqual({ kind: "reset-runtime", stepId: expect.any(String) });
    expect(createBehaviorStepDraft("run-scans", ids)).toMatchObject({ count: 1, kind: "run-scans" });
    expect(createBehaviorStepDraft("set-value", ids)).toMatchObject({ kind: "set-value", value: { type: "BOOL", value: true } });
    expect(createBehaviorStepDraft("expect-value", ids)).toMatchObject({ comparison: "equals", expected: { type: "BOOL", value: true } });

    const previous = [{
      body: "First clue",
      hintId: "00000000-0000-4000-8000-000000009990",
      order: 20,
      title: "Clue",
      unlock: { kind: "immediate" as const },
    }];
    expect(createProgressiveHintDraft(previous, ids)).toMatchObject({
      order: 21,
      unlock: { hintId: previous[0]?.hintId, kind: "after-previous-hint" },
    });
  });

  it("reveals progressive hints one at a time from real attempt evidence", () => {
    const assignment = BUILT_IN_MOTOR_STARTER_ASSIGNMENT;
    expect(projectNextProgressiveHint(assignment, [], 0, [])).toMatchObject({
      hint: null,
      reason: expect.stringContaining("failed attempt"),
    });

    const failedStop = result(assignment.behaviorTests[2]!.testId, "failed");
    const first = projectNextProgressiveHint(assignment, [], 0, [failedStop]);
    expect(first.hint?.title).toBe("Think about the Stop contact");
    const usage = appendHintUsage([], first.hint!);
    expect(usage).toEqual([{ hintId: first.hint?.hintId, sequence: 1 }]);

    const second = projectNextProgressiveHint(assignment, usage, 0, [failedStop]);
    expect(second.hint?.title).toBe("Look for a holding path");
    expect(appendHintUsage(usage, first.hint!)).toBe(usage);
  });

  it("requires a current build, current behavior evidence, and project packaging", () => {
    const assignment = BUILT_IN_MOTOR_STARTER_ASSIGNMENT;
    const passed = assignment.behaviorTests.map((test) => result(test.testId, "passed"));
    expect(inspectSubmissionReadiness(
      assignment,
      { buildState: "current", projectHash: "PROJECT-A" },
      passed,
      "PROJECT-A",
      true,
    )).toEqual({ ready: true, reasons: [] });

    const blocked = inspectSubmissionReadiness(
      assignment,
      { buildState: "stale", projectHash: "PROJECT-B" },
      passed.slice(1),
      "PROJECT-A",
      false,
    );
    expect(blocked.ready).toBe(false);
    expect(blocked.reasons).toHaveLength(4);
  });

  it("builds a valid privacy-minimal submission and records a local review", () => {
    const assignment = BUILT_IN_MOTOR_STARTER_ASSIGNMENT;
    const submission = createEducationSubmission({
      assignment,
      attemptOrdinal: 2,
      behaviorResults: assignment.behaviorTests.map(passingResult),
      compileAttemptCount: 4,
      hintUsage: [],
      idFactory: () => "9e14f0d1-f09a-40b7-884d-c7e53e7a1001",
      project: {
        fileName: "Motor Starter.vlabproj",
        packageBase64: "e30=",
        sha256Hex: "A".repeat(64),
      },
      snapshot: {
        buildState: "current",
        diagnostics: [{
          blocking: false,
          code: "LAD.LEARNING.NOTE",
          diagnosticId: "diagnostic",
          message: "A learning note.",
          objectId: null,
          phase: "Compile",
          severity: "Warning",
        }],
      },
    });

    expect(submission.evidence.compile).toMatchObject({
      attemptCount: 4,
      finalStatus: "current",
      warningDiagnosticCodes: ["LAD.LEARNING.NOTE"],
    });
    expect(inspectSubmissionAgainstAssignment(submission, assignment).ok).toBe(true);
    expect(submission).not.toHaveProperty("studentName");

    const comment = createSubmissionReviewComment(
      "Check the holding branch on this rung.",
      "00000000-0000-4000-8000-000000001201",
      () => "00000000-0000-4000-8000-000000001202",
    );
    const returned = recordSubmissionReview(
      submission,
      "revision-requested",
      "Explain why the Stop contact is normally closed.",
      [comment],
    );
    expect(returned).toMatchObject({
      lifecycle: "returned",
      review: {
        comments: [comment],
        decision: "revision-requested",
        feedback: "Explain why the Stop contact is normally closed.",
      },
    });

    const rereviewed = recordSubmissionReview(returned, "complete", "Good correction.");
    expect(rereviewed.review?.comments).toEqual([comment]);
    expect(rereviewed.review?.comments[0]?.commentId).toBe(comment.commentId);
    expect(acceptReviewedSubmissionForStudent(returned, assignment)).toBe(returned);
    expect(() => acceptReviewedSubmissionForStudent(submission, assignment))
      .toThrow("contains teacher feedback");
    expect(() => acceptReviewedSubmissionForStudent(returned, {
      ...assignment,
      revision: assignment.revision + 1,
    })).toThrow("does not match the active assignment and revision");
  });
});

const requirementObjects = (): Readonly<Record<string, WorkbenchObjectView>> => ({
  controller: object("controller", "Controller", "Controller", { catalogId: "vctrl-c1" }),
  input: object("input", "Module", "VDI16", { catalogId: "vdi16" }),
  motor: object("motor", "Tag", "Motor_Run", {
    addressArea: "Q",
    addressIntent: "explicit",
    bitOffset: unsignedValue(0),
    byteOffset: unsignedValue(0),
    dataType: "BOOL",
  }),
  output: object("output", "Module", "VDO16", { catalogId: "vdo16" }),
  program: object("program", "OB", "MainCycle", ladPayload([
    { mode: "normally-open", nodeKind: "contact" },
    { mode: "normally-closed", nodeKind: "contact" },
    { mode: "normal", nodeKind: "coil" },
  ])),
  start: object("start", "Tag", "Start_PB", {
    addressArea: "I",
    addressIntent: "explicit",
    bitOffset: unsignedValue(0),
    byteOffset: unsignedValue(0),
    dataType: "BOOL",
  }),
  stop: object("stop", "Tag", "Stop_PB", {
    addressArea: "I",
    addressIntent: "explicit",
    bitOffset: unsignedValue(1),
    byteOffset: unsignedValue(0),
    dataType: "BOOL",
  }),
});

const ladPayload = (nodes: readonly ProjectPayload[]): ProjectPayload => ({
  graph: recordValue({
    networks: [recordValue({ nodes: nodes.map((node) => recordValue(node)) })],
  }),
  language: "LAD",
});

const object = (
  id: string,
  kind: WorkbenchObjectView["kind"],
  displayName: string,
  semanticPayload: ProjectPayload,
): WorkbenchObjectView => ({
  children: [],
  creationOrdinal: "1",
  displayName,
  id,
  kind,
  lifecycle: "active",
  objectRevision: "1",
  parentId: null,
  payloadSchema: "test/1",
  presentationPayload: {},
  semanticPayload,
  semanticRevision: "1",
});

const sequentialIds = (start = 1): (() => string) => {
  let next = start;
  return () => `00000000-0000-4000-8000-${String(next++).padStart(12, "0")}`;
};

const result = (
  testId: string,
  status: "failed" | "passed",
): BehaviorTestResultV1 => ({
  checks: [{
    actual: { type: "BOOL", value: status === "passed" },
    comparison: "equals",
    expected: { type: "BOOL", value: true },
    passed: status === "passed",
    stepId: status === "passed" ? "motor-started" : "motor-stopped",
    target: { kind: "plc-tag", name: "Motor_Run" },
  }],
  completedStepCount: 1,
  errorCode: null,
  status,
  testId,
});

const passingResult = (test: BehaviorTestDefinitionV1): BehaviorTestResultV1 => ({
  checks: test.steps.flatMap((step) => step.kind === "expect-value" ? [{
    actual: step.expected,
    comparison: step.comparison,
    expected: step.expected,
    passed: true,
    stepId: step.stepId,
    target: step.target,
  }] : []),
  completedStepCount: test.steps.length,
  errorCode: null,
  status: "passed",
  testId: test.testId,
});
