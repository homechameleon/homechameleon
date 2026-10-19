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

test("the zip export stays, and Apply appears only in the panel", async ({ page }) => {
  const fontStatus: number[] = [];
  page.on("response", (response) => {
    if (response.url().includes("Antonio-Bold")) fontStatus.push(response.status());
  });
  await page.goto("/");
  await expect(page.getByRole("button", { name: "Export zip" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Apply", exact: true })).toHaveCount(0);

  await page.goto(`/?panel=1&palette=${encodeURIComponent(JSON.stringify(palette))}`);
  await page.locator("hc-app[data-ready]").waitFor();
  await expect(page.getByRole("button", { name: "Export zip" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Apply", exact: true })).toBeVisible();
  await expect(page.getByRole("checkbox", { name: "Use as the default theme" })).toBeChecked();
  await page.getByRole("button", { name: "Starship Console" }).click();
  await expect.poll(() => fontStatus, { timeout: 10_000 }).toContain(200);
});

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
