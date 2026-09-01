import {
  EducationContractValidationError,
  inspectAssignmentDocument,
  inspectSubmissionAgainstAssignment,
  parseAssignmentDocument,
  parseSubmissionDocument,
} from "./education-contract";
import type {
  AssignmentDocumentV1,
  BehaviorTestDefinitionV1,
  BehaviorTestStepV1,
  BehaviorTestResultV1,
  EducationScalarValue,
  EducationValidationIssue,
  ProgressiveHintV1,
  ProjectArtifactV1,
  SubmissionDocumentV1,
  SubmissionReviewV1,
} from "./education-contract";
import { canonicalRecordFields } from "./canonical-authoring";
import { findMvpLadInstruction } from "./lad-instruction-catalog";
import type {
  ProjectPayload,
  WorkbenchObjectView,
  WorkbenchSnapshot,
} from "./workbench-types";

export type HintUsageV1 = SubmissionDocumentV1["evidence"]["hintUsage"][number];
export type SubmissionReviewCommentV1 = SubmissionReviewV1["comments"][number];

export type NextHintProjection = Readonly<{
  hint: ProgressiveHintV1 | null;
  reason: string;
}>;

export type SubmissionReadiness = Readonly<{
  ready: boolean;
  reasons: readonly string[];
}>;

export type ProjectRequirementIssue = Readonly<{
  code:
    | "ambiguous-tag"
    | "missing-controller"
    | "missing-module"
    | "missing-tag"
    | "multiple-controllers"
    | "prohibited-instruction"
    | "tag-address-mismatch"
    | "tag-type-mismatch"
    | "uninspectable-lad-graph"
    | "unsupported-controller";
  message: string;
}>;

export type ProjectRequirementInspection = Readonly<{
  issues: readonly ProjectRequirementIssue[];
  ready: boolean;
}>;

export type AssignmentAuthoringInspection = Readonly<{
  issues: readonly EducationValidationIssue[];
  ok: boolean;
}>;

type IdFactory = () => string;

export const createBlankAssignmentDraft = (
  idFactory: IdFactory = () => crypto.randomUUID(),
): AssignmentDocumentV1 => {
  const behaviorTest = createBehaviorTestDraft(idFactory);
  return {
    assignmentId: idFactory(),
    behaviorTests: [behaviorTest],
    documentKind: "vlab-assignment",
    hintPolicy: { hints: [], mode: "progressive", reveal: "student-request" },
    lifecycle: "draft",
    learningObjectives: ["Describe the intended PLC behavior and prove it with the virtual trainer."],
    requirements: {
      permittedLadInstructions: ["contact.no", "contact.nc", "coil.normal"],
      plcCatalogIds: ["vctrl-c1"],
      requiredModules: [
        { catalogId: "vdi16", quantity: 1 },
        { catalogId: "vdo16", quantity: 1 },
      ],
      starterTags: [
        { address: "%I0.0", dataType: "BOOL", description: "Virtual input used by the assignment", name: "Input_1" },
        { address: "%Q0.0", dataType: "BOOL", description: "Virtual output checked by the assignment", name: "Output_1" },
      ],
    },
    revision: 1,
    schemaVersion: 1,
    starterProject: { kind: "blank" },
    summary: "Build and test a virtual PLC solution for the behavior described in this assignment.",
    title: "Untitled PLC assignment",
  };
};

/** Starts a new assignment identity while keeping the source content as a template. */
export const cloneAssignmentAsDraft = (
  source: AssignmentDocumentV1,
  idFactory: IdFactory = () => crypto.randomUUID(),
): AssignmentDocumentV1 => {
  const testIds = new Map(source.behaviorTests.map((test) => [test.testId, idFactory()] as const));
  const hintIds = new Map(source.hintPolicy.hints.map((hint) => [hint.hintId, idFactory()] as const));
  const copyTitle = `${source.title} copy`.slice(0, 160).trim();
  return {
    ...source,
    assignmentId: idFactory(),
    behaviorTests: source.behaviorTests.map((test) => ({
      ...test,
      steps: test.steps.map((step) => ({ ...step })),
      testId: testIds.get(test.testId) ?? idFactory(),
    })),
    hintPolicy: {
      ...source.hintPolicy,
      hints: source.hintPolicy.hints.map((hint) => ({
        ...hint,
        hintId: hintIds.get(hint.hintId) ?? idFactory(),
        unlock: remapHintUnlock(hint.unlock, testIds, hintIds),
      })),
    },
    lifecycle: "draft",
    learningObjectives: [...source.learningObjectives],
    requirements: {
      permittedLadInstructions: [...source.requirements.permittedLadInstructions],
      plcCatalogIds: [...source.requirements.plcCatalogIds],
      requiredModules: source.requirements.requiredModules.map((module) => ({ ...module })),
      starterTags: source.requirements.starterTags.map((tag) => ({ ...tag })),
    },
    revision: 1,
    starterProject: source.starterProject.kind === "embedded-project"
      ? { artifact: { ...source.starterProject.artifact }, kind: "embedded-project" }
      : { ...source.starterProject },
    title: copyTitle || "Untitled PLC assignment copy",
  };
};

/** Revises one assignment in place, preserving assignment, test, hint, and step identities. */
export const reviseAssignmentAsDraft = (source: AssignmentDocumentV1): AssignmentDocumentV1 => ({
  ...source,
  lifecycle: "draft",
  revision: source.lifecycle === "draft" ? source.revision : source.revision + 1,
});

export const publishAssignmentDraft = (draft: AssignmentDocumentV1): AssignmentDocumentV1 => {
  const published = parseAssignmentDocument({ ...draft, lifecycle: "published" });
  const inspection = inspectAssignmentAuthoring(published);
  if (!inspection.ok) {
    throw new EducationContractValidationError(
      "The assignment has behavior targets that cannot be verified.",
      inspection.issues,
    );
  }
  return published;
};

/** Adds authoring semantics that the portable file schema cannot prove alone. */
export const inspectAssignmentAuthoring = (
  assignment: AssignmentDocumentV1,
): AssignmentAuthoringInspection => {
  const strict = inspectAssignmentDocument(assignment);
  const issues: EducationValidationIssue[] = strict.ok ? [] : [...strict.issues];
  if (!strict.ok) return { issues, ok: false };

  const tagsByName = new Map<string, AssignmentDocumentV1["requirements"]["starterTags"]>();
  assignment.requirements.starterTags.forEach((tag) => {
    tagsByName.set(tag.name, [...(tagsByName.get(tag.name) ?? []), tag]);
  });
  assignment.behaviorTests.forEach((test, testIndex) => {
    test.steps.forEach((step, stepIndex) => {
      if (step.kind !== "set-value" && step.kind !== "expect-value") return;
      const stepPath = `$.behaviorTests[${testIndex}].steps[${stepIndex}]`;
      if (step.target.kind === "hmi-control") {
        if (assignment.starterProject.kind !== "embedded-project") {
          issues.push({
            code: "invalid-reference",
            message: "HMI behavior targets require an embedded starter project so the control can be verified.",
            path: `${stepPath}.target`,
          });
        }
        return;
      }

      const matches = tagsByName.get(step.target.name) ?? [];
      if (matches.length !== 1) {
        issues.push({
          code: "invalid-reference",
          message: matches.length === 0
            ? `PLC target ${step.target.name} is not declared in Starter tags.`
            : `PLC target ${step.target.name} does not resolve uniquely in Starter tags.`,
          path: `${stepPath}.target.name`,
        });
        return;
      }
      const value = step.kind === "set-value" ? step.value : step.expected;
      const expectedTagType = educationScalarTagType(value);
      if (matches[0]?.dataType !== expectedTagType) {
        issues.push({
          code: "invalid-type",
          message: `${value.type} behavior evidence requires a ${expectedTagType} starter tag.`,
          path: `${stepPath}.${step.kind === "set-value" ? "value" : "expected"}.type`,
        });
      }
    });
  });
  return { issues, ok: issues.length === 0 };
};

/** Number of compile actions performed since this assignment was activated. */
export const assignmentCompileAttemptDelta = (
  compileAttemptCount: number,
  activationBaseline: number,
): number => Math.max(0, compileAttemptCount - activationBaseline);

/** Inspects assignment requirements against the canonical, persisted project snapshot. */
export const inspectProjectRequirements = (
  assignment: AssignmentDocumentV1,
  snapshot: Pick<WorkbenchSnapshot, "objects">,
): ProjectRequirementInspection => {
  const activeObjects = Object.values(snapshot.objects).filter((object) => object.lifecycle === "active");
  const issues: ProjectRequirementIssue[] = [];

  const controllers = activeObjects.filter((object) => object.kind === "Controller");
  if (controllers.length === 0) {
    issues.push({ code: "missing-controller", message: "Add a virtual PLC allowed by this assignment." });
  } else {
    if (controllers.length > 1) {
      issues.push({
        code: "multiple-controllers",
        message: `Keep one virtual PLC for this assignment; the project currently has ${controllers.length}.`,
      });
    }
    controllers.forEach((controller) => {
      const catalogId = typeof controller.semanticPayload.catalogId === "string"
        ? controller.semanticPayload.catalogId
        : "";
      if (!assignment.requirements.plcCatalogIds.includes(catalogId)) {
        issues.push({
          code: "unsupported-controller",
          message: `${controller.displayName} uses PLC catalog ${catalogId || "(unconfigured)"}, which this assignment does not allow.`,
        });
      }
    });
  }

  const moduleCounts = new Map<string, number>();
  activeObjects.filter((object) => object.kind === "Module").forEach((module) => {
    const catalogId = module.semanticPayload.catalogId;
    if (typeof catalogId === "string") moduleCounts.set(catalogId, (moduleCounts.get(catalogId) ?? 0) + 1);
  });
  assignment.requirements.requiredModules.forEach((required) => {
    const actual = moduleCounts.get(required.catalogId) ?? 0;
    if (actual < required.quantity) {
      const missing = required.quantity - actual;
      issues.push({
        code: "missing-module",
        message: `Add ${missing} more ${required.catalogId} module${missing === 1 ? "" : "s"} (${actual}/${required.quantity} configured).`,
      });
    }
  });

  const tags = activeObjects.filter((object) => object.kind === "Tag");
  assignment.requirements.starterTags.forEach((required) => {
    const matches = tags.filter((tag) => tag.displayName === required.name);
    if (matches.length === 0) {
      issues.push({ code: "missing-tag", message: `Create the required PLC tag ${required.name}.` });
      return;
    }
    if (matches.length > 1) {
      issues.push({ code: "ambiguous-tag", message: `Keep exactly one active PLC tag named ${required.name}.` });
      return;
    }
    const tag = matches[0];
    if (tag === undefined) return;
    const actualType = typeof tag.semanticPayload.dataType === "string"
      ? tag.semanticPayload.dataType.toLocaleUpperCase("en-US")
      : "";
    if (actualType !== required.dataType) {
      issues.push({
        code: "tag-type-mismatch",
        message: `${required.name} must be ${required.dataType}, not ${actualType || "an unconfigured type"}.`,
      });
    }
    const actualAddress = observableTagAddress(tag.semanticPayload);
    if (required.address !== null && actualAddress !== required.address.toLocaleUpperCase("en-US")) {
      issues.push({
        code: "tag-address-mismatch",
        message: `${required.name} must use ${required.address}; the project shows ${actualAddress ?? "an automatic or unresolved address"}.`,
      });
    }
  });

  const permitted = new Set(assignment.requirements.permittedLadInstructions);
  const usedInstructions = inspectUsedLadInstructions(activeObjects, issues);
  [...usedInstructions].sort((left, right) => left.localeCompare(right, "en-US")).forEach((instruction) => {
    if (!permitted.has(instruction)) {
      issues.push({
        code: "prohibited-instruction",
        message: `${instruction} is used in LAD but is not permitted by this assignment.`,
      });
    }
  });

  return { issues, ready: issues.length === 0 };
};

export const createBehaviorTestDraft = (
  idFactory: IdFactory = () => crypto.randomUUID(),
): BehaviorTestDefinitionV1 => ({
  description: "Set a virtual input, run the PLC scan, and verify the expected output.",
  steps: [
    createBehaviorStepDraft("reset-runtime", idFactory),
    createBehaviorStepDraft("set-value", idFactory),
    createBehaviorStepDraft("run-scans", idFactory),
    createBehaviorStepDraft("expect-value", idFactory),
  ],
  testId: idFactory(),
  title: "Expected PLC response",
  visibility: "student",
});

export const createBehaviorStepDraft = (
  kind: BehaviorTestStepV1["kind"],
  idFactory: IdFactory = () => crypto.randomUUID(),
): BehaviorTestStepV1 => {
  const stepId = idFactory();
  switch (kind) {
    case "reset-runtime": return { kind, stepId };
    case "set-value": return {
      kind,
      stepId,
      target: { kind: "plc-tag", name: "Input_1" },
      value: { type: "BOOL", value: true },
    };
    case "run-scans": return { count: 1, kind, stepId };
    case "expect-value": return {
      comparison: "equals",
      expected: { type: "BOOL", value: true },
      kind,
      stepId,
      target: { kind: "plc-tag", name: "Output_1" },
    };
  }
};

export const createProgressiveHintDraft = (
  existing: readonly ProgressiveHintV1[],
  idFactory: IdFactory = () => crypto.randomUUID(),
): ProgressiveHintV1 => {
  const previous = existing.at(-1);
  return {
    body: "Point the learner toward the next debugging observation without giving away the finished rung.",
    hintId: idFactory(),
    order: (previous?.order ?? 0) + 1,
    title: `Hint ${existing.length + 1}`,
    unlock: previous === undefined
      ? { kind: "immediate" }
      : { hintId: previous.hintId, kind: "after-previous-hint" },
  };
};

export const projectNextProgressiveHint = (
  assignment: AssignmentDocumentV1,
  usage: readonly HintUsageV1[],
  compileAttemptCount: number,
  behaviorAttempts: readonly BehaviorTestResultV1[],
): NextHintProjection => {
  const used = new Set(usage.map((entry) => entry.hintId));
  const next = assignment.hintPolicy.hints.find((hint) => !used.has(hint.hintId));
  if (next === undefined) {
    return { hint: null, reason: "You have reviewed every hint in this assignment." };
  }
  const unlock = next.unlock;
  switch (unlock.kind) {
    case "immediate":
      return { hint: next, reason: "The next hint is ready." };
    case "after-compile-attempts":
      return compileAttemptCount >= unlock.minimum
        ? { hint: next, reason: "The next hint is ready." }
        : {
            hint: null,
            reason: `Compile ${pluralize(unlock.minimum, "attempt")} before opening the next hint.`,
          };
    case "after-behavior-failures": {
      const failures = behaviorAttempts.filter((result) =>
        result.status === "failed" && (unlock.testId === null || result.testId === unlock.testId)
      ).length;
      return failures >= unlock.minimum
        ? { hint: next, reason: "The next hint is ready." }
        : {
            hint: null,
            reason: unlock.testId === null
              ? `Try the behavior checks and learn from ${pluralize(unlock.minimum, "failed attempt")} first.`
              : `Try the linked behavior check and learn from ${pluralize(unlock.minimum, "failed attempt")} first.`,
          };
    }
    case "after-previous-hint":
      return used.has(unlock.hintId)
        ? { hint: next, reason: "The next hint is ready." }
        : { hint: null, reason: "Review the earlier hint before opening this one." };
  }
};

export const appendHintUsage = (
  usage: readonly HintUsageV1[],
  hint: ProgressiveHintV1,
): readonly HintUsageV1[] => usage.some((entry) => entry.hintId === hint.hintId)
  ? usage
  : [...usage, { hintId: hint.hintId, sequence: usage.length + 1 }];

export const inspectSubmissionReadiness = (
  assignment: AssignmentDocumentV1,
  snapshot: Pick<WorkbenchSnapshot, "buildState" | "projectHash">,
  latestResults: readonly BehaviorTestResultV1[],
  resultsProjectHash: string | null,
  projectArtifactAvailable: boolean,
  compileEvidenceAvailable = true,
  projectRequirementIssues: readonly ProjectRequirementIssue[] = [],
): SubmissionReadiness => {
  const reasons: string[] = [];
  if (snapshot.buildState !== "current") {
    reasons.push("Compile the current project without blocking errors.");
  }
  if (resultsProjectHash !== snapshot.projectHash) {
    reasons.push("Run the behavior checks again for the current project revision.");
  }
  const resultsByTest = new Map(latestResults.map((result) => [result.testId, result]));
  if (assignment.behaviorTests.some((test) => resultsByTest.get(test.testId)?.status !== "passed")) {
    reasons.push("Pass every teacher-defined behavior check.");
  }
  if (!projectArtifactAvailable) {
    reasons.push("Project packaging must be connected before this submission can be exported.");
  }
  if (!compileEvidenceAvailable) {
    reasons.push("Compile-attempt tracking must be connected before this submission can be exported.");
  }
  projectRequirementIssues.forEach((requirementIssue) => reasons.push(requirementIssue.message));
  return { ready: reasons.length === 0, reasons };
};

export const createEducationSubmission = (options: Readonly<{
  assignment: AssignmentDocumentV1;
  attemptOrdinal: number;
  behaviorResults: readonly BehaviorTestResultV1[];
  compileAttemptCount: number;
  hintUsage: readonly HintUsageV1[];
  idFactory?: () => string;
  project: ProjectArtifactV1;
  snapshot: Pick<WorkbenchSnapshot, "buildState" | "diagnostics">;
}>): SubmissionDocumentV1 => {
  const idFactory = options.idFactory ?? (() => crypto.randomUUID());
  const submission: SubmissionDocumentV1 = {
    assignmentId: options.assignment.assignmentId,
    assignmentRevision: options.assignment.revision,
    attemptOrdinal: options.attemptOrdinal,
    documentKind: "vlab-submission",
    evidence: {
      behaviorResults: options.behaviorResults,
      compile: {
        attemptCount: options.compileAttemptCount,
        blockingDiagnosticCodes: uniqueSorted(options.snapshot.diagnostics
          .filter((diagnostic) => diagnostic.blocking)
          .map((diagnostic) => diagnostic.code)),
        finalStatus: options.snapshot.buildState,
        warningDiagnosticCodes: uniqueSorted(options.snapshot.diagnostics
          .filter((diagnostic) => diagnostic.severity === "Warning")
          .map((diagnostic) => diagnostic.code)),
      },
      hintUsage: options.hintUsage,
    },
    lifecycle: "submitted",
    project: options.project,
    review: null,
    schemaVersion: 1,
    submissionId: idFactory(),
  };
  return parseSubmissionDocument(submission);
};

export const recordSubmissionReview = (
  submission: SubmissionDocumentV1,
  decision: "complete" | "revision-requested",
  feedback: string,
  comments: readonly SubmissionReviewCommentV1[] = submission.review?.comments ?? [],
): SubmissionDocumentV1 => parseSubmissionDocument({
  ...submission,
  lifecycle: decision === "complete" ? "reviewed" : "returned",
  review: {
    comments,
    decision,
    feedback: feedback.trim(),
  },
});

/** Creates a teacher comment once so its identity remains stable through edits and exports. */
export const createSubmissionReviewComment = (
  body: string,
  objectId: string | null,
  idFactory: IdFactory = () => crypto.randomUUID(),
): SubmissionReviewCommentV1 => ({
  body: body.trim(),
  commentId: idFactory(),
  objectId: objectId?.trim() || null,
});

/**
 * Accepts only feedback returned for the exact assignment that is active for the student.
 * This deliberately does not create or mutate a submission attempt.
 */
export const acceptReviewedSubmissionForStudent = (
  submission: SubmissionDocumentV1,
  assignment: AssignmentDocumentV1,
): SubmissionDocumentV1 => {
  const match = inspectSubmissionAgainstAssignment(submission, assignment);
  if (!match.ok) {
    throw new Error("This reviewed submission does not match the active assignment and revision.");
  }
  if (submission.lifecycle === "submitted" || submission.review === null) {
    throw new Error("Choose a submission file that contains teacher feedback.");
  }
  return submission;
};

const educationScalarTagType = (value: EducationScalarValue): "BOOL" | "DINT" | "TIME" => {
  switch (value.type) {
    case "BOOL": return "BOOL";
    case "DINT": return "DINT";
    case "TIME_MS": return "TIME";
  }
};

const observableTagAddress = (payload: ProjectPayload): string | null => {
  if (payload.addressIntent !== "explicit") return null;
  const area = typeof payload.addressArea === "string"
    ? payload.addressArea.toLocaleUpperCase("en-US")
    : null;
  const dataType = typeof payload.dataType === "string"
    ? payload.dataType.toLocaleUpperCase("en-US")
    : null;
  const byteOffset = canonicalUnsigned(payload.byteOffset);
  if ((area !== "I" && area !== "Q") || byteOffset === null) return null;
  if (dataType === "BOOL") {
    const bitOffset = canonicalUnsigned(payload.bitOffset);
    return bitOffset === null ? null : `%${area}${byteOffset}.${bitOffset}`;
  }
  return `%${area}W${byteOffset}`;
};

const inspectUsedLadInstructions = (
  objects: readonly WorkbenchObjectView[],
  issues: ProjectRequirementIssue[],
): ReadonlySet<string> => {
  const used = new Set<string>();
  objects.filter((object) =>
    (object.kind === "OB" || object.kind === "FC" || object.kind === "FB")
    && object.semanticPayload.language === "LAD"
  ).forEach((program) => {
    const graph = canonicalRecordFields(program.semanticPayload.graph);
    if (graph === null || !Array.isArray(graph.networks)) {
      issues.push({
        code: "uninspectable-lad-graph",
        message: `${program.displayName} has a LAD graph that cannot be checked against this assignment.`,
      });
      return;
    }
    let malformed = false;
    graph.networks.forEach((networkValue) => {
      const network = canonicalRecordFields(networkValue);
      if (network === null || !Array.isArray(network.nodes)) {
        malformed = true;
        return;
      }
      network.nodes.forEach((nodeValue) => {
        const node = canonicalRecordFields(nodeValue);
        if (node === null || typeof node.nodeKind !== "string") {
          malformed = true;
          return;
        }
        const token = canonicalLadInstructionToken(node);
        if (token !== null) used.add(token);
      });
    });
    if (malformed) {
      issues.push({
        code: "uninspectable-lad-graph",
        message: `${program.displayName} contains malformed LAD nodes that cannot be checked.`,
      });
    }
  });
  return used;
};

const canonicalLadInstructionToken = (node: ProjectPayload): string | null => {
  switch (node.nodeKind) {
    case "contact": {
      if (node.mode === "normally-open") return "contact.no";
      if (node.mode === "normally-closed") return "contact.nc";
      return `contact.${typeof node.mode === "string" ? node.mode : "unknown"}`;
    }
    case "coil": {
      if (node.mode === "normal") return "coil.normal";
      return `coil.${typeof node.mode === "string" ? node.mode : "unknown"}`;
    }
    case "box": {
      const code = canonicalUnsigned(node.instructionCode);
      const instruction = code === null ? null : findMvpLadInstruction(code);
      return instruction?.key ?? `box.unknown-${code ?? "code"}`;
    }
    case "call": return "call";
    case "instruction": {
      const code = canonicalUnsigned(node.instructionCode);
      const instruction = code === null ? null : findMvpLadInstruction(code);
      return instruction?.key ?? `instruction.unknown-${code ?? "code"}`;
    }
    default: return null;
  }
};

const canonicalUnsigned = (value: unknown): number | null => {
  if (
    typeof value !== "object"
    || value === null
    || Array.isArray(value)
  ) return null;
  const candidate = value as Readonly<Record<string, unknown>>;
  if (candidate.$type !== "u64" || typeof candidate.value !== "string") return null;
  const parsed = Number(candidate.value);
  return Number.isSafeInteger(parsed) && parsed >= 0 ? parsed : null;
};

const uniqueSorted = (values: readonly string[]): readonly string[] =>
  [...new Set(values)].sort((left, right) => left.localeCompare(right, "en-US"));

const remapHintUnlock = (
  unlock: ProgressiveHintV1["unlock"],
  testIds: ReadonlyMap<string, string>,
  hintIds: ReadonlyMap<string, string>,
): ProgressiveHintV1["unlock"] => {
  switch (unlock.kind) {
    case "immediate": return { kind: "immediate" };
    case "after-compile-attempts": return { kind: unlock.kind, minimum: unlock.minimum };
    case "after-behavior-failures": return {
      kind: unlock.kind,
      minimum: unlock.minimum,
      testId: unlock.testId === null ? null : testIds.get(unlock.testId) ?? unlock.testId,
    };
    case "after-previous-hint": return {
      hintId: hintIds.get(unlock.hintId) ?? unlock.hintId,
      kind: unlock.kind,
    };
  }
};

const pluralize = (count: number, noun: string): string => `${count} ${noun}${count === 1 ? "" : "s"}`;
