---
title: ADR-0002 — Rust with the ratatui stack for the TUI (modeled on herdr)
status: accepted
created: 2026-07-11
updated: 2026-07-11
tags: [tui, rust, ratatui, crossterm, tokio, herdr]
supersedes: null
superseded-by: null
---

# ADR-0002: Rust with the ratatui stack for the TUI (modeled on herdr)

## Context

skilloom is TUI-first: the primary surface is an interactive terminal UI for reviewing and reconciling skills, with a headless `--json` mode underneath so the same operations work in scripts and, eventually, from myplace ([ADR-0003](0003-skilloom-engine-design-and-scope.md)).

Constraints and context that shape the choice:

- **Single self-contained binary.** A user should install one binary (`curl | sh`, a release archive, `cargo install`) with no language runtime to bootstrap. skilloom is not a bootstrap-before-anything tool like myplace is, but the same "one static binary, no runtime deps" property is still the cleanest distribution story.
- **Targets:** macOS (Apple Silicon and Intel) and Linux (amd64, arm64) — the machines that run coding agents.
- **The core must be TUI-free.** Every capability also runs headlessly (`skilloom status --json`), so the engine (git fetch, state, symlinks, reconcile classification) can't be welded to the rendering layer.
- **The author already runs [herdr](https://github.com/ogulcancelik/herdr)** — a Rust/ratatui terminal app — day to day, and wants to reuse that stack and the knowledge that comes with it. This project deliberately "largely copies the stack herdr uses."

Note this is the *opposite* call from myplace's ADR-0002, which chose Go + Charm over Rust + ratatui. The deciding factors there — a component/forms toolkit (Bubbles/Huh) and the fact that chezmoi is Go — don't dominate here: skilloom's hard part is the reconcile *model*, not form wizards, and the author's existing Rust/ratatui investment (herdr) tips it the other way. Different project, different context, different answer.

## Options considered

### Option A — Rust + the ratatui stack (chosen)

Static binaries, no runtime deps, and a direct reuse of the herdr stack the author already maintains:

- **[ratatui](https://ratatui.rs)** — the TUI framework (immediate-mode rendering; bring-your-own app architecture)
- **[crossterm](https://github.com/crossterm-rs/crossterm)** — cross-platform terminal backend (input, raw mode, alt-screen)
- **[tokio](https://tokio.rs)** — async runtime for the I/O-bound work (git fetches, filesystem scans) without blocking the render loop
- **[serde](https://serde.rs) / serde_json** — the `--json` contract and on-disk state serialization
- **[clap](https://docs.rs/clap)** — CLI command/flag structure, so `skilloom status --json` and friends exist independently of the TUI

Trade-off vs. Charm/Bubble Tea: ratatui is lower-level (no stock component/forms library equivalent to Bubbles/Huh, no Elm-style framework), so we own more of the app architecture. Accepted — it's the price of matching herdr and it keeps the dependency surface small.

Skilloom is a single-screen app, **not** a multiplexer, so herdr's multiplexer-specific pieces — `portable-pty`, the vendored terminal-VT parser, the unix-socket server — are explicitly *not* carried over. We copy the UI/runtime foundation, not the multiplexer.

### Option B — Go + Charm (the myplace stack)

Reuses myplace's stack and its component/forms toolkit. Rejected: it throws away the author's herdr investment and the explicit goal of copying herdr's stack, for a component library skilloom doesn't lean on heavily.

### Option C — TypeScript + Ink

Fastest iteration, but requires Node at runtime — fails the self-contained-binary goal. Eliminated.

## Decision

Option A: **Rust**, with the **ratatui** stack (ratatui + crossterm + tokio + serde + clap), modeled on herdr and reusing the author's existing knowledge of it. The core engine lives in TUI-free modules; the clap CLI (`--json`) and the ratatui app are both thin layers over it — the same layering discipline myplace enforces, so headless mode is free.

## Consequences

- Release builds cross-compile a matrix (`darwin/arm64`, `darwin/amd64`, `linux/amd64`, `linux/arm64`); distribution starts as GitHub releases + a `curl | sh` installer, with `cargo install` as a source path.
- Contributors (human or AI) need Rust and familiarity with an immediate-mode TUI loop — captured in the companion guide [ratatui-tui-stack.md](../guides/ratatui-tui-stack.md).
- The headless `--json` requirement is enforced structurally: TUI modules must never be imported by engine modules.
- **Follow-ups (implementation phase):** add the stack crates to `Cargo.toml` with pinned versions; decide the async boundary (how much of the engine is `async` vs. blocking work on a thread pool); pick the app-architecture pattern over ratatui's immediate mode (a single `App` state + event loop); choose the domain crates left open here — git access (`git2`/`gix` vs. shelling out to `git`), XDG paths (`directories`/`etcetera`), and diffing (`similar`) — each its own small decision when first needed.
