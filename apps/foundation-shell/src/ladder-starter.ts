import {
  createStarterLadProgramPayload,
  interfaceMemberIdentity,
  unsignedValue,
} from "./canonical-authoring";
import type { ProjectPayload, WorkbenchOperation } from "./workbench-types";

export type LadderStarterPlan = Readonly<{
  operations: readonly WorkbenchOperation[];
  programId: string;
}>;

type IdFactory = () => string;

/**
 * Builds the smallest useful virtual motor-control lab. The ordered operations
 * are deliberately plain project commands so every created object remains
 * visible, editable, saveable, and undoable through the normal workbench.
 */
export const createLadderStarterPlan = (
  projectRootId: string,
  idFactory: IdFactory = () => crypto.randomUUID(),
): LadderStarterPlan => {
  const networkId = idFactory();
  const controllerId = idFactory();
  const rackId = idFactory();
  const inputModuleId = idFactory();
  const outputModuleId = idFactory();
  const programId = idFactory();
  const symbolTableId = idFactory();
  const startTagId = idFactory();
  const stopTagId = idFactory();
  const motorTagId = idFactory();
  const program = createStarterLadProgramPayload(1);
  const startMemberId = requiredMember(program, "Start_PB");
  const stopMemberId = requiredMember(program, "Stop_PB");
  const motorMemberId = requiredMember(program, "Motor_Run");

  const create = (
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

  return {
    operations: [
      create(
        "Virtual network",
        networkId,
        "network",
        projectRootId,
        "edu.virtual-network/1",
        { configuredState: "enabled" },
      ),
      create(
        "Controller",
        controllerId,
        "controller",
        projectRootId,
        "edu.controller/1",
        {
          catalogId: "vctrl-c1",
          profileId: "EDU-21 Core",
          profileVersion: "1.0.0",
        },
      ),
      create(
        "Local rack",
        rackId,
        "rack",
        controllerId,
        "edu.rack/1",
        { slotCount: unsignedValue(8) },
      ),
      create(
        "VDI16",
        inputModuleId,
        "module",
        rackId,
        "edu.module/1",
        { addressIntent: "auto", catalogId: "vdi16", slot: unsignedValue(1) },
      ),
      create(
        "VDO16",
        outputModuleId,
        "module",
        rackId,
        "edu.module/1",
        { addressIntent: "auto", catalogId: "vdo16", slot: unsignedValue(2) },
      ),
      create(
        "MainCycle",
        programId,
        "program-block",
        controllerId,
        "edu.program-block/1",
        program,
      ),
      create(
        "PLC tags",
        symbolTableId,
        "symbol-table",
        controllerId,
        "edu.symbol-table/1",
        {},
      ),
      create(
        "Start_PB",
        startTagId,
        "tag",
        symbolTableId,
        "edu.tag/1",
        tagPayload("I", "Input", programId, startMemberId),
      ),
      create(
        "Stop_PB",
        stopTagId,
        "tag",
        symbolTableId,
        "edu.tag/1",
        tagPayload("I", "Input", programId, stopMemberId),
      ),
      create(
        "Motor_Run",
        motorTagId,
        "tag",
        symbolTableId,
        "edu.tag/1",
        tagPayload("Q", "Output", programId, motorMemberId),
      ),
    ],
    programId,
  };
};

const tagPayload = (
  addressArea: "I" | "Q",
  tagKind: "Input" | "Output",
  blockId: string,
  memberId: string,
): ProjectPayload => ({
  addressArea,
  addressIntent: "auto",
  blockId,
  dataType: "BOOL",
  memberId,
  tagKind,
});

const requiredMember = (payload: ProjectPayload, name: string): string => {
  const memberId = interfaceMemberIdentity(payload, name);
  if (memberId === null) {
    throw new Error(`The starter LAD payload is missing ${name}.`);
  }
  return memberId;
};
