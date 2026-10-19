import { strToU8, zipSync } from "fflate";

export function cardModSnippet(yaml: string): string | null {
  const marker = "card-mod-card: |";
  const at = yaml.indexOf(marker);
  if (at < 0 || !yaml.includes("card-mod-theme")) return null;
  const lines = yaml.slice(at + marker.length).split("\n");
  let indent = 0;
  for (const line of lines.slice(1)) {
    if (line.trim() === "") continue;
    indent = line.match(/^( +)/)?.[1].length ?? 0;
    break;
  }
  const body: string[] = [];
  for (const line of lines.slice(1)) {
    if (indent > 0 && line.startsWith(" ".repeat(indent))) body.push(line.slice(indent));
    else if (line.trim() === "") body.push("");
    else break;
  }
  const text = body.join("\n").trim();
  return text || null;
}

export function installReadme(slug: string, background: boolean, cardMod: boolean): string {
  const lines = [
    "# Install this HomeChameleon theme",
    "",
    "HomeChameleon is not affiliated with or endorsed by the Open Home Foundation or Home Assistant.",
    "",
    "1. Copy `themes/" + slug + ".yaml` into your Home Assistant `themes` folder.",
    "2. Add this to `configuration.yaml` if it is not there yet:",
    "",
    "```yaml",
    "frontend:",
    "  themes: !include_dir_merge_named themes",
    "```",
    "",
    "3. Restart Home Assistant, or call the `frontend.reload_themes` service, then pick the theme.",
    "",
  ];
  if (background) {
    lines.push(
      "The wallpaper is included under `www/homechameleon/" + slug + "/`.",
      "Copy that folder to `config/www/homechameleon/" + slug + "/`.",
      "Home Assistant serves everything under `/local/` without a login, so anyone who can reach the instance and guess the file name can download this wallpaper.",
      "",
    );
  } else {
    lines.push(
      "This zip has no wallpaper. The background stays off unless you turn it on in the editor and export again.",
      "",
    );
  }
  if (cardMod) {
    lines.push(
      "Optional card styling is in `card-mod/snippet.css` and is also embedded in the theme file.",
      "The theme still loads without that extra stylesheet.",
      "",
    );
  }
  lines.push("Photos you dropped in the editor were not uploaded anywhere. They stayed in the browser.");
  return lines.join("\n");
}

export interface ZipInput {
  slug: string;
  yaml: string;
  background?: { path: string; bytes: Uint8Array } | null;
  cardModCss?: string | null;
}

export function buildZip(input: ZipInput): Uint8Array {
  const files: Record<string, Uint8Array> = {
    [`themes/${input.slug}.yaml`]: strToU8(input.yaml),
    "README.md": strToU8(installReadme(input.slug, Boolean(input.background), Boolean(input.cardModCss))),
  };
  if (input.background) {
    files[`www/homechameleon/${input.background.path}`] = input.background.bytes;
  }
  if (input.cardModCss) {
    files["card-mod/snippet.css"] = strToU8(input.cardModCss);
  }
  return zipSync(files, { level: 6 });
}
