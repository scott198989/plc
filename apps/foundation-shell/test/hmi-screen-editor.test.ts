import { describe, expect, it } from "vitest";

import {
  createHmiRuntimeOperationQueue,
  discoverHmiBindableTags,
  hmiEditorPersistenceKey,
} from "../src/HmiScreenEditor";
import type { RuntimeOperation } from "../src/runtime-types";
import type {
  WorkbenchObjectView,
  WorkbenchSnapshot,
} from "../src/workbench-types";

describe("HMI screen editor tag discovery", () => {
  it("projects stable active tag IDs and ignores non-tags, deleted tags, and malformed areas", () => {
    const input = object("10000000-0000-4000-8000-000000000001", "Start_PB", "Tag", "active", "I", "BOOL");
    const output = object("10000000-0000-4000-8000-000000000002", "Motor_Run", "Tag", "active", "Q", "BOOL");
    const memory = object("10000000-0000-4000-8000-000000000003", "Batch_Count", "Tag", "active", "M", "DINT");
    const deleted = object("10000000-0000-4000-8000-000000000004", "Old_Tag", "Tag", "tombstoned", "I", "BOOL");
    const malformed = object("10000000-0000-4000-8000-000000000005", "Bad_Area", "Tag", "active", "X", "BOOL");
    const program = object("10000000-0000-4000-8000-000000000006", "Main", "OB", "active", "I", "BOOL");
    const snapshot = {
      objects: Object.fromEntries([input, output, memory, deleted, malformed, program].map((item) => [item.id, item])),
    } as unknown as WorkbenchSnapshot;

    expect(discoverHmiBindableTags(snapshot)).toEqual([
      { addressArea: "I", dataType: "BOOL", id: input.id, name: "Start_PB" },
      { addressArea: "M", dataType: "DINT", id: memory.id, name: "Batch_Count" },
      { addressArea: "Q", dataType: "BOOL", id: output.id, name: "Motor_Run" },
    ]);
  });

  it("keeps the editor session key stable across runtime-only object snapshots", () => {
    const persisted = object("10000000-0000-4000-8000-000000000001", "Panel", "HmiScreen", "active", "I", "BOOL");
    const runtimeSnapshotCopy: WorkbenchObjectView = {
      ...persisted,
      objectRevision: "99",
      semanticPayload: { ...persisted.semanticPayload },
    };

    expect(runtimeSnapshotCopy).not.toBe(persisted);
    expect(runtimeSnapshotCopy.semanticPayload).not.toBe(persisted.semanticPayload);
    expect(hmiEditorPersistenceKey(runtimeSnapshotCopy)).toBe(hmiEditorPersistenceKey(persisted));
    expect(hmiEditorPersistenceKey({ ...runtimeSnapshotCopy, semanticRevision: "2" }))
      .not.toBe(hmiEditorPersistenceKey(persisted));
  });

  it("serializes a fast release behind the complete press request", async () => {
    const observed: string[] = [];
    let releaseFirst: (() => void) | undefined;
    const firstGate = new Promise<void>((resolve) => { releaseFirst = resolve; });
    let ordinal = 0;
    const queue = createHmiRuntimeOperationQueue(async (operation) => {
      observed.push(operationLabel(operation));
      ordinal += 1;
      if (ordinal === 1) await firstGate;
    });
    const press = queue.enqueue(momentaryOperations(true));
    const release = queue.enqueue(momentaryOperations(false));

    await Promise.resolve();
    await Promise.resolve();
    expect(observed).toEqual(["input:true"]);
    releaseFirst?.();
    await Promise.all([press, release]);
    expect(observed).toEqual(["input:true", "scan", "input:false", "scan"]);
  });

  it("still runs the queued release when a press operation reports an error", async () => {
    const observed: string[] = [];
    let firstScan = true;
    const queue = createHmiRuntimeOperationQueue(async (operation) => {
      const label = operationLabel(operation);
      observed.push(label);
      if (label === "scan" && firstScan) {
        firstScan = false;
        throw new Error("press scan failed after input write");
      }
    });
    const press = queue.enqueue(momentaryOperations(true));
    const release = queue.enqueue(momentaryOperations(false));

    await expect(press).rejects.toThrow("press scan failed after input write");
    await expect(release).resolves.toBeUndefined();
    expect(observed).toEqual(["input:true", "scan", "input:false", "scan"]);
  });
});

const momentaryOperations = (value: boolean): readonly RuntimeOperation[] => [
  {
    kind: "runtime.set-raw-input",
    targetId: "10000000-0000-4000-8000-000000000001",
    value: { type: "BOOL", value },
  },
  { kind: "runtime.run-scan" },
];

const operationLabel = (operation: RuntimeOperation): string =>
  operation.kind === "runtime.set-raw-input" ? `input:${String(operation.value.value)}` : "scan";

const object = (
  id: string,
  displayName: string,
  kind: WorkbenchObjectView["kind"],
  lifecycle: WorkbenchObjectView["lifecycle"],
  addressArea: string,
  dataType: string,
): WorkbenchObjectView => ({
  children: [],
  creationOrdinal: "1",
  displayName,
  id,
  kind,
  lifecycle,
  objectRevision: "1",
  parentId: null,
  payloadSchema: kind === "Tag" ? "plc.tag/2" : "plc.block/2",
  presentationPayload: {},
  semanticPayload: { addressArea, dataType },
  semanticRevision: "1",
});
