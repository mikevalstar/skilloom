---
title: The loom-skills repo layout
status: active
created: 2026-07-11
updated: 2026-07-11
tags: [loom-skills, repo, layout, personal, vendor, skill-md, structure]
audience: both
---

# The loom-skills repo layout

## Purpose

How the **loom-skills** repo — skilloom's source-of-truth hub — is laid out on
disk: which folders exist, what a skill folder looks like, and what skilloom
reads today vs. what it will write later. Read this before adding a skill by hand
or teaching skilloom to write into the repo.

## Background

skilloom is the engine; **loom-skills is the data**, the same way chezmoi is
separate from your dotfiles repo (see [AGENTS.md](../../AGENTS.md) and the
[functional overview](../features/overview.md)). The repo is the **hub**:
third-party skills are copied into `vendor/`, your own skills live in `personal/`,
and skilloom copies skills between the repo and its destinations (global agent
dirs, tracked projects). It starts life as just a `README.md`; skilloom fills in
the structure as you add skills.

## The guide

### Top-level layout

```
loom-skills/
  README.md                     # what the repo is (starts as only this)
  personal/                     # your own authored skills
    <skill-name>/
      SKILL.md                  # required — frontmatter + body
      …                         # any supporting files the skill ships
  vendor/                       # third-party skills, copied from upstream
    <skill-name>/
      SKILL.md
      .skilloom.toml            # provenance (planned — see below)
      …
```

- **`personal/`** — skills you write and own. skilloom captures these *up* from
  your global agent dirs (`import`) and installs them *down* to global/projects.
- **`vendor/`** — skills pulled from an upstream git repo (skills.sh-style). Each
  is a verbatim copy of the upstream skill folder plus provenance metadata.
- Both folders are **flat**: one directory per skill, named by the skill's folder
  name. skilloom keys sync status off that folder name today (a name match — see
  [overview.md](../features/overview.md)).

### A skill folder

Every skill — `personal/` or `vendor/` — is a directory containing a **`SKILL.md`**
with YAML frontmatter, exactly the shape agents (Claude Code, Codex, Cursor)
already expect on disk:

```markdown
---
name: sample-skill
description: A one-line summary shown in lists and the skilloom detail pane.
---

# Sample skill

Body / instructions for the agent.
```

skilloom parses the leading `---` frontmatter block and reads **`name`** and
**`description`** (see `src/skills.rs`). The `description` is what shows as the
grayed subtitle on each card in the Global and Catalog tabs. A skill with no
`SKILL.md`, or no `description`, still lists — it just shows `—` as its subtitle.
A skill folder may ship anything else alongside `SKILL.md` (scripts, templates,
reference docs); skilloom copies the whole folder when it syncs.

There is a **sample skill checked in** at `personal/sample-skill/` so the Catalog
tab has a real entry and the sync paths have something to move while the engine
is built. Delete it once you have real skills.

### What skilloom reads vs. writes

| | Today (read-only) | Planned |
|---|---|---|
| `personal/`, `vendor/` folders | scanned for skill dirs + `SKILL.md` descriptions | written by `import` / `add-remote` |
| `SKILL.md` frontmatter | `name`, `description` parsed | body shown in the detail pane |
| `vendor/<name>/.skilloom.toml` | — (not written yet) | source URL, ref/commit, synced-at |

### `.skilloom.toml` (planned, vendor only)

Vendored skills will carry a per-skill `.skilloom.toml` recording where the copy
came from and how current it is, so a vendored skill remembers its upstream:

```toml
# vendor/<name>/.skilloom.toml  — shape not final
source = "github.com/anthropics/skills"
ref    = "a1b2c3d"
synced_at = "2026-07-11"
```

This is **not implemented**; it's specced here and in the
[overview](../features/overview.md) so the layout is stable before the
`add-remote` write path lands. The formal repo-layout + manifest spec is a TODO.

## Gotchas

- **Folder name is the identity.** Sync status is a folder-**name** match against
  the repo right now (a placeholder until content/commit comparison exists). Two
  different skills must not share a folder name across `personal/`/`vendor/`.
- **Both groups always show.** skilloom's Catalog always renders `personal` and
  `vendor` sections even when a folder is absent on disk (it shows `(none)`), so
  the layout is stable — don't rely on a missing folder to hide a section.
- **Don't put skilloom's own files here.** skilloom's config/state never live in
  loom-skills (or in `~/.claude`/`~/.agents`) — config is `~/.config/skilloom/`,
  state is the XDG state dir. loom-skills holds *only* skills (+ its README).
- **Hidden entries are skipped.** Folders starting with `.` are ignored by the
  scan, so `.git/` and future `.skilloom.toml` sit safely beside skill folders.

## References

- [Functional overview](../features/overview.md) — the repo-as-hub model
- [TUI design](../features/tui-dashboard.md) — how `personal/`/`vendor/` show in Catalog
- [ADR-0003](../adrs/0003-skilloom-engine-design-and-scope.md) — engine scope
- `src/skills.rs` — the scanner (`scan_repo`, `list_skill_entries`, frontmatter parsing)
