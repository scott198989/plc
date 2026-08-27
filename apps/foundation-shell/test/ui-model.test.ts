import { describe, expect, it } from "vitest";

import {
  FOUNDATION_BUILD_IDENTITY,
  FOUNDATION_HEALTHY_STATE,
  FOUNDATION_REQUEST_ID,
  FOUNDATION_STATE_HASH,
} from "@govs/foundation-contract";

import {
  initialFoundationViewState,
  reduceFoundationViewState,
} from "../src/ui-model";

const successResult = {
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
} as const;

describe("foundation view model", () => {
  it("moves deterministically through loading and success", () => {
    const loading = reduceFoundationViewState(initialFoundationViewState, {
      type: "started",
    });
    const success = reduceFoundationViewState(loading, {
      result: successResult,
      type: "succeeded",
    });

    expect(loading).toEqual({ phase: "loading" });
    expect(success).toEqual({ phase: "success", result: successResult });
  });

  it("exposes an explicit error state and permits retry", () => {
    const failed = reduceFoundationViewState(initialFoundationViewState, {
      message: "Unavailable.",
      type: "failed",
    });
    expect(failed).toEqual({ message: "Unavailable.", phase: "error" });
    expect(reduceFoundationViewState(failed, { type: "started" })).toEqual({
      phase: "loading",
    });
  });
});
