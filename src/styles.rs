//! Sizes, fonts and widget styles. Colors come from the theme
//! ([`Colors`]), so every style takes them.

use std::sync::OnceLock;

use iced::font::Weight;
use iced::widget::{Rule, Space, button, container, row, rule, scrollable, text, text_input};
use iced::{Alignment, Background, Border, Color, Element, Fill, Font, Padding, border};

use crate::theme::Colors;

pub const WINDOW_WIDTH: f32 = 560.0;
pub const INPUT_HEIGHT: f32 = 44.0;
pub const ROW_HEIGHT: f32 = 26.0;
/// Above and below the list of rows.
pub const LIST_PADDING: f32 = 6.0;
pub const FOOTER_HEIGHT: f32 = 28.0;
/// Left and right padding of every section.
pub const PAD_X: f32 = 14.0;

pub const INPUT_SIZE: f32 = 15.0;
pub const ROW_SIZE: f32 = 12.5;
pub const SECONDARY_SIZE: f32 = 12.0;
pub const SMALL_SIZE: f32 = 11.0;
pub const ICON_SIZE: f32 = 16.0;

/// Result row columns: icon or tag, title, secondary text, shortcut.
pub const TAG_WIDTH: f32 = 40.0;
pub const SECONDARY_WIDTH: f32 = 190.0;
pub const SHORTCUT_WIDTH: f32 = 44.0;
pub const PANE_WIDTH: f32 = 210.0;
/// Room for result titles: what the window has left after the other
/// columns, the paddings, the frame and an 8px gap.
pub const TITLE_WIDTH: f32 =
    WINDOW_WIDTH - 2.0 - 2.0 * PAD_X - TAG_WIDTH - SECONDARY_WIDTH - SHORTCUT_WIDTH - 8.0;
/// The same with the actions pane open.
pub const PANE_TITLE_WIDTH: f32 =
    WINDOW_WIDTH - 2.0 - PANE_WIDTH - 1.0 - 2.0 * PAD_X - MARK_WIDTH - TAG_WIDTH - 8.0;
/// The accent bar on the left of the highlighted row in the actions pane.
pub const MARK_WIDTH: f32 = 2.0;

/// The font family in its three weights.
#[derive(Debug, Clone, Copy)]
pub struct Fonts {
    /// The family's name, as installed.
    pub family: &'static str,
    pub regular: Font,
    pub medium: Font,
    pub semibold: Font,
}

static FONTS: OnceLock<Fonts> = OnceLock::new();

/// The font revmo ships with, used unless the config names another.
pub const BUNDLED_FAMILY: &str = "IBM Plex Mono";
pub const BUNDLED_FONTS: [&[u8]; 3] = [
    include_bytes!("../fonts/IBMPlexMono-Regular.ttf"),
    include_bytes!("../fonts/IBMPlexMono-Medium.ttf"),
    include_bytes!("../fonts/IBMPlexMono-SemiBold.ttf"),
];

/// The installed font family called `family` (ignoring case), spelled as
/// installed, or `None` if there's no such font.
pub fn installed_family(family: &str) -> Option<String> {
    let mut system = iced::advanced::graphics::text::font_system()
        .write()
        .unwrap_or_else(|e| e.into_inner());
    system.raw().db().faces().find_map(|face| {
        face.families
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(family.trim()))
            .map(|(name, _)| name.clone())
    })
}

/// Sets the font for this run: `family`, or the bundled one if it's empty.
/// Only the first call counts; a new font applies on next start.
pub fn init_fonts(family: &str) {
    let _ = FONTS.set(Fonts::new(family));
}

pub fn fonts() -> Fonts {
    *FONTS.get_or_init(|| Fonts::new(""))
}

impl Fonts {
    fn new(family: &str) -> Self {
        let family: &'static str = match family.trim() {
            "" => BUNDLED_FAMILY,
            // Fonts are named by `&'static str`; this runs once per run.
            family => Box::leak(family.to_string().into_boxed_str()),
        };
        let regular = Font::with_name(family);
        Fonts {
            family,
            regular,
            medium: Font {
                weight: Weight::Medium,
                ..regular
            },
            semibold: Font {
                weight: Weight::Semibold,
                ..regular
            },
        }
    }
}

/// How many characters of row text fit in `width`, for a monospace font
/// (0.6em advance, as IBM Plex Mono's).
pub fn chars_fitting(width: f32) -> usize {
    (width / (ROW_SIZE * 0.6)).floor() as usize
}

fn fill(color: Color) -> Option<Background> {
    Some(Background::Color(color))
}

/// The whole window: background and outer border.
pub fn frame(c: Colors) -> container::Style {
    container::Style {
        text_color: Some(c.text),
        background: fill(c.background),
        border: border::color(c.frame).width(1),
        ..container::Style::default()
    }
}

pub fn footer(c: Colors) -> container::Style {
    container::Style {
        text_color: Some(c.text_muted),
        background: fill(c.raised),
        ..container::Style::default()
    }
}

/// The selected row: the full-width inverted bar.
pub fn selected(c: Colors) -> container::Style {
    container::Style {
        text_color: Some(c.on_selection),
        background: fill(c.selection),
        ..container::Style::default()
    }
}

/// The highlighted row in the list while the actions pane is open.
pub fn soft_selected(c: Colors) -> container::Style {
    container::Style {
        background: fill(c.selected_soft),
        ..container::Style::default()
    }
}

pub fn solid(color: Color) -> container::Style {
    container::Style {
        background: fill(color),
        ..container::Style::default()
    }
}

/// A 1px divider.
pub fn divider<'a>(c: Colors) -> Rule<'a> {
    rule::horizontal(1).style(move |_| line(c.border))
}

pub fn vertical_divider<'a>(c: Colors) -> Rule<'a> {
    rule::vertical(1).style(move |_| line(c.border))
}

fn line(color: Color) -> rule::Style {
    rule::Style {
        color,
        radius: border::Radius::default(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

/// The search box: no box at all, just text.
pub fn search_input(c: Colors) -> text_input::Style {
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        icon: c.text_muted,
        placeholder: c.text_muted,
        value: c.text,
        selection: Color {
            a: 0.35,
            ..c.accent
        },
    }
}

/// An argument field: a raised box, outlined in the accent when focused.
pub fn field(c: Colors, status: text_input::Status) -> text_input::Style {
    let edge = match status {
        text_input::Status::Focused { .. } => c.accent,
        _ => c.border_strong,
    };
    text_input::Style {
        background: Background::Color(c.raised),
        border: border::color(edge).width(1),
        ..search_input(c)
    }
}

/// A thin, flat scrollbar in the theme's colors.
pub fn scrollbar() -> scrollable::Direction {
    scrollable::Direction::Vertical(
        scrollable::Scrollbar::new()
            .width(3)
            .scroller_width(3)
            .margin(2),
    )
}

pub fn scroll(c: Colors, theme: &iced::Theme, status: scrollable::Status) -> scrollable::Style {
    let rail = scrollable::Rail {
        background: None,
        border: Border::default(),
        scroller: scrollable::Scroller {
            background: Background::Color(c.border_strong),
            border: Border::default(),
        },
    };
    scrollable::Style {
        vertical_rail: rail,
        horizontal_rail: rail,
        ..scrollable::default(theme, status)
    }
}

pub fn primary_button(c: Colors) -> button::Style {
    button::Style {
        background: fill(c.accent),
        text_color: c.on_accent,
        border: Border::default(),
        ..button::Style::default()
    }
}

/// A footer bar: `left` items, then `right` pushed to the far side.
pub fn footer_bar<'a, Message: 'a>(
    c: Colors,
    left: impl IntoIterator<Item = Element<'a, Message>>,
    right: Element<'a, Message>,
) -> Element<'a, Message> {
    container(
        row(left)
            .push(Space::new().width(Fill))
            .push(right)
            .spacing(14)
            .align_y(Alignment::Center),
    )
    .padding(Padding::from([0.0, PAD_X]))
    .height(FOOTER_HEIGHT)
    .width(Fill)
    .align_y(Alignment::Center)
    .style(move |_| footer(c))
    .into()
}

/// Small footer text.
pub fn small<'a>(content: impl text::IntoFragment<'a>) -> text::Text<'a> {
    text(content).size(SMALL_SIZE).font(fonts().regular)
}
