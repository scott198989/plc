import { createHash } from "node:crypto";
import { copyFile, lstat, mkdir, readFile, readdir, realpath, rm, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  NATIVE_NODE_VERSION,
  RENDERER_BUILD_RECIPE,
  validatePinnedRendererToolchain,
  validateRendererArtifactInventory,
} from "./native_build_recipe.mjs";
import { validateNativeShellImportText } from "./verify_native_shell_pe_imports.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const rootRealPath = await realpath(root);
const verificationRoot = path.join(root, ".phase2-verification", "native-build");
const packageRoot = path.join(verificationRoot, "package");
const objectRoot = path.join(verificationRoot, "obj");
const vendorRoot = path.join(root, "vendor", "microsoft-webview2", "1.0.4129.50");
const appSource = path.join(root, "dist", "index.html");
const brokerSource = path.join(root, "target", "release", "windows-project-broker.exe");
const reviewedRequirementMapping = path.join(
  root,
  "requirements",
  "phase2-reviewed-requirement-mapping.json",
);
const developmentDirty = process.argv.slice(2).length === 1 &&
  process.argv[2] === "--development-dirty";
if (process.versions.node !== NATIVE_NODE_VERSION) {
  throw new Error(`The strict native build requires Node ${NATIVE_NODE_VERSION}; observed ${process.versions.node}.`);
}

if (process.platform !== "win32") {
  throw new Error("The approved native shell build is Windows-only.");
}
if (process.argv.slice(2).length !== (developmentDirty ? 1 : 0)) {
  throw new Error("Only the explicit --development-dirty pre-commit build mode is accepted.");
}

const run = (command, args) => {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    shell: false,
    stdio: "inherit",
    windowsHide: true,
  });
  if (result.error !== undefined || result.status !== 0) {
    throw result.error ?? new Error(`${command} exited ${result.status}`);
  }
};
const runCapture = (command, args) => {
  const result = spawnSync(command, args, {
    cwd: root, encoding: "utf8", shell: false, windowsHide: true,
  });
  if (result.error !== undefined || result.status !== 0) {
    throw result.error ?? new Error(`${command} ${args.join(" ")} exited ${result.status}`);
  }
  return `${result.stdout}${result.stderr}`;
};

const hashBytes = (bytes) => createHash("sha256").update(bytes).digest("hex").toUpperCase();
const hashFile = async (file) => hashBytes(await readFile(file));
const isContained = (base, candidate) => {
  const relation = path.relative(base, candidate);
  return relation === "" || (!relation.startsWith(`..${path.sep}`) && relation !== "..");
};
const assertRegularFile = async (file, base = rootRealPath) => {
  const info = await lstat(file);
  if (!info.isFile() || info.isSymbolicLink()) {
    throw new Error(`Candidate inventory rejects non-regular or linked file: ${file}`);
  }
  const resolved = await realpath(file);
  const resolvedBase = base === null ? null : (base === rootRealPath ? rootRealPath : await realpath(base));
  if (resolvedBase !== null && !isContained(resolvedBase, resolved)) {
    throw new Error(`Candidate inventory escapes its approved root: ${file}`);
  }
  return resolved;
};
const resolvedPnpm = (() => {
  const result = spawnSync("where.exe", ["pnpm.cmd"], {
    cwd: root, encoding: "utf8", shell: false, windowsHide: true,
  });
  const candidate = result.stdout.split(/\r?\n/u).find((value) => value.trim() !== "")?.trim();
  if (result.error !== undefined || result.status !== 0 || candidate === undefined) {
    throw result.error ?? new Error("The pinned pnpm.cmd wrapper is unavailable.");
  }
  return path.resolve(candidate);
})();
const resolvePnpmEntry = async () => {
  const wrapper = await readFile(resolvedPnpm, "utf8");
  const match = /%~dp0([^"\r\n]+pnpm\.(?:mjs|cjs))/iu.exec(wrapper);
  if (match === null) throw new Error("The resolved pnpm wrapper does not bind one pinned pnpm entry module.");
  return path.resolve(path.dirname(resolvedPnpm), ...match[1].replaceAll("/", "\\").split("\\"));
};
const relative = (file) => path.relative(root, file).replaceAll("\\", "/");
const fileRow = async (file) => {
  await assertRegularFile(file);
  const bytes = await readFile(file);
  return { path: relative(file), bytes: bytes.byteLength, sha256: hashBytes(bytes) };
};
const stableJson = (value) => `${JSON.stringify(value, null, 2)}\n`;
const walkFiles = async (directory, accept) => {
  const output = [];
  const directoryInfo = await lstat(directory);
  if (!directoryInfo.isDirectory() || directoryInfo.isSymbolicLink()) {
    throw new Error(`Candidate inventory rejects non-directory or linked directory: ${directory}`);
  }
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const candidate = path.join(directory, entry.name);
    const info = await lstat(candidate);
    if (info.isSymbolicLink()) {
      throw new Error(`Candidate inventory rejects linked entry: ${candidate}`);
    }
    if (info.isDirectory()) {
      output.push(...await walkFiles(candidate, accept));
    } else if (info.isFile() && accept(candidate)) {
      output.push(candidate);
    } else if (!info.isFile()) {
      throw new Error(`Candidate inventory rejects special entry: ${candidate}`);
    }
  }
  return output;
};
const gitText = (args) => {
  const result = spawnSync("git.exe", args, {
    cwd: root,
    encoding: "utf8",
    shell: false,
    windowsHide: true,
  });
  if (result.error !== undefined || result.status !== 0 || result.stdout.trim() === "") {
    throw result.error ?? new Error(`git ${args.join(" ")} failed closed`);
  }
  return result.stdout.trim();
};
const assertSortedUniqueRows = (rows, label) => {
  const paths = rows.map((row) => row.path);
  const sorted = [...paths].sort((left, right) => left.localeCompare(right, "en"));
  if (new Set(paths).size !== paths.length ||
      paths.some((entry, index) => entry !== sorted[index])) {
    throw new Error(`${label} must be sorted and unique.`);
  }
};
const verifyRows = async (rows, base = root) => {
  for (const row of rows) {
    const file = path.resolve(base, ...row.path.split("/"));
    await assertRegularFile(file, base);
    const bytes = await readFile(file);
    if (bytes.byteLength !== row.bytes || hashBytes(bytes) !== row.sha256) {
      throw new Error(`Candidate-bound input drifted: ${row.path}`);
    }
  }
};

const vendorFiles = [
  "include/WebView2.h",
  "include/WebView2EnvironmentOptions.h",
  "lib/x64/WebView2LoaderStatic.lib",
  "LICENSE.txt",
  "NOTICE.txt",
  "Microsoft.Web.WebView2.nuspec",
  "PROVENANCE.md",
  "FILES.sha256",
].map((entry) => path.join(vendorRoot, ...entry.split("/")));
const reviewedVendorSha256 = new Map(Object.entries({
  "include/WebView2.h": "DFF1E3181EC7EC203A34EF6EFA966590E0EF0BA1A5C3FE3B69DA6508C2F8A02E",
  "include/WebView2EnvironmentOptions.h": "06F44F0569F1415C37CCD9EB6BADE28B803646A73EDCE78B40F7AA8548D015B9",
  "lib/x64/WebView2LoaderStatic.lib": "482F24196B20E784C4D29B752EA760946CB54E22C2532A29699EF538D2D5C28C",
  "LICENSE.txt": "0AF8F1B807512AAE39C2AC1AA4D0CAE65CABECB6FD554B8439A5162A0D6ECA55",
  "NOTICE.txt": "106423785C5B7EBA0A8E61D1837F2132E9C828E20AD530F565D981C1DF60DD90",
  "Microsoft.Web.WebView2.nuspec": "D723E92FAD93DEF946B4D08EA08B5A20A07FDEAC6B5BB9A208A60867961CBB81",
}));
for (const [entry, expected] of reviewedVendorSha256) {
  if (await hashFile(path.join(vendorRoot, ...entry.split("/"))) !== expected) {
    throw new Error(`The reviewed WebView2 package input drifted: ${entry}`);
  }
}
const nativeSources = [
  "ADR/0005-phase-2-native-isolation-shell.md",
  "apps/windows-shell/src/main.cpp",
  "apps/windows-shell/src/broker_client.cpp",
  "apps/windows-shell/src/broker_client.h",
  "apps/windows-shell/src/bridge_protocol.cpp",
  "apps/windows-shell/src/bridge_protocol.h",
  "apps/foundation-shell/src/file-access-broker.ts",
  "crates/windows-project-broker/Cargo.toml",
  "crates/windows-project-broker/README.md",
  "crates/windows-project-broker/src/lib.rs",
  "crates/windows-project-broker/src/main.rs",
  "crates/windows-project-broker/src/protocol.rs",
  "crates/windows-project-broker/src/sha256.rs",
  "crates/windows-project-broker/src/windows.rs",
  "Cargo.toml",
  "Cargo.lock",
  "rust-toolchain.toml",
  "requirements/phase2-reviewed-requirement-mapping.json",
  "tools/phase2/build_windows_shell.mjs",
].map((entry) => path.join(root, ...entry.split("/")));
const candidateSourceFiles = new Set(nativeSources);
for (const directory of [
  path.join(root, "apps", "foundation-shell", "src"),
  path.join(root, "packages", "foundation-contract", "src"),
  path.join(root, "packages", "plc-contract", "src"),
]) {
  for (const file of await walkFiles(directory, (candidate) =>
    !candidate.startsWith(path.join(root, "apps", "foundation-shell", "src", "generated") + path.sep) &&
    [".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"].includes(path.extname(candidate).toLowerCase()))) {
    candidateSourceFiles.add(file);
  }
}
for (const crate of await readdir(path.join(root, "crates"), { withFileTypes: true })) {
  if (!crate.isDirectory()) continue;
  const crateRoot = path.join(root, "crates", crate.name);
  candidateSourceFiles.add(path.join(crateRoot, "Cargo.toml"));
  for (const file of await walkFiles(path.join(crateRoot, "src"), (candidate) =>
    path.extname(candidate).toLowerCase() === ".rs")) {
    candidateSourceFiles.add(file);
  }
}
for (const entry of [
  "apps/foundation-shell/index.html",
  "apps/foundation-shell/package.json",
  "apps/foundation-shell/tsconfig.app.json",
  "apps/foundation-shell/tsconfig.base.json",
  "apps/foundation-shell/tsconfig.worker.json",
  "apps/foundation-shell/vite.config.ts",
  "packages/foundation-contract/package.json",
  "packages/foundation-contract/tsconfig.json",
  "packages/plc-contract/package.json",
  "packages/plc-contract/tsconfig.json",
  "package.json",
  "pnpm-lock.yaml",
  "pnpm-workspace.yaml",
  "tools/foundation/build-wasm.mjs",
  "tools/foundation/embed-wasm.mjs",
  "tools/foundation/inline-shell.mjs",
  "tools/foundation/assert-toolchain.mjs",
  "tools/phase2/build_phase2_native.mjs",
  "tools/phase2/build_native_e2e_launcher.mjs",
  "tools/phase2/build-engineering-wasm.mjs",
  "tools/phase2/embed-engineering-wasm.mjs",
  "tools/phase2/finalize_native_e2e_evidence.mjs",
  "tools/phase2/build_external_observer.mjs",
  "tools/phase2/external_observer_evidence.mjs",
  "tools/phase2/finalize_external_observer_evidence.mjs",
  "tools/phase2/isolation-counterfactual-lib.mjs",
  "tools/phase2/native_e2e_launcher.cpp",
  "tools/phase2/native_build_recipe.mjs",
  "tools/phase2/verify_external_observer_source.mjs",
  "tools/phase2/verify_native_shell_api_allowlist.mjs",
  "tools/phase2/verify_native_shell_pe_imports.mjs",
  "tools/phase2/windows_external_observer.cpp",
]) {
  candidateSourceFiles.add(path.join(root, ...entry.split("/")));
}
const sourceInputFiles = [...new Set([
  ...candidateSourceFiles,
  ...vendorFiles,
])];
const sourceInputs = await Promise.all(sourceInputFiles.map(fileRow));
sourceInputs.sort((left, right) => left.path.localeCompare(right.path, "en"));
assertSortedUniqueRows(sourceInputs, "Native source input inventory");
await verifyRows(sourceInputs);
const reviewedRequirementMappingSha256 = await hashFile(reviewedRequirementMapping);

const gitCommit = gitText(["rev-parse", "HEAD"]);
const gitTree = gitText(["rev-parse", "HEAD^{tree}"]);
if (!developmentDirty) {
  const trackedProductionInputs = [...candidateSourceFiles, ...vendorFiles].map(relative);
  for (const entry of trackedProductionInputs) {
    const tracked = spawnSync(
      "git.exe", ["ls-files", "--error-unmatch", "--", entry],
      { cwd: root, encoding: "utf8", shell: false, windowsHide: true },
    );
    if (tracked.status !== 0) {
      throw new Error(`Untracked native production input: ${entry}`);
    }
  }
  for (const mode of [[], ["--cached"]]) {
    const clean = spawnSync(
      "git.exe", ["diff", "--quiet", ...mode, "--", ...trackedProductionInputs],
      { cwd: root, encoding: "utf8", shell: false, windowsHide: true },
    );
    if (clean.status !== 0) {
      throw new Error("Native production inputs do not match the exact candidate commit.");
    }
  }
}

// Resolve and hash every renderer tool before any pnpm command can execute.
// Neither PATH's node.exe nor a mutable wrapper/entry may receive candidate
// credit merely because it reports a compatible version after the fact.
const pnpmEntry = await resolvePnpmEntry();
await Promise.all([
  assertRegularFile(process.execPath, null),
  assertRegularFile(resolvedPnpm, null),
  assertRegularFile(pnpmEntry, null),
]);
const rendererToolchain = validatePinnedRendererToolchain({
  nodeExecutableSha256: await hashFile(process.execPath),
  nodeVersion: process.versions.node,
  pnpmEntrySha256: await hashFile(pnpmEntry),
  pnpmWrapperSha256: await hashFile(resolvedPnpm),
  // pnpm's own version is checked immediately below, after its wrapper and
  // entry have passed byte-for-byte admission.
  pnpmVersion: "11.19.0",
});

// dist/index.html is deliberately generated and ignored. It is never an
// admissible input: only this candidate-bound, offline recipe may create it.
await rm(path.join(root, "dist"), { force: true, recursive: true });
await mkdir(path.join(root, "dist"), { recursive: true });
run(process.execPath, [path.join(root, "tools", "phase2", "verify_native_shell_api_allowlist.mjs")]);
run(process.execPath, [path.join(root, "tools", "foundation", "assert-toolchain.mjs")]);
const observedPnpmVersion = runCapture(resolvedPnpm, ["--version"]).trim();
validatePinnedRendererToolchain({ ...rendererToolchain, pnpmVersion: observedPnpmVersion });
const pnpmStoreStatus = runCapture(resolvedPnpm, ["store", "status"]);
run(resolvedPnpm, ["--offline", "--frozen-lockfile", "run", "wasm:all:embed"]);
run(resolvedPnpm, ["--offline", "--frozen-lockfile", "--filter", "@govs/foundation-shell", "build"]);
run(process.execPath, [path.join(root, "tools", "foundation", "inline-shell.mjs")]);
const generatedFiles = (await walkFiles(path.join(root, "dist"), () => true)).map(relative).sort((left, right) => left.localeCompare(right, "en"));
const generatedRenderer = validateRendererArtifactInventory(await Promise.all(
  generatedFiles.map((entry) => fileRow(path.join(root, ...entry.split("/")))),
));
const rendererBuild = {
  ...RENDERER_BUILD_RECIPE,
  nodeExecutableSha256: rendererToolchain.nodeExecutableSha256,
  pnpmEntrySha256: rendererToolchain.pnpmEntrySha256,
  pnpmStoreStatusSha256: hashBytes(Buffer.from(pnpmStoreStatus, "utf8")),
  pnpmWrapperSha256: rendererToolchain.pnpmWrapperSha256,
  generatedArtifact: generatedRenderer,
  recipeSha256: hashBytes(Buffer.from(stableJson(RENDERER_BUILD_RECIPE), "utf8")),
};

run("rustup.exe", [
  "run", "1.94.0-x86_64-pc-windows-msvc", "cargo.exe",
  "build", "-p", "windows-project-broker", "--release", "--locked", "--offline",
]);
await rm(verificationRoot, { force: true, recursive: true });
await Promise.all([
  mkdir(path.join(packageRoot, "app"), { recursive: true }),
  mkdir(path.join(packageRoot, "third-party"), { recursive: true }),
  mkdir(objectRoot, { recursive: true }),
]);

const brokerHash = await hashFile(brokerSource);
const appHash = await hashFile(appSource);

const packageContract = {
  schemaVersion: "1.0",
  contract: "govs.windows-fixed-local-shell-package",
  protocolVersion: 1,
  rendererBridge: "govs.project-file-broker",
  reviewedRequirementMappingSha256,
  admittedFiles: {
    application: { path: "app/index.html", sha256: appHash },
    broker: { path: "windows-project-broker.exe", sha256: brokerHash },
  },
  webView2Sdk: {
    package: "Microsoft.Web.WebView2",
    version: "1.0.4129.50",
    nugetSha256: "D3934F482D484B89FB4825DF720C710664E1143A1E90F7B3A60794EF33F473D2",
    loaderSha256: "482F24196B20E784C4D29B752EA760946CB54E22C2532A29699EF538D2D5C28C",
  },
  rendererBuild,
  sourceInputs,
  sourceInputManifestSha256: hashBytes(Buffer.from(stableJson(sourceInputs), "utf8")),
  prohibitedCapabilities: [
    "arbitrary-filesystem", "arbitrary-shell", "network", "device", "plc",
    "industrial-communication", "deployable-export",
  ],
};
const contractPath = path.join(packageRoot, "package-contract-v1.json");
await writeFile(contractPath, stableJson(packageContract), { encoding: "utf8", flag: "wx" });
const contractHash = await hashFile(contractPath);

await Promise.all([
  copyFile(appSource, path.join(packageRoot, "app", "index.html")),
  copyFile(brokerSource, path.join(packageRoot, "windows-project-broker.exe")),
  copyFile(path.join(vendorRoot, "LICENSE.txt"), path.join(packageRoot, "third-party", "Microsoft.Web.WebView2-LICENSE.txt")),
  copyFile(path.join(vendorRoot, "NOTICE.txt"), path.join(packageRoot, "third-party", "Microsoft.Web.WebView2-NOTICE.txt")),
  copyFile(path.join(vendorRoot, "PROVENANCE.md"), path.join(packageRoot, "third-party", "Microsoft.Web.WebView2-PROVENANCE.md")),
]);

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
const sources = ["main.cpp", "broker_client.cpp", "bridge_protocol.cpp"];
const common = [
  "/nologo", "/std:c++20", "/EHsc", "/W4", "/WX", "/permissive-", "/utf-8",
  "/DNOMINMAX", "/DUNICODE", "/D_UNICODE", "/MT", "/GS", "/sdl", "/guard:cf", "/O2", "/Brepro",
  `/DGOVS_BROKER_SHA256=L\\\"${brokerHash}\\\"`,
  `/DGOVS_APP_SHA256=L\\\"${appHash}\\\"`,
  `/DGOVS_PACKAGE_CONTRACT_SHA256=L\\\"${contractHash}\\\"`,
  `/I\"${path.join(vendorRoot, "include")}\"`,
].join(" ");
const commands = [
  "@echo off",
  "setlocal",
  `call \"${vcvars}\" >nul`,
  "if errorlevel 1 exit /b %errorlevel%",
];
const importInventoryPath = path.join(verificationRoot, "GovsPLC.imports.txt");
for (const source of sources) {
  const stem = path.parse(source).name;
  commands.push(
    `cl.exe /c ${common} /Fo\"${path.join(objectRoot, `${stem}.obj`)}\" \"${path.join(root, "apps", "windows-shell", "src", source)}\"`,
    "if errorlevel 1 exit /b %errorlevel%",
  );
}
const objects = sources.map((source) => `\"${path.join(objectRoot, `${path.parse(source).name}.obj`)}\"`).join(" ");
const libraries = [
  path.join(vendorRoot, "lib", "x64", "WebView2LoaderStatic.lib"),
  "user32.lib", "gdi32.lib", "ole32.lib", "shell32.lib", "bcrypt.lib",
  "runtimeobject.lib", "advapi32.lib",
].map((entry) => entry.endsWith(".lib") && path.isAbsolute(entry) ? `\"${entry}\"` : entry).join(" ");
commands.push(
  `link.exe /nologo /Brepro /guard:cf /DYNAMICBASE /NXCOMPAT /HIGHENTROPYVA /CETCOMPAT /SUBSYSTEM:WINDOWS /OUT:\"${path.join(packageRoot, "GovsPLC.exe")}\" ${objects} ${libraries}`,
  "if errorlevel 1 exit /b %errorlevel%",
  `dumpbin.exe /imports \"${path.join(packageRoot, "GovsPLC.exe")}\" > \"${importInventoryPath}\"`,
  "if errorlevel 1 exit /b %errorlevel%",
  "exit /b %errorlevel%",
);
const commandPath = path.join(verificationRoot, "compile-native-shell.cmd");
await writeFile(commandPath, `${commands.join("\r\n")}\r\n`, { encoding: "utf8", flag: "wx" });
run("cmd.exe", ["/d", "/c", commandPath]);
validateNativeShellImportText(await readFile(importInventoryPath, "utf8"));

const packageFiles = [
  "GovsPLC.exe",
  "windows-project-broker.exe",
  "app/index.html",
  "package-contract-v1.json",
  "third-party/Microsoft.Web.WebView2-LICENSE.txt",
  "third-party/Microsoft.Web.WebView2-NOTICE.txt",
  "third-party/Microsoft.Web.WebView2-PROVENANCE.md",
];
const packageRows = await Promise.all(packageFiles.map(async (entry) => {
  const file = path.join(packageRoot, ...entry.split("/"));
  const bytes = await readFile(file);
  return { path: entry, bytes: bytes.byteLength, sha256: hashBytes(bytes) };
}));
packageRows.sort((left, right) => left.path.localeCompare(right.path, "en"));
assertSortedUniqueRows(packageRows, "Native package inventory");
await verifyRows(packageRows, packageRoot);
const candidateManifest = {
  schemaVersion: "1.0",
  evidenceKind: "WINDOWS_NATIVE_CANDIDATE_PACKAGE_MANIFEST",
  gitCommit,
  gitTree,
  developmentDirty,
  packageContractSha256: contractHash,
  reviewedRequirementMappingSha256,
  packageFiles: packageRows,
  rendererBuild,
  sourceInputs,
};
const candidatePath = path.join(packageRoot, "candidate-package-manifest.json");
await writeFile(candidatePath, stableJson(candidateManifest), { encoding: "utf8", flag: "wx" });
await verifyRows(sourceInputs);
await verifyRows(packageRows, packageRoot);
console.log(stableJson({
  result: "PASS",
  package: relative(packageRoot),
  candidateManifestSha256: await hashFile(candidatePath),
  shellSha256: await hashFile(path.join(packageRoot, "GovsPLC.exe")),
  brokerSha256: brokerHash,
  applicationSha256: appHash,
  packageContractSha256: contractHash,
  reviewedRequirementMappingSha256,
}).trim());
