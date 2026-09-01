import { useEffect, useMemo, useRef, useState } from "react";

import "./HmiScreenEditor.css";

import { createHmiRuntimeTagBus } from "./hmi-runtime-tag-bus";
import {
  appendHmiElement,
  createHmiLamp,
  createHmiMomentaryButton,
  createHmiScreen,
  decodeHmiScreenPayload,
  encodeHmiScreenPayload,
  removeHmiElement,
  replaceHmiElement,
  validateHmiScreen,
} from "./hmi-screen-model";
import type {
  HmiBindableTag,
  HmiLampElement,
  HmiMomentaryButtonElement,
  HmiScreen,
  HmiScreenElement,
  HmiScreenMode,
} from "./hmi-screen-model";
import type { RuntimeOperation } from "./runtime-types";
import type {
  WorkbenchObjectView,
  WorkbenchOperation,
  WorkbenchSnapshot,
} from "./workbench-types";

export type HmiScreenEditorProps = Readonly<{
  busy: boolean;
  object: WorkbenchObjectView;
  onOperation: (operation: WorkbenchOperation) => Promise<void>;
  onRuntimeOperation: (operation: RuntimeOperation) => Promise<void>;
  onStartSimulation: () => Promise<void>;
  snapshot: WorkbenchSnapshot;
}>;

export type HmiRuntimeOperationQueue = Readonly<{
  enqueue: (operations: readonly RuntimeOperation[]) => Promise<void>;
}>;

/**
 * Runtime snapshots replace object-view instances, so identity cannot decide
 * when editor-local mode and selection should reset. Only persisted HMI
 * identity or semantic revision may start a new editor session.
 */
export const hmiEditorPersistenceKey = (
  object: Pick<WorkbenchObjectView, "id" | "semanticRevision">,
): string => JSON.stringify([object.id, object.semanticRevision]);

/** Serializes press/scan/release/scan even when pointer-up follows quickly. */
export const createHmiRuntimeOperationQueue = (
  execute: (operation: RuntimeOperation) => Promise<void>,
): HmiRuntimeOperationQueue => {
  let tail: Promise<void> = Promise.resolve();
  return {
    enqueue: (operations) => {
      const queued = tail
        .catch(() => undefined)
        .then(async () => {
          for (const operation of operations) await execute(operation);
        });
      tail = queued;
      return queued;
    },
  };
};

export const HmiScreenEditor = ({
  busy,
  object,
  onOperation,
  onRuntimeOperation,
  onStartSimulation,
  snapshot,
}: HmiScreenEditorProps): React.JSX.Element => {
  const decoded = useMemo(
    () => decodeHmiScreenPayload(object.semanticPayload),
    [object.semanticPayload, object.semanticRevision],
  );
  const [draft, setDraft] = useState<HmiScreen | null>(() => decoded.ok ? decoded.screen : null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [mode, setMode] = useState<HmiScreenMode>("edit");
  const [message, setMessage] = useState<string | null>(null);
  const [applying, setApplying] = useState(false);
  const activePresses = useRef(new Set<string>());
  const runtimeExecutor = useRef(onRuntimeOperation);
  runtimeExecutor.current = onRuntimeOperation;
  const runtimeQueue = useRef<HmiRuntimeOperationQueue | null>(null);
  const operationQueue = runtimeQueue.current ?? createHmiRuntimeOperationQueue(
    (operation) => runtimeExecutor.current(operation),
  );
  runtimeQueue.current = operationQueue;
  const releaseLatest = useRef<() => Promise<void>>(async () => undefined);
  const persistenceKey = hmiEditorPersistenceKey(object);

  const tags = useMemo(() => discoverHmiBindableTags(snapshot), [snapshot]);
  const tagById = useMemo(() => new Map(tags.map((tag) => [tag.id, tag])), [tags]);
  const bus = useMemo(() => createHmiRuntimeTagBus(snapshot.runtime), [snapshot.runtime]);
  const validation = draft === null ? null : validateHmiScreen(draft, tags);
  const selected = draft?.elements.find((element) => element.id === selectedId) ?? null;
  const persistedFingerprint = decoded.ok ? fingerprint(decoded.screen) : null;
  const draftFingerprint = draft === null ? null : fingerprint(draft);
  const dirty = persistedFingerprint !== draftFingerprint;
  const disabled = busy || applying;
  const canRun = draft !== null && validation?.valid === true && !dirty;

  const updateElement = (element: HmiScreenElement): void => {
    setDraft((current) => current === null ? current : replaceHmiElement(current, element));
    setMessage(null);
  };

  const addElement = (kind: HmiScreenElement["kind"]): void => {
    if (draft === null || mode !== "edit") return;
    const ordinal = draft.elements.length;
    const frame = {
      height: kind === "momentary-button" ? 70 : 92,
      width: kind === "momentary-button" ? 170 : 130,
      x: 36 + (ordinal % 4) * 190,
      y: 40 + Math.floor(ordinal / 4) * 126,
    };
    const id = crypto.randomUUID();
    const element = kind === "momentary-button"
      ? createHmiMomentaryButton({ frame, id, label: "Push button" })
      : createHmiLamp({ frame, id, label: "Indicator lamp" });
    setDraft(appendHmiElement(draft, element));
    setSelectedId(id);
    setMessage(null);
  };

  const removeSelected = (): void => {
    if (draft === null || selected === null || mode !== "edit") return;
    setDraft(removeHmiElement(draft, selected.id));
    setSelectedId(null);
    setMessage(null);
  };

  const apply = async (screen: HmiScreen): Promise<void> => {
    const result = validateHmiScreen(screen, tags);
    if (!result.valid) {
      setMessage("Fix the screen bindings before applying these changes.");
      return;
    }
    setApplying(true);
    setMessage(null);
    try {
      await onOperation({
        kind: "project.replace-semantic-payload",
        objectId: object.id,
        semanticPayload: encodeHmiScreenPayload(screen),
      });
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "The HMI screen could not be applied.");
    } finally {
      setApplying(false);
    }
  };

  const initialize = (): void => {
    const screen = createHmiScreen(object.displayName || "Main HMI");
    setDraft(screen);
    setSelectedId(null);
    setMessage("Empty screen ready. Apply it to initialize this HMI object.");
  };

  const sendMomentary = async (
    element: HmiMomentaryButtonElement,
    phase: "press" | "release",
  ): Promise<void> => {
    if (mode !== "run" || element.tagId === null) return;
    if (phase === "press") {
      if (activePresses.current.has(element.id)) return;
    } else if (!activePresses.current.has(element.id)) {
      return;
    }
    const request = bus.createMomentaryRequest(element.tagId, phase);
    if (!request.ok) {
      activePresses.current.delete(element.id);
      setMessage(request.message);
      return;
    }
    if (phase === "press") activePresses.current.add(element.id);
    else activePresses.current.delete(element.id);
    setMessage(null);
    try {
      await operationQueue.enqueue(request.operations);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : `The HMI ${phase} could not reach the virtual PLC.`);
    }
  };

  const releaseAllMomentaries = async (): Promise<void> => {
    const pressed = [...activePresses.current];
    await Promise.all(pressed.map(async (id) => {
      if (draft !== null) {
        const element = draft.elements.find((candidate): candidate is HmiMomentaryButtonElement =>
          candidate.id === id && candidate.kind === "momentary-button"
        );
        if (element !== undefined) {
          await sendMomentary(element, "release");
          return;
        }
      }
      activePresses.current.delete(id);
    }));
  };
  releaseLatest.current = releaseAllMomentaries;

  const enterEditMode = async (): Promise<void> => {
    await releaseAllMomentaries();
    activePresses.current.clear();
    setMode("edit");
  };

  useEffect(() => {
    // Runtime commands publish fresh object-view instances even though the
    // HMI's semantic data has not changed. This effect therefore keys only on
    // persisted HMI identity/revision, never object or payload reference.
    void releaseLatest.current().catch(() => undefined);
    const persisted = decodeHmiScreenPayload(object.semanticPayload);
    setDraft(persisted.ok ? persisted.screen : null);
    setSelectedId(null);
    setMode("edit");
  }, [persistenceKey]);

  useEffect(() => {
    const release = (): void => { void releaseLatest.current().catch(() => undefined); };
    const releaseWhenHidden = (): void => { if (document.hidden) release(); };
    window.addEventListener("blur", release);
    document.addEventListener("visibilitychange", releaseWhenHidden);
    return () => {
      window.removeEventListener("blur", release);
      document.removeEventListener("visibilitychange", releaseWhenHidden);
      release();
    };
  }, []);

  if (draft === null) {
    return (
      <section className="hmi-editor hmi-editor--invalid" aria-labelledby="hmi-invalid-title">
        <div className="hmi-empty-state">
          <span aria-hidden="true">HMI</span>
          <h1 id="hmi-invalid-title">This HMI screen is not initialized</h1>
          <p>{decoded.ok ? "The screen is unavailable." : decoded.message}</p>
          <button disabled={disabled} onClick={initialize} type="button">Initialize empty HMI screen</button>
        </div>
      </section>
    );
  }

  return (
    <section className="hmi-editor" aria-label={`${draft.name} HMI editor`} data-mode={mode}>
      <header className="hmi-editor__header">
        <div>
          <p className="hmi-editor__kicker">Virtual operator screen</p>
          <input
            aria-label="HMI screen name"
            disabled={disabled || mode === "run"}
            maxLength={128}
            onChange={(event) => setDraft({ ...draft, name: event.target.value })}
            value={draft.name}
          />
          <p>Build a simple control station connected to the same simulated PLC as your ladder program.</p>
        </div>
        <div className="hmi-mode-switch" aria-label="HMI mode">
          <button aria-pressed={mode === "edit"} disabled={disabled} onClick={() => void enterEditMode()} type="button">Edit</button>
          <button
            aria-pressed={mode === "run"}
            disabled={disabled || !canRun}
            onClick={() => { setSelectedId(null); setMessage(null); setMode("run"); }}
            type="button"
          >Run HMI</button>
        </div>
      </header>

      {mode === "edit" && (
        <div className="hmi-editor__toolbar" aria-label="HMI elements">
          <button disabled={disabled} onClick={() => addElement("momentary-button")} type="button"><span aria-hidden="true">PB</span> Momentary button</button>
          <button disabled={disabled} onClick={() => addElement("lamp")} type="button"><span aria-hidden="true">●</span> Indicator lamp</button>
          <span className="hmi-editor__save-state" data-dirty={dirty}>{dirty ? "Changes not applied" : "Screen is applied"}</span>
          <button
            className="hmi-editor__apply"
            disabled={disabled || !dirty || validation?.valid !== true}
            onClick={() => void apply(draft)}
            type="button"
          >{applying ? "Applying…" : "Apply screen"}</button>
        </div>
      )}

      {message !== null && <div className="hmi-editor__message" role="alert">{message}</div>}
      {validation !== null && validation.issues.length > 0 && (
        <div className="hmi-editor__validation" role="status">
          <strong>Finish these screen connections</strong>
          <ul>{validation.issues.map((item, index) => <li key={`${item.code}:${item.elementId ?? "screen"}:${index}`}>{item.message}</li>)}</ul>
        </div>
      )}

      {mode === "run" && (
        <div className="hmi-runtime-strip">
          <div>
            <span className="hmi-runtime-dot" data-online={runtimeReady(snapshot)} aria-hidden="true" />
            <strong>{runtimeReady(snapshot) ? "Virtual PLC connected" : "Virtual PLC is not running"}</strong>
            <small>{runtimeSummary(snapshot)}</small>
          </div>
          <button disabled={disabled} onClick={() => void onStartSimulation()} type="button">
            {runtimeReady(snapshot) ? "Restart simulation" : "Start virtual PLC"}
          </button>
        </div>
      )}

      <div className="hmi-editor__workspace">
        <div className="hmi-canvas-scroll">
          <div
            className="hmi-canvas"
            style={{ height: draft.height, width: draft.width }}
            aria-label="HMI screen canvas"
          >
            {draft.elements.length === 0 && mode === "edit" && (
              <div className="hmi-canvas__empty"><strong>Start with an operator control</strong><span>Add a button or lamp from the toolbar.</span></div>
            )}
            {draft.elements.map((element) => element.kind === "momentary-button" ? (
              <MomentaryButton
                bus={bus}
                disabled={disabled}
                element={element}
                key={element.id}
                mode={mode}
                onPress={(phase) => void sendMomentary(element, phase)}
                onSelect={() => setSelectedId(element.id)}
                selected={selectedId === element.id}
              />
            ) : (
              <Lamp
                bus={bus}
                element={element}
                key={element.id}
                mode={mode}
                onSelect={() => setSelectedId(element.id)}
                selected={selectedId === element.id}
              />
            ))}
          </div>
        </div>

        {mode === "edit" && (
          <aside className="hmi-properties" aria-label="HMI element properties">
            {selected === null ? (
              <div className="hmi-properties__empty"><span aria-hidden="true">↖</span><strong>Select an element</strong><p>Choose a control on the canvas to name, bind, size, or position it.</p></div>
            ) : (
              <ElementProperties
                disabled={disabled}
                element={selected}
                onChange={updateElement}
                onRemove={removeSelected}
                tagById={tagById}
                tags={tags}
              />
            )}
          </aside>
        )}
      </div>
    </section>
  );
};

type RuntimeBus = ReturnType<typeof createHmiRuntimeTagBus>;

const MomentaryButton = ({
  bus,
  disabled,
  element,
  mode,
  onPress,
  onSelect,
  selected,
}: Readonly<{
  bus: RuntimeBus;
  disabled: boolean;
  element: HmiMomentaryButtonElement;
  mode: HmiScreenMode;
  onPress: (phase: "press" | "release") => void;
  onSelect: () => void;
  selected: boolean;
}>): React.JSX.Element => {
  const runtime = element.tagId === null ? null : bus.readBoolean(element.tagId);
  const runningProps = mode === "run" ? {
    onBlur: () => onPress("release"),
    onKeyDown: (event: React.KeyboardEvent<HTMLButtonElement>) => {
      if (!disabled && !event.repeat && (event.key === " " || event.key === "Enter")) onPress("press");
    },
    onKeyUp: (event: React.KeyboardEvent<HTMLButtonElement>) => {
      if (event.key === " " || event.key === "Enter") onPress("release");
    },
    onPointerCancel: () => onPress("release"),
    onPointerDown: (event: React.PointerEvent<HTMLButtonElement>) => {
      if (disabled || event.button !== 0) return;
      event.currentTarget.setPointerCapture(event.pointerId);
      onPress("press");
    },
    onPointerUp: (event: React.PointerEvent<HTMLButtonElement>) => {
      if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
      onPress("release");
    },
  } : { onClick: onSelect };
  return (
    <button
      {...runningProps}
      aria-label={mode === "run" ? `${element.label}, hold to operate` : `Edit ${element.label}`}
      aria-pressed={mode === "edit" ? selected : runtime?.truth === "on"}
      className="hmi-element hmi-button"
      aria-disabled={mode === "run" && disabled}
      data-live={runtime?.truth ?? "unknown"}
      data-selected={selected}
      style={elementStyle(element)}
      type="button"
    >
      <span className="hmi-button__cap" aria-hidden="true" />
      <strong>{element.label || "Push button"}</strong>
      <small>{runtimeCaption(runtime, element.tagId)}</small>
    </button>
  );
};

const Lamp = ({
  bus,
  element,
  mode,
  onSelect,
  selected,
}: Readonly<{
  bus: RuntimeBus;
  element: HmiLampElement;
  mode: HmiScreenMode;
  onSelect: () => void;
  selected: boolean;
}>): React.JSX.Element => {
  const runtime = element.tagId === null ? null : bus.readBoolean(element.tagId);
  return (
    <button
      aria-label={mode === "run" ? `${element.label}: ${runtime?.truth ?? "unknown"}` : `Edit ${element.label}`}
      className="hmi-element hmi-lamp"
      data-live={runtime?.truth ?? "unknown"}
      data-selected={selected}
      onClick={mode === "edit" ? onSelect : undefined}
      style={elementStyle(element)}
      tabIndex={mode === "edit" ? 0 : -1}
      type="button"
    >
      <span className="hmi-lamp__lens" aria-hidden="true" />
      <strong>{element.label || "Indicator lamp"}</strong>
      <small>{runtimeCaption(runtime, element.tagId)}</small>
    </button>
  );
};

const ElementProperties = ({
  disabled,
  element,
  onChange,
  onRemove,
  tagById,
  tags,
}: Readonly<{
  disabled: boolean;
  element: HmiScreenElement;
  onChange: (element: HmiScreenElement) => void;
  onRemove: () => void;
  tagById: ReadonlyMap<string, HmiBindableTag>;
  tags: readonly HmiBindableTag[];
}>): React.JSX.Element => {
  const compatibleTags = tags.filter((tag) =>
    tag.dataType.toLocaleUpperCase("en-US") === "BOOL" &&
    (element.kind === "momentary-button" ? tag.addressArea === "I" : tag.addressArea === "M" || tag.addressArea === "Q")
  );
  const currentAvailable = element.tagId === null || compatibleTags.some((tag) => tag.id === element.tagId);
  const updateFrame = (key: keyof HmiScreenElement["frame"], value: string): void => {
    const parsed = Number(value);
    const positive = key === "height" || key === "width";
    if (!Number.isSafeInteger(parsed) || parsed < (positive ? 1 : 0)) return;
    onChange({ ...element, frame: { ...element.frame, [key]: parsed } });
  };
  return (
    <div className="hmi-properties__form">
      <header><span>{element.kind === "momentary-button" ? "PB" : "●"}</span><div><small>Selected element</small><strong>{element.kind === "momentary-button" ? "Momentary button" : "Indicator lamp"}</strong></div></header>
      <label><span>Label</span><input disabled={disabled} maxLength={80} onChange={(event) => onChange({ ...element, label: event.target.value })} value={element.label} /></label>
      <label>
        <span>PLC tag</span>
        <select disabled={disabled} onChange={(event) => onChange({ ...element, tagId: event.target.value || null })} value={element.tagId ?? ""}>
          <option value="">Choose a BOOL tag</option>
          {!currentAvailable && element.tagId !== null && <option value={element.tagId}>Unavailable binding · {tagById.get(element.tagId)?.name ?? element.tagId}</option>}
          {compatibleTags.map((tag) => <option key={tag.id} value={tag.id}>{tag.addressArea} · {tag.name}</option>)}
        </select>
        <small>{element.kind === "momentary-button" ? "Buttons operate virtual input (I) tags." : "Lamps observe output (Q) or memory (M) tags."}</small>
      </label>
      <fieldset><legend>Position and size</legend>{(["x", "y", "width", "height"] as const).map((key) => <label key={key}><span>{key.toLocaleUpperCase("en-US")}</span><input disabled={disabled} min={key === "x" || key === "y" ? 0 : 1} onChange={(event) => updateFrame(key, event.target.value)} type="number" value={element.frame[key]} /></label>)}</fieldset>
      <button className="hmi-properties__remove" disabled={disabled} onClick={onRemove} type="button">Remove element</button>
    </div>
  );
};

export const discoverHmiBindableTags = (
  snapshot: WorkbenchSnapshot,
): readonly HmiBindableTag[] => Object.values(snapshot.objects)
  .flatMap((object): HmiBindableTag[] => {
    if (
      object.lifecycle !== "active" || object.kind !== "Tag" ||
      (object.semanticPayload.addressArea !== "I" && object.semanticPayload.addressArea !== "M" && object.semanticPayload.addressArea !== "Q") ||
      typeof object.semanticPayload.dataType !== "string"
    ) return [];
    return [{
      addressArea: object.semanticPayload.addressArea,
      dataType: object.semanticPayload.dataType,
      id: object.id,
      name: object.displayName,
    }];
  })
  .sort((left, right) => left.addressArea.localeCompare(right.addressArea, "en-US") || left.name.localeCompare(right.name, "en-US"));

const elementStyle = (element: HmiScreenElement): React.CSSProperties => ({
  height: element.frame.height,
  left: element.frame.x,
  position: "absolute",
  top: element.frame.y,
  width: element.frame.width,
});

const fingerprint = (screen: HmiScreen): string => JSON.stringify(encodeHmiScreenPayload(screen));

const runtimeReady = (snapshot: WorkbenchSnapshot): boolean =>
  snapshot.runtime.availability === "READY" &&
  snapshot.runtime.session?.online === true &&
  snapshot.runtime.session.cpuState === "RUN";

const runtimeSummary = (snapshot: WorkbenchSnapshot): string => {
  const session = snapshot.runtime.session;
  if (snapshot.runtime.availability !== "READY") return snapshot.runtime.reason ?? "Runtime unavailable";
  if (session === null) return "Build and start the simulation to operate controls.";
  if (!session.online) return "The virtual controller is offline.";
  if (session.cpuState !== "RUN") return `Controller state: ${session.cpuState}`;
  return `Scan ${session.scanSequence} · ${session.monitorState.toLocaleLowerCase("en-US")} monitoring`;
};

const runtimeCaption = (
  read: ReturnType<RuntimeBus["readBoolean"]> | null,
  tagId: string | null,
): string => {
  if (tagId === null) return "Not bound";
  if (read === null || read.truth === "unknown") return "Value unavailable";
  return read.truth === "on" ? "ON" : "OFF";
};
