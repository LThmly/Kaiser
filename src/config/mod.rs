//! Configuration loading and non-destructive initialization.
mod settings;
#[cfg(test)]
mod tests;
mod themes;

pub use settings::*;
#[cfg(test)]
use themes::{BAGELS_THEME, TALLGEESE_THEME};
use themes::{BUILTIN_THEMES, THEME_ALIASES, parse_theme};
pub use themes::{Theme, builtin_theme_names, canonical_theme_name, is_builtin_theme};

use ratatui::style::Color;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub const DEFAULT_CONFIG: &str = include_str!("../../config/config.toml");
pub fn default_path() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| ".".into())).join(".config")
        })
        .join("kaiser/config.toml")
}

fn at_path(path: &Path, error: impl std::fmt::Display) -> io::Error {
    io::Error::other(format!("{}: {error}", path.display()))
}

pub fn load(path: &Path, required: bool) -> io::Result<(Config, Theme)> {
    let mut config = match fs::read_to_string(path) {
        Ok(raw) => toml::from_str::<Config>(&raw).map_err(|e| at_path(path, e))?,
        Err(e) if e.kind() == io::ErrorKind::NotFound && !required => Config::default(),
        Err(e) => return Err(at_path(path, e)),
    };
    config.theme = canonical_theme_name(&config.theme).to_owned();
    config.validate().map_err(|e| at_path(path, e))?;
    let theme = load_theme(path, &config.theme)?;
    Ok((config, theme))
}

pub fn theme_path(path: &Path, name: &str) -> io::Result<PathBuf> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(at_path(
            path,
            "Theme names may contain only letters, numbers, hyphens, and underscores",
        ));
    }
    Ok(path
        .parent()
        .unwrap_or(Path::new("."))
        .join("themes")
        .join(format!("{}.toml", canonical_theme_name(name))))
}

fn theme_contents(path: &Path, name: &str) -> io::Result<Option<(PathBuf, String)>> {
    let canonical = canonical_theme_name(name);
    let primary = theme_path(path, canonical)?;
    let directory = primary.parent().unwrap();
    let candidates = std::iter::once(primary.clone()).chain(
        THEME_ALIASES
            .iter()
            .filter(|(_, target)| *target == canonical)
            .map(|(alias, _)| directory.join(format!("{alias}.toml"))),
    );
    for candidate in candidates {
        match fs::read_to_string(&candidate) {
            Ok(raw) => return Ok(Some((candidate, raw))),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(at_path(&candidate, error)),
        }
    }
    Ok(None)
}

pub fn load_theme(path: &Path, name: &str) -> io::Result<Theme> {
    let target = theme_path(path, name)?;
    let mut theme = if let Some((source, raw)) = theme_contents(path, name)? {
        parse_theme(&raw).map_err(|error| at_path(&source, error))?
    } else {
        Theme::builtin(name).map_err(|_| at_path(&target, "Theme file not found"))?
    };
    if canonical_theme_name(name) == "kaiser-main" && theme.home_border.is_none() {
        theme.home_border = Some(theme.border);
    }
    Ok(theme)
}

pub fn theme_names(path: &Path) -> io::Result<Vec<String>> {
    let mut names = builtin_theme_names();
    let directory = path.parent().unwrap_or(Path::new(".")).join("themes");
    match fs::read_dir(directory) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry?;
                let file = entry.path();
                if file.extension().is_some_and(|ext| ext == "toml")
                    && file.is_file()
                    && let Some(name) = file.file_stem().and_then(|s| s.to_str())
                    && theme_path(path, name).is_ok()
                {
                    names.push(canonical_theme_name(name).to_owned());
                }
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    names.sort();
    names.dedup();
    Ok(names)
}

pub fn editable_theme(path: &Path, name: &str) -> io::Result<PathBuf> {
    let target = theme_path(path, name)?;
    if target.try_exists()? {
        return Ok(target);
    }
    // Materialize built-ins for editing; custom missing files remain an error.
    let theme = load_theme(path, name)?;
    let fields = [
        ("background", theme.background),
        ("panel", theme.panel),
        ("border", theme.border),
        ("foreground", theme.foreground),
        ("muted", theme.muted),
        ("accent", theme.accent),
        ("success", theme.success),
        ("warning", theme.warning),
        ("selection", theme.selection),
    ];
    let mut contents = format!("# Kaiser theme · edit, then :theme-reload\nbase = \"{name}\"\n\n");
    for (key, color) in fields {
        if let Color::Rgb(r, g, b) = color {
            contents.push_str(&format!("{key} = \"#{r:02X}{g:02X}{b:02X}\"\n"));
        }
    }
    if let Some(Color::Rgb(r, g, b)) = theme.home_border {
        contents.push_str(&format!("home_border = \"#{r:02X}{g:02X}{b:02X}\"\n"));
    }
    if let Some((_, legacy)) = theme_contents(path, name)? {
        contents = legacy;
    }
    fs::create_dir_all(target.parent().unwrap())?;
    use std::io::Write;
    match fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&target)
    {
        Ok(mut file) => {
            file.write_all(contents.as_bytes())?;
            file.sync_all()?;
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e),
    }
    Ok(target)
}

/// Write missing defaults, preserving any existing user configuration and theme.
pub fn initialize(path: &Path) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent.join("themes"))?;
    let mut files = Vec::new();
    for (name, preset) in BUILTIN_THEMES {
        let contents =
            theme_contents(path, name)?.map_or_else(|| preset.to_string(), |(_, raw)| raw);
        files.push((theme_path(path, name)?, contents));
    }
    files.push((path.to_owned(), DEFAULT_CONFIG.to_owned()));
    for (path, contents) in files {
        use std::io::Write;
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                file.write_all(contents.as_bytes())?;
                file.sync_all()?;
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(at_path(&path, e)),
        }
    }
    Ok(())
}
