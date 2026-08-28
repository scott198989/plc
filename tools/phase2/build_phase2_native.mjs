import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const arguments_ = process.argv.slice(2);
if (process.platform !== "win32") {
  throw new Error("The strict Phase 2 native build is Windows-only.");
}
if (arguments_.length !== 0) {
  throw new Error("The strict Phase 2 native build accepts zero arguments.");
}
for (const name of [
  "GOVS_NATIVE_BUILD_ROOT",
  "GOVS_NATIVE_PACKAGE",
  "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
  "WEBVIEW2_BROWSER_EXECUTABLE_FOLDER",
  "WEBVIEW2_RELEASE_CHANNEL_PREFERENCE",
  "WEBVIEW2_USER_DATA_FOLDER",
]) {
  if (Object.hasOwn(process.env, name)) {
    throw new Error(`The strict Phase 2 native build rejects environment override ${name}.`);
  }
}

const runFixed = (relative) => {
  const script = path.join(root, ...relative.split("/"));
  const result = spawnSync(process.execPath, [script], {
    cwd: root,
    encoding: "utf8",
    env: process.env,
    shell: false,
    stdio: "inherit",
    windowsHide: true,
  });
  if (result.error !== undefined || result.status !== 0) {
    throw result.error ?? new Error(`${relative} exited ${result.status}`);
  }
};

runFixed("tools/phase2/build_windows_shell.mjs");
runFixed("tools/phase2/build_native_e2e_launcher.mjs");
