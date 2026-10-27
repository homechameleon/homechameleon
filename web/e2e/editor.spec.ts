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
  await expect(page.getByRole("combobox", { name: "Apply to" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Undo theme" })).toHaveCount(0);

  await page.goto(`/?panel=1&palette=${encodeURIComponent(JSON.stringify(palette))}`);
  await page.locator("hc-app[data-ready]").waitFor();
  await expect(page.getByRole("button", { name: "Export zip" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Apply", exact: true })).toBeVisible();
  await expect(page.getByRole("checkbox", { name: "Use as the default theme" })).not.toBeChecked();
  await expect(page.getByRole("combobox", { name: "Apply to" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Undo theme" })).toBeDisabled();
  await page.getByRole("button", { name: "Starship Console" }).click();
  await expect.poll(() => fontStatus, { timeout: 10_000 }).toContain(200);
});

test("the add-on page applies through the add-on", async ({ page }) => {
  await page.goto(`/?addon=1&palette=${encodeURIComponent(JSON.stringify(palette))}`);
  await page.locator("hc-app[data-ready]").waitFor();
  const seen: { url: string; yaml: string; token?: string }[] = [];
  await page.route("**/api/apply", async (route) => {
    const body = route.request().postDataJSON() as { yaml?: string; token?: string };
    seen.push({ url: route.request().url(), yaml: body.yaml ?? "", token: body.token });
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ theme: "HomeChameleon", warnings: [] }),
    });
  });
  await page.getByRole("button", { name: "Apply", exact: true }).click();
  await expect(page.getByText("Applied HomeChameleon")).toBeVisible();
  expect(seen).toHaveLength(1);
  expect(new URL(seen[0].url).pathname).toBe("/api/apply");
  expect(seen[0].yaml).toContain("primary-color");
  expect(seen[0].token).toBeUndefined();
});

test("the sidebar panel still applies through the integration", async ({ page }) => {
  await page.goto(`/?panel=1&palette=${encodeURIComponent(JSON.stringify(palette))}`);
  await page.locator("hc-app[data-ready]").waitFor();
  let pathname = "";
  await page.route("**/api/homechameleon/apply", async (route) => {
    pathname = new URL(route.request().url()).pathname;
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ theme: "HomeChameleon", warnings: [] }),
    });
  });
  await page.getByRole("button", { name: "Apply", exact: true }).click();
  await expect(page.getByText("Applied HomeChameleon")).toBeVisible();
  expect(pathname).toBe("/api/homechameleon/apply");
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

test("the panel previews a room in light and dark before anything is applied", async ({ page }) => {
  const room = {
    area_id: "living_room",
    name: "Living room",
    view: { title: "Living room", path: "living-room", type: "sections", sections: [] },
    yaml: 'title: "Living room"\npath: "living-room"\ntype: "sections"\n',
    card_set: "core",
    enhancements: [],
    warnings: [],
    wrote: false,
    preview: {
      header: {
        title: "Living room",
        temperature: { entity_id: "sensor.temp", name: "Temperature", state: "21.5", unit: "°C", card: "tile" },
        humidity: { entity_id: "sensor.humidity", name: "Humidity", state: "44", unit: "%", card: "tile" },
        lights_toggle: true,
      },
      sections: [
        {
          id: "lights",
          title: "Lights",
          cards: [
            {
              entity_id: "light.ceiling",
              name: "Ceiling",
              domain: "light",
              state: "on",
              unit: null,
              interaction: "direct",
              card: "tile",
            },
          ],
        },
        {
          id: "security",
          title: "Security",
          cards: [
            {
              entity_id: "lock.front_door",
              name: "Front door",
              domain: "lock",
              state: "locked",
              unit: null,
              interaction: "confirm",
              card: "tile",
              detail: "Asks before changing the lock.",
            },
          ],
        },
        {
          id: "cameras",
          title: "Cameras",
          cards: [
            {
              entity_id: "camera.front",
              name: "Front camera",
              domain: "camera",
              state: "idle",
              unit: null,
              interaction: "view",
              card: "picture-entity",
              detail: "View only.",
            },
          ],
        },
      ],
    },
  };
  let posted = "";
  await page.route("**/api/homechameleon/areas", (route) =>
    route.fulfill({ json: { areas: [{ id: "living_room", name: "Living room", entities: 3 }] } }),
  );
  await page.route("**/api/homechameleon/room", (route) => {
    posted = route.request().postData() ?? "";
    return route.fulfill({ json: room });
  });

  await page.goto("/");
  await expect(page.getByRole("tab", { name: "Room", exact: true })).toHaveCount(0);

  await page.goto(`/?panel=1&palette=${encodeURIComponent(JSON.stringify(palette))}`);
  await page.locator("hc-app[data-ready]").waitFor();
  await page.getByRole("tab", { name: "Room", exact: true }).click();
  await page.getByLabel("Area").selectOption("Living room");
  await expect(page.getByRole("heading", { name: "Living room" }).first()).toBeVisible();
  await expect(page.locator("hc-room-preview .side-label").nth(0)).toHaveText("Dark");
  await expect(page.locator("hc-room-preview .side-label").nth(1)).toHaveText("Light");
  await expect(page.getByText("Confirm").first()).toBeVisible();
  await expect(page.getByText("View only").first()).toBeVisible();
  await expect(page.getByText("does not write the view").first()).toBeVisible();
  await expect(page.getByText("Core cards.")).toBeVisible();
  expect(posted).toContain("living_room");
  const sheets = page.locator("[data-room-sheet]");
  await expect(sheets).toHaveCount(2);
  const darkBg = await sheets.nth(0).evaluate((element) => getComputedStyle(element).backgroundColor);
  const lightBg = await sheets.nth(1).evaluate((element) => getComputedStyle(element).backgroundColor);
  expect(darkBg).not.toBe(lightBg);
  await expect(page.getByRole("heading", { name: "Home" })).toHaveCount(0);
});
