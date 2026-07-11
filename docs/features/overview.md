---
title: Functional overview — what skilloom does and how skills flow
status: draft
created: 2026-07-11
updated: 2026-07-11
tags: [overview, sync, vendor, personal, projects, config, model]
phase: 1
---

# Functional overview

> **Draft / high-level.** This captures *what* we plan to build in medium detail — the mental model, the surfaces, and the sync flows — not the field-level mechanics. Each capability below becomes its own feature spec during planning. It **refines the storage/sync model in [ADR-0003](../adrs/0003-skilloom-engine-design-and-scope.md)** (see [Relationship to ADR-0003](#relationship-to-adr-0003)).

## Summary

skilloom manages a **curated set of AI-agent skills** and keeps them flowing between three places: **remote skill repos** on the internet, your **global** agent dirs (`~/.agents`, `~/.claude`), and individual **project** repos — all through one hub, your **loom-skills** git repo. You decide which skills go where; skilloom does the copying and tracks the state.

## Mental model: the repo is the hub

The **loom-skills repo is the single source of truth.** Everything flows in and out of it:

```
        remote skill repos (skills.sh-style git repos)
                        │  add / update   (copy in → vendor/)
                        ▼
        ┌───────────────────────────────────────┐
        │           loom-skills repo            │
        │                                       │
        │   vendor/<name>/    third-party,      │
        │                     copied in + meta  │
        │   personal/<name>/  your own skills   │
        └───────────────────────────────────────┘
             ▲                              │
   import    │  (global → repo,             │  install  (repo → …, curated,
   personal  │   selective)                 │            selective, per skill)
             │                              ▼
      global agent dirs                 project repos
      ~/.agents, ~/.claude              <proj>/.agents, <proj>/.claude
```

Two important properties:

- **Copy-based, not symlink-based.** skilloom copies skill content between the repo and its destinations, so the repo holds real, committed, versioned files. (How skills land in the *global* dirs specifically — one canonical `~/.agents/skills` + symlinks into `~/.claude/skills`, à la skills.sh, vs. a copy into each — is an open mechanism question, below.)
- **Curated and selective.** A skill is only synced to a destination if you say so. A project-specific skill can stay in that project and never flow up to the repo; a repo skill can be installed into some projects and not others.

## The loom-skills repo layout

skilloom manages this structure (the repo starts as just a README):

```
loom-skills/
  vendor/
    <name>/
      SKILL.md, …                 # copied verbatim from the remote
      .skilloom.toml              # source url, ref/commit, synced-at, notes
  personal/
    <name>/
      SKILL.md, …                 # your own authored skills
```

`.skilloom.toml` per vendored skill records "about the repo, last synced, any other useful details" so a vendored skill remembers where it came from and how current it is.

## Config and state

Split along the standard XDG lines:

- **Config** — `~/.config/skilloom/config.toml` (your intent, portable): the loom-skills repo location, the list of tracked project folders, the agent targets, and which skills you want synced where.
- **Ledger** — `~/.config/skilloom/sync.toml` (**built**): the which-goes-where record — one `[[synced]]` entry per skill × destination (`skill`/`origin`/`destination`). Kept in the config folder (beside `config.toml`, not in it) because it's closer to intent/curation than to derived state.
- **State** — `~/.local/state/skilloom/` (machine-local, derived): last-sync timestamps, resolved commits, caches. Never in `~/.config`. *(Not built yet — the richer per-sync metadata that feeds change-detection lands here; the intent-level `sync.toml` above stays in config.)*

*(Open question: whether the "which skill goes where" curation should also live committed in the repo so it's portable across machines, rather than only in machine-local config.)*

## Capabilities

Mapping directly to the functionality you described:

1. **Add remote skills (skills.sh-style).** Point skilloom at a public git repo, pick the skill(s), and it copies them into `loom-skills/vendor/<name>/` with a `.skilloom.toml` recording the source URL, ref/commit, and sync time.
2. **Track & import global skills.** skilloom discovers skills already in `~/.agents` and `~/.claude` and can sync the ones you choose *up* into `loom-skills/personal/<name>/`.
3. **Track project folders.** Add project paths to your config. skilloom then tracks each project's skills and can install curated skills from the repo into `<project>/.agents` / `<project>/.claude`.
4. **Sync in every needed direction**, tracking which skill is synced to which destination (the ledger):
   - remote → repo (fetch/update a vendored skill)
   - repo → global (install/update a skill into your global agent dirs)
   - global → repo (capture a personal skill up)
   - repo → project (install/update a skill into a tracked project)
   - project → repo (optional, capture a project skill up — only if you opt in)
5. **Curation is the point.** The tool exists to maintain a deliberate set of skills across global and projects. Syncing is **opt-in per skill per destination** — skilloom never force-pushes a project-specific skill up to the repo, nor a repo skill down into a project you didn't choose.
6. **Now: syncing only.** Tagging, searching, filtering, and diff-while-syncing are explicitly **later** — see the roadmap.

## Sync directions (reference)

| From → To | What it does | Trigger | Status |
|-----------|-------------|---------|--------|
| remote → repo `vendor/` | copy/update a third-party skill + refresh its `.skilloom.toml` | add / fetch | planned |
| repo → global | copy to `~/.agents/skills` + symlink into other agent dirs; record in `sync.toml` | `s` on Catalog | **built** |
| global → repo `personal/` | capture a personal global skill up | import | planned |
| repo → project | install/update a curated skill into a tracked project | sync | planned (stub) |
| project → repo | capture a project skill up (opt-in only) | sync | planned |

Global also supports **remove** (`x`) — un-install a skill from an agent dir, symlink- and catalog-aware.

The **ledger** (in state/config) answers "which skills go where" and drives the status shown in the TUI: for each skill × destination, is it in sync, does one side have newer content, or is it not synced at all.

## Roadmap

| Phase | Scope |
|-------|-------|
| 1 | First-run config (point at loom-skills), the repo-as-hub layout, and the dashboard shell. |
| 2 | Add & import: remote skills → `vendor/`, global skills → `personal/`, track project folders. |
| 3 | The sync engine + ledger: all directions above, curated per skill × destination, with status. |
| Later | Tagging, searching, filtering, and **diff-while-syncing**. |
| — | myplace integration (surface skilloom status as a managed tool — a myplace-side follow-up). |

## Relationship to ADR-0003

[ADR-0003](../adrs/0003-skilloom-engine-design-and-scope.md) fixed the *scope* — two source kinds, multi-agent, multi-scope (global + project), its own config/state, a TUI-mutates / `--json`-reports split — and all of that still holds. This overview **refines the mechanism**:

- **Hub is the loom-skills repo**, not a `~/.agents/skills` canonical store.
- **Copy-based sync** between repo and destinations, rather than symlinking a canonical store into each agent dir. (Symlinks may still be how the *global* side fans `~/.agents` → `~/.claude`; open question.)
- **2-way diff/reconcile is re-phased to "later"** as an advanced sync option, not the core. The core is straightforward, curated copying with a which-goes-where ledger.

**Follow-up:** once this model is locked, write a superseding ADR that records the mechanism change (repo-as-hub, copy-based, deferred diff) so ADR-0003's now-outdated storage details don't mislead. Not done yet — the model is still high-level.

## What's built so far

The read side of this model exists: the TUI's **Global** tab scans the agent dirs and browses installed skills (grouped by location, symlink-aware, with `SKILL.md` descriptions), and **Catalog** browses the repo's `personal/`+`vendor/` in the same master-detail widget — each repo skill showing whether it's currently installed in a global dir (the mirror of Global's "in repo" check). A sample skill is checked into loom-skills so the Catalog has a real entry; the repo's on-disk layout is documented in [loom-skills-repo-layout.md](../guides/loom-skills-repo-layout.md).

The **first write path is built**: **repo → global**. From the Catalog you sync a skill (a modal picks Global — Project is a "coming soon" stub) and skilloom copies the real content into the canonical store `~/.agents/skills/<name>/` and **symlinks** it into the other detected agent dirs, recording the sync in a ledger at `~/.config/skilloom/sync.toml`. The Global tab can **remove** an installed skill (with a symlink-aware, catalog-aware confirmation). Walkthroughs: [sync-a-skill-to-global.md](../workflows/sync-a-skill-to-global.md), [remove-a-global-skill.md](../workflows/remove-a-global-skill.md). Still to come: add-remote (remote → `vendor/`), import (global → `personal/`), and the project directions. UI detail: [tui-dashboard.md](tui-dashboard.md).

## Open questions

- **Global mechanism:** one canonical `~/.agents/skills` + symlinks into `~/.claude/skills` (skills.sh-style), or a plain copy into each agent dir? *Observed in practice:* the global dirs are already heavily symlinked — skills.sh symlinks its `~/.agents/skills` store into `~/.claude/skills`, and the owner's `okq-*` skills symlink out to `~/projects/okq`. So skilloom must at least *read through* symlinks (done) and decide how it *writes* around them.
- **Symlinked skills on import:** when a global skill is a symlink to elsewhere (a store, or a project checkout), does importing it into the repo copy the real content, skip it (it's already sourced there), or record a reference? Decide when import is built.
- **What "synced" means:** the Global detail shows a folder-**name** match against the repo today (a placeholder). Real status needs content/commit comparison — feeds the ledger and the eventual diff. See *change detection*.
- **Curation portability:** does the which-goes-where mapping live only in `~/.config` (machine-local) or also committed in the repo (portable across machines)?
- **Change detection:** with no pinned base, how is "one side is newer" computed for status — content hash, git history of the repo, mtime? (Feeds the phase-3 ledger and the eventual diff view.)
- **Vendor granularity:** a remote repo may contain many skills — do we vendor whole repos or individual skill folders? (Leaning: individual folders, each with its own `.skilloom.toml`.)

## Related

- [ADR-0003](../adrs/0003-skilloom-engine-design-and-scope.md) — the scope decision this refines
- [TUI screens and interaction design](tui-dashboard.md) — how this looks on screen
- [myplace ADR-0023](https://github.com/mikevalstar/myplace/blob/main/docs/adrs/0023-managing-ai-skills.md) — real facts about skills.sh, `~/.agents/skills`, the lockfiles
