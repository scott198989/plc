import { afterEach, describe, expect, it } from "vitest";

import { FileAccessBroker, FileAccessError } from "../src/file-access-broker";

const originalWindow = Object.getOwnPropertyDescriptor(globalThis, "window");

afterEach(() => {
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
    const handle = memoryHandle("training.vlabproj", Uint8Array.of(1, 3, 3, 7));
    installWindow({ showOpenFilePicker: async () => [handle] });
    const broker = new FileAccessBroker();

    const opened = await broker.requestOpen();

    expect(opened.displayName).toBe("training.vlabproj");
    expect([...opened.bytes]).toEqual([1, 3, 3, 7]);
    expect(opened.grantId).toMatch(/^[0-9a-f-]{36}$/u);
    expect(opened).not.toHaveProperty("path");
  });

  it("writes, closes, reopens, and byte-verifies a Save As grant", async () => {
    const handle = memoryHandle("cell-a.vlabproj", Uint8Array.of(9));
    installWindow({ showSaveFilePicker: async () => handle });
    const broker = new FileAccessBroker();

    const result = await broker.requestSaveAs("Cell A", Uint8Array.of(4, 5, 6));

    expect(result.displayName).toBe("cell-a.vlabproj");
    expect(result.verifiedBytes).toBe(3);
    expect([...handle.bytes()]).toEqual([4, 5, 6]);
  });

  it("does not report success when reopened bytes differ", async () => {
    const handle = memoryHandle("cell-b.vlabproj", Uint8Array.of(1), true);
    installWindow({ showSaveFilePicker: async () => handle });
    const broker = new FileAccessBroker();

    await expect(
      broker.requestSaveAs("Cell B", Uint8Array.of(4, 5, 6)),
    ).rejects.toMatchObject({ code: "WRITE_FAILED" } satisfies Partial<FileAccessError>);
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
