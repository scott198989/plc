import { once } from "node:events";
import { access, mkdir, readFile } from "node:fs/promises";
import { createServer } from "node:http";
import path from "node:path";

import { chromium } from "playwright-core";

const projectRoot = path.resolve(import.meta.dirname, "../..");
const stagingMode = process.argv.includes("--staging");
const artifactPath = stagingMode
  ? path.join(projectRoot, "dist", "foundation-staging", "index.html")
  : path.join(projectRoot, "dist", "index.html");
const evidenceDirectory = path.join(projectRoot, ".phase2-verification", "P2-02");
const browserCandidates = [
  "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
  "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
  "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
];

await access(artifactPath);
await mkdir(evidenceDirectory, { recursive: true });
const artifact = await readFile(artifactPath);
const stagingAssets = stagingMode
  ? new Map(await Promise.all([
      ["/foundation.js", "text/javascript; charset=utf-8"],
      ["/foundation.css", "text/css; charset=utf-8"],
    ].map(async ([requestPath, contentType]) => [
      requestPath,
      {
        bytes: await readFile(path.join(path.dirname(artifactPath), requestPath.slice(1))),
        contentType,
      },
    ])))
  : new Map();
const server = createServer((request, response) => {
  const requestUrl = new URL(request.url ?? "/", "http://127.0.0.1");
  const stagingAsset = stagingAssets.get(requestUrl.pathname);
  if (stagingAsset !== undefined) {
    response.writeHead(200, {
      "Cache-Control": "no-store",
      "Content-Length": String(stagingAsset.bytes.byteLength),
      "Content-Type": stagingAsset.contentType,
    });
    response.end(stagingAsset.bytes);
    return;
  }
  if (requestUrl.pathname !== "/" && requestUrl.pathname !== "/index.html") {
    response.writeHead(404, { "Content-Type": "text/plain; charset=utf-8" });
    response.end("Not found");
    return;
  }

  response.writeHead(200, {
    "Cache-Control": "no-store",
    "Content-Length": String(artifact.byteLength),
    "Content-Type": "text/html; charset=utf-8",
  });
  response.end(artifact);
});
server.listen(0, "127.0.0.1");
await once(server, "listening");
const address = server.address();
if (address === null || typeof address === "string") {
  throw new Error("Loopback artifact server did not expose a TCP port.");
}
const artifactUrl = `http://127.0.0.1:${address.port}/`;
const artifactOrigin = new URL(artifactUrl).origin;
const browserPath = await findBrowser();
const browser = await chromium.launch({
  args: [
    "--disable-background-networking",
    "--disable-breakpad",
    "--disable-component-update",
    "--disable-default-apps",
    "--disable-domain-reliability",
    "--disable-features=OptimizationHints,MediaRouter,Translate",
    "--disable-sync",
    "--metrics-recording-only",
    "--no-first-run",
    "--no-pings",
  ],
  executablePath: browserPath,
  headless: true,
});

try {
  const context = await browser.newContext({
    javaScriptEnabled: true,
    locale: "en-US",
    viewport: { height: 920, width: 1586 },
  });
  await installMemoryFileAccess(context);
  const page = await context.newPage();
  const pageErrors = [];
  const remoteRequests = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await installNetworkBoundary(context, artifactOrigin, remoteRequests);

  await page.goto(artifactUrl, { waitUntil: "load" });
  await page.getByText("Core plc-engineering-core@0.2.0", { exact: true }).waitFor();
  await page.getByLabel("Project name").fill("End-to-end cell");
  await page.getByRole("button", { name: /^Create/u }).click();
  await page.getByRole("heading", { level: 1, name: "End-to-end cell" }).waitFor();
  await page.getByRole("status", { name: "Unsaved changes", exact: true }).waitFor();
  await page.getByText("EDU-SYS-1001", { exact: true }).waitFor();

  await page.getByRole("tab", { name: "Runtime & commissioning" }).click();
  const unavailableDiagnostic = page
    .locator(".runtime-unavailable__diagnostic")
    .filter({ hasText: "EDU-SYS-1001" });
  await unavailableDiagnostic.waitFor();
  if (await unavailableDiagnostic.getAttribute("aria-disabled") !== "false") {
    throw new Error("the unavailable-runtime diagnostic did not expose semantic navigation");
  }
  await unavailableDiagnostic.click();
  await page.getByRole("heading", { level: 1, name: "End-to-end cell" }).waitFor();
  await page.getByRole("tab", { name: "Runtime & commissioning" }).click();

  await addObject(page, "Virtual network");
  await page.getByRole("heading", { level: 1, name: "Virtual network" }).waitFor();
  await treeItem(page, "End-to-end cell").click();
  await addObject(page, "Controller");
  await page.getByRole("heading", { level: 1, name: "Controller" }).waitFor();
  await addObject(page, "Rack");
  await page.getByRole("heading", { level: 1, name: "Local rack" }).waitFor();
  await addObject(page, "Digital input module");
  await page.getByRole("heading", { level: 1, name: "VDI16" }).waitFor();
  await treeItem(page, "Local rack").click();
  await addObject(page, "Digital output module");
  await page.getByRole("heading", { level: 1, name: "VDO16" }).waitFor();
  await treeItem(page, "Local rack").click();
  await addObject(page, "Analog input module");
  await page.getByRole("heading", { level: 1, name: "VAI4" }).waitFor();
  await treeItem(page, "Local rack").click();
  await addObject(page, "Analog output module");
  await page.getByRole("heading", { level: 1, name: "VAO4" }).waitFor();

  await treeItem(page, "Controller").click();
  await addObject(page, "Reusable SCL function");
  await page.getByRole("heading", { level: 1, name: "Function" }).waitFor();
  const sclSource = "Result := NOT InputValue;";
  await page.getByLabel("SCL source").fill(sclSource);
  await page.getByRole("button", { name: "Apply SCL source" }).click();
  if (await page.getByLabel("SCL source").inputValue() !== sclSource) {
    throw new Error("canonical SCL source did not survive the production worker round trip");
  }

  await treeItem(page, "Controller").click();
  await addObject(page, "FBD function");
  await page.getByRole("heading", { level: 1, name: "FbdFunction" }).waitFor();
  await page.getByRole("region", { name: "FBD network 1" }).waitFor();
  await page.getByText("NOT", { exact: true }).waitFor();
  const fbdScreenshot = path.join(evidenceDirectory, "workbench-fbd-editor.png");
  await page.screenshot({ fullPage: true, path: fbdScreenshot });

  await treeItem(page, "Controller").click();
  await addObject(page, "Ladder organization block");
  await page.getByRole("heading", { level: 1, name: "MainCycle" }).waitFor();
  await page.getByRole("region", { name: "LAD network 1" }).waitFor();
  await page.getByText("CALL", { exact: true }).first().waitFor();
  if (await page.getByText("CALL", { exact: true }).count() !== 2) {
    throw new Error("mixed-language LAD block did not retain both real FC call nodes");
  }
  const contactMode = page.getByLabel("Contact").first();
  await contactMode.selectOption("normally-closed");
  await contactMode.selectOption("normally-open");
  const ladderScreenshot = path.join(evidenceDirectory, "workbench-lad-editor.png");
  await page.screenshot({ fullPage: true, path: ladderScreenshot });

  await treeItem(page, "Controller").click();
  await addObject(page, "State-owning SCL block");
  await page.getByRole("heading", { level: 1, name: "StateBlock" }).waitFor();
  await page.getByLabel("SCL source").fill("Accumulator := InputValue;\nResult := Accumulator;");
  await page.getByRole("button", { name: "Apply SCL source" }).click();

  await treeItem(page, "Controller").click();
  await addObject(page, "Global data block");
  await page.getByRole("heading", { level: 1, name: "GlobalData" }).waitFor();
  await page.getByRole("button", { name: "Add member", exact: true }).click();
  const globalMemberRows = page.locator(".member-editor__table tbody tr");
  await globalMemberRows.nth(1).getByRole("textbox").fill("BatchCount");
  await globalMemberRows.nth(1).getByRole("combobox").selectOption("DINT");
  await globalMemberRows.nth(1).getByRole("checkbox").check();
  await applyMemberChanges(page);
  await page.getByText("2 members", { exact: true }).waitFor();
  await treeItem(page, "Controller").click();
  await addObject(page, "Instance data block");
  await page.getByRole("heading", { level: 1, name: "InstanceData" }).waitFor();
  await treeItem(page, "Controller").click();
  await addObject(page, "Named user structure");
  await page.getByRole("heading", { level: 1, name: "ProcessData" }).waitFor();
  await page.getByRole("button", { name: "Add array", exact: true }).click();
  const typeMemberRows = page.locator(".member-editor__table tbody tr");
  await typeMemberRows.nth(1).getByRole("textbox").fill("Samples");
  await typeMemberRows.nth(1).getByRole("combobox").selectOption("UINT");
  await typeMemberRows.nth(1).getByLabel("Lower").fill("-2");
  await typeMemberRows.nth(1).getByLabel("Upper").fill("12");
  await applyMemberChanges(page);
  await page.getByText("2 members", { exact: true }).waitFor();
  const typeEditorScreenshot = path.join(evidenceDirectory, "workbench-type-editor.png");
  await page.screenshot({ fullPage: true, path: typeEditorScreenshot });

  await treeItem(page, "Controller").click();
  await addObject(page, "Tag table");
  await page.getByRole("heading", { level: 1, name: "PLC tags" }).waitFor();
  await addObject(page, "Input tag");
  await page.getByRole("heading", { level: 1, name: "Input" }).waitFor();
  await treeItem(page, "PLC tags").click();
  await addObject(page, "Output tag");
  await page.getByRole("heading", { level: 1, name: "Output" }).waitFor();

  await treeItem(page, "Controller").click();
  await addObject(page, "Watch table");
  await page.getByRole("heading", { level: 1, name: "Watch table" }).waitFor();
  await treeItem(page, "Controller").click();
  await addObject(page, "Trace configuration");
  await page.getByRole("heading", { level: 1, name: "Trace" }).waitFor();

  await treeItem(page, "End-to-end cell").click();
  await addObject(page, "Folder");
  await page.getByRole("heading", { level: 1, name: "Engineering folder" }).waitFor();
  await page.getByRole("button", { name: "Duplicate with new identity" }).click();
  await treeItem(page, "Engineering folder copy").waitFor();
  await page.getByRole("button", { name: "Delete object" }).click();
  await page.getByRole("button", { name: "Undo last committed change" }).click();
  await treeItem(page, "Engineering folder").waitFor();
  await page.getByRole("button", { name: "Redo last reverted change" }).click();
  await treeItem(page, "Engineering folder").waitFor({ state: "detached" });

  await treeItem(page, "End-to-end cell").click();

  await page.getByLabel("Name").fill("Renamed training cell");
  await page.getByRole("button", { name: "Apply name" }).click();
  await page.getByRole("heading", { level: 1, name: "Renamed training cell" }).waitFor();

  await page.getByRole("button", { name: "Undo last committed change" }).click();
  await page.getByRole("heading", { level: 1, name: "End-to-end cell" }).waitFor();
  await page.getByRole("button", { name: "Redo last reverted change" }).click();
  await page.getByRole("heading", { level: 1, name: "Renamed training cell" }).waitFor();

  await page.getByRole("tab", { name: "Diagnostics" }).click();
  await page.getByRole("status", {
    name: "Canonical project state has no diagnostics.",
    exact: true,
  }).waitFor();
  await page.getByRole("tab", { name: "Runtime & commissioning" }).click();
  await page.getByText("Runtime probes", { exact: true }).waitFor();

  await page.getByRole("button", { name: "Build", exact: true }).click();
  await waitForBuildCurrent(page);
  await page.getByRole("button", { name: "Power on", exact: true }).click();
  await waitForLocatorText(
    page.locator(".runtime-toolbar__identity strong"),
    (value) => value.trim() === "Stop",
    "virtual controller did not reach STOP after power-on",
  );

  await page.getByRole("button", { name: "Preview load", exact: true }).click();
  const loadPreview = page.getByRole("region", { name: "Virtual Download preview" });
  await loadPreview.waitFor();
  const candidateFingerprint = (await loadPreview.locator("small").innerText()).trim();
  if (!/^[0-9a-f]{10}…[0-9a-f]{6}$/iu.test(candidateFingerprint)) {
    throw new Error(`load preview did not expose a canonical candidate fingerprint: ${candidateFingerprint}`);
  }
  await loadPreview.getByText("0", { exact: true }).last().waitFor();
  await page.getByRole("button", { name: "Commit load", exact: true }).click();
  await loadPreview.waitFor({ state: "detached" });
  await waitForEnabled(
    page.getByRole("button", { name: "Go online", exact: true }),
    true,
    "committed load did not enable Go online",
  );

  await page.getByRole("button", { name: "Go online", exact: true }).click();
  await page.getByText("Online session active", { exact: true }).waitFor();
  await page.getByRole("button", { name: "Start monitoring", exact: true }).click();
  await page.getByRole("button", { name: "RUN", exact: true }).click();
  await waitForLocatorText(
    page.locator(".runtime-toolbar__identity strong"),
    (value) => value.trim() === "Run",
    "virtual controller did not enter RUN",
  );

  const inputProbe = runtimeProbe(page, "Input");
  const outputProbe = runtimeProbe(page, "Output");
  await inputProbe.waitFor();
  await outputProbe.waitFor();

  await inputProbe.getByLabel("Value for Input", { exact: true }).selectOption("true");
  await inputProbe.getByRole("button", { name: "Set raw", exact: true }).click();
  await page.getByRole("button", { name: "Scan +1", exact: true }).click();
  await waitForRuntimeCell(inputProbe, 1, "TRUE", "input natural value after first scan");
  await waitForRuntimeCell(outputProbe, 1, "FALSE", "mixed-language output after TRUE input");
  await waitForScanSequence(page, "1");

  await inputProbe.getByLabel("Value for Input", { exact: true }).selectOption("false");
  await inputProbe.getByRole("button", { name: "Set raw", exact: true }).click();
  await page.getByRole("button", { name: "Scan +1", exact: true }).click();
  await waitForRuntimeCell(inputProbe, 1, "FALSE", "input natural value after second scan");
  await waitForRuntimeCell(outputProbe, 1, "TRUE", "mixed-language output after FALSE input");
  await waitForScanSequence(page, "2");

  await waitForWatchValues(page, ["FALSE", "TRUE"]);

  await outputProbe.getByLabel("Value for Output", { exact: true }).selectOption("false");
  await outputProbe.getByRole("button", { name: "Modify", exact: true }).click();
  await waitForRuntimeCell(outputProbe, 2, "FALSE", "one-shot modified output");
  await outputProbe.getByLabel("Value for Output", { exact: true }).selectOption("true");
  await outputProbe.getByRole("button", { name: "Force", exact: true }).click();
  await outputProbe.getByText("FORCED", { exact: true }).waitFor();
  await waitForRuntimeCell(outputProbe, 2, "TRUE", "forced output");
  await outputProbe.getByRole("button", { name: "Remove force", exact: true }).click();
  await outputProbe.getByRole("button", { name: "Force", exact: true }).waitFor();
  if (await outputProbe.getByText("FORCED", { exact: true }).count() !== 0) {
    throw new Error("force provenance remained visible after removing the force");
  }

  const traceRow = page.locator(".trace-row").filter({ hasText: "Trace" });
  await traceRow.getByRole("button", { name: "Arm", exact: true }).click();
  await waitForLocatorText(
    traceRow,
    (value) => value.includes("ARMED"),
    "trace did not enter ARMED",
  );
  await page.getByRole("button", { name: "Scan +1", exact: true }).click();
  await waitForLocatorText(
    traceRow,
    (value) => value.includes("CAPTURING"),
    "trace did not enter CAPTURING",
  );
  let traceCompleted = false;
  for (let additionalScan = 0; additionalScan < 40; additionalScan += 1) {
    const traceText = (await traceRow.innerText()).toLocaleUpperCase("en-US");
    if (traceText.includes("COMPLETE") && traceText.includes("1 CAPTURES")) {
      traceCompleted = true;
      break;
    }
    await page.getByRole("button", { name: "Scan +1", exact: true }).click();
  }
  if (!traceCompleted) {
    const finalTraceText = (await traceRow.innerText()).toLocaleUpperCase("en-US");
    traceCompleted = finalTraceText.includes("COMPLETE") && finalTraceText.includes("1 CAPTURES");
  }
  if (!traceCompleted) {
    throw new Error(`trace did not complete within its bounded post-sample window: ${await traceRow.innerText()}`);
  }

  await page.getByRole("button", { name: "STOP", exact: true }).click();
  await waitForLocatorText(
    page.locator(".runtime-toolbar__identity strong"),
    (value) => value.trim() === "Stop",
    "virtual controller did not stop for snapshot capture",
  );
  await page.getByRole("button", { name: "Capture snapshot", exact: true }).click();
  await waitForEnabled(
    page.getByRole("button", { name: "Restore snapshot", exact: true }),
    true,
    "captured aggregate snapshot was not made restorable",
  );
  const snapshotScanSequence = await readScanSequence(page);

  await inputProbe.getByLabel("Value for Input", { exact: true }).selectOption("true");
  await inputProbe.getByRole("button", { name: "Set raw", exact: true }).click();
  await page.getByRole("button", { name: "RUN", exact: true }).click();
  await page.getByRole("button", { name: "Scan +1", exact: true }).click();
  await waitForRuntimeCell(inputProbe, 1, "TRUE", "mutated input before snapshot restore");
  await waitForRuntimeCell(outputProbe, 1, "FALSE", "mutated output before snapshot restore");
  const mutatedScanSequence = await readScanSequence(page);
  if (mutatedScanSequence === snapshotScanSequence) {
    throw new Error("snapshot mutation did not advance the controller scan sequence");
  }
  await page.getByRole("button", { name: "STOP", exact: true }).click();
  await page.getByRole("button", { name: "Restore snapshot", exact: true }).click();
  await waitForRuntimeCell(inputProbe, 1, "FALSE", "restored input value");
  await waitForRuntimeCell(outputProbe, 1, "TRUE", "restored output value");
  const restoredScanSequence = await readScanSequence(page);
  await waitForLocatorText(
    page.locator(".runtime-toolbar__identity strong"),
    (value) => value.trim() === "Stop",
    "snapshot restore did not recover the captured STOP state",
  );
  await page.getByRole("button", { name: "Verify replay", exact: true }).click();
  await page.getByLabel("Replay verified", { exact: true }).waitFor();
  const replayReceipt = page.getByLabel("Replay verification receipt", { exact: true });
  await replayReceipt.waitFor();
  const replayReceiptText = (await replayReceipt.innerText()).trim();
  if (!/^Deterministic replay verified\s+\d+ events · 1 boundary\s+[A-Fa-f0-9]{10}…[A-Fa-f0-9]{6}$/u.test(replayReceiptText)) {
    throw new Error(`closed replay receipt was incomplete: ${JSON.stringify(replayReceiptText)}`);
  }

  const runtimeScreenshot = path.join(evidenceDirectory, "workbench-runtime-commissioning.png");
  await page.screenshot({ fullPage: true, path: runtimeScreenshot });

  await treeItem(page, "Function").click();
  const faultingSource = "Result := TRUE;\nWHILE TRUE DO\n  CONTINUE;\nEND_WHILE;";
  await page.getByLabel("SCL source").fill(faultingSource);
  await page.getByRole("button", { name: "Apply SCL source" }).click();
  await page.getByText("Build stale", { exact: true }).waitFor();
  await page.getByRole("button", { name: "Build", exact: true }).click();
  await waitForBuildCurrent(page);
  await page.getByRole("button", { name: "Preview load", exact: true }).click();
  const watchdogLoadPreview = page.getByRole("region", { name: "Virtual Download preview" });
  await watchdogLoadPreview.waitFor();
  const watchdogCandidateFingerprint = (await watchdogLoadPreview.locator("small").innerText()).trim();
  if (watchdogCandidateFingerprint === candidateFingerprint) {
    throw new Error("watchdog SCL edit did not change the canonical load-candidate fingerprint");
  }
  await page.getByRole("button", { name: "Commit load", exact: true }).click();
  try {
    await watchdogLoadPreview.waitFor({ state: "detached", timeout: 5_000 });
  } catch (error) {
    const alert = page.getByRole("alert");
    const detail = await alert.count() > 0 ? await alert.innerText() : "no alert was rendered";
    throw new Error(`rebuilt watchdog load did not commit: ${detail}; ${error.message}`);
  }
  const watchdogGoOnline = page.getByRole("button", { name: "Go online", exact: true });
  if (await watchdogGoOnline.isEnabled()) {
    await watchdogGoOnline.click();
    await page.getByText("Online session active", { exact: true }).waitFor();
  }
  await waitForSoftwareMatch(page);
  await page.getByRole("button", { name: "RUN", exact: true }).click();
  await page.getByRole("button", { name: "Scan +1", exact: true }).click();
  try {
    const watchdogState = page.locator(".runtime-toolbar__identity strong");
    await waitForCondition(
      async () => (await watchdogState.innerText()).trim() === "Faulted",
      "watchdog fault did not drive the virtual controller to FAULTED",
      5_000,
    );
  } catch (error) {
    const actualState = (await page.locator(".runtime-toolbar__identity strong").innerText()).trim();
    const runtimeText = (await page.locator("body").innerText()).slice(0, 2_000);
    throw new Error(`${error.message}; actual state=${actualState}; runtime=${runtimeText}`);
  }
  const watchdogDiagnostic = page
    .locator(".runtime-diagnostic-row")
    .filter({ hasText: "EDU-RTM-0004" });
  await watchdogDiagnostic.waitFor();
  if (await watchdogDiagnostic.getAttribute("aria-disabled") !== "false") {
    throw new Error("the causal watchdog diagnostic did not expose semantic navigation");
  }
  await watchdogDiagnostic.click();
  await page.getByRole("heading", { level: 1, name: "Function" }).waitFor();
  if (await page.getByLabel("SCL source").inputValue() !== faultingSource) {
    throw new Error("runtime fault navigation did not return to the exact authored SCL block");
  }
  const diagnosticScreenshot = path.join(evidenceDirectory, "workbench-causal-diagnostic.png");
  await page.screenshot({ fullPage: true, path: diagnosticScreenshot });

  const receiptBeforeSave = (await page.locator(".runtime-toolbar__receipt").innerText()).trim();
  await page.locator('button[title="Save as"]').click();
  try {
    await page.getByRole("status", { name: "Saved", exact: true }).waitFor();
  } catch (error) {
    const alert = page.getByRole("alert");
    const detail = await alert.count() > 0 ? await alert.innerText() : "no alert was rendered";
    const broker = await page.evaluate(() => window.__phase2MemoryFileSnapshot());
    throw new Error(`Save As did not reach a verified clean state: ${detail}; broker=${JSON.stringify(broker)}; ${error.message}`);
  }
  await page.getByText("Online session active", { exact: true }).waitFor();
  const receiptAfterSave = (await page.locator(".runtime-toolbar__receipt").innerText()).trim();
  if (receiptAfterSave !== receiptBeforeSave) {
    throw new Error("Save As changed the loaded runtime epoch or scan sequence");
  }
  const memoryFile = await page.evaluate(() => window.__phase2MemoryFileSnapshot());
  if (memoryFile.byteLength <= 0 || memoryFile.savePickerCalls !== 1 || memoryFile.writeCount !== 1) {
    throw new Error(`memory-backed Save As was not durably verified: ${JSON.stringify(memoryFile)}`);
  }

  await page.getByRole("button", { exact: true, name: "Close" }).click();
  await page.getByRole("heading", {
    level: 1,
    name: "Build logic. Test it safely. Understand every scan.",
  }).waitFor();
  if (await page.getByRole("heading", { level: 2, name: "Save changes before closing?" }).count() !== 0) {
    throw new Error("a verified clean save still produced the unsaved-close decision dialog");
  }
  await page.getByRole("button", { name: "Choose project file", exact: true }).click();
  await page.getByRole("heading", { level: 1, name: "Renamed training cell" }).waitFor();
  await page.getByRole("status", { name: "Saved", exact: true }).waitFor();
  for (const persistedObject of [
    "Controller",
    "Local rack",
    "VDI16",
    "VDO16",
    "VAI4",
    "VAO4",
    "Function",
    "FbdFunction",
    "MainCycle",
    "StateBlock",
    "GlobalData",
    "InstanceData",
    "ProcessData",
    "Input",
    "Output",
    "Watch table",
    "Trace",
  ]) {
    await treeItem(page, persistedObject).waitFor();
  }
  await waitForLocatorText(
    page.locator(".runtime-toolbar__identity strong"),
    (value) => value.trim() === "Powered off",
    "reopened project did not start with a fresh powered-off runtime",
  );
  await waitForEnabled(
    page.getByRole("button", { name: "Preview load", exact: true }),
    false,
    "reopened project incorrectly retained an in-memory build",
  );
  await waitForEnabled(
    page.getByRole("button", { name: "Go online", exact: true }),
    false,
    "reopened project incorrectly retained a loaded runtime",
  );
  const reopenedFile = await page.evaluate(() => window.__phase2MemoryFileSnapshot());
  if (reopenedFile.openPickerCalls !== 1) {
    throw new Error("the saved project was not reopened through the granted production file boundary");
  }
  const reopenScreenshot = path.join(evidenceDirectory, "workbench-runtime-reopened.png");
  await page.screenshot({ fullPage: true, path: reopenScreenshot });

  const workbenchScreenshot = path.join(evidenceDirectory, "workbench-real-kernel.png");
  await page.screenshot({ fullPage: true, path: workbenchScreenshot });

  if (remoteRequests.length > 0) {
    throw new Error(`remote request attempted: ${remoteRequests.join(", ")}`);
  }
  if (pageErrors.length > 0) {
    throw new Error(`page errors: ${pageErrors.join(" | ")}`);
  }
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth > document.documentElement.clientWidth,
  );
  if (overflow) {
    throw new Error("desktop workbench has horizontal document overflow");
  }

  await context.close();

  const mobileContext = await browser.newContext({
    javaScriptEnabled: true,
    locale: "en-US",
    viewport: { height: 823, width: 426 },
  });
  const mobilePage = await mobileContext.newPage();
  const mobilePageErrors = [];
  const mobileRemoteRequests = [];
  mobilePage.on("pageerror", (error) => mobilePageErrors.push(error.message));
  await installNetworkBoundary(mobileContext, artifactOrigin, mobileRemoteRequests);

  await mobilePage.goto(artifactUrl, { waitUntil: "load" });
  await mobilePage.getByLabel("Project name").fill("Mobile training cell");
  await mobilePage.getByRole("button", { name: /^Create/u }).click();
  await mobilePage.getByRole("heading", { level: 1, name: "Mobile training cell" }).waitFor();
  await mobilePage.getByRole("status", { name: "Unsaved changes", exact: true }).waitFor();
  await addObject(mobilePage, "Controller");
  await mobilePage.getByRole("heading", { level: 1, name: "Controller" }).waitFor();
  await addObject(mobilePage, "Named user structure");
  await mobilePage.getByRole("heading", { level: 1, name: "ProcessData" }).waitFor();
  await mobilePage.getByRole("button", { name: "Add array", exact: true }).click();
  const mobileTypeRows = mobilePage.locator(".member-editor__table tbody tr");
  await mobileTypeRows.nth(1).getByRole("textbox").fill("Samples");
  await mobileTypeRows.nth(1).getByRole("combobox").selectOption("UINT");
  await applyMemberChanges(mobilePage);
  await mobilePage.getByText("2 members", { exact: true }).waitFor();
  if (!(await mobilePage.getByRole("button", { name: "Close", exact: true }).isVisible())) {
    throw new Error("mobile workbench does not expose the Close project action");
  }
  const mobileOverflow = await mobilePage.evaluate(
    () => document.documentElement.scrollWidth > document.documentElement.clientWidth,
  );
  if (mobileOverflow) {
    throw new Error("mobile workbench has horizontal document overflow");
  }
  if (mobileRemoteRequests.length > 0) {
    throw new Error(`mobile remote request attempted: ${mobileRemoteRequests.join(", ")}`);
  }
  if (mobilePageErrors.length > 0) {
    throw new Error(`mobile page errors: ${mobilePageErrors.join(" | ")}`);
  }

  const mobileScreenshot = path.join(evidenceDirectory, "workbench-real-kernel-mobile.png");
  await mobilePage.screenshot({ fullPage: true, path: mobileScreenshot });
  await mobileContext.close();

  console.log(JSON.stringify({
    browserPath,
    artifactMode: stagingMode ? "staging" : "candidate-inline",
    commands: [
      "create-project",
      "create-network-controller-rack-digital-analog-io",
      "author-lad-ob-fbd-fc-scl-fc-fb-db-types-and-bound-tags",
      "bind-watch-and-trace-after-tags",
      "copy-with-new-identity",
      "delete",
      "undo",
      "redo",
      "rename",
      "build-power-preview-commit-online-run-scan",
      "causal-io-monitor-modify-force-trace-snapshot-diagnose-navigate",
      "export-and-verify-nonempty-closed-replay",
      "save-as-close-open-project-with-fresh-runtime",
    ],
    isolatedLoopbackArtifact: true,
    networkRequests: 0,
    snapshotSemantics: {
      capturedScanSequence: snapshotScanSequence,
      mutatedScanSequence,
      restoredScanSequence,
    },
    screenshotPaths: [
      ladderScreenshot,
      fbdScreenshot,
      typeEditorScreenshot,
      runtimeScreenshot,
      diagnosticScreenshot,
      reopenScreenshot,
      workbenchScreenshot,
      mobileScreenshot,
    ],
    viewports: ["1586x920", "426x823"],
    wasmCore: "plc-engineering-core@0.2.0",
  }));
} finally {
  await browser.close();
  server.close();
  await once(server, "close");
}

async function findBrowser() {
  for (const candidate of browserCandidates) {
    try {
      await access(candidate);
      return candidate;
    } catch {
      // Continue to the next admitted local browser.
    }
  }
  throw new Error("No admitted system Chromium browser was found.");
}

async function addObject(page, menuItemName) {
  await page.getByRole("button", { name: "Add engineering object" }).click();
  await page.getByRole("menuitem", { name: new RegExp(menuItemName, "u") }).click();
}

async function applyMemberChanges(page) {
  const semanticRevision = page.locator(".navigator-foot span").nth(1);
  const before = (await semanticRevision.innerText()).trim();
  const applyButton = page.getByRole("button", { name: "Apply member changes", exact: true });
  await applyButton.click();
  await waitForLocatorText(
    semanticRevision,
    (value) => value.trim() !== before,
    "member edit did not advance the canonical semantic revision",
  );
  await waitForEnabled(
    applyButton,
    false,
    "member editor remained dirty after the canonical command committed",
  );
}

function treeItem(page, text) {
  return page.getByRole("treeitem", { exact: true, name: text });
}

function runtimeProbe(page, displayName) {
  return page
    .locator(".runtime-probe-row")
    .filter({ has: page.getByText(displayName, { exact: true }) });
}

async function waitForRuntimeCell(row, cellIndex, expected, label) {
  const cell = row.locator('[role="cell"]').nth(cellIndex);
  await waitForLocatorText(
    cell,
    (value) => value.split(/\r?\n/u)[0]?.trim() === expected,
    `${label} did not become ${expected}`,
  );
}

async function waitForWatchValues(page, expected) {
  const rows = page.locator(".watch-row");
  await waitForCondition(async () => {
    if (await rows.count() !== expected.length) {
      return false;
    }
    const values = await rows.allTextContents();
    return values.every((value, index) => value.includes(expected[index]));
  }, `watch values did not publish ${expected.join(", ")}`);
}

async function readScanSequence(page) {
  const row = page.locator(".runtime-summary dl > div").filter({ hasText: "Scan sequence" });
  await row.waitFor();
  const text = await row.locator("dd").innerText();
  return text.trim();
}

async function waitForScanSequence(page, expected) {
  await waitForCondition(
    async () => await readScanSequence(page) === expected,
    `scan sequence did not become ${expected}`,
  );
}

async function waitForSoftwareMatch(page) {
  const software = page.locator(".runtime-summary dl > div").filter({ hasText: "Software" }).locator("dd");
  const alert = page.getByRole("alert");
  await waitForCondition(async () => {
    if ((await software.innerText()).trim() === "Match") {
      return true;
    }
    if (await alert.count() > 0 && await alert.isVisible()) {
      throw new Error(`runtime load failed in the production UI: ${await alert.innerText()}`);
    }
    return false;
  }, "loaded software did not match the rebuilt watchdog artifact");
}

async function waitForBuildCurrent(page) {
  const current = page.getByText("Build current", { exact: true });
  const alert = page.getByRole("alert");
  await waitForCondition(async () => {
    if (await current.count() > 0 && await current.isVisible()) {
      return true;
    }
    if (await alert.count() > 0 && await alert.isVisible()) {
      throw new Error(`runtime build failed in the production UI: ${await alert.innerText()}`);
    }
    return false;
  }, "runtime build did not become current");
}

async function waitForEnabled(locator, expected, message) {
  await waitForCondition(async () => await locator.isEnabled() === expected, message);
}

async function waitForLocatorText(locator, predicate, message) {
  await waitForCondition(async () => predicate(await locator.innerText()), message);
}

async function waitForCondition(condition, message, timeoutMilliseconds = 30_000) {
  const deadline = Date.now() + timeoutMilliseconds;
  let lastError = null;
  while (Date.now() < deadline) {
    try {
      if (await condition()) {
        return;
      }
    } catch (error) {
      lastError = error;
    }
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  throw new Error(`${message}${lastError instanceof Error ? `: ${lastError.message}` : ""}`);
}

async function installMemoryFileAccess(context) {
  await context.addInitScript(() => {
    const fileName = "renamed-training-cell.vlabproj";
    const attestationId = "fixed-local-v1:00000001:0000000000000001";
    const grantId = "p2-native-v1:0000000000000001";
    let committedBytes = null;
    let openPickerCalls = 0;
    let savePickerCalls = 0;
    let writeCount = 0;

    const attestation = Object.freeze({
      attestationId,
      fixedDrive: true,
      kind: "fixed-native-local-v1",
      nativeLocal: true,
      platform: "windows",
      providerBacked: false,
      redirected: false,
      removable: false,
      special: false,
    });
    const savedResult = () => Object.freeze({
      attestationId,
      displayName: fileName,
      grantId,
      protocolVersion: 1,
      verifiedBytes: committedBytes?.byteLength ?? 0,
    });
    const bridge = Object.freeze({
      attestation,
      contract: "govs.project-file-broker",
      open: async () => {
        openPickerCalls += 1;
        if (committedBytes === null) {
          throw Object.freeze({ code: "ACCESS_CANCELLED" });
        }
        return Object.freeze({
          attestationId,
          bytes: committedBytes.slice(),
          displayName: fileName,
          grantId,
          protocolVersion: 1,
        });
      },
      protocolVersion: 1,
      revoke: () => undefined,
      save: async (request) => {
        if (
          request?.protocolVersion !== 1 ||
          request.grantId !== grantId ||
          !(request.bytes instanceof Uint8Array)
        ) {
          throw Object.freeze({ code: "UNKNOWN_GRANT" });
        }
        committedBytes = request.bytes.slice();
        writeCount += 1;
        return savedResult();
      },
      saveAs: async (request) => {
        savePickerCalls += 1;
        if (
          request?.protocolVersion !== 1 ||
          typeof request.projectName !== "string" ||
          !/^[A-Za-z0-9 _().-]+\.vlabproj$/iu.test(request.projectName) ||
          !(request.bytes instanceof Uint8Array)
        ) {
          throw Object.freeze({ code: "ATTESTATION_FAILED" });
        }
        committedBytes = request.bytes.slice();
        writeCount += 1;
        return savedResult();
      },
    });

    Object.defineProperties(window, {
      __phase2MemoryFileSnapshot: {
        configurable: false,
        value: () => ({
          byteLength: committedBytes?.byteLength ?? 0,
          openPickerCalls,
          savePickerCalls,
          writeCount,
        }),
        writable: false,
      },
      govsProjectFileBrokerV1: {
        configurable: false,
        enumerable: false,
        value: bridge,
        writable: false,
      },
    });
  });
}

async function installNetworkBoundary(context, allowedOrigin, rejectedRequests) {
  await context.route(/^(?:ftp|https?|wss?):/iu, async (route) => {
    const requestUrl = new URL(route.request().url());
    if (requestUrl.origin === allowedOrigin) {
      await route.continue();
      return;
    }

    rejectedRequests.push(requestUrl.href);
    await route.abort("blockedbyclient");
  });
}
