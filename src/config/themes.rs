//! Built-in palette registry and custom theme inheritance.
use ratatui::style::Color;
use serde::Deserialize;
use std::io;

pub const BAGELS_THEME: &str = include_str!("../../config/themes/kaiser-bagels.toml");
pub const TALLGEESE_THEME: &str = include_str!("../../config/themes/kaiser-tallgeese.toml");

pub const BUILTIN_THEMES: &[(&str, &str)] = &[
    ("kaiser-bagels", BAGELS_THEME),
    (
        "kaiser-copper",
        include_str!("../../config/themes/kaiser-copper.toml"),
    ),
    (
        "kaiser-ember",
        include_str!("../../config/themes/kaiser-ember.toml"),
    ),
    (
        "kaiser-forest",
        include_str!("../../config/themes/kaiser-forest.toml"),
    ),
    (
        "kaiser-glacier",
        include_str!("../../config/themes/kaiser-glacier.toml"),
    ),
    (
        "kaiser-lavender",
        include_str!("../../config/themes/kaiser-lavender.toml"),
    ),
    (
        "kaiser-main",
        include_str!("../../config/themes/kaiser-main.toml"),
    ),
    (
        "kaiser-ocean",
        include_str!("../../config/themes/kaiser-ocean.toml"),
    ),
    (
        "kaiser-orchid",
        include_str!("../../config/themes/kaiser-orchid.toml"),
    ),
    (
        "kaiser-paper",
        include_str!("../../config/themes/kaiser-paper.toml"),
    ),
    (
        "kaiser-sandstone",
        include_str!("../../config/themes/kaiser-sandstone.toml"),
    ),
    (
        "kaiser-slate",
        include_str!("../../config/themes/kaiser-slate.toml"),
    ),
    ("kaiser-tallgeese", TALLGEESE_THEME),
    (
        "kaiser-violet",
        include_str!("../../config/themes/kaiser-violet.toml"),
    ),
];

pub(super) const THEME_ALIASES: &[(&str, &str)] = &[
    ("bagels", "kaiser-bagels"),
    ("copper", "kaiser-copper"),
    ("dusk", "kaiser-violet"),
    ("ember", "kaiser-ember"),
    ("forest", "kaiser-forest"),
    ("glacier", "kaiser-glacier"),
    ("kaiser", "kaiser-lavender"),
    ("kaiser-home", "kaiser-main"),
    ("ocean", "kaiser-ocean"),
    ("orchid", "kaiser-orchid"),
    ("paper", "kaiser-paper"),
    ("sandstone", "kaiser-sandstone"),
    ("slate", "kaiser-slate"),
    ("tallgeese", "kaiser-tallgeese"),
];

pub fn canonical_theme_name(name: &str) -> &str {
    THEME_ALIASES
        .iter()
        .find(|(alias, _)| *alias == name)
        .map_or(name, |(_, canonical)| *canonical)
}

pub fn builtin_theme_names() -> Vec<String> {
    BUILTIN_THEMES
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect()
}
pub fn is_builtin_theme(name: &str) -> bool {
    BUILTIN_THEMES
        .iter()
        .any(|(builtin, _)| *builtin == canonical_theme_name(name))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub background: Color,
    pub panel: Color,
    pub border: Color,
    pub home_border: Option<Color>,
    pub foreground: Color,
    pub muted: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub selection: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::tallgeese()
    }
}

impl Theme {
    pub fn tallgeese() -> Self {
        Self::builtin("kaiser-tallgeese").expect("bundled Tallgeese theme must be valid")
    }
    pub(super) fn builtin(name: &str) -> io::Result<Self> {
        let name = canonical_theme_name(name);
        let (_, raw) = BUILTIN_THEMES
            .iter()
            .find(|(builtin, _)| *builtin == name)
            .ok_or_else(|| {
                io::Error::other(format!(
                    "Unknown base theme '{name}'; use {}",
                    builtin_theme_names().join(", ")
                ))
            })?;
        // Bundled themes are complete palettes. Their files are the only source
        // of color values; custom files may inherit and override those palettes.
        let file: ThemeFile = toml::from_str(raw).map_err(io::Error::other)?;
        file.complete()
    }
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ThemeFile {
    base: Option<String>,
    background: Option<String>,
    panel: Option<String>,
    border: Option<String>,
    home_border: Option<String>,
    foreground: Option<String>,
    muted: Option<String>,
    accent: Option<String>,
    success: Option<String>,
    warning: Option<String>,
    selection: Option<String>,
}

impl ThemeFile {
    fn complete(self) -> io::Result<Theme> {
        let required = |name, value: Option<String>| {
            let value = value
                .ok_or_else(|| io::Error::other(format!("Bundled theme is missing '{name}'")))?;
            parse_color(name, &value)
        };
        Ok(Theme {
            background: required("background", self.background)?,
            panel: required("panel", self.panel)?,
            border: required("border", self.border)?,
            foreground: required("foreground", self.foreground)?,
            muted: required("muted", self.muted)?,
            accent: required("accent", self.accent)?,
            success: required("success", self.success)?,
            warning: required("warning", self.warning)?,
            selection: required("selection", self.selection)?,
            home_border: self
                .home_border
                .as_deref()
                .map(|value| parse_color("home_border", value))
                .transpose()?,
        })
    }
}

fn parse_color(key: &str, value: &str) -> io::Result<Color> {
    let raw = value.strip_prefix('#').unwrap_or("");
    if raw.len() != 6 || !raw.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(io::Error::other(format!(
            "Theme color '{key}' must be #RRGGBB (got '{value}')"
        )));
    }
    let n = u32::from_str_radix(raw, 16).map_err(io::Error::other)?;
    Ok(Color::Rgb((n >> 16) as u8, (n >> 8) as u8, n as u8))
}

pub(super) fn parse_theme(raw: &str) -> io::Result<Theme> {
    let file: ThemeFile = toml::from_str(raw).map_err(io::Error::other)?;
    let mut theme = Theme::builtin(file.base.as_deref().unwrap_or("tallgeese"))?;
    for (name, value, color) in [
        ("background", file.background, &mut theme.background),
        ("panel", file.panel, &mut theme.panel),
        ("border", file.border, &mut theme.border),
        ("foreground", file.foreground, &mut theme.foreground),
        ("muted", file.muted, &mut theme.muted),
        ("accent", file.accent, &mut theme.accent),
        ("success", file.success, &mut theme.success),
        ("warning", file.warning, &mut theme.warning),
        ("selection", file.selection, &mut theme.selection),
    ] {
        if let Some(value) = value {
            *color = parse_color(name, &value)?;
        }
    }
    if let Some(value) = file.home_border {
        theme.home_border = Some(parse_color("home_border", &value)?);
    }
    Ok(theme)
}
