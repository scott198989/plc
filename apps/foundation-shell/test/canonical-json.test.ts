import { describe, expect, it } from "vitest";

import { encodeCanonicalJson } from "../src/canonical-json";

const text = (value: unknown): string => new TextDecoder().decode(encodeCanonicalJson(value));

describe("encodeCanonicalJson", () => {
  it("orders every record by ordinal key and emits compact UTF-8", () => {
    expect(text({ z: 1, a: { y: true, b: "PLC" }, list: [3, null] })).toBe(
      '{"a":{"b":"PLC","y":true},"list":[3,null],"z":1}',
    );
  });

  it("rejects numbers that cannot round-trip through JavaScript exactly", () => {
    expect(() => text({ revision: Number.MAX_SAFE_INTEGER + 1 })).toThrow(/safe integers/u);
    expect(() => text({ value: 0.25 })).toThrow(/safe integers/u);
  });

  it("rejects undefined, executable objects, and excessive depth", () => {
    expect(() => text({ missing: undefined })).toThrow(/undefined/u);
    expect(() => text(new Date())).toThrow(/plain records/u);
    let nested: unknown = "leaf";
    for (let index = 0; index < 66; index += 1) {
      nested = [nested];
    }
    expect(() => text(nested)).toThrow(/nesting limit/u);
  });
});
