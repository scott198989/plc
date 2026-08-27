import { access } from "node:fs/promises";
import { spawn } from "node:child_process";
import { pathToFileURL } from "node:url";

import { resolveInProject } from "./paths.mjs";

const indexPath = resolveInProject("dist", "index.html");
await access(indexPath);
const url = pathToFileURL(indexPath).href;

if (process.platform !== "win32") {
  console.log(`Open this local file in a browser: ${url}`);
} else {
  const child = spawn("explorer.exe", [indexPath], {
    detached: true,
    shell: false,
    stdio: "ignore",
    windowsHide: true,
  });
  child.unref();
  console.log(`Opened the local foundation artifact: ${url}`);
}
