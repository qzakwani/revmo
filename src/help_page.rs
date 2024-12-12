use crate::styles::{self, Icons};
use crate::{Message, Revmo};
use iced::widget::{column, container, row, text, Space, Text};
use iced::Length::Fill;
use iced::{Alignment, Element};

const CELL_HEIGHT: f32 = 30.0;
const ICON_SIZE: f32 = 25.0;

pub fn view(state: &Revmo) -> Element<'static, Message> {
    container(
        column![
            Icons::get_logo(),
            text("Developed by github.com/qzakwani").font(styles::FONT_BOLD),
            text("For updates visit: github.com/qzakwani/revmo").font(styles::FONT_BOLD),
            Space::with_height(20),
            text(format!("Selected theme: {}", state.theme)),
            text("Keyboard shortcuts:"),
            row![
                column![
                    keyboard_shortcut("\u{f0311}"),
                    keyboard_shortcut("\u{f061}"),
                    keyboard_shortcut("\u{f12b7}"),
                    keyboard_shortcut("\u{f0634} \u{f0c00}"),
                    keyboard_shortcut("\u{f0634} \u{f0c1e}"),
                    keyboard_shortcut("\u{f0634} \u{f0bf4}"),
                    keyboard_shortcut("\u{f0634} \u{f061}"),
                    keyboard_shortcut("\u{f0634} \u{f060}"),
                ],
                column![
                    keyboard_shortcut_exp("Run selected app"),
                    keyboard_shortcut_exp("View/Edit selected app's arguments"),
                    keyboard_shortcut_exp("Close Revmo"),
                    keyboard_shortcut_exp("View/Hide this help page"),
                    keyboard_shortcut_exp("Refresh Revmo: reset to defaults"),
                    keyboard_shortcut_exp("Remove selected app from Revmo"),
                    keyboard_shortcut_exp("Next theme"),
                    keyboard_shortcut_exp("Previous theme"),
                ]
            ]
            .spacing(30)
        ]
        .spacing(12)
        .align_x(Alignment::Center),
    )
    .height(Fill)
    .width(Fill)
    .align_x(Alignment::Center)
    .align_y(Alignment::Center)
    .into()
}

fn keyboard_shortcut(icon: &str) -> Text {
    text(icon)
        .size(ICON_SIZE)
        .height(CELL_HEIGHT)
        .align_y(Alignment::Center)
}
fn keyboard_shortcut_exp(exp: &str) -> Text {
    text(exp).height(CELL_HEIGHT).align_y(Alignment::Center)
}
