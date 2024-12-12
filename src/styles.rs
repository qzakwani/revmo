use iced::{
    border,
    widget::{
        button, container, svg, text,
        text_input::{self, Status},
    },
    Color, Font, Theme,
};
use once_cell::sync::Lazy;

pub const PRIMARY_FONT_SIZE: u16 = 14;
pub const SECONDARY_FONT_SIZE: u16 = 12;
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

pub fn header_style(t: &Theme) -> container::Style {
    match t {
        Theme::Custom(_) => container::Style {
            background: Some(iced::Background::Color(
                t.extended_palette().background.weak.color,
            )),
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
            background: Some(iced::Background::Color(
                t.extended_palette().background.strong.color,
            )),
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
        Theme::Custom(_) => t.extended_palette().background.strong.text,

        _ => t.extended_palette().primary.strong.text,
    }
}

pub static UNSELECTED_STYLE: Lazy<button::Style> =
    Lazy::new(|| button::Style::default().with_background(Color::TRANSPARENT));

pub fn input_style(t: &Theme, s: Status) -> text_input::Style {
    match t {
        Theme::Custom(_) => text_input::Style {
            border: border::color(Color::TRANSPARENT),
            background: iced::Background::Color(Color::TRANSPARENT),
            value: t.extended_palette().background.weak.text,
            ..text_input::default(&t, s)
        },

        _ => text_input::Style {
            border: border::color(Color::TRANSPARENT),
            background: iced::Background::Color(Color::TRANSPARENT),
            value: t.extended_palette().primary.weak.text,
            ..text_input::default(&t, s)
        },
    }
}

pub static PAGE_COUNTER_STYLE: Lazy<fn(&Theme) -> text::Style> = Lazy::new(|| {
    |t: &Theme| text::Style {
        color: Some(t.extended_palette().secondary.strong.color),
    }
});

use std::hash::Hash;

use crate::APP_HEIGHT;

#[derive(Eq, PartialEq, Hash)]
pub enum Icons {
    Launch,
}

impl Icons {
    pub fn get(&self) -> text::Text {
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
    // pub fn get_launch() -> svg::Svg<'static> {
    //     svg(iced::widget::svg::Handle::from_memory(include_bytes!(
    //         "../assets/launch.svg"
    //     )))
    // }
}
