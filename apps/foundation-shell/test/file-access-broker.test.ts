import { afterEach, describe, expect, it, vi } from "vitest";

import { FileAccessBroker, FileAccessError } from "../src/file-access-broker";

const originalWindow = Object.getOwnPropertyDescriptor(globalThis, "window");

afterEach(() => {
  vi.restoreAllMocks();
  if (originalWindow === undefined) {
    Reflect.deleteProperty(globalThis, "window");
  } else {
    Object.defineProperty(globalThis, "window", originalWindow);
  }
});

describe("FileAccessBroker", () => {
  it("fails closed when the browser exposes no file grant API", async () => {
    installWindow({});
    const broker = new FileAccessBroker();

    expect(broker.canOpen()).toBe(false);
    expect(broker.canSave()).toBe(false);
    await expect(broker.requestOpen()).rejects.toMatchObject({
      code: "ACCESS_UNAVAILABLE",
    } satisfies Partial<FileAccessError>);
  });

  it("keeps the handle private and returns exact bytes with an opaque grant", async () => {
    const effects: string[] = [];
    const handle = observedHandle(
      memoryHandle("training.vlabproj", Uint8Array.of(1, 3, 3, 7)),
      effects,
    );
    installWindow({ showOpenFilePicker: async () => [handle] });
    const broker = new FileAccessBroker();

    const opened = await broker.requestOpen();

    expect(opened.displayName).toBe("training.vlabproj");
    expect([...opened.bytes]).toEqual([1, 3, 3, 7]);
    expect(opened.grantId).toMatch(/^[0-9a-f-]{36}$/u);
    expect(opened).not.toHaveProperty("path");
    expect(effects).toEqual(["kind", "name", "getFile"]);
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
    "embedded\u0000nul.vlabproj",
  ])("rejects path, endpoint, device, pipe, and print metadata before selected-byte I/O: %s", async (name) => {
    const effects: string[] = [];
    const handle = observedHandle(memoryHandle(name, Uint8Array.of(1)), effects);
    installWindow({ showOpenFilePicker: async () => [handle] });
    const broker = new FileAccessBroker();

    await expect(broker.requestOpen()).rejects.toMatchObject({
      code: "INVALID_FILE_NAME",
    } satisfies Partial<FileAccessError>);
    expect(effects).toEqual(["kind", "name"]);
  });

  it("rejects a non-file handle and a throwing metadata getter before byte I/O", async () => {
    const nonFileEffects: string[] = [];
    const nonFile = observedHandle(
      { ...memoryHandle("training.vlabproj", Uint8Array.of(1)), kind: "directory" } as unknown as TestHandle,
      nonFileEffects,
    );
    installWindow({ showOpenFilePicker: async () => [nonFile] });
    const broker = new FileAccessBroker();
    await expect(broker.requestOpen()).rejects.toMatchObject({ code: "READ_FAILED" });
    expect(nonFileEffects).toEqual(["kind", "name"]);

    const throwingEffects: string[] = [];
    const throwing = observedHandle(
      memoryHandle("training.vlabproj", Uint8Array.of(1)),
      throwingEffects,
      "name",
    );
    installWindow({ showOpenFilePicker: async () => [throwing] });
    await expect(broker.requestOpen()).rejects.toMatchObject({ code: "READ_FAILED" });
    expect(throwingEffects).toEqual(["kind", "name"]);
  });

  it("rejects a selected file whose declared and delivered byte lengths differ", async () => {
    const changing = {
      ...memoryHandle("training.vlabproj", Uint8Array.of(1)),
      getFile: async () => ({
        arrayBuffer: async () => Uint8Array.of(1).buffer,
        size: 2,
      } as File),
    };
    installWindow({ showOpenFilePicker: async () => [changing] });
    const broker = new FileAccessBroker();

    await expect(broker.requestOpen()).rejects.toMatchObject({ code: "READ_FAILED" });
  });

  it.each([
    "\\\\plc.isolation.invalid\\share\\project.vlabproj",
    "\\\\.\\pipe\\project.vlabproj",
    "PRN.vlabproj",
  ])("rejects unsafe Save As metadata before create, replace, or selected-byte I/O: %s", async (name) => {
    const effects: string[] = [];
    const handle = observedHandle(memoryHandle(name, Uint8Array.of(1)), effects);
    installWindow({ showSaveFilePicker: async () => handle });
    const broker = new FileAccessBroker();

    await expect(broker.requestSaveAs("Safe project", Uint8Array.of(2))).rejects.toMatchObject({
      code: "INVALID_FILE_NAME",
    } satisfies Partial<FileAccessError>);
    expect(effects).toEqual(["kind", "name"]);
  });

  it("writes, closes, reopens, and byte-verifies a Save As grant", async () => {
    let suggestedName: string | undefined;
    const effects: string[] = [];
    const handle = memoryHandle("cell-a.vlabproj", Uint8Array.of(9));
    installWindow({
      showSaveFilePicker: async (options: Readonly<{ suggestedName: string }>) => {
        suggestedName = options.suggestedName;
        return observedHandle(handle, effects);
      },
    });
    const broker = new FileAccessBroker();

    const result = await broker.requestSaveAs("CON", Uint8Array.of(4, 5, 6));

    expect(suggestedName).toBe("Untitled project.vlabproj");
    expect(result.displayName).toBe("cell-a.vlabproj");
    expect(result.verifiedBytes).toBe(3);
    expect([...handle.bytes()]).toEqual([4, 5, 6]);
    expect(effects).toEqual(["kind", "name", "createWritable", "getFile"]);
  });

  it("does not report success when reopened bytes differ", async () => {
    const handle = memoryHandle("cell-b.vlabproj", Uint8Array.of(1), true);
    installWindow({ showSaveFilePicker: async () => handle });
    const broker = new FileAccessBroker();
    const grantId = "11111111-1111-4111-8111-111111111111";
    vi.spyOn(crypto, "randomUUID").mockReturnValue(grantId);

    await expect(
      broker.requestSaveAs("Cell B", Uint8Array.of(4, 5, 6)),
    ).rejects.toMatchObject({ code: "WRITE_FAILED" } satisfies Partial<FileAccessError>);
    await expect(broker.save(grantId, Uint8Array.of(7))).rejects.toMatchObject({
      code: "UNKNOWN_GRANT",
    } satisfies Partial<FileAccessError>);
  });
});

type TestHandle = Readonly<{
  bytes: () => Uint8Array;
  createWritable: () => Promise<Readonly<{
    abort: () => Promise<void>;
    close: () => Promise<void>;
    write: (bytes: Uint8Array) => Promise<void>;
  }>>;
  getFile: () => Promise<File>;
  kind: "file";
  name: string;
}>;

const memoryHandle = (
  name: string,
  initial: Uint8Array,
  corruptReopen = false,
): TestHandle => {
  let committed = initial.slice();
  let staged = committed;
  return {
    bytes: () => committed.slice(),
    createWritable: async () => ({
      abort: async () => { staged = committed; },
      close: async () => { committed = staged.slice(); },
      write: async (bytes) => { staged = bytes.slice(); },
    }),
    getFile: async () => {
      const bytes = corruptReopen && committed.length > 1
        ? Uint8Array.of(committed[0] ?? 0)
        : committed;
      return new File([bytes], name);
    },
    kind: "file",
    name,
  };
};

const installWindow = (value: Readonly<Record<string, unknown>>): void => {
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value,
    writable: true,
  });
};

const observedHandle = (
  handle: TestHandle,
  effects: string[],
  throwingProperty?: string,
): TestHandle => new Proxy(handle, {
  get(target, property, receiver) {
    if (property !== "then") {
      effects.push(String(property));
    }
    if (property === throwingProperty) {
      throw new Error(`unexpected ${String(property)} access`);
    }
    return Reflect.get(target, property, receiver) as unknown;
  },
});
