# WWW Development Standards - Camera WebUI

## Code Formatting & Linting

### Mandatory Before Commit
```bash
cd cross-compile/www
npm run lint                       # ESLint check (zero warnings)
npm run type-check                 # TypeScript validation (no errors)
npm run test                       # All tests must pass
npm run prettier                   # Format code
```

### Pre-Commit One-Liner
```bash
npm run lint && npm run type-check && npm run test
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

**Note**: Project uses Zod ^4.3.6. The core API (`z.object`, `z.string`, `safeParse`, `z.infer`) is compatible with Zod 3 patterns.

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

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

describe('DevicePanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders device information correctly', () => {
    render(<DevicePanel deviceId="test-123" />);
    
    // ALWAYS use data-testid
    expect(screen.getByTestId('device-panel')).toBeInTheDocument();
  });

  it('calls onSave when save button is clicked', async () => {
    const onSave = vi.fn();
    render(<DevicePanel deviceId="test-123" onSave={onSave} />);
    
    await userEvent.click(screen.getByTestId('device-panel-save-button'));
    
    expect(onSave).toHaveBeenCalledTimes(1);
  });
});
```

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

### MSW for API Mocking

```typescript
import { http, HttpResponse } from 'msw';
import { setupServer } from 'msw/node';

const handlers = [
  http.post('/onvif/device_service', () => {
    return HttpResponse.xml(`
      <SOAP-ENV:Envelope>
        <SOAP-ENV:Body>
          <tds:GetDeviceInformationResponse>
            <tds:Manufacturer>Anyka</tds:Manufacturer>
          </tds:GetDeviceInformationResponse>
        </SOAP-ENV:Body>
      </SOAP-ENV:Envelope>
    `);
  }),
];

const server = setupServer(...handlers);

beforeAll(() => server.listen());
afterEach(() => server.resetHandlers());
afterAll(() => server.close());
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

- [ ] ESLint passes (`npm run lint`)
- [ ] TypeScript passes (`npm run type-check`)
- [ ] All tests pass (`npm run test`)
- [ ] Code formatted (`npm run prettier`)
- [ ] No `any` types
- [ ] No `console.log` statements
- [ ] All interactive elements have `data-testid`
- [ ] New components have tests
- [ ] Zod schemas for new forms/API responses
