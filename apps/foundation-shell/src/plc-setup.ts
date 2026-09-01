import { createLadProgramPayload, unsignedValue } from "./canonical-authoring";
import type {
  ProjectPayload,
  WorkbenchObjectView,
  WorkbenchOperation,
  WorkbenchSnapshot,
} from "./workbench-types";

export type VirtualPlcCatalogId = "vctrl-c1" | "vctrl-m1" | "vctrl-p1";

export type VirtualPlcCatalogOption = Readonly<{
  catalogId: VirtualPlcCatalogId;
  description: string;
  expansionSlots: number;
  firstExpansionSlot: number;
  inputBytes: number;
  label: string;
  lastSlot: number;
  outputBytes: number;
  recommended?: boolean;
  requiresPowerModule: boolean;
  shortLabel: string;
}>;

export const compactVirtualPlcCatalog: VirtualPlcCatalogOption = {
    catalogId: "vctrl-c1",
    description: "A simple all-in-one PLC for first circuits and small machines.",
    expansionSlots: 8,
    firstExpansionSlot: 1,
    inputBytes: 1_024,
    label: "Compact PLC",
    lastSlot: 8,
    outputBytes: 1_024,
    recommended: true,
    requiresPowerModule: false,
    shortLabel: "C1",
  };

export const virtualPlcCatalog: readonly VirtualPlcCatalogOption[] = [
  compactVirtualPlcCatalog,
  {
    catalogId: "vctrl-m1",
    description: "A modular rack with separate power and more room for I/O.",
    expansionSlots: 14,
    firstExpansionSlot: 2,
    inputBytes: 8_192,
    label: "Modular PLC",
    lastSlot: 15,
    outputBytes: 8_192,
    requiresPowerModule: true,
    shortLabel: "M1",
  },
  {
    catalogId: "vctrl-p1",
    description: "A larger virtual rack for advanced classroom projects.",
    expansionSlots: 30,
    firstExpansionSlot: 2,
    inputBytes: 32_768,
    label: "Performance PLC",
    lastSlot: 31,
    outputBytes: 32_768,
    requiresPowerModule: true,
    shortLabel: "P1",
  },
] as const;

export type PlcSetupPlan = Readonly<{
  controllerId: string;
  operations: readonly WorkbenchOperation[];
  programId: string;
  rackId: string;
  symbolTableId: string;
}>;

type IdFactory = () => string;

/**
 * Creates the ordinary canonical objects a learner needs before choosing I/O.
 * Modules remain intentionally absent so placement and addressing are learned,
 * not hidden inside another starter template.
 */
export const createPlcSetupPlan = (
  snapshot: WorkbenchSnapshot,
  catalogId: VirtualPlcCatalogId,
  idFactory: IdFactory = () => crypto.randomUUID(),
): PlcSetupPlan => {
  const catalog = virtualPlcCatalog.find((candidate) => candidate.catalogId === catalogId);
  if (catalog === undefined) {
    throw new Error(`Unsupported virtual PLC catalog: ${catalogId}`);
  }
  const controllerId = idFactory();
  const rackId = idFactory();
  const programId = idFactory();
  const symbolTableId = idFactory();
  const operations: WorkbenchOperation[] = [];
  const rootChildren = activeChildren(snapshot, snapshot.projectRootId);

  if (!rootChildren.some((object) => object.kind === "VirtualNetwork")) {
    operations.push(createOperation(
      "Virtual network",
      idFactory(),
      "network",
      snapshot.projectRootId,
      "edu.virtual-network/1",
      { configuredState: "enabled" },
    ));
  }

  operations.push(
    createOperation(
      "PLC_1",
      controllerId,
      "controller",
      snapshot.projectRootId,
      "edu.controller/1",
      {
        catalogId,
        profileId: "EDU-21 Core",
        profileVersion: "1.0.0",
      },
    ),
    createOperation(
      "Local rack",
      rackId,
      "rack",
      controllerId,
      "edu.rack/1",
      { slotCount: unsignedValue(catalog.requiresPowerModule ? catalog.lastSlot + 1 : catalog.expansionSlots) },
    ),
  );

  if (catalog.requiresPowerModule) {
    operations.push(createOperation(
      "Virtual power supply",
      idFactory(),
      "module",
      rackId,
      "edu.module/1",
      {
        addressIntent: "auto",
        catalogId: "vpwr1",
        slot: unsignedValue(0),
      },
    ));
  }

  operations.push(
    createOperation(
      "MainCycle",
      programId,
      "program-block",
      controllerId,
      "edu.program-block/1",
      createLadProgramPayload(1),
    ),
    createOperation(
      "PLC tags",
      symbolTableId,
      "symbol-table",
      controllerId,
      "edu.symbol-table/1",
      {},
    ),
  );

  return { controllerId, operations, programId, rackId, symbolTableId };
};

export const controllerCatalogOption = (
  controller: WorkbenchObjectView,
): VirtualPlcCatalogOption => {
  const catalogId = controller.semanticPayload.catalogId;
  return virtualPlcCatalog.find((candidate) => candidate.catalogId === catalogId) ?? compactVirtualPlcCatalog;
};

export const activeChildren = (
  snapshot: WorkbenchSnapshot,
  parentId: string,
): readonly WorkbenchObjectView[] => Object.values(snapshot.objects)
  .filter((object) => object.lifecycle === "active" && object.parentId === parentId)
  .sort((left, right) =>
    left.creationOrdinal.localeCompare(right.creationOrdinal) ||
    left.displayName.localeCompare(right.displayName, "en-US")
  );

const createOperation = (
  displayName: string,
  objectId: string,
  objectKind: Extract<WorkbenchOperation, Readonly<{ kind: "project.create-object" }>>["objectKind"],
  parentId: string,
  payloadSchema: string,
  semanticPayload: ProjectPayload,
): WorkbenchOperation => ({
  displayName,
  kind: "project.create-object",
  objectId,
  objectKind,
  parentId,
  payloadSchema,
  presentationPayload: {},
  semanticPayload,
});
