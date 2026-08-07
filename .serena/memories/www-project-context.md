# WWW Project Context - Camera WebUI

## Project Description

The `cross-compile/www` directory contains the **Camera WebUI** — a React-based web administration panel for Anyka AK3918 ONVIF cameras. This is the frontend companion to the `onvif-rust` backend, providing a user-friendly interface for camera configuration and live viewing.

**Current Status**: Active development (see `docs/design/prd.md` and `docs/design/design_proposal.md`).

## Technology Stack

| Category | Technology | Version |
|----------|------------|---------|
| **Language** | TypeScript | TS 7 (native) + TS 6 (eslint peer) |
| **Framework** | React | ^19.2.8 |
| **Build Tool** | Vite | ^8.1.5 |
| **Styling** | TailwindCSS | ^4.3.3 |
| **UI Components** | shadcn/ui (Radix-based) | Latest |
| **State Management** | TanStack Query | ^5.101.4 |
| **Routing** | React Router | ^8.3.0 |
| **HTTP Client** | fetch-based `apiClient` (`src/services/api.ts`) | - |
| **Form Handling** | React Hook Form + Zod | ^7.83.0 / ^4.4.3 |
| **XML Parsing** | fast-xml-parser | ^5.10.1 |
| **Icons** | Lucide React | ^1.27.0 |
| **XSS Prevention** | DOMPurify | ^3.4.12 |
| **Toasts** | Sonner | ^2.0.7 |
| **Testing** | Vitest + Testing Library | ^4.1.10 / ^16.3.2 |

## Project Structure

```
cross-compile/www/
├── src/
│   ├── main.tsx             # Entry point
│   ├── App.tsx              # Main application component
│   ├── Layout.tsx           # Application layout with sidebar
│   ├── index.css            # Tailwind entry
│   │
│   ├── styles/
│   │   └── globals.css      # Design tokens + component utilities
│   │
│   ├── components/          # Reusable UI components
│   │   ├── ui/              # shadcn/ui base components
│   │   ├── common/          # Shared components (LoadingState, etc.)
│   │   ├── users/           # Domain-specific components
│   │   └── layout/          # Layout components
│   │
│   ├── pages/               # Route pages
│   │   ├── LiveViewPage.tsx
│   │   ├── DiagnosticsPage.tsx
│   │   ├── LoginPage.tsx
│   │   └── settings/        # Settings sub-pages (7 categories)
│   │
│   ├── services/            # ONVIF SOAP service clients
│   │   ├── soap/            # SOAP client (soapRequest, parseSOAPResponse)
│   │   ├── api.ts           # fetch-based apiClient (Basic Auth)
│   │   ├── schemas/         # Zod schemas
│   │   ├── deviceService.ts
│   │   ├── networkService.ts
│   │   ├── timeService.ts
│   │   ├── imagingService.ts
│   │   ├── userService.ts
│   │   ├── profileService.ts
│   │   ├── ptzService.ts
│   │   ├── maintenanceService.ts
│   │   └── authService.ts
│   │
│   ├── hooks/               # Custom React hooks
│   ├── lib/                 # Utility libraries (queryClient.ts)
│   ├── types/               # TypeScript type definitions
│   ├── utils/               # Utility functions
│   ├── config/              # Application configuration
│   ├── router/              # Route definitions (HashRouter)
│   └── test/                # Test setup + shared helpers
│       ├── componentTestHelpers.tsx
│       ├── formTestHelpers.ts
│       ├── dialogTestHelpers.ts
│       ├── mutationTestHelpers.ts
│       ├── serviceTestHelpers.ts
│       ├── schemaTestHelpers.ts
│       ├── setup.ts
│       └── utils.ts
│
├── vite.config.ts           # Vite configuration
├── tailwind.config.js       # TailwindCSS configuration
├── tsconfig.json            # TypeScript configuration
├── eslint.config.js         # ESLint configuration
└── package.json             # Dependencies and scripts
```

## Essential Commands

```bash
cd cross-compile/www

# Install dependencies
npm ci                             # Clean install (CI-style)
npm install                        # Install/update

# Development
npm run dev                        # Start dev server (default proxy)
VITE_API_TARGET=http://192.168.1.50:8080 npm run dev  # Custom camera

# Build
npm run build                      # Production build

# Testing
npm run test                       # Run Vitest tests
npm run test:coverage              # With coverage report

# Code Quality
npm run lint                       # ESLint check
npm run lint:fix                   # Auto-fix issues
npm run type-check                 # TypeScript validation
npm run prettier                   # Format code
```

### Pre-Commit Command
```bash
npm run lint && npm run type-check && npm run test
```

## Build Configuration

| Setting | Value |
|---------|-------|
| Output Directory | `SD_card_contents/anyka_hack/onvif/www` |
| Compression | Gzip + Brotli pre-compression |
| Code Splitting | Manual chunks (vendors, services, components) |
| Minification | Terser with console/debugger removal |
| Proxy Routes | `/onvif`, `/utilization`, `/snapshot` |

## Key Development Patterns

### Component Structure
```typescript
// 1. Imports (external → internal → types)
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import type { DeviceInfo } from '@/types';

// 2. Props interface
interface Props {
  deviceId: string;
  onSave: (data: DeviceInfo) => void;
}

// 3. Component with hooks first
export function DevicePanel({ deviceId, onSave }: Props) {
  const { data, isLoading } = useDeviceInfo(deviceId);
  const [editing, setEditing] = useState(false);
  
  // Event handlers
  const handleSave = () => { /* ... */ };
  
  // Render
  return (/* ... */);
}
```

### Testing Pattern
```typescript
// Mock service modules with vi.mock (MSW is NOT used)
vi.mock('@/services/deviceService', () => ({
  getInfo: vi.fn(),
  updateSettings: vi.fn(),
}));

vi.mocked(deviceService.getInfo).mockResolvedValue({ name: 'Anyka' });

// Always render via renderWithProviders (QueryClientProvider + AuthProvider)
const { queryClient } = renderWithProviders(<DevicePanel deviceId="test-123" />);

// ALWAYS use data-testid for selectors
screen.getByTestId('device-panel-save-button');

// NEVER use these
screen.getByRole('button');  // ❌
screen.getByText('Save');    // ❌
```

### State Management
```typescript
// Use React Query for server state
const { data, error, isLoading } = useQuery({
  queryKey: ['device', deviceId],
  queryFn: () => deviceService.getInfo(deviceId),
});

// Use Zod 4 for validation
import { z } from 'zod';

const schema = z.object({
  name: z.string().min(1).max(64),
});
```

**Note**: Project uses Zod 4.x which has the same API as Zod 3 for common patterns.

## Integration with Backend

| Aspect | Implementation |
|--------|----------------|
| Protocol | ONVIF SOAP over HTTP |
| Authentication | HTTP Basic Auth |
| Routing | HashRouter (SPA compatibility) |
| Session | sessionStorage (cleared on close) |
| XML Parsing | fast-xml-parser with safe defaults |

## Specifications & Design

| Document | Location |
|----------|----------|
| Functional Spec (PRD) | `docs/design/prd.md` |
| Design Proposal | `docs/design/design_proposal.md` |
| Design Review | `docs/design/DESIGN_REVIEW.md` |
| Figma Source | `docs/design/ONVIF.fig` |
| Theme CSS (ground truth) | `src/styles/globals.css` |
| Design System | See `www-design-system` memory |

## Performance Requirements

| Metric | Target |
|--------|--------|
| Initial Load | < 3s on local network |
| Page Transitions | < 500ms |
| Asset Delivery | Gzip/Brotli compression |
| Code Splitting | Per-route chunks |

## Related Memories

- `www-development-standards` - Coding standards and conventions
- `www-design-system` - Visual design system and components
- `review-prompt-www` - Code review guidelines
- `testing-framework` - Testing patterns (shared with Rust)
