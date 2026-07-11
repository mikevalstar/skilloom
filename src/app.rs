//! Application state and input handling.
//!
//! Kept free of terminal I/O so it's unit-testable: `on_key`/`on_mouse` mutate
//! state and set flags (`should_quit`, `save_requested`); the runner in `main`
//! performs the actual drawing and persistence.

use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::complete;
use crate::config::Config;
use crate::paths;
use crate::skills;
use crate::ui;

/// The major tabs across the top of the main app.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tab {
    #[default]
    Dashboard,
    Projects,
    Global,
    Catalog,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Dashboard, Tab::Projects, Tab::Global, Tab::Catalog];

    pub fn title(self) -> &'static str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::Projects => "Projects",
            Tab::Global => "Global",
            Tab::Catalog => "Catalog",
        }
    }

    /// Placeholder body shown while each tab is still a stub.
    pub fn placeholder(self) -> &'static str {
        match self {
            Tab::Dashboard => "Dashboard — hello world",
            Tab::Projects => "Projects — hello world",
            Tab::Global => "Global — hello world",
            Tab::Catalog => "Catalog — hello world",
        }
    }

    pub fn index(self) -> usize {
        Tab::ALL.iter().position(|t| *t == self).unwrap_or(0)
    }
}

/// Which top-level screen is showing.
pub enum Screen {
    Setup(SetupState),
    Main(MainState),
}

/// First-run screen: a path field with directory autocomplete.
pub struct SetupState {
    pub input: String,
    pub suggestions: Vec<String>,
    pub error: Option<String>,
}

impl SetupState {
    pub fn new(initial: String) -> Self {
        let mut state = SetupState {
            input: initial,
            suggestions: Vec::new(),
            error: None,
        };
        state.refresh();
        state
    }

    fn refresh(&mut self) {
        self.suggestions = complete::complete_dirs(&self.input);
    }

    fn type_char(&mut self, c: char) {
        self.input.push(c);
        self.error = None;
        self.refresh();
    }

    fn backspace(&mut self) {
        self.input.pop();
        self.error = None;
        self.refresh();
    }

    /// Tab-completion: fill to the longest common directory prefix, or all the
    /// way if there's a single match.
    fn complete(&mut self) {
        if self.suggestions.is_empty() {
            return;
        }
        if self.suggestions.len() == 1 {
            self.input = format!("{}/", self.suggestions[0]);
        } else if let Some(prefix) = complete::common_prefix(&self.suggestions) {
            self.input = prefix;
        }
        self.error = None;
        self.refresh();
    }
}

/// Main app: the active tab, and whether the settings page is open over it.
#[derive(Default)]
pub struct MainState {
    pub active: Tab,
    pub settings_open: bool,
}

impl MainState {
    pub fn new() -> Self {
        MainState {
            active: Tab::Dashboard,
            settings_open: false,
        }
    }

    fn next_tab(&mut self) {
        self.select((self.active.index() + 1) % Tab::ALL.len());
    }

    fn prev_tab(&mut self) {
        self.select((self.active.index() + Tab::ALL.len() - 1) % Tab::ALL.len());
    }

    fn select(&mut self, index: usize) {
        if let Some(&tab) = Tab::ALL.get(index) {
            self.active = tab;
            self.settings_open = false;
        }
    }

    fn open_settings(&mut self) {
        self.settings_open = true;
    }

    /// Close settings if open; returns whether it was open.
    fn close_settings(&mut self) -> bool {
        let was = self.settings_open;
        self.settings_open = false;
        was
    }
}

/// The whole application.
pub struct App {
    pub screen: Screen,
    pub config: Config,
    pub should_quit: bool,
    /// Set when the config needs to be written; the runner persists and clears it.
    pub save_requested: bool,
    /// Skills found in the global agent dirs (`~/.claude/skills`, …).
    pub global: skills::GlobalScan,
    /// Global left-nav rows (location headers + selectable skills).
    pub global_rows: Vec<skills::NavRow>,
    /// Selected skill index within the Global left-nav.
    pub global_sel: usize,
    /// Skills found in the loom-skills repo (`personal/`, `vendor/`).
    pub repo: skills::RepoScan,
}

impl App {
    pub fn new(config: Config) -> Self {
        let screen = if config.is_configured() {
            Screen::Main(MainState::new())
        } else {
            Screen::Setup(SetupState::new(default_setup_input()))
        };
        let mut app = App {
            screen,
            config,
            should_quit: false,
            save_requested: false,
            global: skills::GlobalScan::default(),
            global_rows: Vec::new(),
            global_sel: 0,
            repo: skills::RepoScan::default(),
        };
        if app.config.is_configured() {
            app.rescan();
        }
        app
    }

    /// Re-read installed skills from disk: the global agent dirs and the repo.
    pub fn rescan(&mut self) {
        self.global = skills::scan_global();
        self.global_rows = skills::nav_rows(&self.global);
        self.repo = self
            .config
            .repo_path
            .as_deref()
            .map(skills::scan_repo)
            .unwrap_or_default();
        let count = skills::skill_count(&self.global_rows);
        if self.global_sel >= count {
            self.global_sel = count.saturating_sub(1);
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        match &self.screen {
            Screen::Setup(_) => self.on_key_setup(key),
            Screen::Main(_) => self.on_key_main(key),
        }
    }

    fn on_key_setup(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.should_quit = true,
            KeyCode::Enter => self.confirm_setup(),
            KeyCode::Tab => {
                if let Some(s) = self.setup_mut() {
                    s.complete();
                }
            }
            KeyCode::Backspace => {
                if let Some(s) = self.setup_mut() {
                    s.backspace();
                }
            }
            KeyCode::Char(c) => {
                if let Some(s) = self.setup_mut() {
                    s.type_char(c);
                }
            }
            _ => {}
        }
    }

    fn on_key_main(&mut self, key: KeyEvent) {
        match key.code {
            // esc/q close the settings page if it's open, otherwise quit.
            KeyCode::Char('q') | KeyCode::Esc => {
                let closed = self
                    .main_mut()
                    .map(MainState::close_settings)
                    .unwrap_or(false);
                if !closed {
                    self.should_quit = true;
                }
            }
            KeyCode::Tab => {
                if let Some(m) = self.main_mut() {
                    m.next_tab();
                }
            }
            KeyCode::BackTab => {
                if let Some(m) = self.main_mut() {
                    m.prev_tab();
                }
            }
            KeyCode::Char(',') => {
                if let Some(m) = self.main_mut() {
                    m.open_settings();
                }
            }
            KeyCode::Char('f') => self.rescan(),
            KeyCode::Up | KeyCode::Char('k') => self.global_select_prev(),
            KeyCode::Down | KeyCode::Char('j') => self.global_select_next(),
            KeyCode::Char(c @ '1'..='4') => {
                if let Some(m) = self.main_mut() {
                    m.select((c as u8 - b'1') as usize);
                }
            }
            _ => {}
        }
    }

    /// Left-click support on the main screen: click a tab or the gear.
    pub fn on_mouse(&mut self, event: MouseEvent, area: Rect) {
        if !matches!(self.screen, Screen::Main(_)) {
            return;
        }
        if event.kind != MouseEventKind::Down(MouseButton::Left) {
            return;
        }
        let regions = ui::regions(area);
        let (tabs, gear) = ui::tab_spans(regions.tabbar);
        let (col, row) = (event.column, event.row);

        if let Some(gear_rect) = gear
            && contains(gear_rect, col, row)
        {
            if let Some(m) = self.main_mut() {
                m.open_settings();
            }
            return;
        }
        for (tab, rect) in tabs {
            if contains(rect, col, row) {
                if let Some(m) = self.main_mut() {
                    m.select(tab.index());
                }
                return;
            }
        }

        // Clicking a skill row in the Global left nav selects it.
        if self.on_global_tab() {
            let (nav, _detail) = ui::global_layout(regions.content);
            if let Some(index) = nav_row_hit(&self.global_rows, nav, col, row) {
                self.global_sel = index;
            }
        }
    }

    fn on_global_tab(&self) -> bool {
        matches!(&self.screen, Screen::Main(m) if m.active == Tab::Global && !m.settings_open)
    }

    fn global_select_next(&mut self) {
        if !self.on_global_tab() {
            return;
        }
        let count = skills::skill_count(&self.global_rows);
        if count > 0 && self.global_sel + 1 < count {
            self.global_sel += 1;
        }
    }

    fn global_select_prev(&mut self) {
        if !self.on_global_tab() {
            return;
        }
        self.global_sel = self.global_sel.saturating_sub(1);
    }

    /// Validate the setup input and, if good, save it and enter the main app.
    fn confirm_setup(&mut self) {
        let Screen::Setup(setup) = &self.screen else {
            return;
        };
        let trimmed = setup.input.trim().to_string();
        if trimmed.is_empty() {
            self.set_setup_error("Enter a path to your loom-skills repo.");
            return;
        }
        let expanded = paths::expand_tilde(&trimmed);
        if !expanded.is_dir() {
            self.set_setup_error(&format!("Not a directory: {}", expanded.display()));
            return;
        }
        self.config.repo_path = Some(expanded.to_string_lossy().into_owned());
        self.save_requested = true;
        self.rescan();
        self.screen = Screen::Main(MainState::new());
    }

    fn set_setup_error(&mut self, msg: &str) {
        if let Some(s) = self.setup_mut() {
            s.error = Some(msg.to_string());
        }
    }

    fn setup_mut(&mut self) -> Option<&mut SetupState> {
        match &mut self.screen {
            Screen::Setup(s) => Some(s),
            _ => None,
        }
    }

    fn main_mut(&mut self) -> Option<&mut MainState> {
        match &mut self.screen {
            Screen::Main(m) => Some(m),
            _ => None,
        }
    }
}

fn default_setup_input() -> String {
    paths::home_dir()
        .map(|h| format!("{}/", h.display()))
        .unwrap_or_default()
}

fn contains(rect: Rect, col: u16, row: u16) -> bool {
    col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}

/// Which skill (flat index) a click at `(col, row)` lands on inside the Global
/// left-nav rect, accounting for the border. `None` if it's a header/empty/gap.
fn nav_row_hit(rows: &[skills::NavRow], nav: Rect, col: u16, row: u16) -> Option<usize> {
    if nav.width < 2 || nav.height < 2 {
        return None;
    }
    let inner_x = nav.x + 1;
    let inner_y = nav.y + 1;
    if col < inner_x || col >= inner_x + (nav.width - 2) {
        return None;
    }
    if row < inner_y || row >= inner_y + (nav.height - 2) {
        return None;
    }
    let visual = (row - inner_y) as usize;
    skills::skill_index_at_line(rows, visual)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn main_app() -> App {
        App::new(Config {
            repo_path: Some("/somewhere".to_string()),
        })
    }

    fn active(app: &App) -> Tab {
        match &app.screen {
            Screen::Main(m) => m.active,
            _ => panic!("not on main screen"),
        }
    }

    #[test]
    fn configured_starts_on_main_dashboard() {
        let app = main_app();
        assert_eq!(active(&app), Tab::Dashboard);
    }

    #[test]
    fn unconfigured_starts_on_setup() {
        let app = App::new(Config::default());
        assert!(matches!(app.screen, Screen::Setup(_)));
    }

    #[test]
    fn tab_and_number_keys_switch_tabs() {
        let mut app = main_app();
        app.on_key(key(KeyCode::Tab));
        assert_eq!(active(&app), Tab::Projects);
        app.on_key(key(KeyCode::BackTab));
        assert_eq!(active(&app), Tab::Dashboard);
        app.on_key(key(KeyCode::Char('3')));
        assert_eq!(active(&app), Tab::Global);
    }

    #[test]
    fn tab_wraps_around() {
        let mut app = main_app();
        for _ in 0..Tab::ALL.len() {
            app.on_key(key(KeyCode::Tab));
        }
        assert_eq!(active(&app), Tab::Dashboard);
    }

    #[test]
    fn comma_opens_settings_and_esc_closes_before_quitting() {
        let mut app = main_app();
        app.on_key(key(KeyCode::Char(',')));
        assert!(matches!(&app.screen, Screen::Main(m) if m.settings_open));
        app.on_key(key(KeyCode::Esc));
        assert!(matches!(&app.screen, Screen::Main(m) if !m.settings_open));
        assert!(!app.should_quit);
        app.on_key(key(KeyCode::Esc));
        assert!(app.should_quit);
    }

    #[test]
    fn setup_typing_and_confirm_valid_dir_enters_main() {
        let tmp = tempfile::tempdir().unwrap();
        let mut app = App::new(Config::default());
        // clear the prefilled home path
        if let Some(s) = app.setup_mut() {
            s.input.clear();
        }
        for c in tmp.path().to_string_lossy().chars() {
            app.on_key(key(KeyCode::Char(c)));
        }
        app.on_key(key(KeyCode::Enter));
        assert!(matches!(app.screen, Screen::Main(_)));
        assert!(app.save_requested);
        assert_eq!(
            app.config.repo_path.as_deref(),
            Some(tmp.path().to_string_lossy().as_ref())
        );
    }

    #[test]
    fn global_selection_moves_only_on_global_tab() {
        let mut app = main_app();
        app.global = skills::GlobalScan {
            groups: vec![skills::SkillGroup {
                label: "L".to_string(),
                skills: vec![
                    skills::SkillEntry::new("a"),
                    skills::SkillEntry::new("b"),
                    skills::SkillEntry::new("c"),
                ],
            }],
        };
        app.global_rows = skills::nav_rows(&app.global);
        app.global_sel = 0;

        // On Dashboard, arrow keys are a no-op for the Global selection.
        app.on_key(key(KeyCode::Down));
        assert_eq!(app.global_sel, 0);

        app.on_key(key(KeyCode::Char('3'))); // switch to Global
        app.on_key(key(KeyCode::Down));
        assert_eq!(app.global_sel, 1);
        app.on_key(key(KeyCode::Char('j')));
        assert_eq!(app.global_sel, 2);
        app.on_key(key(KeyCode::Down)); // clamps at the end
        assert_eq!(app.global_sel, 2);
        app.on_key(key(KeyCode::Up));
        assert_eq!(app.global_sel, 1);
    }

    #[test]
    fn setup_confirm_bad_dir_shows_error() {
        let mut app = App::new(Config::default());
        if let Some(s) = app.setup_mut() {
            s.input = "/no/such/dir/skilloom-xyz".to_string();
        }
        app.on_key(key(KeyCode::Enter));
        match &app.screen {
            Screen::Setup(s) => assert!(s.error.is_some()),
            _ => panic!("should have stayed on setup"),
        }
        assert!(!app.save_requested);
    }
}
