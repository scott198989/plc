import { spawnSync } from "node:child_process";

const EXPECTED = Object.freeze({
  cargo: "cargo 1.94.0 (85eff7c80 2026-01-15)",
  node: "24.19.0",
  pnpm: "11.19.0",
  rustc: "rustc 1.94.0 (4a4ef493e 2026-03-02)",
});

const failures = [];
const evidence = {};

const capture = (command, args) => {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    shell: false,
    windowsHide: true,
  });
  if (result.error || result.status !== 0) {
    failures.push(
      `${command} ${args.join(" ")} failed: ${result.error?.message ?? result.stderr.trim()}`,
    );
    return "";
  }
  return result.stdout.trim();
};

evidence.node = process.versions.node;
evidence.pnpm = capture("cmd.exe", ["/d", "/s", "/c", "pnpm --version"]);
evidence.rustc = capture("rustup.exe", [
  "run",
  "1.94.0-x86_64-pc-windows-msvc",
  "rustc.exe",
  "--version",
]);
evidence.cargo = capture("rustup.exe", [
  "run",
  "1.94.0-x86_64-pc-windows-msvc",
  "cargo.exe",
  "--version",
]);
const installedTargets = capture("rustup.exe", [
  "target",
  "list",
  "--installed",
  "--toolchain",
  "1.94.0-x86_64-pc-windows-msvc",
]);
const installedComponents = capture("rustup.exe", [
  "component",
  "list",
  "--installed",
  "--toolchain",
  "1.94.0-x86_64-pc-windows-msvc",
]);

for (const [tool, expected] of Object.entries(EXPECTED)) {
  if (evidence[tool] !== expected) {
    failures.push(`${tool} must be ${expected}; observed ${evidence[tool] || "unavailable"}.`);
  }
}
if (!installedTargets.split(/\r?\n/u).includes("wasm32-unknown-unknown")) {
  failures.push("The exact Rust toolchain is missing wasm32-unknown-unknown.");
}
for (const component of ["clippy", "rustfmt"]) {
  if (!installedComponents.split(/\r?\n/u).some((line) => line.startsWith(`${component}-`))) {
    failures.push(`The exact Rust toolchain is missing ${component}.`);
  }
}

if (failures.length > 0) {
  console.error("Foundation toolchain admission failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exitCode = 1;
} else {
  console.log(JSON.stringify({ ...evidence, wasmTarget: "wasm32-unknown-unknown" }));
}
