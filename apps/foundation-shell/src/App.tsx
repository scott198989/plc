import { useCallback, useEffect, useRef, useState } from "react";

import { EngineeringClient } from "./engineering-client";
import { EngineeringWorkbench } from "./EngineeringWorkbench";
import { FileAccessBroker, FileAccessError } from "./file-access-broker";
import { GuidedTutorial } from "./GuidedTutorial";
import {
  guidedTutorialExitStatus,
  guidedTutorialResumeStep,
  nextGuidedTutorialStep,
  readGuidedTutorialStatus,
  writeGuidedTutorialStatus,
} from "./guided-tutorial";
import type { GuidedTutorialStep } from "./guided-tutorial";
import { projectWorkbenchMotorStarterGuide } from "./motor-starter-guide";
import { ProjectHome } from "./ProjectHome";
import type { ReplayVerificationReceipt } from "./replay-types";
import type { RuntimeOperation } from "./runtime-types";
import { applyTheme, readInitialTheme } from "./ThemeToggle";
import type { AppTheme } from "./ThemeToggle";
import type { WorkbenchOperation, WorkbenchSnapshot } from "./workbench-types";

type AppServices = Readonly<{
  client: EngineeringClient;
  files: FileAccessBroker;
}>;

export const App = (): React.JSX.Element => {
  const [services] = useState<AppServices>(() => ({
    client: new EngineeringClient(),
    files: new FileAccessBroker(),
  }));
  const [coreLabel, setCoreLabel] = useState<string | null>(null);
  const [snapshot, setSnapshot] = useState<WorkbenchSnapshot | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [replayReceipt, setReplayReceipt] = useState<ReplayVerificationReceipt | null>(null);
  const [closeRequested, setCloseRequested] = useState(false);
  const [theme, setTheme] = useState<AppTheme>(readInitialTheme);
  const [tutorialStep, setTutorialStep] = useState<GuidedTutorialStep | null>(() =>
    readGuidedTutorialStatus() === null ? "create-project" : null
  );
  const [tutorialStartProven, setTutorialStartProven] = useState(false);
  const tutorialStepRef = useRef<GuidedTutorialStep | null>(tutorialStep);
  const tutorialRuntimeProof = useRef<TutorialRuntimeProof>(createTutorialRuntimeProof());

  useEffect(() => applyTheme(theme), [theme]);

  useEffect(() => {
    tutorialStepRef.current = tutorialStep;
    if (tutorialStep === "press-start") {
      tutorialRuntimeProof.current = createTutorialRuntimeProof();
    } else if (tutorialStep === "press-stop") {
      tutorialRuntimeProof.current.stopPressed = false;
    }
  }, [tutorialStep]);

  useEffect(() => {
    if (tutorialStep === "create-project" && snapshot !== null) {
      setTutorialStep("create-lab");
      return;
    }
    if (tutorialStep === "create-lab" && snapshot !== null && hasLearnerMotorLab(snapshot)) {
      setTutorialStep("select-stop");
    }
  }, [snapshot, tutorialStep]);

  useEffect(() => {
    if (snapshot === null || tutorialStep === null) {
      return;
    }
    const session = snapshot.runtime.session;
    if (
      tutorialStep === "start-simulation" &&
      session?.online === true &&
      session.cpuState === "RUN"
    ) {
      setTutorialStep("press-start");
      return;
    }
    const motorState = tutorialMotorOutput(snapshot);
    const startState = tutorialBooleanInput(snapshot, "start_pb");
    const stopState = tutorialBooleanInput(snapshot, "stop_pb");
    if (
      tutorialStep === "press-start" &&
      tutorialRuntimeProof.current.startReleaseScanned &&
      startState === false &&
      motorState === true
    ) {
      setTutorialStartProven(true);
      setTutorialStep("press-stop");
      return;
    }
    if (
      tutorialStep === "press-stop" &&
      tutorialStartProven &&
      tutorialRuntimeProof.current.stopPressed &&
      stopState === true &&
      motorState === false
    ) {
      writeGuidedTutorialStatus("complete");
      setTutorialStep("complete");
    }
  }, [snapshot, tutorialStartProven, tutorialStep]);

  useEffect(() => {
    let active = true;
    services.client.initialize().then(
      (health) => {
        if (active) {
          setCoreLabel(health.coreVersion);
          setError(null);
        }
      },
      (reason: unknown) => {
        if (active) {
          setError(errorMessage(reason));
        }
      },
    );
    return () => {
      active = false;
      services.client.dispose();
    };
  }, [services]);

  useEffect(() => {
    if (snapshot?.dirtyState === "clean") {
      return;
    }
    const protectDirtyProject = (event: BeforeUnloadEvent): void => {
      event.preventDefault();
    };
    window.addEventListener("beforeunload", protectDirtyProject);
    return () => window.removeEventListener("beforeunload", protectDirtyProject);
  }, [snapshot?.dirtyState]);

  const runBusy = useCallback(async <T,>(operation: () => Promise<T>): Promise<T | null> => {
    setBusy(true);
    setError(null);
    try {
      return await operation();
    } catch (reason) {
      if (!(reason instanceof FileAccessError && reason.code === "ACCESS_CANCELLED")) {
        setError(errorMessage(reason));
      }
      return null;
    } finally {
      setBusy(false);
    }
  }, []);

  const createProject = useCallback(async (displayName: string): Promise<void> => {
    const created = await runBusy(() => services.client.createProject(displayName));
    if (created !== null) {
      setSnapshot(created);
      setReplayReceipt(null);
    }
  }, [runBusy, services]);

  const openProject = useCallback(async (): Promise<void> => {
    await runBusy(async () => {
      const opened = await services.files.requestOpen();
      try {
        const next = await services.client.openProject(opened.bytes, opened.grantId);
        setSnapshot(next);
        setReplayReceipt(null);
      } catch (reason) {
        services.files.revoke(opened.grantId);
        throw reason;
      }
    });
  }, [runBusy, services]);

  const executeOperation = useCallback(async (operation: WorkbenchOperation): Promise<void> => {
    setReplayReceipt(null);
    const result = await runBusy(() => services.client.execute(operation));
    if (result === null) {
      return;
    }
    setSnapshot(result.snapshot);
    if (result.outcome !== "committed") {
      const first = result.diagnostics[0];
      setError(first?.message ?? `The command was ${result.outcome}.`);
    }
  }, [runBusy, services]);

  const executeRuntimeOperation = useCallback(async (operation: RuntimeOperation): Promise<void> => {
    setReplayReceipt(null);
    const next = await runBusy(() => services.client.executeRuntime(operation));
    if (next !== null) {
      observeTutorialRuntimeOperation(
        operation,
        next,
        tutorialStepRef.current,
        tutorialRuntimeProof.current,
      );
      setSnapshot(next);
    }
  }, [runBusy, services]);

  const startSimulation = useCallback(async (): Promise<void> => {
    if (snapshot === null) {
      return;
    }
    await runBusy(async () => {
      let current = snapshot;
      const advance = async (operation: RuntimeOperation): Promise<void> => {
        current = await services.client.executeRuntime(operation);
        setSnapshot(current);
      };
      const session = (): NonNullable<WorkbenchSnapshot["runtime"]["session"]> => {
        if (current.runtime.session === null) {
          throw new Error(current.runtime.reason ?? "The virtual controller is not ready yet.");
        }
        return current.runtime.session;
      };

      if (!current.runtime.canBuild) {
        throw new Error(current.runtime.reason ?? "Resolve the blocking project issues before starting simulation.");
      }
      if (!session().buildCurrent) {
        await advance({ kind: "runtime.build" });
      }
      if (session().cpuState === "POWERED_OFF") {
        await advance({ kind: "runtime.power-on" });
      }
      if (!session().loaded || session().loadedArtifactFingerprint !== session().buildFingerprint) {
        if (session().loadPreview === null) {
          await advance({ kind: "runtime.preview-load", postLoadMode: "STOP" });
        }
        await advance({ kind: "runtime.commit-load" });
      }
      if (!session().online) {
        await advance({ kind: "runtime.go-online" });
      }
      if (session().monitorState !== "ACTIVE") {
        await advance({ kind: "runtime.start-monitoring" });
      }
      await advance({ kind: "runtime.capture-snapshot" });
      if (session().cpuState === "STOP") {
        await advance({ kind: "runtime.request-run" });
      }
      await advance({ kind: "runtime.run-scan" });
    });
  }, [runBusy, services, snapshot]);

  const resetSimulation = useCallback(async (): Promise<void> => {
    if (snapshot === null) {
      return;
    }
    await runBusy(async () => {
      let current = snapshot;
      const advance = async (operation: RuntimeOperation): Promise<void> => {
        current = await services.client.executeRuntime(operation);
        setSnapshot(current);
      };
      const session = (): NonNullable<WorkbenchSnapshot["runtime"]["session"]> => {
        if (current.runtime.session === null) {
          throw new Error(current.runtime.reason ?? "The virtual controller is not ready yet.");
        }
        return current.runtime.session;
      };

      if (!session().snapshotAvailable) {
        throw new Error("Start simulation once before resetting the lab.");
      }
      if (session().cpuState === "RUN") {
        await advance({ kind: "runtime.request-stop" });
      }
      await advance({ kind: "runtime.restore-snapshot" });
      if (session().monitorState !== "ACTIVE") {
        await advance({ kind: "runtime.start-monitoring" });
      }
      if (session().cpuState === "STOP") {
        await advance({ kind: "runtime.request-run" });
      }
      await advance({ kind: "runtime.run-scan" });
    });
  }, [runBusy, services, snapshot]);

  const verifyReplay = useCallback(async (): Promise<void> => {
    const verified = await runBusy(async () => {
      const replayPackage = await services.client.exportReplayPackage();
      return services.client.verifyReplayPackage(new Uint8Array(replayPackage.bytes));
    });
    if (verified !== null) {
      setReplayReceipt(verified);
    }
  }, [runBusy, services]);

  const saveProject = useCallback(async (requestedMode: "save" | "save-as"): Promise<boolean> => {
    if (snapshot === null) {
      return false;
    }
    const mode = requestedMode === "save" && snapshot.fileGrantId === null ? "save-as" : requestedMode;
    const savedSuccessfully = await runBusy(async () => {
      const prepared = await services.client.prepareSave(mode);
      try {
        const saved = mode === "save-as"
          ? await services.files.requestSaveAs(prepared.suggestedName, new Uint8Array(prepared.bytes))
          : await services.files.save(
              requireGrant(snapshot.fileGrantId),
              new Uint8Array(prepared.bytes),
            );
        const committed = await services.client.commitSave(
          prepared.pendingSaveId,
          saved.grantId,
          saved.verifiedBytes,
        );
        setSnapshot(committed);
        return true;
      } catch (reason) {
        await services.client.abortSave(prepared.pendingSaveId).catch(() => undefined);
        throw reason;
      }
    });
    return savedSuccessfully === true;
  }, [runBusy, services, snapshot]);

  const closeProject = useCallback((): void => {
    if (snapshot?.dirtyState !== "clean") {
      setCloseRequested(true);
      return;
    }
    setSnapshot(null);
    setError(null);
    setReplayReceipt(null);
  }, [snapshot]);

  const discardAndClose = useCallback((): void => {
    setCloseRequested(false);
    setSnapshot(null);
    setError(null);
    setReplayReceipt(null);
  }, []);

  const saveAndClose = useCallback(async (): Promise<void> => {
    if (await saveProject("save")) {
      discardAndClose();
    }
  }, [discardAndClose, saveProject]);

  const advanceTutorial = useCallback((): void => {
    setTutorialStep((current) => current === null ? null : nextGuidedTutorialStep(current));
  }, []);

  const handleTutorialMilestone = useCallback((milestone: "seal-in-added" | "stop-contact-added"): void => {
    setTutorialStep((current) => {
      if (current === "add-stop-nc" && milestone === "stop-contact-added") {
        return "select-seal-in";
      }
      if (current === "add-seal-in" && milestone === "seal-in-added") {
        return "start-simulation";
      }
      return current;
    });
  }, []);

  const startTutorial = useCallback((): void => {
    const status = readGuidedTutorialStatus();
    if (snapshot === null) {
      setTutorialStartProven(false);
      setError(null);
      setTutorialStep(status === "complete" ? "review" : "create-project");
      return;
    }
    const guide = projectWorkbenchMotorStarterGuide(snapshot);
    const session = snapshot.runtime.session;
    const learnerLabAvailable = hasLearnerMotorLab(snapshot);
    const activeObjectCount = Object.values(snapshot.objects).filter((object) => object.lifecycle === "active").length;
    if (status !== "complete" && !learnerLabAvailable && activeObjectCount > 1) {
      setTutorialStep(null);
      setError("The first ladder tutorial needs a blank project. Save or close this project, then start the tutorial from Home.");
      return;
    }
    setError(null);
    const nextStep = guidedTutorialResumeStep({
      completed: status === "complete",
      hasLearnerMotorLab: learnerLabAvailable,
      hasSealInBranch: guide?.hasSealInBranch === true,
      hasStopContact: guide?.hasStopContact === true,
      motorOutput: tutorialMotorOutput(snapshot),
      simulationRunning: session?.online === true &&
        session.cpuState === "RUN",
    });
    setTutorialStartProven(nextStep === "press-stop");
    setTutorialStep(nextStep);
  }, [snapshot]);

  const exitTutorial = useCallback((): void => {
    if (tutorialStep !== null) {
      writeGuidedTutorialStatus(guidedTutorialExitStatus(tutorialStep));
    }
    setTutorialStep(null);
  }, [tutorialStep]);

  const finishTutorial = useCallback((): void => {
    writeGuidedTutorialStatus("complete");
    setTutorialStep(null);
  }, []);

  const reviewTutorial = useCallback((): void => {
    writeGuidedTutorialStatus("complete");
    setTutorialStep("review");
  }, []);

  const tutorial = tutorialStep === null ? null : (
    <GuidedTutorial
      onAdvance={advanceTutorial}
      onExit={exitTutorial}
      onFinish={finishTutorial}
      onReview={reviewTutorial}
      step={tutorialStep}
    />
  );

  if (snapshot === null) {
    return (
      <>
        <ProjectHome
          busy={busy}
          coreLabel={coreLabel}
          error={error}
          fileAccessAvailable={services.files.canOpen() && services.files.canSave()}
          onCreate={createProject}
          onOpen={openProject}
          onStartTutorial={startTutorial}
          onToggleTheme={() => setTheme((current) => current === "dark" ? "light" : "dark")}
          theme={theme}
        />
        {tutorial}
      </>
    );
  }

  return (
    <>
      <EngineeringWorkbench
        busy={busy}
        error={error}
        onClose={closeProject}
        onOperation={executeOperation}
        onResetSimulation={resetSimulation}
        onRuntimeOperation={executeRuntimeOperation}
        onStartSimulation={startSimulation}
        onStartTutorial={startTutorial}
        onToggleTheme={() => setTheme((current) => current === "dark" ? "light" : "dark")}
        onTutorialMilestone={handleTutorialMilestone}
        onVerifyReplay={verifyReplay}
        onSave={async (mode) => { await saveProject(mode); }}
        replayReceipt={replayReceipt}
        snapshot={snapshot}
        theme={theme}
        tutorialStep={tutorialStep}
      />
      {closeRequested && (
        <div className="dialog-backdrop" role="presentation">
          <section
            aria-describedby="close-project-description"
            aria-labelledby="close-project-title"
            aria-modal="true"
            className="decision-dialog"
            role="dialog"
          >
            <p className="action-kicker">Unsaved project</p>
            <h2 id="close-project-title">Save changes before closing?</h2>
            <p id="close-project-description">
              {snapshot.projectName} has {snapshot.dirtyState === "semantic-dirty" ? "semantic" : "presentation"} changes
              that are not in its last verified save.
            </p>
            <div className="decision-dialog__actions">
              <button disabled={busy} onClick={() => setCloseRequested(false)} type="button">Cancel</button>
              <button className="danger-action" disabled={busy} onClick={discardAndClose} type="button">Discard</button>
              <button className="primary-button" disabled={busy} onClick={() => void saveAndClose()} type="button">Save and close</button>
            </div>
          </section>
        </div>
      )}
      {tutorial}
    </>
  );
};

const errorMessage = (reason: unknown): string =>
  reason instanceof Error ? reason.message : "The requested action did not complete.";

const requireGrant = (grantId: string | null): string => {
  if (grantId === null) {
    throw new Error("Save As is required before this project can be saved.");
  }
  return grantId;
};

type TutorialRuntimeProof = {
  startPressed: boolean;
  startReleasePending: boolean;
  startReleaseScanned: boolean;
  stopPressed: boolean;
};

const createTutorialRuntimeProof = (): TutorialRuntimeProof => ({
  startPressed: false,
  startReleasePending: false,
  startReleaseScanned: false,
  stopPressed: false,
});

const observeTutorialRuntimeOperation = (
  operation: RuntimeOperation,
  snapshot: WorkbenchSnapshot,
  step: GuidedTutorialStep | null,
  proof: TutorialRuntimeProof,
): void => {
  if (operation.kind === "runtime.set-raw-input") {
    const probe = snapshot.runtime.session?.probes.find((candidate) => candidate.id === operation.targetId);
    const name = probe === undefined ? "" : normalizeTutorialName(probe.displayName);
    const value = operation.value.type === "BOOL" ? operation.value.value : null;
    if (step === "press-start" && name === "start_pb" && typeof value === "boolean") {
      if (value) {
        proof.startPressed = true;
        proof.startReleasePending = false;
        proof.startReleaseScanned = false;
      } else if (proof.startPressed) {
        proof.startReleasePending = true;
      }
    }
    if (step === "press-stop" && name === "stop_pb" && value === true) {
      proof.stopPressed = true;
    }
    return;
  }
  if (operation.kind === "runtime.run-scan" && step === "press-start" && proof.startReleasePending) {
    proof.startReleaseScanned = true;
  }
};

const hasLearnerMotorLab = (snapshot: WorkbenchSnapshot): boolean => {
  const active = Object.values(snapshot.objects).filter((object) => object.lifecycle === "active");
  const normalizedNames = new Set(active.map((object) => normalizeTutorialName(object.displayName)));
  return active.some((object) => object.kind === "VirtualNetwork") &&
    active.some((object) => object.kind === "Controller") &&
    active.some((object) => object.kind === "Rack") &&
    active.filter((object) => object.kind === "Module").length >= 2 &&
    active.some((object) => object.kind === "OB" && object.semanticPayload.language === "LAD") &&
    ["start_pb", "stop_pb", "motor_run"].every((name) => normalizedNames.has(name));
};

const tutorialMotorOutput = (snapshot: WorkbenchSnapshot): boolean | null => {
  return tutorialBooleanProbe(snapshot, "output", "motor_run");
};

const tutorialBooleanInput = (
  snapshot: WorkbenchSnapshot,
  normalizedName: string,
): boolean | null => tutorialBooleanProbe(snapshot, "input", normalizedName);

const tutorialBooleanProbe = (
  snapshot: WorkbenchSnapshot,
  kind: "input" | "output",
  normalizedName: string,
): boolean | null => {
  const probe = snapshot.runtime.session?.probes.find((candidate) =>
    candidate.kind === kind && normalizeTutorialName(candidate.displayName) === normalizedName
  );
  const value = probe?.effectiveValue;
  return value?.type === "BOOL" && typeof value.value === "boolean" ? value.value : null;
};

const normalizeTutorialName = (value: string): string =>
  value.trim().toLocaleLowerCase("en-US").replaceAll(/[^a-z0-9]+/gu, "_").replaceAll(/^_+|_+$/gu, "");
