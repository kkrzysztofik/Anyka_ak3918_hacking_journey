# Research: Frontend ↔ ONVIF Services

**Branch**: `003-frontend-onvif-spec` | **Date**: Saturday Dec 13, 2025

## Overview

This document captures research findings for the frontend implementation connecting to the onvif-rust backend.

---

## R1: ONVIF SOAP Communication Pattern

### Decision

Use existing `onvifClient.ts` pattern with `fast-xml-parser` for XML/SOAP serialization.

### Rationale

- Already implemented and tested in `cross-compile/www/src/services/onvifClient.ts`
- Handles SOAP envelope construction with proper namespaces
- Parses XML responses into JavaScript objects
- Supports WS-UsernameToken authentication headers

### Implementation Notes

```typescript
// Existing pattern in onvifClient.ts
const soapEnvelope = XMLParserService.createSOAPEnvelope(body);
const response = await axios.post(url, soapEnvelope, {
  headers: { 'Content-Type': 'application/soap+xml' }
});
const parsed = XMLParserService.parse(response.data);
```

### Alternatives Considered

| Alternative | Why Rejected |
|-------------|--------------|
| Raw fetch + manual XML | More error-prone; duplicates existing work |
| gSOAP-generated client | Requires code generation; C-focused |
| ws-security library | Overkill; backend handles auth validation |

---

## R2: Authentication Approach

### Decision

HTTP session cookies with WS-UsernameToken embedded in SOAP headers for ONVIF operations.

### Rationale

- Backend (onvif-rust) validates WS-UsernameToken in SOAP requests
- Session cookies provide persistent auth state without frontend token management
- Follows existing pattern in deviceManagementService.ts

### Implementation Notes

- Login: POST credentials to backend auth endpoint
- Session: Browser manages cookies automatically
- ONVIF calls: Include WS-UsernameToken in SOAP header

### Flow

```text
1. User submits credentials on LoginPage
2. Backend validates and sets session cookie
3. Frontend stores auth state in Redux (role, username)
4. Subsequent ONVIF calls include WS-UsernameToken
5. Backend validates token + session on each request
```

### Alternatives Considered

| Alternative | Why Rejected |
|-------------|--------------|
| JWT tokens | Requires backend changes; non-standard for ONVIF |
| OAuth2/OIDC | Overkill for single-device embedded system |
| Basic Auth only | Less secure; password in every request |

---

## R3: State Management Strategy

### Decision

Redux Toolkit for global application state; local component state for forms.

### Rationale

- Already configured in `cross-compile/www/src/store/`
- Redux DevTools support for debugging
- Slices pattern provides clean separation
- Form state doesn't need global persistence

### State Structure

```typescript
// Global Redux state
interface RootState {
  auth: {
    isAuthenticated: boolean;
    user: { username: string; role: UserRole } | null;
    isLoading: boolean;
    error: string | null;
  };
  device: {
    info: DeviceInfo | null;
    status: 'online' | 'offline' | 'checking';
    lastChecked: Date | null;
  };
  ui: {
    sidebarOpen: boolean;
    currentTheme: 'dark';
    toasts: Toast[];
  };
}

// Local state for forms (example)
const [formData, setFormData] = useState<NetworkFormData>(initialValues);
const [isSaving, setIsSaving] = useState(false);
```

### Alternatives Considered

| Alternative | Why Rejected |
|-------------|--------------|
| Zustand | Would require migration; Redux already works |
| Jotai/Recoil | Atomic state less suited for complex forms |
| React Query | Good for caching; but forms need local state anyway |
| Context only | Insufficient for auth + device + UI state |

---

## R4: Form Handling Pattern

### Decision

Controlled components with local `useState`; validate on blur and before submit.

### Rationale

- Simple and predictable
- Matches patterns in existing design components (`.ai/design/components/`)
- No additional dependencies
- Easy to implement validation rules from spec

### Implementation Pattern

```typescript
const [ipAddress, setIpAddress] = useState(initialValue);
const [errors, setErrors] = useState<Record<string, string>>({});

const validateField = (name: string, value: string) => {
  switch (name) {
    case 'ipAddress':
      return isValidIPv4(value) ? null : 'Invalid IP address format';
    // ...
  }
};

const handleSave = async () => {
  const allErrors = validateAll(formData);
  if (Object.keys(allErrors).length > 0) {
    setErrors(allErrors);
    return;
  }
  await saveSettings(formData);
};
```

### Alternatives Considered

| Alternative | Why Rejected |
|-------------|--------------|
| React Hook Form | Adds 20KB; overkill for ~10 forms |
| Formik | Heavy; complex API |
| Final Form | Less popular; smaller ecosystem |

---

## R5: UI Component Strategy

### Decision

Use **@shadcn/ui** components (already configured in project) with custom theme overrides to match "Camera.UI" dark design.

### Rationale

- Project already has shadcn/ui configured with "new-york" style variant (per `.cursor/rules/shadcn.mdc`)
- Components are accessible by default (WCAG compliant)
- Can customize via CSS variables to match dark theme with red accents
- Use `src/components/ui/` for shadcn components
- Import via `@/components/ui/` alias

### Design System Theme Override

```css
/* Override shadcn defaults for Camera.UI theme */
:root {
  --background: #0d0d0d;
  --card: #1c1c1e;
  --border: #3a3a3c;
  --foreground: #ffffff;
  --muted-foreground: #a1a1a6;
  --primary: #ff3b30;        /* Red accent */
  --destructive: #dc2626;
  --ring: #ff3b30;
}
```

### Shadcn Components to Use

| Component | Usage |
|-----------|-------|
| Button | Primary, secondary, destructive variants |
| Input | Form inputs with labels |
| Select | Dropdown selects |
| Switch | Toggle switches |
| Dialog | Modal dialogs (About, Confirm) |
| Card | Settings section containers |
| Sonner | Toast notifications |
| Tabs | Settings category navigation |
| Table | User list, profiles list |
| Slider | Imaging settings controls |
| Skeleton | Loading states |

### Install Additional Components

```bash
npx shadcn@latest add dialog sonner tabs slider table skeleton
```

### Alternatives Considered

| Alternative | Why Rejected |
|-------------|--------------|
| Pure custom Tailwind | Reinventing the wheel; shadcn already available |
| Material UI | Wrong aesthetic; heavy bundle |
| Chakra UI | Not configured in project |

---

## R5b: React Patterns (from .cursor/rules/react.mdc)

### Decision

Follow project React conventions for performance and maintainability.

### Required Patterns

1. **Functional Components Only** - No class components
2. **Custom Hooks Location** - Extract reusable logic to `src/components/hooks/`
3. **Performance Optimization**:
   - `React.memo()` for expensive components with stable props
   - `React.lazy()` + `Suspense` for route-level code splitting
   - `useCallback` for event handlers passed to children
   - `useMemo` for expensive calculations
4. **Accessibility**:
   - `useId()` for generating unique IDs for form labels
5. **UX Enhancements**:
   - `useTransition` for non-urgent updates (keeps UI responsive)
   - `useOptimistic` for optimistic form updates

### Implementation Examples

```typescript
// Route-level code splitting
const SettingsPage = React.lazy(() => import('./pages/SettingsPage'));

// Memoized component
const ProfileCard = React.memo(({ profile }: Props) => {
  // Only re-renders when profile changes
});

// Stable event handler
const handleSave = useCallback(async () => {
  await saveSettings(data);
}, [data]);

// Expensive calculation
const filteredUsers = useMemo(() =>
  users.filter(u => u.role === selectedRole),
  [users, selectedRole]
);

// Accessible form field
const NameInput = () => {
  const id = useId();
  return (
    <>
      <label htmlFor={id}>Name</label>
      <Input id={id} />
    </>
  );
};
```

---

## R5c: Accessibility Requirements (from .cursor/rules/frontend.mdc)

### Decision

Implement WCAG-compliant accessibility using ARIA best practices.

### Required ARIA Patterns

| Pattern | Usage |
|---------|-------|
| ARIA landmarks | `<main>`, `<nav>`, `role="navigation"` for page regions |
| `aria-expanded` | Sidebar toggles, dropdowns, accordions |
| `aria-controls` | Link expandable triggers to their panels |
| `aria-live` | Toast notifications (`polite`), error messages |
| `aria-hidden` | Decorative icons, duplicate content |
| `aria-label` | Icon-only buttons, unlabeled inputs |
| `aria-describedby` | Form field help text, error messages |
| `aria-current` | Active navigation item (`aria-current="page"`) |

### Implementation Examples

```tsx
// Sidebar navigation
<nav aria-label="Main navigation">
  <a href="/live" aria-current={isActive ? "page" : undefined}>
    Live View
  </a>
</nav>

// Expandable settings sidebar
<button
  aria-expanded={isOpen}
  aria-controls="settings-panel"
  onClick={toggle}
>
  Settings
</button>
<div id="settings-panel" hidden={!isOpen}>
  {/* Settings content */}
</div>

// Toast notifications
<div role="status" aria-live="polite">
  {toast.message}
</div>

// Icon button with label
<button aria-label="Save changes">
  <SaveIcon aria-hidden="true" />
</button>
```

---

## R6: Error Handling Strategy

### Decision

- React Error Boundaries at route level to catch render errors
- Toast notifications for operation success/failure
- Inline form validation errors
- Connection status indicator for backend availability

### Rationale

- Prevents entire app from crashing on component errors
- Provides immediate user feedback
- Clear visual distinction between validation and system errors

### Implementation Pattern

```typescript
// Error boundary wrapper
<ErrorBoundary fallback={<ErrorFallback />}>
  <SettingsPage />
</ErrorBoundary>

// Toast for operations
const { showToast } = useToast();
try {
  await saveSettings(data);
  showToast({ type: 'success', message: 'Settings saved' });
} catch (error) {
  showToast({ type: 'error', message: error.message });
}

// Connection status
<ConnectionStatusBadge status={deviceStatus} />
```

### Alternatives Considered

| Alternative | Why Rejected |
|-------------|--------------|
| Global error modal | Interrupts user flow; feels heavy |
| Console-only errors | Poor UX; user doesn't see issues |
| Sentry/error reporting | Overkill for embedded device |

---

## R7: Routing Structure

### Decision

React Router v6 with nested routes for settings.

### Route Structure

```text
/                    → Redirects to /live or /login
/login               → LoginPage
/live                → LiveViewPage (placeholder)
/diagnostics         → DiagnosticsPage (placeholder)
/settings            → SettingsLayout (with nested routes)
  /settings/identification  → IdentificationPage
  /settings/network         → NetworkPage
  /settings/time            → TimePage
  /settings/imaging         → ImagingPage
  /settings/users           → UserManagementPage
  /settings/maintenance     → MaintenancePage
  /settings/profiles        → ProfilesPage
```

### Protected Routes

All routes except `/login` require authentication. Redirect to `/login` if not authenticated.

---

## R8: Backend Integration Points

### ONVIF Service Endpoints (onvif-rust)

| Endpoint | Operations Used |
|----------|-----------------|
| `/onvif/device_service` | GetDeviceInformation, GetSystemDateAndTime, SetSystemDateAndTime, GetUsers, CreateUsers, SetUser, DeleteUsers, SystemReboot |
| `/onvif/media_service` | GetProfiles, CreateProfile, DeleteProfile |
| `/onvif/imaging_service` | GetImagingSettings, SetImagingSettings, GetOptions |
| `/onvif/ptz_service` | (Out of scope this phase - placeholder only) |

### Request Format

All ONVIF operations use SOAP 1.2 over HTTP POST with WS-UsernameToken authentication.

### Response Handling

- Success: Parse SOAP body, extract response data
- Fault: Parse SOAP Fault, display user-friendly error message
- Network error: Show connection issue UI

---

## Summary

All research items resolved. No NEEDS CLARIFICATION remaining. Ready for Phase 1 design artifacts.
