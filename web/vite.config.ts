import { defineConfig } from "vite";

export default defineConfig({
  worker: { format: "es" },
  build: {
    target: "es2022",
    cssCodeSplit: false,
    sourcemap: false,
    assetsInlineLimit: 0,
  },
  test: {
    include: ["src/**/*.test.ts"],
  },
});
