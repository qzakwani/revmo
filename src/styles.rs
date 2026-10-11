use iced::theme::palette::{Pair, mix};
use iced::{
    Color, Font, Theme, border,
    widget::{
        button, container, svg, text,
        text_input::{self, Status},
    },
};
use std::sync::LazyLock;

pub const PRIMARY_FONT_SIZE: u32 = 14;
pub const SECONDARY_FONT_SIZE: u32 = 12;
pub const ICON_SIZE: f32 = APP_HEIGHT * 0.5;

pub const FONT: Font = Font::with_name("JetBrainsMono NF");
pub const FONT_BOLD: Font = Font {
    weight: iced::font::Weight::Bold,
    ..FONT
};
pub const FONT_MEDIUM: Font = Font {
    weight: iced::font::Weight::Medium,
    ..FONT
};
pub const FONT_LIGHT: Font = Font {
    weight: iced::font::Weight::Light,
    ..FONT
};

// The "Nude" themes shade their header and selected row by mixing the
// background toward the text color, with the theme's text on top if it has
// AAA contrast (7:1) and black or white otherwise. iced 0.14's own
// weak/strong background shades barely differ from the background on dark
// themes, which would hide the selection.
const NUDE_WEAK: f32 = 0.15;
const NUDE_STRONG: f32 = 0.40;

fn nude_shade(t: &Theme, factor: f32) -> Pair {
    let palette = t.palette();
    let color = mix(palette.background, palette.text, factor);
    let text = if color.relative_contrast(palette.text) >= 7.0 {
        palette.text
    } else if color.relative_contrast(Color::WHITE) >= color.relative_contrast(Color::BLACK) {
        Color::WHITE
    } else {
        Color::BLACK
    };
    Pair { color, text }
}

pub fn header_style(t: &Theme) -> container::Style {
    match t {
        Theme::Custom(_) => container::Style {
            background: Some(iced::Background::Color(nude_shade(t, NUDE_WEAK).color)),
            ..container::Style::default()
        },

        _ => container::Style {
            background: Some(iced::Background::Color(
                t.extended_palette().primary.weak.color,
            )),
            ..container::Style::default()
        },
    }
}

pub fn selected_container_style(t: &Theme) -> container::Style {
    match t {
        Theme::Custom(_) => container::Style {
            background: Some(iced::Background::Color(nude_shade(t, NUDE_STRONG).color)),
            ..container::Style::default()
        },

        _ => container::Style {
            background: Some(iced::Background::Color(
                t.extended_palette().primary.strong.color,
            )),
            ..container::Style::default()
        },
    }
}

pub fn selected_color(t: &Theme) -> Color {
    match t {
        Theme::Custom(_) => nude_shade(t, NUDE_STRONG).text,

        _ => t.extended_palette().primary.strong.text,
    }
}

pub static UNSELECTED_STYLE: LazyLock<button::Style> =
    LazyLock::new(|| button::Style::default().with_background(Color::TRANSPARENT));

pub fn input_style(t: &Theme, s: Status) -> text_input::Style {
    match t {
        Theme::Custom(_) => text_input::Style {
            border: border::color(Color::TRANSPARENT),
            background: iced::Background::Color(Color::TRANSPARENT),
            value: nude_shade(t, NUDE_WEAK).text,
            ..text_input::default(t, s)
        },

        _ => text_input::Style {
            border: border::color(Color::TRANSPARENT),
            background: iced::Background::Color(Color::TRANSPARENT),
            value: t.extended_palette().primary.weak.text,
            ..text_input::default(t, s)
        },
    }
}

pub fn page_counter_style(t: &Theme) -> text::Style {
    text::Style {
        color: Some(t.extended_palette().secondary.strong.color),
    }
}

use std::hash::Hash;

use crate::APP_HEIGHT;

#[derive(Eq, PartialEq, Hash)]
pub enum Icons {
    Launch,
}

impl Icons {
    pub fn get(&self) -> text::Text<'_> {
        match self {
            Icons::Launch => text('\u{f14de}'),
        }
    }

    pub fn get_logo() -> svg::Svg<'static> {
        svg(iced::widget::svg::Handle::from_memory(include_bytes!(
            "../assets/logo.svg"
        )))
        .style(|t: &Theme, _| svg::Style {
            color: Some(t.palette().primary),
        })
        .height(120)
        .width(120)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::theme::palette::is_dark;

    #[test]
    fn nude_selection_stands_out() {
        for theme in crate::theme::THEMES.iter() {
            let Theme::Custom(_) = theme else { continue };
            let background = theme.palette().background;
            let selected = nude_shade(theme, NUDE_STRONG);
            // Light themes have always had a subtler selection. On dark ones
            // these shades give 2.5 or more; iced 0.14's own gave under 1.9.
            if is_dark(background) {
                let contrast = selected.color.relative_contrast(background);
                assert!(contrast >= 2.0, "{theme}: selection contrast {contrast}");
            }
            // WCAG AA for normal text.
            let text_contrast = selected.text.relative_contrast(selected.color);
            assert!(
                text_contrast >= 4.5,
                "{theme}: text contrast {text_contrast}"
            );
        }
    }
}
