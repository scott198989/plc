#!/usr/bin/env node

import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { lstat, mkdir, readFile, realpath, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const root = path.resolve(path.dirname(scriptPath), "..", "..");
const nativeBuild = path.join(root, ".phase2-verification", "native-build");
const packageRoot = path.join(nativeBuild, "package");
const objectRoot = path.join(nativeBuild, "obj", "external-observer");
const manifestPath = path.join(packageRoot, "candidate-package-manifest.json");
const candidateImagePath = path.join(packageRoot, "GovsPLC.exe");
const launcherPath = path.join(nativeBuild, "Run-Native-E2E.exe");
const sourcePath = path.join(root, "tools", "phase2", "windows_external_observer.cpp");
const analyzerPath = path.join(root, "tools", "phase2", "external_observer_evidence.mjs");
const finalizerPath = path.join(root, "tools", "phase2", "finalize_external_observer_evidence.mjs");
const verifierPath = path.join(root, "tools", "phase2", "verify_external_observer_source.mjs");
const outputPath = path.join(nativeBuild, "Run-Phase2-External-Observer.exe");
const sourceInputs = new Map([
  ["tools/phase2/build_external_observer.mjs", scriptPath],
  ["tools/phase2/external_observer_evidence.mjs", analyzerPath],
  ["tools/phase2/finalize_external_observer_evidence.mjs", finalizerPath],
  ["tools/phase2/verify_external_observer_source.mjs", verifierPath],
  ["tools/phase2/windows_external_observer.cpp", sourcePath],
]);

if (process.platform !== "win32" || process.arch !== "x64") {
  throw new Error("The Phase 2 external ETW observer build is Windows x64 only.");
}
if (process.argv.slice(2).length !== 0) {
  throw new Error("The Phase 2 external ETW observer build accepts zero arguments.");
}
for (const name of [
  "GOVS_EXTERNAL_OBSERVER_COMMAND",
  "GOVS_EXTERNAL_OBSERVER_EXECUTABLE",
  "GOVS_EXTERNAL_OBSERVER_PROVIDER",
  "GOVS_EXTERNAL_OBSERVER_TARGET",
]) {
  if (Object.hasOwn(process.env, name)) throw new Error(`External observer build rejects override ${name}.`);
}

const hashBytes = (bytes) => createHash("sha256").update(bytes).digest("hex").toUpperCase();
const hashFile = async (file) => hashBytes(await readFile(file));
const GIT_OBJECT = /^[a-f0-9]{40}$/u;
const exactKeys = (value, keys) => value !== null && typeof value === "object" &&
  !Array.isArray(value) && Object.keys(value).sort().join("\0") === [...keys].sort().join("\0");
const sameWindowsPath = (left, right) => path.resolve(left).toLocaleLowerCase("en-US") ===
  path.resolve(right).toLocaleLowerCase("en-US");

async function requireFixedFile(file, maximumBytes) {
  const [status, canonical] = await Promise.all([lstat(file), realpath(file)]);
  if (!status.isFile() || status.isSymbolicLink() || status.size < 1 || status.size > maximumBytes ||
      !sameWindowsPath(canonical, file)) {
    throw new Error(`External observer build input is not a bounded fixed regular file: ${path.basename(file)}`);
  }
  return readFile(file);
}

function requireSourceRow(manifest, sourcePath, bytes) {
  const rows = manifest.sourceInputs.filter((row) => row?.path === sourcePath);
  if (rows.length !== 1 || !exactKeys(rows[0], ["bytes", "path", "sha256"]) ||
      rows[0].bytes !== bytes.byteLength || rows[0].sha256 !== hashBytes(bytes)) {
    throw new Error(`Candidate manifest does not bind the external observer source: ${sourcePath}`);
  }
}

const [manifestBytes, candidateImageBytes, launcherBytes] = await Promise.all([
  requireFixedFile(manifestPath, 16 * 1024 * 1024),
  requireFixedFile(candidateImagePath, 256 * 1024 * 1024),
  requireFixedFile(launcherPath, 256 * 1024 * 1024),
]);
const manifest = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(manifestBytes));
if (manifest?.schemaVersion !== "1.0" ||
    manifest?.evidenceKind !== "WINDOWS_NATIVE_CANDIDATE_PACKAGE_MANIFEST" ||
    !GIT_OBJECT.test(manifest?.gitCommit) || !GIT_OBJECT.test(manifest?.gitTree) ||
    manifest?.developmentDirty !== false || !Array.isArray(manifest?.packageFiles) ||
    !Array.isArray(manifest?.sourceInputs)) {
  throw new Error("The external observer requires a clean exact native candidate manifest.");
}
const candidateRows = manifest.packageFiles.filter((row) => row?.path === "GovsPLC.exe");
if (candidateRows.length !== 1 || !exactKeys(candidateRows[0], ["bytes", "path", "sha256"]) ||
    candidateRows[0].bytes !== candidateImageBytes.byteLength ||
    candidateRows[0].sha256 !== hashBytes(candidateImageBytes)) {
  throw new Error("The external observer candidate executable is not manifest-bound.");
}
const sourceBytes = new Map();
for (const [relative, file] of sourceInputs) {
  const bytes = await requireFixedFile(file, 8 * 1024 * 1024);
  sourceBytes.set(relative, bytes);
  requireSourceRow(manifest, relative, bytes);
  const tracked = spawnSync("git.exe", ["ls-files", "--error-unmatch", "--", relative], {
    cwd: root, encoding: "utf8", shell: false, windowsHide: true,
  });
  if (tracked.status !== 0) throw new Error(`Untracked external observer source: ${relative}`);
  for (const mode of [[], ["--cached"]]) {
    const clean = spawnSync("git.exe", ["diff", "--quiet", ...mode, "--", relative], {
      cwd: root, encoding: "utf8", shell: false, windowsHide: true,
    });
    if (clean.status !== 0) throw new Error(`External observer source differs from HEAD: ${relative}`);
  }
}

const header = [
  "#pragma once",
  "// Generated from exact candidate and observer inputs. Do not edit.",
  `inline constexpr wchar_t kCandidateCommit[] = L\"${manifest.gitCommit}\";`,
  `inline constexpr wchar_t kCandidateTree[] = L\"${manifest.gitTree}\";`,
  `inline constexpr wchar_t kCandidateManifestSha256[] = L\"${hashBytes(manifestBytes)}\";`,
  `inline constexpr wchar_t kCandidateImageSha256[] = L\"${candidateRows[0].sha256}\";`,
  `inline constexpr wchar_t kLauncherSha256[] = L\"${hashBytes(launcherBytes)}\";`,
  `inline constexpr wchar_t kObserverSourceSha256[] = L\"${hashBytes(sourceBytes.get("tools/phase2/windows_external_observer.cpp"))}\";`,
  `inline constexpr wchar_t kObserverBuildScriptSha256[] = L\"${hashBytes(sourceBytes.get("tools/phase2/build_external_observer.mjs"))}\";`,
  "",
].join("\r\n");
await mkdir(objectRoot, { recursive: true });
const headerPath = path.join(objectRoot, "external_observer_candidate.h");
await writeFile(headerPath, header, { encoding: "utf8" });

const vswhere = "C:\\Program Files (x86)\\Microsoft Visual Studio\\Installer\\vswhere.exe";
const located = spawnSync(vswhere, [
  "-latest", "-products", "*", "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
  "-property", "installationPath",
], { encoding: "utf8", shell: false, windowsHide: true });
if (located.status !== 0 || located.stdout.trim() === "") {
  throw new Error("The pinned MSVC x64 build environment is unavailable.");
}
const vcvars = path.join(located.stdout.trim(), "VC", "Auxiliary", "Build", "vcvars64.bat");
const objectPath = path.join(objectRoot, "windows_external_observer.obj");
const commandPath = path.join(objectRoot, "compile-external-observer.cmd");
const commands = [
  "@echo off",
  "setlocal",
  `call \"${vcvars}\" >nul`,
  "if errorlevel 1 exit /b %errorlevel%",
  `cl.exe /nologo /c /std:c++20 /EHsc /W4 /WX /permissive- /utf-8 /DNOMINMAX /DUNICODE /D_UNICODE /MT /GS /sdl /guard:cf /O2 /Brepro /I\"${objectRoot}\" /Fo\"${objectPath}\" \"${sourcePath}\"`,
  "if errorlevel 1 exit /b %errorlevel%",
  `link.exe /nologo /Brepro /guard:cf /DYNAMICBASE /NXCOMPAT /HIGHENTROPYVA /CETCOMPAT /SUBSYSTEM:WINDOWS /OUT:\"${outputPath}\" \"${objectPath}\" advapi32.lib bcrypt.lib tdh.lib user32.lib`,
  "exit /b %errorlevel%",
  "",
];
await writeFile(commandPath, commands.join("\r\n"), { encoding: "utf8" });
const built = spawnSync("cmd.exe", ["/d", "/c", commandPath], {
  cwd: root, encoding: "utf8", shell: false, stdio: "inherit", windowsHide: true,
});
if (built.error !== undefined || built.status !== 0) {
  throw built.error ?? new Error(`External observer build exited ${built.status}`);
}
const outputBytes = await requireFixedFile(outputPath, 256 * 1024 * 1024);
console.log(`${JSON.stringify({
  candidateCommit: manifest.gitCommit,
  candidateManifestSha256: hashBytes(manifestBytes),
  candidateTree: manifest.gitTree,
  observer: path.relative(root, outputPath).replaceAll("\\", "/"),
  observerSha256: hashBytes(outputBytes),
  result: "PASS",
}, null, 2)}\n`);
