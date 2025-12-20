# WWW Project Context - Camera WebUI

## Project Description

The `cross-compile/www` directory contains the **Camera WebUI** — a React-based web administration panel for Anyka AK3918 ONVIF cameras. This is the frontend companion to the `onvif-rust` backend, providing a user-friendly interface for camera configuration and live viewing.

## Technology Stack

| Category | Technology | Version |
|----------|------------|---------|
| **Framework** | React | 19.x |
| **Build Tool** | Vite | 7.x |
| **Language** | TypeScript | 5.x |
| **Styling** | TailwindCSS | 4.x |
| **State Management** | React Query (TanStack) | 5.x |
| **Routing** | React Router | 7.x |
| **UI Components** | shadcn/ui (Radix UI) | Latest |
| **HTTP Client** | Axios | 1.x |
| **XML Parsing** | fast-xml-parser | 5.x |
| **Form Handling** | React Hook Form + Zod | 7.x / 4.x |
| **Testing** | Vitest + React Testing Library + MSW | 4.x / 16.x / 2.x |
| **Icons** | Lucide React | 0.5x |

## Project Structure

```text
cross-compile/www/
├── src/
│   ├── App.tsx              # Main application component
│   ├── Layout.tsx           # Application layout with sidebar navigation
│   ├── main.tsx             # Entry point
│   ├── index.css            # Global styles
│   ├── components/          # Reusable UI components
│   │   ├── ui/              # Base UI components (Button, Dialog, etc.)
│   │   ├── common/          # Shared components (LoadingState, etc.)
│   │   └── users/           # User-specific components
│   ├── pages/               # Route pages
│   │   ├── LiveViewPage.tsx
│   │   ├── DiagnosticsPage.tsx
│   │   ├── LoginPage.tsx
│   │   └── settings/        # Settings sub-pages (7 categories)
│   ├── services/            # ONVIF SOAP service clients
│   │   ├── soap/            # SOAP client and message builders
│   │   ├── deviceService.ts
│   │   ├── networkService.ts
│   │   ├── timeService.ts
│   │   ├── imagingService.ts
│   │   ├── userService.ts
│   │   ├── profileService.ts
│   │   └── authService.ts
│   ├── hooks/               # Custom React hooks
│   ├── lib/                 # Utility libraries
│   ├── types/               # TypeScript type definitions
│   └── test/                # Test setup and utilities
├── vite.config.ts           # Vite configuration with proxy and compression
├── tailwind.config.js       # TailwindCSS configuration
└── package.json             # Dependencies and scripts
```

## Key Commands

```bash
# Development server (proxies to camera at http://192.168.2.198:80)
npm run dev

# Connect to specific camera IP
VITE_API_TARGET=http://192.168.1.50:8080 npm run dev

# Production build (outputs to SD_card_contents/anyka_hack/onvif/www)
npm run build

# Run tests
npm run test

# Run linting
npm run lint
```

## Build Configuration

- **Output Directory**: `SD_card_contents/anyka_hack/onvif/www`
- **Compression**: Gzip and Brotli pre-compression enabled
- **Code Splitting**: Manual chunks for vendors, services, and components
- **Minification**: Terser with console/debugger removal
- **Proxy**: `/onvif`, `/utilization`, `/snapshot` routes proxied to camera

## Key Features & Phase 1 Scope (Spec: 003-frontend-onvif-spec)

Based on `specs/003-frontend-onvif-spec/spec.md`, the current phase includes:

1. **Bootstrap & Authentication**:
    - Initial connection handling
    - HTTP Basic Authentication (Admin/Operator/User roles)
2. **Settings Management** (Core functional area):
    - **Identification**: Device name, location, read-only hardware IDs
    - **Network**: IP, DNS, Ports, ONVIF Discovery
    - **Time**: Manual/NTP server configuration
    - **Imaging**: Brightness, Contrast, Saturation
    - **Maintenance**: Reboot, Reset, Backup/Restore
    - **User Management**: full CRUD for users
    - **Profiles**: CRUD for media profiles
3. **Placeholders (Future Phases)**:
    - **Live View**: Shows connection status and disabled PTZ controls (Streaming not yet available)
    - **Diagnostics**: UI scaffolding with mock data (Real-time metrics not yet available)

## Compliance & Specifications

- **Functional Spec**: `specs/003-frontend-onvif-spec/spec.md`
- **Implementation Plan**: `specs/003-frontend-onvif-spec/plan.md`
- **Design Assets**: `.ai/design/` (Mandatory compliance)

## Integration with Backend

- Communicates via ONVIF SOAP over HTTP
- Authentication via HTTP Basic Auth
- Uses HashRouter for SPA routing compatibility
- Session storage for auth state persistence
