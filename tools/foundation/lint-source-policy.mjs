import { readFile, readdir } from "node:fs/promises";
import path from "node:path";

import { resolveInProject } from "./paths.mjs";

const sourceRoots = [
  resolveInProject("apps", "foundation-shell", "src"),
  resolveInProject("packages", "foundation-contract", "src"),
];
const rustRoots = [
  resolveInProject("crates", "foundation-wasm", "src"),
  resolveInProject("crates", "plc-commissioning", "src"),
  resolveInProject("crates", "plc-compiler", "src"),
  resolveInProject("crates", "plc-core", "src"),
  resolveInProject("crates", "plc-engineering-wasm", "src"),
  resolveInProject("crates", "plc-hardware", "src"),
  resolveInProject("crates", "plc-language-tools", "src"),
  resolveInProject("crates", "plc-program", "src"),
  resolveInProject("crates", "plc-runtime", "src"),
];
const failures = [];

const typescriptPatterns = [
  [
    "forbidden production import",
    /\bfrom\s+["'](?:node:)?(?:child_process|cluster|dgram|dns|http|https|net|tls|worker_threads)["']/u,
  ],
  [
    "forbidden production capability",
    /\b(?:new\s+)?(?:EventSource|RTCPeerConnection|WebSocket|WebTransport|XMLHttpRequest|fetch|importScripts)\s*\(/u,
  ],
  [
    "forbidden browser device capability",
    /\bnavigator\.(?:bluetooth|hid|mediaDevices|midi|nfc|serial|serviceWorker|usb)\b/u,
  ],
  [
    "endpoint-shaped string",
    /\b(?:ftp|https?|wss?):\/\/|\blocalhost\b|\b127\.0\.0\.1\b|\b0\.0\.0\.0\b|\[::1\]/iu,
  ],
  ["runtime dynamic import", /\bimport\s*\(/u],
  ["dynamic execution", /\beval\s*\(|\bnew\s+Function\s*\(/u],
];

const walk = async (directory, extensions) => {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      if (entry.name !== "generated") {
        files.push(...(await walk(absolute, extensions)));
      }
    } else if (extensions.has(path.extname(entry.name))) {
      files.push(absolute);
    }
  }
  return files;
};

for (const root of sourceRoots) {
  for (const file of await walk(root, new Set([".ts", ".tsx"]))) {
    const sourceText = await readFile(file, "utf8");
    for (const [reason, pattern] of typescriptPatterns) {
      const match = pattern.exec(sourceText);
      if (match) {
        const line = sourceText.slice(0, match.index).split(/\r?\n/u).length;
        failures.push(`${path.relative(resolveInProject(), file)}:${line} ${reason}`);
      }
    }
  }
}

const rustPattern = /std::(?:net|process)|TcpStream|UdpSocket|Command::new|extern\s+"system"|wasi|wasm_bindgen/iu;
for (const rustRoot of rustRoots) {
  for (const file of await walk(rustRoot, new Set([".rs"]))) {
    const source = await readFile(file, "utf8");
    const match = rustPattern.exec(source);
    if (match) {
      failures.push(`${path.relative(resolveInProject(), file)} contains ${match[0]}`);
    }
  }
}

if (failures.length > 0) {
  console.error("Production source isolation scan failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exitCode = 1;
} else {
  console.log("Production source isolation scan passed (TypeScript/TSX/Rust). ");
}
