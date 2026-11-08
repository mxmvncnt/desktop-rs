//! GitHub-flavored Markdown (pull request descriptions and comments) drawn with gpui elements: paragraphs and headings as
//! styled text (bold, italic, strikethrough, inline code, links that open the browser), fenced code (a ```suggestion block
//! is labelled as one), lists with task boxes, quotes, rules and tables. Raw HTML is left out.

use std::ops::Range;

use gd_ui::Theme;
use gpui::{
    AnyElement, ColorExt, Div, ElementId, FontStyle, FontWeight, HighlightStyle, InteractiveText,
    IntoElement, ParentElement, SharedString, StrikethroughStyle, Styled, StyledText, div,
    prelude::*, px,
};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// `text` as Markdown. `id` names the clickable texts; it must be unique on screen (a comment id).
pub fn markdown(id: &str, text: &str, theme: &Theme) -> Div {
    let mut out = Builder {
        id,
        theme,
        texts: 0,
        stack: vec![Container::Root(Vec::new())],
        inline: Inline::default(),
        code: None,
    };
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    for event in Parser::new_ext(text, options) {
        out.event(event);
    }
    out.flush();
    let Some(Container::Root(blocks)) = out.stack.pop() else {
        unreachable!("the root is never popped by an event");
    };
    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .min_w_0()
        .children(blocks)
}

/// A block being filled, innermost last.
enum Container {
    Root(Vec<AnyElement>),
    Quote(Vec<AnyElement>),
    /// `Some(n)`: numbered from n.
    List(Option<u64>, Vec<AnyElement>),
    Item(Vec<AnyElement>),
    Table(Vec<AnyElement>),
    Row(Vec<AnyElement>, bool),
}

impl Container {
    fn children(&mut self) -> &mut Vec<AnyElement> {
        match self {
            Self::Root(c)
            | Self::Quote(c)
            | Self::List(_, c)
            | Self::Item(c)
            | Self::Table(c)
            | Self::Row(c, _) => c,
        }
    }
}

/// The text of the paragraph, heading, item or cell being read, and its styled ranges.
#[derive(Default)]
struct Inline {
    text: String,
    highlights: Vec<(Range<usize>, HighlightStyle)>,
    links: Vec<(Range<usize>, String)>,
    bold: usize,
    italic: usize,
    strike: usize,
    link: Option<String>,
    heading: Option<HeadingLevel>,
}

struct Builder<'a> {
    id: &'a str,
    theme: &'a Theme,
    /// Clickable texts made so far, for their ids.
    texts: usize,
    stack: Vec<Container>,
    inline: Inline,
    /// A fenced or indented code block being read: its language and text.
    code: Option<(String, String)>,
}

impl Builder<'_> {
    fn event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(t) => match &mut self.code {
                Some((_, code)) => code.push_str(&t),
                None => self.push(&t, None),
            },
            Event::Code(t) => {
                let style = HighlightStyle {
                    background_color: Some(self.theme.element_active),
                    ..Default::default()
                };
                self.push(&t, Some(style));
            }
            Event::SoftBreak => self.push(" ", None),
            Event::HardBreak => self.push("\n", None),
            Event::TaskListMarker(done) => self.push(if done { "☑ " } else { "☐ " }, None),
            Event::Rule => {
                self.flush();
                let rule = div().h(px(1.0)).bg(self.theme.border).into_any_element();
                self.add(rule);
            }
            // Raw HTML, footnotes, math: not drawn.
            _ => {}
        }
    }

    fn start(&mut self, tag: Tag) {
        match tag {
            Tag::Heading { level, .. } => {
                self.flush();
                self.inline.heading = Some(level);
            }
            Tag::Paragraph => self.flush(),
            Tag::BlockQuote(_) => {
                self.flush();
                self.stack.push(Container::Quote(Vec::new()));
            }
            Tag::List(start) => {
                self.flush();
                self.stack.push(Container::List(start, Vec::new()));
            }
            Tag::Item => {
                self.flush();
                self.stack.push(Container::Item(Vec::new()));
            }
            Tag::Table(_) => {
                self.flush();
                self.stack.push(Container::Table(Vec::new()));
            }
            Tag::TableHead => self.stack.push(Container::Row(Vec::new(), true)),
            Tag::TableRow => self.stack.push(Container::Row(Vec::new(), false)),
            Tag::TableCell => self.flush(),
            Tag::CodeBlock(kind) => {
                self.flush();
                let lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                self.code = Some((lang, String::new()));
            }
            Tag::Strong => self.inline.bold += 1,
            Tag::Emphasis => self.inline.italic += 1,
            Tag::Strikethrough => self.inline.strike += 1,
            Tag::Link { dest_url, .. } => self.inline.link = Some(dest_url.to_string()),
            Tag::Image { dest_url, .. } => {
                self.inline.link = Some(dest_url.to_string());
                self.push("🖼 ", None);
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::TableCell => self.flush(),
            TagEnd::Strong => self.inline.bold -= 1,
            TagEnd::Emphasis => self.inline.italic -= 1,
            TagEnd::Strikethrough => self.inline.strike -= 1,
            TagEnd::Link | TagEnd::Image => self.inline.link = None,
            TagEnd::CodeBlock => {
                if let Some((lang, code)) = self.code.take() {
                    let block = self.code_block(&lang, code.trim_end());
                    self.add(block);
                }
            }
            TagEnd::BlockQuote(_)
            | TagEnd::List(_)
            | TagEnd::Item
            | TagEnd::Table
            | TagEnd::TableHead
            | TagEnd::TableRow => {
                self.flush();
                let Some(done) = self.stack.pop() else { return };
                let el = self.close(done);
                self.add(el);
            }
            _ => {}
        }
    }

    /// The element of a finished container.
    fn close(&self, c: Container) -> AnyElement {
        let theme = self.theme;
        match c {
            Container::Quote(children) => div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .pl(px(12.0))
                .border_l_2()
                .border_color(theme.border_strong)
                .text_color(theme.text_muted)
                .children(children)
                .into_any_element(),
            Container::List(start, items) => div()
                .flex()
                .flex_col()
                .gap(px(3.0))
                .children(items.into_iter().enumerate().map(|(ix, item)| {
                    let marker = match start {
                        Some(n) => format!("{}.", n + ix as u64),
                        None => "•".into(),
                    };
                    div()
                        .flex()
                        .gap(px(6.0))
                        .child(
                            div()
                                .flex_none()
                                .min_w(px(14.0))
                                .text_color(theme.text_muted)
                                .child(marker),
                        )
                        .child(div().flex_1().min_w_0().child(item))
                }))
                .into_any_element(),
            Container::Item(children) => div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .children(children)
                .into_any_element(),
            Container::Table(rows) => div()
                .flex()
                .flex_col()
                .border_1()
                .border_color(theme.border)
                .rounded(px(6.0))
                .children(rows)
                .into_any_element(),
            Container::Row(cells, head) => div()
                .flex()
                .border_b_1()
                .border_color(theme.border)
                .when(head, |d| d.font_weight(FontWeight::SEMIBOLD))
                .children(
                    cells
                        .into_iter()
                        .map(|c| div().flex_1().min_w_0().px(px(8.0)).py(px(4.0)).child(c)),
                )
                .into_any_element(),
            Container::Root(children) => div().children(children).into_any_element(),
        }
    }

    fn code_block(&self, lang: &str, code: &str) -> AnyElement {
        let theme = self.theme;
        let suggestion = lang == "suggestion";
        div()
            .flex()
            .flex_col()
            .rounded(px(6.0))
            .border_1()
            .border_color(if suggestion {
                theme.success.opacity(0.5)
            } else {
                theme.border
            })
            .overflow_hidden()
            .children(suggestion.then(|| {
                div()
                    .px(px(10.0))
                    .py(px(4.0))
                    .text_size(px(11.5))
                    .text_color(theme.success)
                    .bg(theme.success.opacity(0.08))
                    .child("Suggested change")
            }))
            .child(
                div()
                    .px(px(10.0))
                    .py(px(8.0))
                    .bg(if suggestion {
                        theme.diff_add_bg
                    } else {
                        theme.element_hover
                    })
                    .font_family(theme.font_mono.clone())
                    .text_size(px(12.0))
                    .line_height(px(18.0))
                    .child(code.to_string()),
            )
            .into_any_element()
    }

    /// Append `text` to the inline being read, with the styles open around it.
    fn push(&mut self, text: &str, extra: Option<HighlightStyle>) {
        let i = &mut self.inline;
        let start = i.text.len();
        i.text.push_str(text);
        let range = start..i.text.len();
        let mut style = extra.unwrap_or_default();
        if i.bold > 0 {
            style.font_weight = Some(FontWeight::SEMIBOLD);
        }
        if i.italic > 0 {
            style.font_style = Some(FontStyle::Italic);
        }
        if i.strike > 0 {
            style.strikethrough = Some(StrikethroughStyle {
                thickness: px(1.0),
                color: None,
            });
        }
        if let Some(url) = &i.link {
            style.color = Some(self.theme.accent);
            i.links.push((range.clone(), url.clone()));
        }
        if style != HighlightStyle::default() {
            i.highlights.push((range, style));
        }
    }

    /// Turn the inline read so far into a text element of the innermost container.
    fn flush(&mut self) {
        let inline = std::mem::take(&mut self.inline);
        let heading = inline.heading;
        if inline.text.trim().is_empty() {
            self.inline.heading = heading.filter(|_| inline.text.is_empty());
            return;
        }
        let styled =
            StyledText::new(SharedString::from(inline.text)).with_highlights(inline.highlights);
        let text = if inline.links.is_empty() {
            styled.into_any_element()
        } else {
            self.texts += 1;
            let id = ElementId::NamedInteger(
                SharedString::from(format!("md-{}", self.id)),
                self.texts as u64,
            );
            let (ranges, urls): (Vec<_>, Vec<_>) = inline.links.into_iter().unzip();
            InteractiveText::new(id, styled)
                .on_click(ranges, move |ix, _, cx| cx.open_url(&urls[ix]))
                .into_any_element()
        };
        let el = match heading {
            Some(level) => div()
                .font_weight(FontWeight::SEMIBOLD)
                .text_size(px(match level {
                    HeadingLevel::H1 => 18.0,
                    HeadingLevel::H2 => 16.0,
                    HeadingLevel::H3 => 14.5,
                    _ => 13.0,
                }))
                .child(text)
                .into_any_element(),
            None => div().child(text).into_any_element(),
        };
        self.add(el);
    }

    fn add(&mut self, el: AnyElement) {
        if let Some(top) = self.stack.last_mut() {
            top.children().push(el);
        }
    }
}
