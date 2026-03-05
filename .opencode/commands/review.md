---
description: Code review current branch changes against project standards
agent: reviewer
subtask: true
---

Review the code changes on the current branch. Start by gathering context:

!`git diff --stat main...HEAD`
!`git log --oneline main...HEAD`

Review all changed files against the project's quality gates and coding standards:

**For Rust files**: Check error handling (no unwrap/expect), naming conventions (snake_case/CamelCase), async patterns (tokio::sync only), tracing logging, unsafe documentation, test coverage.

**For TypeScript/React files**: Check strict TypeScript (no `any`), shadcn/ui usage, data-testid attributes, Zod validation, CSS variable usage, accessibility.

**For both**: Check security (input validation, auth, no hardcoded secrets), documentation, and test coverage.

Provide a structured review with CRITICAL/WARNING/INFO severities and specific line references.
