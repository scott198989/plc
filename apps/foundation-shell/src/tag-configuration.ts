import type {
  ProjectPayloadValue,
  WorkbenchObjectView,
  WorkbenchOperation,
  WorkbenchSnapshot,
} from "./workbench-types";

export const tagAddressAreas = ["I", "Q", "M"] as const;
export type TagAddressArea = typeof tagAddressAreas[number];

export const ioTagDataTypes = ["BOOL", "INT"] as const;
export const memoryTagDataTypes = [
  "BOOL",
  "SINT",
  "INT",
  "DINT",
  "LINT",
  "USINT",
  "UINT",
  "UDINT",
  "ULINT",
  "BYTE",
  "WORD",
  "DWORD",
  "LWORD",
  "REAL",
  "LREAL",
  "CHAR",
  "STRING[80]",
  "TIME",
] as const;

export type TagDataType = typeof memoryTagDataTypes[number];
export type TagAddressIntent = "auto" | "explicit";

export type TagBindingOption = Readonly<{
  areas: readonly TagAddressArea[];
  blockId: string;
  blockKind: WorkbenchObjectView["kind"];
  blockName: string;
  dataType: string;
  key: string;
  memberId: string;
  memberName: string;
  role: string;
}>;

export type TagConfigurationDraft = Readonly<{
  addressIntent: TagAddressIntent;
  addressText: string;
  area: TagAddressArea;
  bindingKey: string | null;
  dataType: string;
  name: string;
}>;

export type ParsedTagAddress = Readonly<{
  bitOffset: number;
  byteOffset: number;
  canonicalText: string;
}>;

export type TagConfigurationErrors = Readonly<
  Partial<Record<"address" | "area" | "binding" | "dataType" | "name", string>>
>;

export type TagConfigurationValidation = Readonly<{
  errors: TagConfigurationErrors;
  parsedAddress: ParsedTagAddress | null;
  valid: boolean;
}>;

const PLC_IDENTIFIER = /^[A-Za-z_][A-Za-z0-9_]{0,127}$/u;
const CANONICAL_UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/iu;
const MAX_UINT32 = 4_294_967_295;

export const tagKindForArea = (area: TagAddressArea): "Input" | "Memory" | "Output" => {
  switch (area) {
    case "I": return "Input";
    case "Q": return "Output";
    case "M": return "Memory";
  }
};

export const dataTypesForTagArea = (area: TagAddressArea): readonly TagDataType[] =>
  area === "M" ? memoryTagDataTypes : ioTagDataTypes;

export const bindingKey = (blockId: string, memberId: string): string => `${blockId}:${memberId}`;

/**
 * Reads the current canonical tag without inventing defaults for its program
 * binding. A malformed binding remains visible as unavailable rather than being
 * silently replaced by renderer-local state.
 */
export const readTagConfiguration = (object: WorkbenchObjectView): TagConfigurationDraft => {
  const area = readArea(object.semanticPayload.addressArea);
  const dataType = typeof object.semanticPayload.dataType === "string"
    ? object.semanticPayload.dataType.toLocaleUpperCase("en-US")
    : "BOOL";
  const blockId = text(object.semanticPayload.blockId);
  const memberId = text(object.semanticPayload.memberId);
  const addressIntent = area === "M" || object.semanticPayload.addressIntent !== "explicit"
    ? "auto"
    : "explicit";
  const byteOffset = canonicalUnsigned(object.semanticPayload.byteOffset) ?? 0;
  const bitOffset = canonicalUnsigned(object.semanticPayload.bitOffset) ?? 0;
  return {
    addressIntent,
    addressText: addressIntent === "explicit"
      ? formatTagAddress(area, dataType, byteOffset, bitOffset)
      : suggestedAddress(area, dataType),
    area,
    bindingKey: blockId !== null && memberId !== null ? bindingKey(blockId, memberId) : null,
    dataType,
    name: object.displayName,
  };
};

/** Finds only well-formed, same-controller canonical interface members. */
export const discoverTagBindings = (
  snapshot: WorkbenchSnapshot,
  tag: WorkbenchObjectView,
): readonly TagBindingOption[] => {
  const controllerId = controllerAncestor(snapshot, tag);
  if (controllerId === null) {
    return [];
  }
  const options: TagBindingOption[] = [];
  for (const block of Object.values(snapshot.objects)) {
    if (
      block.lifecycle !== "active" ||
      !isProgramStorage(block.kind) ||
      !isDescendantOf(snapshot, block, controllerId)
    ) {
      continue;
    }
    const members = block.semanticPayload.interface ?? block.semanticPayload.members;
    if (!Array.isArray(members)) {
      continue;
    }
    for (const member of members) {
      const fields = recordFields(member);
      if (fields === null) {
        continue;
      }
      const memberId = text(fields.id);
      const memberName = text(fields.name);
      const dataType = text(fields.typeId) ?? text(fields.type);
      const role = text(fields.role)?.toLocaleLowerCase("en-US") ?? null;
      if (
        memberId === null ||
        !CANONICAL_UUID.test(memberId) ||
        memberName === null ||
        memberName.length === 0 ||
        dataType === null ||
        role === null
      ) {
        continue;
      }
      const areas = areasForRole(role);
      if (areas.length === 0) {
        continue;
      }
      options.push({
        areas,
        blockId: block.id,
        blockKind: block.kind,
        blockName: block.displayName,
        dataType: dataType.toLocaleUpperCase("en-US"),
        key: bindingKey(block.id, memberId),
        memberId,
        memberName,
        role,
      });
    }
  }
  return options.sort((left, right) =>
    left.blockName.localeCompare(right.blockName, "en-US") ||
    left.memberName.localeCompare(right.memberName, "en-US") ||
    left.memberId.localeCompare(right.memberId, "en-US")
  );
};

export const compatibleTagBindings = (
  bindings: readonly TagBindingOption[],
  area: TagAddressArea,
  dataType: string,
): readonly TagBindingOption[] => bindings.filter((candidate) =>
  candidate.areas.includes(area) && candidate.dataType === dataType.toLocaleUpperCase("en-US")
);

/**
 * The current workbench command surface updates one top-level field per
 * transaction. Rebinding within one block changes only memberId and therefore
 * keeps edu.tag/1 valid at every committed intermediate state.
 */
export const sequentiallySafeTagBindings = (
  bindings: readonly TagBindingOption[],
  source: TagConfigurationDraft,
): readonly TagBindingOption[] => {
  const current = bindings.find((candidate) => candidate.key === source.bindingKey);
  return current === undefined
    ? []
    : compatibleTagBindings(bindings, source.area, source.dataType)
        .filter((candidate) => candidate.blockId === current.blockId);
};

export const parseManualTagAddress = (
  addressText: string,
  area: TagAddressArea,
  dataType: string,
): Readonly<{ error: string; ok: false }> | Readonly<{ ok: true; value: ParsedTagAddress }> => {
  if (area === "M") {
    return { error: "Memory tags use program memory and do not take a hardware address.", ok: false };
  }
  const normalized = addressText.trim().toLocaleUpperCase("en-US");
  if (dataType === "BOOL") {
    const matched = /^%?([IQ])(\d+)\.([0-7])$/u.exec(normalized);
    if (matched === null) {
      return { error: `Enter a bit address such as %${area}0.0.`, ok: false };
    }
    if (matched[1] !== area) {
      return { error: `This ${tagKindForArea(area).toLocaleLowerCase("en-US")} tag needs a %${area} address.`, ok: false };
    }
    const byteOffset = Number(matched[2]);
    const bitOffset = Number(matched[3]);
    if (!Number.isSafeInteger(byteOffset) || byteOffset > MAX_UINT32) {
      return { error: "The address byte must be between 0 and 4294967295.", ok: false };
    }
    return {
      ok: true,
      value: {
        bitOffset,
        byteOffset,
        canonicalText: `%${area}${byteOffset}.${bitOffset}`,
      },
    };
  }
  if (dataType === "INT") {
    const matched = /^%?([IQ])W(\d+)$/u.exec(normalized);
    if (matched === null) {
      return { error: `Enter a word address such as %${area}W0.`, ok: false };
    }
    if (matched[1] !== area) {
      return { error: `This ${tagKindForArea(area).toLocaleLowerCase("en-US")} tag needs a %${area}W address.`, ok: false };
    }
    const byteOffset = Number(matched[2]);
    if (!Number.isSafeInteger(byteOffset) || byteOffset > MAX_UINT32) {
      return { error: "The address byte must be between 0 and 4294967295.", ok: false };
    }
    if (byteOffset % 2 !== 0) {
      return { error: "INT channels start on an even byte address.", ok: false };
    }
    return {
      ok: true,
      value: { bitOffset: 0, byteOffset, canonicalText: `%${area}W${byteOffset}` },
    };
  }
  return {
    error: "Physical I/O tags currently support BOOL digital channels and INT analog channels.",
    ok: false,
  };
};

export const validateTagConfiguration = (
  draft: TagConfigurationDraft,
  source: TagConfigurationDraft,
  bindings: readonly TagBindingOption[],
  snapshot: WorkbenchSnapshot,
  tag: WorkbenchObjectView,
): TagConfigurationValidation => {
  const errors: Partial<Record<keyof TagConfigurationErrors, string>> = {};
  if (!PLC_IDENTIFIER.test(draft.name)) {
    errors.name = "Use 1–128 letters, digits, or underscores, starting with a letter or underscore.";
  } else if (hasSiblingName(snapshot, tag, draft.name)) {
    errors.name = "Another tag in this symbol table already uses this name.";
  }

  const admittedTypes = dataTypesForTagArea(draft.area) as readonly string[];
  if (draft.area !== source.area) {
    errors.area = "Changing an existing tag area requires an atomic project update that is not available yet.";
  }
  if (draft.dataType !== source.dataType) {
    errors.dataType = "Changing an existing tag type requires an atomic project update that is not available yet.";
  } else if (!admittedTypes.includes(draft.dataType)) {
    errors.dataType = draft.area === "M"
      ? "Choose a canonical scalar PLC type."
      : "Physical I/O currently supports BOOL digital channels and INT analog channels.";
  }

  let parsedAddress: ParsedTagAddress | null = null;
  if (draft.area === "M" && draft.addressIntent !== "auto") {
    errors.address = "Memory tags are allocated in program memory and must use automatic allocation.";
  } else if (draft.addressIntent === "explicit") {
    const parsed = parseManualTagAddress(draft.addressText, draft.area, draft.dataType);
    if (!parsed.ok) {
      errors.address = parsed.error;
    } else {
      parsedAddress = parsed.value;
    }
  }

  const selectedBinding = bindings.find((candidate) => candidate.key === draft.bindingKey);
  const sourceBinding = bindings.find((candidate) => candidate.key === source.bindingKey);
  const sourceBindingStillOpaque =
    selectedBinding === undefined &&
    draft.bindingKey !== null &&
    draft.bindingKey === source.bindingKey &&
    draft.area === source.area &&
    draft.dataType === source.dataType;
  if (!sourceBindingStillOpaque) {
    if (selectedBinding === undefined) {
      errors.binding = "Choose a program variable for this tag.";
    } else if (
      selectedBinding.dataType !== draft.dataType ||
      !selectedBinding.areas.includes(draft.area)
    ) {
      errors.binding = "The program variable type or role does not match this tag.";
    } else if (
      draft.bindingKey !== source.bindingKey &&
      (sourceBinding === undefined || selectedBinding.blockId !== sourceBinding.blockId)
    ) {
      errors.binding = "Choose a compatible variable in the current program block. Moving a tag between blocks requires an atomic project update.";
    }
  }

  return { errors, parsedAddress, valid: Object.keys(errors).length === 0 };
};

export const tagConfigurationChanged = (
  draft: TagConfigurationDraft,
  source: TagConfigurationDraft,
): boolean =>
  draft.name !== source.name ||
  draft.area !== source.area ||
  draft.dataType !== source.dataType ||
  draft.addressIntent !== source.addressIntent ||
  (draft.addressIntent === "explicit" && draft.addressText !== source.addressText) ||
  draft.bindingKey !== source.bindingKey;

/**
 * Produces ordinary canonical workbench commands. The caller remains the only
 * authority that commits them through the existing engineering client.
 */
export const buildTagConfigurationOperations = (
  draft: TagConfigurationDraft,
  source: TagConfigurationDraft,
  validation: TagConfigurationValidation,
  bindings: readonly TagBindingOption[],
  tag: WorkbenchObjectView,
): readonly WorkbenchOperation[] => {
  if (!validation.valid) {
    return [];
  }
  const operations: WorkbenchOperation[] = [];
  if (draft.name !== source.name) {
    operations.push({
      displayName: draft.name,
      kind: "project.rename-object",
      objectId: tag.id,
    });
  }

  const selectedBinding = bindings.find((candidate) => candidate.key === draft.bindingKey);
  const currentBlockId = text(tag.semanticPayload.blockId);
  const currentMemberId = text(tag.semanticPayload.memberId);
  pushTextField(operations, tag, "blockId", selectedBinding?.blockId ?? currentBlockId);
  pushTextField(operations, tag, "memberId", selectedBinding?.memberId ?? currentMemberId);
  pushTextField(operations, tag, "dataType", draft.dataType);

  if (draft.addressIntent === "explicit" && validation.parsedAddress !== null) {
    pushUnsignedField(operations, tag, "byteOffset", validation.parsedAddress.byteOffset);
    pushUnsignedField(operations, tag, "bitOffset", validation.parsedAddress.bitOffset);
  }
  pushTextField(operations, tag, "addressIntent", draft.area === "M" ? "auto" : draft.addressIntent);
  pushTextField(operations, tag, "addressArea", draft.area);
  pushTextField(operations, tag, "tagKind", tagKindForArea(draft.area));
  return operations;
};

export const addressHelp = (area: TagAddressArea, dataType: string): string => {
  if (area === "M") {
    return "Program memory is resolved from the selected variable; no rack address is needed.";
  }
  return dataType === "BOOL"
    ? `Bit address format: %${area}byte.bit, for example %${area}0.0.`
    : `INT word address format: %${area}Wbyte, for example %${area}W0.`;
};

const readArea = (value: ProjectPayloadValue | undefined): TagAddressArea =>
  value === "Q" || value === "M" ? value : "I";

const suggestedAddress = (area: TagAddressArea, dataType: string): string => {
  if (area === "M") {
    return "";
  }
  return dataType === "BOOL" ? `%${area}0.0` : `%${area}W0`;
};

const formatTagAddress = (
  area: TagAddressArea,
  dataType: string,
  byteOffset: number,
  bitOffset: number,
): string => {
  if (area === "M") {
    return "";
  }
  return dataType === "BOOL"
    ? `%${area}${byteOffset}.${bitOffset}`
    : `%${area}W${byteOffset}`;
};

const canonicalUnsigned = (value: ProjectPayloadValue | undefined): number | null => {
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
  const parsed = Number(value.value);
  return Number.isSafeInteger(parsed) && parsed >= 0 ? parsed : null;
};

const text = (value: ProjectPayloadValue | undefined): string | null =>
  typeof value === "string" ? value : null;

const recordFields = (
  value: ProjectPayloadValue,
): Readonly<Record<string, ProjectPayloadValue>> | null => {
  if (
    typeof value !== "object" ||
    value === null ||
    Array.isArray(value) ||
    !("$type" in value) ||
    value.$type !== "record" ||
    !("value" in value) ||
    typeof value.value !== "object" ||
    value.value === null ||
    Array.isArray(value.value)
  ) {
    return null;
  }
  return value.value;
};

const isProgramStorage = (kind: WorkbenchObjectView["kind"]): boolean =>
  kind === "OB" || kind === "FC" || kind === "FB" || kind === "GlobalDB" || kind === "InstanceDB";

const controllerAncestor = (
  snapshot: WorkbenchSnapshot,
  object: WorkbenchObjectView,
): string | null => {
  let candidate: WorkbenchObjectView | undefined = object;
  const visited = new Set<string>();
  while (candidate !== undefined && !visited.has(candidate.id)) {
    if (candidate.kind === "Controller") {
      return candidate.id;
    }
    visited.add(candidate.id);
    candidate = candidate.parentId === null ? undefined : snapshot.objects[candidate.parentId];
  }
  return null;
};

const isDescendantOf = (
  snapshot: WorkbenchSnapshot,
  object: WorkbenchObjectView,
  ancestorId: string,
): boolean => {
  let parentId = object.parentId;
  const visited = new Set<string>();
  while (parentId !== null && !visited.has(parentId)) {
    if (parentId === ancestorId) {
      return true;
    }
    visited.add(parentId);
    parentId = snapshot.objects[parentId]?.parentId ?? null;
  }
  return false;
};

const areasForRole = (role: string): readonly TagAddressArea[] => {
  switch (role) {
    case "input": return ["I", "M"];
    case "output": return ["Q", "M"];
    case "inout":
    case "temp": return tagAddressAreas;
    case "static":
    case "return": return ["M"];
    default: return [];
  }
};

const hasSiblingName = (
  snapshot: WorkbenchSnapshot,
  tag: WorkbenchObjectView,
  name: string,
): boolean => Object.values(snapshot.objects).some((candidate) =>
  candidate.id !== tag.id &&
  candidate.lifecycle === "active" &&
  candidate.kind === "Tag" &&
  candidate.parentId === tag.parentId &&
  candidate.displayName.toLocaleLowerCase("en-US") === name.toLocaleLowerCase("en-US")
);

const pushTextField = (
  operations: WorkbenchOperation[],
  tag: WorkbenchObjectView,
  key: string,
  value: string | null,
): void => {
  if (value !== null && tag.semanticPayload[key] !== value) {
    operations.push({
      key,
      kind: "project.set-semantic-field",
      objectId: tag.id,
      value,
    });
  }
};

const pushUnsignedField = (
  operations: WorkbenchOperation[],
  tag: WorkbenchObjectView,
  key: string,
  value: number,
): void => {
  if (canonicalUnsigned(tag.semanticPayload[key]) !== value) {
    operations.push({
      key,
      kind: "project.set-semantic-field",
      objectId: tag.id,
      value: { $type: "u64", value: value.toString(10) },
    });
  }
};
