import { describe, expect, it } from "vitest";

import { projectLadLiveMonitoring } from "../src/lad-live-monitoring";
import type { EngineeringRuntimeView, RuntimeProbeView } from "../src/runtime-types";
import type {
  ProjectPayload,
  ProjectPayloadValue,
  WorkbenchObjectView,
  WorkbenchSnapshot,
} from "../src/workbench-types";

const PROGRAM_ID = "10000000-0000-4000-8000-000000000001";
const START_MEMBER_ID = "20000000-0000-4000-8000-000000000001";
const MOTOR_MEMBER_ID = "20000000-0000-4000-8000-000000000002";
const START_TAG_ID = "30000000-0000-4000-8000-000000000001";
const MOTOR_TAG_ID = "30000000-0000-4000-8000-000000000002";

describe("LAD live monitoring projection", () => {
  it("joins active BOOL member bindings to the authoritative tag probes", () => {
    const snapshot = fixture({
      objects: [
        program(),
        tag(START_TAG_ID, "Start pushbutton", START_MEMBER_ID, "active"),
        tag(MOTOR_TAG_ID, "Motor run", MOTOR_MEMBER_ID, "active"),
        tag("30000000-0000-4000-8000-000000000099", "Old motor", MOTOR_MEMBER_ID, "tombstoned"),
      ],
      probes: [
        probe(START_TAG_ID, "input", true, "GOOD"),
        probe(MOTOR_TAG_ID, "output", false, "FORCED", false),
      ],
    });

    const result = projectLadLiveMonitoring(snapshot, PROGRAM_ID);
    expect(result.ok).toBe(true);
    if (!result.ok) {
      throw new Error(result.message);
    }

    expect(result.projection).toMatchObject({
      cpuState: "RUN",
      monitorState: "ACTIVE",
      online: true,
      programBlockId: PROGRAM_ID,
      scanSequence: "12",
    });
    expect(result.projection.members).toEqual([
      expect.objectContaining({
        forced: false,
        memberId: START_MEMBER_ID,
        memberName: "Start_PB",
        observedValue: true,
        probeId: START_TAG_ID,
        probeKind: "input",
        quality: "GOOD",
        runtimeAddress: "%I0.0",
        tagId: START_TAG_ID,
        tagName: "Start pushbutton",
        truth: "on",
        unknownReason: null,
      }),
      expect.objectContaining({
        forced: true,
        forcedValue: false,
        memberId: MOTOR_MEMBER_ID,
        observedValue: false,
        probeId: MOTOR_TAG_ID,
        quality: "FORCED",
        truth: "off",
        unknownReason: null,
      }),
    ]);
  });

  it("fails closed when a member binding is missing or ambiguous", () => {
    const snapshot = fixture({
      objects: [
        program(),
        tag(START_TAG_ID, "Start one", START_MEMBER_ID, "active"),
        tag("30000000-0000-4000-8000-000000000003", "Start two", START_MEMBER_ID, "active"),
      ],
      probes: [probe(START_TAG_ID, "input", true, "GOOD")],
    });

    const result = projectLadLiveMonitoring(snapshot, PROGRAM_ID);
    expect(result.ok).toBe(true);
    if (!result.ok) {
      throw new Error(result.message);
    }
    expect(result.projection.members.map(({ truth, unknownReason }) => ({ truth, unknownReason })))
      .toEqual([
        { truth: "unknown", unknownReason: "ambiguous-active-tag-bindings" },
        { truth: "unknown", unknownReason: "no-active-tag-binding" },
      ]);
  });

  it("retains the observed value but withholds a live truth for stale monitoring or bad quality", () => {
    const stale = fixture({
      monitorState: "STALE",
      objects: [program(), tag(START_TAG_ID, "Start", START_MEMBER_ID, "active")],
      probes: [probe(START_TAG_ID, "input", true, "GOOD")],
    });
    const staleResult = projectLadLiveMonitoring(stale, PROGRAM_ID);
    expect(staleResult.ok).toBe(true);
    if (!staleResult.ok) {
      throw new Error(staleResult.message);
    }
    expect(staleResult.projection.members[0]).toMatchObject({
      observedValue: true,
      truth: "unknown",
      unknownReason: "monitoring-stale",
    });

    const bad = fixture({
      objects: [program(), tag(START_TAG_ID, "Start", START_MEMBER_ID, "active")],
      probes: [probe(START_TAG_ID, "input", false, "BAD")],
    });
    const badResult = projectLadLiveMonitoring(bad, PROGRAM_ID);
    expect(badResult.ok).toBe(true);
    if (!badResult.ok) {
      throw new Error(badResult.message);
    }
    expect(badResult.projection.members[0]).toMatchObject({
      observedValue: false,
      quality: "BAD",
      truth: "unknown",
      unknownReason: "quality-bad",
    });
  });

  it("keeps bound members unknown while the runtime is unavailable", () => {
    const snapshot = fixture({
      objects: [program(), tag(START_TAG_ID, "Start", START_MEMBER_ID, "active")],
      probes: [],
      runtimeAvailability: "UNAVAILABLE",
    });

    const result = projectLadLiveMonitoring(snapshot, PROGRAM_ID);
    expect(result.ok).toBe(true);
    if (!result.ok) {
      throw new Error(result.message);
    }
    expect(result.projection.monitorState).toBe("UNAVAILABLE");
    expect(result.projection.members[0]).toMatchObject({
      tagId: START_TAG_ID,
      truth: "unknown",
      unknownReason: "runtime-unavailable",
    });
  });

  it("rejects duplicate BOOL interface identities instead of choosing one", () => {
    const duplicate = program({
      interface: [
        member(START_MEMBER_ID, "Start_PB", "input", "BOOL"),
        member(START_MEMBER_ID, "Duplicate", "temp", "BOOL"),
      ],
      language: "LAD",
    });
    const result = projectLadLiveMonitoring(fixture({ objects: [duplicate], probes: [] }), PROGRAM_ID);
    expect(result).toEqual({
      code: "duplicate-interface-member",
      message: "The selected LAD block repeats a BOOL interface identity.",
      ok: false,
    });
  });
});

const fixture = ({
  monitorState = "ACTIVE",
  objects,
  probes,
  runtimeAvailability = "READY",
}: Readonly<{
  monitorState?: "ACTIVE" | "DEGRADED" | "INACTIVE" | "STALE";
  objects: readonly WorkbenchObjectView[];
  probes: readonly RuntimeProbeView[];
  runtimeAvailability?: "READY" | "UNAVAILABLE";
}>): Pick<WorkbenchSnapshot, "objects" | "runtime"> => ({
  objects: Object.fromEntries(objects.map((object) => [object.id, object])),
  runtime: runtimeAvailability === "UNAVAILABLE"
    ? {
        availability: "UNAVAILABLE",
        canBuild: false,
        diagnostics: [],
        reason: "No controller",
        schemaVersion: 1,
        session: null,
        sourceDocumentHash: "document",
        sourceSemanticFingerprint: "semantic",
      }
    : runtime(probes, monitorState),
});

const runtime = (
  probes: readonly RuntimeProbeView[],
  monitorState: "ACTIVE" | "DEGRADED" | "INACTIVE" | "STALE",
): EngineeringRuntimeView => ({
  availability: "READY",
  canBuild: true,
  diagnostics: [],
  reason: null,
  schemaVersion: 1,
  session: {
    buildCurrent: true,
    buildFingerprint: "build",
    controllerEpoch: "1",
    controllerObjectId: "40000000-0000-4000-8000-000000000001",
    cpuState: "RUN",
    diagnosticReplayHash: "diagnostics",
    diagnostics: [],
    documentDirty: false,
    forceCount: 0,
    forceRegistryVersion: "0",
    forces: [],
    hardwareToLoaded: "MATCH",
    hashes: null,
    loadPreview: null,
    loaded: true,
    loadedArtifactFingerprint: "loaded",
    monitorState,
    online: true,
    probes,
    runtimeControllerId: "40000000-0000-4000-8000-000000000002",
    runtimeReplayHash: "runtime-replay",
    scanSequence: "12",
    snapshotAvailable: false,
    softwareToLoaded: "MATCH",
    traces: [],
    universeEpoch: "1",
    universeId: "40000000-0000-4000-8000-000000000003",
    virtualTimeMilliseconds: "120",
    watches: [],
  },
  sourceDocumentHash: "document",
  sourceSemanticFingerprint: "semantic",
});

const probe = (
  id: string,
  kind: "input" | "output",
  value: boolean,
  quality: RuntimeProbeView["quality"],
  forcedValue: boolean | null = null,
): RuntimeProbeView => ({
  committedOutputValue: kind === "output" ? { type: "BOOL", value } : null,
  deliveredOutputValue: kind === "output" ? { type: "BOOL", value } : null,
  displayName: id,
  effectiveValue: { type: "BOOL", value },
  forcedValue: forcedValue === null ? null : { type: "BOOL", value: forcedValue },
  id,
  kind,
  naturalValue: { type: "BOOL", value },
  quality,
  rawInputValue: kind === "input" ? { type: "BOOL", value } : null,
  runtimeAddress: kind === "input" ? "%I0.0" : "%Q0.0",
  valueType: "BOOL",
});

const program = (semanticPayload: ProjectPayload = {
  interface: [
    member(START_MEMBER_ID, "Start_PB", "input", "BOOL"),
    member(MOTOR_MEMBER_ID, "Motor_Run", "output", "BOOL"),
    member("20000000-0000-4000-8000-000000000003", "Counter", "temp", "DINT"),
  ],
  language: "LAD",
}): WorkbenchObjectView => object(PROGRAM_ID, "OB", "Main cycle", semanticPayload);

const tag = (
  id: string,
  name: string,
  memberId: string,
  lifecycle: "active" | "tombstoned",
): WorkbenchObjectView => ({
  ...object(id, "Tag", name, {
    addressArea: "I",
    addressIntent: "auto",
    blockId: PROGRAM_ID,
    dataType: "BOOL",
    memberId,
    tagKind: "Input",
  }),
  lifecycle,
});

const object = (
  id: string,
  kind: WorkbenchObjectView["kind"],
  displayName: string,
  semanticPayload: ProjectPayload,
): WorkbenchObjectView => ({
  children: [],
  creationOrdinal: "1",
  displayName,
  id,
  kind,
  lifecycle: "active",
  objectRevision: "1",
  parentId: null,
  payloadSchema: "test/1",
  presentationPayload: {},
  semanticPayload,
  semanticRevision: "1",
});

const member = (
  id: string,
  name: string,
  role: string,
  type: string,
): ProjectPayloadValue => ({
  $type: "record",
  value: { id, name, role, type },
});
