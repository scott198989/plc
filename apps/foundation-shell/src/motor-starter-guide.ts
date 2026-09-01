import { canonicalRecordFields } from "./canonical-authoring";
import type {
  LadNetworkTopology,
  LadTopologyElement,
  LadTopologyItem,
  LadTopologyParallel,
} from "./lad-topology";
import { projectLadNetworkTopology } from "./lad-topology";
import type { WorkbenchSnapshot } from "./workbench-types";

export type MotorStarterGuideMember = Readonly<{
  id: string;
  name: string;
  operandLabel?: string;
}>;

export type MotorStarterGuideProjection = Readonly<{
  available: boolean;
  complete: boolean;
  hasMotorCoil: boolean;
  hasSealInBranch: boolean;
  hasStopContact: boolean;
  motorMemberId: string | null;
  reason: string | null;
  startContactNodeId: string | null;
  startMemberId: string | null;
  stopInsertionEdgeId: string | null;
  stopMemberId: string | null;
}>;

/** Recognizes the beginner motor-starter exercise without changing its graph. */
export const projectMotorStarterGuide = (
  topology: LadNetworkTopology,
  members: readonly MotorStarterGuideMember[],
): MotorStarterGuideProjection => {
  const start = findNamedMember(members, "startpb");
  const stop = findNamedMember(members, "stoppb");
  const motor = findNamedMember(members, "motorrun");
  if (start === null || stop === null || motor === null) {
    return unavailable("The motor-starter coach appears when Start_PB, Stop_PB, and Motor_Run are available.");
  }

  const topLevel = topology.items;
  const topLevelStartIndex = topLevel.findIndex((item) =>
    item.kind === "element" && isContact(item, start.id, "normally-open")
  );
  const startContact = allElements(topLevel).find((item) =>
    isContact(item, start.id, "normally-open")
  ) ?? null;
  const sealBranchIndex = topLevel.findIndex((item) =>
    item.kind === "parallel" && isSealInBranch(item, start.id, motor.id)
  );
  const sealBranch = sealBranchIndex < 0
    ? allParallels(topLevel).find((item) => isSealInBranch(item, start.id, motor.id)) ?? null
    : topLevel[sealBranchIndex] as LadTopologyParallel;
  const targetIndex = sealBranchIndex >= 0 ? sealBranchIndex : topLevelStartIndex;
  const hasStopContact = targetIndex > 0 && topLevel
    .slice(0, targetIndex)
    .some((item) => item.kind === "element" && isContact(item, stop.id, "normally-closed"));
  const hasMotorCoil = allElements(topLevel).some((item) =>
    item.nodeKind === "coil" && memberId(item) === motor.id
  );

  return {
    available: startContact !== null && hasMotorCoil,
    complete: hasStopContact && sealBranch !== null && hasMotorCoil,
    hasMotorCoil,
    hasSealInBranch: sealBranch !== null,
    hasStopContact,
    motorMemberId: motor.id,
    reason: startContact === null
      ? "Add a normally open Start_PB contact to use the guided motor-starter exercise."
      : hasMotorCoil
        ? null
        : "Assign the rung coil to Motor_Run to use the guided motor-starter exercise.",
    startContactNodeId: startContact?.nodeId ?? null,
    startMemberId: start.id,
    stopInsertionEdgeId: startContact?.beforeEdgeId ?? null,
    stopMemberId: stop.id,
  };
};

/** Finds the canonical beginner motor-starter circuit anywhere in the active project. */
export const projectWorkbenchMotorStarterGuide = (
  snapshot: WorkbenchSnapshot,
): MotorStarterGuideProjection | null => {
  const programs = Object.values(snapshot.objects).filter((object) =>
    object.lifecycle === "active" &&
    object.kind === "OB" &&
    object.semanticPayload.language === "LAD"
  );

  for (const program of programs) {
    const graph = canonicalRecordFields(program.semanticPayload.graph);
    const network = graph !== null && Array.isArray(graph.networks)
      ? canonicalRecordFields(graph.networks[0])
      : null;
    const topology = network === null ? null : projectLadNetworkTopology(network);
    if (topology?.ok !== true || !Array.isArray(program.semanticPayload.interface)) {
      continue;
    }
    const members = program.semanticPayload.interface.flatMap((value) => {
      const member = canonicalRecordFields(value);
      return member !== null && typeof member.id === "string" && typeof member.name === "string"
        ? [{ id: member.id, name: member.name }]
        : [];
    });
    const projection = projectMotorStarterGuide(topology.topology, members);
    if (projection.available) {
      return projection;
    }
  }
  return null;
};

const unavailable = (reason: string): MotorStarterGuideProjection => ({
  available: false,
  complete: false,
  hasMotorCoil: false,
  hasSealInBranch: false,
  hasStopContact: false,
  motorMemberId: null,
  reason,
  startContactNodeId: null,
  startMemberId: null,
  stopInsertionEdgeId: null,
  stopMemberId: null,
});

const findNamedMember = (
  members: readonly MotorStarterGuideMember[],
  normalizedName: string,
): MotorStarterGuideMember | null => members.find((member) =>
  normalize(member.name) === normalizedName ||
  (member.operandLabel !== undefined && normalize(member.operandLabel) === normalizedName)
) ?? null;

const normalize = (value: string): string =>
  value.toLocaleLowerCase("en-US").replaceAll(/[^a-z0-9]/gu, "");

const memberId = (element: LadTopologyElement): string | null => {
  const operand = canonicalRecordFields(element.node.operand);
  return typeof operand?.memberId === "string" ? operand.memberId : null;
};

const isContact = (
  element: LadTopologyElement,
  expectedMemberId: string,
  expectedMode: "normally-closed" | "normally-open",
): boolean => element.nodeKind === "contact" &&
  element.node.mode === expectedMode &&
  memberId(element) === expectedMemberId;

const isSealInBranch = (
  parallel: LadTopologyParallel,
  startMemberId: string,
  motorMemberId: string,
): boolean => {
  const startPaths = new Set<number>();
  const motorPaths = new Set<number>();
  parallel.paths.forEach((path, index) => {
    const elements = allElements(path.items);
    if (elements.some((element) => isContact(element, startMemberId, "normally-open"))) {
      startPaths.add(index);
    }
    if (elements.some((element) => isContact(element, motorMemberId, "normally-open"))) {
      motorPaths.add(index);
    }
  });
  return [...startPaths].some((startPath) =>
    [...motorPaths].some((motorPath) => startPath !== motorPath)
  );
};

const allElements = (items: readonly LadTopologyItem[]): readonly LadTopologyElement[] =>
  items.flatMap((item): readonly LadTopologyElement[] => item.kind === "element"
    ? [item]
    : item.paths.flatMap((path) => allElements(path.items))
  );

const allParallels = (items: readonly LadTopologyItem[]): readonly LadTopologyParallel[] =>
  items.flatMap((item): readonly LadTopologyParallel[] => item.kind === "parallel"
    ? [item, ...item.paths.flatMap((path) => allParallels(path.items))]
    : []
  );
