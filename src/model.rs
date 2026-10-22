use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::time::{SystemTime, UNIX_EPOCH};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[cfg(test)]
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: u64,
    pub name: String,
    pub content: String,
    pub selected: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Citation {
    pub source_id: u64,
    pub source_name: String,
    pub line: usize,
    pub passage: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub text: String,
    pub citations: Vec<Citation>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Review {
    pub at: u64,
    pub rating: u8,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: u64,
    pub front: String,
    pub back: String,
    pub citation: Option<Citation>,
    pub draft: bool,
    pub history: Vec<Review>,
}

#[cfg(test)]
impl Card {
    pub fn rate(&mut self, rating: u8, at: u64) {
        self.history.push(Review { at, rating });
    }
}

pub fn current_directory() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Notebook {
    pub name: String,
    #[serde(default = "current_directory")]
    pub directory: PathBuf,
    pub sources: Vec<Source>,
    pub messages: Vec<Message>,
    pub cards: Vec<Card>,
    pub notes: String,
}

impl Notebook {
    pub fn empty(name: String) -> Self {
        Self {
            name,
            directory: current_directory(),
            sources: vec![],
            messages: vec![],
            cards: vec![],
            notes: String::new(),
        }
    }
    pub fn flashcard_count(&self) -> usize {
        self.cards.len()
    }
    pub fn passages(&self, query: &str) -> Vec<Citation> {
        crate::retrieval::ranked(&self.sources, query)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Library {
    pub version: u32,
    pub next_id: u64,
    pub active: usize,
    pub notebooks: Vec<Notebook>,
    #[serde(default)]
    pub review_activity: Vec<crate::progress::StudyDay>,
}

impl Library {
    pub fn empty() -> Self {
        Self {
            version: 1,
            next_id: 1,
            active: 0,
            notebooks: vec![],
            review_activity: vec![],
        }
    }
    pub fn load(path: &Path) -> io::Result<Self> {
        match fs::read_to_string(path) {
            Ok(raw) => {
                let library: Self = serde_json::from_str(&raw).map_err(io::Error::other)?;
                if library.version != 1
                    || (library.notebooks.is_empty() && library.active != 0)
                    || (!library.notebooks.is_empty() && library.active >= library.notebooks.len())
                {
                    return Err(io::Error::other(
                        "Unsupported or invalid Kaiser library; original file was left untouched",
                    ));
                }
                Ok(library)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Self::empty()),
            Err(e) => Err(e),
        }
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temp = path.with_extension("json.tmp");
        let mut file = fs::File::create(&temp)?;
        use std::io::Write;
        file.write_all(&serde_json::to_vec_pretty(self).map_err(io::Error::other)?)?;
        file.sync_all()?;
        fs::rename(temp, path)
    }
    pub fn book(&self) -> &Notebook {
        &self.notebooks[self.active]
    }
    pub fn book_mut(&mut self) -> &mut Notebook {
        &mut self.notebooks[self.active]
    }
    pub fn id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
    #[cfg(test)]
    pub fn import(&mut self, path: &Path) -> io::Result<()> {
        let source = crate::import::extract(path)?;
        self.add_source(self.active, source);
        Ok(())
    }
    pub fn add_source(&mut self, notebook: usize, source: crate::import::ExtractedSource) {
        let id = self.id();
        self.notebooks[notebook].sources.push(Source {
            id,
            name: source.name,
            content: source.content,
            selected: true,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retrieval_respects_selection_and_notebook_boundaries() {
        let mut lib = Library::fixture();
        assert!(!lib.book().passages("replication stale").is_empty());
        lib.book_mut()
            .sources
            .iter_mut()
            .for_each(|s| s.selected = false);
        assert!(lib.book().passages("replication stale").is_empty());
        lib.active = 1;
        assert!(lib.book().passages("replication").is_empty());
    }
    #[test]
    fn rating_records_recall_without_scheduling() {
        let mut card = Library::fixture().book().cards[0].clone();
        card.rate(3, 100);
        card.rate(1, 200);
        assert_eq!(card.history.len(), 2);
        assert_eq!(card.history[0].rating, 3);
        assert_eq!(card.history[1].at, 200);
        let value = serde_json::to_value(card).unwrap();
        assert!(value.get("due").is_none());
        assert!(value.get("interval").is_none());
        assert!(value["history"][0].get("interval").is_none());
    }

    #[test]
    fn legacy_schedules_are_ignored_while_cards_and_recall_history_survive() {
        let mut value = serde_json::to_value(Library::fixture()).unwrap();
        let card = &mut value["notebooks"][0]["cards"][0];
        card["due"] = serde_json::json!(u64::MAX);
        card["interval"] = serde_json::json!(86400);
        card["history"] = serde_json::json!([{ "at": 100, "rating": 3, "interval": 86400 }]);
        let library: Library = serde_json::from_value(value).unwrap();
        assert_eq!(library.book().flashcard_count(), 3);
        assert_eq!(library.book().cards[0].history[0].rating, 3);
        assert_eq!(library.book().cards[0].history[0].at, 100);
        let saved = serde_json::to_value(library).unwrap();
        let card = &saved["notebooks"][0]["cards"][0];
        assert!(card.get("due").is_none());
        assert!(card.get("interval").is_none());
        assert!(card["history"][0].get("interval").is_none());
    }

    #[test]
    fn imported_utf8_source_is_selected_and_independent_of_original_file() {
        let path = std::env::temp_dir().join(format!("kaiser-import-{}.md", std::process::id()));
        fs::write(&path, "# Learning\n\nA useful example: λ and 中文.\n").unwrap();
        let mut library = Library::fixture();
        library.active = 1;
        library.import(&path).unwrap();
        fs::remove_file(&path).unwrap();
        let source = &library.book().sources[0];
        assert!(source.selected);
        assert!(source.content.contains("中文"));
        assert_eq!(library.book().passages("example")[0].line, 3);
        assert!(library.import(Path::new("unsupported.pdf")).is_err());
    }
    #[test]
    fn old_notebooks_default_to_the_launch_directory() {
        let mut value = serde_json::to_value(Library::fixture()).unwrap();
        for notebook in value["notebooks"].as_array_mut().unwrap() {
            notebook.as_object_mut().unwrap().remove("directory");
        }
        let library: Library = serde_json::from_value(value).unwrap();
        assert!(
            library
                .notebooks
                .iter()
                .all(|book| book.directory == current_directory())
        );
    }

    #[test]
    fn fresh_libraries_start_empty_and_saved_notebooks_remain_intact() {
        let root =
            std::env::temp_dir().join(format!("kaiser-fresh-{}-{}", std::process::id(), now()));
        let path = root.join("state.json");
        let mut library = Library::load(&path).unwrap();
        assert!(library.notebooks.is_empty());
        assert_eq!(library.id(), 1);
        assert!(!path.exists());
        library
            .notebooks
            .push(Notebook::empty("My notebook".into()));
        library.save(&path).unwrap();
        assert_eq!(Library::load(&path).unwrap().book().name, "My notebook");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn library_roundtrip_and_invalid_data_is_not_replaced() {
        let root =
            std::env::temp_dir().join(format!("kaiser-test-{}-{}", std::process::id(), now()));
        let path = root.join("state.json");
        let mut lib = Library::fixture();
        lib.book_mut().notes = "Unicode: λ 中文 🦀".into();
        lib.book_mut().directory = root.join("course notes");
        lib.save(&path).unwrap();
        assert_eq!(
            Library::load(&path).unwrap().book().notes,
            "Unicode: λ 中文 🦀"
        );
        assert_eq!(
            Library::load(&path).unwrap().book().directory,
            root.join("course notes")
        );
        fs::write(&path, "broken").unwrap();
        assert!(Library::load(&path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "broken");
        fs::remove_dir_all(root).unwrap();
    }
}
