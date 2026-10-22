//! Bounded chat/card jobs, result handling, and cancellation.
use super::{App, Card, JobKind, Message, Modal, Pending, Tab, local_answer, mpsc};

impl App {
    /// Discard pending results and stop subsequent pipeline requests.
    pub(super) fn cancel_generation(&mut self) {
        self.pending = None;
        self.progress_receiver = None;
        self.generation_progress.clear();
    }
    pub fn send(&mut self) {
        self.send_approved(None);
    }
    pub(super) fn run_approved_plan(
        &mut self,
        cards: Option<crate::flashcards::Request>,
        fingerprint: String,
        requests: usize,
    ) {
        if let Some(request) = cards {
            self.generate_cards(request, Some((fingerprint, requests)));
        } else {
            self.send_approved(Some((fingerprint, requests)));
        }
    }
    pub(super) fn generate_cards(
        &mut self,
        request: crate::flashcards::Request,
        approved: Option<(String, usize)>,
    ) {
        if !self.require_notebook() {
            return;
        }
        if self.pending.is_some() {
            self.status = "Wait for the current generation to finish.".into();
            return;
        }
        let Some(provider) = self.provider.clone() else {
            self.status =
                "Connect a model with :login or configure an API provider to generate flashcards."
                    .into();
            return;
        };
        if provider.model.trim().is_empty() {
            self.load_models();
            self.status = "Choose a model with :model to generate flashcards.".into();
            return;
        }
        let plan = match crate::retrieval::plan(
            self.library.book(),
            &request.question(),
            &self.ai.policy(),
        ) {
            Ok(plan) => plan,
            Err(error) => {
                self.status = error;
                return;
            }
        };
        if plan.requests > 1 || approved.is_some() {
            let fingerprint = crate::usage::digest(
                format!(
                    "cards:{}:{}:{}:{}:{}",
                    self.library.active,
                    plan.signature,
                    request.question(),
                    provider.model,
                    provider.scope()
                )
                .as_bytes(),
            );
            if approved != Some((fingerprint.clone(), plan.requests)) {
                self.modal_index = 0;
                self.editing = false;
                self.modal = Some(Modal::SummaryPlan {
                    cards: Some(request),
                    fingerprint,
                    requests: plan.requests,
                    label: plan.label.clone(),
                    bytes: plan.bytes,
                });
                return;
            }
        }
        let (sender, receiver) = mpsc::channel();
        let (progress, progress_receiver) = mpsc::channel();
        self.progress_receiver = Some(progress_receiver);
        self.generation_progress = "Generating flashcard drafts…".into();
        self.pending = Some(Pending {
            kind: JobKind::Cards(request.count),
            receiver,
            notebook: self.library.active,
            citations: plan.citations.clone(),
        });
        std::thread::spawn(move || {
            let _ = sender.send(provider.generate_cards(&request, &plan, &progress));
        });
        self.status = "Generating flashcard drafts from selected sources…".into();
    }
    pub(super) fn send_approved(&mut self, approved: Option<(String, usize)>) {
        if !self.require_notebook() {
            return;
        }
        let question = self.input.trim().to_owned();
        if question.is_empty() {
            return;
        }
        if self.pending.is_some() {
            self.status = "Wait for the current answer before sending another question.".into();
            return;
        }
        if let Some(provider) = &self.provider {
            if let Err(error) = provider.check_question(&question) {
                self.status = error;
                return;
            }
            if provider.uses_chatgpt() && provider.model.is_empty() {
                self.load_models();
                self.status =
                    "Choose a model once; Kaiser saves your selection for this account.".into();
                return;
            }
        }
        if !self.library.book().sources.iter().any(|s| s.selected) {
            self.status = "Select at least one source with x in the Sources panel.".into();
            return;
        }
        let policy = self.ai.policy();
        let mut plan = if self.provider.is_some() {
            match crate::retrieval::plan(self.library.book(), &question, &policy) {
                Ok(plan) => plan,
                Err(error) => {
                    self.status = error;
                    return;
                }
            }
        } else {
            crate::retrieval::offline(self.library.book(), &question)
        };
        let words: Vec<String> = question
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase)
            .collect();
        let follow_up = words
            .iter()
            .any(|word| ["this", "that", "it"].contains(&word.as_str()))
            || ["why", "how so", "tell me more", "explain further"]
                .contains(&words.join(" ").as_str());
        if plan.citations.is_empty() && follow_up {
            // Short follow-ups can reuse evidence, but never a deselected source.
            if let Some(previous) = self
                .library
                .book()
                .messages
                .iter()
                .rev()
                .find(|m| m.role == "assistant")
            {
                let previous_citations = previous
                    .citations
                    .iter()
                    .filter(|citation| {
                        self.library
                            .book()
                            .sources
                            .iter()
                            .any(|s| s.id == citation.source_id && s.selected)
                    })
                    .cloned()
                    .collect();
                plan.reuse(
                    previous_citations,
                    policy.source_bytes.min(
                        policy
                            .max_input_bytes
                            .saturating_sub(question.len() * 6 + 1600),
                    ),
                );
            }
        }
        if (plan.requests > 1 || approved.is_some())
            && let Some(provider) = &self.provider
        {
            let fingerprint = crate::usage::digest(
                format!(
                    "{}:{}:{}:{}",
                    plan.signature,
                    question,
                    provider.model,
                    provider.scope()
                )
                .as_bytes(),
            );
            if approved.as_ref() != Some(&(fingerprint.clone(), plan.requests)) {
                self.modal_index = 0;
                self.editing = false;
                self.modal = Some(Modal::SummaryPlan {
                    cards: None,
                    fingerprint,
                    requests: plan.requests,
                    label: plan.label.clone(),
                    bytes: plan.bytes,
                });
                return;
            }
        }
        let citations = plan.citations.clone();
        self.library.book_mut().messages.push(Message {
            role: "user".into(),
            text: question,
            citations: vec![],
        });
        self.input.clear();
        self.editing = false;
        self.notes.normal();
        self.scroll = None;
        self.dirty = true;
        if let Some(provider) = self.provider.clone() {
            let mut fingerprint = ring::digest::Context::new(&ring::digest::SHA256);
            for source in self
                .library
                .book()
                .sources
                .iter()
                .filter(|source| source.selected)
            {
                fingerprint.update(&source.id.to_le_bytes());
                fingerprint.update(&(source.content.len() as u64).to_le_bytes());
                fingerprint.update(source.content.as_bytes());
                fingerprint.update(&(source.name.len() as u64).to_le_bytes());
                fingerprint.update(source.name.as_bytes());
            }
            let source_scope = crate::usage::digest(fingerprint.finish().as_ref());
            let messages = &self.library.book().messages;
            let history = messages[messages.len().saturating_sub(10)..].to_vec();
            let (sender, receiver) = mpsc::channel();
            let (progress, progress_receiver) = mpsc::channel();
            self.progress_receiver = Some(progress_receiver);
            self.generation_progress = if plan.requests > 1 {
                format!("Summary · up to {} requests", plan.requests)
            } else {
                "Reading source text…".into()
            };
            std::thread::spawn(move || {
                let _ = sender.send(provider.answer(&history, &plan, &source_scope, &progress));
            });
            self.pending = Some(Pending {
                kind: JobKind::Chat,
                receiver,
                notebook: self.library.active,
                citations,
            });
            self.status = "Thinking with your selected sources… You can keep navigating.".into();
        } else {
            self.library.book_mut().messages.push(Message {
                role: "assistant".into(),
                text: local_answer(&citations),
                citations,
            });
            self.status = format!(
                "Source passages ready · {} m to draft a card · Open Sources to inspect the original text",
                self.leader_label()
            );
        }
    }
    pub(super) fn finish_generated_cards(&mut self, pending: Pending, text: &str, count: usize) {
        let cards = match crate::flashcards::parse(text, count, &pending.citations) {
            Ok(cards) => cards,
            Err(error) => {
                self.status = error;
                return;
            }
        };
        let mut added = Vec::new();
        for card in cards {
            if self.library.notebooks[pending.notebook]
                .cards
                .iter()
                .any(|existing| {
                    crate::flashcards::normalized(&existing.front)
                        == crate::flashcards::normalized(&card.front)
                })
            {
                continue;
            }
            let id = self.library.id();
            self.library.notebooks[pending.notebook].cards.push(Card {
                id,
                front: card.front,
                back: card.back,
                citation: Some(card.citation),
                draft: true,
                history: vec![],
            });
            added.push(id);
        }
        if added.is_empty() {
            self.status =
                "No new flashcards: generated questions already exist in this notebook.".into();
            return;
        }
        self.dirty = true;
        if self.notebook_selected
            && self.library.active == pending.notebook
            && self.modal.is_none()
            && !self.editing
        {
            self.drafts_only = true;
            self.open(Tab::Cards);
            let first = self
                .cards()
                .iter()
                .position(|card| card.id == added[0])
                .unwrap_or(0);
            self.select_card(first);
        }
        self.status = format!(
            "Generated {} drafts in {} · :drafts to review",
            added.len(),
            self.library.notebooks[pending.notebook].name
        );
    }
    pub fn poll_answer(&mut self) {
        if let Some(receiver) = &self.progress_receiver
            && let Some(progress) = receiver.try_iter().last()
        {
            self.generation_progress = progress.clone();
            if self
                .pending
                .as_ref()
                .is_some_and(|pending| pending.notebook == self.library.active)
            {
                self.status = progress;
            }
        }
        let result = self.pending.as_ref().map(|p| p.receiver.try_recv());
        match result {
            Some(Ok(result)) => {
                let pending = self.pending.take().unwrap();
                self.progress_receiver = None;
                self.generation_progress.clear();
                match result {
                    Ok(text) => {
                        if let JobKind::Cards(count) = pending.kind {
                            self.finish_generated_cards(pending, &text, count);
                            return;
                        }
                        self.library.notebooks[pending.notebook]
                            .messages
                            .push(Message {
                                role: "assistant".into(),
                                text,
                                citations: pending.citations,
                            });
                        self.dirty = true;
                        self.scroll = None;
                        self.status =
                            format!("Answer ready · {} m to draft a card", self.leader_label());
                    }
                    Err(error) => self.status = error,
                }
            }
            Some(Err(mpsc::TryRecvError::Disconnected)) => {
                self.cancel_generation();
                self.status = "Model worker stopped unexpectedly. Try again.".into();
            }
            _ => {}
        }
    }
}
