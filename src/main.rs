mod args_page;
mod batch;
mod config;
mod help_page;
mod helpers;
mod parser;
mod store;
mod styles;
mod theme;

use std::sync::LazyLock;
use std::time::Duration;
use std::{env, thread, vec};

use iced::keyboard::key::{Key, Named};
use iced::widget::text_input::focus;
use iced::widget::{column, container, focus_next, focus_previous, row, text, text_input, Space};
use iced::window::settings::PlatformSpecific;
use iced::{event, keyboard, window, Alignment, Event, Padding, Settings};
use iced::{Element, Subscription, Task};
use iced::{Fill, Theme};
use parser::App;
use revmo::xdg::exec::FieldCode;
use revmo::{instance, search, usage, xdg};
use styles::Icons;

static HOME_DIR: LazyLock<String> = LazyLock::new(|| {
    env::var_os("HOME")
        .expect("os $HOME not found")
        .to_string_lossy()
        .to_string()
});
static INPUT: LazyLock<text_input::Id> = LazyLock::new(|| text_input::Id::new("input"));
static CONTAINER: LazyLock<container::Id> = LazyLock::new(|| container::Id::new("main"));
const DEFAULT_APP_ICON: char = '\u{f40e}';
const APP_HEIGHT: f32 = 60.0;
const APP_PADDING: f32 = 4.0;
const BATCH_SIZE: usize = 8;
// total app height = header + body + footer
const HEADER_HEIGHT: f32 = 60.0;
const BODY_HEIGHT: f32 = BATCH_SIZE as f32 * APP_HEIGHT;
const FOOTER_HEIGHT: f32 = 40.0;
const WINDOW_HEIGHT: f32 = HEADER_HEIGHT + BODY_HEIGHT + FOOTER_HEIGHT;

static USAGE_FILE: LazyLock<std::path::PathBuf> = LazyLock::new(|| {
    xdg::base_dirs::state_home(std::path::Path::new(HOME_DIR.as_str())).join("revmo/usage")
});

// How much launch history counts against match quality; a full-word match
// is worth roughly 100.
const FRECENCY_WEIGHT: f64 = 10.0;

// Actions ("New Window", ...) rank a little below their app on similar matches.
const ACTION_WEIGHT: f64 = 0.9;

const TOAST_DURATION: Duration = Duration::from_secs(5);

const LAUNCH_ICON: &[u8] = include_bytes!("../assets/launch.svg");

/// The listening socket of this (primary) instance, handed to the
/// subscription that watches it.
static INSTANCE_SOCKET: std::sync::Mutex<Option<std::os::unix::net::UnixListener>> =
    std::sync::Mutex::new(None);

fn main() -> iced::Result {
    let cache_dir =
        xdg::base_dirs::cache_home(std::path::Path::new(HOME_DIR.as_str())).join("revmo");
    match instance::acquire(&instance::socket_path(&cache_dir)) {
        Some(instance::Instance::Secondary) => return Ok(()),
        Some(instance::Instance::Primary(listener)) => {
            *INSTANCE_SOCKET.lock().unwrap() = Some(listener);
        }
        None => {}
    }

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
    /// Every app not hidden by the user, most relevant first.
    apps: Vec<App>,
    /// Search data for each of `apps`.
    haystacks: Vec<search::Haystack>,
    /// Indices into `apps` matching the search, in display order.
    visible: Vec<usize>,
    toast: Option<(String, ToastLevel)>,
    /// Incremented per toast, so a timer only clears the toast it was started for.
    toast_id: u64,
    help: bool,
    is_args: bool,
    app_args: Vec<FieldCode>,
    exec_args: Vec<String>,
    usage: usage::Usage,
    is_loading: bool,
    /// Set once the window has had focus; losing it after that closes revmo.
    was_focused: bool,
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
    RunWithArgs(usize, Vec<String>),
    RunPress(usize),
    RemoveApp,
    RunFailed,
    Refresh,
    Exit,
    HelpPage,
    ToastDone(u64),
    Junk(()),
    ForwardTheme,
    BackwardTheme,
    WindowFocused,
    WindowUnfocused,
}

impl Revmo {
    fn new() -> (Self, Task<Message>) {
        (Revmo::_new(), load_apps(false))
    }

    fn _new() -> Self {
        let config = config::Config::load();
        let theme = theme::by_name(&config.theme).clone();
        Self {
            config,
            search: String::new(),
            selected: 0,
            theme,
            batch_size: BATCH_SIZE,
            apps: vec![],
            haystacks: vec![],
            visible: vec![],
            toast: None,
            toast_id: 0,
            help: false,
            is_args: false,
            app_args: Vec::new(),
            exec_args: Vec::new(),
            usage: usage::Usage::load(&USAGE_FILE),
            is_loading: true,
            was_focused: false,
        }
    }

    /// Launches the app with the given id and closes revmo on success.
    fn run(&mut self, id: usize, inputs: &[String]) -> Task<Message> {
        let Some(app) = self.apps.iter().find(|a| a.id == id) else {
            return Task::done(Message::RunFailed);
        };
        if let Err(err) = app.launch(inputs) {
            eprintln!("Error: failed to launch {}: {err}", app.name);
            return Task::done(Message::RunFailed);
        }
        let key = app.key.clone();
        self.usage.record(&key, usage::now());
        if let Err(e) = self.usage.save(&USAGE_FILE) {
            eprintln!("Failed to save {}: {e}", USAGE_FILE.display());
        }
        Task::done(Message::Exit)
    }

    /// Shows a toast and starts the timer that hides it.
    fn notify(&mut self, msg: String, level: ToastLevel) -> Task<Message> {
        self.toast_id += 1;
        self.toast = Some((msg, level));
        let id = self.toast_id;
        Task::perform(
            helpers::on_thread(|| thread::sleep(TOAST_DURATION)),
            move |_| Message::ToastDone(id),
        )
    }

    fn set_theme(&mut self, theme: Theme) {
        self.config.theme = theme.to_string();
        self.config.save();
        self.theme = theme;
    }

    fn selected_app(&self) -> Option<&App> {
        self.visible.get(self.selected).map(|&i| &self.apps[i])
    }

    /// Recomputes the visible apps from the search text: best matches
    /// first, with frequently used apps winning close calls.
    fn filter(&mut self) {
        let now = usage::now();
        let mut scored: Vec<(f64, usize)> = self
            .haystacks
            .iter()
            .enumerate()
            .filter_map(|(i, h)| {
                let app = &self.apps[i];
                let mut score = h.score(&self.search)?;
                if app.action.is_some() {
                    score *= ACTION_WEIGHT;
                }
                let usage = self.usage.score(&app.key, now);
                Some((score + FRECENCY_WEIGHT * usage.ln_1p(), i))
            })
            .collect();
        // Stable, so ties keep the frecency order of `apps`.
        scored.sort_by(|a, b| b.0.total_cmp(&a.0));
        self.visible = scored.into_iter().map(|(_, i)| i).collect();
        self.selected = 0;
    }

    fn theme(&self) -> iced::Theme {
        self.theme.clone()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AppsLoaded(mut apps) => {
                apps.retain(|a| !self.config.hidden.contains(&a.key));
                // Stable sort: apps never launched stay alphabetical.
                let now = usage::now();
                apps.sort_by(|a, b| {
                    let score = |app: &App| self.usage.score(&app.key, now);
                    score(b).total_cmp(&score(a))
                });
                self.haystacks = apps.iter().map(App::haystack).collect();
                self.apps = apps;
                self.is_loading = false;
                self.filter();
                focus(INPUT.clone()).map(Message::Junk)
            }
            Message::SearchChanged(input) => {
                self.search = input;
                self.filter();
                Task::none()
            }

            Message::RunIndex => match self.selected_app() {
                Some(app) => self.run(app.id, &[]),
                None => Task::done(Message::RunFailed),
            },

            Message::SelectDown => {
                if self.is_args {
                    return focus_next();
                }
                if self.selected + 1 < self.visible.len() {
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
                self.app_args = self.selected_app().map(App::arg_codes).unwrap_or_default();
                self.exec_args = vec![String::new(); self.app_args.len()];
                self.is_args = !self.app_args.is_empty();
                focus("0")
            }

            Message::HideArgs => {
                self.is_args = false;
                self.app_args.clear();
                self.exec_args.clear();
                focus(INPUT.clone())
            }

            Message::RunWithArgs(id, args) => self.run(id, &args),

            Message::RunPress(id) => self.run(id, &[]),

            Message::RemoveApp => {
                if let Some(&index) = self.visible.get(self.selected) {
                    let removed = self.apps.remove(index);
                    self.haystacks.remove(index);
                    self.config.hidden.push(removed.key.clone());
                    self.config.save();
                    self.search.clear();
                    self.filter();
                    return self.notify(format!("{} removed!", removed.name), ToastLevel::Success);
                }
                Task::none()
            }
            Message::HelpPage => {
                self.help = !self.help;
                Task::none()
            }

            Message::ToastDone(id) => {
                if id == self.toast_id {
                    self.toast = None;
                }
                Task::none()
            }

            Message::Exit => window::get_latest().and_then(window::close),

            Message::WindowFocused => {
                self.was_focused = true;
                Task::none()
            }
            // Like other launchers: clicking elsewhere dismisses revmo. Some
            // window managers never focus the window, so only close after it
            // actually had focus.
            Message::WindowUnfocused if self.was_focused => Task::done(Message::Exit),
            Message::WindowUnfocused => Task::none(),

            Message::Junk(_) => Task::none(),
            Message::RunFailed => self.notify("Failed to launch app!".into(), ToastLevel::Error),

            Message::Refresh => {
                // Rescan only: history, removed apps and the theme stay.
                self.search.clear();
                self.visible.clear();
                self.selected = 0;
                self.is_loading = true;
                let toast = self.notify("Apps rescanned.".into(), ToastLevel::Success);
                Task::batch([load_apps(true), toast])
            }

            Message::ForwardTheme => {
                self.set_theme(theme::next(&self.theme).clone());
                Task::none()
            }

            Message::BackwardTheme => {
                self.set_theme(theme::previous(&self.theme).clone());
                Task::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            Subscription::run(other_instances),
            event::listen_with(handle_event),
        ])
    }

    fn view(&self) -> Element<'_, Message> {
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
            if let Some(app) = self.selected_app() {
                return args_page::view(self, app);
            }
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
        .style(styles::header_style)
        .align_y(Alignment::Center)
        .padding([0, 20]);

        let (active_batch, total_batches, selected, apps) =
            batch::get_batch(&self.visible, self.batch_size, self.selected);
        let mut batch: Vec<Element<Message>> = Vec::with_capacity(apps.len());

        for (i, &index) in apps.iter().enumerate() {
            batch.push(helpers::get_app_view(&self.apps[index], i == selected, &self.theme).into());
        }

        let footer = if let Some((msg, level)) = &self.toast {
            helpers::show_toast(msg.clone(), level.clone(), &self.theme)
        } else {
            container(
                row![
                    text(format!("{}/{}", active_batch, total_batches))
                        .style(styles::page_counter_style)
                        .size(18),
                    Space::with_width(Fill),
                    text("\u{f0634} \u{f0fd} ")
                        .size(24)
                        .style(styles::page_counter_style)
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

/// Scans (or loads the cached) apps off the UI thread.
fn load_apps(force_scan: bool) -> Task<Message> {
    Task::perform(
        helpers::on_thread(move || App::get(force_scan)),
        Message::AppsLoaded,
    )
}

fn handle_event(e: Event, _: event::Status, _: window::Id) -> Option<Message> {
    match e {
        Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => match key {
            Key::Named(Named::ArrowDown) => Some(Message::SelectDown),
            Key::Named(Named::ArrowUp) => Some(Message::SelectUp),
            Key::Named(Named::Escape) => Some(Message::Exit),
            Key::Named(Named::ArrowRight) if modifiers.command() => Some(Message::ForwardTheme),
            Key::Named(Named::ArrowRight) => Some(Message::ViewArgs),
            Key::Named(Named::ArrowLeft) if modifiers.command() => Some(Message::BackwardTheme),
            Key::Named(Named::ArrowLeft) => Some(Message::HideArgs),
            Key::Named(Named::Tab) => Some(Message::SelectDown),
            Key::Character(c) if modifiers.command() && c.as_str() == "h" => {
                Some(Message::HelpPage)
            }
            Key::Character(c) if modifiers.command() && c.as_str() == "d" => {
                Some(Message::RemoveApp)
            }
            Key::Character(c) if modifiers.command() && c.as_str() == "r" => Some(Message::Refresh),
            _ => None,
        },
        Event::Window(window::Event::Focused) => Some(Message::WindowFocused),
        Event::Window(window::Event::Unfocused) => Some(Message::WindowUnfocused),
        _ => None,
    }
}

/// Emits `Exit` whenever another `revmo` starts (see instance.rs).
fn other_instances() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(1, |output| async move {
        if let Some(listener) = INSTANCE_SOCKET.lock().unwrap().take() {
            thread::spawn(move || {
                for _ in listener.incoming() {
                    let _ = output.clone().try_send(Message::Exit);
                }
            });
        }
        std::future::pending::<()>().await
    })
}
