import { once } from "node:events";
import { access, mkdir, readFile } from "node:fs/promises";
import { createServer } from "node:http";
import path from "node:path";

import { chromium } from "playwright-core";

const projectRoot = path.resolve(import.meta.dirname, "../..");
const artifactPath = path.join(projectRoot, "dist", "index.html");
const evidenceDirectory = path.join(projectRoot, ".phase2-verification", "P2-02");
const browserCandidates = [
  "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
  "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
  "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
];

await access(artifactPath);
await mkdir(evidenceDirectory, { recursive: true });
const artifact = await readFile(artifactPath);
const server = createServer((request, response) => {
  const requestUrl = new URL(request.url ?? "/", "http://127.0.0.1");
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
  await page.getByRole("status", {
    name: "Canonical project state has no diagnostics.",
    exact: true,
  }).waitFor();

  await treeItem(page, "Controller").click();
  await addObject(page, "Tag table");
  await page.getByRole("heading", { level: 1, name: "PLC tags" }).waitFor();
  await addObject(page, "Input tag");
  await page.getByRole("heading", { level: 1, name: "Input" }).waitFor();

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

  const workbenchScreenshot = path.join(evidenceDirectory, "workbench-real-kernel.png");
  await page.screenshot({ fullPage: true, path: workbenchScreenshot });

  await page.getByRole("button", { exact: true, name: "Close" }).click();
  await page.getByRole("heading", { level: 2, name: "Save changes before closing?" }).waitFor();
  await page.getByRole("button", { name: "Discard" }).click();
  await page.getByRole("heading", {
    level: 1,
    name: "Build logic. Test it safely. Understand every scan.",
  }).waitFor();

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
    commands: [
      "create-project",
      "create-network-controller-rack-io-tag-table-tag",
      "copy-with-new-identity",
      "delete",
      "undo",
      "redo",
      "rename",
      "close-discard",
    ],
    isolatedLoopbackArtifact: true,
    networkRequests: 0,
    screenshotPaths: [workbenchScreenshot, mobileScreenshot],
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

function treeItem(page, text) {
  return page.getByRole("treeitem", { exact: true, name: text });
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
