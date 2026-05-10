use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub fn render(md: &str) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();

    // Style stack — each entry is a Style modifier active at that nesting level
    let mut style_stack: Vec<Style> = vec![Style::default()];

    // Track list state: (ordered, current_index)
    let mut list_stack: Vec<Option<u64>> = Vec::new();
    // Track if we're inside a code block
    let mut in_code_block = false;
    // Track blockquote depth
    let mut blockquote_depth: usize = 0;

    let opts = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
    let parser = Parser::new_ext(md, opts);

    let flush = |spans: &mut Vec<Span<'static>>,
                 lines: &mut Vec<Line<'static>>,
                 blockquote_depth: usize| {
        let mut all_spans = Vec::new();
        if blockquote_depth > 0 {
            all_spans.push(Span::styled(
                "│ ".repeat(blockquote_depth),
                Style::default().fg(Color::DarkGray),
            ));
        }
        all_spans.extend(spans.drain(..));
        lines.push(Line::from(all_spans));
    };

    for event in parser {
        match event {
            // ── Block open ──────────────────────────────────────────────────
            Event::Start(Tag::Heading { level, .. }) => {
                flush(&mut current_spans, &mut lines, blockquote_depth);
                let (color, prefix) = heading_style(level);
                current_spans.push(Span::styled(
                    prefix,
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ));
                style_stack.push(Style::default().fg(color).add_modifier(Modifier::BOLD));
            }
            Event::End(TagEnd::Heading(_)) => {
                style_stack.pop();
                flush(&mut current_spans, &mut lines, blockquote_depth);
                lines.push(Line::from(vec![])); // blank after heading
            }

            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                flush(&mut current_spans, &mut lines, blockquote_depth);
                lines.push(Line::from(vec![]));
            }

            Event::Start(Tag::BlockQuote(_)) => {
                blockquote_depth += 1;
                style_stack.push(Style::default().fg(Color::DarkGray));
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                blockquote_depth = blockquote_depth.saturating_sub(1);
                style_stack.pop();
                flush(&mut current_spans, &mut lines, blockquote_depth);
            }

            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                flush(&mut current_spans, &mut lines, blockquote_depth);
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                lines.push(Line::from(vec![]));
            }

            Event::Start(Tag::List(start)) => {
                list_stack.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
                lines.push(Line::from(vec![]));
            }

            Event::Start(Tag::Item) => {
                flush(&mut current_spans, &mut lines, blockquote_depth);
                let indent = "  ".repeat(list_stack.len().saturating_sub(1));
                let bullet = match list_stack.last() {
                    Some(Some(n)) => {
                        let idx = *n;
                        // bump the counter on the stack
                        if let Some(Some(v)) = list_stack.last_mut() {
                            *v += 1;
                        }
                        format!("{}{}. ", indent, idx)
                    }
                    _ => format!("{}• ", indent),
                };
                current_spans.push(Span::styled(bullet, Style::default().fg(Color::Yellow)));
            }
            Event::End(TagEnd::Item) => {
                flush(&mut current_spans, &mut lines, blockquote_depth);
            }

            Event::Start(Tag::Strong) => {
                let base = current_style(&style_stack);
                style_stack.push(base.add_modifier(Modifier::BOLD));
            }
            Event::End(TagEnd::Strong) => {
                style_stack.pop();
            }

            Event::Start(Tag::Emphasis) => {
                let base = current_style(&style_stack);
                style_stack.push(base.add_modifier(Modifier::ITALIC));
            }
            Event::End(TagEnd::Emphasis) => {
                style_stack.pop();
            }

            Event::Start(Tag::Strikethrough) => {
                let base = current_style(&style_stack);
                style_stack.push(base.add_modifier(Modifier::CROSSED_OUT));
            }
            Event::End(TagEnd::Strikethrough) => {
                style_stack.pop();
            }

            Event::Start(Tag::Link { dest_url, .. }) => {
                style_stack.push(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::UNDERLINED),
                );
                // stash url for End
                current_spans.push(Span::raw("__LINK_URL__".to_string()));
                let _ = dest_url; // used at End via a different approach below
            }
            Event::End(TagEnd::Link) => {
                style_stack.pop();
            }

            Event::Start(Tag::Image { .. }) => {
                // alt text comes as Text events inside; we push a placeholder
                // and replace it at End
                current_spans.push(Span::raw("__IMAGE_START__"));
            }
            Event::End(TagEnd::Image) => {
                // collect any text spans since __IMAGE_START__ as alt text
                let start = current_spans
                    .iter()
                    .rposition(|s| s.content == "__IMAGE_START__")
                    .unwrap_or(0);
                let alt: String = current_spans[start + 1..]
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect();
                current_spans.truncate(start);
                let label = if alt.is_empty() {
                    "[image]".to_string()
                } else {
                    format!("[image: {}]", alt)
                };
                current_spans.push(Span::styled(
                    label,
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                ));
            }

            // ── Table (best-effort) ─────────────────────────────────────────
            Event::Start(Tag::Table(_)) => {
                flush(&mut current_spans, &mut lines, blockquote_depth);
            }
            Event::End(TagEnd::Table) => {
                flush(&mut current_spans, &mut lines, blockquote_depth);
                lines.push(Line::from(vec![]));
            }
            Event::Start(Tag::TableHead) => {}
            Event::End(TagEnd::TableHead) => {
                flush(&mut current_spans, &mut lines, blockquote_depth);
                // separator line
                lines.push(Line::from(Span::styled(
                    "─".repeat(40),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            Event::Start(Tag::TableRow) => {}
            Event::End(TagEnd::TableRow) => {
                flush(&mut current_spans, &mut lines, blockquote_depth);
            }
            Event::Start(Tag::TableCell) => {
                current_spans.push(Span::styled("│ ", Style::default().fg(Color::DarkGray)));
            }
            Event::End(TagEnd::TableCell) => {
                current_spans.push(Span::styled(" ", Style::default()));
            }

            // ── Inline ──────────────────────────────────────────────────────
            Event::Text(text) => {
                let style = current_style(&style_stack);
                if in_code_block {
                    for line in text.lines() {
                        current_spans.push(Span::styled(
                            line.to_string(),
                            Style::default().fg(Color::Green),
                        ));
                        flush(&mut current_spans, &mut lines, blockquote_depth);
                    }
                } else {
                    // Remove the link url placeholder if present
                    current_spans.retain(|s| s.content != "__LINK_URL__");
                    current_spans.push(Span::styled(text.to_string(), style));
                }
            }

            Event::Code(text) => {
                current_spans.push(Span::styled(
                    text.to_string(),
                    Style::default().fg(Color::Green),
                ));
            }

            Event::SoftBreak => {
                current_spans.push(Span::raw(" "));
            }
            Event::HardBreak => {
                flush(&mut current_spans, &mut lines, blockquote_depth);
            }

            Event::Rule => {
                flush(&mut current_spans, &mut lines, blockquote_depth);
                lines.push(Line::from(Span::styled(
                    "─".repeat(60),
                    Style::default().fg(Color::DarkGray),
                )));
                lines.push(Line::from(vec![]));
            }

            Event::Html(_) | Event::InlineHtml(_) => {
                // skip raw HTML
            }

            _ => {}
        }
    }

    // flush anything remaining
    if !current_spans.is_empty() {
        flush(&mut current_spans, &mut lines, blockquote_depth);
    }

    lines
}

fn current_style(stack: &[Style]) -> Style {
    stack.last().copied().unwrap_or_default()
}

fn heading_style(level: HeadingLevel) -> (Color, String) {
    match level {
        HeadingLevel::H1 => (Color::Cyan, "█ ".to_string()),
        HeadingLevel::H2 => (Color::Blue, "▌ ".to_string()),
        HeadingLevel::H3 => (Color::Magenta, "▎ ".to_string()),
        HeadingLevel::H4 => (Color::Yellow, "  ".to_string()),
        HeadingLevel::H5 | HeadingLevel::H6 => (Color::DarkGray, "  ".to_string()),
    }
}
