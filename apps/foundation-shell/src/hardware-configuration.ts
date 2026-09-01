import { unsignedValue } from "./canonical-authoring";
import type {
  ProjectPayload,
  ProjectPayloadValue,
  WorkbenchObjectView,
  WorkbenchOperation,
  WorkbenchSnapshot,
} from "./workbench-types";

export const compactControllerCatalog = {
  catalogId: "vctrl-c1",
  description: "A compact virtual PLC with eight local expansion slots.",
  displayName: "Compact training PLC",
  firstExpansionSlot: 1,
  inputBytes: 1_024,
  lastExpansionSlot: 8,
  outputBytes: 1_024,
  requiresPowerModule: false,
} as const;

export const modularControllerCatalog = {
  catalogId: "vctrl-m1",
  description: "A modular virtual PLC with fourteen local expansion slots.",
  displayName: "Modular training PLC",
  firstExpansionSlot: 2,
  inputBytes: 8_192,
  lastExpansionSlot: 15,
  outputBytes: 8_192,
  requiresPowerModule: true,
} as const;

export const performanceControllerCatalog = {
  catalogId: "vctrl-p1",
  description: "A high-capacity virtual PLC with thirty local expansion slots.",
  displayName: "Performance training PLC",
  firstExpansionSlot: 2,
  inputBytes: 32_768,
  lastExpansionSlot: 31,
  outputBytes: 32_768,
  requiresPowerModule: true,
} as const;

export const controllerCatalogs = [
  compactControllerCatalog,
  modularControllerCatalog,
  performanceControllerCatalog,
] as const;
export type ControllerCatalogId = typeof controllerCatalogs[number]["catalogId"];
export type ControllerCatalogDescriptor = typeof controllerCatalogs[number];

export const digitalModuleCatalogs = [
  {
    addressArea: "I",
    addressBytes: 2,
    addressStartField: "inputStart",
    catalogId: "vdi16",
    channelCount: 16,
    description: "Sixteen 24 V-style virtual digital input channels.",
    displayName: "16-channel digital input",
    modelName: "VDI16",
  },
  {
    addressArea: "Q",
    addressBytes: 2,
    addressStartField: "outputStart",
    catalogId: "vdo16",
    channelCount: 16,
    description: "Sixteen 24 V-style virtual digital output channels.",
    displayName: "16-channel digital output",
    modelName: "VDO16",
  },
] as const;

export type DigitalModuleCatalogDescriptor = typeof digitalModuleCatalogs[number];
export type DigitalModuleCatalogId = DigitalModuleCatalogDescriptor["catalogId"];
export type ModuleAddressIntent = "auto" | "explicit";

export type ModuleConfigurationDraft = Readonly<{
  addressIntent: ModuleAddressIntent;
  catalogId: string;
  slotText: string;
  startByteText: string;
}>;

export type ModuleConfigurationErrors = Readonly<
  Partial<Record<"catalog" | "context" | "slot" | "startByte", string>>
>;

export type ModuleConfigurationValidation = Readonly<{
  catalog: DigitalModuleCatalogDescriptor | null;
  errors: ModuleConfigurationErrors;
  parsedSlot: number | null;
  parsedStartByte: number | null;
  valid: boolean;
}>;

export type ModuleAddressSelection =
  | Readonly<{ addressIntent: "auto" }>
  | Readonly<{ addressIntent: "explicit"; startByte: number }>;

export type ReplaceSemanticPayloadOperation = Extract<
  WorkbenchOperation,
  Readonly<{ kind: "project.replace-semantic-payload" }>
>;

const MAX_UINT32 = 4_294_967_295;

export const controllerCatalog = (catalogId: string): ControllerCatalogDescriptor | null =>
  controllerCatalogs.find((candidate) => candidate.catalogId === catalogId) ?? null;

export const digitalModuleCatalog = (catalogId: string): DigitalModuleCatalogDescriptor | null =>
  digitalModuleCatalogs.find((candidate) => candidate.catalogId === catalogId) ?? null;

/** Reads only canonical module fields; malformed values stay empty for correction. */
export const readModuleConfiguration = (
  module: WorkbenchObjectView,
): ModuleConfigurationDraft => {
  const catalogId = text(module.semanticPayload.catalogId) ?? "";
  const catalog = digitalModuleCatalog(catalogId);
  const startValue = catalog === null
    ? module.semanticPayload.inputStart ?? module.semanticPayload.outputStart
    : module.semanticPayload[catalog.addressStartField];
  return {
    addressIntent: module.semanticPayload.addressIntent === "explicit" ? "explicit" : "auto",
    catalogId,
    slotText: unsignedText(module.semanticPayload.slot) ?? "",
    startByteText: unsignedText(startValue) ?? "",
  };
};

export const rackForModule = (
  snapshot: WorkbenchSnapshot,
  module: WorkbenchObjectView,
): WorkbenchObjectView | null => {
  if (module.kind !== "Module" || module.parentId === null) {
    return null;
  }
  const rack = snapshot.objects[module.parentId];
  return rack?.kind === "Rack" && rack.lifecycle === "active" ? rack : null;
};

export const controllerForRack = (
  snapshot: WorkbenchSnapshot,
  rack: WorkbenchObjectView,
): WorkbenchObjectView | null => {
  if (rack.kind !== "Rack" || rack.parentId === null) {
    return null;
  }
  const controller = snapshot.objects[rack.parentId];
  return controller?.kind === "Controller" && controller.lifecycle === "active" ? controller : null;
};

export const controllerForModule = (
  snapshot: WorkbenchSnapshot,
  module: WorkbenchObjectView,
): WorkbenchObjectView | null => {
  const rack = rackForModule(snapshot, module);
  return rack === null ? null : controllerForRack(snapshot, rack);
};

export const activeRackForController = (
  snapshot: WorkbenchSnapshot,
  controller: WorkbenchObjectView,
): WorkbenchObjectView | null => Object.values(snapshot.objects)
  .filter((candidate) =>
    candidate.kind === "Rack" &&
    candidate.lifecycle === "active" &&
    candidate.parentId === controller.id
  )
  .sort(compareCanonicalOrder)[0] ?? null;

export const activeRackModules = (
  snapshot: WorkbenchSnapshot,
  rack: WorkbenchObjectView,
): readonly WorkbenchObjectView[] => Object.values(snapshot.objects)
  .filter((candidate) =>
    candidate.kind === "Module" &&
    candidate.lifecycle === "active" &&
    candidate.parentId === rack.id
  )
  .sort((left, right) => {
    const leftSlot = canonicalUnsigned(left.semanticPayload.slot) ?? Number.MAX_SAFE_INTEGER;
    const rightSlot = canonicalUnsigned(right.semanticPayload.slot) ?? Number.MAX_SAFE_INTEGER;
    return leftSlot - rightSlot || compareCanonicalOrder(left, right);
  });

export const legalModuleSlots = (
  snapshot: WorkbenchSnapshot,
  rack: WorkbenchObjectView,
): readonly number[] => {
  const controller = controllerForRack(snapshot, rack);
  const catalog = controller === null
    ? null
    : controllerCatalog(text(controller.semanticPayload.catalogId) ?? "");
  if (catalog === null) {
    return [];
  }
  return Array.from(
    { length: catalog.lastExpansionSlot - catalog.firstExpansionSlot + 1 },
    (_, index) => catalog.firstExpansionSlot + index,
  );
};

/** Finds the first legal slot not occupied by an active canonical module. */
export const firstFreeModuleSlot = (
  snapshot: WorkbenchSnapshot,
  rack: WorkbenchObjectView,
): number | null => {
  const occupied = new Set(
    activeRackModules(snapshot, rack)
      .map((module) => canonicalUnsigned(module.semanticPayload.slot))
      .filter((slot): slot is number => slot !== null),
  );
  return legalModuleSlots(snapshot, rack).find((slot) => !occupied.has(slot)) ?? null;
};

export const formatModuleAddressRange = (
  catalogId: string,
  startByte: number,
): string | null => {
  const catalog = digitalModuleCatalog(catalogId);
  if (catalog === null || !isUint32(startByte) || startByte > MAX_UINT32 - catalog.addressBytes + 1) {
    return null;
  }
  return `%${catalog.addressArea}${startByte}..%${catalog.addressArea}${startByte + catalog.addressBytes - 1}`;
};

export const validateModuleConfiguration = (
  draft: ModuleConfigurationDraft,
  snapshot: WorkbenchSnapshot,
  module: WorkbenchObjectView,
): ModuleConfigurationValidation => {
  const errors: Partial<Record<keyof ModuleConfigurationErrors, string>> = {};
  const catalog = digitalModuleCatalog(draft.catalogId);
  if (catalog === null) {
    errors.catalog = "Choose a supported digital input or digital output module.";
  }

  const rack = rackForModule(snapshot, module);
  const controller = rack === null ? null : controllerForRack(snapshot, rack);
  const controllerDefinition = controller === null
    ? null
    : controllerCatalog(text(controller.semanticPayload.catalogId) ?? "");
  if (rack === null || controller === null) {
    errors.context = "This module is not attached to an active local rack and controller.";
  } else if (controllerDefinition === null) {
    errors.context = "This controller model is not available in the guided hardware editor.";
  }

  const slotResult = parseUnsignedDecimal(draft.slotText);
  const parsedSlot = slotResult.ok ? slotResult.value : null;
  if (!slotResult.ok) {
    errors.slot = "Enter a whole-number rack slot.";
  } else if (
    controllerDefinition !== null &&
    (slotResult.value < controllerDefinition.firstExpansionSlot ||
      slotResult.value > controllerDefinition.lastExpansionSlot)
  ) {
    errors.slot = `Choose slot ${controllerDefinition.firstExpansionSlot}–${controllerDefinition.lastExpansionSlot}.`;
  } else if (
    rack !== null &&
    activeRackModules(snapshot, rack).some((candidate) =>
      candidate.id !== module.id && canonicalUnsigned(candidate.semanticPayload.slot) === slotResult.value
    )
  ) {
    errors.slot = `Slot ${slotResult.value} already contains another module.`;
  }

  let parsedStartByte: number | null = null;
  if (draft.addressIntent === "explicit") {
    const startResult = parseUnsignedDecimal(draft.startByteText);
    if (!startResult.ok) {
      errors.startByte = "Enter a start byte from 0 to 4294967295.";
    } else {
      parsedStartByte = startResult.value;
      if (catalog !== null && controllerDefinition !== null) {
        const capacity = catalog.addressArea === "I"
          ? controllerDefinition.inputBytes
          : controllerDefinition.outputBytes;
        if (startResult.value > capacity - catalog.addressBytes) {
          errors.startByte = `${catalog.modelName} needs two bytes inside the 0–${capacity - 1} ${catalog.addressArea}-area.`;
        } else if (
          rack !== null &&
          hasManualAddressOverlap(snapshot, rack, module.id, catalog, startResult.value)
        ) {
          const range = formatModuleAddressRange(catalog.catalogId, startResult.value);
          errors.startByte = `${range ?? "That address range"} overlaps another active ${catalog.addressArea}-area module.`;
        }
      }
    }
  }

  return {
    catalog,
    errors,
    parsedSlot,
    parsedStartByte,
    valid: Object.keys(errors).length === 0,
  };
};

/**
 * Replaces the complete semantic payload in one canonical operation. This
 * keeps unrelated extension fields while removing input/output starts that no
 * longer belong to the selected catalog or automatic allocation mode.
 */
export const buildModuleConfigurationOperation = (
  draft: ModuleConfigurationDraft,
  validation: ModuleConfigurationValidation,
  module: WorkbenchObjectView,
): ReplaceSemanticPayloadOperation | null => {
  if (!validation.valid || validation.catalog === null || validation.parsedSlot === null) {
    return null;
  }
  const { inputStart: _inputStart, outputStart: _outputStart, ...unrelated } = module.semanticPayload;
  const semanticPayload: ProjectPayload = {
    ...unrelated,
    addressIntent: draft.addressIntent,
    catalogId: validation.catalog.catalogId,
    slot: unsignedValue(validation.parsedSlot),
    ...(draft.addressIntent === "explicit" && validation.parsedStartByte !== null
      ? { [validation.catalog.addressStartField]: unsignedValue(validation.parsedStartByte) }
      : {}),
  };
  return {
    kind: "project.replace-semantic-payload",
    objectId: module.id,
    semanticPayload,
  } as ReplaceSemanticPayloadOperation;
};

/** Creates a complete edu.module/1 semantic payload for a valid digital module. */
export const createDigitalModulePayload = (
  catalogId: DigitalModuleCatalogId,
  slot: number,
  address: ModuleAddressSelection = { addressIntent: "auto" },
): ProjectPayload => {
  const catalog = digitalModuleCatalog(catalogId);
  if (catalog === null) {
    throw new Error("Unsupported digital module catalog.");
  }
  if (!Number.isInteger(slot) || slot < 0 || slot > MAX_UINT32) {
    throw new Error("The module slot must be an unsigned 32-bit integer.");
  }
  const payload: Record<string, ProjectPayloadValue> = {
    addressIntent: address.addressIntent,
    catalogId,
    slot: unsignedValue(slot),
  };
  if (address.addressIntent === "explicit") {
    if (!isUint32(address.startByte)) {
      throw new Error("The module start byte must be an unsigned 32-bit integer.");
    }
    payload[catalog.addressStartField] = unsignedValue(address.startByte);
  }
  return payload;
};

const hasManualAddressOverlap = (
  snapshot: WorkbenchSnapshot,
  rack: WorkbenchObjectView,
  moduleId: string,
  catalog: DigitalModuleCatalogDescriptor,
  startByte: number,
): boolean => activeRackModules(snapshot, rack).some((candidate) => {
  if (candidate.id === moduleId) {
    return false;
  }
  const siblingCatalog = digitalModuleCatalog(text(candidate.semanticPayload.catalogId) ?? "");
  if (siblingCatalog === null || siblingCatalog.addressArea !== catalog.addressArea) {
    return false;
  }
  // The canonical projection treats a present start field as explicit even if
  // an older payload still says addressIntent=auto.
  const siblingStart = canonicalUnsigned(candidate.semanticPayload[siblingCatalog.addressStartField]);
  return siblingStart !== null &&
    startByte < siblingStart + siblingCatalog.addressBytes &&
    siblingStart < startByte + catalog.addressBytes;
});

const parseUnsignedDecimal = (
  value: string,
): Readonly<{ ok: false }> | Readonly<{ ok: true; value: number }> => {
  const normalized = value.trim();
  if (!/^\d+$/u.test(normalized)) {
    return { ok: false };
  }
  const parsed = BigInt(normalized);
  return parsed <= BigInt(MAX_UINT32)
    ? { ok: true, value: Number(parsed) }
    : { ok: false };
};

const canonicalUnsigned = (value: ProjectPayloadValue | undefined): number | null => {
  const canonical = unsignedText(value);
  if (canonical === null || !/^\d+$/u.test(canonical)) {
    return null;
  }
  const parsed = BigInt(canonical);
  return parsed <= BigInt(Number.MAX_SAFE_INTEGER) ? Number(parsed) : null;
};

const unsignedText = (value: ProjectPayloadValue | undefined): string | null => {
  if (
    typeof value !== "object" ||
    value === null ||
    Array.isArray(value) ||
    !("$type" in value) ||
    value.$type !== "u64" ||
    !("value" in value) ||
    typeof value.value !== "string"
  ) {
    return null;
  }
  return value.value;
};

const text = (value: ProjectPayloadValue | undefined): string | null =>
  typeof value === "string" ? value : null;

const isUint32 = (value: number): boolean =>
  Number.isInteger(value) && value >= 0 && value <= MAX_UINT32;

const compareCanonicalOrder = (
  left: WorkbenchObjectView,
  right: WorkbenchObjectView,
): number => left.creationOrdinal.localeCompare(right.creationOrdinal) || left.id.localeCompare(right.id);
