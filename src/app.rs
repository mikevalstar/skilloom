//! Application state and input handling.
//!
//! Kept free of terminal I/O so it's unit-testable: `on_key`/`on_mouse` mutate
//! state and set flags (`should_quit`, `save_requested`); the runner in `main`
//! performs the actual drawing and persistence.

use std::path::Path;

use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::complete;
use crate::config::Config;
use crate::ledger::{self, Ledger, SyncRecord};
use crate::paths;
use crate::scroll::Scroll;
use crate::skills;
use crate::sync;
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

/// A scrollable master-detail left nav over a set of skill groups. Reused by the
/// Global and Catalog tabs: each owns one, built from its own scan. Holds only
/// view state (the flattened rows, the selection, the scroll offset) — the source
/// scan lives on [`App`].
#[derive(Default)]
pub struct NavState {
    /// Flattened rows (group headers + selectable skills) for the left nav.
    pub rows: Vec<skills::NavRow>,
    /// Selected skill index (flat, across groups).
    pub sel: usize,
    /// Scroll offset that keeps the selection in view.
    pub scroll: Scroll,
}

impl NavState {
    /// Rebuild the rows from freshly scanned groups, clamping the selection.
    fn rebuild(&mut self, groups: &[skills::SkillGroup]) {
        self.rows = skills::nav_rows(groups);
        let count = skills::skill_count(&self.rows);
        if self.sel >= count {
            self.sel = count.saturating_sub(1);
        }
    }

    fn select_next(&mut self) {
        let count = skills::skill_count(&self.rows);
        if count > 0 && self.sel + 1 < count {
            self.sel += 1;
        }
    }

    fn select_prev(&mut self) {
        self.sel = self.sel.saturating_sub(1);
    }
}

/// Where a Catalog skill can be synced. `Project` is a "coming soon" stub.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SyncDest {
    Global,
    Project,
}

/// A toggleable symlink target in the sync modal (a detected agent dir).
#[derive(Clone, Debug)]
pub struct LinkToggle {
    pub rel: String,
    pub label: String,
    pub on: bool,
}

/// The "sync this skill" modal (opened from Catalog). Its interactive items form
/// a flat ring so keyboard focus and mouse clicks share one index space:
/// `0` = Global, `1` = Project, `2..2+links` = each link toggle, then Sync, Cancel.
pub struct SyncModal {
    pub skill: String,
    pub origin: String,
    pub dest: SyncDest,
    pub links: Vec<LinkToggle>,
    pub focus: usize,
}

impl SyncModal {
    /// Total interactive items: Global, Project, one per link, Sync, Cancel.
    pub fn item_count(&self) -> usize {
        4 + self.links.len()
    }
    /// Index of the Sync (confirm) item.
    pub fn confirm_index(&self) -> usize {
        2 + self.links.len()
    }
}

/// The "remove this global skill" confirmation (opened from Global). Focus ring:
/// `0` = Remove, `1` = Cancel.
pub struct RemoveModal {
    pub skill: String,
    /// The skill's group label, e.g. `~/.agents/skills`.
    pub location: String,
    /// The real on-disk path removed (the dir or the symlink).
    pub path: String,
    pub is_symlink: bool,
    pub link_target: Option<String>,
    /// Whether it's tracked in the repo; if not, removal is permanent.
    pub in_catalog: bool,
    pub focus: usize,
}

/// The active modal overlay, drawn over the current tab.
pub enum Overlay {
    Sync(SyncModal),
    Remove(RemoveModal),
}

/// A filesystem mutation queued by the (pure) input handler and executed by the
/// runner in `main` after input — keeps `on_key` free of I/O and unit-testable.
pub enum PendingOp {
    SyncGlobal {
        origin: String,
        name: String,
        link_rels: Vec<String>,
    },
    RemoveGlobal {
        name: String,
        path: String,
    },
}

/// The whole application.
pub struct App {
    pub screen: Screen,
    pub config: Config,
    pub should_quit: bool,
    /// Set when the config needs to be written; the runner persists and clears it.
    pub save_requested: bool,
    /// Skills found in the global agent dirs (`~/.claude/skills`, …).
    pub global_scan: skills::GlobalScan,
    /// Skills found in the loom-skills repo (`personal/`, `vendor/`).
    pub repo_scan: skills::RepoScan,
    /// Left-nav view state for the Global tab (over `global_scan`).
    pub global: NavState,
    /// Left-nav view state for the Catalog tab (over `repo_scan`).
    pub catalog: NavState,
    /// The sync ledger (`sync.toml`): what's been synced where.
    pub ledger: Ledger,
    /// The active modal overlay, if any.
    pub overlay: Option<Overlay>,
    /// A filesystem mutation queued for the runner to execute after input.
    pub pending: Option<PendingOp>,
    /// Transient footer status (last action result / error).
    pub status: Option<String>,
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
            global_scan: skills::GlobalScan::default(),
            repo_scan: skills::RepoScan::default(),
            global: NavState::default(),
            catalog: NavState::default(),
            ledger: Ledger::load(),
            overlay: None,
            pending: None,
            status: None,
        };
        if app.config.is_configured() {
            app.rescan();
        }
        app
    }

    /// Re-read installed skills from disk: the global agent dirs and the repo,
    /// then rebuild both left-nav views.
    pub fn rescan(&mut self) {
        self.global_scan = skills::scan_global();
        self.repo_scan = self
            .config
            .repo_path
            .as_deref()
            .map(skills::scan_repo)
            .unwrap_or_default();
        self.global.rebuild(&self.global_scan.groups);
        self.catalog.rebuild(&self.repo_scan.groups);
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        // A fresh keypress clears the last action's status line.
        self.status = None;
        if self.overlay.is_some() {
            self.on_key_overlay(key);
            return;
        }
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
            // Sync the selected Catalog skill; remove the selected Global skill.
            KeyCode::Char('s') => self.open_sync_modal(),
            KeyCode::Char('x') => self.open_remove_modal(),
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(nav) = self.active_nav_mut() {
                    nav.select_prev();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(nav) = self.active_nav_mut() {
                    nav.select_next();
                }
            }
            KeyCode::Char(c @ '1'..='4') => {
                if let Some(m) = self.main_mut() {
                    m.select((c as u8 - b'1') as usize);
                }
            }
            _ => {}
        }
    }

    /// Mouse support on the main screen: the wheel scrolls the active nav (by
    /// moving the selection, which the view follows); left-click hits tabs, the
    /// gear, or a skill row.
    pub fn on_mouse(&mut self, event: MouseEvent, area: Rect) {
        if !matches!(self.screen, Screen::Main(_)) {
            return;
        }
        // An open overlay swallows the mouse (only left-clicks matter to it).
        if self.overlay.is_some() {
            if let MouseEventKind::Down(MouseButton::Left) = event.kind {
                self.status = None;
                self.on_overlay_click(event.column, event.row, area);
            }
            return;
        }
        match event.kind {
            MouseEventKind::ScrollDown => {
                if let Some(nav) = self.active_nav_mut() {
                    nav.select_next();
                }
            }
            MouseEventKind::ScrollUp => {
                if let Some(nav) = self.active_nav_mut() {
                    nav.select_prev();
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                self.status = None;
                self.on_left_click(event.column, event.row, area)
            }
            _ => {}
        }
    }

    /// A click inside an open modal: resolve it to an interactive item and
    /// activate it, exactly as pressing ⏎ on that item would.
    fn on_overlay_click(&mut self, col: u16, row: u16, area: Rect) {
        let Some(overlay) = self.overlay.as_ref() else {
            return;
        };
        let hit = ui::modal_hitmap(area, overlay)
            .into_iter()
            .find(|(rect, _)| contains(*rect, col, row))
            .map(|(_, idx)| idx);
        let Some(idx) = hit else { return };
        match self.overlay.take() {
            Some(Overlay::Sync(m)) => self.sync_activate(m, idx),
            Some(Overlay::Remove(m)) => self.remove_activate(m, idx),
            None => {}
        }
    }

    fn on_left_click(&mut self, col: u16, row: u16, area: Rect) {
        let regions = ui::regions(area);
        let (tabs, gear) = ui::tab_spans(regions.tabbar);

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

        // Master-detail left nav (Global/Catalog): the detail action button opens
        // the modal; a skill row selects it.
        if let Some(tab) = self.active_nav_tab() {
            let (nav_rect, detail) = ui::nav_detail_layout(regions.content);

            // The detail action button (Sync → / Remove), only when a skill is selected.
            let label = ui::detail_action_label(tab);
            if self.active_nav_has_selection()
                && let Some(rect) = ui::nav_detail_action_rect(detail, label)
                && contains(rect, col, row)
            {
                match tab {
                    Tab::Catalog => self.open_sync_modal(),
                    Tab::Global => self.open_remove_modal(),
                    _ => {}
                }
                return;
            }

            if let Some(nav) = self.active_nav_mut()
                && let Some(index) = nav_row_hit(&nav.rows, nav_rect, col, row, nav.scroll.offset)
            {
                nav.sel = index;
            }
        }
    }

    /// Whether the active nav tab currently has a skill selected.
    fn active_nav_has_selection(&self) -> bool {
        match self.active_nav_tab() {
            Some(Tab::Global) => skills::skill_at(&self.global.rows, self.global.sel).is_some(),
            Some(Tab::Catalog) => skills::skill_at(&self.catalog.rows, self.catalog.sel).is_some(),
            _ => false,
        }
    }

    /// The tab whose left nav is currently active (Global or Catalog, and only
    /// when settings aren't covering it), if any.
    fn active_nav_tab(&self) -> Option<Tab> {
        match &self.screen {
            Screen::Main(m)
                if !m.settings_open && matches!(m.active, Tab::Global | Tab::Catalog) =>
            {
                Some(m.active)
            }
            _ => None,
        }
    }

    /// Mutable access to the active tab's left-nav view state, if a nav tab is up.
    fn active_nav_mut(&mut self) -> Option<&mut NavState> {
        match self.active_nav_tab()? {
            Tab::Global => Some(&mut self.global),
            Tab::Catalog => Some(&mut self.catalog),
            _ => None,
        }
    }

    /// Open the "sync this skill" modal for the selected Catalog skill.
    fn open_sync_modal(&mut self) {
        if self.active_nav_tab() != Some(Tab::Catalog) {
            return;
        }
        let Some(skills::NavRow::Skill { name, location, .. }) =
            skills::skill_at(&self.catalog.rows, self.catalog.sel)
        else {
            return;
        };
        let (skill, origin) = (name.clone(), location.clone());
        let links: Vec<LinkToggle> = sync::detected_link_targets()
            .into_iter()
            .map(|t| LinkToggle {
                rel: t.rel,
                label: t.label,
                on: true,
            })
            .collect();
        let mut modal = SyncModal {
            skill,
            origin,
            dest: SyncDest::Global,
            links,
            focus: 0,
        };
        // Default focus to the Sync button: open → ⏎ syncs with the defaults.
        modal.focus = modal.confirm_index();
        self.overlay = Some(Overlay::Sync(modal));
    }

    /// Open the remove-confirmation for the selected Global skill.
    fn open_remove_modal(&mut self) {
        if self.active_nav_tab() != Some(Tab::Global) {
            return;
        }
        let Some(skills::NavRow::Skill {
            name,
            location,
            link_target,
            ..
        }) = skills::skill_at(&self.global.rows, self.global.sel)
        else {
            return;
        };
        let (name, location, link_target) = (name.clone(), location.clone(), link_target.clone());
        let path = paths::expand_tilde(&location)
            .join(&name)
            .to_string_lossy()
            .into_owned();
        let in_catalog = skills::is_in_repo(&self.repo_scan, &name);
        self.overlay = Some(Overlay::Remove(RemoveModal {
            skill: name,
            location,
            path,
            is_symlink: link_target.is_some(),
            link_target,
            in_catalog,
            focus: 1, // default to Cancel — the safe choice for a destructive action
        }));
    }

    fn on_key_overlay(&mut self, key: KeyEvent) {
        // Take the overlay out so activation can freely mutate `self`; nav keys
        // put it back, actions decide whether to reopen or close.
        let Some(overlay) = self.overlay.take() else {
            return;
        };
        match overlay {
            Overlay::Sync(mut m) => match key.code {
                KeyCode::Esc => {}
                KeyCode::Up | KeyCode::Char('k') => {
                    m.focus = m.focus.saturating_sub(1);
                    self.overlay = Some(Overlay::Sync(m));
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    m.focus = (m.focus + 1).min(m.item_count() - 1);
                    self.overlay = Some(Overlay::Sync(m));
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    let focus = m.focus;
                    self.sync_activate(m, focus);
                }
                _ => self.overlay = Some(Overlay::Sync(m)),
            },
            Overlay::Remove(mut m) => match key.code {
                KeyCode::Esc | KeyCode::Char('n') => {}
                KeyCode::Char('y') => self.remove_activate(m, 0),
                KeyCode::Up
                | KeyCode::Down
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::Char('h')
                | KeyCode::Char('l')
                | KeyCode::Char('k')
                | KeyCode::Char('j')
                | KeyCode::Tab => {
                    m.focus = 1 - m.focus.min(1);
                    self.overlay = Some(Overlay::Remove(m));
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    let focus = m.focus;
                    self.remove_activate(m, focus);
                }
                _ => self.overlay = Some(Overlay::Remove(m)),
            },
        }
    }

    /// Activate interactive item `index` of a sync modal (from ⏎ or a click).
    fn sync_activate(&mut self, mut m: SyncModal, index: usize) {
        let confirm = m.confirm_index();
        if index == 0 {
            m.dest = SyncDest::Global;
            self.overlay = Some(Overlay::Sync(m));
        } else if index == 1 {
            m.dest = SyncDest::Project;
            self.overlay = Some(Overlay::Sync(m));
        } else if index < confirm {
            let i = index - 2;
            m.links[i].on = !m.links[i].on;
            self.overlay = Some(Overlay::Sync(m));
        } else if index == confirm {
            match m.dest {
                SyncDest::Global => {
                    let link_rels = m
                        .links
                        .iter()
                        .filter(|l| l.on)
                        .map(|l| l.rel.clone())
                        .collect();
                    self.pending = Some(PendingOp::SyncGlobal {
                        origin: m.origin,
                        name: m.skill,
                        link_rels,
                    });
                    // overlay stays None (taken); the runner executes the op.
                }
                SyncDest::Project => {
                    self.status = Some("Project sync is coming soon.".to_string());
                    self.overlay = Some(Overlay::Sync(m)); // keep the modal open
                }
            }
        } else {
            // Cancel: leave the overlay closed.
        }
    }

    /// Activate item `index` of a remove modal (`0` = Remove, `1` = Cancel).
    fn remove_activate(&mut self, m: RemoveModal, index: usize) {
        if index == 0 {
            self.pending = Some(PendingOp::RemoveGlobal {
                name: m.skill,
                path: m.path,
            });
        }
        // Either way the overlay is now closed (it was taken).
    }

    /// Execute a queued mutation: run the fs op, update the ledger, rescan, and
    /// report via the status line. Called by the runner after input.
    pub fn apply_pending(&mut self) {
        let Some(op) = self.pending.take() else {
            return;
        };
        match op {
            PendingOp::SyncGlobal {
                origin,
                name,
                link_rels,
            } => {
                let repo = self.config.repo_path.clone().unwrap_or_default();
                match sync::sync_to_global(&repo, &origin, &name, &link_rels) {
                    Ok(()) => {
                        self.ledger.record(SyncRecord {
                            skill: name.clone(),
                            origin,
                            destination: ledger::GLOBAL.to_string(),
                        });
                        self.status = match self.ledger.save() {
                            Ok(()) => Some(format!("Synced '{name}' → global.")),
                            Err(e) => Some(format!("Synced '{name}', but ledger save failed: {e}")),
                        };
                        self.rescan();
                    }
                    Err(e) => self.status = Some(format!("Sync failed: {e}")),
                }
            }
            PendingOp::RemoveGlobal { name, path } => match sync::remove_path(Path::new(&path)) {
                Ok(()) => {
                    self.ledger.forget(&name, ledger::GLOBAL);
                    let _ = self.ledger.save();
                    self.status = Some(format!("Removed '{name}'."));
                    self.rescan();
                }
                Err(e) => self.status = Some(format!("Remove failed: {e}")),
            },
        }
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
/// left-nav rect, accounting for the border and scroll `offset`. `None` if it's
/// a header/empty/gap.
fn nav_row_hit(
    rows: &[skills::NavRow],
    nav: Rect,
    col: u16,
    row: u16,
    offset: usize,
) -> Option<usize> {
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
    let visual = offset + (row - inner_y) as usize;
    skills::skill_index_at_line(rows, visual)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn wheel(kind: MouseEventKind) -> MouseEvent {
        MouseEvent {
            kind,
            column: 5,
            row: 5,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn skills_of(names: &[&str]) -> App {
        let mut app = main_app();
        app.global_scan = skills::GlobalScan {
            groups: vec![skills::SkillGroup {
                label: "L".to_string(),
                skills: names.iter().map(|n| skills::SkillEntry::new(*n)).collect(),
            }],
        };
        app.global.rebuild(&app.global_scan.groups);
        app
    }

    /// A configured app whose Catalog nav is populated with the given repo skills.
    fn catalog_of(names: &[&str]) -> App {
        let mut app = main_app();
        app.repo_scan = skills::RepoScan {
            groups: vec![skills::SkillGroup {
                label: "personal".to_string(),
                skills: names.iter().map(|n| skills::SkillEntry::new(*n)).collect(),
            }],
        };
        app.catalog.rebuild(&app.repo_scan.groups);
        app
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
        let mut app = skills_of(&["a", "b", "c"]);

        // On Dashboard, arrow keys are a no-op for the Global selection.
        app.on_key(key(KeyCode::Down));
        assert_eq!(app.global.sel, 0);

        app.on_key(key(KeyCode::Char('3'))); // switch to Global
        app.on_key(key(KeyCode::Down));
        assert_eq!(app.global.sel, 1);
        app.on_key(key(KeyCode::Char('j')));
        assert_eq!(app.global.sel, 2);
        app.on_key(key(KeyCode::Down)); // clamps at the end
        assert_eq!(app.global.sel, 2);
        app.on_key(key(KeyCode::Up));
        assert_eq!(app.global.sel, 1);
    }

    #[test]
    fn catalog_selection_moves_on_catalog_tab() {
        let mut app = catalog_of(&["a", "b", "c"]);

        // On Dashboard, arrow keys don't move the Catalog selection.
        app.on_key(key(KeyCode::Down));
        assert_eq!(app.catalog.sel, 0);

        app.on_key(key(KeyCode::Char('4'))); // switch to Catalog
        app.on_key(key(KeyCode::Down));
        assert_eq!(app.catalog.sel, 1);
        app.on_key(key(KeyCode::Char('j')));
        assert_eq!(app.catalog.sel, 2);
        app.on_key(key(KeyCode::Down)); // clamps at the end
        assert_eq!(app.catalog.sel, 2);
        // The Global nav is untouched by Catalog navigation.
        assert_eq!(app.global.sel, 0);
    }

    #[test]
    fn scroll_wheel_moves_selection_on_global_tab() {
        let mut app = skills_of(&["a", "b", "c"]);
        let area = Rect::new(0, 0, 100, 24);

        // Off the Global tab, the wheel does nothing.
        app.on_mouse(wheel(MouseEventKind::ScrollDown), area);
        assert_eq!(app.global.sel, 0);

        app.on_key(key(KeyCode::Char('3'))); // Global
        app.on_mouse(wheel(MouseEventKind::ScrollDown), area);
        assert_eq!(app.global.sel, 1);
        app.on_mouse(wheel(MouseEventKind::ScrollDown), area);
        assert_eq!(app.global.sel, 2);
        app.on_mouse(wheel(MouseEventKind::ScrollDown), area); // clamps
        assert_eq!(app.global.sel, 2);
        app.on_mouse(wheel(MouseEventKind::ScrollUp), area);
        assert_eq!(app.global.sel, 1);
    }

    #[test]
    fn s_opens_sync_modal_on_catalog_and_esc_closes() {
        let mut app = catalog_of(&["a", "b"]);
        app.on_key(key(KeyCode::Char('4'))); // Catalog
        app.on_key(key(KeyCode::Char('s')));
        assert!(matches!(app.overlay, Some(Overlay::Sync(_))));
        app.on_key(key(KeyCode::Esc));
        assert!(app.overlay.is_none());
    }

    #[test]
    fn x_opens_remove_modal_on_global_and_swallows_other_keys() {
        let mut app = skills_of(&["a"]);
        app.on_key(key(KeyCode::Char('3'))); // Global
        app.on_key(key(KeyCode::Char('x')));
        assert!(matches!(app.overlay, Some(Overlay::Remove(_))));
        // While the modal is open, unrelated keys don't reach the app (no quit).
        app.on_key(key(KeyCode::Char('q')));
        assert!(!app.should_quit);
        assert!(matches!(app.overlay, Some(Overlay::Remove(_))));
    }

    #[test]
    fn sync_modal_confirm_queues_sync_with_only_enabled_links() {
        let mut app = main_app();
        let modal = SyncModal {
            skill: "sample".to_string(),
            origin: "personal".to_string(),
            dest: SyncDest::Global,
            links: vec![
                LinkToggle {
                    rel: ".claude/skills".to_string(),
                    label: "~/.claude/skills".to_string(),
                    on: true,
                },
                LinkToggle {
                    rel: ".codex/skills".to_string(),
                    label: "~/.codex/skills".to_string(),
                    on: false,
                },
            ],
            focus: 0,
        };
        let confirm = modal.confirm_index();
        app.sync_activate(modal, confirm);
        match &app.pending {
            Some(PendingOp::SyncGlobal {
                origin,
                name,
                link_rels,
            }) => {
                assert_eq!(name, "sample");
                assert_eq!(origin, "personal");
                assert_eq!(link_rels, &vec![".claude/skills".to_string()]); // the off one is dropped
            }
            _ => panic!("expected a queued SyncGlobal"),
        }
        assert!(app.overlay.is_none());
    }

    #[test]
    fn sync_modal_project_is_coming_soon_and_does_not_queue() {
        let mut app = main_app();
        let modal = SyncModal {
            skill: "s".to_string(),
            origin: "personal".to_string(),
            dest: SyncDest::Project,
            links: vec![],
            focus: 0,
        };
        let confirm = modal.confirm_index();
        app.sync_activate(modal, confirm);
        assert!(app.pending.is_none());
        assert!(matches!(app.overlay, Some(Overlay::Sync(_)))); // stays open
        assert!(app.status.as_deref().unwrap().contains("coming soon"));
    }

    #[test]
    fn remove_modal_confirm_queues_removal_but_cancel_does_not() {
        let make = |focus| RemoveModal {
            skill: "x".to_string(),
            location: "~/.agents/skills".to_string(),
            path: "/tmp/x".to_string(),
            is_symlink: false,
            link_target: None,
            in_catalog: false,
            focus,
        };
        let mut confirm = main_app();
        confirm.remove_activate(make(0), 0);
        assert!(matches!(
            confirm.pending,
            Some(PendingOp::RemoveGlobal { .. })
        ));

        let mut cancel = main_app();
        cancel.remove_activate(make(1), 1);
        assert!(cancel.pending.is_none());
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
