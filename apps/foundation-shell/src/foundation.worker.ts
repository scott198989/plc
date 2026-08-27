/// <reference lib="webworker" />

import { executeFoundationCommand } from "./worker-handler";

const workerScope: DedicatedWorkerGlobalScope = self;

workerScope.addEventListener("message", (event: MessageEvent<unknown>) => {
  void executeFoundationCommand(event.data)
    .then((result) => workerScope.postMessage(result))
    .catch(() => {
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
    });
});
