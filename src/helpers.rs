use std::thread;

use iced::alignment::Horizontal;
use iced::futures::channel::oneshot;
use iced::widget::text::{Span, Wrapping};
use iced::widget::{Space, container, image, mouse_area, rich_text, row, span, svg, text};
use iced::{Alignment, Color, Element, Fill, Padding, mouse};

use crate::styles::{self, fonts};
use crate::theme::Colors;
use crate::{Message, ToastLevel, parser::App};
use revmo::search::highlight;

/// Runs blocking work on its own thread. iced's default executor is a small
/// thread pool, so sleeping or doing file IO inside a future would stall it.
pub fn on_thread<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> impl std::future::Future<Output = T> {
    let (tx, rx) = oneshot::channel();
    thread::spawn(move || {
        let _ = tx.send(work());
    });
    async move { rx.await.expect("worker thread panicked") }
}

/// The footer while a toast is showing.
pub fn toast<'a>(msg: &str, level: &ToastLevel, c: Colors) -> Element<'a, Message> {
    let color = match level {
        ToastLevel::Error => c.danger,
    };
    // One line: cut long messages short rather than overflow the footer.
    let room = styles::WINDOW_WIDTH - 2.0 - 2.0 * styles::PAD_X;
    let limit = (room / (styles::SMALL_SIZE * 0.6)).floor() as usize;
    let msg: String = if msg.chars().count() > limit {
        let mut short: String = msg.chars().take(limit - 1).collect();
        short.push('…');
        short
    } else {
        msg.to_string()
    };
    styles::footer_bar(
        c,
        [styles::small(msg).color(color).into()],
        Space::new().into(),
    )
}

/// How a result row is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RowState {
    Normal,
    /// The selection: a bar in the selection color.
    Selected,
    /// The selection while the actions pane is open.
    Marked,
    /// Any other row while the actions pane is open.
    Dimmed,
}

/// One app in the result list. `full` adds the secondary text and shortcut
/// columns, which the actions pane leaves no room for.
pub fn app_row<'a>(
    app: &'a App,
    query: &str,
    state: RowState,
    full: bool,
    c: Colors,
) -> Element<'a, Message> {
    let selected = state == RowState::Selected;
    let (fg, muted) = match state {
        RowState::Selected => (
            c.on_selection,
            Color {
                a: 0.75,
                ..c.on_selection
            },
        ),
        RowState::Dimmed => (c.text_dim, c.text_muted),
        RowState::Normal | RowState::Marked => (c.text, c.text_muted),
    };
    // Dimmed rows stay quiet so the marked row stands out: their matches
    // keep the semibold, so no text moves when the pane opens, but not the
    // highlight color.
    let matched = (!selected && state != RowState::Dimmed).then_some(c.highlight);
    let title_width = if full {
        styles::TITLE_WIDTH
    } else {
        styles::PANE_TITLE_WIDTH
    };

    let mut columns = row![
        container(app_icon(app.icon.as_deref(), styles::ICON_SIZE, muted)).width(styles::TAG_WIDTH),
        container(
            rich_text(highlighted(
                &app.name,
                query,
                true,
                matched,
                styles::chars_fitting(title_width)
            ))
            .size(styles::ROW_SIZE)
            .font(fonts().regular)
            .color(fg)
            .wrapping(Wrapping::None)
        )
        .width(Fill)
        .padding(Padding::ZERO.right(8))
        .clip(true),
    ]
    .align_y(Alignment::Center);

    if full {
        // What the app is, or else what it runs.
        let secondary = match app.generic_name.as_deref() {
            Some(generic) if !generic.trim().is_empty() => highlighted(
                generic,
                query,
                false,
                matched,
                styles::chars_fitting(styles::SECONDARY_WIDTH),
            ),
            _ => highlighted(
                &app.exec.program(),
                "",
                false,
                None,
                styles::chars_fitting(styles::SECONDARY_WIDTH),
            ),
        };
        columns = columns
            .push(
                container(
                    rich_text(secondary)
                        .size(styles::ROW_SIZE)
                        .font(fonts().regular)
                        .color(muted)
                        .wrapping(Wrapping::None),
                )
                .width(styles::SECONDARY_WIDTH)
                .clip(true),
            )
            .push(
                text(if selected { "↲" } else { "" })
                    .size(styles::ROW_SIZE)
                    .font(fonts().medium)
                    .color(fg)
                    .width(styles::SHORTCUT_WIDTH)
                    .align_x(Horizontal::Right),
            );
    }

    let content = container(columns)
        .height(styles::ROW_HEIGHT)
        .width(Fill)
        .align_y(Alignment::Center);
    // Every row has the marker bar, in its left padding, and only a marked
    // row colors it: opening the actions pane then moves nothing.
    let mark = container(Space::new())
        .width(styles::MARK_WIDTH)
        .height(styles::ROW_HEIGHT)
        .style(move |_| match state {
            RowState::Marked => styles::solid(c.accent),
            _ => container::Style::default(),
        });
    let content = content
        .padding(Padding::from([0.0, styles::PAD_X]).left(styles::PAD_X - styles::MARK_WIDTH));
    let body = container(row![mark, content]).style(move |_| match state {
        RowState::Selected => styles::selected(c),
        RowState::Marked => styles::soft_selected(c),
        RowState::Normal | RowState::Dimmed => container::Style::default(),
    });
    mouse_area(body)
        .on_press(Message::RunPress(app.key.clone()))
        .interaction(mouse::Interaction::Pointer)
        .into()
}

/// `content` as spans, the characters matching `query` in semibold and,
/// unless `color` is `None`, in that color. `letters` also marks scattered
/// letters, not just runs (see [`highlight`]). Text longer than `limit`
/// characters is cut short with "…".
pub fn highlighted<'a>(
    content: &str,
    query: &str,
    letters: bool,
    color: Option<Color>,
    limit: usize,
) -> Vec<Span<'a, (), iced::Font>> {
    let marked = highlight(content, query, letters);
    let cut = content.chars().count() > limit;
    let shown = if cut { limit.saturating_sub(1) } else { limit };
    let mut spans = Vec::new();
    let mut run = String::new();
    let mut run_marked = false;
    let mut flush = |run: &mut String, is_marked: bool| {
        if run.is_empty() {
            return;
        }
        let part = span(std::mem::take(run));
        spans.push(if is_marked {
            part.font(fonts().semibold).color_maybe(color)
        } else {
            part
        });
    };
    for (i, ch) in content.chars().take(shown).enumerate() {
        let is_marked = marked.binary_search(&i).is_ok();
        if is_marked != run_marked {
            flush(&mut run, run_marked);
            run_marked = is_marked;
        }
        run.push(ch);
    }
    flush(&mut run, run_marked);
    if cut {
        spans.push(span("…"));
    }
    spans
}

/// The app's icon at `size`, or an `app` tag if it has none.
pub fn app_icon(icon: Option<&str>, size: f32, tag_color: Color) -> Element<'static, Message> {
    match icon {
        Some(p) if p.ends_with(".svg") => svg(p).width(size).height(size).into(),
        Some(p) => image(p).width(size).height(size).into(),
        None => text("app")
            .size(styles::ROW_SIZE)
            .font(fonts().regular)
            .color(tag_color)
            .into(),
    }
}
