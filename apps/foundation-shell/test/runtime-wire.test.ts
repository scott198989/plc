import { describe, expect, it } from "vitest";

import {
  RuntimeWireError,
  encodeRuntimeOperation,
  parseEngineeringRuntimeView,
} from "../src/runtime-wire";

const UUIDS = {
  author: "10000000-0000-4000-8000-000000000001",
  command: "20000000-0000-4000-8000-000000000001",
  force: "30000000-0000-4000-8000-000000000001",
  idempotency: "40000000-0000-4000-8000-000000000001",
  target: "50000000-0000-4000-8000-000000000001",
} as const;

const identity = {
  authorId: UUIDS.author,
  commandId: UUIDS.command,
  idempotencyKey: UUIDS.idempotency,
} as const;

describe("runtime WASM wire", () => {
  it("encodes bounded typed operations deterministically", () => {
    const operation = {
      forceId: UUIDS.force,
      kind: "runtime.create-force",
      reason: "Commissioning exercise",
      targetId: UUIDS.target,
      value: { type: "BOOL", value: true },
    } as const;
    const first = encodeRuntimeOperation(operation, identity);
    const second = encodeRuntimeOperation(operation, identity);

    expect(first).toEqual(second);
    expect(new TextDecoder().decode(first)).toBe([
      "PES-SYSTEM-COMMAND-1",
      "CREATE_FORCE",
      UUIDS.command,
      UUIDS.idempotency,
      UUIDS.author,
      UUIDS.force,
      UUIDS.target,
      "BOOL",
      "true",
      "Commissioning exercise",
    ].join("\n"));
  });

  it("rejects invalid scalar ranges and delimiter injection before WASM", () => {
    expect(() => encodeRuntimeOperation({
      kind: "runtime.modify-once",
      targetId: UUIDS.target,
      value: { type: "U32", value: "4294967296" },
    }, identity)).toThrow(RuntimeWireError);
    expect(() => encodeRuntimeOperation({
      forceId: UUIDS.force,
      kind: "runtime.remove-force",
      reason: "line one\nline two",
    }, identity)).toThrow(RuntimeWireError);
  });

  it("accepts an exact unavailable read model and rejects extra fields", () => {
    const view = {
      availability: "UNAVAILABLE",
      canBuild: false,
      diagnostics: [{
        blocking: true,
        code: "EDU-SYS-1001",
        message: "A valid fictional controller is required.",
        objectId: null,
      }],
      reason: "Canonical project hardware is not runnable.",
      schemaVersion: 1,
      session: null,
      sourceDocumentHash: "A".repeat(64),
      sourceSemanticFingerprint: "B".repeat(64),
    };
    expect(parseEngineeringRuntimeView(jsonBytes(view))).toEqual(view);
    expect(() => parseEngineeringRuntimeView(jsonBytes({ ...view, endpoint: "https://invalid.test" })))
      .toThrow(RuntimeWireError);
  });
});

const jsonBytes = (value: unknown): Uint8Array<ArrayBuffer> =>
  new TextEncoder().encode(JSON.stringify(value));

