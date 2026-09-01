import { describe, expect, it } from "vitest";

import {
  activeRackForController,
  activeRackModules,
  buildModuleConfigurationOperation,
  controllerCatalogs,
  controllerForModule,
  createDigitalModulePayload,
  digitalModuleCatalogs,
  firstFreeModuleSlot,
  formatModuleAddressRange,
  legalModuleSlots,
  readModuleConfiguration,
  validateModuleConfiguration,
} from "../src/hardware-configuration";
import type { ModuleConfigurationDraft } from "../src/hardware-configuration";
import type {
  ProjectPayload,
  ProjectPayloadValue,
  WorkbenchObjectView,
  WorkbenchSnapshot,
} from "../src/workbench-types";

const ROOT_ID = "00000000-0000-4000-8000-000000000001";
const CONTROLLER_ID = "00000000-0000-4000-8000-000000000002";
const RACK_ID = "00000000-0000-4000-8000-000000000003";
const INPUT_ID = "00000000-0000-4000-8000-000000000004";
const OUTPUT_ID = "00000000-0000-4000-8000-000000000005";
const TOMBSTONED_ID = "00000000-0000-4000-8000-000000000006";

describe("hardware configuration", () => {
  it("offers the three proven brand-neutral controllers and the two MVP digital modules", () => {
    expect(controllerCatalogs.map((catalog) => ({
      catalogId: catalog.catalogId,
      firstSlot: catalog.firstExpansionSlot,
      lastSlot: catalog.lastExpansionSlot,
      power: catalog.requiresPowerModule,
    }))).toEqual([
      { catalogId: "vctrl-c1", firstSlot: 1, lastSlot: 8, power: false },
      { catalogId: "vctrl-m1", firstSlot: 2, lastSlot: 15, power: true },
      { catalogId: "vctrl-p1", firstSlot: 2, lastSlot: 31, power: true },
    ]);
    expect(digitalModuleCatalogs.map((catalog) => ({
      area: catalog.addressArea,
      bytes: catalog.addressBytes,
      catalogId: catalog.catalogId,
      model: catalog.modelName,
    }))).toEqual([
      { area: "I", bytes: 2, catalogId: "vdi16", model: "VDI16" },
      { area: "Q", bytes: 2, catalogId: "vdo16", model: "VDO16" },
    ]);
    expect(formatModuleAddressRange("vdi16", 0)).toBe("%I0..%I1");
    expect(formatModuleAddressRange("vdo16", 10)).toBe("%Q10..%Q11");
    expect(formatModuleAddressRange("unknown", 0)).toBeNull();
  });

  it("reads canonical explicit module fields for editing", () => {
    const { input } = fixture();
    expect(readModuleConfiguration({
      ...input,
      semanticPayload: {
        ...input.semanticPayload,
        addressIntent: "explicit",
        inputStart: unsigned(24),
        slot: unsigned(7),
      },
    })).toEqual({
      addressIntent: "explicit",
      catalogId: "vdi16",
      slotText: "7",
      startByteText: "24",
    });
  });

  it("discovers the rack/controller and finds free legal slots from active children only", () => {
    const { controller, input, rack, snapshot } = fixture();
    expect(controllerForModule(snapshot, input)?.id).toBe(CONTROLLER_ID);
    expect(activeRackForController(snapshot, controller)?.id).toBe(RACK_ID);
    expect(legalModuleSlots(snapshot, rack)).toEqual([1, 2, 3, 4, 5, 6, 7, 8]);
    expect(activeRackModules(snapshot, rack).map((module) => module.id)).toEqual([INPUT_ID, OUTPUT_ID]);
    expect(firstFreeModuleSlot(snapshot, rack)).toBe(3);

    const modularController = {
      ...controller,
      semanticPayload: { catalogId: "vctrl-m1" },
    };
    const modularSnapshot = withObjects(snapshot, modularController);
    expect(legalModuleSlots(modularSnapshot, rack)).toEqual(
      Array.from({ length: 14 }, (_, index) => index + 2),
    );
    expect(firstFreeModuleSlot(modularSnapshot, rack)).toBe(3);

    const performanceController = {
      ...controller,
      semanticPayload: { catalogId: "vctrl-p1" },
    };
    const performanceSlots = legalModuleSlots(withObjects(snapshot, performanceController), rack);
    expect(performanceSlots).toHaveLength(30);
    expect(performanceSlots.at(0)).toBe(2);
    expect(performanceSlots.at(-1)).toBe(31);
  });

  it("validates catalog, legal and unique slot, u32 start, and controller address range", () => {
    const { controller, input, snapshot } = fixture();
    const source = readModuleConfiguration(input);

    expect(validateModuleConfiguration({ ...source, catalogId: "vai4" }, snapshot, input)).toMatchObject({
      errors: { catalog: expect.any(String) },
      valid: false,
    });
    expect(validateModuleConfiguration({ ...source, slotText: "2" }, snapshot, input)).toMatchObject({
      errors: { slot: "Slot 2 already contains another module." },
      valid: false,
    });
    expect(validateModuleConfiguration({ ...source, slotText: "9" }, snapshot, input)).toMatchObject({
      errors: { slot: "Choose slot 1–8." },
      valid: false,
    });
    expect(validateModuleConfiguration({
      ...source,
      addressIntent: "explicit",
      startByteText: "4294967296",
    }, snapshot, input)).toMatchObject({
      errors: { startByte: "Enter a start byte from 0 to 4294967295." },
      valid: false,
    });
    expect(validateModuleConfiguration({
      ...source,
      addressIntent: "explicit",
      startByteText: "1023",
    }, snapshot, input)).toMatchObject({
      errors: { startByte: "VDI16 needs two bytes inside the 0–1023 I-area." },
      valid: false,
    });

    const modularSnapshot = withObjects(snapshot, {
      ...controller,
      semanticPayload: { catalogId: "vctrl-m1" },
    });
    expect(validateModuleConfiguration(source, modularSnapshot, input)).toMatchObject({
      errors: { slot: "Choose slot 2–15." },
      valid: false,
    });

    const performanceSnapshot = withObjects(snapshot, {
      ...controller,
      semanticPayload: { catalogId: "vctrl-p1" },
    });
    expect(validateModuleConfiguration({
      ...source,
      addressIntent: "explicit",
      slotText: "3",
      startByteText: "32766",
    }, performanceSnapshot, input)).toMatchObject({ parsedStartByte: 32_766, valid: true });
    expect(validateModuleConfiguration({
      ...source,
      addressIntent: "explicit",
      slotText: "3",
      startByteText: "32767",
    }, performanceSnapshot, input)).toMatchObject({
      errors: { startByte: "VDI16 needs two bytes inside the 0–32767 I-area." },
      valid: false,
    });
  });

  it("rejects two-byte manual overlap, permits adjacent spans, and keeps I/Q independent", () => {
    const { input, rack, snapshot } = fixture();
    const occupiedInput = object({
      displayName: "Second input",
      id: "00000000-0000-4000-8000-000000000010",
      kind: "Module",
      parentId: RACK_ID,
      semanticPayload: {
        // A stale start still projects explicitly even though the intent says auto.
        addressIntent: "auto",
        catalogId: "vdi16",
        inputStart: unsigned(4),
        slot: unsigned(3),
      },
    });
    const withOccupied = withObjects(snapshot, occupiedInput);
    const source = readModuleConfiguration(input);

    expect(validateModuleConfiguration({
      ...source,
      addressIntent: "explicit",
      startByteText: "5",
    }, withOccupied, input)).toMatchObject({
      errors: { startByte: "%I5..%I6 overlaps another active I-area module." },
      valid: false,
    });
    expect(validateModuleConfiguration({
      ...source,
      addressIntent: "explicit",
      startByteText: "2",
    }, withOccupied, input)).toMatchObject({ parsedStartByte: 2, valid: true });

    const outputAtSameBytes = object({
      displayName: "Manual output",
      id: "00000000-0000-4000-8000-000000000011",
      kind: "Module",
      parentId: RACK_ID,
      semanticPayload: {
        addressIntent: "explicit",
        catalogId: "vdo16",
        outputStart: unsigned(2),
        slot: unsigned(4),
      },
    });
    expect(validateModuleConfiguration({
      ...source,
      addressIntent: "explicit",
      startByteText: "2",
    }, withObjects(withOccupied, outputAtSameBytes), input).valid).toBe(true);
    expect(activeRackModules(withOccupied, rack)).toHaveLength(3);
  });

  it("builds one atomic replacement that preserves extensions and clears stale starts", () => {
    const { input, snapshot } = fixture();
    const moduleWithExtensions: WorkbenchObjectView = {
      ...input,
      semanticPayload: {
        ...input.semanticPayload,
        classroomNote: "Panel A",
        inputStart: unsigned(40),
        outputStart: unsigned(50),
      },
    };
    const automaticDraft: ModuleConfigurationDraft = {
      addressIntent: "auto",
      catalogId: "vdo16",
      slotText: "6",
      startByteText: "40",
    };
    const automaticValidation = validateModuleConfiguration(
      automaticDraft,
      withObjects(snapshot, moduleWithExtensions),
      moduleWithExtensions,
    );
    expect(buildModuleConfigurationOperation(
      automaticDraft,
      automaticValidation,
      moduleWithExtensions,
    )).toEqual({
      kind: "project.replace-semantic-payload",
      objectId: INPUT_ID,
      semanticPayload: {
        addressIntent: "auto",
        catalogId: "vdo16",
        classroomNote: "Panel A",
        slot: unsigned(6),
      },
    });

    const explicitDraft: ModuleConfigurationDraft = {
      ...automaticDraft,
      addressIntent: "explicit",
      startByteText: "12",
    };
    const explicitValidation = validateModuleConfiguration(
      explicitDraft,
      withObjects(snapshot, moduleWithExtensions),
      moduleWithExtensions,
    );
    expect(buildModuleConfigurationOperation(
      explicitDraft,
      explicitValidation,
      moduleWithExtensions,
    )).toEqual({
      kind: "project.replace-semantic-payload",
      objectId: INPUT_ID,
      semanticPayload: {
        addressIntent: "explicit",
        catalogId: "vdo16",
        classroomNote: "Panel A",
        outputStart: unsigned(12),
        slot: unsigned(6),
      },
    });
  });

  it("creates complete automatic and explicit module payloads", () => {
    expect(createDigitalModulePayload("vdi16", 1)).toEqual({
      addressIntent: "auto",
      catalogId: "vdi16",
      slot: unsigned(1),
    });
    expect(createDigitalModulePayload("vdo16", 2, {
      addressIntent: "explicit",
      startByte: 8,
    })).toEqual({
      addressIntent: "explicit",
      catalogId: "vdo16",
      outputStart: unsigned(8),
      slot: unsigned(2),
    });
    expect(() => createDigitalModulePayload("vdi16", 1, {
      addressIntent: "explicit",
      startByte: -1,
    })).toThrow("unsigned 32-bit integer");
  });
});

const fixture = (): Readonly<{
  controller: WorkbenchObjectView;
  input: WorkbenchObjectView;
  rack: WorkbenchObjectView;
  snapshot: WorkbenchSnapshot;
}> => {
  const root = object({ displayName: "Training project", id: ROOT_ID, kind: "ProjectRoot" });
  const controller = object({
    displayName: "PLC_1",
    id: CONTROLLER_ID,
    kind: "Controller",
    parentId: ROOT_ID,
    semanticPayload: { catalogId: "vctrl-c1" },
  });
  const rack = object({
    displayName: "Local rack",
    id: RACK_ID,
    kind: "Rack",
    parentId: CONTROLLER_ID,
    semanticPayload: { slotCount: unsigned(8) },
  });
  const input = object({
    displayName: "Input module",
    id: INPUT_ID,
    kind: "Module",
    parentId: RACK_ID,
    payloadSchema: "edu.module/1",
    semanticPayload: { addressIntent: "auto", catalogId: "vdi16", slot: unsigned(1) },
  });
  const output = object({
    displayName: "Output module",
    id: OUTPUT_ID,
    kind: "Module",
    parentId: RACK_ID,
    payloadSchema: "edu.module/1",
    semanticPayload: { addressIntent: "auto", catalogId: "vdo16", slot: unsigned(2) },
  });
  const tombstoned = {
    ...object({
      displayName: "Removed module",
      id: TOMBSTONED_ID,
      kind: "Module",
      parentId: RACK_ID,
      semanticPayload: { addressIntent: "auto", catalogId: "vdi16", slot: unsigned(3) },
    }),
    lifecycle: "tombstoned" as const,
  };
  const snapshot = snapshotFor([root, controller, rack, input, output, tombstoned]);
  return { controller, input, rack, snapshot };
};

const withObjects = (
  snapshot: WorkbenchSnapshot,
  ...objects: readonly WorkbenchObjectView[]
): WorkbenchSnapshot => ({
  ...snapshot,
  objects: {
    ...snapshot.objects,
    ...Object.fromEntries(objects.map((object) => [object.id, object])),
  },
});

const snapshotFor = (objects: readonly WorkbenchObjectView[]): WorkbenchSnapshot => ({
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
});

const object = (values: Readonly<{
  displayName: string;
  id: string;
  kind: WorkbenchObjectView["kind"];
  parentId?: string | null;
  payloadSchema?: string;
  semanticPayload?: ProjectPayload;
}>): WorkbenchObjectView => ({
  children: [],
  creationOrdinal: values.id,
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

const unsigned = (value: number): ProjectPayloadValue => ({
  $type: "u64",
  value: value.toString(10),
});
