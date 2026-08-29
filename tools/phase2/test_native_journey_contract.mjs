import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");

test("native verification journey authors minimum runnable hardware before persistence", async () => {
  const source = await readFile(path.join(root, "apps/windows-shell/src/bridge_protocol.cpp"), "utf8");
  const steps = [
    'createChild("Virtual network", "Virtual network")',
    'selectTreeItem("Phase 2 Native Verification")',
    'createChild("Controller", "Controller")',
    'createChild("Rack", "Local rack")',
    'selectTreeItem("Local rack")',
    'createChild("VDI16", "VDI16")',
    'selectTreeItem("Local rack")',
    'createChild("VDO16", "VDO16")',
    'selectTreeItem("Controller")',
    'createChild("Organization block", "Main_cycle")',
    'button.getAttribute("title") === "Save as"',
  ];
  let offset = -1;
  for (const step of steps) {
    const next = source.indexOf(step, offset + 1);
    assert.ok(next > offset, `journey must contain ${step} in order`);
    offset = next;
  }
  assert.match(source, /diagnostics settled/u);
  assert.match(source, /verification UI alert during/u);
  assert.match(source, /input\.labels/u);
  assert.match(source, /button\.childNodes/u);
  assert.match(source, /button\.getAttribute\("aria-label"\)/u);
  assert.match(source, /settled\(buildIsCurrent\)/u);
  assert.match(source, /cpuIs\("STOP"\).*buttonWithText\("Power off"\)/u);
  assert.match(source, /scanSequenceIs\(1\)/u);
  assert.match(source, /STOP state before capture/u);
  assert.match(source, /captured replay snapshot/u);
  assert.match(source, /govs-p2-native-verification-uuid-v1/u);
  const runtimeSteps = ["Build", "Power on", "Preview load", "Commit load", "Go online", "RUN", "Scan +1", "STOP", "Capture snapshot"];
  let runtimeOffset = source.indexOf('verificationResponses >= 2');
  for (const step of runtimeSteps) {
    runtimeOffset = source.indexOf(`buttonWithText("${step}")`, runtimeOffset + 1);
    assert.ok(runtimeOffset > 0, `runtime journey must use ${step} in order`);
  }
});
