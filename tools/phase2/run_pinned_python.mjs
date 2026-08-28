#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { join } from "node:path";

// Phase 2 tooling uses the workspace-owned Python runtime distributed with the
// active Codex dependency bundle.  This is intentionally separate from the
// historical Phase 1 verifier's frozen interpreter admission.
const EXPECTED_VERSION = "3.12.13";
const EXIT_TOOL_ERROR = 2;

function fail(message) {
  console.error(`ERROR P2-RUN-0001 ${message}`);
  process.exit(EXIT_TOOL_ERROR);
}

function candidatePythons() {
  const candidates = [];
  for (const variable of ["PHASE2_PYTHON", "PHASE1_PYTHON"]) {
    if (process.env[variable]) candidates.push(process.env[variable]);
  }
  candidates.push(process.platform === "win32" ? "python.exe" : "python3", "python");
  const home = process.env.USERPROFILE ?? process.env.HOME;
  if (home) {
    candidates.push(
      join(
        home,
        ".cache",
        "codex-runtimes",
        "codex-primary-runtime",
        "dependencies",
        "python",
        process.platform === "win32" ? "python.exe" : "bin/python3",
      ),
    );
  }
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
    { encoding: "utf8", windowsHide: true, timeout: 10_000 },
  );
  const lines = `${result.stdout ?? ""}`.trim().split(/\r?\n/u);
  return { result, version: lines[0] ?? "", executable: lines[1] ?? "" };
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
  env: { ...process.env, PHASE1_PYTHON: python, PHASE2_PYTHON: python },
  stdio: "inherit",
  windowsHide: true,
});
if (child.error) fail(`Unable to start ${python}: ${child.error.message}`);
if (child.signal) fail(`Pinned Python process terminated by signal ${child.signal}`);
process.exit(Number.isInteger(child.status) ? child.status : EXIT_TOOL_ERROR);
