//! Rendering. Reads `App` state and draws it — no state changes here.
//!
//! Tab-bar geometry ([`tab_spans`]) is shared with the mouse hit-testing in
//! `app`, so what's drawn and what's clickable can't drift apart.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, MainState, Screen, SetupState, Tab};
use crate::config::Config;
use crate::skills::{self, NavRow, RepoScan};

const PREFIX: &str = " skilloom  ";
const GEAR: &str = "⚙";

/// The three horizontal bands of the main screen.
pub struct Regions {
    pub tabbar: Rect,
    pub content: Rect,
    pub footer: Rect,
}

pub fn regions(area: Rect) -> Regions {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);
    Regions {
        tabbar: chunks[0],
        content: chunks[1],
        footer: chunks[2],
    }
}

fn tab_label(tab: Tab) -> String {
    format!(" {} ", tab.title())
}

/// Each tab's on-screen rect and the gear's rect, laid left-to-right after the
/// `skilloom` prefix. Used both to draw the bar and to hit-test clicks.
pub fn tab_spans(tabbar: Rect) -> (Vec<(Tab, Rect)>, Option<Rect>) {
    let mut spans = Vec::new();
    let mut x = tabbar.x.saturating_add(PREFIX.chars().count() as u16);
    for tab in Tab::ALL {
        let width = tab_label(tab).chars().count() as u16;
        if x >= tabbar.right() {
            break;
        }
        let w = width.min(tabbar.right().saturating_sub(x));
        spans.push((tab, Rect::new(x, tabbar.y, w, 1)));
        x = x.saturating_add(width + 1);
    }
    let gear_w = GEAR.chars().count() as u16;
    let gear = if tabbar.width > gear_w + 1 {
        Some(Rect::new(
            tabbar.right().saturating_sub(gear_w + 1),
            tabbar.y,
            gear_w,
            1,
        ))
    } else {
        None
    };
    (spans, gear)
}

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    match &app.screen {
        Screen::Setup(setup) => render_setup(frame, area, setup),
        Screen::Main(main) => {
            let r = regions(area);
            render_tabbar(frame, r.tabbar, main);
            render_content(frame, r.content, app, main);
            render_footer(frame, r.footer, &app.screen);
        }
    }
}

fn render_tabbar(frame: &mut Frame, area: Rect, main: &MainState) {
    let base = Style::default().fg(Color::Gray);
    let selected = Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    let prefix_w = (PREFIX.chars().count() as u16).min(area.width);
    frame.render_widget(
        Paragraph::new(PREFIX).style(Style::default().add_modifier(Modifier::BOLD)),
        Rect::new(area.x, area.y, prefix_w, 1),
    );

    let (tabs, gear) = tab_spans(area);
    for (tab, rect) in tabs {
        let style = if tab == main.active && !main.settings_open {
            selected
        } else {
            base
        };
        frame.render_widget(Paragraph::new(tab_label(tab)).style(style), rect);
    }
    if let Some(rect) = gear {
        let style = if main.settings_open { selected } else { base };
        frame.render_widget(Paragraph::new(GEAR).style(style), rect);
    }
}

fn render_content(frame: &mut Frame, area: Rect, app: &App, main: &MainState) {
    if main.settings_open {
        render_settings(frame, area, &app.config);
        return;
    }
    match main.active {
        Tab::Global => render_global(frame, area, &app.global_rows, app.global_sel, &app.repo),
        Tab::Catalog => render_catalog(frame, area, &app.config, &app.repo),
        other => render_placeholder(frame, area, other),
    }
}

fn render_settings(frame: &mut Frame, area: Rect, config: &Config) {
    let repo = config.repo_path.clone().unwrap_or_else(|| "—".to_string());
    let body = vec![
        Line::from("Settings — hello world"),
        Line::from(""),
        Line::from(format!("loom-skills repo: {repo}")),
        Line::from(""),
        Line::from(Span::styled(
            "esc to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    let block = Block::default().borders(Borders::ALL).title(" ⚙ settings ");
    frame.render_widget(Paragraph::new(body).block(block), area);
}

fn render_placeholder(frame: &mut Frame, area: Rect, tab: Tab) {
    let body = vec![
        Line::from(Span::styled(
            tab.placeholder(),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "(nothing here yet)",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", tab.title()));
    frame.render_widget(Paragraph::new(body).block(block), area);
}

/// Split the Global content area into (left nav, detail). Shared with mouse
/// hit-testing in `app` so clickable rows match what's drawn.
pub fn global_layout(content: Rect) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(32), Constraint::Min(1)])
        .split(content);
    (chunks[0], chunks[1])
}

fn render_global(frame: &mut Frame, area: Rect, rows: &[NavRow], selected: usize, repo: &RepoScan) {
    let (nav, detail) = global_layout(area);
    render_global_nav(frame, nav, rows, selected);
    render_global_detail(frame, detail, rows, selected, repo);
}

fn render_global_nav(frame: &mut Frame, area: Rect, rows: &[NavRow], selected: usize) {
    let block = Block::default().borders(Borders::ALL).title(" Global ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let width = inner.width as usize;
    let sel_bg = Style::default().fg(Color::Black).bg(Color::Cyan);

    let mut lines: Vec<Line<'static>> = Vec::new();
    if rows.is_empty() {
        lines.push(Line::from(Span::styled(
            "(no skills found)",
            Style::default().fg(Color::DarkGray),
        )));
    }
    for row in rows {
        match row {
            NavRow::Header(label) => lines.push(Line::from(Span::styled(
                label.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ))),
            NavRow::Empty => lines.push(Line::from(Span::styled(
                "  (none)",
                Style::default().fg(Color::DarkGray),
            ))),
            NavRow::Skill {
                index,
                name,
                link_target,
                description,
                ..
            } => {
                let is_sel = *index == selected;
                // Card line 1: "▸ name" with the symlink `@` floated to the right.
                let marker = if is_sel { "▸ " } else { "  " };
                let flag = if link_target.is_some() { "@" } else { "" };
                let head = float_right(&format!("{marker}{name}"), flag, width);
                let head_style = if is_sel {
                    sel_bg.add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                lines.push(Line::from(Span::styled(head, head_style)));
                // Card line 2: the SKILL.md description, grayed and truncated.
                let sub_text = description.as_deref().unwrap_or("—");
                let sub = pad_to(
                    &format!("  {}", truncate_chars(sub_text, width.saturating_sub(2))),
                    width,
                );
                let sub_style = if is_sel {
                    sel_bg
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                lines.push(Line::from(Span::styled(sub, sub_style)));
            }
        }
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_global_detail(
    frame: &mut Frame,
    area: Rect,
    rows: &[NavRow],
    selected: usize,
    repo: &RepoScan,
) {
    let Some(NavRow::Skill {
        name,
        location,
        link_target,
        ..
    }) = skills::skill_at(rows, selected)
    else {
        let block = Block::default().borders(Borders::ALL).title(" skill ");
        let empty = Paragraph::new(Line::from(Span::styled(
            "No skill selected.",
            Style::default().fg(Color::DarkGray),
        )))
        .block(block);
        frame.render_widget(empty, area);
        return;
    };

    // Header card: the selected skill's metadata.
    let status = if skills::is_in_repo(repo, name) {
        Span::styled("● synced (in repo)", Style::default().fg(Color::Green))
    } else {
        Span::styled(
            "○ not synced (not in repo)",
            Style::default().fg(Color::Yellow),
        )
    };
    let mut meta: Vec<Line<'static>> = vec![Line::from(vec![
        Span::styled("location  ", Style::default().fg(Color::DarkGray)),
        Span::raw(location.clone()),
    ])];
    if let Some(target) = link_target {
        meta.push(Line::from(vec![
            Span::styled("links to  ", Style::default().fg(Color::DarkGray)),
            Span::styled(target.clone(), Style::default().fg(Color::Cyan)),
            Span::styled("  (symlink)", Style::default().fg(Color::DarkGray)),
        ]));
    }
    meta.push(Line::from(vec![
        Span::styled("status    ", Style::default().fg(Color::DarkGray)),
        status,
    ]));

    let card_h = (meta.len() as u16 + 2).min(area.height);
    let card = Rect::new(area.x, area.y, area.width, card_h);
    let card_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {name} "));
    frame.render_widget(Paragraph::new(meta).block(card_block), card);

    // Room below the header card for full details, later.
    let below_y = area.y + card_h;
    let below_h = (area.y + area.height).saturating_sub(below_y);
    if below_h > 0 {
        let below = Rect::new(area.x, below_y, area.width, below_h);
        let block = Block::default().borders(Borders::ALL).title(" details ");
        let placeholder = Paragraph::new(Line::from(Span::styled(
            "SKILL.md contents will show here.",
            Style::default().fg(Color::DarkGray),
        )))
        .block(block);
        frame.render_widget(placeholder, below);
    }
}

/// Truncate the string to at most `width` characters (spaces added if shorter).
fn pad_to(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        s.chars().take(width).collect()
    } else {
        format!("{s}{}", " ".repeat(width - len))
    }
}

/// Truncate to `max` chars, adding an ellipsis when it doesn't fit.
fn truncate_chars(s: &str, max: usize) -> String {
    let count = s.chars().count();
    if count <= max {
        s.to_string()
    } else if max == 0 {
        String::new()
    } else {
        let head: String = s.chars().take(max - 1).collect();
        format!("{head}…")
    }
}

/// `left` padded so `right` sits flush against the right edge of `width`.
fn float_right(left: &str, right: &str, width: usize) -> String {
    let rlen = right.chars().count();
    let avail = width.saturating_sub(rlen);
    let left: String = left.chars().take(avail).collect();
    let pad = width.saturating_sub(left.chars().count() + rlen);
    format!("{left}{}{right}", " ".repeat(pad))
}

fn render_catalog(frame: &mut Frame, area: Rect, config: &Config, repo: &RepoScan) {
    let repo_path = config.repo_path.clone().unwrap_or_else(|| "—".to_string());
    let mut lines: Vec<Line<'static>> = vec![
        Line::from(Span::styled(
            format!("loom-skills · {repo_path}"),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("personal ({})", repo.personal.len()),
            Style::default().fg(Color::Cyan),
        )),
    ];
    push_names(&mut lines, &repo.personal);
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("vendor ({})", repo.vendor.len()),
        Style::default().fg(Color::Cyan),
    )));
    push_names(&mut lines, &repo.vendor);

    let block = Block::default().borders(Borders::ALL).title(" Catalog ");
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn push_names(lines: &mut Vec<Line<'static>>, names: &[String]) {
    if names.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (none yet)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for name in names {
            lines.push(Line::from(format!("  {name}")));
        }
    }
}

fn render_footer(frame: &mut Frame, area: Rect, screen: &Screen) {
    let text = match screen {
        Screen::Setup(_) => "Tab complete · ⏎ continue · Esc quit",
        Screen::Main(m) if m.settings_open => "esc close settings · q quit",
        Screen::Main(_) => "↑↓ select · ↹ tab · f refresh · , settings · q quit",
    };
    frame.render_widget(
        Paragraph::new(text).style(Style::default().fg(Color::DarkGray)),
        area,
    );
}

fn render_setup(frame: &mut Frame, area: Rect, setup: &SetupState) {
    let mut lines: Vec<Line> = vec![
        Line::from("Point skilloom at your skills repo (loom-skills)."),
        Line::from(""),
        Line::from(vec![
            Span::styled("path  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}\u{2588}", setup.input),
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ];

    let shown = setup.suggestions.len().min(6);
    if shown > 0 {
        lines.push(Line::from(""));
        for suggestion in setup.suggestions.iter().take(shown) {
            lines.push(Line::from(Span::styled(
                format!("  {suggestion}"),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }
    if let Some(err) = &setup.error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            err.clone(),
            Style::default().fg(Color::Red),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Tab complete · ⏎ continue · Esc quit",
        Style::default().fg(Color::DarkGray),
    )));

    let height = lines.len() as u16 + 2;
    let rect = centered(area, 74, height);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" skilloom · first run ");
    frame.render_widget(Paragraph::new(lines).block(block), rect);
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    Rect::new(x, y, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::config::Config;
    use crate::skills::{GlobalScan, RepoScan, SkillEntry, SkillGroup};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn draw(app: &App) -> String {
        let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
        terminal.draw(|frame| render(frame, app)).unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    fn press(app: &mut App, code: KeyCode) {
        app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    #[test]
    fn main_screen_shows_tabs_and_placeholder() {
        let app = App::new(Config {
            repo_path: Some("/x".to_string()),
        });
        let text = draw(&app);
        assert!(text.contains("Dashboard"));
        assert!(text.contains("Catalog"));
        assert!(text.contains("hello world"));
    }

    #[test]
    fn setup_screen_shows_prompt() {
        let app = App::new(Config::default());
        let text = draw(&app);
        assert!(text.contains("first run"));
    }

    #[test]
    fn global_tab_groups_by_location_and_shows_sync() {
        let mut app = App::new(Config {
            repo_path: Some("/x".to_string()),
        });
        app.global = GlobalScan {
            groups: vec![SkillGroup {
                label: "~/.claude/skills".to_string(),
                skills: vec![
                    SkillEntry {
                        name: "herdr".to_string(),
                        link_target: Some("~/.agents/skills/herdr".to_string()),
                        description: Some("Control herdr from inside it".to_string()),
                    },
                    SkillEntry::new("okq"),
                ],
            }],
        };
        app.global_rows = crate::skills::nav_rows(&app.global);
        app.repo = RepoScan::default();
        app.global_sel = 0; // herdr, the symlink
        press(&mut app, KeyCode::Char('3')); // Global
        let text = draw(&app);
        assert!(text.contains("~/.claude/skills")); // group header in the nav
        assert!(text.contains("herdr")); // card name
        assert!(text.contains("Control herdr from inside it")); // card subtitle (description)
        assert!(text.contains('@')); // symlink indicator in the nav
        assert!(text.contains("links to")); // header card shows the real location
        assert!(text.contains("~/.agents/skills/herdr")); // the symlink target
        assert!(text.contains("not synced")); // header card status (repo empty)
        assert!(text.contains("details")); // the details box below the header card
    }

    #[test]
    fn catalog_tab_shows_repo_sections() {
        let mut app = App::new(Config {
            repo_path: Some("/x".to_string()),
        });
        app.repo = RepoScan {
            personal: vec!["mine".to_string()],
            vendor: Vec::new(),
        };
        press(&mut app, KeyCode::Char('4')); // Catalog
        let text = draw(&app);
        assert!(text.contains("personal (1)"));
        assert!(text.contains("mine"));
        assert!(text.contains("vendor (0)"));
    }
}
