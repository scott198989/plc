import { describe, expect, it } from "vitest";

import {
  ContractValidationError,
  FOUNDATION_BUILD_IDENTITY,
  FOUNDATION_HEALTHY_STATE,
  FOUNDATION_REQUEST_ID,
  FOUNDATION_STATE_HASH,
  createFoundationFailure,
  createFoundationHealthCommand,
  validateFoundationHealthCommand,
  validateFoundationHealthResult,
} from "../src/index";

const validSuccess = () =>
  ({
    affectedObjectIds: [],
    afterHash: FOUNDATION_STATE_HASH,
    beforeHash: FOUNDATION_STATE_HASH,
    diagnostics: [],
    events: [],
    kind: "domain.result",
    requestId: FOUNDATION_REQUEST_ID,
    schemaVersion: 1,
    success: true,
    value: {
      buildIdentity: FOUNDATION_BUILD_IDENTITY,
      healthState: FOUNDATION_HEALTHY_STATE,
      schemaVersion: 1,
      wasmSha256: "A".repeat(64),
    },
  }) as const;

describe("foundation contract", () => {
  it("creates and accepts the only authorized command", () => {
    const command = createFoundationHealthCommand();
    expect(validateFoundationHealthCommand(command)).toEqual(command);
  });

  it.each([
    null,
    [],
    {},
    { ...createFoundationHealthCommand(), kind: "foundation.other" },
    { ...createFoundationHealthCommand(), schemaVersion: 2 },
    { ...createFoundationHealthCommand(), requestId: "other" },
    { ...createFoundationHealthCommand(), endpoint: "loopback" },
  ])("rejects invalid or capability-shaped commands", (input) => {
    expect(() => validateFoundationHealthCommand(input)).toThrow(
      ContractValidationError,
    );
  });

  it("accepts the reconciled deterministic DomainResult envelope", () => {
    const result = validSuccess();
    expect(validateFoundationHealthResult(result)).toEqual(result);
    expect(result.beforeHash).toBe(result.afterHash);
    expect(result.events).toEqual([]);
    expect(result.diagnostics).toEqual([]);
    expect(result.affectedObjectIds).toEqual([]);
  });

  it("creates a failure that is also a valid DomainResult", () => {
    const failure = createFoundationFailure(
      "INVALID_COMMAND",
      "The command was rejected.",
    );
    expect(validateFoundationHealthResult(failure)).toEqual(failure);
    expect(failure.success).toBe(false);
    expect(failure.diagnostics).toHaveLength(1);
    expect(failure.beforeHash).toBe(failure.afterHash);
  });

  it.each([
    { ...validSuccess(), afterHash: "B".repeat(64) },
    { ...validSuccess(), diagnostics: [{ code: "BAD" }] },
    { ...validSuccess(), events: [{}] },
    { ...validSuccess(), affectedObjectIds: ["object-1"] },
    { ...validSuccess(), unknown: true },
    {
      ...validSuccess(),
      value: { ...validSuccess().value, wasmSha256: "short" },
    },
  ])("rejects malformed or mutated DomainResult envelopes", (input) => {
    expect(() => validateFoundationHealthResult(input)).toThrow(
      ContractValidationError,
    );
  });
});
