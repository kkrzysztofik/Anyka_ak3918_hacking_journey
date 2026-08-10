# Skill & Complexity Policy for AGENTS.md — Design

Date: 2026-08-07
Status: Design (pending spec review)

## Problem

Ponytail (shortest diff, YAGNI, stdlib-first) and superpowers (process discipline:
brainstorm → plan → test → verify → review) are both active in this environment, but
AGENTS.md never states how they apply. The result is tension the user must resolve every
session:

- AGENTS.md's MANDATORY DEVELOPMENT WORKFLOW says "NO SHORTCUTS, NO SKIPPING TESTS, NO
  SKIPPING DOCUMENTATION."
- Ponytail says "shortest working diff, YAGNI, skip ceremony."

The user wants to **stop thinking about when to use either**. Usage should be automatic.

## Decision 1: Division of labor

| Skill family | Governs | Scope |
|---|---|---|
| **superpowers** | Process — which steps run and in what order | brainstorm, plan, test, verify, review |
| **ponytail** | Code + plans — how much gets written | shortest diff, YAGNI, no bloat |

**Conflict rule (the teeth):** Process is never skipped. Tests, lint, docs, and review
stay mandatory. Ponytail only cuts bloat *within* a step (code, plan, spec); it is not a
license to skip a step.

## Decision 2: Ponytail also reviews plans and own code

Two ponytail-review hooks beyond the default code-writing mode:

1. **Plan review** — when a plan/spec is written, `ponytail-review` it before executing.
   Keeps designs lean; prevents a bloated plan from becoming a bloated implementation.
2. **Self-review pre-pass (step 8)** — before `requesting-code-review`, the author runs
   `ponytail-review` on their own diff and cuts bloat first. Correctness reviewers then
   see lean code and stay focused on bugs, not cleanup. No change to the
   `reviewer-consensus` team (memory/architecture/security/testing).

## Decision 3: Auto-trigger table (core 7 + secondary 4)

| Skill                                      | Auto-triggers when                                  |
| ------------------------------------------ | --------------------------------------------------- |
| `superpowers:brainstorming`                  | A new feature, component, or behavior change begins |
| `superpowers:writing-plans`                  | A brainstormed design is approved                   |
| `ponytail-review`                            | A plan/spec is written, or at step 8 on your own diff |
| `superpowers:executing-plans`                | Work follows a written plan                         |
| `superpowers:test-driven-development`        | Implementation code is about to be written          |
| `superpowers:systematic-debugging`           | Investigating a bug or test failure                 |
| `superpowers:verification-before-completion` | About to claim work is done                         |
| `superpowers:requesting-code-review`         | Work is complete, pre-merge / pre-push              |
| `superpowers:receiving-code-review`          | Review feedback arrives                             |
| `superpowers:using-git-worktrees`            | Starting isolated feature work                      |
| `superpowers:dispatching-parallel-agents`    | 2+ independent tasks are available                  |
| `superpowers:subagent-driven-development`    | Executing a plan with independent subtasks          |

## Decision 4: Placement and scope

- **AGENTS.md**: one compact section "⚡ Skill & Complexity Policy", placed immediately
  after MANDATORY DEVELOPMENT WORKFLOW. Contains the defaults line, the trigger table,
  and the conflict rule (~20 lines). Workflow step 8 is updated to name `ponytail-review`.
- **CLAUDE.md**: replace its four superpowers skill lines (37–40) with a pointer to the
  AGENTS.md policy. The Claude-specific project skills (`sc:implement`,
  `onvif-service-impl`, etc.) stay — they are routing, not policy.
- **`.opencode/agents/`**: no changes — none reference skills today.

## Resulting review flow

```
implement (ponytail) → self-review step 8 (ponytail-review own diff)
→ requesting-code-review (correctness: memory/arch/security/testing)
```

## Out of scope

- Changes to `reviewer-consensus` or its specialist agents.
- Adding the policy to `.opencode/agents/`.
- Enforcing a specific ponytail intensity level beyond "full".

## Implementation steps

1. AGENTS.md: insert "⚡ Skill & Complexity Policy" section after MANDATORY DEVELOPMENT
   WORKFLOW; update step 8 to include `ponytail-review` of the diff.
2. CLAUDE.md: replace superpowers lines 37–40 with the policy pointer.
3. Verify: both files read coherently; no dangling references to removed lines.
