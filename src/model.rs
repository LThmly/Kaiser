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
    pub fn demo() -> Self {
        let content = include_str!("../examples/replication.md").to_owned();
        let citation = Citation { source_id: 1, source_name: "replication.md".into(), line: 5, passage: "Replication copies data across multiple nodes. A leader accepts writes and sends updates to replicas. With asynchronous replication, the leader acknowledges a write before every replica has received it. This reduces write latency, but a replica can temporarily serve stale reads.".into() };
        let make_card = |id, front: &str, back: &str, draft| Card {
            id,
            front: front.into(),
            back: back.into(),
            citation: Some(citation.clone()),
            draft,
            history: vec![],
        };
        let systems = Notebook {
            name: "Distributed systems".into(),
            directory: current_directory(),
            sources: vec![
                Source { id: 1, name: "replication.md".into(), content, selected: true },
                Source { id: 2, name: "consistency-notes.md".into(), selected: true, content: "# Consistency notes\n\nRead-your-writes consistency ensures a client can see its own completed writes. Routing that client's reads to the leader is one way to achieve it.\n\nEventual consistency means that, if no new updates occur, replicas converge to the same value. It does not promise that every read immediately returns the latest write.\n\nA network partition prevents some nodes from communicating. Systems must decide whether to reject some requests or allow responses that may be inconsistent.\n".into() },
                Source { id: 3, name: "practice-prompts.md".into(), selected: false, content: "# Practice\n\nA user edits their profile and then refreshes the page. The old name appears. Explain how replication lag could cause this and propose a way to provide read-your-writes consistency.\n".into() },
            ],
            messages: vec![
                Message { role: "user".into(), text: "Why can a replica return a stale read?".into(), citations: vec![] },
                Message { role: "assistant".into(), text: "In asynchronous replication, the leader can acknowledge a write before a replica receives the update. A read sent to that replica during the delay can return the older value. [1]\n\nTry explaining what would happen if you updated your profile and immediately refreshed the page.".into(), citations: vec![citation.clone()] },
            ],
            cards: vec![
                make_card(4, "Why can asynchronous replication produce stale reads?", "The leader may acknowledge a write before a replica receives it. Reads from that replica can return an older value.", false),
                make_card(5, "What does read-your-writes consistency guarantee?", "A client can see its own completed writes in subsequent reads.", false),
                make_card(6, "How could you avoid showing an old profile immediately after an edit?", "Route the user's subsequent reads to the leader, or wait until the chosen replica has caught up.", true),
            ],
            notes: "# Replication\n\nUnderstanding: replicas trade freshness for lower latency and availability.\n\nQuestion to revisit: when is a stale read acceptable?\n\nPractice: explain the profile-refresh example without looking at the sources.".into(),
        };
        // Fix the source for the read-your-writes card; all provenance is explicit.
        let mut systems = systems;
        systems.cards[1].citation = Some(Citation { source_id: 2, source_name: "consistency-notes.md".into(), line: 3, passage: "Read-your-writes consistency ensures a client can see its own completed writes. Routing that client's reads to the leader is one way to achieve it.".into() });
        systems.cards[2].citation = Some(Citation { source_id: 1, source_name: "replication.md".into(), line: 17, passage: "Read-your-writes consistency ensures you can see your own completed updates. One approach is to route your reads to the leader after you write. Another is to track a write position and only read from replicas that have reached that position.".into() });
        Self {
            version: 1,
            review_activity: vec![],
            next_id: 7,
            active: 0,
            notebooks: vec![
                systems,
                Notebook::empty("Biology".into()),
                Notebook::empty("Reading room".into()),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retrieval_respects_selection_and_notebook_boundaries() {
        let mut lib = Library::demo();
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
        let mut card = Library::demo().book().cards[0].clone();
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
        let mut value = serde_json::to_value(Library::demo()).unwrap();
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
        let mut library = Library::demo();
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
        let mut value = serde_json::to_value(Library::demo()).unwrap();
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
        let mut lib = Library::demo();
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
