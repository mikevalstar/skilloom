# skilloom

A TUI-first manager for AI agent skills.

`skilloom` keeps your **personal skills** versioned in a dedicated git repo ([loom-skills](https://github.com/mikevalstar/loom-skills)) and tracks **third-party skills** from upstream git repos — then syncs a curated selection out to every agent and project that needs them, and back. Think *chezmoi, but for the `SKILL.md` folders that Claude Code, Codex, Cursor and friends discover on disk.*

- **Repo as hub** — your loom-skills repo holds the real files: `vendor/` for third-party skills (copied in, with source metadata) and `personal/` for your own.
- **Flows every way** — copy skills out of the repo into your global agent dirs (`~/.agents`, `~/.claude`) and into project repos, and capture skills back up into the repo.
- **Global + per-project** — install a curated skill globally and/or into individual project repos; you choose where each one goes.
- **Curated & selective** — syncing is opt-in per skill per destination: a project-specific skill needn't flow up, a repo skill needn't land in every project.
- **TUI-first, headless underneath** — an interactive terminal UI does the mutating; `skilloom … --json` gives scripts (and [myplace](https://github.com/mikevalstar/myplace)) a read-only status view. *(Diff-while-syncing, tags, search, and filtering come later.)*

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

A draft plan — see the [functional overview](docs/features/overview.md) for the model and [ADR-0003](docs/adrs/0003-skilloom-engine-design-and-scope.md) for the scope.

| Phase | Scope |
|-------|-------|
| 1 | First-run config (point at loom-skills), the repo-as-hub layout, and the dashboard shell. |
| 2 | Add & import: remote skills → `vendor/`, global skills (`~/.agents`/`~/.claude`) → `personal/`, track project folders. |
| 3 | Sync engine + ledger: repo ↔ global, repo → projects, remote → repo — curated per skill × destination, with status. |
| Later | Tagging, searching, filtering, and diff-while-syncing. |
| — | myplace integration (surface skilloom as a managed tool — a myplace-side follow-up). |

## Stack

Built with **Rust** and the **ratatui** stack — ratatui + crossterm + tokio + serde + clap — modeled on [herdr](https://github.com/ogulcancelik/herdr) ([ADR-0002](docs/adrs/0002-rust-and-ratatui-for-the-tui.md)). The engine is TUI-free so the `--json` surface (and any future integration) is free.

## Documentation

This is a **documentation-first** project ([ADR-0001](docs/adrs/0001-documentation-first.md)): design decisions, feature specs, and workflows are written in [docs/](docs/README.md) before (or alongside) the code.

- [docs/adrs](docs/adrs/) — architecture decision records
- [docs/features](docs/features/) — feature specs
- [docs/workflows](docs/workflows/) — end-to-end flows the tool supports
- [docs/guides](docs/guides/) — developer guides for this repo and the libraries it uses

## License

MIT — see [LICENSE](LICENSE).
