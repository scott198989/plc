import { afterEach, describe, expect, it } from "vitest";

import {
  DEFAULT_FUZZ_CASES,
  FUZZ_CASE_IDS_SHA256,
  FUZZ_CORPUS_SHA256,
} from "../../../tools/phase2/isolation-counterfactual-lib.mjs";
import {
  FileAccessBroker,
  FileAccessError,
  type NativeFixedLocalAttestationV1,
  type NativeOpenedProjectV1,
  type NativeProjectFileBrokerV1,
  type NativeSavedProjectV1,
} from "../src/file-access-broker";
import {
  RuntimeWireError,
  parseEngineeringRuntimeView,
  parseRuntimeOperation,
} from "../src/runtime-wire";

const unavailableView = {
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
} as const;

const originalWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
const attestation = Object.freeze({
  attestationId: "fixed-local-v1:3CDECAFE:0000000000000001",
  fixedDrive: true,
  kind: "fixed-native-local-v1",
  nativeLocal: true,
  platform: "windows",
  providerBacked: false,
  redirected: false,
  removable: false,
  special: false,
} satisfies NativeFixedLocalAttestationV1);

afterEach(() => {
  if (originalWindow === undefined) {
    Reflect.deleteProperty(globalThis, "window");
  } else {
    Object.defineProperty(globalThis, "window", originalWindow);
  }
});

describe("Phase 2 isolation typed-boundary fuzz matrix", () => {
  it("binds the complete deterministic corpus", () => {
    expect(DEFAULT_FUZZ_CASES).toHaveLength(27);
    expect(FUZZ_CORPUS_SHA256).toMatch(/^[A-F0-9]{64}$/u);
    expect(FUZZ_CASE_IDS_SHA256).toMatch(/^[A-F0-9]{64}$/u);
  });

  it.each(DEFAULT_FUZZ_CASES)(
    "rejects $id at the semantic-navigation UUID boundary without navigation",
    (fuzzCase) => {
      const response = {
        ...unavailableView,
        diagnostics: [{ ...unavailableView.diagnostics[0], objectId: fuzzCase.value }],
      };
      expect(() => parseEngineeringRuntimeView(jsonBytes(response))).toThrow(RuntimeWireError);
    },
  );

  it.each(DEFAULT_FUZZ_CASES)(
    "rejects $id at the Virtual Download typed operation boundary",
    (fuzzCase) => {
      expect(() => parseRuntimeOperation({
        endpoint: fuzzCase.value,
        kind: "runtime.preview-load",
        postLoadMode: "STOP",
      })).toThrow(RuntimeWireError);
    },
  );

  it.each(DEFAULT_FUZZ_CASES)(
    "rejects $id at native open metadata before renderer byte access",
    async (fuzzCase) => {
      let bytesRead = false;
      const opened = Object.defineProperties({}, {
        attestationId: { enumerable: true, value: attestation.attestationId },
        bytes: {
          enumerable: true,
          get: () => {
            bytesRead = true;
            return Uint8Array.of(1);
          },
        },
        displayName: { enumerable: true, value: fuzzCase.value },
        grantId: { enumerable: true, value: "p2-native-v1:0000000000000001" },
        protocolVersion: { enumerable: true, value: 1 },
      }) as NativeOpenedProjectV1;
      installNativeBridge({ open: async () => opened });
      const broker = new FileAccessBroker();

      await expect(broker.requestOpen()).rejects.toBeInstanceOf(FileAccessError);
      expect(bytesRead).toBe(false);
    },
  );

  it.each(DEFAULT_FUZZ_CASES)(
    "rejects $id at native create metadata result",
    async (fuzzCase) => {
      installNativeBridge({
        saveAs: async () => savedResult(fuzzCase.value),
      });
      const broker = new FileAccessBroker();

      await expect(
        broker.requestSaveAs("Isolation fuzz", Uint8Array.of(1)),
      ).rejects.toBeInstanceOf(FileAccessError);
    },
  );

  it.each(DEFAULT_FUZZ_CASES)(
    "rejects $id at native replace metadata result",
    async (fuzzCase) => {
      installNativeBridge({
        save: async () => savedResult(fuzzCase.value),
      });
      const broker = new FileAccessBroker();
      const opened = await broker.requestOpen();

      await expect(
        broker.save(opened.grantId, Uint8Array.of(2)),
      ).rejects.toBeInstanceOf(FileAccessError);
    },
  );
});

const jsonBytes = (value: unknown): Uint8Array<ArrayBuffer> =>
  new TextEncoder().encode(JSON.stringify(value));

const openedResult = (): NativeOpenedProjectV1 => Object.freeze({
  attestationId: attestation.attestationId,
  bytes: Uint8Array.of(1),
  displayName: "isolation-safe.vlabproj",
  grantId: "p2-native-v1:0000000000000001",
  protocolVersion: 1,
});

const savedResult = (displayName: string): NativeSavedProjectV1 => Object.freeze({
  attestationId: attestation.attestationId,
  displayName,
  grantId: "p2-native-v1:0000000000000001",
  protocolVersion: 1,
  verifiedBytes: 1,
});

const installNativeBridge = (
  overrides: Partial<Pick<NativeProjectFileBrokerV1, "open" | "save" | "saveAs">>,
): void => {
  const bridge = Object.freeze({
    attestation,
    contract: "govs.project-file-broker",
    open: overrides.open ?? (async () => openedResult()),
    protocolVersion: 1,
    revoke: (_grantId: string) => undefined,
    save: overrides.save ?? (async () => savedResult("isolation-safe.vlabproj")),
    saveAs: overrides.saveAs ?? (async () => savedResult("isolation-safe.vlabproj")),
  } satisfies NativeProjectFileBrokerV1);
  const fakeWindow: Record<string, unknown> = {};
  Object.defineProperty(fakeWindow, "govsProjectFileBrokerV1", {
    configurable: false,
    enumerable: false,
    value: bridge,
    writable: false,
  });
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: fakeWindow,
    writable: true,
  });
};
