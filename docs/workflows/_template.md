---
title: Workflow name (verb phrase, e.g. "Reconcile a locally-edited skill")
status: draft # draft | active | deprecated
created: 2026-01-01
updated: 2026-01-01
tags: []
actors: [user] # who/what participates: user, tui, skilloom, git, agent-dir
---

# Workflow name

## Goal

What the user is trying to accomplish, in one sentence.

## Preconditions

What must be true before this workflow starts (e.g. "git is installed", "the loom-skills repo is configured", "skilloom has been initialised on this machine").

## Steps

1. Each step from the user's point of view, noting what skilloom does underneath.
2. Include the actual operations orchestrated (e.g. `git fetch`, write to `~/.agents/skills/<name>/`, symlink into `.claude/skills/`).
3. Note decision points and branches ("if the skill changed on both sides, present the diff and ask which side to keep").

## Outcome

The end state when the workflow succeeds — what changed on disk / in state, what the user sees.

## Failure modes

| What can go wrong | How the user finds out | Recovery |
|-------------------|------------------------|----------|
| e.g. upstream source gone | error/warn row in the reconcile view | keep local / detach |

## Related

- Features that implement this workflow, relevant ADRs.
