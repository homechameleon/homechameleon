// Renders docs/styles/<style>.png from the editor preview.
// From web/: npm run build:wasm && node scripts/style-previews.mjs
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "@playwright/test";
import { createServer } from "vite";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const outDir = resolve(root, "docs/styles");
const styles = ["sheer", "bubbles", "material", "flat", "starship"];
const palette = {
  schema: "homechameleon-palette/1",
  theme: "harbor",
  mode: "dark",
  colors: {
    accent: "#ff8a3d",
    background: "#12141c",
    foreground: "#f4efe6",
    lighter_background: "#1d2230",
    dark_background: "#0c0e14",
    darker_background: "#07080c",
    dark_foreground: "#c8c2b8",
    muted: "#8d8794",
    red: "#e85d4c",
    green: "#7dce6a",
    yellow: "#f0b45a",
    blue: "#6aa7ff",
    magenta: "#c7a2ff",
  },
};

const server = await createServer({
  root: resolve(root, "web"),
  server: { host: "127.0.0.1", port: 4181, strictPort: true },
  logLevel: "error",
});
await server.listen();
const browser = await chromium.launch();
const page = await browser.newPage({
  viewport: { width: 1280, height: 800 },
  deviceScaleFactor: 2,
});
await mkdir(outDir, { recursive: true });

try {
  for (const style of styles) {
    const url = `http://127.0.0.1:4181/?board=1&style=${style}&palette=${encodeURIComponent(JSON.stringify(palette))}`;
    await page.goto(url);
    await page.locator("hc-app[data-ready]").waitFor();
    await page.locator("hc-preview").locator(".stage").waitFor();
    await page.waitForTimeout(150);
    const shot = await page.locator("hc-preview .stage").screenshot({ type: "png" });
    const path = resolve(outDir, `${style}.png`);
    await writeFile(path, shot);
    console.log(path);
  }
} finally {
  await browser.close();
  await server.close();
}
