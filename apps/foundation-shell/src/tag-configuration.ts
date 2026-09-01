import type {
  ProjectPayload,
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
export const TAG_DESCRIPTION_MAX_LENGTH = 1_000;

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
  description: string;
  name: string;
}>;

export type TagProgramOption = Readonly<{
  id: string;
  name: string;
}>;

export type TagWithMemberCreationDraft = Readonly<{
  addressIntent: TagAddressIntent;
  addressText: string;
  area: TagAddressArea;
  dataType: string;
  description: string;
  name: string;
  programId: string | null;
}>;

export type TagWithMemberCreationPlan = Readonly<{
  memberId: string;
  operations: readonly WorkbenchOperation[];
  tagId: string;
}>;

export type ParsedTagAddress = Readonly<{
  bitOffset: number;
  byteOffset: number;
  canonicalText: string;
}>;

export type TagConfigurationErrors = Readonly<
  Partial<Record<"address" | "area" | "binding" | "dataType" | "description" | "name", string>>
>;

export type TagConfigurationValidation = Readonly<{
  errors: TagConfigurationErrors;
  parsedAddress: ParsedTagAddress | null;
  valid: boolean;
}>;

const PLC_IDENTIFIER = /^[A-Za-z_][A-Za-z0-9_]{0,127}$/u;
const CANONICAL_UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/iu;
const MAX_UINT32 = 4_294_967_295;
const defaultIdFactory = (): string => crypto.randomUUID();

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
    description: text(object.semanticPayload.comment) ?? "",
    name: object.displayName,
  };
};

/** Finds only well-formed, same-controller canonical interface members. */
export const discoverTagBindings = (
  snapshot: WorkbenchSnapshot,
  owner: WorkbenchObjectView,
): readonly TagBindingOption[] => {
  const controllerId = controllerAncestor(snapshot, owner);
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

/** Backward-compatible alias now that complete tag payloads update atomically. */
export const sequentiallySafeTagBindings = (
  bindings: readonly TagBindingOption[],
  source: TagConfigurationDraft,
): readonly TagBindingOption[] => compatibleTagBindings(bindings, source.area, source.dataType);

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
  const { errors, parsedAddress } = validateTagFields(
    draft,
    snapshot,
    tag.parentId,
    tag.id,
  );

  const selectedBinding = bindings.find((candidate) => candidate.key === draft.bindingKey);
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
  draft.description !== source.description ||
  draft.addressIntent !== source.addressIntent ||
  (draft.addressIntent === "explicit" && draft.addressText !== source.addressText) ||
  draft.bindingKey !== source.bindingKey;

/**
 * Renames remain their own identity command, while every semantic field is
 * replaced in one transaction so an area, type, address, and binding change
 * can never leave a partially updated tag in the project.
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
  const semanticChangeRequested =
    draft.area !== source.area ||
    draft.dataType !== source.dataType ||
    draft.description !== source.description ||
    draft.addressIntent !== source.addressIntent ||
    (draft.addressIntent === "explicit" && draft.addressText !== source.addressText) ||
    draft.bindingKey !== source.bindingKey;
  if (semanticChangeRequested) {
    const semanticPayload = createTagSemanticPayload(
      draft,
      validation,
      selectedBinding?.blockId ?? currentBlockId,
      selectedBinding?.memberId ?? currentMemberId,
      tag.semanticPayload,
    );
    if (semanticPayload !== null) {
      operations.push({
        kind: "project.replace-semantic-payload",
        objectId: tag.id,
        semanticPayload,
      });
    }
  }
  return operations;
};

/** Empty learner-facing values for creating a tag from an existing variable. */
export const createTagDraftDefaults = (
  area: TagAddressArea = "I",
): TagConfigurationDraft => ({
  addressIntent: "auto",
  addressText: suggestedAddress(area, "BOOL"),
  area,
  bindingKey: null,
  dataType: "BOOL",
  description: "",
  name: "",
});

export const validateTagCreation = (
  draft: TagConfigurationDraft,
  snapshot: WorkbenchSnapshot,
  symbolTable: WorkbenchObjectView,
): TagConfigurationValidation => {
  const { errors, parsedAddress } = validateTagFields(draft, snapshot, symbolTable.id, null);
  const bindings = discoverTagBindings(snapshot, symbolTable);
  const selectedBinding = bindings.find((candidate) => candidate.key === draft.bindingKey);
  if (selectedBinding === undefined) {
    errors.binding = "Choose a program variable for this tag.";
  } else if (
    selectedBinding.dataType !== draft.dataType ||
    !selectedBinding.areas.includes(draft.area)
  ) {
    errors.binding = "The program variable type or role does not match this tag.";
  }
  return { errors, parsedAddress, valid: Object.keys(errors).length === 0 };
};

/** Emits one complete edu.tag/1 create command after all learner choices validate. */
export const buildTagCreationOperation = (
  draft: TagConfigurationDraft,
  validation: TagConfigurationValidation,
  snapshot: WorkbenchSnapshot,
  symbolTable: WorkbenchObjectView,
  idFactory: () => string = defaultIdFactory,
): WorkbenchOperation | null => {
  const currentValidation = validateTagCreation(draft, snapshot, symbolTable);
  if (!validation.valid || !currentValidation.valid || symbolTable.kind !== "SymbolTable") {
    return null;
  }
  const selectedBinding = discoverTagBindings(snapshot, symbolTable)
    .find((candidate) => candidate.key === draft.bindingKey);
  if (selectedBinding === undefined) {
    return null;
  }
  const semanticPayload = createTagSemanticPayload(
    draft,
    currentValidation,
    selectedBinding.blockId,
    selectedBinding.memberId,
  );
  if (semanticPayload === null) {
    return null;
  }
  const objectId = idFactory();
  if (!CANONICAL_UUID.test(objectId)) {
    return null;
  }
  return {
    displayName: draft.name,
    kind: "project.create-object",
    objectId,
    objectKind: "tag",
    parentId: symbolTable.id,
    payloadSchema: "edu.tag/1",
    presentationPayload: {},
    semanticPayload,
  };
};

/** Finds active LAD organization blocks under the tag table's controller. */
export const discoverLadTagPrograms = (
  snapshot: WorkbenchSnapshot,
  symbolTable: WorkbenchObjectView,
): readonly TagProgramOption[] => {
  const controllerId = controllerAncestor(snapshot, symbolTable);
  if (controllerId === null) {
    return [];
  }
  return Object.values(snapshot.objects)
    .filter((candidate) =>
      candidate.lifecycle === "active" &&
      candidate.kind === "OB" &&
      candidate.semanticPayload.language === "LAD" &&
      Array.isArray(candidate.semanticPayload.interface) &&
      isDescendantOf(snapshot, candidate, controllerId)
    )
    .map((candidate) => ({ id: candidate.id, name: candidate.displayName }))
    .sort((left, right) =>
      left.name.localeCompare(right.name, "en-US") || left.id.localeCompare(right.id, "en-US")
    );
};

export const createTagWithMemberDraftDefaults = (
  programId: string | null = null,
  area: TagAddressArea = "I",
): TagWithMemberCreationDraft => {
  const tag = createTagDraftDefaults(area);
  return {
    addressIntent: tag.addressIntent,
    addressText: tag.addressText,
    area: tag.area,
    dataType: tag.dataType,
    description: tag.description,
    name: tag.name,
    programId,
  };
};

export const validateTagWithMemberCreation = (
  draft: TagWithMemberCreationDraft,
  snapshot: WorkbenchSnapshot,
  symbolTable: WorkbenchObjectView,
): TagConfigurationValidation => {
  const tagDraft = withBinding(draft, null);
  const { errors, parsedAddress } = validateTagFields(tagDraft, snapshot, symbolTable.id, null);
  const programs = discoverLadTagPrograms(snapshot, symbolTable);
  if (draft.programId === null || !programs.some((program) => program.id === draft.programId)) {
    errors.binding = "Choose a LAD program in this controller for the new variable.";
  }
  return { errors, parsedAddress, valid: Object.keys(errors).length === 0 };
};

/**
 * Adds one valid temporary LAD interface member, then creates the complete tag
 * bound to it. The first committed state merely has an unused member; the
 * second connects it, so both intermediate and final project states are valid.
 */
export const buildTagWithMemberCreationPlan = (
  draft: TagWithMemberCreationDraft,
  validation: TagConfigurationValidation,
  snapshot: WorkbenchSnapshot,
  symbolTable: WorkbenchObjectView,
  idFactory: () => string = defaultIdFactory,
): TagWithMemberCreationPlan | null => {
  const currentValidation = validateTagWithMemberCreation(draft, snapshot, symbolTable);
  if (
    !validation.valid ||
    !currentValidation.valid ||
    symbolTable.kind !== "SymbolTable" ||
    draft.programId === null
  ) {
    return null;
  }
  const program = snapshot.objects[draft.programId];
  const allowed = discoverLadTagPrograms(snapshot, symbolTable)
    .some((candidate) => candidate.id === draft.programId);
  const currentInterface = program?.semanticPayload.interface;
  if (!allowed || program === undefined || !Array.isArray(currentInterface)) {
    return null;
  }

  const memberId = idFactory();
  const tagId = idFactory();
  if (!CANONICAL_UUID.test(memberId) || !CANONICAL_UUID.test(tagId)) {
    return null;
  }
  const member = interfaceMemberPayload(
    memberId,
    draft.name,
    draft.dataType,
    nextInterfaceOrder(currentInterface),
  );
  const tagDraft = withBinding(draft, bindingKey(program.id, memberId));
  const semanticPayload = createTagSemanticPayload(
    tagDraft,
    currentValidation,
    program.id,
    memberId,
  );
  if (semanticPayload === null) {
    return null;
  }
  return {
    memberId,
    operations: [
      {
        key: "interface",
        kind: "project.set-semantic-field",
        objectId: program.id,
        value: [...currentInterface, member],
      },
      {
        displayName: draft.name,
        kind: "project.create-object",
        objectId: tagId,
        objectKind: "tag",
        parentId: symbolTable.id,
        payloadSchema: "edu.tag/1",
        presentationPayload: {},
        semanticPayload,
      },
    ],
    tagId,
  };
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

const validateTagFields = (
  draft: TagConfigurationDraft,
  snapshot: WorkbenchSnapshot,
  symbolTableId: string | null,
  excludedTagId: string | null,
): Readonly<{
  errors: Partial<Record<keyof TagConfigurationErrors, string>>;
  parsedAddress: ParsedTagAddress | null;
}> => {
  const errors: Partial<Record<keyof TagConfigurationErrors, string>> = {};
  if (!PLC_IDENTIFIER.test(draft.name)) {
    errors.name = "Use 1–128 letters, digits, or underscores, starting with a letter or underscore.";
  } else if (hasTagName(snapshot, symbolTableId, excludedTagId, draft.name)) {
    errors.name = "Another tag in this symbol table already uses this name.";
  }

  if (draft.description.length > TAG_DESCRIPTION_MAX_LENGTH) {
    errors.description = `Keep the description to ${TAG_DESCRIPTION_MAX_LENGTH} characters or fewer.`;
  }

  const areaIsValid = (tagAddressAreas as readonly string[]).includes(draft.area);
  if (!areaIsValid) {
    errors.area = "Choose Input (I), Output (Q), or Memory (M).";
  } else {
    const admittedTypes = dataTypesForTagArea(draft.area) as readonly string[];
    if (!admittedTypes.includes(draft.dataType)) {
      errors.dataType = draft.area === "M"
        ? "Choose a canonical scalar PLC type."
        : "Physical I/O currently supports BOOL digital channels and INT analog channels.";
    }
  }

  let parsedAddress: ParsedTagAddress | null = null;
  if (draft.area === "M" && draft.addressIntent !== "auto") {
    errors.address = "Memory tags are allocated in program memory and must use automatic allocation.";
  } else if (draft.addressIntent === "explicit" && areaIsValid) {
    const parsed = parseManualTagAddress(draft.addressText, draft.area, draft.dataType);
    if (!parsed.ok) {
      errors.address = parsed.error;
    } else {
      parsedAddress = parsed.value;
      const conflict = explicitAddressConflict(
        snapshot,
        symbolTableId,
        excludedTagId,
        draft.area,
        draft.dataType,
        parsedAddress,
      );
      if (conflict !== null) {
        errors.address = `That manual address overlaps ${conflict}. Choose another I/O address.`;
      }
    }
  }

  return { errors, parsedAddress };
};

const hasTagName = (
  snapshot: WorkbenchSnapshot,
  symbolTableId: string | null,
  excludedTagId: string | null,
  name: string,
): boolean => Object.values(snapshot.objects).some((candidate) =>
  candidate.id !== excludedTagId &&
  candidate.lifecycle === "active" &&
  candidate.kind === "Tag" &&
  candidate.parentId === symbolTableId &&
  candidate.displayName.toLocaleLowerCase("en-US") === name.toLocaleLowerCase("en-US")
);

const explicitAddressConflict = (
  snapshot: WorkbenchSnapshot,
  symbolTableId: string | null,
  excludedTagId: string | null,
  area: TagAddressArea,
  dataType: string,
  address: ParsedTagAddress,
): string | null => {
  if (area === "M") {
    return null;
  }
  const requested = addressSpan(dataType, address.byteOffset, address.bitOffset);
  if (requested === null) {
    return null;
  }
  for (const candidate of Object.values(snapshot.objects)) {
    if (
      candidate.id === excludedTagId ||
      candidate.lifecycle !== "active" ||
      candidate.kind !== "Tag" ||
      candidate.parentId !== symbolTableId ||
      candidate.semanticPayload.addressArea !== area ||
      candidate.semanticPayload.addressIntent !== "explicit"
    ) {
      continue;
    }
    const byteOffset = canonicalUnsigned(candidate.semanticPayload.byteOffset);
    const bitOffset = canonicalUnsigned(candidate.semanticPayload.bitOffset) ?? 0;
    const candidateType = text(candidate.semanticPayload.dataType)?.toLocaleUpperCase("en-US") ?? "";
    const occupied = byteOffset === null ? null : addressSpan(candidateType, byteOffset, bitOffset);
    if (
      occupied !== null &&
      requested.firstBit <= occupied.lastBit &&
      occupied.firstBit <= requested.lastBit
    ) {
      return `“${candidate.displayName}”`;
    }
  }
  return null;
};

const addressSpan = (
  dataType: string,
  byteOffset: number,
  bitOffset: number,
): Readonly<{ firstBit: number; lastBit: number }> | null => {
  if (dataType === "BOOL" && bitOffset <= 7) {
    const firstBit = byteOffset * 8 + bitOffset;
    return { firstBit, lastBit: firstBit };
  }
  if (dataType === "INT") {
    const firstBit = byteOffset * 8;
    return { firstBit, lastBit: firstBit + 15 };
  }
  return null;
};

const createTagSemanticPayload = (
  draft: TagConfigurationDraft,
  validation: TagConfigurationValidation,
  blockId: string | null,
  memberId: string | null,
  base: ProjectPayload = {},
): ProjectPayload | null => {
  if (blockId === null || memberId === null) {
    return null;
  }
  const payload: Record<string, ProjectPayloadValue> = {
    ...base,
    addressArea: draft.area,
    addressIntent: draft.area === "M" ? "auto" : draft.addressIntent,
    blockId,
    comment: draft.description,
    dataType: draft.dataType,
    memberId,
    tagKind: tagKindForArea(draft.area),
  };
  if (draft.addressIntent === "explicit" && draft.area !== "M") {
    if (validation.parsedAddress === null) {
      return null;
    }
    payload.byteOffset = unsignedPayload(validation.parsedAddress.byteOffset);
    payload.bitOffset = unsignedPayload(validation.parsedAddress.bitOffset);
  } else {
    delete payload.byteOffset;
    delete payload.bitOffset;
  }
  return payload;
};

const withBinding = (
  draft: TagWithMemberCreationDraft,
  selectedBindingKey: string | null,
): TagConfigurationDraft => ({
  addressIntent: draft.addressIntent,
  addressText: draft.addressText,
  area: draft.area,
  bindingKey: selectedBindingKey,
  dataType: draft.dataType,
  description: draft.description,
  name: draft.name,
});

const interfaceMemberPayload = (
  id: string,
  name: string,
  dataType: string,
  order: number,
): ProjectPayloadValue => ({
  $type: "record",
  value: {
    id,
    name,
    order: unsignedPayload(order),
    requiredOutput: false,
    retentive: false,
    role: "temp",
    type: dataType,
  },
});

const nextInterfaceOrder = (members: readonly ProjectPayloadValue[]): number => {
  let maximum = -1;
  for (const member of members) {
    const order = recordFields(member);
    const parsed = order === null ? null : canonicalUnsigned(order.order);
    if (parsed !== null && parsed > maximum) {
      maximum = parsed;
    }
  }
  const next = maximum + 1;
  return Number.isSafeInteger(next) ? next : members.length;
};

const unsignedPayload = (value: number): ProjectPayloadValue => ({
  $type: "u64",
  value: value.toString(10),
});
