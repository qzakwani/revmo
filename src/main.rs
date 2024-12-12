mod args_page;
mod batch;
mod config;
mod help_page;
mod helpers;
mod parser;
mod store;
mod styles;
mod theme;

// use std::time::Instant;
use std::{env, vec};

use iced::keyboard::key::{Key, Named};
use iced::widget::text_input::focus;
use iced::widget::{column, container, focus_next, focus_previous, row, text, text_input, Space};
use iced::window::settings::PlatformSpecific;
use iced::{event, keyboard, window, Alignment, Event, Padding, Settings};
use iced::{Element, Subscription, Task};
use iced::{Fill, Theme};
use once_cell::sync::Lazy;
use parser::App;
use styles::Icons;

static mut APPS: Vec<App> = Vec::new();
static HOME_DIR: Lazy<String> = Lazy::new(|| {
    env::var_os("HOME")
        .expect("os $HOME not found")
        .to_string_lossy()
        .to_string()
});
static INPUT: Lazy<text_input::Id> = Lazy::new(|| text_input::Id::new("input"));
static CONTAINER: Lazy<container::Id> = Lazy::new(|| container::Id::new("main"));
const DEFAULT_APP_ICON: char = '\u{f40e}';
const APP_HEIGHT: f32 = 60.0;
const APP_PADDING: f32 = 4.0;
const BATCH_SIZE: usize = 8;
// total app height = header + body + footer
const HEADER_HEIGHT: f32 = 60.0;
const BODY_HEIGHT: f32 = BATCH_SIZE as f32 * APP_HEIGHT;
const FOOTER_HEIGHT: f32 = 40.0;
const WINDOW_HEIGHT: f32 = HEADER_HEIGHT + BODY_HEIGHT + FOOTER_HEIGHT;

const LAUNCH_ICON: &[u8] = include_bytes!("../assets/launch.svg");

fn main() -> iced::Result {
    let settings = window::settings::Settings {
        size: iced::Size::new(1000.0, WINDOW_HEIGHT),
        position: window::Position::Centered,
        min_size: None,
        max_size: None,
        visible: true,
        resizable: false,
        decorations: false,
        transparent: false,
        level: window::Level::AlwaysOnTop,
        icon: None,
        #[cfg(target_os = "linux")]
        platform_specific: PlatformSpecific {
            application_id: String::from("revmo"),
            ..PlatformSpecific::default()
        },
        exit_on_close_request: true,
    };
    iced::application("REVMO", Revmo::update, Revmo::view)
        .settings(Settings {
            id: Some(String::from("revmo")),
            ..Settings::default()
        })
        .window(settings)
        .theme(Revmo::theme)
        .font(include_bytes!("../fonts/JetBrainsMonoNerdFont-Light.ttf").as_slice())
        .font(include_bytes!("../fonts/JetBrainsMonoNerdFont-Medium.ttf").as_slice())
        .font(include_bytes!("../fonts/JetBrainsMonoNerdFont-Bold.ttf").as_slice())
        .default_font(styles::FONT_LIGHT)
        .subscription(Revmo::subscription)
        .run_with(Revmo::new)
}

struct Revmo {
    config: config::Config,
    search: String,
    selected: usize,
    theme: iced::Theme,
    batch_size: usize,
    apps: Vec<App>,
    toast: Option<(String, ToastLevel)>,
    help: bool,
    is_args: bool,
    app_args: Vec<String>,
    exec_args: Vec<String>,
    store: bool,
    is_loading: bool,
}

#[derive(Clone, PartialEq)]
enum ToastLevel {
    Error,
    Success,
}

#[derive(Debug, Clone)]
enum Message {
    AppsLoaded(Vec<App>),
    SearchChanged(String),
    RunIndex,
    SelectDown,
    SelectUp,
    ViewArgs,
    HideArgs,
    ArgsChange(usize, String),
    RunWithArgs(usize, String, Vec<String>),
    RunPress(usize, String),
    RemoveApp,
    RunFailed,
    Refresh,
    Exit,
    HelpPage,
    ToastDone(()),
    Junk(()),
    ForwardTheme,
    BackwardTheme,
    // Resized(iced::Size),
}

impl Revmo {
    fn new() -> (Self, Task<Message>) {
        (
            Revmo::_new(),
            Task::perform(App::get(), Message::AppsLoaded),
        )
    }

    fn _new() -> Self {
        let c = config::Config::load();
        let theme_id = c.theme_id;
        Self {
            config: c,
            search: String::new(),
            selected: 0,
            theme: theme::get_theme(theme_id).to_owned(),
            batch_size: BATCH_SIZE,
            apps: vec![],
            toast: None,
            help: false,
            is_args: false,
            app_args: Vec::new(),
            exec_args: Vec::new(),
            store: false,
            is_loading: true,
        }
    }

    fn theme(&self) -> iced::Theme {
        self.theme.clone()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AppsLoaded(apps) => {
                unsafe {
                    APPS = apps.clone();
                    self.apps = apps;
                    self.is_loading = false;
                }
                focus(INPUT.clone()).map(Message::Junk)
            }
            Message::SearchChanged(input) => {
                self.search = input;
                self.selected = 0;

                unsafe {
                    if self.search.is_empty() {
                        self.apps = APPS.clone();
                        return Task::none();
                    }
                    self.apps = APPS
                        .iter()
                        .filter(|app| {
                            app.name
                                .to_lowercase()
                                .contains(&self.search.to_lowercase())
                        })
                        .cloned()
                        .collect();
                }
                Task::none()
            }

            Message::RunIndex => {
                if let Some(app) = self.apps.get(self.selected).cloned() {
                    helpers::set_app_history(self, app.id);
                    return Task::done(run(app.exec, None));
                }
                Task::done(Message::RunFailed)
            }

            Message::SelectDown => {
                if self.is_args {
                    return focus_next();
                }
                let l = if self.apps.len() > 0 {
                    self.apps.len() - 1
                } else {
                    0
                };
                if self.selected < l {
                    self.selected += 1;
                }
                Task::none()
            }
            Message::SelectUp => {
                if self.is_args {
                    return focus_previous();
                }
                if self.selected > 0 {
                    self.selected -= 1;
                }
                Task::none()
            }

            Message::ArgsChange(i, arg) => {
                self.exec_args[i] = arg;
                Task::none()
            }

            Message::ViewArgs => {
                if self.is_args {
                    return Task::none();
                }
                self.is_args = if self.apps[self.selected].args.is_some() {
                    self.app_args = self.apps[self.selected]
                        .args
                        .clone()
                        .unwrap()
                        .split(',')
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>();
                    self.exec_args = vec!["".to_string(); self.app_args.len()];
                    true
                } else {
                    false
                };
                focus("0")
            }

            Message::HideArgs => {
                self.is_args = false;
                self.app_args.clear();
                self.exec_args.clear();
                focus(INPUT.clone())
            }

            Message::RunWithArgs(id, exec, args) => {
                helpers::set_app_history(self, id);
                Task::done(run(exec, Some(args)))
            }

            Message::RunPress(id, exec) => {
                helpers::set_app_history(self, id);
                Task::done(run(exec, None))
            }

            Message::RemoveApp => {
                if let Some(app) = self.apps.get(self.selected).cloned() {
                    self.toast = helpers::remove_app(self, app.id);
                    if self.toast.as_ref().unwrap().1 == ToastLevel::Success {
                        unsafe {
                            self.selected = 0;
                            self.search = String::new();
                            self.apps = APPS.clone();
                        }
                    }
                }
                Task::perform(helpers::toast_task(), Message::ToastDone)
            }
            Message::HelpPage => {
                self.help = !self.help;
                Task::none()
            }

            Message::ToastDone(_) => {
                self.toast = None;
                Task::none()
            }

            Message::Exit => {
                if self.store {
                    unsafe {
                        store::store_apps(APPS.as_slice());
                    };
                }
                window::get_latest().and_then(window::close)
            }

            Message::Junk(_) => Task::none(),
            Message::RunFailed => {
                self.toast = Some(("Failed to launch app!".to_string(), ToastLevel::Error));
                Task::perform(helpers::toast_task(), Message::ToastDone)
            }

            Message::Refresh => {
                let res = helpers::refresh_app();
                if res.is_none() {
                    self.toast = Some(("Failed to refresh app!".to_string(), ToastLevel::Error));
                    return Task::perform(helpers::toast_task(), Message::ToastDone);
                }
                *self = Revmo::_new();
                self.toast = Some(("Revmo is refreshed.".to_string(), ToastLevel::Success));

                Task::perform(App::get(), Message::AppsLoaded)
                    .chain(Task::perform(helpers::toast_task(), Message::ToastDone))
            }

            Message::ForwardTheme => {
                let (theme, i) = theme::forward_theme(self.config.theme_id);
                self.theme = theme.to_owned();
                self.config.theme_id = i;
                self.config.save();
                Task::none()
            }

            Message::BackwardTheme => {
                let (theme, i) = theme::backward_theme(self.config.theme_id);
                self.theme = theme.to_owned();
                self.config.theme_id = i;
                self.config.save();
                Task::none()
            } // Message::Resized(size) => {
              // self.batch_size = helpers::batch_size(size.height, APP_HEIGHT);
              // Task::none()
              // }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        event::listen_with(|e, _, _| match e {
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => match key {
                Key::Named(Named::ArrowDown) => Some(Message::SelectDown),
                Key::Named(Named::ArrowUp) => Some(Message::SelectUp),
                Key::Named(Named::Escape) => Some(Message::Exit),
                Key::Named(Named::ArrowRight) if modifiers.command() => Some(Message::ForwardTheme),
                Key::Named(Named::ArrowRight) => Some(Message::ViewArgs),
                Key::Named(Named::ArrowLeft) if modifiers.command() => Some(Message::BackwardTheme),
                Key::Named(Named::ArrowLeft) => Some(Message::HideArgs),
                Key::Named(Named::Tab) => Some(Message::SelectDown),
                Key::Character(c) if modifiers.command() && *c == *smol_str::SmolStr::new("h") => {
                    Some(Message::HelpPage)
                }
                Key::Character(c) if modifiers.command() && *c == *smol_str::SmolStr::new("d") => {
                    Some(Message::RemoveApp)
                }
                Key::Character(c) if modifiers.command() && *c == *smol_str::SmolStr::new("r") => {
                    Some(Message::Refresh)
                }
                _ => None,
            },
            // Event::Window(window::Event::Resized(s)) => Some(Message::Resized(s)),
            _ => None,
        })
    }

    fn view(&self) -> Element<Message> {
        if self.is_loading {
            return container(text("Loading apps...").size(30))
                .height(Fill)
                .width(Fill)
                .center(Fill)
                .into();
        }
        if self.help {
            return help_page::view(self);
        }

        if self.is_args {
            return args_page::view(self, &self.apps[self.selected]);
        }

        let input = text_input("", &self.search)
            .id(INPUT.clone())
            .on_input(Message::SearchChanged)
            .on_submit(Message::RunIndex)
            .style(|t: &Theme, s| styles::input_style(t, s))
            .size(20)
            .padding(0);

        let header = container(
            row![
                Icons::Launch.get().size(30).align_y(Alignment::Center),
                input
            ]
            .align_y(Alignment::Center)
            .spacing(20),
        )
        .style(|t| styles::header_style(t))
        .align_y(Alignment::Center)
        .padding([0, 20]);

        let (active_batch, total_batches, selected, apps) =
            batch::get_batch(&self.apps, self.batch_size, self.selected);
        let mut batch: Vec<Element<Message>> = Vec::with_capacity(apps.len());

        for (i, app) in apps.iter().enumerate() {
            batch.push(helpers::get_app_view(app, i == selected, &self.theme).into());
        }

        let footer = if let Some((msg, level)) = &self.toast {
            helpers::show_toast(msg.clone(), level.clone(), &self.theme)
        } else {
            container(
                row![
                    text(format!("{}/{}", active_batch, total_batches))
                        .style(|t| styles::PAGE_COUNTER_STYLE.clone()(t))
                        .size(18),
                    Space::with_width(Fill),
                    text("\u{f0634} \u{f0fd} ")
                        .size(24)
                        .style(|t| styles::PAGE_COUNTER_STYLE.clone()(t))
                        .align_y(Alignment::Center),
                    Space::with_width(Fill),
                    text("\u{f12b7} ")
                        .size(24)
                        .color(self.theme.palette().danger)
                        .align_y(Alignment::Center)
                ]
                .align_y(Alignment::Center)
                .padding(Padding {
                    top: 4.0,
                    right: 10.0,
                    bottom: 4.0,
                    left: 10.0,
                }),
            )
        };
        column![
            header.height(HEADER_HEIGHT),
            container(column(batch))
                .height(Fill)
                .width(Fill)
                .id(CONTAINER.clone())
                .height(BODY_HEIGHT),
            footer.height(FOOTER_HEIGHT)
        ]
        .height(Fill)
        .width(Fill)
        .into()
    }
}

fn run(exec: String, args: Option<Vec<String>>) -> Message {
    let mut binding = std::process::Command::new("sh");

    let mut command = exec;
    if args.is_some() {
        for arg in args.unwrap() {
            command.push_str(" ");
            command.push_str(&arg);
        }
    }

    binding.arg("-c").arg(command);

    if let Err(err) = binding.spawn() {
        eprintln!("Error: {:?}", err);
        return Message::RunFailed;
    }

    Message::Exit
}
