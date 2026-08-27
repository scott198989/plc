#!/usr/bin/env node

import { randomUUID } from "node:crypto";
import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const EXIT_TOOL_ERROR = 2;
const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");

function fail(message) {
  console.error(`ERROR VER-RUN-0001 ${message}`);
  process.exit(EXIT_TOOL_ERROR);
}

function argumentValue(name, fallback) {
  const index = process.argv.indexOf(name);
  if (index < 0) return fallback;
  if (index + 1 >= process.argv.length) fail(`${name} requires a value`);
  return process.argv[index + 1];
}

function resolvePinnedPython() {
  const resolver = join(root, "tools", "phase1", "run_pinned_python.mjs");
  const result = spawnSync(process.execPath, [resolver, "--resolve-only"], {
    cwd: root,
    encoding: "utf8",
    env: process.env,
    windowsHide: true,
    timeout: 15_000,
  });
  if (result.status !== 0) {
    const detail = `${result.stdout ?? ""}${result.stderr ?? ""}`.trim();
    fail(`Unable to resolve pinned Python 3.13.12${detail ? `: ${detail}` : ""}`);
  }
  const executable = result.stdout.trim();
  if (!executable) fail("Pinned Python resolver returned no executable path");
  return executable;
}

function resolvePowerShell() {
  const candidates = process.platform === "win32" ? ["pwsh.exe", "powershell.exe"] : ["pwsh"];
  for (const candidate of candidates) {
    const probe = spawnSync(
      candidate,
      ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", "$PSVersionTable.PSVersion.ToString()"],
      { encoding: "utf8", windowsHide: true, timeout: 10_000 },
    );
    if (probe.status === 0) return candidate;
  }
  fail("PowerShell is unavailable; the mutation harness requires PowerShell 5.1 or newer");
}

const baselineRef = argumentValue("--baseline-ref", "HEAD");
const scratchRoot = join(tmpdir(), `govs-plc-phase1-mutations-${randomUUID()}`);
const resolvedScratchRoot = resolve(scratchRoot);
const resolvedTempRoot = resolve(tmpdir());
const scratchRelative = relative(resolvedTempRoot, resolvedScratchRoot);
if (
  !scratchRelative ||
  scratchRelative.startsWith("..") ||
  isAbsolute(scratchRelative) ||
  dirname(scratchRelative) !== "." ||
  !basename(scratchRelative).startsWith("govs-plc-phase1-mutations-")
) {
  fail(`Refusing unsafe mutation scratch path: ${resolvedScratchRoot}`);
}
const python = resolvePinnedPython();
const powerShell = resolvePowerShell();
const harness = join(root, "tests", "phase1", "run_phase1_mutations.ps1");
const evidencePath = join(root, ".phase1-verification", "mutations", "mutation-results.json");
const scratchResultPath = join(resolvedScratchRoot, "mutation-results.json");
mkdirSync(dirname(evidencePath), { recursive: true });
rmSync(evidencePath, { force: true });

let child;
let summary = null;
try {
  child = spawnSync(
    powerShell,
    [
      "-NoLogo",
      "-NoProfile",
      "-NonInteractive",
      "-ExecutionPolicy",
      "Bypass",
      "-File",
      harness,
      "-RepositoryRoot",
      root,
      "-BaselineRef",
      baselineRef,
      "-ScratchRoot",
      resolvedScratchRoot,
      "-PythonExe",
      python,
    ],
    {
      cwd: root,
      env: process.env,
      stdio: "inherit",
      windowsHide: true,
      timeout: 30 * 60 * 1000,
    },
  );
  if (existsSync(scratchResultPath)) {
    copyFileSync(scratchResultPath, evidencePath);
    summary = JSON.parse(readFileSync(evidencePath, "utf8"));
  }
} finally {
  rmSync(resolvedScratchRoot, { recursive: true, force: true, maxRetries: 3, retryDelay: 250 });
}

if (summary) {
  console.log(`EVIDENCE_PATH=${evidencePath}`);
  console.log(`MUTATION_SCORE=${summary.intendedMutationDetections}/${summary.prescribedMutationCount}`);
  console.log(`MANIFEST_TAMPER_TEST=${summary.manifestTamperTestPassed}`);
  console.log(`OVERALL_PASS=${summary.overallPassed}`);
  console.log(`SCRATCH_REMOVED=${!existsSync(resolvedScratchRoot)}`);
}
if (child?.error) fail(`Unable to run mutation harness: ${child.error.message}`);
if (child?.signal) fail(`Mutation harness terminated by signal ${child.signal}`);
if (!summary) fail("Mutation harness produced no durable mutation-results.json evidence");
process.exit(Number.isInteger(child.status) ? child.status : EXIT_TOOL_ERROR);
