# AGENTS.md

## What this project is

`skilloom` is a **TUI-first manager for AI agent skills** — the `SKILL.md` folders that Claude Code, Codex, Cursor and other agents discover on disk. It keeps **personal skills** versioned in a separate git repo (**loom-skills**, private) and tracks **third-party skills** from upstream git URLs, then reconciles them in both directions across every agent and project that uses them. The relationship is chezmoi-shaped: skilloom is the engine, loom-skills is the source-of-truth repo.

skilloom was spun out of [myplace](https://github.com/mikevalstar/myplace) — see myplace's ADR-0024. The design intent handed over is re-homed in this repo's [ADR-0003](docs/adrs/0003-skilloom-engine-design-and-scope.md). Read that first; it defines the model (two source kinds, canonical store + per-agent symlinks, global + per-project scopes, 2-way reconcile, own state dir, TUI-mutates/`--json`-read-only).

> **Status: scaffold.** Documentation-first design + a minimal Rust skeleton only. No engine, TUI, or CLI yet. We are heading into a planning phase to spec the feature set before building.

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

- **Stack: Rust + the ratatui stack** (ratatui, crossterm, tokio, serde, clap), modeled on [herdr](https://github.com/ogulcancelik/herdr) — see [ADR-0002](docs/adrs/0002-rust-and-ratatui-for-the-tui.md) and `docs/guides/ratatui-tui-stack.md`. skilloom is a single-screen app, **not** a multiplexer — herdr's PTY/socket/VT pieces are not carried over.
- **Engine model** ([ADR-0003](docs/adrs/0003-skilloom-engine-design-and-scope.md)): two source kinds (personal via loom-skills, third-party via git URLs); canonical `~/.agents/skills/<name>/` symlinked into each agent dir; global + per-project (vendored, committed) scopes; 2-way "track latest" reconcile (classify in-sync / locally-changed / upstream-changed / changed-on-both / source-gone, diff, pick-a-side — no pinned base, no auto-merge).
- **Headless `--json` from day one**: every capability works as a plain CLI command with `--json` output and meaningful exit codes. The core engine must never be welded to the TUI layer, so `--json` (and any future myplace integration) stays free.
- **TUI mutates, `--json` reports**: interactive reconcile/apply lives in the TUI; `--json` is read-only status/inventory. Mutation off a TTY is explicit and flag-driven, never a side effect of a status read.
- **Machine-local state under the state dir** (`$XDG_STATE_HOME/skilloom` or platform equivalent), **never** `~/.config` — that's an agent/tool config tree.
- **loom-skills is a separate repo**: the personal-skills source of truth, like a chezmoi dotfiles repo. skilloom manages its structure; it starts as just a README.

## Layout

```
Cargo.toml, src/       the Rust binary (scaffold for now)
rust-toolchain.toml    pinned to stable + rustfmt + clippy
docs/                  documentation-first tree (adrs, features, workflows, guides)
.github/workflows/     CI: fmt + clippy + build + test
```

When code lands, keep the engine modules TUI-free: the ratatui/crossterm layer must never be imported by the fetch/state/reconcile core. That structural rule is what makes `--json` free.

## Project state

Scaffold only. This repo contains: the documentation-first `docs/` tree with the three foundational ADRs (docs-first, Rust+ratatui stack, engine design & scope) and the four doc templates; a minimal Cargo skeleton that builds and prints a placeholder; `.gitignore`, `rust-toolchain.toml`, and a basic CI workflow. Next up: the planning phase — turn ADR-0003's model into feature/workflow specs, then implement phase 1 (core engine + `--json`). See [TODO.md](TODO.md).
