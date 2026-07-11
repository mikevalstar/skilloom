# skilloom

A TUI-first manager for AI agent skills.

`skilloom` keeps your **personal skills** versioned in a dedicated git repo ([loom-skills](https://github.com/mikevalstar/loom-skills)) and tracks **third-party skills** from upstream git repos — then reconciles them, in both directions, across every agent and project that uses them. Think *chezmoi, but for the `SKILL.md` folders that Claude Code, Codex, Cursor and friends discover on disk.*

- **Two source kinds** — personal skills you author (round-tripped through the loom-skills repo) and third-party skills pulled from public git URLs.
- **Multi-agent** — one canonical `~/.agents/skills/<name>/`, symlinked into each agent's dir (`~/.claude/skills/`, `~/.cursor/skills/`, …).
- **Multi-scope** — applied globally, or vendored as real committed files into an individual project repo.
- **2-way reconcile** — a skill can change locally *and* upstream; skilloom classifies each as in-sync / locally-changed / upstream-changed / changed-on-both / source-gone, shows a diff, and lets you pick a side. No pinned base, no auto-merge.
- **TUI-first, headless underneath** — an interactive terminal UI does the mutating; `skilloom … --json` gives scripts (and [myplace](https://github.com/mikevalstar/myplace)) a read-only status/inventory view.

> 🚧 **Scaffold.** Nothing is built yet. This repo currently holds the documentation-first design (see [docs/](docs/README.md)) and a minimal Rust skeleton. The engine, the TUI, and the `--json` surface come next, in the planning and implementation phases. skilloom was spun out of myplace ([its ADR-0024](https://github.com/mikevalstar/myplace/blob/main/docs/adrs/0024-skills-management-as-separate-project.md)); the design intent is re-homed in [ADR-0003](docs/adrs/0003-skilloom-engine-design-and-scope.md).

## Two repos

skilloom is the **tool**; your skills live in a **separate repo**, the same way chezmoi is separate from your dotfiles repo:

| Repo | What it is |
|------|------------|
| [`skilloom`](https://github.com/mikevalstar/skilloom) (this repo, public) | The Rust TUI + engine that fetches, links, vendors, and reconciles skills |
| [`loom-skills`](https://github.com/mikevalstar/loom-skills) (private) | The source-of-truth git repo for your personal skills. skilloom manages its structure |

## Build from source

```sh
cargo build            # once dependencies land, produces target/debug/skilloom
cargo run              # currently prints a scaffold notice
```

Rust toolchain is pinned to stable via `rust-toolchain.toml`.

## Roadmap

A draft plan, to be refined in the planning phase — see [ADR-0003](docs/adrs/0003-skilloom-engine-design-and-scope.md) for the model behind it.

| Phase | Scope |
|-------|-------|
| 1 | **Core engine + `--json`.** Fetch personal skills from loom-skills and third-party skills from git URLs; canonical `~/.agents/skills/` store + per-agent symlinks; machine-local state; read-only `status`/inventory as `--json`. |
| 2 | **Reconcile TUI.** The interactive ratatui surface: classify (in-sync / locally-changed / upstream-changed / changed-on-both / source-gone), diff, and pick-a-side. |
| 3 | **Per-project vendoring.** Real committed `.agents/skills/` under a project repo, with project-scoped reconcile. |
| 4 | **myplace integration.** skilloom installed and surfaced as a managed tool / currency source by myplace (a myplace-side follow-up). |

## Stack

Built with **Rust** and the **ratatui** stack — ratatui + crossterm + tokio + serde + clap — modeled on [herdr](https://github.com/ogulcancelik/herdr) ([ADR-0002](docs/adrs/0002-rust-and-ratatui-for-the-tui.md)). The engine is TUI-free so the `--json` surface (and any future integration) is free.

## Documentation

This is a **documentation-first** project ([ADR-0001](docs/adrs/0001-documentation-first.md)): design decisions, feature specs, and workflows are written in [docs/](docs/README.md) before (or alongside) the code.

- [docs/adrs](docs/adrs/) — architecture decision records
- [docs/features](docs/features/) — feature specs
- [docs/workflows](docs/workflows/) — end-to-end flows the tool supports
- [docs/guides](docs/guides/) — developer guides for this repo and the libraries it uses
