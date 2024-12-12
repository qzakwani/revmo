use std::{thread, time::Duration};

use iced::{
    widget::{button, column, container, image, row, svg, text, Column, Container, Space},
    Alignment, Color, Element,
    Length::Fill,
    Theme,
};

use crate::{
    config, parser::App, store, styles, Message, Revmo, ToastLevel, APPS, APP_HEIGHT, APP_PADDING,
    DEFAULT_APP_ICON,
};

pub fn set_app_history(state: &mut Revmo, id: usize) {
    unsafe {
        if let Some(pos) = APPS.iter().position(|app| app.id == id) {
            let removed = APPS.remove(pos);
            APPS.insert(0, removed);
            state.store = true;
        }
    }
}

pub fn refresh_app() -> Option<()> {
    store::empty_app_store()?;
    config::Config::reset()?;
    Some(())
}

pub fn remove_app(state: &mut Revmo, id: usize) -> Option<(String, ToastLevel)> {
    unsafe {
        if let Some(pos) = APPS.iter().position(|app| app.id == id) {
            let removed = APPS.remove(pos);
            state.store = true;
            return Some((format!("{} removed!", removed.name), ToastLevel::Success));
        }
    }
    Some(("Error removing app!".to_string(), ToastLevel::Error))
}

pub async fn toast_task() {
    thread::sleep(Duration::from_secs(5));
}

pub fn show_toast(msg: String, level: ToastLevel, t: &Theme) -> Container<Message> {
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

// pub fn batch_size(container_h: f32, item_h: f32) -> usize {
//     (container_h / item_h).floor() as usize
// }

pub fn get_app_view<'a>(app: &'a App, selected: bool, theme: &'a Theme) -> Container<'a, Message> {
    let icon = view_app_icon(app.icon.clone());
    if selected {
        container(
            row![
                Space::with_width(6),
                icon,
                Space::with_width(6),
                button(get_inner_button(app, theme, true))
                    .on_press(Message::RunPress(app.id.clone(), app.exec.clone()))
                    .style(|_, _| styles::UNSELECTED_STYLE.clone())
                    .width(Fill)
            ]
            .align_y(Alignment::Center),
        )
        .height(APP_HEIGHT)
        .align_y(Alignment::Center)
        .padding(APP_PADDING)
        .style(|t| styles::selected_container_style(t))
    } else {
        container(
            row![
                Space::with_width(6),
                icon,
                Space::with_width(6),
                button(get_inner_button(app, theme, false))
                    .on_press(Message::RunPress(app.id.clone(), app.exec.clone()))
                    .width(Fill)
                    .style(|_, _| styles::UNSELECTED_STYLE.clone())
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
            text(DEFAULT_APP_ICON.clone())
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

    if app.action.is_some() && app.args.is_some() {
        inner = inner.push(
            row![
                text(format!("{} {}", '\u{f140b}', app.action.clone().unwrap()))
                    .size(styles::SECONDARY_FONT_SIZE)
                    .color(if selected {
                        styles::selected_color(theme)
                    } else {
                        theme.palette().text
                    }),
                text(format!("{} {}", '\u{f061}', app.args.clone().unwrap()))
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

    if let Some(args) = &app.args {
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
