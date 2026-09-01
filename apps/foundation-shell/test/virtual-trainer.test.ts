import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import {
  createMomentaryPulseOperationSequence,
  VirtualTrainer,
  VirtualTrainerTutorialProvider,
} from "../src/VirtualTrainer";
import type { EngineeringRuntimeView, RuntimeProbeView } from "../src/runtime-types";

const booleanProbe = (
  id: string,
  kind: "input" | "output",
  value: boolean,
): RuntimeProbeView => ({
  committedOutputValue: kind === "output" ? { type: "BOOL", value } : null,
  deliveredOutputValue: kind === "output" ? { type: "BOOL", value } : null,
  displayName: id === "start" ? "Start button" : "Conveyor motor",
  effectiveValue: { type: "BOOL", value },
  forcedValue: null,
  id,
  kind,
  naturalValue: { type: "BOOL", value },
  quality: "GOOD",
  rawInputValue: kind === "input" ? { type: "BOOL", value } : null,
  runtimeAddress: kind === "input" ? "%I0.0" : "%Q0.0",
  valueType: "BOOL",
});

const runtime = {
  availability: "READY",
  canBuild: true,
  diagnostics: [],
  reason: null,
  schemaVersion: 1,
  session: {
    buildCurrent: true,
    buildFingerprint: "build",
    controllerEpoch: "1",
    controllerObjectId: "controller",
    cpuState: "RUN",
    diagnosticReplayHash: "diagnostics",
    diagnostics: [],
    documentDirty: false,
    forceCount: 0,
    forceRegistryVersion: "0",
    forces: [],
    hardwareToLoaded: "MATCH",
    hashes: null,
    loadPreview: null,
    loaded: true,
    loadedArtifactFingerprint: "loaded",
    monitorState: "ACTIVE",
    online: true,
    probes: [booleanProbe("start", "input", false), booleanProbe("motor", "output", true)],
    runtimeControllerId: "runtime-controller",
    runtimeReplayHash: "runtime-replay",
    scanSequence: "4",
    snapshotAvailable: false,
    softwareToLoaded: "MATCH",
    traces: [],
    universeEpoch: "1",
    universeId: "universe",
    virtualTimeMilliseconds: "40",
    watches: [],
  },
  sourceDocumentHash: "document",
  sourceSemanticFingerprint: "semantic",
} satisfies EngineeringRuntimeView;

describe("Virtual Trainer", () => {
  it("creates an authoritative press-scan-release-scan pulse", () => {
    expect(createMomentaryPulseOperationSequence("start")).toEqual([
      {
        kind: "runtime.set-raw-input",
        targetId: "start",
        value: { type: "BOOL", value: true },
      },
      { kind: "runtime.run-scan" },
      {
        kind: "runtime.set-raw-input",
        targetId: "start",
        value: { type: "BOOL", value: false },
      },
      { kind: "runtime.run-scan" },
    ]);
  });

  it("renders learner-facing input controls and output indicators", () => {
    const markup = renderToStaticMarkup(createElement(VirtualTrainer, {
      busy: false,
      inputControls: { start: "momentary" },
      onOperation: async () => undefined,
      outputDevices: { motor: "actuator" },
      runtime,
    }));

    expect(markup).toContain('aria-label="Virtual Trainer"');
    expect(markup).toContain("Learn by operating");
    expect(markup).toContain('aria-label="Start button input behavior"');
    expect(markup).toContain("Switch");
    expect(markup).toContain("Pushbutton");
    expect(markup).toContain('aria-label="Pulse Start button"');
    expect(markup).toContain('aria-label="Conveyor motor actuator is on"');
    expect(markup).toContain("Controller reads <strong>FALSE</strong>");
    expect(markup).toContain('data-active="true"');
  });

  it("renders a maintained input as an accessible switch", () => {
    const maintainedRuntime = {
      ...runtime,
      session: runtime.session === null ? null : {
        ...runtime.session,
        probes: [booleanProbe("start", "input", true)],
      },
    } satisfies EngineeringRuntimeView;
    const markup = renderToStaticMarkup(createElement(VirtualTrainer, {
      busy: false,
      inputControls: { start: "maintained" },
      onOperation: async () => undefined,
      runtime: maintainedRuntime,
    }));

    expect(markup).toContain('role="switch"');
    expect(markup).toContain('aria-checked="true"');
    expect(markup).toContain("ON");
  });

  it("temporarily presents the active tutorial input as a momentary pushbutton", () => {
    const tutorialRuntime = {
      ...runtime,
      session: runtime.session === null ? null : {
        ...runtime.session,
        probes: [{
          ...booleanProbe("start", "input", false),
          displayName: "Start_PB",
        }],
      },
    } satisfies EngineeringRuntimeView;
    const markup = renderToStaticMarkup(
      createElement(
        VirtualTrainerTutorialProvider,
        { target: "press-start" },
        createElement(VirtualTrainer, {
          busy: false,
          inputControls: { start: "maintained" },
          onOperation: async () => undefined,
          runtime: tutorialRuntime,
        }),
      ),
    );

    expect(markup).toContain('aria-label="Pulse Start_PB"');
    expect(markup).toContain('data-tutorial-target="press-start"');
    expect(markup).not.toContain('role="switch"');
  });

  it("uses learner-friendly device defaults for starter tag names", () => {
    const markup = renderToStaticMarkup(createElement(VirtualTrainer, {
      busy: false,
      onOperation: async () => undefined,
      runtime,
    }));

    expect(markup).toContain('aria-label="Pulse Start button"');
    expect(markup).toContain('aria-label="Conveyor motor actuator is on"');
  });
});
