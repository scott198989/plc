import { validateFoundationHealthResult } from "@govs/foundation-contract";
import type { FoundationHealthSuccess } from "@govs/foundation-contract";

import FoundationWorker from "./foundation.worker?worker&inline";

const FAILURE_MESSAGE = "The local foundation could not be verified.";

export const verifyLocalFoundation = async (): Promise<FoundationHealthSuccess> =>
  new Promise((resolve, reject) => {
    const worker = new FoundationWorker({ name: "foundation-health" });
    let settled = false;

    const finish = (
      continuation: () => void,
    ): void => {
      if (settled) {
        return;
      }
      settled = true;
      worker.terminate();
      continuation();
    };

    worker.addEventListener("message", (event: MessageEvent<unknown>) => {
      try {
        const result = validateFoundationHealthResult(event.data);
        if (!result.success) {
          finish(() => reject(new Error(result.diagnostics[0].message)));
          return;
        }
        finish(() => resolve(result));
      } catch {
        finish(() => reject(new Error(FAILURE_MESSAGE)));
      }
    });

    worker.addEventListener("messageerror", () => {
      finish(() => reject(new Error(FAILURE_MESSAGE)));
    });

    worker.addEventListener("error", () => {
      finish(() => reject(new Error(FAILURE_MESSAGE)));
    });

    worker.postMessage({
      kind: "foundation.health",
      requestId: "phase1-foundation-health",
      schemaVersion: 1,
    });
  });
