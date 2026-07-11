---
title: ADR-0001 — Documentation-first development
status: accepted
created: 2026-07-11
updated: 2026-07-11
tags: [process, docs]
supersedes: null
superseded-by: null
---

# ADR-0001: Documentation-first development

## Context

`skilloom` is a new tool spun out of [myplace](https://github.com/mikevalstar/myplace) (its ADR-0024 hands the design intent to this project). It will be developed incrementally, largely with AI-assisted tooling, and it manages something with a lot of moving parts: skills that live in a personal git repo *and* upstream repos, applied globally *and* per-project, symlinked into several agents' directories, and reconciled in both directions. Decisions made early — the storage model, the reconcile semantics, what skilloom owns versus delegates to git — need to stay legible months later, and AI agents working in the repo need durable context beyond the code.

myplace proved this approach works: a `docs/` tree of ADRs, feature specs, workflows, and guides kept a fast-moving, AI-assisted project coherent. skilloom adopts the same discipline from day one.

## Options considered

### Option A — Code-first, document later

Fastest to start. In practice "later" rarely arrives, rationale is lost, and agents (and future-me) re-litigate settled decisions. Especially costly here, where the hard part is the *model* (reconcile classification, scope, source kinds), not the rendering.

### Option B — Documentation-first

Write ADRs, feature specs, and workflows before or alongside implementation, in a structured `docs/` tree with frontmatter for searchability. Slower per feature, but decisions stay traceable and the docs become the working spec.

## Decision

Option B. The repo carries a `docs/` tree with four sections — `adrs/`, `features/`, `workflows/`, `guides/` — each with a `_template.md`. All docs carry YAML frontmatter (`title`, `status`, `created`, `updated`, `tags`, plus type-specific fields). ADRs are numbered and immutable: changes are made by superseding, not editing.

## Consequences

- Every significant choice (stack, storage model, reconcile semantics) gets an ADR before code depends on it.
- New features start as a spec in `features/`, usually paired with a workflow doc.
- Frontmatter makes the docs greppable/filterable, and could later drive tooling (a docs index, a status dashboard).
- Small overhead per change; accepted as the cost of long-term legibility.
- Design intent inherited from myplace ADR-0024 is re-homed into skilloom's own ADRs (starting with [ADR-0003](0003-skilloom-engine-design-and-scope.md)) rather than left in another repo.
