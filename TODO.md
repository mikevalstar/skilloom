# TODO

High-level next steps. Detail lands in `docs/` as we go (documentation-first — [ADR-0001](docs/adrs/0001-documentation-first.md)).

## Planning phase (in progress)

Design docs (medium detail) — done:

- [x] **Functional overview** — repo-as-hub model, surfaces, sync directions: [docs/features/overview.md](docs/features/overview.md).
- [x] **TUI design sketch** — screens, legend, keymap, navigation: [docs/features/tui-dashboard.md](docs/features/tui-dashboard.md).

Specs still to write before engine code:

- [ ] **Config format** (`~/.config/skilloom/config.toml`) — repo location, tracked projects, agent targets, and the which-goes-where curation.
- [ ] **Repo layout spec** — `vendor/` + `personal/` and the per-skill `.skilloom.toml` (source, ref, synced-at).
- [ ] **Sync engine + ledger** — the directions in the overview, how "which skill is synced where" is tracked, and how status (`●↑▲↕○✗`) is computed.
- [ ] **Global mechanism decision** — canonical `~/.agents/skills` + symlinks vs. copy into each agent dir (an ADR).
- [ ] **Headless `--json` contract** — commands, output schema, exit codes; the read-only/mutation split.
- [ ] **Workflow docs** — "add a remote skill", "import a global skill", "install a skill into a project".
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
- [ ] Confirm the loom-skills repo layout once the source/manifest spec exists (it currently holds only a README on purpose).
