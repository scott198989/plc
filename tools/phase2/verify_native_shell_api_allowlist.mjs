import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const rules = Object.freeze([
  ["apps/windows-shell/src/broker_client.cpp", "CreateFileW", 2, ["HeldHandle volume(CreateFileW", "const auto handle = CreateFileW"]],
  ["apps/windows-shell/src/broker_client.cpp", "ReadFile", 2, ["void read_exact", "fixed broker hash read failed closed"]],
  ["apps/windows-shell/src/broker_client.cpp", "WriteFile", 1, ["void write_exact"]],
  ["apps/windows-shell/src/broker_client.cpp", "std::filesystem", 12, ["open_attested_path", "prepare_fixed_user_data_folder"]],
  ["apps/windows-shell/src/main.cpp", "std::filesystem", 5, ["executable_directory", "application_folder_"]],
  ["apps/windows-shell/src/main.cpp", "std::ofstream", 1, ["write_verification_manifest"]],
  ["crates/windows-project-broker/src/windows.rs", "SetFileInformationByHandle", 2, ["restore_held_original", "fn SetFileInformationByHandle"]],
  ["crates/windows-project-broker/src/windows.rs", "ReplaceFileW", 3, ["overwrite_attested", "fn ReplaceFileW"]],
  ["crates/windows-project-broker/src/windows.rs", "MoveFileExW", 2, ["commit_new_temp", "fn MoveFileExW"]],
]);
const files = [...new Set(rules.map(([file]) => file))];
const sensitive = [
  "CreateFileW", "ReadFile", "WriteFile", "DeleteFileW", "CopyFileW", "MoveFileExW",
  "ReplaceFileW", "std::filesystem", "std::ifstream", "std::ofstream", "std::fstream",
  "SetFileInformationByHandle",
  "GetProcAddress", "LoadLibrary", "LoadLibraryW", "LoadLibraryExW",
];
const count = (source, token) => (source.match(new RegExp(`\\b${token.replaceAll(".", "\\.")}\\b`, "gu")) ?? []).length;

export async function verifyNativeShellApiAllowlist(base = root, sourceOverrides = new Map()) {
  const sources = new Map();
  for (const relative of files) {
    sources.set(relative, sourceOverrides.get(relative) ?? await readFile(path.join(base, ...relative.split("/")), "utf8"));
  }
  const findings = [];
  for (const [file, token, expectedCount, anchors] of rules) {
    const source = sources.get(file);
    if (count(source, token) !== expectedCount || anchors.some((anchor) => !source.includes(anchor))) {
      findings.push(`${file}: approved ${token} inventory or wrapper scope drifted`);
    }
  }
  for (const [file, source] of sources) {
    const approved = new Set(rules.filter(([ruleFile]) => ruleFile === file).map(([, token]) => token));
    for (const token of sensitive) {
      if (count(source, token) > 0 && !approved.has(token)) {
        findings.push(`${file}: unapproved direct native file API ${token}`);
      }
    }
  }
  if (findings.length > 0) throw new Error(findings.join("; "));
  return Object.freeze(rules.map(([file, token, expectedCount]) => ({ file, token, expectedCount })));
}

if (process.argv[1] !== undefined && path.resolve(process.argv[1]) === path.resolve(fileURLToPath(import.meta.url))) {
  verifyNativeShellApiAllowlist().then((inventory) => {
    console.log(JSON.stringify({ result: "PASS", inventory }, null, 2));
  }).catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
