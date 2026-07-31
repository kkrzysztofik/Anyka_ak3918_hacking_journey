# Implementation Plan: Frontend ↔ ONVIF Services

**Branch**: `003-frontend-onvif-spec` | **Date**: Saturday Dec 13, 2025 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/003-frontend-onvif-spec/spec.md`

## Summary

Build a React-based admin panel for managing a single ONVIF camera device. The frontend provides authentication, device settings management (Identification, Network, Time, Imaging, Users, Maintenance, Profiles), and placeholder views for Diagnostics and Live Video streaming. Communicates with the onvif-rust backend via SOAP/XML over HTTP.

## Technical Context

**Language/Version**: TypeScript 5.x with React 19.x
**Primary Dependencies**:

- React 19
- **TanStack Query v5** (Server Access & Caching)
- React Router 6.x
- Axios
- Tailwind CSS 3.x
- **@shadcn/ui** (new-york style)
- Lucide React (icons)
- fast-xml-parser
- **react-hook-form** (Form management)
- **zod** & **@hookform/resolvers** (Validation)

**Storage**: N/A (all state from backend; session via memory/cookies)
**Testing**: Vitest + React Testing Library (to be added)
**Target Platform**: Modern browsers (Chrome 90+, Firefox 88+, Safari 14+, Edge 90+); responsive (desktop + mobile)
**Project Type**: Web application (frontend-only, backend exists at `cross-compile/onvif-rust`)
**Performance Goals**:

- Initial load < 3s on local network
- Page transitions < 500ms
- **Gzip/Brotli Compression** enabled for serving assets
- **Code Splitting** implemented for all routes
**Constraints**: Must work on embedded device (limited bandwidth); graceful degradation when backend unavailable
**Scale/Scope**: Single camera; ~15 views/screens; 3 user roles

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Applicable | Status | Notes |
|-----------|------------|--------|-------|
| **I. Security First** | ✅ Yes | PASS | Sign-in required; role-based access; input validation on forms |
| **II. Standards Compliance** | ✅ Yes | PASS | Follows ONVIF service contracts; uses standard SOAP/XML communication |
| **III. Open Source Transparency** | ✅ Yes | PASS | Code in public repo; clear component structure |
| **IV. Hardware Compatibility** | ⚪ N/A | - | Frontend runs in browser, not on device |
| **V. Developer Experience** | ✅ Yes | PASS | TypeScript; ESLint/Prettier; component-based architecture |
| **VI. Performance Optimization** | ✅ Yes | PASS | Code splitting; lazy loading; CDN externals for React |
| **VII. Reliability** | ✅ Yes | PASS | Error boundaries; graceful backend failure handling; no broken UI |
| **VIII. Educational Value** | ✅ Yes | PASS | Clear component patterns; documented services |

**Rust Addendum**: Per constitution, the Rust-specific addendum applies to `cross-compile/onvif-rust/`. This frontend plan follows web development standards while respecting the backend's ONVIF service contracts.

**Gate Result**: ✅ PASS - No violations requiring justification

## Project Structure

### Documentation (this feature)

```text
specs/003-frontend-onvif-spec/
├── plan.md              # This file
├── architecture-decisions.md # Architecture log
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (API contracts)
└── checklists/          # Quality validation
    └── requirements.md
```

### Source Code (repository root)

```text
mock-server/              # Local dev mock server
├── package.json
└── server.js

cross-compile/www/
├── src/
│   ├── components/           # Reusable UI components
│   │   ├── ui/              # @shadcn/ui components (Button, Input, Sheet, etc.)
│   │   ├── hooks/           # Custom React hooks (per react.mdc guidelines)
│   │   ├── layout/          # Layout components (Sidebar, Layout, Nav)
│   │   └── settings/        # Settings-specific components
│   ├── pages/               # Route-level page components
│   │   ├── LoginPage.tsx
│   │   ├── LiveViewPage.tsx
│   │   ├── DiagnosticsPage.tsx
│   │   └── settings/        # Settings sub-pages
│   │       ├── IdentificationPage.tsx
│   │       ├── NetworkPage.tsx
│   │       ├── TimePage.tsx
│   │       ├── ImagingPage.tsx
│   │       ├── UserManagementPage.tsx
│   │       ├── MaintenancePage.tsx
│   │       └── ProfilesPage.tsx
│   ├── services/            # API/backend communication
│   │   ├── soap/            # Low-level SOAP client
│   │   ├── authService.ts   # Authentication
│   │   ├── deviceService.ts # Device management
│   │   ├── networkService.ts
│   │   ├── timeService.ts
│   │   ├── imagingService.ts
│   │   ├── userService.ts
│   │   ├── maintenanceService.ts
│   │   └── profileService.ts
│   ├── lib/                 # Shared utilities & State
│   │   ├── queryClient.ts   # TanStack Query Client
│   │   ├── store.ts         # User session store (Context/Zustand if needed)
│   │   └── utils.ts
│   ├── hooks/               # Custom React hooks
│   │   ├── useAuth.ts
│   │   ├── useDevice.ts
│   │   └── useToast.ts
│   ├── types/               # TypeScript type definitions
│   │   └── index.ts
│   ├── utils/               # Utility functions
│   │   ├── xmlParser.ts
│   │   ├── validation.ts
│   │   └── errorHandler.ts
│   ├── config/              # Configuration
│   │   └── index.ts
│   ├── router/              # React Router configuration
│   │   └── index.tsx
│   ├── styles/              # Global styles
│   │   └── globals.css
│   ├── Layout.tsx           # Main App Wrapper
│   ├── main.tsx
│   └── index.css
├── tests/                   # Test files (to be added)
│   ├── unit/
│   ├── integration/
│   └── e2e/
├── package.json
├── tailwind.config.js
├── tsconfig.json
└── vite.config.ts
```

**Structure Decision**: Using the existing `cross-compile/www/` project structure. The application follows a standard React SPA pattern with:

- **Pages** for route-level components
- **Components** for reusable UI elements
- **Services** for API communication (SOAP/XML to onvif-rust)
- **TanStack Query** for server state management & caching
- **Hooks** for shared logic

## Phase 0: Research Summary

### R1: ONVIF SOAP Communication Pattern

**Decision**: **Custom Lightweight Client** with `axios` + `fast-xml-parser`
**Rationale**: Browser-compatible; decoupled from UI; allows precise SOAP envelope control

### R2: Authentication Approach

**Decision**: **HTTP Basic Auth** as primary
**Rationale**: Backend supports it explicitly; simpler than WS-UsernameToken; bypasses clock skew issues

### R3: State Management

**Decision**: **TanStack Query (React Query)** for server state; Context for auth.
**Rationale**: Most state is server-side (settings). React Query handles caching, deduplication, and revalidation out of the box, reducing boilerplate compared to Redux.

### R4: Form Handling

**Decision**: **React Hook Form** + **Zod**
**Rationale**: Robust validation schema; better performance than controlled components

### R5: UI Component Library (from `.cursor/rules/shadcn.mdc`)

**Decision**: Use **@shadcn/ui** components with theme overrides for "Camera.UI" dark theme
**Rationale**: Already configured in project with "new-york" style; accessible by default; use `@/components/ui/` alias
**Required Components**: Button, Input, Select, Switch, Dialog, Card, Sonner (toast), Tabs, Table, Slider, Skeleton, **Sheet**, **Collapsible**, **Form**

### R5b: React Patterns (from `.cursor/rules/react.mdc`)

**Decision**: Follow project React conventions
**Requirements**:

- Functional components only (no class components)
- Custom hooks in `src/components/hooks/`
- `React.memo()` for expensive components
- `React.lazy()` + `Suspense` for route code-splitting
- `useCallback`, `useMemo` for performance
- `useId()` for accessible form labels
- `useTransition` for non-urgent updates

### R5c: Accessibility (from `.cursor/rules/frontend.mdc`)

**Decision**: WCAG-compliant ARIA implementation
**Requirements**:

- ARIA landmarks (`<main>`, `<nav>`, roles)
- `aria-expanded`/`aria-controls` for expandable content
- `aria-live` for toasts and dynamic updates
- `aria-label` for icon buttons
- `aria-current="page"` for active navigation

### R6: Error Handling

**Decision**: Error boundaries at route level; toast notifications for operations
**Rationale**: Prevents broken UI; provides user feedback

## Phase 1: Design Artifacts

### Data Model

See [data-model.md](./data-model.md) for complete entity definitions.

**Core Entities**:

- `AuthState`: Current user session (username, role, isAuthenticated)
- `DeviceInfo`: Device identity and status from GetDeviceInformation
- `NetworkSettings`: Network configuration from GetNetworkInterfaces
- `TimeSettings`: Time configuration from GetSystemDateAndTime
- `ImagingSettings`: Image parameters from GetImagingSettings
- `User`: User account from GetUsers
- `Profile`: Media profile from GetProfiles

### API Contracts

See [contracts/](./contracts/) directory:

- `contracts/auth.md`: Authentication endpoints
- `contracts/device.md`: Device service operations
- `contracts/network.md`: Network service operations
- `contracts/imaging.md`: Imaging service operations
- `contracts/user.md`: User management operations
- `contracts/media.md`: Profile management operations

### Quickstart

See [quickstart.md](./quickstart.md) for development setup instructions.

## Complexity Tracking

> No violations requiring justification - all checks passed.

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| Single project | ✅ | Frontend-only; backend already exists |
| No new abstractions | ✅ | Using existing service patterns |
| Standard routing | ✅ | React Router with nested settings routes |
| Minimal state | ✅ | React Query for async state; local state for forms |
