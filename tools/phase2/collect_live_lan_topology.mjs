#!/usr/bin/env node

// External verification tooling only.  This program deliberately never builds,
// starts, configures, or otherwise controls the candidate or a network adapter.

import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { lstat, readFile, writeFile } from "node:fs/promises";
import { arch, platform } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

import { stableJson } from "./isolation-counterfactual-lib.mjs";

const execFileAsync = promisify(execFile);
const SCRIPT_PATH = fileURLToPath(import.meta.url);
const SHA256 = /^[A-F0-9]{64}$/u;
const GIT_OBJECT = /^[a-f0-9]{40}$/u;
const REQUIRED_NATIVE_FILES = Object.freeze([
  "candidate-package-manifest.json",
  "native-launcher-transcript.log",
  "native-netlog.json",
  "native-network-analysis.json",
  "native-platform-observer-manifest.json",
  "native-process-endpoints.json",
  "native-run-raw.json",
]);

const sha256 = (value) => createHash("sha256").update(value).digest("hex").toUpperCase();

export function canonicalTopology(value) {
  if (!Array.isArray(value?.adapters) || !Array.isArray(value?.profiles)) {
    throw new Error("Windows topology capture is malformed.");
  }
  const normalize = (record, fields) => Object.fromEntries(fields.map((field) => [field, String(record?.[field] ?? "")]));
  const adapters = value.adapters.map((adapter) => ({
    ...normalize(adapter, ["interfaceIndex", "interfaceGuid", "name", "description", "status", "mediaState", "classification", "macAddress", "linkSpeed"]),
    dnsServers: [...new Set((adapter.dnsServers ?? []).map(String))].sort(),
    gateways: [...new Set((adapter.gateways ?? []).map(String))].sort(),
    unicast: (adapter.unicast ?? []).map((address) => normalize(address, ["address", "family", "prefixLength"])).sort((a, b) => stableJson(a).localeCompare(stableJson(b))),
  })).sort((a, b) => `${a.interfaceGuid}\0${a.interfaceIndex}`.localeCompare(`${b.interfaceGuid}\0${b.interfaceIndex}`));
  const profiles = value.profiles.map((profile) => normalize(profile, ["interfaceGuid", "interfaceIndex", "name", "networkCategory"])).sort((a, b) => stableJson(a).localeCompare(stableJson(b)));
  return { adapters, profiles };
}

export function topologySnapshotRecord(boundary, captured, collectorSourceSha256) {
  if (!/[a-z][a-z0-9-]{0,63}/u.test(String(boundary))) throw new Error("Topology boundary must be a bounded lowercase identifier.");
  if (!SHA256.test(String(collectorSourceSha256))) throw new Error("Collector source binding is malformed.");
  const topology = canonicalTopology(captured);
  return {
    architecture: "x64",
    captureBoundary: boundary,
    collectorSourceSha256,
    evidenceKind: "WINDOWS_LIVE_ADAPTER_SNAPSHOT",
    platform: "windows",
    schemaVersion: "1.0",
    topology,
    topologyFingerprint: sha256(Buffer.from(stableJson(topology), "utf8")),
    topologySource: "WINDOWS_LIVE_ADAPTER_SNAPSHOT",
  };
}

export function assembleLiveLanScenario({ scenarioId, preSnapshot, postSnapshot, nativeBundle, collectorSourceSha256 }) {
  if (!/^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/u.test(String(scenarioId))) throw new Error("Scenario ID is malformed.");
  validateSnapshot(preSnapshot, "pre", collectorSourceSha256);
  validateSnapshot(postSnapshot, "post", collectorSourceSha256);
  if (preSnapshot?.topologyFingerprint !== postSnapshot?.topologyFingerprint) throw new Error("Topology changed during the product run; the scenario is unstable.");
  const manifest = nativeBundle?.manifest;
  if (!manifest || manifest.result !== "PASS" || manifest.instrumentationComplete !== true ||
      manifest.zeroExternalAttempts !== true || manifest.externalAttemptCount !== 0 ||
      manifest.productionPathExercised !== true || manifest.candidateDevelopmentDirty !== false) {
    throw new Error("Finalized native bundle is not complete zero-attempt production-path PASS evidence.");
  }
  const raw = nativeBundle.raw;
  if (!raw || raw.result !== "PASS" || raw.fixedLocalBacking !== true || raw.providerBacked !== false ||
      raw.remote !== false || raw.removable !== false || raw.special !== false || raw.redirected !== false ||
      raw.metadataOnlyBeforeAcceptance !== true || raw.selectedByteIoBeforeAcceptance !== false || raw.verificationStage !== 4 ||
      raw.instrumentationStatus !== "REQUIRES_EXTERNAL_HARNESS" ||
      JSON.stringify(raw.operations) !== JSON.stringify(["create", "open", "replace"])) {
    throw new Error("Finalized native bundle does not retain the complete native backing receipt.");
  }
  for (const field of ["candidateCommit", "candidateTree"]) if (!GIT_OBJECT.test(String(manifest[field]))) throw new Error(`Native bundle ${field} is malformed.`);
  for (const field of ["controlledInputSha256", "deterministicOutputSha256"]) if (!SHA256.test(String(manifest[field]))) throw new Error(`Native bundle ${field} is malformed.`);
  return {
    evidenceKind: "WINDOWS_LIVE_LAN_SCENARIO_EVIDENCE",
    nativeBackingReceipt: {
      candidateCommit: manifest.candidateCommit,
      candidateTree: manifest.candidateTree,
      evidenceManifestSha256: nativeBundle.manifestSha256,
      rawHostManifestSha256: sha256(nativeBundle.rawBytes),
    },
    nativeEvidenceBundle: nativeBundle.files,
    scenario: {
      architecture: "x64",
      candidateCommit: manifest.candidateCommit,
      candidateTree: manifest.candidateTree,
      completeLogs: true,
      configurationId: "windows-x64-chromium-native-broker-adapters-on",
      controlledInputSha256: manifest.controlledInputSha256,
      deterministicOutputSha256: manifest.deterministicOutputSha256,
      evidenceManifestSha256: nativeBundle.manifestSha256,
      externalAttemptCount: 0,
      platform: "windows",
      postTopologyFingerprint: postSnapshot.topologyFingerprint,
      preTopologyFingerprint: preSnapshot.topologyFingerprint,
      productionPathExercised: true,
      result: "PASS",
      scenarioId,
      topologyFingerprint: preSnapshot.topologyFingerprint,
      topologyMutationControl: "EXTERNAL_LAB_OR_OPERATOR_CONTROLLED",
      topologySource: "WINDOWS_LIVE_ADAPTER_SNAPSHOT",
    },
    schemaVersion: "1.0",
    snapshots: { post: postSnapshot, pre: preSnapshot },
  };
}

function validateSnapshot(snapshot, boundary, collectorSourceSha256) {
  const required = ["architecture", "captureBoundary", "collectorSourceSha256", "evidenceKind", "platform", "schemaVersion", "topology", "topologyFingerprint", "topologySource"].sort();
  const observed = snapshot !== null && typeof snapshot === "object" && !Array.isArray(snapshot) ? Object.keys(snapshot).sort() : [];
  if (observed.length !== required.length || observed.some((key, index) => key !== required[index]) ||
      snapshot.schemaVersion !== "1.0" || snapshot.evidenceKind !== "WINDOWS_LIVE_ADAPTER_SNAPSHOT" || snapshot.platform !== "windows" ||
      snapshot.architecture !== "x64" || snapshot.topologySource !== "WINDOWS_LIVE_ADAPTER_SNAPSHOT" || snapshot.captureBoundary !== boundary ||
      !SHA256.test(String(snapshot.collectorSourceSha256)) || snapshot.collectorSourceSha256 !== collectorSourceSha256) {
    throw new Error(`Live-LAN ${boundary} snapshot is not a source-bound Windows topology capture.`);
  }
  const canonical = canonicalTopology(snapshot.topology);
  if (snapshot.topologyFingerprint !== sha256(Buffer.from(stableJson(canonical), "utf8"))) {
    throw new Error(`Live-LAN ${boundary} topology fingerprint does not bind its canonical snapshot.`);
  }
}

async function readBoundedJson(file, maximum = 32 * 1024 * 1024) {
  const status = await lstat(file);
  if (!status.isFile() || status.isSymbolicLink() || status.size < 1 || status.size > maximum) throw new Error(`Evidence input is not a bounded regular file: ${path.basename(file)}`);
  const bytes = await readFile(file);
  try { return { bytes, value: JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)) }; }
  catch (error) { throw new Error(`Evidence JSON is malformed: ${error instanceof Error ? error.message : String(error)}`); }
}

async function readNativeBundle(directory) {
  const root = path.resolve(directory);
  const { bytes: manifestBytes, value: manifest } = await readBoundedJson(path.join(root, "native-platform-evidence-manifest.json"));
  const { bytes: rawBytes, value: raw } = await readBoundedJson(path.join(root, "native-run-raw.json"));
  const rows = manifest.evidenceFiles;
  if (!Array.isArray(rows) || rows.length !== REQUIRED_NATIVE_FILES.length || new Set(rows.map((row) => row?.path)).size !== rows.length) throw new Error("Finalized native bundle has an incomplete log inventory.");
  const files = [];
  for (const name of REQUIRED_NATIVE_FILES) {
    const row = rows.find((candidate) => candidate?.path === name);
    if (!row || !SHA256.test(String(row.sha256)) || !Number.isSafeInteger(row.bytes) || row.bytes < 1) throw new Error(`Finalized native bundle is missing a valid ${name} row.`);
    const status = await lstat(path.join(root, name));
    if (!status.isFile() || status.isSymbolicLink() || status.size !== row.bytes) throw new Error(`Finalized native bundle log is invalid: ${name}`);
    const bytes = await readFile(path.join(root, name));
    if (sha256(bytes) !== row.sha256) throw new Error(`Finalized native bundle log hash drifted: ${name}`);
    files.push({ bytes: row.bytes, path: name, sha256: row.sha256 });
  }
  return { files, manifest, manifestSha256: sha256(manifestBytes), raw, rawBytes };
}

async function captureWindowsTopology() {
  if (platform() !== "win32" || arch() !== "x64") throw new Error("Live-LAN capture is supported only on Windows x64.");
  // Read-only cmdlets only; no adapter mutation, probing, resolving, or connection occurs here.
  const command = "$ErrorActionPreference='Stop'; $a=Get-NetAdapter -IncludeHidden | ForEach-Object {$i=$_.ifIndex; [pscustomobject]@{interfaceIndex=$i;interfaceGuid=$_.InterfaceGuid.ToString();name=$_.Name;description=$_.InterfaceDescription;status=$_.Status.ToString();mediaState=$_.MediaConnectionState.ToString();classification=if($_.Virtual){'virtual'}else{'physical'};macAddress=$_.MacAddress;linkSpeed=$_.LinkSpeed;unicast=@(Get-NetIPAddress -InterfaceIndex $i -ErrorAction SilentlyContinue | ForEach-Object {[pscustomobject]@{address=$_.IPAddress;family=$_.AddressFamily.ToString();prefixLength=$_.PrefixLength}});gateways=@(Get-NetRoute -InterfaceIndex $i -DestinationPrefix '0.0.0.0/0','::/0' -ErrorAction SilentlyContinue | ForEach-Object {$_.NextHop});dnsServers=@(Get-DnsClientServerAddress -InterfaceIndex $i -ErrorAction SilentlyContinue | ForEach-Object {$_.ServerAddresses})}}; $p=Get-NetConnectionProfile | ForEach-Object {$i=$_.InterfaceIndex;$g=($a | Where-Object {$_.interfaceIndex -eq $i} | Select-Object -First 1).interfaceGuid;[pscustomobject]@{interfaceGuid=$g;interfaceIndex=$i;name=$_.Name;networkCategory=$_.NetworkCategory.ToString()}}; [pscustomobject]@{adapters=@($a);profiles=@($p)} | ConvertTo-Json -Depth 8 -Compress";
  const { stdout } = await execFileAsync("powershell.exe", ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", command], { maxBuffer: 8 * 1024 * 1024, windowsHide: true });
  return JSON.parse(stdout);
}

async function main() {
  const [command, ...rest] = process.argv.slice(2);
  const options = parseOptions(rest);
  if (command === "snapshot") {
    const record = topologySnapshotRecord(
      requireOption(options, "boundary"),
      await captureWindowsTopology(),
      sha256(await readFile(SCRIPT_PATH)),
    );
    await writeExclusive(requireOption(options, "output"), record);
    console.log(stableJson({ topologyFingerprint: record.topologyFingerprint }).trim());
    return;
  }
  if (command === "assemble-scenario") {
    const [pre, post] = await Promise.all([readBoundedJson(requireOption(options, "pre")), readBoundedJson(requireOption(options, "post"))]);
    const record = assembleLiveLanScenario({ scenarioId: requireOption(options, "scenarioId"), preSnapshot: pre.value, postSnapshot: post.value, nativeBundle: await readNativeBundle(requireOption(options, "nativeBundle")), collectorSourceSha256: sha256(await readFile(SCRIPT_PATH)) });
    await writeExclusive(requireOption(options, "output"), record);
    console.log(stableJson({ evidenceManifestSha256: record.scenario.evidenceManifestSha256, scenarioId: record.scenario.scenarioId, topologyFingerprint: record.scenario.topologyFingerprint }).trim());
    return;
  }
  throw new Error("Use either snapshot or assemble-scenario.");
}

function parseOptions(argv) {
  const keys = new Map([["--boundary", "boundary"], ["--native-bundle", "nativeBundle"], ["--output", "output"], ["--post", "post"], ["--pre", "pre"], ["--scenario-id", "scenarioId"]]);
  const result = {};
  for (let i = 0; i < argv.length; i += 2) { const key = keys.get(argv[i]); if (!key || result[key] !== undefined || argv[i + 1] === undefined) throw new Error(`Unknown, duplicate, or incomplete argument: ${String(argv[i])}`); result[key] = argv[i + 1]; }
  return result;
}
function requireOption(options, key) { if (!options[key]) throw new Error(`Missing required argument: ${key}`); return options[key]; }
async function writeExclusive(file, value) { await writeFile(path.resolve(file), stableJson(value), { encoding: "utf8", flag: "wx" }); }

if (process.argv[1] && path.resolve(process.argv[1]) === SCRIPT_PATH) main().catch((error) => { console.error(error instanceof Error ? error.message : String(error)); process.exitCode = 1; });
