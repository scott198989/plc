import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";

import {
  analyzeBoundNetLogObject,
  analyzeProcessEvidence,
  parseBoundNetLogText,
  validateEvidenceRows,
  validateRawHostManifest,
} from "./finalize_native_e2e_evidence.mjs";

const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex").toUpperCase();
const runtimeSha256 = "A".repeat(64);

const completeRawHostManifest = {
  schemaVersion: "1.0",
  evidenceKind: "WINDOWS_NATIVE_BRIDGE_RAW_RUN",
  result: "PASS",
  fixedLocalBacking: true,
  providerBacked: false,
  remote: false,
  removable: false,
  special: false,
  redirected: false,
  metadataOnlyBeforeAcceptance: true,
  selectedByteIoBeforeAcceptance: false,
  verificationStage: 4,
  operations: ["create", "open", "replace"],
  instrumentationStatus: "REQUIRES_EXTERNAL_HARNESS",
  controlledInputSha256: "D".repeat(64),
  deterministicOutputSha256: "E".repeat(64),
  runtimeReplaySha256: "B".repeat(64),
  canonicalReplaySha256: "C".repeat(64),
  verifiedReplayEventCount: 7,
  verifiedReplayBoundaryCount: 2,
};

function netLog(typeName, params) {
  return {
    constants: {
      logCaptureMode: "Everything",
      logEventTypes: { [typeName]: 101 },
      logSourceType: { URL_REQUEST: 7 },
    },
    events: [
      {
        params,
        phase: 1,
        source: { id: 1, type: 7 },
        type: 101,
      },
    ],
  };
}

test("clean fixed virtual-host NetLog is fully classified", () => {
  const result = analyzeBoundNetLogObject(
    netLog("URL_REQUEST_START_JOB", { url: "https://govs-plc.local/index.html" }),
  );
  assert.equal(result.relevantEventCount, 1);
  assert.equal(result.actionableEventCount, 1);
  assert.deepEqual(result.externalTargets, []);
  assert.deepEqual(result.unknownTargets, []);
  assert.ok(result.allowedTargets.length >= 1);
});

test("external DNS host is rejected", () => {
  const result = analyzeBoundNetLogObject(
    netLog("HOST_RESOLVER_MANAGER_REQUEST", { hostname: "resolver.example.invalid" }),
  );
  assert.ok(result.externalTargets.length >= 1);
});

test("external URL is rejected", () => {
  const result = analyzeBoundNetLogObject(
    netLog("URL_REQUEST_START_JOB", { url: "https://updates.example.com/payload" }),
  );
  assert.ok(result.externalTargets.length >= 1);
});

test("external socket address is rejected", () => {
  const result = analyzeBoundNetLogObject(
    netLog("TCP_CONNECT", { address: "[2001:db8::1]:443" }),
  );
  assert.ok(result.externalTargets.length >= 1);
});

test("malformed or truncated NetLog is rejected", () => {
  assert.throws(
    () => parseBoundNetLogText('{"constants":{"logCaptureMode":"Everything"},"events":['),
    /malformed or truncated/u,
  );
});

test("tampered observer evidence hash is rejected", () => {
  const original = Buffer.from("bound evidence\n", "utf8");
  const tampered = Buffer.from("tampered evidence\n", "utf8");
  assert.throws(
    () => validateEvidenceRows(
      [{ bytes: original.byteLength, path: "native-run-raw.json", sha256: sha256(original) }],
      new Map([["native-run-raw.json", tampered]]),
    ),
    /hash drifted/u,
  );
});

test("complete verified replay receipt is bound from the raw host manifest", () => {
  assert.deepEqual(validateRawHostManifest(completeRawHostManifest), {
    controlledInputSha256: "D".repeat(64),
    deterministicOutputSha256: "E".repeat(64),
    runtimeReplaySha256: "B".repeat(64),
    canonicalReplaySha256: "C".repeat(64),
    verifiedReplayEventCount: 7,
    verifiedReplayBoundaryCount: 2,
  });
});

test("missing or nonpositive verified replay receipt fields are rejected", () => {
  assert.throws(
    () => validateRawHostManifest({
      ...completeRawHostManifest,
      controlledInputSha256: "not-a-sha",
    }),
    /verified replay journey/u,
  );
  assert.throws(
    () => validateRawHostManifest({
      ...completeRawHostManifest,
      runtimeReplaySha256: undefined,
    }),
    /verified replay journey/u,
  );
  assert.throws(
    () => validateRawHostManifest({
      ...completeRawHostManifest,
      verifiedReplayBoundaryCount: 0,
    }),
    /verified replay journey/u,
  );
});

test("clean local-only process evidence is accounted", () => {
  const result = analyzeProcessEvidence(
    {
      schemaVersion: "1.0",
      evidenceKind: "WINDOWS_NATIVE_EXTERNAL_PROCESS_ENDPOINT_CAPTURE",
      captureComplete: true,
      snapshotIntervalMilliseconds: 50,
      snapshotCount: 4,
      processes: [
        {
          executableSha256: runtimeSha256,
          imageName: "msedgewebview2.exe",
          parentProcessId: 10,
          processId: 11,
        },
      ],
      endpoints: [
        {
          external: false,
          family: "ipv4",
          localAddress: "127.0.0.1",
          localPort: 49152,
          processId: 11,
          protocol: "tcp",
          remoteAddress: "0.0.0.0",
          remotePort: 0,
          state: 2,
        },
      ],
    },
    runtimeSha256,
  );
  assert.deepEqual(result.externalEndpoints, []);
  assert.equal(result.accountedEndpoints.length, 1);
  assert.equal(result.runtimeProcessCount, 1);
});

test("wildcard UDP ownership cannot receive zero-attempt credit", () => {
  const result = analyzeProcessEvidence(
    {
      schemaVersion: "1.0",
      evidenceKind: "WINDOWS_NATIVE_EXTERNAL_PROCESS_ENDPOINT_CAPTURE",
      captureComplete: true,
      snapshotIntervalMilliseconds: 50,
      snapshotCount: 1,
      processes: [
        { executableSha256: runtimeSha256, imageName: "msedgewebview2.exe" },
      ],
      endpoints: [
        {
          external: true,
          family: "ipv4",
          localAddress: "0.0.0.0",
          localPort: 5353,
          processId: 11,
          protocol: "udp",
          remoteAddress: "",
          remotePort: 0,
          state: 0,
        },
      ],
    },
    runtimeSha256,
  );
  assert.equal(result.externalEndpoints.length, 1);
  assert.match(result.externalEndpoints[0].reason, /udp/u);
});
