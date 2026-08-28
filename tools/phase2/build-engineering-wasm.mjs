import { spawnSync } from "node:child_process";

import { PROJECT_ROOT } from "../foundation/paths.mjs";

const result = spawnSync(
  "rustup.exe",
  [
    "run",
    "1.94.0-x86_64-pc-windows-msvc",
    "cargo.exe",
    "build",
    "--locked",
    "--offline",
    "--package",
    "plc-engineering-wasm",
    "--release",
    "--target",
    "wasm32-unknown-unknown",
  ],
  {
    cwd: PROJECT_ROOT,
    encoding: "utf8",
    shell: false,
    stdio: "inherit",
    windowsHide: true,
  },
);

if (result.error) {
  throw result.error;
}
if (result.status !== 0) {
  process.exitCode = result.status ?? 1;
}
