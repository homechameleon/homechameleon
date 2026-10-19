import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

import { unzipSync, strFromU8 } from "fflate";
import { describe, expect, it } from "vitest";

import { SHEER_NAME } from "./names";
import { detectPalette, fnv32, freshSnapshot, slugify, stitchYaml } from "./model";
import { buildZip } from "./zip";

describe("editor helpers", () => {
  it("keeps the sheer display name in one place", () => {
    expect(SHEER_NAME).toBe("Sheer");
    const source = readFileSync(new URL("./names.ts", import.meta.url), "utf8");
    expect(source.match(/=\s*"Sheer"/g)).toHaveLength(1);
  });

  it("hashes empty bytes the same way as the core", () => {
    expect(fnv32(new Uint8Array())).toBe("811c9dc5");
  });

  it("detects palette files", () => {
    expect(detectPalette('accent = "#7aa2f7"\nbackground = "#1a1b26"\nforeground = "#c0caf5"\n')?.format).toBe(
      "omarchy-colors-toml",
    );
    expect(detectPalette("scheme: dusk\nbase00: 1a1b26\nbase05: c0caf5\n")?.format).toBe("base16");
    expect(detectPalette("#112233 #abcdef")?.format).toBe("hex-list");
    const doc = '{"schema":"homechameleon-palette/1","theme":"dusk","mode":"dark","colors":{"accent":"#7aa2f7"}}';
    expect(detectPalette(doc)?.format).toBe("homechameleon-palette");
    expect(detectPalette('{"bg":"#111111","primary":"#7aa2f7","text":"#ffffff"}')?.format).toBe("roles");
  });

  it("slugifies theme names", () => {
    expect(slugify("Home Chameleon")).toBe("home-chameleon");
    expect(slugify("")).toBe("theme");
  });

  it("omits the wallpaper folder when the background is off", () => {
    expect(freshSnapshot().backgroundEnabled).toBe(false);
    const bytes = buildZip({
      slug: "desk",
      yaml: "# HomeChameleon theme\n# style: sheer\n# background: off\nDesk:\n  modes:\n    dark:\n      primary-color: \"#ffffff\"\n",
    });
    const files = unzipSync(bytes);
    expect(Object.keys(files).some((name) => name.startsWith("www/"))).toBe(false);
    const theme = strFromU8(files["themes/desk.yaml"]);
    expect(theme).toContain("Desk:\n  modes:\n");
    expect(theme).toContain("primary-color");
    expect(strFromU8(files["README.md"])).toContain("Home Assistant");
    expect(strFromU8(files["README.md"])).not.toMatch(/\bHA\b/);
  });

  it("stitches one mode block", () => {
    const yaml = stitchYaml({
      dark: "# HomeChameleon theme\n# style: bubbles\n# background: off\nDesk:\n  modes:\n    dark:\n      primary-color: \"#fff\"\n",
    });
    expect(yaml).toContain("# style: bubbles");
    expect(yaml.startsWith("# HomeChameleon theme\n")).toBe(true);
    expect(yaml).toContain("Desk:\n  modes:\n    dark:");
    expect(yaml).not.toContain("    light:");
  });
});

describe("wording", () => {
  it("keeps forbidden words out of the editor source", () => {
    const needles = ["st" + "udio", "gall" + "ery", "pro" + "file", "gl" + "ass", "--home" + "chameleon"];
    const root = new URL(".", import.meta.url);
    const files = walk(root.pathname).filter((file) => /\.(ts|css|html)$/.test(file) && !file.includes(`${join("src", "generated")}`));
    const hits: string[] = [];
    for (const file of files) {
      if (file.endsWith("model.test.ts")) continue;
      const text = readFileSync(file, "utf8").toLowerCase();
      for (const needle of needles) {
        if (text.includes(needle)) hits.push(`${file} ${needle}`);
      }
    }
    expect(hits).toEqual([]);
  });
});

function walk(dir: string): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    if (entry === "generated" || entry === "node_modules") continue;
    const stat = statSync(path);
    if (stat.isDirectory()) out.push(...walk(path));
    else out.push(path);
  }
  return out;
}
