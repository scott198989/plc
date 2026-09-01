import { describe, expect, it } from "vitest";

import {
  appendHmiElement,
  createHmiLamp,
  createHmiMomentaryButton,
  createHmiScreen,
  createHmiSession,
  decodeHmiScreenPayload,
  encodeHmiScreenPayload,
  HMI_SCREEN_PAYLOAD_SCHEMA,
  projectHmiMomentaryWrites,
  setHmiMomentaryPressed,
  setHmiSessionMode,
  validateHmiScreen,
} from "../src/hmi-screen-model";
import type {
  HmiBindableTag,
  HmiScreen,
} from "../src/hmi-screen-model";

const BUTTON_ID = "00000000-0000-4000-8000-000000000101";
const LAMP_ID = "00000000-0000-4000-8000-000000000102";
const START_TAG_ID = "00000000-0000-4000-8000-000000000201";
const MOTOR_TAG_ID = "00000000-0000-4000-8000-000000000202";

const tags: readonly HmiBindableTag[] = [
  { addressArea: "I", dataType: "BOOL", id: START_TAG_ID, name: "HMI_Start" },
  { addressArea: "Q", dataType: "BOOL", id: MOTOR_TAG_ID, name: "Motor_Output" },
];

const completeScreen = (): HmiScreen => appendHmiElement(
  appendHmiElement(
    createHmiScreen("Motor station"),
    createHmiMomentaryButton({ id: BUTTON_ID, label: "Start", tagId: START_TAG_ID }),
  ),
  createHmiLamp({ id: LAMP_ID, label: "Motor running", tagId: MOTOR_TAG_ID }),
);

describe("educational HMI screen model", () => {
  it("uses an explicit versioned project payload contract and round-trips stable tag IDs", () => {
    const source = completeScreen();
    const payload = encodeHmiScreenPayload(source);

    expect(HMI_SCREEN_PAYLOAD_SCHEMA).toBe("edu.hmi-screen/1");
    expect(payload).toMatchObject({
      runtimeTarget: "virtual",
      schemaVersion: { $type: "u64", value: "1" },
    });
    expect(decodeHmiScreenPayload(payload)).toEqual({ ok: true, screen: source });
    expect(JSON.stringify(payload)).toContain(START_TAG_ID);
    expect(JSON.stringify(payload)).toContain(MOTOR_TAG_ID);
  });

  it("accepts only compatible BOOL bindings for the two MVP element kinds", () => {
    expect(validateHmiScreen(completeScreen(), tags)).toEqual({ issues: [], valid: true });

    const wrongTags: readonly HmiBindableTag[] = [
      { addressArea: "M", dataType: "BOOL", id: START_TAG_ID, name: "Internal_Start" },
      { addressArea: "M", dataType: "DINT", id: MOTOR_TAG_ID, name: "Motor_Count" },
    ];
    const result = validateHmiScreen(completeScreen(), wrongTags);
    expect(result.valid).toBe(false);
    expect(result.issues.map((issue) => issue.code)).toEqual([
      "HMI_MOMENTARY_BINDING_INCOMPATIBLE",
      "HMI_TAG_TYPE_INCOMPATIBLE",
    ]);
  });

  it("reports unbound and deleted tag references instead of guessing replacements", () => {
    const screen = appendHmiElement(
      appendHmiElement(
        createHmiScreen(),
        createHmiMomentaryButton({ id: BUTTON_ID }),
      ),
      createHmiLamp({ id: LAMP_ID, tagId: MOTOR_TAG_ID }),
    );
    const result = validateHmiScreen(screen, []);

    expect(result.valid).toBe(false);
    expect(result.issues).toEqual([
      expect.objectContaining({ code: "HMI_BINDING_REQUIRED", elementId: BUTTON_ID }),
      expect.objectContaining({ code: "HMI_TAG_NOT_FOUND", elementId: LAMP_ID }),
    ]);
  });

  it("keeps edit interactions inert and produces momentary values only in run mode", () => {
    const screen = completeScreen();
    const editing = createHmiSession();
    expect(setHmiMomentaryPressed(screen, editing, BUTTON_ID, true)).toBe(editing);

    const running = setHmiSessionMode(editing, "run");
    const pressed = setHmiMomentaryPressed(screen, running, BUTTON_ID, true);
    expect(projectHmiMomentaryWrites(screen, pressed)).toEqual([
      { tagId: START_TAG_ID, value: true },
    ]);

    const released = setHmiMomentaryPressed(screen, pressed, BUTTON_ID, false);
    expect(projectHmiMomentaryWrites(screen, released)).toEqual([
      { tagId: START_TAG_ID, value: false },
    ]);
    expect(setHmiSessionMode(pressed, "edit")).toEqual({ mode: "edit", pressedButtonIds: [] });
  });

  it("rejects malformed persisted data and any non-virtual runtime target", () => {
    const payload = encodeHmiScreenPayload(completeScreen());
    expect(decodeHmiScreenPayload({ ...payload, runtimeTarget: "physical" })).toEqual({
      message: "HMI runtime target must be virtual.",
      ok: false,
    });

    const unsafe = { ...completeScreen(), runtimeTarget: "physical" } as unknown as HmiScreen;
    expect(validateHmiScreen(unsafe, tags).issues).toContainEqual(expect.objectContaining({
      code: "HMI_RUNTIME_TARGET_UNSAFE",
    }));
  });
});
