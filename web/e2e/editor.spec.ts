import { expect, test } from "@playwright/test";

const palette = {
  schema: "homechameleon-palette/1",
  theme: "harbor",
  mode: "dark",
  colors: {
    accent: "#7aa2f7",
    background: "#1a1b26",
    foreground: "#c0caf5",
    lighter_background: "#24283b",
    dark_background: "#16161e",
    darker_background: "#101014",
    dark_foreground: "#a9b1d6",
    muted: "#565f89",
    red: "#f7768e",
    green: "#9ece6a",
    yellow: "#e0af68",
    blue: "#7aa2f7",
    magenta: "#bb9af7",
  },
};

test("palette, style, and a slider update the sample preview", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`/?palette=${encodeURIComponent(JSON.stringify(palette))}`);
  await page.locator("hc-app[data-ready]").waitFor();
  await expect(page.getByRole("heading", { name: "Home" })).toBeVisible();
  const before = Number(await page.locator("hc-app").getAttribute("data-gen-count"));
  await page.getByRole("button", { name: "Bubbles" }).click();
  await page.waitForFunction(
    (count) => Number(document.querySelector("hc-app")?.getAttribute("data-gen-count")) > count,
    before,
  );
  const slider = page.getByRole("slider", { name: "Roundness" });
  const count = Number(await page.locator("hc-app").getAttribute("data-gen-count"));
  await slider.fill("12");
  await page.waitForFunction(
    (seen) => Number(document.querySelector("hc-app")?.getAttribute("data-gen-count")) > seen,
    count,
  );
  const ms = Number(await page.locator("hc-app").getAttribute("data-gen-ms"));
  expect(ms).toBeGreaterThan(0);
  expect(ms).toBeLessThan(16);
  await page.getByRole("tab", { name: "YAML" }).click();
  const yaml = page.locator(".yaml");
  await expect(yaml).toContainText("primary-color");
  await expect(yaml).not.toContainText("lovelace-background");
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.getByRole("heading", { name: "Home" })).toBeVisible();
});
