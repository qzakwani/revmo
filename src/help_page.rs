use iced::widget::{Space, column, container, rich_text, row, scrollable, span, svg, text};
use iced::{Alignment, Color, Element, Fill, Padding};

use crate::styles::{self, fonts};
use crate::theme::Colors;
use crate::{Message, Revmo, VERSION};

const KEY_WIDTH: f32 = 130.0;
/// About as tall as the two lines beside it.
const LOGO_HEIGHT: f32 = 34.0;

const SHORTCUTS: [(&str, &str); 7] = [
    ("↲", "run the selected app"),
    ("↑↓", "move the selection"),
    ("→", "actions; arguments of an action"),
    ("←", "back"),
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
    let about = row![logo(c), column![header, settings].spacing(6)]
        .spacing(12)
        .align_y(Alignment::Center);
    let mut content = column![about].spacing(6);
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

    let hint = |s| styles::hint(c, s);
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

/// The logo's mark (logo/revmo-icon-dark.svg without its tile): the r in
/// the text color and its cursor square in the accent, so it suits every
/// theme.
fn logo<'a>(c: Colors) -> svg::Svg<'a> {
    let hex = |color: Color| {
        let [r, g, b, _] = color.into_rgba8();
        format!("#{r:02x}{g:02x}{b:02x}")
    };
    let (ink, accent) = (hex(c.text), hex(c.accent));
    let mark = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="132 180 236 216">
<path d="M160 208v160" stroke="{ink}" stroke-width="56" stroke-linecap="square"/>
<path d="M160 288c0-48 36-80 88-80h32" stroke="{ink}" stroke-width="56" stroke-linecap="square" fill="none"/>
<rect x="312" y="312" width="56" height="56" fill="{accent}"/>
</svg>"#
    );
    svg(svg::Handle::from_memory(mark.into_bytes()))
        .height(LOGO_HEIGHT)
        .width(LOGO_HEIGHT * 236.0 / 216.0)
}
