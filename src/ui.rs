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
            render_content(frame, r.content, main, &app.config);
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

fn render_content(frame: &mut Frame, area: Rect, main: &MainState, config: &Config) {
    if main.settings_open {
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
        return;
    }

    let body = vec![
        Line::from(Span::styled(
            main.active.placeholder(),
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
        .title(format!(" {} ", main.active.title()));
    frame.render_widget(Paragraph::new(body).block(block), area);
}

fn render_footer(frame: &mut Frame, area: Rect, screen: &Screen) {
    let text = match screen {
        Screen::Setup(_) => "Tab complete · ⏎ continue · Esc quit",
        Screen::Main(m) if m.settings_open => "esc close settings · q quit",
        Screen::Main(_) => "↹ tab · 1-4 jump · , settings · click tabs · q quit",
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
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

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
}
