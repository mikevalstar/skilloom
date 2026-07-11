# TODO

High-level next steps. Detail lands in `docs/` as we go (documentation-first — [ADR-0001](docs/adrs/0001-documentation-first.md)).

## Planning phase (next)

Turn the model in [ADR-0003](docs/adrs/0003-skilloom-engine-design-and-scope.md) into concrete specs before writing engine code:

- [ ] **Feature spec: skill source config / manifest** — how personal (loom-skills) and third-party (git URL) sources are declared and resolved.
- [ ] **Feature spec: canonical store + multi-agent symlinks** — `~/.agents/skills/<name>/` and which agent dirs get linked.
- [ ] **Feature spec: machine-local state** — what skilloom records (name → source, scope, resolved commit, targets) and where.
- [ ] **Feature spec: reconcile** — the classifier (in-sync / locally-changed / upstream-changed / changed-on-both / source-gone), the diff, and pick-a-side semantics.
- [ ] **Feature spec: headless `--json` contract** — commands, output schema, exit codes; the read-only/mutation split.
- [ ] **Workflow docs** — e.g. "add a third-party skill", "reconcile a locally-edited skill", "vendor a skill into a project".
- [ ] **ADR: per-project vendoring model** — committed `.agents/skills/` under a project repo (may fold into a feature spec).

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
