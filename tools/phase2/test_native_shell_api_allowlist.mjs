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

test("native replacement closes the verified temp and uses supported ReplaceFileW flags", async () => {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
  const source = await readFile(
    path.join(root, "crates", "windows-project-broker", "src", "windows.rs"),
    "utf8",
  );
  const overwriteStart = source.indexOf("fn overwrite_attested(");
  const overwriteEnd = source.indexOf("\n    }\n}\n\nfn authoritative_local_app_data", overwriteStart);
  assert.notEqual(overwriteStart, -1);
  assert.notEqual(overwriteEnd, -1);
  const overwrite = source.slice(overwriteStart, overwriteEnd);
  const closeIndex = overwrite.indexOf("drop(temp_handle);");
  const replaceIndex = overwrite.indexOf("ReplaceFileW(");
  assert.ok(closeIndex >= 0 && closeIndex < replaceIndex);
  assert.match(overwrite, /ReplaceFileW\([\s\S]*?backup_wide\.as_ptr\(\),\s*0,/u);
  assert.match(overwrite, /file\.token == temp_token/u);
  assert.doesNotMatch(source, /REPLACEFILE_WRITE_THROUGH/u);
});
