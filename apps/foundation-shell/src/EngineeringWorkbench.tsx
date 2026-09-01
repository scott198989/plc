import { useEffect, useMemo, useRef, useState } from "react";

import {
  canonicalRecordFields,
  createArrayTypeExpression,
  createDataBlockPayload,
  createFbdProgramPayload,
  createInterfaceMemberPayload,
  createLadProgramPayload,
  createNamedTypeMemberPayload,
  createNamedTypePayload,
  createSclProgramPayload,
  createTracePayload,
  createWatchPayload,
  interfaceMemberIdentity,
  plcScalarTypeTokens,
  recordValue,
  signedValue,
  unsignedValue,
  updateGraphNodeFields,
} from "./canonical-authoring";
import type { PlcScalarTypeToken } from "./canonical-authoring";
import { createAssignmentStarterPlan } from "./assignment-starter-plan";
import { EducationWorkspace } from "./EducationWorkspace";
import type { AssignmentDocumentV1, ProjectArtifactV1 } from "./education-contract";
import {
  addLadNetwork,
  insertSeriesContact,
  moveLadNetwork,
  removeContactAndReconnect,
  removeLadNetwork,
  updateLadCoil,
  updateLadContact,
  wrapContactWithParallelContact,
} from "./lad-authoring";
import type { LadAuthoringResult } from "./lad-authoring";
import {
  insertMvpLadInstructionBoxOnEdge,
  removeLadBoxAndReconnect,
  updateLadBoxPinBinding,
} from "./lad-box-authoring";
import type { LadBoxAuthoringResult } from "./lad-box-authoring";
import {
  findMvpLadInstruction,
  getMvpLadInstruction,
  MVP_LAD_INSTRUCTION_CATALOG,
} from "./lad-instruction-catalog";
import type {
  CanonicalLadPinBinding,
  MvpLadInstructionDefinition,
  MvpLadInstructionKey,
} from "./lad-instruction-catalog";
import { projectLadLiveMonitoring } from "./lad-live-monitoring";
import type { LadBooleanMemberLiveState } from "./lad-live-monitoring";
import { projectLadPowerFlow } from "./lad-power-flow";
import type { LadPowerFlowProjection, LadPowerState } from "./lad-power-flow";
import { createLadderStarterPlan } from "./ladder-starter";
import { createLadStateStoragePlan } from "./lad-state-storage";
import { createSafeLadRungDuplicationPlan } from "./lad-rung-duplication";
import { projectLadNetworkTopology } from "./lad-topology";
import type { LadTopologyItem, LadTopologyParallel } from "./lad-topology";
import { projectMotorStarterGuide } from "./motor-starter-guide";
import type { MotorStarterGuideProjection } from "./motor-starter-guide";
import { TutorialLaunchButton } from "./GuidedTutorial";
import type { GuidedTutorialStep } from "./guided-tutorial";
import { HardwareConfigurationEditor } from "./HardwareConfigurationEditor";
import { firstFreeModuleSlot } from "./hardware-configuration";
import { HmiScreenEditor } from "./HmiScreenEditor";
import { createHmiScreen, decodeHmiScreenPayload, encodeHmiScreenPayload } from "./hmi-screen-model";
import { createPlcSetupPlan } from "./plc-setup";
import type { VirtualPlcCatalogId } from "./plc-setup";
import { ProjectSetupPanel } from "./ProjectSetupPanel";
import { RuntimeInspector, RuntimeToolbar } from "./RuntimeWorkbench";
import type { ReplayVerificationReceipt } from "./replay-types";
import type { EngineeringRuntimeView, RuntimeOperation } from "./runtime-types";
import { TagConfigurationEditor } from "./TagConfigurationEditor";
import { TagTableEditor } from "./TagTableEditor";
import { ThemeToggle } from "./ThemeToggle";
import type { AppTheme } from "./ThemeToggle";
import { VirtualTrainerTutorialProvider } from "./VirtualTrainer";
import type {
  ProjectPayload,
  ProjectPayloadValue,
  ProjectStorageKind,
  WorkbenchObjectView,
  WorkbenchOperation,
  WorkbenchSnapshot,
} from "./workbench-types";

type EngineeringWorkbenchProps = Readonly<{
  busy: boolean;
  compileAttemptCount: number;
  error: string | null;
  onClose: () => void;
  onDeleteProject: () => void;
  onEducationResetRuntime: () => Promise<EngineeringRuntimeView>;
  onEducationRuntimeOperation: (operation: RuntimeOperation) => Promise<EngineeringRuntimeView>;
  onExportProjectArtifact: () => Promise<ProjectArtifactV1>;
  onOpenEducationProject: (artifact: ProjectArtifactV1, allowReplaceEmptyProject: boolean) => Promise<void>;
  onOperation: (operation: WorkbenchOperation) => Promise<void>;
  onResetSimulation: () => Promise<void>;
  onRuntimeOperation: (operation: RuntimeOperation) => Promise<void>;
  onSave: (mode: "save" | "save-as") => Promise<void>;
  onStartSimulation: () => Promise<void>;
  onStartTutorial: () => void;
  onToggleTheme: () => void;
  onTutorialMilestone: (milestone: "seal-in-added" | "stop-contact-added") => void;
  onVerifyReplay: () => Promise<void>;
  replayReceipt: ReplayVerificationReceipt | null;
  snapshot: WorkbenchSnapshot;
  theme: AppTheme;
  tutorialStep: GuidedTutorialStep | null;
}>;

const ladderTutorialEditorSteps: ReadonlySet<GuidedTutorialStep> = new Set([
  "select-stop",
  "add-stop-nc",
  "select-seal-in",
  "add-seal-in",
  "start-simulation",
  "press-start",
  "press-stop",
]);

const ENGINEERING_TOOLS_DEFAULT_HEIGHT = 300;
const ENGINEERING_TOOLS_MIN_HEIGHT = 150;
const ENGINEERING_TOOLS_EDITOR_MIN_HEIGHT = 220;
const ENGINEERING_TOOLS_KEYBOARD_STEP = 24;

const kindLabel: Readonly<Record<WorkbenchObjectView["kind"], string>> = {
  BuildRecord: "Build record",
  Channel: "Channel",
  Constant: "Constant",
  Controller: "Controller",
  Device: "Device",
  FB: "Function block",
  FC: "Function",
  Folder: "Folder",
  GlobalDB: "Global data block",
  HmiScreen: "HMI screen",
  InstanceDB: "Instance data block",
  Module: "Module",
  NamedType: "Named type",
  OB: "Organization block",
  ProjectRoot: "Project",
  Rack: "Rack",
  SnapshotReference: "Snapshot reference",
  SymbolTable: "Symbol table",
  Tag: "Tag",
  TraceConfiguration: "Trace configuration",
  VirtualInterface: "Virtual interface",
  VirtualNetwork: "Virtual network",
  WatchTable: "Watch table",
};

const projectHmiControlTagIds = (
  objects: readonly WorkbenchObjectView[],
): Readonly<Record<string, string>> => {
  const candidates = new Map<string, Set<string>>();
  const add = (name: string, tagId: string): void => {
    const normalized = name.trim();
    if (normalized.length === 0) return;
    const existing = candidates.get(normalized);
    if (existing === undefined) {
      candidates.set(normalized, new Set([tagId]));
    } else {
      existing.add(tagId);
    }
  };

  for (const object of objects) {
    if (object.kind !== "HmiScreen") continue;
    const decoded = decodeHmiScreenPayload(object.semanticPayload);
    if (!decoded.ok) continue;
    for (const element of decoded.screen.elements) {
      if (element.tagId === null) continue;
      add(element.id, element.tagId);
      add(element.label, element.tagId);
      add(`${decoded.screen.name}.${element.label}`, element.tagId);
    }
  }

  return Object.fromEntries(
    [...candidates.entries()]
      .filter((entry): entry is [string, Set<string>] => entry[1].size === 1)
      .map(([name, tagIds]) => [name, [...tagIds][0] as string]),
  );
};

export const EngineeringWorkbench = ({
  busy,
  compileAttemptCount,
  error,
  onClose,
  onDeleteProject,
  onEducationResetRuntime,
  onEducationRuntimeOperation,
  onExportProjectArtifact,
  onOpenEducationProject,
  onOperation,
  onResetSimulation,
  onRuntimeOperation,
  onSave,
  onStartSimulation,
  onStartTutorial,
  onToggleTheme,
  onTutorialMilestone,
  onVerifyReplay,
  replayReceipt,
  snapshot,
  theme,
  tutorialStep,
}: EngineeringWorkbenchProps): React.JSX.Element => {
  const [selectedId, setSelectedId] = useState(snapshot.projectRootId);
  const [openTabs, setOpenTabs] = useState<readonly string[]>([snapshot.projectRootId]);
  const [bottomPane, setBottomPane] = useState<"diagnostics" | "runtime" | null>(null);
  const [bottomPaneHeight, setBottomPaneHeight] = useState(ENGINEERING_TOOLS_DEFAULT_HEIGHT);
  const [createMenuOpen, setCreateMenuOpen] = useState(false);
  const [educationOpen, setEducationOpen] = useState(false);
  const [ladderFocus, setLadderFocus] = useState(false);
  const [projectMenuOpen, setProjectMenuOpen] = useState(false);
  const editorRegionRef = useRef<HTMLElement | null>(null);
  const lastBottomPaneRef = useRef<"diagnostics" | "runtime">("diagnostics");
  const bottomPaneResizeRef = useRef<Readonly<{
    pointerId: number;
    startHeight: number;
    startY: number;
  }> | null>(null);

  const selected = snapshot.objects[selectedId] ?? snapshot.objects[snapshot.projectRootId];
  const resolvedSelectedId = selected?.id ?? snapshot.projectRootId;
  const ladderSelected = selected !== undefined &&
    isGraphicalProgramBlock(selected) &&
    selected.semanticPayload.language !== "FBD";

  useEffect(() => {
    if (snapshot.objects[selectedId]?.lifecycle !== "active") {
      setSelectedId(snapshot.projectRootId);
    }
    setOpenTabs((current) => {
      const valid = current.filter((id) => snapshot.objects[id]?.lifecycle === "active");
      return valid.length === 0 ? [snapshot.projectRootId] : valid;
    });
  }, [selectedId, snapshot]);

  useEffect(() => {
    if (ladderFocus && !ladderSelected) {
      setLadderFocus(false);
    }
  }, [ladderFocus, ladderSelected]);

  useEffect(() => {
    if (bottomPane !== null) {
      lastBottomPaneRef.current = bottomPane;
    }
  }, [bottomPane]);

  useEffect(() => {
    const keepDrawerWithinEditor = (): void => {
      setBottomPaneHeight((current) => {
        const availableHeight = editorRegionRef.current?.clientHeight ?? window.innerHeight;
        const maximum = Math.max(
          ENGINEERING_TOOLS_MIN_HEIGHT,
          availableHeight - ENGINEERING_TOOLS_EDITOR_MIN_HEIGHT - 38,
        );
        return Math.min(Math.max(current, ENGINEERING_TOOLS_MIN_HEIGHT), maximum);
      });
    };
    keepDrawerWithinEditor();
    window.addEventListener("resize", keepDrawerWithinEditor);
    return () => window.removeEventListener("resize", keepDrawerWithinEditor);
  }, [bottomPane]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.defaultPrevented) {
        return;
      }
      const target = event.target;
      const isEditing =
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        target instanceof HTMLSelectElement ||
        (target instanceof HTMLElement && target.isContentEditable);
      if ((event.ctrlKey || event.metaKey) && event.key.toLocaleLowerCase("en-US") === "s") {
        event.preventDefault();
        void onSave(event.shiftKey ? "save-as" : "save");
        return;
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLocaleLowerCase("en-US") === "z") {
        event.preventDefault();
        if (!busy && snapshot.undo.canUndo) {
          void onOperation({ kind: "project.undo" });
        }
        return;
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLocaleLowerCase("en-US") === "y") {
        event.preventDefault();
        if (!busy && snapshot.undo.canRedo) {
          void onOperation({ kind: "project.redo" });
        }
        return;
      }
      if ((event.ctrlKey || event.metaKey) && event.shiftKey && event.key.toLocaleLowerCase("en-US") === "f") {
        if (ladderSelected) {
          event.preventDefault();
          setLadderFocus((current) => {
            if (!current) {
              setBottomPane(null);
            }
            return !current;
          });
        }
        return;
      }
      if (event.key === "Escape" && ladderFocus) {
        event.preventDefault();
        setLadderFocus(false);
        return;
      }
      if (!isEditing && event.key === "Delete" && resolvedSelectedId !== snapshot.projectRootId) {
        event.preventDefault();
        void onOperation({ kind: "project.delete-object", objectId: resolvedSelectedId });
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [busy, ladderFocus, ladderSelected, onOperation, onSave, resolvedSelectedId, snapshot.projectRootId, snapshot.undo]);

  const selectObject = (objectId: string): void => {
    const object = snapshot.objects[objectId];
    // A freshly created object can arrive in the parent snapshot in the same
    // React batch as this selection request. Allow that known new identity;
    // the snapshot-lifecycle effect still rejects tombstoned objects.
    if (object !== undefined && object.lifecycle !== "active") {
      return;
    }
    setSelectedId(objectId);
    setOpenTabs((current) =>
      current.includes(objectId) ? current : [...current, objectId],
    );
  };

  const activeObjects = useMemo(
    () => Object.values(snapshot.objects).filter((object) => object.lifecycle === "active"),
    [snapshot.objects],
  );
  const hmiControlTagIds = useMemo(
    () => projectHmiControlTagIds(activeObjects),
    [activeObjects],
  );
  const learnerLadProgram = activeObjects.find((object) =>
    object.kind === "OB" && object.semanticPayload.language === "LAD"
  );
  useEffect(() => {
    if (tutorialStep === "create-lab") {
      setSelectedId(snapshot.projectRootId);
      setOpenTabs((current) => current.includes(snapshot.projectRootId)
        ? current
        : [...current, snapshot.projectRootId]);
      setBottomPane(null);
      setLadderFocus(false);
      return;
    }
    if (tutorialStep === null || learnerLadProgram === undefined || !ladderTutorialEditorSteps.has(tutorialStep)) {
      return;
    }
    setSelectedId(learnerLadProgram.id);
    setOpenTabs((current) => current.includes(learnerLadProgram.id)
      ? current
      : [...current, learnerLadProgram.id]);
    const runtimeTutorialStep = tutorialStep === "press-start" || tutorialStep === "press-stop";
    setBottomPane(runtimeTutorialStep ? "runtime" : null);
    if (runtimeTutorialStep) {
      setLadderFocus(false);
    }
  }, [learnerLadProgram, snapshot.projectRootId, tutorialStep]);
  const tombstoneCount = Object.values(snapshot.objects).length - activeObjects.length;
  const blockingCount = snapshot.diagnostics.filter((diagnostic) => diagnostic.blocking).length;
  const createOptions = selected === undefined ? [] : creationOptions(selected, snapshot);

  const createObject = async (template: CreateObjectTemplate): Promise<void> => {
    const objectId = crypto.randomUUID();
    await onOperation({
      displayName: nextObjectName(
        template.baseName,
        resolvedSelectedId,
        snapshot,
        requiresPlcIdentifier(template),
      ),
      kind: "project.create-object",
      objectId,
      objectKind: template.objectKind,
      parentId: resolvedSelectedId,
      payloadSchema: template.payloadSchema,
      presentationPayload: {},
      semanticPayload: typeof template.semanticPayload === "function"
        ? template.semanticPayload()
        : template.semanticPayload,
    });
    setCreateMenuOpen(false);
    setSelectedId(objectId);
    setOpenTabs((current) => [...current, objectId]);
  };

  const openLadderProgram = (programId: string): void => {
    setSelectedId(programId);
    setOpenTabs((current) => current.includes(programId) ? current : [...current, programId]);
    setBottomPane(null);
  };

  const startLadderLab = async (): Promise<void> => {
    if (learnerLadProgram !== undefined) {
      openLadderProgram(learnerLadProgram.id);
      return;
    }
    const plan = createLadderStarterPlan(snapshot.projectRootId);
    for (const operation of plan.operations) {
      await onOperation(operation);
    }
    openLadderProgram(plan.programId);
  };

  const createPlcWorkspace = async (catalogId: VirtualPlcCatalogId): Promise<void> => {
    const plan = createPlcSetupPlan(snapshot, catalogId);
    for (const operation of plan.operations) {
      await onOperation(operation);
    }
    selectObject(plan.rackId);
  };

  const loadAssignmentStarter = async (assignment: AssignmentDocumentV1): Promise<void> => {
    const starterAlreadyPresent = activeObjects.some((object) => object.kind === "VirtualNetwork") &&
      activeObjects.some((object) => object.kind === "Controller") &&
      activeObjects.some((object) => object.kind === "Rack") &&
      activeObjects.some((object) => object.kind === "SymbolTable") &&
      activeObjects.some((object) => object.kind === "OB" && object.semanticPayload.language === "LAD");
    if (starterAlreadyPresent) {
      return;
    }
    if (activeObjects.length !== 1) {
      throw new Error("Load this starter into an empty project, or continue with the PLC already in this project.");
    }
    if (assignment.starterProject.kind === "embedded-project") {
      await onOpenEducationProject(assignment.starterProject.artifact, true);
      return;
    }
    if (
      assignment.starterProject.kind === "built-in-template" &&
      assignment.starterProject.templateId !== "builtin.virtual-plc-blank/1"
    ) {
      throw new Error("This assignment starter is not available in the current offline build.");
    }
    const plan = createAssignmentStarterPlan(snapshot, assignment);
    for (const operation of plan.operations) {
      await onOperation(operation);
    }
    selectObject(plan.rackId);
  };

  const startAndOpenSimulation = async (): Promise<void> => {
    await onStartSimulation();
    setLadderFocus(false);
    setBottomPane("runtime");
  };

  const toggleLadderFocus = (): void => {
    if (!ladderSelected && !ladderFocus) {
      return;
    }
    setLadderFocus((current) => {
      if (!current) {
        setBottomPane(null);
      }
      return !current;
    });
  };

  const toggleEducationWorkspace = (): void => {
    setEducationOpen((current) => {
      if (!current) {
        setLadderFocus(false);
        setBottomPane(null);
      }
      return !current;
    });
  };

  const maximumBottomPaneHeight = (): number => {
    const availableHeight = editorRegionRef.current?.clientHeight ?? window.innerHeight;
    return Math.max(
      ENGINEERING_TOOLS_MIN_HEIGHT,
      availableHeight - ENGINEERING_TOOLS_EDITOR_MIN_HEIGHT - 38,
    );
  };

  const clampBottomPaneHeight = (height: number): number => Math.min(
    Math.max(height, ENGINEERING_TOOLS_MIN_HEIGHT),
    maximumBottomPaneHeight(),
  );

  const resizeBottomPaneByKeyboard = (event: React.KeyboardEvent<HTMLDivElement>): void => {
    const multiplier = event.shiftKey ? 3 : 1;
    switch (event.key) {
      case "ArrowUp":
        event.preventDefault();
        setBottomPaneHeight((current) => clampBottomPaneHeight(
          current + ENGINEERING_TOOLS_KEYBOARD_STEP * multiplier,
        ));
        break;
      case "ArrowDown":
        event.preventDefault();
        setBottomPaneHeight((current) => clampBottomPaneHeight(
          current - ENGINEERING_TOOLS_KEYBOARD_STEP * multiplier,
        ));
        break;
      case "Home":
        event.preventDefault();
        setBottomPaneHeight(ENGINEERING_TOOLS_MIN_HEIGHT);
        break;
      case "End":
        event.preventDefault();
        setBottomPaneHeight(maximumBottomPaneHeight());
        break;
      case "Escape":
        event.preventDefault();
        setBottomPane(null);
        break;
    }
  };

  return (
    <div
      className="workbench-shell"
      data-education-open={educationOpen}
      data-ladder-focus={ladderFocus}
    >
      <header className="workbench-header">
        <div className="workbench-brand">
          <span className="workbench-brand__mark" aria-hidden="true">VL</span>
          <span>PLC Engineering Simulator</span>
        </div>
        <div className="project-identity">
          <strong>{snapshot.projectName}</strong>
          <span
            aria-label={formatDirtyState(snapshot.dirtyState)}
            aria-live="polite"
            className="dirty-indicator"
            data-dirty={snapshot.dirtyState !== "clean"}
            role="status"
          >
            <span aria-hidden="true">●</span>
            {formatDirtyState(snapshot.dirtyState)}
          </span>
        </div>
        <div className="header-actions" aria-label="Project commands">
          <button
            aria-pressed={educationOpen}
            className="text-button education-launch"
            disabled={busy}
            onClick={toggleEducationWorkspace}
            type="button"
          >{educationOpen ? "Engineering" : "Learning"}</button>
          <TutorialLaunchButton compact onClick={onStartTutorial} />
          {ladderSelected && (
            <button
              aria-controls="main-content"
              aria-label={ladderFocus ? "Restore workbench layout" : "Expand ladder editor"}
              aria-pressed={ladderFocus}
              className="icon-button workbench-focus-toggle"
              onClick={toggleLadderFocus}
              title={`${ladderFocus ? "Restore workbench layout" : "Expand ladder editor"} (Ctrl+Shift+F)`}
              type="button"
            >{ladderFocus ? "↙" : "↗"}</button>
          )}
          <ThemeToggle onToggle={onToggleTheme} theme={theme} />
          <button
            aria-label={snapshot.undo.undoLabel ?? "Undo"}
            className="icon-button"
            disabled={busy || !snapshot.undo.canUndo}
            onClick={() => void onOperation({ kind: "project.undo" })}
            title={snapshot.undo.undoLabel ?? "Undo"}
            type="button"
          >↶</button>
          <button
            aria-label={snapshot.undo.redoLabel ?? "Redo"}
            className="icon-button"
            disabled={busy || !snapshot.undo.canRedo}
            onClick={() => void onOperation({ kind: "project.redo" })}
            title={snapshot.undo.redoLabel ?? "Redo"}
            type="button"
          >↷</button>
          <span className="header-divider" aria-hidden="true" />
          <button className="text-button" disabled={busy} onClick={() => void onSave("save")} type="button">
            Save
          </button>
          <div className="project-actions-menu">
            <button
              aria-expanded={projectMenuOpen}
              aria-haspopup="menu"
              aria-label="More project actions"
              className="icon-button icon-button--menu"
              disabled={busy}
              onClick={() => setProjectMenuOpen((open) => !open)}
              title="More project actions"
              type="button"
            >⋯</button>
            {projectMenuOpen && (
              <div aria-label="More project actions" className="project-actions-menu__popover" role="menu">
                <button
                  disabled={busy}
                  onClick={() => {
                    setProjectMenuOpen(false);
                    void onSave("save-as");
                  }}
                  role="menuitem"
                  type="button"
                >
                  <strong>Duplicate project…</strong>
                  <small>Save an independent copy with a new document identity</small>
                </button>
                <button
                  className="project-actions-menu__danger"
                  disabled={busy}
                  onClick={() => {
                    setProjectMenuOpen(false);
                    onDeleteProject();
                  }}
                  role="menuitem"
                  type="button"
                >
                  <strong>Delete from workspace</strong>
                  <small>Close this working copy; saved files stay on your computer</small>
                </button>
              </div>
            )}
          </div>
          <button className="text-button text-button--quiet" disabled={busy} onClick={onClose} type="button">
            Close
          </button>
        </div>
      </header>

      <RuntimeToolbar
        busy={busy}
        onOperation={onRuntimeOperation}
        onResetSimulation={onResetSimulation}
        onStartSimulation={startAndOpenSimulation}
        onVerifyReplay={onVerifyReplay}
        replayReceipt={replayReceipt}
        runtime={snapshot.runtime}
      />

      <div className="workbench-body">
        <aside className="navigator-pane" aria-label="Project navigator">
          <div className="pane-heading">
            <span>Project</span>
            <div className="navigator-heading-actions">
              <span className="object-count">{activeObjects.length}</span>
              <div className="create-object-control">
                <button
                  aria-expanded={createMenuOpen}
                  aria-haspopup="menu"
                  aria-label="Add engineering object"
                  className="navigator-add"
                  disabled={busy || createOptions.length === 0}
                  onClick={() => setCreateMenuOpen((open) => !open)}
                  title={createOptions.length === 0 ? "No child objects are valid here" : "Add engineering object"}
                  type="button"
                >+</button>
                {createMenuOpen && createOptions.length > 0 && (
                  <div aria-label={`Add to ${selected?.displayName ?? "selection"}`} className="create-object-menu" role="menu">
                    <div className="create-object-menu__heading">
                      <span>New object</span>
                      <small>{selected?.displayName}</small>
                    </div>
                    {createOptions.map((option) => (
                      <button
                        disabled={busy}
                        key={`${option.objectKind}:${option.payloadSchema}:${option.baseName}`}
                        onClick={() => void createObject(option)}
                        role="menuitem"
                        type="button"
                      >
                        <span className="create-object-menu__glyph" aria-hidden="true">{option.glyph}</span>
                        <span><strong>{option.label}</strong><small>{option.description}</small></span>
                      </button>
                    ))}
                  </div>
                )}
              </div>
            </div>
          </div>
          <div className="tree-scroll" role="tree" aria-label="Project objects">
            <ProjectTree
              objectId={snapshot.projectRootId}
              objects={snapshot.objects}
              onSelect={selectObject}
              selectedId={resolvedSelectedId}
            />
          </div>
          <div className="navigator-foot">
            <span>Document r{snapshot.documentRevision}</span>
            <span>Semantic r{snapshot.semanticRevision}</span>
          </div>
        </aside>

        <main
          className="editor-region"
          data-tools-open={bottomPane !== null}
          id="main-content"
          ref={editorRegionRef}
          style={{ "--engineering-tools-height": `${bottomPaneHeight}px` } as React.CSSProperties}
        >
          <div className="education-workspace-host" hidden={!educationOpen}>
              <div className="education-workspace-host__bar">
                <button disabled={busy} onClick={() => setEducationOpen(false)} type="button">
                  <span aria-hidden="true">←</span> Back to engineering
                </button>
                <span>Student work and teacher review stay local until a file is exported.</span>
              </div>
              <EducationWorkspace
                busy={busy}
                compileAttemptCount={compileAttemptCount}
                hmiControlTagIds={hmiControlTagIds}
                onExportProjectArtifact={onExportProjectArtifact}
                onLoadAssignmentStarter={loadAssignmentStarter}
                onOpenSubmittedProject={(artifact) => onOpenEducationProject(artifact, false)}
                onResetRuntime={onEducationResetRuntime}
                onRuntimeOperation={onEducationRuntimeOperation}
                snapshot={snapshot}
              />
          </div>
          <div
            aria-hidden={educationOpen}
            className="editor-tabs"
            inert={educationOpen}
            role="tablist"
            aria-label="Open engineering objects"
          >
            {openTabs.map((objectId) => {
              const object = snapshot.objects[objectId];
              if (object === undefined || object.lifecycle !== "active") {
                return null;
              }
              const selectedTab = objectId === resolvedSelectedId;
              return (
                <div className="editor-tab-wrap" key={objectId}>
                  <button
                    aria-selected={selectedTab}
                    className="editor-tab"
                    onClick={() => setSelectedId(objectId)}
                    role="tab"
                    type="button"
                  >
                    <ObjectGlyph kind={object.kind} />
                    <span>{object.displayName}</span>
                  </button>
                  {objectId !== snapshot.projectRootId && (
                    <button
                      aria-label={`Close ${object.displayName}`}
                      className="tab-close"
                      onClick={() => {
                        const next = openTabs.filter((id) => id !== objectId);
                        setOpenTabs(next);
                        if (selectedTab) {
                          setSelectedId(next.at(-1) ?? snapshot.projectRootId);
                        }
                      }}
                      type="button"
                    >×</button>
                  )}
                </div>
              );
            })}
          </div>

          <section aria-hidden={educationOpen} className="object-editor" inert={educationOpen} role="tabpanel">
            {selected === undefined ? (
              <p>Object unavailable.</p>
            ) : selected.kind === "ProjectRoot" ? (
              <ProjectOverview
                activeCount={activeObjects.length}
                blockingCount={blockingCount}
                busy={busy}
                canCreateLadderLab={activeObjects.length === 1}
                ladderProgramName={learnerLadProgram?.displayName ?? null}
                onCreatePlc={createPlcWorkspace}
                onLadderAction={() => void startLadderLab()}
                onOpenObject={selectObject}
                snapshot={snapshot}
                tombstoneCount={tombstoneCount}
              />
            ) : selected.kind === "Controller" || selected.kind === "Rack" || selected.kind === "Module" ? (
              <HardwareConfigurationEditor
                busy={busy}
                object={selected}
                onOperation={onOperation}
                onSelectObject={selectObject}
                snapshot={snapshot}
              />
            ) : selected.kind === "Tag" ? (
              <TagConfigurationEditor
                busy={busy}
                object={selected}
                onOperation={onOperation}
                snapshot={snapshot}
              />
            ) : selected.kind === "SymbolTable" ? (
              <TagTableEditor
                busy={busy}
                object={selected}
                onOperation={onOperation}
                onSelectObject={selectObject}
                snapshot={snapshot}
              />
            ) : selected.kind === "HmiScreen" ? (
              <HmiScreenEditor
                busy={busy}
                object={selected}
                onOperation={onOperation}
                onRuntimeOperation={onRuntimeOperation}
                onStartSimulation={startAndOpenSimulation}
                snapshot={snapshot}
              />
            ) : isEditableMemberContainer(selected) ? (
              <MemberTableEditor
                busy={busy}
                object={selected}
                onOperation={onOperation}
                snapshot={snapshot}
              />
            ) : isSclProgramBlock(selected) ? (
              <SclProgramEditor busy={busy} object={selected} onOperation={onOperation} />
            ) : isGraphicalProgramBlock(selected) ? (
              <GraphicalProgramEditor
                busy={busy}
                ladderFocus={ladderFocus}
                object={selected}
                onOperation={onOperation}
                onStartSimulation={startAndOpenSimulation}
                onToggleLadderFocus={toggleLadderFocus}
                onTutorialMilestone={onTutorialMilestone}
                snapshot={snapshot}
              />
            ) : (
              <ObjectOverview object={selected} snapshot={snapshot} />
            )}
          </section>

          <section
            aria-hidden={educationOpen}
            aria-label="Engineering tools"
            className="diagnostics-pane"
            data-open={bottomPane !== null}
            id="engineering-tools-drawer"
            inert={educationOpen}
          >
            {bottomPane !== null && (
              <div
                aria-controls="engineering-tools-drawer"
                aria-label="Resize engineering tools drawer"
                aria-orientation="horizontal"
                aria-valuemax={maximumBottomPaneHeight()}
                aria-valuemin={ENGINEERING_TOOLS_MIN_HEIGHT}
                aria-valuenow={Math.round(bottomPaneHeight)}
                className="engineering-tools-resize-handle"
                onKeyDown={resizeBottomPaneByKeyboard}
                onPointerCancel={() => {
                  bottomPaneResizeRef.current = null;
                }}
                onPointerDown={(event) => {
                  event.preventDefault();
                  event.currentTarget.focus();
                  bottomPaneResizeRef.current = {
                    pointerId: event.pointerId,
                    startHeight: bottomPaneHeight,
                    startY: event.clientY,
                  };
                  event.currentTarget.setPointerCapture(event.pointerId);
                }}
                onPointerMove={(event) => {
                  const resize = bottomPaneResizeRef.current;
                  if (resize === null || resize.pointerId !== event.pointerId) {
                    return;
                  }
                  setBottomPaneHeight(clampBottomPaneHeight(
                    resize.startHeight + resize.startY - event.clientY,
                  ));
                }}
                onPointerUp={(event) => {
                  if (bottomPaneResizeRef.current?.pointerId === event.pointerId) {
                    bottomPaneResizeRef.current = null;
                    event.currentTarget.releasePointerCapture(event.pointerId);
                  }
                }}
                role="separator"
                tabIndex={0}
                title="Drag to resize. Arrow keys resize; Escape closes."
              >
                <span aria-hidden="true" />
              </div>
            )}
            <div aria-label="Engineering tools" className="engineering-tools-heading" role="tablist">
              <button
                aria-controls="engineering-tools-panel-diagnostics"
                aria-selected={bottomPane === "diagnostics"}
                className="engineering-tool-tab"
                id="engineering-tools-tab-diagnostics"
                onClick={() => setBottomPane("diagnostics")}
                role="tab"
                type="button"
              >Diagnostics</button>
              <button
                aria-controls="engineering-tools-panel-runtime"
                aria-selected={bottomPane === "runtime"}
                className="engineering-tool-tab"
                id="engineering-tools-tab-runtime"
                onClick={() => setBottomPane("runtime")}
                role="tab"
                type="button"
              >Runtime &amp; commissioning</button>
              <button
                aria-controls="engineering-tools-drawer"
                aria-expanded={bottomPane !== null}
                aria-label={bottomPane === null ? "Open engineering tools" : "Collapse engineering tools"}
                className="diagnostics-summary engineering-tools-collapse"
                onClick={() => setBottomPane((current) => current === null ? lastBottomPaneRef.current : null)}
                type="button"
              >
                <span>
                {(bottomPane ?? lastBottomPaneRef.current) === "runtime"
                  ? runtimeSummary(snapshot.runtime)
                  : blockingCount > 0 ? `${blockingCount} blocking` : "No blocking issues"}
                </span>
                <span aria-hidden="true">{bottomPane === null ? "⌃" : "⌄"}</span>
              </button>
            </div>
            {bottomPane === "diagnostics" && (
              <div
                aria-labelledby="engineering-tools-tab-diagnostics"
                className="diagnostics-list"
                id="engineering-tools-panel-diagnostics"
                role="tabpanel"
              >
                <div role="list">
                {snapshot.diagnostics.length === 0 ? (
                  <div
                    aria-label="Canonical project state has no diagnostics."
                    className="empty-diagnostics"
                    role="status"
                  >
                    <span aria-hidden="true">✓</span>
                    Canonical project state has no diagnostics.
                  </div>
                ) : (
                  snapshot.diagnostics.map((diagnostic) => (
                    <button
                      className="diagnostic-row"
                      data-severity={diagnostic.severity}
                      key={diagnostic.diagnosticId}
                      onClick={() => {
                        if (diagnostic.objectId !== null) {
                          selectObject(diagnostic.objectId);
                        }
                      }}
                      role="listitem"
                      type="button"
                    >
                      <span className="diagnostic-severity">{diagnostic.severity}</span>
                      <code>{diagnostic.code}</code>
                      <span>{diagnostic.message}</span>
                    </button>
                  ))
                )}
                </div>
              </div>
            )}
            {bottomPane === "runtime" && (
              <div
                aria-labelledby="engineering-tools-tab-runtime"
                className="runtime-pane-scroll"
                id="engineering-tools-panel-runtime"
                role="tabpanel"
              >
                <VirtualTrainerTutorialProvider
                  target={tutorialStep === "press-start" || tutorialStep === "press-stop"
                    ? tutorialStep
                    : null}
                >
                  <RuntimeInspector
                    busy={busy}
                    onNavigate={selectObject}
                    onOperation={onRuntimeOperation}
                    onVerifyReplay={onVerifyReplay}
                    replayReceipt={replayReceipt}
                    runtime={snapshot.runtime}
                  />
                </VirtualTrainerTutorialProvider>
              </div>
            )}
          </section>
        </main>

        {selected !== undefined && (
          <PropertiesPane
            busy={busy}
            object={selected}
            onOperation={onOperation}
            projectRootId={snapshot.projectRootId}
          />
        )}
      </div>

      <footer className="status-bar">
        <span className="status-segment status-segment--safe">
          <span aria-hidden="true">◇</span> Virtual only
        </span>
        <span className="status-segment">{formatBuildState(snapshot.buildState)}</span>
        <span className="status-segment">{runtimeSummary(snapshot.runtime)}</span>
        <span className="status-spacer" />
        <span className="status-segment status-segment--hash" title={snapshot.projectHash}>
          Project {snapshot.projectHash.slice(0, 10)}
        </span>
        {busy && <span className="status-segment status-segment--busy">Working…</span>}
      </footer>

      {error !== null && (
        <div className="workbench-toast" role="alert">
          <strong>Command not completed</strong>
          <span>{error}</span>
        </div>
      )}
    </div>
  );
};

type TreeProps = Readonly<{
  objectId: string;
  objects: WorkbenchSnapshot["objects"];
  onSelect: (objectId: string) => void;
  selectedId: string;
}>;

const ProjectTree = ({ objectId, objects, onSelect, selectedId }: TreeProps): React.JSX.Element | null => {
  const object = objects[objectId];
  if (object === undefined || object.lifecycle !== "active") {
    return null;
  }
  const children = object.children
    .map((id) => objects[id])
    .filter((child): child is WorkbenchObjectView => child !== undefined && child.lifecycle === "active");

  return (
    <div className="tree-branch" role="group">
      <button
        aria-selected={selectedId === objectId}
        className="tree-row"
        data-selected={selectedId === objectId}
        onClick={() => onSelect(objectId)}
        role="treeitem"
        type="button"
      >
        <span className="tree-chevron" aria-hidden="true">{children.length > 0 ? "⌄" : ""}</span>
        <ObjectGlyph kind={object.kind} />
        <span className="tree-label">{object.displayName}</span>
      </button>
      {children.length > 0 && (
        <div className="tree-children">
          {children.map((child) => (
            <ProjectTree
              key={child.id}
              objectId={child.id}
              objects={objects}
              onSelect={onSelect}
              selectedId={selectedId}
            />
          ))}
        </div>
      )}
    </div>
  );
};

const ObjectGlyph = ({ kind }: Readonly<{ kind: WorkbenchObjectView["kind"] }>): React.JSX.Element => (
  <span className="object-glyph" data-kind={kind} aria-hidden="true">
    {kind === "ProjectRoot" ? "P" : kind === "Folder" ? "▰" : kind.slice(0, 2).toLocaleUpperCase("en-US")}
  </span>
);

type ProjectOverviewProps = Readonly<{
  activeCount: number;
  blockingCount: number;
  busy: boolean;
  canCreateLadderLab: boolean;
  ladderProgramName: string | null;
  onCreatePlc: (catalogId: VirtualPlcCatalogId) => Promise<void>;
  onLadderAction: () => void;
  onOpenObject: (objectId: string) => void;
  snapshot: WorkbenchSnapshot;
  tombstoneCount: number;
}>;

const ProjectOverview = ({
  activeCount,
  blockingCount,
  busy,
  canCreateLadderLab,
  ladderProgramName,
  onCreatePlc,
  onLadderAction,
  onOpenObject,
  snapshot,
  tombstoneCount,
}: ProjectOverviewProps): React.JSX.Element => {
  const ladderActionAvailable = ladderProgramName !== null || canCreateLadderLab;
  const projectDescription = snapshot.objects[snapshot.projectRootId]?.semanticPayload.description;
  return (
    <div className="overview-layout">
      <header className="editor-title">
        <p className="eyebrow">Learning workspace</p>
        <h1>{snapshot.projectName}</h1>
        <p>{typeof projectDescription === "string" && projectDescription.trim().length > 0
          ? projectDescription
          : "Build a ladder program, run it on the virtual PLC, and watch the logic respond."}</p>
      </header>
      <ProjectSetupPanel
        busy={busy}
        onCreatePlc={onCreatePlc}
        onOpenObject={onOpenObject}
        snapshot={snapshot}
      />
      {ladderActionAvailable && (
        <section className="learning-start-card" aria-labelledby="learning-start-title">
          <div className="learning-start-card__symbol" aria-hidden="true">
            <span>—| |—</span><span>—( )—</span>
          </div>
          <div className="learning-start-card__copy">
            <p className="action-kicker">Start here</p>
            <h2 id="learning-start-title">
              {ladderProgramName === null ? "Create your first ladder circuit" : "Continue your ladder program"}
            </h2>
            <p>
              {ladderProgramName === null
                ? "Set up a safe virtual PLC, Start and Stop pushbuttons, a motor output, and an editable MainCycle rung."
                : `${ladderProgramName} is ready for contacts, branches, coils, and live simulation.`}
            </p>
            <ol aria-label="Ladder learning steps">
              <li><strong>1</strong><span>Choose variables</span></li>
              <li><strong>2</strong><span>Build the rung</span></li>
              <li><strong>3</strong><span>Run and observe</span></li>
            </ol>
          </div>
          <button
            className="learning-start-card__action"
            data-tutorial-target="create-lab"
            disabled={busy}
            onClick={onLadderAction}
            type="button"
          >
            {busy
              ? "Setting up…"
              : ladderProgramName === null
                ? "Create motor starter lab"
                : `Open ${ladderProgramName}`}
            <span aria-hidden="true">→</span>
          </button>
        </section>
      )}
      <p className="overview-section-label">Engineering details</p>
    <div className="metric-grid">
      <article className="metric-card">
        <span>Active objects</span>
        <strong>{activeCount}</strong>
        <small>Stable identities</small>
      </article>
      <article className="metric-card">
        <span>Document revision</span>
        <strong>{snapshot.documentRevision}</strong>
        <small>All committed mutations</small>
      </article>
      <article className="metric-card">
        <span>Semantic revision</span>
        <strong>{snapshot.semanticRevision}</strong>
        <small>Build-affecting mutations</small>
      </article>
      <article className="metric-card" data-alert={blockingCount > 0}>
        <span>Blocking diagnostics</span>
        <strong>{blockingCount}</strong>
        <small>{tombstoneCount} retained tombstones</small>
      </article>
    </div>
    <section className="integrity-card">
      <div>
        <p className="action-kicker">Identity boundary</p>
        <h2>Project metadata</h2>
      </div>
      <dl className="identity-list">
        <div><dt>Project root</dt><dd>{snapshot.projectRootId}</dd></div>
        <div><dt>Document</dt><dd>{snapshot.documentId}</dd></div>
        <div><dt>Project hash</dt><dd>{snapshot.projectHash}</dd></div>
      </dl>
    </section>
    </div>
  );
};

const ObjectOverview = ({
  object,
  snapshot,
}: Readonly<{ object: WorkbenchObjectView; snapshot: WorkbenchSnapshot }>): React.JSX.Element => {
  const parent = object.parentId === null ? null : snapshot.objects[object.parentId];
  return (
    <div className="overview-layout">
      <header className="editor-title">
        <p className="eyebrow">{kindLabel[object.kind]}</p>
        <h1>{object.displayName}</h1>
        <p>This object is read directly from the canonical project graph.</p>
      </header>
      <section className="integrity-card">
        <div>
          <p className="action-kicker">Graph record</p>
          <h2>Object identity</h2>
        </div>
        <dl className="identity-list">
          <div><dt>Object ID</dt><dd>{object.id}</dd></div>
          <div><dt>Parent</dt><dd>{parent?.displayName ?? "Project root"}</dd></div>
          <div><dt>Creation ordinal</dt><dd>{object.creationOrdinal}</dd></div>
          <div><dt>Object revision</dt><dd>{object.objectRevision}</dd></div>
          <div><dt>Semantic revision</dt><dd>{object.semanticRevision}</dd></div>
        </dl>
      </section>
    </div>
  );
};

const isEditableMemberContainer = (object: WorkbenchObjectView): boolean =>
  object.kind === "NamedType" || object.kind === "GlobalDB";

type MemberTableEditorProps = Readonly<{
  busy: boolean;
  object: WorkbenchObjectView;
  onOperation: (operation: WorkbenchOperation) => Promise<void>;
  snapshot: WorkbenchSnapshot;
}>;

const PLC_IDENTIFIER = /^[A-Za-z_][A-Za-z0-9_]{0,127}$/u;

const MemberTableEditor = ({
  busy,
  object,
  onOperation,
  snapshot,
}: MemberTableEditorProps): React.JSX.Element => {
  const namedType = object.kind === "NamedType";
  const canonicalMembers = Array.isArray(object.semanticPayload.members)
    ? object.semanticPayload.members
    : [];
  const [members, setMembers] = useState<readonly ProjectPayloadValue[]>(canonicalMembers);
  useEffect(() => setMembers(canonicalMembers), [object.id, object.semanticRevision]);

  const namedTypeOptions = Object.values(snapshot.objects)
    .filter((candidate) =>
      candidate.lifecycle === "active" &&
      candidate.kind === "NamedType" &&
      candidate.id !== object.id
    )
    .sort((left, right) => left.displayName.localeCompare(right.displayName, "en-US"));
  const recognized = members.map((member) => canonicalRecordFields(member));
  const normalizedNames = recognized.flatMap((fields) =>
    fields !== null && typeof fields.name === "string"
      ? [fields.name.toLocaleLowerCase("en-US")]
      : []
  );
  const duplicateNames = new Set(
    normalizedNames.filter((name, index) => normalizedNames.indexOf(name) !== index),
  );
  const invalidNames = recognized.some((fields) =>
    fields !== null &&
    (typeof fields.name !== "string" ||
      !PLC_IDENTIFIER.test(fields.name) ||
      duplicateNames.has(fields.name.toLocaleLowerCase("en-US")))
  );
  const changed = JSON.stringify(members) !== JSON.stringify(canonicalMembers);

  const updateMember = (index: number, patch: ProjectPayload): void => {
    setMembers((current) => current.map((value, candidateIndex) => {
      if (candidateIndex !== index) {
        return value;
      }
      const fields = canonicalRecordFields(value);
      return fields === null ? value : recordValue({ ...fields, ...patch });
    }));
  };

  const addMember = (array: boolean): void => {
    const name = nextMemberName(members);
    const created = namedType
      ? createNamedTypeMemberPayload(
          name,
          members.length,
          array ? createArrayTypeExpression() : "DINT",
        )
      : createInterfaceMemberPayload(name, "static", members.length, "DINT");
    setMembers((current) => [...current, created]);
  };

  const apply = (): void => {
    if (busy || invalidNames || !changed) {
      return;
    }
    const orderKey = namedType ? "declaredOrder" : "order";
    const normalized = members.map((member, index) => {
      const fields = canonicalRecordFields(member);
      return fields === null
        ? member
        : recordValue({ ...fields, [orderKey]: unsignedValue(index) });
    });
    void onOperation({
      key: "members",
      kind: "project.set-semantic-field",
      objectId: object.id,
      value: normalized,
    });
  };

  return (
    <div className="member-editor">
      <header className="member-editor__header">
        <div>
          <p className="action-kicker">Canonical type and memory editor</p>
          <h1>{object.displayName}</h1>
          <p>
            {namedType
              ? "Define reusable structure members and bounded array fields. Stable member identities survive edits and saves."
              : "Define global memory members, PLC types, and retention policy on the canonical data block."}
          </p>
        </div>
        <div className="member-editor__actions">
          <button disabled={busy} onClick={() => addMember(false)} type="button">Add member</button>
          {namedType && (
            <button disabled={busy} onClick={() => addMember(true)} type="button">Add array</button>
          )}
        </div>
      </header>

      <div className="member-editor__table-wrap">
        <table className="member-editor__table">
          <thead>
            <tr>
              <th scope="col">Order</th>
              <th scope="col">Name</th>
              <th scope="col">Data type</th>
              {!namedType && <th className="member-editor__retain" scope="col">Retain</th>}
              <th scope="col"><span className="sr-only">Actions</span></th>
            </tr>
          </thead>
          <tbody>
            {members.length === 0 && (
              <tr className="member-editor__empty">
                <td colSpan={namedType ? 4 : 5}>No members. Add one to define this object.</td>
              </tr>
            )}
            {members.map((member, index) => {
              const fields = recognized[index] ?? null;
              if (fields === null) {
                return (
                  <tr data-invalid="true" key={`invalid-${index}`}>
                    <td>{index + 1}</td>
                    <td colSpan={namedType ? 2 : 3}>Unrecognized canonical member is retained unchanged.</td>
                    <td><RemoveMemberButton busy={busy} index={index} setMembers={setMembers} /></td>
                  </tr>
                );
              }
              const memberId = typeof fields.id === "string" ? fields.id : `invalid-${index}`;
              const name = typeof fields.name === "string" ? fields.name : "";
              const duplicate = duplicateNames.has(name.toLocaleLowerCase("en-US"));
              const typeKey = namedType ? "typeId" : "type";
              const typeValue = fields[typeKey];
              const typeRecord = canonicalRecordFields(typeValue);
              const isArray = namedType && typeRecord?.kind === "array";
              return (
                <tr data-invalid={!PLC_IDENTIFIER.test(name) || duplicate} key={memberId}>
                  <td><span className="member-editor__order">{index + 1}</span></td>
                  <td>
                    <label>
                      <span className="sr-only">Member {index + 1} name</span>
                      <input
                        aria-invalid={!PLC_IDENTIFIER.test(name) || duplicate}
                        disabled={busy}
                        maxLength={128}
                        onChange={(event) => updateMember(index, { name: event.target.value })}
                        spellCheck="false"
                        value={name}
                      />
                    </label>
                    {duplicate && <small>Names must be unique.</small>}
                    {!duplicate && name.length > 0 && !PLC_IDENTIFIER.test(name) && (
                      <small>Use a PLC identifier: letters, digits, and underscores.</small>
                    )}
                  </td>
                  <td>
                    {isArray && typeRecord !== null ? (
                      <ArrayTypeField
                        busy={busy}
                        namedTypeOptions={namedTypeOptions}
                        onChange={(value) => updateMember(index, { [typeKey]: value })}
                        typeRecord={typeRecord}
                      />
                    ) : (
                      <div className="member-editor__type-field">
                        <TypeSelect
                          busy={busy}
                          namedTypeOptions={namedTypeOptions}
                          onChange={(value) => updateMember(index, { [typeKey]: value })}
                          value={typeof typeValue === "string" ? typeValue : "DINT"}
                        />
                        {namedType && (
                          <button
                            className="member-editor__type-toggle"
                            disabled={busy}
                            onClick={() => updateMember(index, { [typeKey]: createArrayTypeExpression() })}
                            type="button"
                          >Array</button>
                        )}
                      </div>
                    )}
                  </td>
                  {!namedType && (
                    <td className="member-editor__retain">
                      <input
                        aria-label={`Retain ${name || `member ${index + 1}`}`}
                        checked={fields.retentive === true}
                        disabled={busy}
                        onChange={(event) => updateMember(index, { retentive: event.target.checked })}
                        type="checkbox"
                      />
                    </td>
                  )}
                  <td><RemoveMemberButton busy={busy} index={index} setMembers={setMembers} /></td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      <footer className="member-editor__footer">
        <div>
          <strong>{members.length} member{members.length === 1 ? "" : "s"}</strong>
          <span>{invalidNames ? "Resolve invalid or duplicate names before applying." : "Build validation reports unsupported or incompatible type use."}</span>
        </div>
        <button disabled={busy || invalidNames || !changed} onClick={apply} type="button">
          Apply member changes
        </button>
      </footer>
    </div>
  );
};

type TypeOptionObject = Pick<WorkbenchObjectView, "displayName" | "id">;

const TypeSelect = ({
  busy,
  namedTypeOptions,
  onChange,
  value,
}: Readonly<{
  busy: boolean;
  namedTypeOptions: readonly TypeOptionObject[];
  onChange: (value: string) => void;
  value: string;
}>): React.JSX.Element => {
  const admitted = new Set<string>([
    ...plcScalarTypeTokens,
    ...namedTypeOptions.map((candidate) => `TYPE:${candidate.id}`),
  ]);
  return (
    <select disabled={busy} onChange={(event) => onChange(event.target.value)} value={value}>
      {!admitted.has(value) && <option value={value}>{value} · unresolved</option>}
      <optgroup label="PLC scalar types">
        {plcScalarTypeTokens.map((token) => <option key={token} value={token}>{token}</option>)}
      </optgroup>
      {namedTypeOptions.length > 0 && (
        <optgroup label="Named types">
          {namedTypeOptions.map((candidate) => (
            <option key={candidate.id} value={`TYPE:${candidate.id}`}>{candidate.displayName}</option>
          ))}
        </optgroup>
      )}
    </select>
  );
};

const ArrayTypeField = ({
  busy,
  namedTypeOptions,
  onChange,
  typeRecord,
}: Readonly<{
  busy: boolean;
  namedTypeOptions: readonly TypeOptionObject[];
  onChange: (value: ProjectPayloadValue) => void;
  typeRecord: Readonly<Record<string, ProjectPayloadValue>>;
}>): React.JSX.Element => {
  const dimensions = Array.isArray(typeRecord.dimensions) ? typeRecord.dimensions : [];
  const bound = canonicalRecordFields(dimensions[0]);
  const lower = readCanonicalInteger(bound?.lower, 0);
  const upper = readCanonicalInteger(bound?.upper, 9);
  const elementType = typeof typeRecord.elementType === "string" ? typeRecord.elementType : "DINT";
  const update = (next: Readonly<{ elementType?: string; lower?: number; upper?: number }>): void => {
    onChange(createArrayTypeExpression(
      next.elementType ?? elementType,
      next.lower ?? lower,
      next.upper ?? upper,
    ));
  };
  return (
    <div className="member-editor__array-field">
      <div>
        <span>ARRAY</span>
        <TypeSelect
          busy={busy}
          namedTypeOptions={namedTypeOptions}
          onChange={(value) => update({ elementType: value })}
          value={elementType}
        />
      </div>
      <label>
        <span>Lower</span>
        <input disabled={busy} onChange={(event) => update({ lower: Number(event.target.value) })} type="number" value={lower} />
      </label>
      <label>
        <span>Upper</span>
        <input aria-invalid={upper < lower} disabled={busy} onChange={(event) => update({ upper: Number(event.target.value) })} type="number" value={upper} />
      </label>
      <button
        className="member-editor__type-toggle"
        disabled={busy}
        onClick={() => onChange(elementType)}
        type="button"
      >Scalar</button>
    </div>
  );
};

const RemoveMemberButton = ({
  busy,
  index,
  setMembers,
}: Readonly<{
  busy: boolean;
  index: number;
  setMembers: React.Dispatch<React.SetStateAction<readonly ProjectPayloadValue[]>>;
}>): React.JSX.Element => (
  <button
    aria-label={`Remove member ${index + 1}`}
    className="member-editor__remove"
    disabled={busy}
    onClick={() => setMembers((current) => current.filter((_, candidate) => candidate !== index))}
    title="Remove member"
    type="button"
  >×</button>
);

const readCanonicalInteger = (value: ProjectPayloadValue | undefined, fallback: number): number => {
  if (
    typeof value === "object" &&
    value !== null &&
    !Array.isArray(value) &&
    "$type" in value &&
    (value.$type === "i64" || value.$type === "u64") &&
    "value" in value &&
    typeof value.value === "string"
  ) {
    const parsed = Number(value.value);
    return Number.isSafeInteger(parsed) ? parsed : fallback;
  }
  return fallback;
};

const nextMemberName = (members: readonly ProjectPayloadValue[]): string => {
  const names = new Set(members.flatMap((member) => {
    const fields = canonicalRecordFields(member);
    return fields !== null && typeof fields.name === "string"
      ? [fields.name.toLocaleLowerCase("en-US")]
      : [];
  }));
  for (let suffix = 1; suffix <= 9_999; suffix += 1) {
    const candidate = suffix === 1 ? "Member" : `Member_${suffix}`;
    if (!names.has(candidate.toLocaleLowerCase("en-US"))) {
      return candidate;
    }
  }
  return `Member_${crypto.randomUUID().slice(0, 8)}`;
};

const isSclProgramBlock = (object: WorkbenchObjectView): boolean =>
  (object.kind === "OB" || object.kind === "FC" || object.kind === "FB") &&
  object.semanticPayload.language === "SCL";

const isGraphicalProgramBlock = (object: WorkbenchObjectView): boolean =>
  (object.kind === "OB" || object.kind === "FC" || object.kind === "FB") &&
  (object.semanticPayload.language === "LAD" || object.semanticPayload.language === "FBD");

type SclProgramEditorProps = Readonly<{
  busy: boolean;
  object: WorkbenchObjectView;
  onOperation: (operation: WorkbenchOperation) => Promise<void>;
}>;

const SclProgramEditor = ({
  busy,
  object,
  onOperation,
}: SclProgramEditorProps): React.JSX.Element => {
  const canonicalSource = typeof object.semanticPayload.sourceText === "string"
    ? object.semanticPayload.sourceText
    : "";
  const [source, setSource] = useState(canonicalSource);
  useEffect(() => setSource(canonicalSource), [canonicalSource, object.id]);
  const changed = source !== canonicalSource;

  const applySource = (): void => {
    if (!busy && changed) {
      void onOperation({
        key: "sourceText",
        kind: "project.set-semantic-field",
        objectId: object.id,
        value: source,
      });
    }
  };

  return (
    <div className="scl-editor">
      <header className="scl-editor__header">
        <div>
          <p className="action-kicker">Semantic text editor</p>
          <h1>{object.displayName}</h1>
          <p>SCL source is stored on this canonical block identity and compiled by the shared PLC pipeline.</p>
        </div>
        <div className="scl-editor__identity">
          <span>{object.kind}</span>
          <code>r{object.semanticRevision}</code>
        </div>
      </header>
      <label className="scl-editor__field">
        <span>SCL source</span>
        <textarea
          aria-describedby="scl-source-help"
          disabled={busy}
          maxLength={1_048_576}
          onChange={(event) => setSource(event.target.value)}
          onKeyDown={(event) => {
            if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
              event.preventDefault();
              applySource();
            }
          }}
          placeholder={"WorkingValue := 7;\nOutputValue := InputValue;"}
          spellCheck="false"
          value={source}
        />
      </label>
      <footer className="scl-editor__footer">
        <span id="scl-source-help">
          Ctrl+Enter applies source to the canonical project. Build diagnostics will retain text anchors.
        </span>
        <span>{source.length.toLocaleString("en-US")} characters</span>
        <button
          disabled={busy || !changed}
          onClick={applySource}
          type="button"
        >
          Apply SCL source
        </button>
      </footer>
    </div>
  );
};

type GraphInterfaceMember = Readonly<{
  dataType: string;
  id: string;
  name: string;
  operandLabel?: string;
  role: string;
  writable: boolean;
}>;

type LadInsertionTarget = Readonly<{
  edgeId: string;
  networkId: string;
  rungNumber: number;
}>;

const LAD_PALETTE_GROUPS = ["Move", "Compare", "Math", "Edges", "Timers", "Counters"] as const;

type GraphicalProgramEditorProps = Readonly<{
  busy: boolean;
  ladderFocus: boolean;
  object: WorkbenchObjectView;
  onOperation: (operation: WorkbenchOperation) => Promise<void>;
  onStartSimulation: () => Promise<void>;
  onToggleLadderFocus: () => void;
  onTutorialMilestone: (milestone: "seal-in-added" | "stop-contact-added") => void;
  snapshot: WorkbenchSnapshot;
}>;

const GraphicalProgramEditor = ({
  busy,
  ladderFocus,
  object,
  onOperation,
  onStartSimulation,
  onToggleLadderFocus,
  onTutorialMilestone,
  snapshot,
}: GraphicalProgramEditorProps): React.JSX.Element => {
  const language = object.semanticPayload.language === "FBD" ? "FBD" : "LAD";
  const graph = object.semanticPayload.graph;
  const graphRecord = canonicalRecordFields(graph);
  const networks = graphRecord !== null && Array.isArray(graphRecord.networks)
    ? graphRecord.networks
    : [];
  const members = readGraphInterfaceMembers(object.semanticPayload);
  const boundTags = Object.values(snapshot.objects).filter((candidate) =>
    candidate.lifecycle === "active" &&
    candidate.kind === "Tag" &&
    candidate.semanticPayload.blockId === object.id &&
    typeof candidate.semanticPayload.memberId === "string"
  );
  const tagByMemberId = new Map(
    boundTags.map((tag) => [String(tag.semanticPayload.memberId), tag] as const),
  );
  const displayedMembers = members.map((member) => {
    const tag = tagByMemberId.get(member.id);
    return tag === undefined
      ? member
      : {
          ...member,
          operandLabel: tag.displayName === member.name
            ? member.name
            : `${tag.displayName} (${member.name})`,
          writable: tag.semanticPayload.addressArea !== "I",
        };
  });
  const readableBooleanMembers = displayedMembers.filter((member) => member.dataType === "BOOL");
  const writableBooleanMembers = readableBooleanMembers.filter((member) => member.writable);
  const preferredCoilMember = writableBooleanMembers.find((member) =>
    tagByMemberId.get(member.id)?.semanticPayload.addressArea === "Q"
  ) ?? writableBooleanMembers.at(-1);
  const liveResult = language === "LAD"
    ? projectLadLiveMonitoring(snapshot, object.id)
    : null;
  const liveMembers = new Map(
    liveResult?.ok === true
      ? liveResult.projection.members.map((member) => [member.memberId, member] as const)
      : [],
  );
  const [ladAuthoringError, setLadAuthoringError] = useState<string | null>(null);
  const [ladInsertTarget, setLadInsertTarget] = useState<LadInsertionTarget | null>(null);
  const [ladPaletteMemberId, setLadPaletteMemberId] = useState(readableBooleanMembers[0]?.id ?? "");
  const [ladPaletteInstructionKey, setLadPaletteInstructionKey] = useState<MvpLadInstructionKey>("add");
  const [ladGuideTargetNodeId, setLadGuideTargetNodeId] = useState<string | null>(null);
  const [ladGuidePrompt, setLadGuidePrompt] = useState<string | null>(null);
  const [ladSelectedNetworkId, setLadSelectedNetworkId] = useState<string | null>(null);
  const [ladClipboardNetworkId, setLadClipboardNetworkId] = useState<string | null>(null);
  const firstNetwork = canonicalRecordFields(networks[0]);
  const firstTopology = firstNetwork === null ? null : projectLadNetworkTopology(firstNetwork);
  const selectedPaletteInstruction = getMvpLadInstruction(ladPaletteInstructionKey);
  const motorGuide = language === "LAD" && firstTopology?.ok === true
    ? projectMotorStarterGuide(firstTopology.topology, readableBooleanMembers)
    : null;
  useEffect(() => {
    if (motorGuide?.hasStopContact === true) {
      onTutorialMilestone("stop-contact-added");
    }
    if (motorGuide?.hasSealInBranch === true) {
      onTutorialMilestone("seal-in-added");
    }
  }, [motorGuide?.hasSealInBranch, motorGuide?.hasStopContact, onTutorialMilestone]);
  useEffect(() => {
    setLadAuthoringError(null);
    setLadInsertTarget(null);
    setLadGuideTargetNodeId(null);
    setLadGuidePrompt(null);
    setLadSelectedNetworkId((current) => {
      const stillExists = networks.some((value) => canonicalRecordFields(value)?.id === current);
      return stillExists ? current : typeof firstNetwork?.id === "string" ? firstNetwork.id : null;
    });
    setLadClipboardNetworkId((current) =>
      networks.some((value) => canonicalRecordFields(value)?.id === current) ? current : null
    );
    setLadPaletteMemberId((current) =>
      readableBooleanMembers.some((member) => member.id === current)
        ? current
        : readableBooleanMembers[0]?.id ?? ""
    );
  }, [object.id, object.semanticRevision]);

  const handleLadKeyboard = (event: React.KeyboardEvent<HTMLDivElement>): void => {
    if (language !== "LAD" || busy || graph === undefined) {
      return;
    }
    const target = event.target;
    const editing = target instanceof HTMLInputElement ||
      target instanceof HTMLTextAreaElement ||
      target instanceof HTMLSelectElement ||
      (target instanceof HTMLElement && target.isContentEditable);
    if (editing) {
      return;
    }
    const key = event.key.toLocaleLowerCase("en-US");
    if ((event.ctrlKey || event.metaKey) && key === "c" && ladSelectedNetworkId !== null) {
      event.preventDefault();
      event.stopPropagation();
      setLadClipboardNetworkId(ladSelectedNetworkId);
      setLadGuidePrompt("Rung copied. Press Ctrl+V or use Duplicate rung to paste a new independent copy.");
      return;
    }
    if ((event.ctrlKey || event.metaKey) && key === "v" && ladClipboardNetworkId !== null) {
      event.preventDefault();
      event.stopPropagation();
      void duplicateNetworkSafely(ladClipboardNetworkId);
      return;
    }
    if (event.key === "Delete" && ladSelectedNetworkId !== null) {
      event.preventDefault();
      event.stopPropagation();
      commitLadMutation(removeLadNetwork(graph, { networkId: ladSelectedNetworkId }));
    }
  };

  const commitGraph = (updated: ProjectPayloadValue): void => {
    if (busy) {
      return;
    }
    void onOperation({
      key: "graph",
      kind: "project.set-semantic-field",
      objectId: object.id,
      value: updated,
    });
  };

  const commitLadMutation = (result: LadAuthoringResult | LadBoxAuthoringResult): void => {
    if (result.ok === false) {
      setLadAuthoringError(result.message);
      return;
    }
    setLadAuthoringError(null);
    setLadInsertTarget(null);
    setLadGuideTargetNodeId(null);
    setLadGuidePrompt(null);
    commitGraph(result.graph);
  };

  const duplicateNetworkSafely = async (networkId: string): Promise<void> => {
    if (busy || graph === undefined) {
      return;
    }
    const plan = createSafeLadRungDuplicationPlan(snapshot, object, graph, { networkId });
    if (plan.ok === false) {
      setLadAuthoringError(plan.message);
      return;
    }
    setLadAuthoringError(null);
    try {
      for (const operation of plan.operations) {
        await onOperation(operation);
      }
      setLadSelectedNetworkId(plan.createdNetworkId);
      setLadInsertTarget(null);
    } catch (caught) {
      setLadAuthoringError(caught instanceof Error ? caught.message : "The rung could not be duplicated.");
    }
  };

  const prepareStopContact = (): void => {
    if (
      motorGuide?.available !== true ||
      motorGuide.stopInsertionEdgeId === null ||
      motorGuide.stopMemberId === null ||
      firstTopology?.ok !== true
    ) {
      return;
    }
    setLadPaletteMemberId(motorGuide.stopMemberId);
    setLadInsertTarget({
      edgeId: motorGuide.stopInsertionEdgeId,
      networkId: firstTopology.topology.networkId,
      rungNumber: 1,
    });
    setLadGuideTargetNodeId(null);
    setLadGuidePrompt("Stop_PB and its rung position are selected. Click NC contact in the instruction palette.");
  };

  const prepareSealInContact = (): void => {
    if (motorGuide?.available !== true || motorGuide.motorMemberId === null) {
      return;
    }
    setLadPaletteMemberId(motorGuide.motorMemberId);
    setLadInsertTarget(null);
    setLadGuideTargetNodeId(motorGuide.startContactNodeId);
    setLadGuidePrompt("Motor_Run is selected. On the Start_PB instruction below, click Parallel + Motor_Run.");
  };

  const insertPaletteContact = (mode: "normally-closed" | "normally-open"): void => {
    if (graph === undefined || ladInsertTarget === null || ladPaletteMemberId.length === 0) {
      return;
    }
    commitLadMutation(insertSeriesContact(graph, {
      edgeId: ladInsertTarget.edgeId,
      memberId: ladPaletteMemberId,
      mode,
      networkId: ladInsertTarget.networkId,
    }));
  };

  const insertPaletteInstruction = async (): Promise<void> => {
    if (graph === undefined || ladInsertTarget === null || busy) {
      return;
    }
    const instruction = getMvpLadInstruction(ladPaletteInstructionKey);
    try {
      const statePlan = instruction.stateKind === null
        ? null
        : createLadStateStoragePlan(snapshot, object, instruction.key);
      if (statePlan !== null) {
        for (const operation of statePlan.operations) {
          await onOperation(operation);
        }
      }
      const result = insertMvpLadInstructionBoxOnEdge(graph, {
        bindings: defaultLadInstructionBindings(instruction, displayedMembers, "DINT"),
        edgeId: ladInsertTarget.edgeId,
        instruction: instruction.key,
        networkId: ladInsertTarget.networkId,
        ...(statePlan === null ? {} : { stateBinding: statePlan.stateBinding }),
        valueDataType: "DINT",
      });
      commitLadMutation(result);
    } catch (caught) {
      setLadAuthoringError(caught instanceof Error ? caught.message : "The instruction could not be inserted.");
    }
  };

  const commitNodeFields = (
    nodeId: string,
    fields: ProjectPayload,
  ): void => {
    if (busy || graph === undefined) {
      return;
    }
    const updated = updateGraphNodeFields(graph, nodeId, fields);
    if (updated !== null) {
      commitGraph(updated);
    }
  };

  return (
    <div
      className="graph-editor"
      data-language={language}
      onKeyDown={handleLadKeyboard}
    >
      <header className="graph-editor__header">
        <div>
          <p className="action-kicker">{language === "LAD" ? "Ladder program" : "Function block diagram"}</p>
          <h1>{object.displayName}</h1>
          <p>
            {language === "LAD"
              ? "Build each rung from left to right. Contacts test conditions; the coil controls the result."
              : "Typed ports and data dependencies are stored independently from this visual arrangement."}
          </p>
        </div>
        <div className="graph-editor__identity">
          <span>{language}</span>
          <code>{networks.length} {language === "LAD" ? "rung" : "network"}{networks.length === 1 ? "" : "s"}</code>
          {language === "LAD" && liveResult?.ok === true && (
            <span
              className="graph-editor__live-state"
              data-live={liveResult.projection.monitorState === "ACTIVE"}
              title={`CPU ${liveResult.projection.cpuState ?? "unavailable"} · scan ${liveResult.projection.scanSequence ?? "—"}`}
            >
              {liveResult.projection.monitorState === "ACTIVE"
                ? `Live · scan ${liveResult.projection.scanSequence ?? "—"}`
                : "Monitoring off"}
            </span>
          )}
          {language === "LAD" && (
            <button
              aria-controls="main-content"
              aria-label={ladderFocus ? "Restore workbench layout" : "Expand ladder editor"}
              aria-pressed={ladderFocus}
              className="graph-editor__focus"
              onClick={onToggleLadderFocus}
              title="Expand the ladder canvas (Ctrl+Shift+F)"
              type="button"
            >
              <span aria-hidden="true">{ladderFocus ? "↙" : "↗"}</span>
              {ladderFocus ? "Restore layout" : "Expand ladder"}
            </button>
          )}
        </div>
      </header>

      {language === "LAD" && (
        <div className="lad-workspace-tools">
          {motorGuide?.available === true && (
            <MotorStarterCoach
              busy={busy}
              guide={motorGuide}
              motorLive={motorGuide.motorMemberId === null ? null : liveMembers.get(motorGuide.motorMemberId) ?? null}
              onPrepareSealIn={prepareSealInContact}
              onPrepareStop={prepareStopContact}
              onStartSimulation={onStartSimulation}
              prompt={ladGuidePrompt}
              simulationRunning={snapshot.runtime.session?.online === true && snapshot.runtime.session.cpuState === "RUN"}
            />
          )}
          <section className="lad-instruction-palette" aria-label="Ladder instructions">
            <div className="lad-instruction-palette__intro">
              <span>Instructions</span>
              <strong>
                {ladInsertTarget === null
                  ? "Choose a blue insert point on a rung"
                  : `Insert point selected · Rung ${ladInsertTarget.rungNumber}`}
              </strong>
            </div>
            <label className="lad-instruction-palette__operand">
              <span>Variable</span>
              <select
                aria-label="Instruction variable"
                disabled={busy || readableBooleanMembers.length === 0}
                onChange={(event) => setLadPaletteMemberId(event.target.value)}
                value={ladPaletteMemberId}
              >
                {memberOptions(readableBooleanMembers, ladPaletteMemberId)}
              </select>
            </label>
            <div className="lad-instruction-palette__buttons">
              <button
                aria-label="NO contact"
                disabled={busy || ladInsertTarget === null || ladPaletteMemberId.length === 0}
                onClick={() => insertPaletteContact("normally-open")}
                title={ladInsertTarget === null ? "Choose an insert point first" : "Insert a normally open contact"}
                type="button"
              >
                <span aria-hidden="true">—| |—</span>
                <strong>NO contact</strong>
              </button>
              <button
                aria-label="NC contact"
                data-tutorial-target="add-stop-nc"
                disabled={busy || ladInsertTarget === null || ladPaletteMemberId.length === 0}
                onClick={() => insertPaletteContact("normally-closed")}
                title={ladInsertTarget === null ? "Choose an insert point first" : "Insert a normally closed contact"}
                type="button"
              >
                <span aria-hidden="true">—|/|—</span>
                <strong>NC contact</strong>
              </button>
              <button
                aria-label="New rung"
                disabled={busy || graph === undefined || preferredCoilMember === undefined}
                onClick={() => {
                  if (graph !== undefined && preferredCoilMember !== undefined) {
                    commitLadMutation(addLadNetwork(graph, { coilMemberId: preferredCoilMember.id }));
                  }
                }}
                title={preferredCoilMember === undefined
                  ? "Add a BOOL variable before creating another rung"
                  : `Add a rung with ${preferredCoilMember.operandLabel ?? preferredCoilMember.name} as its coil`}
                type="button"
              >
                <span aria-hidden="true">—( )—</span>
                <strong>New rung</strong>
              </button>
            </div>
            <div className="lad-box-palette">
              <label>
                <span>Instruction box</span>
                <select
                  aria-label="Instruction box"
                  disabled={busy}
                  onChange={(event) => setLadPaletteInstructionKey(event.target.value as MvpLadInstructionKey)}
                  value={ladPaletteInstructionKey}
                >
                  {LAD_PALETTE_GROUPS.map((group) => (
                    <optgroup key={group} label={group}>
                      {MVP_LAD_INSTRUCTION_CATALOG.filter((instruction) => instruction.group === group).map((instruction) => (
                        <option key={instruction.key} value={instruction.key}>
                          {instruction.mnemonic} · {instruction.learning.title}
                        </option>
                      ))}
                    </optgroup>
                  ))}
                </select>
              </label>
              <div className="lad-box-palette__lesson">
                <strong>{selectedPaletteInstruction.learning.title}</strong>
                <span>{selectedPaletteInstruction.learning.plainLanguage}</span>
                <small>{selectedPaletteInstruction.learning.tip}</small>
              </div>
              <button
                className="primary-button"
                disabled={busy || graph === undefined || ladInsertTarget === null}
                onClick={() => void insertPaletteInstruction()}
                title={ladInsertTarget === null ? "Choose a blue insert point first" : `Insert ${selectedPaletteInstruction.mnemonic}`}
                type="button"
              >
                Insert {selectedPaletteInstruction.mnemonic}
              </button>
            </div>
          </section>
          <LadVariablesEditor
            boundTags={tagByMemberId}
            busy={busy}
            object={object}
            onOperation={onOperation}
          />
        </div>
      )}

      {ladAuthoringError !== null && (
        <div className="graph-editor__authoring-error" role="alert">
          {ladAuthoringError}
        </div>
      )}

      {networks.length === 0 ? (
        <div className="graph-editor__invalid" role="alert">
          This block has no valid canonical {language} network to edit.
        </div>
      ) : (
        <div className="graph-editor__networks">
          {networks.map((networkValue, networkIndex) => {
            const network = canonicalRecordFields(networkValue);
            if (network === null || !Array.isArray(network.nodes)) {
              return (
                <div className="graph-editor__invalid" key={`invalid-${networkIndex}`} role="alert">
                  Network {networkIndex + 1} is malformed and will not compile.
                </div>
              );
            }
            return language === "LAD" ? (
              <LadderNetworkEditor
                busy={busy}
                graph={graph}
                guideTargetNodeId={ladGuideTargetNodeId}
                instructionMembers={displayedMembers}
                key={String(network.id ?? networkIndex)}
                liveMembers={liveMembers}
                members={readableBooleanMembers}
                network={network}
                networkCount={networks.length}
                networkIndex={networkIndex}
                onDuplicateNetwork={(networkId) => void duplicateNetworkSafely(networkId)}
                onMutation={commitLadMutation}
                onSelectNetwork={setLadSelectedNetworkId}
                onSelectInsertTarget={setLadInsertTarget}
                paletteMemberId={ladPaletteMemberId}
                selectedEdgeId={ladInsertTarget?.edgeId ?? null}
                selectedNetworkId={ladSelectedNetworkId}
                writableMembers={writableBooleanMembers}
              />
            ) : (
              <FbdNetworkEditor
                busy={busy}
                commitNodeFields={commitNodeFields}
                key={String(network.id ?? networkIndex)}
                members={members}
                network={network}
                networkIndex={networkIndex}
              />
            );
          })}
        </div>
      )}

      <footer className="graph-editor__footer">
        <span>Select an insert point, choose a variable, then add a contact.</span>
        <span>Use Add parallel on a contact to begin a seal-in branch.</span>
      </footer>
    </div>
  );
};

const defaultLadInstructionBindings = (
  instruction: MvpLadInstructionDefinition,
  members: readonly GraphInterfaceMember[],
  valueDataType: PlcScalarTypeToken,
): Readonly<Record<string, CanonicalLadPinBinding | null>> => {
  const bindings: Record<string, CanonicalLadPinBinding | null> = {};
  const resolvedTypes = new Map<number, PlcScalarTypeToken>();
  const usedMemberIds = new Set<string>();
  for (const formal of instruction.formals.filter((candidate) => candidate.surface === "data-pin")) {
    const dataType = (() => {
      switch (formal.typeConstraint.kind) {
        case "exact": return formal.typeConstraint.dataType;
        case "any-value":
        case "numeric": return valueDataType;
        case "same-as": return resolvedTypes.get(formal.typeConstraint.formalId) ?? valueDataType;
        case "instruction-state": return valueDataType;
      }
    })();
    resolvedTypes.set(formal.id, dataType);
    if (formal.name === "PT") {
      bindings[formal.name] = { dataType: "TIME", kind: "constant", value: signedValue(1_000) };
      continue;
    }
    if (formal.name === "PV") {
      bindings[formal.name] = { dataType: "DINT", kind: "constant", value: signedValue(10) };
      continue;
    }
    const compatible = members.filter((member) =>
      member.dataType === dataType &&
      (formal.lvalue === "value" || member.writable)
    );
    const selected = compatible.find((member) => !usedMemberIds.has(member.id)) ?? compatible[0];
    if (selected !== undefined) {
      usedMemberIds.add(selected.id);
    }
    bindings[formal.name] = selected === undefined
      ? null
      : { kind: "caller-member", memberId: selected.id };
  }
  return bindings;
};

const MotorStarterCoach = ({
  busy,
  guide,
  motorLive,
  onPrepareSealIn,
  onPrepareStop,
  onStartSimulation,
  prompt,
  simulationRunning,
}: Readonly<{
  busy: boolean;
  guide: MotorStarterGuideProjection;
  motorLive: LadBooleanMemberLiveState | null;
  onPrepareSealIn: () => void;
  onPrepareStop: () => void;
  onStartSimulation: () => Promise<void>;
  prompt: string | null;
  simulationRunning: boolean;
}>): React.JSX.Element => {
  const completed = Number(guide.hasStopContact) + Number(guide.hasSealInBranch) + Number(simulationRunning);
  const motorState = motorLive?.truth === "on"
    ? "Motor output is ON"
    : motorLive?.truth === "off"
      ? "Motor output is OFF"
      : "Motor output appears after simulation starts";
  return (
    <section aria-labelledby="motor-starter-coach-title" className="motor-starter-coach">
      <header>
        <div>
          <span>Guided lab · Motor starter</span>
          <h2 id="motor-starter-coach-title">Build a Start / Stop seal-in circuit</h2>
          <p>You place each instruction. The coach selects the right variable and position, then explains what the circuit does.</p>
        </div>
        <strong aria-label={`${completed} of 3 lab stages complete`}>{completed}/3</strong>
      </header>
      <ol>
        <li data-complete={guide.hasStopContact}>
          <span aria-hidden="true">{guide.hasStopContact ? "✓" : "1"}</span>
          <div>
            <strong>Stop the whole rung safely</strong>
            <small>Put Stop_PB normally closed in series so pressing Stop interrupts every path.</small>
          </div>
          {!guide.hasStopContact && (
            <button data-tutorial-target="select-stop" disabled={busy} onClick={onPrepareStop} type="button">Select Stop instruction</button>
          )}
        </li>
        <li data-complete={guide.hasSealInBranch}>
          <span aria-hidden="true">{guide.hasSealInBranch ? "✓" : "2"}</span>
          <div>
            <strong>Keep the motor on after Start is released</strong>
            <small>Add a Motor_Run normally open contact in parallel with Start_PB.</small>
          </div>
          {!guide.hasSealInBranch && guide.hasStopContact && (
            <button data-tutorial-target="select-seal-in" disabled={busy} onClick={onPrepareSealIn} type="button">Select seal-in contact</button>
          )}
        </li>
        <li data-complete={simulationRunning}>
          <span aria-hidden="true">{simulationRunning ? "✓" : "3"}</span>
          <div>
            <strong>Prove the sequence on the trainer</strong>
            <small>{simulationRunning ? motorState : "Start the virtual PLC, press Start, then press Stop."}</small>
          </div>
          {guide.complete && !simulationRunning && (
            <button data-tutorial-target="start-simulation" disabled={busy} onClick={() => void onStartSimulation()} type="button">Start simulation</button>
          )}
        </li>
      </ol>
      {prompt !== null && <p className="motor-starter-coach__prompt" role="status">{prompt}</p>}
      {guide.complete && (
        <p className="motor-starter-coach__explanation">
          Stop_PB is in series. Start_PB and Motor_Run share parallel paths. When the coil turns on, its own contact closes and holds the path until Stop_PB opens it.
        </p>
      )}
    </section>
  );
};

type LadVariablesEditorProps = Readonly<{
  boundTags: ReadonlyMap<string, WorkbenchObjectView>;
  busy: boolean;
  object: WorkbenchObjectView;
  onOperation: (operation: WorkbenchOperation) => Promise<void>;
}>;

const LadVariablesEditor = ({
  boundTags,
  busy,
  object,
  onOperation,
}: LadVariablesEditorProps): React.JSX.Element => {
  const canonical = Array.isArray(object.semanticPayload.interface)
    ? object.semanticPayload.interface
    : [];
  const [drafts, setDrafts] = useState<readonly ProjectPayloadValue[]>(canonical);
  useEffect(() => setDrafts(canonical), [object.id, object.semanticRevision]);

  const rows = drafts.map((value) => canonicalRecordFields(value));
  const normalizedNames = rows.flatMap((row) =>
    row !== null && typeof row.name === "string"
      ? [row.name.toLocaleLowerCase("en-US")]
      : []
  );
  const duplicateNames = new Set(
    normalizedNames.filter((name, index) => normalizedNames.indexOf(name) !== index),
  );
  const invalid = rows.some((row) =>
    row === null ||
    typeof row.name !== "string" ||
    !PLC_IDENTIFIER.test(row.name) ||
    duplicateNames.has(row.name.toLocaleLowerCase("en-US"))
  );
  const changed = JSON.stringify(drafts) !== JSON.stringify(canonical);

  const updateName = (index: number, name: string): void => {
    setDrafts((current) => current.map((value, candidateIndex) => {
      if (candidateIndex !== index) {
        return value;
      }
      const fields = canonicalRecordFields(value);
      return fields === null ? value : recordValue({ ...fields, name });
    }));
  };

  const updateType = (index: number, dataType: "BOOL" | "DINT" | "TIME"): void => {
    setDrafts((current) => current.map((value, candidateIndex) => {
      if (candidateIndex !== index) {
        return value;
      }
      const fields = canonicalRecordFields(value);
      return fields === null ? value : recordValue({ ...fields, type: dataType });
    }));
  };

  return (
    <details className="lad-variables">
      <summary>
        <span>Program variables</span>
        <strong>{rows.length}</strong>
        <small>BOOL for contacts; DINT/TIME for instruction boxes</small>
      </summary>
      <div className="lad-variables__body">
        <div className="lad-variables__rows">
          {rows.map((row, index) => {
            const id = typeof row?.id === "string" ? row.id : `invalid-${index}`;
            const name = typeof row?.name === "string" ? row.name : "";
            const dataType = typeof row?.type === "string" ? row.type : "?";
            const tag = boundTags.get(id);
            const invalidName = !PLC_IDENTIFIER.test(name) ||
              duplicateNames.has(name.toLocaleLowerCase("en-US"));
            return (
              <div className="lad-variables__row" key={id}>
                <label>
                  <span>Variable {index + 1}</span>
                  <input
                    aria-invalid={invalidName}
                    aria-label={`Variable ${index + 1} name`}
                    disabled={busy}
                    onChange={(event) => updateName(index, event.target.value)}
                    spellCheck={false}
                    value={name}
                  />
                </label>
                <label>
                  <span>Data type</span>
                  <select
                    aria-label={`Variable ${index + 1} data type`}
                    disabled={busy || tag !== undefined}
                    onChange={(event) => updateType(
                      index,
                      event.target.value as "BOOL" | "DINT" | "TIME",
                    )}
                    value={dataType}
                  >
                    {!(["BOOL", "DINT", "TIME"] as const).includes(dataType as "BOOL" | "DINT" | "TIME") && (
                      <option value={dataType}>{dataType}</option>
                    )}
                    <option value="BOOL">BOOL</option>
                    <option value="DINT">DINT</option>
                    <option value="TIME">TIME</option>
                  </select>
                </label>
                <span className="lad-variables__binding" data-bound={tag !== undefined}>
                  {tag === undefined
                    ? "Program variable"
                    : `${String(tag.semanticPayload.addressArea ?? "Tag")} · ${tag.displayName}`}
                </span>
              </div>
            );
          })}
        </div>
        <div className="lad-variables__actions">
          <button
            disabled={busy}
            onClick={() => setDrafts((current) => [
              ...current,
              createInterfaceMemberPayload(nextLadVariableName(current), "temp", current.length, "BOOL"),
            ])}
            type="button"
          >
            + Add variable
          </button>
          <span aria-live="polite">
            {invalid ? "Use unique PLC names: letters, numbers, and underscores." : "Renaming keeps rung connections intact."}
          </span>
          <button
            className="primary-button"
            disabled={busy || invalid || !changed}
            onClick={() => void onOperation({
              key: "interface",
              kind: "project.set-semantic-field",
              objectId: object.id,
              value: drafts,
            })}
            type="button"
          >
            Apply variables
          </button>
        </div>
      </div>
    </details>
  );
};

const nextLadVariableName = (members: readonly ProjectPayloadValue[]): string => {
  const names = new Set(members.flatMap((value) => {
    const fields = canonicalRecordFields(value);
    return fields !== null && typeof fields.name === "string"
      ? [fields.name.toLocaleLowerCase("en-US")]
      : [];
  }));
  for (let suffix = 1; suffix <= 9_999; suffix += 1) {
    const candidate = `Bool_${suffix}`;
    if (!names.has(candidate.toLocaleLowerCase("en-US"))) {
      return candidate;
    }
  }
  return `Bool_${crypto.randomUUID().slice(0, 8)}`;
};

type GraphNetworkEditorProps = Readonly<{
  busy: boolean;
  commitNodeFields: (nodeId: string, fields: ProjectPayload) => void;
  members: readonly GraphInterfaceMember[];
  network: ProjectPayload;
  networkIndex: number;
}>;

type LadderNetworkEditorProps = Readonly<{
  busy: boolean;
  graph: ProjectPayloadValue | undefined;
  guideTargetNodeId: string | null;
  instructionMembers: readonly GraphInterfaceMember[];
  liveMembers: ReadonlyMap<string, LadBooleanMemberLiveState>;
  members: readonly GraphInterfaceMember[];
  network: ProjectPayload;
  networkCount: number;
  networkIndex: number;
  onDuplicateNetwork: (networkId: string) => void;
  onMutation: (result: LadAuthoringResult | LadBoxAuthoringResult) => void;
  onSelectNetwork: (networkId: string) => void;
  onSelectInsertTarget: (target: LadInsertionTarget) => void;
  paletteMemberId: string;
  selectedEdgeId: string | null;
  selectedNetworkId: string | null;
  writableMembers: readonly GraphInterfaceMember[];
}>;

const LadderNetworkEditor = ({
  busy,
  graph,
  guideTargetNodeId,
  instructionMembers,
  liveMembers,
  members,
  network,
  networkCount,
  networkIndex,
  onDuplicateNetwork,
  onMutation,
  onSelectNetwork,
  onSelectInsertTarget,
  paletteMemberId,
  selectedEdgeId,
  selectedNetworkId,
  writableMembers,
}: LadderNetworkEditorProps): React.JSX.Element => {
  const nodes = Array.isArray(network.nodes)
    ? network.nodes.map(canonicalRecordFields).filter((node): node is ProjectPayload => node !== null)
    : [];
  const edgeCount = Array.isArray(network.edges) ? network.edges.length : 0;
  const instructionCount = nodes.filter((node) =>
    !["power-source", "branch-split", "branch-join"].includes(String(node.nodeKind)),
  ).length;
  const networkId = typeof network.id === "string" ? network.id : null;
  const projection = projectLadNetworkTopology(network);

  if (graph === undefined || networkId === null || projection.ok === false) {
    const reason = projection.ok === false
      ? projection.message
      : "The canonical LAD document or network identity is unavailable.";
    return (
      <section className="lad-network" aria-label={`LAD network ${networkIndex + 1}`}>
        <div className="graph-network__heading">
          <span>Rung {networkIndex + 1}</span>
          <code>Invalid topology</code>
        </div>
        <div className="graph-editor__invalid" role="alert">{reason}</div>
      </section>
    );
  }

  const powerFlow = projectLadPowerFlow(projection.topology, liveMembers);

  const renderContext: LadRenderContext = {
    busy,
    graph,
    guideTargetNodeId,
    instructionMembers,
    liveMembers,
    members,
    networkId,
    onMutation,
    onSelectInsertTarget,
    paletteMemberId,
    powerFlow,
    rungNumber: networkIndex + 1,
    selectedEdgeId,
    writableMembers,
  };
  return (
    <section
      aria-label={`LAD network ${networkIndex + 1}`}
      className="lad-network"
      data-selected={selectedNetworkId === networkId}
      onClick={() => onSelectNetwork(networkId)}
      onFocusCapture={() => onSelectNetwork(networkId)}
      tabIndex={0}
    >
      <div className="graph-network__heading">
        <span>Rung {networkIndex + 1}</span>
        <span className="graph-network__heading-actions">
          <LadRungPowerStatus power={powerFlow.rungState} />
          <code>{instructionCount} instruction{instructionCount === 1 ? "" : "s"} · {edgeCount} connection{edgeCount === 1 ? "" : "s"}</code>
          <button
            aria-label={`Move rung ${networkIndex + 1} up`}
            disabled={busy || networkIndex === 0}
            onClick={() => onMutation(moveLadNetwork(graph, { direction: "up", networkId }))}
            type="button"
          >
            ↑
          </button>
          <button
            aria-label={`Move rung ${networkIndex + 1} down`}
            disabled={busy || networkIndex >= networkCount - 1}
            onClick={() => onMutation(moveLadNetwork(graph, { direction: "down", networkId }))}
            type="button"
          >
            ↓
          </button>
          <button
            aria-label={`Duplicate rung ${networkIndex + 1}`}
            disabled={busy}
            onClick={() => onDuplicateNetwork(networkId)}
            title="Duplicate rung (Ctrl+C, Ctrl+V)"
            type="button"
          >
            Duplicate rung
          </button>
          <button
            aria-label={`Remove LAD network ${networkIndex + 1}`}
            className="lad-rung-remove"
            disabled={busy}
            onClick={() => onMutation(removeLadNetwork(graph, { networkId }))}
            type="button"
          >
            Remove rung
          </button>
        </span>
      </div>
      <div className="lad-rung" data-power={powerFlow.rungState}>
        <span className="lad-rail" data-power="on" aria-hidden="true" />
        <LadTopologySeries context={renderContext} items={projection.topology.items} />
        <span className="lad-rail lad-rail--right" data-power={powerFlow.rungState} aria-hidden="true" />
      </div>
    </section>
  );
};

type LadRenderContext = Readonly<{
  busy: boolean;
  graph: ProjectPayloadValue;
  guideTargetNodeId: string | null;
  instructionMembers: readonly GraphInterfaceMember[];
  liveMembers: ReadonlyMap<string, LadBooleanMemberLiveState>;
  members: readonly GraphInterfaceMember[];
  networkId: string;
  onMutation: (result: LadAuthoringResult | LadBoxAuthoringResult) => void;
  onSelectInsertTarget: (target: LadInsertionTarget) => void;
  paletteMemberId: string;
  powerFlow: LadPowerFlowProjection;
  rungNumber: number;
  selectedEdgeId: string | null;
  writableMembers: readonly GraphInterfaceMember[];
}>;

const LadRungPowerStatus = ({ power }: Readonly<{ power: LadPowerState }>): React.JSX.Element => (
  <output className="lad-rung-power" data-power={power}>
    <i aria-hidden="true" />
    {power === "on" ? "Power reaches coil" : power === "off" ? "Power stopped" : "Awaiting live values"}
  </output>
);

const LadTopologySeries = ({
  context,
  items,
}: Readonly<{
  context: LadRenderContext;
  items: readonly LadTopologyItem[];
}>): React.JSX.Element => (
  <div className="lad-series">
    {items.map((item) => (
      <div
        className="lad-series__segment"
        data-power={itemPowerState(context, item)}
        key={item.kind === "element" ? item.nodeId : item.branchId}
      >
        {item.beforeEdgeId !== null && (
          <LadInsertContact context={context} edgeId={item.beforeEdgeId} />
        )}
        {item.kind === "parallel" ? (
          <LadParallel context={context} parallel={item} />
        ) : (
          <LadElement context={context} element={item} />
        )}
      </div>
    ))}
  </div>
);

const itemPowerState = (
  context: LadRenderContext,
  item: LadTopologyItem,
): LadPowerState => item.kind === "parallel"
  ? context.powerFlow.edgeStates.get(item.afterEdgeId) ?? "unknown"
  : context.powerFlow.nodeStates.get(item.nodeId)?.outgoing ?? "unknown";

const LadInsertContact = ({
  context,
  edgeId,
}: Readonly<{
  context: LadRenderContext;
  edgeId: string;
}>): React.JSX.Element => {
  const selected = context.selectedEdgeId === edgeId;
  return (
    <button
      aria-label={`Choose insert point on rung ${context.rungNumber}`}
      aria-pressed={selected}
      className="lad-insert-contact"
      data-power={context.powerFlow.edgeStates.get(edgeId) ?? "unknown"}
      data-selected={selected}
      disabled={context.busy || context.members.length === 0}
      onClick={() => context.onSelectInsertTarget({
        edgeId,
        networkId: context.networkId,
        rungNumber: context.rungNumber,
      })}
      title={context.members.length === 0
        ? "Add a BOOL variable before inserting a contact"
        : "Choose this point, then select an instruction above"}
      type="button"
    >
      <span aria-hidden="true">+</span>
      {selected ? "Selected" : "Insert here"}
    </button>
  );
};

const LadParallel = ({
  context,
  parallel,
}: Readonly<{
  context: LadRenderContext;
  parallel: LadTopologyParallel;
}>): React.JSX.Element => (
  <div
    className="lad-parallel"
    aria-label={`${parallel.paths.length}-path parallel branch`}
    data-power={context.powerFlow.edgeStates.get(parallel.afterEdgeId) ?? "unknown"}
  >
    <span
      className="lad-parallel__bus"
      data-power={context.powerFlow.edgeStates.get(parallel.beforeEdgeId) ?? "unknown"}
      aria-hidden="true"
    />
    <div className="lad-parallel__paths">
      {parallel.paths.map((path, index) => (
        <div
          className="lad-parallel__path"
          data-power={context.powerFlow.pathStates.get(path.pathId) ?? "unknown"}
          key={path.pathId}
        >
          <span className="lad-parallel__path-label">Path {index + 1}</span>
          <LadTopologySeries context={context} items={path.items} />
          <LadInsertContact context={context} edgeId={path.exitEdgeId} />
        </div>
      ))}
    </div>
    <span
      className="lad-parallel__bus lad-parallel__bus--right"
      data-power={context.powerFlow.edgeStates.get(parallel.afterEdgeId) ?? "unknown"}
      aria-hidden="true"
    />
  </div>
);

const LadElement = ({
  context,
  element,
}: Readonly<{
  context: LadRenderContext;
  element: Extract<LadTopologyItem, Readonly<{ kind: "element" }>>;
}>): React.JSX.Element => {
  const node = element.node;
  const nodeId = element.nodeId;
  const operand = canonicalRecordFields(node.operand);
  const memberId = typeof operand?.memberId === "string" ? operand.memberId : "";
  const live = context.liveMembers.get(memberId) ?? null;
  const power = context.powerFlow.nodeStates.get(nodeId) ?? null;

  if (element.nodeKind === "power-source") {
    return <div className="lad-power-source" data-power="on"><span aria-hidden="true">L+</span><small>Power</small></div>;
  }
  if (element.nodeKind === "box") {
    return (
      <LadInstructionBox
        context={context}
        node={node}
        nodeId={nodeId}
        power={power}
      />
    );
  }
  if (element.nodeKind === "call") {
    const targetBlockId = typeof node.targetBlockId === "string" ? node.targetBlockId : "unresolved";
    return (
      <div className="lad-element lad-call" data-power={power?.outgoing ?? "unknown"}>
        <div className="lad-call__title"><span>CALL</span><strong>FC</strong></div>
        <div className="lad-call__target">
          <span>Target block</span>
          <code title={targetBlockId}>{targetBlockId.slice(0, 8)}…{targetBlockId.slice(-4)}</code>
        </div>
        <div className="lad-call__pins"><span>InputValue</span><span>Result</span></div>
      </div>
    );
  }
  if (element.nodeKind === "contact") {
    const selectedMember = context.members.find((member) => member.id === memberId) ?? context.members[0];
    const parallelMember = context.members.find((member) => member.id === context.paletteMemberId) ?? selectedMember;
    return (
      <div
        className="lad-element lad-contact"
        data-guide-target={context.guideTargetNodeId === nodeId}
        data-power={power?.outgoing ?? "unknown"}
      >
        <LadLiveOperandBadge contactCondition={power?.condition ?? "unknown"} live={live} />
        <div className="lad-symbol" aria-hidden="true">
          <span>—|</span><strong>{node.mode === "normally-closed" ? "/" : ""}</strong><span>|—</span>
        </div>
        <strong className="lad-element__name">
          {selectedMember?.operandLabel ?? selectedMember?.name ?? "Choose variable"}
        </strong>
        <label>
          <span>Operand</span>
          <select
            disabled={context.busy || context.members.length === 0}
            onChange={(event) => context.onMutation(updateLadContact(context.graph, {
              contactNodeId: nodeId,
              memberId: event.target.value,
              networkId: context.networkId,
            }))}
            value={memberId}
          >
            {memberOptions(context.members, memberId)}
          </select>
        </label>
        <label>
          <span>Contact</span>
          <select
            disabled={context.busy}
            onChange={(event) => context.onMutation(updateLadContact(context.graph, {
              contactNodeId: nodeId,
              mode: event.target.value as "normally-closed" | "normally-open",
              networkId: context.networkId,
            }))}
            value={typeof node.mode === "string" ? node.mode : "normally-open"}
          >
            <option value="normally-open">Normally open</option>
            <option value="normally-closed">Normally closed</option>
          </select>
        </label>
        <div className="lad-element__actions">
          <button
            aria-label={`Add ${parallelMember?.operandLabel ?? parallelMember?.name ?? "variable"} parallel with ${selectedMember?.operandLabel ?? selectedMember?.name ?? "selected contact"}`}
            data-tutorial-target={context.guideTargetNodeId === nodeId ? "add-seal-in" : undefined}
            disabled={context.busy || parallelMember === undefined}
            onClick={() => {
              if (parallelMember !== undefined) {
                context.onMutation(wrapContactWithParallelContact(context.graph, {
                  contactNodeId: nodeId,
                  memberId: parallelMember.id,
                  networkId: context.networkId,
                }));
              }
            }}
            type="button"
          >
            Parallel + {parallelMember?.operandLabel ?? parallelMember?.name ?? "variable"}
          </button>
          <button
            disabled={context.busy}
            onClick={() => context.onMutation(removeContactAndReconnect(context.graph, {
              contactNodeId: nodeId,
              networkId: context.networkId,
            }))}
            type="button"
          >
            Remove
          </button>
        </div>
      </div>
    );
  }
  if (element.nodeKind === "coil") {
    const selectedMember = context.writableMembers.find((member) => member.id === memberId);
    return (
      <div className="lad-element lad-coil" data-power={power?.incoming ?? "unknown"}>
        <LadLiveOperandBadge contactCondition={null} live={live} />
        <div className="lad-symbol" aria-hidden="true"><span>—(</span><strong>{coilMark(node.mode)}</strong><span>)—</span></div>
        <strong className="lad-element__name">
          {selectedMember?.operandLabel ?? selectedMember?.name ?? "Choose variable"}
        </strong>
        <label>
          <span>Operand</span>
          <select
            disabled={context.busy || context.writableMembers.length === 0}
            onChange={(event) => context.onMutation(updateLadCoil(context.graph, {
              coilNodeId: nodeId,
              memberId: event.target.value,
              networkId: context.networkId,
            }))}
            value={memberId}
          >
            {memberOptions(context.writableMembers, memberId)}
          </select>
        </label>
        <label>
          <span>Coil</span>
          <select
            disabled={context.busy}
            onChange={(event) => context.onMutation(updateLadCoil(context.graph, {
              coilNodeId: nodeId,
              mode: event.target.value as "negated" | "normal" | "reset" | "set",
              networkId: context.networkId,
            }))}
            value={typeof node.mode === "string" ? node.mode : "normal"}
          >
            <option value="normal">Normal</option>
            <option value="negated">Negated</option>
            <option value="set">Set</option>
            <option value="reset">Reset</option>
          </select>
        </label>
      </div>
    );
  }
  if (element.nodeKind === "return") {
    return <div className="lad-element lad-return"><strong>RETURN</strong></div>;
  }
  return (
    <div className="lad-element lad-element--unsupported">
      <strong>{element.nodeKind}</strong>
      <small>This canonical node is visible but not editable in the basic LAD palette yet.</small>
    </div>
  );
};

const LadInstructionBox = ({
  context,
  node,
  nodeId,
  power,
}: Readonly<{
  context: LadRenderContext;
  node: ProjectPayload;
  nodeId: string;
  power: Readonly<{ condition: LadPowerState | null; incoming: LadPowerState; outgoing: LadPowerState }> | null;
}>): React.JSX.Element => {
  const instruction = findMvpLadInstruction(readCanonicalInteger(node.instructionCode, -1));
  const pins = Array.isArray(node.pins)
    ? node.pins.map(canonicalRecordFields).filter((pin): pin is ProjectPayload => pin !== null)
    : [];
  if (instruction === null) {
    return (
      <div className="lad-element lad-element--unsupported">
        <strong>Unknown instruction box</strong>
        <small>The instruction code is not in the learner MVP catalog.</small>
      </div>
    );
  }
  return (
    <article className="lad-element lad-box" data-power={power?.outgoing ?? "unknown"}>
      <header className="lad-box__header">
        <span>{instruction.group}</span>
        <strong>{instruction.mnemonic}</strong>
        <small>{instruction.learning.title}</small>
      </header>
      <p>{instruction.learning.plainLanguage}</p>
      <div className="lad-box__pins">
        {pins.map((pin, index) => (
          <LadInstructionPinEditor
            context={context}
            key={typeof pin.id === "string" ? pin.id : `${nodeId}-${index}`}
            nodeId={nodeId}
            pin={pin}
          />
        ))}
      </div>
      {instruction.stateKind !== null && (
        <span className="lad-box__state">
          <i aria-hidden="true" />
          Private {instruction.stateKind} memory
        </span>
      )}
      <footer>
        <small>{instruction.learning.tip}</small>
        <button
          disabled={context.busy}
          onClick={() => context.onMutation(removeLadBoxAndReconnect(context.graph, {
            boxNodeId: nodeId,
            networkId: context.networkId,
          }))}
          type="button"
        >
          Remove
        </button>
      </footer>
    </article>
  );
};

const LadInstructionPinEditor = ({
  context,
  nodeId,
  pin,
}: Readonly<{
  context: LadRenderContext;
  nodeId: string;
  pin: ProjectPayload;
}>): React.JSX.Element => {
  const pinId = typeof pin.id === "string" ? pin.id : "";
  const name = typeof pin.name === "string" ? pin.name : "?";
  const dataType = typeof pin.dataType === "string" ? pin.dataType : "?";
  const direction = pin.direction === "output" ? "output" : "input";
  const binding = canonicalRecordFields(pin.binding);
  const memberId = binding?.kind === "caller-member" && typeof binding.memberId === "string"
    ? binding.memberId
    : null;
  const constant = binding?.kind === "constant";
  const candidates = context.instructionMembers.filter((member) =>
    member.dataType === dataType &&
    (direction === "input" || member.writable)
  );
  const selectValue = constant ? "__constant__" : memberId ?? "";
  const update = (next: ProjectPayloadValue | null): void => {
    context.onMutation(updateLadBoxPinBinding(context.graph, {
      binding: next,
      boxNodeId: nodeId,
      networkId: context.networkId,
      pinId,
    }));
  };
  const constantValue = binding === null ? 0 : readCanonicalInteger(binding.value, 0);
  const boolConstant = binding?.value === true;
  return (
    <div className="lad-box-pin" data-bound={binding !== null} data-direction={direction}>
      <div>
        <strong>{name}</strong>
        <code>{dataType}</code>
      </div>
      <select
        aria-label={`${name} ${direction} binding`}
        disabled={context.busy || pinId.length === 0}
        onChange={(event) => {
          const selected = event.target.value;
          if (selected.length === 0) {
            update(null);
          } else if (selected === "__constant__") {
            update(recordValue({
              dataType,
              kind: "constant",
              value: dataType === "BOOL" ? false : signedValue(dataType === "TIME" ? 1_000 : 0),
            }));
          } else {
            update(recordValue({ kind: "caller-member", memberId: selected }));
          }
        }}
        value={selectValue}
      >
        <option value="">Choose variable</option>
        {direction === "input" && <option value="__constant__">Constant value</option>}
        {candidates.map((member) => (
          <option key={member.id} value={member.id}>{member.operandLabel ?? member.name}</option>
        ))}
      </select>
      {constant && dataType === "BOOL" && (
        <label className="lad-box-pin__constant lad-box-pin__constant--bool">
          <input
            checked={boolConstant}
            disabled={context.busy}
            onChange={(event) => update(recordValue({
              dataType,
              kind: "constant",
              value: event.target.checked,
            }))}
            type="checkbox"
          />
          <span>{boolConstant ? "TRUE" : "FALSE"}</span>
        </label>
      )}
      {constant && dataType !== "BOOL" && (
        <label className="lad-box-pin__constant">
          <span>{dataType === "TIME" ? "Milliseconds" : "Value"}</span>
          <input
            disabled={context.busy}
            onChange={(event) => {
              const value = Number(event.target.value);
              if (Number.isSafeInteger(value)) {
                update(recordValue({ dataType, kind: "constant", value: signedValue(value) }));
              }
            }}
            step="1"
            type="number"
            value={constantValue}
          />
        </label>
      )}
    </div>
  );
};

const LadLiveOperandBadge = ({
  contactCondition,
  live,
}: Readonly<{
  contactCondition: LadPowerState | null;
  live: LadBooleanMemberLiveState | null;
}>): React.JSX.Element => {
  const truth = live?.truth ?? "unknown";
  const rawLabel = truth === "on" ? "TRUE" : truth === "off" ? "FALSE" : "—";
  const state = contactCondition ?? truth;
  const label = contactCondition === null
    ? `Output ${truth === "on" ? "ON" : truth === "off" ? "OFF" : "—"}`
    : contactCondition === "on"
      ? "Closed · passes power"
      : contactCondition === "off"
        ? "Open · blocks power"
        : "Contact state unknown";
  const reason = live === null
    ? "No live binding"
    : live.unknownReason === null
      ? `${live.tagName ?? live.memberName}${live.runtimeAddress === null ? "" : ` · ${live.runtimeAddress}`}`
      : live.unknownReason.replaceAll("-", " ");
  return (
    <span
      aria-label={`${label}; operand ${rawLabel}${live?.forced === true ? ", forced" : ""}`}
      className="lad-live-operand"
      data-forced={live?.forced === true}
      data-truth={state}
      title={reason}
    >
      <i aria-hidden="true" />
      {label}
      {live?.forced === true && <strong>F</strong>}
    </span>
  );
};

const FbdNetworkEditor = ({
  busy,
  commitNodeFields,
  members,
  network,
  networkIndex,
}: GraphNetworkEditorProps): React.JSX.Element => {
  const nodes = Array.isArray(network.nodes)
    ? network.nodes.map(canonicalRecordFields).filter((node): node is ProjectPayload => node !== null)
    : [];
  const connectionCount = Array.isArray(network.connections) ? network.connections.length : 0;
  return (
    <section className="fbd-network" aria-label={`FBD network ${networkIndex + 1}`}>
      <div className="graph-network__heading">
        <span>Network {networkIndex + 1}</span>
        <code>{nodes.length} nodes · {connectionCount} typed connections</code>
      </div>
      <div className="fbd-flow">
        {nodes.map((node, index) => {
          const nodeId = typeof node.id === "string" ? node.id : "";
          const nodeKind = typeof node.nodeKind === "string" ? node.nodeKind : "unresolved";
          const isMemberNode = nodeKind === "load-member" || nodeKind === "store-member";
          const memberId = typeof node.memberId === "string" ? node.memberId : "";
          return (
            <div className="fbd-flow__step" key={nodeId}>
              <article className="fbd-node" data-kind={nodeKind}>
                <div className="fbd-node__title">
                  <span>{fbdNodeLabel(node)}</span>
                  <code>{index + 1}</code>
                </div>
                {isMemberNode ? (
                  <label>
                    <span>{nodeKind === "load-member" ? "Read member" : "Write member"}</span>
                    <select
                      disabled={busy}
                      onChange={(event) => commitNodeFields(nodeId, { memberId: event.target.value })}
                      value={memberId}
                    >
                      {memberOptions(members, memberId)}
                    </select>
                  </label>
                ) : (
                  <div className="fbd-node__instruction">
                    <span>IN</span><strong>{node.instructionCode === undefined ? nodeKind : "NOT"}</strong><span>OUT</span>
                  </div>
                )}
                <small>{Array.isArray(node.ports) ? node.ports.length : 0} typed port{Array.isArray(node.ports) && node.ports.length === 1 ? "" : "s"}</small>
              </article>
              {index < nodes.length - 1 && <span className="fbd-connection" aria-label="Typed data connection">→</span>}
            </div>
          );
        })}
      </div>
    </section>
  );
};

const readGraphInterfaceMembers = (payload: ProjectPayload): readonly GraphInterfaceMember[] => {
  if (!Array.isArray(payload.interface)) {
    return [];
  }
  return payload.interface.flatMap((value) => {
    const member = canonicalRecordFields(value);
    return member !== null &&
      typeof member.id === "string" &&
      typeof member.name === "string" &&
      typeof member.role === "string" &&
      typeof member.type === "string"
      ? [{
          dataType: member.type,
          id: member.id,
          name: member.name,
          role: member.role,
          writable: member.role !== "input" && member.role !== "constant",
        }]
      : [];
  });
};

const memberOptions = (
  members: readonly GraphInterfaceMember[],
  selectedId: string,
): React.JSX.Element[] => {
  const values = members.some((member) => member.id === selectedId)
    ? members
    : [{
        dataType: "?",
        id: selectedId,
        name: "Unresolved member",
        role: "unresolved",
        writable: false,
      }, ...members];
  return values.map((member) => (
    <option key={member.id} value={member.id}>{member.operandLabel ?? member.name} · {member.dataType}</option>
  ));
};

const coilMark = (mode: ProjectPayloadValue | undefined): string => {
  switch (mode) {
    case "negated": return "/";
    case "set": return "S";
    case "reset": return "R";
    default: return " ";
  }
};

const fbdNodeLabel = (node: ProjectPayload): string => {
  switch (node.nodeKind) {
    case "load-member": return "Member source";
    case "store-member": return "Member sink";
    case "instruction": return "Boolean instruction";
    case "call": return "Block call";
    default: return typeof node.nodeKind === "string" ? node.nodeKind : "Unresolved node";
  }
};

type PropertiesPaneProps = Readonly<{
  busy: boolean;
  object: WorkbenchObjectView;
  onOperation: (operation: WorkbenchOperation) => Promise<void>;
  projectRootId: string;
}>;

const PropertiesPane = ({
  busy,
  object,
  onOperation,
  projectRootId,
}: PropertiesPaneProps): React.JSX.Element => {
  const [name, setName] = useState(object.displayName);
  const persistedDescription = typeof object.semanticPayload.description === "string"
    ? object.semanticPayload.description
    : "";
  const [description, setDescription] = useState(persistedDescription);
  const [confirmDelete, setConfirmDelete] = useState(false);
  useEffect(() => {
    setName(object.displayName);
    setDescription(persistedDescription);
    setConfirmDelete(false);
  }, [object.displayName, object.id, object.semanticRevision, persistedDescription]);
  const isRoot = object.id === projectRootId;
  const nameChanged = name.trim().length > 0 && name.trim() !== object.displayName;
  const descriptionChanged = isRoot && description.trim() !== persistedDescription;
  const canDuplicate = object.kind !== "Controller" &&
    object.kind !== "Rack" &&
    object.kind !== "Module" &&
    object.kind !== "SymbolTable" &&
    object.kind !== "Tag";

  return (
    <aside className="properties-pane" aria-label="Properties">
      <div className="pane-heading"><span>Properties</span></div>
      <form
        className="properties-form"
        onSubmit={(event) => {
          event.preventDefault();
          const normalized = name.trim();
          if (!busy && (nameChanged || descriptionChanged)) {
            void (async () => {
              if (nameChanged) {
                await onOperation({
                  displayName: normalized,
                  kind: "project.rename-object",
                  objectId: object.id,
                });
              }
              if (descriptionChanged) {
                await onOperation({
                  kind: "project.replace-semantic-payload",
                  objectId: object.id,
                  semanticPayload: {
                    ...object.semanticPayload,
                    description: description.trim(),
                  },
                });
              }
            })();
          }
        }}
      >
        <div className="property-kind">
          <ObjectGlyph kind={object.kind} />
          <div><strong>{kindLabel[object.kind]}</strong><span>Active</span></div>
        </div>
        <label>
          <span>Name</span>
          <input
            disabled={busy}
            maxLength={128}
            onChange={(event) => setName(event.target.value)}
            spellCheck="false"
            value={name}
          />
        </label>
        {isRoot && (
          <label>
            <span>Description</span>
            <textarea
              disabled={busy}
              maxLength={1_000}
              onChange={(event) => setDescription(event.target.value)}
              placeholder="What will students build or learn in this project?"
              rows={4}
              value={description}
            />
          </label>
        )}
        <button
          className="property-apply"
          disabled={busy || (!nameChanged && !descriptionChanged)}
          type="submit"
        >{isRoot ? "Apply project details" : "Apply name"}</button>
      </form>
      {!isRoot && (
        <div className="object-actions">
          <p className="property-section-title">Object actions</p>
          {canDuplicate && (
            <button
              disabled={busy || object.parentId === null}
              onClick={() => {
                if (object.parentId !== null) {
                  void onOperation({
                    kind: "project.copy-objects",
                    sourceObjectIds: [object.id],
                    targetParentId: object.parentId,
                  });
                }
              }}
              type="button"
            >Duplicate with new identity</button>
          )}
          {confirmDelete ? (
            <div className="property-delete-confirm">
              <span>Delete {object.displayName}? Undo remains available.</span>
              <div>
                <button disabled={busy} onClick={() => setConfirmDelete(false)} type="button">Cancel</button>
                <button
                  className="danger-action"
                  disabled={busy}
                  onClick={() => void onOperation({ kind: "project.delete-object", objectId: object.id })}
                  type="button"
                >Confirm delete</button>
              </div>
            </div>
          ) : (
            <button
              className="danger-action"
              disabled={busy}
              onClick={() => setConfirmDelete(true)}
              type="button"
            >Delete object</button>
          )}
        </div>
      )}
      <div className="properties-foot">
        <span>UUID</span>
        <code title={object.id}>{object.id.slice(0, 8)}…{object.id.slice(-4)}</code>
      </div>
    </aside>
  );
};

type CreateObjectTemplate = Readonly<{
  baseName: string;
  description: string;
  glyph: string;
  label: string;
  objectKind: ProjectStorageKind;
  payloadSchema: string;
  semanticPayload: ProjectPayload | (() => ProjectPayload);
}>;

const creationOptions = (
  parent: WorkbenchObjectView,
  snapshot: WorkbenchSnapshot,
): readonly CreateObjectTemplate[] => {
  switch (parent.kind) {
    case "ProjectRoot":
    case "Folder":
      return [
        {
          baseName: "Controller",
          description: "EDU-21 virtual controller",
          glyph: "C",
          label: "Controller",
          objectKind: "controller",
          payloadSchema: "edu.controller/1",
          semanticPayload: {
            catalogId: "vctrl-c1",
            profileId: "EDU-21 Core",
            profileVersion: "1.0.0",
          },
        },
        {
          baseName: "Virtual network",
          description: "Data-only training network",
          glyph: "VN",
          label: "Virtual network",
          objectKind: "network",
          payloadSchema: "edu.virtual-network/1",
          semanticPayload: { configuredState: "enabled" },
        },
        {
          baseName: "Engineering folder",
          description: "Organizational project folder",
          glyph: "▰",
          label: "Folder",
          objectKind: "folder",
          payloadSchema: "edu.folder/1",
          semanticPayload: {},
        },
      ];
    case "Controller": {
      const targetTagIds = Object.values(snapshot.objects)
        .filter((object) =>
          object.lifecycle === "active" &&
          object.kind === "Tag" &&
          isDescendantOf(object, parent.id, snapshot)
        )
        .sort((left, right) => left.creationOrdinal.localeCompare(right.creationOrdinal))
        .map((object) => object.id);
      return [
        {
          baseName: "Local rack",
          description: "Eight-slot controller rack",
          glyph: "R",
          label: "Rack",
          objectKind: "rack",
          payloadSchema: "edu.rack/1",
          semanticPayload: { slotCount: unsignedValue(8) },
        },
        {
          baseName: "PLC tags",
          description: "Controller-wide symbol table",
          glyph: "ST",
          label: "Tag table",
          objectKind: "symbol-table",
          payloadSchema: "edu.symbol-table/1",
          semanticPayload: {},
        },
        {
          baseName: "Main HMI",
          description: "Virtual operator controls and indicators",
          glyph: "HM",
          label: "HMI screen",
          objectKind: "generic",
          payloadSchema: "edu.hmi-screen/1",
          semanticPayload: () => encodeHmiScreenPayload(createHmiScreen("Main HMI")),
        },
        {
          baseName: "ProcessData",
          description: "Named user structure",
          glyph: "UD",
          label: "Named structure",
          objectKind: "type-definition",
          payloadSchema: "edu.named-type/1",
          semanticPayload: createNamedTypePayload,
        },
        {
          baseName: "Main_cycle",
          description: "Cyclic SCL organization block",
          glyph: "OB",
          label: "Organization block",
          objectKind: "program-block",
          payloadSchema: "edu.program-block/1",
          semanticPayload: () => createSclProgramPayload(
            "cyclic-ob",
            nextEngineeringNumber(snapshot, "OB"),
          ),
        },
        {
          baseName: "MainCycle",
          description: "Editable semantic LAD organization block",
          glyph: "LD",
          label: "Ladder organization block",
          objectKind: "program-block",
          payloadSchema: "edu.program-block/1",
          semanticPayload: () => createLadProgramPayload(
            nextEngineeringNumber(snapshot, "OB"),
            ladFcCallTargets(snapshot, parent.id),
          ),
        },
        {
          baseName: "Function",
          description: "Reusable SCL function",
          glyph: "FC",
          label: "Function",
          objectKind: "program-block",
          payloadSchema: "edu.program-block/1",
          semanticPayload: () => createSclProgramPayload(
            "fc",
            nextEngineeringNumber(snapshot, "FC"),
          ),
        },
        {
          baseName: "FbdFunction",
          description: "Typed function-block diagram",
          glyph: "FD",
          label: "FBD function",
          objectKind: "program-block",
          payloadSchema: "edu.program-block/1",
          semanticPayload: () => createFbdProgramPayload(nextEngineeringNumber(snapshot, "FC")),
        },
        {
          baseName: "StateBlock",
          description: "State-owning SCL block",
          glyph: "FB",
          label: "Function block",
          objectKind: "program-block",
          payloadSchema: "edu.program-block/1",
          semanticPayload: () => createSclProgramPayload(
            "fb",
            nextEngineeringNumber(snapshot, "FB"),
          ),
        },
        {
          baseName: "GlobalData",
          description: "Controller global data block",
          glyph: "DB",
          label: "Global data block",
          objectKind: "data-block",
          payloadSchema: "edu.data-block/1",
          semanticPayload: () => createDataBlockPayload(
            "GlobalDB",
            null,
            nextEngineeringNumber(snapshot, "GlobalDB"),
          ),
        },
        {
          baseName: "InstanceData",
          description: "Function-block instance data",
          glyph: "ID",
          label: "Instance data block",
          objectKind: "data-block",
          payloadSchema: "edu.data-block/1",
          semanticPayload: () => createDataBlockPayload(
            "InstanceDB",
            Object.values(snapshot.objects).find(
              (object) => object.lifecycle === "active" && object.kind === "FB",
            )?.id ?? null,
            nextEngineeringNumber(snapshot, "InstanceDB"),
          ),
        },
        {
          baseName: "Watch table",
          description: "Persistent monitoring targets",
          glyph: "W",
          label: "Watch table",
          objectKind: "generic",
          payloadSchema: "edu.watch-table/1",
          semanticPayload: () => createWatchPayload(targetTagIds),
        },
        {
          baseName: "Trace",
          description: "Bounded virtual trace configuration",
          glyph: "T",
          label: "Trace configuration",
          objectKind: "generic",
          payloadSchema: "edu.trace-configuration/1",
          semanticPayload: () => createTracePayload(targetTagIds),
        },
      ];
    }
    case "Rack": {
      const slot = firstFreeModuleSlot(snapshot, parent);
      if (slot === null) {
        return [];
      }
      return [
        moduleTemplate("Digital input module", "VDI16", "vdi16", slot),
        moduleTemplate("Digital output module", "VDO16", "vdo16", slot),
      ];
    }
    // The tag-table editor owns tag creation because it validates the address
    // and creates the matching LAD program binding as one learner workflow.
    case "SymbolTable":
      return [];
    default:
      return [];
  }
};

const moduleTemplate = (
  label: string,
  baseName: string,
  catalogId: string,
  slot: number,
): CreateObjectTemplate => ({
  baseName,
  description: `EDU-21 ${catalogId.toUpperCase()} in slot ${slot}`,
  glyph: catalogId.startsWith("vd") ? "D" : "A",
  label,
  objectKind: "module",
  payloadSchema: "edu.module/1",
  semanticPayload: {
    addressIntent: "auto",
    catalogId,
    slot: unsignedValue(slot),
  },
});

const nextEngineeringNumber = (
  snapshot: WorkbenchSnapshot,
  blockKind: "FB" | "FC" | "GlobalDB" | "InstanceDB" | "OB",
): number => {
  let maximum = 0;
  for (const object of Object.values(snapshot.objects)) {
    if (object.lifecycle !== "active") {
      continue;
    }
    const authoredKind = object.semanticPayload.blockKind ?? object.semanticPayload.dbKind;
    const sameNumberingFamily = blockKind === "GlobalDB" || blockKind === "InstanceDB"
      ? authoredKind === "GlobalDB" || authoredKind === "InstanceDB"
      : authoredKind === blockKind;
    if (!sameNumberingFamily) {
      continue;
    }
    const value = object.semanticPayload.engineeringNumber;
    if (
      typeof value === "object" &&
      value !== null &&
      !Array.isArray(value) &&
      "$type" in value &&
      value.$type === "u64" &&
      "value" in value &&
      typeof value.value === "string"
    ) {
      const parsed = Number(value.value);
      if (Number.isSafeInteger(parsed) && parsed > maximum) {
        maximum = parsed;
      }
    }
  }
  return Math.min(maximum + 1, 4_294_967_295);
};

const ladFcCallTargets = (
  snapshot: WorkbenchSnapshot,
  controllerId: string,
): readonly Readonly<{
  inputFormalId: string;
  outputFormalId: string;
  resultName: string;
  targetBlockId: string;
}>[] => {
  const compatible = Object.values(snapshot.objects)
    .filter((object) =>
      object.lifecycle === "active" &&
      object.parentId === controllerId &&
      object.kind === "FC" &&
      (object.semanticPayload.language === "FBD" || object.semanticPayload.language === "SCL")
    )
    .sort((left, right) => left.creationOrdinal.localeCompare(right.creationOrdinal));
  const selected = ["FBD", "SCL"].flatMap((language) => {
    const block = compatible.find((candidate) => candidate.semanticPayload.language === language);
    if (block === undefined) {
      return [];
    }
    const inputFormalId = interfaceMemberIdentity(block.semanticPayload, "InputValue");
    const outputFormalId = interfaceMemberIdentity(block.semanticPayload, "Result");
    return inputFormalId === null || outputFormalId === null
      ? []
      : [{
          inputFormalId,
          outputFormalId,
          resultName: language === "FBD" ? "FbdResult" : "SclResult",
          targetBlockId: block.id,
        }];
  });
  return selected.length === 2 ? selected : [];
};

const nextObjectName = (
  baseName: string,
  parentId: string,
  snapshot: WorkbenchSnapshot,
  plcIdentifier: boolean,
): string => {
  const siblingNames = new Set(
    Object.values(snapshot.objects)
      .filter((object) => object.lifecycle === "active" && object.parentId === parentId)
      .map((object) => object.displayName.toLocaleLowerCase("en-US")),
  );
  if (!siblingNames.has(baseName.toLocaleLowerCase("en-US"))) {
    return baseName;
  }
  for (let suffix = 2; suffix <= 9_999; suffix += 1) {
    const candidate = `${baseName}${plcIdentifier ? "_" : " "}${suffix}`;
    if (!siblingNames.has(candidate.toLocaleLowerCase("en-US"))) {
      return candidate;
    }
  }
  return `${baseName}${plcIdentifier ? "_" : " "}${crypto.randomUUID().slice(0, 8)}`;
};

const requiresPlcIdentifier = (template: CreateObjectTemplate): boolean =>
  template.payloadSchema === "edu.program-block/1" ||
  template.payloadSchema === "edu.data-block/1" ||
  template.payloadSchema === "edu.named-type/1" ||
  template.payloadSchema === "edu.tag/1";

const isDescendantOf = (
  object: WorkbenchObjectView,
  ancestorId: string,
  snapshot: WorkbenchSnapshot,
): boolean => {
  let parentId = object.parentId;
  const visited = new Set<string>();
  while (parentId !== null && !visited.has(parentId)) {
    if (parentId === ancestorId) {
      return true;
    }
    visited.add(parentId);
    parentId = snapshot.objects[parentId]?.parentId ?? null;
  }
  return false;
};

const formatDirtyState = (state: WorkbenchSnapshot["dirtyState"]): string => {
  switch (state) {
    case "clean": return "Saved";
    case "presentation-dirty": return "Unsaved layout";
    case "semantic-dirty": return "Unsaved changes";
  }
};

const formatBuildState = (state: WorkbenchSnapshot["buildState"]): string => {
  switch (state) {
    case "not-built": return "Not built";
    case "current": return "Build current";
    case "stale": return "Build stale";
    case "blocked": return "Build blocked";
  }
};

const runtimeSummary = (runtime: EngineeringRuntimeView): string => {
  const session = runtime.session;
  if (session === null) {
    return "Runtime unavailable";
  }
  const mode = session.cpuState === "PAUSED_EDUCATIONAL" ? "PAUSED" : session.cpuState;
  return `${mode} · ${session.online ? "online" : "offline"} · scan ${session.scanSequence}`;
};
