export const NATIVE_NODE_VERSION = "24.19.0";
export const PINNED_RENDERER_TOOLCHAIN = Object.freeze({
  nodeExecutableSha256: "3602F2BB1A10F2CBAB4C36886218A33C1AB3DB87290E73B033C46C77147D0237",
  pnpmEntrySha256: "FF3224D46B47FBB24A7E9FE15FEDEDEF7E00892D07D4E376B6762D4899906BFD",
  pnpmVersion: "11.19.0",
  pnpmWrapperSha256: "1B93F82A7506B0F644F5ADC64A77470351D589DCBE4291D621147B72827F1D96",
});
export const RENDERER_BUILD_RECIPE = Object.freeze({
  schemaVersion: "1.0",
  toolchain: {
    cargo: "cargo 1.94.0 (85eff7c80 2026-01-15)",
    node: NATIVE_NODE_VERSION,
    pnpm: "11.19.0",
    rustc: "rustc 1.94.0 (4a4ef493e 2026-03-02)",
  },
  commands: [
    ["node-process-execPath", "tools/foundation/assert-toolchain.mjs"],
    ["resolved-pnpm-wrapper", "store", "status"],
    ["resolved-pnpm-wrapper", "--offline", "--frozen-lockfile", "run", "wasm:all:embed"],
    ["resolved-pnpm-wrapper", "--offline", "--frozen-lockfile", "--filter", "@govs/foundation-shell", "build"],
    ["node-process-execPath", "tools/foundation/inline-shell.mjs"],
  ],
});

export const validateRendererArtifactInventory = (rows) => {
  if (!Array.isArray(rows) || rows.length !== 1 || rows[0]?.path !== "dist/index.html" ||
      !Number.isSafeInteger(rows[0]?.bytes) || rows[0].bytes < 1 ||
      !/^[A-F0-9]{64}$/u.test(rows[0]?.sha256 ?? "")) {
    throw new Error("The renderer recipe must produce exactly one self-contained dist/index.html artifact.");
  }
  return rows[0];
};

export const validatePinnedRendererToolchain = (observed) => {
  const expected = {
    nodeVersion: NATIVE_NODE_VERSION,
    ...PINNED_RENDERER_TOOLCHAIN,
  };
  for (const [field, value] of Object.entries(expected)) {
    if (observed?.[field] !== value) {
      throw new Error(`Pinned renderer toolchain mismatch for ${field}.`);
    }
  }
  return Object.freeze({ ...expected });
};
