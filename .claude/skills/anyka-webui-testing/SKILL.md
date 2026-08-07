---
name: anyka-webui-testing
description: Use when writing or debugging tests for the camera WebUI React components (Vitest, React Testing Library, data-testid, renderWithProviders, mock service functions, vi.mock, page/component/dialog/form/mutation test helpers).
version: 2.0.0
---

# Camera WebUI Testing

Write tests for `cross-compile/www` React components. The project uses **Vitest + React Testing Library + user-event + vi.mock**. **MSW is not used** for component tests — services are mocked with `vi.mock`. Use the shared helpers in `src/test/`.

## Run Tests

```bash
cd cross-compile/www
npm run test                    # all tests
npm run test -- TimePage        # specific test file
npm run test -- --watch
npm run test -- --coverage
```

Quality gates: `npm run lint && npm run type-check && npm run test`.

## Selector Rule (MANDATORY)

Use `data-testid` only. No role/text/class selectors (see www AGENTS.md):

```typescript
// ✅
screen.getByTestId('device-panel-save-button');
screen.queryByTestId('time-loading');
// ❌ not allowed
screen.getByRole('button', { name: 'Save' });
screen.getByText('Save');
```

## Shared Helpers (`src/test/`)

Use these instead of hand-rolling wrappers:

- **`componentTestHelpers.tsx`** — `renderWithProviders(ui)` (wraps component in QueryClientProvider + AuthProvider, returns `queryClient` too), `createTestQueryClient()` (retries off), `mockToast` (from sonner), `MOCK_ENDPOINTS`, `MOCK_DATA`, `waitForPageLoad(testId)`, `openDialog`/`closeDialog`/`submitDialog`, `fillFormField`, `toggleSwitch`, `selectOption`, `verifyPasswordVisibilityToggle`.
- **`formTestHelpers.ts`** — `testFormValidation`, `fillFormFields`, `fillAndSubmitForm`, `submitFormAndWait`, `waitForFormValue(s)`, `testFormFieldValidation`.
- **`dialogTestHelpers.ts`** — `testDialogCancel`, `testDialogCancelByButton`, `testDialogLoadingState`, `assertDialogClosed`.
- **`mutationTestHelpers.ts`** — `testMutationWithSuccessToast`, `testMutationWithErrorToast`, `testMutationWithLoadingState`, `testMutationWithError`.
- **`serviceTestHelpers.ts`** — `serviceTestPatterns` for service-layer tests.
- **`schemaTestHelpers.ts`** — validation/schema tests.

## Standard Page Test Pattern

Mock the service module with `vi.mock`, then use `renderWithProviders`:

```typescript
// src/pages/settings/TimePage.test.tsx
import { screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getDateTime, setNTP } from '@/services/timeService';
import { mockToast, renderWithProviders, waitForPageLoad } from '@/test/componentTestHelpers';

import TimePage from './TimePage';

vi.mock('@/services/timeService', () => ({
  getDateTime: vi.fn(),
  setDateTime: vi.fn(),
  setNTP: vi.fn(),
}));

describe('TimePage', () => {
  const renderTimePage = async () => {
    const result = renderWithProviders(<TimePage />);
    await waitForPageLoad('time-title');
    return result;
  };

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getDateTime).mockResolvedValue(mockTimeConfig);
    vi.mocked(setNTP).mockResolvedValue(undefined);
  });

  it('should render page with loading state', async () => {
    vi.mocked(getDateTime).mockImplementation(() => new Promise(() => {}));
    renderWithProviders(<TimePage />);
    expect(screen.getByTestId('time-loading')).toBeInTheDocument();
  });

  it('should render form with fetched config', async () => {
    await renderTimePage();
    expect(screen.getByTestId('time-synchronization-title')).toBeInTheDocument();
  });
});
```

## Mocking Services

Two layers, both `vi.mock`-based:

1. **Service modules** (e.g. `@/services/timeService`, `ptzService`) — `vi.mock('@/services/...', () => ({ fn: vi.fn() }))`, then `vi.mocked(fn).mockResolvedValue(...)`. The module exports are ES function declarations, so `vi.mocked()` typing works.
2. **`@/services/api`** — `vi.mock('@/services/api', () => ({ apiClient: { post: vi.fn() } }))`, then `vi.mocked(apiClient.post).mockResolvedValue({ data: '<soap .../>', status: 200 })`. Used by `soap/client.test.ts` and service tests.

Mock data lives in `MOCK_DATA` / `MOCK_ENDPOINTS` in `componentTestHelpers.tsx` where applicable.

## Mutation Testing

For save/submit flows use `mutationTestHelpers`:

```typescript
await testMutationWithSuccessToast(() => userEvent.click(screen.getByTestId('save-button')), {
  successMessage: 'Settings saved',
});
```

or assert loading state + error toast with `testMutationWithLoadingState` / `testMutationWithErrorToast`.

## Form Validation Testing

```typescript
await testFormValidation({
  // e.g. required field: submit empty, expect error testid text
});
```

See `formTestHelpers.ts` signatures for exact options (validation testids, expected messages, fill functions).

## Setup & Globals

`src/test/setup.ts` (wired in `vite.config.ts`):
- `@testing-library/jest-dom/vitest` matchers, auto `cleanup()` after each test.
- Global mocks: `matchMedia`, `ResizeObserver`, `Element.setPointerCapture/releasePointerCapture/scrollIntoView`, and `sonner` (`toast` → `mockToast`).
- Filters `act(...)` console.error noise.

`IS_REACT_ACT_ENVIRONMENT` is set globally. Use `userEvent.setup()` per test for interactions.

## Reference

See `.serena/memories/www-development-standards.md` (Testing Standards section), `www/src/test/` helpers, and `www/AGENTS.md`.
