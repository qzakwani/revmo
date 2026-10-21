//! The modes screen, opened by typing `?` in the launcher: every mode the
//! search box can switch to, with its prefix, so they can be found without
//! knowing them.

use iced::alignment::Horizontal;
use iced::widget::{column, container, mouse_area, row, text};
use iced::{Alignment, Element, Fill, Padding, mouse};

use super::clipboard;
use crate::styles::{self, fonts};
use crate::theme::Colors;
use crate::{Message, Revmo};

/// Typed first in the launcher's search box, opens this screen.
pub const PREFIX: char = '?';

/// A mode the launcher's search box switches to.
pub struct Mode {
    /// Typed first, it switches to the mode.
    pub prefix: char,
    pub name: &'static str,
    /// Something one might type after the prefix.
    pub example: &'static str,
}

pub const MODES: [Mode; 1] = [Mode {
    prefix: clipboard::PREFIX,
    name: "clipboard history",
    example: "url",
}];

/// The mode whose prefix `search` starts with, and what follows it.
pub fn split(search: &str) -> Option<(&'static Mode, &str)> {
    let first = search.chars().next()?;
    let mode = MODES.iter().find(|m| m.prefix == first)?;
    Some((mode, &search[first.len_utf8()..]))
}

/// Shown before the input on this screen, instead of "›".
pub fn badge<'a>(c: Colors) -> Element<'a, Message> {
    container(
        text("MODES")
            .size(styles::SMALL_SIZE)
            .font(fonts().medium)
            .color(c.on_selection),
    )
    .padding([2.0, 6.0])
    .style(move |_| styles::selected(c))
    .into()
}

/// Every mode, `selected` highlighted.
pub fn view(state: &Revmo, selected: usize) -> Element<'_, Message> {
    let c = state.colors;
    let rows = MODES.iter().enumerate().map(|(i, mode)| {
        let is_selected = i == selected;
        let (fg, muted, sign) = match is_selected {
            true => (c.on_selection, c.on_selection, c.on_selection),
            false => (c.text, c.text_muted, c.accent),
        };
        let cell = |content: String, color| {
            text(content)
                .size(styles::ROW_SIZE)
                .font(fonts().regular)
                .color(color)
        };
        let columns = row![
            cell(mode.prefix.to_string(), sign)
                .font(fonts().medium)
                .width(styles::TAG_WIDTH),
            cell(mode.name.to_string(), fg).width(Fill),
            cell(format!("{} {}", mode.prefix, mode.example), muted).width(styles::SECONDARY_WIDTH),
            cell(if is_selected { "↲" } else { "" }.to_string(), fg)
                .font(fonts().medium)
                .width(styles::SHORTCUT_WIDTH)
                .align_x(Horizontal::Right),
        ]
        .align_y(Alignment::Center);
        let body = container(columns)
            .height(styles::ROW_HEIGHT)
            .width(Fill)
            .align_y(Alignment::Center)
            .padding([0.0, styles::PAD_X])
            .style(move |_| match is_selected {
                true => styles::selected(c),
                false => container::Style::default(),
            });
        mouse_area(body)
            .on_press(Message::ModePress(i))
            .interaction(mouse::Interaction::Pointer)
            .into()
    });
    container(column(rows))
        .padding(Padding::from([styles::LIST_PADDING, 0.0]))
        .width(Fill)
        .into()
}

pub fn footer<'a>(c: Colors) -> Element<'a, Message> {
    let hint = |s| styles::hint(c, s);
    styles::footer_bar(
        c,
        [hint("↲ enter mode"), hint("esc back")],
        styles::small("no prefix searches apps")
            .color(c.text_faint)
            .into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_off_a_mode_prefix() {
        let (mode, rest) = split(";git push").unwrap();
        assert_eq!((mode.name, rest), ("clipboard history", "git push"));
        assert_eq!(split(";").map(|(_, rest)| rest), Some(""));
        assert!(split("fire;fox").is_none());
        assert!(split(" ;x").is_none());
        assert!(split("").is_none());
    }

    #[test]
    fn prefixes_are_unique_and_not_the_list_prefix() {
        for (i, mode) in MODES.iter().enumerate() {
            assert_ne!(mode.prefix, PREFIX);
            assert!(MODES[i + 1..].iter().all(|m| m.prefix != mode.prefix));
        }
    }
}
