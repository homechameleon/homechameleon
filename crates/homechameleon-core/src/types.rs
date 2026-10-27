//! Request and result types. Field names follow the frozen generate request
//! so a later Home Assistant endpoint can pass them through.

pub const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_IMAGES: usize = 8;

/// Launch style ids. Display names are separate from the ids.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StyleId {
    #[default]
    Sheer,
    Bubbles,
    Material,
    Flat,
    Starship,
}

impl StyleId {
    pub fn parse(id: &str) -> Result<Self, GenerateError> {
        match id {
            "sheer" => Ok(Self::Sheer),
            "bubbles" => Ok(Self::Bubbles),
            "material" => Ok(Self::Material),
            "flat" => Ok(Self::Flat),
            "starship" => Ok(Self::Starship),
            other => Err(GenerateError::UnknownStyle {
                style: other.to_string(),
            }),
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Sheer => "sheer",
            Self::Bubbles => "bubbles",
            Self::Material => "material",
            Self::Flat => "flat",
            Self::Starship => "starship",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Sheer => "Sheer",
            Self::Bubbles => "Bubbles",
            Self::Material => "Material",
            Self::Flat => "Flat",
            Self::Starship => "Starship Console",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Self::Sheer => "Translucent cards, a soft blur, and a bright rim.",
            Self::Bubbles => "Large corners, soft color, and pill controls.",
            Self::Material => "Tonal surfaces from a Material Design color scheme.",
            Self::Flat => "Solid surfaces with no shadow and no blur.",
            Self::Starship => "Dark panels, pill controls, and a colored cap on each card.",
        }
    }

    /// Slider defaults for this style. Values are absolute, not offsets.
    pub fn default_tweaks(self) -> Tweaks {
        match self {
            Self::Sheer => Tweaks {
                roundness: 26.0,
                blur: 28.0,
                transparency: 78.0,
                glow: 38.0,
                saturation: 0.0,
                temperature: 0.0,
            },
            Self::Bubbles => Tweaks {
                roundness: 30.0,
                blur: 6.0,
                transparency: 22.0,
                glow: 18.0,
                saturation: 0.0,
                temperature: 0.0,
            },
            Self::Material => Tweaks {
                roundness: 16.0,
                blur: 0.0,
                transparency: 0.0,
                glow: 0.0,
                saturation: 0.0,
                temperature: 0.0,
            },
            Self::Flat => Tweaks {
                roundness: 8.0,
                blur: 0.0,
                transparency: 0.0,
                glow: 0.0,
                saturation: 0.0,
                temperature: 0.0,
            },
            Self::Starship => Tweaks {
                roundness: 4.0,
                blur: 0.0,
                transparency: 0.0,
                glow: 12.0,
                saturation: 0.0,
                temperature: 0.0,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    Dark,
    Light,
    #[default]
    Both,
}

impl Mode {
    pub fn includes(self, mode: ColorMode) -> bool {
        match self {
            Self::Both => true,
            Self::Dark => mode == ColorMode::Dark,
            Self::Light => mode == ColorMode::Light,
        }
    }
}

/// One scheme side. `Mode::Both` means one theme file with both of these.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorMode {
    Dark,
    Light,
}

impl ColorMode {
    pub fn id(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    pub fn is_dark(self) -> bool {
        matches!(self, Self::Dark)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ExtractionMode {
    #[default]
    Normal,
    Pastel,
    Material,
    Colorful,
    Muted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ContrastTarget {
    #[default]
    Aa,
    Aaa,
}

impl ContrastTarget {
    pub fn text_ratio(self) -> f64 {
        match self {
            Self::Aa => 4.5,
            Self::Aaa => 7.0,
        }
    }

    pub fn ui_ratio(self) -> f64 {
        match self {
            Self::Aa => 3.0,
            Self::Aaa => 4.5,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Aa => "AA",
            Self::Aaa => "AAA",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PaletteFormat {
    OmarchyColorsToml,
    Base16,
    Roles,
    HexList,
    /// `homechameleon-palette/1` JSON, including files written by `homechameleon-sync --export`.
    HomeChameleonPalette,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PaletteInput {
    pub format: PaletteFormat,
    pub data: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImageInput {
    pub width: u32,
    pub height: u32,
    /// Raw RGBA, length `width * height * 4`. JSON carries this as base64.
    pub rgba: Vec<u8>,
    pub weight: f64,
    pub use_as_background: bool,
}

impl ImageInput {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        Self {
            width,
            height,
            rgba,
            weight: 1.0,
            use_as_background: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BackgroundOptions {
    /// Wallpaper is opt-in. Off leaves images as extraction input only.
    pub enabled: bool,
    pub blur: u32,
    pub dim: f64,
    pub max_edge: u32,
    /// File name the host will write. Used only when `enabled` is true.
    pub file_name: Option<String>,
}

impl Default for BackgroundOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            blur: 0,
            dim: 0.0,
            max_edge: 2560,
            file_name: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThemeOptions {
    pub card_mod: bool,
    pub bubble: bool,
    pub mushroom: bool,
    pub contrast: ContrastTarget,
}

impl Default for ThemeOptions {
    fn default() -> Self {
        Self {
            card_mod: false,
            bubble: true,
            mushroom: true,
            contrast: ContrastTarget::Aa,
        }
    }
}

/// Editor sliders. Omitted tweaks use the style's own defaults.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tweaks {
    /// Card corner radius, 0–40 px.
    pub roundness: f64,
    /// Backdrop blur, 0–40 px. Material and Starship Console ignore this.
    pub blur: f64,
    /// How see-through a card is, 0–100. 0 is opaque.
    pub transparency: f64,
    /// Highlight and shadow strength, 0–100.
    pub glow: f64,
    /// Chroma shift, −100–100.
    pub saturation: f64,
    /// Warm (positive) or cool, −50–50.
    pub temperature: f64,
}

impl Tweaks {
    pub fn clamped(self) -> Self {
        Self {
            roundness: self.roundness.clamp(0.0, 40.0),
            blur: self.blur.clamp(0.0, 40.0),
            transparency: self.transparency.clamp(0.0, 100.0),
            glow: self.glow.clamp(0.0, 100.0),
            saturation: self.saturation.clamp(-100.0, 100.0),
            temperature: self.temperature.clamp(-50.0, 50.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenerateRequest {
    pub name: String,
    pub style: StyleId,
    pub mode: Mode,
    pub extraction: ExtractionMode,
    pub palette: Option<PaletteInput>,
    pub images: Vec<ImageInput>,
    pub background: BackgroundOptions,
    pub options: ThemeOptions,
    /// `None` selects [`StyleId::default_tweaks`].
    pub tweaks: Option<Tweaks>,
}

impl Default for GenerateRequest {
    fn default() -> Self {
        Self {
            name: "HomeChameleon".into(),
            style: StyleId::Sheer,
            mode: Mode::Both,
            extraction: ExtractionMode::Normal,
            palette: None,
            images: Vec::new(),
            background: BackgroundOptions::default(),
            options: ThemeOptions::default(),
            tweaks: None,
        }
    }
}

impl GenerateRequest {
    pub fn tweaks(&self) -> Tweaks {
        self.tweaks
            .unwrap_or_else(|| self.style.default_tweaks())
            .clamped()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RoleColors {
    pub bg: String,
    pub surface: String,
    pub primary: String,
    pub accent: String,
    pub text: String,
    pub success: String,
    pub warn: String,
    pub error: String,
    pub on_primary: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContrastCheck {
    pub mode: String,
    pub pair: String,
    pub ratio: f64,
    pub minimum: f64,
    pub pass: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContrastReport {
    pub pass: bool,
    pub checks: Vec<ContrastCheck>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenerateResult {
    pub theme: String,
    pub slug: String,
    pub style: StyleId,
    pub files: Vec<String>,
    pub yaml: String,
    pub css: String,
    pub light: Option<RoleColors>,
    pub dark: Option<RoleColors>,
    pub palette_json: String,
    pub contrast: ContrastReport,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GenerateError {
    MissingSource,
    UnknownStyle { style: String },
    TooManyImages { count: usize, limit: usize },
    ImageTooLarge { bytes: usize, limit: usize },
    BadImage { reason: String },
    BadPalette { reason: String },
}

impl std::fmt::Display for GenerateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSource => {
                write!(f, "provide a palette or at least one image")
            }
            Self::UnknownStyle { style } => {
                write!(f, "unknown style '{style}'")
            }
            Self::TooManyImages { count, limit } => {
                write!(f, "{count} images were sent; the limit is {limit}")
            }
            Self::ImageTooLarge { bytes, limit } => {
                write!(f, "image is {bytes} bytes; the limit is {limit}")
            }
            Self::BadImage { reason } | Self::BadPalette { reason } => write!(f, "{reason}"),
        }
    }
}

impl std::error::Error for GenerateError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyleInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub summary: &'static str,
}

pub fn styles() -> Vec<StyleInfo> {
    [
        StyleId::Sheer,
        StyleId::Bubbles,
        StyleId::Material,
        StyleId::Flat,
        StyleId::Starship,
    ]
    .into_iter()
    .map(|style| StyleInfo {
        id: style.id(),
        name: style.display_name(),
        summary: style.summary(),
    })
    .collect()
}

/// Theme file name stem: lowercase `[a-z0-9_-]`.
pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            dash = false;
        } else if (c == '-' || c == '_' || c.is_whitespace()) && !out.is_empty() && !dash {
            out.push(if c == '_' { '_' } else { '-' });
            dash = true;
        }
    }
    while out.ends_with('-') || out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() {
        "theme".into()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_rules() {
        assert_eq!(slugify("Omarchy Desk"), "omarchy-desk");
        assert_eq!(slugify("  --Hello_World-- "), "hello_world");
        assert_eq!(slugify("***"), "theme");
        assert_eq!(slugify("Café Night"), "caf-night");
    }

    #[test]
    fn unknown_style_is_rejected() {
        assert!(matches!(
            StyleId::parse("glass"),
            Err(GenerateError::UnknownStyle { .. })
        ));
    }
}
