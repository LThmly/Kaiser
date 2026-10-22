//! Fictional, disposable showcase data and starting views for screenshots.
use crate::{
    app::{Action, App, Tab},
    model::{Card, Citation, Library, Message, Notebook, Source},
};
use std::{path::PathBuf, str::FromStr};

mod data;
use data::{ALGEBRA, ALGORITHMS, DEEP_LEARNING, RUST, STATISTICS, SYSTEMS};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum View {
    #[default]
    Home,
    Chat,
    Flashcards,
    Notes,
}
impl FromStr for View {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "home" => Ok(Self::Home),
            "chat" => Ok(Self::Chat),
            "flashcards" | "cards" => Ok(Self::Flashcards),
            "notes" => Ok(Self::Notes),
            _ => Err("Use --demo-view home, chat, flashcards, or notes.".into()),
        }
    }
}

type Fact = (&'static str, &'static str);

fn notebook(library: &mut Library, name: &str, slug: &str, files: &[&str], facts: &[Fact]) {
    let mut book = Notebook::empty(name.into());
    book.directory = PathBuf::from("demo").join(slug);
    for (index, filename) in files.iter().enumerate() {
        let location = if filename.ends_with(".pptx") {
            "Slide"
        } else {
            "Page"
        };
        let mut content = format!("# {name} · sample study source\n\n## {location} 1\n\n");
        for (front, back) in facts.iter().skip(index).step_by(files.len()) {
            content.push_str(&format!("### {front}\n\n{back}\n\n"));
        }
        book.sources.push(Source {
            id: library.id(),
            name: (*filename).into(),
            content,
            selected: index < 3,
        });
    }
    for (index, (front, back)) in facts.iter().enumerate() {
        let source = &book.sources[index % files.len()];
        book.cards.push(Card {
            id: library.id(),
            front: (*front).into(),
            back: (*back).into(),
            citation: Some(Citation {
                source_id: source.id,
                source_name: source.name.clone(),
                line: source
                    .content
                    .lines()
                    .position(|line| line == *back)
                    .unwrap()
                    + 1,
                passage: (*back).into(),
            }),
            draft: index >= facts.len() - 2,
            history: vec![],
        });
    }
    book.notes = format!("# {name}\n\nA notebook for connecting the core ideas, worked examples, and questions worth revisiting.\n\n## Study checklist\n\n- [x] Import the reading and lecture materials\n- [x] Summarize the main ideas in my own words\n- [x] Turn important concepts into flashcards\n- [ ] Explain the difficult examples without looking\n\n## Key ideas\n\n{}
\n## Next session\n\nReview the draft cards, explain one concept aloud, and add a worked example.", facts.iter().take(4).map(|(front, back)| format!("### {front}\n\n{back}\n")).collect::<Vec<_>>().join("\n"));
    library.notebooks.push(book);
}

pub fn library() -> Library {
    let mut library = Library::empty();
    notebook(
        &mut library,
        "Deep Learning",
        "deep-learning",
        &[
            "neural-networks.pdf",
            "training-workshop.pptx",
            "learning-notes.md",
            "model-evaluation.docx",
        ],
        DEEP_LEARNING,
    );
    notebook(
        &mut library,
        "Distributed Systems",
        "distributed-systems",
        &[
            "replication.pdf",
            "consistency-notes.md",
            "systems-design.pptx",
        ],
        SYSTEMS,
    );
    notebook(
        &mut library,
        "Linear Algebra",
        "linear-algebra",
        &["vectors-and-matrices.pdf", "worked-examples.md"],
        ALGEBRA,
    );
    notebook(
        &mut library,
        "Rust",
        "rust",
        &[
            "ownership-and-borrowing.md",
            "rust-workshop.pdf",
            "practice-problems.md",
        ],
        RUST,
    );
    notebook(
        &mut library,
        "Algorithms",
        "algorithms",
        &[
            "algorithm-design.pdf",
            "graph-algorithms.pptx",
            "complexity-notes.md",
        ],
        ALGORITHMS,
    );
    notebook(
        &mut library,
        "Statistics",
        "statistics",
        &["probability.pdf", "inference-notes.md"],
        STATISTICS,
    );
    let book = &mut library.notebooks[0];
    book.notes = "# Deep learning\n\nUnderstand the training loop, then learn to recognize when it stops generalizing.\n\n## This week's focus\n\n- [x] Connect the forward pass, loss, and backpropagation\n- [x] Compare training and validation loss\n- [x] Draft cards from the lecture and reading\n- [ ] Explain the learning-rate experiment without notes\n\n## The training loop\n\n1. Sample a mini-batch from the training data.\n2. Run a **forward pass** to produce predictions.\n3. Measure the **loss** against the targets.\n4. Backpropagate gradients through the network.\n5. Let the optimizer update the parameters.\n\n```python\nfor inputs, targets in training_loader:\n    optimizer.zero_grad()\n    loss = criterion(model(inputs), targets)\n    loss.backward()\n    optimizer.step()\n```\n\n## Diagnose before tuning\n\n| Observation | Next experiment |\n| --- | --- |\n| Training improves, validation worsens | Inspect overfitting and data splits |\n| Loss oscillates or diverges | Try a lower learning rate |\n| Both losses remain high | Check the baseline, features, and model |\n\n> Keep the final test set separate from model selection.\n\n## Questions to revisit\n\n- When does a simpler model generalize better?\n- How do batch size and learning rate interact?\n".into();
    let citation = |index: usize| book.cards[index].citation.clone().unwrap();
    book.messages = vec![
        Message { role: "user".into(), text: "Explain the training loop and what I should focus on first.".into(), citations: vec![] },
        Message { role: "assistant".into(), text: "A training step connects **prediction, loss, and gradients**:\n\n- Run a forward pass and compute the loss.\n- Backpropagate gradients through the layers.\n- Let the optimizer update the parameters. [1][2]".into(), citations: vec![citation(2), citation(3)] },
        Message { role: "user".into(), text: "My training loss keeps falling, but validation loss is rising. What should I investigate?".into(), citations: vec![] },
        Message { role: "assistant".into(), text: "Investigate **overfitting**: the model may be learning training-specific patterns that fail to generalize. [1]\n\nBefore changing the architecture:\n\n- Check that your splits and preprocessing are consistent.\n- Compare representative training and validation examples.\n- Try one controlled change: early stopping, regularization, or a simpler baseline.\n\nUse the validation set for those choices and keep the final test set separate. [2]\n\n**Recall check:** why is lower training loss alone insufficient evidence of improvement?".into(), citations: vec![citation(4), citation(15)] },
    ];
    library
}

pub fn prepare(app: &mut App, view: View) {
    app.demo_mode = true;
    if view != View::Home {
        app.action(Action::Notebook(0));
        app.open(match view {
            View::Chat => Tab::Chat,
            View::Flashcards => Tab::Cards,
            View::Notes => Tab::Notes,
            View::Home => unreachable!(),
        });
        if view == View::Flashcards {
            app.select_card(4);
            app.revealed = true;
        }
    }
    app.status = "DEMO · sample data · changes are discarded".into();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn showcase_has_unique_ids_and_valid_source_passages() {
        let library = library();
        assert_eq!(library.notebooks.len(), 6);
        assert_eq!(
            library
                .notebooks
                .iter()
                .map(|book| book.cards.len())
                .sum::<usize>(),
            84
        );
        let mut ids = std::collections::HashSet::new();
        for book in &library.notebooks {
            assert!(!book.notes.is_empty());
            for source in &book.sources {
                assert!(ids.insert(source.id));
            }
            for card in &book.cards {
                assert!(ids.insert(card.id));
                let citation = card.citation.as_ref().unwrap();
                let source = book
                    .sources
                    .iter()
                    .find(|source| source.id == citation.source_id)
                    .unwrap();
                assert_eq!(
                    source.content.lines().nth(citation.line - 1),
                    Some(citation.passage.as_str())
                );
            }
        }
        assert!(ids.iter().all(|id| *id < library.next_id));
    }
    #[test]
    fn all_showcase_views_render_at_supported_sizes() {
        use ratatui::{Terminal, backend::TestBackend};
        for (width, height) in [(58, 18), (85, 25), (120, 40), (160, 45)] {
            for view in [View::Home, View::Chat, View::Flashcards, View::Notes] {
                let mut app = App::new(library(), None);
                prepare(&mut app, view);
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                terminal
                    .draw(|frame| crate::ui::draw(frame, &mut app))
                    .unwrap();
                let text: String = terminal
                    .backend()
                    .buffer()
                    .content
                    .iter()
                    .map(|cell| cell.symbol())
                    .collect();
                assert!(text.contains("DEMO"));
                if view == View::Flashcards && width >= 120 {
                    assert!(text.contains("ANSWER / DEFINITION"));
                }
            }
        }
    }
    #[test]
    fn starting_views_select_the_intended_workspace_without_recording_reviews() {
        for view in [View::Home, View::Chat, View::Flashcards, View::Notes] {
            let mut app = App::new(library(), None);
            prepare(&mut app, view);
            assert_eq!(app.notebook_selected, view != View::Home);
            if view == View::Flashcards {
                assert!(app.tab == Tab::Cards && app.revealed);
            }
            if view == View::Notes {
                assert!(app.tab == Tab::Notes && app.notes.preview);
            }
            assert!(app.library.review_activity.is_empty());
            assert!(app.provider.is_none() && app.chatgpt.is_none());
            assert!(app.demo_mode);
        }
        assert!("invalid".parse::<View>().is_err());
    }
}
