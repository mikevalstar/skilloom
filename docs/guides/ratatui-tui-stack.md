---
title: Working with the ratatui TUI stack
status: draft
created: 2026-07-11
updated: 2026-07-11
tags: [rust, ratatui, crossterm, tokio, tui]
audience: both
---

# Working with the ratatui TUI stack

> **Draft / starter.** Nothing is built yet, so this is the intended shape plus what we already know from the ecosystem and from herdr. It grows into a real gotchas-from-experience guide (like myplace's Charm guide) as we build. Chosen in [ADR-0002](../adrs/0002-rust-and-ratatui-for-the-tui.md).

## Purpose

How to build and extend the skilloom TUI with ratatui — the intended architecture, where each crate fits, and the sharp edges to expect.

## Background

The stack, modeled on [herdr](https://github.com/ogulcancelik/herdr) (minus its multiplexer-specific pieces — we do not carry over `portable-pty`, the vendored VT parser, or the unix-socket server):

| Crate | Role in skilloom |
|-------|------------------|
| [ratatui](https://ratatui.rs) | The TUI framework. **Immediate-mode**: you re-render the whole UI from state every frame via `terminal.draw(\|f\| …)`. No retained widget tree, no Elm-style framework — you own the app loop and state |
| [crossterm](https://github.com/crossterm-rs/crossterm) | Terminal backend: raw mode, the alternate screen, and the input event stream (`crossterm::event`). ratatui's default backend |
| [tokio](https://tokio.rs) | Async runtime for I/O-bound work — git fetches, filesystem scans — so it never blocks the render loop |
| [serde](https://serde.rs) / serde_json | The `--json` output contract and on-disk state (de)serialization |
| [clap](https://docs.rs/clap) | CLI command/flag structure, so `skilloom status --json` exists independently of the TUI |

## The guide

### The immediate-mode loop (the 30-second version)

Unlike Bubble Tea's Model/Update/View, ratatui gives you a render function and an event stream; **you** write the loop:

```rust
// sketch — not final architecture
let mut terminal = ratatui::init();           // raw mode + alt screen
let mut app = App::new(/* engine state */);
while app.running {
    terminal.draw(|frame| ui::render(frame, &app))?;   // pure: reads state, draws
    if let Some(ev) = events.next().await {            // input or an async result
        app.handle(ev);                                // the only place state changes
    }
}
ratatui::restore();                            // leave raw mode + alt screen
```

Keep the discipline myplace enforces on the Charm side: **`render` is pure** (reads `App`, draws, no side effects), and **state changes in exactly one place** (`handle`). It isn't enforced by the framework here, so it's on us.

### Side effects don't block the draw loop

All I/O — every git fetch, every skill-tree scan — runs as a tokio task whose result arrives back on the event stream (e.g. via an `mpsc` channel merged with the crossterm event stream). Never `await` a network/disk call inline in the draw path. This is the ratatui equivalent of "never block in `Update`" — the mistake to watch for in review.

### The engine is not the TUI

The layering rule from [ADR-0002](../adrs/0002-rust-and-ratatui-for-the-tui.md): the fetch / state / symlink / reconcile **engine** modules must never import ratatui or crossterm. The clap commands (`--json`) and the ratatui app are both thin layers over that engine. This is what makes headless mode — and any future myplace integration — free. Enforce it structurally (separate modules; engine has no TUI deps).

### Handing the terminal to a subprocess

Some operations may need the real terminal (`$EDITOR` on a skill, an interactive `git` prompt). Leave the alt-screen/raw-mode, run the child, then re-enter — don't run an interactive child underneath the running TUI. For non-interactive git, prefer explicit non-prompting invocations (see gotchas).

## Gotchas

*(Seeded from ecosystem knowledge and the myplace Charm guide's hard-won lessons; confirm/expand as we hit them.)*

- **Never block the draw loop.** Subprocess and network I/O go through tokio tasks that report back via a channel, not inline `await`s in `terminal.draw`.
- **A subprocess can still hang you** even off the render path, if the child opens `/dev/tty` for a prompt (git credential/passphrase prompts) — it grabs the terminal the TUI owns and waits for a keypress the TUI is consuming. Defend at the exec layer: close the child's stdin and pass non-prompting flags (`GIT_TERMINAL_PROMPT=0`). For genuinely interactive children, do the opposite — suspend the TUI and hand over the real terminal.
- **Always restore the terminal on exit *and* on panic.** Raw mode + alt screen must be undone or the user's shell is left wrecked. Install a panic hook that restores before printing the panic (ratatui provides helpers; wire one up early).
- **Measure display width, not bytes.** Terminal cells ≠ `str::len()`; use `unicode-width` for anything you truncate or align, or wide/CJK glyphs and emoji break fixed layouts.
- **Handle resize.** ratatui re-lays-out from the frame size each draw, so this is mostly free — but any cached layout math must react to the new size.
- **Don't `println!` while the TUI owns the screen.** Stray stdout corrupts the render. Route logs to a file (via `tracing`) while the alt screen is active.
- **Pick the app architecture deliberately.** ratatui is unopinionated; decide up front on a single `App` struct + event enum (vs. a component trait) so it stays consistent — a planning-phase decision, noted in ADR-0002's follow-ups.

## References

- ratatui docs & site: https://ratatui.rs (see the tutorials and the `examples/` in the repo — fastest way to learn a widget)
- ratatui repo: https://github.com/ratatui/ratatui
- crossterm: https://github.com/crossterm-rs/crossterm
- herdr (the stack we model on): https://github.com/ogulcancelik/herdr
- [ADR-0002 — Rust with the ratatui stack](../adrs/0002-rust-and-ratatui-for-the-tui.md)
