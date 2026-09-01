import { createContext, useContext, useState } from "react";
import type { ReactNode } from "react";

import type {
  EngineeringRuntimeView,
  RuntimeOperation,
  RuntimeProbeView,
} from "./runtime-types";
import { createMomentaryPulseOperationSequence } from "./VirtualTrainerOperations";

export { createMomentaryPulseOperationSequence } from "./VirtualTrainerOperations";

export type BooleanInputControl = "maintained" | "momentary";

export type BooleanOutputDevice = "actuator" | "lamp";

type VirtualTrainerTutorialTarget = "press-start" | "press-stop";

const VirtualTrainerTutorialContext = createContext<VirtualTrainerTutorialTarget | null>(null);

export const VirtualTrainerTutorialProvider = ({
  children,
  target,
}: Readonly<{
  children?: ReactNode;
  target: VirtualTrainerTutorialTarget | null;
}>): React.JSX.Element => (
  <VirtualTrainerTutorialContext.Provider value={target}>
    {children}
  </VirtualTrainerTutorialContext.Provider>
);

export type VirtualTrainerProps = Readonly<{
  busy: boolean;
  inputControls?: Readonly<Record<string, BooleanInputControl>>;
  onOperation: (operation: RuntimeOperation) => Promise<void>;
  outputDevices?: Readonly<Record<string, BooleanOutputDevice>>;
  runtime: EngineeringRuntimeView;
}>;

const createMaintainedInputOperationSequence = (
  targetId: RuntimeProbeView["id"],
  value: boolean,
): readonly RuntimeOperation[] => [
  { kind: "runtime.set-raw-input", targetId, value: { type: "BOOL", value } },
  { kind: "runtime.run-scan" },
];

export const VirtualTrainer = ({
  busy,
  inputControls = {},
  onOperation,
  outputDevices = {},
  runtime,
}: VirtualTrainerProps): React.JSX.Element => {
  const [activeControlId, setActiveControlId] = useState<string | null>(null);
  const [controlModes, setControlModes] = useState<Record<string, BooleanInputControl>>(
    () => ({ ...inputControls }),
  );
  const tutorialTarget = useContext(VirtualTrainerTutorialContext);
  const session = runtime.session;
  const inputs = session?.probes.filter(isBooleanInput) ?? [];
  const outputs = session?.probes.filter(isBooleanOutput) ?? [];
  const trainerReady = session !== null && session.online && session.cpuState === "RUN";
  const controlsDisabled = busy || activeControlId !== null || !trainerReady;

  const execute = async (
    targetId: string,
    operations: readonly RuntimeOperation[],
  ): Promise<void> => {
    setActiveControlId(targetId);
    try {
      for (const operation of operations) {
        await onOperation(operation);
      }
    } finally {
      setActiveControlId(null);
    }
  };

  return (
    <section aria-label="Virtual Trainer" className="virtual-trainer">
      <header className="virtual-trainer__header">
        <div>
          <span className="virtual-trainer__eyebrow">Learn by operating</span>
          <h2>Virtual Trainer</h2>
          <p>Use the controls like a small training panel and watch the virtual controller respond.</p>
        </div>
        <span
          aria-label={trainerReady ? "Trainer ready" : "Trainer waiting for controller RUN mode"}
          className="virtual-trainer__state"
          data-ready={trainerReady}
        >
          <span aria-hidden="true" />
          {trainerReady ? "Ready" : "Start controller"}
        </span>
      </header>

      {session === null ? (
        <p className="virtual-trainer__notice">Build and load a valid controller program to open the trainer.</p>
      ) : (
        <div className="virtual-trainer__panels">
          <section aria-labelledby="virtual-trainer-inputs" className="virtual-trainer__panel">
            <div className="virtual-trainer__panel-heading">
              <div>
                <span>Control panel</span>
                <h3 id="virtual-trainer-inputs">Inputs</h3>
              </div>
              <small>{inputs.length} connected</small>
            </div>
            {inputs.length === 0 ? (
              <p className="virtual-trainer__empty">Add Boolean input tags to create switches and buttons.</p>
            ) : (
              <div className="virtual-trainer__device-grid">
                {inputs.map((probe) => {
                  const inputTutorialTarget = tutorialTargetForInput(probe.displayName);
                  const configuredMode = controlModes[probe.id] ?? inputControls[probe.id] ?? inferredInputControl(probe);
                  const mode = tutorialTarget === inputTutorialTarget ? "momentary" : configuredMode;
                  const rawValue = booleanValue(probe.rawInputValue);
                  const effectiveValue = booleanValue(probe.effectiveValue);
                  const pending = activeControlId === probe.id;

                  return (
                    <article className="virtual-trainer__device" key={probe.id}>
                      <div className="virtual-trainer__device-label">
                        <strong>{probe.displayName}</strong>
                        <code>{probe.runtimeAddress}</code>
                      </div>
                      <div
                        aria-label={`${probe.displayName} input behavior`}
                        className="virtual-trainer__mode-picker"
                        role="group"
                      >
                        <button
                          aria-pressed={mode === "maintained"}
                          onClick={() => setControlModes((current) => ({
                            ...current,
                            [probe.id]: "maintained",
                          }))}
                          type="button"
                        >
                          Switch
                        </button>
                        <button
                          aria-pressed={mode === "momentary"}
                          onClick={() => setControlModes((current) => ({
                            ...current,
                            [probe.id]: "momentary",
                          }))}
                          type="button"
                        >
                          Pushbutton
                        </button>
                      </div>
                      {mode === "momentary" ? (
                        <button
                          aria-label={`Pulse ${probe.displayName}`}
                          className="virtual-trainer__pushbutton"
                          data-active={pending}
                          data-tutorial-target={inputTutorialTarget}
                          disabled={controlsDisabled}
                          onClick={() => void execute(
                            probe.id,
                            createMomentaryPulseOperationSequence(probe.id),
                          )}
                          type="button"
                        >
                          <span aria-hidden="true" />
                          {pending ? "Pulsing…" : "Press"}
                        </button>
                      ) : (
                        <button
                          aria-checked={rawValue}
                          aria-label={`${probe.displayName} maintained switch`}
                          className="virtual-trainer__switch"
                          disabled={controlsDisabled}
                          onClick={() => void execute(
                            probe.id,
                            createMaintainedInputOperationSequence(probe.id, !rawValue),
                          )}
                          role="switch"
                          type="button"
                        >
                          <span aria-hidden="true"><i /></span>
                          {rawValue ? "ON" : "OFF"}
                        </button>
                      )}
                      <small className="virtual-trainer__feedback">
                        Controller reads <strong>{effectiveValue ? "TRUE" : "FALSE"}</strong>
                      </small>
                    </article>
                  );
                })}
              </div>
            )}
          </section>

          <section aria-labelledby="virtual-trainer-outputs" className="virtual-trainer__panel">
            <div className="virtual-trainer__panel-heading">
              <div>
                <span>Machine response</span>
                <h3 id="virtual-trainer-outputs">Outputs</h3>
              </div>
              <small>{outputs.length} connected</small>
            </div>
            {outputs.length === 0 ? (
              <p className="virtual-trainer__empty">Add Boolean output tags to create lamps and actuators.</p>
            ) : (
              <div className="virtual-trainer__device-grid">
                {outputs.map((probe) => {
                  const device = outputDevices[probe.id] ?? inferredOutputDevice(probe);
                  const value = booleanValue(
                    probe.deliveredOutputValue ?? probe.committedOutputValue ?? probe.effectiveValue,
                  );

                  return (
                    <article className="virtual-trainer__device" key={probe.id}>
                      <div className="virtual-trainer__device-label">
                        <strong>{probe.displayName}</strong>
                        <code>{probe.runtimeAddress}</code>
                      </div>
                      <output
                        aria-label={`${probe.displayName} ${device} is ${value ? "on" : "off"}`}
                        className={`virtual-trainer__indicator virtual-trainer__indicator--${device}`}
                        data-active={value}
                      >
                        <span aria-hidden="true" />
                        <strong>{value ? "ON" : "OFF"}</strong>
                        <small>{device}</small>
                      </output>
                    </article>
                  );
                })}
              </div>
            )}
          </section>
        </div>
      )}

      {!trainerReady && session !== null && (
        <p className="virtual-trainer__notice" role="status">
          Load the program, go online, and put the virtual controller in RUN to use the controls.
        </p>
      )}
    </section>
  );
};

const isBooleanInput = (probe: RuntimeProbeView): boolean =>
  probe.kind === "input" && probe.valueType === "BOOL";

const isBooleanOutput = (probe: RuntimeProbeView): boolean =>
  probe.kind === "output" && probe.valueType === "BOOL";

const booleanValue = (value: RuntimeProbeView["effectiveValue"]): boolean =>
  value?.type === "BOOL" && value.value === true;

const inferredInputControl = (probe: RuntimeProbeView): BooleanInputControl => {
  const name = probe.displayName.toLocaleLowerCase("en-US");
  return /(?:^|[_\s-])(?:pb|button)(?:$|[_\s-])/u.test(name) ||
    /(?:^|[_\s-])(?:start|stop|reset)(?:$|[_\s-])/u.test(name)
    ? "momentary"
    : "maintained";
};

const inferredOutputDevice = (probe: RuntimeProbeView): BooleanOutputDevice => {
  const name = probe.displayName.toLocaleLowerCase("en-US");
  return /(?:motor|pump|fan|conveyor|valve|actuator)/u.test(name) ? "actuator" : "lamp";
};

const tutorialTargetForInput = (displayName: string): "press-start" | "press-stop" | undefined => {
  const normalized = displayName
    .trim()
    .toLocaleLowerCase("en-US")
    .replaceAll(/[^a-z0-9]+/gu, "_")
    .replaceAll(/^_+|_+$/gu, "");
  return normalized === "start_pb"
    ? "press-start"
    : normalized === "stop_pb"
      ? "press-stop"
      : undefined;
};
