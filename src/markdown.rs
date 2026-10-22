use crate::config::Theme;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthChar;

pub fn render(text: &str, width: u16, theme: Theme) -> Vec<Line<'static>> {
    let mut lines = vec![];
    let mut row: Vec<Span<'static>> = vec![];
    let mut styles = vec![Style::default().fg(theme.foreground)];
    let mut lists: Vec<Option<u64>> = vec![];
    let mut quote = 0;
    let flush = |row: &mut Vec<Span<'static>>, lines: &mut Vec<Line<'static>>| {
        if !row.is_empty() {
            lines.push(Line::from(std::mem::take(row)));
        }
    };
    for event in Parser::new_ext(
        text,
        Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH,
    ) {
        let current = *styles.last().unwrap();
        match event {
            Event::Start(tag) => {
                let style = match &tag {
                    Tag::Heading { .. } => {
                        flush(&mut row, &mut lines);
                        current.fg(theme.accent).add_modifier(Modifier::BOLD)
                    }
                    Tag::Strong => current.add_modifier(Modifier::BOLD),
                    Tag::Emphasis => current.add_modifier(Modifier::ITALIC),
                    Tag::Strikethrough => current.add_modifier(Modifier::CROSSED_OUT),
                    Tag::Link { .. } => {
                        current.fg(theme.success).add_modifier(Modifier::UNDERLINED)
                    }
                    Tag::CodeBlock(_) => {
                        flush(&mut row, &mut lines);
                        current.fg(theme.success).bg(theme.selection)
                    }
                    Tag::BlockQuote(_) => {
                        flush(&mut row, &mut lines);
                        quote += 1;
                        current.fg(theme.muted)
                    }
                    Tag::List(start) => {
                        flush(&mut row, &mut lines);
                        lists.push(*start);
                        current
                    }
                    Tag::Item => {
                        flush(&mut row, &mut lines);
                        let indent = "  ".repeat(lists.len().saturating_sub(1));
                        let prefix = if let Some(Some(n)) = lists.last_mut() {
                            let prefix = format!("{n}. ");
                            *n += 1;
                            prefix
                        } else {
                            "• ".into()
                        };
                        row.push(Span::styled(
                            format!("{indent}{prefix}"),
                            Style::default().fg(theme.muted),
                        ));
                        current
                    }
                    Tag::TableHead => {
                        flush(&mut row, &mut lines);
                        current.add_modifier(Modifier::BOLD)
                    }
                    Tag::Paragraph => {
                        if quote > 0 {
                            row.push(Span::styled("│ ".repeat(quote), current));
                        }
                        current
                    }
                    _ => current,
                };
                styles.push(style);
            }
            Event::End(tag) => {
                match tag {
                    TagEnd::Paragraph
                    | TagEnd::Heading(_)
                    | TagEnd::CodeBlock
                    | TagEnd::Item
                    | TagEnd::TableRow
                    | TagEnd::TableHead => flush(&mut row, &mut lines),
                    TagEnd::List(_) => {
                        lists.pop();
                        flush(&mut row, &mut lines);
                    }
                    TagEnd::BlockQuote(_) => {
                        quote = quote.saturating_sub(1);
                        flush(&mut row, &mut lines);
                    }
                    TagEnd::TableCell => {
                        row.push(Span::styled(" │ ", Style::default().fg(theme.border)))
                    }
                    _ => {}
                }
                if matches!(
                    tag,
                    TagEnd::Paragraph
                        | TagEnd::Heading(_)
                        | TagEnd::CodeBlock
                        | TagEnd::List(_)
                        | TagEnd::Table
                ) && lists.is_empty()
                {
                    lines.push(Line::raw(""));
                }
                if styles.len() > 1 {
                    styles.pop();
                }
            }
            Event::Text(value) => {
                for (i, part) in value.split('\n').enumerate() {
                    if i > 0 {
                        lines.push(Line::from(std::mem::take(&mut row)));
                    }
                    if !part.is_empty() {
                        row.push(Span::styled(part.to_owned(), current));
                    }
                }
            }
            Event::Code(value) => row.push(Span::styled(
                value.into_string(),
                current.fg(theme.success).bg(theme.selection),
            )),
            Event::SoftBreak => row.push(Span::raw(" ")),
            Event::HardBreak => lines.push(Line::from(std::mem::take(&mut row))),
            Event::Rule => {
                flush(&mut row, &mut lines);
                lines.push(Line::styled(
                    "─".repeat(width as usize),
                    Style::default().fg(theme.border),
                ));
            }
            Event::TaskListMarker(done) => row.push(Span::styled(
                if done { "☑ " } else { "☐ " },
                current.fg(theme.success),
            )),
            Event::Html(value) | Event::InlineHtml(value) => {
                row.push(Span::styled(value.into_string(), current.fg(theme.muted)))
            }
            _ => {}
        }
    }
    flush(&mut row, &mut lines);
    while lines.last().is_some_and(|line| line.spans.is_empty()) {
        lines.pop();
    }
    // Wrap styled spans without dropping emphasis, links, or Unicode cell widths.
    let mut wrapped = vec![];
    for line in lines {
        let mut out = vec![];
        let mut used = 0;
        for span in line.spans {
            let mut part = String::new();
            for c in span.content.chars() {
                let cw = c.width().unwrap_or(0);
                if used + cw > usize::from(width.max(1)) && used > 0 {
                    if !part.is_empty() {
                        out.push(Span::styled(std::mem::take(&mut part), span.style));
                    }
                    wrapped.push(Line::from(std::mem::take(&mut out)));
                    used = 0;
                }
                part.push(c);
                used += cw;
            }
            if !part.is_empty() {
                out.push(Span::styled(part, span.style));
            }
        }
        wrapped.push(Line::from(out));
    }
    wrapped
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn markdown_preserves_semantic_styles_and_wraps_unicode() {
        let theme = Theme::default();
        let lines = render(
            "# Heading\n\n**bold** *italic* `code` [link](https://example.com)\n\n- [x] done\n\n> quote\n\n```rust\nlet x = 1;\n```\n\n| A | B |\n|---|---|\n| 中 | 文 |",
            30,
            theme,
        );
        let text = lines
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!text.contains("**"));
        assert!(!text.contains("```"));
        assert!(text.contains("Heading"));
        assert!(text.contains("☑ done"));
        assert!(text.contains("│ quote"));
        assert!(
            lines
                .iter()
                .flat_map(|l| &l.spans)
                .any(|s| s.content == "bold" && s.style.add_modifier.contains(Modifier::BOLD))
        );
        assert!(lines.iter().all(|line| line.width() <= 30));
    }
}
