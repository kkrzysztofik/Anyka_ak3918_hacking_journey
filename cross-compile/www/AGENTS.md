# Codex Instructions (www)

This directory contains the camera WebUI (React + TypeScript) used to configure and monitor the device.

## Mandatory docs to load (before changes)

When working in this subtree, load and follow:

- `.serena/memories/www-development-standards.md` (TS/React standards + testing)
- `.serena/memories/www-design-system.md` (design system: colors/spacing/components)
- `.serena/memories/quality-gates.md` (review checklist and required gates)
- `.serena/memories/security-guidelines.md` (auth/input validation guidance)

## Quality gates

```bash
cd cross-compile/www
npm run lint
npm run type-check   # TS 7 (`tsc`) then TS 6 (`tsc6`) side-by-side
npm run test
```

## Non-negotiable rules (summary)

- Match the design assets (see `.serena/memories/www-design-system.md`).
- Prefer shadcn/ui primitives from `src/components/ui/` (don’t invent new base components).
- Strict TypeScript: avoid `any`; use `unknown` + type guards where needed.
- Testing: use `data-testid` selectors (no role/text/class selectors).
