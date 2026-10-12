use iced::widget::{Space, column, container, rich_text, row, scrollable, span, text};
use iced::{Element, Fill, Padding};

use crate::styles::{self, fonts};
use crate::{Message, Revmo, VERSION};

const KEY_WIDTH: f32 = 130.0;

const SHORTCUTS: [(&str, &str); 8] = [
    ("↲", "run the selected app"),
    ("↑↓ / tab", "move the selection"),
    ("→", "actions; arguments of an action"),
    ("←", "back"),
    ("ctrl+r", "reload the config, themes and apps"),
    ("ctrl+→ / ctrl+←", "next / previous theme"),
    ("ctrl+h", "this help"),
    ("esc", "back, or close revmo"),
];

pub fn view(state: &Revmo) -> Element<'_, Message> {
    let c = state.colors;
    let font = fonts().family;
    let scale = match state.scale_auto {
        true => format!("auto ({})", state.scale),
        false => format!("{}", state.scale),
    };

    let header = rich_text([
        span::<(), _>("revmo ").color(c.accent).font(fonts().medium),
        span(VERSION),
        span(" · github.com/qzakwani/revmo").color(c.text_muted),
    ])
    .size(styles::ROW_SIZE)
    .font(fonts().regular)
    .color(c.text);

    let settings = text(format!(
        "theme: {} · font: {font} · scale: {scale}",
        state.theme.name
    ))
    .size(styles::SECONDARY_SIZE)
    .font(fonts().regular)
    .color(c.text_muted);

    let shortcuts = column(SHORTCUTS.iter().map(|&(key, what)| {
        row![
            text(key)
                .size(styles::ROW_SIZE)
                .font(fonts().regular)
                .color(c.accent)
                .width(KEY_WIDTH),
            text(what)
                .size(styles::ROW_SIZE)
                .font(fonts().regular)
                .color(c.text),
        ]
        .into()
    }))
    .spacing(5);

    // What's wrong in the config file, if anything, comes first.
    let problems = state.problems();
    let mut content = column![header, settings].spacing(6);
    if !problems.is_empty() {
        content = content.push(Space::new().height(8)).push(
            text(format!("config problems ({})", problems.len()))
                .size(styles::SMALL_SIZE)
                .font(fonts().medium)
                .color(c.danger),
        );
        content = content.push(
            column(problems.into_iter().map(|problem| {
                text(problem)
                    .size(styles::SECONDARY_SIZE)
                    .font(fonts().regular)
                    .color(c.text)
                    .into()
            }))
            .spacing(3),
        );
    }
    content = content.push(Space::new().height(8)).push(shortcuts);

    // Scrolls when there are many problems.
    let body = container(
        scrollable(
            container(content)
                .padding(Padding::from([16.0, styles::PAD_X]))
                .width(Fill),
        )
        .width(Fill)
        .direction(styles::scrollbar())
        .style(move |theme, status| styles::scroll(c, theme, status)),
    )
    .width(Fill)
    .height(Fill);

    let hint = |s: &'static str| styles::small(s).color(c.text_muted).into();
    column![
        body,
        styles::divider(c),
        styles::footer_bar(
            c,
            [hint("ctrl+h back"), hint("esc close")],
            styles::small(format!("revmo {VERSION}"))
                .color(c.text_faint)
                .into(),
        ),
    ]
    .into()
}
