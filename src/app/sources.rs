//! Background document imports and source attachment.
use super::{App, PathBuf, PendingImport, mpsc};

impl App {
    pub fn start_import(&mut self, path: PathBuf) {
        if self.pending_import.is_some() {
            self.status =
                "An import is already running. You can keep studying while it finishes.".into();
            return;
        }
        let path = self.source_path(&path);
        let notebook = self.library.active;
        let (sender, receiver) = mpsc::channel();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(|| crate::import::extract(&path))
                .map_err(|_| "The document parser could not read this file".to_owned())
                .and_then(|result| result.map_err(|e| e.to_string()));
            let _ = sender.send(result);
        });
        self.pending_import = Some(PendingImport { receiver, notebook });
        self.status = format!("Importing {name} locally · You can keep studying");
    }
    pub fn poll_import(&mut self) {
        let result = self.pending_import.as_ref().map(|p| p.receiver.try_recv());
        match result {
            Some(Ok(result)) => {
                let pending = self.pending_import.take().unwrap();
                match result {
                    Ok(source) => {
                        let name = source.name.clone();
                        self.library.add_source(pending.notebook, source);
                        self.dirty = true;
                        if self.notebook_selected && self.library.active == pending.notebook {
                            self.source = self.library.book().sources.len() - 1;
                        }
                        self.status = format!(
                            "Imported {name} into {} · Text is ready for questions and flashcards",
                            self.library.notebooks[pending.notebook].name
                        );
                    }
                    Err(error) => self.status = format!("Import failed: {error}"),
                }
            }
            Some(Err(mpsc::TryRecvError::Disconnected)) => {
                self.pending_import = None;
                self.status = "Import worker stopped. Try importing the file again.".into();
            }
            _ => {}
        }
    }
}
