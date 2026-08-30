import { describe, expect, it } from "vitest";

import {
  buildTagConfigurationOperations,
  compatibleTagBindings,
  discoverTagBindings,
  parseManualTagAddress,
  readTagConfiguration,
  sequentiallySafeTagBindings,
  validateTagConfiguration,
} from "../src/tag-configuration";
import type { TagConfigurationDraft } from "../src/tag-configuration";
import type {
  ProjectPayload,
  ProjectPayloadValue,
  WorkbenchObjectView,
  WorkbenchSnapshot,
} from "../src/workbench-types";

const ROOT_ID = "00000000-0000-4000-8000-000000000001";
const CONTROLLER_ID = "00000000-0000-4000-8000-000000000002";
const SYMBOL_TABLE_ID = "00000000-0000-4000-8000-000000000003";
const TAG_ID = "00000000-0000-4000-8000-000000000004";
const OB_ID = "00000000-0000-4000-8000-000000000005";
const INPUT_MEMBER_ID = "00000000-0000-4000-8000-000000000006";
const OUTPUT_MEMBER_ID = "00000000-0000-4000-8000-000000000007";
const DB_ID = "00000000-0000-4000-8000-000000000008";
const MEMORY_MEMBER_ID = "00000000-0000-4000-8000-000000000009";
const OTHER_CONTROLLER_ID = "00000000-0000-4000-8000-000000000010";
const OTHER_OB_ID = "00000000-0000-4000-8000-000000000011";
const OTHER_MEMBER_ID = "00000000-0000-4000-8000-000000000012";

describe("tag configuration", () => {
  it("reads an explicit canonical BOOL input tag", () => {
    const { tag } = fixture();
    const configured = readTagConfiguration({
      ...tag,
      semanticPayload: {
        ...tag.semanticPayload,
        addressIntent: "explicit",
        bitOffset: unsigned(3),
        byteOffset: unsigned(12),
      },
    });

    expect(configured).toEqual({
      addressIntent: "explicit",
      addressText: "%I12.3",
      area: "I",
      bindingKey: `${OB_ID}:${INPUT_MEMBER_ID}`,
      dataType: "BOOL",
      name: "Start_PB",
    });
  });

  it("discovers only safe same-controller members and preserves role compatibility", () => {
    const { snapshot, tag } = fixture();
    const bindings = discoverTagBindings(snapshot, tag);

    expect(bindings.map((binding) => [binding.blockName, binding.memberName, binding.areas])).toEqual([
      ["Main", "Motor", ["Q", "M"]],
      ["Main", "Start", ["I", "M"]],
      ["Memory", "Latched", ["M"]],
    ]);
    expect(bindings.some((binding) => binding.memberId === OTHER_MEMBER_ID)).toBe(false);
    expect(compatibleTagBindings(bindings, "I", "BOOL").map((binding) => binding.memberName)).toEqual(["Start"]);
    expect(compatibleTagBindings(bindings, "Q", "BOOL").map((binding) => binding.memberName)).toEqual(["Motor"]);
    expect(compatibleTagBindings(bindings, "M", "DINT").map((binding) => binding.memberName)).toEqual(["Latched"]);
    expect(sequentiallySafeTagBindings(bindings, readTagConfiguration(tag)).map((binding) => binding.memberName)).toEqual(["Start"]);
  });

  it("parses only engine-supported manual I/O address shapes", () => {
    expect(parseManualTagAddress("i7.6", "I", "BOOL")).toEqual({
      ok: true,
      value: { bitOffset: 6, byteOffset: 7, canonicalText: "%I7.6" },
    });
    expect(parseManualTagAddress("%QW12", "Q", "INT")).toEqual({
      ok: true,
      value: { bitOffset: 0, byteOffset: 12, canonicalText: "%QW12" },
    });
    expect(parseManualTagAddress("%I0.8", "I", "BOOL")).toMatchObject({ ok: false });
    expect(parseManualTagAddress("%Q0.0", "I", "BOOL")).toMatchObject({
      error: "This input tag needs a %I address.",
      ok: false,
    });
    expect(parseManualTagAddress("%IW3", "I", "INT")).toMatchObject({
      error: "INT channels start on an even byte address.",
      ok: false,
    });
    expect(parseManualTagAddress("%M0.0", "M", "BOOL")).toMatchObject({ ok: false });
  });

  it("validates names, bindings, types, and manual addresses before emitting operations", () => {
    const { snapshot, tag } = fixture();
    const source = readTagConfiguration(tag);
    const bindings = discoverTagBindings(snapshot, tag);
    const invalid: TagConfigurationDraft = {
      ...source,
      addressIntent: "explicit",
      addressText: "%Q0.9",
      area: "Q",
      bindingKey: `${OB_ID}:${INPUT_MEMBER_ID}`,
      dataType: "DINT",
      name: "Bad tag name",
    };
    const result = validateTagConfiguration(invalid, source, bindings, snapshot, tag);

    expect(result.valid).toBe(false);
    expect(result.errors).toMatchObject({
      address: expect.any(String),
      binding: expect.any(String),
      dataType: expect.any(String),
      name: expect.any(String),
    });
    expect(buildTagConfigurationOperations(invalid, source, result, bindings, tag)).toEqual([]);
  });

  it("emits only individually valid canonical operations for a name and input-address edit", () => {
    const { snapshot, tag } = fixture();
    const source = readTagConfiguration(tag);
    const bindings = discoverTagBindings(snapshot, tag);
    const draft: TagConfigurationDraft = {
      addressIntent: "explicit",
      addressText: "i2.5",
      area: "I",
      bindingKey: `${OB_ID}:${INPUT_MEMBER_ID}`,
      dataType: "BOOL",
      name: "Motor_Run",
    };
    const validation = validateTagConfiguration(draft, source, bindings, snapshot, tag);
    const operations = buildTagConfigurationOperations(draft, source, validation, bindings, tag);

    expect(validation).toMatchObject({
      parsedAddress: { bitOffset: 5, byteOffset: 2, canonicalText: "%I2.5" },
      valid: true,
    });
    expect(operations).toEqual([
      { displayName: "Motor_Run", kind: "project.rename-object", objectId: TAG_ID },
      {
        key: "byteOffset",
        kind: "project.set-semantic-field",
        objectId: TAG_ID,
        value: unsigned(2),
      },
      {
        key: "bitOffset",
        kind: "project.set-semantic-field",
        objectId: TAG_ID,
        value: unsigned(5),
      },
      {
        key: "addressIntent",
        kind: "project.set-semantic-field",
        objectId: TAG_ID,
        value: "explicit",
      },
    ]);
  });

  it("allows a safely preserved opaque binding only while type and area stay unchanged", () => {
    const { snapshot, tag } = fixture();
    const opaqueTag: WorkbenchObjectView = {
      ...tag,
      semanticPayload: {
        ...tag.semanticPayload,
        blockId: "00000000-0000-4000-8000-000000000099",
        memberId: "00000000-0000-4000-8000-000000000098",
      },
    };
    const source = readTagConfiguration(opaqueTag);
    const bindings = discoverTagBindings(snapshot, opaqueTag);

    expect(validateTagConfiguration(
      { ...source, name: "Start_Station" },
      source,
      bindings,
      snapshot,
      opaqueTag,
    ).valid).toBe(true);
    expect(validateTagConfiguration(
      { ...source, area: "Q" },
      source,
      bindings,
      snapshot,
      opaqueTag,
    ).errors.binding).toBeDefined();
  });
});

const fixture = (): Readonly<{
  snapshot: WorkbenchSnapshot;
  tag: WorkbenchObjectView;
}> => {
  const root = object({ displayName: "Training project", id: ROOT_ID, kind: "ProjectRoot" });
  const controller = object({
    displayName: "PLC_1",
    id: CONTROLLER_ID,
    kind: "Controller",
    parentId: ROOT_ID,
  });
  const symbolTable = object({
    displayName: "PLC tags",
    id: SYMBOL_TABLE_ID,
    kind: "SymbolTable",
    parentId: CONTROLLER_ID,
  });
  const tag = object({
    displayName: "Start_PB",
    id: TAG_ID,
    kind: "Tag",
    parentId: SYMBOL_TABLE_ID,
    payloadSchema: "edu.tag/1",
    semanticPayload: {
      addressArea: "I",
      addressIntent: "auto",
      blockId: OB_ID,
      dataType: "BOOL",
      memberId: INPUT_MEMBER_ID,
      tagKind: "Input",
    },
  });
  const existingTag = object({
    displayName: "Existing_Tag",
    id: "00000000-0000-4000-8000-000000000013",
    kind: "Tag",
    parentId: SYMBOL_TABLE_ID,
  });
  const main = object({
    displayName: "Main",
    id: OB_ID,
    kind: "OB",
    parentId: CONTROLLER_ID,
    semanticPayload: {
      interface: [
        member(INPUT_MEMBER_ID, "Start", "input", "BOOL"),
        member(OUTPUT_MEMBER_ID, "Motor", "output", "BOOL"),
        record({ id: "malformed", name: "Ignored", role: "temp", type: "BOOL" }),
      ],
      language: "LAD",
    },
  });
  const memory = object({
    displayName: "Memory",
    id: DB_ID,
    kind: "GlobalDB",
    parentId: CONTROLLER_ID,
    semanticPayload: { members: [member(MEMORY_MEMBER_ID, "Latched", "static", "DINT")] },
  });
  const otherController = object({
    displayName: "PLC_2",
    id: OTHER_CONTROLLER_ID,
    kind: "Controller",
    parentId: ROOT_ID,
  });
  const otherMain = object({
    displayName: "Other main",
    id: OTHER_OB_ID,
    kind: "OB",
    parentId: OTHER_CONTROLLER_ID,
    semanticPayload: {
      interface: [member(OTHER_MEMBER_ID, "Foreign", "input", "BOOL")],
      language: "LAD",
    },
  });
  const objects = [root, controller, symbolTable, tag, existingTag, main, memory, otherController, otherMain];
  return {
    snapshot: {
      buildState: "not-built",
      diagnostics: [],
      dirtyState: "clean",
      documentId: "00000000-0000-4000-8000-000000000020",
      documentRevision: "0",
      fileGrantId: null,
      lastSavedProjectHash: null,
      objects: Object.fromEntries(objects.map((candidate) => [candidate.id, candidate])),
      projectHash: "hash",
      projectName: "Training project",
      projectRootId: ROOT_ID,
      runtime: {
        availability: "UNAVAILABLE",
        canBuild: false,
        diagnostics: [],
        reason: "fixture",
        schemaVersion: 1,
        session: null,
        sourceDocumentHash: "hash",
        sourceSemanticFingerprint: "fingerprint",
      },
      semanticRevision: "0",
      undo: { canRedo: false, canUndo: false, redoLabel: null, undoLabel: null },
    },
    tag,
  };
};

const object = (values: Readonly<{
  displayName: string;
  id: string;
  kind: WorkbenchObjectView["kind"];
  parentId?: string | null;
  payloadSchema?: string;
  semanticPayload?: ProjectPayload;
}>): WorkbenchObjectView => ({
  children: [],
  creationOrdinal: "1",
  displayName: values.displayName,
  id: values.id,
  kind: values.kind,
  lifecycle: "active",
  objectRevision: "0",
  parentId: values.parentId ?? null,
  payloadSchema: values.payloadSchema ?? "fixture/1",
  presentationPayload: {},
  semanticPayload: values.semanticPayload ?? {},
  semanticRevision: "0",
});

const member = (
  id: string,
  name: string,
  role: string,
  type: string,
): ProjectPayloadValue => record({ id, name, order: unsigned(0), role, type });

const record = (value: Readonly<Record<string, ProjectPayloadValue>>): ProjectPayloadValue => ({
  $type: "record",
  value,
});

const unsigned = (value: number): ProjectPayloadValue => ({
  $type: "u64",
  value: value.toString(10),
});
