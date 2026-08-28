import { afterEach, describe, expect, it, vi } from "vitest";

import {
  FileAccessBroker,
  FileAccessError,
  NATIVE_PROJECT_BROKER_CONTRACT,
  NATIVE_PROJECT_BROKER_GLOBAL,
  NATIVE_PROJECT_BROKER_VERSION,
  type NativeFixedLocalAttestationV1,
  type NativeProjectFileBrokerV1,
} from "../src/file-access-broker";

const originalWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
const ATTESTATION_ID = "fixed-local-v1:3CDECAFE:0000000000000001";
const GRANT_ONE = "p2-native-v1:0000000000000001";

const fixedAttestation = (): NativeFixedLocalAttestationV1 => Object.freeze({
  attestationId: ATTESTATION_ID,
  fixedDrive: true,
  kind: "fixed-native-local-v1",
  nativeLocal: true,
  platform: "windows",
  providerBacked: false,
  redirected: false,
  removable: false,
  special: false,
});

const nativeBridge = (
  overrides: Partial<NativeProjectFileBrokerV1> = {},
): NativeProjectFileBrokerV1 => Object.freeze({
  attestation: fixedAttestation(),
  contract: NATIVE_PROJECT_BROKER_CONTRACT,
  open: vi.fn(async () => Object.freeze({
    attestationId: ATTESTATION_ID,
    bytes: Uint8Array.of(1, 3, 3, 7),
    displayName: "training.vlabproj",
    grantId: GRANT_ONE,
    protocolVersion: NATIVE_PROJECT_BROKER_VERSION,
  })),
  protocolVersion: NATIVE_PROJECT_BROKER_VERSION,
  revoke: vi.fn(),
  save: vi.fn(async ({ bytes, grantId }) => Object.freeze({
    attestationId: ATTESTATION_ID,
    displayName: "training.vlabproj",
    grantId,
    protocolVersion: NATIVE_PROJECT_BROKER_VERSION,
    verifiedBytes: bytes.byteLength,
  })),
  saveAs: vi.fn(async ({ bytes, projectName }) => Object.freeze({
    attestationId: ATTESTATION_ID,
    displayName: projectName,
    grantId: GRANT_ONE,
    protocolVersion: NATIVE_PROJECT_BROKER_VERSION,
    verifiedBytes: bytes.byteLength,
  })),
  ...overrides,
});

afterEach(() => {
  vi.restoreAllMocks();
  if (originalWindow === undefined) {
    Reflect.deleteProperty(globalThis, "window");
  } else {
    Object.defineProperty(globalThis, "window", originalWindow);
  }
});

describe("FileAccessBroker native boundary", () => {
  it("gives no credit to web-only picker adapters", async () => {
    const showOpenFilePicker = vi.fn();
    const showSaveFilePicker = vi.fn();
    installWindow({ showOpenFilePicker, showSaveFilePicker });
    const broker = new FileAccessBroker();

    expect(broker.canOpen()).toBe(false);
    expect(broker.canSave()).toBe(false);
    await expect(broker.requestOpen()).rejects.toMatchObject({
      code: "ACCESS_UNAVAILABLE",
    } satisfies Partial<FileAccessError>);
    await expect(broker.requestSaveAs("Cell", Uint8Array.of(1))).rejects.toMatchObject({
      code: "ACCESS_UNAVAILABLE",
    } satisfies Partial<FileAccessError>);
    expect(showOpenFilePicker).not.toHaveBeenCalled();
    expect(showSaveFilePicker).not.toHaveBeenCalled();
  });

  it("accepts only the non-replaceable exact native bridge and copies opened bytes", async () => {
    const bridge = nativeBridge();
    installNativeBridge(bridge);
    const broker = new FileAccessBroker();

    expect(broker.canOpen()).toBe(true);
    expect(broker.canSave()).toBe(true);
    const opened = await broker.requestOpen();

    expect(opened).toEqual({
      bytes: Uint8Array.of(1, 3, 3, 7),
      displayName: "training.vlabproj",
      grantId: GRANT_ONE,
    });
    expect(opened).not.toHaveProperty("path");
    expect(opened.bytes).not.toBe((await bridge.open()).bytes);
  });

  it.each([
    ["fixedDrive", false],
    ["nativeLocal", false],
    ["providerBacked", true],
    ["redirected", true],
    ["removable", true],
    ["special", true],
    ["platform", "linux"],
    ["kind", "unattested"],
    ["attestationId", "fixed-local-v1:INVALID"],
  ] as const)("rejects an unsafe attestation mutation without invoking open: %s", async (key, value) => {
    const open = vi.fn();
    const attestation = { ...fixedAttestation(), [key]: value } as NativeFixedLocalAttestationV1;
    installNativeBridge(nativeBridge({ attestation, open }));
    const broker = new FileAccessBroker();

    expect(broker.canOpen()).toBe(false);
    await expect(broker.requestOpen()).rejects.toMatchObject({ code: "ACCESS_UNAVAILABLE" });
    expect(open).not.toHaveBeenCalled();
  });

  it("rejects replaceable, enumerable, accessor, and expanded generic bridge surfaces", async () => {
    for (const descriptor of [
      { configurable: true, enumerable: false, writable: false },
      { configurable: false, enumerable: true, writable: false },
      { configurable: false, enumerable: false, writable: true },
    ]) {
      installNativeBridge(nativeBridge(), descriptor);
      expect(new FileAccessBroker().canOpen()).toBe(false);
    }

    installWindow({});
    Object.defineProperty(window, NATIVE_PROJECT_BROKER_GLOBAL, {
      configurable: false,
      enumerable: false,
      get: () => nativeBridge(),
    });
    expect(new FileAccessBroker().canOpen()).toBe(false);

    installNativeBridge({
      ...nativeBridge(),
      invoke: vi.fn(),
    } as unknown as NativeProjectFileBrokerV1);
    const expanded = new FileAccessBroker();
    expect(expanded.canOpen()).toBe(false);
    await expect(expanded.requestOpen()).rejects.toMatchObject({ code: "ACCESS_UNAVAILABLE" });

    for (const hiddenKey of ["invoke", Symbol("invoke")] as const) {
      const hidden = { ...nativeBridge() } as NativeProjectFileBrokerV1;
      Object.defineProperty(hidden, hiddenKey, {
        configurable: false,
        enumerable: false,
        value: vi.fn(),
        writable: false,
      });
      Object.freeze(hidden);
      installNativeBridge(hidden);
      expect(new FileAccessBroker().canOpen()).toBe(false);
    }
  });

  it.each([
    "https://plc.isolation.invalid/project.vlabproj",
    "file://plc.isolation.invalid/share/project.vlabproj",
    "\\\\plc.isolation.invalid\\share\\project.vlabproj",
    "\\\\.\\pipe\\project.vlabproj",
    "\\\\.\\COM1.vlabproj",
    "C:\\redirect\\project.vlabproj",
    "..\\escape\\project.vlabproj",
    "PRN.vlabproj",
    "AUX.report.vlabproj",
    "COM1.vlabproj",
    "LPT9.vlabproj",
    "trailing-space .vlabproj",
    "trailing-dot..vlabproj",
    "café.vlabproj",
  ])("rejects unsafe native result metadata: %s", async (displayName) => {
    installNativeBridge(nativeBridge({
      open: vi.fn(async () => ({
        attestationId: ATTESTATION_ID,
        bytes: Uint8Array.of(1),
        displayName,
        grantId: GRANT_ONE,
        protocolVersion: NATIVE_PROJECT_BROKER_VERSION,
      })),
    }));
    await expect(new FileAccessBroker().requestOpen()).rejects.toBeInstanceOf(FileAccessError);
  });

  it("normalizes Save As, transfers a copy, and requires byte-complete native verification", async () => {
    let requestBytes: Uint8Array | undefined;
    const saveAs = vi.fn(async (request: Readonly<{
      bytes: Uint8Array<ArrayBuffer>;
      projectName: string;
    }>) => {
      requestBytes = request.bytes;
      return Object.freeze({
        attestationId: ATTESTATION_ID,
        displayName: request.projectName,
        grantId: GRANT_ONE,
        protocolVersion: NATIVE_PROJECT_BROKER_VERSION,
        verifiedBytes: request.bytes.byteLength,
      } as const);
    });
    installNativeBridge(nativeBridge({ saveAs }));
    const broker = new FileAccessBroker();
    const source = Uint8Array.of(4, 5, 6);

    const saved = await broker.requestSaveAs("CON", source);

    expect(saved).toEqual({
      displayName: "Untitled project.vlabproj",
      grantId: GRANT_ONE,
      verifiedBytes: 3,
    });
    expect(saveAs).toHaveBeenCalledWith({
      bytes: Uint8Array.of(4, 5, 6),
      projectName: "Untitled project.vlabproj",
      protocolVersion: 1,
    });
    expect(requestBytes).not.toBe(source);

    installNativeBridge(nativeBridge({
      saveAs: vi.fn(async () => Object.freeze({
        attestationId: ATTESTATION_ID,
        displayName: "cell.vlabproj",
        grantId: GRANT_ONE,
        protocolVersion: NATIVE_PROJECT_BROKER_VERSION,
        verifiedBytes: 2,
      })),
    }));
    await expect(new FileAccessBroker().requestSaveAs("cell", source)).rejects.toMatchObject({
      code: "WRITE_FAILED",
    });
  });

  it("saves only broker-issued grants and revokes both renderer and native authority", async () => {
    const revoke = vi.fn();
    const bridge = nativeBridge({ revoke });
    installNativeBridge(bridge);
    const broker = new FileAccessBroker();

    await expect(broker.save(GRANT_ONE, Uint8Array.of(1))).rejects.toMatchObject({
      code: "UNKNOWN_GRANT",
    });
    const opened = await broker.requestOpen();
    await expect(broker.save(opened.grantId, Uint8Array.of(8, 9))).resolves.toMatchObject({
      grantId: GRANT_ONE,
      verifiedBytes: 2,
    });
    broker.revoke(opened.grantId);
    expect(revoke).toHaveBeenCalledWith(GRANT_ONE);
    await expect(broker.save(opened.grantId, Uint8Array.of(1))).rejects.toMatchObject({
      code: "UNKNOWN_GRANT",
    });
  });

  it("rejects stale attestation, protocol, grant, and unexpected path fields", async () => {
    const mutations: Array<Record<string, unknown>> = [
      { attestationId: "fixed-local-v1:3CDECAFE:0000000000000002" },
      { protocolVersion: 2 },
      { grantId: "browser-grant" },
      { path: "C:\\unsafe\\training.vlabproj" },
    ];
    for (const mutation of mutations) {
      installNativeBridge(nativeBridge({
        open: vi.fn(async () => ({
          attestationId: ATTESTATION_ID,
          bytes: Uint8Array.of(1),
          displayName: "training.vlabproj",
          grantId: GRANT_ONE,
          protocolVersion: NATIVE_PROJECT_BROKER_VERSION,
          ...mutation,
        } as never)),
      }));
      await expect(new FileAccessBroker().requestOpen()).rejects.toBeInstanceOf(FileAccessError);
    }
  });

  it("best-effort revokes an exact returned grant when native result validation fails", async () => {
    const revoke = vi.fn();
    installNativeBridge(nativeBridge({
      open: vi.fn(async () => Object.freeze({
        attestationId: ATTESTATION_ID,
        bytes: Uint8Array.of(1),
        displayName: "C:\\unsafe\\training.vlabproj",
        grantId: GRANT_ONE,
        protocolVersion: NATIVE_PROJECT_BROKER_VERSION,
      })),
      revoke,
    }));

    await expect(new FileAccessBroker().requestOpen()).rejects.toBeInstanceOf(FileAccessError);
    expect(revoke).toHaveBeenCalledOnce();
    expect(revoke).toHaveBeenCalledWith(GRANT_ONE);
  });

  it("maps only bounded native cancellation, attestation, and stale-grant errors", async () => {
    for (const [nativeCode, expected] of [
      ["ACCESS_CANCELLED", "ACCESS_CANCELLED"],
      ["ATTESTATION_FAILED", "ATTESTATION_FAILED"],
      ["STALE_GRANT", "UNKNOWN_GRANT"],
      ["ARBITRARY_NATIVE_ERROR", "READ_FAILED"],
    ] as const) {
      installNativeBridge(nativeBridge({
        open: vi.fn(async () => { throw { code: nativeCode }; }),
      }));
      await expect(new FileAccessBroker().requestOpen()).rejects.toMatchObject({ code: expected });
    }
  });

  it("retires renderer authority after a native stale-grant rejection", async () => {
    const save = vi.fn(async () => {
      throw Object.freeze({ code: "STALE_GRANT" });
    });
    installNativeBridge(nativeBridge({ save }));
    const broker = new FileAccessBroker();
    const opened = await broker.requestOpen();

    await expect(broker.save(opened.grantId, Uint8Array.of(9))).rejects.toMatchObject({
      code: "UNKNOWN_GRANT",
    });
    await expect(broker.save(opened.grantId, Uint8Array.of(9))).rejects.toMatchObject({
      code: "UNKNOWN_GRANT",
    });
    expect(save).toHaveBeenCalledTimes(1);
  });

  it("retires local authority and revokes an exact grant after an invalid save result", async () => {
    const revoke = vi.fn();
    const save = vi.fn(async () => Object.freeze({
      attestationId: ATTESTATION_ID,
      displayName: "training.vlabproj",
      grantId: GRANT_ONE,
      path: "C:\\unsafe\\training.vlabproj",
      protocolVersion: NATIVE_PROJECT_BROKER_VERSION,
      verifiedBytes: 1,
    }) as never);
    installNativeBridge(nativeBridge({ revoke, save }));
    const broker = new FileAccessBroker();
    const opened = await broker.requestOpen();

    await expect(broker.save(opened.grantId, Uint8Array.of(9))).rejects.toBeInstanceOf(FileAccessError);
    expect(revoke).toHaveBeenCalledWith(GRANT_ONE);
    await expect(broker.save(opened.grantId, Uint8Array.of(9))).rejects.toMatchObject({
      code: "UNKNOWN_GRANT",
    });
    expect(save).toHaveBeenCalledTimes(1);
  });
});

const installWindow = (value: Readonly<Record<string, unknown>>): void => {
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value,
    writable: true,
  });
};

const installNativeBridge = (
  bridge: NativeProjectFileBrokerV1,
  descriptor: Readonly<{
    configurable: boolean;
    enumerable: boolean;
    writable: boolean;
  }> = { configurable: false, enumerable: false, writable: false },
): void => {
  installWindow({});
  Object.defineProperty(window, NATIVE_PROJECT_BROKER_GLOBAL, {
    ...descriptor,
    value: bridge,
  });
};
