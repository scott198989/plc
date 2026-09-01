import {
  canonicalRecordFields,
  recordValue,
  unsignedValue,
} from "./canonical-authoring";
import type {
  ProjectPayload,
  ProjectPayloadValue,
} from "./workbench-types";

/**
 * The first educational HMI contract is intentionally small. It is a virtual,
 * offline companion to the simulated PLC; it is not a protocol or physical
 * device abstraction.
 */
export const HMI_SCREEN_PAYLOAD_SCHEMA = "edu.hmi-screen/1";
export const HMI_SCREEN_SCHEMA_VERSION = 1 as const;

export type HmiScreenMode = "edit" | "run";
export type HmiElementKind = "lamp" | "momentary-button";

export type HmiElementFrame = Readonly<{
  height: number;
  width: number;
  x: number;
  y: number;
}>;

type HmiElementBase = Readonly<{
  frame: HmiElementFrame;
  id: string;
  label: string;
  /** A stable project tag ID. Null is allowed while a learner is editing. */
  tagId: string | null;
}>;

export type HmiMomentaryButtonElement = HmiElementBase & Readonly<{
  kind: "momentary-button";
}>;

export type HmiLampElement = HmiElementBase & Readonly<{
  kind: "lamp";
}>;

export type HmiScreenElement = HmiMomentaryButtonElement | HmiLampElement;

export type HmiScreen = Readonly<{
  elements: readonly HmiScreenElement[];
  height: number;
  name: string;
  runtimeTarget: "virtual";
  schemaVersion: typeof HMI_SCREEN_SCHEMA_VERSION;
  width: number;
}>;

export type HmiBindableTag = Readonly<{
  addressArea: "I" | "M" | "Q";
  dataType: string;
  id: string;
  name: string;
}>;

export type HmiValidationIssueCode =
  | "HMI_BINDING_REQUIRED"
  | "HMI_DUPLICATE_ELEMENT_ID"
  | "HMI_ELEMENT_FRAME_INVALID"
  | "HMI_LAMP_BINDING_INCOMPATIBLE"
  | "HMI_MOMENTARY_BINDING_INCOMPATIBLE"
  | "HMI_NAME_REQUIRED"
  | "HMI_RUNTIME_TARGET_UNSAFE"
  | "HMI_SCHEMA_UNSUPPORTED"
  | "HMI_TAG_NOT_FOUND"
  | "HMI_TAG_TYPE_INCOMPATIBLE";

export type HmiValidationIssue = Readonly<{
  code: HmiValidationIssueCode;
  elementId: string | null;
  message: string;
}>;

export type HmiValidationResult = Readonly<{
  issues: readonly HmiValidationIssue[];
  valid: boolean;
}>;

export type HmiSessionState = Readonly<{
  mode: HmiScreenMode;
  pressedButtonIds: readonly string[];
}>;

export type HmiVirtualTagWrite = Readonly<{
  tagId: string;
  value: boolean;
}>;

export type HmiPayloadDecodeResult =
  | Readonly<{ ok: true; screen: HmiScreen }>
  | Readonly<{ message: string; ok: false }>;

const DEFAULT_BUTTON_FRAME: HmiElementFrame = { height: 64, width: 144, x: 32, y: 32 };
const DEFAULT_LAMP_FRAME: HmiElementFrame = { height: 72, width: 104, x: 208, y: 28 };

export const createHmiScreen = (name = "Main HMI"): HmiScreen => ({
  elements: [],
  height: 540,
  name,
  runtimeTarget: "virtual",
  schemaVersion: HMI_SCREEN_SCHEMA_VERSION,
  width: 960,
});

export const createHmiMomentaryButton = (options: Readonly<{
  frame?: HmiElementFrame;
  id: string;
  label?: string;
  tagId?: string | null;
}>): HmiMomentaryButtonElement => ({
  frame: options.frame ?? DEFAULT_BUTTON_FRAME,
  id: options.id,
  kind: "momentary-button",
  label: options.label ?? "Push button",
  tagId: options.tagId ?? null,
});

export const createHmiLamp = (options: Readonly<{
  frame?: HmiElementFrame;
  id: string;
  label?: string;
  tagId?: string | null;
}>): HmiLampElement => ({
  frame: options.frame ?? DEFAULT_LAMP_FRAME,
  id: options.id,
  kind: "lamp",
  label: options.label ?? "Lamp",
  tagId: options.tagId ?? null,
});

export const appendHmiElement = (
  screen: HmiScreen,
  element: HmiScreenElement,
): HmiScreen => ({ ...screen, elements: [...screen.elements, element] });

export const replaceHmiElement = (
  screen: HmiScreen,
  element: HmiScreenElement,
): HmiScreen => ({
  ...screen,
  elements: screen.elements.map((candidate) => candidate.id === element.id ? element : candidate),
});

export const removeHmiElement = (
  screen: HmiScreen,
  elementId: string,
): HmiScreen => ({
  ...screen,
  elements: screen.elements.filter((candidate) => candidate.id !== elementId),
});

/**
 * Validates a screen against the current project tag catalog. Momentary
 * controls deliberately target virtual input tags, because raw virtual inputs
 * are the only runtime values an operator control may safely write. Lamps can
 * observe M or Q tags. Physical bindings do not exist.
 */
export const validateHmiScreen = (
  screen: HmiScreen,
  tags: readonly HmiBindableTag[],
): HmiValidationResult => {
  const issues: HmiValidationIssue[] = [];
  if (screen.schemaVersion !== HMI_SCREEN_SCHEMA_VERSION) {
    issues.push(issue("HMI_SCHEMA_UNSUPPORTED", null, "This HMI screen version is not supported."));
  }
  if (screen.runtimeTarget !== "virtual") {
    issues.push(issue(
      "HMI_RUNTIME_TARGET_UNSAFE",
      null,
      "Educational HMI screens can run only against the virtual PLC.",
    ));
  }
  if (screen.name.trim().length === 0) {
    issues.push(issue("HMI_NAME_REQUIRED", null, "Give the HMI screen a name."));
  }

  const tagsById = new Map(tags.map((tag) => [tag.id, tag]));
  const elementIds = new Set<string>();
  for (const element of screen.elements) {
    if (elementIds.has(element.id)) {
      issues.push(issue(
        "HMI_DUPLICATE_ELEMENT_ID",
        element.id,
        "Every HMI element needs a unique identity.",
      ));
    }
    elementIds.add(element.id);
    if (!validFrame(element.frame)) {
      issues.push(issue(
        "HMI_ELEMENT_FRAME_INVALID",
        element.id,
        "The HMI element must have a non-negative position and a positive whole-number size.",
      ));
    }
    if (element.tagId === null || element.tagId.length === 0) {
      issues.push(issue(
        "HMI_BINDING_REQUIRED",
        element.id,
        `Bind ${element.label || "this element"} to a PLC tag.`,
      ));
      continue;
    }
    const tag = tagsById.get(element.tagId);
    if (tag === undefined) {
      issues.push(issue(
        "HMI_TAG_NOT_FOUND",
        element.id,
        "The bound PLC tag no longer exists. Choose another tag.",
      ));
      continue;
    }
    if (tag.dataType.toLocaleUpperCase("en-US") !== "BOOL") {
      issues.push(issue(
        "HMI_TAG_TYPE_INCOMPATIBLE",
        element.id,
        `${element.label || "This element"} needs a BOOL tag; ${tag.name} is ${tag.dataType}.`,
      ));
      continue;
    }
    if (element.kind === "momentary-button" && tag.addressArea !== "I") {
      issues.push(issue(
        "HMI_MOMENTARY_BINDING_INCOMPATIBLE",
        element.id,
        "A momentary HMI button can operate only a virtual input (I) BOOL tag.",
      ));
    }
    if (element.kind === "lamp" && tag.addressArea !== "M" && tag.addressArea !== "Q") {
      issues.push(issue(
        "HMI_LAMP_BINDING_INCOMPATIBLE",
        element.id,
        "An HMI lamp can observe only a virtual memory (M) or output (Q) BOOL tag.",
      ));
    }
  }
  return { issues, valid: issues.length === 0 };
};

export const createHmiSession = (mode: HmiScreenMode = "edit"): HmiSessionState => ({
  mode,
  pressedButtonIds: [],
});

export const setHmiSessionMode = (
  session: HmiSessionState,
  mode: HmiScreenMode,
): HmiSessionState => mode === session.mode
  ? session
  : { mode, pressedButtonIds: [] };

/**
 * Tracks pointer/key press state without performing I/O. Edit mode ignores
 * control operation so arranging the screen cannot mutate the simulated PLC.
 */
export const setHmiMomentaryPressed = (
  screen: HmiScreen,
  session: HmiSessionState,
  elementId: string,
  pressed: boolean,
): HmiSessionState => {
  if (
    session.mode !== "run" ||
    !screen.elements.some((element) => element.id === elementId && element.kind === "momentary-button")
  ) {
    return session;
  }
  const current = new Set(session.pressedButtonIds);
  if (pressed) {
    current.add(elementId);
  } else {
    current.delete(elementId);
  }
  return { ...session, pressedButtonIds: [...current].sort() };
};

/**
 * Produces values for an in-process virtual tag bus. Callers must validate the
 * screen first. This API contains no transport, URL, socket, or PLC endpoint.
 */
export const projectHmiMomentaryWrites = (
  screen: HmiScreen,
  session: HmiSessionState,
): readonly HmiVirtualTagWrite[] => {
  const pressed = new Set(session.pressedButtonIds);
  return screen.elements.flatMap((element) =>
    element.kind === "momentary-button" && element.tagId !== null
      ? [{ tagId: element.tagId, value: session.mode === "run" && pressed.has(element.id) }]
      : []
  );
};

/** Canonical persistence shape for a project object's semantic payload. */
export const encodeHmiScreenPayload = (screen: HmiScreen): ProjectPayload => ({
  elements: screen.elements.map((element) => recordValue({
    height: unsignedValue(element.frame.height),
    id: element.id,
    kind: element.kind,
    label: element.label,
    tagId: element.tagId,
    width: unsignedValue(element.frame.width),
    x: unsignedValue(element.frame.x),
    y: unsignedValue(element.frame.y),
  })),
  height: unsignedValue(screen.height),
  name: screen.name,
  runtimeTarget: screen.runtimeTarget,
  schemaVersion: unsignedValue(screen.schemaVersion),
  width: unsignedValue(screen.width),
});

/** Reads persisted HMI data without silently repairing malformed project data. */
export const decodeHmiScreenPayload = (payload: ProjectPayload): HmiPayloadDecodeResult => {
  const schemaVersion = readUnsigned(payload.schemaVersion);
  if (schemaVersion !== HMI_SCREEN_SCHEMA_VERSION) {
    return { message: "Unsupported or missing HMI screen schema version.", ok: false };
  }
  if (payload.runtimeTarget !== "virtual") {
    return { message: "HMI runtime target must be virtual.", ok: false };
  }
  const name = readText(payload.name);
  const width = readPositiveUnsigned(payload.width);
  const height = readPositiveUnsigned(payload.height);
  if (name === null || width === null || height === null || !Array.isArray(payload.elements)) {
    return { message: "Malformed HMI screen payload.", ok: false };
  }
  const elements: HmiScreenElement[] = [];
  for (const [index, value] of payload.elements.entries()) {
    const fields = canonicalRecordFields(value);
    if (fields === null) {
      return { message: `Malformed HMI element at index ${index}.`, ok: false };
    }
    const id = readText(fields.id);
    const label = readText(fields.label);
    const tagId = fields.tagId === null ? null : readText(fields.tagId);
    const x = readUnsigned(fields.x);
    const y = readUnsigned(fields.y);
    const elementWidth = readPositiveUnsigned(fields.width);
    const elementHeight = readPositiveUnsigned(fields.height);
    if (
      id === null || label === null || tagId === undefined || x === null || y === null ||
      elementWidth === null || elementHeight === null ||
      (fields.kind !== "momentary-button" && fields.kind !== "lamp")
    ) {
      return { message: `Malformed HMI element at index ${index}.`, ok: false };
    }
    const common = {
      frame: { height: elementHeight, width: elementWidth, x, y },
      id,
      label,
      tagId,
    };
    elements.push(fields.kind === "momentary-button"
      ? { ...common, kind: "momentary-button" }
      : { ...common, kind: "lamp" });
  }
  return {
    ok: true,
    screen: {
      elements,
      height,
      name,
      runtimeTarget: "virtual",
      schemaVersion: HMI_SCREEN_SCHEMA_VERSION,
      width,
    },
  };
};

const issue = (
  code: HmiValidationIssueCode,
  elementId: string | null,
  message: string,
): HmiValidationIssue => ({ code, elementId, message });

const validFrame = (frame: HmiElementFrame): boolean =>
  Number.isSafeInteger(frame.x) && frame.x >= 0 &&
  Number.isSafeInteger(frame.y) && frame.y >= 0 &&
  Number.isSafeInteger(frame.width) && frame.width > 0 &&
  Number.isSafeInteger(frame.height) && frame.height > 0;

const readText = (value: ProjectPayloadValue | undefined): string | null =>
  typeof value === "string" && value.length > 0 ? value : null;

const readUnsigned = (value: ProjectPayloadValue | undefined): number | null => {
  if (
    typeof value !== "object" || value === null || Array.isArray(value) ||
    !("$type" in value) || value.$type !== "u64"
  ) {
    return null;
  }
  const parsed = Number(value.value);
  return Number.isSafeInteger(parsed) && parsed >= 0 ? parsed : null;
};

const readPositiveUnsigned = (value: ProjectPayloadValue | undefined): number | null => {
  const parsed = readUnsigned(value);
  return parsed !== null && parsed > 0 ? parsed : null;
};
