#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { join } from "node:path";

const EXPECTED_VERSION = "3.13.12";
const EXIT_TOOL_ERROR = 2;

function fail(message) {
  console.error(`ERROR VER-RUN-0001 ${message}`);
  process.exit(EXIT_TOOL_ERROR);
}

function candidatePythons() {
  const candidates = [];
  if (process.env.PHASE1_PYTHON) candidates.push(process.env.PHASE1_PYTHON);
  candidates.push(process.platform === "win32" ? "python.exe" : "python3", "python");
  if (process.platform === "win32" && process.env.LOCALAPPDATA) {
    candidates.push(
      join(process.env.LOCALAPPDATA, "Programs", "Python", "Python313", "python.exe"),
    );
  }

  const seen = new Set();
  return candidates.filter((candidate) => {
    const key = candidate.toLowerCase();
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

function inspect(candidate) {
  const result = spawnSync(
    candidate,
    [
      "-c",
      "import sys; print('.'.join(str(part) for part in sys.version_info[:3])); print(sys.executable)",
    ],
    {
      encoding: "utf8",
      windowsHide: true,
      timeout: 10_000,
    },
  );
  const lines = `${result.stdout ?? ""}`.trim().split(/\r?\n/);
  return { candidate, result, version: lines[0] ?? "", executable: lines[1] ?? "" };
}

const observations = [];
let python = null;
for (const candidate of candidatePythons()) {
  const inspected = inspect(candidate);
  if (
    inspected.result.status === 0 &&
    inspected.version === EXPECTED_VERSION &&
    inspected.executable
  ) {
    python = inspected.executable;
    break;
  }
  if (inspected.version) observations.push(`${candidate}: ${inspected.version}`);
}

if (!python) {
  fail(
    `Python runtime mismatch: required Python ${EXPECTED_VERSION}; observed ${observations.join(", ") || "none"}`,
  );
}

if (process.argv[2] === "--resolve-only") {
  console.log(python);
  process.exit(0);
}

const child = spawnSync(python, process.argv.slice(2), {
  cwd: process.cwd(),
  env: { ...process.env, PHASE1_PYTHON: python },
  stdio: "inherit",
  windowsHide: true,
});
if (child.error) fail(`Unable to start ${python}: ${child.error.message}`);
if (child.signal) fail(`Pinned Python process terminated by signal ${child.signal}`);
process.exit(Number.isInteger(child.status) ? child.status : EXIT_TOOL_ERROR);
