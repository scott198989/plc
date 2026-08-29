import assert from "node:assert/strict";
import test from "node:test";
import { tmpdir } from "node:os";
import path from "node:path";
import { mkdtempSync, writeFileSync, rmSync } from "node:fs";

import { DEFAULT_FUZZ_CASES, FUZZ_CASE_IDS_SHA256, FUZZ_CORPUS_SHA256, REQUIRED_EXPORT_SURFACE_IDS, REQUIRED_FUZZ_BOUNDARY_IDS } from "../../tools/phase2/isolation-counterfactual-lib.mjs";
import { assembleLiveLanScenario, canonicalTopology, topologySnapshotRecord, validateSnapshot } from "../../tools/phase2/collect_live_lan_topology.mjs";
import { assembleIsolationClosure, readEvidence, validateScenarioBundle } from "../../tools/phase2/assemble_isolation_closure.mjs";
import { fixedCommandDescriptors, nonCreditTestSeamProof } from "../../tools/phase2/finalize_external_isolation_proofs.mjs";

const digest = "A".repeat(64);
const candidate = { approvalDecisionId: "P2-DEC-ISO-NATIVE-001", approvalSha256: digest, commit: "1".repeat(40), tree: "2".repeat(40) };
const command = "B".repeat(64);
const log = "C".repeat(64);

test("topology canonicalization is stable across ordering and excludes volatile fields", () => {
  const one = { adapters: [{ interfaceIndex: 3, interfaceGuid: "g2", name: "Ethernet", description: "adapter", status: "Up", mediaState: "Connected", classification: "physical", macAddress: "aa", linkSpeed: "1 Gbps", unicast: [{ address: "10.0.0.2", family: "IPv4", prefixLength: 24 }], gateways: ["10.0.0.1"], dnsServers: ["10.0.0.1"], capturedAt: "volatile" }], profiles: [{ interfaceGuid: "g2", interfaceIndex: 3, name: "LAN", networkCategory: "Private" }] };
  const two = { adapters: [{ ...one.adapters[0], capturedAt: "different", dnsServers: ["10.0.0.1"], gateways: ["10.0.0.1"] }], profiles: [...one.profiles] };
  assert.deepEqual(canonicalTopology(one), canonicalTopology(two));
  assert.equal(topologySnapshotRecord("pre", one, digest).topologyFingerprint, topologySnapshotRecord("pre", two, digest).topologyFingerprint);
  assert.throws(() => topologySnapshotRecord("PRE!", one, digest));
  assert.throws(() => validateSnapshot({ ...topologySnapshotRecord("pre", one, digest), injected: true }, "pre", digest), /source-bound/u);
});

test("closure assembly accepts only independently shaped runtime records and rejects synthetic closure-shaped input", () => {
  const scenarios = ["A", "B"].map((scenarioId, index) => ({ value: scenario(scenarioId, index ? "F".repeat(64) : "E".repeat(64), index ? "9".repeat(64) : digest) }));
  const closure = assembleIsolationClosure({ candidate, adaptersOff: { value: adaptersOff(), sha256: digest }, boundary: { value: boundary(), sha256: digest }, exportRejection: { value: exportsProof(), sha256: digest }, nativeBacking: { value: backing(), sha256: digest }, scenarios });
  assert.equal(closure.configurationCoverage.evidenceBindings.length, 2);
  assert.equal(closure.liveLanTopologyVariation.scenarios.length, 2);
  assert.equal(closure.boundaryFuzzCoverage.boundaries.length, 10);
  assert.equal(closure.vendorDeployableExportRejection.surfaces.length, 4);
  assert.throws(() => assembleIsolationClosure({ candidate, adaptersOff: { value: adaptersOff() }, boundary: { value: { ...boundary(), evidenceKind: "PHASE2_ISOLATION_CLOSURE_INPUT" } }, exportRejection: { value: exportsProof() }, nativeBacking: { value: backing() }, scenarios }), /runtime evidence/u);
  assert.throws(() => assembleIsolationClosure({ candidate, adaptersOff: { value: adaptersOff() }, boundary: { value: { ...boundary(), boundaries: boundary().boundaries.slice(1) } }, exportRejection: { value: exportsProof() }, nativeBacking: { value: backing() }, scenarios }), /exactly 10 rows/u);
  assert.throws(() => assembleIsolationClosure({ candidate, adaptersOff: { value: adaptersOff() }, boundary: { value: boundary() }, exportRejection: { value: { ...exportsProof(), surfaces: exportsProof().surfaces.slice(1) } }, nativeBacking: { value: backing() }, scenarios }), /exactly 4 rows/u);
  assert.throws(() => assembleIsolationClosure({ candidate, adaptersOff: { value: adaptersOff() }, boundary: { value: boundary() }, exportRejection: { value: exportsProof() }, nativeBacking: { value: backing() }, scenarios: [{ value: scenario("A", "E".repeat(64), digest) }, { value: scenario("B", "F".repeat(64), digest) }] }), /independently finalized/u);
});

test("scenario assembly rejects unstable topology and incomplete finalized native bundles", () => {
  const pre = topologySnapshotRecord("pre", capturedTopology(), digest);
  const post = { ...topologySnapshotRecord("post", capturedTopology(), digest), topologyFingerprint: "F".repeat(64) };
  assert.throws(() => assembleLiveLanScenario({ scenarioId: "A", preSnapshot: pre, postSnapshot: post, nativeBundle: bundle(), collectorSourceSha256: digest }), /fingerprint/u);
  const stablePost = topologySnapshotRecord("post", capturedTopology(), digest);
  assert.throws(() => assembleLiveLanScenario({ scenarioId: "A", preSnapshot: pre, postSnapshot: stablePost, nativeBundle: { ...bundle(), manifest: { ...bundle().manifest, zeroExternalAttempts: false } }, collectorSourceSha256: digest }), /zero-attempt/u);
});

test("scenario evidence without an emitted sidecar cannot receive runtime credit", async () => {
  const missing = path.join(tmpdir(), `missing-scenario-sidecar-${process.pid}.json`);
  await assert.rejects(validateScenarioBundle(missing, `${missing}.bundle`, scenario("A", "E".repeat(64), digest)), /sidecar is missing/u);
});

test("fixed producer exposes no caller-authored command seam and test output is non-credit", () => {
  assert.ok(fixedCommandDescriptors().every((descriptor) => descriptor.executable && descriptor.args.length > 0));
  const seam = nonCreditTestSeamProof(candidate);
  assert.equal(seam.candidateCommit, candidate.commit);
  assert.equal(seam.candidateTree, candidate.tree);
  assert.equal(seam.result, "NON_CREDIT_TEST_SEAM");
  assert.equal(seam.completeLogs, false);
  const invalid = {
    candidate,
    adaptersOff: { value: adaptersOff() },
    boundary: { value: seam },
    exportRejection: { value: exportsProof() },
    nativeBacking: { value: backing() },
    scenarios: [{ value: scenario("A", "E".repeat(64), digest) }, { value: scenario("B", "F".repeat(64), "9".repeat(64)) }],
  };
  assert.throws(() => assembleIsolationClosure(invalid), /runtime evidence/u);
});

test("production reader blocks forged runtime PASS records before sidecar claims are considered", async () => {
  const directory = mkdtempSync(path.join(tmpdir(), "phase2-forged-runtime-"));
  const proof = path.join(directory, "forged.json");
  try {
    writeFileSync(proof, JSON.stringify({ evidenceKind: "PHASE2_BOUNDARY_FUZZ_RUNTIME_EVIDENCE", result: "PASS", commandSha256: digest, logSha256: digest }), "utf8");
    await assert.rejects(readEvidence(proof), /blocked/u);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

function envelope(kind) { return { candidateCommit: candidate.commit, candidateTree: candidate.tree, commandSha256: command, completeLogs: true, evidenceKind: kind, evidenceManifestSha256: digest, logSha256: log, productionPathExercised: true, result: "PASS", schemaVersion: "1.0", zeroExternalAttempts: true }; }
function adaptersOff() { return { ...envelope("PHASE2_ADAPTERS_OFF_RUNTIME_EVIDENCE"), architecture: "x64", browserExecutableSha256: digest, browserFamily: "chromium", browserRuntimeProduct: "microsoft-edge", browserRuntimeVersion: "140.0.0.0", configurationId: "windows-x64-chromium-packaged-adapters-off", externalAttemptCount: 0, hostNetworkAdaptersDisabled: true, platform: "windows", sideEffectsObserved: false }; }
function boundary() { return { ...envelope("PHASE2_BOUNDARY_FUZZ_RUNTIME_EVIDENCE"), boundaries: REQUIRED_FUZZ_BOUNDARY_IDS.map((boundaryId) => ({ boundaryId, caseCount: DEFAULT_FUZZ_CASES.length, caseIdsSha256: FUZZ_CASE_IDS_SHA256, commandSha256: command, corpusSha256: FUZZ_CORPUS_SHA256, externalAttemptCount: 0, logSha256: log, productionPathExercised: true, result: "PASS", sideEffectsObserved: false })) }; }
function exportsProof() { return { ...envelope("PHASE2_VENDOR_EXPORT_REJECTION_RUNTIME_EVIDENCE"), surfaces: REQUIRED_EXPORT_SURFACE_IDS.map((surfaceId) => ({ closedFormatSet: true, commandSha256: command, deployableArtifactAttemptsRejected: true, externalAttemptCount: 0, logSha256: log, productionPathExercised: true, result: "PASS", sideEffectsObserved: false, surfaceId, vendorArtifactAttemptsRejected: true })) }; }
function backing() { return { ...envelope("PHASE2_NATIVE_BACKING_RUNTIME_EVIDENCE"), architecture: "x64", browserExecutableSha256: digest, browserRuntimeProduct: "microsoft-edge-webview2", browserRuntimeVersion: "140.0.0.0", externalAttemptCount: 0, operations: ["open", "create", "replace"].map((operationId) => ({ attestationVersion: 1, commandSha256: command, fixedLocalBacking: true, logSha256: log, metadataOnlyBeforeAcceptance: true, operationId, productionPathExercised: true, providerBacked: false, redirected: false, remote: false, removable: false, result: "PASS", selectedByteIoBeforeAcceptance: false, sideEffectsObserved: false, special: false, unapprovedHelperEffectObserved: false, unsafeTarget: false, zeroExternalAttempts: true })), platform: "windows", sideEffectsObserved: false }; }
function scenario(scenarioId, fingerprint, manifestSha256) { return { evidenceKind: "WINDOWS_LIVE_LAN_SCENARIO_EVIDENCE", nativeBackingReceipt: { candidateCommit: candidate.commit, candidateTree: candidate.tree, evidenceManifestSha256: manifestSha256, rawHostManifestSha256: manifestSha256 }, nativeEvidenceBundle: Array.from({ length: 7 }, (_, index) => ({ bytes: index + 1, path: `log-${index}`, sha256: digest })), scenario: { architecture: "x64", candidateCommit: candidate.commit, candidateTree: candidate.tree, completeLogs: true, configurationId: "windows-x64-chromium-native-broker-adapters-on", controlledInputSha256: "D".repeat(64), deterministicOutputSha256: "E".repeat(64), evidenceManifestSha256: manifestSha256, externalAttemptCount: 0, platform: "windows", postTopologyFingerprint: fingerprint, preTopologyFingerprint: fingerprint, productionPathExercised: true, result: "PASS", scenarioId, topologyFingerprint: fingerprint, topologyMutationControl: "EXTERNAL_LAB_OR_OPERATOR_CONTROLLED", topologySource: "WINDOWS_LIVE_ADAPTER_SNAPSHOT" }, schemaVersion: "1.0", snapshots: {} }; }
function capturedTopology() { return { adapters: [], profiles: [] }; }
function bundle() { return { manifest: { candidateCommit: candidate.commit, candidateDevelopmentDirty: false, candidateTree: candidate.tree, controlledInputSha256: "D".repeat(64), deterministicOutputSha256: "E".repeat(64), externalAttemptCount: 0, instrumentationComplete: true, productionPathExercised: true, result: "PASS", zeroExternalAttempts: true }, manifestSha256: digest, raw: { fixedLocalBacking: true, instrumentationStatus: "REQUIRES_EXTERNAL_HARNESS", metadataOnlyBeforeAcceptance: true, operations: ["create", "open", "replace"], providerBacked: false, redirected: false, remote: false, removable: false, result: "PASS", selectedByteIoBeforeAcceptance: false, special: false, verificationStage: 4 }, rawBytes: Buffer.from("raw"), files: [] }; }
