---
title: ADR-0003 — Skilloom engine design and scope
status: accepted
created: 2026-07-11
updated: 2026-07-11
tags: [skills, ai, claude-code, agents, scope, reconcile, vendoring, loom-skills]
supersedes: null
superseded-by: null
---

# ADR-0003: Skilloom engine design and scope

## Context

skilloom exists because managing AI-agent skills turned out to be a whole application, not a subcommand. [myplace ADR-0024](https://github.com/mikevalstar/myplace/blob/main/docs/adrs/0024-skills-management-as-separate-project.md) spun this domain out of myplace and **handed the design intent here to be specified properly**. This ADR re-homes that intent as skilloom's own foundational scope decision. The individual mechanisms below (state format, reconcile UX, source manifest, symlink layout) each get their own feature spec during planning — this ADR fixes *what skilloom is and the shape of its model*, not the field-level details.

What an AI-agent "skill" is: a folder of Markdown (a `SKILL.md` plus optional supporting files/scripts) that Claude Code, Codex, Cursor, and other agents discover from the filesystem and invoke. There is no registry — discovery is by directory. Two kinds of skills coexist, and both must be first-class:

1. **Personal skills the owner authors** — written and edited locally, versioned in a dedicated git repo (**loom-skills**, private). This is the chezmoi-of-skills relationship: skilloom is the engine, loom-skills is the source-of-truth repo it applies from and captures edits back to.
2. **Third-party skills** — pulled from public git repos on the internet and kept roughly current, linked by source URL (chezmoi-like "track this upstream").

The requirements that make this application-sized (verified in myplace ADR-0023/0024 against the real tools, not inferred):

- **Multi-scope** — skills applied globally *and* vendored into individual project repos (real committed files under `<project>/.agents/skills/`).
- **Multi-agent** — one canonical `~/.agents/skills/<name>/` symlinked into each agent's vendor dir (`~/.claude/skills/`, `~/.cursor/skills/`, …). This mirrors what the skills.sh CLI does with `~/.agents/skills/`.
- **Bidirectional, no-pinned-base reconcile ("track latest")** — a skill can be edited locally *and* change upstream. skilloom must classify each skill as one of: **in-sync**, **locally-changed**, **upstream-changed**, **changed-on-both**, or **source-gone**, show a diff, and let the user pick a side. No pinned base commit, no auto-merge — classify and choose.
- **Its own machine-local state** — a record of what's installed where (name → source, scope, resolved version/commit, target agent dirs), separate from `~/.config` (which is chezmoi's tree), following myplace's state-dir discipline.
- **A TUI-mutates / read-only-`--json`-status split** — the interactive TUI is where changes happen (apply, reconcile, pick-a-side); `--json` commands are read-only status/inventory for scripts and for myplace to surface. Mutation off a TTY is explicit and flag-driven, never a silent side effect of a status read.

Why not just keep orchestrating the skills.sh CLI (the myplace ADR-0023 stopgap)? Because its global store can't be restored on a new machine ([vercel-labs/skills#683](https://github.com/vercel-labs/skills/issues/683)), it has no per-project model, and no bidirectional reconcile. skilloom owns these directly.

## Options considered

### Option A — thin wrapper over the skills.sh CLI

Shell out to `skills add/update/check`. Rejected: inherits exactly the gaps that motivated spinning skilloom out — no global restore, no per-project vendoring, no 2-way reconcile. It's the stopgap skilloom replaces, not the foundation it builds on.

### Option B — native engine over git + filesystem, with its own state (chosen)

skilloom fetches skill sources itself (git), owns a canonical `~/.agents/skills/` store, manages the per-agent symlinks and per-project vendored copies, keeps its own machine-local state, and computes the reconcile classification. Personal skills round-trip through the loom-skills repo; third-party skills track their upstream source. More to build, but it's the only option that satisfies the requirements above, and it's the mandate myplace ADR-0024 handed over.

### Option C — Claude Code plugins/marketplaces only

Lean on native plugin auto-update. Rejected as the *foundation*: it bundles far more than skills, is Claude-specific (not multi-agent), and gives no committed-source → restore story skilloom can drive headlessly. May be a future *source kind*, not the engine.

## Decision

**skilloom is a native skill-management engine over git and the filesystem, with its own machine-local state and a TUI-first / `--json`-read-only surface.** Concretely, the settled shape:

- **Two source kinds, both first-class:** personal skills versioned in the **loom-skills** git repo (edited locally, round-tripped through the repo), and third-party skills tracked from upstream git URLs.
- **Canonical store + multi-agent symlinks:** one real copy per skill under `~/.agents/skills/<name>/`, symlinked into each configured agent dir (`~/.claude/skills/`, `~/.cursor/skills/`, …).
- **Two scopes:** global (as above) and per-project vendoring — real, committed files under `<project>/.agents/skills/`, symlinked into that project's agent dirs.
- **2-way "track latest" reconcile:** classify each skill (in-sync / locally-changed / upstream-changed / changed-on-both / source-gone), present a diff, and let the user pick a side. No pinned base, no auto-merge.
- **Own state directory:** machine-local record of what's installed where, under the state dir (`$XDG_STATE_HOME/skilloom`, or platform equivalent) — never `~/.config`.
- **TUI mutates, `--json` reports:** interactive reconcile/apply in the TUI; read-only status/inventory via `--json` with meaningful exit codes, so scripts and myplace can consume it without mutating anything.

The **loom-skills** repo is deliberately a *separate* repository from this tool (like a chezmoi dotfiles repo is separate from chezmoi). skilloom manages its structure; the repo starts as just a README.

## Consequences

**Easier**

- A coherent model for the whole problem: one canonical store, symlinks per agent, vendored copies per project, one reconcile classifier — no dependence on a CLI whose global store can't be restored.
- Personal-skill history and third-party currency are both first-class, versioned in git rather than trapped in an unrestorable lockfile.
- The `--json`/read-only contract lets myplace surface skilloom status the same informational way it surfaces `outdated`/`sysinfo`, when that integration lands (myplace ADR-0024 follow-up).

**Harder / committed to**

- **We own the reconcile classifier.** Getting in-sync / locally-changed / upstream-changed / changed-on-both / source-gone right (and their diffs) is the core engineering, and it's on us.
- **Two repos to keep coherent** — skilloom (tool) and loom-skills (personal source). Plus the eventual skilloom↔myplace seam (how myplace installs skilloom and surfaces its status), which is a myplace-side follow-up, not this repo's now.
- **The sub-designs still need specs.** This ADR fixes the model; the state-file format, the reconcile TUI, the source manifest / config format, the symlink and vendoring mechanics, and the headless exit-code contract each become their own feature spec in the planning phase.

## Related

- [myplace ADR-0024](https://github.com/mikevalstar/myplace/blob/main/docs/adrs/0024-skills-management-as-separate-project.md) — the decision to spin skilloom out; the source of this design intent
- [myplace ADR-0023](https://github.com/mikevalstar/myplace/blob/main/docs/adrs/0023-managing-ai-skills.md) — the superseded in-myplace approach; documents the real tool facts (skills.sh store at `~/.agents/skills/`, the two lockfiles, #683)
- [ADR-0002](0002-rust-and-ratatui-for-the-tui.md) — the stack this engine is built on
- [vercel-labs/skills#683](https://github.com/vercel-labs/skills/issues/683) — the global-restore gap that helped justify a native engine
