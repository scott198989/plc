import {
  canonicalRecordFields,
  recordValue,
  unsignedValue,
} from "./canonical-authoring";
import {
  getMvpLadInstruction,
  ladInstructionStateRequirement,
} from "./lad-instruction-catalog";
import type {
  CanonicalLadBoxStateBinding,
  MvpLadInstructionKey,
} from "./lad-instruction-catalog";
import type {
  ProjectPayloadValue,
  WorkbenchObjectView,
  WorkbenchOperation,
  WorkbenchSnapshot,
} from "./workbench-types";

export const LAD_STATE_DB_ROLE = "lad-instruction-state";

export type LadStateStoragePlan = Readonly<{
  dataBlockId: string;
  memberId: string;
  memberName: string;
  operations: readonly WorkbenchOperation[];
  stateBinding: CanonicalLadBoxStateBinding;
}>;

export type LadStateStorageBatchEntry = Readonly<{
  instruction: MvpLadInstructionKey | number;
  memberId: string;
  memberName: string;
  stateBinding: CanonicalLadBoxStateBinding;
}>;

export type LadStateStorageBatchPlan = Readonly<{
  dataBlockId: string;
  entries: readonly LadStateStorageBatchEntry[];
  operations: readonly WorkbenchOperation[];
}>;

/**
 * Creates one private, persistent instruction-state member for a LAD box.
 * The state data block is reused per program and remains an ordinary project
 * object so save/open, undo, diagnostics, and teacher review see the truth.
 */
export const createLadStateStoragePlan = (
  snapshot: WorkbenchSnapshot,
  program: WorkbenchObjectView,
  instruction: MvpLadInstructionKey | number,
  idFactory: () => string = () => crypto.randomUUID(),
): LadStateStoragePlan => {
  const batch = createLadStateStorageBatchPlan(snapshot, program, [instruction], idFactory);
  const entry = batch.entries[0];
  if (entry === undefined) {
    throw new Error("The LAD state storage planner did not allocate the requested member.");
  }
  return {
    dataBlockId: batch.dataBlockId,
    memberId: entry.memberId,
    memberName: entry.memberName,
    operations: batch.operations,
    stateBinding: entry.stateBinding,
  };
};

/**
 * Allocates several private instruction-state members in one data-block
 * mutation. This is required when a whole rung is duplicated: planning each
 * box independently from the same snapshot would make later replacements
 * overwrite earlier members.
 */
export const createLadStateStorageBatchPlan = (
  snapshot: WorkbenchSnapshot,
  program: WorkbenchObjectView,
  instructions: readonly (MvpLadInstructionKey | number)[],
  idFactory: () => string = () => crypto.randomUUID(),
): LadStateStorageBatchPlan => {
  if (instructions.length === 0) {
    throw new Error("At least one stateful LAD instruction is required.");
  }
  const requested = instructions.map((instruction) => {
    const definition = getMvpLadInstruction(instruction);
    const requirement = ladInstructionStateRequirement(instruction);
    if (requirement === null) {
      throw new Error(`${definition.mnemonic} does not require persistent state.`);
    }
    return { definition, instruction, requirement };
  });
  const controllerId = program.parentId;
  if (controllerId === null || snapshot.objects[controllerId]?.kind !== "Controller") {
    throw new Error("The LAD program must belong directly to one virtual PLC.");
  }
  const dataBlock = Object.values(snapshot.objects).find((candidate) =>
    candidate.lifecycle === "active" &&
    candidate.kind === "GlobalDB" &&
    candidate.parentId === controllerId &&
    candidate.semanticPayload.educationRole === LAD_STATE_DB_ROLE &&
    candidate.semanticPayload.ownerProgramId === program.id
  );
  const dataBlockId = dataBlock?.id ?? idFactory();
  const members: ProjectPayloadValue[] = dataBlock === undefined
    ? []
    : Array.isArray(dataBlock.semanticPayload.members)
      ? [...dataBlock.semanticPayload.members]
      : [];
  const entries = requested.map(({ definition, instruction, requirement }) => {
    const memberId = idFactory();
    const memberName = nextStateMemberName(
      members,
      `${definition.mnemonic}_${requirement.suggestedNameSuffix}`,
    );
    members.push(recordValue({
      id: memberId,
      name: memberName,
      order: unsignedValue(members.length),
      requiredOutput: false,
      retentive: false,
      role: "static",
      type: requirement.dataType,
    }));
    return {
      instruction,
      memberId,
      memberName,
      stateBinding: {
        storage: {
          dataBlockId,
          kind: "data-block-member" as const,
          memberId,
        },
      },
    };
  });
  const operations: WorkbenchOperation[] = dataBlock === undefined
    ? [{
        displayName: `${program.displayName} instances`,
        kind: "project.create-object",
        objectId: dataBlockId,
        objectKind: "data-block",
        parentId: controllerId,
        payloadSchema: "edu.data-block/1",
        presentationPayload: {},
        semanticPayload: {
          dbKind: "GlobalDB",
          educationRole: LAD_STATE_DB_ROLE,
          engineeringNumber: unsignedValue(nextDataBlockNumber(snapshot)),
          members,
          ownerProgramId: program.id,
        },
      }]
    : [{
        kind: "project.replace-semantic-payload",
        objectId: dataBlock.id,
        semanticPayload: {
          ...dataBlock.semanticPayload,
          members,
        },
      }];
  return {
    dataBlockId,
    entries,
    operations,
  };
};

const nextStateMemberName = (
  members: readonly ProjectPayloadValue[],
  baseName: string,
): string => {
  const names = new Set(members.flatMap((value) => {
    const fields = canonicalRecordFields(value);
    return fields !== null && typeof fields.name === "string"
      ? [fields.name.toLocaleLowerCase("en-US")]
      : [];
  }));
  for (let suffix = 1; suffix <= 9_999; suffix += 1) {
    const candidate = `${baseName}_${suffix}`;
    if (!names.has(candidate.toLocaleLowerCase("en-US"))) {
      return candidate;
    }
  }
  throw new Error("The LAD instance data block cannot allocate another readable state name.");
};

const nextDataBlockNumber = (snapshot: WorkbenchSnapshot): number => {
  let maximum = 0;
  for (const object of Object.values(snapshot.objects)) {
    if (
      object.lifecycle !== "active" ||
      (object.kind !== "GlobalDB" && object.kind !== "InstanceDB")
    ) {
      continue;
    }
    const number = canonicalUnsigned(object.semanticPayload.engineeringNumber);
    if (number !== null) {
      maximum = Math.max(maximum, number);
    }
  }
  return Math.min(maximum + 1, 4_294_967_295);
};

const canonicalUnsigned = (value: ProjectPayloadValue | undefined): number | null => {
  if (
    typeof value !== "object" ||
    value === null ||
    Array.isArray(value) ||
    !("$type" in value) ||
    value.$type !== "u64"
  ) {
    return null;
  }
  const parsed = Number(value.value);
  return Number.isSafeInteger(parsed) && parsed >= 0 ? parsed : null;
};
