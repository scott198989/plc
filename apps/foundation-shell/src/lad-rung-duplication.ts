import {
  canonicalRecordFields,
  recordValue,
} from "./canonical-authoring";
import {
  duplicateLadNetwork,
} from "./lad-authoring";
import {
  findMvpLadInstruction,
  ladInstructionStateRequirement,
} from "./lad-instruction-catalog";
import {
  createLadStateStorageBatchPlan,
} from "./lad-state-storage";
import type {
  ProjectPayload,
  ProjectPayloadValue,
  WorkbenchObjectView,
  WorkbenchOperation,
  WorkbenchSnapshot,
} from "./workbench-types";

export type SafeLadRungDuplicationRequest = Readonly<{
  idFactory?: () => string;
  networkId: string;
}>;

export type SafeLadRungDuplicationResult =
  | Readonly<{
      createdIds: readonly string[];
      createdNetworkId: string;
      graph: ProjectPayloadValue;
      ok: true;
      /** Ordered transaction: state DB creation/replacement first, graph last. */
      operations: readonly WorkbenchOperation[];
      stateMemberIds: readonly string[];
    }>
  | Readonly<{
      code: "invalid-graph" | "invalid-stateful-rung" | "state-storage-plan-failed" | string;
      message: string;
      ok: false;
    }>;

/**
 * Plans a compile-safe rung duplicate as one ordered UI transaction.
 *
 * Graph-owned IDs are remapped by `duplicateLadNetwork`. Every copied edge,
 * timer, or counter box then receives a new typed state member. All members
 * are appended in one DB operation so several boxes cannot overwrite one
 * another when they are planned from the same snapshot.
 */
export const createSafeLadRungDuplicationPlan = (
  snapshot: WorkbenchSnapshot,
  program: WorkbenchObjectView,
  graph: ProjectPayloadValue,
  request: SafeLadRungDuplicationRequest,
): SafeLadRungDuplicationResult => {
  const originalNetworks = networkRecords(graph);
  if (originalNetworks === null) {
    return failure("invalid-graph", "The LAD graph does not contain canonical rungs.");
  }
  const duplicated = duplicateLadNetwork(graph, request);
  if (duplicated.ok === false) {
    return duplicated;
  }
  const duplicatedNetworks = networkRecords(duplicated.graph);
  if (duplicatedNetworks === null) {
    return failure("invalid-graph", "The duplicated LAD graph is malformed.");
  }
  const originalIds = new Set(originalNetworks.map((network) => identityField(network, "id")));
  const newNetworks = duplicatedNetworks.filter((network) => !originalIds.has(identityField(network, "id")));
  const copiedNetwork = newNetworks[0];
  const createdNetworkId = copiedNetwork === undefined ? null : identityField(copiedNetwork, "id");
  if (newNetworks.length !== 1 || copiedNetwork === undefined || createdNetworkId === null) {
    return failure("invalid-graph", "The newly duplicated LAD rung could not be identified uniquely.");
  }
  if (!isPayloadList(copiedNetwork.nodes)) {
    return failure("invalid-graph", "The duplicated LAD rung does not contain canonical nodes.");
  }

  const statefulBoxes: Array<Readonly<{
    instructionCode: number;
    nodeIndex: number;
    state: ProjectPayload;
  }>> = [];
  for (const [nodeIndex, nodeValue] of copiedNetwork.nodes.entries()) {
    const node = canonicalRecordFields(nodeValue);
    if (node === null || node.nodeKind !== "box") {
      continue;
    }
    const instructionCode = canonicalUnsignedNumber(node.instructionCode);
    const instruction = instructionCode === null ? null : findMvpLadInstruction(instructionCode);
    const state = canonicalRecordFields(node.state);
    if (instruction === null) {
      if (state !== null) {
        return failure(
          "invalid-stateful-rung",
          "A copied stateful box is not part of the supported learner instruction catalog.",
        );
      }
      continue;
    }
    const requirement = ladInstructionStateRequirement(instruction.key);
    if (requirement === null) {
      if (state !== null) {
        return failure(
          "invalid-stateful-rung",
          `${instruction.mnemonic} is stateless but contains persistent state metadata.`,
        );
      }
      continue;
    }
    if (
      state === null ||
      state.stateKind !== requirement.stateKind ||
      identityField(state, "invocationId") === null
    ) {
      return failure(
        "invalid-stateful-rung",
        `${instruction.mnemonic} does not contain a complete ${requirement.stateKind} state binding.`,
      );
    }
    statefulBoxes.push({ instructionCode: instruction.code, nodeIndex, state });
  }

  if (statefulBoxes.length === 0) {
    const graphOperation = replaceProgramGraphOperation(program.id, duplicated.graph);
    return {
      createdIds: duplicated.createdIds,
      createdNetworkId,
      graph: duplicated.graph,
      ok: true,
      operations: [graphOperation],
      stateMemberIds: [],
    };
  }

  let storagePlan;
  try {
    storagePlan = createLadStateStorageBatchPlan(
      snapshot,
      program,
      statefulBoxes.map((box) => box.instructionCode),
      request.idFactory,
    );
  } catch (caught) {
    return failure(
      "state-storage-plan-failed",
      caught instanceof Error ? caught.message : "Private LAD state storage could not be allocated.",
    );
  }

  const nodes = [...copiedNetwork.nodes];
  for (const [index, box] of statefulBoxes.entries()) {
    const node = canonicalRecordFields(nodes[box.nodeIndex]);
    const entry = storagePlan.entries[index];
    if (node === null || entry === undefined) {
      return failure("state-storage-plan-failed", "Not every copied stateful box received private storage.");
    }
    nodes[box.nodeIndex] = recordValue({
      ...node,
      state: recordValue({
        ...box.state,
        storage: recordValue(entry.stateBinding.storage),
      }),
    });
  }
  if (storagePlan.entries.length !== statefulBoxes.length) {
    return failure("state-storage-plan-failed", "Not every copied stateful box received private storage.");
  }

  const graphFields = canonicalRecordFields(duplicated.graph);
  if (graphFields === null || !isPayloadList(graphFields.networks)) {
    return failure("invalid-graph", "The duplicated LAD graph cannot accept the new state bindings.");
  }
  const updatedNetwork = recordValue({ ...copiedNetwork, nodes });
  const updatedGraph = recordValue({
    ...graphFields,
    networks: graphFields.networks.map((networkValue) => {
      const network = canonicalRecordFields(networkValue);
      return network !== null && identityField(network, "id") === createdNetworkId
        ? updatedNetwork
        : networkValue;
    }),
  });
  const graphOperation = replaceProgramGraphOperation(program.id, updatedGraph);
  return {
    createdIds: duplicated.createdIds,
    createdNetworkId,
    graph: updatedGraph,
    ok: true,
    operations: [...storagePlan.operations, graphOperation],
    stateMemberIds: storagePlan.entries.map((entry) => entry.memberId),
  };
};

const replaceProgramGraphOperation = (
  programId: string,
  graph: ProjectPayloadValue,
): WorkbenchOperation => ({
  key: "graph",
  kind: "project.set-semantic-field",
  objectId: programId,
  value: graph,
});

const networkRecords = (graph: ProjectPayloadValue): readonly ProjectPayload[] | null => {
  const fields = canonicalRecordFields(graph);
  if (fields === null || !isPayloadList(fields.networks)) {
    return null;
  }
  const networks = fields.networks.map(canonicalRecordFields);
  return networks.some((network) => network === null)
    ? null
    : networks.filter((network): network is ProjectPayload => network !== null);
};

const identityField = (fields: ProjectPayload, key: string): string | null => {
  const value = fields[key];
  return typeof value === "string" ? value : null;
};

const canonicalUnsignedNumber = (value: ProjectPayloadValue | undefined): number | null => {
  if (
    typeof value !== "object" ||
    value === null ||
    isPayloadList(value) ||
    !("$type" in value) ||
    value.$type !== "u64"
  ) {
    return null;
  }
  const parsed = Number(value.value);
  return Number.isSafeInteger(parsed) && parsed >= 0 ? parsed : null;
};

const isPayloadList = (
  value: ProjectPayloadValue | undefined,
): value is readonly ProjectPayloadValue[] => Array.isArray(value);

const failure = (
  code: Extract<SafeLadRungDuplicationResult, Readonly<{ ok: false }>>["code"],
  message: string,
): SafeLadRungDuplicationResult => ({ code, message, ok: false });
