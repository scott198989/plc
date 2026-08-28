import { createHash } from "node:crypto";
import { lstat, mkdir, readFile, realpath, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const nativeBuild = path.join(root, ".phase2-verification", "native-build");
const packageRoot = path.join(nativeBuild, "package");
const objectRoot = path.join(nativeBuild, "obj", "native-e2e-launcher");
const manifestPath = path.join(packageRoot, "candidate-package-manifest.json");
const sourcePath = path.join(root, "tools", "phase2", "native_e2e_launcher.cpp");
const finalizerPath = path.join(root, "tools", "phase2", "finalize_native_e2e_evidence.mjs");
const analysisLibraryPath = path.join(root, "tools", "phase2", "isolation-counterfactual-lib.mjs");
const scriptPath = fileURLToPath(import.meta.url);
const developmentDirty = process.argv.slice(2).length === 1 &&
  process.argv[2] === "--development-dirty";

if (process.platform !== "win32") {
  throw new Error("The native product-path verification launcher is Windows-only.");
}
if (process.argv.slice(2).length !== (developmentDirty ? 1 : 0)) {
  throw new Error("Only the explicit --development-dirty pre-commit build mode is accepted.");
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
    throw new Error(`The native launcher build rejects environment override ${name}.`);
  }
}

const hashBytes = (bytes) => createHash("sha256").update(bytes).digest("hex").toUpperCase();
const hashFile = async (file) => hashBytes(await readFile(file));
const isSha256 = (value) => typeof value === "string" && /^[A-F0-9]{64}$/u.test(value);
const isGitObject = (value) => typeof value === "string" && /^[a-f0-9]{40}$/u.test(value);
const stableJson = (value) => `${JSON.stringify(value, null, 2)}\n`;
const exactKeys = (value, expected) =>
  value !== null && typeof value === "object" && !Array.isArray(value) &&
  Object.keys(value).sort().join("\0") === [...expected].sort().join("\0");
const requireExactKeys = (value, expected, label) => {
  if (!exactKeys(value, expected)) {
    throw new Error(`${label} does not have the exact bounded key schema.`);
  }
};
const isSafeRelativePath = (value) => {
  if (
    typeof value !== "string" || value.length === 0 || value.length > 512 ||
    value !== value.trim() || value.includes("\\") || value.includes(":") ||
    value.includes("\0") || path.posix.isAbsolute(value) ||
    path.posix.normalize(value) !== value
  ) {
    return false;
  }
  return value.split("/").every((segment) =>
    segment.length > 0 && segment === segment.trim() &&
    segment !== "." && segment !== ".." &&
    !/[<>"|?*\u0000-\u001F]/u.test(segment) &&
    !segment.endsWith("."));
};
const sameWindowsPath = (left, right) =>
  path.resolve(left).toLocaleLowerCase("en-US") ===
  path.resolve(right).toLocaleLowerCase("en-US");
const resolveBoundPath = (base, relative) => {
  if (!isSafeRelativePath(relative)) {
    throw new Error(`Unsafe candidate-bound relative path: ${String(relative)}`);
  }
  const resolved = path.resolve(base, ...relative.split("/"));
  const relation = path.relative(base, resolved);
  if (
    relation === "" || relation === ".." || relation.startsWith(`..${path.sep}`) ||
    path.isAbsolute(relation)
  ) {
    throw new Error(`Candidate-bound path escaped its fixed root: ${relative}`);
  }
  return resolved;
};
const validateRow = (row, label) => {
  if (
    !exactKeys(row, ["bytes", "path", "sha256"]) ||
    !isSafeRelativePath(row.path) ||
    !Number.isSafeInteger(row.bytes) || row.bytes < 1 || !isSha256(row.sha256)
  ) {
    throw new Error(`Invalid ${label} row: ${String(row?.path ?? "<missing>")}`);
  }
};
const verifyRow = async (row, base, canonicalBase, label) => {
  validateRow(row, label);
  const file = resolveBoundPath(base, row.path);
  const status = await lstat(file);
  if (!status.isFile() || status.isSymbolicLink()) {
    throw new Error(`${label} is not a fixed regular file: ${row.path}`);
  }
  const canonicalFile = await realpath(file);
  const expectedCanonicalFile = path.resolve(canonicalBase, ...row.path.split("/"));
  if (!sameWindowsPath(canonicalFile, expectedCanonicalFile)) {
    throw new Error(`${label} traversed a redirected or linked path: ${row.path}`);
  }
  const bytes = await readFile(file);
  if (bytes.byteLength !== row.bytes || hashBytes(bytes) !== row.sha256) {
    throw new Error(`${label} bytes drifted: ${row.path}`);
  }
  return bytes;
};
const allowedPackagePaths = [
  "GovsPLC.exe",
  "app/index.html",
  "package-contract-v1.json",
  "third-party/Microsoft.Web.WebView2-LICENSE.txt",
  "third-party/Microsoft.Web.WebView2-NOTICE.txt",
  "third-party/Microsoft.Web.WebView2-PROVENANCE.md",
  "windows-project-broker.exe",
].sort((left, right) => left.localeCompare(right, "en"));

const manifestBytes = await readFile(manifestPath);
const manifest = JSON.parse(manifestBytes.toString("utf8"));
requireExactKeys(manifest, [
  "schemaVersion",
  "evidenceKind",
  "gitCommit",
  "gitTree",
  "developmentDirty",
  "packageContractSha256",
  "reviewedRequirementMappingSha256",
  "packageFiles",
  "rendererBuild",
  "sourceInputs",
], "Native candidate package manifest");
if (
  manifest.schemaVersion !== "1.0" ||
  manifest.evidenceKind !== "WINDOWS_NATIVE_CANDIDATE_PACKAGE_MANIFEST" ||
  !isGitObject(manifest.gitCommit) || !isGitObject(manifest.gitTree) ||
  typeof manifest.developmentDirty !== "boolean" ||
  !isSha256(manifest.packageContractSha256) ||
  !isSha256(manifest.reviewedRequirementMappingSha256) ||
  manifest.rendererBuild?.schemaVersion !== "1.0" ||
  manifest.rendererBuild?.generatedArtifact?.path !== "dist/index.html" ||
  !isSha256(manifest.rendererBuild?.generatedArtifact?.sha256) ||
  !Number.isSafeInteger(manifest.rendererBuild?.generatedArtifact?.bytes) ||
  manifest.rendererBuild.generatedArtifact.bytes < 1 ||
  !isSha256(manifest.rendererBuild?.nodeExecutableSha256) ||
  !isSha256(manifest.rendererBuild?.pnpmEntrySha256) ||
  !isSha256(manifest.rendererBuild?.pnpmStoreStatusSha256) ||
  !isSha256(manifest.rendererBuild?.pnpmWrapperSha256) ||
  !isSha256(manifest.rendererBuild?.recipeSha256) ||
  !Array.isArray(manifest.packageFiles) || !Array.isArray(manifest.sourceInputs) ||
  manifest.sourceInputs.length === 0
) {
  throw new Error("The native candidate package manifest is not the bounded schema 1.0 input.");
}
if (manifest.developmentDirty !== developmentDirty) {
  throw new Error("Launcher build mode does not match the candidate package exactness state.");
}

const rows = manifest.packageFiles;
const observedPaths = rows.map((row) => row?.path);
if (
  rows.length !== allowedPackagePaths.length ||
  observedPaths.some((entry, index) => entry !== allowedPackagePaths[index]) ||
  new Set(observedPaths).size !== observedPaths.length
) {
  throw new Error("The candidate package inventory is not the exact fixed launcher inventory.");
}
const canonicalPackageRoot = await realpath(packageRoot);
const packageBytes = new Map();
for (const row of rows) {
  packageBytes.set(
    row.path,
    await verifyRow(row, packageRoot, canonicalPackageRoot, "Candidate package"),
  );
}
const contract = rows.find((row) => row.path === "package-contract-v1.json");
if (contract.sha256 !== manifest.packageContractSha256) {
  throw new Error("The candidate package contract binding drifted.");
}

const sourceRows = manifest.sourceInputs;
for (const row of sourceRows) {
  validateRow(row, "Candidate source input");
}
const sourcePaths = sourceRows.map((row) => row?.path);
const sortedSourcePaths = [...sourcePaths].sort((left, right) => left.localeCompare(right, "en"));
if (
  new Set(sourcePaths).size !== sourcePaths.length ||
  sourcePaths.some((entry, index) => entry !== sortedSourcePaths[index])
) {
  throw new Error("Candidate sourceInputs must be nonempty, sorted, and unique.");
}
const canonicalRoot = await realpath(root);
for (const row of sourceRows) {
  await verifyRow(row, root, canonicalRoot, "Candidate source input");
}
const reviewedMapping = sourceRows.find((row) =>
  row.path === "requirements/phase2-reviewed-requirement-mapping.json");
if (
  reviewedMapping === undefined ||
  reviewedMapping.sha256 !== manifest.reviewedRequirementMappingSha256
) {
  throw new Error("The reviewed requirement mapping is not bound into the candidate source inventory.");
}

const packageContractBytes = packageBytes.get("package-contract-v1.json");
const packageContract = JSON.parse(packageContractBytes.toString("utf8"));
requireExactKeys(packageContract, [
  "schemaVersion",
  "contract",
  "protocolVersion",
  "rendererBridge",
  "reviewedRequirementMappingSha256",
  "admittedFiles",
  "webView2Sdk",
  "rendererBuild",
  "sourceInputs",
  "sourceInputManifestSha256",
  "prohibitedCapabilities",
], "Native package contract");
requireExactKeys(packageContract.admittedFiles, ["application", "broker"], "Package admittedFiles");
requireExactKeys(
  packageContract.admittedFiles.application,
  ["path", "sha256"],
  "Package admitted application",
);
requireExactKeys(
  packageContract.admittedFiles.broker,
  ["path", "sha256"],
  "Package admitted broker",
);
requireExactKeys(
  packageContract.webView2Sdk,
  ["package", "version", "nugetSha256", "loaderSha256"],
  "Package WebView2 SDK",
);
const packageRowsByPath = new Map(rows.map((row) => [row.path, row]));
const expectedProhibitedCapabilities = [
  "arbitrary-filesystem",
  "arbitrary-shell",
  "network",
  "device",
  "plc",
  "industrial-communication",
  "deployable-export",
];
if (
  packageContract.schemaVersion !== "1.0" ||
  packageContract.contract !== "govs.windows-fixed-local-shell-package" ||
  packageContract.protocolVersion !== 1 ||
  packageContract.rendererBridge !== "govs.project-file-broker" ||
  packageContract.reviewedRequirementMappingSha256 !==
    manifest.reviewedRequirementMappingSha256 ||
  packageContract.admittedFiles.application.path !== "app/index.html" ||
  packageContract.admittedFiles.application.sha256 !==
    packageRowsByPath.get("app/index.html").sha256 ||
  packageContract.admittedFiles.broker.path !== "windows-project-broker.exe" ||
  packageContract.admittedFiles.broker.sha256 !==
    packageRowsByPath.get("windows-project-broker.exe").sha256 ||
  packageContract.webView2Sdk.package !== "Microsoft.Web.WebView2" ||
  packageContract.webView2Sdk.version !== "1.0.4129.50" ||
  packageContract.webView2Sdk.nugetSha256 !==
    "D3934F482D484B89FB4825DF720C710664E1143A1E90F7B3A60794EF33F473D2" ||
  packageContract.webView2Sdk.loaderSha256 !==
    "482F24196B20E784C4D29B752EA760946CB54E22C2532A29699EF538D2D5C28C" ||
  JSON.stringify(packageContract.rendererBuild) !== JSON.stringify(manifest.rendererBuild) ||
  packageContract.rendererBuild.generatedArtifact.sha256 !==
    packageRowsByPath.get("app/index.html").sha256 ||
  !Array.isArray(packageContract.sourceInputs) ||
  JSON.stringify(packageContract.sourceInputs) !== JSON.stringify(sourceRows) ||
  packageContract.sourceInputManifestSha256 !==
    hashBytes(Buffer.from(stableJson(sourceRows), "utf8")) ||
  JSON.stringify(packageContract.prohibitedCapabilities) !==
    JSON.stringify(expectedProhibitedCapabilities)
) {
  throw new Error("The native package contract content is not exactly bound to the candidate.");
}

const gitText = (arguments_) => {
  const result = spawnSync("git.exe", arguments_, {
    cwd: root,
    encoding: "utf8",
    shell: false,
    windowsHide: true,
  });
  if (result.error !== undefined || result.status !== 0 || result.stdout.trim() === "") {
    throw result.error ?? new Error(`git ${arguments_.join(" ")} failed closed`);
  }
  return result.stdout.trim();
};
if (!developmentDirty) {
  if (
    gitText(["rev-parse", "HEAD"]) !== manifest.gitCommit ||
    gitText(["rev-parse", "HEAD^{tree}"]) !== manifest.gitTree
  ) {
    throw new Error("The native launcher candidate does not match the current commit and tree.");
  }
  for (const file of [sourcePath, scriptPath, finalizerPath, analysisLibraryPath]) {
    const relative = path.relative(root, file).replaceAll("\\", "/");
    const tracked = spawnSync(
      "git.exe", ["ls-files", "--error-unmatch", "--", relative],
      { cwd: root, encoding: "utf8", shell: false, windowsHide: true },
    );
    if (tracked.status !== 0) {
      throw new Error(`Untracked native verification launcher input: ${relative}`);
    }
    for (const mode of [[], ["--cached"]]) {
      const clean = spawnSync(
        "git.exe", ["diff", "--quiet", ...mode, "--", relative],
        { cwd: root, encoding: "utf8", shell: false, windowsHide: true },
      );
      if (clean.status !== 0) {
        throw new Error(`Native verification launcher input differs from HEAD: ${relative}`);
      }
    }
  }
}

const escapeWide = (value) => value
  .replaceAll("\\", "\\\\")
  .replaceAll("\"", "\\\"");
const header = [
  "#pragma once",
  "// Generated from the exact candidate-package-manifest.json. Do not edit.",
  `inline constexpr wchar_t kCandidateManifestSha256[] = L\"${hashBytes(manifestBytes)}\";`,
  `inline constexpr std::uint64_t kCandidateManifestBytes = ${manifestBytes.byteLength}ULL;`,
  `inline constexpr wchar_t kCandidateCommit[] = L\"${manifest.gitCommit}\";`,
  `inline constexpr wchar_t kCandidateTree[] = L\"${manifest.gitTree}\";`,
  `inline constexpr bool kCandidateDevelopmentDirty = ${manifest.developmentDirty ? "true" : "false"};`,
  `inline constexpr wchar_t kCandidatePackageContractSha256[] = L\"${manifest.packageContractSha256}\";`,
  `inline constexpr wchar_t kReviewedRequirementMappingSha256[] = L\"${manifest.reviewedRequirementMappingSha256}\";`,
  `inline constexpr wchar_t kLauncherSourceSha256[] = L\"${await hashFile(sourcePath)}\";`,
  `inline constexpr wchar_t kLauncherBuildScriptSha256[] = L\"${await hashFile(scriptPath)}\";`,
  `inline constexpr wchar_t kNativeEvidenceFinalizerSha256[] = L\"${await hashFile(finalizerPath)}\";`,
  `inline constexpr wchar_t kIsolationAnalysisLibrarySha256[] = L\"${await hashFile(analysisLibraryPath)}\";`,
  "inline constexpr CandidateFile kCandidateFiles[] = {",
  ...rows.map((row) =>
    `    {L\"${escapeWide(row.path)}\", ${row.bytes}ULL, L\"${row.sha256}\"},`),
  "};",
  "",
].join("\r\n");

await mkdir(objectRoot, { recursive: true });
const headerPath = path.join(objectRoot, "native_e2e_candidate.h");
await writeFile(headerPath, header, { encoding: "utf8" });

const vswhere = "C:\\Program Files (x86)\\Microsoft Visual Studio\\Installer\\vswhere.exe";
const located = spawnSync(vswhere, [
  "-latest", "-products", "*",
  "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
  "-property", "installationPath",
], { encoding: "utf8", shell: false, windowsHide: true });
if (located.status !== 0 || located.stdout.trim() === "") {
  throw new Error("The pinned MSVC x64 build environment is unavailable.");
}
const vcvars = path.join(located.stdout.trim(), "VC", "Auxiliary", "Build", "vcvars64.bat");
const objectPath = path.join(objectRoot, "native_e2e_launcher.obj");
const outputPath = path.join(nativeBuild, "Run-Native-E2E.exe");
const commandPath = path.join(objectRoot, "compile-native-e2e-launcher.cmd");
const commands = [
  "@echo off",
  "setlocal",
  `call \"${vcvars}\" >nul`,
  "if errorlevel 1 exit /b %errorlevel%",
  `cl.exe /nologo /c /std:c++20 /EHsc /W4 /WX /permissive- /utf-8 /DNOMINMAX /DUNICODE /D_UNICODE /MT /GS /sdl /guard:cf /O2 /Brepro /I\"${objectRoot}\" /Fo\"${objectPath}\" \"${sourcePath}\"`,
  "if errorlevel 1 exit /b %errorlevel%",
  `link.exe /nologo /Brepro /guard:cf /DYNAMICBASE /NXCOMPAT /HIGHENTROPYVA /CETCOMPAT /SUBSYSTEM:WINDOWS /OUT:\"${outputPath}\" \"${objectPath}\" bcrypt.lib shell32.lib ole32.lib iphlpapi.lib ws2_32.lib version.lib user32.lib advapi32.lib`,
  "exit /b %errorlevel%",
  "",
];
await writeFile(commandPath, commands.join("\r\n"), { encoding: "utf8" });
const built = spawnSync("cmd.exe", ["/d", "/c", commandPath], {
  cwd: root,
  encoding: "utf8",
  shell: false,
  stdio: "inherit",
  windowsHide: true,
});
if (built.error !== undefined || built.status !== 0) {
  throw built.error ?? new Error(`Native E2E launcher build exited ${built.status}`);
}

console.log(`${JSON.stringify({
  result: developmentDirty ? "INCONCLUSIVE_DEVELOPMENT" : "PASS",
  launcher: path.relative(root, outputPath).replaceAll("\\", "/"),
  launcherSha256: await hashFile(outputPath),
  candidateManifestSha256: hashBytes(manifestBytes),
  candidateCommit: manifest.gitCommit,
  candidateTree: manifest.gitTree,
}, null, 2)}\n`);
