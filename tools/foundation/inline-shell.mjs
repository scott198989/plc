import { createHash } from "node:crypto";
import { mkdir, readdir, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";

import { resolveInProject } from "./paths.mjs";

const distDirectory = resolveInProject("dist");
const stagingDirectory = resolveInProject("dist", "foundation-staging");
const outputPath = resolveInProject("dist", "index.html");

const existing = await readdir(distDirectory, { withFileTypes: true }).catch((error) => {
  if (error.code === "ENOENT") {
    return [];
  }
  throw error;
});
const unexpected = existing
  .map(({ name }) => name)
  .filter((name) => name !== "foundation-staging" && name !== "index.html");
if (unexpected.length > 0) {
  throw new Error(`Refusing to overwrite unexpected dist entries: ${unexpected.join(", ")}`);
}

const files = await readdir(stagingDirectory);
const scriptName = files.find((name) => name === "foundation.js");
const styleName = files.find((name) => name === "foundation.css");
if (scriptName === undefined || styleName === undefined) {
  throw new Error(`Vite staging output is incomplete: ${files.join(", ")}`);
}

const [script, style] = await Promise.all([
  readFile(path.join(stagingDirectory, scriptName), "utf8"),
  readFile(path.join(stagingDirectory, styleName), "utf8"),
]);
if (/<\/script/iu.test(script) || /<\/style/iu.test(style)) {
  throw new Error("Bundled content contains an unsafe inline closing tag.");
}

const digest = (content) =>
  createHash("sha256").update(content, "utf8").digest("base64");
const scriptHash = digest(script);
const styleHash = digest(style);
const csp = [
  "default-src 'none'",
  "base-uri 'none'",
  "connect-src 'none'",
  "form-action 'none'",
  "img-src data:",
  "object-src 'none'",
  `script-src 'wasm-unsafe-eval' 'sha256-${scriptHash}'`,
  `style-src 'sha256-${styleHash}'`,
  "worker-src blob:",
].join("; ");

const html = `<!doctype html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<meta name="color-scheme" content="light">
<meta http-equiv="Content-Security-Policy" content="${csp}">
<title>PLC Engineering Simulator</title>
<style>${style}</style>
</head>
<body>
<div id="root"></div>
<noscript>The PLC Engineering Simulator requires JavaScript.</noscript>
<script type="module">${script}</script>
</body>
</html>
`;

await mkdir(distDirectory, { recursive: true });
await writeFile(outputPath, html, { encoding: "utf8", flag: "w" });
await rm(stagingDirectory, { recursive: true });
console.log(
  JSON.stringify({
    bytes: Buffer.byteLength(html),
    output: path.relative(resolveInProject(), outputPath).replaceAll("\\", "/"),
    scriptSha256: scriptHash,
    styleSha256: styleHash,
  }),
);
