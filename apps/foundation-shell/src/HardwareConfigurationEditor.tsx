import { useEffect, useId, useMemo, useState } from "react";

import { createLadProgramPayload, unsignedValue } from "./canonical-authoring";
import {
  activeRackForController,
  activeRackModules,
  buildModuleConfigurationOperation,
  controllerForRack,
  createDigitalModulePayload,
  digitalModuleCatalog,
  digitalModuleCatalogs,
  firstFreeModuleSlot,
  formatModuleAddressRange,
  legalModuleSlots,
  readModuleConfiguration,
  validateModuleConfiguration,
} from "./hardware-configuration";
import type {
  DigitalModuleCatalogId,
  ModuleConfigurationDraft,
} from "./hardware-configuration";
import { activeChildren, controllerCatalogOption } from "./plc-setup";
import type {
  ProjectPayload,
  ProjectPayloadValue,
  WorkbenchObjectView,
  WorkbenchOperation,
  WorkbenchSnapshot,
} from "./workbench-types";

export type HardwareConfigurationEditorProps = Readonly<{
  busy: boolean;
  object: WorkbenchObjectView;
  onOperation: (operation: WorkbenchOperation) => Promise<void>;
  onSelectObject: (objectId: string) => void;
  snapshot: WorkbenchSnapshot;
}>;

export const HardwareConfigurationEditor = ({
  busy,
  object,
  onOperation,
  onSelectObject,
  snapshot,
}: HardwareConfigurationEditorProps): React.JSX.Element => {
  if (object.kind === "Controller") {
    return (
      <ControllerConfiguration
        busy={busy}
        controller={object}
        onOperation={onOperation}
        onSelectObject={onSelectObject}
        snapshot={snapshot}
      />
    );
  }
  if (object.kind === "Rack") {
    return (
      <RackConfiguration
        busy={busy}
        onOperation={onOperation}
        onSelectObject={onSelectObject}
        rack={object}
        snapshot={snapshot}
      />
    );
  }
  return (
    <ModuleConfiguration
      busy={busy}
      module={object}
      onOperation={onOperation}
      onSelectObject={onSelectObject}
      snapshot={snapshot}
    />
  );
};

type SharedConfigurationProps = Readonly<{
  busy: boolean;
  onOperation: (operation: WorkbenchOperation) => Promise<void>;
  onSelectObject: (objectId: string) => void;
  snapshot: WorkbenchSnapshot;
}>;

const ControllerConfiguration = ({
  busy,
  controller,
  onOperation,
  onSelectObject,
  snapshot,
}: SharedConfigurationProps & Readonly<{ controller: WorkbenchObjectView }>): React.JSX.Element => {
  const catalog = controllerCatalogOption(controller);
  const children = activeChildren(snapshot, controller.id);
  const rack = activeRackForController(snapshot, controller);
  const tagTable = children.find((candidate) => candidate.kind === "SymbolTable") ?? null;
  const ladder = children.find((candidate) =>
    candidate.kind === "OB" && candidate.semanticPayload.language === "LAD"
  ) ?? null;
  const modules = rack === null
    ? []
    : activeRackModules(snapshot, rack).filter((candidate) => digitalModuleCatalog(text(candidate.semanticPayload.catalogId) ?? "") !== null);
  const tagCount = tagTable === null ? 0 : activeChildren(snapshot, tagTable.id).filter((candidate) => candidate.kind === "Tag").length;

  const createMissingChild = async (kind: "ladder" | "rack" | "tags"): Promise<void> => {
    const objectId = crypto.randomUUID();
    const operations: WorkbenchOperation[] = [];
    if (kind === "rack") {
      operations.push(createObject(
        "Local rack",
        objectId,
        "rack",
        controller.id,
        "edu.rack/1",
        { slotCount: unsignedValue(catalog.requiresPowerModule ? catalog.lastSlot + 1 : catalog.expansionSlots) },
      ));
      if (catalog.requiresPowerModule) {
        operations.push(createObject(
          "Virtual power supply",
          crypto.randomUUID(),
          "module",
          objectId,
          "edu.module/1",
          { addressIntent: "auto", catalogId: "vpwr1", slot: unsignedValue(0) },
        ));
      }
    } else if (kind === "tags") {
      operations.push(createObject("PLC tags", objectId, "symbol-table", controller.id, "edu.symbol-table/1", {}));
    } else {
      operations.push(createObject(
        "MainCycle",
        objectId,
        "program-block",
        controller.id,
        "edu.program-block/1",
        createLadProgramPayload(nextObNumber(snapshot, controller.id)),
      ));
    }
    for (const operation of operations) {
      await onOperation(operation);
    }
    onSelectObject(objectId);
  };

  return (
    <div className="hardware-config hardware-config--controller">
      <header className="hardware-config__hero">
        <div className="hardware-config__cpu-mark" aria-hidden="true">
          <span>{catalog.shortLabel}</span>
          <i /><i /><i />
        </div>
        <div>
          <p className="action-kicker">Virtual PLC configuration</p>
          <h1>{controller.displayName}</h1>
          <p>{catalog.label} · {catalog.description}</p>
        </div>
        <span className="hardware-config__virtual-badge">Virtual only</span>
      </header>

      <section className="hardware-spec-grid" aria-label="PLC specifications">
        <article><span>Expansion slots</span><strong>{catalog.expansionSlots}</strong><small>Slots {catalog.firstExpansionSlot}–{catalog.lastSlot}</small></article>
        <article><span>Input image</span><strong>{formatCapacity(catalog.inputBytes)}</strong><small>Virtual I area</small></article>
        <article><span>Output image</span><strong>{formatCapacity(catalog.outputBytes)}</strong><small>Virtual Q area</small></article>
        <article><span>Installed digital I/O</span><strong>{modules.length}</strong><small>{modules.length * 16} channels</small></article>
      </section>

      <section className="hardware-path" aria-labelledby="hardware-path-title">
        <div className="hardware-section-heading">
          <div><p className="action-kicker">Student workflow</p><h2 id="hardware-path-title">Build this PLC in three steps</h2></div>
          <p>Each step edits the same saved project that the compiler and simulator use.</p>
        </div>
        <div className="hardware-path__grid">
          <SetupStep
            actionLabel={rack === null ? "Create rack" : "Configure rack"}
            complete={rack !== null && modules.length > 0}
            description={rack === null ? "Add the local rack, then place input and output cards." : `${modules.length} digital module${modules.length === 1 ? "" : "s"} installed.`}
            disabled={busy}
            number="1"
            onAction={() => rack === null ? void createMissingChild("rack") : onSelectObject(rack.id)}
            title="PLC and I/O"
          />
          <SetupStep
            actionLabel={tagTable === null ? "Create tag table" : "Open PLC tags"}
            complete={tagCount > 0}
            description={tagTable === null ? "Create names for buttons, sensors, lamps, and memory." : `${tagCount} tag${tagCount === 1 ? "" : "s"} ready for the program.`}
            disabled={busy}
            number="2"
            onAction={() => tagTable === null ? void createMissingChild("tags") : onSelectObject(tagTable.id)}
            title="PLC tags"
          />
          <SetupStep
            actionLabel={ladder === null ? "Create MainCycle" : "Open MainCycle"}
            complete={ladder !== null}
            description="Build rungs, compile them, and run the cyclic scan."
            disabled={busy}
            number="3"
            onAction={() => ladder === null ? void createMissingChild("ladder") : onSelectObject(ladder.id)}
            title="Ladder program"
          />
        </div>
      </section>
    </div>
  );
};

const SetupStep = ({
  actionLabel,
  complete,
  description,
  disabled,
  number,
  onAction,
  title,
}: Readonly<{
  actionLabel: string;
  complete: boolean;
  description: string;
  disabled: boolean;
  number: string;
  onAction: () => void;
  title: string;
}>): React.JSX.Element => (
  <article className="hardware-path__step" data-complete={complete}>
    <div className="hardware-path__number">{complete ? "✓" : number}</div>
    <div><h3>{title}</h3><p>{description}</p></div>
    <button disabled={disabled} onClick={onAction} type="button">{actionLabel}<span aria-hidden="true">→</span></button>
  </article>
);

const RackConfiguration = ({
  busy,
  onOperation,
  onSelectObject,
  rack,
  snapshot,
}: SharedConfigurationProps & Readonly<{ rack: WorkbenchObjectView }>): React.JSX.Element => {
  const controller = controllerForRack(snapshot, rack);
  const catalog = controller === null ? null : controllerCatalogOption(controller);
  const modules = activeRackModules(snapshot, rack);
  const digitalModules = modules.filter((candidate) => digitalModuleCatalog(text(candidate.semanticPayload.catalogId) ?? "") !== null);
  const moduleBySlot = new Map(digitalModules.flatMap((module) => {
    const slot = canonicalUnsigned(module.semanticPayload.slot);
    return slot === null ? [] : [[slot, module] as const];
  }));
  const slots = legalModuleSlots(snapshot, rack);
  const firstFree = firstFreeModuleSlot(snapshot, rack);
  const [selectedSlot, setSelectedSlot] = useState<number | null>(firstFree);
  const [adding, setAdding] = useState(false);

  useEffect(() => {
    if (selectedSlot === null || moduleBySlot.has(selectedSlot) || !slots.includes(selectedSlot)) {
      setSelectedSlot(firstFree);
    }
  }, [firstFree, moduleBySlot, selectedSlot, slots]);

  const addModule = async (catalogId: DigitalModuleCatalogId): Promise<void> => {
    if (selectedSlot === null || busy || adding) {
      return;
    }
    const descriptor = digitalModuleCatalog(catalogId);
    if (descriptor === null) {
      return;
    }
    const objectId = crypto.randomUUID();
    setAdding(true);
    try {
      await onOperation(createObject(
        nextModuleName(descriptor.modelName, snapshot, rack.id),
        objectId,
        "module",
        rack.id,
        "edu.module/1",
        createDigitalModulePayload(catalogId, selectedSlot),
      ));
      onSelectObject(objectId);
    } finally {
      setAdding(false);
    }
  };

  return (
    <div className="hardware-config hardware-config--rack">
      <header className="hardware-config__title">
        <div>
          <p className="action-kicker">Device configuration</p>
          <h1>{rack.displayName}</h1>
          <p>{catalog === null ? "Virtual PLC rack" : `${catalog.label} · choose a slot, then install a digital module.`}</p>
        </div>
        <div className="hardware-config__rack-summary"><strong>{digitalModules.length}</strong><span>digital modules</span></div>
      </header>

      {catalog?.requiresPowerModule === true && (
        <section className="rack-fixed-slots" aria-label="Fixed controller slots">
          <div><span>0</span><strong>PS</strong><small>Virtual power</small></div>
          <div><span>1</span><strong>CPU</strong><small>{catalog.shortLabel} controller</small></div>
        </section>
      )}

      <section className="rack-layout" aria-labelledby="rack-layout-title">
        <div className="hardware-section-heading">
          <div><p className="action-kicker">Local rack</p><h2 id="rack-layout-title">Expansion slots</h2></div>
          <p>Module position and address are separate: the slot is physical layout; I/Q is process-image memory.</p>
        </div>
        <div className="rack-slot-strip" role="list" aria-label="Expansion slots">
          {slots.map((slot) => {
            const module = moduleBySlot.get(slot);
            const moduleCatalog = module === undefined ? null : digitalModuleCatalog(text(module.semanticPayload.catalogId) ?? "");
            const selected = module === undefined && selectedSlot === slot;
            return (
              <button
                aria-label={module === undefined ? `Empty slot ${slot}` : `Open ${module.displayName} in slot ${slot}`}
                aria-pressed={module === undefined ? selected : undefined}
                className="rack-slot"
                data-area={moduleCatalog?.addressArea ?? "empty"}
                data-empty={module === undefined}
                data-selected={selected}
                key={slot}
                onClick={() => module === undefined ? setSelectedSlot(slot) : onSelectObject(module.id)}
                role="listitem"
                type="button"
              >
                <span className="rack-slot__number">{slot}</span>
                {module === undefined ? (
                  <><strong>+</strong><small>Empty</small></>
                ) : (
                  <>
                    <strong>{moduleCatalog?.modelName ?? "Module"}</strong>
                    <small>{moduleAddressLabel(module)}</small>
                    <i aria-hidden="true">····</i>
                  </>
                )}
              </button>
            );
          })}
        </div>
      </section>

      <section className="module-picker" aria-labelledby="module-picker-title">
        <div>
          <p className="action-kicker">Module catalog</p>
          <h2 id="module-picker-title">Install in {selectedSlot === null ? "an empty slot" : `slot ${selectedSlot}`}</h2>
          <p>Start with one input card for field signals and one output card for actuators.</p>
        </div>
        <div className="module-picker__options">
          {digitalModuleCatalogs.map((module) => (
            <button
              data-area={module.addressArea}
              disabled={busy || adding || selectedSlot === null}
              key={module.catalogId}
              onClick={() => void addModule(module.catalogId)}
              type="button"
            >
              <span>{module.addressArea}</span>
              <div><strong>{module.displayName}</strong><small>{module.channelCount} BOOL channels · 2 bytes</small></div>
              <b aria-hidden="true">+</b>
            </button>
          ))}
        </div>
        {firstFree === null && <p className="hardware-config__notice">Every expansion slot is occupied. Remove or move a module to make room.</p>}
      </section>
    </div>
  );
};

const ModuleConfiguration = ({
  busy,
  module,
  onOperation,
  onSelectObject,
  snapshot,
}: SharedConfigurationProps & Readonly<{ module: WorkbenchObjectView }>): React.JSX.Element => {
  const source = useMemo(() => readModuleConfiguration(module), [module]);
  const [draft, setDraft] = useState<ModuleConfigurationDraft>(source);
  const [name, setName] = useState(module.displayName);
  const [applying, setApplying] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const fieldId = useId();
  const catalog = digitalModuleCatalog(draft.catalogId);
  const rack = module.parentId === null ? null : snapshot.objects[module.parentId] ?? null;

  useEffect(() => {
    setDraft(source);
    setName(module.displayName);
    setConfirmDelete(false);
  }, [module.displayName, module.id, module.semanticRevision, source]);

  if (catalog === null) {
    return (
      <div className="hardware-config hardware-config--support-module">
        <header className="hardware-config__title"><div><p className="action-kicker">Rack support module</p><h1>{module.displayName}</h1><p>This virtual power component is managed as part of the modular PLC.</p></div></header>
        {rack !== null && <button className="hardware-back-action" onClick={() => onSelectObject(rack.id)} type="button">← Back to rack</button>}
      </div>
    );
  }

  const validation = validateModuleConfiguration(draft, snapshot, module);
  const changed = JSON.stringify(draft) !== JSON.stringify(source) || name.trim() !== module.displayName;
  const disabled = busy || applying;
  const parsedStart = draft.addressIntent === "explicit" && /^\d+$/u.test(draft.startByteText.trim())
    ? Number(draft.startByteText)
    : null;

  const apply = async (): Promise<void> => {
    if (disabled || !validation.valid || !changed) {
      return;
    }
    const operation = buildModuleConfigurationOperation(draft, validation, module);
    if (operation === null) {
      return;
    }
    setApplying(true);
    try {
      const normalizedName = name.trim();
      if (normalizedName !== module.displayName) {
        await onOperation({ displayName: normalizedName, kind: "project.rename-object", objectId: module.id });
      }
      await onOperation(operation);
    } finally {
      setApplying(false);
    }
  };

  const deleteModule = async (): Promise<void> => {
    if (disabled || rack === null) {
      return;
    }
    await onOperation({ kind: "project.delete-object", objectId: module.id });
    onSelectObject(rack.id);
  };

  return (
    <div className="hardware-config hardware-config--module">
      <header className="hardware-config__title">
        <div><p className="action-kicker">Digital {catalog.addressArea === "I" ? "input" : "output"} module</p><h1>{module.displayName}</h1><p>{catalog.description}</p></div>
        <div className="hardware-config__module-mark" data-area={catalog.addressArea}><span>{catalog.addressArea}</span><strong>{catalog.modelName}</strong><small>16 × BOOL</small></div>
      </header>

      <form className="module-config-form" onSubmit={(event) => { event.preventDefault(); void apply(); }}>
        <section>
          <div className="hardware-section-heading"><div><p className="action-kicker">Placement</p><h2>Name and rack slot</h2></div></div>
          <div className="module-config-form__grid">
            <label htmlFor={`${fieldId}-name`}><span>Module name</span><input disabled={disabled} id={`${fieldId}-name`} maxLength={128} onChange={(event) => setName(event.target.value)} value={name} /></label>
            <label htmlFor={`${fieldId}-slot`}><span>Rack slot</span><select aria-invalid={validation.errors.slot !== undefined} disabled={disabled} id={`${fieldId}-slot`} onChange={(event) => setDraft((current) => ({ ...current, slotText: event.target.value }))} value={draft.slotText}>{rack !== null && legalModuleSlots(snapshot, rack).map((slot) => <option key={slot} value={slot}>Slot {slot}</option>)}</select>{validation.errors.slot !== undefined && <em>{validation.errors.slot}</em>}</label>
          </div>
        </section>

        <section>
          <div className="hardware-section-heading"><div><p className="action-kicker">Process image</p><h2>{catalog.addressArea === "I" ? "Input" : "Output"} addressing</h2></div><p>The module occupies two consecutive bytes: sixteen individual bit channels.</p></div>
          <fieldset className="module-address-mode" disabled={disabled}>
            <legend>Allocation mode</legend>
            <label data-selected={draft.addressIntent === "auto"}><input checked={draft.addressIntent === "auto"} name={`${fieldId}-address-mode`} onChange={() => setDraft((current) => ({ ...current, addressIntent: "auto" }))} type="radio" /><span><strong>Automatic</strong><small>The compiler finds the next free two-byte span.</small></span></label>
            <label data-selected={draft.addressIntent === "explicit"}><input checked={draft.addressIntent === "explicit"} name={`${fieldId}-address-mode`} onChange={() => setDraft((current) => ({ ...current, addressIntent: "explicit", startByteText: current.startByteText || "0" }))} type="radio" /><span><strong>Manual</strong><small>You choose the first I/Q byte.</small></span></label>
          </fieldset>
          {draft.addressIntent === "explicit" && (
            <label className="module-start-address" htmlFor={`${fieldId}-start`}>
              <span>Start byte</span><div><b>%{catalog.addressArea}</b><input aria-invalid={validation.errors.startByte !== undefined} disabled={disabled} id={`${fieldId}-start`} inputMode="numeric" onChange={(event) => setDraft((current) => ({ ...current, startByteText: event.target.value }))} value={draft.startByteText} /></div>
              <small>{parsedStart === null ? "Enter a byte number." : `Requested span ${formatModuleAddressRange(catalog.catalogId, parsedStart) ?? "is outside the process image"}.`}</small>
              {validation.errors.startByte !== undefined && <em>{validation.errors.startByte}</em>}
            </label>
          )}
        </section>

        <section>
          <div className="hardware-section-heading"><div><p className="action-kicker">Channel map</p><h2>Sixteen digital channels</h2></div><p>{draft.addressIntent === "auto" ? "Addresses resolve when the project compiles." : "These are the exact requested bit addresses."}</p></div>
          <div className="module-channel-grid" role="list" aria-label={`${module.displayName} channels`}>
            {Array.from({ length: 16 }, (_, channel) => (
              <div key={channel} role="listitem"><span>{String(channel).padStart(2, "0")}</span><strong>{draft.addressIntent === "explicit" && parsedStart !== null ? `%${catalog.addressArea}${parsedStart + Math.floor(channel / 8)}.${channel % 8}` : `Auto +${Math.floor(channel / 8)}.${channel % 8}`}</strong><small>BOOL</small></div>
            ))}
          </div>
        </section>

        <footer className="module-config-form__footer">
          <div><strong>{applying ? "Applying module changes…" : changed ? "Unsaved module changes" : "Module matches the project"}</strong><span>{validation.valid ? "The compiler will perform the final rack and address checks." : "Correct the marked configuration first."}</span></div>
          <div><button disabled={disabled || !changed} onClick={() => { setDraft(source); setName(module.displayName); }} type="button">Reset</button><button className="primary-action" disabled={disabled || !changed || !validation.valid || name.trim().length === 0} type="submit">Apply configuration</button></div>
        </footer>
      </form>

      <section className="module-danger-zone">
        <div><strong>Remove this module</strong><p>Its virtual channels will no longer be available to PLC tags. Undo remains available.</p></div>
        {confirmDelete ? <div><button onClick={() => setConfirmDelete(false)} type="button">Cancel</button><button className="danger-action" disabled={disabled} onClick={() => void deleteModule()} type="button">Confirm remove</button></div> : <button className="danger-action" disabled={disabled} onClick={() => setConfirmDelete(true)} type="button">Remove module</button>}
      </section>
    </div>
  );
};

const createObject = (
  displayName: string,
  objectId: string,
  objectKind: Extract<WorkbenchOperation, Readonly<{ kind: "project.create-object" }>>["objectKind"],
  parentId: string,
  payloadSchema: string,
  semanticPayload: ProjectPayload,
): WorkbenchOperation => ({
  displayName,
  kind: "project.create-object",
  objectId,
  objectKind,
  parentId,
  payloadSchema,
  presentationPayload: {},
  semanticPayload,
});

const nextModuleName = (baseName: string, snapshot: WorkbenchSnapshot, rackId: string): string => {
  const names = new Set(activeChildren(snapshot, rackId).map((object) => object.displayName.toLocaleLowerCase("en-US")));
  if (!names.has(baseName.toLocaleLowerCase("en-US"))) {
    return baseName;
  }
  let suffix = 2;
  while (names.has(`${baseName} ${suffix}`.toLocaleLowerCase("en-US"))) {
    suffix += 1;
  }
  return `${baseName} ${suffix}`;
};

const moduleAddressLabel = (module: WorkbenchObjectView): string => {
  const catalog = digitalModuleCatalog(text(module.semanticPayload.catalogId) ?? "");
  if (catalog === null) {
    return "Support";
  }
  const start = canonicalUnsigned(module.semanticPayload[catalog.addressStartField]);
  return start === null ? "Automatic" : formatModuleAddressRange(catalog.catalogId, start) ?? "Manual";
};

const nextObNumber = (snapshot: WorkbenchSnapshot, controllerId: string): number => {
  const numbers = activeChildren(snapshot, controllerId).flatMap((object) => {
    if (object.kind !== "OB") {
      return [];
    }
    const number = canonicalUnsigned(object.semanticPayload.engineeringNumber);
    return number === null ? [] : [number];
  });
  return Math.min(Math.max(0, ...numbers) + 1, 4_294_967_295);
};

const formatCapacity = (bytes: number): string => bytes >= 1_024 ? `${bytes / 1_024} KB` : `${bytes} B`;

const canonicalUnsigned = (value: ProjectPayloadValue | undefined): number | null => {
  if (typeof value !== "object" || value === null || Array.isArray(value) || !("$type" in value) || value.$type !== "u64" || !("value" in value) || typeof value.value !== "string") {
    return null;
  }
  const parsed = Number(value.value);
  return Number.isSafeInteger(parsed) && parsed >= 0 ? parsed : null;
};

const text = (value: ProjectPayloadValue | undefined): string | null => typeof value === "string" ? value : null;
