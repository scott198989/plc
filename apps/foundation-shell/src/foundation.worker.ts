/// <reference lib="webworker" />

import { executeEngineeringRequest } from "./engineering-worker-handler";
import { executeFoundationCommand } from "./worker-handler";

const workerScope: DedicatedWorkerGlobalScope = self;

workerScope.addEventListener("message", (event: MessageEvent<unknown>) => {
  const request = event.data;
  const execute = isFoundationHealthRequest(request)
    ? executeFoundationCommand(request)
    : executeEngineeringRequest(request);
  void execute
    .then((result) => {
      const transferable = preparedSaveBytes(result);
      workerScope.postMessage(result, transferable === null ? [] : [transferable]);
    })
    .catch(() => {
      if (isFoundationHealthRequest(request)) {
        workerScope.postMessage({
          affectedObjectIds: [],
          afterHash:
            "64E21C28C534606DD9C9AA27A56C928DC09574CD70B56B6D468FE3F96C2F5A94",
          beforeHash:
            "64E21C28C534606DD9C9AA27A56C928DC09574CD70B56B6D468FE3F96C2F5A94",
          diagnostics: [{
            code: "WORKER_FAILURE",
            message: "The isolated foundation worker could not complete.",
            severity: "error",
          }],
          events: [],
          kind: "domain.result",
          requestId: "phase1-foundation-health",
          schemaVersion: 1,
          success: false,
        });
        return;
      }
      workerScope.postMessage({
        error: {
          code: "WORKER_FAILURE",
          message: "The isolated engineering core could not complete.",
        },
        inReplyTo: requestIdOf(request),
        kind: "engineering.response",
        ok: false,
      });
    });
});

const isRecord = (value: unknown): value is Readonly<Record<string, unknown>> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const isFoundationHealthRequest = (value: unknown): boolean =>
  isRecord(value) && value.kind === "foundation.health";

const requestIdOf = (value: unknown): string =>
  isRecord(value) && typeof value.requestId === "string"
    ? value.requestId
    : "00000000-0000-4000-8000-000000000000";

const preparedSaveBytes = (value: unknown): ArrayBuffer | null => {
  if (!isRecord(value) || value.kind !== "engineering.response" || !isRecord(value.value)) {
    return null;
  }
  return value.value.bytes instanceof ArrayBuffer ? value.value.bytes : null;
};
