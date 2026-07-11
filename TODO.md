# TODO

High-level next steps. Detail lands in `docs/` as we go (documentation-first — [ADR-0001](docs/adrs/0001-documentation-first.md)).

## Building — phase 1 (in progress)

Small testable slices:

- [x] First-run setup screen — repo path field with directory autocomplete; saves `~/.config/skilloom/config.toml`.
- [x] Tabbed app shell — Dashboard / Projects / Global / Catalog + settings gear; keyboard (`↹`/`1-4`/`,`) and mouse (click tabs) nav; placeholder per tab.
- [x] Global tab: left-nav grouped by location (`~/.claude`/`.agents`/`.codex`/`.cursor`), keyboard + mouse select a skill → detail; synced status (in repo or not). `f` rescans.
- [x] Symlink-aware: flag symlinked skills (`@`) and show their real target.
- [x] Two-line skill cards (name + `SKILL.md` description subtitle, `@` floated right); right pane is a metadata header card with a "details" box below.
- [x] Left-nav scrolling: reusable `scroll::Scroll` keeps the selection in view, with a scrollbar. Built content-agnostic for reuse.
- [x] Promote the card list + master-detail into a reusable widget — `app::NavState` + the shared `skills::nav_rows`/`NavRow` geometry and `ui::render_master_detail`; Global and Catalog are now two callers.
- [x] Catalog tab: repo skills (`personal/` + `vendor/`) in that master-detail widget, with an "installed globally" status; a `sample-skill` is checked into loom-skills. Layout documented in [docs/guides/loom-skills-repo-layout.md](docs/guides/loom-skills-repo-layout.md).
- [x] **First write path — sync repo → global.** Sync modal on Catalog (`s` / `[ Sync → ]`): copies to canonical `~/.agents/skills/<name>` + symlinks into detected agent dirs (default-on toggles), records in `~/.config/skilloom/sync.toml`. Project destination is a stub. [Workflow](docs/workflows/sync-a-skill-to-global.md).
- [x] **Remove a global skill** — confirm modal on Global (`x` / `[ Remove ]`), symlink- and catalog-aware ("permanent" when not in the repo). [Workflow](docs/workflows/remove-a-global-skill.md).
- [x] Modal/overlay system + pending-op execution (pure input → `main` runs the fs op); footer status line.
- [ ] Fill the "details" box with the `SKILL.md` body (both tabs).
- [ ] Populate Projects from config + disk (the third `NavState` caller) → then wire Project as a real sync destination.
- [ ] Add-remote flow (git repo → `vendor/<name>/` + `.skilloom.toml`).
- [ ] Grow the ledger into change-detection: content/commit compare (repo vs. installed), and "remove everywhere" (canonical + its symlinks).

## Planning phase (in progress)

Design docs (medium detail) — done:

- [x] **Functional overview** — repo-as-hub model, surfaces, sync directions: [docs/features/overview.md](docs/features/overview.md).
- [x] **TUI design sketch** — screens, legend, keymap, navigation: [docs/features/tui-dashboard.md](docs/features/tui-dashboard.md).

Specs still to write before engine code:

- [ ] **Config format** (`~/.config/skilloom/config.toml`) — repo location, tracked projects, agent targets, and the which-goes-where curation. *(Partly settled: the sync ledger `sync.toml` format is in use — see the sync workflow doc.)*
- [ ] **Repo layout spec** — `vendor/` + `personal/` and the per-skill `.skilloom.toml` (source, ref, synced-at). *(Layout guide written; the `.skilloom.toml` manifest is still specced-not-built.)*
- [~] **Sync engine + ledger** — repo → global is built (canonical + symlinks + `sync.toml`); remaining directions and the `●↑▲↕○✗` change-detection status still to spec/build.
- [x] **Global mechanism decision** — settled: canonical `~/.agents/skills` copy + symlinks into the other detected agent dirs (per-sync toggles). A formal ADR would still be good; captured in the [sync workflow](docs/workflows/sync-a-skill-to-global.md) + overview for now.
- [ ] **Headless `--json` contract** — commands, output schema, exit codes; the read-only/mutation split.
- [~] **Workflow docs** — done: [sync a skill to global](docs/workflows/sync-a-skill-to-global.md), [remove a global skill](docs/workflows/remove-a-global-skill.md). Still: "add a remote skill", "import a global skill", "install a skill into a project".
- [ ] **Superseding ADR for ADR-0003** — record the mechanism change (repo-as-hub, copy-based, diff deferred) once the model locks.

## Implementation phase (after planning)

- [ ] Add the stack crates to `Cargo.toml` with pinned versions (ratatui, crossterm, tokio, serde/serde_json, clap) — [ADR-0002](docs/adrs/0002-rust-and-ratatui-for-the-tui.md).
- [ ] Decide the open domain-crate choices: git access (`git2`/`gix` vs. shell out), XDG paths, diffing (`similar`).
- [ ] Phase 1: core engine + `status --json` (fetch, store, symlinks, state).
- [ ] Phase 2: the reconcile TUI.
- [ ] Phase 3: per-project vendoring.
- [ ] Release pipeline (cross-compile matrix + `curl | sh` installer) and, separately, the myplace integration (myplace-side).

## Deferred / open

- [x] Add MIT `LICENSE` and set `license = "MIT"` in `Cargo.toml`.
- [x] Document the loom-skills repo layout — [docs/guides/loom-skills-repo-layout.md](docs/guides/loom-skills-repo-layout.md); a `sample-skill` is checked in under `personal/`. The per-skill `.skilloom.toml` manifest is still specced-not-built (see the Repo layout spec above).
