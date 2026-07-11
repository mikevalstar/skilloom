---
title: Sync a skill to global
status: active
created: 2026-07-11
updated: 2026-07-11
tags: [sync, global, catalog, ledger, symlink, agents, write-path]
actors: [user, tui, skilloom, agent-dir]
---

# Sync a skill to global

## Goal

Copy a skill from the loom-skills repo (Catalog) into the global agent dirs so
every agent on this machine can use it, and record that it was synced.

## Preconditions

- skilloom is configured (the loom-skills repo path is set).
- The skill exists in the repo under `personal/<name>/` or `vendor/<name>/` — it
  shows in the **Catalog** tab.
- `$HOME` is set (the agent dirs and the ledger live under it).

## Steps

1. On the **Catalog** tab, select a skill and press **`s`** (or click the
   **`[ Sync → ]`** button in the detail pane). A **sync modal** opens.
2. Choose a **destination**:
   - **Global** — selected by default; this is what works today.
   - **Project** — a stub ("coming soon"); selecting it and confirming just shows
     a note and changes nothing.
3. Under **link into**, toggle which detected agent dirs get a symlink. skilloom
   lists the agent dirs that exist on disk *other than* the canonical store
   (`~/.claude/skills`, `~/.codex/skills`, `~/.cursor/skills`); all are **on by
   default**. Navigate with `↑↓`, toggle with `space`/`⏎`, or click a row.
4. Activate **`[ Sync ]`** (focused by default, so open → `⏎` syncs with the
   defaults). skilloom then, under the hood:
   - copies the real skill folder to the **canonical store**
     `~/.agents/skills/<name>/` (replacing any existing copy);
   - **symlinks** `<name>` into each toggled-on agent dir, pointing at the
     canonical copy (replacing whatever was there);
   - appends a record to the ledger `~/.config/skilloom/sync.toml`:
     ```toml
     [[synced]]
     skill = "<name>"
     origin = "personal"   # or "vendor"
     destination = "global"
     ```
   - rescans, so the footer confirms `Synced '<name>' → global.` and the skill's
     Catalog status flips to **`● installed globally`**.

The copy-once-symlink-many model mirrors how skills.sh fans a store out: the
content lives in one place, and the other agents reference it.

## Outcome

- Real content at `~/.agents/skills/<name>/`; symlinks to it in the chosen agent
  dirs.
- A `[[synced]]` entry in `~/.config/skilloom/sync.toml`.
- The Catalog detail shows `● installed globally`; the Global tab lists the new
  skill (the canonical copy plain, the symlinked ones flagged `@`).

## Failure modes

| What can go wrong | How the user finds out | Recovery |
|-------------------|------------------------|----------|
| Skill missing in the repo | footer: `Sync failed: skill not found in repo: …` | nothing was written; re-check the Catalog selection |
| `$HOME` unset | footer: `Sync failed: HOME is not set` | run in a normal shell session |
| Copy/symlink IO error (permissions, etc.) | footer: `Sync failed: …` | fix the underlying cause; the ledger is only written after a successful copy |
| Ledger couldn't be written | footer: `Synced '<name>', but ledger save failed: …` | the files are synced; fix `~/.config/skilloom/` and re-sync to re-record |

## Related

- [remove-a-global-skill.md](remove-a-global-skill.md) — the reverse
- [overview.md](../features/overview.md) — the repo-as-hub model and sync directions
- [loom-skills-repo-layout.md](../guides/loom-skills-repo-layout.md) — where the source lives
- [tui-dashboard.md](../features/tui-dashboard.md) — the modal in the UI
- `src/sync.rs` (engine), `src/ledger.rs` (the `sync.toml` ledger)
