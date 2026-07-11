# Documentation

This project is documentation-first: decisions, features, and workflows are written here before (or alongside) the code that implements them ([ADR-0001](adrs/0001-documentation-first.md)).

## Structure

| Folder | What goes here | When to write one |
|--------|----------------|-------------------|
| [adrs/](adrs/) | Architecture Decision Records — why we chose X over Y | Any time we pick a technology, library, pattern, or approach that would be expensive to reverse |
| [features/](features/) | Feature specs — what a capability does, its scope, and its acceptance criteria | Before building a new user-visible capability |
| [workflows/](workflows/) | End-to-end flows the tool supports (e.g. "reconcile a locally-edited skill") from the user's point of view | When defining or changing how a user accomplishes a goal |
| [guides/](guides/) | Developer guides — how to work on this repo, how the libraries we depend on behave, conventions, gotchas | When you learn something a future developer (or AI agent) will need |

Each folder contains a `_template.md` showing the expected format for that doc type. Copy it as the starting point for new docs.

## Frontmatter

Every doc starts with YAML frontmatter so docs can be searched and filtered programmatically:

```yaml
---
title: Short human-readable title
status: draft        # draft | accepted | active | superseded | deprecated
created: 2026-07-11  # ISO date
updated: 2026-07-11  # ISO date, bump when meaningfully edited
tags: [tui, skills]  # lowercase, kebab-case
---
```

Doc types add fields on top of this (ADRs have `supersedes`/`superseded-by`, features have `phase`, workflows have `actors`, guides have `audience`) — see each `_template.md`.

## Conventions

- **ADRs are numbered**: `0001-documentation-first.md`, `0002-...`. Numbers are never reused.
- **Other docs are kebab-case**: `reconcile-a-skill.md`.
- **Decisions are immutable**: don't rewrite an accepted ADR — write a new one that supersedes it and flip the old one's `status` to `superseded`.
- **Statuses flow forward**: `draft` → `accepted`/`active` → `superseded`/`deprecated`.
