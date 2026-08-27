import { defineConfig } from "vite";

export default defineConfig({
  base: "./",
  build: {
    assetsInlineLimit: Number.MAX_SAFE_INTEGER,
    cssCodeSplit: false,
    emptyOutDir: true,
    modulePreload: false,
    outDir: "../../dist/foundation-staging",
    reportCompressedSize: false,
    rollupOptions: {
      output: {
        assetFileNames: "foundation[extname]",
        codeSplitting: false,
        entryFileNames: "foundation.js",
      },
    },
    sourcemap: false,
    target: "es2022",
  },
  worker: {
    format: "iife",
  },
});
