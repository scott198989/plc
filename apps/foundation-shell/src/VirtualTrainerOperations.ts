import type { RuntimeOperation, RuntimeProbeView } from "./runtime-types";

/**
 * Models one physical press and release without maintaining a second copy of
 * controller state in the trainer UI.
 */
export const createMomentaryPulseOperationSequence = (
  targetId: RuntimeProbeView["id"],
): readonly RuntimeOperation[] => [
  { kind: "runtime.set-raw-input", targetId, value: { type: "BOOL", value: true } },
  { kind: "runtime.run-scan" },
  { kind: "runtime.set-raw-input", targetId, value: { type: "BOOL", value: false } },
  { kind: "runtime.run-scan" },
];
