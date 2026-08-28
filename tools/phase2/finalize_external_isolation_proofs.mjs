#!/usr/bin/env node

// External-only fixed command recorder. It intentionally grants no closure
// credit: test exits do not observe host egress, adapter state, or product use.

import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { stableJson } from "./isolation-counterfactual-lib.mjs";

const exec = promisify(execFile);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const sha256 = (value) => createHash("sha256").update(value).digest("hex").toUpperCase();

const FIXED_COMMANDS = Object.freeze([
  { commandId: "foundation-isolation-boundary-fuzz", executable: "pnpm", args: ["--filter", "@govs/foundation-shell", "test", "--", "isolation-boundary-fuzz"], testIds: ["isolation-boundary-fuzz"] },
  { commandId: "plc-core-isolation-boundary-fuzz", executable: "cargo", args: ["test", "-p", "plc-core", "--test", "isolation_boundary_fuzz", "--locked"], testIds: ["isolation_boundary_fuzz"] },
  { commandId: "plc-compiler-isolation-boundary-fuzz", executable: "cargo", args: ["test", "-p", "plc-compiler", "--test", "isolation_boundary_fuzz", "--locked"], testIds: ["isolation_boundary_fuzz"] },
  { commandId: "plc-observability-isolation-boundary-fuzz", executable: "cargo", args: ["test", "-p", "plc-observability", "--test", "isolation_boundary_fuzz", "--locked"], testIds: ["isolation_boundary_fuzz"] },
  { commandId: "plc-system-replay-rejection", executable: "cargo", args: ["test", "-p", "plc-system", "--test", "system_journeys", "replay", "--locked"], testIds: ["queued_generated_replay_output_rejects_payload_injection_before_passive_match", "production_replay_executor_rejects_non_simulator_ingress_without_state_change"] },
  { commandId: "windows-broker-isolation-fuzz", executable: "cargo", args: ["test", "-p", "windows-project-broker", "--test", "isolation_boundary_fuzz", "--locked"], testIds: ["isolation_boundary_fuzz"] },
  { commandId: "plc-core-persistence-adversarial", executable: "cargo", args: ["test", "-p", "plc-core", "--test", "persistence_adversarial", "--locked"], testIds: ["persistence_adversarial"] },
]);

export function fixedCommandDescriptors() { return FIXED_COMMANDS.map((row) => structuredClone(row)); }
export function nonCreditTestSeamProof(candidate) { return { candidateCommit: candidate.commit, candidateTree: candidate.tree, completeLogs: false, evidenceKind: "PHASE2_FIXED_COMMAND_RECEIPT", externalAttemptCount: null, productionPathExercised: false, result: "NON_CREDIT_TEST_SEAM", schemaVersion: "1.0", zeroExternalAttempts: false }; }

async function main() {
  const options = parse(process.argv.slice(2));
  await mkdir(options.output, { recursive: false });
  if (options.testSeam) {
    await writeFile(path.join(options.output, "NON_CREDIT_TEST_SEAM.json"), stableJson(nonCreditTestSeamProof({ commit: "test-seam", tree: "test-seam" })), { flag: "wx" });
    return;
  }
  const identity = await exactCleanCandidate();
  const files = [];
  for (const fixed of FIXED_COMMANDS) {
    const descriptor = { ...fixed, evidenceKind: "PHASE2_FIXED_COMMAND_DESCRIPTOR", schemaVersion: "1.0" };
    const descriptorBytes = Buffer.from(stableJson(descriptor)); const descriptorPath = `${fixed.commandId}.command.json`;
    await writeFile(path.join(options.output, descriptorPath), descriptorBytes, { flag: "wx" });
    const outcome = await exec(fixed.executable, fixed.args, { cwd: root, windowsHide: true, maxBuffer: 64 * 1024 * 1024 })
      .then(({ stdout, stderr }) => ({ exitCode: 0, stderr, stdout }))
      .catch((error) => ({ exitCode: Number.isInteger(error?.code) ? error.code : 1, stderr: String(error?.stderr ?? error), stdout: String(error?.stdout ?? "") }));
    const transcript = Buffer.from(`${outcome.stdout}\n${outcome.stderr}`, "utf8"); const transcriptPath = `${fixed.commandId}.transcript.log`;
    await writeFile(path.join(options.output, transcriptPath), transcript, { flag: "wx" });
    const receipt = { candidateCommit: identity.commit, candidateTree: identity.tree, commandId: fixed.commandId, commandSha256: sha256(descriptorBytes), completeLogs: true, descriptorPath, evidenceKind: "PHASE2_FIXED_COMMAND_RECEIPT", exitCode: outcome.exitCode, externalAttemptCount: null, productionPathExercised: false, result: outcome.exitCode === 0 ? "BLOCKED_EXTERNAL_OBSERVER_REQUIRED" : "FAIL", schemaVersion: "1.0", testIdsExpected: fixed.testIds, transcriptPath, transcriptSha256: sha256(transcript), zeroExternalAttempts: false };
    const receiptBytes = Buffer.from(stableJson(receipt)); const receiptPath = `${fixed.commandId}.receipt.json`;
    await writeFile(path.join(options.output, receiptPath), receiptBytes, { flag: "wx" });
    files.push({ bytes: descriptorBytes.byteLength, path: descriptorPath, sha256: sha256(descriptorBytes) }, { bytes: transcript.byteLength, path: transcriptPath, sha256: sha256(transcript) }, { bytes: receiptBytes.byteLength, path: receiptPath, sha256: sha256(receiptBytes) });
  }
  await writeFile(path.join(options.output, "fixed-command-manifest.json"), stableJson({ candidateCommit: identity.commit, candidateTree: identity.tree, evidenceKind: "PHASE2_FIXED_COMMAND_MANIFEST", files: files.sort((a, b) => a.path.localeCompare(b.path, "en")), result: "BLOCKED_EXTERNAL_OBSERVER_REQUIRED", schemaVersion: "1.0" }), { flag: "wx" });
}

async function exactCleanCandidate() {
  const invoke = async (...args) => (await exec("git", args, { cwd: root, windowsHide: true })).stdout.trim();
  const [commit, tree, status] = await Promise.all([invoke("rev-parse", "HEAD^{commit}"), invoke("rev-parse", "HEAD^{tree}"), invoke("status", "--porcelain=v1")]);
  if (status) throw new Error("Fixed proof collection requires a clean exact candidate.");
  if (!/^[0-9a-f]{40}$/u.test(commit) || !/^[0-9a-f]{40}$/u.test(tree)) throw new Error("Exact candidate Git binding is malformed.");
  return { commit, tree };
}

function parse(argv) {
  const options = {};
  for (let index = 0; index < argv.length;) {
    if (argv[index] === "--test-seam" && options.testSeam === undefined) { options.testSeam = true; index += 1; continue; }
    if (argv[index] !== "--output" || options.output !== undefined || argv[index + 1] === undefined) throw new Error("Only --output and --test-seam are accepted; the command inventory is fixed.");
    options.output = path.resolve(argv[index + 1]); index += 2;
  }
  if (options.output === undefined) throw new Error("Missing --output.");
  return options;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main().catch((error) => { console.error(error instanceof Error ? error.message : String(error)); process.exitCode = 1; });
