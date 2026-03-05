---
description: Run WebUI lint, type-check, and test suite
agent: devops
---

Run the full WebUI quality checks:

```bash
cd cross-compile/www
npm run lint
npm run type-check
npm run test
```

If any step fails, analyze the failure and suggest fixes. Report:
- Lint warnings/errors with file locations
- Type errors with specific type mismatches
- Test failures with component name and assertion details
