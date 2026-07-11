//! skilloom — a TUI-first manager for AI agent skills.
//!
//! First slice: the first-run setup screen (point at the loom-skills repo, with
//! directory autocomplete) and the tabbed app shell (Dashboard / Projects /
//! Global / Catalog + a settings gear), each tab a placeholder for now.
//!
//! Layering (per docs/adrs/0002): `app` holds testable state + input handling;
//! `ui` only draws; this file owns the terminal, the event loop, and config I/O.

mod app;
mod complete;
mod config;
mod paths;
mod skills;
mod ui;

use std::io::{self, Stdout};
use std::time::Duration;

use anyhow::Result;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::layout::Rect;

use crate::app::App;
use crate::config::Config;

type Tui = Terminal<CrosstermBackend<Stdout>>;

fn main() -> Result<()> {
    let config = Config::load()?;
    let mut terminal = init_terminal()?;
    install_panic_hook();
    let mut app = App::new(config);
    let result = run(&mut terminal, &mut app);
    restore_terminal(&mut terminal)?;
    result
}

fn run(terminal: &mut Tui, app: &mut App) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::render(frame, app))?;

        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => app.on_key(key),
                Event::Mouse(mouse) => {
                    let size = terminal.size()?;
                    app.on_mouse(mouse, Rect::new(0, 0, size.width, size.height));
                }
                _ => {}
            }
        }

        if app.save_requested {
            app.config.save()?;
            app.save_requested = false;
        }
    }
    Ok(())
}

fn init_terminal() -> Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Tui) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Restore the terminal on panic so a crash doesn't leave the shell in raw mode.
fn install_panic_hook() {
    let original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original(info);
    }));
}
