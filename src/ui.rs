//! Rendering. Reads `App` state and draws it — no state changes here.
//!
//! Tab-bar geometry ([`tab_spans`]) is shared with the mouse hit-testing in
//! `app`, so what's drawn and what's clickable can't drift apart.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};

use crate::app::{App, MainState, NavState, Overlay, Screen, SetupState, SyncDest, Tab};
use crate::config::Config;
use crate::skills::{self, GlobalScan, NavRow, RepoScan};

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

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Stateful pre-step: keep the active nav's selection scrolled into view. Uses
    // the live viewport height, so it lives here rather than in the event handlers.
    let nav_tab = match &app.screen {
        Screen::Main(m) if !m.settings_open => Some(m.active),
        _ => None,
    };
    let nav = match nav_tab {
        Some(Tab::Global) => Some(&mut app.global),
        Some(Tab::Catalog) => Some(&mut app.catalog),
        _ => None,
    };
    if let Some(nav) = nav {
        let (nav_area, _) = nav_detail_layout(regions(area).content);
        let viewport = nav_area.height.saturating_sub(2) as usize; // inside borders
        let total = skills::total_lines(&nav.rows);
        let (start, len) = skills::selected_line_range(&nav.rows, nav.sel);
        nav.scroll.focus(start, len, viewport, total);
    }

    let app = &*app;
    match &app.screen {
        Screen::Setup(setup) => render_setup(frame, area, setup),
        Screen::Main(main) => {
            let r = regions(area);
            render_tabbar(frame, r.tabbar, main);
            render_content(frame, r.content, app, main);
            render_footer(frame, r.footer, app);
            if let Some(overlay) = &app.overlay {
                render_overlay(frame, area, overlay);
            }
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
        Tab::Global => {
            let status = global_status(&app.global, &app.repo_scan);
            let action = detail_action_label(Tab::Global);
            render_master_detail(frame, area, " Global ", &app.global, status, action);
        }
        Tab::Catalog => {
            let status = catalog_status(&app.catalog, &app.global_scan);
            let action = detail_action_label(Tab::Catalog);
            render_master_detail(frame, area, " Catalog ", &app.catalog, status, action);
        }
        other => render_placeholder(frame, area, other),
    }
}

/// The detail-pane action-button label for a master-detail tab (empty = none).
/// Shared by the renderer and the click hit-testing in `app`.
pub fn detail_action_label(tab: Tab) -> &'static str {
    match tab {
        Tab::Global => "Remove",
        Tab::Catalog => "Sync →",
        _ => "",
    }
}

/// Rect of the detail-pane action button — top-right of the detail area — or
/// `None` when there's no action or no room. Shared with `app` hit-testing.
pub fn nav_detail_action_rect(detail: Rect, label: &str) -> Option<Rect> {
    if label.is_empty() || detail.width < 8 || detail.height < 3 {
        return None;
    }
    let w = (label.chars().count() as u16 + 4).min(detail.width.saturating_sub(2)); // "[ label ]"
    let x = detail.right().saturating_sub(w + 1);
    Some(Rect::new(x, detail.y, w, 1))
}

/// Status Span for the selected Global skill: is it tracked in the repo?
fn global_status(nav: &NavState, repo: &RepoScan) -> Option<Span<'static>> {
    match skills::skill_at(&nav.rows, nav.sel)? {
        NavRow::Skill { name, .. } => Some(if skills::is_in_repo(repo, name) {
            styled_status("● synced (in repo)", Color::Green)
        } else {
            styled_status("○ not synced (not in repo)", Color::Yellow)
        }),
        _ => None,
    }
}

/// Status Span for the selected Catalog skill: is it installed in a global dir?
fn catalog_status(nav: &NavState, global: &GlobalScan) -> Option<Span<'static>> {
    match skills::skill_at(&nav.rows, nav.sel)? {
        NavRow::Skill { name, .. } => Some(if skills::groups_contain(&global.groups, name) {
            styled_status("● installed globally", Color::Green)
        } else {
            styled_status("○ not installed", Color::Yellow)
        }),
        _ => None,
    }
}

fn styled_status(text: &str, color: Color) -> Span<'static> {
    Span::styled(text.to_string(), Style::default().fg(color))
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

/// Split a master-detail content area into (left nav, detail). Shared by the
/// Global and Catalog tabs, and with mouse hit-testing in `app` so clickable rows
/// match what's drawn.
pub fn nav_detail_layout(content: Rect) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(32), Constraint::Min(1)])
        .split(content);
    (chunks[0], chunks[1])
}

/// A scrollable skill left-nav beside a detail pane. `title` names the nav block;
/// `status` is the selected skill's tab-specific sync status line (if any).
fn render_master_detail(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    nav: &NavState,
    status: Option<Span<'static>>,
    action_label: &str,
) {
    let (nav_area, detail) = nav_detail_layout(area);
    render_nav(
        frame,
        nav_area,
        title,
        &nav.rows,
        nav.sel,
        nav.scroll.offset,
    );
    render_nav_detail(frame, detail, &nav.rows, nav.sel, status, action_label);
}

fn render_nav(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    rows: &[NavRow],
    selected: usize,
    offset: usize,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", title.trim()));
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

    // Show only the scrolled-into-view slice; draw a scrollbar when it overflows.
    let viewport = inner.height as usize;
    let start = offset.min(lines.len());
    let end = (start + viewport).min(lines.len());
    let visible = lines[start..end].to_vec();
    frame.render_widget(Paragraph::new(visible), inner);

    if lines.len() > viewport {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);
        let mut state = ScrollbarState::new(lines.len()).position(offset);
        frame.render_stateful_widget(scrollbar, area, &mut state);
    }
}

/// Right pane: a metadata header card for the selected skill over a details box.
/// `status` is the tab-specific sync line (Global: in-repo; Catalog: installed).
fn render_nav_detail(
    frame: &mut Frame,
    area: Rect,
    rows: &[NavRow],
    selected: usize,
    status: Option<Span<'static>>,
    action_label: &str,
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

    // Reserve the top row for a right-aligned action button (Sync → / Remove);
    // the card + details box render in the body below it.
    let body = if let Some(rect) = nav_detail_action_rect(area, action_label) {
        let button = Paragraph::new(Line::from(Span::styled(
            format!("[ {action_label} ]"),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        frame.render_widget(button, rect);
        Rect::new(
            area.x,
            area.y + 1,
            area.width,
            area.height.saturating_sub(1),
        )
    } else {
        area
    };

    // Header card: the selected skill's metadata.
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
    if let Some(status) = status {
        meta.push(Line::from(vec![
            Span::styled("status    ", Style::default().fg(Color::DarkGray)),
            status,
        ]));
    }

    let card_h = (meta.len() as u16 + 2).min(body.height);
    let card = Rect::new(body.x, body.y, body.width, card_h);
    let card_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {name} "));
    frame.render_widget(Paragraph::new(meta).block(card_block), card);

    // Room below the header card for full details, later.
    let below_y = body.y + card_h;
    let below_h = (body.y + body.height).saturating_sub(below_y);
    if below_h > 0 {
        let below = Rect::new(body.x, below_y, body.width, below_h);
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

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    // A pending status line takes over the footer until the next keypress.
    if let Some(status) = &app.status {
        let color = if status.contains("failed") {
            Color::Red
        } else {
            Color::Green
        };
        frame.render_widget(
            Paragraph::new(status.as_str()).style(Style::default().fg(color)),
            area,
        );
        return;
    }
    let text = match &app.screen {
        Screen::Setup(_) => "Tab complete · ⏎ continue · Esc quit",
        Screen::Main(_) if app.overlay.is_some() => {
            "↑↓ move · space toggle · ⏎ select · esc cancel"
        }
        Screen::Main(m) if m.settings_open => "esc close settings · q quit",
        Screen::Main(m) => match m.active {
            Tab::Global => "↑↓ select · x remove · ↹ tab · f refresh · , settings · q quit",
            Tab::Catalog => "↑↓ select · s sync · ↹ tab · f refresh · , settings · q quit",
            _ => "↑↓ select · ↹ tab · f refresh · , settings · q quit",
        },
    };
    frame.render_widget(
        Paragraph::new(text).style(Style::default().fg(Color::DarkGray)),
        area,
    );
}

/// The centered modal box + its content lines + the clickable rects for each
/// interactive item. Built once so [`render_overlay`] and [`modal_hitmap`] agree.
pub struct ModalView {
    pub rect: Rect,
    pub title: &'static str,
    pub lines: Vec<Line<'static>>,
    /// `(clickable rect, activation index)` — the index space `app` activates on.
    pub hits: Vec<(Rect, usize)>,
}

/// Clickable regions of the open modal (delegates to [`modal_view`]).
pub fn modal_hitmap(area: Rect, overlay: &Overlay) -> Vec<(Rect, usize)> {
    modal_view(area, overlay).hits
}

/// Append an interactive modal item as its own line, recording it for hit-testing
/// and highlighting it when focused.
fn modal_item(
    lines: &mut Vec<Line<'static>>,
    items: &mut Vec<(usize, usize)>,
    focus: usize,
    idx: usize,
    text: String,
) {
    let style = if focus == idx {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    items.push((lines.len(), idx));
    lines.push(Line::from(Span::styled(text, style)));
}

fn modal_view(area: Rect, overlay: &Overlay) -> ModalView {
    // Each interactive item gets its own full-width line, so a click maps to one
    // row. `modal_item` records the item's line index for hit-testing.
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut items: Vec<(usize, usize)> = Vec::new(); // (line index, item index)

    let dim = Style::default().fg(Color::DarkGray);

    let title = match overlay {
        Overlay::Sync(m) => {
            lines.push(Line::from(Span::styled(
                format!("Sync '{}'  ({})", m.skill, m.origin),
                Style::default().add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("destination", dim)));
            let dot = |on: bool| if on { "(•)" } else { "( )" };
            modal_item(
                &mut lines,
                &mut items,
                m.focus,
                0,
                format!("  {} Global", dot(m.dest == SyncDest::Global)),
            );
            modal_item(
                &mut lines,
                &mut items,
                m.focus,
                1,
                format!(
                    "  {} Project — coming soon",
                    dot(m.dest == SyncDest::Project)
                ),
            );
            lines.push(Line::from(""));
            if m.links.is_empty() {
                lines.push(Line::from(Span::styled(
                    "link into: (no other agent dirs detected)",
                    dim,
                )));
            } else {
                lines.push(Line::from(Span::styled("link into", dim)));
                for (i, link) in m.links.iter().enumerate() {
                    let mark = if link.on { "[x]" } else { "[ ]" };
                    modal_item(
                        &mut lines,
                        &mut items,
                        m.focus,
                        2 + i,
                        format!("  {} {}", mark, link.label),
                    );
                }
            }
            lines.push(Line::from(""));
            let confirm = 2 + m.links.len();
            modal_item(
                &mut lines,
                &mut items,
                m.focus,
                confirm,
                "  [ Sync ]".to_string(),
            );
            modal_item(
                &mut lines,
                &mut items,
                m.focus,
                confirm + 1,
                "  [ Cancel ]".to_string(),
            );
            " sync skill "
        }
        Overlay::Remove(m) => {
            lines.push(Line::from(Span::styled(
                format!("Remove '{}'?", m.skill),
                Style::default().add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("from  ", dim),
                Span::raw(m.location.clone()),
            ]));
            if let Some(target) = &m.link_target {
                lines.push(Line::from(Span::styled(
                    format!("symlink → {target} (only the link is removed)"),
                    dim,
                )));
            }
            lines.push(Line::from(""));
            if m.in_catalog {
                lines.push(Line::from(Span::styled(
                    "In your Catalog — you can re-sync it afterward.",
                    Style::default().fg(Color::Green),
                )));
            } else if m.is_symlink {
                lines.push(Line::from(Span::styled(
                    "Not in your Catalog — but this only unlinks; the target is kept.",
                    Style::default().fg(Color::Yellow),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    "⚠ Not in your Catalog — removing it is permanent.",
                    Style::default().fg(Color::Red),
                )));
            }
            if !m.dependent_links.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!(
                        "also removes {} symlink(s): {}",
                        m.dependent_links.len(),
                        m.dependent_links.join(", ")
                    ),
                    dim,
                )));
            }
            lines.push(Line::from(""));
            modal_item(
                &mut lines,
                &mut items,
                m.focus,
                0,
                "  [ Remove ]".to_string(),
            );
            modal_item(
                &mut lines,
                &mut items,
                m.focus,
                1,
                "  [ Cancel ]".to_string(),
            );
            " remove skill "
        }
    };

    let width = 64.min(area.width.saturating_sub(4)).max(24);
    let height = (lines.len() as u16 + 2).min(area.height.max(3));
    let rect = centered(area, width, height);
    let inner_x = rect.x + 1;
    let inner_y = rect.y + 1;
    let inner_w = rect.width.saturating_sub(2);
    let hits = items
        .iter()
        .map(|(line_idx, item_idx)| {
            (
                Rect::new(inner_x, inner_y + *line_idx as u16, inner_w, 1),
                *item_idx,
            )
        })
        .collect();

    ModalView {
        rect,
        title,
        lines,
        hits,
    }
}

fn render_overlay(frame: &mut Frame, area: Rect, overlay: &Overlay) {
    let view = modal_view(area, overlay);
    frame.render_widget(Clear, view.rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(view.title)
        .border_style(Style::default().fg(Color::Cyan));
    frame.render_widget(Paragraph::new(view.lines).block(block), view.rect);
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

    fn draw(app: &mut App) -> String {
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
        let mut app = App::new(Config {
            repo_path: Some("/x".to_string()),
        });
        let text = draw(&mut app);
        assert!(text.contains("Dashboard"));
        assert!(text.contains("Catalog"));
        assert!(text.contains("hello world"));
    }

    #[test]
    fn setup_screen_shows_prompt() {
        let mut app = App::new(Config::default());
        let text = draw(&mut app);
        assert!(text.contains("first run"));
    }

    #[test]
    fn global_tab_groups_by_location_and_shows_sync() {
        let mut app = App::new(Config {
            repo_path: Some("/x".to_string()),
        });
        app.global_scan = GlobalScan {
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
        app.global.rows = crate::skills::nav_rows(&app.global_scan.groups);
        app.repo_scan = RepoScan::default();
        app.global.sel = 0; // herdr, the symlink
        press(&mut app, KeyCode::Char('3')); // Global
        let text = draw(&mut app);
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
    fn catalog_tab_shows_repo_skills_as_master_detail() {
        let mut app = App::new(Config {
            repo_path: Some("/x".to_string()),
        });
        app.repo_scan = RepoScan {
            groups: vec![
                SkillGroup {
                    label: "personal".to_string(),
                    skills: vec![SkillEntry::new("mine")],
                },
                SkillGroup {
                    label: "vendor".to_string(),
                    skills: Vec::new(),
                },
            ],
        };
        app.catalog.rows = crate::skills::nav_rows(&app.repo_scan.groups);
        app.catalog.sel = 0; // "mine"
        press(&mut app, KeyCode::Char('4')); // Catalog
        let text = draw(&mut app);
        assert!(text.contains("Catalog")); // nav block title
        assert!(text.contains("personal")); // group header
        assert!(text.contains("vendor")); // group header (empty → (none))
        assert!(text.contains("mine")); // card name + detail card title
        assert!(text.contains("not installed")); // detail status (not in a global dir)
        assert!(text.contains("details")); // details box below the header card
        assert!(text.contains("[ Sync")); // the detail action button
    }

    /// A configured app with one Global skill (in `~/.claude/skills`), repo empty.
    fn app_with_global_skill() -> App {
        let mut app = App::new(Config {
            repo_path: Some("/x".to_string()),
        });
        app.global_scan = GlobalScan {
            groups: vec![SkillGroup {
                label: "~/.claude/skills".to_string(),
                skills: vec![SkillEntry::new("orphan")],
            }],
        };
        app.global.rows = crate::skills::nav_rows(&app.global_scan.groups);
        app.global.sel = 0;
        app
    }

    #[test]
    fn global_detail_shows_remove_button() {
        let mut app = app_with_global_skill();
        press(&mut app, KeyCode::Char('3')); // Global
        assert!(draw(&mut app).contains("[ Remove ]"));
    }

    #[test]
    fn sync_modal_renders_destinations_links_and_buttons() {
        let mut app = App::new(Config {
            repo_path: Some("/x".to_string()),
        });
        app.repo_scan = RepoScan {
            groups: vec![SkillGroup {
                label: "personal".to_string(),
                skills: vec![SkillEntry::new("mine")],
            }],
        };
        app.catalog.rows = crate::skills::nav_rows(&app.repo_scan.groups);
        press(&mut app, KeyCode::Char('4')); // Catalog
        press(&mut app, KeyCode::Char('s')); // open sync modal
        let text = draw(&mut app);
        assert!(text.contains("Sync 'mine'"));
        assert!(text.contains("Global"));
        assert!(text.contains("Project"));
        assert!(text.contains("coming soon"));
        assert!(text.contains("[ Sync ]"));
        assert!(text.contains("[ Cancel ]"));
    }

    #[test]
    fn remove_modal_warns_when_skill_is_not_in_catalog() {
        let mut app = app_with_global_skill(); // repo is empty → not in catalog
        press(&mut app, KeyCode::Char('3')); // Global
        press(&mut app, KeyCode::Char('x')); // open remove modal
        let text = draw(&mut app);
        assert!(text.contains("Remove 'orphan'?"));
        assert!(text.contains("permanent")); // the not-in-catalog warning
        assert!(text.contains("[ Remove ]"));
    }

    #[test]
    fn remove_modal_lists_dependent_symlinks_for_the_canonical() {
        let mut app = app_with_global_skill();
        press(&mut app, KeyCode::Char('3')); // Global
        app.overlay = Some(crate::app::Overlay::Remove(crate::app::RemoveModal {
            skill: "orphan".to_string(),
            location: "~/.agents/skills".to_string(),
            path: "/whatever/orphan".to_string(),
            is_symlink: false,
            link_target: None,
            in_catalog: true,
            dependent_links: vec![
                "~/.claude/skills".to_string(),
                "~/.codex/skills".to_string(),
            ],
            focus: 1,
        }));
        let text = draw(&mut app);
        assert!(text.contains("also removes 2 symlink(s)"));
        assert!(text.contains("~/.claude/skills"));
    }
}
