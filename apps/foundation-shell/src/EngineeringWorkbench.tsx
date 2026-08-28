import { useEffect, useMemo, useState } from "react";

import type {
  ProjectPayload,
  ProjectStorageKind,
  WorkbenchObjectView,
  WorkbenchOperation,
  WorkbenchSnapshot,
} from "./workbench-types";

type EngineeringWorkbenchProps = Readonly<{
  busy: boolean;
  error: string | null;
  onClose: () => void;
  onOperation: (operation: WorkbenchOperation) => Promise<void>;
  onSave: (mode: "save" | "save-as") => Promise<void>;
  snapshot: WorkbenchSnapshot;
}>;

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

export const EngineeringWorkbench = ({
  busy,
  error,
  onClose,
  onOperation,
  onSave,
  snapshot,
}: EngineeringWorkbenchProps): React.JSX.Element => {
  const [selectedId, setSelectedId] = useState(snapshot.projectRootId);
  const [openTabs, setOpenTabs] = useState<readonly string[]>([snapshot.projectRootId]);
  const [diagnosticsOpen, setDiagnosticsOpen] = useState(true);
  const [createMenuOpen, setCreateMenuOpen] = useState(false);

  const selected = snapshot.objects[selectedId] ?? snapshot.objects[snapshot.projectRootId];
  const resolvedSelectedId = selected?.id ?? snapshot.projectRootId;

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
    const onKeyDown = (event: KeyboardEvent): void => {
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
      if (!isEditing && event.key === "Delete" && resolvedSelectedId !== snapshot.projectRootId) {
        event.preventDefault();
        void onOperation({ kind: "project.delete-object", objectId: resolvedSelectedId });
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [busy, onOperation, onSave, resolvedSelectedId, snapshot.projectRootId, snapshot.undo]);

  const selectObject = (objectId: string): void => {
    const object = snapshot.objects[objectId];
    if (object === undefined || object.lifecycle !== "active") {
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
  const tombstoneCount = Object.values(snapshot.objects).length - activeObjects.length;
  const blockingCount = snapshot.diagnostics.filter((diagnostic) => diagnostic.blocking).length;
  const createOptions = selected === undefined ? [] : creationOptions(selected, snapshot);

  const createObject = async (template: CreateObjectTemplate): Promise<void> => {
    const objectId = crypto.randomUUID();
    await onOperation({
      displayName: nextObjectName(template.baseName, resolvedSelectedId, snapshot),
      kind: "project.create-object",
      objectId,
      objectKind: template.objectKind,
      parentId: resolvedSelectedId,
      payloadSchema: template.payloadSchema,
      presentationPayload: {},
      semanticPayload: template.semanticPayload,
    });
    setCreateMenuOpen(false);
    setSelectedId(objectId);
    setOpenTabs((current) => [...current, objectId]);
  };

  return (
    <div className="workbench-shell">
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
          <button className="icon-button icon-button--menu" disabled={busy} onClick={() => void onSave("save-as")} title="Save as" type="button">
            ⋯
          </button>
          <button className="text-button text-button--quiet" disabled={busy} onClick={onClose} type="button">
            Close
          </button>
        </div>
      </header>

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

        <main className="editor-region" id="main-content">
          <div className="editor-tabs" role="tablist" aria-label="Open engineering objects">
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

          <section className="object-editor" role="tabpanel">
            {selected === undefined ? (
              <p>Object unavailable.</p>
            ) : selected.kind === "ProjectRoot" ? (
              <ProjectOverview
                activeCount={activeObjects.length}
                blockingCount={blockingCount}
                snapshot={snapshot}
                tombstoneCount={tombstoneCount}
              />
            ) : isSclProgramBlock(selected) ? (
              <SclProgramEditor busy={busy} object={selected} onOperation={onOperation} />
            ) : (
              <ObjectOverview object={selected} snapshot={snapshot} />
            )}
          </section>

          <section className="diagnostics-pane" data-open={diagnosticsOpen}>
            <button
              aria-expanded={diagnosticsOpen}
              className="diagnostics-heading"
              onClick={() => setDiagnosticsOpen((open) => !open)}
              type="button"
            >
              <span>Diagnostics</span>
              <span className="diagnostics-summary">
                {blockingCount > 0 ? `${blockingCount} blocking` : "No blocking issues"}
                <span aria-hidden="true">{diagnosticsOpen ? "⌄" : "⌃"}</span>
              </span>
            </button>
            {diagnosticsOpen && (
              <div className="diagnostics-list" role="list">
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
  snapshot: WorkbenchSnapshot;
  tombstoneCount: number;
}>;

const ProjectOverview = ({
  activeCount,
  blockingCount,
  snapshot,
  tombstoneCount,
}: ProjectOverviewProps): React.JSX.Element => (
  <div className="overview-layout">
    <header className="editor-title">
      <p className="eyebrow">Project overview</p>
      <h1>{snapshot.projectName}</h1>
      <p>Canonical project identity, revision state, and integrity at a glance.</p>
    </header>
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

const isSclProgramBlock = (object: WorkbenchObjectView): boolean =>
  (object.kind === "OB" || object.kind === "FC" || object.kind === "FB") &&
  object.semanticPayload.language === "SCL";

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
          placeholder={"InputValue := 7;\nOutputValue := InputValue + 5;"}
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
  useEffect(() => setName(object.displayName), [object.displayName, object.id]);
  const isRoot = object.id === projectRootId;

  return (
    <aside className="properties-pane" aria-label="Properties">
      <div className="pane-heading"><span>Properties</span></div>
      <form
        className="properties-form"
        onSubmit={(event) => {
          event.preventDefault();
          const normalized = name.trim();
          if (!busy && normalized.length > 0 && normalized !== object.displayName) {
            void onOperation({
              displayName: normalized,
              kind: "project.rename-object",
              objectId: object.id,
            });
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
        <button
          className="property-apply"
          disabled={busy || name.trim().length === 0 || name.trim() === object.displayName}
          type="submit"
        >Apply name</button>
      </form>
      {!isRoot && (
        <div className="object-actions">
          <p className="property-section-title">Object actions</p>
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
          <button
            className="danger-action"
            disabled={busy}
            onClick={() => void onOperation({ kind: "project.delete-object", objectId: object.id })}
            type="button"
          >Delete object</button>
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
  semanticPayload: ProjectPayload;
}>;

const unsigned = (value: number): Readonly<{ $type: "u64"; value: string }> => ({
  $type: "u64",
  value: value.toString(10),
});

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
    case "Controller":
      return [
        {
          baseName: "Local rack",
          description: "Eight-slot controller rack",
          glyph: "R",
          label: "Rack",
          objectKind: "rack",
          payloadSchema: "edu.rack/1",
          semanticPayload: { slotCount: unsigned(8) },
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
          baseName: "Process data",
          description: "Named user structure",
          glyph: "UD",
          label: "Named structure",
          objectKind: "type-definition",
          payloadSchema: "edu.named-type/1",
          semanticPayload: { members: [] },
        },
        {
          baseName: "Main cycle",
          description: "Cyclic SCL organization block",
          glyph: "OB",
          label: "Organization block",
          objectKind: "program-block",
          payloadSchema: "edu.program-block/1",
          semanticPayload: {
            blockKind: "OB",
            engineeringNumber: unsigned(1),
            language: "SCL",
            obRole: "CyclicMain",
            sourceText: "",
          },
        },
        {
          baseName: "Function",
          description: "Reusable SCL function",
          glyph: "FC",
          label: "Function",
          objectKind: "program-block",
          payloadSchema: "edu.program-block/1",
          semanticPayload: {
            blockKind: "FC",
            engineeringNumber: unsigned(1),
            language: "SCL",
            sourceText: "",
          },
        },
        {
          baseName: "Function block",
          description: "State-owning SCL block",
          glyph: "FB",
          label: "Function block",
          objectKind: "program-block",
          payloadSchema: "edu.program-block/1",
          semanticPayload: {
            blockKind: "FB",
            engineeringNumber: unsigned(1),
            language: "SCL",
            sourceText: "",
          },
        },
        {
          baseName: "Global data",
          description: "Controller global data block",
          glyph: "DB",
          label: "Global data block",
          objectKind: "data-block",
          payloadSchema: "edu.data-block/1",
          semanticPayload: { dbKind: "GlobalDB", members: [] },
        },
        {
          baseName: "Instance data",
          description: "Function-block instance data",
          glyph: "ID",
          label: "Instance data block",
          objectKind: "data-block",
          payloadSchema: "edu.data-block/1",
          semanticPayload: { dbKind: "InstanceDB", instanceOf: null },
        },
        {
          baseName: "Watch table",
          description: "Persistent monitoring targets",
          glyph: "W",
          label: "Watch table",
          objectKind: "generic",
          payloadSchema: "edu.watch-table/1",
          semanticPayload: { rows: [] },
        },
        {
          baseName: "Trace",
          description: "Bounded virtual trace configuration",
          glyph: "T",
          label: "Trace configuration",
          objectKind: "generic",
          payloadSchema: "edu.trace-configuration/1",
          semanticPayload: { channels: [], state: "idle" },
        },
      ];
    case "Rack": {
      const slot = parent.children.filter((id) => snapshot.objects[id]?.kind === "Module").length + 1;
      return [
        moduleTemplate("Digital input module", "VDI16", "vdi16", slot),
        moduleTemplate("Digital output module", "VDO16", "vdo16", slot),
        moduleTemplate("Analog input module", "VAI4", "vai4", slot),
        moduleTemplate("Analog output module", "VAO4", "vao4", slot),
      ];
    }
    case "SymbolTable":
      return [
        tagTemplate("Input tag", "I", "Input"),
        tagTemplate("Output tag", "Q", "Output"),
        tagTemplate("Memory tag", "M", "Memory"),
      ];
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
    slot: unsigned(slot),
  },
});

const tagTemplate = (label: string, area: "I" | "M" | "Q", baseName: string): CreateObjectTemplate => ({
  baseName,
  description: `${area}-area BOOL with automatic allocation`,
  glyph: area,
  label,
  objectKind: "tag",
  payloadSchema: "edu.tag/1",
  semanticPayload: {
    addressArea: area,
    addressIntent: "auto",
    dataType: "BOOL",
    tagKind: area === "I" ? "Input" : area === "Q" ? "Output" : "Memory",
  },
});

const nextObjectName = (
  baseName: string,
  parentId: string,
  snapshot: WorkbenchSnapshot,
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
    const candidate = `${baseName} ${suffix}`;
    if (!siblingNames.has(candidate.toLocaleLowerCase("en-US"))) {
      return candidate;
    }
  }
  return `${baseName} ${crypto.randomUUID().slice(0, 8)}`;
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
