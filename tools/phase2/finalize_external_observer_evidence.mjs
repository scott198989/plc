#!/usr/bin/env node

import { createHash } from "node:crypto";
import { constants } from "node:fs";
import { lstat, open, realpath, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  analyzeExternalObserverEvidence,
  fixedObserverFileNames,
  hashExternalObserverBytes,
} from "./external_observer_evidence.mjs";

const scriptPath = fileURLToPath(import.meta.url);
const root = path.resolve(path.dirname(scriptPath), "..", "..");
const nativeBuild = path.join(root, ".phase2-verification", "native-build");
const evidenceRoot = path.join(root, ".phase2-verification", "native-e2e");
const rawPath = path.join(evidenceRoot, "native-gap-free-external-observer-raw.json");
const outputPath = path.join(evidenceRoot, "native-gap-free-external-observer-analysis.json");
const candidateManifestPath = path.join(evidenceRoot, "candidate-package-manifest.json");
const candidateImagePath = path.join(nativeBuild, "package", "GovsPLC.exe");
const launcherPath = path.join(nativeBuild, "Run-Native-E2E.exe");
const observerPath = path.join(nativeBuild, "Run-Phase2-External-Observer.exe");
const observerSourcePath = path.join(root, "tools", "phase2", "windows_external_observer.cpp");
const observerBuildScriptPath = path.join(root, "tools", "phase2", "build_external_observer.mjs");
const observerAnalyzerPath = path.join(root, "tools", "phase2", "external_observer_evidence.mjs");
const observerSourceVerifierPath = path.join(root, "tools", "phase2", "verify_external_observer_source.mjs");
const FILES = fixedObserverFileNames();

const stableJson = (value) => `${JSON.stringify(value, null, 2)}\n`;
const sameWindowsPath = (left, right) => path.resolve(left).toLocaleLowerCase("en-US") ===
  path.resolve(right).toLocaleLowerCase("en-US");

async function openFixedRegularFile(file, maximumBytes) {
  const parent = path.dirname(file);
  const [parentReal, status, fileReal] = await Promise.all([realpath(parent), lstat(file), realpath(file)]);
  if (!status.isFile() || status.isSymbolicLink() || status.size < 1 || status.size > maximumBytes ||
      !sameWindowsPath(parentReal, parent) || !sameWindowsPath(fileReal, file)) {
    throw new Error(`External observer input is not a bounded fixed regular file: ${path.basename(file)}`);
  }
  const handle = await open(file, constants.O_RDONLY);
  const opened = await handle.stat();
  if (!opened.isFile() || opened.size !== status.size ||
      (status.ino !== 0 && opened.ino !== 0 && status.ino !== opened.ino)) {
    await handle.close();
    throw new Error(`External observer input changed while opening: ${path.basename(file)}`);
  }
  return { handle, size: opened.size };
}

async function readFixedFile(file, maximumBytes) {
  const opened = await openFixedRegularFile(file, maximumBytes);
  try {
    const bytes = await opened.handle.readFile();
    if (bytes.byteLength !== opened.size) throw new Error(`Short read: ${path.basename(file)}`);
    return bytes;
  } finally {
    await opened.handle.close();
  }
}

async function hashFixedFile(file, maximumBytes) {
  const opened = await openFixedRegularFile(file, maximumBytes);
  const hash = createHash("sha256");
  let offset = 0;
  try {
    const buffer = Buffer.allocUnsafe(1024 * 1024);
    while (offset < opened.size) {
      const length = Math.min(buffer.byteLength, opened.size - offset);
      const { bytesRead } = await opened.handle.read(buffer, 0, length, offset);
      if (bytesRead <= 0) throw new Error(`Short read: ${path.basename(file)}`);
      hash.update(buffer.subarray(0, bytesRead));
      offset += bytesRead;
    }
    return { bytes: opened.size, sha256: hash.digest("hex").toUpperCase() };
  } finally {
    await opened.handle.close();
  }
}

async function parseJson(bytes, label) {
  try {
    return JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes));
  } catch {
    throw new Error(`${label} is malformed or is not valid UTF-8.`);
  }
}

export async function finalizeFixedExternalObserverEvidence() {
  if (process.platform !== "win32") throw new Error("The Phase 2 ETW evidence finalizer is Windows-only.");
  if (process.argv.slice(2).length !== 0) throw new Error("The Phase 2 ETW evidence finalizer accepts zero arguments.");
  const [rawBytes, candidateManifestBytes, candidateImageBytes, launcherBytes, observerBytes,
    observerSourceBytes, observerBuildScriptBytes, observerAnalyzerSourceBytes,
    observerFinalizerSourceBytes, etlDigest, eventsBytes, metadataBytes, transcriptBytes] =
    await Promise.all([
      readFixedFile(rawPath, 4 * 1024 * 1024),
      readFixedFile(candidateManifestPath, 16 * 1024 * 1024),
      readFixedFile(candidateImagePath, 256 * 1024 * 1024),
      readFixedFile(launcherPath, 256 * 1024 * 1024),
      readFixedFile(observerPath, 256 * 1024 * 1024),
      readFixedFile(observerSourcePath, 4 * 1024 * 1024),
      readFixedFile(observerBuildScriptPath, 4 * 1024 * 1024),
      readFixedFile(observerAnalyzerPath, 4 * 1024 * 1024),
      readFixedFile(scriptPath, 4 * 1024 * 1024),
      readFixedFile(observerSourceVerifierPath, 4 * 1024 * 1024),
      hashFixedFile(path.join(evidenceRoot, FILES.etl), 2 * 1024 * 1024 * 1024),
      readFixedFile(path.join(evidenceRoot, FILES.events), 256 * 1024 * 1024),
      readFixedFile(path.join(evidenceRoot, FILES.metadata), 32 * 1024 * 1024),
      readFixedFile(path.join(evidenceRoot, FILES.transcript), 32 * 1024 * 1024),
    ]);
  const [raw, candidateManifest, metadata] = await Promise.all([
    parseJson(rawBytes, "Raw ETW observer manifest"),
    parseJson(candidateManifestBytes, "Exact candidate manifest"),
    parseJson(metadataBytes, "ETW provider metadata"),
  ]);
  const files = new Map([
    [FILES.etl, etlDigest],
    [FILES.events, eventsBytes],
    [FILES.metadata, metadataBytes],
    [FILES.transcript, transcriptBytes],
  ]);
  const analysis = analyzeExternalObserverEvidence({
    candidateImageBytes,
    candidateManifest,
    candidateManifestBytes,
    files,
    launcherBytes,
    metadata,
    observerAnalyzerSourceBytes,
    observerBuildScriptBytes,
    observerBytes,
    observerFinalizerSourceBytes,
    observerSourceBytes,
    observerSourceVerifierBytes,
    raw,
    rawBytes,
  });
  const outputBytes = Buffer.from(stableJson(analysis), "utf8");
  await writeFile(outputPath, outputBytes, { flag: "wx" });
  return {
    analysisPath: path.relative(root, outputPath).replaceAll("\\", "/"),
    analysisSha256: hashExternalObserverBytes(outputBytes),
    externalAttemptCount: analysis.externalAttemptCount,
    result: analysis.result,
    unknownEventCount: analysis.unknownEventCount,
  };
}

const invokedDirectly = process.argv[1] !== undefined &&
  path.resolve(process.argv[1]) === path.resolve(scriptPath);
if (invokedDirectly) {
  finalizeFixedExternalObserverEvidence()
    .then((summary) => console.log(stableJson(summary).trim()))
    .catch((error) => {
      console.error(error instanceof Error ? error.message : String(error));
      process.exitCode = 1;
    });
}
