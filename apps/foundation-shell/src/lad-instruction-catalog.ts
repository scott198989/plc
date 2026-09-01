import {
  recordValue,
  unsignedValue,
} from "./canonical-authoring";
import type { PlcScalarTypeToken } from "./canonical-authoring";
import type { ProjectPayloadValue } from "./workbench-types";

/** The immutable Rust registry version mirrored by this learner-facing subset. */
export const LAD_INSTRUCTION_REGISTRY_VERSION = "EDU-INSTRUCTION-REGISTRY-2.2.0";

/** Stable codes from `plc_program::phase2_instruction_registry`. */
export const LAD_INSTRUCTION_CODES = {
  ADD: 0x0030,
  COMPARE_EQ: 0x0020,
  COMPARE_GE: 0x0025,
  COMPARE_GT: 0x0024,
  COMPARE_LE: 0x0023,
  COMPARE_LT: 0x0022,
  COMPARE_NE: 0x0021,
  COUNTER_DOWN: 0x0121,
  COUNTER_UP: 0x0120,
  COUNTER_UP_DOWN: 0x0122,
  DIVIDE: 0x0033,
  FALLING_EDGE: 0x0101,
  MOVE: 0x0002,
  MULTIPLY: 0x0032,
  RISING_EDGE: 0x0100,
  SUBTRACT: 0x0031,
  TIMER_OFF_DELAY: 0x0111,
  TIMER_ON_DELAY: 0x0110,
  TIMER_PULSE: 0x0112,
} as const;

/** Stable formal identities from `plc_program::phase2_instruction_registry`. */
export const LAD_FORMAL_IDS = {
  CLOCK: 0x0030,
  COUNT_DOWN: 0x0041,
  COUNT_UP: 0x0040,
  CURRENT_VALUE: 0x0045,
  ELAPSED_TIME: 0x0032,
  ENABLE: 0x0001,
  ENABLE_OUTPUT: 0x0002,
  INPUT: 0x0010,
  LEFT: 0x0020,
  LOAD: 0x0043,
  OUTPUT: 0x0011,
  PRESET_TIME: 0x0031,
  PRESET_VALUE: 0x0044,
  QD: 0x0047,
  QU: 0x0046,
  RESET: 0x0042,
  RIGHT: 0x0021,
  STATE: 0x00ff,
} as const;

export type MvpLadInstructionKey =
  | "move"
  | "compare-eq"
  | "compare-ne"
  | "compare-lt"
  | "compare-le"
  | "compare-gt"
  | "compare-ge"
  | "add"
  | "subtract"
  | "multiply"
  | "divide"
  | "rising-edge"
  | "falling-edge"
  | "ton"
  | "tof"
  | "tp"
  | "ctu"
  | "ctd"
  | "ctud";

export type MvpLadInstructionGroup =
  | "Move"
  | "Compare"
  | "Math"
  | "Edges"
  | "Timers"
  | "Counters";

export type LadInstructionStateKind = "edge" | "timer" | "counter";
export type LadInstructionStateDataType = "EDGESTATE" | "TIMERSTATE" | "COUNTERSTATE";
export type LadInstructionPinDirection =
  | "activation"
  | "input"
  | "output"
  | "status"
  | "state";
export type LadInstructionLvalue = "value" | "writable" | "read-write";
export type LadInstructionFormalSurface = "power-flow" | "data-pin" | "state-binding";

export type LadInstructionTypeConstraint =
  | Readonly<{ kind: "exact"; dataType: PlcScalarTypeToken }>
  | Readonly<{ kind: "any-value" }>
  | Readonly<{ kind: "numeric" }>
  | Readonly<{ formalId: number; kind: "same-as" }>
  | Readonly<{ kind: "instruction-state"; stateKind: LadInstructionStateKind }>;

export type MvpLadInstructionFormal = Readonly<{
  direction: LadInstructionPinDirection;
  id: number;
  lvalue: LadInstructionLvalue;
  name: string;
  required: boolean;
  surface: LadInstructionFormalSurface;
  typeConstraint: LadInstructionTypeConstraint;
}>;

export type MvpLadInstructionLearning = Readonly<{
  plainLanguage: string;
  summary: string;
  tip: string;
  title: string;
}>;

export type MvpLadInstructionDefinition = Readonly<{
  code: number;
  formals: readonly MvpLadInstructionFormal[];
  group: MvpLadInstructionGroup;
  key: MvpLadInstructionKey;
  learning: MvpLadInstructionLearning;
  mnemonic: string;
  stateKind: LadInstructionStateKind | null;
}>;

export type LadInstructionStateRequirement = Readonly<{
  dataType: LadInstructionStateDataType;
  explanation: string;
  memberRole: "static";
  stateKind: LadInstructionStateKind;
  suggestedNameSuffix: string;
}>;

export type CanonicalLadStateStorage =
  | Readonly<{ kind: "caller-member"; memberId: string }>
  | Readonly<{
      dataBlockId: string;
      kind: "data-block-member";
      memberId: string;
    }>;

export type CanonicalLadBoxStateBinding = Readonly<{
  invocationId?: string;
  storage: CanonicalLadStateStorage;
}>;

/** A binding template; omitted operand IDs are allocated with the box IDs. */
export type CanonicalLadPinBinding =
  | Readonly<{ id?: string; kind: "caller-member"; memberId: string }>
  | Readonly<{
      dataBlockId: string;
      id?: string;
      kind: "data-block-member";
      memberId: string;
    }>
  | Readonly<{
      dataType?: PlcScalarTypeToken;
      id?: string;
      kind: "constant";
      value: ProjectPayloadValue;
    }>
  | Readonly<{ id?: string; kind: "unresolved"; spelling: string }>
  | Readonly<{ id?: string; kind: "expression"; source: string }>;

export type BuildCanonicalLadBoxNodeRequest = Readonly<{
  bindings?: Readonly<Record<string, CanonicalLadPinBinding | null | undefined>>;
  idFactory?: () => string;
  instruction: MvpLadInstructionKey | number;
  semanticOrder: number;
  stateBinding?: CanonicalLadBoxStateBinding;
  valueDataType?: PlcScalarTypeToken;
}>;

export type CanonicalLadBoxNodeBuild = Readonly<{
  createdIds: readonly string[];
  inputPowerPortId: string;
  node: ProjectPayloadValue;
  nodeId: string;
  outputPowerPortId: string;
  pinIds: Readonly<Record<string, string>>;
  stateRequirement: LadInstructionStateRequirement | null;
}>;

export type BuildCanonicalLadBoxNodeResult = CanonicalLadBoxNodeBuild;

const exact = (dataType: PlcScalarTypeToken): LadInstructionTypeConstraint => ({
  dataType,
  kind: "exact",
});
const sameAs = (formalId: number): LadInstructionTypeConstraint => ({ formalId, kind: "same-as" });
const instructionState = (
  stateKind: LadInstructionStateKind,
): LadInstructionTypeConstraint => ({ kind: "instruction-state", stateKind });

const formal = (
  id: number,
  name: string,
  direction: LadInstructionPinDirection,
  typeConstraint: LadInstructionTypeConstraint,
  required: boolean,
  lvalue: LadInstructionLvalue,
  surface: LadInstructionFormalSurface = "data-pin",
): MvpLadInstructionFormal => ({
  direction,
  id,
  lvalue,
  name,
  required,
  surface,
  typeConstraint,
});

const ENABLE = formal(
  LAD_FORMAL_IDS.ENABLE,
  "EN",
  "activation",
  exact("BOOL"),
  false,
  "value",
  "power-flow",
);
const ENABLE_OUTPUT = formal(
  LAD_FORMAL_IDS.ENABLE_OUTPUT,
  "ENO",
  "status",
  exact("BOOL"),
  false,
  "writable",
  "power-flow",
);
const INPUT = formal(LAD_FORMAL_IDS.INPUT, "IN", "input", { kind: "any-value" }, true, "value");
const OUTPUT_SAME_AS_INPUT = formal(
  LAD_FORMAL_IDS.OUTPUT,
  "OUT",
  "output",
  sameAs(LAD_FORMAL_IDS.INPUT),
  true,
  "writable",
);
const LEFT_ANY = formal(LAD_FORMAL_IDS.LEFT, "A", "input", { kind: "any-value" }, true, "value");
const LEFT_NUMERIC = formal(
  LAD_FORMAL_IDS.LEFT,
  "A",
  "input",
  { kind: "numeric" },
  true,
  "value",
);
const RIGHT_SAME_AS_LEFT = formal(
  LAD_FORMAL_IDS.RIGHT,
  "B",
  "input",
  sameAs(LAD_FORMAL_IDS.LEFT),
  true,
  "value",
);
const BOOL_OUTPUT = formal(
  LAD_FORMAL_IDS.OUTPUT,
  "OUT",
  "output",
  exact("BOOL"),
  true,
  "writable",
);
const NUMERIC_OUTPUT = formal(
  LAD_FORMAL_IDS.OUTPUT,
  "OUT",
  "output",
  sameAs(LAD_FORMAL_IDS.LEFT),
  true,
  "writable",
);

const stateFormal = (stateKind: LadInstructionStateKind): MvpLadInstructionFormal => formal(
  LAD_FORMAL_IDS.STATE,
  "STATE",
  "state",
  instructionState(stateKind),
  true,
  "read-write",
  "state-binding",
);

const EDGE_FORMALS = [
  ENABLE,
  formal(LAD_FORMAL_IDS.CLOCK, "CLK", "input", exact("BOOL"), true, "value"),
  formal(LAD_FORMAL_IDS.OUTPUT, "Q", "output", exact("BOOL"), true, "writable"),
  ENABLE_OUTPUT,
  stateFormal("edge"),
] as const;

const TIMER_FORMALS = [
  ENABLE,
  formal(LAD_FORMAL_IDS.INPUT, "IN", "input", exact("BOOL"), true, "value"),
  formal(LAD_FORMAL_IDS.PRESET_TIME, "PT", "input", exact("TIME"), true, "value"),
  formal(LAD_FORMAL_IDS.OUTPUT, "Q", "output", exact("BOOL"), true, "writable"),
  formal(
    LAD_FORMAL_IDS.ELAPSED_TIME,
    "ET",
    "output",
    exact("TIME"),
    true,
    "writable",
  ),
  ENABLE_OUTPUT,
  stateFormal("timer"),
] as const;

const COUNTER_UP_FORMALS = [
  ENABLE,
  formal(LAD_FORMAL_IDS.COUNT_UP, "CU", "input", exact("BOOL"), true, "value"),
  formal(LAD_FORMAL_IDS.RESET, "R", "input", exact("BOOL"), true, "value"),
  formal(LAD_FORMAL_IDS.PRESET_VALUE, "PV", "input", exact("DINT"), true, "value"),
  formal(LAD_FORMAL_IDS.OUTPUT, "Q", "output", exact("BOOL"), true, "writable"),
  formal(
    LAD_FORMAL_IDS.CURRENT_VALUE,
    "CV",
    "output",
    exact("DINT"),
    true,
    "writable",
  ),
  ENABLE_OUTPUT,
  stateFormal("counter"),
] as const;

const COUNTER_DOWN_FORMALS = [
  ENABLE,
  formal(LAD_FORMAL_IDS.COUNT_DOWN, "CD", "input", exact("BOOL"), true, "value"),
  formal(LAD_FORMAL_IDS.LOAD, "LD", "input", exact("BOOL"), true, "value"),
  formal(LAD_FORMAL_IDS.PRESET_VALUE, "PV", "input", exact("DINT"), true, "value"),
  formal(LAD_FORMAL_IDS.OUTPUT, "Q", "output", exact("BOOL"), true, "writable"),
  formal(
    LAD_FORMAL_IDS.CURRENT_VALUE,
    "CV",
    "output",
    exact("DINT"),
    true,
    "writable",
  ),
  ENABLE_OUTPUT,
  stateFormal("counter"),
] as const;

const COUNTER_UP_DOWN_FORMALS = [
  ENABLE,
  formal(LAD_FORMAL_IDS.COUNT_UP, "CU", "input", exact("BOOL"), true, "value"),
  formal(LAD_FORMAL_IDS.COUNT_DOWN, "CD", "input", exact("BOOL"), true, "value"),
  formal(LAD_FORMAL_IDS.RESET, "R", "input", exact("BOOL"), true, "value"),
  formal(LAD_FORMAL_IDS.LOAD, "LD", "input", exact("BOOL"), true, "value"),
  formal(LAD_FORMAL_IDS.PRESET_VALUE, "PV", "input", exact("DINT"), true, "value"),
  formal(LAD_FORMAL_IDS.QU, "QU", "output", exact("BOOL"), true, "writable"),
  formal(LAD_FORMAL_IDS.QD, "QD", "output", exact("BOOL"), true, "writable"),
  formal(
    LAD_FORMAL_IDS.CURRENT_VALUE,
    "CV",
    "output",
    exact("DINT"),
    true,
    "writable",
  ),
  ENABLE_OUTPUT,
  stateFormal("counter"),
] as const;

const definition = (
  key: MvpLadInstructionKey,
  code: number,
  mnemonic: string,
  group: MvpLadInstructionGroup,
  stateKind: LadInstructionStateKind | null,
  formals: readonly MvpLadInstructionFormal[],
  title: string,
  summary: string,
  plainLanguage: string,
  tip: string,
): MvpLadInstructionDefinition => ({
  code,
  formals,
  group,
  key,
  learning: { plainLanguage, summary, tip, title },
  mnemonic,
  stateKind,
});

const move = definition(
  "move",
  LAD_INSTRUCTION_CODES.MOVE,
  "MOVE",
  "Move",
  null,
  [ENABLE, INPUT, OUTPUT_SAME_AS_INPUT, ENABLE_OUTPUT],
  "Move a value",
  "Copies one value into another tag.",
  "When power reaches MOVE, the value at IN is written to OUT.",
  "IN and OUT must use the same data type.",
);

const comparison = (
  key: Extract<MvpLadInstructionKey, `compare-${string}`>,
  code: number,
  mnemonic: string,
  title: string,
  plainLanguage: string,
): MvpLadInstructionDefinition => definition(
  key,
  code,
  mnemonic,
  "Compare",
  null,
  [ENABLE, LEFT_ANY, RIGHT_SAME_AS_LEFT, BOOL_OUTPUT, ENABLE_OUTPUT],
  title,
  "Compares two values and produces a true or false result.",
  plainLanguage,
  "A and B must use the same data type.",
);

const math = (
  key: "add" | "subtract" | "multiply" | "divide",
  code: number,
  mnemonic: string,
  title: string,
  plainLanguage: string,
  tip: string,
): MvpLadInstructionDefinition => definition(
  key,
  code,
  mnemonic,
  "Math",
  null,
  [ENABLE, LEFT_NUMERIC, RIGHT_SAME_AS_LEFT, NUMERIC_OUTPUT, ENABLE_OUTPUT],
  title,
  "Calculates a numeric result from A and B.",
  plainLanguage,
  tip,
);

const edge = (
  key: "rising-edge" | "falling-edge",
  code: number,
  mnemonic: string,
  title: string,
  plainLanguage: string,
): MvpLadInstructionDefinition => definition(
  key,
  code,
  mnemonic,
  "Edges",
  "edge",
  EDGE_FORMALS,
  title,
  "Produces a one-scan pulse when a Boolean signal changes.",
  plainLanguage,
  "Edge instructions need their own EDGESTATE memory and must not share it.",
);

const timer = (
  key: "ton" | "tof" | "tp",
  code: number,
  mnemonic: string,
  title: string,
  plainLanguage: string,
): MvpLadInstructionDefinition => definition(
  key,
  code,
  mnemonic,
  "Timers",
  "timer",
  TIMER_FORMALS,
  title,
  "Uses elapsed time to control a Boolean output.",
  plainLanguage,
  "PT is the preset time; ET shows elapsed time. Each timer needs its own TIMERSTATE memory.",
);

const counter = (
  key: "ctu" | "ctd" | "ctud",
  code: number,
  mnemonic: string,
  formals: readonly MvpLadInstructionFormal[],
  title: string,
  plainLanguage: string,
): MvpLadInstructionDefinition => definition(
  key,
  code,
  mnemonic,
  "Counters",
  "counter",
  formals,
  title,
  "Counts Boolean input transitions and reports a DINT current value.",
  plainLanguage,
  "PV is the preset value; CV is the current value. Each counter needs its own COUNTERSTATE memory.",
);

/**
 * The learner-facing LAD MVP. Ordering follows the stable registry code so
 * menus and serialized choices remain deterministic.
 */
export const MVP_LAD_INSTRUCTION_CATALOG: readonly MvpLadInstructionDefinition[] = [
  move,
  comparison("compare-eq", LAD_INSTRUCTION_CODES.COMPARE_EQ, "EQ", "Equal", "Q is true when A equals B."),
  comparison("compare-ne", LAD_INSTRUCTION_CODES.COMPARE_NE, "NE", "Not equal", "Q is true when A does not equal B."),
  comparison("compare-lt", LAD_INSTRUCTION_CODES.COMPARE_LT, "LT", "Less than", "Q is true when A is less than B."),
  comparison("compare-le", LAD_INSTRUCTION_CODES.COMPARE_LE, "LE", "Less than or equal", "Q is true when A is less than or equal to B."),
  comparison("compare-gt", LAD_INSTRUCTION_CODES.COMPARE_GT, "GT", "Greater than", "Q is true when A is greater than B."),
  comparison("compare-ge", LAD_INSTRUCTION_CODES.COMPARE_GE, "GE", "Greater than or equal", "Q is true when A is greater than or equal to B."),
  math("add", LAD_INSTRUCTION_CODES.ADD, "ADD", "Add", "OUT is A plus B.", "A, B, and OUT must use the same numeric type."),
  math("subtract", LAD_INSTRUCTION_CODES.SUBTRACT, "SUB", "Subtract", "OUT is A minus B.", "A, B, and OUT must use the same numeric type."),
  math("multiply", LAD_INSTRUCTION_CODES.MULTIPLY, "MUL", "Multiply", "OUT is A multiplied by B.", "A, B, and OUT must use the same numeric type."),
  math("divide", LAD_INSTRUCTION_CODES.DIVIDE, "DIV", "Divide", "OUT is A divided by B.", "Never allow B to be zero; division by zero creates a runtime fault."),
  edge("rising-edge", LAD_INSTRUCTION_CODES.RISING_EDGE, "R_TRIG", "Rising edge", "Q is true for one scan when CLK changes from false to true."),
  edge("falling-edge", LAD_INSTRUCTION_CODES.FALLING_EDGE, "F_TRIG", "Falling edge", "Q is true for one scan when CLK changes from true to false."),
  timer("ton", LAD_INSTRUCTION_CODES.TIMER_ON_DELAY, "TON", "On-delay timer", "Q turns on after IN has stayed true for PT."),
  timer("tof", LAD_INSTRUCTION_CODES.TIMER_OFF_DELAY, "TOF", "Off-delay timer", "Q stays on for PT after IN turns false."),
  timer("tp", LAD_INSTRUCTION_CODES.TIMER_PULSE, "TP", "Pulse timer", "A rising edge at IN turns Q on for exactly PT."),
  counter("ctu", LAD_INSTRUCTION_CODES.COUNTER_UP, "CTU", COUNTER_UP_FORMALS, "Count up", "Each rising edge at CU adds one; R clears the count."),
  counter("ctd", LAD_INSTRUCTION_CODES.COUNTER_DOWN, "CTD", COUNTER_DOWN_FORMALS, "Count down", "Each rising edge at CD subtracts one; LD loads PV."),
  counter("ctud", LAD_INSTRUCTION_CODES.COUNTER_UP_DOWN, "CTUD", COUNTER_UP_DOWN_FORMALS, "Count up/down", "CU adds, CD subtracts, R clears, and LD loads PV."),
];

const instructionByKey = new Map(
  MVP_LAD_INSTRUCTION_CATALOG.map((value) => [value.key, value] as const),
);
const instructionByCode = new Map(
  MVP_LAD_INSTRUCTION_CATALOG.map((value) => [value.code, value] as const),
);

export const findMvpLadInstruction = (
  instruction: MvpLadInstructionKey | number,
): MvpLadInstructionDefinition | null => (
  typeof instruction === "number"
    ? (instructionByCode.get(instruction) ?? null)
    : (instructionByKey.get(instruction) ?? null)
);

export const getMvpLadInstruction = (
  instruction: MvpLadInstructionKey | number,
): MvpLadInstructionDefinition => {
  const definition = findMvpLadInstruction(instruction);
  if (definition === null) {
    throw new RangeError(`Instruction '${instruction}' is not in the learner LAD catalog.`);
  }
  return definition;
};

export const ladInstructionStateRequirement = (
  instruction: MvpLadInstructionKey | number,
): LadInstructionStateRequirement | null => {
  const stateKind = getMvpLadInstruction(instruction).stateKind;
  if (stateKind === null) {
    return null;
  }
  switch (stateKind) {
    case "edge":
      return {
        dataType: "EDGESTATE",
        explanation: "Stores the previous CLK value so a one-scan edge can be detected.",
        memberRole: "static",
        stateKind,
        suggestedNameSuffix: "EdgeState",
      };
    case "timer":
      return {
        dataType: "TIMERSTATE",
        explanation: "Stores timing progress between PLC scans.",
        memberRole: "static",
        stateKind,
        suggestedNameSuffix: "TimerState",
      };
    case "counter":
      return {
        dataType: "COUNTERSTATE",
        explanation: "Stores the count and input-edge history between PLC scans.",
        memberRole: "static",
        stateKind,
        suggestedNameSuffix: "CounterState",
      };
  }
};

const NUMERIC_TYPES = new Set<PlcScalarTypeToken>([
  "SINT",
  "INT",
  "DINT",
  "LINT",
  "USINT",
  "UINT",
  "UDINT",
  "ULINT",
  "REAL",
  "LREAL",
]);

/**
 * Builds the canonical payload for one LAD `box` node. The node is always
 * structurally decodable. Bindings may be left null while the learner chooses
 * tags; stateful instructions expose the exact memory requirement alongside
 * the node and accept a complete state binding when one is available.
 */
export const buildCanonicalLadBoxNode = (
  request: BuildCanonicalLadBoxNodeRequest,
): CanonicalLadBoxNodeBuild => {
  const selected = getMvpLadInstruction(request.instruction);
  if (!Number.isSafeInteger(request.semanticOrder) || request.semanticOrder < 0 || request.semanticOrder > 0xffff_ffff) {
    throw new RangeError("LAD semanticOrder must be an unsigned 32-bit integer.");
  }
  const allocator = identityAllocator(request.idFactory);
  const nodeId = allocator.take();
  const inputPowerPortId = allocator.take();
  const outputPowerPortId = allocator.take();
  const resolvedTypes = new Map<number, PlcScalarTypeToken>();
  const pinIds: Record<string, string> = {};
  const pins = selected.formals
    .filter((value) => value.surface === "data-pin")
    .map((value) => {
      const dataType = resolveFormalDataType(value, resolvedTypes, request.valueDataType);
      resolvedTypes.set(value.id, dataType);
      const id = allocator.take();
      pinIds[value.name] = id;
      return recordValue({
        binding: canonicalBindingPayload(
          request.bindings?.[value.name] ?? null,
          dataType,
          allocator,
        ),
        dataType,
        direction: value.direction,
        formalId: unsignedValue(value.id),
        formalKind: "instruction",
        id,
        name: value.name,
        required: value.required,
        status: "active",
      });
    });

  const stateRequirement = ladInstructionStateRequirement(selected.key);
  let state: ProjectPayloadValue = null;
  if (stateRequirement === null) {
    if (request.stateBinding !== undefined) {
      throw new Error(`${selected.mnemonic} is stateless and cannot accept a state binding.`);
    }
  } else if (request.stateBinding !== undefined) {
    const invocationId = request.stateBinding.invocationId === undefined
      ? allocator.take()
      : allocator.claim(request.stateBinding.invocationId, "state invocation");
    state = recordValue({
      invocationId,
      stateKind: stateRequirement.stateKind,
      storage: stateStoragePayload(request.stateBinding.storage),
    });
  }

  return {
    createdIds: allocator.created(),
    inputPowerPortId,
    node: recordValue({
      id: nodeId,
      instructionCode: unsignedValue(selected.code),
      nodeKind: "box",
      pins,
      powerPorts: [
        recordValue({ direction: "input", id: inputPowerPortId }),
        recordValue({ direction: "output", id: outputPowerPortId }),
      ],
      semanticOrder: unsignedValue(request.semanticOrder),
      state,
    }),
    nodeId,
    outputPowerPortId,
    pinIds,
    stateRequirement,
  };
};

const canonicalBindingPayload = (
  binding: CanonicalLadPinBinding | null,
  pinDataType: PlcScalarTypeToken,
  allocator: IdentityAllocator,
): ProjectPayloadValue => {
  if (binding === null) {
    return null;
  }
  const id = binding.id === undefined
    ? allocator.take()
    : allocator.claim(binding.id, "LAD operand");
  switch (binding.kind) {
    case "caller-member":
      assertIdentity(binding.memberId, "caller member");
      return recordValue({ id, kind: binding.kind, memberId: binding.memberId });
    case "data-block-member":
      assertIdentity(binding.dataBlockId, "operand data block");
      assertIdentity(binding.memberId, "data-block member");
      return recordValue({
        dataBlockId: binding.dataBlockId,
        id,
        kind: binding.kind,
        memberId: binding.memberId,
      });
    case "constant":
      if (binding.dataType !== undefined && binding.dataType !== pinDataType) {
        throw new Error(
          `The ${binding.dataType} constant does not match its ${pinDataType} LAD pin.`,
        );
      }
      return recordValue({
        dataType: pinDataType,
        id,
        kind: binding.kind,
        value: binding.value,
      });
    case "unresolved":
      return recordValue({ id, kind: binding.kind, spelling: binding.spelling });
    case "expression":
      return recordValue({ id, kind: binding.kind, source: binding.source });
  }
};

const resolveFormalDataType = (
  formal: MvpLadInstructionFormal,
  resolved: ReadonlyMap<number, PlcScalarTypeToken>,
  selected: PlcScalarTypeToken | undefined,
): PlcScalarTypeToken => {
  switch (formal.typeConstraint.kind) {
    case "exact":
      return formal.typeConstraint.dataType;
    case "any-value":
      return selected ?? "DINT";
    case "numeric": {
      const dataType = selected ?? "DINT";
      if (!NUMERIC_TYPES.has(dataType)) {
        throw new Error(`${dataType} is not a numeric LAD instruction type.`);
      }
      return dataType;
    }
    case "same-as": {
      const dataType = resolved.get(formal.typeConstraint.formalId);
      if (dataType === undefined) {
        throw new Error(`Formal ${formal.name} refers to an unresolved earlier type binding.`);
      }
      return dataType;
    }
    case "instruction-state":
      throw new Error("Instruction state is projected through the box state binding, not a data pin.");
  }
};

const stateStoragePayload = (storage: CanonicalLadStateStorage): ProjectPayloadValue => {
  assertIdentity(storage.memberId, "state member");
  if (storage.kind === "caller-member") {
    return recordValue({ kind: storage.kind, memberId: storage.memberId });
  }
  assertIdentity(storage.dataBlockId, "state data block");
  return recordValue({
    dataBlockId: storage.dataBlockId,
    kind: storage.kind,
    memberId: storage.memberId,
  });
};

const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/iu;
const MAX_ID_ATTEMPTS = 32;

type IdentityAllocator = Readonly<{
  claim: (value: string, label: string) => string;
  created: () => readonly string[];
  take: () => string;
}>;

const assertIdentity = (value: string, label: string): void => {
  if (!UUID_PATTERN.test(value)) {
    throw new Error(`The ${label} identity is not a canonical UUID.`);
  }
};

const identityAllocator = (
  factory: () => string = () => crypto.randomUUID(),
): IdentityAllocator => {
  const created: string[] = [];
  const used = new Set<string>();
  const claim = (value: string, label: string): string => {
    assertIdentity(value, label);
    const normalized = value.toLocaleLowerCase("en-US");
    if (used.has(normalized)) {
      throw new Error(`The ${label} identity is duplicated inside one LAD box.`);
    }
    used.add(normalized);
    created.push(value);
    return value;
  };
  return {
    claim,
    created: () => [...created],
    take: () => {
      for (let attempt = 0; attempt < MAX_ID_ATTEMPTS; attempt += 1) {
        const candidate = factory();
        if (UUID_PATTERN.test(candidate) && !used.has(candidate.toLocaleLowerCase("en-US"))) {
          return claim(candidate, "generated LAD");
        }
      }
      throw new Error("A unique canonical LAD identity could not be allocated.");
    },
  };
};
