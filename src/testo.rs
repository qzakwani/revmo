use iced::widget::{column, container, row, text};
use iced::{Element, Theme};

use crate::Message;

pub fn view<'a>(t: &'a Theme) -> Element<'a, Message> {
    row![
        column![
            container(
                text("HELLO")
                    .size(12)
                    .color(t.extended_palette().background.strong.text)
            )
            .style(|_| {
                container::Style {
                    background: Some(iced::Background::Color(
                        t.extended_palette().background.strong.color,
                    )),
                    ..container::Style::default()
                }
            })
            .padding(10),
            container(
                text("HELLO")
                    .size(12)
                    .color(t.extended_palette().background.base.text)
            )
            .style(|_| {
                container::Style {
                    background: Some(iced::Background::Color(
                        t.extended_palette().background.base.color,
                    )),
                    ..container::Style::default()
                }
            })
            .padding(10),
            container(
                text("HELLO")
                    .size(12)
                    .color(t.extended_palette().background.weak.text)
            )
            .style(|_| {
                container::Style {
                    background: Some(iced::Background::Color(
                        t.extended_palette().background.weak.color,
                    )),
                    ..container::Style::default()
                }
            })
            .padding(10),
            container(
                text("HELLO")
                    .size(12)
                    .color(t.extended_palette().primary.strong.text)
            )
            .style(|_| {
                container::Style {
                    background: Some(iced::Background::Color(
                        t.extended_palette().primary.strong.color,
                    )),
                    ..container::Style::default()
                }
            })
            .padding(10),
            container(
                text("HELLO")
                    .size(12)
                    .color(t.extended_palette().primary.base.text)
            )
            .style(|_| {
                container::Style {
                    background: Some(iced::Background::Color(
                        t.extended_palette().primary.base.color,
                    )),
                    ..container::Style::default()
                }
            })
            .padding(10),
            container(
                text("HELLO")
                    .size(12)
                    .color(t.extended_palette().primary.weak.text)
            )
            .style(|_| {
                container::Style {
                    background: Some(iced::Background::Color(
                        t.extended_palette().primary.weak.color,
                    )),
                    ..container::Style::default()
                }
            })
            .padding(10),
            container(
                text("HELLO")
                    .size(12)
                    .color(t.extended_palette().secondary.strong.text)
            )
            .style(|_| {
                container::Style {
                    background: Some(iced::Background::Color(
                        t.extended_palette().secondary.strong.color,
                    )),
                    ..container::Style::default()
                }
            })
            .padding(10),
            container(
                text("HELLO")
                    .size(12)
                    .color(t.extended_palette().secondary.base.text)
            )
            .style(|_| {
                container::Style {
                    background: Some(iced::Background::Color(
                        t.extended_palette().secondary.base.color,
                    )),
                    ..container::Style::default()
                }
            })
            .padding(10),
            container(
                text("HELLO")
                    .size(12)
                    .color(t.extended_palette().secondary.weak.text)
            )
            .style(|_| {
                container::Style {
                    background: Some(iced::Background::Color(
                        t.extended_palette().secondary.weak.color,
                    )),
                    ..container::Style::default()
                }
            })
            .padding(10),
        ],
        column![
            container(
                text("HELLO")
                    .size(12)
                    .color(t.extended_palette().success.strong.text)
            )
            .style(|_| {
                container::Style {
                    background: Some(iced::Background::Color(
                        t.extended_palette().success.strong.color,
                    )),
                    ..container::Style::default()
                }
            })
            .padding(10),
            container(
                text("HELLO")
                    .size(12)
                    .color(t.extended_palette().success.base.text)
            )
            .style(|_| {
                container::Style {
                    background: Some(iced::Background::Color(
                        t.extended_palette().success.base.color,
                    )),
                    ..container::Style::default()
                }
            })
            .padding(10),
            container(
                text("HELLO")
                    .size(12)
                    .color(t.extended_palette().success.weak.text)
            )
            .style(|_| {
                container::Style {
                    background: Some(iced::Background::Color(
                        t.extended_palette().success.weak.color,
                    )),
                    ..container::Style::default()
                }
            })
            .padding(10),
            container(
                text("HELLO")
                    .size(12)
                    .color(t.extended_palette().danger.strong.text)
            )
            .style(|_| {
                container::Style {
                    background: Some(iced::Background::Color(
                        t.extended_palette().danger.strong.color,
                    )),
                    ..container::Style::default()
                }
            })
            .padding(10),
            container(
                text("HELLO")
                    .size(12)
                    .color(t.extended_palette().danger.base.text)
            )
            .style(|_| {
                container::Style {
                    background: Some(iced::Background::Color(
                        t.extended_palette().danger.base.color,
                    )),
                    ..container::Style::default()
                }
            })
            .padding(10),
            container(
                text("HELLO")
                    .size(12)
                    .color(t.extended_palette().danger.weak.text)
            )
            .style(|_| {
                container::Style {
                    background: Some(iced::Background::Color(
                        t.extended_palette().danger.weak.color,
                    )),
                    ..container::Style::default()
                }
            })
            .padding(10),
        ]
    ]
    .into()
}
