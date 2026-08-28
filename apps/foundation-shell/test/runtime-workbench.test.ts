import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { RuntimeInspector, RuntimeToolbar } from "../src/RuntimeWorkbench";
import type { EngineeringRuntimeView } from "../src/runtime-types";

const onOperation = async (): Promise<void> => undefined;

const readyRuntime = {
  availability: "READY",
  canBuild: true,
  diagnostics: [],
  reason: null,
  schemaVersion: 1,
  session: {
    buildCurrent: true,
    buildFingerprint: "build-fingerprint",
    controllerEpoch: "12",
    controllerObjectId: "controller-object",
    cpuState: "RUN",
    diagnosticReplayHash: "diagnostic-replay-hash",
    diagnostics: [{
      active: true,
      code: "RUN.WATCH.STALE",
      message: "A watch sample is older than the active controller epoch.",
      navigationObjectId: "watch-table",
      occurrenceId: "diagnostic-1",
      severity: "WARNING",
    }],
    documentDirty: false,
    forceCount: 1,
    forceRegistryVersion: "3",
    forces: [{
      forceId: "force-1",
      reason: "Training exercise",
      targetId: "input-1",
      value: { type: "BOOL", value: true },
    }],
    hardwareToLoaded: "MATCH",
    hashes: {
      controllerState: "controller-hash",
      diagnosticReplay: "diagnostic-replay-hash",
      runtimeReplay: "runtime-replay-hash",
      universeState: "universe-hash",
    },
    loadPreview: {
      blockerCount: 0,
      candidateFingerprint: "candidate-0123456789abcdef",
      compatibility: "Compatible",
      initializationCount: 2,
      previewFingerprint: "preview-fingerprint",
      previewId: "preview-1",
      removalCount: 0,
      requiresStop: true,
      warningCount: 0,
    },
    loaded: true,
    loadedArtifactFingerprint: "loaded-fingerprint",
    monitorState: "ACTIVE",
    online: true,
    probes: [{
      committedOutputValue: null,
      deliveredOutputValue: null,
      displayName: "StartButton",
      effectiveValue: { type: "BOOL", value: true },
      forcedValue: { type: "BOOL", value: true },
      id: "input-1",
      kind: "input",
      naturalValue: { type: "BOOL", value: false },
      quality: "FORCED",
      rawInputValue: { type: "BOOL", value: false },
      runtimeAddress: "%I0.0",
      valueType: "BOOL",
    }],
    runtimeControllerId: "runtime-controller",
    runtimeReplayHash: "runtime-replay-hash",
    scanSequence: "41",
    snapshotAvailable: true,
    softwareToLoaded: "MATCH",
    traces: [{
      captureCount: 3,
      id: "trace-1",
      name: "Startup trace",
      state: "IDLE",
    }],
    universeEpoch: "7",
    universeId: "universe-1",
    virtualTimeMilliseconds: "410",
    watches: [{
      id: "watch-table",
      name: "Commissioning values",
      rows: [{
        displayBase: "BOOL",
        latestValue: { type: "BOOL", value: true },
        quality: "FORCED",
        rowId: "watch-row-1",
        targetId: "input-1",
      }],
    }],
  },
  sourceDocumentHash: "document-hash",
  sourceSemanticFingerprint: "semantic-fingerprint",
} satisfies EngineeringRuntimeView;

describe("runtime workbench presentation", () => {
  it("renders the runnable controller state and consequential runtime markers", () => {
    const toolbar = renderToStaticMarkup(createElement(RuntimeToolbar, {
      busy: false,
      onOperation,
      runtime: readyRuntime,
    }));
    const inspector = renderToStaticMarkup(createElement(RuntimeInspector, {
      busy: false,
      onOperation,
      runtime: readyRuntime,
    }));

    expect(toolbar).toContain("Virtual controller commands");
    expect(toolbar).toContain("Commit load");
    expect(toolbar).toContain("e12 · s41");
    expect(inspector).toContain('data-state="RUN"');
    expect(inspector).toContain('data-forced="true"');
    expect(inspector).toContain("FORCED");
    expect(inspector).toContain("Approval boundary");
    expect(inspector).toContain("Runtime diagnostics");
  });

  it("keeps runtime controls closed and explains an unavailable canonical controller", () => {
    const runtime = {
      ...readyRuntime,
      availability: "UNAVAILABLE",
      canBuild: false,
      diagnostics: [{
        blocking: true,
        code: "SYS.HARDWARE.INVALID",
        message: "The controller requires one canonical rack.",
        objectId: "controller-object",
      }],
      reason: "Canonical hardware is incomplete.",
      session: null,
    } satisfies EngineeringRuntimeView;

    const toolbar = renderToStaticMarkup(createElement(RuntimeToolbar, {
      busy: false,
      onOperation,
      runtime,
    }));
    const inspector = renderToStaticMarkup(createElement(RuntimeInspector, {
      busy: false,
      onOperation,
      runtime,
    }));

    expect(toolbar).toContain("No runnable controller");
    expect(toolbar).toContain("disabled");
    expect(inspector).toContain("Runnable core unavailable");
    expect(inspector).toContain("Canonical hardware is incomplete.");
    expect(inspector).toContain("SYS.HARDWARE.INVALID");
  });
});
