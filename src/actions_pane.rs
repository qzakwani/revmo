//! The actions pane (→ on a result): the list on the left, and on the right
//! the selected app's entries: the app itself and its desktop actions.

use iced::widget::text::Wrapping;
use iced::widget::{column, container, mouse_area, row, text};
use iced::{Alignment, Element, Fill, Padding, mouse};

use crate::styles::{self, fonts};
use crate::{Message, Revmo, helpers};

const DETAIL_ICON: f32 = 24.0;

/// `line` cut to `limit` characters, ending in "…" if it was longer.
fn clip(line: &str, limit: usize) -> String {
    if line.chars().count() <= limit {
        return line.to_string();
    }
    let mut short: String = line.chars().take(limit.saturating_sub(1)).collect();
    short.push('…');
    short
}

pub fn view<'a>(state: &'a Revmo, list: Element<'a, Message>) -> Element<'a, Message> {
    let c = state.colors;
    let selected = state.pane.unwrap_or(0);

    let items = state
        .pane_items()
        .into_iter()
        .enumerate()
        .map(|(i, index)| {
            let is_selected = i == selected;
            let app = &state.apps[index];
            let mut nouns: Vec<&str> = app.arg_codes().iter().map(|c| c.noun()).collect();
            nouns.dedup();
            let codes = (!nouns.is_empty()).then(|| format!("→ {}", nouns.join(" ")));
            let label = app.action.clone().unwrap_or_else(|| "open".into());
            let message = Message::RunPress(app.key.clone());
            let (fg, muted) = if is_selected {
                (c.on_selection, c.on_selection)
            } else {
                (c.text, c.text_muted)
            };
            // The selected row shows only ↲, like the main list.
            let hint = codes.filter(|_| !is_selected);
            let mut width = styles::PANE_WIDTH - 2.0 * styles::PAD_X;
            if let Some(hint) = &hint {
                width -= 8.0 + hint.chars().count() as f32 * styles::ROW_SIZE * 0.6;
            }
            if is_selected {
                width -= 8.0 + styles::ROW_SIZE * 0.6;
            }
            let mut line = row![
                container(
                    text(clip(&label, styles::chars_fitting(width)))
                        .size(styles::ROW_SIZE)
                        .font(fonts().regular)
                        .color(fg)
                        .wrapping(Wrapping::None),
                )
                .width(Fill)
                .clip(true),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            if let Some(hint) = hint {
                line = line.push(
                    text(hint)
                        .size(styles::ROW_SIZE)
                        .font(fonts().regular)
                        .color(muted),
                );
            }
            if is_selected {
                line = line.push(
                    text("↲")
                        .size(styles::ROW_SIZE)
                        .font(fonts().medium)
                        .color(fg),
                );
            }
            let line = container(line)
                .padding([0.0, styles::PAD_X])
                .height(styles::ROW_HEIGHT)
                .width(Fill)
                .align_y(Alignment::Center);
            let line = if is_selected {
                line.style(move |_| styles::selected(c))
            } else {
                line
            };
            mouse_area(line)
                .on_press(message)
                .interaction(mouse::Interaction::Pointer)
                .into()
        });

    // What the app is and runs, under its actions.
    let details = state.selected_app().map(|app| {
        let mut lines: Vec<String> = app.generic_name.iter().cloned().collect();
        lines.push(app.exec.program());
        if app.terminal {
            lines.push("runs in a terminal".into());
        }
        let width = styles::PANE_WIDTH - 2.0 * styles::PAD_X - DETAIL_ICON - 10.0;
        let limit = (width / (styles::SMALL_SIZE * 0.6)).floor() as usize;
        let lines = column(lines.into_iter().map(|line| {
            text(clip(&line, limit))
                .size(styles::SMALL_SIZE)
                .line_height(1.6)
                .font(fonts().regular)
                .color(c.text_muted)
                .wrapping(Wrapping::None)
                .into()
        }));
        row![
            container(helpers::app_icon(
                app.icon.as_deref(),
                DETAIL_ICON,
                c.text_muted
            ))
            .padding(Padding::ZERO.top(2)),
            lines
        ]
        .spacing(10)
    });

    let mut pane = column![column(items)]
        .padding(Padding::from([styles::LIST_PADDING, 0.0]))
        .width(styles::PANE_WIDTH)
        .height(Fill);
    if let Some(details) = details {
        pane = pane.push(
            container(details)
                .padding(Padding::from([14.0, styles::PAD_X]).bottom(8))
                .clip(true),
        );
    }

    row![
        container(list).width(Fill).height(Fill),
        styles::vertical_divider(c),
        pane
    ]
    .height(Fill)
    .into()
}
