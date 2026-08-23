# WWW Development Standards - Camera WebUI

## Code Formatting & Linting

### Mandatory Before Commit

```bash
cd cross-compile/www
npm run verify                     # THE gate: type-check + lint + format:check
npm run test                       # All tests must pass
```

`verify` is the single definition of the WebUI gate list. `main-ci.yml` and
`scripts/build_sd_contents.sh` both call it, so a new gate goes in `verify`,
never into one caller — that drift is what let three TS 7 errors merge green
and stop a fleet rollout at the deploy gate.

Individual parts, for iterating:

```bash
npm run type-check                 # TS 7 (`tsc`) then TS 6 (`tsc6`) side-by-side
npm run type-check:ts7             # TypeScript 7 native checker only
npm run type-check:ts6             # TypeScript 6 checker only (eslint API peer)
npm run lint                       # ESLint check (zero warnings)
npm run format:check               # Prettier, check only
npm run prettier                   # Prettier, rewrite in place
```

### Pre-Commit One-Liner

```bash
npm run verify && npm run test
```

## Naming Conventions

| Element | Convention | Example |
|---------|------------|---------|
| Components | PascalCase | `DevicePanel`, `UserDialog` |
| Hooks | use + camelCase | `useDeviceInfo`, `useAuth` |
| Utilities | camelCase | `formatDate`, `parseXml` |
| Services | camelCase + Service | `deviceService`, `authService` |
| Types/Interfaces | PascalCase | `DeviceInfo`, `NetworkSettings` |
| Constants | SCREAMING_SNAKE | `API_TIMEOUT`, `MAX_RETRIES` |
| Files (components) | PascalCase | `DevicePanel.tsx`, `UserDialog.tsx` |
| Files (utilities) | camelCase | `formatDate.ts`, `parseXml.ts` |
| Test files | .test.tsx suffix | `DevicePanel.test.tsx` |
| CSS classes | kebab-case | `device-panel`, `user-dialog` |

## TypeScript Requirements

### Strict Mode (Mandatory)
```json
// tsconfig.json
{
  "compilerOptions": {
    "strict": true,
    "noImplicitAny": true,
    "strictNullChecks": true
  }
}
```

### Type Patterns

```typescript
// ❌ WRONG - Never use 'any'
function process(data: any) { }

// ✅ CORRECT - Use proper types or 'unknown' with guards
function process(data: unknown) {
  if (isDeviceInfo(data)) {
    // data is now typed as DeviceInfo
  }
}

// ❌ WRONG - Non-null assertion
const name = user!.name;

// ✅ CORRECT - Optional chaining
const name = user?.name ?? 'Unknown';

// ✅ CORRECT - Type guards
function isDeviceInfo(data: unknown): data is DeviceInfo {
  return typeof data === 'object' && data !== null && 'model' in data;
}
```

### Zod 4 Validation (Mandatory for Forms/API)

**Note**: Project uses Zod ^4.4.3. The core API (`z.object`, `z.string`, `safeParse`, `z.infer`) is compatible with Zod 3 patterns.

```typescript
import { z } from 'zod';

// Define schema
const deviceInfoSchema = z.object({
  name: z.string().min(1).max(64),
  location: z.string().max(128).optional(),
  model: z.string(),
});

// Infer type from schema
type DeviceInfo = z.infer<typeof deviceInfoSchema>;

// Validate API response
const result = deviceInfoSchema.safeParse(apiResponse);
if (!result.success) {
  console.error('Validation failed:', result.error);
}
```

## Component Standards

### UI Library Rules

```typescript
// ✅ CORRECT - Use shadcn/ui components
import { Button } from '@/components/ui/button';
import { Dialog } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';

// ❌ WRONG - Don't create custom primitives
const MyButton = styled.button`...`;  // NO
```

### Component Structure

```typescript
// 1. External imports
import { useState, useCallback } from 'react';
import { useQuery } from '@tanstack/react-query';

// 2. Internal imports (absolute paths)
import { Button } from '@/components/ui/button';
import { deviceService } from '@/services/deviceService';

// 3. Type imports
import type { DeviceInfo } from '@/types';

// 4. Props interface (always explicit)
interface DevicePanelProps {
  deviceId: string;
  onSave: (data: DeviceInfo) => void;
}

// 5. Component definition (named export preferred)
export function DevicePanel({ deviceId, onSave }: DevicePanelProps) {
  // 6. Hooks first (in consistent order)
  const { data, isLoading, error } = useDeviceInfo(deviceId);
  const [editing, setEditing] = useState(false);
  
  // 7. Callbacks (memoized if passed to children)
  const handleSave = useCallback(() => {
    if (data) onSave(data);
  }, [data, onSave]);
  
  // 8. Early returns for loading/error states
  if (isLoading) return <LoadingState />;
  if (error) return <ErrorState error={error} />;
  
  // 9. Main render
  return (
    <div data-testid="device-panel">
      {/* ... */}
    </div>
  );
}
```

### Data Attributes (Mandatory for Testing)

```typescript
// ✅ CORRECT - All interactive elements need data-testid
<Button data-testid="device-panel-save-button">Save</Button>
<Input data-testid="device-panel-name-input" />
<Dialog data-testid="user-dialog">...</Dialog>

// Naming pattern: {component}-{element}-{type}
// Examples:
// - device-panel-save-button
// - user-dialog-cancel-button
// - network-settings-ip-input
```

## Error Handling

### React Query Pattern

```typescript
const { data, error, isLoading, isError } = useQuery({
  queryKey: ['device', deviceId],
  queryFn: () => deviceService.getInfo(deviceId),
  retry: 3,
  staleTime: 30000,
});

// Handle states explicitly
if (isLoading) return <LoadingSpinner />;
if (isError) return <ErrorMessage error={error} />;
if (!data) return <EmptyState />;

return <DeviceInfo data={data} />;
```

### Error Boundaries

```typescript
// Wrap pages/features in error boundaries
<ErrorBoundary fallback={<ErrorFallback />}>
  <DeviceSettings />
</ErrorBoundary>
```

### API Error Handling

```typescript
// Services should throw typed errors
class ApiError extends Error {
  constructor(
    message: string,
    public statusCode: number,
    public code: string
  ) {
    super(message);
  }
}

// Catch and handle appropriately
try {
  await deviceService.updateSettings(settings);
} catch (error) {
  if (error instanceof ApiError && error.statusCode === 401) {
    // Handle unauthorized
  }
  throw error; // Re-throw for React Query to handle
}
```

## State Management

### Server State (React Query)

```typescript
// Queries - for fetching data
const { data } = useQuery({
  queryKey: ['devices'],
  queryFn: deviceService.getAll,
});

// Mutations - for modifying data
const mutation = useMutation({
  mutationFn: deviceService.update,
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['devices'] });
  },
});
```

### Client State (React useState/useReducer)

```typescript
// Simple state
const [isOpen, setIsOpen] = useState(false);

// Complex state
const [state, dispatch] = useReducer(reducer, initialState);
```

### Form State (React Hook Form)

```typescript
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';

const form = useForm<DeviceInfo>({
  resolver: zodResolver(deviceInfoSchema),
  defaultValues: { name: '', location: '' },
});
```

## Testing Standards

### Test Structure

Tests use `it('should ...')` inside `describe` (NOT Rust-style `test_<component>` snake naming). Always render through the shared helper `renderWithProviders` from `src/test/componentTestHelpers.tsx` (wraps in QueryClientProvider + AuthProvider, returns the `queryClient`).

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '@/test/componentTestHelpers';

describe('DevicePanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render device information correctly', () => {
    renderWithProviders(<DevicePanel deviceId="test-123" />);

    // ALWAYS use data-testid
    expect(screen.getByTestId('device-panel')).toBeInTheDocument();
  });

  it('should call onSave when save button is clicked', async () => {
    const onSave = vi.fn();
    renderWithProviders(<DevicePanel deviceId="test-123" onSave={onSave} />);

    await userEvent.click(screen.getByTestId('device-panel-save-button'));

    expect(onSave).toHaveBeenCalledTimes(1);
  });
});
```

### Shared Test Helpers (`src/test/`)

- `componentTestHelpers.tsx` — `renderWithProviders`, `createTestQueryClient`, `mockToast`, `MOCK_ENDPOINTS`, `MOCK_DATA`, `waitForPageLoad`, `openDialog`/`closeDialog`/`submitDialog`, `fillFormField`, `toggleSwitch`, `selectOption`
- `formTestHelpers.ts`, `dialogTestHelpers.ts`, `mutationTestHelpers.ts`, `serviceTestHelpers.ts`, `schemaTestHelpers.ts`
- `setup.ts` — mocks matchMedia, ResizeObserver, element pointer-capture, sonner toast; filters `act()` noise

### Selector Rules (MANDATORY)

```typescript
// ✅ CORRECT - Always use data-testid
screen.getByTestId('device-panel-save-button');
screen.queryByTestId('error-message');
screen.findByTestId('loading-spinner');

// ❌ WRONG - Never use these in tests
screen.getByRole('button', { name: 'Save' });  // NO
screen.getByText('Save');                       // NO
screen.getByClassName('btn-primary');           // NO
```

### Service Mocking (vi.mock — MSW is NOT used)

Mock service modules with `vi.mock` at the top of the test file, then control return values with `vi.mocked(...)`. There is no `src/test/mocks/` or `src/test/fixtures/soap/` directory.

```typescript
import { deviceService } from '@/services/deviceService';
import { renderWithProviders } from '@/test/componentTestHelpers';

vi.mock('@/services/deviceService', () => ({
  getInfo: vi.fn(),
  updateSettings: vi.fn(),
}));

describe('DevicePanel', () => {
  it('should load device info', async () => {
    vi.mocked(deviceService.getInfo).mockResolvedValue({ name: 'Anyka' });

    renderWithProviders(<DevicePanel deviceId="test-123" />);

    expect(await screen.findByTestId('device-panel')).toBeInTheDocument();
    expect(deviceService.getInfo).toHaveBeenCalledWith('test-123');
  });
});
```

## Security Requirements

| Requirement | Implementation |
|-------------|----------------|
| XSS Prevention | DOMPurify for HTML content |
| Input Validation | Zod schemas on all forms |
| Auth State | sessionStorage (cleared on close) |
| XML Security | fast-xml-parser with safe defaults |
| No Secrets | Never hardcode credentials |

## Performance Guidelines

| Guideline | Implementation |
|-----------|----------------|
| Code Splitting | Lazy load routes with `React.lazy()` |
| Memoization | Use `useMemo`/`useCallback` for expensive ops |
| Image Optimization | Proper sizing, lazy loading |
| Bundle Size | Monitor with `npm run build` output |
| No Console | Remove `console.log` in production |

## Import Organization

```typescript
// 1. React/external libraries
import { useState, useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';

// 2. Internal absolute imports
import { Button } from '@/components/ui/button';
import { deviceService } from '@/services/deviceService';
import { useAuth } from '@/hooks/useAuth';

// 3. Relative imports (same module)
import { DeviceCard } from './DeviceCard';

// 4. Type imports (last)
import type { DeviceInfo } from '@/types';
```

## Pre-Commit Checklist

- [ ] Gate list passes (`npm run verify` — type-check + lint + format:check)
- [ ] All tests pass (`npm run test`)
- [ ] Code formatted (`npm run prettier`)
- [ ] No `any` types
- [ ] No `console.log` statements
- [ ] All interactive elements have `data-testid`
- [ ] New components have tests
- [ ] Zod schemas for new forms/API responses
