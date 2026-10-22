use std::path::{Path, PathBuf};

pub struct Entry {
    pub path: PathBuf,
    pub label: String,
    pub directory: bool,
}

/// Search one directory at a time; never recursively scan the user's filesystem.
pub fn matches(path: &Path, trailing_separator: bool) -> Result<Vec<Entry>, String> {
    let (directory, query) = if trailing_separator {
        (path, String::new())
    } else {
        (
            path.parent().unwrap_or(path),
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase(),
        )
    };
    let reader =
        std::fs::read_dir(directory).map_err(|error| format!("Cannot read directory: {error}"))?;
    let mut entries = Vec::new();
    for entry in reader.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.to_lowercase().contains(&query)
            || (name.starts_with('.') && !query.starts_with('.'))
        {
            continue;
        }
        let path = entry.path();
        let directory = path.is_dir();
        if directory || path.is_file() {
            entries.push(Entry {
                path,
                label: format!("{name}{}", if directory { "/" } else { "" }),
                directory,
            });
        }
    }
    entries.sort_by_key(|entry| {
        let name = entry.label.to_lowercase();
        (!entry.directory, !name.starts_with(&query), name)
    });
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn searches_names_case_insensitively_and_browses_directories() {
        let root = std::env::temp_dir().join(format!("kaiser-files-{}", std::process::id()));
        std::fs::create_dir_all(root.join("Papers")).unwrap();
        for name in ["Study Guide.pdf", "Guide.pdf", ".hidden.pdf", "notes.md"] {
            std::fs::write(root.join(name), "test").unwrap();
        }
        let found = matches(&root.join("GUIDE"), false).unwrap();
        assert_eq!(
            found.iter().map(|e| e.label.as_str()).collect::<Vec<_>>(),
            ["Guide.pdf", "Study Guide.pdf"]
        );
        let found = matches(&root, true).unwrap();
        assert_eq!(found[0].label, "Papers/");
        assert!(!found.iter().any(|e| e.label.starts_with('.')));
        assert_eq!(matches(&root.join(".hid"), false).unwrap().len(), 1);
        assert!(matches(&root.join("missing/file"), false).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
