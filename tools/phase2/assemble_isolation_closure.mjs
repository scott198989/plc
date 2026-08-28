#!/usr/bin/env node

// This is an external evidence assembler.  It does not build or launch the
// candidate, and it does not mutate adapters, files outside --output, or any
// product configuration.  Hand-authored closure-shaped JSON is intentionally
// not an input: every claim must first arrive in one of the raw runtime forms.

import { createHash } from "node:crypto";
import { lstat, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  FUZZ_CASE_IDS_SHA256,
  FUZZ_CORPUS_SHA256,
  ISOLATION_APPROVAL_DECISION_ID,
  REQUIRED_EXPORT_SURFACE_IDS,
  REQUIRED_FUZZ_BOUNDARY_IDS,
  assessIsolationClosureEvidence,
  stableJson,
} from "./isolation-counterfactual-lib.mjs";
import { validateSnapshot } from "./collect_live_lan_topology.mjs";

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const SHA256 = /^[A-F0-9]{64}$/u;
const GIT_OBJECT = /^[a-f0-9]{40}$/u;
const REQUIRED_COMMAND_LOG_FIELDS = ["candidateCommit", "candidateTree", "commandSha256", "logSha256", "productionPathExercised", "zeroExternalAttempts"];
const FIXED_PRODUCER_COMMAND_IDS = new Set(["foundation-isolation-boundary-fuzz", "plc-core-isolation-boundary-fuzz", "plc-compiler-isolation-boundary-fuzz", "plc-observability-isolation-boundary-fuzz", "plc-system-replay-rejection", "windows-broker-isolation-fuzz", "plc-core-persistence-adversarial"]);
const sha256 = (value) => createHash("sha256").update(value).digest("hex").toUpperCase();

export function assembleIsolationClosure({ candidate, adaptersOff, boundary, exportRejection, nativeBacking, scenarios }) {
  validateCandidate(candidate);
  const adapters = validateAdaptersOff(adaptersOff.value, candidate, adaptersOff.sha256);
  const boundaryCoverage = validateBoundaryEvidence(boundary.value, candidate);
  const exportCoverage = validateExportEvidence(exportRejection.value, candidate);
  const backing = validateNativeBacking(nativeBacking.value, candidate);
  const live = validateLiveLanScenarios(scenarios.map((entry) => entry.value), candidate, backing.runtime);
  const closure = {
    boundaryFuzzCoverage: boundaryCoverage,
    candidateCommit: candidate.commit,
    candidateTree: candidate.tree,
    configurationCoverage: {
      approvalDecisionId: candidate.approvalDecisionId,
      approvalSha256: candidate.approvalSha256,
      approvalStatus: "APPROVED",
      evidenceBindings: [
        live.configurationBinding,
        adapters.configurationBinding,
      ],
      expectedConfigurationIds: ["windows-x64-chromium-native-broker-adapters-on", "windows-x64-chromium-packaged-adapters-off"],
      status: "COMPLETE",
    },
    evidenceKind: "PHASE2_ISOLATION_CLOSURE_INPUT",
    fixedNativeBackingAttestation: backing.attestation,
    liveLanTopologyVariation: live.topology,
    schemaVersion: "1.0",
    vendorDeployableExportRejection: exportCoverage,
  };
  const assessed = assessIsolationClosureEvidence(closure, {
    commit: candidate.commit,
    isolationApprovalDecisionId: candidate.approvalDecisionId,
    isolationApprovalSha256: candidate.approvalSha256,
    tree: candidate.tree,
  });
  if (!assessed.complete) throw new Error(`Assembled closure is non-credit: ${assessed.failures.join("; ")}`);
  return closure;
}

function validateNativeBacking(value, candidate) {
  requireRuntimeEnvelope(value, "PHASE2_NATIVE_BACKING_RUNTIME_EVIDENCE", candidate);
  requireExactFields(value, ["architecture", "browserExecutableSha256", "browserRuntimeProduct", "browserRuntimeVersion", "candidateCommit", "candidateTree", "commandSha256", "completeLogs", "evidenceKind", "evidenceManifestSha256", "externalAttemptCount", "logSha256", "operations", "platform", "productionPathExercised", "result", "schemaVersion", "sideEffectsObserved", "zeroExternalAttempts"], "Native backing runtime evidence");
  if (value.platform !== "windows" || value.architecture !== "x64" || value.externalAttemptCount !== 0 || value.sideEffectsObserved !== false ||
      value.browserRuntimeProduct !== "microsoft-edge-webview2" || !SHA256.test(String(value.browserExecutableSha256)) || !/^[A-Za-z0-9][A-Za-z0-9._+ -]{0,127}$/u.test(String(value.browserRuntimeVersion))) throw new Error("Native backing runtime evidence has incomplete platform/runtime or side-effect proof.");
  const operations = requireRows(value.operations, ["open", "create", "replace"], "operationId", "Native backing runtime evidence");
  for (const row of operations) {
    requireExactFields(row, ["attestationVersion", "commandSha256", "fixedLocalBacking", "logSha256", "metadataOnlyBeforeAcceptance", "operationId", "productionPathExercised", "providerBacked", "redirected", "remote", "removable", "result", "selectedByteIoBeforeAcceptance", "sideEffectsObserved", "special", "unapprovedHelperEffectObserved", "unsafeTarget", "zeroExternalAttempts"], `Native backing ${row?.operationId}`);
    if (row.attestationVersion !== 1 || row.fixedLocalBacking !== true || row.providerBacked !== false || row.remote !== false || row.removable !== false || row.special !== false || row.redirected !== false || row.unsafeTarget !== false || row.metadataOnlyBeforeAcceptance !== true || row.selectedByteIoBeforeAcceptance !== false || row.unapprovedHelperEffectObserved !== false || row.productionPathExercised !== true || row.zeroExternalAttempts !== true || row.sideEffectsObserved !== false || row.result !== "PASS" || !SHA256.test(String(row.commandSha256)) || !SHA256.test(String(row.logSha256))) throw new Error(`Native backing ${row.operationId} is not complete runtime PASS evidence.`);
  }
  return { evidenceManifestSha256: value.evidenceManifestSha256, attestation: { architecture: "x64", candidateCommit: candidate.commit, candidateTree: candidate.tree, complete: true, decisionId: candidate.approvalDecisionId, evidenceManifestSha256: value.evidenceManifestSha256, operations: operations.map(({ commandSha256, logSha256, sideEffectsObserved, zeroExternalAttempts, ...row }) => row), platform: "windows", result: "PASS", schemaVersion: "1.0" }, runtime: value };
}

function validateCandidate(candidate) {
  if (!GIT_OBJECT.test(String(candidate?.commit)) || !GIT_OBJECT.test(String(candidate?.tree)) ||
      candidate?.approvalDecisionId !== ISOLATION_APPROVAL_DECISION_ID || !SHA256.test(String(candidate?.approvalSha256))) {
    throw new Error("Exact candidate identity or approved isolation binding is malformed.");
  }
}

function requireRuntimeEnvelope(value, kind, candidate) {
  if (value === null || typeof value !== "object" || Array.isArray(value) ||
      value.schemaVersion !== "1.0" || value.evidenceKind !== kind || value.result !== "PASS" ||
      value.completeLogs !== true || value.productionPathExercised !== true || value.zeroExternalAttempts !== true ||
      value.candidateCommit !== candidate.commit || value.candidateTree !== candidate.tree) {
    throw new Error(`${kind} is not complete exact-candidate runtime evidence.`);
  }
  for (const field of REQUIRED_COMMAND_LOG_FIELDS) {
    if (field.endsWith("Sha256") && !SHA256.test(String(value[field]))) throw new Error(`${kind} lacks a valid ${field}.`);
  }
  if (!SHA256.test(String(value.evidenceManifestSha256))) throw new Error(`${kind} lacks a valid evidence manifest hash.`);
}

function validateAdaptersOff(value, candidate, inputSha256) {
  requireRuntimeEnvelope(value, "PHASE2_ADAPTERS_OFF_RUNTIME_EVIDENCE", candidate);
  const expected = ["architecture", "browserExecutableSha256", "browserFamily", "browserRuntimeProduct", "browserRuntimeVersion", "candidateCommit", "candidateTree", "commandSha256", "completeLogs", "configurationId", "evidenceKind", "evidenceManifestSha256", "externalAttemptCount", "hostNetworkAdaptersDisabled", "logSha256", "platform", "productionPathExercised", "result", "schemaVersion", "sideEffectsObserved", "zeroExternalAttempts"];
  requireExactFields(value, expected, "Adapters-off evidence");
  if (value.configurationId !== "windows-x64-chromium-packaged-adapters-off" || value.platform !== "windows" || value.architecture !== "x64" ||
      value.browserFamily !== "chromium" || !["google-chrome", "microsoft-edge"].includes(value.browserRuntimeProduct) ||
      !/^[A-Za-z0-9][A-Za-z0-9._+ -]{0,127}$/u.test(String(value.browserRuntimeVersion)) || !SHA256.test(String(value.browserExecutableSha256)) ||
      value.hostNetworkAdaptersDisabled !== true || value.externalAttemptCount !== 0 || value.sideEffectsObserved !== false) {
    throw new Error("Adapters-off evidence is not a genuine zero-attempt, side-effect-free Windows packaged run.");
  }
  return { configurationBinding: {
    architecture: value.architecture, browserExecutableSha256: value.browserExecutableSha256, browserFamily: value.browserFamily,
    browserRuntimeProduct: value.browserRuntimeProduct, browserRuntimeVersion: value.browserRuntimeVersion,
    candidateCommit: value.candidateCommit, candidateTree: value.candidateTree, completeLogs: true,
    configurationId: value.configurationId, evidenceManifestSha256: value.evidenceManifestSha256,
    fileAccessPosture: "packaged-browser-disabled", hostNetworkPosture: "adapters-off", matchesCandidate: true,
    platform: "windows", productionPathExercised: true, result: "PASS", zeroExternalAttempts: true,
  }, inputSha256 };
}

function validateBoundaryEvidence(value, candidate) {
  requireRuntimeEnvelope(value, "PHASE2_BOUNDARY_FUZZ_RUNTIME_EVIDENCE", candidate);
  requireExactFields(value, ["boundaries", "candidateCommit", "candidateTree", "commandSha256", "completeLogs", "evidenceKind", "evidenceManifestSha256", "logSha256", "productionPathExercised", "result", "schemaVersion", "zeroExternalAttempts"], "Boundary fuzz runtime evidence");
  const boundaries = requireRows(value.boundaries, REQUIRED_FUZZ_BOUNDARY_IDS, "boundaryId", "Boundary fuzz runtime evidence");
  for (const row of boundaries) {
    requireExactFields(row, ["boundaryId", "caseCount", "caseIdsSha256", "commandSha256", "corpusSha256", "externalAttemptCount", "logSha256", "productionPathExercised", "result", "sideEffectsObserved"], `Boundary ${row?.boundaryId}`);
    if (row.caseCount !== 27 || row.caseIdsSha256 !== FUZZ_CASE_IDS_SHA256 || row.corpusSha256 !== FUZZ_CORPUS_SHA256 ||
        !SHA256.test(String(row.commandSha256)) || !SHA256.test(String(row.logSha256)) || row.externalAttemptCount !== 0 ||
        row.productionPathExercised !== true || row.sideEffectsObserved !== false || row.result !== "PASS") throw new Error(`Boundary ${row.boundaryId} is not complete runtime PASS evidence.`);
  }
  return { boundaries: boundaries.map(({ commandSha256, logSha256, ...row }) => row), caseCount: 27, caseIdsSha256: FUZZ_CASE_IDS_SHA256, complete: true, corpusSha256: FUZZ_CORPUS_SHA256, result: "PASS", schemaVersion: "1.0" };
}

function validateExportEvidence(value, candidate) {
  requireRuntimeEnvelope(value, "PHASE2_VENDOR_EXPORT_REJECTION_RUNTIME_EVIDENCE", candidate);
  requireExactFields(value, ["candidateCommit", "candidateTree", "commandSha256", "completeLogs", "evidenceKind", "evidenceManifestSha256", "logSha256", "productionPathExercised", "result", "schemaVersion", "surfaces", "zeroExternalAttempts"], "Export runtime evidence");
  const surfaces = requireRows(value.surfaces, REQUIRED_EXPORT_SURFACE_IDS, "surfaceId", "Export runtime evidence");
  for (const row of surfaces) {
    requireExactFields(row, ["closedFormatSet", "commandSha256", "deployableArtifactAttemptsRejected", "externalAttemptCount", "logSha256", "productionPathExercised", "result", "sideEffectsObserved", "surfaceId", "vendorArtifactAttemptsRejected"], `Export surface ${row?.surfaceId}`);
    if (row.closedFormatSet !== true || row.deployableArtifactAttemptsRejected !== true || row.vendorArtifactAttemptsRejected !== true ||
        row.productionPathExercised !== true || row.sideEffectsObserved !== false || row.externalAttemptCount !== 0 || row.result !== "PASS" ||
        !SHA256.test(String(row.commandSha256)) || !SHA256.test(String(row.logSha256))) throw new Error(`Export surface ${row.surfaceId} is not complete runtime rejection evidence.`);
  }
  return { complete: true, result: "PASS", schemaVersion: "1.0", surfaces: surfaces.map(({ commandSha256, externalAttemptCount, logSha256, ...row }) => row) };
}

function validateLiveLanScenarios(values, candidate, nativeRuntime) {
  if (values.length < 2) throw new Error("At least two live-LAN scenario evidence files are required.");
  const scenarios = [];
  let backing = null;
  for (const value of values) {
    requireExactFields(value, ["evidenceKind", "nativeBackingReceipt", "nativeEvidenceBundle", "scenario", "schemaVersion", "snapshots"], "Live-LAN scenario evidence");
    if (value.evidenceKind !== "WINDOWS_LIVE_LAN_SCENARIO_EVIDENCE" || value.schemaVersion !== "1.0" || !Array.isArray(value.nativeEvidenceBundle) || value.nativeEvidenceBundle.length === 0) throw new Error("Live-LAN scenario evidence is malformed.");
    const scenario = value.scenario;
    if (!scenario || scenario.candidateCommit !== candidate.commit || scenario.candidateTree !== candidate.tree || scenario.configurationId !== "windows-x64-chromium-native-broker-adapters-on" ||
        scenario.platform !== "windows" || scenario.architecture !== "x64" || scenario.completeLogs !== true || scenario.externalAttemptCount !== 0 || scenario.productionPathExercised !== true || scenario.result !== "PASS" ||
        scenario.preTopologyFingerprint !== scenario.topologyFingerprint || scenario.postTopologyFingerprint !== scenario.topologyFingerprint ||
        !SHA256.test(String(scenario.evidenceManifestSha256))) throw new Error("Live-LAN scenario does not satisfy exact native adapters-on runtime requirements.");
    const receipt = value.nativeBackingReceipt;
    if (!receipt || receipt.candidateCommit !== candidate.commit || receipt.candidateTree !== candidate.tree || !SHA256.test(String(receipt.evidenceManifestSha256)) || !SHA256.test(String(receipt.rawHostManifestSha256))) throw new Error("Live-LAN scenario lacks an exact native backing receipt.");
    if (backing && backing.evidenceManifestSha256 === receipt.evidenceManifestSha256) throw new Error("Each live-LAN scenario must preserve its own independently finalized native bundle.");
    backing = receipt;
    scenarios.push(scenario);
  }
  const ids = new Set(scenarios.map((row) => row.scenarioId)); const topologies = new Set(scenarios.map((row) => row.topologyFingerprint));
  const manifests = new Set(values.map((value) => value.nativeBackingReceipt.evidenceManifestSha256)); const receipts = new Set(values.map((value) => value.nativeBackingReceipt.rawHostManifestSha256));
  const inputs = new Set(scenarios.map((row) => row.controlledInputSha256)); const outputs = new Set(scenarios.map((row) => row.deterministicOutputSha256));
  if (ids.size !== scenarios.length || topologies.size < 2 || manifests.size !== scenarios.length || receipts.size !== scenarios.length || inputs.size !== 1 || outputs.size !== 1) throw new Error("Live-LAN scenarios do not show distinct stable topologies and independently finalized runs with invariant controlled input/output.");
  return {
    configurationBinding: { architecture: "x64", browserExecutableSha256: nativeRuntime.browserExecutableSha256, browserFamily: "chromium", browserRuntimeProduct: nativeRuntime.browserRuntimeProduct, browserRuntimeVersion: nativeRuntime.browserRuntimeVersion, candidateCommit: candidate.commit, candidateTree: candidate.tree, completeLogs: true, configurationId: "windows-x64-chromium-native-broker-adapters-on", evidenceManifestSha256: scenarios[0].evidenceManifestSha256, fileAccessPosture: "native-broker", hostNetworkPosture: "adapters-on-controlled-lan", matchesCandidate: true, platform: "windows", productionPathExercised: true, result: "PASS", zeroExternalAttempts: true },
    topology: { applicationNetworkCapabilityPresent: false, complete: true, discoveryApiSurfacePresent: false, result: "PASS", scenarios, schemaVersion: "1.0" },
  };
}

function requireRows(value, required, key, label) { if (!Array.isArray(value) || value.length !== required.length) throw new Error(`${label} must provide exactly ${required.length} rows.`); const ids = value.map((row) => row?.[key]); if (new Set(ids).size !== ids.length || required.some((id) => !ids.includes(id))) throw new Error(`${label} does not exactly enumerate required rows.`); return value; }
function requireExactFields(value, expected, label) { const observed = value && typeof value === "object" && !Array.isArray(value) ? Object.keys(value).sort() : []; const wanted = [...expected].sort(); if (observed.length !== wanted.length || observed.some((key, index) => key !== wanted[index])) throw new Error(`${label} contains missing or unrecognized fields.`); }

export async function readEvidence(file) {
  const absolute = path.resolve(file); const status = await lstat(absolute);
  if (!status.isFile() || status.isSymbolicLink() || status.size < 1 || status.size > 64 * 1024 * 1024) throw new Error(`Evidence is not a bounded regular file: ${absolute}`);
  const bytes = await readFile(absolute); let value;
  try { value = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)); } catch { throw new Error(`Evidence JSON is malformed: ${absolute}`); }
  if (/^PHASE2_(?:ADAPTERS_OFF|BOUNDARY_FUZZ|NATIVE_BACKING|VENDOR_EXPORT_REJECTION)_RUNTIME_EVIDENCE$/u.test(String(value?.evidenceKind ?? ""))) {
    throw new Error("Runtime closure assembly is blocked: no approved external observer currently produces these raw proof records.");
  }
  if (String(value?.evidenceKind ?? "").startsWith("PHASE2_")) await validateContentBundle(`${absolute}.bundle.json`, path.dirname(absolute), value);
  if (value?.evidenceKind === "WINDOWS_LIVE_LAN_SCENARIO_EVIDENCE") await validateScenarioBundle(`${absolute}.bundle.json`, `${absolute}.bundle`, value);
  return { path: absolute, sha256: sha256(bytes), value };
}

export async function validateScenarioBundle(sidecarPath, bundleRoot, record) {
  let sidecarStatus; try { sidecarStatus = await lstat(sidecarPath); } catch { throw new Error("Live-LAN scenario content sidecar is missing."); }
  if (!sidecarStatus.isFile() || sidecarStatus.isSymbolicLink() || sidecarStatus.size < 1 || sidecarStatus.size > 2 * 1024 * 1024) throw new Error("Live-LAN scenario content sidecar is missing.");
  let sidecar; try { sidecar = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(await readFile(sidecarPath))); } catch { throw new Error("Live-LAN scenario content sidecar is malformed."); }
  requireExactFields(sidecar, ["collectorSourceSha256", "evidenceKind", "files", "schemaVersion"], "Live-LAN scenario content sidecar");
  if (sidecar.schemaVersion !== "1.0" || sidecar.evidenceKind !== "WINDOWS_LIVE_LAN_SCENARIO_CONTENT_BUNDLE" || !SHA256.test(String(sidecar.collectorSourceSha256)) || !Array.isArray(sidecar.files) || sidecar.files.length < 5) throw new Error("Live-LAN scenario content inventory is incomplete.");
  const byPath = new Map();
  for (const row of sidecar.files) {
    requireExactFields(row, ["bytes", "path", "sha256"], "Live-LAN scenario content file");
    if (typeof row.path !== "string" || !/^(?:collector-source\.mjs|(?:pre|post)-snapshot\.json|native\/[A-Za-z0-9][A-Za-z0-9._-]{0,127})$/u.test(row.path) || !Number.isSafeInteger(row.bytes) || row.bytes < 1 || !SHA256.test(String(row.sha256)) || byPath.has(row.path)) throw new Error("Live-LAN scenario content inventory is invalid.");
    const file = path.join(bundleRoot, ...row.path.split("/")); const status = await lstat(file); if (!status.isFile() || status.isSymbolicLink() || status.size !== row.bytes) throw new Error(`Live-LAN scenario content file is invalid: ${row.path}`);
    const bytes = await readFile(file); if (sha256(bytes) !== row.sha256) throw new Error(`Live-LAN scenario content hash drifted: ${row.path}`); byPath.set(row.path, { bytes, row });
  }
  for (const name of ["collector-source.mjs", "pre-snapshot.json", "post-snapshot.json", "native/native-platform-evidence-manifest.json", "native/native-run-raw.json"]) if (!byPath.has(name)) throw new Error(`Live-LAN scenario content is missing ${name}.`);
  const sourceSha256 = sha256(byPath.get("collector-source.mjs").bytes);
  if (sourceSha256 !== sidecar.collectorSourceSha256) throw new Error("Live-LAN collector source hash drifted.");
  const snapshots = {};
  for (const [name, boundary] of [["pre-snapshot.json", "pre"], ["post-snapshot.json", "post"]]) {
    let snapshot; try { snapshot = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(byPath.get(name).bytes)); } catch { throw new Error(`Live-LAN ${boundary} snapshot is malformed.`); }
    try { validateSnapshot(snapshot, boundary, sourceSha256); } catch (error) { throw new Error(`Live-LAN ${boundary} snapshot does not bind a canonical source-verified Windows capture: ${error instanceof Error ? error.message : String(error)}`); }
    snapshots[boundary] = snapshot;
  }
  let finalManifest; let raw;
  try { finalManifest = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(byPath.get("native/native-platform-evidence-manifest.json").bytes)); raw = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(byPath.get("native/native-run-raw.json").bytes)); } catch { throw new Error("Live-LAN native bundle JSON is malformed."); }
  const finalSha = sha256(byPath.get("native/native-platform-evidence-manifest.json").bytes); const rawSha = sha256(byPath.get("native/native-run-raw.json").bytes);
  const inventory = Array.isArray(finalManifest?.evidenceFiles) ? finalManifest.evidenceFiles : [];
  if (inventory.length === 0 || new Set(inventory.map((row) => row?.path)).size !== inventory.length) throw new Error("Finalized native manifest inventory is malformed.");
  const expectedBundle = inventory.map((row) => ({ bytes: row?.bytes, path: row?.path, sha256: row?.sha256 }));
  const recordedBundle = record?.nativeEvidenceBundle;
  if (JSON.stringify(recordedBundle) !== JSON.stringify(expectedBundle)) throw new Error("Live-LAN scenario native evidence inventory does not exactly match the finalized manifest.");
  if (record?.scenario?.evidenceManifestSha256 !== finalSha || record?.nativeBackingReceipt?.evidenceManifestSha256 !== finalSha || record?.nativeBackingReceipt?.rawHostManifestSha256 !== rawSha || record?.scenario?.preTopologyFingerprint !== snapshots.pre.topologyFingerprint || record?.scenario?.postTopologyFingerprint !== snapshots.post.topologyFingerprint || finalManifest?.candidateCommit !== record?.scenario?.candidateCommit || finalManifest?.candidateTree !== record?.scenario?.candidateTree || finalManifest?.controlledInputSha256 !== record?.scenario?.controlledInputSha256 || finalManifest?.deterministicOutputSha256 !== record?.scenario?.deterministicOutputSha256 || raw?.verificationStage !== 4 || raw?.instrumentationStatus !== "REQUIRES_EXTERNAL_HARNESS") throw new Error("Live-LAN scenario record is not bound to copied snapshots and finalized native bytes.");
  for (const row of expectedBundle) {
    if (typeof row.path !== "string" || !/^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/u.test(row.path) || !Number.isSafeInteger(row.bytes) || row.bytes < 1 || !SHA256.test(String(row.sha256))) throw new Error("Finalized native manifest has an unsafe evidence row.");
    const copied = byPath.get(`native/${row.path}`); if (!copied || copied.row.bytes !== row.bytes || copied.row.sha256 !== row.sha256) throw new Error(`Live-LAN scenario content omits or changes native log ${row.path}.`);
  }
}

async function validateContentBundle(bundlePath, root, proof) {
  const status = await lstat(bundlePath);
  if (!status.isFile() || status.isSymbolicLink() || status.size < 1 || status.size > 2 * 1024 * 1024) throw new Error(`Runtime proof content bundle is missing: ${bundlePath}`);
  let bundle;
  try { bundle = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(await readFile(bundlePath))); } catch { throw new Error(`Runtime proof content bundle is malformed: ${bundlePath}`); }
  requireExactFields(bundle, ["command", "evidenceKind", "files", "log", "schemaVersion"], "Runtime proof content bundle");
  if (bundle.schemaVersion !== "1.0" || bundle.evidenceKind !== "PHASE2_EXTERNAL_RUNTIME_CONTENT_BUNDLE" || !Array.isArray(bundle.files) || bundle.files.length < 2) throw new Error("Runtime proof content bundle is incomplete.");
  const byPath = new Map();
  for (const row of bundle.files) {
    requireExactFields(row, ["bytes", "path", "sha256"], "Runtime proof content file");
    if (typeof row.path !== "string" || !/^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/u.test(row.path) || !Number.isSafeInteger(row.bytes) || row.bytes < 1 || !SHA256.test(String(row.sha256)) || byPath.has(row.path)) throw new Error("Runtime proof content inventory is invalid.");
    const file = path.join(root, row.path); const fileStatus = await lstat(file);
    if (!fileStatus.isFile() || fileStatus.isSymbolicLink() || fileStatus.size !== row.bytes) throw new Error(`Runtime proof content file is invalid: ${row.path}`);
    const actual = sha256(await readFile(file)); if (actual !== row.sha256) throw new Error(`Runtime proof content hash drifted: ${row.path}`); byPath.set(row.path, row);
  }
  for (const field of ["command", "log"]) {
    const pointer = bundle[field]; requireExactFields(pointer, ["path", "sha256"], `Runtime proof ${field} pointer`);
    const row = byPath.get(pointer.path); if (!row || row.sha256 !== pointer.sha256 || pointer.sha256 !== proof[`${field}Sha256`]) throw new Error(`Runtime proof ${field} is not content-addressed by its manifest.`);
  }
  let descriptor; let result;
  try { descriptor = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode((await readFile(path.join(root, bundle.command.path))))); result = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode((await readFile(path.join(root, bundle.log.path))))); } catch { throw new Error("Runtime proof command descriptor or structured result is malformed."); }
  if (descriptor?.schemaVersion !== "1.0" || descriptor?.evidenceKind !== "PHASE2_FIXED_COMMAND_DESCRIPTOR" || !FIXED_PRODUCER_COMMAND_IDS.has(descriptor?.commandId) || result?.schemaVersion !== "1.0" || result?.evidenceKind !== "PHASE2_FIXED_COMMAND_RESULT" || result?.result !== "PASS" || result?.exitCode !== 0 || result?.commandId !== descriptor.commandId || !Array.isArray(result?.testIds) || !result.testIds.includes(descriptor.commandId) || !SHA256.test(String(result?.transcriptSha256))) throw new Error("Runtime proof bundle lacks an expected fixed-command structured PASS result.");
  if (proof.evidenceKind === "PHASE2_ADAPTERS_OFF_RUNTIME_EVIDENCE") {
    for (const name of ["pre-adapters.json", "post-adapters.json"]) {
      const row = byPath.get(name); if (!row) throw new Error(`Adapters-off proof is missing measured ${name}.`);
      let snapshot; try { snapshot = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(await readFile(path.join(root, name)))); } catch { throw new Error(`Adapters-off snapshot is malformed: ${name}`); }
      const observed = snapshot?.topology?.adapters ?? snapshot?.adapters;
      if (!Array.isArray(observed) || observed.some((adapter) => /^(up|connected)$/iu.test(String(adapter?.status ?? adapter?.Status)) || /^(connected|1)$/iu.test(String(adapter?.mediaState ?? adapter?.MediaConnectionState)))) throw new Error(`Adapters-off snapshot does not prove every adapter disabled or absent: ${name}`);
    }
  }
}
function parse(argv) { const scalar = new Map([["--adapters-off-raw", "adaptersOff"], ["--approval-decision-id", "approvalDecisionId"], ["--approval-sha256", "approvalSha256"], ["--boundary-fuzz-raw", "boundary"], ["--candidate-commit", "commit"], ["--candidate-tree", "tree"], ["--export-rejection-raw", "exportRejection"], ["--native-backing-raw", "nativeBacking"], ["--output", "output"]]); const result = { scenarios: [] }; for (let i=0;i<argv.length;i+=2) { const flag=argv[i], value=argv[i+1]; if (value===undefined) throw new Error(`Incomplete argument: ${flag}`); if (flag === "--live-lan-scenario") { result.scenarios.push(value); continue; } const key=scalar.get(flag); if (!key || result[key]!==undefined) throw new Error(`Unknown or duplicate argument: ${flag}`); result[key]=value; } for(const key of [...scalar.values()]) if(!result[key]) throw new Error(`Missing required argument: ${key}`); if(result.scenarios.length<2) throw new Error("At least two --live-lan-scenario arguments are required."); return result; }
async function main() { const options=parse(process.argv.slice(2)); const candidate={commit:options.commit,tree:options.tree,approvalDecisionId:options.approvalDecisionId,approvalSha256:options.approvalSha256}; const [adaptersOff,boundary,exportRejection,nativeBacking,...scenarios]=await Promise.all([readEvidence(options.adaptersOff),readEvidence(options.boundary),readEvidence(options.exportRejection),readEvidence(options.nativeBacking),...options.scenarios.map(readEvidence)]); const closure=assembleIsolationClosure({candidate,adaptersOff,boundary,exportRejection,nativeBacking,scenarios}); await writeFile(path.resolve(options.output),stableJson(closure),{encoding:"utf8",flag:"wx"}); console.log(stableJson({ candidateCommit:candidate.commit,candidateTree:candidate.tree,closureSha256:sha256(Buffer.from(stableJson(closure))),sourceSha256:sha256(await readFile(SCRIPT_PATH)) }).trim()); }
if (process.argv[1] && path.resolve(process.argv[1]) === SCRIPT_PATH) main().catch((error)=>{console.error(error instanceof Error?error.message:String(error));process.exitCode=1;});
