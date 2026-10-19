export interface RoleColors {
  bg: string;
  surface: string;
  primary: string;
  accent: string;
  text: string;
  success: string;
  warn: string;
  error: string;
  on_primary: string;
}

export interface ContrastCheck {
  mode: string;
  pair: string;
  ratio: number;
  minimum: number;
  pass: boolean;
}

export interface GenResult {
  error?: string;
  theme: string;
  slug: string;
  style: string;
  files: string[];
  yaml: string;
  css: string;
  light?: RoleColors;
  dark?: RoleColors;
  palette_json: string;
  contrast: { pass: boolean; checks: ContrastCheck[] };
  warnings: string[];
}

export function parseResult(raw: string): GenResult {
  return JSON.parse(raw) as GenResult;
}

export function rampColors(paletteJson: string): string[] {
  try {
    const parsed = JSON.parse(paletteJson) as { colors?: Record<string, string> };
    const colors = parsed.colors ?? {};
    const ramp: string[] = [];
    for (let i = 0; i < 16; i += 1) {
      const hex = colors[`color${i}`];
      if (hex) ramp.push(hex);
    }
    return ramp;
  } catch {
    return [];
  }
}
