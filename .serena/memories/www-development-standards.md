# WWW Development Standards - Camera WebUI

## Code Standards

### Naming Conventions

| Element | Convention | Example |
|---------|------------|---------|
| Components | PascalCase | `LiveViewPage`, `UserDialog` |
| Hooks | use prefix + camelCase | `useDeviceInfo`, `useAuth` |
| Utilities | camelCase | `formatDate`, `parseXml` |
| Constants | SCREAMING_SNAKE_CASE | `API_TIMEOUT`, `MAX_RETRIES` |
| Services | camelCase + Service suffix | `deviceService`, `authService` |
| Types/Interfaces | PascalCase | `DeviceInfo`, `NetworkSettings` |

### TypeScript Requirements

- **Strict Mode**: Enabled (`strict: true` in tsconfig.json)
- **No `any` Types**: Use proper typing or `unknown` with type guards
- **No Non-null Assertions**: Avoid `!` operator, use optional chaining
- **Zod Validation**: All forms and API responses must use Zod schemas

### UI Library

- **Base Components**: Use `shadcn/ui` components located in `@/components/ui`.
- **Customization**: Do not build custom UI primitives (buttons, inputs) from scratch. Extend the provided shadcn components.

### Component Structure

```typescript
// 1. Imports (external, internal, types)
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import type { DeviceInfo } from '@/types';

// 2. Type definitions
interface Props {
  deviceId: string;
  onSave: (data: DeviceInfo) => void;
}

// 3. Component definition
export function DevicePanel({ deviceId, onSave }: Props) {
  // 4. Hooks first
  const { data, isLoading } = useDeviceInfo(deviceId);
  const [editing, setEditing] = useState(false);

  // 5. Event handlers
  const handleSave = () => { /* ... */ };

  // 6. Render
  return (/* ... */);
}
```

### Error Handling

```typescript
// Use React Query for API calls
const { data, error, isLoading } = useQuery({
  queryKey: ['device', deviceId],
  queryFn: () => deviceService.getInfo(deviceId),
  retry: 3,
});

// Show error states
if (error) {
  return <ErrorBoundary error={error} />;
}
```

## Testing Standards

### Test Structure

 ```typescript
 import { describe, it, expect, vi } from 'vitest';
 import { render, screen } from '@testing-library/react';
 import userEvent from '@testing-library/user-event';

 describe('ComponentName', () => {
   it('renders correctly with default props', () => {
     render(<ComponentName />);
     // ALWAYS use data-testid for selectors
     expect(screen.getByTestId('component-name-button')).toBeInTheDocument();
   });

   it('calls onSave when form is submitted', async () => {
     const onSave = vi.fn();
     render(<ComponentName onSave={onSave} />);
     // Do NOT use getByRole, getByText etc.
     await userEvent.click(screen.getByTestId('component-name-save-button'));
     expect(onSave).toHaveBeenCalled();
   });
 });
 ```

### MSW for API Mocking

```typescript
import { http, HttpResponse } from 'msw';
import { setupServer } from 'msw/node';

const server = setupServer(
  http.post('/onvif/device_service', () => {
    return HttpResponse.xml(/* SOAP response */);
  })
);

beforeAll(() => server.listen());
afterEach(() => server.resetHandlers());
afterAll(() => server.close());
```

## Essential Commands

```bash
# Development
npm run dev                              # Start dev server
VITE_API_TARGET=http://IP:PORT npm run dev  # Connect to specific camera

# Build & Deploy
npm run build                            # Production build

# Quality Assurance
npm run lint                             # ESLint check
npm run test                             # Run Vitest tests
npm run test -- --coverage               # Coverage report

# Type Checking
npm run type-check                       # TypeScript validation
```

## Performance Goals (From Spec 003)

- **Initial Load**: < 3s on local network
- **Page Transitions**: < 500ms
- **Asset Delivery**: Gzip/Brotli compression REQUIRED
- **Optimization**: Code splitting for all routes

## Security Requirements & Constitution

Adhere to the "Security First" principle from the project constitution:

| Requirement | Implementation |
|-------------|----------------|
| **Authentication** | HTTP Basic Auth (per Spec 003), Role-based access control |
| **XSS Prevention** | DOMPurify for HTML, Zod for all inputs |
| **Input Validation** | Zod schemas on all forms (Client-side) + Backend validation |
| **State Storage** | `sessionStorage` for auth state (cleared on window close) |
| **XML Security** | `fast-xml-parser` with safe defaults (ignore attributes if unsafe) |
| **Network** | Graceful handling of backend failures (no broken UI) |

## Bundle Optimization

- **Code Splitting**: Automatic chunks for vendors, services, components
- **Compression**: Gzip + Brotli pre-compression
- **Tree Shaking**: Enabled by default
- **Console Removal**: Production builds strip console/debugger statements
