import { recordValue, unsignedValue } from "./canonical-authoring";
import {
  inspectAssignmentDocument,
  type AssignmentDocumentV1,
} from "./education-contract";
import {
  createDigitalModulePayload,
  digitalModuleCatalog,
} from "./hardware-configuration";
import {
  createPlcSetupPlan,
  virtualPlcCatalog,
  type VirtualPlcCatalogId,
} from "./plc-setup";
import {
  parseManualTagAddress,
  TAG_DESCRIPTION_MAX_LENGTH,
  tagKindForArea,
  type TagAddressArea,
} from "./tag-configuration";
import type {
  ProjectPayload,
  ProjectPayloadValue,
  WorkbenchOperation,
  WorkbenchSnapshot,
} from "./workbench-types";

export type AssignmentStarterPlanErrorCode =
  | "duplicate-identity"
  | "embedded-project"
  | "invalid-assignment"
  | "invalid-identity"
  | "invalid-starter-tag"
  | "module-capacity"
  | "project-not-empty"
  | "unsupported-controller"
  | "unsupported-module"
  | "unsupported-template";

export class AssignmentStarterPlanError extends Error {
  public readonly code: AssignmentStarterPlanErrorCode;

  public constructor(code: AssignmentStarterPlanErrorCode, message: string) {
    super(message);
    this.name = "AssignmentStarterPlanError";
    this.code = code;
  }
}

export type AssignmentStarterPlan = Readonly<{
  catalogId: VirtualPlcCatalogId;
  controllerId: string;
  operations: readonly WorkbenchOperation[];
  programId: string;
  rackId: string;
  symbolTableId: string;
}>;

type IdFactory = () => string;
type CreateOperation = Extract<WorkbenchOperation, Readonly<{ kind: "project.create-object" }>>;

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/iu;
const PLC_IDENTIFIER = /^[A-Za-z_][A-Za-z0-9_]{0,127}$/u;
const BUILT_IN_BLANK_TEMPLATE = "builtin.virtual-plc-blank/1";

/**
 * Converts a validated assignment into ordinary canonical workbench commands.
 * The caller applies the returned operations through the same transaction path
 * as learner-authored changes; this helper never mutates the snapshot.
 */
export const createAssignmentStarterPlan = (
  snapshot: WorkbenchSnapshot,
  assignment: AssignmentDocumentV1,
  idFactory: IdFactory = () => crypto.randomUUID(),
): AssignmentStarterPlan => {
  assertAssignment(assignment);
  assertSupportedStarter(assignment);
  assertEmptyProject(snapshot);

  const catalog = assignment.requirements.plcCatalogIds
    .map((catalogId) => virtualPlcCatalog.find((candidate) => candidate.catalogId === catalogId))
    .find((candidate) => candidate !== undefined);
  if (catalog === undefined) {
    throw new AssignmentStarterPlanError(
      "unsupported-controller",
      `This assignment does not offer a supported virtual PLC. Supported catalog IDs: ${virtualPlcCatalog.map((item) => item.catalogId).join(", ")}.`,
    );
  }

  const nextId = freshIdFactory(snapshot, idFactory);
  const setup = createPlcSetupPlan(snapshot, catalog.catalogId, nextId);
  const programCreate = setup.operations.find((operation): operation is CreateOperation =>
    operation.kind === "project.create-object" && operation.objectId === setup.programId
  );
  if (programCreate === undefined || !Array.isArray(programCreate.semanticPayload.interface)) {
    throw new AssignmentStarterPlanError(
      "invalid-assignment",
      "The local MainCycle template is unavailable or does not expose a canonical LAD interface.",
    );
  }

  const moduleOperations = createModuleOperations(
    assignment,
    catalog.firstExpansionSlot,
    catalog.lastSlot,
    setup.rackId,
    nextId,
  );
  const tagPlan = createStarterTags(
    assignment,
    catalog.inputBytes,
    catalog.outputBytes,
    programCreate.semanticPayload.interface,
    setup.programId,
    setup.symbolTableId,
    nextId,
  );
  const setupOperations = setup.operations.map((operation) =>
    operation === programCreate
      ? { ...programCreate, semanticPayload: { ...programCreate.semanticPayload, interface: tagPlan.interface } }
      : operation
  );

  return {
    catalogId: catalog.catalogId,
    controllerId: setup.controllerId,
    operations: [...setupOperations, ...moduleOperations, ...tagPlan.operations],
    programId: setup.programId,
    rackId: setup.rackId,
    symbolTableId: setup.symbolTableId,
  };
};

const assertAssignment = (assignment: AssignmentDocumentV1): void => {
  const result = inspectAssignmentDocument(assignment);
  if (!result.ok) {
    const first = result.issues[0];
    throw new AssignmentStarterPlanError(
      "invalid-assignment",
      first === undefined
        ? "The assignment document is invalid."
        : `The assignment document is invalid at ${first.path}: ${first.message}`,
    );
  }
};

const assertSupportedStarter = (assignment: AssignmentDocumentV1): void => {
  if (assignment.starterProject.kind === "embedded-project") {
    throw new AssignmentStarterPlanError(
      "embedded-project",
      "Embedded assignment projects must be verified and opened through the project-file workflow.",
    );
  }
  if (
    assignment.starterProject.kind === "built-in-template"
    && assignment.starterProject.templateId !== BUILT_IN_BLANK_TEMPLATE
  ) {
    throw new AssignmentStarterPlanError(
      "unsupported-template",
      `Built-in starter template '${assignment.starterProject.templateId}' is not available in this offline build.`,
    );
  }
};

const assertEmptyProject = (snapshot: WorkbenchSnapshot): void => {
  const root = snapshot.objects[snapshot.projectRootId];
  const activeNonRoot = Object.values(snapshot.objects).filter((object) =>
    object.lifecycle === "active" && object.id !== snapshot.projectRootId
  );
  if (root?.lifecycle !== "active" || root.kind !== "ProjectRoot" || activeNonRoot.length !== 0) {
    throw new AssignmentStarterPlanError(
      "project-not-empty",
      "Assignment starters can only be created in a new empty project.",
    );
  }
};

const freshIdFactory = (snapshot: WorkbenchSnapshot, source: IdFactory): IdFactory => {
  const used = new Set(Object.keys(snapshot.objects).map((id) => id.toLocaleLowerCase("en-US")));
  return () => {
    const id = source();
    if (!UUID.test(id)) {
      throw new AssignmentStarterPlanError("invalid-identity", `The identity factory returned an invalid UUID: '${id}'.`);
    }
    const key = id.toLocaleLowerCase("en-US");
    if (used.has(key)) {
      throw new AssignmentStarterPlanError("duplicate-identity", `The identity factory reused UUID '${id}'.`);
    }
    used.add(key);
    return id;
  };
};

const createModuleOperations = (
  assignment: AssignmentDocumentV1,
  firstSlot: number,
  lastSlot: number,
  rackId: string,
  nextId: IdFactory,
): readonly WorkbenchOperation[] => {
  const requestedCount = assignment.requirements.requiredModules.reduce(
    (total, module) => total + module.quantity,
    0,
  );
  const capacity = lastSlot - firstSlot + 1;
  if (!Number.isSafeInteger(requestedCount) || requestedCount > capacity) {
    throw new AssignmentStarterPlanError(
      "module-capacity",
      `The selected virtual PLC has ${capacity} expansion slots, but the assignment requires ${requestedCount} digital modules.`,
    );
  }

  const operations: WorkbenchOperation[] = [];
  let slot = firstSlot;
  for (const requirement of assignment.requirements.requiredModules) {
    const descriptor = digitalModuleCatalog(requirement.catalogId);
    if (descriptor === null) {
      throw new AssignmentStarterPlanError(
        "unsupported-module",
        `Required module catalog '${requirement.catalogId}' is not supported by this offline build.`,
      );
    }
    for (let index = 0; index < requirement.quantity; index += 1) {
      operations.push(createOperation(
        requirement.quantity === 1 ? descriptor.modelName : `${descriptor.modelName} ${index + 1}`,
        nextId(),
        "module",
        rackId,
        "edu.module/1",
        createDigitalModulePayload(descriptor.catalogId, slot),
      ));
      slot += 1;
    }
  }
  return operations;
};

const createStarterTags = (
  assignment: AssignmentDocumentV1,
  inputBytes: number,
  outputBytes: number,
  currentInterface: readonly ProjectPayloadValue[],
  programId: string,
  symbolTableId: string,
  nextId: IdFactory,
): Readonly<{
  interface: readonly ProjectPayloadValue[];
  operations: readonly WorkbenchOperation[];
}> => {
  const members: ProjectPayloadValue[] = [];
  const operations: WorkbenchOperation[] = [];
  const names = new Set<string>();
  const addresses = new Set<string>();

  assignment.requirements.starterTags.forEach((tag, index) => {
    const normalizedName = tag.name.toLocaleLowerCase("en-US");
    if (!PLC_IDENTIFIER.test(tag.name) || names.has(normalizedName)) {
      throw invalidTag(tag.name, "Use a unique PLC identifier of 1-128 letters, digits, or underscores.");
    }
    if (tag.description.length > TAG_DESCRIPTION_MAX_LENGTH) {
      throw invalidTag(tag.name, `Descriptions are limited to ${TAG_DESCRIPTION_MAX_LENGTH} characters.`);
    }
    names.add(normalizedName);

    const memberId = nextId();
    const tagId = nextId();
    members.push(recordValue({
      id: memberId,
      name: tag.name,
      order: unsignedValue(currentInterface.length + index),
      requiredOutput: false,
      retentive: false,
      role: "temp",
      type: tag.dataType,
    }));

    const payload: Record<string, ProjectPayloadValue> = {
      blockId: programId,
      comment: tag.description,
      dataType: tag.dataType,
      memberId,
    };
    if (tag.address === null) {
      payload.addressArea = "M";
      payload.addressIntent = "auto";
      payload.tagKind = tagKindForArea("M");
    } else {
      if (tag.dataType !== "BOOL") {
        throw invalidTag(tag.name, "Only BOOL starter tags can use an explicit I/O address; use no address for DINT or TIME memory.");
      }
      const area = explicitArea(tag.name, tag.address);
      const parsed = parseManualTagAddress(tag.address, area, "BOOL");
      if (!parsed.ok) {
        throw invalidTag(tag.name, parsed.error);
      }
      const capacity = area === "I" ? inputBytes : outputBytes;
      if (parsed.value.byteOffset >= capacity) {
        throw invalidTag(tag.name, `%${area}${parsed.value.byteOffset}.${parsed.value.bitOffset} is outside the selected PLC's ${capacity}-byte ${area} image.`);
      }
      const addressKey = `${area}:${parsed.value.byteOffset}:${parsed.value.bitOffset}`;
      if (addresses.has(addressKey)) {
        throw invalidTag(tag.name, `The address ${parsed.value.canonicalText} is already used by another starter tag.`);
      }
      addresses.add(addressKey);
      payload.addressArea = area;
      payload.addressIntent = "explicit";
      payload.bitOffset = unsignedValue(parsed.value.bitOffset);
      payload.byteOffset = unsignedValue(parsed.value.byteOffset);
      payload.tagKind = tagKindForArea(area);
    }

    operations.push(createOperation(
      tag.name,
      tagId,
      "tag",
      symbolTableId,
      "edu.tag/1",
      payload,
    ));
  });

  return { interface: [...currentInterface, ...members], operations };
};

const explicitArea = (name: string, address: string): Extract<TagAddressArea, "I" | "Q"> => {
  const match = /^%?([IQ])/iu.exec(address);
  if (match?.[1] === undefined) {
    throw invalidTag(name, "Explicit starter addresses must be BOOL input or output bits such as %I0.0 or %Q0.0.");
  }
  return match[1].toLocaleUpperCase("en-US") as Extract<TagAddressArea, "I" | "Q">;
};

const invalidTag = (name: string, reason: string): AssignmentStarterPlanError =>
  new AssignmentStarterPlanError("invalid-starter-tag", `Starter tag '${name}' is not supported: ${reason}`);

const createOperation = (
  displayName: string,
  objectId: string,
  objectKind: CreateOperation["objectKind"],
  parentId: string,
  payloadSchema: string,
  semanticPayload: ProjectPayload,
): CreateOperation => ({
  displayName,
  kind: "project.create-object",
  objectId,
  objectKind,
  parentId,
  payloadSchema,
  presentationPayload: {},
  semanticPayload,
});
