//! The arguments form (→ on an actions pane entry that takes files or
//! URLs): one field per argument, then run.

use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, rich_text, row, span, text, text_input};
use iced::{Alignment, Element, Fill, Padding};

use crate::styles::{self, fonts};
use crate::{Message, Revmo, parser::App};

pub fn view<'a>(state: &'a Revmo, app: &'a App) -> Element<'a, Message> {
    let c = state.colors;
    let run = move || Message::RunWithArgs(app.key.clone(), state.exec_args.clone());

    let title = rich_text([
        span::<(), _>(app.name.as_str()).font(fonts().medium),
        span(" › ").color(c.text_muted),
        span(app.action.as_deref().unwrap_or("open")),
        span(" with arguments").color(c.text_muted),
    ])
    .size(styles::ROW_SIZE)
    .font(fonts().regular)
    .color(c.text)
    .wrapping(Wrapping::None);

    let fields = state.app_args.iter().enumerate().map(|(i, code)| {
        row![
            text(code.noun())
                .size(styles::ROW_SIZE)
                .font(fonts().medium)
                .color(c.accent)
                .width(styles::TAG_WIDTH),
            text_input(code.description(), &state.exec_args[i])
                .id(i.to_string())
                .size(styles::ROW_SIZE)
                .font(fonts().regular)
                .padding([5, 8])
                .style(move |_, status| styles::field(c, status))
                .on_input(move |input| Message::ArgsChange(i, input))
                .on_submit(run()),
        ]
        .align_y(Alignment::Center)
        .into()
    });

    let mut form = column![title, column(fields).spacing(8)].spacing(14);
    if state.app_args.iter().any(|code| code.is_list()) {
        form = form.push(
            text("separate several with spaces; wrap names containing spaces in quotes")
                .size(styles::SECONDARY_SIZE)
                .font(fonts().regular)
                .color(c.text_muted),
        );
    }
    let run_button = button(text("run ↲").size(styles::ROW_SIZE).font(fonts().medium))
        .padding([7, 14])
        .style(move |_, _| styles::primary_button(c))
        .on_press_with(run);

    container(column![form, Space::new().height(Fill), run_button])
        .padding(Padding::from([12.0, styles::PAD_X]))
        .width(Fill)
        .height(Fill)
        .into()
}
