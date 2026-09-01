/// <reference types="vite/client" />

import { useEffect, useMemo, useRef, useState } from "react";

import "./EducationWorkspace.css";
import {
  ASSIGNMENT_FILE_EXTENSION,
  BUILT_IN_MOTOR_STARTER_ASSIGNMENT,
  SUBMISSION_FILE_EXTENSION,
  inspectSubmissionAgainstAssignment,
  parseAssignmentDocument,
} from "./education-contract";
import type {
  AssignmentDocumentV1,
  BehaviorTestDefinitionV1,
  BehaviorTestStepV1,
  BehaviorTestResultV1,
  EducationScalarValue,
  ProgressiveHintV1,
  ProjectArtifactV1,
  SubmissionDocumentV1,
} from "./education-contract";
import {
  downloadEducationDocument,
  educationFileName,
  readEducationFile,
} from "./education-file-io";
import {
  inspectEducationRuntimeReadiness,
  runBehaviorTestAgainstRuntime,
} from "./education-runtime-adapter";
import type { EducationRuntimeBridge } from "./education-runtime-adapter";
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
} from "./education-workflow";
import type { HintUsageV1, SubmissionReviewCommentV1 } from "./education-workflow";
import type { EngineeringRuntimeView, RuntimeOperation } from "./runtime-types";
import { controllerCatalogs, digitalModuleCatalogs } from "./hardware-configuration";
import { MVP_LAD_INSTRUCTION_CATALOG } from "./lad-instruction-catalog";
import type { WorkbenchSnapshot } from "./workbench-types";

export type EducationWorkspaceProps = Readonly<{
  busy: boolean;
  compileAttemptCount?: number;
  hmiControlTagIds?: Readonly<Record<string, string>>;
  onExportProjectArtifact?: () => Promise<ProjectArtifactV1>;
  onLoadAssignmentStarter?: (assignment: AssignmentDocumentV1) => Promise<void>;
  onOpenSubmittedProject?: (artifact: ProjectArtifactV1) => Promise<void>;
  onResetRuntime?: () => Promise<EngineeringRuntimeView>;
  onRuntimeOperation?: (operation: RuntimeOperation) => Promise<EngineeringRuntimeView>;
  snapshot: Pick<
    WorkbenchSnapshot,
    "buildState" | "diagnostics" | "objects" | "projectHash" | "projectName" | "runtime"
  >;
}>;

type WorkspaceRole = "student" | "teacher";
type ReviewDecision = "complete" | "revision-requested";

export const isEducatorModeLocked = (role: WorkspaceRole, hasActiveAssignment: boolean): boolean =>
  role === "student" && hasActiveAssignment;

export const EducationWorkspace = ({
  busy,
  compileAttemptCount,
  hmiControlTagIds,
  onExportProjectArtifact,
  onLoadAssignmentStarter,
  onOpenSubmittedProject,
  onResetRuntime,
  onRuntimeOperation,
  snapshot,
}: EducationWorkspaceProps): React.JSX.Element => {
  const [role, setRole] = useState<WorkspaceRole>("student");
  const [assignment, setAssignment] = useState<AssignmentDocumentV1 | null>(null);
  const [assignmentDraft, setAssignmentDraft] = useState<AssignmentDocumentV1 | null>(null);
  const [behaviorAttempts, setBehaviorAttempts] = useState<readonly BehaviorTestResultV1[]>([]);
  const [latestResults, setLatestResults] = useState<readonly BehaviorTestResultV1[]>([]);
  const [resultsProjectHash, setResultsProjectHash] = useState<string | null>(null);
  const [hintUsage, setHintUsage] = useState<readonly HintUsageV1[]>([]);
  const [activeAction, setActiveAction] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [latestSubmission, setLatestSubmission] = useState<SubmissionDocumentV1 | null>(null);
  const [compileAttemptBaseline, setCompileAttemptBaseline] = useState<number | null>(null);
  const [studentReviewedSubmission, setStudentReviewedSubmission] = useState<SubmissionDocumentV1 | null>(null);
  const [teacherSubmission, setTeacherSubmission] = useState<SubmissionDocumentV1 | null>(null);
  const [attemptOrdinal, setAttemptOrdinal] = useState(1);
  const [reviewDecision, setReviewDecision] = useState<ReviewDecision>("complete");
  const [reviewFeedback, setReviewFeedback] = useState("");
  const [reviewComments, setReviewComments] = useState<readonly SubmissionReviewCommentV1[]>([]);
  const [educatorUnlocked, setEducatorUnlocked] = useState(false);
  const [educatorUnlockOpen, setEducatorUnlockOpen] = useState(false);
  const runtimeRef = useRef(snapshot.runtime);
  runtimeRef.current = snapshot.runtime;
  const assignmentInput = useRef<HTMLInputElement>(null);
  const teacherAssignmentInput = useRef<HTMLInputElement>(null);
  const submissionInput = useRef<HTMLInputElement>(null);
  const reviewedSubmissionInput = useRef<HTMLInputElement>(null);
  const educatorUnlockDialog = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = educatorUnlockDialog.current;
    if (dialog === null) return;
    if (educatorUnlockOpen && !dialog.open) dialog.showModal();
    if (!educatorUnlockOpen && dialog.open) dialog.close();
  }, [educatorUnlockOpen]);

  const latestResultById = useMemo(
    () => new Map(latestResults.map((result) => [result.testId, result])),
    [latestResults],
  );
  const runtimeReadiness = inspectEducationRuntimeReadiness(snapshot.runtime);
  const observedCompileAttemptCount = compileAttemptCount === undefined || compileAttemptBaseline === null
    ? 0
    : assignmentCompileAttemptDelta(compileAttemptCount, compileAttemptBaseline);
  const projectRequirementInspection = useMemo(
    () => assignment === null ? null : inspectProjectRequirements(assignment, snapshot),
    [assignment, snapshot.objects],
  );
  const nextHint = assignment === null
    ? null
    : projectNextProgressiveHint(assignment, hintUsage, observedCompileAttemptCount, behaviorAttempts);
  const submissionReadiness = assignment === null
    ? null
    : inspectSubmissionReadiness(
        assignment,
        snapshot,
        latestResults,
        resultsProjectHash,
        onExportProjectArtifact !== undefined,
        compileAttemptCount !== undefined,
        projectRequirementInspection?.issues ?? [],
      );
  const currentAssignmentForTeacher = assignment ?? BUILT_IN_MOTOR_STARTER_ASSIGNMENT;
  const workspaceBusy = busy || activeAction !== null;
  const studentAssignmentActive = isEducatorModeLocked(role, assignment !== null);

  const clearMessage = (): void => {
    setNotice(null);
    setError(null);
  };

  const activateAssignment = (next: AssignmentDocumentV1): void => {
    setAssignment(next);
    setCompileAttemptBaseline(compileAttemptCount ?? 0);
    setBehaviorAttempts([]);
    setLatestResults([]);
    setResultsProjectHash(null);
    setHintUsage([]);
    setLatestSubmission(null);
    setStudentReviewedSubmission(null);
    setAttemptOrdinal(1);
    setNotice(`Opened “${next.title}”.`);
    setError(null);
  };

  const importAssignment = async (file: File): Promise<void> => {
    setActiveAction("import-assignment");
    clearMessage();
    try {
      activateAssignment(await readEducationFile(file, "assignment"));
    } catch (reason) {
      setError(errorMessage(reason, "The assignment could not be opened."));
    } finally {
      setActiveAction(null);
    }
  };

  const runtimeBridge = (): EducationRuntimeBridge | null => {
    if (onRuntimeOperation === undefined || onResetRuntime === undefined) return null;
    return {
      current: () => runtimeRef.current,
      execute: async (operation) => {
        const next = await onRuntimeOperation(operation);
        runtimeRef.current = next;
        return next;
      },
      ...(hmiControlTagIds === undefined ? {} : { hmiControlTagIds }),
      reset: async () => {
        const next = await onResetRuntime();
        runtimeRef.current = next;
        return next;
      },
    };
  };

  const runChecks = async (tests: readonly BehaviorTestDefinitionV1[]): Promise<void> => {
    const bridge = runtimeBridge();
    if (assignment === null || bridge === null) {
      setError("Behavior checks need connected runtime command and reset callbacks.");
      return;
    }
    if (!runtimeReadiness.ready) {
      setError(runtimeReadiness.reason);
      return;
    }
    setActiveAction(tests.length === 1 ? tests[0]?.testId ?? "run-check" : "run-all");
    clearMessage();
    try {
      const completed: BehaviorTestResultV1[] = [];
      for (const test of tests) {
        completed.push(await runBehaviorTestAgainstRuntime(test, bridge));
      }
      setBehaviorAttempts((current) => [...current, ...completed]);
      setLatestResults((current) => replaceLatestResults(current, completed));
      setResultsProjectHash(snapshot.projectHash);
      const passed = completed.filter((result) => result.status === "passed").length;
      const errors = completed.filter((result) => result.status === "error").length;
      setNotice(errors > 0
        ? `${errors} ${errors === 1 ? "check could" : "checks could"} not complete. Verify runtime readiness and tag bindings.`
        : `${passed} of ${completed.length} ${completed.length === 1 ? "check" : "checks"} passed.`);
    } catch (reason) {
      setError(errorMessage(reason, "The behavior checks could not run."));
    } finally {
      setActiveAction(null);
    }
  };

  const exportSubmission = async (): Promise<void> => {
    if (assignment === null || onExportProjectArtifact === undefined || submissionReadiness?.ready !== true) {
      setError(submissionReadiness?.reasons[0] ?? "Project packaging is not connected.");
      return;
    }
    setActiveAction("export-submission");
    clearMessage();
    try {
      const submission = createEducationSubmission({
        assignment,
        attemptOrdinal,
        behaviorResults: latestResults,
        compileAttemptCount: observedCompileAttemptCount,
        hintUsage,
        project: await onExportProjectArtifact(),
        snapshot,
      });
      const references = inspectSubmissionAgainstAssignment(submission, assignment);
      if (!references.ok) throw new Error("Submission evidence does not match the active assignment.");
      downloadEducationDocument(
        submission,
        educationFileName(`${snapshot.projectName} submission`, SUBMISSION_FILE_EXTENSION),
      );
      setLatestSubmission(submission);
      setAttemptOrdinal((current) => current + 1);
      setNotice("Submission created and downloaded. Keep the file for your teacher.");
    } catch (reason) {
      setError(errorMessage(reason, "The submission could not be created."));
    } finally {
      setActiveAction(null);
    }
  };

  const importTeacherSubmission = async (file: File): Promise<void> => {
    setActiveAction("import-submission");
    clearMessage();
    try {
      const submission = await readEducationFile(file, "submission");
      const match = inspectSubmissionAgainstAssignment(submission, currentAssignmentForTeacher);
      if (!match.ok) {
        throw new Error("This submission does not match the assignment and revision currently open.");
      }
      setTeacherSubmission(submission);
      setReviewFeedback(submission.review?.feedback ?? "");
      setReviewDecision(submission.review?.decision ?? "complete");
      setReviewComments(submission.review?.comments ?? []);
      setNotice("Submission imported for local review.");
    } catch (reason) {
      setError(errorMessage(reason, "The submission could not be imported."));
    } finally {
      setActiveAction(null);
    }
  };

  const importStudentReviewedSubmission = async (file: File): Promise<void> => {
    if (assignment === null) return;
    setActiveAction("import-reviewed-submission");
    clearMessage();
    try {
      const reviewed = acceptReviewedSubmissionForStudent(
        await readEducationFile(file, "submission"),
        assignment,
      );
      setStudentReviewedSubmission(reviewed);
      setNotice(reviewed.lifecycle === "returned"
        ? `Teacher feedback opened for attempt ${reviewed.attemptOrdinal}. A revision was requested.`
        : `Teacher feedback opened for attempt ${reviewed.attemptOrdinal}. Work is complete.`);
    } catch (reason) {
      setError(errorMessage(reason, "The reviewed submission could not be opened."));
    } finally {
      setActiveAction(null);
    }
  };

  const recordReview = (): void => {
    if (teacherSubmission === null || reviewFeedback.trim().length === 0) return;
    try {
      const reviewed = recordSubmissionReview(
        teacherSubmission,
        reviewDecision,
        reviewFeedback,
        reviewComments,
      );
      setTeacherSubmission(reviewed);
      setReviewComments(reviewed.review?.comments ?? []);
      setNotice(reviewDecision === "complete"
        ? "Feedback recorded and work marked complete."
        : "Feedback recorded and revision requested.");
      setError(null);
    } catch (reason) {
      setError(errorMessage(reason, "Feedback could not be recorded."));
    }
  };

  return (
    <section aria-label="Education workspace" className="education-workspace">
      <header className="education-workspace__masthead">
        <div>
          <p className="education-workspace__eyebrow">Offline learning loop</p>
          <h1>{role === "student" ? "Student Mission" : "Teacher Portal"}</h1>
          <p>
            {role === "student"
              ? "Build, test, learn from evidence, and submit your virtual PLC project."
              : "Inspect the assignment, verify submitted evidence, and return focused feedback."}
          </p>
        </div>
        <div aria-label="Education workspace role" className="education-role-tabs" role="tablist">
          <button
            aria-selected={role === "student"}
            onClick={() => setRole("student")}
            role="tab"
            type="button"
          >Student</button>
          <button
            aria-selected={role === "teacher"}
            aria-describedby={studentAssignmentActive ? "educator-mode-locked" : undefined}
            disabled={studentAssignmentActive}
            onClick={() => {
              if (educatorUnlocked) {
                setRole("teacher");
              } else {
                setEducatorUnlockOpen(true);
              }
            }}
            role="tab"
            type="button"
          >Teacher</button>
          {studentAssignmentActive && (
            <span className="visually-hidden" id="educator-mode-locked">
              Exit the active student assignment before entering Educator mode.
            </span>
          )}
        </div>
      </header>

      <dialog
        aria-labelledby="educator-unlock-title"
        className="educator-unlock-dialog"
        onCancel={(event) => { event.preventDefault(); setEducatorUnlockOpen(false); }}
        ref={educatorUnlockDialog}
      >
        <p className="education-workspace__eyebrow">Role boundary</p>
        <h2 id="educator-unlock-title">Enter Educator mode?</h2>
        <p>Educator mode reveals teacher-only behavior checks, hints, authoring tools, and submitted evidence. Use it only in an educator-controlled session.</p>
        <p className="education-inline-note">This offline MVP uses a deliberate session unlock, not a password or identity claim.</p>
        <div>
          <button className="education-button" onClick={() => setEducatorUnlockOpen(false)} type="button">Stay in Student mode</button>
          <button
            className="education-button education-button--primary"
            onClick={() => {
              setEducatorUnlocked(true);
              setEducatorUnlockOpen(false);
              setRole("teacher");
            }}
            type="button"
          >Enter Educator mode</button>
        </div>
      </dialog>

      {error !== null && (
        <div className="education-banner education-banner--error" role="alert">
          <strong>Action not completed</strong><span>{error}</span>
        </div>
      )}
      {notice !== null && (
        <div className="education-banner education-banner--notice" role="status">
          <strong>Workspace update</strong><span>{notice}</span>
        </div>
      )}

      {role === "student" ? (
        assignment === null ? (
          <AssignmentLibrary
            disabled={workspaceBusy}
            inputRef={assignmentInput}
            onImport={importAssignment}
            onOpenBuiltIn={() => activateAssignment(BUILT_IN_MOTOR_STARTER_ASSIGNMENT)}
          />
        ) : (
          <StudentMission
            assignment={assignment}
            behaviorAttempts={behaviorAttempts}
            compileAttemptCount={compileAttemptCount === undefined ? undefined : observedCompileAttemptCount}
            disabled={workspaceBusy}
            hintUsage={hintUsage}
            latestResultById={latestResultById}
            latestSubmission={latestSubmission}
            nextHint={nextHint}
            onBack={() => {
              setAssignment(null);
              setCompileAttemptBaseline(null);
              setNotice(null);
              setError(null);
            }}
            onExportSubmission={exportSubmission}
            onLoadStarter={onLoadAssignmentStarter === undefined ? undefined : async () => {
              setActiveAction("load-starter");
              clearMessage();
              try {
                await onLoadAssignmentStarter(assignment);
                if (assignment.starterProject.kind === "embedded-project") {
                  setCompileAttemptBaseline(0);
                }
                setNotice("Assignment starter project loaded.");
              } catch (reason) {
                setError(errorMessage(reason, "The starter project could not be loaded."));
              } finally {
                setActiveAction(null);
              }
            }}
            onRevealHint={() => {
              if (nextHint?.hint !== null && nextHint?.hint !== undefined) {
                setHintUsage((current) => appendHintUsage(current, nextHint.hint as NonNullable<typeof nextHint.hint>));
              }
            }}
            onImportReviewedSubmission={importStudentReviewedSubmission}
            onRunAll={() => void runChecks(assignment.behaviorTests)}
            onRunTest={(test) => void runChecks([test])}
            projectHash={snapshot.projectHash}
            projectRequirementInspection={projectRequirementInspection}
            resultsProjectHash={resultsProjectHash}
            reviewedSubmission={studentReviewedSubmission}
            reviewedSubmissionInputRef={reviewedSubmissionInput}
            runtimeBridgeAvailable={onRuntimeOperation !== undefined && onResetRuntime !== undefined}
            runtimeReadiness={runtimeReadiness}
            snapshot={snapshot}
            submissionReadiness={submissionReadiness}
          />
        )
      ) : (
        <TeacherPortal
          assignment={currentAssignmentForTeacher}
          assignmentInputRef={teacherAssignmentInput}
          disabled={workspaceBusy}
          draft={assignmentDraft}
          latestLocalSubmission={latestSubmission}
          onCaptureStarter={onExportProjectArtifact === undefined ? undefined : async () => {
            setActiveAction("capture-assignment-starter");
            clearMessage();
            try {
              const artifact = await onExportProjectArtifact();
              setNotice("Current project captured as the assignment starter.");
              return artifact;
            } catch (reason) {
              setError(errorMessage(reason, "The current project could not be captured."));
              throw reason;
            } finally {
              setActiveAction(null);
            }
          }}
          onCloseDraft={() => setAssignmentDraft(null)}
          onCloneDraft={() => {
            setAssignmentDraft(cloneAssignmentAsDraft(currentAssignmentForTeacher));
            setNotice("A new draft was cloned with its own assignment and grading IDs.");
            setError(null);
          }}
          onCreateDraft={() => {
            setAssignmentDraft(createBlankAssignmentDraft());
            setNotice("New assignment draft ready to edit.");
            setError(null);
          }}
          onDraftChange={setAssignmentDraft}
          onExportDraft={() => {
            if (assignmentDraft === null) return;
            try {
              const validDraft = parseAssignmentDocument(assignmentDraft);
              downloadEducationDocument(
                validDraft,
                educationFileName(`${validDraft.title} draft`, ASSIGNMENT_FILE_EXTENSION),
              );
              setNotice("Validated draft assignment downloaded.");
              setError(null);
            } catch (reason) {
              setError(errorMessage(reason, "Fix the highlighted assignment issues before exporting."));
            }
          }}
          onExportAssignment={() => downloadEducationDocument(
            currentAssignmentForTeacher,
            educationFileName(currentAssignmentForTeacher.title, ASSIGNMENT_FILE_EXTENSION),
          )}
          onExportReviewed={() => {
            if (teacherSubmission !== null) {
              try {
                const reviewed = recordSubmissionReview(
                  teacherSubmission,
                  reviewDecision,
                  reviewFeedback,
                  reviewComments,
                );
                setTeacherSubmission(reviewed);
                downloadEducationDocument(
                  reviewed,
                  educationFileName(`${snapshot.projectName} reviewed`, SUBMISSION_FILE_EXTENSION),
                );
                setNotice("Current feedback recorded and reviewed submission downloaded for the student.");
                setError(null);
              } catch (reason) {
                setError(errorMessage(reason, "The reviewed submission could not be exported."));
              }
            }
          }}
          onImportAssignment={importAssignment}
          onImportSubmission={importTeacherSubmission}
          onOpenProject={onOpenSubmittedProject === undefined || teacherSubmission === null
            ? undefined
            : async () => {
                setActiveAction("open-submitted-project");
                clearMessage();
                try {
                  await onOpenSubmittedProject(teacherSubmission.project);
                  setNotice("Submitted project opened for inspection.");
                } catch (reason) {
                  setError(errorMessage(reason, "The submitted project could not be opened."));
                } finally {
                  setActiveAction(null);
                }
              }}
          onPublishDraft={() => {
            if (assignmentDraft === null) return;
            try {
              const published = publishAssignmentDraft(assignmentDraft);
              activateAssignment(published);
              downloadEducationDocument(
                published,
                educationFileName(published.title, ASSIGNMENT_FILE_EXTENSION),
              );
              setAssignmentDraft(null);
              setNotice("Assignment validated, published, and downloaded for students.");
            } catch (reason) {
              setError(errorMessage(reason, "Fix the highlighted assignment issues before publishing."));
            }
          }}
          onRecordReview={recordReview}
          onReviseDraft={() => {
            setAssignmentDraft(reviseAssignmentAsDraft(currentAssignmentForTeacher));
            setNotice("Revision draft opened with stable assignment and grading IDs.");
            setError(null);
          }}
          onUseLocalSubmission={() => {
            if (latestSubmission !== null) {
              setTeacherSubmission(latestSubmission);
              setReviewFeedback("");
              setReviewDecision("complete");
              setReviewComments([]);
              setNotice("Latest local submission opened for review.");
            }
          }}
          reviewDecision={reviewDecision}
          reviewFeedback={reviewFeedback}
          reviewComments={reviewComments}
          setReviewDecision={setReviewDecision}
          setReviewFeedback={setReviewFeedback}
          setReviewComments={setReviewComments}
          submission={teacherSubmission}
          submissionInputRef={submissionInput}
        />
      )}
    </section>
  );
};

const AssignmentLibrary = ({
  disabled,
  inputRef,
  onImport,
  onOpenBuiltIn,
}: Readonly<{
  disabled: boolean;
  inputRef: React.RefObject<HTMLInputElement | null>;
  onImport: (file: File) => Promise<void>;
  onOpenBuiltIn: () => void;
}>): React.JSX.Element => (
  <div className="education-library">
    <header>
      <p className="education-workspace__eyebrow">Choose an assignment</p>
      <h2>Start with a real PLC task</h2>
      <p>Assignments and project work stay on this computer unless you export a file.</p>
    </header>
    <article className="education-assignment-card">
      <div className="education-assignment-card__index" aria-hidden="true">01</div>
      <div>
        <span className="education-status" data-tone="published">Built in · Published</span>
        <h3>{BUILT_IN_MOTOR_STARTER_ASSIGNMENT.title}</h3>
        <p>{BUILT_IN_MOTOR_STARTER_ASSIGNMENT.summary}</p>
        <ul>
          <li>{BUILT_IN_MOTOR_STARTER_ASSIGNMENT.learningObjectives.length} learning objectives</li>
          <li>{BUILT_IN_MOTOR_STARTER_ASSIGNMENT.behaviorTests.length} behavior checks</li>
          <li>{BUILT_IN_MOTOR_STARTER_ASSIGNMENT.hintPolicy.hints.length} staged hints</li>
        </ul>
      </div>
      <button className="education-button education-button--primary" disabled={disabled} onClick={onOpenBuiltIn} type="button">
        Open mission <span aria-hidden="true">→</span>
      </button>
    </article>
    <div className="education-file-action">
      <div><strong>Assignment from your teacher</strong><span>Open a local {ASSIGNMENT_FILE_EXTENSION} file.</span></div>
      <button className="education-button" disabled={disabled} onClick={() => inputRef.current?.click()} type="button">
        Import assignment
      </button>
      <input
        accept={ASSIGNMENT_FILE_EXTENSION}
        className="visually-hidden"
        onChange={(event) => {
          const file = event.target.files?.[0];
          if (file !== undefined) void onImport(file);
          event.target.value = "";
        }}
        ref={inputRef}
        type="file"
      />
    </div>
  </div>
);

type StudentMissionProps = Readonly<{
  assignment: AssignmentDocumentV1;
  behaviorAttempts: readonly BehaviorTestResultV1[];
  compileAttemptCount: number | undefined;
  disabled: boolean;
  hintUsage: readonly HintUsageV1[];
  latestResultById: ReadonlyMap<string, BehaviorTestResultV1>;
  latestSubmission: SubmissionDocumentV1 | null;
  nextHint: ReturnType<typeof projectNextProgressiveHint> | null;
  onBack: () => void;
  onExportSubmission: () => Promise<void>;
  onImportReviewedSubmission: (file: File) => Promise<void>;
  onLoadStarter: (() => Promise<void>) | undefined;
  onRevealHint: () => void;
  onRunAll: () => void;
  onRunTest: (test: BehaviorTestDefinitionV1) => void;
  projectHash: string;
  projectRequirementInspection: ReturnType<typeof inspectProjectRequirements> | null;
  resultsProjectHash: string | null;
  reviewedSubmission: SubmissionDocumentV1 | null;
  reviewedSubmissionInputRef: React.RefObject<HTMLInputElement | null>;
  runtimeBridgeAvailable: boolean;
  runtimeReadiness: ReturnType<typeof inspectEducationRuntimeReadiness>;
  snapshot: EducationWorkspaceProps["snapshot"];
  submissionReadiness: ReturnType<typeof inspectSubmissionReadiness> | null;
}>;

const StudentMission = ({
  assignment,
  behaviorAttempts,
  compileAttemptCount,
  disabled,
  hintUsage,
  latestResultById,
  latestSubmission,
  nextHint,
  onBack,
  onExportSubmission,
  onImportReviewedSubmission,
  onLoadStarter,
  onRevealHint,
  onRunAll,
  onRunTest,
  projectHash,
  projectRequirementInspection,
  resultsProjectHash,
  reviewedSubmission,
  reviewedSubmissionInputRef,
  runtimeBridgeAvailable,
  runtimeReadiness,
  snapshot,
  submissionReadiness,
}: StudentMissionProps): React.JSX.Element => {
  const passedCount = assignment.behaviorTests.filter((test) => latestResultById.get(test.testId)?.status === "passed").length;
  const visibleTests = assignment.behaviorTests.filter((test) => test.visibility === "student");
  const usedHints = assignment.hintPolicy.hints.filter((hint) => hintUsage.some((entry) => entry.hintId === hint.hintId));
  const resultsStale = resultsProjectHash !== null && resultsProjectHash !== projectHash;
  return (
    <div className="student-mission">
      <div className="education-breadcrumb">
        <button disabled={disabled} onClick={onBack} type="button">Assignments</button>
        <span aria-hidden="true">/</span><span>Mission</span>
      </div>
      <header className="student-mission__header">
        <div>
          <span className="education-status" data-tone="published">Assignment r{assignment.revision}</span>
          <h2>{assignment.title}</h2>
          <p>{assignment.summary}</p>
        </div>
        <div className="student-mission__next">
          <span>Current project</span>
          <strong>{snapshot.projectName}</strong>
          <small>Build state: {snapshot.buildState.replace("-", " ")}</small>
        </div>
      </header>

      <div className="education-progress" aria-label="Assignment progress">
        <ProgressMetric label="Compile attempts" value={compileAttemptCount === undefined ? "Not tracked" : String(compileAttemptCount)} tone={snapshot.buildState === "current" && compileAttemptCount !== undefined ? "good" : "neutral"} />
        <ProgressMetric label="Behavior checks" value={`${passedCount}/${assignment.behaviorTests.length}`} tone={passedCount === assignment.behaviorTests.length ? "good" : "neutral"} />
        <ProgressMetric label="Hints used" value={`${hintUsage.length}/${assignment.hintPolicy.hints.length}`} tone="neutral" />
        <ProgressMetric label="Submission" value={latestSubmission === null ? "Not submitted" : "Exported"} tone={latestSubmission === null ? "neutral" : "good"} />
      </div>

      <div className="student-mission__grid">
        <main className="student-mission__main">
          <section className="education-panel" aria-labelledby="mission-objectives">
            <div className="education-panel__heading">
              <div><span>What you will prove</span><h3 id="mission-objectives">Mission objectives</h3></div>
              {onLoadStarter === undefined ? (
                <span className="education-availability">Starter loading not connected</span>
              ) : (
                <button className="education-button education-button--quiet" disabled={disabled} onClick={() => void onLoadStarter()} type="button">
                  Load starter project
                </button>
              )}
            </div>
            <ol className="education-objectives">
              {assignment.learningObjectives.map((objective, index) => (
                <li key={objective}><span>{String(index + 1).padStart(2, "0")}</span><p>{objective}</p></li>
              ))}
            </ol>
          </section>

          <section className="education-panel" aria-labelledby="behavior-checks">
            <div className="education-panel__heading">
              <div><span>Real runtime evidence</span><h3 id="behavior-checks">Behavior checks</h3></div>
              <button
                className="education-button education-button--primary"
                disabled={disabled || !runtimeBridgeAvailable || !runtimeReadiness.ready}
                onClick={onRunAll}
                type="button"
              >Run all checks</button>
            </div>
            {!runtimeBridgeAvailable || !runtimeReadiness.ready ? (
              <p className="education-inline-note" role="note">
                {!runtimeBridgeAvailable
                  ? "Runtime command and reset callbacks must be connected before grading can run."
                  : runtimeReadiness.reason}
              </p>
            ) : (
              <p className="education-inline-note education-inline-note--ready" role="status">{runtimeReadiness.reason}</p>
            )}
            {resultsStale && (
              <p className="education-inline-note education-inline-note--warning" role="status">
                The project changed after the last check. Run the checks again before submitting.
              </p>
            )}
            <div className="education-check-list">
              {visibleTests.map((test, index) => {
                const result = latestResultById.get(test.testId);
                return (
                  <article className="education-check" data-status={result?.status ?? "not-run"} key={test.testId}>
                    <span className="education-check__index">{String(index + 1).padStart(2, "0")}</span>
                    <div>
                      <h4>{test.title}</h4><p>{test.description}</p>
                      <BehaviorResultSummary result={result} />
                    </div>
                    <button className="education-button" disabled={disabled || !runtimeBridgeAvailable || !runtimeReadiness.ready} onClick={() => onRunTest(test)} type="button">
                      {result === undefined ? "Run check" : "Run again"}
                    </button>
                  </article>
                );
              })}
              {assignment.behaviorTests.length > visibleTests.length && (
                <p className="education-inline-note">
                  {assignment.behaviorTests.length - visibleTests.length} teacher-only check will run without revealing its expected outcome.
                </p>
              )}
            </div>
          </section>
        </main>

        <aside className="student-mission__aside">
          <section className="education-panel education-panel--compact" aria-labelledby="mission-constraints">
            <div className="education-panel__heading"><div><span>Teacher settings</span><h3 id="mission-constraints">Lab constraints</h3></div></div>
            <dl className="education-constraint-list">
              <div><dt>Virtual PLCs</dt><dd>{assignment.requirements.plcCatalogIds.join(", ")}</dd></div>
              <div><dt>Modules</dt><dd>{assignment.requirements.requiredModules.map((module) => `${module.quantity}× ${module.catalogId}`).join(", ")}</dd></div>
              <div><dt>LAD instructions</dt><dd>{assignment.requirements.permittedLadInstructions.map(humanizeToken).join(", ")}</dd></div>
            </dl>
            <details>
              <summary>Starter tags ({assignment.requirements.starterTags.length})</summary>
              <div className="education-tag-list">
                {assignment.requirements.starterTags.map((tag) => (
                  <div key={tag.name}><strong>{tag.name}</strong><code>{tag.address ?? "Automatic"}</code><span>{tag.description}</span></div>
                ))}
              </div>
            </details>
            {projectRequirementInspection?.ready === true ? (
              <p className="education-inline-note education-inline-note--ready" role="status">
                Current hardware, tags, and LAD instructions match this assignment.
              </p>
            ) : (
              <div className="education-inline-note education-inline-note--warning" role="note">
                <strong>Project requirements still to fix</strong>
                <ul className="education-readiness-list">
                  {projectRequirementInspection?.issues.map((issue, index) => (
                    <li key={`${issue.code}-${index}`}>{issue.message}</li>
                  ))}
                </ul>
              </div>
            )}
          </section>

          <section className="education-panel education-panel--compact" aria-labelledby="mission-hints">
            <div className="education-panel__heading">
              <div><span>Use only when needed</span><h3 id="mission-hints">Progressive hints</h3></div>
              <strong className="education-count">{hintUsage.length}/{assignment.hintPolicy.hints.length}</strong>
            </div>
            {usedHints.length === 0 ? <p className="education-muted">No hints used yet.</p> : (
              <ol className="education-hints">
                {usedHints.map((hint) => <li key={hint.hintId}><strong>{hint.title}</strong><p>{hint.body}</p></li>)}
              </ol>
            )}
            <button className="education-button education-button--wide" disabled={disabled || nextHint?.hint === null} onClick={onRevealHint} type="button">
              Reveal next hint
            </button>
            <p className="education-muted">{nextHint?.reason}</p>
            <small>{behaviorAttempts.filter((result) => result.status === "failed").length} failed behavior attempts recorded.</small>
          </section>

          <section className="education-panel education-panel--submission" aria-labelledby="mission-submit">
            <div className="education-panel__heading"><div><span>Turn in your work</span><h3 id="mission-submit">Submission</h3></div></div>
            {submissionReadiness?.ready === true ? (
              <p className="education-inline-note education-inline-note--ready">The compiled project and all behavior evidence are ready.</p>
            ) : (
              <ul className="education-readiness-list">
                {submissionReadiness?.reasons.map((reason) => <li key={reason}>{reason}</li>)}
              </ul>
            )}
            <button className="education-button education-button--primary education-button--wide" disabled={disabled || submissionReadiness?.ready !== true} onClick={() => void onExportSubmission()} type="button">
              Create &amp; export submission
            </button>
            <small>Exports one local {SUBMISSION_FILE_EXTENSION} file. No network connection is used.</small>
          </section>

          <section className="education-panel education-panel--feedback" aria-labelledby="mission-feedback">
            <div className="education-panel__heading">
              <div><span>Returned by your teacher</span><h3 id="mission-feedback">Review feedback</h3></div>
              {reviewedSubmission !== null && (
                <span className="education-status" data-tone={reviewedSubmission.lifecycle === "reviewed" ? "published" : "draft"}>
                  {humanizeToken(reviewedSubmission.lifecycle)}
                </span>
              )}
            </div>
            <input
              accept={SUBMISSION_FILE_EXTENSION}
              className="visually-hidden"
              onChange={(event) => {
                const file = event.target.files?.[0];
                if (file !== undefined) void onImportReviewedSubmission(file);
                event.target.value = "";
              }}
              ref={reviewedSubmissionInputRef}
              type="file"
            />
            {reviewedSubmission === null ? (
              <div className="education-feedback-empty">
                <p>Open a reviewed submission file to read teacher feedback for this exact assignment revision.</p>
                <button className="education-button education-button--wide" disabled={disabled} onClick={() => reviewedSubmissionInputRef.current?.click()} type="button">
                  Import reviewed submission
                </button>
              </div>
            ) : (
              <div className="student-review-feedback">
                <dl>
                  <div><dt>Attempt</dt><dd>#{reviewedSubmission.attemptOrdinal}</dd></div>
                  <div><dt>Lifecycle</dt><dd>{humanizeToken(reviewedSubmission.lifecycle)}</dd></div>
                  <div><dt>Decision</dt><dd>{humanizeToken(reviewedSubmission.review?.decision ?? "pending")}</dd></div>
                </dl>
                <div className="student-review-feedback__general">
                  <span>Teacher feedback</span>
                  <p>{reviewedSubmission.review?.feedback}</p>
                </div>
                {(reviewedSubmission.review?.comments.length ?? 0) > 0 && (
                  <ol aria-label="Teacher object and rung comments" className="student-review-comments">
                    {reviewedSubmission.review?.comments.map((comment) => (
                      <li key={comment.commentId}>
                        <span>{comment.objectId === null ? "General project comment" : "Object or rung"}</span>
                        {comment.objectId !== null && <code>{comment.objectId}</code>}
                        <p>{comment.body}</p>
                      </li>
                    ))}
                  </ol>
                )}
                <button className="education-button education-button--wide" disabled={disabled} onClick={() => reviewedSubmissionInputRef.current?.click()} type="button">
                  Import different feedback
                </button>
              </div>
            )}
            <small>Feedback files are read-only here. Importing one does not create a new attempt or replace your project.</small>
          </section>
        </aside>
      </div>
    </div>
  );
};

type TeacherPortalProps = Readonly<{
  assignment: AssignmentDocumentV1;
  assignmentInputRef: React.RefObject<HTMLInputElement | null>;
  disabled: boolean;
  draft: AssignmentDocumentV1 | null;
  latestLocalSubmission: SubmissionDocumentV1 | null;
  onCaptureStarter: (() => Promise<ProjectArtifactV1>) | undefined;
  onCloseDraft: () => void;
  onCloneDraft: () => void;
  onCreateDraft: () => void;
  onDraftChange: (draft: AssignmentDocumentV1) => void;
  onExportDraft: () => void;
  onExportAssignment: () => void;
  onExportReviewed: () => void;
  onImportAssignment: (file: File) => Promise<void>;
  onImportSubmission: (file: File) => Promise<void>;
  onOpenProject: (() => Promise<void>) | undefined;
  onPublishDraft: () => void;
  onRecordReview: () => void;
  onReviseDraft: () => void;
  onUseLocalSubmission: () => void;
  reviewDecision: ReviewDecision;
  reviewFeedback: string;
  reviewComments: readonly SubmissionReviewCommentV1[];
  setReviewDecision: (decision: ReviewDecision) => void;
  setReviewFeedback: (feedback: string) => void;
  setReviewComments: (comments: readonly SubmissionReviewCommentV1[]) => void;
  submission: SubmissionDocumentV1 | null;
  submissionInputRef: React.RefObject<HTMLInputElement | null>;
}>;

const TeacherPortal = ({
  assignment,
  assignmentInputRef,
  disabled,
  draft,
  latestLocalSubmission,
  onCaptureStarter,
  onCloseDraft,
  onCloneDraft,
  onCreateDraft,
  onDraftChange,
  onExportDraft,
  onExportAssignment,
  onExportReviewed,
  onImportAssignment,
  onImportSubmission,
  onOpenProject,
  onPublishDraft,
  onRecordReview,
  onReviseDraft,
  onUseLocalSubmission,
  reviewDecision,
  reviewFeedback,
  reviewComments,
  setReviewDecision,
  setReviewFeedback,
  setReviewComments,
  submission,
  submissionInputRef,
}: TeacherPortalProps): React.JSX.Element => (
  <div className="teacher-portal">
    <header className="teacher-portal__header">
      <div>
        <span className="education-status" data-tone="published">{assignment.lifecycle} · revision {assignment.revision}</span>
        <h2>{assignment.title}</h2><p>{assignment.summary}</p>
      </div>
      <div className="teacher-portal__actions">
        <button className="education-button education-button--quiet" disabled={disabled} onClick={onCreateDraft} type="button">New draft</button>
        <button className="education-button education-button--quiet" disabled={disabled} onClick={onCloneDraft} type="button">Clone</button>
        <button className="education-button education-button--quiet" disabled={disabled} onClick={onReviseDraft} type="button">Revise</button>
        <button className="education-button" disabled={disabled} onClick={() => assignmentInputRef.current?.click()} type="button">Import assignment</button>
        <button className="education-button education-button--primary" disabled={disabled} onClick={onExportAssignment} type="button">Export assignment</button>
        <input
          accept={ASSIGNMENT_FILE_EXTENSION}
          className="visually-hidden"
          onChange={(event) => {
            const file = event.target.files?.[0];
            if (file !== undefined) void onImportAssignment(file);
            event.target.value = "";
          }}
          ref={assignmentInputRef}
          type="file"
        />
      </div>
    </header>

    {draft !== null && (
      <TeacherAssignmentStudio
        disabled={disabled}
        draft={draft}
        onCaptureStarter={onCaptureStarter}
        onChange={onDraftChange}
        onClose={onCloseDraft}
        onExport={onExportDraft}
        onPublish={onPublishDraft}
      />
    )}

    <div className="teacher-portal__grid">
      <main>
        <section className="education-panel" aria-labelledby="teacher-tests">
          <div className="education-panel__heading"><div><span>Deterministic grading plan</span><h3 id="teacher-tests">Behavior tests</h3></div><strong className="education-count">{assignment.behaviorTests.length}</strong></div>
          <div className="teacher-test-table" role="table" aria-label="Assignment behavior tests">
            <div className="teacher-test-table__head" role="row"><span>Test</span><span>Visibility</span><span>Steps</span></div>
            {assignment.behaviorTests.map((test) => (
              <details key={test.testId}>
                <summary role="row"><span><strong>{test.title}</strong><small>{test.description}</small></span><span>{test.visibility}</span><span>{test.steps.length}</span></summary>
                <ol>
                  {test.steps.map((step) => <li key={step.stepId}><code>{step.kind}</code><span>{teacherStepSummary(step)}</span></li>)}
                </ol>
              </details>
            ))}
          </div>
        </section>

        <section className="education-panel" aria-labelledby="teacher-review">
          <div className="education-panel__heading">
            <div><span>Offline hand-in</span><h3 id="teacher-review">Submission review</h3></div>
            <div className="education-panel__actions">
              {latestLocalSubmission !== null && <button className="education-button education-button--quiet" disabled={disabled} onClick={onUseLocalSubmission} type="button">Use latest local</button>}
              <button className="education-button" disabled={disabled} onClick={() => submissionInputRef.current?.click()} type="button">Import submission</button>
            </div>
          </div>
          <input
            accept={SUBMISSION_FILE_EXTENSION}
            className="visually-hidden"
            onChange={(event) => {
              const file = event.target.files?.[0];
              if (file !== undefined) void onImportSubmission(file);
              event.target.value = "";
            }}
            ref={submissionInputRef}
            type="file"
          />
          {submission === null ? (
            <div className="education-empty-state">
              <strong>No submission open</strong>
              <p>Import a {SUBMISSION_FILE_EXTENSION} file to inspect compile evidence, test results, hints used, and the submitted project.</p>
            </div>
          ) : (
            <div className="teacher-review">
              <SubmissionEvidence assignment={assignment} submission={submission} />
              <div className="teacher-project-artifact">
                <div><span>Submitted project</span><strong>{submission.project.fileName}</strong><code>{submission.project.sha256Hex.slice(0, 16)}…</code></div>
                <button className="education-button" disabled={disabled || onOpenProject === undefined} onClick={() => void onOpenProject?.()} type="button">Open submitted project</button>
                {onOpenProject === undefined && <small>Project opening is not connected in this host.</small>}
              </div>
              <form
                className="teacher-feedback"
                onSubmit={(event) => { event.preventDefault(); onRecordReview(); }}
              >
                <label htmlFor="teacher-review-decision">Review decision</label>
                <select id="teacher-review-decision" onChange={(event) => setReviewDecision(event.target.value as ReviewDecision)} value={reviewDecision}>
                  <option value="complete">Complete</option>
                  <option value="revision-requested">Return for revision</option>
                </select>
                <label htmlFor="teacher-review-feedback">Feedback for the student</label>
                <textarea id="teacher-review-feedback" maxLength={8000} onChange={(event) => setReviewFeedback(event.target.value)} rows={5} value={reviewFeedback} />
                <SubmissionReviewCommentEditor
                  comments={reviewComments}
                  disabled={disabled}
                  onChange={setReviewComments}
                />
                <div>
                  <button className="education-button education-button--primary" disabled={disabled || reviewFeedback.trim().length === 0} type="submit">Record feedback locally</button>
                  <button className="education-button" disabled={disabled || reviewFeedback.trim().length === 0} onClick={onExportReviewed} type="button">Export reviewed submission</button>
                </div>
                <small>Feedback remains in this session until you export the reviewed file.</small>
              </form>
            </div>
          )}
        </section>
      </main>

      <aside>
        <section className="education-panel education-panel--compact" aria-labelledby="teacher-hints">
          <div className="education-panel__heading"><div><span>Student-controlled reveal</span><h3 id="teacher-hints">Hint policy</h3></div></div>
          <p className="education-muted">Mode: {assignment.hintPolicy.mode}; reveal: {assignment.hintPolicy.reveal}.</p>
          <ol className="teacher-hint-policy">
            {assignment.hintPolicy.hints.map((hint) => (
              <li key={hint.hintId}><span>{hint.order}</span><div><strong>{hint.title}</strong><p>{hint.body}</p><small>{hintUnlockSummary(hint.unlock)}</small></div></li>
            ))}
          </ol>
        </section>
        <section className="education-panel education-panel--compact" aria-labelledby="teacher-requirements">
          <div className="education-panel__heading"><div><span>Assignment boundary</span><h3 id="teacher-requirements">Required setup</h3></div></div>
          <dl className="education-constraint-list">
            <div><dt>PLC choices</dt><dd>{assignment.requirements.plcCatalogIds.join(", ")}</dd></div>
            <div><dt>Digital modules</dt><dd>{assignment.requirements.requiredModules.map((module) => `${module.quantity}× ${module.catalogId}`).join(", ")}</dd></div>
            <div><dt>Starter tags</dt><dd>{assignment.requirements.starterTags.map((tag) => tag.name).join(", ")}</dd></div>
            <div><dt>Instructions</dt><dd>{assignment.requirements.permittedLadInstructions.map(humanizeToken).join(", ")}</dd></div>
          </dl>
        </section>
      </aside>
    </div>
  </div>
);

const CANONICAL_UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;

export const SubmissionReviewCommentEditor = ({
  comments,
  disabled,
  onChange,
}: Readonly<{
  comments: readonly SubmissionReviewCommentV1[];
  disabled: boolean;
  onChange: (comments: readonly SubmissionReviewCommentV1[]) => void;
}>): React.JSX.Element => {
  const [body, setBody] = useState("");
  const [objectId, setObjectId] = useState("");
  const normalizedObjectId = objectId.trim();
  const targetValid = normalizedObjectId.length === 0 || CANONICAL_UUID.test(normalizedObjectId);
  const addComment = (): void => {
    if (body.trim().length === 0 || !targetValid) return;
    onChange([...comments, createSubmissionReviewComment(body, normalizedObjectId || null)]);
    setBody("");
    setObjectId("");
  };

  return (
    <fieldset className="teacher-review-comments" disabled={disabled}>
      <legend>Object and rung comments</legend>
      <p>Attach focused guidance to a project object or rung UUID, or leave the target blank for a project-wide comment.</p>
      {comments.length > 0 && (
        <ol aria-label="Review comments">
          {comments.map((comment, index) => {
            const existingTargetValid = comment.objectId === null || CANONICAL_UUID.test(comment.objectId);
            return (
              <li key={comment.commentId}>
                <div>
                  <label htmlFor={`teacher-comment-target-${comment.commentId}`}>Object or rung ID</label>
                  <input
                    aria-invalid={!existingTargetValid}
                    id={`teacher-comment-target-${comment.commentId}`}
                    onChange={(event) => onChange(comments.map((current) => current.commentId === comment.commentId
                      ? { ...current, objectId: event.target.value.trim().length === 0 ? null : event.target.value }
                      : current))}
                    placeholder="Optional UUID"
                    value={comment.objectId ?? ""}
                  />
                </div>
                <div>
                  <label htmlFor={`teacher-comment-body-${comment.commentId}`}>Comment {index + 1}</label>
                  <textarea
                    id={`teacher-comment-body-${comment.commentId}`}
                    maxLength={8000}
                    onChange={(event) => onChange(comments.map((current) => current.commentId === comment.commentId
                      ? { ...current, body: event.target.value }
                      : current))}
                    rows={2}
                    value={comment.body}
                  />
                </div>
                <button
                  aria-label={`Remove review comment ${index + 1}`}
                  className="education-button education-button--quiet"
                  onClick={() => onChange(comments.filter((current) => current.commentId !== comment.commentId))}
                  type="button"
                >Remove</button>
                <code title={comment.commentId}>ID {comment.commentId.slice(0, 8)}</code>
              </li>
            );
          })}
        </ol>
      )}
      <div className="teacher-review-comments__new">
        <label htmlFor="teacher-new-comment-target">Object or rung ID <span>(optional)</span></label>
        <input
          aria-describedby={!targetValid ? "teacher-new-comment-target-error" : undefined}
          aria-invalid={!targetValid}
          id="teacher-new-comment-target"
          onChange={(event) => setObjectId(event.target.value)}
          placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
          value={objectId}
        />
        {!targetValid && <small id="teacher-new-comment-target-error">Use a canonical lowercase UUID, or leave this blank.</small>}
        <label htmlFor="teacher-new-comment-body">New focused comment</label>
        <textarea id="teacher-new-comment-body" maxLength={8000} onChange={(event) => setBody(event.target.value)} rows={2} value={body} />
        <button className="education-button" disabled={body.trim().length === 0 || !targetValid} onClick={addComment} type="button">Add comment</button>
      </div>
    </fieldset>
  );
};

const PLC_CATALOG_OPTIONS = controllerCatalogs.map((catalog) => ({
  id: catalog.catalogId,
  label: catalog.displayName,
}));

const DIGITAL_MODULE_OPTIONS = digitalModuleCatalogs.map((catalog) => ({
  id: catalog.catalogId,
  label: catalog.displayName,
}));

const MVP_LAD_INSTRUCTION_OPTIONS = [
  ["contact.no", "Normally open contact"],
  ["contact.nc", "Normally closed contact"],
  ["coil.normal", "Output coil"],
  ["coil.set", "Set coil"],
  ["coil.reset", "Reset coil"],
  ...MVP_LAD_INSTRUCTION_CATALOG.map((instruction) => [instruction.key, instruction.learning.title] as const),
] as const;
const KNOWN_PLC_CATALOG_IDS = new Set<string>(PLC_CATALOG_OPTIONS.map((option) => option.id));
const KNOWN_DIGITAL_MODULE_IDS = new Set<string>(DIGITAL_MODULE_OPTIONS.map((option) => option.id));
const KNOWN_MVP_LAD_INSTRUCTIONS = new Set<string>(MVP_LAD_INSTRUCTION_OPTIONS.map(([id]) => id));

type TeacherAssignmentStudioProps = Readonly<{
  disabled: boolean;
  draft: AssignmentDocumentV1;
  onCaptureStarter: (() => Promise<ProjectArtifactV1>) | undefined;
  onChange: (draft: AssignmentDocumentV1) => void;
  onClose: () => void;
  onExport: () => void;
  onPublish: () => void;
}>;

export const TeacherAssignmentStudio = ({
  disabled,
  draft,
  onCaptureStarter,
  onChange,
  onClose,
  onExport,
  onPublish,
}: TeacherAssignmentStudioProps): React.JSX.Element => {
  const validation = inspectAssignmentAuthoring(draft);
  const issueCount = validation.ok ? 0 : validation.issues.length;
  const updateRequirements = (requirements: AssignmentDocumentV1["requirements"]): void => {
    onChange({ ...draft, requirements });
  };
  const updateTest = (index: number, test: BehaviorTestDefinitionV1): void => {
    onChange({
      ...draft,
      behaviorTests: draft.behaviorTests.map((current, currentIndex) => currentIndex === index ? test : current),
    });
  };
  const updateHint = (index: number, hint: ProgressiveHintV1): void => {
    onChange({
      ...draft,
      hintPolicy: {
        ...draft.hintPolicy,
        hints: draft.hintPolicy.hints.map((current, currentIndex) => currentIndex === index ? hint : current),
      },
    });
  };

  return (
    <section aria-labelledby="teacher-authoring-title" className="teacher-authoring">
      <header className="teacher-authoring__masthead">
        <div>
          <span>Assignment authoring · draft revision {draft.revision}</span>
          <h3 id="teacher-authoring-title">Build a student mission</h3>
          <p>Use the guided fields below. IDs stay stable as you edit, and the same strict format validator guards every export.</p>
        </div>
        <div className="teacher-authoring__actions">
          <button className="education-button education-button--quiet" disabled={disabled} onClick={onClose} type="button">Close editor</button>
          <button className="education-button" disabled={disabled || !validation.ok} onClick={onExport} type="button">Export draft</button>
          <button className="education-button education-button--primary" disabled={disabled || !validation.ok} onClick={onPublish} type="button">Publish &amp; export</button>
        </div>
      </header>

      <div className="teacher-authoring__validation" data-valid={validation.ok} role="status">
        <div>
          <strong>{validation.ok ? "Ready to export" : `${issueCount} ${issueCount === 1 ? "issue" : "issues"} to fix`}</strong>
          <span>{validation.ok ? "This draft passes the complete assignment contract." : "Keep editing; export stays locked until the document is valid."}</span>
        </div>
        {!validation.ok && (
          <ul>
            {validation.issues.slice(0, 8).map((issue, index) => (
              <li key={`${issue.path}-${index}`}><code>{friendlyIssuePath(issue.path)}</code>{issue.message}</li>
            ))}
            {validation.issues.length > 8 && <li>And {validation.issues.length - 8} more issues.</li>}
          </ul>
        )}
      </div>

      <fieldset className="teacher-authoring__layout" disabled={disabled}>
        <main>
          <AuthoringSection eyebrow="Mission brief" title="Title, summary & objectives">
            <div className="authoring-fields authoring-fields--two">
              <AuthoringField label="Assignment title">
                <input maxLength={160} onChange={(event) => onChange({ ...draft, title: event.target.value })} value={draft.title} />
              </AuthoringField>
              <div className="authoring-id"><span>Stable assignment ID</span><code>{draft.assignmentId}</code></div>
              <AuthoringField className="authoring-field--wide" label="Student-facing summary">
                <textarea maxLength={8000} onChange={(event) => onChange({ ...draft, summary: event.target.value })} rows={3} value={draft.summary} />
              </AuthoringField>
            </div>
            <div className="authoring-repeat-list">
              {draft.learningObjectives.map((objective, index) => (
                <div className="authoring-repeat-row" key={`objective-${index}`}>
                  <span>{String(index + 1).padStart(2, "0")}</span>
                  <input
                    aria-label={`Learning objective ${index + 1}`}
                    maxLength={8000}
                    onChange={(event) => onChange({
                      ...draft,
                      learningObjectives: draft.learningObjectives.map((current, currentIndex) => currentIndex === index ? event.target.value : current),
                    })}
                    value={objective}
                  />
                  <button
                    aria-label={`Remove learning objective ${index + 1}`}
                    className="authoring-icon-button"
                    disabled={disabled}
                    onClick={() => onChange({ ...draft, learningObjectives: draft.learningObjectives.filter((_, currentIndex) => currentIndex !== index) })}
                    type="button"
                  >×</button>
                </div>
              ))}
              <button className="education-button education-button--quiet" disabled={disabled} onClick={() => onChange({ ...draft, learningObjectives: [...draft.learningObjectives, "New learning objective"] })} type="button">+ Add objective</button>
            </div>
          </AuthoringSection>

          <AuthoringSection eyebrow="Starting point" title="Starter project & hardware boundary">
            <div className="authoring-fields authoring-fields--three">
              <AuthoringField label="Starter mode">
                <select
                  onChange={(event) => {
                    if (event.target.value === "blank") onChange({ ...draft, starterProject: { kind: "blank" } });
                    if (event.target.value === "built-in-template") onChange({ ...draft, starterProject: { kind: "built-in-template", templateId: "builtin.virtual-plc-blank/1" } });
                  }}
                  value={draft.starterProject.kind}
                >
                  <option value="blank">Blank project</option>
                  <option value="built-in-template">Built-in virtual PLC template</option>
                  <option disabled={draft.starterProject.kind !== "embedded-project"} value="embedded-project">Captured project</option>
                </select>
              </AuthoringField>
              {draft.starterProject.kind === "built-in-template" && (
                <AuthoringField label="Template ID">
                  <input maxLength={160} onChange={(event) => onChange({ ...draft, starterProject: { kind: "built-in-template", templateId: event.target.value } })} value={draft.starterProject.templateId} />
                </AuthoringField>
              )}
              <div className="authoring-capture">
                <button
                  className="education-button"
                  disabled={disabled || onCaptureStarter === undefined}
                  onClick={() => {
                    if (onCaptureStarter === undefined) return;
                    void onCaptureStarter()
                      .then((artifact) => onChange({ ...draft, starterProject: { artifact, kind: "embedded-project" } }))
                      .catch(() => undefined);
                  }}
                  type="button"
                >Capture current project</button>
                <small>{onCaptureStarter === undefined ? "Project packaging is not connected." : draft.starterProject.kind === "embedded-project" ? draft.starterProject.artifact.fileName : "Embeds the current local project into the assignment."}</small>
              </div>
            </div>

            <div className="authoring-subsection">
              <h4>Allowed virtual PLCs</h4>
              <div className="authoring-choice-grid">
                {PLC_CATALOG_OPTIONS.map((option) => (
                  <label key={option.id}>
                    <input
                      checked={draft.requirements.plcCatalogIds.includes(option.id)}
                      onChange={(event) => updateRequirements({
                        ...draft.requirements,
                        plcCatalogIds: event.target.checked
                          ? [...draft.requirements.plcCatalogIds, option.id]
                          : draft.requirements.plcCatalogIds.filter((id) => id !== option.id),
                      })}
                      type="checkbox"
                    />
                    <span><strong>{option.label}</strong><code>{option.id}</code></span>
                  </label>
                ))}
              </div>
              <UnlistedTokenList
                label="Unlisted PLC IDs from the imported assignment"
                onRemove={(id) => updateRequirements({ ...draft.requirements, plcCatalogIds: draft.requirements.plcCatalogIds.filter((current) => current !== id) })}
                values={draft.requirements.plcCatalogIds.filter((id) => !KNOWN_PLC_CATALOG_IDS.has(id))}
              />
            </div>

            <div className="authoring-subsection">
              <h4>Required digital modules</h4>
              <div className="authoring-module-grid">
                {DIGITAL_MODULE_OPTIONS.map((option) => {
                  const quantity = draft.requirements.requiredModules.find((module) => module.catalogId === option.id)?.quantity ?? 0;
                  return (
                    <label key={option.id}>
                      <span><strong>{option.label}</strong><code>{option.id}</code></span>
                      <input
                        aria-label={`${option.label} quantity`}
                        max={32}
                        min={0}
                        onChange={(event) => {
                          const nextQuantity = Math.max(0, Number.parseInt(event.target.value, 10) || 0);
                          const remaining = draft.requirements.requiredModules.filter((module) => module.catalogId !== option.id);
                          updateRequirements({
                            ...draft.requirements,
                            requiredModules: nextQuantity === 0 ? remaining : [...remaining, { catalogId: option.id, quantity: nextQuantity }],
                          });
                        }}
                        type="number"
                        value={quantity}
                      />
                    </label>
                  );
                })}
              </div>
              <UnlistedTokenList
                label="Unlisted module IDs from the imported assignment"
                onRemove={(id) => updateRequirements({ ...draft.requirements, requiredModules: draft.requirements.requiredModules.filter((module) => module.catalogId !== id) })}
                values={draft.requirements.requiredModules.map((module) => module.catalogId).filter((id) => !KNOWN_DIGITAL_MODULE_IDS.has(id))}
              />
            </div>
          </AuthoringSection>

          <AuthoringSection eyebrow="Project vocabulary" title="Starter tags">
            <div className="authoring-tag-table">
              <div className="authoring-tag-table__head"><span>Name</span><span>Type</span><span>Address</span><span>Description</span><span /></div>
              {draft.requirements.starterTags.map((tag, index) => (
                <div className="authoring-tag-row" key={`starter-tag-${index}`}>
                  <input aria-label={`Starter tag ${index + 1} name`} maxLength={160} onChange={(event) => updateStarterTag(draft, index, { ...tag, name: event.target.value }, updateRequirements)} value={tag.name} />
                  <select aria-label={`Starter tag ${index + 1} data type`} onChange={(event) => updateStarterTag(draft, index, { ...tag, dataType: event.target.value as typeof tag.dataType }, updateRequirements)} value={tag.dataType}>
                    <option value="BOOL">BOOL</option><option value="DINT">DINT</option><option value="TIME">TIME</option>
                  </select>
                  <input aria-label={`Starter tag ${index + 1} address`} maxLength={160} onChange={(event) => updateStarterTag(draft, index, { ...tag, address: event.target.value.trim().length === 0 ? null : event.target.value }, updateRequirements)} placeholder="Automatic" value={tag.address ?? ""} />
                  <input aria-label={`Starter tag ${index + 1} description`} maxLength={8000} onChange={(event) => updateStarterTag(draft, index, { ...tag, description: event.target.value }, updateRequirements)} value={tag.description} />
                  <button aria-label={`Remove starter tag ${index + 1}`} className="authoring-icon-button" disabled={disabled} onClick={() => updateRequirements({ ...draft.requirements, starterTags: draft.requirements.starterTags.filter((_, currentIndex) => currentIndex !== index) })} type="button">×</button>
                </div>
              ))}
            </div>
            <button className="education-button education-button--quiet" disabled={disabled} onClick={() => updateRequirements({
              ...draft.requirements,
              starterTags: [...draft.requirements.starterTags, {
                address: null,
                dataType: "BOOL",
                description: "Student starter tag",
                name: nextStarterTagName(draft.requirements.starterTags.map((tag) => tag.name)),
              }],
            })} type="button">+ Add starter tag</button>
          </AuthoringSection>

          <AuthoringSection eyebrow="LAD guardrails" title="Permitted MVP instructions">
            <div className="authoring-instruction-grid">
              {MVP_LAD_INSTRUCTION_OPTIONS.map(([id, label]) => (
                <label key={id}>
                  <input
                    checked={draft.requirements.permittedLadInstructions.includes(id)}
                    onChange={(event) => updateRequirements({
                      ...draft.requirements,
                      permittedLadInstructions: event.target.checked
                        ? [...draft.requirements.permittedLadInstructions, id]
                        : draft.requirements.permittedLadInstructions.filter((instruction) => instruction !== id),
                    })}
                    type="checkbox"
                  />
                  <span><strong>{label}</strong><code>{id}</code></span>
                </label>
              ))}
            </div>
            <UnlistedTokenList
              label="Unlisted instructions from the imported assignment"
              onRemove={(id) => updateRequirements({ ...draft.requirements, permittedLadInstructions: draft.requirements.permittedLadInstructions.filter((instruction) => instruction !== id) })}
              values={draft.requirements.permittedLadInstructions.filter((instruction) => !KNOWN_MVP_LAD_INSTRUCTIONS.has(instruction))}
            />
          </AuthoringSection>

          <AuthoringSection eyebrow="Evidence plan" title="Behavior tests">
            <p className="authoring-section-note">Tests run against the authoritative virtual PLC in this exact order. Every test needs at least one expectation.</p>
            <div className="behavior-builder">
              {draft.behaviorTests.map((test, testIndex) => (
                <details className="behavior-builder__test" key={test.testId} open={draft.behaviorTests.length === 1}>
                  <summary><span>{String(testIndex + 1).padStart(2, "0")}</span><strong>{test.title || "Untitled behavior test"}</strong><small>{test.steps.length} steps · {test.visibility}</small></summary>
                  <div className="behavior-builder__body">
                    <div className="authoring-fields authoring-fields--three">
                      <AuthoringField label="Test title"><input maxLength={160} onChange={(event) => updateTest(testIndex, { ...test, title: event.target.value })} value={test.title} /></AuthoringField>
                      <AuthoringField label="Student visibility"><select onChange={(event) => updateTest(testIndex, { ...test, visibility: event.target.value as BehaviorTestDefinitionV1["visibility"] })} value={test.visibility}><option value="student">Student can see it</option><option value="teacher-only">Teacher-only check</option></select></AuthoringField>
                      <div className="authoring-id"><span>Stable test ID</span><code>{test.testId}</code></div>
                      <AuthoringField className="authoring-field--wide" label="What this test proves"><textarea maxLength={8000} onChange={(event) => updateTest(testIndex, { ...test, description: event.target.value })} rows={2} value={test.description} /></AuthoringField>
                    </div>
                    <div className="behavior-steps">
                      {test.steps.map((step, stepIndex) => (
                        <BehaviorStepEditor
                          disabled={disabled}
                          key={step.stepId}
                          onChange={(next) => updateTest(testIndex, { ...test, steps: test.steps.map((current, currentIndex) => currentIndex === stepIndex ? next : current) })}
                          onMoveDown={stepIndex === test.steps.length - 1 ? undefined : () => updateTest(testIndex, { ...test, steps: moveItem(test.steps, stepIndex, stepIndex + 1) })}
                          onMoveUp={stepIndex === 0 ? undefined : () => updateTest(testIndex, { ...test, steps: moveItem(test.steps, stepIndex, stepIndex - 1) })}
                          onRemove={() => updateTest(testIndex, { ...test, steps: test.steps.filter((_, currentIndex) => currentIndex !== stepIndex) })}
                          ordinal={stepIndex + 1}
                          step={step}
                        />
                      ))}
                    </div>
                    <div className="behavior-builder__add-steps" aria-label={`Add step to ${test.title}`}>
                      <span>Add operation</span>
                      {(["reset-runtime", "set-value", "run-scans", "expect-value"] as const).map((kind) => (
                        <button className="education-button education-button--quiet" disabled={disabled} key={kind} onClick={() => updateTest(testIndex, { ...test, steps: [...test.steps, createBehaviorStepDraft(kind)] })} type="button">+ {behaviorKindLabel(kind)}</button>
                      ))}
                    </div>
                    <button className="education-button authoring-danger-button" disabled={disabled} onClick={() => onChange(removeBehaviorTest(draft, testIndex))} type="button">Remove behavior test</button>
                  </div>
                </details>
              ))}
            </div>
            <button className="education-button education-button--quiet" disabled={disabled} onClick={() => onChange({ ...draft, behaviorTests: [...draft.behaviorTests, createBehaviorTestDraft()] })} type="button">+ Add behavior test</button>
          </AuthoringSection>
        </main>

        <aside>
          <AuthoringSection eyebrow="Scaffolded help" title="Progressive hints" compact>
            <p className="authoring-section-note">Students reveal hints one at a time. Later hints can wait for effort evidence.</p>
            <div className="hint-builder">
              {draft.hintPolicy.hints.map((hint, hintIndex) => (
                <article key={hint.hintId}>
                  <header><span>{String(hint.order).padStart(2, "0")}</span><code>{hint.hintId.slice(0, 8)}</code><button aria-label={`Remove hint ${hint.order}`} className="authoring-icon-button" disabled={disabled} onClick={() => onChange(removeHint(draft, hintIndex))} type="button">×</button></header>
                  <AuthoringField label="Hint title"><input maxLength={160} onChange={(event) => updateHint(hintIndex, { ...hint, title: event.target.value })} value={hint.title} /></AuthoringField>
                  <AuthoringField label="Helpful nudge"><textarea maxLength={8000} onChange={(event) => updateHint(hintIndex, { ...hint, body: event.target.value })} rows={4} value={hint.body} /></AuthoringField>
                  <AuthoringField label="Unlock after">
                    <select onChange={(event) => updateHint(hintIndex, { ...hint, unlock: hintUnlockForKind(event.target.value as ProgressiveHintV1["unlock"]["kind"], hintIndex, draft) })} value={hint.unlock.kind}>
                      <option value="immediate">Immediately</option>
                      <option value="after-compile-attempts">Compile attempts</option>
                      <option value="after-behavior-failures">Failed behavior attempts</option>
                      <option disabled={hintIndex === 0} value="after-previous-hint">Previous hint</option>
                    </select>
                  </AuthoringField>
                  {hint.unlock.kind === "after-compile-attempts" && (
                    <AuthoringField label="Minimum compile attempts"><input min={1} onChange={(event) => updateHint(hintIndex, updateCompileHintMinimum(hint, Math.max(1, Number.parseInt(event.target.value, 10) || 1)))} type="number" value={hint.unlock.minimum} /></AuthoringField>
                  )}
                  {hint.unlock.kind === "after-behavior-failures" && (
                    <div className="authoring-fields">
                      <AuthoringField label="Minimum failures"><input min={1} onChange={(event) => updateHint(hintIndex, updateBehaviorHintMinimum(hint, Math.max(1, Number.parseInt(event.target.value, 10) || 1)))} type="number" value={hint.unlock.minimum} /></AuthoringField>
                      <AuthoringField label="Count failures from"><select onChange={(event) => updateHint(hintIndex, updateBehaviorHintTest(hint, event.target.value === "any" ? null : event.target.value))} value={hint.unlock.testId ?? "any"}><option value="any">Any behavior test</option>{draft.behaviorTests.map((test) => <option key={test.testId} value={test.testId}>{test.title}</option>)}</select></AuthoringField>
                    </div>
                  )}
                </article>
              ))}
            </div>
            <button className="education-button education-button--wide" disabled={disabled} onClick={() => onChange({ ...draft, hintPolicy: { ...draft.hintPolicy, hints: [...draft.hintPolicy.hints, createProgressiveHintDraft(draft.hintPolicy.hints)] } })} type="button">+ Add progressive hint</button>
          </AuthoringSection>

          <section className="teacher-authoring__identity">
            <span>Document identity</span>
            <dl><div><dt>Lifecycle</dt><dd>{draft.lifecycle}</dd></div><div><dt>Revision</dt><dd>{draft.revision}</dd></div><div><dt>Schema</dt><dd>v{draft.schemaVersion}</dd></div></dl>
            <small>Editing never replaces assignment, test, hint, or step IDs. Clone intentionally creates a new assignment identity.</small>
          </section>
        </aside>
      </fieldset>
    </section>
  );
};

const AuthoringSection = ({ children, compact = false, eyebrow, title }: Readonly<{
  children: React.ReactNode;
  compact?: boolean;
  eyebrow: string;
  title: string;
}>): React.JSX.Element => (
  <section className="authoring-section" data-compact={compact}>
    <header><span>{eyebrow}</span><h4>{title}</h4></header>
    <div className="authoring-section__body">{children}</div>
  </section>
);

const AuthoringField = ({ children, className = "", label }: Readonly<{
  children: React.ReactNode;
  className?: string;
  label: string;
}>): React.JSX.Element => <label className={`authoring-field ${className}`}><span>{label}</span>{children}</label>;

const UnlistedTokenList = ({ label, onRemove, values }: Readonly<{
  label: string;
  onRemove: (value: string) => void;
  values: readonly string[];
}>): React.JSX.Element | null => values.length === 0 ? null : (
  <div className="authoring-unlisted" role="note">
    <span>{label}</span>
    <div>{values.map((value) => <button key={value} onClick={() => onRemove(value)} type="button"><code>{value}</code><span aria-hidden="true">×</span></button>)}</div>
  </div>
);

const BehaviorStepEditor = ({
  disabled,
  onChange,
  onMoveDown,
  onMoveUp,
  onRemove,
  ordinal,
  step,
}: Readonly<{
  disabled: boolean;
  onChange: (step: BehaviorTestStepV1) => void;
  onMoveDown: (() => void) | undefined;
  onMoveUp: (() => void) | undefined;
  onRemove: () => void;
  ordinal: number;
  step: BehaviorTestStepV1;
}>): React.JSX.Element => (
  <article className="behavior-step">
    <div className="behavior-step__index"><span>{String(ordinal).padStart(2, "0")}</span><code>{step.stepId.slice(0, 8)}</code></div>
    <AuthoringField label="Operation">
      <select onChange={(event) => onChange(changeBehaviorStepKind(step, event.target.value as BehaviorTestStepV1["kind"]))} value={step.kind}>
        <option value="reset-runtime">Reset runtime</option>
        <option value="set-value">Set virtual value</option>
        <option value="run-scans">Run PLC scans</option>
        <option value="expect-value">Expect value</option>
      </select>
    </AuthoringField>
    {(step.kind === "set-value" || step.kind === "expect-value") && (
      <>
        <AuthoringField label="Target source"><select onChange={(event) => onChange({ ...step, target: { ...step.target, kind: event.target.value as typeof step.target.kind } })} value={step.target.kind}><option value="plc-tag">PLC tag</option><option value="hmi-control">HMI control</option></select></AuthoringField>
        <AuthoringField label="Target name"><input maxLength={160} onChange={(event) => onChange({ ...step, target: { ...step.target, name: event.target.value } })} value={step.target.name} /></AuthoringField>
        <ScalarValueEditor onChange={(value) => onChange(step.kind === "set-value" ? { ...step, value } : { ...step, expected: value })} value={step.kind === "set-value" ? step.value : step.expected} />
      </>
    )}
    {step.kind === "run-scans" && <AuthoringField label="Scan count"><input max={10000} min={1} onChange={(event) => onChange({ ...step, count: Math.max(1, Number.parseInt(event.target.value, 10) || 1) })} type="number" value={step.count} /></AuthoringField>}
    {step.kind === "reset-runtime" && <p>Restore the clean runtime snapshot before the next operation.</p>}
    <div className="behavior-step__actions">
      <button aria-label={`Move step ${ordinal} up`} className="authoring-icon-button" disabled={disabled || onMoveUp === undefined} onClick={onMoveUp} type="button">↑</button>
      <button aria-label={`Move step ${ordinal} down`} className="authoring-icon-button" disabled={disabled || onMoveDown === undefined} onClick={onMoveDown} type="button">↓</button>
      <button aria-label={`Remove step ${ordinal}`} className="authoring-icon-button" disabled={disabled} onClick={onRemove} type="button">×</button>
    </div>
  </article>
);

const ScalarValueEditor = ({ onChange, value }: Readonly<{
  onChange: (value: EducationScalarValue) => void;
  value: EducationScalarValue;
}>): React.JSX.Element => (
  <div className="scalar-editor">
    <AuthoringField label="Value type"><select onChange={(event) => onChange(defaultScalar(event.target.value as EducationScalarValue["type"]))} value={value.type}><option value="BOOL">BOOL</option><option value="DINT">DINT</option><option value="TIME_MS">TIME (ms)</option></select></AuthoringField>
    <AuthoringField label="Value">
      {value.type === "BOOL"
        ? <select onChange={(event) => onChange({ type: "BOOL", value: event.target.value === "true" })} value={String(value.value)}><option value="true">TRUE</option><option value="false">FALSE</option></select>
        : <input min={value.type === "TIME_MS" ? 0 : undefined} onChange={(event) => onChange({ type: value.type, value: Number.parseInt(event.target.value, 10) || 0 })} type="number" value={value.value} />}
    </AuthoringField>
  </div>
);

const updateStarterTag = (
  draft: AssignmentDocumentV1,
  index: number,
  tag: AssignmentDocumentV1["requirements"]["starterTags"][number],
  updateRequirements: (requirements: AssignmentDocumentV1["requirements"]) => void,
): void => updateRequirements({
  ...draft.requirements,
  starterTags: draft.requirements.starterTags.map((current, currentIndex) => currentIndex === index ? tag : current),
});

const nextStarterTagName = (names: readonly string[]): string => {
  let ordinal = names.length + 1;
  while (names.includes(`Tag_${ordinal}`)) ordinal += 1;
  return `Tag_${ordinal}`;
};

const moveItem = <T,>(values: readonly T[], from: number, to: number): readonly T[] => {
  const next = [...values];
  const [item] = next.splice(from, 1);
  if (item !== undefined) next.splice(to, 0, item);
  return next;
};

const changeBehaviorStepKind = (
  step: BehaviorTestStepV1,
  kind: BehaviorTestStepV1["kind"],
): BehaviorTestStepV1 => ({ ...createBehaviorStepDraft(kind), stepId: step.stepId } as BehaviorTestStepV1);

const defaultScalar = (type: EducationScalarValue["type"]): EducationScalarValue => {
  switch (type) {
    case "BOOL": return { type, value: true };
    case "DINT": return { type, value: 0 };
    case "TIME_MS": return { type, value: 0 };
  }
};

const behaviorKindLabel = (kind: BehaviorTestStepV1["kind"]): string => {
  switch (kind) {
    case "reset-runtime": return "Reset";
    case "set-value": return "Set value";
    case "run-scans": return "Run scans";
    case "expect-value": return "Expect value";
  }
};

const hintUnlockForKind = (
  kind: ProgressiveHintV1["unlock"]["kind"],
  hintIndex: number,
  draft: AssignmentDocumentV1,
): ProgressiveHintV1["unlock"] => {
  switch (kind) {
    case "immediate": return { kind };
    case "after-compile-attempts": return { kind, minimum: 1 };
    case "after-behavior-failures": return { kind, minimum: 1, testId: null };
    case "after-previous-hint": {
      const previous = draft.hintPolicy.hints[hintIndex - 1];
      return previous === undefined ? { kind: "immediate" } : { hintId: previous.hintId, kind };
    }
  }
};

const updateCompileHintMinimum = (hint: ProgressiveHintV1, minimum: number): ProgressiveHintV1 => ({
  ...hint,
  unlock: { kind: "after-compile-attempts", minimum },
});

const updateBehaviorHintMinimum = (hint: ProgressiveHintV1, minimum: number): ProgressiveHintV1 => ({
  ...hint,
  unlock: {
    kind: "after-behavior-failures",
    minimum,
    testId: hint.unlock.kind === "after-behavior-failures" ? hint.unlock.testId : null,
  },
});

const updateBehaviorHintTest = (hint: ProgressiveHintV1, testId: string | null): ProgressiveHintV1 => ({
  ...hint,
  unlock: {
    kind: "after-behavior-failures",
    minimum: hint.unlock.kind === "after-behavior-failures" ? hint.unlock.minimum : 1,
    testId,
  },
});

const removeHint = (draft: AssignmentDocumentV1, removeIndex: number): AssignmentDocumentV1 => {
  const removedId = draft.hintPolicy.hints[removeIndex]?.hintId;
  const remaining = draft.hintPolicy.hints.filter((_, index) => index !== removeIndex);
  const repaired = remaining.map((hint, index): ProgressiveHintV1 => {
    const order = index + 1;
    if (hint.unlock.kind !== "after-previous-hint" || hint.unlock.hintId !== removedId) return { ...hint, order };
    const previous = remaining[index - 1];
    return { ...hint, order, unlock: previous === undefined ? { kind: "immediate" } : { hintId: previous.hintId, kind: "after-previous-hint" } };
  });
  return { ...draft, hintPolicy: { ...draft.hintPolicy, hints: repaired } };
};

const removeBehaviorTest = (draft: AssignmentDocumentV1, removeIndex: number): AssignmentDocumentV1 => {
  const removedId = draft.behaviorTests[removeIndex]?.testId;
  return {
    ...draft,
    behaviorTests: draft.behaviorTests.filter((_, index) => index !== removeIndex),
    hintPolicy: {
      ...draft.hintPolicy,
      hints: draft.hintPolicy.hints.map((hint) => hint.unlock.kind === "after-behavior-failures" && hint.unlock.testId === removedId
        ? { ...hint, unlock: { ...hint.unlock, testId: null } }
        : hint),
    },
  };
};

const friendlyIssuePath = (path: string): string => path
  .replace(/^\$\./u, "")
  .replaceAll(/\[(\d+)\]/gu, " $1")
  .replaceAll(".", " › ");

const SubmissionEvidence = ({ assignment, submission }: Readonly<{
  assignment: AssignmentDocumentV1;
  submission: SubmissionDocumentV1;
}>): React.JSX.Element => {
  const tests = new Map(assignment.behaviorTests.map((test) => [test.testId, test]));
  return (
    <div className="submission-evidence">
      <div className="education-progress">
        <ProgressMetric label="Attempt" value={`#${submission.attemptOrdinal}`} tone="neutral" />
        <ProgressMetric label="Lifecycle" value={humanizeToken(submission.lifecycle)} tone={submission.lifecycle === "returned" ? "bad" : "neutral"} />
        <ProgressMetric label="Compile attempts" value={String(submission.evidence.compile.attemptCount)} tone="neutral" />
        <ProgressMetric label="Final build" value={submission.evidence.compile.finalStatus} tone={submission.evidence.compile.finalStatus === "current" ? "good" : "bad"} />
        <ProgressMetric label="Tests passed" value={`${submission.evidence.behaviorResults.filter((result) => result.status === "passed").length}/${assignment.behaviorTests.length}`} tone="good" />
        <ProgressMetric label="Hints used" value={String(submission.evidence.hintUsage.length)} tone="neutral" />
      </div>
      <div className="submission-result-list">
        {submission.evidence.behaviorResults.map((result) => (
          <div data-status={result.status} key={result.testId}><strong>{tests.get(result.testId)?.title ?? result.testId}</strong><span>{result.status}</span></div>
        ))}
      </div>
      {(submission.evidence.compile.blockingDiagnosticCodes.length > 0 || submission.evidence.compile.warningDiagnosticCodes.length > 0) && (
        <details><summary>Compile diagnostics</summary><code>{[...submission.evidence.compile.blockingDiagnosticCodes, ...submission.evidence.compile.warningDiagnosticCodes].join(", ")}</code></details>
      )}
    </div>
  );
};

const ProgressMetric = ({ label, tone, value }: Readonly<{ label: string; tone: "bad" | "good" | "neutral"; value: string }>): React.JSX.Element => (
  <div className="education-progress__metric" data-tone={tone}><span>{label}</span><strong>{value}</strong></div>
);

const BehaviorResultSummary = ({ result }: Readonly<{ result: BehaviorTestResultV1 | undefined }>): React.JSX.Element => {
  if (result === undefined) return <small className="education-result-summary">Not run</small>;
  const failed = result.checks.find((check) => !check.passed);
  return (
    <small className="education-result-summary" data-status={result.status}>
      {result.status === "passed" && "Passed against the virtual PLC"}
      {result.status === "error" && "Could not complete — check runtime and bindings"}
      {result.status === "failed" && failed !== undefined && `Expected ${formatValue(failed.expected)}, observed ${formatValue(failed.actual)}`}
    </small>
  );
};

const replaceLatestResults = (
  current: readonly BehaviorTestResultV1[],
  replacements: readonly BehaviorTestResultV1[],
): readonly BehaviorTestResultV1[] => {
  const next = new Map(current.map((result) => [result.testId, result]));
  replacements.forEach((result) => next.set(result.testId, result));
  return [...next.values()];
};

const formatValue = (value: BehaviorTestResultV1["checks"][number]["actual"]): string => {
  if (value === null) return "unavailable";
  if (value.type === "BOOL") return value.value ? "TRUE" : "FALSE";
  return value.type === "TIME_MS" ? `${value.value} ms` : String(value.value);
};

const humanizeToken = (token: string): string => token
  .replaceAll(/[._-]+/gu, " ")
  .replace(/\b\w/gu, (character) => character.toLocaleUpperCase("en-US"));

const teacherStepSummary = (step: BehaviorTestDefinitionV1["steps"][number]): string => {
  switch (step.kind) {
    case "reset-runtime": return "Restore the clean simulation snapshot.";
    case "set-value": return `Set ${step.target.name} to ${formatValue(step.value)}.`;
    case "run-scans": return `Run ${step.count} PLC ${step.count === 1 ? "scan" : "scans"}.`;
    case "expect-value": return `Expect ${step.target.name} to equal ${formatValue(step.expected)}.`;
  }
};

const hintUnlockSummary = (unlock: AssignmentDocumentV1["hintPolicy"]["hints"][number]["unlock"]): string => {
  switch (unlock.kind) {
    case "immediate": return "Available immediately";
    case "after-compile-attempts": return `After ${unlock.minimum} compile ${unlock.minimum === 1 ? "attempt" : "attempts"}`;
    case "after-behavior-failures": return `After ${unlock.minimum} failed behavior ${unlock.minimum === 1 ? "attempt" : "attempts"}`;
    case "after-previous-hint": return "After the preceding hint is used";
  }
};

const errorMessage = (reason: unknown, fallback: string): string => reason instanceof Error && reason.message.trim().length > 0
  ? reason.message
  : fallback;
