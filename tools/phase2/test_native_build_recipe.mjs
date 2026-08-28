import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  NATIVE_NODE_VERSION,
  PINNED_RENDERER_TOOLCHAIN,
  RENDERER_BUILD_RECIPE,
  validatePinnedRendererToolchain,
  validateRendererArtifactInventory,
} from "./native_build_recipe.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const buildPath = path.join(root, "tools", "phase2", "build_windows_shell.mjs");

test("strict native packaging regenerates ignored renderer only after exact-input admission", async () => {
  const source = await readFile(buildPath, "utf8");
  const admission = source.indexOf("Native production inputs do not match the exact candidate commit.");
  const renderer = source.indexOf("dist/index.html is deliberately generated and ignored.");
  const copy = source.indexOf("copyFile(appSource");
  assert.ok(admission >= 0 && renderer > admission && copy > renderer);
  assert.match(source, /await rm\(path\.join\(root, "dist"\)/u);
  assert.match(source, /validateRendererArtifactInventory/u);
  assert.match(source, /verify_native_shell_api_allowlist\.mjs/u);
  assert.doesNotMatch(source, /\.\.\.vendorFiles,\s*appSource/u);
});

test("renderer build planner accepts only its sole self-contained generated artifact", () => {
  assert.deepEqual(RENDERER_BUILD_RECIPE.commands[2], [
    "resolved-pnpm-wrapper", "--offline", "--frozen-lockfile", "run", "wasm:all:embed",
  ]);
  assert.deepEqual(
    validateRendererArtifactInventory([{ path: "dist/index.html", bytes: 1, sha256: "A".repeat(64) }]),
    { path: "dist/index.html", bytes: 1, sha256: "A".repeat(64) },
  );
  assert.throws(
    () => validateRendererArtifactInventory([
      { path: "dist/index.html", bytes: 1, sha256: "A".repeat(64) },
      { path: "dist/leftover.js", bytes: 1, sha256: "B".repeat(64) },
    ]),
    /exactly one/u,
  );
});

test("renderer toolchain admission rejects each pinned binary or version mutation before build planning", () => {
  const admitted = {
    nodeVersion: NATIVE_NODE_VERSION,
    ...PINNED_RENDERER_TOOLCHAIN,
  };
  assert.deepEqual(validatePinnedRendererToolchain(admitted), admitted);
  for (const field of Object.keys(admitted)) {
    const mutated = { ...admitted, [field]: field.endsWith("Sha256") ? "0".repeat(64) : "mutated" };
    assert.throws(() => validatePinnedRendererToolchain(mutated), new RegExp(field, "u"));
  }
});
