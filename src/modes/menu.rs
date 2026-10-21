//! The line-picking mode: a list of text lines to filter and pick from,
//! instead of apps. `revmo menu` fills it from stdin and prints the pick,
//! like dmenu; clipboard mode fills it from the clipboard manager (see
//! [`super::clipboard`]).

use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicI32, Ordering};

use iced::alignment::Horizontal;
use iced::futures::{SinkExt, StreamExt};
use iced::widget::text::Wrapping;
use iced::widget::{Space, column, container, mouse_area, rich_text, row, text};
use iced::{Alignment, Element, Fill, Padding, mouse};
use revmo::search::lines::LineSearch;

use super::columns::Columns;
use crate::helpers::highlighted;
use crate::styles::{self, fonts};
use crate::theme::Colors;
use crate::{Message, Revmo};

/// `revmo menu`'s exit status: 1 (like dmenu) unless something was printed.
pub static EXIT_CODE: AtomicI32 = AtomicI32::new(1);

/// Lines read from stdin are sent on in batches of at most this many.
const BATCH_LINES: usize = 1024;
/// How much of stdin is read at once.
const READ_BUFFER: usize = 64 * 1024;

/// Only this much of a line is drawn; the row cuts it shorter anyway.
const SHOWN_CHARS: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    /// `revmo menu`: lines from stdin, the pick printed to stdout.
    Stdin,
    /// Clipboard history, the pick handed to the clipboard manager.
    Clipboard,
}

/// One line to pick from.
#[derive(Debug, Clone)]
pub struct Line {
    text: String,
    /// What's shown and searched, when only some columns are (see
    /// [`Columns`]); the whole line is still what's picked.
    shown: Option<String>,
    /// The bytes as read, when they aren't valid UTF-8 (`text` has them
    /// repaired for showing), so the pick is printed exactly as it came.
    raw: Option<Vec<u8>>,
}

impl Line {
    /// A line read from stdin, without its line ending (`\n` or `\r\n`).
    fn from_bytes(bytes: &[u8]) -> Self {
        let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
        let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
        match std::str::from_utf8(bytes) {
            Ok(text) => Line {
                text: text.to_string(),
                shown: None,
                raw: None,
            },
            Err(_) => Line {
                text: String::from_utf8_lossy(bytes).into_owned(),
                shown: None,
                raw: Some(bytes.to_vec()),
            },
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// The part of the line that's shown and searched.
    pub fn shown(&self) -> &str {
        self.shown.as_deref().unwrap_or(&self.text)
    }

    pub fn bytes(&self) -> &[u8] {
        self.raw.as_deref().unwrap_or(self.text.as_bytes())
    }
}

impl AsRef<str> for Line {
    fn as_ref(&self) -> &str {
        self.shown()
    }
}

pub struct Menu {
    pub kind: Kind,
    /// Shown before the input instead of "›".
    pub prompt: Option<String>,
    /// Which columns of the lines to show; all of each line if `None`.
    columns: Option<Columns>,
    lines: Vec<Line>,
    search: LineSearch,
    /// Position of the selected line in the list.
    pub selected: usize,
    /// Until all the lines are in.
    pub loading: bool,
    /// Why there are no lines, if getting them failed.
    pub error: Option<String>,
    /// Lines printed with ctrl+enter, marked in the list.
    picked: Vec<usize>,
    /// While the clipboard manager copies the pick, so it isn't asked twice.
    pub picking: bool,
}

impl Menu {
    pub fn new(kind: Kind, prompt: Option<String>, columns: Option<Columns>) -> Self {
        Menu {
            kind,
            prompt,
            columns,
            lines: Vec::new(),
            search: LineSearch::new(),
            selected: 0,
            loading: true,
            error: None,
            picked: Vec::new(),
            picking: false,
        }
    }

    /// Adds lines after the ones there, keeping the selection.
    pub fn push(&mut self, mut lines: Vec<Line>) {
        if let Some(columns) = &self.columns {
            for line in &mut lines {
                let shown = columns.shown(&line.text);
                // Kept only when it's less than the line.
                if shown.len() != line.text.len() {
                    line.shown = Some(shown.into_owned());
                }
            }
        }
        let at = self.search.locate(self.selected);
        let added = self.search.push(&lines);
        self.lines.extend(lines);
        // New lines join the end of their group, so the selected line only
        // moves down by those that joined a group above its own.
        if let Some((group, _)) = at {
            self.selected += added[..group].iter().sum::<usize>();
        }
    }

    /// Replaces the lines.
    pub fn set_lines(&mut self, lines: Vec<String>) {
        self.lines.clear();
        self.search.clear();
        self.picked.clear();
        self.selected = 0;
        self.push(
            lines
                .into_iter()
                .map(|text| Line {
                    text,
                    shown: None,
                    raw: None,
                })
                .collect(),
        );
    }

    pub fn set_query(&mut self, query: &str) {
        self.search.set_query(query);
        self.selected = 0;
    }

    /// How many lines match the query.
    pub fn len(&self) -> usize {
        self.search.len()
    }

    pub fn selected_line(&self) -> Option<&Line> {
        Some(&self.lines[self.search.get(self.selected)?])
    }

    /// Selects the line with index `line`; false if it isn't listed.
    pub fn select_line(&mut self, line: usize) -> bool {
        match self.search.position_of(line) {
            Some(position) => {
                self.selected = position;
                true
            }
            None => false,
        }
    }

    pub fn select_down(&mut self) {
        if self.selected + 1 < self.len() {
            self.selected += 1;
        }
    }

    pub fn select_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// Marks the selected line as printed.
    pub fn mark_picked(&mut self) {
        if let Some(i) = self.search.get(self.selected)
            && !self.picked.contains(&i)
        {
            self.picked.push(i);
        }
    }
}

/// Prints `line` for `revmo menu` and sets the exit status to success.
/// Returns false if stdout is gone (the reader quit), when there's no
/// point going on.
pub fn print(line: &[u8]) -> bool {
    let mut out = io::stdout().lock();
    let written = out
        .write_all(line)
        .and_then(|_| out.write_all(b"\n"))
        .and_then(|_| out.flush());
    match written {
        Ok(()) => {
            EXIT_CODE.store(0, Ordering::Relaxed);
            true
        }
        Err(e) => {
            if e.kind() != io::ErrorKind::BrokenPipe {
                eprintln!("revmo: can't print: {e}");
            }
            false
        }
    }
}

/// Reads stdin line by line on its own thread, sending the lines on in
/// batches, then `MenuLines(_, true)` at the end.
pub fn read_stdin() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(4, async move |mut output| {
        let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
        std::thread::spawn(move || {
            let mut reader = io::BufReader::with_capacity(READ_BUFFER, io::stdin().lock());
            let mut batch = Vec::new();
            let mut bytes = Vec::new();
            loop {
                bytes.clear();
                match reader.read_until(b'\n', &mut bytes) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => batch.push(Line::from_bytes(&bytes)),
                }
                // Sent before a read that may wait for more, so every line
                // shows as soon as it's in; a fast producer fills batches.
                if batch.len() >= BATCH_LINES || reader.buffer().is_empty() {
                    let lines = std::mem::take(&mut batch);
                    if tx.unbounded_send((lines, false)).is_err() {
                        return;
                    }
                }
            }
            let _ = tx.unbounded_send((batch, true));
        });
        while let Some((lines, done)) = rx.next().await {
            if output.send(Message::MenuLines(lines, done)).await.is_err() {
                break;
            }
        }
        std::future::pending::<()>().await
    })
}

/// The current page of lines, or why there are none.
pub fn view<'a>(state: &'a Revmo, menu: &'a Menu, query: &str) -> Element<'a, Message> {
    let c = state.colors;
    let start = menu.selected / state.batch_size * state.batch_size;
    let end = (start + state.batch_size).min(menu.len());
    let rows = (start..end).filter_map(|position| {
        let line = menu.search.get(position)?;
        Some(line_row(
            menu.lines[line].shown(),
            query,
            position == menu.selected,
            menu.picked.contains(&line),
            line,
            c,
        ))
    });
    let mut list = column(rows);
    let note = match (&menu.error, menu.len() == 0) {
        (Some(error), _) => Some(error.as_str()),
        (None, false) => None,
        (None, true) if menu.loading => Some("loading…"),
        (None, true) if menu.lines.is_empty() => Some(match menu.kind {
            Kind::Stdin => "nothing to pick: stdin was empty",
            Kind::Clipboard => "the clipboard history is empty",
        }),
        (None, true) => Some("no lines match"),
    };
    if let Some(note) = note {
        let color = match menu.error {
            Some(_) => c.danger,
            None => c.text_muted,
        };
        list = list.push(
            container(
                text(note)
                    .size(styles::ROW_SIZE)
                    .font(fonts().regular)
                    .color(color),
            )
            .padding([0.0, styles::PAD_X])
            .height(styles::ROW_HEIGHT)
            .align_y(Alignment::Center),
        );
    }
    container(list)
        .padding(Padding::from([styles::LIST_PADDING, 0.0]))
        .width(Fill)
        .into()
}

/// One line of the list, the one with index `index`. Lines printed with
/// ctrl+enter get the accent bar.
fn line_row<'a>(
    line: &str,
    query: &str,
    selected: bool,
    picked: bool,
    index: usize,
    c: Colors,
) -> Element<'a, Message> {
    let fg = if selected { c.on_selection } else { c.text };
    let matched = (!selected).then_some(c.highlight);
    // Tabs and other control characters would draw as boxes.
    let shown: String = line
        .chars()
        .take(SHOWN_CHARS)
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect();
    let columns = row![
        container(
            rich_text(highlighted(
                &shown,
                query,
                false,
                matched,
                styles::chars_fitting(styles::LINE_WIDTH),
            ))
            .size(styles::ROW_SIZE)
            .font(fonts().regular)
            .color(fg)
            .wrapping(Wrapping::None)
        )
        .width(Fill)
        .padding(Padding::ZERO.right(8))
        .clip(true),
        text(if selected { "↲" } else { "" })
            .size(styles::ROW_SIZE)
            .font(fonts().medium)
            .color(fg)
            .width(styles::SHORTCUT_WIDTH)
            .align_x(Horizontal::Right),
    ]
    .align_y(Alignment::Center);

    let content = container(columns)
        .height(styles::ROW_HEIGHT)
        .width(Fill)
        .align_y(Alignment::Center)
        .padding(Padding::from([0.0, styles::PAD_X]).left(styles::PAD_X - styles::MARK_WIDTH));
    let mark = container(Space::new())
        .width(styles::MARK_WIDTH)
        .height(styles::ROW_HEIGHT)
        .style(move |_| match picked {
            true => styles::solid(c.accent),
            false => container::Style::default(),
        });
    let body = container(row![mark, content]).style(move |_| match selected {
        true => styles::selected(c),
        false => container::Style::default(),
    });
    mouse_area(body)
        .on_press(Message::MenuPress(index))
        .interaction(mouse::Interaction::Pointer)
        .into()
}

/// The footer's hints for this mode, and `position` (see `Revmo::position`).
pub fn footer<'a>(menu: &Menu, position: String, c: Colors) -> Element<'a, Message> {
    let hint = |s| styles::hint(c, s);
    match menu.kind {
        Kind::Stdin => styles::footer_bar(
            c,
            [
                hint("↲ pick"),
                hint("ctrl+↲ more"),
                hint("shift+↲ typed"),
                hint("tab fill"),
                hint("esc close"),
            ],
            // No room for the mode's name beside these hints.
            styles::small(position).color(c.text_muted).into(),
        ),
        Kind::Clipboard => styles::footer_bar(
            c,
            [
                hint("↑↓ move"),
                hint("↲ copy"),
                hint("⌫ apps"),
                hint("esc close"),
            ],
            styles::small(format!("clipboard · {position}"))
                .color(c.text_muted)
                .into(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(list: &[&str]) -> Vec<Line> {
        list.iter()
            .map(|s| Line::from_bytes(s.as_bytes()))
            .collect()
    }

    fn selected(menu: &Menu) -> Option<&str> {
        menu.selected_line().map(Line::text)
    }

    #[test]
    fn keeps_the_selected_line_when_more_arrive() {
        let mut menu = Menu::new(Kind::Stdin, None, None);
        menu.push(lines(&["xx foo", "bar foo"]));
        menu.set_query("foo");
        menu.select_down();
        assert_eq!(selected(&menu), Some("bar foo"));
        // "foo" ranks first, above both, yet "bar foo" stays selected.
        menu.push(lines(&["foo", "zz foo"]));
        assert_eq!(menu.search.get(0), Some(2));
        assert_eq!(selected(&menu), Some("bar foo"));
    }

    #[test]
    fn selection_stays_in_the_list() {
        let mut menu = Menu::new(Kind::Clipboard, None, None);
        menu.set_lines(vec!["a".into(), "b".into()]);
        menu.select_up();
        assert_eq!(selected(&menu), Some("a"));
        menu.select_down();
        menu.select_down();
        assert_eq!(selected(&menu), Some("b"));
        assert!(menu.select_line(0));
        assert_eq!(selected(&menu), Some("a"));
        menu.set_query("zzz");
        assert_eq!(selected(&menu), None);
        assert!(!menu.select_line(0));
        menu.select_down();
        assert_eq!(menu.selected, 0);
    }

    #[test]
    fn shows_and_searches_only_the_columns() {
        let columns = Columns::parse("2..", None).unwrap();
        let mut menu = Menu::new(Kind::Clipboard, None, Some(columns));
        menu.set_lines(vec![
            "41\tgit push".into(),
            "42\tcargo".into(),
            "plain".into(),
        ]);
        assert_eq!(menu.selected_line().map(Line::shown), Some("git push"));
        // The hidden ids aren't searched...
        menu.set_query("42");
        assert_eq!(menu.len(), 0);
        menu.set_query("cargo");
        // ...but the pick is still the whole line.
        assert_eq!(menu.selected_line().map(Line::text), Some("42\tcargo"));
        menu.set_query("plain");
        assert_eq!(menu.selected_line().map(Line::shown), Some("plain"));
    }

    #[test]
    fn lines_keep_their_bytes_but_not_their_endings() {
        let line = Line::from_bytes(b"caf\xe9\r\n");
        assert_eq!(line.text(), "caf\u{fffd}");
        assert_eq!(line.bytes(), b"caf\xe9");
        let line = Line::from_bytes(b"plain\n");
        assert_eq!((line.text(), line.bytes()), ("plain", &b"plain"[..]));
        assert_eq!(Line::from_bytes(b"a\rb").text(), "a\rb");
    }
}
