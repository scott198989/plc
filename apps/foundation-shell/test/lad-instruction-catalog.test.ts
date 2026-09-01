import { describe, expect, it } from "vitest";

import {
  canonicalRecordFields,
  signedValue,
} from "../src/canonical-authoring";
import {
  buildCanonicalLadBoxNode,
  findMvpLadInstruction,
  getMvpLadInstruction,
  LAD_FORMAL_IDS,
  LAD_INSTRUCTION_CODES,
  ladInstructionStateRequirement,
  MVP_LAD_INSTRUCTION_CATALOG,
} from "../src/lad-instruction-catalog";
import type {
  LadInstructionTypeConstraint,
  MvpLadInstructionDefinition,
} from "../src/lad-instruction-catalog";
import type { ProjectPayloadValue } from "../src/workbench-types";

describe("learner LAD instruction catalog", () => {
  it("mirrors the exact MVP instruction codes in deterministic registry order", () => {
    expect(MVP_LAD_INSTRUCTION_CATALOG.map(({ key, code, mnemonic }) => ({ code, key, mnemonic })))
      .toEqual([
        { code: 0x0002, key: "move", mnemonic: "MOVE" },
        { code: 0x0020, key: "compare-eq", mnemonic: "EQ" },
        { code: 0x0021, key: "compare-ne", mnemonic: "NE" },
        { code: 0x0022, key: "compare-lt", mnemonic: "LT" },
        { code: 0x0023, key: "compare-le", mnemonic: "LE" },
        { code: 0x0024, key: "compare-gt", mnemonic: "GT" },
        { code: 0x0025, key: "compare-ge", mnemonic: "GE" },
        { code: 0x0030, key: "add", mnemonic: "ADD" },
        { code: 0x0031, key: "subtract", mnemonic: "SUB" },
        { code: 0x0032, key: "multiply", mnemonic: "MUL" },
        { code: 0x0033, key: "divide", mnemonic: "DIV" },
        { code: 0x0100, key: "rising-edge", mnemonic: "R_TRIG" },
        { code: 0x0101, key: "falling-edge", mnemonic: "F_TRIG" },
        { code: 0x0110, key: "ton", mnemonic: "TON" },
        { code: 0x0111, key: "tof", mnemonic: "TOF" },
        { code: 0x0112, key: "tp", mnemonic: "TP" },
        { code: 0x0120, key: "ctu", mnemonic: "CTU" },
        { code: 0x0121, key: "ctd", mnemonic: "CTD" },
        { code: 0x0122, key: "ctud", mnemonic: "CTUD" },
      ]);
    expect(LAD_INSTRUCTION_CODES.TIMER_ON_DELAY).toBe(0x0110);
    expect(findMvpLadInstruction(LAD_INSTRUCTION_CODES.COUNTER_UP_DOWN)?.key).toBe("ctud");
    expect(findMvpLadInstruction(0xffff)).toBeNull();
    expect(() => getMvpLadInstruction(0xffff)).toThrow(/not in the learner LAD catalog/u);
  });

  it("projects every exact registry formal through power, data-pin, or state surfaces", () => {
    const expected = new Map<string, readonly string[]>([
      ["move", moveFormals()],
      ...["compare-eq", "compare-ne", "compare-lt", "compare-le", "compare-gt", "compare-ge"]
        .map((key) => [key, compareFormals()] as const),
      ...["add", "subtract", "multiply", "divide"]
        .map((key) => [key, mathFormals()] as const),
      ...["rising-edge", "falling-edge"]
        .map((key) => [key, edgeFormals()] as const),
      ...["ton", "tof", "tp"]
        .map((key) => [key, timerFormals()] as const),
      ["ctu", counterUpFormals()],
      ["ctd", counterDownFormals()],
      ["ctud", counterUpDownFormals()],
    ]);

    for (const instruction of MVP_LAD_INSTRUCTION_CATALOG) {
      expect(instruction.formals.map(formalSignature), instruction.key)
        .toEqual(expected.get(instruction.key));
      expect(new Set(instruction.formals.map((formal) => formal.id)).size, instruction.key)
        .toBe(instruction.formals.length);
      expect(instruction.learning.title.length).toBeGreaterThan(2);
      expect(instruction.learning.summary.length).toBeGreaterThan(10);
      expect(instruction.learning.plainLanguage.length).toBeGreaterThan(10);
      expect(instruction.learning.tip.length).toBeGreaterThan(10);
    }
  });

  it("describes the exact private state member required by edges, timers, and counters", () => {
    expect(ladInstructionStateRequirement("move")).toBeNull();
    expect(ladInstructionStateRequirement("rising-edge")).toMatchObject({
      dataType: "EDGESTATE",
      memberRole: "static",
      stateKind: "edge",
    });
    expect(ladInstructionStateRequirement("ton")).toMatchObject({
      dataType: "TIMERSTATE",
      memberRole: "static",
      stateKind: "timer",
    });
    expect(ladInstructionStateRequirement("ctud")).toMatchObject({
      dataType: "COUNTERSTATE",
      memberRole: "static",
      stateKind: "counter",
    });
  });
});

describe("canonical LAD box-node builder", () => {
  it("builds a complete TON box with canonical pins, power ports, operands, and timer state", () => {
    const built = buildCanonicalLadBoxNode({
      bindings: {
        ET: { kind: "caller-member", memberId: identity(13) },
        IN: { kind: "caller-member", memberId: identity(10) },
        PT: { kind: "constant", value: signedValue(250) },
        Q: { kind: "caller-member", memberId: identity(12) },
      },
      idFactory: deterministicIds(100),
      instruction: "ton",
      semanticOrder: 3,
      stateBinding: {
        storage: {
          dataBlockId: identity(20),
          kind: "data-block-member",
          memberId: identity(21),
        },
      },
    });
    const node = record(built.node);
    expect(node).toMatchObject({
      id: built.nodeId,
      instructionCode: unsigned(0x0110),
      nodeKind: "box",
      semanticOrder: unsigned(3),
    });
    expect(records(node.powerPorts)).toEqual([
      { direction: "input", id: built.inputPowerPortId },
      { direction: "output", id: built.outputPowerPortId },
    ]);

    const pins = records(node.pins);
    expect(pins.map((pin) => [pin.name, pin.formalId, pin.direction, pin.dataType])).toEqual([
      ["IN", unsigned(LAD_FORMAL_IDS.INPUT), "input", "BOOL"],
      ["PT", unsigned(LAD_FORMAL_IDS.PRESET_TIME), "input", "TIME"],
      ["Q", unsigned(LAD_FORMAL_IDS.OUTPUT), "output", "BOOL"],
      ["ET", unsigned(LAD_FORMAL_IDS.ELAPSED_TIME), "output", "TIME"],
    ]);
    expect(pins.every((pin) => pin.formalKind === "instruction" && pin.status === "active"))
      .toBe(true);
    expect(record(pins.find((pin) => pin.name === "IN")?.binding)).toMatchObject({
      kind: "caller-member",
      memberId: identity(10),
    });
    expect(record(pins.find((pin) => pin.name === "PT")?.binding)).toMatchObject({
      dataType: "TIME",
      kind: "constant",
      value: signedValue(250),
    });
    expect(record(node.state)).toMatchObject({
      stateKind: "timer",
      storage: expect.any(Object),
    });
    expect(record(record(node.state).storage)).toEqual({
      dataBlockId: identity(20),
      kind: "data-block-member",
      memberId: identity(21),
    });
    expect(built.stateRequirement?.dataType).toBe("TIMERSTATE");
    expect(built.createdIds).toHaveLength(12);
    expect(new Set(built.createdIds).size).toBe(built.createdIds.length);
  });

  it("resolves generic MOVE, comparison, and math pin types without inventing registry formals", () => {
    const move = buildCanonicalLadBoxNode({
      idFactory: deterministicIds(200),
      instruction: "move",
      semanticOrder: 0,
      valueDataType: "TIME",
    });
    expect(pinTypes(move.node)).toEqual({ IN: "TIME", OUT: "TIME" });
    expect(record(move.node).state).toBeNull();

    const compare = buildCanonicalLadBoxNode({
      idFactory: deterministicIds(300),
      instruction: "compare-ge",
      semanticOrder: 1,
      valueDataType: "REAL",
    });
    expect(pinTypes(compare.node)).toEqual({ A: "REAL", B: "REAL", OUT: "BOOL" });

    const add = buildCanonicalLadBoxNode({
      idFactory: deterministicIds(400),
      instruction: "add",
      semanticOrder: 2,
      valueDataType: "LINT",
    });
    expect(pinTypes(add.node)).toEqual({ A: "LINT", B: "LINT", OUT: "LINT" });
    expect(() => buildCanonicalLadBoxNode({
      idFactory: deterministicIds(500),
      instruction: "divide",
      semanticOrder: 0,
      valueDataType: "BOOL",
    })).toThrow(/not a numeric LAD instruction type/u);
  });

  it("fails closed on impossible state and identity combinations", () => {
    expect(() => buildCanonicalLadBoxNode({
      idFactory: deterministicIds(600),
      instruction: "move",
      semanticOrder: 0,
      stateBinding: {
        storage: { kind: "caller-member", memberId: identity(1) },
      },
    })).toThrow(/stateless/u);
    expect(() => buildCanonicalLadBoxNode({
      idFactory: () => "not-an-id",
      instruction: "ton",
      semanticOrder: 0,
    })).toThrow(/could not be allocated/u);
    expect(() => buildCanonicalLadBoxNode({
      idFactory: deterministicIds(700),
      instruction: "ton",
      semanticOrder: 0,
      stateBinding: {
        storage: { kind: "caller-member", memberId: "not-an-id" },
      },
    })).toThrow(/state member identity/u);
  });
});

const moveFormals = (): readonly string[] => [
  sig(0x0001, "EN", "activation", "exact:BOOL", false, "value", "power-flow"),
  sig(0x0010, "IN", "input", "any-value", true, "value", "data-pin"),
  sig(0x0011, "OUT", "output", "same-as:16", true, "writable", "data-pin"),
  sig(0x0002, "ENO", "status", "exact:BOOL", false, "writable", "power-flow"),
];

const compareFormals = (): readonly string[] => [
  sig(0x0001, "EN", "activation", "exact:BOOL", false, "value", "power-flow"),
  sig(0x0020, "A", "input", "any-value", true, "value", "data-pin"),
  sig(0x0021, "B", "input", "same-as:32", true, "value", "data-pin"),
  sig(0x0011, "OUT", "output", "exact:BOOL", true, "writable", "data-pin"),
  sig(0x0002, "ENO", "status", "exact:BOOL", false, "writable", "power-flow"),
];

const mathFormals = (): readonly string[] => [
  sig(0x0001, "EN", "activation", "exact:BOOL", false, "value", "power-flow"),
  sig(0x0020, "A", "input", "numeric", true, "value", "data-pin"),
  sig(0x0021, "B", "input", "same-as:32", true, "value", "data-pin"),
  sig(0x0011, "OUT", "output", "same-as:32", true, "writable", "data-pin"),
  sig(0x0002, "ENO", "status", "exact:BOOL", false, "writable", "power-flow"),
];

const edgeFormals = (): readonly string[] => [
  sig(0x0001, "EN", "activation", "exact:BOOL", false, "value", "power-flow"),
  sig(0x0030, "CLK", "input", "exact:BOOL", true, "value", "data-pin"),
  sig(0x0011, "Q", "output", "exact:BOOL", true, "writable", "data-pin"),
  sig(0x0002, "ENO", "status", "exact:BOOL", false, "writable", "power-flow"),
  sig(0x00ff, "STATE", "state", "instruction-state:edge", true, "read-write", "state-binding"),
];

const timerFormals = (): readonly string[] => [
  sig(0x0001, "EN", "activation", "exact:BOOL", false, "value", "power-flow"),
  sig(0x0010, "IN", "input", "exact:BOOL", true, "value", "data-pin"),
  sig(0x0031, "PT", "input", "exact:TIME", true, "value", "data-pin"),
  sig(0x0011, "Q", "output", "exact:BOOL", true, "writable", "data-pin"),
  sig(0x0032, "ET", "output", "exact:TIME", true, "writable", "data-pin"),
  sig(0x0002, "ENO", "status", "exact:BOOL", false, "writable", "power-flow"),
  sig(0x00ff, "STATE", "state", "instruction-state:timer", true, "read-write", "state-binding"),
];

const counterUpFormals = (): readonly string[] => counterFormals([
  sig(0x0040, "CU", "input", "exact:BOOL", true, "value", "data-pin"),
  sig(0x0042, "R", "input", "exact:BOOL", true, "value", "data-pin"),
], [sig(0x0011, "Q", "output", "exact:BOOL", true, "writable", "data-pin")]);

const counterDownFormals = (): readonly string[] => counterFormals([
  sig(0x0041, "CD", "input", "exact:BOOL", true, "value", "data-pin"),
  sig(0x0043, "LD", "input", "exact:BOOL", true, "value", "data-pin"),
], [sig(0x0011, "Q", "output", "exact:BOOL", true, "writable", "data-pin")]);

const counterUpDownFormals = (): readonly string[] => counterFormals([
  sig(0x0040, "CU", "input", "exact:BOOL", true, "value", "data-pin"),
  sig(0x0041, "CD", "input", "exact:BOOL", true, "value", "data-pin"),
  sig(0x0042, "R", "input", "exact:BOOL", true, "value", "data-pin"),
  sig(0x0043, "LD", "input", "exact:BOOL", true, "value", "data-pin"),
], [
  sig(0x0046, "QU", "output", "exact:BOOL", true, "writable", "data-pin"),
  sig(0x0047, "QD", "output", "exact:BOOL", true, "writable", "data-pin"),
]);

const counterFormals = (
  inputs: readonly string[],
  outputs: readonly string[],
): readonly string[] => [
  sig(0x0001, "EN", "activation", "exact:BOOL", false, "value", "power-flow"),
  ...inputs,
  sig(0x0044, "PV", "input", "exact:DINT", true, "value", "data-pin"),
  ...outputs,
  sig(0x0045, "CV", "output", "exact:DINT", true, "writable", "data-pin"),
  sig(0x0002, "ENO", "status", "exact:BOOL", false, "writable", "power-flow"),
  sig(0x00ff, "STATE", "state", "instruction-state:counter", true, "read-write", "state-binding"),
];

const formalSignature = (
  formal: MvpLadInstructionDefinition["formals"][number],
): string => sig(
  formal.id,
  formal.name,
  formal.direction,
  constraintSignature(formal.typeConstraint),
  formal.required,
  formal.lvalue,
  formal.surface,
);

const constraintSignature = (constraint: LadInstructionTypeConstraint): string => {
  switch (constraint.kind) {
    case "exact": return `exact:${constraint.dataType}`;
    case "same-as": return `same-as:${constraint.formalId}`;
    case "instruction-state": return `instruction-state:${constraint.stateKind}`;
    case "any-value":
    case "numeric": return constraint.kind;
  }
};

const sig = (
  id: number,
  name: string,
  direction: string,
  constraint: string,
  required: boolean,
  lvalue: string,
  surface: string,
): string => `${id}:${name}:${direction}:${constraint}:${required}:${lvalue}:${surface}`;

const pinTypes = (node: ProjectPayloadValue): Readonly<Record<string, unknown>> => Object.fromEntries(
  records(record(node).pins).map((pin) => [pin.name, pin.dataType]),
);

const record = (value: unknown): Record<string, ProjectPayloadValue> => {
  const fields = canonicalRecordFields(value as ProjectPayloadValue);
  if (fields === null) {
    throw new Error("Expected a canonical record value.");
  }
  return fields as Record<string, ProjectPayloadValue>;
};

const records = (value: unknown): readonly Record<string, ProjectPayloadValue>[] => {
  if (!Array.isArray(value)) {
    throw new Error("Expected a canonical record list.");
  }
  return value.map(record);
};

const unsigned = (value: number): Readonly<{ $type: "u64"; value: string }> => ({
  $type: "u64",
  value: value.toString(10),
});

const identity = (value: number): string => {
  const suffix = value.toString(16).padStart(12, "0");
  return `a0000000-0000-4000-8000-${suffix}`;
};

const deterministicIds = (start: number): (() => string) => {
  let value = start;
  return () => {
    const result = identity(value);
    value += 1;
    return result;
  };
};
