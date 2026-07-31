# WWW Project Context - Camera WebUI

## Project Description

The `cross-compile/www` directory contains the **Camera WebUI** — a React-based web administration panel for Anyka AK3918 ONVIF cameras. This is the frontend companion to the `onvif-rust` backend, providing a user-friendly interface for camera configuration and live viewing.

**Current Status**: Active development following Spec 003 (Frontend ONVIF Spec).

## Technology Stack

| Category | Technology | Version |
|----------|------------|---------|
| **Language** | TypeScript | ~5.8 (strict mode) |
| **Framework** | React | ^19.2.4 |
| **Build Tool** | Vite | ^7.3.1 |
| **Styling** | TailwindCSS | ^4.1.18 |
| **UI Components** | shadcn/ui (Radix-based) | Latest |
| **State Management** | TanStack Query | ^5.90.21 |
| **Routing** | React Router | ^7.13.0 |
| **HTTP Client** | Axios | ^1.13.5 |
| **Form Handling** | React Hook Form + Zod | ^7.71.1 / ^4.3.6 |
| **XML Parsing** | fast-xml-parser | ^5.3.6 |
| **Icons** | Lucide React | ^0.564.0 |
| **XSS Prevention** | DOMPurify | ^3.3.1 |
| **Charts** | Recharts | ^3.7.0 |
| **Toasts** | Sonner | ^2.0.7 |
| **Testing** | Vitest + Testing Library + MSW | ^4.0.18 / ^16.3.2 / ^2.12.10 |

## Project Structure

```
cross-compile/www/
├── src/
│   ├── main.tsx             # Entry point
│   ├── App.tsx              # Main application component
│   ├── Layout.tsx           # Application layout with sidebar
│   ├── index.css            # Global styles (TailwindCSS)
│   │
│   ├── components/          # Reusable UI components
│   │   ├── ui/              # shadcn/ui base components
│   │   ├── common/          # Shared components (LoadingState, etc.)
│   │   └── users/           # Domain-specific components
│   │
│   ├── pages/               # Route pages
│   │   ├── LiveViewPage.tsx
│   │   ├── DiagnosticsPage.tsx
│   │   ├── LoginPage.tsx
│   │   └── settings/        # Settings sub-pages (7 categories)
│   │
│   ├── services/            # ONVIF SOAP service clients
│   │   ├── soap/            # SOAP client and message builders
│   │   ├── deviceService.ts
│   │   ├── networkService.ts
│   │   ├── timeService.ts
│   │   ├── imagingService.ts
│   │   ├── userService.ts
│   │   ├── profileService.ts
│   │   └── authService.ts
│   │
│   ├── hooks/               # Custom React hooks
│   ├── lib/                 # Utility libraries
│   ├── types/               # TypeScript type definitions
│   ├── config/              # Application configuration
│   ├── router/              # Route definitions
│   └── test/                # Test setup and utilities
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
| Functional Spec | `specs/003-frontend-onvif-spec/spec.md` |
| Implementation Plan | `specs/003-frontend-onvif-spec/plan.md` |
| Design Assets | `docs/design/` (Figma + CSS) |
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
