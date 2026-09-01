import { useEffect, useId, useMemo, useRef, useState } from "react";

import {
  addressHelp,
  buildTagWithMemberCreationPlan,
  createTagWithMemberDraftDefaults,
  dataTypesForTagArea,
  discoverLadTagPrograms,
  readTagConfiguration,
  TAG_DESCRIPTION_MAX_LENGTH,
  tagAddressAreas,
  tagKindForArea,
  validateTagWithMemberCreation,
} from "./tag-configuration";
import type {
  TagAddressArea,
  TagWithMemberCreationDraft,
} from "./tag-configuration";
import { activeChildren } from "./plc-setup";
import type {
  ProjectPayloadValue,
  WorkbenchObjectView,
  WorkbenchOperation,
  WorkbenchSnapshot,
} from "./workbench-types";

export type TagTableEditorProps = Readonly<{
  busy: boolean;
  object: WorkbenchObjectView;
  onOperation: (operation: WorkbenchOperation) => Promise<void>;
  onSelectObject: (objectId: string) => void;
  snapshot: WorkbenchSnapshot;
}>;

export const TagTableEditor = ({
  busy,
  object,
  onOperation,
  onSelectObject,
  snapshot,
}: TagTableEditorProps): React.JSX.Element => {
  const tags = useMemo(
    () => activeChildren(snapshot, object.id).filter((candidate) => candidate.kind === "Tag"),
    [object.id, snapshot],
  );
  const programs = useMemo(() => discoverLadTagPrograms(snapshot, object), [object, snapshot]);
  const [creatorOpen, setCreatorOpen] = useState(false);
  const [filter, setFilter] = useState("");
  const [pendingDeleteId, setPendingDeleteId] = useState<string | null>(null);
  const visibleTags = tags.filter((tag) => {
    const needle = filter.trim().toLocaleLowerCase("en-US");
    if (needle.length === 0) {
      return true;
    }
    const configuration = readTagConfiguration(tag);
    return [tag.displayName, configuration.description, configuration.addressText, configuration.dataType]
      .some((value) => value.toLocaleLowerCase("en-US").includes(needle));
  });
  const counts = Object.fromEntries(tagAddressAreas.map((area) => [
    area,
    tags.filter((tag) => readTagConfiguration(tag).area === area).length,
  ])) as Readonly<Record<TagAddressArea, number>>;

  return (
    <div className="tag-table-editor">
      <header className="tag-table-editor__header">
        <div>
          <p className="action-kicker">PLC data</p>
          <h1>{object.displayName}</h1>
          <p>Name the real-world signals your ladder program will read, write, and remember.</p>
        </div>
        <button className="tag-table-editor__add" disabled={busy || programs.length === 0} onClick={() => setCreatorOpen(true)} type="button"><span aria-hidden="true">+</span> New PLC tag</button>
      </header>

      <section className="tag-table-summary" aria-label="Tag summary">
        <div data-area="I"><span>I</span><strong>{counts.I}</strong><small>Inputs</small></div>
        <div data-area="Q"><span>Q</span><strong>{counts.Q}</strong><small>Outputs</small></div>
        <div data-area="M"><span>M</span><strong>{counts.M}</strong><small>Memory</small></div>
        <p><strong>{tags.length} total tags</strong><span>Every tag is bound to a real MainCycle variable and saved with the project.</span></p>
      </section>

      {programs.length === 0 && (
        <section className="tag-table-editor__notice" role="status">
          <span aria-hidden="true">LD</span>
          <div><strong>Create a ladder MainCycle first</strong><p>A PLC tag needs one program variable to carry its value through the scan. Select the controller and create a ladder program.</p></div>
        </section>
      )}

      <section className="tag-table-card" aria-labelledby="tag-list-title">
        <div className="tag-table-toolbar">
          <div><p className="action-kicker">Tag list</p><h2 id="tag-list-title">Configured signals</h2></div>
          <label><span>Filter tags</span><input aria-label="Filter PLC tags" onChange={(event) => setFilter(event.target.value)} placeholder="Search name, address, or description" type="search" value={filter} /></label>
        </div>
        {visibleTags.length === 0 ? (
          <div className="tag-table-empty">
            <span aria-hidden="true">I0.0</span>
            <h3>{tags.length === 0 ? "No PLC tags yet" : "No tags match that filter"}</h3>
            <p>{tags.length === 0 ? "Create a tag for a pushbutton, sensor, lamp, motor, or internal memory value." : "Try a different name, address, or description."}</p>
            {tags.length === 0 && <button disabled={busy || programs.length === 0} onClick={() => setCreatorOpen(true)} type="button">Create first tag</button>}
          </div>
        ) : (
          <div className="tag-table-scroll">
            <table className="tag-table">
              <thead><tr><th scope="col">Name</th><th scope="col">Area</th><th scope="col">Type</th><th scope="col">Address</th><th scope="col">Description</th><th scope="col">Program variable</th><th scope="col"><span className="visually-hidden">Actions</span></th></tr></thead>
              <tbody>
                {visibleTags.map((tag) => {
                  const config = readTagConfiguration(tag);
                  const blockId = text(tag.semanticPayload.blockId);
                  const memberId = text(tag.semanticPayload.memberId);
                  const program = blockId === null ? null : snapshot.objects[blockId] ?? null;
                  const memberName = program === null || memberId === null ? null : findMemberName(program, memberId);
                  return (
                    <tr key={tag.id}>
                      <th scope="row"><button onClick={() => onSelectObject(tag.id)} type="button">{tag.displayName}</button></th>
                      <td><span className="tag-area-chip" data-area={config.area}>{config.area}</span></td>
                      <td><code>{config.dataType}</code></td>
                      <td><code>{config.area === "M" ? "Memory" : config.addressIntent === "auto" ? "Automatic" : config.addressText}</code></td>
                      <td><span className="tag-description-cell">{config.description || "—"}</span></td>
                      <td><span className="tag-binding-cell"><strong>{memberName ?? "Unavailable"}</strong><small>{program?.displayName ?? "Missing program"}</small></span></td>
                      <td>
                        <div className="tag-row-actions">
                          <button aria-label={`Edit ${tag.displayName}`} onClick={() => onSelectObject(tag.id)} title="Edit tag" type="button">Edit</button>
                          {pendingDeleteId === tag.id ? (
                            <><button onClick={() => setPendingDeleteId(null)} type="button">Cancel</button><button className="danger-action" disabled={busy} onClick={() => { void onOperation({ kind: "project.delete-object", objectId: tag.id }); setPendingDeleteId(null); }} type="button">Confirm</button></>
                          ) : (
                            <button aria-label={`Delete ${tag.displayName}`} disabled={busy} onClick={() => setPendingDeleteId(tag.id)} title="Delete tag" type="button">Delete</button>
                          )}
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </section>

      {creatorOpen && (
        <TagCreator
          busy={busy}
          onClose={() => setCreatorOpen(false)}
          onCreated={onSelectObject}
          onOperation={onOperation}
          programs={programs}
          snapshot={snapshot}
          symbolTable={object}
        />
      )}
    </div>
  );
};

const TagCreator = ({
  busy,
  onClose,
  onCreated,
  onOperation,
  programs,
  snapshot,
  symbolTable,
}: Readonly<{
  busy: boolean;
  onClose: () => void;
  onCreated: (tagId: string) => void;
  onOperation: (operation: WorkbenchOperation) => Promise<void>;
  programs: readonly Readonly<{ id: string; name: string }>[];
  snapshot: WorkbenchSnapshot;
  symbolTable: WorkbenchObjectView;
}>): React.JSX.Element => {
  const [draft, setDraft] = useState<TagWithMemberCreationDraft>(() =>
    createTagWithMemberDraftDefaults(programs[0]?.id ?? null)
  );
  const [creating, setCreating] = useState(false);
  const nameRef = useRef<HTMLInputElement>(null);
  const dialogId = useId();
  const validation = validateTagWithMemberCreation(draft, snapshot, symbolTable);
  const errors = Object.values(validation.errors);
  const disabled = busy || creating;

  useEffect(() => nameRef.current?.focus(), []);

  const updateArea = (area: TagAddressArea): void => {
    setDraft((current) => ({
      ...current,
      addressIntent: area === "M" ? "auto" : current.addressIntent,
      addressText: area === "M" ? "" : manualAddress(area, "BOOL"),
      area,
      dataType: "BOOL",
    }));
  };

  const create = async (): Promise<void> => {
    if (disabled || !validation.valid) {
      return;
    }
    const plan = buildTagWithMemberCreationPlan(draft, validation, snapshot, symbolTable);
    if (plan === null) {
      return;
    }
    setCreating(true);
    try {
      for (const operation of plan.operations) {
        await onOperation(operation);
      }
      onClose();
      onCreated(plan.tagId);
    } finally {
      setCreating(false);
    }
  };

  return (
    <div className="tag-create-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget && !disabled) onClose(); }}>
      <section aria-labelledby={`${dialogId}-title`} aria-modal="true" className="tag-create-dialog" role="dialog">
        <header><div><p className="action-kicker">New PLC tag</p><h2 id={`${dialogId}-title`}>Create a named signal</h2><p>The matching MainCycle variable is created automatically, so the tag is immediately usable in ladder.</p></div><button aria-label="Close new tag form" disabled={disabled} onClick={onClose} type="button">×</button></header>
        {errors.length > 0 && draft.name.length > 0 && <div className="tag-create-errors" role="alert"><strong>Review the marked fields</strong><ul>{errors.map((error) => <li key={error}>{error}</li>)}</ul></div>}
        <form onSubmit={(event) => { event.preventDefault(); void create(); }}>
          <div className="tag-create-grid">
            <label className="tag-create-field"><span>Tag name</span><input aria-invalid={validation.errors.name !== undefined} autoComplete="off" disabled={disabled} maxLength={128} onChange={(event) => setDraft((current) => ({ ...current, name: event.target.value }))} placeholder="Start_PB" ref={nameRef} spellCheck="false" value={draft.name} /><small>Letters, numbers, and underscores.</small>{validation.errors.name !== undefined && <em>{validation.errors.name}</em>}</label>
            <label className="tag-create-field"><span>Data type</span><select aria-invalid={validation.errors.dataType !== undefined} disabled={disabled} onChange={(event) => setDraft((current) => ({ ...current, dataType: event.target.value, addressText: current.addressIntent === "explicit" ? manualAddress(current.area, event.target.value) : current.addressText }))} value={draft.dataType}>{dataTypesForTagArea(draft.area).map((type) => <option key={type} value={type}>{type}</option>)}</select><small>{draft.area === "M" ? "Program memory supports PLC scalar types." : "Digital BOOL or word-sized INT."}</small></label>
          </div>

          <fieldset className="tag-create-areas" disabled={disabled}><legend>Where does this value live?</legend>{tagAddressAreas.map((area) => <label data-selected={draft.area === area} key={area}><input checked={draft.area === area} name={`${dialogId}-area`} onChange={() => updateArea(area)} type="radio" /><span>{area}</span><div><strong>{tagKindForArea(area)}</strong><small>{area === "I" ? "Button or sensor" : area === "Q" ? "Lamp or actuator" : "Internal PLC value"}</small></div></label>)}</fieldset>

          {draft.area !== "M" && (
            <div className="tag-create-address">
              <fieldset disabled={disabled}><legend>I/O allocation</legend><label data-selected={draft.addressIntent === "auto"}><input checked={draft.addressIntent === "auto"} name={`${dialogId}-intent`} onChange={() => setDraft((current) => ({ ...current, addressIntent: "auto" }))} type="radio" /><span><strong>Automatic</strong><small>Next compatible channel</small></span></label><label data-selected={draft.addressIntent === "explicit"}><input checked={draft.addressIntent === "explicit"} name={`${dialogId}-intent`} onChange={() => setDraft((current) => ({ ...current, addressIntent: "explicit", addressText: current.addressText || manualAddress(current.area, current.dataType) }))} type="radio" /><span><strong>Manual</strong><small>Exact process-image address</small></span></label></fieldset>
              {draft.addressIntent === "explicit" && <label className="tag-create-field"><span>Manual address</span><input aria-invalid={validation.errors.address !== undefined} autoCapitalize="characters" disabled={disabled} onChange={(event) => setDraft((current) => ({ ...current, addressText: event.target.value }))} spellCheck="false" value={draft.addressText} /><small>{addressHelp(draft.area, draft.dataType)}</small>{validation.errors.address !== undefined && <em>{validation.errors.address}</em>}</label>}
            </div>
          )}

          <label className="tag-create-field"><span>Description</span><textarea aria-invalid={validation.errors.description !== undefined} disabled={disabled} maxLength={TAG_DESCRIPTION_MAX_LENGTH} onChange={(event) => setDraft((current) => ({ ...current, description: event.target.value }))} placeholder="What this signal represents in the machine" rows={3} value={draft.description} /><small>{draft.description.length}/{TAG_DESCRIPTION_MAX_LENGTH} characters</small>{validation.errors.description !== undefined && <em>{validation.errors.description}</em>}</label>

          <label className="tag-create-field"><span>Ladder program</span><select aria-invalid={validation.errors.binding !== undefined} disabled={disabled} onChange={(event) => setDraft((current) => ({ ...current, programId: event.target.value || null }))} value={draft.programId ?? ""}><option value="">Choose a LAD program</option>{programs.map((program) => <option key={program.id} value={program.id}>{program.name}</option>)}</select><small>A same-name variable is added to this program and bound to the tag.</small>{validation.errors.binding !== undefined && <em>{validation.errors.binding}</em>}</label>

          <footer><div><strong>{creating ? "Creating tag and program variable…" : "Ready to create"}</strong><span>{validation.valid ? "This will add two ordinary, undoable project edits." : "Complete the required fields."}</span></div><div><button disabled={disabled} onClick={onClose} type="button">Cancel</button><button className="primary-action" disabled={disabled || !validation.valid} type="submit">Create PLC tag</button></div></footer>
        </form>
      </section>
    </div>
  );
};

const findMemberName = (program: WorkbenchObjectView, memberId: string): string | null => {
  const members = program.semanticPayload.interface ?? program.semanticPayload.members;
  if (!Array.isArray(members)) return null;
  for (const member of members) {
    const fields = recordFields(member);
    if (fields?.id === memberId && typeof fields.name === "string") return fields.name;
  }
  return null;
};

const recordFields = (value: ProjectPayloadValue): Readonly<Record<string, ProjectPayloadValue>> | null => {
  if (typeof value !== "object" || value === null || Array.isArray(value) || !("$type" in value) || value.$type !== "record" || !("value" in value) || typeof value.value !== "object" || value.value === null || Array.isArray(value.value)) return null;
  return value.value;
};

const manualAddress = (area: TagAddressArea, dataType: string): string => dataType === "BOOL" ? `%${area}0.0` : `%${area}W0`;
const text = (value: ProjectPayloadValue | undefined): string | null => typeof value === "string" ? value : null;
