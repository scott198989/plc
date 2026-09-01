import { useMemo, useState } from "react";

import {
  activeChildren,
  controllerCatalogOption,
  virtualPlcCatalog,
} from "./plc-setup";
import type { VirtualPlcCatalogId } from "./plc-setup";
import type { WorkbenchObjectView, WorkbenchSnapshot } from "./workbench-types";

export type ProjectSetupPanelProps = Readonly<{
  busy: boolean;
  onCreatePlc: (catalogId: VirtualPlcCatalogId) => Promise<void>;
  onOpenObject: (objectId: string) => void;
  snapshot: WorkbenchSnapshot;
}>;

export const ProjectSetupPanel = ({
  busy,
  onCreatePlc,
  onOpenObject,
  snapshot,
}: ProjectSetupPanelProps): React.JSX.Element => {
  const controller = useMemo(
    () => Object.values(snapshot.objects).find((object) =>
      object.lifecycle === "active" && object.kind === "Controller"
    ) ?? null,
    [snapshot.objects],
  );
  const [selectedCatalogId, setSelectedCatalogId] = useState<VirtualPlcCatalogId>("vctrl-c1");
  const [creating, setCreating] = useState(false);

  if (controller !== null) {
    return (
      <ConfiguredProjectPath
        busy={busy}
        controller={controller}
        onOpenObject={onOpenObject}
        snapshot={snapshot}
      />
    );
  }

  const create = async (): Promise<void> => {
    if (busy || creating) {
      return;
    }
    setCreating(true);
    try {
      await onCreatePlc(selectedCatalogId);
    } finally {
      setCreating(false);
    }
  };

  return (
    <section className="project-setup" aria-labelledby="project-setup-title">
      <div className="project-setup__heading">
        <div>
          <p className="action-kicker">Build your own project</p>
          <h2 id="project-setup-title">Choose a virtual PLC</h2>
          <p>All three run the same student ladder programs. The rack size changes how much I/O you can install.</p>
        </div>
        <span>Step 1 of 3</span>
      </div>
      <div className="plc-catalog" role="radiogroup" aria-label="Virtual PLC model">
        {virtualPlcCatalog.map((catalog) => (
          <label data-selected={catalog.catalogId === selectedCatalogId} key={catalog.catalogId}>
            <input
              checked={catalog.catalogId === selectedCatalogId}
              disabled={busy || creating}
              name="virtual-plc-catalog"
              onChange={() => setSelectedCatalogId(catalog.catalogId)}
              type="radio"
              value={catalog.catalogId}
            />
            <span className="plc-catalog__mark" aria-hidden="true">{catalog.shortLabel}</span>
            <div className="plc-catalog__copy">
              <span>{catalog.recommended === true ? "Recommended first PLC" : `${catalog.expansionSlots} expansion slots`}</span>
              <strong>{catalog.label}</strong>
              <p>{catalog.description}</p>
              <dl>
                <div><dt>I/O image</dt><dd>{catalog.inputBytes / 1_024} KB each</dd></div>
                <div><dt>Rack</dt><dd>{catalog.expansionSlots} slots</dd></div>
              </dl>
            </div>
            <i aria-hidden="true" />
          </label>
        ))}
      </div>
      <div className="project-setup__footer">
        <p><strong>What gets created?</strong> The PLC, its rack, an empty tag table, and a cyclic MainCycle program. You will place the I/O modules yourself.</p>
        <button disabled={busy || creating} onClick={() => void create()} type="button">
          {creating ? "Creating PLC workspace…" : "Create PLC workspace"}<span aria-hidden="true">→</span>
        </button>
      </div>
    </section>
  );
};

const ConfiguredProjectPath = ({
  busy,
  controller,
  onOpenObject,
  snapshot,
}: Readonly<{
  busy: boolean;
  controller: WorkbenchObjectView;
  onOpenObject: (objectId: string) => void;
  snapshot: WorkbenchSnapshot;
}>): React.JSX.Element => {
  const catalog = controllerCatalogOption(controller);
  const controllerChildren = activeChildren(snapshot, controller.id);
  const rack = controllerChildren.find((object) => object.kind === "Rack") ?? null;
  const tagTable = controllerChildren.find((object) => object.kind === "SymbolTable") ?? null;
  const ladder = controllerChildren.find((object) =>
    object.kind === "OB" && object.semanticPayload.language === "LAD"
  ) ?? null;
  const modules = rack === null ? [] : activeChildren(snapshot, rack.id).filter((object) =>
    object.kind === "Module" && (object.semanticPayload.catalogId === "vdi16" || object.semanticPayload.catalogId === "vdo16")
  );
  const tags = tagTable === null ? [] : activeChildren(snapshot, tagTable.id).filter((object) => object.kind === "Tag");
  return (
    <section className="project-path-summary" aria-labelledby="project-path-title">
      <div className="project-path-summary__plc" aria-hidden="true"><span>{catalog.shortLabel}</span><i /><i /><i /></div>
      <div className="project-path-summary__copy">
        <p className="action-kicker">Your virtual PLC</p>
        <h2 id="project-path-title">{controller.displayName} · {catalog.label}</h2>
        <p>Continue the normal engineering path: configure I/O, name the signals, then build the ladder program.</p>
        <div className="project-path-summary__steps">
          <button disabled={busy} onClick={() => onOpenObject(rack?.id ?? controller.id)} type="button"><span data-complete={modules.length >= 2}>{modules.length >= 2 ? "✓" : "1"}</span><div><strong>Configure I/O</strong><small>{modules.length} digital modules</small></div><b>→</b></button>
          <button disabled={busy} onClick={() => onOpenObject(tagTable?.id ?? controller.id)} type="button"><span data-complete={tags.length > 0}>{tags.length > 0 ? "✓" : "2"}</span><div><strong>Create PLC tags</strong><small>{tags.length} named signals</small></div><b>→</b></button>
          <button disabled={busy} onClick={() => onOpenObject(ladder?.id ?? controller.id)} type="button"><span data-complete={ladder !== null}>{ladder !== null ? "✓" : "3"}</span><div><strong>Write ladder</strong><small>{ladder?.displayName ?? "Main program needed"}</small></div><b>→</b></button>
        </div>
      </div>
      <button className="project-path-summary__manage" disabled={busy} onClick={() => onOpenObject(controller.id)} type="button">PLC overview</button>
    </section>
  );
};
