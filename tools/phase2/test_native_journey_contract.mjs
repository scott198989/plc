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
    'createChild("Digital input module", "VDI16")',
    'selectTreeItem("Local rack")',
    'createChild("Digital output module", "VDO16")',
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
  assert.match(source, /tabWithText\("Runtime & commissioning"\)/u);
  assert.match(source, /document\.querySelector\("\.runtime-summary"\)/u);
  assert.match(source, /settled\(buildIsCurrent\)/u);
  assert.match(source, /cpuIs\("STOP"\).*buttonWithText\("Power off"\)/u);
  assert.match(source, /scanSequenceIs\(1\)/u);
  assert.match(source, /STOP state before capture/u);
  assert.match(source, /captured replay snapshot/u);
  assert.match(source, /getAttribute\("data-fingerprint"\)/u);
  assert.match(source, /getAttribute\("data-event-count"\)/u);
  assert.match(source, /getAttribute\("data-boundary-count"\)/u);
  assert.match(source, /getAttribute\("data-runtime-replay-hash"\)/u);
  assert.match(source, /govs-p2-native-verification-uuid-v1/u);
  const runtimeSteps = ["Build", "Power on", "Preview load", "Commit load", "Go online", "RUN", "Scan +1", "STOP", "Capture snapshot"];
  let runtimeOffset = source.indexOf('verificationResponses >= 2');
  for (const step of runtimeSteps) {
    runtimeOffset = source.indexOf(`buttonWithText("${step}")`, runtimeOffset + 1);
    assert.ok(runtimeOffset > 0, `runtime journey must use ${step} in order`);
  }
});

test("native launcher attributes WebView2 through exact job membership", async () => {
  const source = await readFile(path.join(root, "tools/phase2/native_e2e_launcher.cpp"), "utf8");
  assert.match(source, /QueryInformationJobObject\([\s\S]*?JobObjectBasicProcessIdList/u);
  assert.match(source, /CreateIoCompletionPort\(/u);
  assert.match(source, /JobObjectAssociateCompletionPortInformation/u);
  assert.match(source, /GetQueuedCompletionStatus\(/u);
  assert.match(source, /JOB_OBJECT_MSG_NEW_PROCESS/u);
  assert.match(source, /const auto admitted = job_processes\(process_job\);/u);
  assert.match(source, /admitted\.contains\(root_process\)/u);
  const identity = source.slice(
    source.indexOf("void capture_process_identity("),
    source.indexOf("void capture_job_notifications("),
  );
  assert.ok(identity.indexOf("const auto existing = observation.processes.find(process_id);") <
    identity.indexOf("sha256_file(image_path)"), "process digest lookup must precede hashing");
  assert.equal(identity.match(/sha256_handle\(observation\.runtime_authorities\.back\(\)\.get\(\)\)/gu)?.length, 1,
    "the held WebView2 runtime image must be hashed once per observation identity");
  assert.match(identity,
    /open_attested_path\([\s\S]*?image_path,[\s\S]*?HardlinkPolicy::allow_multiple\)/u,
    "the installed WebView2 runtime may use Microsoft-managed hardlinks while its no-write handle is retained");
  const capture = source.slice(source.indexOf("void capture_external_observation(\n    HANDLE process_job"));
  assert.doesNotMatch(capture.slice(0, capture.indexOf("\n}\n\n}  // namespace")), /descendant_processes\(root_process/u);
});
