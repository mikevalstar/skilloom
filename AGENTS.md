# AGENTS.md

## What this project is

`skilloom` is a **TUI-first manager for AI agent skills** — the `SKILL.md` folders that Claude Code, Codex, Cursor and other agents discover on disk. It keeps **personal skills** versioned in a separate git repo (**loom-skills**, private) and tracks **third-party skills** from upstream git URLs, then syncs a curated selection to every agent and project that uses them, and back. The relationship is chezmoi-shaped: skilloom is the engine, loom-skills is the source-of-truth repo.

skilloom was spun out of [myplace](https://github.com/mikevalstar/myplace) — see myplace's ADR-0024. The design intent handed over is re-homed in this repo's [ADR-0003](docs/adrs/0003-skilloom-engine-design-and-scope.md) and **refined** by the [functional overview](docs/features/overview.md) — read both. In short: the loom-skills repo is a copy-based hub; skilloom copies skills between it and its destinations (global agent dirs + tracked projects); syncing is curated and opt-in; the interactive TUI mutates while a future `--json` surface reports. (ADR-0003's original "canonical store + symlinks + 2-way-diff-as-core" is superseded by that overview.)

> **Status: early, running — now mutating.** Built and verified: first-run setup (repo path + autocomplete, `~/.config/skilloom/config.toml`), the tabbed TUI shell (Dashboard/Projects/Global/Catalog + settings gear, keyboard + mouse incl. wheel), and two master-detail tabs sharing one widget: **Global** browses installed skills grouped by agent dir (symlink-aware, `SKILL.md` descriptions, scrollable) and **Catalog** browses the loom-skills repo's `personal/`+`vendor/` the same way. The **first write path** is in: from Catalog, **sync a skill → global** (copies to the canonical `~/.agents/skills/<name>` + symlinks into the other detected agent dirs; records it in the ledger `~/.config/skilloom/sync.toml`) via a modal (Project destination is a stub); and from Global, **remove** an installed skill with a symlink-/catalog-aware confirmation. A sample skill is checked into loom-skills. **Not yet built:** add-remote (remote → `vendor/`), import (global → `personal/`), project sync, and the `--json`/CLI surface. See [TODO.md](TODO.md).

## Documentation-first

This project is documentation-first ([ADR-0001](docs/adrs/0001-documentation-first.md)). **Before implementing a feature or making an architectural choice, write (or update) the relevant doc:**

- New tech/library/architecture choice → ADR in `docs/adrs/`
- New user-visible capability → spec in `docs/features/`
- New end-to-end flow the tool supports → `docs/workflows/`
- Knowledge a developer of this repo needs (library usage, conventions, gotchas) → `docs/guides/`

Each folder has a `_template.md` showing the expected format. **All docs use YAML frontmatter** (title, status, dates, tags) so they can be searched and filtered — never omit it. See [docs/README.md](docs/README.md) for structure and conventions.

When a decision changes, don't edit history: supersede the old ADR with a new one and update the old ADR's `status` field.

## Conventions

- ADRs are numbered sequentially: `0001-some-decision.md`, `0002-...`
- Other docs use kebab-case descriptive names: `reconcile-a-skill.md`
- Doc `status` values: `draft` → `accepted`/`active` → `superseded`/`deprecated`
- Dates in frontmatter are ISO format: `2026-07-11`
- **The README is part of the spec**: its install/usage/roadmap sections must be updated in the same change whenever the command surface, flags, or plan change. Docs explain design; the README shows a user how to run it.

## Settled design points

Decided but not all spec'd yet — write the feature/workflow doc before building on one of these:

- **Stack: Rust + the ratatui stack**, modeled on [herdr](https://github.com/ogulcancelik/herdr) — see [ADR-0002](docs/adrs/0002-rust-and-ratatui-for-the-tui.md) and `docs/guides/ratatui-tui-stack.md`. skilloom is a single-screen app, **not** a multiplexer — herdr's PTY/socket/VT pieces are not carried over. **Actual deps today:** `ratatui` 0.30, `crossterm` (via ratatui), `serde`+`toml`, `anyhow`. **Deferred until needed:** `tokio` (everything is synchronous fs so far — added only if network git needs it) and `clap` (added with the `--json`/CLI surface). Edition 2024; let-chains in use.
- **Engine model** — scope in [ADR-0003](docs/adrs/0003-skilloom-engine-design-and-scope.md); the current, refined mechanism is the [functional overview](docs/features/overview.md) and [TUI design](docs/features/tui-dashboard.md). In short: the **loom-skills repo is the hub** (`vendor/` for third-party, `personal/` for your own); skilloom **copies** skills between the repo and its destinations (global `~/.agents`/`~/.claude`, tracked project repos); syncing is **curated and opt-in per skill × destination**. **2-way diff/reconcile is deferred to "later"** — ADR-0003's canonical-store + symlink + reconcile-as-core details are superseded by the overview, with a formal superseding ADR as a follow-up.
- **Headless `--json` from day one**: every capability works as a plain CLI command with `--json` output and meaningful exit codes. The core engine must never be welded to the TUI layer, so `--json` (and any future myplace integration) stays free.
- **TUI mutates, `--json` reports**: interactive reconcile/apply lives in the TUI; `--json` is read-only status/inventory. Mutation off a TTY is explicit and flag-driven, never a side effect of a status read.
- **Config vs. state vs. ledger.** skilloom's *config* (loom-skills repo location, tracked projects, agent targets, curation) lives in `~/.config/skilloom/config.toml`; the *sync ledger* (which skill × destination is synced) lives beside it in `~/.config/skilloom/sync.toml` — it's curation/intent, not derived; derived *state* (last-sync timestamps, resolved commits, content hashes for change-detection) will live in the state dir (`$XDG_STATE_HOME/skilloom` or platform equivalent), not yet built. Never write skilloom's files into another tool's config tree (`~/.claude`, `~/.agents`) — except deliberate skill *content* the user synced there.
- **loom-skills is a separate repo**: the personal-skills source of truth, like a chezmoi dotfiles repo. skilloom manages its structure; it starts as just a README.

## Layout

```
src/main.rs        terminal setup, event loop, config I/O
src/app.rs         App/Screen/Tab state + input handling (no terminal I/O — unit-tested)
src/ui.rs          rendering only (ratatui); shares hit-test geometry with app
src/config.rs      ~/.config/skilloom/config.toml load/save
src/paths.rs       $HOME / XDG / ~ expansion
src/complete.rs    directory autocomplete for the setup field
src/skills.rs      scan skills (global agent dirs + repo), SKILL.md frontmatter   ← engine, TUI-free
src/sync.rs        the write path: copy/symlink/remove skills (repo → global)     ← engine, TUI-free
src/ledger.rs      the sync ledger (~/.config/skilloom/sync.toml)                 ← engine, TUI-free
src/scroll.rs      reusable, content-agnostic vertical scroll state               ← reusable
docs/              documentation-first tree (adrs, features, workflows, guides)
.github/workflows/ CI: fmt + clippy + build + test
```

Mutations use a **pending-op** pattern: `on_key`/`on_mouse` stay pure (they queue an `App.pending` op and manage modal state), and the runner in `main` executes it after input — running the `sync` engine, saving the `ledger`, rescanning, and reporting via a footer status line. That keeps input handling unit-testable and the fs writes in one place.

Keep the engine modules TUI-free: `skills`/`sync`/`ledger`/`config`/`paths`/`complete`/`scroll` must never import ratatui/crossterm — only `app`/`ui` (and `main`) touch the terminal. That split is what will make the future `--json` surface free. `app` holds testable state + input handling (no terminal I/O); `ui` only draws; geometry used for mouse hit-testing (tab spans, the master-detail layout, the modal hitmap) is computed by shared functions so what's drawn and what's clickable can't drift.

## Project state

Running TUI with its **first write path**. Implemented + tested (54 tests: unit + ratatui `TestBackend` render checks + `sync`/`ledger` fs tests on tempdirs; fmt/clippy clean under `-D warnings`; CI green on Linux):

- **Setup** — first-run screen: repo-path field with directory autocomplete; writes `~/.config/skilloom/config.toml`.
- **Shell** — tab bar (Dashboard/Projects/Global/Catalog) + settings gear; keyboard (`↹`/`1-4`/`,`/`q`) and mouse (click tabs & gear, scroll wheel).
- **Master-detail widget** (shared by Global + Catalog) — a scrollable left nav of two-line skill cards beside a detail pane (metadata header card + details box + a top-right action button). `app::NavState` holds the per-tab view state; `skills::nav_rows` + the `NavRow` geometry drive both drawing and click hit-testing.
- **Global tab** — installed skills grouped by agent dir; symlinks flagged (`@`); name-match "synced (in repo)" status; **`x`** / `[ Remove ]` opens a remove-confirm modal.
- **Catalog tab** — the repo's `personal/`+`vendor/` in that widget; name-match "installed globally" status; **`s`** / `[ Sync → ]` opens the sync modal. A `sample-skill` is checked into loom-skills.
- **Sync (repo → global)** — `sync::sync_to_global` copies to the canonical `~/.agents/skills/<name>` and symlinks into the detected agent dirs; `ledger` appends to `sync.toml`. **Remove** — `sync::remove_installed` unlinks/deletes an installed skill, and when it's the canonical copy also cleans up the symlinks that pointed at it (so nothing dangles). Both run via the pending-op pattern (queued in pure input handling, executed in `main`), with a footer status line and modal overlays (`ui::modal_view`/`modal_hitmap` share geometry between draw + click).

**Not built:** add-remote (remote → `vendor/`), import (global → `personal/`), project sync (a modal stub), `--json`/CLI. Dashboard and Projects are still placeholders. Verification note: the interactive TUI can't be driven headlessly here, so behavior is covered by `TestBackend` render tests + input unit tests + tempdir fs tests, and confirmed against the real machine via throwaway `--ignored` render dumps. Next steps: [TODO.md](TODO.md).
