#!/usr/bin/env node

import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";

import {
  ISOLATION_APPROVAL_DECISION_ID,
  isolationGateProofFieldsFromClosure,
  sha256,
  stableJson,
} from "./isolation-counterfactual-lib.mjs";

const options = parseArguments(process.argv.slice(2));
const candidate = {
  commit: validateObjectId(options.candidateCommit, "candidate commit"),
  isolationApprovalDecisionId: validateDecisionId(options.approvalDecisionId),
  isolationApprovalSha256: validateSha256(options.approvalSha256, "approval SHA-256"),
  tree: validateObjectId(options.candidateTree, "candidate tree"),
};
const inputPath = path.resolve(options.input);
const outputPath = path.resolve(options.output);

if (inputPath === outputPath) {
  throw new Error("Closure input and gate-proof output paths must be distinct.");
}

const inputBytes = await readFile(inputPath);
let closure;
try {
  closure = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(inputBytes));
} catch (error) {
  throw new Error(`Closure input is not strict UTF-8 JSON: ${errorMessage(error)}`);
}

const gateProofFields = isolationGateProofFieldsFromClosure(closure, candidate);
const outputBytes = Buffer.from(stableJson(gateProofFields), "utf8");
await writeFile(outputPath, outputBytes, { flag: "w" });

console.log(stableJson({
  candidateCommit: candidate.commit,
  candidateTree: candidate.tree,
  isolationApprovalDecisionId: candidate.isolationApprovalDecisionId,
  isolationApprovalSha256: candidate.isolationApprovalSha256,
  inputSha256: sha256(inputBytes),
  outputPath,
  outputSha256: sha256(outputBytes),
  proofFieldCount: Object.keys(gateProofFields).length,
}).trimEnd());

function parseArguments(arguments_) {
  const parsed = {};
  const keys = {
    "--approval-decision-id": "approvalDecisionId",
    "--approval-sha256": "approvalSha256",
    "--candidate-commit": "candidateCommit",
    "--candidate-tree": "candidateTree",
    "--input": "input",
    "--output": "output",
  };
  for (let index = 0; index < arguments_.length; index += 2) {
    const argument = arguments_[index];
    const key = keys[argument];
    const value = arguments_[index + 1];
    if (key === undefined || value === undefined || parsed[key] !== undefined) {
      throw new Error(`Unknown, duplicate, or incomplete argument: ${String(argument)}`);
    }
    parsed[key] = value;
  }
  for (const key of Object.values(keys)) {
    if (parsed[key] === undefined) {
      throw new Error(`Missing required argument: ${key}`);
    }
  }
  return parsed;
}

function validateDecisionId(value) {
  if (value !== ISOLATION_APPROVAL_DECISION_ID) {
    throw new Error(`The approval decision must be ${ISOLATION_APPROVAL_DECISION_ID}.`);
  }
  return value;
}

function validateSha256(value, label) {
  if (!/^[A-F0-9]{64}$/u.test(String(value))) {
    throw new Error(`The ${label} must be one uppercase 64-hex digest.`);
  }
  return String(value);
}

function validateObjectId(value, label) {
  if (!/^[0-9a-f]{40}$/u.test(String(value))) {
    throw new Error(`The ${label} must be one lowercase 40-hex Git object ID.`);
  }
  return String(value);
}

function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}
