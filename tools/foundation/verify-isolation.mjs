import { createHash } from "node:crypto";
import { readdir, readFile } from "node:fs/promises";
import path from "node:path";

import { resolveInProject } from "./paths.mjs";

const failures = [];
const distDirectory = resolveInProject("dist");
const generatedModulePath = resolveInProject(
  "apps",
  "foundation-shell",
  "src",
  "generated",
  "foundation-wasm.ts",
);

const entries = await readdir(distDirectory, { withFileTypes: true });
if (
  entries.length !== 1 ||
  entries[0]?.name !== "index.html" ||
  !entries[0].isFile()
) {
  failures.push(`dist must contain only index.html; found ${entries.map(({ name }) => name).join(", ")}`);
}

const html = await readFile(path.join(distDirectory, "index.html"), "utf8");
const requiredCsp = [
  "default-src 'none'",
  "connect-src 'none'",
  "object-src 'none'",
  "script-src 'wasm-unsafe-eval'",
  "worker-src blob:",
];
for (const directive of requiredCsp) {
  if (!html.includes(directive)) {
    failures.push(`CSP is missing ${directive}`);
  }
}
if (/<(?:script|link)[^>]+(?:src|href)\s*=/iu.test(html)) {
  failures.push("dist/index.html contains an external script or stylesheet reference");
}
if (/<(?:iframe|object|embed|form)\b/iu.test(html)) {
  failures.push("dist/index.html contains a forbidden active element");
}

const forbiddenBundlePatterns = [
  ["network API", /\b(?:XMLHttpRequest|WebSocket|EventSource|WebTransport|RTCPeerConnection)\b/u],
  ["device API", /navigator\.(?:serial|usb|bluetooth|hid|nfc|midi|mediaDevices|serviceWorker)\b/u],
  ["endpoint string", /\b(?:ftp|wss?):\/\/|\blocalhost\b|\b127\.0\.0\.1\b|\b0\.0\.0\.0\b|\[::1\]/iu],
  ["dynamic execution", /\beval\s*\(|\bnew\s+Function\s*\(/u],
];
for (const [label, pattern] of forbiddenBundlePatterns) {
  if (pattern.test(html)) {
    failures.push(`production bundle contains forbidden ${label}`);
  }
}

const urls =
  html.match(/https?:\/\/[A-Za-z0-9._~:/?#\[\]@!$&'()*+,;=%-]+/gu) ?? [];
const allowedInertUris = [
  /^http:\/\/www\.w3\.org\/(?:1999\/xhtml|1999\/xlink|2000\/svg|1998\/Math\/MathML|XML\/1998\/namespace)$/u,
  /^https:\/\/react\.dev\/errors\//u,
];
for (const url of urls) {
  if (!allowedInertUris.some((pattern) => pattern.test(url))) {
    failures.push(`production bundle contains an unapproved URI: ${url.slice(0, 120)}`);
  }
}

const generated = await readFile(generatedModulePath, "utf8");
const digestMatch = /FOUNDATION_WASM_SHA256 = "([A-F0-9]{64})"/u.exec(generated);
const base64Match = /FOUNDATION_WASM_BASE64 = "([A-Za-z0-9+/=]+)"/u.exec(generated);
if (!digestMatch || !base64Match) {
  failures.push("generated WASM module metadata is malformed");
} else {
  const bytes = Buffer.from(base64Match[1], "base64");
  const observed = createHash("sha256").update(bytes).digest("hex").toUpperCase();
  if (observed !== digestMatch[1]) {
    failures.push("embedded WASM SHA-256 does not match its bytes");
  }
  try {
    const module = new WebAssembly.Module(bytes);
    const imports = WebAssembly.Module.imports(module);
    if (imports.length !== 0) {
      failures.push(`embedded WASM has imports: ${JSON.stringify(imports)}`);
    }
  } catch (error) {
    failures.push(`embedded WASM is invalid: ${error.message}`);
  }
  if (!html.includes(base64Match[1]) || !html.includes(digestMatch[1])) {
    failures.push("dist/index.html is not bound to the admitted embedded WASM");
  }
}

if (failures.length > 0) {
  console.error("Foundation production isolation verification failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exitCode = 1;
} else {
  console.log(JSON.stringify({ artifactSha256: createHash("sha256").update(html).digest("hex").toUpperCase(), files: ["dist/index.html"], wasmImports: 0 }));
}
