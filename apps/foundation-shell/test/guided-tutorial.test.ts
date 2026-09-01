import { describe, expect, it } from "vitest";

import {
  guidedTutorialDefinitions,
  guidedTutorialExitStatus,
  guidedTutorialOrder,
  guidedTutorialProgress,
  guidedTutorialResumeStep,
  nextGuidedTutorialStep,
  readGuidedTutorialStatus,
  writeGuidedTutorialStatus,
} from "../src/guided-tutorial";
import type { GuidedTutorialStep } from "../src/guided-tutorial";

describe("first ladder guided tutorial", () => {
  it("defines one stable highlighted target for every hands-on step", () => {
    expect(guidedTutorialOrder).toHaveLength(9);
    expect(guidedTutorialOrder.map((step) => guidedTutorialDefinitions[step].target)).toEqual([
      "create-project",
      "create-lab",
      "select-stop",
      "add-stop-nc",
      "select-seal-in",
      "add-seal-in",
      "start-simulation",
      "press-start",
      "press-stop",
    ]);
  });

  it("advances through the closed lesson sequence and then completes", () => {
    let step: GuidedTutorialStep = guidedTutorialOrder[0];
    for (const expected of guidedTutorialOrder.slice(1)) {
      step = nextGuidedTutorialStep(step);
      expect(step).toBe(expected);
    }
    expect(nextGuidedTutorialStep(step)).toBe("complete");
    expect(guidedTutorialProgress("press-stop")).toEqual({ current: 9, total: 9 });
    expect(guidedTutorialProgress("review")).toBeNull();
  });

  it("persists only validated completion or dismissal tokens", () => {
    const values = new Map<string, string>();
    const storage = {
      getItem: (key: string): string | null => values.get(key) ?? null,
      setItem: (key: string, value: string): void => { values.set(key, value); },
    };

    expect(readGuidedTutorialStatus(storage)).toBeNull();
    writeGuidedTutorialStatus("dismissed", storage);
    expect(readGuidedTutorialStatus(storage)).toBe("dismissed");
    writeGuidedTutorialStatus("complete", storage);
    expect(readGuidedTutorialStatus(storage)).toBe("complete");
    values.clear();
    values.set("plc-engineering-simulator.first-ladder-tutorial", "unexpected");
    expect(readGuidedTutorialStatus(storage)).toBeNull();
  });

  it("keeps completed lessons complete when their finish or review dialog is dismissed", () => {
    expect(guidedTutorialExitStatus("press-stop")).toBe("dismissed");
    expect(guidedTutorialExitStatus("complete")).toBe("complete");
    expect(guidedTutorialExitStatus("review")).toBe("complete");
  });

  it("resumes an unfinished lesson from canonical and runtime progress", () => {
    const baseline = {
      completed: false,
      hasLearnerMotorLab: true,
      hasSealInBranch: false,
      hasStopContact: false,
      motorOutput: null,
      simulationRunning: false,
    } as const;
    expect(guidedTutorialResumeStep({ ...baseline, hasLearnerMotorLab: false })).toBe("create-lab");
    expect(guidedTutorialResumeStep({ ...baseline, completed: true, hasLearnerMotorLab: false })).toBe("review");
    expect(guidedTutorialResumeStep(baseline)).toBe("select-stop");
    expect(guidedTutorialResumeStep({ ...baseline, hasStopContact: true })).toBe("select-seal-in");
    expect(guidedTutorialResumeStep({ ...baseline, hasStopContact: true, hasSealInBranch: true })).toBe("start-simulation");
    expect(guidedTutorialResumeStep({
      ...baseline,
      hasStopContact: true,
      hasSealInBranch: true,
      simulationRunning: true,
      motorOutput: false,
    })).toBe("press-start");
    expect(guidedTutorialResumeStep({
      ...baseline,
      hasStopContact: true,
      hasSealInBranch: true,
      simulationRunning: true,
      motorOutput: true,
    })).toBe("press-stop");
    expect(guidedTutorialResumeStep({ ...baseline, completed: true })).toBe("review");
  });
});
