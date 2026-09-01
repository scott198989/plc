/** File formats exchanged by the offline student and teacher workflows. */
export const ASSIGNMENT_FILE_EXTENSION = ".vlabassign" as const;
export const SUBMISSION_FILE_EXTENSION = ".vlabsubmit" as const;
export const EDUCATION_CONTRACT_SCHEMA_VERSION = 1 as const;

const PROJECT_FILE_EXTENSION = ".vlabproj";
const MAX_PROJECT_PACKAGE_BYTES = 32 * 1024 * 1024;
const MAX_ISSUES = 100;
const MAX_SHORT_TEXT = 160;
const MAX_LONG_TEXT = 8_000;
const MAX_COLLECTION_ITEMS = 256;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;
const SHA256 = /^[A-F0-9]{64}$/u;
const BASE64 = /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/u;

export type EducationScalarValue =
  | Readonly<{ type: "BOOL"; value: boolean }>
  | Readonly<{ type: "DINT"; value: number }>
  | Readonly<{ type: "TIME_MS"; value: number }>;

export type BehaviorTarget = Readonly<{
  kind: "hmi-control" | "plc-tag";
  name: string;
}>;

export type BehaviorTestStepV1 =
  | Readonly<{ kind: "reset-runtime"; stepId: string }>
  | Readonly<{
      kind: "set-value";
      stepId: string;
      target: BehaviorTarget;
      value: EducationScalarValue;
    }>
  | Readonly<{ count: number; kind: "run-scans"; stepId: string }>
  | Readonly<{
      comparison: "equals";
      expected: EducationScalarValue;
      kind: "expect-value";
      stepId: string;
      target: BehaviorTarget;
    }>;

export type BehaviorTestDefinitionV1 = Readonly<{
  description: string;
  steps: readonly BehaviorTestStepV1[];
  testId: string;
  title: string;
  visibility: "student" | "teacher-only";
}>;

export type BehaviorCheckResultV1 = Readonly<{
  actual: EducationScalarValue | null;
  comparison: "equals";
  expected: EducationScalarValue;
  passed: boolean;
  stepId: string;
  target: BehaviorTarget;
}>;

export type BehaviorTestResultV1 = Readonly<{
  checks: readonly BehaviorCheckResultV1[];
  completedStepCount: number;
  errorCode: "driver-error" | null;
  status: "error" | "failed" | "passed";
  testId: string;
}>;

export type BehaviorTestDriver = Readonly<{
  readValue: (target: BehaviorTarget) => EducationScalarValue | null;
  resetRuntime: () => void;
  runScans: (count: number) => void;
  setValue: (target: BehaviorTarget, value: EducationScalarValue) => void;
}>;

export type ProgressiveHintUnlockV1 =
  | Readonly<{ kind: "immediate" }>
  | Readonly<{ kind: "after-compile-attempts"; minimum: number }>
  | Readonly<{
      kind: "after-behavior-failures";
      minimum: number;
      testId: string | null;
    }>
  | Readonly<{ hintId: string; kind: "after-previous-hint" }>;

export type ProgressiveHintV1 = Readonly<{
  body: string;
  hintId: string;
  order: number;
  title: string;
  unlock: ProgressiveHintUnlockV1;
}>;

export type ProgressiveHintPolicyV1 = Readonly<{
  hints: readonly ProgressiveHintV1[];
  mode: "progressive";
  reveal: "student-request";
}>;

export type ProjectArtifactV1 = Readonly<{
  fileName: string;
  packageBase64: string;
  sha256Hex: string;
}>;

export type AssignmentStarterProjectV1 =
  | Readonly<{ kind: "blank" }>
  | Readonly<{ kind: "built-in-template"; templateId: string }>
  | Readonly<{ artifact: ProjectArtifactV1; kind: "embedded-project" }>;

export type AssignmentRequirementV1 = Readonly<{
  permittedLadInstructions: readonly string[];
  plcCatalogIds: readonly string[];
  requiredModules: readonly Readonly<{ catalogId: string; quantity: number }>[];
  starterTags: readonly Readonly<{
    address: string | null;
    dataType: "BOOL" | "DINT" | "TIME";
    description: string;
    name: string;
  }>[];
}>;

export type AssignmentDocumentV1 = Readonly<{
  assignmentId: string;
  behaviorTests: readonly BehaviorTestDefinitionV1[];
  documentKind: "vlab-assignment";
  hintPolicy: ProgressiveHintPolicyV1;
  lifecycle: "archived" | "draft" | "published";
  learningObjectives: readonly string[];
  requirements: AssignmentRequirementV1;
  revision: number;
  schemaVersion: typeof EDUCATION_CONTRACT_SCHEMA_VERSION;
  starterProject: AssignmentStarterProjectV1;
  summary: string;
  title: string;
}>;

export type SubmissionReviewV1 = Readonly<{
  comments: readonly Readonly<{
    body: string;
    commentId: string;
    objectId: string | null;
  }>[];
  decision: "complete" | "revision-requested";
  feedback: string;
}>;

export type SubmissionDocumentV1 = Readonly<{
  assignmentId: string;
  assignmentRevision: number;
  attemptOrdinal: number;
  documentKind: "vlab-submission";
  evidence: Readonly<{
    behaviorResults: readonly BehaviorTestResultV1[];
    compile: Readonly<{
      attemptCount: number;
      blockingDiagnosticCodes: readonly string[];
      finalStatus: "blocked" | "current" | "not-built" | "stale";
      warningDiagnosticCodes: readonly string[];
    }>;
    hintUsage: readonly Readonly<{ hintId: string; sequence: number }>[];
  }>;
  lifecycle: "returned" | "reviewed" | "submitted";
  project: ProjectArtifactV1;
  review: SubmissionReviewV1 | null;
  schemaVersion: typeof EDUCATION_CONTRACT_SCHEMA_VERSION;
  submissionId: string;
}>;

export type EducationValidationIssue = Readonly<{
  code:
    | "duplicate-value"
    | "invalid-format"
    | "invalid-reference"
    | "invalid-type"
    | "invalid-value"
    | "missing-field"
    | "unexpected-field";
  message: string;
  path: string;
}>;

export type EducationValidationResult<T> =
  | Readonly<{ issues: readonly []; ok: true; value: T }>
  | Readonly<{ issues: readonly EducationValidationIssue[]; ok: false }>;

export class EducationContractValidationError extends Error {
  public readonly issues: readonly EducationValidationIssue[];

  public constructor(message: string, issues: readonly EducationValidationIssue[]) {
    super(message);
    this.name = "EducationContractValidationError";
    this.issues = issues;
  }
}

/** Executes ordered, synchronous simulator steps and records only deterministic evidence. */
export const runBehaviorTest = (
  definition: BehaviorTestDefinitionV1,
  driver: BehaviorTestDriver,
): BehaviorTestResultV1 => {
  const checks: BehaviorCheckResultV1[] = [];
  let completedStepCount = 0;

  for (const step of definition.steps) {
    try {
      switch (step.kind) {
        case "reset-runtime":
          driver.resetRuntime();
          break;
        case "set-value":
          driver.setValue(step.target, step.value);
          break;
        case "run-scans":
          driver.runScans(step.count);
          break;
        case "expect-value": {
          const actual = driver.readValue(step.target);
          checks.push({
            actual,
            comparison: step.comparison,
            expected: step.expected,
            passed: scalarValuesEqual(actual, step.expected),
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

export const inspectAssignmentDocument = (
  value: unknown,
): EducationValidationResult<AssignmentDocumentV1> => {
  const issues: EducationValidationIssue[] = [];
  validateAssignment(value, "$", issues);
  return validationResult(value, issues);
};

export const inspectSubmissionDocument = (
  value: unknown,
): EducationValidationResult<SubmissionDocumentV1> => {
  const issues: EducationValidationIssue[] = [];
  validateSubmission(value, "$", issues);
  return validationResult(value, issues);
};

export const parseAssignmentDocument = (value: unknown): AssignmentDocumentV1 => {
  const result = inspectAssignmentDocument(value);
  if (!result.ok) {
    throw new EducationContractValidationError("The assignment file is invalid.", result.issues);
  }
  return result.value;
};

export const parseSubmissionDocument = (value: unknown): SubmissionDocumentV1 => {
  const result = inspectSubmissionDocument(value);
  if (!result.ok) {
    throw new EducationContractValidationError("The submission file is invalid.", result.issues);
  }
  return result.value;
};

/** Checks references that cannot be proven by validating either document alone. */
export const inspectSubmissionAgainstAssignment = (
  submission: SubmissionDocumentV1,
  assignment: AssignmentDocumentV1,
): EducationValidationResult<SubmissionDocumentV1> => {
  const issues: EducationValidationIssue[] = [];
  if (submission.assignmentId !== assignment.assignmentId) {
    issue(issues, "invalid-reference", "$.assignmentId", "Submission assignment does not match.");
  }
  if (submission.assignmentRevision !== assignment.revision) {
    issue(issues, "invalid-reference", "$.assignmentRevision", "Submission assignment revision does not match.");
  }

  const expectedTests = new Set(assignment.behaviorTests.map((test) => test.testId));
  const actualTests = new Set(submission.evidence.behaviorResults.map((result) => result.testId));
  for (const testId of expectedTests) {
    if (!actualTests.has(testId)) {
      issue(issues, "missing-field", "$.evidence.behaviorResults", `Missing behavior result for ${testId}.`);
    }
  }
  for (const testId of actualTests) {
    if (!expectedTests.has(testId)) {
      issue(issues, "invalid-reference", "$.evidence.behaviorResults", `Unknown behavior test ${testId}.`);
    }
  }

  const definitionsById = new Map(assignment.behaviorTests.map((test) => [test.testId, test] as const));
  submission.evidence.behaviorResults.forEach((result, resultIndex) => {
    const definition = definitionsById.get(result.testId);
    if (definition === undefined) return;
    const resultPath = `$.evidence.behaviorResults[${resultIndex}]`;
    if (result.completedStepCount > definition.steps.length) {
      issue(
        issues,
        "invalid-value",
        `${resultPath}.completedStepCount`,
        "Completed step count exceeds the behavior definition.",
      );
    }
    const expectedChecks = new Map(definition.steps.flatMap((step) =>
      step.kind === "expect-value" ? [[step.stepId, step] as const] : []
    ));
    const seenChecks = new Set<string>();
    result.checks.forEach((check, checkIndex) => {
      const checkPath = `${resultPath}.checks[${checkIndex}]`;
      const expected = expectedChecks.get(check.stepId);
      if (expected === undefined) {
        issue(issues, "invalid-reference", `${checkPath}.stepId`, "Check references an unknown expectation step.");
        return;
      }
      duplicate(check.stepId, seenChecks, `${checkPath}.stepId`, issues);
      if (!targetsEqual(check.target, expected.target)) {
        issue(issues, "invalid-reference", `${checkPath}.target`, "Check target differs from the assignment.");
      }
      if (!scalarValuesEqual(check.expected, expected.expected)) {
        issue(issues, "invalid-reference", `${checkPath}.expected`, "Expected value differs from the assignment.");
      }
    });
    if (result.status !== "error") {
      for (const stepId of expectedChecks.keys()) {
        if (!seenChecks.has(stepId)) {
          issue(issues, "missing-field", `${resultPath}.checks`, `Missing check for expectation ${stepId}.`);
        }
      }
    }
  });

  const knownHints = new Set(assignment.hintPolicy.hints.map((hint) => hint.hintId));
  submission.evidence.hintUsage.forEach((usage, index) => {
    if (!knownHints.has(usage.hintId)) {
      issue(
        issues,
        "invalid-reference",
        `$.evidence.hintUsage[${index}].hintId`,
        "Hint usage references an unknown assignment hint.",
      );
    }
  });
  return validationResult(submission, issues);
};

const scalarValuesEqual = (
  actual: EducationScalarValue | null,
  expected: EducationScalarValue,
): boolean => actual !== null && actual.type === expected.type && actual.value === expected.value;

const targetsEqual = (left: BehaviorTarget, right: BehaviorTarget): boolean =>
  left.kind === right.kind && left.name === right.name;

const validationResult = <T>(
  value: unknown,
  issues: readonly EducationValidationIssue[],
): EducationValidationResult<T> => issues.length === 0
  ? { issues: [], ok: true, value: value as T }
  : { issues, ok: false };

const validateAssignment = (
  value: unknown,
  path: string,
  issues: EducationValidationIssue[],
): void => {
  const record = exactRecord(value, path, [
    "assignmentId",
    "behaviorTests",
    "documentKind",
    "hintPolicy",
    "lifecycle",
    "learningObjectives",
    "requirements",
    "revision",
    "schemaVersion",
    "starterProject",
    "summary",
    "title",
  ], issues);
  if (record === null) return;
  literal(record.documentKind, "vlab-assignment", `${path}.documentKind`, issues);
  literal(record.schemaVersion, EDUCATION_CONTRACT_SCHEMA_VERSION, `${path}.schemaVersion`, issues);
  identifier(record.assignmentId, `${path}.assignmentId`, issues);
  positiveInteger(record.revision, `${path}.revision`, issues);
  boundedText(record.title, `${path}.title`, issues, MAX_SHORT_TEXT);
  boundedText(record.summary, `${path}.summary`, issues, MAX_LONG_TEXT);
  oneOf(record.lifecycle, ["archived", "draft", "published"], `${path}.lifecycle`, issues);
  textArray(record.learningObjectives, `${path}.learningObjectives`, issues, true);
  validateStarterProject(record.starterProject, `${path}.starterProject`, issues);
  validateRequirements(record.requirements, `${path}.requirements`, issues);
  validateHintPolicy(record.hintPolicy, `${path}.hintPolicy`, issues);
  validateBehaviorTests(record.behaviorTests, `${path}.behaviorTests`, issues);
  validateAssignmentReferences(record, path, issues);
};

const validateAssignmentReferences = (
  record: Readonly<Record<string, unknown>>,
  path: string,
  issues: EducationValidationIssue[],
): void => {
  if (!Array.isArray(record.behaviorTests) || !isPlainRecord(record.hintPolicy)) return;
  const knownTestIds = new Set(record.behaviorTests.flatMap((test) =>
    isPlainRecord(test) && typeof test.testId === "string" ? [test.testId] : []
  ));
  if (!Array.isArray(record.hintPolicy.hints)) return;
  record.hintPolicy.hints.forEach((hint, index) => {
    if (!isPlainRecord(hint) || !isPlainRecord(hint.unlock)) return;
    const testId = hint.unlock.testId;
    if (
      hint.unlock.kind === "after-behavior-failures"
      && typeof testId === "string"
      && !knownTestIds.has(testId)
    ) {
      issue(
        issues,
        "invalid-reference",
        `${path}.hintPolicy.hints[${index}].unlock.testId`,
        "Hint unlock references an unknown behavior test.",
      );
    }
  });
};

const validateStarterProject = (
  value: unknown,
  path: string,
  issues: EducationValidationIssue[],
): void => {
  const record = plainRecord(value, path, issues);
  if (record === null || typeof record.kind !== "string") return;
  switch (record.kind) {
    case "blank":
      enforceKeys(record, path, ["kind"], issues);
      return;
    case "built-in-template":
      enforceKeys(record, path, ["kind", "templateId"], issues);
      safeToken(record.templateId, `${path}.templateId`, issues);
      return;
    case "embedded-project":
      enforceKeys(record, path, ["artifact", "kind"], issues);
      validateProjectArtifact(record.artifact, `${path}.artifact`, issues);
      return;
    default:
      issue(issues, "invalid-value", `${path}.kind`, "Unknown starter project kind.");
  }
};

const validateRequirements = (
  value: unknown,
  path: string,
  issues: EducationValidationIssue[],
): void => {
  const record = exactRecord(value, path, [
    "permittedLadInstructions",
    "plcCatalogIds",
    "requiredModules",
    "starterTags",
  ], issues);
  if (record === null) return;
  tokenArray(record.permittedLadInstructions, `${path}.permittedLadInstructions`, issues, true);
  tokenArray(record.plcCatalogIds, `${path}.plcCatalogIds`, issues, true);

  const modules = boundedArray(record.requiredModules, `${path}.requiredModules`, issues, true);
  modules?.forEach((item, index) => {
    const module = exactRecord(item, `${path}.requiredModules[${index}]`, ["catalogId", "quantity"], issues);
    if (module === null) return;
    safeToken(module.catalogId, `${path}.requiredModules[${index}].catalogId`, issues);
    positiveInteger(module.quantity, `${path}.requiredModules[${index}].quantity`, issues);
  });

  const tags = boundedArray(record.starterTags, `${path}.starterTags`, issues, false);
  const tagNames = new Set<string>();
  tags?.forEach((item, index) => {
    const tagPath = `${path}.starterTags[${index}]`;
    const tag = exactRecord(item, tagPath, ["address", "dataType", "description", "name"], issues);
    if (tag === null) return;
    boundedText(tag.name, `${tagPath}.name`, issues, MAX_SHORT_TEXT);
    if (typeof tag.name === "string") duplicate(tag.name, tagNames, `${tagPath}.name`, issues);
    oneOf(tag.dataType, ["BOOL", "DINT", "TIME"], `${tagPath}.dataType`, issues);
    nullableText(tag.address, `${tagPath}.address`, issues, MAX_SHORT_TEXT);
    boundedText(tag.description, `${tagPath}.description`, issues, MAX_LONG_TEXT, true);
  });
};

const validateHintPolicy = (
  value: unknown,
  path: string,
  issues: EducationValidationIssue[],
): void => {
  const record = exactRecord(value, path, ["hints", "mode", "reveal"], issues);
  if (record === null) return;
  literal(record.mode, "progressive", `${path}.mode`, issues);
  literal(record.reveal, "student-request", `${path}.reveal`, issues);
  const hints = boundedArray(record.hints, `${path}.hints`, issues, false);
  const ids = new Set<string>();
  let previousOrder = 0;
  const priorIds = new Set<string>();
  hints?.forEach((item, index) => {
    const hintPath = `${path}.hints[${index}]`;
    const hint = exactRecord(item, hintPath, ["body", "hintId", "order", "title", "unlock"], issues);
    if (hint === null) return;
    identifier(hint.hintId, `${hintPath}.hintId`, issues);
    if (typeof hint.hintId === "string") duplicate(hint.hintId, ids, `${hintPath}.hintId`, issues);
    positiveInteger(hint.order, `${hintPath}.order`, issues);
    if (typeof hint.order === "number" && Number.isSafeInteger(hint.order) && hint.order <= previousOrder) {
      issue(issues, "invalid-value", `${hintPath}.order`, "Hint order must increase strictly.");
    }
    if (typeof hint.order === "number") previousOrder = hint.order;
    boundedText(hint.title, `${hintPath}.title`, issues, MAX_SHORT_TEXT);
    boundedText(hint.body, `${hintPath}.body`, issues, MAX_LONG_TEXT);
    validateHintUnlock(hint.unlock, `${hintPath}.unlock`, priorIds, issues);
    if (typeof hint.hintId === "string") priorIds.add(hint.hintId);
  });
};

const validateHintUnlock = (
  value: unknown,
  path: string,
  priorHintIds: ReadonlySet<string>,
  issues: EducationValidationIssue[],
): void => {
  const record = plainRecord(value, path, issues);
  if (record === null || typeof record.kind !== "string") return;
  switch (record.kind) {
    case "immediate":
      enforceKeys(record, path, ["kind"], issues);
      return;
    case "after-compile-attempts":
      enforceKeys(record, path, ["kind", "minimum"], issues);
      positiveInteger(record.minimum, `${path}.minimum`, issues);
      return;
    case "after-behavior-failures":
      enforceKeys(record, path, ["kind", "minimum", "testId"], issues);
      positiveInteger(record.minimum, `${path}.minimum`, issues);
      if (record.testId !== null) identifier(record.testId, `${path}.testId`, issues);
      return;
    case "after-previous-hint":
      enforceKeys(record, path, ["hintId", "kind"], issues);
      identifier(record.hintId, `${path}.hintId`, issues);
      if (typeof record.hintId === "string" && !priorHintIds.has(record.hintId)) {
        issue(issues, "invalid-reference", `${path}.hintId`, "Previous hint must appear earlier in the policy.");
      }
      return;
    default:
      issue(issues, "invalid-value", `${path}.kind`, "Unknown hint unlock rule.");
  }
};

const validateBehaviorTests = (
  value: unknown,
  path: string,
  issues: EducationValidationIssue[],
): void => {
  const tests = boundedArray(value, path, issues, true);
  const testIds = new Set<string>();
  tests?.forEach((item, index) => {
    const testPath = `${path}[${index}]`;
    const test = exactRecord(item, testPath, ["description", "steps", "testId", "title", "visibility"], issues);
    if (test === null) return;
    identifier(test.testId, `${testPath}.testId`, issues);
    if (typeof test.testId === "string") duplicate(test.testId, testIds, `${testPath}.testId`, issues);
    boundedText(test.title, `${testPath}.title`, issues, MAX_SHORT_TEXT);
    boundedText(test.description, `${testPath}.description`, issues, MAX_LONG_TEXT);
    oneOf(test.visibility, ["student", "teacher-only"], `${testPath}.visibility`, issues);
    const steps = boundedArray(test.steps, `${testPath}.steps`, issues, true);
    const stepIds = new Set<string>();
    let expectationCount = 0;
    steps?.forEach((step, stepIndex) => {
      const stepPath = `${testPath}.steps[${stepIndex}]`;
      if (validateBehaviorStep(step, stepPath, issues)) expectationCount += 1;
      const record = isPlainRecord(step) ? step : null;
      if (record !== null && typeof record.stepId === "string") {
        duplicate(record.stepId, stepIds, `${stepPath}.stepId`, issues);
      }
    });
    if (steps !== null && expectationCount === 0) {
      issue(issues, "invalid-value", `${testPath}.steps`, "A behavior test needs at least one expectation.");
    }
  });
};

const validateBehaviorStep = (
  value: unknown,
  path: string,
  issues: EducationValidationIssue[],
): boolean => {
  const record = plainRecord(value, path, issues);
  if (record === null || typeof record.kind !== "string") return false;
  switch (record.kind) {
    case "reset-runtime":
      enforceKeys(record, path, ["kind", "stepId"], issues);
      safeToken(record.stepId, `${path}.stepId`, issues);
      return false;
    case "set-value":
      enforceKeys(record, path, ["kind", "stepId", "target", "value"], issues);
      safeToken(record.stepId, `${path}.stepId`, issues);
      validateTarget(record.target, `${path}.target`, issues);
      validateScalar(record.value, `${path}.value`, issues);
      return false;
    case "run-scans":
      enforceKeys(record, path, ["count", "kind", "stepId"], issues);
      safeToken(record.stepId, `${path}.stepId`, issues);
      boundedPositiveInteger(record.count, `${path}.count`, issues, 10_000);
      return false;
    case "expect-value":
      enforceKeys(record, path, ["comparison", "expected", "kind", "stepId", "target"], issues);
      safeToken(record.stepId, `${path}.stepId`, issues);
      literal(record.comparison, "equals", `${path}.comparison`, issues);
      validateTarget(record.target, `${path}.target`, issues);
      validateScalar(record.expected, `${path}.expected`, issues);
      return true;
    default:
      issue(issues, "invalid-value", `${path}.kind`, "Unknown behavior step kind.");
      return false;
  }
};

const validateSubmission = (
  value: unknown,
  path: string,
  issues: EducationValidationIssue[],
): void => {
  const record = exactRecord(value, path, [
    "assignmentId",
    "assignmentRevision",
    "attemptOrdinal",
    "documentKind",
    "evidence",
    "lifecycle",
    "project",
    "review",
    "schemaVersion",
    "submissionId",
  ], issues);
  if (record === null) return;
  literal(record.documentKind, "vlab-submission", `${path}.documentKind`, issues);
  literal(record.schemaVersion, EDUCATION_CONTRACT_SCHEMA_VERSION, `${path}.schemaVersion`, issues);
  identifier(record.submissionId, `${path}.submissionId`, issues);
  identifier(record.assignmentId, `${path}.assignmentId`, issues);
  positiveInteger(record.assignmentRevision, `${path}.assignmentRevision`, issues);
  positiveInteger(record.attemptOrdinal, `${path}.attemptOrdinal`, issues);
  oneOf(record.lifecycle, ["returned", "reviewed", "submitted"], `${path}.lifecycle`, issues);
  validateProjectArtifact(record.project, `${path}.project`, issues);
  validateSubmissionEvidence(record.evidence, `${path}.evidence`, issues);
  if (record.review !== null) validateReview(record.review, `${path}.review`, issues);
  if (record.lifecycle === "submitted" && record.review !== null) {
    issue(issues, "invalid-value", `${path}.review`, "A newly submitted file cannot already contain a review.");
  }
  if ((record.lifecycle === "returned" || record.lifecycle === "reviewed") && record.review === null) {
    issue(issues, "missing-field", `${path}.review`, "Returned and reviewed submissions require feedback.");
  }
  if (
    record.lifecycle === "returned"
    && isPlainRecord(record.review)
    && record.review.decision !== "revision-requested"
  ) {
    issue(issues, "invalid-value", `${path}.review.decision`, "Returned work must request a revision.");
  }
  if (
    record.lifecycle === "reviewed"
    && isPlainRecord(record.review)
    && record.review.decision !== "complete"
  ) {
    issue(issues, "invalid-value", `${path}.review.decision`, "Reviewed work must be marked complete.");
  }
};

const validateSubmissionEvidence = (
  value: unknown,
  path: string,
  issues: EducationValidationIssue[],
): void => {
  const record = exactRecord(value, path, ["behaviorResults", "compile", "hintUsage"], issues);
  if (record === null) return;
  const compile = exactRecord(record.compile, `${path}.compile`, [
    "attemptCount",
    "blockingDiagnosticCodes",
    "finalStatus",
    "warningDiagnosticCodes",
  ], issues);
  if (compile !== null) {
    nonNegativeInteger(compile.attemptCount, `${path}.compile.attemptCount`, issues);
    oneOf(compile.finalStatus, ["blocked", "current", "not-built", "stale"], `${path}.compile.finalStatus`, issues);
    tokenArray(compile.blockingDiagnosticCodes, `${path}.compile.blockingDiagnosticCodes`, issues, false);
    tokenArray(compile.warningDiagnosticCodes, `${path}.compile.warningDiagnosticCodes`, issues, false);
  }
  const results = boundedArray(record.behaviorResults, `${path}.behaviorResults`, issues, false);
  const resultIds = new Set<string>();
  results?.forEach((item, index) => {
    validateBehaviorResult(item, `${path}.behaviorResults[${index}]`, resultIds, issues);
  });
  const usage = boundedArray(record.hintUsage, `${path}.hintUsage`, issues, false);
  const hintIds = new Set<string>();
  usage?.forEach((item, index) => {
    const usagePath = `${path}.hintUsage[${index}]`;
    const entry = exactRecord(item, usagePath, ["hintId", "sequence"], issues);
    if (entry === null) return;
    identifier(entry.hintId, `${usagePath}.hintId`, issues);
    if (typeof entry.hintId === "string") duplicate(entry.hintId, hintIds, `${usagePath}.hintId`, issues);
    positiveInteger(entry.sequence, `${usagePath}.sequence`, issues);
    if (entry.sequence !== index + 1) {
      issue(issues, "invalid-value", `${usagePath}.sequence`, "Hint usage sequence must be contiguous.");
    }
  });
};

const validateBehaviorResult = (
  value: unknown,
  path: string,
  resultIds: Set<string>,
  issues: EducationValidationIssue[],
): void => {
  const record = exactRecord(value, path, [
    "checks",
    "completedStepCount",
    "errorCode",
    "status",
    "testId",
  ], issues);
  if (record === null) return;
  identifier(record.testId, `${path}.testId`, issues);
  if (typeof record.testId === "string") duplicate(record.testId, resultIds, `${path}.testId`, issues);
  nonNegativeInteger(record.completedStepCount, `${path}.completedStepCount`, issues);
  oneOf(record.status, ["error", "failed", "passed"], `${path}.status`, issues);
  if (record.errorCode !== null) literal(record.errorCode, "driver-error", `${path}.errorCode`, issues);
  if ((record.status === "error") !== (record.errorCode === "driver-error")) {
    issue(issues, "invalid-value", `${path}.errorCode`, "Only error results carry driver-error.");
  }
  const checks = boundedArray(record.checks, `${path}.checks`, issues, false);
  let failedChecks = 0;
  checks?.forEach((item, index) => {
    const checkPath = `${path}.checks[${index}]`;
    const check = exactRecord(item, checkPath, [
      "actual",
      "comparison",
      "expected",
      "passed",
      "stepId",
      "target",
    ], issues);
    if (check === null) return;
    safeToken(check.stepId, `${checkPath}.stepId`, issues);
    literal(check.comparison, "equals", `${checkPath}.comparison`, issues);
    validateTarget(check.target, `${checkPath}.target`, issues);
    validateScalar(check.expected, `${checkPath}.expected`, issues);
    if (check.actual !== null) validateScalar(check.actual, `${checkPath}.actual`, issues);
    if (typeof check.passed !== "boolean") {
      issue(issues, "invalid-type", `${checkPath}.passed`, "Expected a Boolean.");
    } else {
      const expectedPass = isScalar(check.expected)
        && (check.actual === null || isScalar(check.actual))
        && scalarValuesEqual(check.actual as EducationScalarValue | null, check.expected);
      if (check.passed !== expectedPass) {
        issue(issues, "invalid-value", `${checkPath}.passed`, "Check outcome does not match its values.");
      }
      if (!check.passed) failedChecks += 1;
    }
  });
  if (record.status === "passed" && failedChecks > 0) {
    issue(issues, "invalid-value", `${path}.status`, "Passed results cannot contain a failed check.");
  }
  if (record.status === "failed" && failedChecks === 0) {
    issue(issues, "invalid-value", `${path}.status`, "Failed results require a failed check.");
  }
  if ((record.status === "passed" || record.status === "failed") && checks?.length === 0) {
    issue(issues, "missing-field", `${path}.checks`, "Completed results require expectation checks.");
  }
};

const validateReview = (value: unknown, path: string, issues: EducationValidationIssue[]): void => {
  const record = exactRecord(value, path, ["comments", "decision", "feedback"], issues);
  if (record === null) return;
  oneOf(record.decision, ["complete", "revision-requested"], `${path}.decision`, issues);
  boundedText(record.feedback, `${path}.feedback`, issues, MAX_LONG_TEXT);
  const comments = boundedArray(record.comments, `${path}.comments`, issues, false);
  const ids = new Set<string>();
  comments?.forEach((item, index) => {
    const commentPath = `${path}.comments[${index}]`;
    const comment = exactRecord(item, commentPath, ["body", "commentId", "objectId"], issues);
    if (comment === null) return;
    identifier(comment.commentId, `${commentPath}.commentId`, issues);
    if (typeof comment.commentId === "string") duplicate(comment.commentId, ids, `${commentPath}.commentId`, issues);
    boundedText(comment.body, `${commentPath}.body`, issues, MAX_LONG_TEXT);
    if (comment.objectId !== null) identifier(comment.objectId, `${commentPath}.objectId`, issues);
  });
};

const validateProjectArtifact = (
  value: unknown,
  path: string,
  issues: EducationValidationIssue[],
): void => {
  const record = exactRecord(value, path, ["fileName", "packageBase64", "sha256Hex"], issues);
  if (record === null) return;
  if (
    typeof record.fileName !== "string"
    || record.fileName.length === 0
    || record.fileName.length > 255
    || record.fileName !== record.fileName.trim()
    || !record.fileName.toLocaleLowerCase("en-US").endsWith(PROJECT_FILE_EXTENSION)
    || /[\\/:*?"<>|\u0000-\u001f\u007f]/u.test(record.fileName)
  ) {
    issue(issues, "invalid-format", `${path}.fileName`, "Expected one safe .vlabproj file name.");
  }
  if (typeof record.sha256Hex !== "string" || !SHA256.test(record.sha256Hex)) {
    issue(issues, "invalid-format", `${path}.sha256Hex`, "Expected an uppercase SHA-256 digest.");
  }
  if (
    typeof record.packageBase64 !== "string"
    || record.packageBase64.length === 0
    || !BASE64.test(record.packageBase64)
    || decodedBase64Size(record.packageBase64) > MAX_PROJECT_PACKAGE_BYTES
  ) {
    issue(issues, "invalid-format", `${path}.packageBase64`, "Expected a bounded canonical Base64 project package.");
  }
};

const validateTarget = (value: unknown, path: string, issues: EducationValidationIssue[]): void => {
  const record = exactRecord(value, path, ["kind", "name"], issues);
  if (record === null) return;
  oneOf(record.kind, ["hmi-control", "plc-tag"], `${path}.kind`, issues);
  boundedText(record.name, `${path}.name`, issues, MAX_SHORT_TEXT);
};

const validateScalar = (value: unknown, path: string, issues: EducationValidationIssue[]): void => {
  const record = exactRecord(value, path, ["type", "value"], issues);
  if (record === null || typeof record.type !== "string") return;
  switch (record.type) {
    case "BOOL":
      if (typeof record.value !== "boolean") {
        issue(issues, "invalid-type", `${path}.value`, "BOOL values require a Boolean.");
      }
      return;
    case "DINT":
      if (
        typeof record.value !== "number"
        || !Number.isSafeInteger(record.value)
        || record.value < -2_147_483_648
        || record.value > 2_147_483_647
      ) {
        issue(issues, "invalid-value", `${path}.value`, "DINT value is out of range.");
      }
      return;
    case "TIME_MS":
      boundedNonNegativeInteger(record.value, `${path}.value`, issues, 2_147_483_647);
      return;
    default:
      issue(issues, "invalid-value", `${path}.type`, "Unknown education scalar type.");
  }
};

const exactRecord = (
  value: unknown,
  path: string,
  keys: readonly string[],
  issues: EducationValidationIssue[],
): Readonly<Record<string, unknown>> | null => {
  const record = plainRecord(value, path, issues);
  if (record !== null) enforceKeys(record, path, keys, issues);
  return record;
};

const plainRecord = (
  value: unknown,
  path: string,
  issues: EducationValidationIssue[],
): Readonly<Record<string, unknown>> | null => {
  if (!isPlainRecord(value)) {
    issue(issues, "invalid-type", path, "Expected a plain record.");
    return null;
  }
  return value;
};

const isPlainRecord = (value: unknown): value is Readonly<Record<string, unknown>> =>
  typeof value === "object"
  && value !== null
  && !Array.isArray(value)
  && Object.getPrototypeOf(value) === Object.prototype;

const enforceKeys = (
  record: Readonly<Record<string, unknown>>,
  path: string,
  keys: readonly string[],
  issues: EducationValidationIssue[],
): void => {
  const expected = new Set(keys);
  keys.forEach((key) => {
    if (!Object.hasOwn(record, key)) issue(issues, "missing-field", `${path}.${key}`, "Required field is missing.");
  });
  Object.keys(record).forEach((key) => {
    if (!expected.has(key)) issue(issues, "unexpected-field", `${path}.${key}`, "Field is not part of this schema version.");
  });
};

const boundedArray = (
  value: unknown,
  path: string,
  issues: EducationValidationIssue[],
  requireItem: boolean,
): readonly unknown[] | null => {
  if (!Array.isArray(value)) {
    issue(issues, "invalid-type", path, "Expected a list.");
    return null;
  }
  if ((requireItem && value.length === 0) || value.length > MAX_COLLECTION_ITEMS) {
    issue(issues, "invalid-value", path, `Expected ${requireItem ? "1" : "0"}-${MAX_COLLECTION_ITEMS} items.`);
  }
  return value;
};

const textArray = (
  value: unknown,
  path: string,
  issues: EducationValidationIssue[],
  requireItem: boolean,
): void => {
  const values = boundedArray(value, path, issues, requireItem);
  const seen = new Set<string>();
  values?.forEach((item, index) => {
    boundedText(item, `${path}[${index}]`, issues, MAX_LONG_TEXT);
    if (typeof item === "string") duplicate(item, seen, `${path}[${index}]`, issues);
  });
};

const tokenArray = (
  value: unknown,
  path: string,
  issues: EducationValidationIssue[],
  requireItem: boolean,
): void => {
  const values = boundedArray(value, path, issues, requireItem);
  const seen = new Set<string>();
  values?.forEach((item, index) => {
    safeToken(item, `${path}[${index}]`, issues);
    if (typeof item === "string") duplicate(item, seen, `${path}[${index}]`, issues);
  });
};

const boundedText = (
  value: unknown,
  path: string,
  issues: EducationValidationIssue[],
  maximum: number,
  allowEmpty = false,
): void => {
  if (
    typeof value !== "string"
    || value !== value.trim()
    || (!allowEmpty && value.length === 0)
    || value.length > maximum
    || /[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/u.test(value)
  ) {
    issue(issues, "invalid-format", path, `Expected trimmed text up to ${maximum} characters.`);
  }
};

const nullableText = (
  value: unknown,
  path: string,
  issues: EducationValidationIssue[],
  maximum: number,
): void => {
  if (value !== null) boundedText(value, path, issues, maximum);
};

const safeToken = (value: unknown, path: string, issues: EducationValidationIssue[]): void => {
  if (typeof value !== "string" || !/^[A-Za-z0-9][A-Za-z0-9._:/-]{0,127}$/u.test(value)) {
    issue(issues, "invalid-format", path, "Expected a stable identifier token.");
  }
};

const identifier = (value: unknown, path: string, issues: EducationValidationIssue[]): void => {
  if (typeof value !== "string" || !UUID.test(value)) {
    issue(issues, "invalid-format", path, "Expected a canonical UUID.");
  }
};

const literal = (
  value: unknown,
  expected: string | number,
  path: string,
  issues: EducationValidationIssue[],
): void => {
  if (value !== expected) issue(issues, "invalid-value", path, `Expected ${JSON.stringify(expected)}.`);
};

const oneOf = (
  value: unknown,
  choices: readonly string[],
  path: string,
  issues: EducationValidationIssue[],
): void => {
  if (typeof value !== "string" || !choices.includes(value)) {
    issue(issues, "invalid-value", path, `Expected one of ${choices.join(", ")}.`);
  }
};

const positiveInteger = (value: unknown, path: string, issues: EducationValidationIssue[]): void => {
  boundedPositiveInteger(value, path, issues, Number.MAX_SAFE_INTEGER);
};

const boundedPositiveInteger = (
  value: unknown,
  path: string,
  issues: EducationValidationIssue[],
  maximum: number,
): void => {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 1 || value > maximum) {
    issue(issues, "invalid-value", path, `Expected an integer from 1 to ${maximum}.`);
  }
};

const nonNegativeInteger = (value: unknown, path: string, issues: EducationValidationIssue[]): void => {
  boundedNonNegativeInteger(value, path, issues, Number.MAX_SAFE_INTEGER);
};

const boundedNonNegativeInteger = (
  value: unknown,
  path: string,
  issues: EducationValidationIssue[],
  maximum: number,
): void => {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0 || value > maximum) {
    issue(issues, "invalid-value", path, `Expected an integer from 0 to ${maximum}.`);
  }
};

const duplicate = (
  value: string,
  seen: Set<string>,
  path: string,
  issues: EducationValidationIssue[],
): void => {
  if (seen.has(value)) issue(issues, "duplicate-value", path, "Value must be unique in this list.");
  seen.add(value);
};

const issue = (
  issues: EducationValidationIssue[],
  code: EducationValidationIssue["code"],
  path: string,
  message: string,
): void => {
  if (issues.length < MAX_ISSUES) issues.push({ code, message, path });
};

const decodedBase64Size = (value: string): number => {
  const padding = value.endsWith("==") ? 2 : value.endsWith("=") ? 1 : 0;
  return (value.length / 4) * 3 - padding;
};

const isScalar = (value: unknown): value is EducationScalarValue => {
  if (!isPlainRecord(value)) return false;
  return (value.type === "BOOL" && typeof value.value === "boolean")
    || (value.type === "DINT" && typeof value.value === "number")
    || (value.type === "TIME_MS" && typeof value.value === "number");
};

const ids = {
  assignment: "2a3188ba-8d40-4f24-8d89-99a73e00a111",
  hint1: "d47fa57d-fd3f-44a4-ac73-bd013db1a101",
  hint2: "d47fa57d-fd3f-44a4-ac73-bd013db1a102",
  hint3: "d47fa57d-fd3f-44a4-ac73-bd013db1a103",
  startTest: "422221c8-c9b5-4ba3-b890-6ab5eb13a101",
  latchTest: "422221c8-c9b5-4ba3-b890-6ab5eb13a102",
  stopTest: "422221c8-c9b5-4ba3-b890-6ab5eb13a103",
} as const;

const plcTag = (name: string): BehaviorTarget => ({ kind: "plc-tag", name });
const bool = (value: boolean): EducationScalarValue => ({ type: "BOOL", value });

/** A UI-independent first lab that can ship with the offline application. */
export const BUILT_IN_MOTOR_STARTER_ASSIGNMENT: AssignmentDocumentV1 = {
  assignmentId: ids.assignment,
  behaviorTests: [
    {
      description: "Pressing Start while Stop is released energizes the motor output.",
      steps: [
        { kind: "reset-runtime", stepId: "reset" },
        { kind: "set-value", stepId: "stop-released", target: plcTag("Stop_PB"), value: bool(false) },
        { kind: "set-value", stepId: "start-pressed", target: plcTag("Start_PB"), value: bool(true) },
        { count: 1, kind: "run-scans", stepId: "scan-start" },
        {
          comparison: "equals",
          expected: bool(true),
          kind: "expect-value",
          stepId: "motor-started",
          target: plcTag("Motor_Run"),
        },
      ],
      testId: ids.startTest,
      title: "Start turns the motor on",
      visibility: "student",
    },
    {
      description: "The seal-in path keeps the motor energized after Start is released.",
      steps: [
        { kind: "reset-runtime", stepId: "reset" },
        { kind: "set-value", stepId: "stop-released", target: plcTag("Stop_PB"), value: bool(false) },
        { kind: "set-value", stepId: "start-pressed", target: plcTag("Start_PB"), value: bool(true) },
        { count: 1, kind: "run-scans", stepId: "scan-start" },
        { kind: "set-value", stepId: "start-released", target: plcTag("Start_PB"), value: bool(false) },
        { count: 1, kind: "run-scans", stepId: "scan-release" },
        {
          comparison: "equals",
          expected: bool(true),
          kind: "expect-value",
          stepId: "motor-sealed-in",
          target: plcTag("Motor_Run"),
        },
      ],
      testId: ids.latchTest,
      title: "Motor stays on after Start is released",
      visibility: "student",
    },
    {
      description: "Pressing Stop drops the motor, and releasing Stop does not restart it.",
      steps: [
        { kind: "reset-runtime", stepId: "reset" },
        { kind: "set-value", stepId: "stop-released", target: plcTag("Stop_PB"), value: bool(false) },
        { kind: "set-value", stepId: "start-pressed", target: plcTag("Start_PB"), value: bool(true) },
        { count: 1, kind: "run-scans", stepId: "scan-start" },
        { kind: "set-value", stepId: "start-released", target: plcTag("Start_PB"), value: bool(false) },
        { count: 1, kind: "run-scans", stepId: "scan-latch" },
        { kind: "set-value", stepId: "stop-pressed", target: plcTag("Stop_PB"), value: bool(true) },
        { count: 1, kind: "run-scans", stepId: "scan-stop" },
        {
          comparison: "equals",
          expected: bool(false),
          kind: "expect-value",
          stepId: "motor-stopped",
          target: plcTag("Motor_Run"),
        },
        { kind: "set-value", stepId: "stop-released-again", target: plcTag("Stop_PB"), value: bool(false) },
        { count: 1, kind: "run-scans", stepId: "scan-after-stop" },
        {
          comparison: "equals",
          expected: bool(false),
          kind: "expect-value",
          stepId: "motor-remains-stopped",
          target: plcTag("Motor_Run"),
        },
      ],
      testId: ids.stopTest,
      title: "Stop turns the motor off safely",
      visibility: "student",
    },
  ],
  documentKind: "vlab-assignment",
  hintPolicy: {
    hints: [
      {
        body: "Check what electrical state the Stop pushbutton has when nobody is pressing it. The ladder contact should make that normal state safe and useful.",
        hintId: ids.hint1,
        order: 1,
        title: "Think about the Stop contact",
        unlock: { kind: "after-behavior-failures", minimum: 1, testId: ids.stopTest },
      },
      {
        body: "The motor output must provide a second path around Start. That path should become true only after the motor has started.",
        hintId: ids.hint2,
        order: 2,
        title: "Look for a holding path",
        unlock: { hintId: ids.hint1, kind: "after-previous-hint" },
      },
      {
        body: "Put the Stop condition in series with a parallel branch containing Start on one path and a Motor_Run contact on the other, then drive the Motor_Run coil.",
        hintId: ids.hint3,
        order: 3,
        title: "Build the seal-in structure",
        unlock: { hintId: ids.hint2, kind: "after-previous-hint" },
      },
    ],
    mode: "progressive",
    reveal: "student-request",
  },
  learningObjectives: [
    "Build a safe Start and Stop control rung.",
    "Use a parallel seal-in branch to hold an output on.",
    "Verify logic by changing inputs and observing scan results.",
  ],
  lifecycle: "published",
  requirements: {
    permittedLadInstructions: ["contact.no", "contact.nc", "coil.normal"],
    plcCatalogIds: ["vctrl-c1", "vctrl-m1", "vctrl-p1"],
    requiredModules: [
      { catalogId: "vdi16", quantity: 1 },
      { catalogId: "vdo16", quantity: 1 },
    ],
    starterTags: [
      { address: "%I0.0", dataType: "BOOL", description: "Momentary Start pushbutton", name: "Start_PB" },
      { address: "%I0.1", dataType: "BOOL", description: "Momentary Stop pushbutton", name: "Stop_PB" },
      { address: "%Q0.0", dataType: "BOOL", description: "Motor starter output", name: "Motor_Run" },
    ],
  },
  revision: 1,
  schemaVersion: EDUCATION_CONTRACT_SCHEMA_VERSION,
  starterProject: { kind: "built-in-template", templateId: "builtin.virtual-plc-blank/1" },
  summary: "Configure a virtual PLC and build a Start/Stop motor circuit with a seal-in branch.",
  title: "Your first motor starter",
};
