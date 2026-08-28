import { useCallback, useEffect, useState } from "react";

import { EngineeringClient } from "./engineering-client";
import { EngineeringWorkbench } from "./EngineeringWorkbench";
import { FileAccessBroker, FileAccessError } from "./file-access-broker";
import { ProjectHome } from "./ProjectHome";
import type { RuntimeOperation } from "./runtime-types";
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
  const [closeRequested, setCloseRequested] = useState(false);

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
    }
  }, [runBusy, services]);

  const openProject = useCallback(async (): Promise<void> => {
    await runBusy(async () => {
      const opened = await services.files.requestOpen();
      try {
        const next = await services.client.openProject(opened.bytes, opened.grantId);
        setSnapshot(next);
      } catch (reason) {
        services.files.revoke(opened.grantId);
        throw reason;
      }
    });
  }, [runBusy, services]);

  const executeOperation = useCallback(async (operation: WorkbenchOperation): Promise<void> => {
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
    const next = await runBusy(() => services.client.executeRuntime(operation));
    if (next !== null) {
      setSnapshot(next);
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
  }, [snapshot]);

  const discardAndClose = useCallback((): void => {
    setCloseRequested(false);
    setSnapshot(null);
    setError(null);
  }, []);

  const saveAndClose = useCallback(async (): Promise<void> => {
    if (await saveProject("save")) {
      discardAndClose();
    }
  }, [discardAndClose, saveProject]);

  if (snapshot === null) {
    return (
      <ProjectHome
        busy={busy}
        coreLabel={coreLabel}
        error={error}
        fileAccessAvailable={services.files.canOpen() && services.files.canSave()}
        onCreate={createProject}
        onOpen={openProject}
      />
    );
  }

  return (
    <>
      <EngineeringWorkbench
        busy={busy}
        error={error}
        onClose={closeProject}
        onOperation={executeOperation}
        onRuntimeOperation={executeRuntimeOperation}
        onSave={async (mode) => { await saveProject(mode); }}
        snapshot={snapshot}
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
