import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { verifyNativeShellApiAllowlist } from "./verify_native_shell_api_allowlist.mjs";

test("native shell direct file API inventory is closed and explicit", async () => {
  const inventory = await verifyNativeShellApiAllowlist();
  assert.ok(inventory.length > 0);
});

test("a new direct call to an otherwise approved API fails the exact inventory", async () => {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
  const file = "apps/windows-shell/src/broker_client.cpp";
  const original = await readFile(path.join(root, ...file.split("/")), "utf8");
  await assert.rejects(
    verifyNativeShellApiAllowlist(root, new Map([[file, `${original}\nCreateFileW(nullptr, 0, 0, nullptr, 0, 0, nullptr);`]])),
    /approved CreateFileW inventory/u,
  );
});
