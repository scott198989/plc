import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  forbiddenExternalObserverImport,
  verifyExternalObserverSources,
} from "./verify_external_observer_source.mjs";

const directory = path.dirname(fileURLToPath(import.meta.url));
const sources = Object.fromEntries(await Promise.all([
  ["analyzer", "external_observer_evidence.mjs"],
  ["build", "build_external_observer.mjs"],
  ["finalizer", "finalize_external_observer_evidence.mjs"],
  ["source", "windows_external_observer.cpp"],
].map(async ([key, name]) => [key, await readFile(path.join(directory, name), "utf8")])));

test("external observer source exposes only the fixed ETW evidence path", () => {
  assert.equal(verifyExternalObserverSources(sources), true);
});

test("external observer source verifier rejects missing provider, stop, and zero-argument invariants", () => {
  for (const [field, token] of [
    ["source", "0xe53c6823, 0x7bb8, 0x44bb"],
    ["source", "EVENT_TRACE_CONTROL_STOP"],
    ["build", "process.argv.slice(2).length !== 0"],
    ["finalizer", "flag: \"wx\""],
    ["analyzer", "eventsLost === 0"],
    ["analyzer", "ParentProcessSequenceNumber"],
  ]) {
    const changed = { ...sources, [field]: sources[field].replaceAll(token, "") };
    assert.throws(() => verifyExternalObserverSources(changed), /invariant|provider/u);
  }
});

test("external observer source verifier rejects paged system-logger buffers and missing diagnostics", () => {
  assert.throws(() => verifyExternalObserverSources({
    ...sources,
    source: sources.source.replace("EVENT_TRACE_SYSTEM_LOGGER_MODE", "EVENT_TRACE_USE_PAGED_MEMORY"),
  }), /invariant|nonpaged/u);
  assert.throws(() => verifyExternalObserverSources({
    ...sources,
    source: sources.source.replace("could not start; win32=", "could not start"),
  }), /Win32 status/u);
});

test("external observer source requires opcode classification and a validated linked standard-user token", () => {
  for (const token of [
    "EVENT_TRACE_TYPE_STOP",
    "record->EventHeader.EventDescriptor.Opcode",
    "EVENT_TRACE_NO_PER_PROCESSOR_BUFFERING",
    "TokenLinkedToken",
    "TokenElevationTypeLimited",
    "GetShellWindow()",
    "CreateProcessWithTokenW(",
    "CreateProcessAsUserW(",
    "ERROR_ACCESS_DENIED",
    "AdjustTokenPrivileges(",
    "SE_ASSIGNPRIMARYTOKEN_NAME",
    "SE_INCREASE_QUOTA_NAME",
    "~TraceSession() noexcept",
    "failed after its trace was preserved",
  ]) {
    assert.throws(() => verifyExternalObserverSources({
      ...sources,
      source: sources.source.replaceAll(token, ""),
    }), /invariant/u);
  }
  assert.throws(() => verifyExternalObserverSources({
    ...sources,
    source: `${sources.source}\nCreateProcessW();`,
  }), /elevated process token/u);
});

test("external observer source verifier rejects network, shell, device, and industrial capabilities", () => {
  for (const injected of [
    "WinHttpOpen();",
    "ShellExecute();",
    "DeviceIoControl();",
    "socket();",
    "const char* x = \"https://example.invalid\";",
    "const char* x = \"modbus\";",
  ]) {
    assert.throws(() => verifyExternalObserverSources({ ...sources, source: `${sources.source}\n${injected}` }),
      /forbidden capability/u);
  }
  assert.throws(() => verifyExternalObserverSources({
    ...sources,
    build: sources.build.replace("advapi32.lib bcrypt.lib tdh.lib user32.lib",
      "advapi32.lib bcrypt.lib tdh.lib user32.lib ws2_32.lib"),
  }), /link-library/u);
});

test("PE import inspection rejects network and device DLLs", () => {
  assert.equal(forbiddenExternalObserverImport("KERNEL32.dll\nADVAPI32.dll\nTDH.dll\nUSER32.dll"), null);
  for (const name of ["WS2_32.dll", "WINHTTP.dll", "DNSAPI.dll", "SETUPAPI.dll", "WINUSB.dll"]) {
    assert.equal(forbiddenExternalObserverImport(`KERNEL32.dll\n${name}`), name);
  }
  assert.equal(forbiddenExternalObserverImport(""), "missing-import-inventory");
});
