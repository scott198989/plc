import { useEffect, useState } from "react";

import type { ReplayVerificationReceipt } from "./replay-types";
import { VirtualTrainer } from "./VirtualTrainer";
import type {
  EngineeringRuntimeView,
  RuntimeForceView,
  RuntimeOperation,
  RuntimeProbeView,
  RuntimeSessionView,
  RuntimeValue,
} from "./runtime-types";

type RuntimeSurfaceProps = Readonly<{
  busy: boolean;
  onNavigate?: (objectId: string) => void;
  onOperation: (operation: RuntimeOperation) => Promise<void>;
  onStartSimulation?: () => Promise<void>;
  onVerifyReplay: () => Promise<void>;
  replayReceipt: ReplayVerificationReceipt | null;
  runtime: EngineeringRuntimeView;
}>;

export const RuntimeToolbar = ({
  busy,
  onOperation,
  onStartSimulation,
  onVerifyReplay,
  replayReceipt,
  runtime,
}: RuntimeSurfaceProps): React.JSX.Element => {
  const session = runtime.session;
  const cpuState = session?.cpuState ?? "POWERED_OFF";
  const disabled = busy || session === null;
  const preview = session?.loadPreview ?? null;

  return (
    <div aria-label="Virtual controller commands" className="runtime-toolbar" role="toolbar">
      <div className="runtime-toolbar__identity">
        <span className="runtime-toolbar__safety" aria-hidden="true">◇</span>
        <span>
          <small>Internal simulator</small>
          <strong>{session === null ? "No runnable controller" : cpuStateLabel(cpuState)}</strong>
        </span>
      </div>
      <span className="runtime-toolbar__divider" aria-hidden="true" />
      {onStartSimulation !== undefined && (
        <button
          className="runtime-toolbar__guided"
          disabled={busy || session === null || !runtime.canBuild || (session.online && cpuState === "RUN")}
          onClick={() => void onStartSimulation()}
          title={!runtime.canBuild
            ? "Resolve blocking project issues before starting the virtual PLC"
            : "Build, load, go online, start monitoring, and enter RUN"}
          type="button"
        >
          {session?.online === true && cpuState === "RUN" ? "Simulation running" : "Start simulation"}
        </button>
      )}
      <button
        disabled={busy || session === null || !runtime.canBuild}
        onClick={() => void onOperation({ kind: "runtime.build" })}
        title={!runtime.canBuild ? "Resolve blocking project diagnostics before building" : "Build the selected virtual controller"}
        type="button"
      >
        Build
      </button>
      {preview === null ? (
        <button
          disabled={disabled || !session.buildCurrent}
          onClick={() => void onOperation({ kind: "runtime.preview-load", postLoadMode: "STOP" })}
          title="Prepare a deterministic Virtual Download preview"
          type="button"
        >
          Preview load
        </button>
      ) : (
        <button
          className="runtime-toolbar__commit"
          disabled={disabled || preview.blockerCount > 0}
          onClick={() => void onOperation({ kind: "runtime.commit-load" })}
          title={preview.blockerCount > 0 ? "The preview contains load blockers" : "Approve and commit this exact preview"}
          type="button"
        >
          Commit load
        </button>
      )}
      <span className="runtime-toolbar__divider" aria-hidden="true" />
      {cpuState === "POWERED_OFF" ? (
        <button disabled={disabled} onClick={() => void onOperation({ kind: "runtime.power-on" })} type="button">
          Power on
        </button>
      ) : (
        <button disabled={disabled} onClick={() => void onOperation({ kind: "runtime.power-off" })} type="button">
          Power off
        </button>
      )}
      <button
        disabled={disabled || !session.loaded || session.online}
        onClick={() => void onOperation({ kind: "runtime.go-online" })}
        type="button"
      >
        Go online
      </button>
      <button
        className="runtime-toolbar__run"
        disabled={disabled || !session.loaded || !session.online || cpuState !== "STOP"}
        onClick={() => void onOperation({ kind: "runtime.request-run" })}
        type="button"
      >
        RUN
      </button>
      <button
        disabled={disabled || cpuState !== "RUN"}
        onClick={() => void onOperation({ kind: "runtime.request-stop" })}
        type="button"
      >
        STOP
      </button>
      <button
        disabled={disabled || !session.online || cpuState !== "RUN"}
        onClick={() => void onOperation({ kind: "runtime.run-scan" })}
        title="Execute one deterministic controller scan"
        type="button"
      >
        Scan +1
      </button>
      <button
        disabled={disabled || !session.snapshotAvailable}
        onClick={() => void onVerifyReplay()}
        title="Export and execute a closed deterministic replay package from the captured aggregate snapshot"
        type="button"
      >
        Verify replay
      </button>
      <span className="runtime-toolbar__spacer" />
      {replayReceipt !== null && (
        <span
          aria-label="Replay verified"
          className="runtime-toolbar__receipt"
          data-event-count={replayReceipt.eventCount}
          data-fingerprint={replayReceipt.contentFingerprint}
          title={replayReceipt.contentFingerprint}
        >
          Replay verified · {replayReceipt.eventCount} events
        </span>
      )}
      <span
        className="runtime-toolbar__receipt"
        data-runtime-replay-hash={session?.runtimeReplayHash}
        title={session?.runtimeReplayHash ?? undefined}
      >
        {session === null ? "Awaiting configuration" : `e${session.controllerEpoch} · s${session.scanSequence}`}
      </span>
    </div>
  );
};

export const RuntimeInspector = ({
  busy,
  onNavigate,
  onOperation,
  onVerifyReplay,
  replayReceipt,
  runtime,
}: RuntimeSurfaceProps): React.JSX.Element => {
  const session = runtime.session;
  if (session === null) {
    return (
      <UnavailableRuntime
        {...(onNavigate === undefined ? {} : { onNavigate })}
        runtime={runtime}
      />
    );
  }

  return (
    <div className="runtime-inspector">
      <RuntimeSummary
        busy={busy}
        onOperation={onOperation}
        replayReceipt={replayReceipt}
        session={session}
      />
      {session.loadPreview !== null && (
        <section aria-label="Virtual Download preview" className="load-preview-card">
          <div>
            <span className="runtime-card__kicker">Approval boundary</span>
            <strong>{session.loadPreview.compatibility}</strong>
            <small>{shortHash(session.loadPreview.candidateFingerprint)}</small>
          </div>
          <dl>
            <div><dt>STOP required</dt><dd>{session.loadPreview.requiresStop ? "Yes" : "No"}</dd></div>
            <div><dt>Initialize</dt><dd>{session.loadPreview.initializationCount}</dd></div>
            <div><dt>Remove</dt><dd>{session.loadPreview.removalCount}</dd></div>
            <div><dt>Blockers</dt><dd>{session.loadPreview.blockerCount}</dd></div>
          </dl>
        </section>
      )}
      <VirtualTrainer
        busy={busy}
        onOperation={onOperation}
        runtime={runtime}
      />
      <section className="runtime-tool-section">
        <div className="runtime-tool-section__heading">
          <span><small>Live values</small><strong>Runtime probes</strong></span>
          <span>{session.probes.length} targets</span>
        </div>
        {session.probes.length === 0 ? (
          <RuntimeEmpty message="Build and load a program with bound tags to publish probes." />
        ) : (
          <div className="runtime-probe-table" role="table" aria-label="Runtime probes">
            <div className="runtime-probe-table__head" role="row">
              <span role="columnheader">Target</span>
              <span role="columnheader">Natural</span>
              <span role="columnheader">Effective</span>
              <span role="columnheader">Engineering action</span>
            </div>
            {session.probes.map((probe) => (
              <RuntimeProbeRow
                busy={busy}
                force={session.forces.find((entry) => entry.targetId === probe.id) ?? null}
                key={probe.id}
                online={session.online}
                onOperation={onOperation}
                probe={probe}
              />
            ))}
          </div>
        )}
      </section>
      <div className="runtime-tool-grid">
        <section className="runtime-tool-section">
          <div className="runtime-tool-section__heading">
            <span><small>Persistent views</small><strong>Watch tables</strong></span>
            <button
              disabled={busy || !session.online || session.monitorState === "ACTIVE"}
              onClick={() => void onOperation({ kind: "runtime.start-monitoring" })}
              type="button"
            >
              Start monitoring
            </button>
          </div>
          {session.watches.length === 0 ? (
            <RuntimeEmpty message="Add a watch table to the controller." />
          ) : session.watches.map((table) => (
            <div className="watch-card" key={table.id}>
              <strong>{table.name}</strong>
              {table.rows.map((row) => (
                <div className="watch-row" key={row.rowId}>
                  <code>{shortIdentity(row.targetId)}</code>
                  <span>{formatValue(row.latestValue)}</span>
                  <small>{row.quality ?? "—"}</small>
                </div>
              ))}
            </div>
          ))}
        </section>
        <section className="runtime-tool-section">
          <div className="runtime-tool-section__heading">
            <span><small>Deterministic capture</small><strong>Traces</strong></span>
            <span>{session.traces.length}</span>
          </div>
          {session.traces.length === 0 ? (
            <RuntimeEmpty message="Add a trace configuration to the controller." />
          ) : session.traces.map((trace) => (
            <div className="trace-row" key={trace.id}>
              <span><strong>{trace.name}</strong><small>{trace.state} · {trace.captureCount} captures</small></span>
              <button
                disabled={busy || !session.online || trace.state !== "IDLE"}
                onClick={() => void onOperation({ kind: "runtime.arm-trace", traceId: trace.id })}
                type="button"
              >Arm</button>
            </div>
          ))}
        </section>
      </div>
      <section className="runtime-tool-section">
        <div className="runtime-tool-section__heading">
          <span><small>Causal ledger</small><strong>Runtime diagnostics</strong></span>
          <span>{session.diagnostics.filter((diagnostic) => diagnostic.active).length} active</span>
        </div>
        {session.diagnostics.length === 0 ? (
          <RuntimeEmpty message="No runtime diagnostics have been published." />
        ) : session.diagnostics.map((diagnostic) => (
          <button
            aria-disabled={diagnostic.navigationObjectId === null || onNavigate === undefined}
            className="runtime-diagnostic-row"
            data-severity={diagnostic.severity}
            key={diagnostic.occurrenceId}
            onClick={() => diagnostic.navigationObjectId !== null && onNavigate?.(diagnostic.navigationObjectId)}
            type="button"
          >
            <span>{diagnostic.active ? "●" : "○"}</span>
            <code>{diagnostic.code}</code>
            <span>{diagnostic.message}</span>
          </button>
        ))}
      </section>
    </div>
  );
};

const RuntimeSummary = ({
  busy,
  onOperation,
  replayReceipt,
  session,
}: Readonly<{
  busy: boolean;
  onOperation: (operation: RuntimeOperation) => Promise<void>;
  replayReceipt: ReplayVerificationReceipt | null;
  session: RuntimeSessionView;
}>): React.JSX.Element => (
  <section className="runtime-summary">
    <div className="runtime-summary__state" data-state={session.cpuState}>
      <small>Virtual controller</small>
      <strong>{cpuStateLabel(session.cpuState)}</strong>
      <span>{session.online ? "Online session active" : "Offline engineering"}</span>
    </div>
    <dl>
      <div><dt>Controller epoch</dt><dd>{session.controllerEpoch}</dd></div>
      <div><dt>Scan sequence</dt><dd>{session.scanSequence}</dd></div>
      <div><dt>Virtual time</dt><dd>{session.virtualTimeMilliseconds} ms</dd></div>
      <div><dt>Monitor</dt><dd>{titleCase(session.monitorState)}</dd></div>
      <div><dt>Software</dt><dd>{comparisonLabel(session.softwareToLoaded)}</dd></div>
      <div><dt>Hardware</dt><dd>{comparisonLabel(session.hardwareToLoaded)}</dd></div>
    </dl>
    <div className="runtime-summary__actions">
      <button
        disabled={busy || !session.online || !session.loaded}
        onClick={() => void onOperation({ kind: "runtime.capture-snapshot" })}
        type="button"
      >Capture snapshot</button>
      <button
        disabled={busy || !session.snapshotAvailable}
        onClick={() => void onOperation({ kind: "runtime.restore-snapshot" })}
        type="button"
      >Restore snapshot</button>
    </div>
    {replayReceipt !== null && (
      <output
        aria-label="Replay verification receipt"
        className="runtime-summary__replay"
        data-boundary-count={replayReceipt.observedBoundaryCount}
        data-event-count={replayReceipt.eventCount}
        data-fingerprint={replayReceipt.contentFingerprint}
      >
        <strong>Deterministic replay verified</strong>
        <span>{replayReceipt.eventCount} events · {replayReceipt.observedBoundaryCount} boundary</span>
        <code>{shortHash(replayReceipt.contentFingerprint)}</code>
      </output>
    )}
  </section>
);

const RuntimeProbeRow = ({
  busy,
  force,
  online,
  onOperation,
  probe,
}: Readonly<{
  busy: boolean;
  force: RuntimeForceView | null;
  online: boolean;
  onOperation: (operation: RuntimeOperation) => Promise<void>;
  probe: RuntimeProbeView;
}>): React.JSX.Element => {
  const [draft, setDraft] = useState(() => editableValue(probe.effectiveValue, probe.valueType));
  useEffect(() => {
    setDraft(editableValue(probe.effectiveValue, probe.valueType));
  }, [probe.effectiveValue, probe.valueType]);
  const parsed = parseDraftValue(draft, probe.valueType);
  const canWrite = parsed !== null && !busy && online;

  return (
    <div className="runtime-probe-row" data-forced={force !== null} role="row">
      <span className="runtime-probe-row__target" role="cell">
        <strong>{probe.displayName}</strong>
        <small>{probe.kind} · {probe.runtimeAddress} · {probe.valueType}</small>
      </span>
      <span className="runtime-value-cell" role="cell">
        {formatValue(probe.naturalValue)}
        <small>{titleCase(probe.quality)}</small>
      </span>
      <span className="runtime-value-cell" role="cell">
        {formatValue(probe.effectiveValue)}
        {force !== null && <small className="forced-label">FORCED</small>}
      </span>
      <span className="runtime-probe-row__actions" role="cell">
        {probe.valueType === "BOOL" ? (
          <select aria-label={`Value for ${probe.displayName}`} disabled={busy} onChange={(event) => setDraft(event.target.value)} value={draft}>
            <option value="false">FALSE</option>
            <option value="true">TRUE</option>
          </select>
        ) : (
          <input aria-label={`Value for ${probe.displayName}`} disabled={busy} onChange={(event) => setDraft(event.target.value)} value={draft} />
        )}
        {probe.kind === "input" && (
          <button
            disabled={!canWrite}
            onClick={() => parsed !== null && void onOperation({ kind: "runtime.set-raw-input", targetId: probe.id, value: parsed })}
            type="button"
          >Set raw</button>
        )}
        <button
          disabled={!canWrite}
          onClick={() => parsed !== null && void onOperation({ kind: "runtime.modify-once", targetId: probe.id, value: parsed })}
          type="button"
        >Modify</button>
        {force === null ? (
          <button
            className="force-action"
            disabled={!canWrite}
            onClick={() => parsed !== null && void onOperation({
              forceId: crypto.randomUUID(),
              kind: "runtime.create-force",
              reason: "Workbench operator force",
              targetId: probe.id,
              value: parsed,
            })}
            type="button"
          >Force</button>
        ) : (
          <button
            className="force-action force-action--active"
            disabled={busy || !online}
            onClick={() => void onOperation({
              forceId: force.forceId,
              kind: "runtime.remove-force",
              reason: "Workbench operator removed force",
            })}
            type="button"
          >Remove force</button>
        )}
      </span>
    </div>
  );
};

const UnavailableRuntime = ({
  onNavigate,
  runtime,
}: Readonly<{
  onNavigate?: (objectId: string) => void;
  runtime: EngineeringRuntimeView;
}>): React.JSX.Element => (
  <div className="runtime-unavailable">
    <span aria-hidden="true">◇</span>
    <div>
      <small>Runnable core unavailable</small>
      <strong>{runtime.reason ?? "Configure one valid fictional controller."}</strong>
      <p>The editable project remains available. Build and runtime actions stay closed until canonical validation succeeds.</p>
      {runtime.diagnostics.slice(0, 4).map((diagnostic) => (
        <button
          aria-disabled={diagnostic.objectId === null || onNavigate === undefined}
          className="runtime-unavailable__diagnostic"
          key={`${diagnostic.code}:${diagnostic.objectId ?? "project"}`}
          onClick={() => diagnostic.objectId !== null && onNavigate?.(diagnostic.objectId)}
          type="button"
        >
          <code>{diagnostic.code}</code><span>{diagnostic.message}</span>
        </button>
      ))}
    </div>
  </div>
);

const RuntimeEmpty = ({ message }: Readonly<{ message: string }>): React.JSX.Element => (
  <div className="runtime-empty"><span aria-hidden="true">◇</span>{message}</div>
);

const parseDraftValue = (draft: string, type: RuntimeValue["type"]): RuntimeValue | null => {
  if (type === "BOOL") {
    return draft === "true" ? { type, value: true } : draft === "false" ? { type, value: false } : null;
  }
  if (!/^-?(?:0|[1-9][0-9]*)$/u.test(draft)) {
    return null;
  }
  try {
    const numeric = BigInt(draft);
    const valid = type === "I32"
      ? numeric >= -(1n << 31n) && numeric <= (1n << 31n) - 1n
      : type === "I64"
        ? numeric >= -(1n << 63n) && numeric <= (1n << 63n) - 1n
        : type === "U32"
          ? numeric >= 0n && numeric <= (1n << 32n) - 1n
          : numeric >= 0n && numeric <= (1n << 64n) - 1n;
    return valid ? { type, value: draft } : null;
  } catch {
    return null;
  }
};

const editableValue = (value: RuntimeValue | null, type: RuntimeValue["type"]): string => {
  if (value === null || value.type !== type) {
    return type === "BOOL" ? "false" : "0";
  }
  return String(value.value);
};

const formatValue = (value: RuntimeValue | null): string => {
  if (value === null) {
    return "—";
  }
  if (value.type === "BOOL") {
    return value.value ? "TRUE" : "FALSE";
  }
  return `${value.value}${value.type === "TIME_MS" ? " ms" : ""}`;
};

const cpuStateLabel = (state: RuntimeSessionView["cpuState"]): string =>
  state === "PAUSED_EDUCATIONAL" ? "Paused" : titleCase(state);

const comparisonLabel = (value: string | null): string => value === null ? "Not loaded" : titleCase(value);

const shortHash = (value: string): string => `${value.slice(0, 10)}…${value.slice(-6)}`;

const shortIdentity = (value: string): string => value.slice(0, 8);

const titleCase = (value: string): string =>
  value.toLocaleLowerCase("en-US").replaceAll("_", " ").replace(/^./u, (character) => character.toLocaleUpperCase("en-US"));
