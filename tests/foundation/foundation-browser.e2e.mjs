import { access, mkdir, readFile } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";

import { chromium } from "playwright-core";

const projectRoot = path.resolve(import.meta.dirname, "../..");
const artifactPath = path.join(projectRoot, "dist", "index.html");
const evidenceDirectory = path.join(
  projectRoot,
  ".phase1-verification",
  "foundation",
);
const browserCandidates = [
  "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
  "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
  "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
];

await access(artifactPath);
const browserPath = await findBrowser();
await mkdir(evidenceDirectory, { recursive: true });

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

const runViewport = async ({ name, viewport }) => {
  const context = await browser.newContext({
    deviceScaleFactor: name === "mobile" ? 2 : 1,
    javaScriptEnabled: true,
    locale: "en-US",
    offline: true,
    viewport,
  });
  const page = await context.newPage();
  const pageErrors = [];
  const remoteRequests = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("request", (request) => {
    if (/^(?:ftp|https?|wss?):/iu.test(request.url())) {
      remoteRequests.push(request.url());
    }
  });

  await page.goto(pathToFileURL(artifactPath).href, { waitUntil: "load" });
  await page.getByRole("heading", { level: 1 }).waitFor();
  const verifyButton = page.getByRole("button", {
    name: "Verify local foundation",
  });
  await verifyButton.focus();
  if (!(await verifyButton.evaluate((element) => element === document.activeElement))) {
    throw new Error(`${name}: primary action did not receive keyboard focus`);
  }
  await page.keyboard.press("Enter");
  await page.getByText("HEALTHY", { exact: true }).waitFor();

  const firstResult = await page.locator(".result-list").innerText();
  await page.getByRole("button", { name: "Verify local foundation" }).focus();
  await page.keyboard.press("Enter");
  await page.getByText("HEALTHY", { exact: true }).waitFor();
  const repeatedResult = await page.locator(".result-list").innerText();
  if (firstResult !== repeatedResult) {
    throw new Error(`${name}: repeated health check was not deterministic`);
  }
  if (remoteRequests.length > 0) {
    throw new Error(`${name}: remote request attempted: ${remoteRequests.join(", ")}`);
  }
  if (pageErrors.length > 0) {
    throw new Error(`${name}: page errors: ${pageErrors.join(" | ")}`);
  }
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth > document.documentElement.clientWidth,
  );
  if (overflow) {
    throw new Error(`${name}: horizontal overflow detected`);
  }

  const screenshotPath = path.join(
    evidenceDirectory,
    `foundation-${name}.png`,
  );
  await page.screenshot({ fullPage: true, path: screenshotPath });
  await context.close();
  return { name, result: firstResult.replaceAll("\n", " | "), screenshotPath, viewport };
};

try {
  const results = [];
  results.push(
    await runViewport({ name: "desktop", viewport: { height: 920, width: 1586 } }),
  );
  results.push(
    await runViewport({ name: "mobile", viewport: { height: 823, width: 426 } }),
  );
  console.log(JSON.stringify({ browserPath, fileUrl: true, networkRequests: 0, results }));
} finally {
  await browser.close();
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
