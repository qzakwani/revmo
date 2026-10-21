mod actions_pane;
mod args_page;
mod batch;
mod bundled_themes;
mod cli;
mod config;
mod help_page;
mod helpers;
mod modes;
mod parser;
mod store;
mod styles;
mod theme;

use std::io::IsTerminal;
use std::sync::LazyLock;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use std::{env, thread, vec};

use iced::keyboard::key::{Code, Key, Named, Physical};
use iced::widget::operation::{focus, focus_next, focus_previous, move_cursor_to_end};
use iced::widget::{Space, column, container, row, text, text_input};
use iced::window::settings::PlatformSpecific;
use iced::{Alignment, Event, Padding, Settings, event, keyboard, widget, window};
use iced::{Element, Fill, Subscription, Task};
use modes::menu::{self, Menu};
use modes::{clipboard, list};
use parser::App;
use revmo::xdg::exec::FieldCode;
use revmo::{instance, search, usage, xdg};
use styles::fonts;
use theme::{Colors, ThemeDef};

static HOME_DIR: LazyLock<String> = LazyLock::new(|| {
    env::var_os("HOME")
        .expect("os $HOME not found")
        .to_string_lossy()
        .to_string()
});
static INPUT: LazyLock<widget::Id> = LazyLock::new(|| widget::Id::new("input"));
const BATCH_SIZE: usize = 8;
// input + divider + list + divider + footer, inside the 1px frame
const LIST_HEIGHT: f32 = BATCH_SIZE as f32 * styles::ROW_HEIGHT + 2.0 * styles::LIST_PADDING;
const WINDOW_HEIGHT: f32 =
    styles::INPUT_HEIGHT + 1.0 + LIST_HEIGHT + 1.0 + styles::FOOTER_HEIGHT + 2.0;

/// Everything revmo keeps for itself: launch history and caches. What the user writes lives in `~/.config/revmo` instead.
static APP_DIR: LazyLock<std::path::PathBuf> = LazyLock::new(|| {
    xdg::base_dirs::data_home(std::path::Path::new(HOME_DIR.as_str())).join("revmo")
});
static USAGE_FILE: LazyLock<std::path::PathBuf> = LazyLock::new(|| APP_DIR.join("usage"));

// How much launch history counts against match quality; a full-word match
// is worth roughly 100.
const FREQUENCY_WEIGHT: f64 = 10.0;

// Actions ("New Window", ...) rank a little below their app on similar matches.
const ACTION_WEIGHT: f64 = 0.9;

// Key repeats closer together than this are dropped. Repeats that queued up
// behind a slow frame arrive in a burst, and would otherwise keep moving the
// selection after the key is released. Kept well below any key repeat
// interval (GNOME's default is 30ms), so a held key never loses a step.
const REPEAT_GAP: Duration = Duration::from_millis(5);

const TOAST_DURATION: Duration = Duration::from_secs(5);
// Config mistakes stay up longer: there may be several to read.
const PROBLEM_TOAST_DURATION: Duration = Duration::from_secs(10);

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The UI scale of this run; see `centered`.
static SCALE: std::sync::OnceLock<f32> = std::sync::OnceLock::new();

/// The listening socket of this (primary) instance, handed to the
/// subscription that watches it.
static INSTANCE_SOCKET: std::sync::Mutex<Option<std::os::unix::net::UnixListener>> =
    std::sync::Mutex::new(None);

fn main() -> iced::Result {
    let args = env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned());
    let command = match cli::parse(args) {
        Ok(command) => command,
        Err(e) => {
            eprintln!("revmo: {e}\nRun `revmo --help` to see the commands.");
            std::process::exit(2);
        }
    };
    // Everything but --help and --version reads files under $HOME.
    let needs_home = !matches!(command, cli::Command::Help | cli::Command::Version);
    if needs_home && env::var_os("HOME").is_none_or(|home| home.is_empty()) {
        eprintln!("revmo: $HOME is not set");
        std::process::exit(1);
    }
    if !command.opens_window() {
        // Before the instance check, so a running launcher stays open.
        cli::run(command);
        return Ok(());
    }
    let is_menu = matches!(command, cli::Command::Menu { .. });
    if is_menu && std::io::stdin().is_terminal() {
        eprintln!("revmo: menu picks from the lines on stdin, e.g. `ls | revmo menu`");
        std::process::exit(2);
    }
    // Without one, iced fails to start with a panic and a backtrace hint.
    let has_display = ["WAYLAND_DISPLAY", "WAYLAND_SOCKET", "DISPLAY"]
        .iter()
        .any(|var| env::var_os(var).is_some_and(|v| !v.is_empty()));
    if !has_display {
        eprintln!("revmo: no display to open a window on (DISPLAY and WAYLAND_DISPLAY are unset)");
        std::process::exit(1);
    }

    // A menu belongs to the script that started it: it neither closes an
    // open launcher nor is closed by the next `revmo`.
    if !is_menu {
        match instance::acquire(&instance::socket_path(&APP_DIR)) {
            Some(instance::Instance::Secondary) => return Ok(()),
            Some(instance::Instance::Primary(listener)) => {
                *INSTANCE_SOCKET.lock().unwrap() = Some(listener);
            }
            None => {}
        }
    }

    let loaded = config::Config::load();
    let (font, font_problem) = check_font(&loaded.config.font);
    styles::init_fonts(&font);
    let scale = loaded
        .config
        .scale
        .unwrap_or_else(revmo::display::auto_scale);
    SCALE.get_or_init(|| scale);

    let settings = window::settings::Settings {
        size: iced::Size::new(styles::WINDOW_WIDTH, WINDOW_HEIGHT),
        position: window::Position::SpecificWith(centered),
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
        ..window::Settings::default()
    };
    let mut app = iced::application(
        move || Revmo::new(loaded.clone(), scale, font_problem.clone(), command.clone()),
        Revmo::update,
        Revmo::view,
    )
    .title("REVMO")
    .settings(Settings {
        id: Some(String::from("revmo")),
        ..Settings::default()
    })
    .window(settings)
    .theme(Revmo::theme)
    // Scales everything, the window size included.
    .scale_factor(move |_| scale)
    .default_font(fonts().regular)
    .subscription(Revmo::subscription);
    for font in styles::BUNDLED_FONTS {
        app = app.font(font);
    }
    app.run()?;
    if is_menu {
        std::process::exit(menu::EXIT_CODE.load(Ordering::Relaxed));
    }
    Ok(())
}

struct Revmo {
    config: config::Config,
    /// What's wrong in the config file, one line each.
    config_problems: Vec<String>,
    /// Whether this run's scale was picked from the monitor.
    scale_auto: bool,
    /// Whether to report problems once the theme files are read.
    report_pending: bool,
    search: String,
    selected: usize,
    /// Every theme: the built-in ones, then the bundled ones and the theme
    /// files', by name.
    themes: Vec<ThemeDef>,
    theme: ThemeDef,
    colors: Colors,
    /// The UI scale this run uses, from the config or the monitor.
    scale: f32,
    iced_theme: iced::Theme,
    batch_size: usize,
    /// Every app and action not hidden in the config, most relevant first.
    apps: Vec<App>,
    /// Search data for each of `apps`.
    haystacks: Vec<search::Haystack>,
    /// For each of `apps`, the index of the app it's listed under.
    parents: Vec<usize>,
    /// Indices into `apps` of the apps matching the search, in display order.
    visible: Vec<usize>,
    toast: Option<(String, ToastLevel)>,
    /// Incremented per toast, so a timer only clears the toast it was started for.
    toast_id: u64,
    help: bool,
    /// The selected item of the actions pane, while it's open.
    pane: Option<usize>,
    /// The app or action whose arguments are being entered.
    args_key: Option<String>,
    app_args: Vec<FieldCode>,
    exec_args: Vec<String>,
    usage: usage::Usage,
    is_loading: bool,
    /// Set once the window has had focus; losing it after that closes revmo.
    was_focused: bool,
    /// When a held arrow key last moved the selection (see `REPEAT_GAP`).
    last_repeat: Option<Instant>,
    /// The line menu shown instead of the apps, in `revmo menu` and
    /// clipboard mode.
    menu: Option<Menu>,
    /// The modifier keys held, which change what enter does in a menu.
    modifiers: keyboard::Modifiers,
    /// The selected mode while the modes screen is open (see `list`).
    modes: Option<usize>,
}

#[derive(Clone, PartialEq)]
enum ToastLevel {
    Error,
}

#[derive(Debug, Clone)]
enum Message {
    AppsLoaded(Vec<App>),
    /// The theme files, if they were (re)read.
    ThemesScanned(Option<Vec<ThemeDef>>),
    SearchChanged(String),
    RunIndex,
    SelectDown,
    SelectUp,
    /// A key repeat of ↓ (`true`) or ↑ while the key is held.
    RepeatStep(bool),
    /// → : opens the actions pane, or the arguments of its selected item.
    Forward,
    /// ← : closes the arguments, or the actions pane.
    Back,
    /// Esc: like `Back`, or closes revmo when there's nothing to go back from.
    Escape,
    ArgsChange(usize, String),
    /// Apps are named by key: ids would change if a rescan lands in between.
    RunWithArgs(String, Vec<String>),
    RunPress(String),
    RunFailed,
    Exit,
    HelpPage,
    ToastDone(u64),
    Junk(()),
    ForwardTheme,
    BackwardTheme,
    WindowFocused,
    WindowUnfocused,
    ModifiersChanged(keyboard::Modifiers),
    /// Lines read from stdin for `revmo menu`; `true` once it's all read.
    MenuLines(Vec<menu::Line>, bool),
    /// Enter in a menu, with the modifier keys held then.
    MenuEnter(keyboard::Modifiers),
    /// A click on the menu line with this index.
    MenuPress(usize),
    /// A click on the mode at this position on the modes screen.
    ModePress(usize),
    /// Tab: copies the selected menu line into the input.
    Complete,
    ClipboardListed(Result<Vec<String>, String>),
    ClipboardPicked(Result<(), String>),
}

impl Revmo {
    /// Opens with the cached apps and themes right away, then checks in the
    /// background whether they're stale; if so they update once scanned.
    fn new(
        mut loaded: config::Loaded,
        scale: f32,
        font_problem: Option<String>,
        command: cli::Command,
    ) -> (Self, Task<Message>) {
        let (theme_files, scanning) = match theme::cached() {
            Some((fingerprint, files)) => (files, Some(fingerprint)),
            // Few small files: read them now rather than flash another theme.
            None => (theme::rescan(), None),
        };
        loaded.problems.extend(font_problem);
        let scale_auto = loaded.config.scale.is_none();
        let mut revmo = Revmo::_new(loaded, theme_files);
        revmo.scale = scale;
        revmo.scale_auto = scale_auto;
        // The configured theme may be in files newer than the cache, so
        // report once those are checked.
        let themes_task = match scanning {
            Some(fingerprint) => {
                revmo.report_pending = true;
                scan_themes_if_changed(Some(fingerprint))
            }
            None => revmo.report(),
        };
        // A menu has its lines; apps aren't needed.
        if let cli::Command::Menu { prompt, columns } = command {
            revmo.menu = Some(Menu::new(menu::Kind::Stdin, prompt, columns));
            revmo.is_loading = false;
            return (revmo, Task::batch([themes_task, focus(INPUT.clone())]));
        }
        let mode = match command {
            cli::Command::Clipboard => Task::batch([
                revmo.update(Message::SearchChanged(clipboard::PREFIX.into())),
                // The apps may take a while to load, and with them the focus.
                focus(INPUT.clone()),
            ]),
            _ => Task::none(),
        };
        let Some((fingerprint, apps)) = App::cached() else {
            // First run or unreadable cache: show "Loading" until scanned.
            return (
                revmo,
                Task::batch([mode, themes_task, scan_if_changed(None)]),
            );
        };
        let shown = revmo.update(Message::AppsLoaded(apps));
        (
            revmo,
            Task::batch([mode, shown, themes_task, scan_if_changed(Some(fingerprint))]),
        )
    }

    fn _new(loaded: config::Loaded, theme_files: Vec<ThemeDef>) -> Self {
        let themes = theme::with_built_in(theme_files);
        let theme = themes[0].clone();
        let mut revmo = Self {
            config: loaded.config,
            config_problems: loaded.problems,
            scale_auto: true,
            report_pending: false,
            search: String::new(),
            selected: 0,
            colors: theme.colors(),
            scale: 1.0,
            iced_theme: theme.to_iced(),
            theme,
            themes,
            batch_size: BATCH_SIZE,
            apps: vec![],
            haystacks: vec![],
            parents: vec![],
            visible: vec![],
            toast: None,
            toast_id: 0,
            help: false,
            pane: None,
            args_key: None,
            app_args: Vec::new(),
            exec_args: Vec::new(),
            usage: usage::Usage::load(&USAGE_FILE),
            is_loading: true,
            was_focused: false,
            last_repeat: None,
            menu: None,
            modifiers: keyboard::Modifiers::default(),
            modes: None,
        };
        revmo.apply_theme();
        revmo
    }

    /// Launches the app with the given key and closes revmo on success.
    fn run(&mut self, key: &str, inputs: &[String]) -> Task<Message> {
        let Some(app) = self.apps.iter().find(|a| a.key == key) else {
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
        self.notify_for(msg, level, TOAST_DURATION)
    }

    fn notify_for(&mut self, msg: String, level: ToastLevel, duration: Duration) -> Task<Message> {
        self.toast_id += 1;
        self.toast = Some((msg, level));
        let id = self.toast_id;
        Task::perform(
            helpers::on_thread(move || thread::sleep(duration)),
            move |_| Message::ToastDone(id),
        )
    }

    /// Everything wrong with the settings: the config file's mistakes and
    /// a theme that doesn't exist.
    fn problems(&self) -> Vec<String> {
        let mut problems = self.config_problems.clone();
        if theme::by_name(&self.themes, &self.config.theme).is_none() {
            problems.push(format!(
                "theme \"{}\" not found; using {}",
                self.config.theme, self.theme.name
            ));
        }
        problems
    }

    /// Toasts the first problem; the help page lists them all.
    fn report(&mut self) -> Task<Message> {
        let problems = self.problems();
        for problem in &problems {
            eprintln!("revmo: config: {problem}");
        }
        match problems.first() {
            Some(first) => {
                let more = match problems.len() {
                    1 => String::new(),
                    n => format!(" (+{} more, ctrl+h)", n - 1),
                };
                self.notify_for(
                    format!("config: {first}{more}"),
                    ToastLevel::Error,
                    PROBLEM_TOAST_DURATION,
                )
            }
            None => Task::none(),
        }
    }

    /// Shows the configured theme, or the default one if it's gone.
    fn apply_theme(&mut self) {
        let theme = theme::by_name(&self.themes, &self.config.theme)
            .unwrap_or(&self.themes[0])
            .clone();
        self.colors = theme.colors();
        self.iced_theme = theme.to_iced();
        self.theme = theme;
    }

    /// Switches theme and saves it, unless saving would lose what's in
    /// the config file.
    fn set_theme(&mut self, name: String) -> Task<Message> {
        self.config.theme = name;
        self.apply_theme();
        // Refuses, with the reason, when the file can't be read or
        // understood, so it's never written over.
        match config::save_theme(&self.config.theme) {
            Ok(()) => Task::none(),
            Err(e) => self.notify(format!("theme not saved: {e}"), ToastLevel::Error),
        }
    }

    /// Replaces the apps, keeping their search data and grouping in step.
    fn set_apps(&mut self, apps: Vec<App>) {
        self.haystacks = apps.iter().map(App::haystack).collect();
        self.parents = parser::parents(&apps);
        self.apps = apps;
    }

    /// The selected app (never an action: those are in the actions pane).
    fn selected_app(&self) -> Option<&App> {
        self.visible.get(self.selected).map(|&i| &self.apps[i])
    }

    /// The actions pane's entries for the selected app, as indices into
    /// `apps`: the app itself, then its actions.
    fn pane_items(&self) -> Vec<usize> {
        let Some(&group) = self.visible.get(self.selected) else {
            return Vec::new();
        };
        let mut items = vec![group];
        items.extend((0..self.apps.len()).filter(|&i| i != group && self.parents[i] == group));
        items
    }

    fn selected_pane_item(&self) -> Option<usize> {
        self.pane.and_then(|i| self.pane_items().get(i).copied())
    }

    /// Recomputes the visible apps from the search text: best matches
    /// first, with frequently used apps winning close calls. An app ranks
    /// by its best match among itself and its actions.
    fn filter(&mut self) {
        let now = usage::now();
        let scores: Vec<Option<f64>> = self
            .haystacks
            .iter()
            .enumerate()
            .map(|(i, h)| {
                let app = &self.apps[i];
                let mut score = h.score(&self.search)?;
                if app.action.is_some() {
                    score *= ACTION_WEIGHT;
                }
                let usage = self.usage.score(&app.key, now);
                Some(score + FREQUENCY_WEIGHT * usage.ln_1p())
            })
            .collect();
        self.visible = parser::rank_groups(&scores, &self.parents);
        self.selected = 0;
    }

    fn close_args(&mut self) {
        self.args_key = None;
        self.app_args.clear();
        self.exec_args.clear();
    }

    /// Switches to the mode with `prefix`, searching for `query`. The
    /// prefix becomes the mode's sign before the input.
    fn enter_mode(&mut self, prefix: char, query: &str) -> Task<Message> {
        self.modes = None;
        self.close_args();
        match prefix {
            clipboard::PREFIX => {
                let mut m = Menu::new(
                    menu::Kind::Clipboard,
                    Some(prefix.to_string()),
                    self.config.clipboard_columns.clone(),
                );
                m.set_query(query);
                self.menu = Some(m);
                self.search = query.to_string();
                let command = self.config.clipboard_list.clone();
                Task::perform(
                    helpers::on_thread(move || clipboard::list(&command)),
                    Message::ClipboardListed,
                )
            }
            _ => Task::none(),
        }
    }

    /// Back to the apps from a mode or the modes screen.
    fn leave_mode(&mut self) {
        self.menu = None;
        self.modes = None;
        self.search.clear();
        self.filter();
    }

    /// Enter in a menu, with `modifiers` held. `revmo menu` prints the
    /// selected line, or what's typed if nothing matches or with shift;
    /// with ctrl it prints the line and stays open. Clipboard mode copies
    /// the selected entry.
    fn pick_line(&mut self, modifiers: keyboard::Modifiers) -> Task<Message> {
        let typed = self.search.clone();
        let (shift, ctrl) = (modifiers.shift(), modifiers.command());
        let Some(menu) = &mut self.menu else {
            return Task::none();
        };
        let selected = menu.selected_line().cloned();
        if menu.kind == menu::Kind::Clipboard {
            let Some(line) = selected.filter(|_| !menu.picking) else {
                return Task::none();
            };
            menu.picking = true;
            let command = self.config.clipboard_pick.clone();
            let line = line.text().to_string();
            return Task::perform(
                helpers::on_thread(move || clipboard::pick(&command, &line)),
                Message::ClipboardPicked,
            );
        }
        match (selected, shift, ctrl) {
            (_, true, _) | (None, _, false) if !typed.is_empty() => {
                menu::print(typed.as_bytes());
                Task::done(Message::Exit)
            }
            (Some(line), false, true) => {
                if !menu::print(line.bytes()) {
                    return Task::done(Message::Exit);
                }
                menu.mark_picked();
                Task::none()
            }
            (Some(line), false, false) => {
                menu::print(line.bytes());
                Task::done(Message::Exit)
            }
            _ => Task::none(),
        }
    }

    fn theme(&self) -> iced::Theme {
        self.iced_theme.clone()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AppsLoaded(mut apps) => {
                // This can arrive while the list is in use (a background
                // rescan), so keep the selection, the actions pane and the
                // arguments.
                let selected = self.selected_app().map(|a| a.key.clone());
                let args = self.args_key.as_ref().and_then(|key| {
                    let app = self.apps.iter().find(|a| &a.key == key)?;
                    Some((key.clone(), app.arg_codes()))
                });
                let was_loading = self.is_loading;
                apps.retain(|a| !a.is_hidden(&self.config.hidden));
                // Stable sort: apps never launched stay alphabetical.
                let now = usage::now();
                apps.sort_by(|a, b| {
                    let score = |app: &App| self.usage.score(&app.key, now);
                    score(b).total_cmp(&score(a))
                });
                self.set_apps(apps);
                self.is_loading = false;
                self.filter();
                if let Some(key) = selected {
                    let pos = self.visible.iter().position(|&i| self.apps[i].key == key);
                    match pos {
                        Some(pos) => self.selected = pos,
                        None => self.pane = None,
                    }
                }
                let items = self.pane_items().len();
                self.pane = self.pane.filter(|_| items > 0).map(|i| i.min(items - 1));
                if let Some((key, codes)) = args {
                    // Its inputs only make sense for the same app and arguments.
                    let same = self.pane.is_some()
                        && self.apps.iter().find(|a| a.key == key).map(App::arg_codes)
                            == Some(codes);
                    if !same {
                        self.close_args();
                        return focus(INPUT.clone());
                    }
                }
                if was_loading {
                    focus(INPUT.clone()).map(Message::Junk)
                } else {
                    Task::none()
                }
            }

            Message::ThemesScanned(files) => {
                if let Some(files) = files {
                    self.themes = theme::with_built_in(files);
                    self.apply_theme();
                }
                match std::mem::take(&mut self.report_pending) {
                    true => self.report(),
                    false => Task::none(),
                }
            }

            Message::SearchChanged(input) => {
                self.pane = None;
                let kind = self.menu.as_ref().map(|m| m.kind);
                // Backspace with nothing typed leaves a mode or the modes
                // screen: the input reports it as its empty text, unchanged.
                let in_mode = kind == Some(menu::Kind::Clipboard) || self.modes.is_some();
                if in_mode && input.is_empty() && self.search.is_empty() {
                    self.leave_mode();
                    return Task::none();
                }
                if kind.is_none() {
                    if self.modes.is_none()
                        && let Some(rest) = input.strip_prefix(list::PREFIX)
                    {
                        self.modes = Some(0);
                        self.search.clear();
                        self.close_args();
                        // Typed on along with the `?`, e.g. pasted.
                        return match rest.is_empty() {
                            true => Task::none(),
                            false => self.update(Message::SearchChanged(rest.to_string())),
                        };
                    }
                    if let Some((mode, query)) = list::split(&input) {
                        return self.enter_mode(mode.prefix, query);
                    }
                    // Anything else typed on the modes screen searches apps.
                    self.modes = None;
                }
                self.search = input;
                match &mut self.menu {
                    Some(m) => m.set_query(&self.search),
                    None => self.filter(),
                }
                Task::none()
            }

            Message::MenuLines(lines, done) => {
                if let Some(m) = self.menu.as_mut().filter(|m| m.kind == menu::Kind::Stdin) {
                    m.push(lines);
                    m.loading = !done;
                }
                Task::none()
            }

            Message::ClipboardListed(result) => {
                // Clipboard mode may have been left since.
                if let Some(m) = self
                    .menu
                    .as_mut()
                    .filter(|m| m.kind == menu::Kind::Clipboard)
                {
                    m.loading = false;
                    match result {
                        Ok(lines) => {
                            m.error = None;
                            m.set_lines(lines);
                        }
                        Err(e) => m.error = Some(e),
                    }
                }
                Task::none()
            }

            Message::ClipboardPicked(Ok(())) => Task::done(Message::Exit),
            Message::ClipboardPicked(Err(e)) => {
                if let Some(m) = &mut self.menu {
                    m.picking = false;
                }
                // The pick may have been a click, which took the focus.
                Task::batch([self.notify(e, ToastLevel::Error), focus(INPUT.clone())])
            }

            Message::MenuEnter(modifiers) => self.pick_line(modifiers),

            Message::ModePress(i) => {
                let Some(mode) = list::MODES.get(i).filter(|_| self.modes.is_some()) else {
                    return Task::none();
                };
                // Clicking took the focus from the input.
                Task::batch([self.enter_mode(mode.prefix, ""), focus(INPUT.clone())])
            }

            Message::MenuPress(line) => {
                // The list may have changed under the pointer.
                if !self.menu.as_mut().is_some_and(|m| m.select_line(line)) {
                    return Task::none();
                }
                // Shift picks what's typed; a click is about the line.
                let modifiers = self.modifiers - keyboard::Modifiers::SHIFT;
                // Clicking took the focus from the input.
                Task::batch([self.pick_line(modifiers), focus(INPUT.clone())])
            }

            Message::Complete => {
                let Some(m) = self.menu.as_mut().filter(|m| m.kind == menu::Kind::Stdin) else {
                    return Task::none();
                };
                // What's shown, as that's what the search looks at.
                let Some(line) = m.selected_line().map(|l| l.shown().to_string()) else {
                    return Task::none();
                };
                m.set_query(&line);
                self.search = line;
                move_cursor_to_end(INPUT.clone())
            }

            Message::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers;
                Task::none()
            }

            Message::RunIndex if self.modes.is_some() => {
                let i = self.modes.unwrap_or_default();
                self.enter_mode(list::MODES[i].prefix, "")
            }
            Message::RunIndex => match (self.pane.is_some(), self.selected_pane_item()) {
                (true, Some(i)) => self.run(&self.apps[i].key.clone(), &[]),
                (false, _) => match self.selected_app() {
                    Some(app) => self.run(&app.key.clone(), &[]),
                    None => Task::done(Message::RunFailed),
                },
                (true, None) => Task::done(Message::RunFailed),
            },

            Message::SelectDown => {
                if let Some(m) = &mut self.menu {
                    m.select_down();
                    return Task::none();
                }
                if let Some(i) = self.modes {
                    self.modes = Some((i + 1).min(list::MODES.len() - 1));
                    return Task::none();
                }
                if self.args_key.is_some() {
                    return focus_next();
                }
                if let Some(i) = self.pane {
                    if i + 1 < self.pane_items().len() {
                        self.pane = Some(i + 1);
                    }
                } else if self.selected + 1 < self.visible.len() {
                    self.selected += 1;
                }
                Task::none()
            }
            Message::SelectUp => {
                if let Some(m) = &mut self.menu {
                    m.select_up();
                    return Task::none();
                }
                if let Some(i) = self.modes {
                    self.modes = Some(i.saturating_sub(1));
                    return Task::none();
                }
                if self.args_key.is_some() {
                    return focus_previous();
                }
                if let Some(i) = self.pane {
                    self.pane = Some(i.saturating_sub(1));
                } else if self.selected > 0 {
                    self.selected -= 1;
                }
                Task::none()
            }

            Message::RepeatStep(down) => {
                let now = Instant::now();
                if self
                    .last_repeat
                    .is_some_and(|t| now.duration_since(t) < REPEAT_GAP)
                {
                    return Task::none();
                }
                self.last_repeat = Some(now);
                self.update(if down {
                    Message::SelectDown
                } else {
                    Message::SelectUp
                })
            }

            Message::ArgsChange(i, arg) => {
                // A rescan may have closed the page since this was typed.
                if let Some(input) = self.exec_args.get_mut(i) {
                    *input = arg;
                }
                Task::none()
            }

            Message::Forward => {
                // Menu lines and modes have no actions.
                if self.menu.is_some()
                    || self.modes.is_some()
                    || self.args_key.is_some()
                    || self.selected_app().is_none()
                {
                    return Task::none();
                }
                let Some(i) = self.selected_pane_item() else {
                    if self.pane.is_none() {
                        self.pane = Some(0);
                    }
                    return Task::none();
                };
                let app = &self.apps[i];
                self.app_args = app.arg_codes();
                if self.app_args.is_empty() {
                    return Task::none();
                }
                self.exec_args = vec![String::new(); self.app_args.len()];
                self.args_key = Some(app.key.clone());
                focus("0")
            }

            Message::Back => {
                if self.modes.is_some() {
                    self.leave_mode();
                } else if self.args_key.is_some() {
                    self.close_args();
                } else {
                    self.pane = None;
                }
                focus(INPUT.clone())
            }

            Message::Escape => {
                if self.args_key.is_some() || self.pane.is_some() || self.modes.is_some() {
                    self.update(Message::Back)
                } else {
                    Task::done(Message::Exit)
                }
            }

            Message::RunWithArgs(key, args) => self.run(&key, &args),

            Message::RunPress(key) => self.run(&key, &[]),

            Message::HelpPage => {
                self.help = !self.help;
                // The help page replaces the page underneath, fields and
                // all, so their focus is gone when it closes.
                match (self.help, self.args_key.is_some()) {
                    (true, _) => Task::none(),
                    (false, true) => focus("0"),
                    (false, false) => focus(INPUT.clone()),
                }
            }

            Message::ToastDone(id) => {
                if id == self.toast_id {
                    self.toast = None;
                }
                Task::none()
            }

            Message::Exit => window::latest().and_then(window::close),

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

            Message::ForwardTheme => {
                let name = theme::next(&self.themes, &self.theme.name).name.clone();
                self.set_theme(name)
            }

            Message::BackwardTheme => {
                let name = theme::previous(&self.themes, &self.theme.name).name.clone();
                self.set_theme(name)
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let stdin = match &self.menu {
            Some(m) if m.kind == menu::Kind::Stdin => Subscription::run(menu::read_stdin),
            _ => Subscription::none(),
        };
        Subscription::batch([
            Subscription::run(other_instances),
            event::listen_with(handle_event),
            stdin,
        ])
    }

    fn view(&self) -> Element<'_, Message> {
        let c = self.colors;
        let content: Element<Message> = if self.is_loading && self.menu.is_none() {
            container(styles::small("loading apps…").color(c.text_muted))
                .center(Fill)
                .into()
        } else if self.help {
            help_page::view(self)
        } else {
            column![
                self.input_row(),
                styles::divider(c),
                self.body(),
                styles::divider(c),
                self.footer(),
            ]
            .into()
        };
        styles::frame_edges(
            c,
            container(content)
                .width(Fill)
                .height(Fill)
                .style(move |_| styles::frame(c)),
        )
    }

    fn input_row(&self) -> Element<'_, Message> {
        let c = self.colors;
        let placeholder = match (&self.menu, self.modes) {
            (Some(m), _) if m.kind == menu::Kind::Stdin => "type to filter",
            (Some(_), _) => "type to search the clipboard history",
            (None, Some(_)) => "pick one, or type its prefix",
            (None, None) => "type to search apps",
        };
        let input = text_input(placeholder, &self.search)
            .id(INPUT.clone())
            .on_input(Message::SearchChanged)
            // A menu's enter comes with its modifiers (see `handle_event`).
            .on_submit_maybe(self.menu.is_none().then_some(Message::RunIndex))
            .style(move |_, _| styles::search_input(c))
            .font(fonts().regular)
            .size(styles::INPUT_SIZE)
            .padding(0);
        // Until something's typed, the launcher points to the modes.
        let right: Element<Message> = match (&self.menu, self.modes) {
            (None, None) if self.search.is_empty() => styles::hint(c, "? modes"),
            _ => Space::new().into(),
        };
        let sign: Element<Message> = match (&self.menu, self.modes) {
            (_, Some(_)) => list::badge(c),
            (menu, None) => text(
                menu.as_ref()
                    .and_then(|m| m.prompt.as_deref())
                    .unwrap_or("›"),
            )
            .size(styles::INPUT_SIZE)
            .font(fonts().medium)
            .color(c.accent)
            .into(),
        };
        container(
            row![sign, input, right]
                .spacing(10)
                .align_y(Alignment::Center),
        )
        .padding([0.0, styles::PAD_X])
        .height(styles::INPUT_HEIGHT)
        .align_y(Alignment::Center)
        .into()
    }

    /// The part between the input row and the footer.
    fn body(&self) -> Element<'_, Message> {
        if let Some(selected) = self.modes {
            return container(list::view(self, selected))
                .height(LIST_HEIGHT)
                .into();
        }
        if let Some(m) = &self.menu {
            return container(menu::view(self, m, &self.search))
                .height(LIST_HEIGHT)
                .into();
        }
        let args = self
            .args_key
            .as_ref()
            .and_then(|key| self.apps.iter().find(|a| &a.key == key));
        let body = match (args, self.pane) {
            (Some(app), _) => args_page::view(self, app),
            (None, Some(_)) => actions_pane::view(self, self.list()),
            (None, None) => self.list(),
        };
        container(body).height(LIST_HEIGHT).into()
    }

    /// The current page of results.
    fn list(&self) -> Element<'_, Message> {
        let c = self.colors;
        let (_, _, selected, apps) =
            batch::get_batch(&self.visible, self.batch_size, self.selected);
        let pane = self.pane.is_some();
        let rows = apps.iter().enumerate().map(|(i, &index)| {
            let state = match (i == selected, pane) {
                (true, false) => helpers::RowState::Selected,
                (true, true) => helpers::RowState::Marked,
                (false, false) => helpers::RowState::Normal,
                (false, true) => helpers::RowState::Dimmed,
            };
            helpers::app_row(&self.apps[index], &self.search, state, !pane, c)
        });
        let mut list = column(rows);
        if self.visible.is_empty() && !self.search.trim().is_empty() {
            list = list.push(
                container(
                    text("no apps match")
                        .size(styles::ROW_SIZE)
                        .font(fonts().regular)
                        .color(c.text_muted),
                )
                .padding([0.0, styles::PAD_X])
                .height(styles::ROW_HEIGHT)
                .align_y(Alignment::Center),
            );
        }
        container(list)
            .padding(Padding::from([styles::LIST_PADDING, 0.0]))
            .width(Fill)
            .into()
    }

    /// Where the selection is in the list, e.g. "3 / 54", for the footer.
    fn position(&self) -> String {
        let (total, selected) = match &self.menu {
            Some(m) => (m.len(), m.selected),
            None => (self.visible.len(), self.selected),
        };
        match total {
            0 => "0 / 0".to_string(),
            total => format!("{} / {total}", selected + 1),
        }
    }

    fn footer(&self) -> Element<'_, Message> {
        let c = self.colors;
        if let Some((msg, level)) = &self.toast {
            return helpers::toast(msg, level, c);
        }
        if let Some(m) = &self.menu {
            return menu::footer(m, self.position(), c);
        }
        if self.modes.is_some() {
            return list::footer(c);
        }
        let hint = |s| styles::hint(c, s);
        if self.args_key.is_some() {
            return styles::footer_bar(
                c,
                [hint("↲ run"), hint("↑↓ move"), hint("← back")],
                styles::small("arguments").color(c.text_faint).into(),
            );
        }
        if self.pane.is_some() {
            let name = self.selected_app().map(|a| a.name.as_str()).unwrap_or("");
            return styles::footer_bar(
                c,
                [styles::small(format!("actions: {name}"))
                    .color(c.text_muted)
                    .into()],
                hint("← back"),
            );
        }
        styles::footer_bar(
            c,
            [
                hint("↑↓ move"),
                hint("↲ run"),
                hint("→ actions"),
                hint("ctrl+h help"),
                hint("esc close"),
            ],
            styles::small(self.position()).color(c.text_muted).into(),
        )
    }
}

/// The window's spot in the middle of the monitor. iced's own `Centered`
/// leaves out the scale factor, putting a scaled window off-center.
fn centered(window: iced::Size, monitor: iced::Size) -> iced::Point {
    let scale = SCALE.get().copied().unwrap_or(1.0);
    iced::Point::new(
        ((monitor.width - window.width * scale) / 2.0).max(0.0),
        ((monitor.height - window.height * scale) / 2.0).max(0.0),
    )
}

/// Rescans off the UI thread if the apps changed since `fingerprint`.
fn scan_if_changed(fingerprint: Option<u64>) -> Task<Message> {
    Task::perform(
        helpers::on_thread(move || App::scan_if_changed(fingerprint)),
        |apps| apps,
    )
    .and_then(|apps| Task::done(Message::AppsLoaded(apps)))
}

/// Rereads the theme files off the UI thread if they changed since
/// `fingerprint`.
fn scan_themes_if_changed(fingerprint: Option<u64>) -> Task<Message> {
    Task::perform(
        helpers::on_thread(move || theme::scan_if_changed(fingerprint)),
        Message::ThemesScanned,
    )
}

/// The font family to use for `family` from the config, spelled as
/// installed: empty for the bundled one, which is also used (with the
/// problem) if `family` isn't installed.
fn check_font(family: &str) -> (String, Option<String>) {
    let family = family.trim();
    if family.is_empty() || family.eq_ignore_ascii_case(styles::BUNDLED_FAMILY) {
        return (String::new(), None);
    }
    match styles::installed_family(family) {
        Some(installed) => (installed, None),
        None => (
            String::new(),
            Some(format!(
                "font \"{family}\" isn't installed; using {}",
                styles::BUNDLED_FAMILY
            )),
        ),
    }
}

fn handle_event(e: Event, _: event::Status, _: window::Id) -> Option<Message> {
    match e {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key,
            physical_key,
            modifiers,
            repeat,
            ..
        }) => match key {
            Key::Named(Named::ArrowDown) if repeat => Some(Message::RepeatStep(true)),
            Key::Named(Named::ArrowUp) if repeat => Some(Message::RepeatStep(false)),
            Key::Named(Named::ArrowDown) => Some(Message::SelectDown),
            Key::Named(Named::ArrowUp) => Some(Message::SelectUp),
            Key::Named(Named::Escape) => Some(Message::Escape),
            Key::Named(Named::ArrowRight) if modifiers.command() => Some(Message::ForwardTheme),
            Key::Named(Named::ArrowRight) => Some(Message::Forward),
            Key::Named(Named::ArrowLeft) if modifiers.command() => Some(Message::BackwardTheme),
            Key::Named(Named::ArrowLeft) => Some(Message::Back),
            Key::Named(Named::Tab) => Some(Message::Complete),
            // Only used in a menu; elsewhere the input's submit is enter.
            Key::Named(Named::Enter) => Some(Message::MenuEnter(modifiers)),
            // The key that types h; on a layout without Latin letters
            // (Arabic, Russian, ...), the key in h's place.
            Key::Character(c)
                if modifiers.command()
                    && (c.as_str() == "h"
                        || !c.is_ascii() && physical_key == Physical::Code(Code::KeyH)) =>
            {
                Some(Message::HelpPage)
            }
            _ => None,
        },
        Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
            Some(Message::ModifiersChanged(modifiers))
        }
        Event::Window(window::Event::Focused) => Some(Message::WindowFocused),
        Event::Window(window::Event::Unfocused) => Some(Message::WindowUnfocused),
        _ => None,
    }
}

/// Emits `Exit` whenever another `revmo` starts (see instance.rs).
fn other_instances() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(1, async move |output| {
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
