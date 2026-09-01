export type GuidedTutorialStep =
  | "create-project"
  | "create-lab"
  | "select-stop"
  | "add-stop-nc"
  | "select-seal-in"
  | "add-seal-in"
  | "start-simulation"
  | "press-start"
  | "press-stop"
  | "complete"
  | "review";

export type GuidedTutorialDefinition = Readonly<{
  advanceOnTargetClick: boolean;
  body: string;
  target: string | null;
  tip: string | null;
  title: string;
}>;

export const guidedTutorialOrder = [
  "create-project",
  "create-lab",
  "select-stop",
  "add-stop-nc",
  "select-seal-in",
  "add-seal-in",
  "start-simulation",
  "press-start",
  "press-stop",
] as const satisfies readonly GuidedTutorialStep[];

export const guidedTutorialDefinitions: Readonly<Record<GuidedTutorialStep, GuidedTutorialDefinition>> = {
  "create-project": {
    advanceOnTargetClick: false,
    body: "Give your training project a name, then click Create. The project stays on this computer.",
    target: "create-project",
    tip: "This first project will become a complete Start / Stop motor-control lesson.",
    title: "Create your training project",
  },
  "create-lab": {
    advanceOnTargetClick: false,
    body: "Create the safe virtual controller, rack, I/O modules, three tags, and an editable MainCycle rung.",
    target: "create-lab",
    tip: "Later you can configure each hardware object yourself. This shortcut gets the first lesson moving.",
    title: "Set up the virtual PLC",
  },
  "select-stop": {
    advanceOnTargetClick: true,
    body: "Choose the first coached step. It selects Stop_PB and the exact series position on the rung.",
    target: "select-stop",
    tip: "A Stop contact belongs in series so it can interrupt every path to the motor coil.",
    title: "Prepare the Stop instruction",
  },
  "add-stop-nc": {
    advanceOnTargetClick: false,
    body: "Place a normally closed contact. In normal operation it passes power; pressing Stop opens the path.",
    target: "add-stop-nc",
    tip: "Normally closed is the familiar stop-circuit pattern: the rung is permitted until Stop is pressed.",
    title: "Add the Stop_PB contact",
  },
  "select-seal-in": {
    advanceOnTargetClick: true,
    body: "Choose the seal-in step. The coach selects Motor_Run and points to the Start_PB contact.",
    target: "select-seal-in",
    tip: "The new branch will let the motor output hold itself on after the Start button is released.",
    title: "Prepare the holding branch",
  },
  "add-seal-in": {
    advanceOnTargetClick: false,
    body: "Add Motor_Run in parallel with Start_PB. This is the seal-in, or holding, contact.",
    target: "add-seal-in",
    tip: "Either Start_PB or the Motor_Run contact can complete this parallel portion of the rung.",
    title: "Build the seal-in path",
  },
  "start-simulation": {
    advanceOnTargetClick: false,
    body: "Build, load, connect, monitor, and run the project on the internal virtual PLC.",
    target: "start-simulation",
    tip: "No physical controller or industrial network is contacted—the entire lesson stays inside the simulator.",
    title: "Start the virtual controller",
  },
  "press-start": {
    advanceOnTargetClick: false,
    body: "Press Start_PB. Watch the motor turn on and remain on after the pushbutton returns to FALSE.",
    target: "press-start",
    tip: "The Motor_Run contact closes when the coil energizes, preserving the powered path.",
    title: "Prove the seal-in action",
  },
  "press-stop": {
    advanceOnTargetClick: false,
    body: "Press Stop_PB. Its normally closed contact opens, power flow stops, and the motor turns off.",
    target: "press-stop",
    tip: "You have now proven the complete Start, hold, and Stop sequence.",
    title: "Stop the motor",
  },
  complete: {
    advanceOnTargetClick: false,
    body: "You configured a virtual PLC, built a real seal-in rung, and proved it with live inputs and outputs.",
    target: null,
    tip: null,
    title: "First ladder program complete",
  },
  review: {
    advanceOnTargetClick: false,
    body: "The motor starter uses one series safety condition and two parallel ways to keep the output path complete.",
    target: null,
    tip: null,
    title: "Motor starter tutorial review",
  },
};

export const nextGuidedTutorialStep = (step: GuidedTutorialStep): GuidedTutorialStep => {
  const index = guidedTutorialOrder.indexOf(step as (typeof guidedTutorialOrder)[number]);
  return index < 0 || index === guidedTutorialOrder.length - 1
    ? "complete"
    : guidedTutorialOrder[index + 1] ?? "complete";
};

export const guidedTutorialProgress = (step: GuidedTutorialStep): Readonly<{
  current: number;
  total: number;
}> | null => {
  const index = guidedTutorialOrder.indexOf(step as (typeof guidedTutorialOrder)[number]);
  return index < 0 ? null : { current: index + 1, total: guidedTutorialOrder.length };
};

type TutorialStorage = Readonly<{
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
}>;

export type GuidedTutorialStatus = "complete" | "dismissed";

export type GuidedTutorialResumeState = Readonly<{
  completed: boolean;
  hasLearnerMotorLab: boolean;
  hasSealInBranch: boolean;
  hasStopContact: boolean;
  motorOutput: boolean | null;
  simulationRunning: boolean;
}>;

export const guidedTutorialResumeStep = (
  state: GuidedTutorialResumeState,
): GuidedTutorialStep => {
  if (state.completed) {
    return "review";
  }
  if (!state.hasLearnerMotorLab) {
    return "create-lab";
  }
  if (!state.hasStopContact) {
    return "select-stop";
  }
  if (!state.hasSealInBranch) {
    return "select-seal-in";
  }
  if (!state.simulationRunning) {
    return "start-simulation";
  }
  return state.motorOutput === true ? "press-stop" : "press-start";
};

export const guidedTutorialExitStatus = (
  step: GuidedTutorialStep,
): GuidedTutorialStatus => step === "complete" || step === "review" ? "complete" : "dismissed";

const tutorialStorageKey = "plc-engineering-simulator.first-ladder-tutorial";

export const readGuidedTutorialStatus = (
  storage: TutorialStorage = window.localStorage,
): GuidedTutorialStatus | null => {
  try {
    const value = storage.getItem(tutorialStorageKey);
    return value === "complete" || value === "dismissed" ? value : null;
  } catch {
    return null;
  }
};

export const writeGuidedTutorialStatus = (
  status: GuidedTutorialStatus,
  storage: TutorialStorage = window.localStorage,
): void => {
  try {
    storage.setItem(tutorialStorageKey, status);
  } catch {
    // The tutorial remains usable for this session when browser storage is restricted.
  }
};
