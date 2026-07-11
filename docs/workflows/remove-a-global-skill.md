---
title: Remove a global skill
status: active
created: 2026-07-11
updated: 2026-07-11
tags: [remove, global, symlink, catalog, ledger, write-path]
actors: [user, tui, skilloom, agent-dir]
---

# Remove a global skill

## Goal

Delete an installed skill from a global agent dir — mainly to clean up while
testing sync, but generally to un-install a skill.

## Preconditions

- A skill is listed on the **Global** tab (it exists in one of the scanned agent
  dirs).

## Steps

1. On the **Global** tab, select a skill and press **`x`** (or click the
   **`[ Remove ]`** button). A **confirmation modal** opens, stating:
   - **from** — the agent dir the entry lives in;
   - if it's a **symlink**, that only the link is removed (its target is kept);
   - a **permanence** line that depends on whether the skill is in your Catalog:
     - **in Catalog** → green: "you can re-sync it afterward";
     - **not in Catalog, but a symlink** → yellow: "this only unlinks; the target
       is kept";
     - **not in Catalog, a real dir** → red: **"⚠ removing it is permanent."**
2. Confirm with **`[ Remove ]`** (or `y`); cancel with **`[ Cancel ]`**, `esc`, or
   `n`. Focus defaults to **Cancel** — the safe choice for a destructive action.
3. On confirm, skilloom removes exactly that entry (`remove_dir_all` for a real
   dir; unlink for a symlink — never following it), drops any matching
   `destination = "global"` record from `~/.config/skilloom/sync.toml`, rescans,
   and the footer shows `Removed '<name>'.`

## Outcome

- The entry is gone from that agent dir. A symlink's target (e.g. the canonical
  `~/.agents/skills/<name>/` or a project checkout) is untouched.
- Any `global` ledger record for the skill is forgotten.

## Failure modes

| What can go wrong | How the user finds out | Recovery |
|-------------------|------------------------|----------|
| Permission error deleting | footer: `Remove failed: …` | fix permissions and retry |
| Skill already gone | it isn't listed to select | rescan with `f` |

> **Note:** removing the canonical copy at `~/.agents/skills/<name>/` leaves any
> symlinks in other agent dirs dangling. Removing symlinks individually is safe;
> a fuller "remove everywhere" that also cleans the canonical store + its links
> arrives with the sync ledger's per-destination view.

## Related

- [sync-a-skill-to-global.md](sync-a-skill-to-global.md) — the reverse
- [tui-dashboard.md](../features/tui-dashboard.md) — the Global tab and the modal
- `src/sync.rs` (`remove_path`), `src/ledger.rs` (`forget`)
