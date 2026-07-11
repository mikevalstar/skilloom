---
title: Working with the ratatui TUI stack
status: active
created: 2026-07-11
updated: 2026-07-11
tags: [rust, ratatui, crossterm, tui]
audience: both
---

# Working with the ratatui TUI stack

> **Active — reflects what's built.** The setup screen, tabbed shell, and Global browser are implemented, so this guide describes the real patterns (a synchronous event loop, `render(&mut App)` for scroll, geometry shared between drawing and mouse hit-testing, `TestBackend` render tests) alongside ecosystem knowledge. Chosen in [ADR-0002](../adrs/0002-rust-and-ratatui-for-the-tui.md).

## Purpose

How to build and extend the skilloom TUI with ratatui — the architecture we settled on, where each crate fits, and the sharp edges.

## Background

The stack, modeled on [herdr](https://github.com/ogulcancelik/herdr) (minus its multiplexer-specific pieces — we do not carry over `portable-pty`, the vendored VT parser, or the unix-socket server):

| Crate | Role in skilloom |
|-------|------------------|
| [ratatui](https://ratatui.rs) 0.30 | The TUI framework. **Immediate-mode**: re-render the whole UI from state every frame via `terminal.draw(\|f\| …)`. No retained widget tree — you own the app loop and state. crossterm is re-exported at `ratatui::crossterm` (use that, don't add crossterm separately, to avoid version skew) |
| crossterm (via ratatui) | Terminal backend: raw mode, alt screen, mouse capture, the input event stream |
| [serde](https://serde.rs) + [toml](https://docs.rs/toml) | Config load/save today; the `--json` contract and on-disk state later |
| [anyhow](https://docs.rs/anyhow) | Error handling in `main` and the IO paths |
| ~~tokio~~ · **deferred** | Not used yet — fs scanning is synchronous and fast, and the loop is a blocking `event::poll` + `read`. Add only if network git wants concurrency |
| ~~clap~~ · **deferred** | Added with the `--json`/CLI surface; the TUI needs no flags yet |

## The guide

### Module layout (the TUI is a skin)

```
main.rs      terminal setup/restore (+ panic hook), the event loop, config I/O
app.rs       App / Screen / Tab state + input handling — NO terminal I/O (unit-tested)
ui.rs        rendering only; owns the ratatui calls
config.rs    ~/.config/skilloom/config.toml
paths.rs     $HOME / XDG / ~ expansion
complete.rs  directory autocomplete
skills.rs    scan skills + parse SKILL.md frontmatter   ← engine, TUI-free
scroll.rs    reusable vertical scroll state             ← reusable
```

`skills`/`config`/`paths`/`complete`/`scroll` must never import ratatui/crossterm. Only `app`/`ui`/`main` touch the terminal. That's what keeps the future `--json` surface cheap.

### The event loop (synchronous)

We do **not** use async. The loop is a blocking poll:

```rust
while !app.should_quit {
    terminal.draw(|frame| ui::render(frame, app))?;
    if event::poll(Duration::from_millis(250))? {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => app.on_key(key),
            Event::Mouse(m) => app.on_mouse(m, terminal_area(terminal)?),
            _ => {}
        }
    }
    if app.save_requested { app.config.save()?; app.save_requested = false; }
}
```

`app.on_key`/`on_mouse` only mutate state and set flags (`should_quit`, `save_requested`); `main` performs the actual persistence. That keeps input handling pure enough to unit-test without a terminal. Guard on `KeyEventKind::Press` (Windows sends Release/Repeat too).

### `render` is (almost) pure — the one stateful exception is scroll

Rendering reads `App` and draws. The **one** thing it mutates is view state that depends on the live viewport size — the scroll offset — done in a small pre-step at the top of `render`, the ratatui `StatefulWidget` philosophy:

```rust
pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    // keep the Global selection scrolled into view (needs the live height)
    if /* on Global tab */ {
        let viewport = /* nav inner height */;
        app.global_scroll.focus(start, len, viewport, total); // scroll::Scroll
    }
    let app = &*app; // reborrow immutable; the rest is pure
    // …draw from state…
}
```

`scroll::Scroll` is content-agnostic and reusable (offset + minimal-movement `focus`). Keeping the offset in `App` (not recomputed each frame) is what makes scrolling only move when the selection leaves the viewport.

### Mouse: share geometry between drawing and hit-testing

Mouse capture is enabled at startup (`EnableMouseCapture`). To make clicks land where things are drawn, compute layout geometry **once** in a shared function that both the renderer and the hit-tester call — never duplicate the math. Examples: `ui::tab_spans(tabbar)` returns each tab's `Rect` (used to draw the bar *and* to resolve a click to a tab); `ui::global_layout(content)` splits the nav/detail the same way for both. The Global left-nav is scroll-aware, so click hit-testing adds the scroll `offset` before mapping a row to a skill. The scroll **wheel** (`MouseEventKind::ScrollUp/Down`) moves the selection (the view follows), consistent with `↑↓`.

### Two-line cards & fixed-width lines

The Global nav renders each skill as a 2-line card (name + grayed `SKILL.md` description). Because a skill spans two visual lines, one shared helper (`skills::nav_row_height` / `skill_index_at_line`) drives both rendering order and click math so they can't drift. Right-floating the symlink `@` and truncating the subtitle use small string helpers (`float_right`, `truncate_chars`, `pad_to`) that pad to the inner width so the selection highlight fills the row.

### Handing the terminal to a subprocess

Some future operations need the real terminal (`$EDITOR` on a skill, an interactive `git` prompt). Leave the alt-screen/raw-mode, run the child, then re-enter — don't run an interactive child underneath the running TUI. For non-interactive git, pass explicit non-prompting invocations (see gotchas).

## Gotchas

- **`render` takes `&mut App` (for scroll) but stays otherwise pure.** Do the scroll pre-step, then `let app = &*app;` and draw immutably. Don't sneak domain-state changes into render.
- **Test with `ratatui::backend::TestBackend`.** The interactive TUI can't be driven headlessly here, so `terminal.draw` into a `TestBackend` and assert the buffer text. Input handling (`app.on_key`/`on_mouse`) is plain unit-testable since it does no I/O. For a real visual check against the machine, a throwaway `#[ignore]` test that prints the `TestBackend` buffer beats guessing — write it, look, delete it.
- **Measure display width, not bytes.** Terminal cells ≠ `str::len()`. We currently use `chars().count()` for widths/truncation, which is fine for the ASCII-ish skill names but *wrong* for wide/CJK/emoji — switch to `unicode-width` if that becomes real (tracked as a possible follow-up).
- **Always restore the terminal on exit *and* panic.** We install a panic hook that disables raw mode + leaves the alt screen before the default hook runs. Without it a crash wrecks the shell.
- **Clippy runs with `-D warnings` in CI.** Collapse nested `if`s into let-chains (edition 2024), give any argless `new()` a `Default`, and gate test-only helpers with `#[cfg(test)]` (a bin crate warns on unused `pub`). These have all bitten us once.
- **Don't `println!` while the TUI owns the screen** — it corrupts the render. (When logging arrives, route it to a file.)
- **Handle `Event::Resize` implicitly:** ratatui re-lays-out from `frame.area()` each draw, so resize mostly just works — but any cached geometry must react to the new size.
- **A subprocess can hang you** if a child opens `/dev/tty` for a prompt (git credential/passphrase). Defend at the exec layer: close the child's stdin and pass non-prompting flags (`GIT_TERMINAL_PROMPT=0`); for genuinely interactive children, suspend the TUI and hand over the real terminal.

## References

- ratatui docs & site: https://ratatui.rs (tutorials + `examples/`)
- ratatui repo: https://github.com/ratatui/ratatui
- crossterm: https://github.com/crossterm-rs/crossterm
- herdr (the stack we model on): https://github.com/ogulcancelik/herdr
- [ADR-0002 — Rust with the ratatui stack](../adrs/0002-rust-and-ratatui-for-the-tui.md)
