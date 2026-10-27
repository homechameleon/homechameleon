export const SHEER_NAME = "Sheer";

export const STYLE_IDS = ["sheer", "bubbles", "material", "flat", "starship"] as const;
export type StyleId = (typeof STYLE_IDS)[number];

export const STYLE_BLURB: Record<StyleId, string> = {
  sheer: "Frosted translucent panes, a saturated blur, and a bright rim.",
  bubbles: "Blob cards, puffy shadows, and bouncy pill controls.",
  material: "Tonal containers, elevation, and a 12 to 28 pixel shape scale.",
  flat: "No shadows, bold color blocks, and strong type.",
  starship: "Sweep elbows, thick color bars, and condensed labels.",
};

export function styleLabel(id: StyleId): string {
  if (id === "sheer") return SHEER_NAME;
  if (id === "bubbles") return "Bubbles";
  if (id === "material") return "Material";
  if (id === "flat") return "Flat";
  return "Starship Console";
}

export type ModeId = "dark" | "light" | "both";
export type PreviewSide = "dark" | "light";
export type ExtractionId = "normal" | "pastel" | "material" | "colorful" | "muted";
export type RoleKey = "bg" | "surface" | "primary" | "accent" | "text" | "success" | "warn" | "error";
export type DeviceId = "desktop" | "tablet" | "phone";
export type TabId = "editor" | "wallpaper" | "blueprints" | "room" | "yaml";
export type PaletteFormat =
  | "omarchy-colors-toml"
  | "base16"
  | "hex-list"
  | "homechameleon-palette"
  | "roles";

export interface Tweaks {
  roundness: number;
  blur: number;
  transparency: number;
  glow: number;
  saturation: number;
  temperature: number;
}

export const STYLE_DEFAULTS: Record<StyleId, Tweaks> = {
  sheer: { roundness: 28, blur: 28, transparency: 70, glow: 46, saturation: 0, temperature: 0 },
  bubbles: { roundness: 36, blur: 0, transparency: 0, glow: 34, saturation: 0, temperature: 0 },
  material: { roundness: 16, blur: 0, transparency: 0, glow: 20, saturation: 0, temperature: 0 },
  flat: { roundness: 8, blur: 0, transparency: 0, glow: 0, saturation: 0, temperature: 0 },
  starship: { roundness: 26, blur: 0, transparency: 0, glow: 40, saturation: 0, temperature: 0 },
};

export function blurApplies(style: StyleId): boolean {
  return style === "sheer";
}

export const ROLE_KEYS: RoleKey[] = [
  "bg",
  "surface",
  "primary",
  "accent",
  "text",
  "success",
  "warn",
  "error",
];

export const ROLE_LABEL: Record<RoleKey, string> = {
  bg: "Bg",
  surface: "Surface",
  primary: "Primary",
  accent: "Accent",
  text: "Text",
  success: "Success",
  warn: "Warn",
  error: "Error",
};

export const ROLE_VARS: Record<RoleKey, string[]> = {
  bg: ["primary-background-color"],
  surface: ["card-background-color", "ha-card-background"],
  primary: ["primary-color"],
  accent: ["accent-color"],
  text: ["primary-text-color"],
  success: ["success-color"],
  warn: ["warning-color"],
  error: ["error-color"],
};
