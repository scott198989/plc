import { useEffect, useId, useMemo, useState } from "react";

import {
  addressHelp,
  buildTagConfigurationOperations,
  compatibleTagBindings,
  dataTypesForTagArea,
  discoverTagBindings,
  readTagConfiguration,
  TAG_DESCRIPTION_MAX_LENGTH,
  tagAddressAreas,
  tagConfigurationChanged,
  tagKindForArea,
  validateTagConfiguration,
} from "./tag-configuration";
import type {
  TagAddressArea,
  TagBindingOption,
  TagConfigurationDraft,
} from "./tag-configuration";
import type {
  WorkbenchObjectView,
  WorkbenchOperation,
  WorkbenchSnapshot,
} from "./workbench-types";

export type TagConfigurationEditorProps = Readonly<{
  busy: boolean;
  object: WorkbenchObjectView;
  onOperation: (operation: WorkbenchOperation) => Promise<void>;
  snapshot: WorkbenchSnapshot;
}>;

const areaLabels: Readonly<Record<TagAddressArea, Readonly<{
  description: string;
  label: string;
}>>> = {
  I: { description: "Read a virtual input channel", label: "Input · I" },
  Q: { description: "Drive a virtual output channel", label: "Output · Q" },
  M: { description: "Keep a value in program memory", label: "Memory · M" },
};

/**
 * Learner-facing editor for one canonical edu.tag/1 object. All writes are
 * ordinary workbench operations; this component never owns a second tag model.
 */
export const TagConfigurationEditor = ({
  busy,
  object,
  onOperation,
  snapshot,
}: TagConfigurationEditorProps): React.JSX.Element => {
  const source = useMemo(
    () => readTagConfiguration(object),
    [object.displayName, object.id, object.semanticPayload, object.semanticRevision],
  );
  const bindings = useMemo(
    () => discoverTagBindings(snapshot, object),
    [object.id, snapshot],
  );
  const [draft, setDraft] = useState<TagConfigurationDraft>(source);
  const [applying, setApplying] = useState(false);
  const [applyError, setApplyError] = useState<string | null>(null);
  const fieldId = useId();

  useEffect(() => {
    setDraft(source);
    setApplyError(null);
  }, [source]);

  const compatibleBindings = useMemo(
    () => compatibleTagBindings(bindings, draft.area, draft.dataType),
    [bindings, draft.area, draft.dataType],
  );
  const bindingUnavailable =
    source.bindingKey !== null && !bindings.some((candidate) => candidate.key === source.bindingKey);
  const validation = useMemo(
    () => validateTagConfiguration(draft, source, bindings, snapshot, object),
    [bindings, draft, object, snapshot, source],
  );
  const changed = tagConfigurationChanged(draft, source);
  const disabled = busy || applying;
  const errors = Object.values(validation.errors);

  const apply = async (): Promise<void> => {
    if (disabled || !changed || !validation.valid) {
      return;
    }
    const operations = buildTagConfigurationOperations(
      draft,
      source,
      validation,
      bindings,
      object,
    );
    if (operations.length === 0) {
      return;
    }
    setApplying(true);
    setApplyError(null);
    try {
      for (const operation of operations) {
        await onOperation(operation);
      }
    } catch (reason) {
      setApplyError(reason instanceof Error ? reason.message : "The tag changes could not be applied.");
    } finally {
      setApplying(false);
    }
  };

  return (
    <div className="tag-config">
      <header className="tag-config__header">
        <div>
          <p className="action-kicker">PLC tag configuration</p>
          <h1>{object.displayName}</h1>
          <p>Connect a student-friendly tag name to one real program variable and, when needed, a virtual I/O channel.</p>
        </div>
        <div aria-label="Current tag classification" className="tag-config__classification">
          <span data-area={draft.area}>{draft.area}</span>
          <div>
            <strong>{tagKindForArea(draft.area)} tag</strong>
            <small>{draft.addressIntent === "auto" ? "Automatic allocation" : draft.addressText}</small>
          </div>
        </div>
      </header>

      {(errors.length > 0 || applyError !== null) && (
        <section
          aria-live="polite"
          className="tag-config__error-summary"
          id={`${fieldId}-errors`}
          role="alert"
        >
          <strong>Review this tag before applying</strong>
          <ul>
            {errors.map((message) => <li key={message}>{message}</li>)}
            {applyError !== null && <li>{applyError}</li>}
          </ul>
        </section>
      )}

      <form
        className="tag-config__form"
        onSubmit={(event) => {
          event.preventDefault();
          void apply();
        }}
      >
        <section className="tag-config__section">
          <header>
            <span>01</span>
            <div>
              <h2>Name and value</h2>
              <p>Use the same short PLC name students will see in ladder logic, watch tables, and the trainer.</p>
            </div>
          </header>
          <div className="tag-config__field-grid">
            <label className="tag-config__field" htmlFor={`${fieldId}-name`}>
              <span>Tag name</span>
              <input
                aria-describedby={validation.errors.name === undefined ? `${fieldId}-name-help` : `${fieldId}-name-error`}
                aria-invalid={validation.errors.name !== undefined}
                autoComplete="off"
                disabled={disabled}
                id={`${fieldId}-name`}
                maxLength={128}
                onChange={(event) => setDraft((current) => ({ ...current, name: event.target.value }))}
                spellCheck="false"
                value={draft.name}
              />
              <small id={`${fieldId}-name-help`}>Letters, digits, and underscores; for example <code>Start_PB</code>.</small>
              {validation.errors.name !== undefined && <em id={`${fieldId}-name-error`}>{validation.errors.name}</em>}
            </label>

            <label className="tag-config__field" htmlFor={`${fieldId}-type`}>
              <span>Data type</span>
              <select
                aria-describedby={validation.errors.dataType === undefined ? `${fieldId}-type-help` : `${fieldId}-type-error`}
                aria-invalid={validation.errors.dataType !== undefined}
                disabled={disabled}
                id={`${fieldId}-type`}
                onChange={(event) => setDraft((current) => ({
                  ...current,
                  addressText: current.addressIntent === "explicit"
                    ? manualAddress(current.area, event.target.value)
                    : current.addressText,
                  bindingKey: compatibleTagBindings(bindings, current.area, event.target.value)
                    .some((candidate) => candidate.key === current.bindingKey)
                      ? current.bindingKey
                      : null,
                  dataType: event.target.value,
                }))}
                value={draft.dataType}
              >
                {dataTypesForTagArea(draft.area).map((dataType) => (
                  <option key={dataType} value={dataType}>{dataType}</option>
                ))}
              </select>
              <small id={`${fieldId}-type-help`}>
                {draft.area === "M" ? "PLC scalar stored in program memory." : "Digital BOOL or word-sized INT I/O."}
              </small>
              {validation.errors.dataType !== undefined && <em id={`${fieldId}-type-error`}>{validation.errors.dataType}</em>}
            </label>
          </div>
          <label className="tag-config__field tag-config__field--description" htmlFor={`${fieldId}-description`}>
            <span>Description</span>
            <textarea
              aria-describedby={validation.errors.description === undefined ? `${fieldId}-description-help` : `${fieldId}-description-error`}
              aria-invalid={validation.errors.description !== undefined}
              disabled={disabled}
              id={`${fieldId}-description`}
              maxLength={TAG_DESCRIPTION_MAX_LENGTH}
              onChange={(event) => setDraft((current) => ({ ...current, description: event.target.value }))}
              placeholder="What this tag represents in the machine"
              rows={3}
              value={draft.description}
            />
            <small id={`${fieldId}-description-help`}>{draft.description.length}/{TAG_DESCRIPTION_MAX_LENGTH} characters · shown in the tag table.</small>
            {validation.errors.description !== undefined && <em id={`${fieldId}-description-error`}>{validation.errors.description}</em>}
          </label>
        </section>

        <section className="tag-config__section">
          <header>
            <span>02</span>
            <div>
              <h2>Tag area</h2>
              <p>Choose where the value enters, leaves, or lives inside the simulated PLC.</p>
            </div>
          </header>
          <fieldset className="tag-config__areas" disabled={disabled}>
            <legend>Tag kind and address area</legend>
            {tagAddressAreas.map((area) => (
              <label data-selected={draft.area === area} key={area}>
                <input
                  checked={draft.area === area}
                  name={`${fieldId}-area`}
                  onChange={() => setDraft((current) => {
                    const availableTypes = dataTypesForTagArea(area) as readonly string[];
                    const dataType = availableTypes.includes(current.dataType) ? current.dataType : "BOOL";
                    const bindingStillCompatible = compatibleTagBindings(bindings, area, dataType)
                      .some((candidate) => candidate.key === current.bindingKey);
                    return {
                      ...current,
                      addressIntent: area === "M" ? "auto" : current.addressIntent,
                      addressText: area === "M" ? "" : manualAddress(area, dataType),
                      area,
                      bindingKey: bindingStillCompatible ? current.bindingKey : null,
                      dataType,
                    };
                  })}
                  type="radio"
                  value={area}
                />
                <span aria-hidden="true">{area}</span>
                <div>
                  <strong>{areaLabels[area].label}</strong>
                  <small>{areaLabels[area].description}</small>
                </div>
              </label>
            ))}
          </fieldset>
          <p className="tag-config__locked-note">Changing area or type may require choosing another compatible program variable below. The complete tag payload is applied as one undoable project edit.</p>
        </section>

        <section className="tag-config__section">
          <header>
            <span>03</span>
            <div>
              <h2>{draft.area === "M" ? "Memory allocation" : "Virtual I/O address"}</h2>
              <p>{addressHelp(draft.area, draft.dataType)}</p>
            </div>
          </header>
          {draft.area === "M" ? (
            <div className="tag-config__automatic-note">
              <strong>Automatic program memory</strong>
              <span>The compiler resolves this tag to the selected canonical variable on every build.</span>
            </div>
          ) : (
            <div className="tag-config__address-panel">
              <fieldset className="tag-config__intent" disabled={disabled}>
                <legend>Channel allocation</legend>
                <label data-selected={draft.addressIntent === "auto"}>
                  <input
                    checked={draft.addressIntent === "auto"}
                    name={`${fieldId}-intent`}
                    onChange={() => setDraft((current) => ({ ...current, addressIntent: "auto" }))}
                    type="radio"
                  />
                  <span><strong>Automatic</strong><small>Use the next compatible virtual channel.</small></span>
                </label>
                <label data-selected={draft.addressIntent === "explicit"}>
                  <input
                    checked={draft.addressIntent === "explicit"}
                    name={`${fieldId}-intent`}
                    onChange={() => setDraft((current) => ({
                      ...current,
                      addressIntent: "explicit",
                      addressText: current.addressText || manualAddress(current.area, current.dataType),
                    }))}
                    type="radio"
                  />
                  <span><strong>Manual</strong><small>Request one exact process-image address.</small></span>
                </label>
              </fieldset>
              {draft.addressIntent === "explicit" && (
                <label className="tag-config__field tag-config__field--address" htmlFor={`${fieldId}-address`}>
                  <span>Manual address</span>
                  <input
                    aria-describedby={validation.errors.address === undefined ? `${fieldId}-address-help` : `${fieldId}-address-error`}
                    aria-invalid={validation.errors.address !== undefined}
                    autoCapitalize="characters"
                    autoComplete="off"
                    disabled={disabled}
                    id={`${fieldId}-address`}
                    onChange={(event) => setDraft((current) => ({ ...current, addressText: event.target.value }))}
                    spellCheck="false"
                    value={draft.addressText}
                  />
                  <small id={`${fieldId}-address-help`}>{addressHelp(draft.area, draft.dataType)}</small>
                  {validation.errors.address !== undefined && <em id={`${fieldId}-address-error`}>{validation.errors.address}</em>}
                </label>
              )}
            </div>
          )}
        </section>

        <section className="tag-config__section">
          <header>
            <span>04</span>
            <div>
              <h2>Program binding</h2>
              <p>The tag and the PLC variable share one value; no duplicate simulator state is created.</p>
            </div>
          </header>
          <div className="tag-config__binding-stack">
            {bindingUnavailable && (
              <div className="tag-config__binding-warning" role="status">
              <strong>Current binding is preserved</strong>
              <p>The referenced program variable is not discoverable in this project view. Keep this tag’s current area and type, or choose a compatible variable below.</p>
              <code>{source.bindingKey}</code>
              </div>
            )}
            <label className="tag-config__field tag-config__field--binding" htmlFor={`${fieldId}-binding`}>
              <span>Program variable</span>
              <select
                aria-describedby={validation.errors.binding === undefined ? `${fieldId}-binding-help` : `${fieldId}-binding-error`}
                aria-invalid={validation.errors.binding !== undefined}
                disabled={disabled || compatibleBindings.length === 0}
                id={`${fieldId}-binding`}
                onChange={(event) => setDraft((current) => ({ ...current, bindingKey: event.target.value || null }))}
                value={compatibleBindings.some((candidate) => candidate.key === draft.bindingKey) ? draft.bindingKey ?? "" : ""}
              >
                <option value="">Choose a program variable</option>
                {bindingGroups(compatibleBindings).map(([blockName, options]) => (
                  <optgroup key={blockName} label={blockName}>
                    {options.map((candidate) => (
                      <option key={candidate.key} value={candidate.key}>
                        {candidate.memberName} · {candidate.dataType} · {roleLabel(candidate.role)}
                      </option>
                    ))}
                  </optgroup>
                ))}
              </select>
              <small id={`${fieldId}-binding-help`}>
                {compatibleBindings.length === 0
                  ? `No compatible ${draft.dataType} variables were found. Create one from the PLC tag table first.`
                  : `${compatibleBindings.length} compatible variable${compatibleBindings.length === 1 ? "" : "s"} found in this controller.`}
              </small>
              {validation.errors.binding !== undefined && <em id={`${fieldId}-binding-error`}>{validation.errors.binding}</em>}
            </label>
          </div>
        </section>

        <footer className="tag-config__footer">
          <div aria-live="polite">
            <strong>{applying ? "Applying tag changes…" : changed ? "Unsaved tag changes" : "Tag matches the project"}</strong>
            <span>{validation.valid ? "Build will perform final channel and program checks." : "Resolve the fields marked above."}</span>
          </div>
          <div>
            <button
              className="tag-config__reset"
              disabled={disabled || !changed}
              onClick={() => setDraft(source)}
              type="button"
            >
              Reset
            </button>
            <button
              className="tag-config__apply"
              disabled={disabled || !changed || !validation.valid}
              type="submit"
            >
              Apply tag changes
            </button>
          </div>
        </footer>
      </form>
    </div>
  );
};

const manualAddress = (area: TagAddressArea, dataType: string): string => {
  if (area === "M") {
    return "";
  }
  return dataType === "BOOL" ? `%${area}0.0` : `%${area}W0`;
};

const bindingGroups = (
  bindings: readonly TagBindingOption[],
): readonly (readonly [string, readonly TagBindingOption[]])[] => {
  const groups = new Map<string, TagBindingOption[]>();
  for (const binding of bindings) {
    const label = `${binding.blockName} · ${binding.blockKind}`;
    const group = groups.get(label) ?? [];
    group.push(binding);
    groups.set(label, group);
  }
  return [...groups.entries()];
};

const roleLabel = (role: string): string => {
  switch (role) {
    case "input": return "input";
    case "output": return "output";
    case "inout": return "in/out";
    case "static": return "static memory";
    case "temp": return "local";
    case "return": return "return";
    default: return role;
  }
};
