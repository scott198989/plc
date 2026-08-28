import assert from "node:assert/strict";
import test from "node:test";

import { validateNativeShellImportText } from "./verify_native_shell_pe_imports.mjs";

test("forbidden network imports fail native shell PE inventory", () => {
  assert.equal(validateNativeShellImportText("KERNEL32.dll\n"), true);
  assert.throws(
    () => validateNativeShellImportText("KERNEL32.dll\nWS2_32.dll\n"),
    /forbidden PE imports/u,
  );
});
