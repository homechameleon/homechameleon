import {
  type ExtractionId,
  type ModeId,
  type PaletteFormat,
  type PreviewSide,
  type RoleKey,
  ROLE_VARS,
  type StyleId,
  STYLE_DEFAULTS,
  type Tweaks,
} from "./names";

export interface EditorSnapshot {
  name: string;
  style: StyleId;
  mode: ModeId;
  previewSide: PreviewSide;
  extraction: ExtractionId;
  tweaks: Tweaks;
  locks: Partial<Record<RoleKey, string>>;
  backgroundEnabled: boolean;
  bgBlur: number;
  bgDim: number;
  bubble: boolean;
  mushroom: boolean;
  cardMod: boolean;
  importedFormat: PaletteFormat | null;
  importedData: string | null;
}

export function freshSnapshot(): EditorSnapshot {
  return {
    name: "HomeChameleon",
    style: "sheer",
    mode: "both",
    previewSide: "dark",
    extraction: "normal",
    tweaks: { ...STYLE_DEFAULTS.sheer },
    locks: {},
    backgroundEnabled: false,
    bgBlur: 0,
    bgDim: 0,
    bubble: true,
    mushroom: true,
    cardMod: false,
    importedFormat: null,
    importedData: null,
  };
}

export interface ImagePayload {
  id: string;
  name: string;
  weight: number;
  useAsBackground: boolean;
  width: number;
  height: number;
  rgba: string;
}

export function fnv32(bytes: Uint8Array): string {
  let hash = 0x811c9dc5;
  for (let i = 0; i < bytes.length; i += 1) {
    hash ^= bytes[i];
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}

export function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  const step = 0x8000;
  for (let i = 0; i < bytes.length; i += step) {
    binary += String.fromCharCode(...bytes.subarray(i, i + step));
  }
  return btoa(binary);
}

export function slugify(name: string): string {
  let out = "";
  let dash = false;
  for (const char of name) {
    if (/[a-zA-Z0-9]/.test(char)) {
      out += char.toLowerCase();
      dash = false;
    } else if ((char === "-" || char === "_" || /\s/.test(char)) && out && !dash) {
      out += char === "_" ? "_" : "-";
      dash = true;
    }
  }
  out = out.replace(/[-_]+$/, "");
  return out || "theme";
}

export function buildRequest(
  snap: EditorSnapshot,
  images: ImagePayload[],
  fileName: string | null,
  fastPreview: boolean,
  paletteOverride?: { format: PaletteFormat; data: string } | null,
): Record<string, unknown> {
  const mode = fastPreview ? snap.previewSide : snap.mode;
  const request: Record<string, unknown> = {
    name: snap.name.trim() || "HomeChameleon",
    style: snap.style,
    mode,
    extraction: snap.extraction,
    background: {
      enabled: snap.backgroundEnabled,
      blur: snap.bgBlur,
      dim: snap.bgDim,
      max_edge: 2560,
      file_name: snap.backgroundEnabled ? fileName : null,
    },
    options: {
      card_mod: snap.cardMod,
      bubble: snap.bubble,
      mushroom: snap.mushroom,
      contrast: "AA",
    },
    tweaks: {
      roundness: snap.tweaks.roundness,
      blur: snap.tweaks.blur,
      transparency: snap.tweaks.transparency,
      glow: snap.tweaks.glow,
      saturation: snap.tweaks.saturation,
      temperature: snap.tweaks.temperature,
    },
  };
  const palette =
    paletteOverride ??
    (snap.importedFormat && snap.importedData
      ? { format: snap.importedFormat, data: snap.importedData }
      : null);
  if (palette) {
    request.palette = palette;
    return request;
  }
  request.images = images.map((image) => ({
    width: image.width,
    height: image.height,
    rgba: image.rgba,
    weight: image.weight,
    use_as_background: image.useAsBackground,
  }));
  return request;
}

export function detectPalette(text: string): { format: PaletteFormat; data: string } | null {
  const trimmed = text.trim();
  if (!trimmed) return null;
  if (trimmed.startsWith("{")) {
    try {
      const parsed = JSON.parse(trimmed) as { schema?: string; colors?: unknown };
      if (parsed.schema === "homechameleon-palette/1") {
        return { format: "homechameleon-palette", data: trimmed };
      }
      if (parsed && typeof parsed === "object" && !parsed.colors) {
        return { format: "roles", data: trimmed };
      }
    } catch {
      return null;
    }
  }
  if (/^base00\s*:/im.test(trimmed) || /^scheme\s*:/im.test(trimmed)) {
    return { format: "base16", data: trimmed };
  }
  if (/^[a-z0-9_]+\s*=\s*["']?#?[0-9a-fA-F]{3,8}/m.test(trimmed)) {
    return { format: "omarchy-colors-toml", data: trimmed };
  }
  if (/#?[0-9a-fA-F]{6}/.test(trimmed)) {
    return { format: "hex-list", data: trimmed };
  }
  return null;
}

export function decodePaletteParam(raw: string): string {
  const text = raw.trim();
  if (text.startsWith("{") || text.includes("accent")) return text;
  try {
    const normalized = text.replace(/-/g, "+").replace(/_/g, "/");
    const decoded = atob(normalized);
    if (decoded.trim().startsWith("{") || decoded.includes("accent")) return decoded;
  } catch {
    /* not base64 */
  }
  return text;
}

export function cssForSide(css: string, side: PreviewSide): string {
  const marker = `/* ${side} */`;
  const start = css.indexOf(marker);
  if (start < 0) return css;
  const body = css.slice(start + marker.length);
  const next = body.search(/\n\/\* (?:dark|light) \*\//);
  return (next < 0 ? body : body.slice(0, next)).trim();
}

export function varCount(css: string): number {
  return (css.match(/--[a-z0-9-]+:/gi) ?? []).length;
}

export function readVar(css: string, name: string): string | null {
  const match = css.match(new RegExp(`--${name}\\s*:\\s*([^;]+)`));
  return match ? match[1].trim() : null;
}

function themeKey(yaml: string): string {
  for (const line of yaml.split("\n")) {
    if (!line || line.startsWith("#") || line.startsWith(" ")) continue;
    if (line.endsWith(":")) return line.slice(0, -1);
  }
  return "HomeChameleon";
}

function modeBlock(yaml: string, side: PreviewSide): string | null {
  const lines = yaml.split("\n");
  const start = lines.findIndex((line) => line === `    ${side}:`);
  if (start < 0) return null;
  const block = [lines[start]];
  for (const line of lines.slice(start + 1)) {
    if (/^    (?:dark|light):$/.test(line)) break;
    block.push(line);
  }
  return block.join("\n").trimEnd();
}

export function stitchYaml(parts: Partial<Record<PreviewSide, string>>): string {
  const blocks: string[] = [];
  for (const side of ["light", "dark"] as const) {
    const yaml = parts[side];
    if (!yaml) continue;
    const block = modeBlock(yaml, side);
    if (block) blocks.push(block);
  }
  const sample = parts.light ?? parts.dark ?? "";
  const style = sample.match(/# style: (.+)/)?.[1] ?? "sheer";
  const background = sample.match(/# background: (.+)/)?.[1] ?? "off";
  const name = themeKey(sample);
  return `# HomeChameleon theme\n# style: ${style}\n# background: ${background}\n${name}:\n  modes:\n${blocks.join("\n")}\n`;
}

export function stitchCss(parts: Partial<Record<PreviewSide, string>>): string {
  const blocks: string[] = [];
  for (const side of ["light", "dark"] as const) {
    const css = parts[side];
    if (!css) continue;
    const body = cssForSide(css, side);
    blocks.push(`/* ${side} */\n${body}`);
  }
  return `${blocks.join("\n\n")}\n`;
}

const LOCK_GLOBALS = "g";

export function applyLocks(css: string, locks: Partial<Record<RoleKey, string>>): string {
  let next = css;
  for (const [role, hex] of Object.entries(locks) as [RoleKey, string][]) {
    if (!hex) continue;
    for (const name of ROLE_VARS[role]) {
      const pattern = new RegExp(`(--${name}\\s*:\\s*)([^;]+)`, LOCK_GLOBALS);
      next = next.replace(pattern, `$1${hex}`);
    }
  }
  return next;
}

export function applyLocksToYaml(yaml: string, locks: Partial<Record<RoleKey, string>>): string {
  let next = yaml;
  for (const [role, hex] of Object.entries(locks) as [RoleKey, string][]) {
    if (!hex) continue;
    for (const name of ROLE_VARS[role]) {
      const pattern = new RegExp(`(${name}:\\s*")([^"]+)(")`, LOCK_GLOBALS);
      next = next.replace(pattern, `$1${hex}$3`);
    }
  }
  return next;
}
