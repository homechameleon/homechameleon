import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "e2e",
  timeout: 60_000,
  use: {
    ...devices["Desktop Chrome"],
    baseURL: "http://127.0.0.1:4173",
  },
  webServer: {
    command: "npm run preview -- --port 4173 --strictPort",
    port: 4173,
    reuseExistingServer: false,
  },
});
