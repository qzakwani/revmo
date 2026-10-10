use std::thread;

use iced::futures::channel::oneshot;
use iced::{
    widget::{button, column, container, image, row, svg, text, Column, Container, Space},
    Alignment, Color, Element,
    Length::Fill,
    Theme,
};

use crate::{parser::App, styles, Message, ToastLevel, APP_HEIGHT, APP_PADDING, DEFAULT_APP_ICON};

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

pub fn show_toast(msg: String, level: ToastLevel, t: &Theme) -> Container<'_, Message> {
    let txt: Color;
    let bg: Color;
    match level {
        ToastLevel::Error => {
            txt = t.extended_palette().danger.strong.text;
            bg = t.extended_palette().danger.strong.color;
        }
        ToastLevel::Success => {
            txt = t.extended_palette().success.strong.text;
            bg = t.extended_palette().success.strong.color;
        }
    }
    container(text(msg).color(txt).size(20))
        .width(Fill)
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(bg)),
            ..container::Style::default()
        })
        .align_y(Alignment::Center)
        .align_x(Alignment::Center)
        .width(Fill)
}

pub fn get_app_view<'a>(app: &'a App, selected: bool, theme: &'a Theme) -> Container<'a, Message> {
    let icon = view_app_icon(app.icon.clone());
    if selected {
        container(
            row![
                Space::with_width(6),
                icon,
                Space::with_width(6),
                button(get_inner_button(app, theme, true))
                    .on_press(Message::RunPress(app.id))
                    .style(|_, _| *styles::UNSELECTED_STYLE)
                    .width(Fill)
            ]
            .align_y(Alignment::Center),
        )
        .height(APP_HEIGHT)
        .align_y(Alignment::Center)
        .padding(APP_PADDING)
        .style(styles::selected_container_style)
    } else {
        container(
            row![
                Space::with_width(6),
                icon,
                Space::with_width(6),
                button(get_inner_button(app, theme, false))
                    .on_press(Message::RunPress(app.id))
                    .width(Fill)
                    .style(|_, _| *styles::UNSELECTED_STYLE)
            ]
            .align_y(Alignment::Center),
        )
        .height(APP_HEIGHT)
        .align_y(Alignment::Center)
        .padding(APP_PADDING)
    }
}

pub fn view_app_icon(icon: Option<String>) -> Element<'static, Message> {
    let _icon: Element<Message> = if let Some(p) = icon {
        if p.ends_with(".svg") {
            svg(p)
                .height(styles::ICON_SIZE)
                .width(styles::ICON_SIZE)
                .into()
        } else {
            image(p)
                .height(styles::ICON_SIZE)
                .width(styles::ICON_SIZE)
                .into()
        }
    } else {
        container(
            text(DEFAULT_APP_ICON)
                .color(Color::BLACK)
                .size(styles::ICON_SIZE * 0.9)
                .align_y(Alignment::Center)
                .align_x(Alignment::Center),
        )
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .height(styles::ICON_SIZE)
        .width(styles::ICON_SIZE)
        .into()
    };
    _icon
}

fn get_inner_button<'a>(app: &'a App, theme: &'a Theme, selected: bool) -> Column<'a, Message> {
    let codes = app.arg_codes();
    let args = (!codes.is_empty()).then(|| {
        codes
            .iter()
            .map(|c| c.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    });
    let mut inner = column![if selected {
        text(app.name.clone())
            .size(styles::PRIMARY_FONT_SIZE)
            .font(styles::FONT_BOLD)
            .color(styles::selected_color(theme))
            .align_y(Alignment::Center)
    } else {
        text(app.name.clone())
            .size(styles::PRIMARY_FONT_SIZE)
            .font(styles::FONT_MEDIUM)
            .color(theme.palette().text)
    }];

    if app.action.is_some() && args.is_some() {
        inner = inner.push(
            row![
                text(format!("{} {}", '\u{f140b}', app.action.clone().unwrap()))
                    .size(styles::SECONDARY_FONT_SIZE)
                    .color(if selected {
                        styles::selected_color(theme)
                    } else {
                        theme.palette().text
                    }),
                text(format!("{} {}", '\u{f061}', args.clone().unwrap()))
                    .size(styles::SECONDARY_FONT_SIZE)
                    .color(if selected {
                        styles::selected_color(theme)
                    } else {
                        theme.palette().text
                    }),
            ]
            .spacing(10),
        );

        return inner;
    }

    if let Some(action) = &app.action {
        inner = inner.push(
            text(format!("{} {}", '\u{f140b}', action))
                .size(styles::SECONDARY_FONT_SIZE)
                .color(if selected {
                    styles::selected_color(theme)
                } else {
                    theme.palette().text
                }),
        );
    }

    if let Some(args) = &args {
        inner = inner.push(
            text(format!("{} {}", '\u{f061}', args))
                .size(styles::SECONDARY_FONT_SIZE)
                .color(if selected {
                    styles::selected_color(theme)
                } else {
                    theme.palette().text
                }),
        );
    }

    inner.spacing(4)
}
