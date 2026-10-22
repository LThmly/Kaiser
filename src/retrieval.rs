//! Local source scoping, complete-context planning, and bounded lexical retrieval.
use crate::{
    model::{Citation, Notebook, Source},
    usage::{self, Policy},
};
use std::collections::HashSet;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Complete,
    Focused,
    Summary,
}
#[derive(Clone)]
pub struct Plan {
    pub mode: Mode,
    pub citations: Vec<Citation>,
    pub batches: Vec<Vec<usize>>,
    pub requests: usize,
    pub note_limit: usize,
    pub label: String,
    pub bytes: usize,
    pub signature: String,
}
impl Plan {
    pub fn reuse(&mut self, candidates: Vec<Citation>, budget: usize) {
        let mut citations = Vec::new();
        for candidate in candidates {
            citations.push(candidate);
            if cost(&evidence(
                &citations,
                &(0..citations.len()).collect::<Vec<_>>(),
            )) > budget
            {
                citations.pop();
            }
        }
        self.citations = citations;
        self.mode = Mode::Focused;
        self.batches = vec![(0..self.citations.len()).collect()];
        self.requests = 1;
        self.bytes = self.citations.iter().map(|c| c.passage.len()).sum();
        self.signature = usage::digest(evidence(&self.citations, &self.batches[0]).as_bytes());
    }
}
pub fn offline(book: &Notebook, question: &str) -> Plan {
    let mut plan = Plan {
        mode: Mode::Focused,
        citations: vec![],
        batches: vec![],
        requests: 1,
        note_limit: 0,
        label: "local passages".into(),
        bytes: 0,
        signature: String::new(),
    };
    plan.reuse(book.passages(question), Policy::default().source_bytes);
    plan
}
struct Scope<'a> {
    source: &'a Source,
    text: &'a str,
    line: usize,
}
struct Chunk<'a> {
    source: &'a Source,
    text: &'a str,
    line: usize,
}

pub fn is_summary(question: &str) -> bool {
    let lower = question.to_lowercase();
    words(&lower).iter().any(|word| {
        ["summarize", "summarise", "summary", "overview", "outline"].contains(&word.as_str())
    }) || [
        "key ideas",
        "main ideas",
        "main points",
        "key points",
        "key takeaways",
    ]
    .iter()
    .any(|term| lower.contains(term))
}
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase)
        .collect()
}
fn selector(question: &str) -> Option<(String, usize, usize)> {
    let tokens = words(question);
    for pair in tokens.windows(2) {
        let kind = match pair[0].as_str() {
            "chapter" | "chapters" => "chapter",
            "page" | "pages" => "page",
            "slide" | "slides" => "slide",
            _ => continue,
        };
        if let Ok(start) = pair[1].parse::<usize>() {
            if start == 0 {
                continue;
            }
            let lower = question.to_lowercase();
            // A range must immediately follow this selector, never a later unrelated number.
            let marker = format!("{} {}", pair[0], pair[1]);
            let end = lower
                .find(&marker)
                .and_then(|at| {
                    let rest = lower[at + marker.len()..].trim_start();
                    let tail = rest
                        .strip_prefix('-')
                        .or_else(|| rest.strip_prefix('–'))
                        .or_else(|| rest.strip_prefix("to "))?;
                    tail.trim_start()
                        .split(|c: char| !c.is_ascii_digit())
                        .next()?
                        .parse::<usize>()
                        .ok()
                })
                .unwrap_or(start);
            return Some((kind.into(), start, end.max(start)));
        }
    }
    None
}
fn heading_number(line: &str, kind: &str) -> Option<usize> {
    let line = line.trim().trim_start_matches('#').trim();
    if line.len() > 160 || line.contains("...") || line.contains('…') {
        return None;
    }
    let tokens = words(line);
    if tokens.first()?.as_str() != kind {
        return None;
    }
    tokens.get(1)?.parse().ok()
}
fn scoped<'a>(sources: &'a [Source], question: &str) -> Result<(Vec<Scope<'a>>, String), String> {
    let target = selector(question);
    if let Some((kind, first, last)) = &target {
        if kind == "chapter" && first != last {
            return Err("Use a single chapter number or a page range for this summary.".into());
        }
        if last.saturating_sub(*first) > 1000 {
            return Err(
                "The requested page range is too large; narrow it before summarizing.".into(),
            );
        }
    }
    let mut scopes = Vec::new();
    for source in sources.iter().filter(|s| s.selected) {
        let Some((kind, first, last)) = &target else {
            scopes.push(Scope {
                source,
                text: &source.content,
                line: 1,
            });
            continue;
        };
        let mut offset = 0;
        let mut headings = Vec::new();
        for (index, line) in source.content.split_inclusive('\n').enumerate() {
            if let Some(number) = heading_number(line, kind) {
                headings.push((number, offset, index + 1));
            }
            offset += line.len();
        }
        if kind == "page" || kind == "slide" {
            for (index, &(number, start, line)) in headings.iter().enumerate() {
                if (*first..=*last).contains(&number) {
                    let end = headings
                        .get(index + 1)
                        .map(|h| h.1)
                        .unwrap_or(source.content.len());
                    scopes.push(Scope {
                        source,
                        text: &source.content[start..end],
                        line,
                    });
                }
            }
        } else {
            // Repeated running chapter headings do not terminate the chapter. Prefer
            // the substantive chapter body over a short table-of-contents occurrence.
            let mut best = None;
            for (index, &(number, start, line)) in headings.iter().enumerate() {
                if number != *first || (index > 0 && headings[index - 1].0 == number) {
                    continue;
                }
                let end = headings
                    .iter()
                    .skip(index + 1)
                    .find(|h| h.0 != number)
                    .map(|h| h.1)
                    .unwrap_or(source.content.len());
                if best.is_none_or(|(_, old_start, old_end, _)| end - start > old_end - old_start) {
                    best = Some((number, start, end, line));
                }
            }
            if let Some((_, start, end, line)) = best {
                if heading_only(&source.content[start..end]) {
                    continue;
                }
                scopes.push(Scope {
                    source,
                    text: &source.content[start..end],
                    line,
                });
            } else if headings.is_empty()
                && words(&source.name)
                    .windows(2)
                    .any(|pair| pair[0] == "chapter" && pair[1].parse::<usize>() == Ok(*first))
            {
                scopes.push(Scope {
                    source,
                    text: &source.content,
                    line: 1,
                });
            }
        }
    }
    let label = target
        .as_ref()
        .map(|(kind, first, last)| {
            if first == last {
                format!("{kind} {first}")
            } else {
                format!("{kind}s {first}–{last}")
            }
        })
        .unwrap_or_else(|| "selected sources".into());
    if scopes.is_empty() {
        return Err(format!(
            "Could not locate {label} in the selected sources. Select the chapter file or specify a page range."
        ));
    }
    if let Some((kind, first, last)) = target
        && (kind == "page" || kind == "slide")
    {
        let found: HashSet<_> = scopes
            .iter()
            .filter_map(|scope| heading_number(scope.text.lines().next().unwrap_or(""), &kind))
            .collect();
        if let Some(missing) = (first..=last).find(|number| !found.contains(number)) {
            return Err(format!(
                "{kind} {missing} is missing from the selected extracted text. Choose an available range."
            ));
        }
    }
    Ok((scopes, label))
}
fn chunks<'a>(scope: &Scope<'a>, max: usize) -> Vec<Chunk<'a>> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut line = scope.line;
    while start < scope.text.len() {
        let mut end = scope
            .text
            .floor_char_boundary((start + max.max(4)).min(scope.text.len()));
        if end < scope.text.len() {
            let slice = &scope.text[start..end];
            if let Some(boundary) = slice.rfind("\n\n").filter(|at| *at >= slice.len() / 2) {
                end = start + boundary + 2;
            } else if let Some(boundary) = slice.rfind('\n').filter(|at| *at >= slice.len() / 2) {
                end = start + boundary + 1;
            }
        }
        let text = &scope.text[start..end];
        result.push(Chunk {
            source: scope.source,
            text,
            line,
        });
        line += text.bytes().filter(|b| *b == b'\n').count();
        start = end;
    }
    result
}
fn owned(chunk: &Chunk<'_>) -> Citation {
    Citation {
        source_id: chunk.source.id,
        source_name: chunk.source.name.clone(),
        line: chunk.line,
        passage: chunk.text.to_owned(),
    }
}
fn heading_only(text: &str) -> bool {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .all(|line| {
            let trimmed = line.trim();
            trimmed.starts_with('#')
                || ["chapter", "page", "slide"]
                    .iter()
                    .any(|kind| heading_number(trimmed, kind).is_some())
                || (trimmed.len() < 100
                    && trimmed.chars().any(char::is_alphabetic)
                    && trimmed
                        .chars()
                        .filter(|c| c.is_alphabetic())
                        .all(char::is_uppercase))
        })
}
pub fn ranked(sources: &[Source], question: &str) -> Vec<Citation> {
    let Ok((scopes, _)) = scoped(sources, question) else {
        return vec![];
    };
    let stop = [
        "the", "and", "what", "why", "how", "does", "this", "that", "with", "from", "are", "can",
        "for", "about", "chapter", "page", "pages", "explain",
    ];
    let terms: HashSet<_> = words(question)
        .into_iter()
        .filter(|term| {
            (term.len() > 2 || term.chars().all(|c| c.is_ascii_digit()))
                && !stop.contains(&term.as_str())
        })
        .collect();
    let mut ranked = Vec::new();
    let mut seen = HashSet::new();
    for scope in &scopes {
        // Paragraphs keep definitions and explanations intact; long paragraphs are
        // split into bounded chunks rather than dropping their later contents.
        let mut offset = 0;
        let mut line = scope.line;
        for paragraph in scope.text.split_inclusive("\n\n") {
            let part = Scope {
                source: scope.source,
                text: paragraph,
                line,
            };
            for chunk in chunks(&part, 1800) {
                if heading_only(chunk.text) {
                    continue;
                }
                let tokens: HashSet<_> = words(chunk.text).into_iter().collect();
                let matches = terms.intersection(&tokens).count();
                if matches > 0 && seen.insert((chunk.source.id, words(chunk.text).join(" "))) {
                    ranked.push((matches, chunk.text.len().min(512), owned(&chunk)));
                }
            }
            offset += paragraph.len();
            line += paragraph.bytes().filter(|b| *b == b'\n').count();
        }
        debug_assert_eq!(offset, scope.text.len());
    }
    ranked.sort_by_key(|(score, length, _)| std::cmp::Reverse((*score, *length)));
    ranked
        .into_iter()
        .take(16)
        .map(|(_, _, citation)| citation)
        .collect()
}
pub fn evidence(citations: &[Citation], indices: &[usize]) -> String {
    indices
        .iter()
        .map(|index| {
            let c = &citations[*index];
            format!(
                "[{}] {} · line {}\n{}\n",
                index + 1,
                c.source_name,
                c.line,
                c.passage
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn cost(text: &str) -> usize {
    serde_json::to_string(text).unwrap().len()
}
pub fn reduction_calls(mut count: usize) -> usize {
    let mut calls = 0;
    while count > 1 {
        calls += count / 3 + usize::from(count % 3 > 1);
        count = count.div_ceil(3);
    }
    calls
}
pub fn plan(book: &Notebook, question: &str, policy: &Policy) -> Result<Plan, String> {
    let (scopes, label) = scoped(&book.sources, question)?;
    let count = scopes
        .iter()
        .map(|scope| scope.source.id)
        .collect::<HashSet<_>>()
        .len();
    let label = if count == 1 {
        format!("{label} · {}", scopes[0].source.name)
    } else {
        format!("{label} · {count} sources")
    };
    let bytes: usize = scopes.iter().map(|scope| scope.text.len()).sum();
    if scopes.iter().all(|scope| scope.text.trim().is_empty()) {
        return Err("The selected sources contain no readable text.".into());
    }
    let full_budget = policy.max_input_bytes.saturating_sub(cost(question) + 1600);
    let map_budget = policy.source_bytes.min(full_budget);
    let mut mode = Mode::Complete;
    let mut citations = Vec::new();
    // Bound work before allocating copies of a potentially very large library.
    if bytes <= full_budget {
        for scope in &scopes {
            citations.extend(chunks(scope, 5000).iter().map(owned));
        }
    }
    let all: Vec<_> = (0..citations.len()).collect();
    if citations.is_empty() || cost(&evidence(&citations, &all)) > full_budget {
        citations.clear();
        if is_summary(question) {
            mode = Mode::Summary;
            if map_budget < 2048 || bytes > map_budget.saturating_mul(policy.max_summary_requests) {
                return Err(format!(
                    "This summary exceeds the configured {}-request budget. Select fewer pages or increase model.usage.max_summary_requests / source_bytes.",
                    policy.max_summary_requests
                ));
            }
            for scope in &scopes {
                citations.extend(
                    chunks(scope, (map_budget.saturating_sub(512) / 2).max(128))
                        .iter()
                        .map(owned),
                );
            }
        } else {
            mode = Mode::Focused;
            for candidate in book.passages(question) {
                citations.push(candidate);
                let indices: Vec<_> = (0..citations.len()).collect();
                if cost(&evidence(&citations, &indices)) > map_budget {
                    citations.pop();
                }
            }
        }
    }
    let mut batches = Vec::new();
    if mode == Mode::Summary {
        let mut batch = Vec::new();
        for index in 0..citations.len() {
            batch.push(index);
            if cost(&evidence(&citations, &batch)) > map_budget {
                batch.pop();
                if batch.is_empty() {
                    return Err("A source chunk cannot fit the summary input budget; increase source_bytes.".into());
                }
                batches.push(std::mem::take(&mut batch));
                batch.push(index);
                if cost(&evidence(&citations, &batch)) > map_budget {
                    return Err("A source chunk cannot fit the summary input budget; increase source_bytes.".into());
                }
            }
        }
        if !batch.is_empty() {
            batches.push(batch);
        }
    } else {
        batches.push((0..citations.len()).collect());
    }
    let requests = batches.len() + reduction_calls(batches.len());
    if requests > policy.max_summary_requests && mode == Mode::Summary {
        return Err(format!(
            "This summary needs up to {requests} AI requests; the configured maximum is {}. Narrow the source selection or adjust max_summary_requests.",
            policy.max_summary_requests
        ));
    }
    let signature = usage::digest(
        format!(
            "{label}:{requests}:{}",
            evidence(&citations, &(0..citations.len()).collect::<Vec<_>>())
        )
        .as_bytes(),
    );
    Ok(Plan {
        mode,
        citations,
        batches,
        requests,
        note_limit: map_budget.saturating_sub(768) / 3,
        label,
        bytes,
        signature,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn book(text: &str) -> Notebook {
        let mut book = Notebook::empty("Test".into());
        book.sources = vec![Source {
            id: 1,
            name: "chapter-1-introduction.pdf".into(),
            content: text.into(),
            selected: true,
        }];
        book
    }
    fn text(plan: &Plan) -> String {
        plan.citations.iter().map(|c| c.passage.as_str()).collect()
    }
    #[test]
    fn small_documents_are_complete_even_when_the_question_has_no_keyword_match() {
        let book = book("## Page 1\n\nBeginning.\n\nRare facts at the very end.");
        let plan = plan(&book, "What can I learn?", &Policy::default()).unwrap();
        assert!(plan.mode == Mode::Complete);
        assert_eq!(text(&plan), book.sources[0].content);
        assert_eq!(plan.requests, 1);
    }
    #[test]
    fn chapter_scope_keeps_running_headers_and_late_sections_but_excludes_other_chapters() {
        let book = book(
            "CONTENTS\n\nChapter 1 Introduction\nChapter 2 Advanced topics\n\nChapter 1 Introduction\n\nBEGINNING explains the basics.\n\nCHAPTER 1. INTRODUCTION\n\nMIDDLE explains the mechanisms.\n\nChapter 1\n\nENDING draws important conclusions.\n\nChapter 2 Advanced topics\n\nFORBIDDEN outside the requested chapter.",
        );
        let plan = plan(&book, "Summarize chapter 1", &Policy::default()).unwrap();
        let covered = text(&plan);
        for term in ["BEGINNING", "MIDDLE", "ENDING"] {
            assert!(covered.contains(term));
        }
        assert!(!covered.contains("FORBIDDEN"));
        assert!(!covered.contains("CONTENTS"));
    }
    #[test]
    fn explicit_pages_and_chapter_filenames_are_scoped_without_confusing_one_with_ten() {
        let pages = book(
            "## Page 1\n\nExclude one.\n\n## Page 2\n\nInclude two.\n\n## Page 3\n\nInclude three.\n\n## Page 4\n\nExclude four.",
        );
        let scoped = plan(&pages, "Summarize pages 2-3", &Policy::default()).unwrap();
        assert!(text(&scoped).contains("Include three"));
        assert!(!text(&scoped).contains("Exclude"));
        assert!(plan(&pages, "Summarize pages 2-5", &Policy::default()).is_err());
        let mut chapters = book("Included body without a chapter heading.");
        chapters.sources.push(Source {
            id: 2,
            name: "chapter-10.pdf".into(),
            content: "Exclude chapter ten body.".into(),
            selected: true,
        });
        assert!(
            !text(&plan(&chapters, "Summarize chapter 1", &Policy::default()).unwrap())
                .contains("Exclude")
        );
        assert!(plan(&chapters, "Summarize chapter 2", &Policy::default()).is_err());
    }
    #[test]
    fn large_summary_covers_every_chunk_in_order_and_respects_request_limits() {
        let content = (0..30)
            .map(|index| {
                format!(
                    "Section {index}\n{}Ending {index}.\n\n",
                    "A meaningful explanation about this section and its evidence. ".repeat(30)
                )
            })
            .collect::<String>();
        let book = book(&content);
        let planned = plan(&book, "Summarize the document", &Policy::default()).unwrap();
        assert!(planned.mode == Mode::Summary);
        assert!(planned.requests > 1 && planned.requests <= 16);
        assert_eq!(text(&planned), content);
        assert_eq!(
            planned
                .batches
                .iter()
                .flatten()
                .copied()
                .collect::<Vec<_>>(),
            (0..planned.citations.len()).collect::<Vec<_>>()
        );
        for batch in &planned.batches {
            assert!(cost(&evidence(&planned.citations, batch)) <= Policy::default().source_bytes);
        }
        assert!(
            plan(
                &book,
                "Summarize the document",
                &Policy {
                    max_summary_requests: 1,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(plan(&super::tests::book("  \n"), "Summarize", &Policy::default()).is_err());
    }
    #[test]
    fn focused_retrieval_rejects_repeated_headers_and_can_select_more_than_four_passages() {
        let content = format!(
            "{}{}",
            "CHAPTER 1. INTRODUCTION\n\n".repeat(2000),
            (0..10)
                .map(|index| format!(
                    "Photosynthesis mechanism {index}: {}\n\n",
                    "Light supports chemical energy conversion. ".repeat(30)
                ))
                .collect::<String>()
        );
        let book = book(&content);
        let planned = plan(&book, "Explain photosynthesis", &Policy::default()).unwrap();
        assert!(planned.mode == Mode::Focused);
        assert!(planned.citations.len() > 4);
        assert!(
            planned
                .citations
                .iter()
                .all(|c| c.passage.contains("Photosynthesis"))
        );
        assert!(
            cost(&evidence(&planned.citations, &planned.batches[0]))
                <= Policy::default().source_bytes
        );
    }
}
