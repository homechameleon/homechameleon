export const SHEER_NAME = "Sheer";

export const STYLE_IDS = ["sheer", "bubbles", "material", "flat", "starship"] as const;
export type StyleId = (typeof STYLE_IDS)[number];

export const STYLE_BLURB: Record<StyleId, string> = {
  sheer: "Translucent cards, a soft blur, and a bright rim.",
  bubbles: "Large corners, soft color, and pill controls.",
  material: "Tonal surfaces and a quiet shadow.",
  flat: "Solid surfaces with no shadow and no blur.",
  starship: "Dark panels, pill controls, and a colored cap.",
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
  sheer: { roundness: 26, blur: 28, transparency: 78, glow: 38, saturation: 0, temperature: 0 },
  bubbles: { roundness: 30, blur: 6, transparency: 22, glow: 18, saturation: 0, temperature: 0 },
  material: { roundness: 16, blur: 0, transparency: 0, glow: 0, saturation: 0, temperature: 0 },
  flat: { roundness: 8, blur: 0, transparency: 0, glow: 0, saturation: 0, temperature: 0 },
  starship: { roundness: 4, blur: 0, transparency: 0, glow: 12, saturation: 0, temperature: 0 },
};

export function blurApplies(style: StyleId): boolean {
  return style === "sheer" || style === "bubbles";
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
