use iced::Length::Fill;
use iced::widget::{Space, button, column, container, row, svg, text, text_input};
use iced::{Alignment, Element, border, padding};

use crate::{FOOTER_HEIGHT, LAUNCH_ICON, Revmo, helpers, styles};
use crate::{Message, parser::App};

pub fn view<'a>(state: &'a Revmo, app: &'a App) -> Element<'a, Message> {
    if state.app_args.is_empty() {
        return column![].into();
    }
    let header = column([
        helpers::view_app_icon(app.icon.clone()),
        text(app.name.clone())
            .size(30)
            .font(styles::FONT_BOLD)
            .into(),
    ])
    .align_x(Alignment::Center)
    .spacing(20);

    let mut inputs: Vec<Element<'a, Message>> = Vec::new();
    for (i, code) in state.app_args.iter().enumerate() {
        inputs.push(
            row![
                text(format!("{}: ", code.as_str()))
                    .size(20)
                    .font(styles::FONT_BOLD),
                text_input(code.description(), &state.exec_args[i])
                    .id(i.to_string())
                    .padding(12)
                    .on_input(move |input| Message::ArgsChange(i, input))
                    .on_submit(Message::RunWithArgs(
                        app.key.clone(),
                        state.exec_args.clone()
                    ))
            ]
            .align_y(Alignment::Center)
            .into(),
        );
    }

    let action = button(
        container(
            svg(iced::widget::svg::Handle::from_memory(LAUNCH_ICON)).style(|_, _| svg::Style {
                color: Some(state.theme.extended_palette().primary.base.text),
            }),
        )
        .padding(padding::Padding {
            top: 10.0,
            bottom: 20.0,
            left: 0.0,
            right: 0.0,
        })
        .align_x(Alignment::Center)
        .align_y(Alignment::Center),
    )
    .width(100)
    .height(80)
    .style(|t, s| button::Style {
        border: border::rounded(10),
        ..button::primary(t, s)
    })
    .on_press_with(|| Message::RunWithArgs(app.key.clone(), state.exec_args.clone()));

    let footer = if let Some((msg, level)) = &state.toast {
        helpers::show_toast(msg.clone(), level.clone(), &state.theme)
    } else {
        container("").width(Fill)
    };

    column![
        row![
            text('\u{f060}')
                .size(20)
                .color(state.theme.palette().danger),
            Space::new().width(Fill),
            text("\u{f0311} Run")
                .size(20)
                .color(state.theme.palette().success)
        ]
        .padding(10)
        .width(Fill),
        header.padding(30),
        column(inputs).height(Fill).padding(30),
        action,
        footer.height(FOOTER_HEIGHT)
    ]
    .height(Fill)
    .align_x(Alignment::Center)
    .into()
}
