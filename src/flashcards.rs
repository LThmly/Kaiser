use crate::model::Citation;
use serde::Deserialize;

#[derive(Clone)]
pub struct Request {
    pub count: usize,
    pub scope: String,
}
impl Request {
    pub fn parse(input: &str) -> Result<Self, String> {
        let input = input.trim();
        let (first, rest) = input.split_once(char::is_whitespace).unwrap_or((input, ""));
        let (count, scope) = if first.chars().all(|c| c.is_ascii_digit()) && !first.is_empty() {
            (
                first
                    .parse::<usize>()
                    .map_err(|_| "Use a card count from 1 to 20")?,
                rest.trim(),
            )
        } else {
            (10, input)
        };
        if !(1..=20).contains(&count) {
            return Err(
                "Use :generate-cards [1–20] [scope], for example :generate-cards 10 chapter 1"
                    .into(),
            );
        }
        Ok(Self {
            count,
            scope: scope.into(),
        })
    }
    pub fn question(&self) -> String {
        format!(
            "Generate up to {} flashcards covering the key ideas in {}. Each card should test one important concept with a clear question and concise, self-contained answer. Avoid trivia, ambiguous questions, and duplicates. Use fewer cards if the evidence is insufficient. Return only a JSON array of objects: {{\"front\":\"question\",\"back\":\"answer\",\"source\":1}}. The source is the original [n] evidence ID supporting the answer, not a file number. Every card must have a valid source. No Markdown fences or text outside the array.",
            self.count,
            if self.scope.is_empty() {
                "the selected sources"
            } else {
                &self.scope
            }
        )
    }
}

pub struct Generated {
    pub front: String,
    pub back: String,
    pub citation: Citation,
}
pub fn normalized(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
pub fn parse(text: &str, count: usize, citations: &[Citation]) -> Result<Vec<Generated>, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Item {
        front: String,
        back: String,
        source: usize,
    }
    if text.len() > 65536 {
        return Err(
            "Generated flashcards exceeded the response size limit; no cards were added.".into(),
        );
    }
    let text = text.trim();
    let text = text
        .strip_prefix("```json")
        .or_else(|| text.strip_prefix("```"))
        .and_then(|s| s.trim_end().strip_suffix("```"))
        .unwrap_or(text)
        .trim();
    let items: Vec<Item> = serde_json::from_str(text).map_err(
        |_| "The model returned invalid flashcards. No cards were added; try a smaller batch.",
    )?;
    if items.is_empty() || items.len() > count {
        return Err(
            "The model returned an invalid number of flashcards; no cards were added.".into(),
        );
    }
    let mut result: Vec<Generated> = vec![];
    for item in items {
        let front = item.front.trim().to_owned();
        let back = item.back.trim().to_owned();
        let citation = item
            .source
            .checked_sub(1)
            .and_then(|i| citations.get(i))
            .ok_or("A generated card had an invalid source citation; no cards were added.")?;
        if front.is_empty() || back.is_empty() || front.len() > 1000 || back.len() > 6000 {
            return Err(
                "A generated card had empty or oversized fields; no cards were added.".into(),
            );
        }
        if !result
            .iter()
            .any(|card| normalized(&card.front) == normalized(&front))
        {
            result.push(Generated {
                front,
                back,
                citation: citation.clone(),
            });
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requests_validate_count_and_preserve_scope() {
        assert_eq!(Request::parse("").unwrap().count, 10);
        assert_eq!(Request::parse("5 pages 2–4").unwrap().scope, "pages 2–4");
        assert_eq!(Request::parse("chapter 1").unwrap().count, 10);
        for value in ["0", "21", "9999999999999999999999999"] {
            assert!(Request::parse(value).is_err());
        }
    }
    #[test]
    fn parsing_is_atomic_validates_citations_and_deduplicates() {
        let citations = vec![Citation {
            source_id: 42,
            source_name: "notes.pdf".into(),
            line: 4,
            passage: "Evidence".into(),
        }];
        let cards = parse(r#"[{"front":" What is recall? ","back":"Retrieving knowledge.","source":1},{"front":"what  is recall?","back":"Duplicate","source":1}]"#, 3, &citations).unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].citation.source_id, 42);
        for value in [
            r#"[{"front":"Q","back":"A","source":1},{"front":"Bad","back":"A","source":2}]"#,
            r#"[{"front":"Q","back":"A","source":0}]"#,
            r#"[{"front":"Q","back":"","source":1}]"#,
            "[]",
            "not JSON",
        ] {
            assert!(parse(value, 3, &citations).is_err());
        }
    }
}
