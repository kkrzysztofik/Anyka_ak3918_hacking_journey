# Research: Frontend ↔ ONVIF Services

**Branch**: `003-frontend-onvif-spec` | **Date**: Saturday Dec 13, 2025

## Overview

This document captures research findings for the frontend implementation connecting to the onvif-rust backend.

---

## R1: ONVIF SOAP Communication Pattern

### Decision

**Custom Lightweight Client** using `axios` and `fast-xml-parser`.

### Rationale

- **Browser Compatibility**: Existing libraries like `agsh/onvif` rely on Node.js modules (`net`, `dgram`) and do not work in browsers.
- **Control**: Allows precise control over SOAP envelope construction and XML parsing to match `onvif-rust` expectations.
- **Decoupling**: Services are decoupled from the transport layer.

### Implementation Notes

```typescript
// src/services/soap/client.ts
const soapEnvelope = createSOAPEnvelope(body, authHeader);
const response = await axios.post(url, soapEnvelope, {
  headers: {
    'Content-Type': 'application/soap+xml; charset=utf-8'
  }
});
const parsed = parser.parse(response.data);
```

### Alternatives Considered

| Alternative | Why Rejected |
|-------------|--------------|
| `agsh/onvif` | Node.js only; relies on `net`/`fs` modules |
| Raw fetch | `axios` offers better interceptor support for auth/errors |
| gSOAP | Requires C++ code generation; overkill for Web UI |

---

## R2: Authentication Approach

### R2: Decision

**HTTP Basic Authentication** (`Authorization: Basic ...`) as primary mechanism.

### R2: Rationale

- **Backend Support**: `onvif-rust` dispatcher explicitly supports Basic Auth validation before WS-Security.
- **Simplicity**: Eliminates complex client-side digest generation (SHA1, Nonce, Created) required for `UsernameToken`.
- **Robustness**: Bypasses potential time-sync issues (clock skew) that affect `UsernameToken` validation.

### R2: Implementation Notes

- **Credential Storage**: Store username/password in **Redux (memory only)**.
- **Header Injection**: Axios interceptor injects `Authorization: Basic <base64>` on every request.
- **Fallback**: WS-Security `UsernameToken` logic retained in codebase as backup if needed.

### Flow

```text
1. User enters credentials.
2. App stores credentials in memory (Redux).
3. Axios interceptor adds 'Authorization: Basic ...' header.
4. Backend verifies credentials against internal UserStorage.
```

### R2: Alternatives Considered

| Alternative | Why Rejected |
|-------------|--------------|
| WS-UsernameToken | Complex to implement correctly in browser; sensitive to clock skew |
| Session Cookies | Basic Auth is stateless and standard for API clients |

---

## R3: State Management Strategy

### R3: Decision

**Redux Toolkit** for global state; **React Hook Form** for form state.

### R3: Rationale

- **Global**: Redux handles Auth, Device Status, and UI preferences effectively.
- **Forms**: React Hook Form manages complex form validation and dirty states better than manual controlled components.
- **Validation**: `zod` schema validation ensures type safety and robust input checking.

### State Structure

```typescript
// Global (Redux)
interface RootState {
  auth: { user: User | null; credentials: BasicCredentials | null };
  device: { info: DeviceInfo | null; status: DeviceStatus };
}

// Local (React Hook Form)
const { register, handleSubmit } = useForm<NetworkSettings>({
  resolver: zodResolver(networkSchema)
});
```

---

## R4: Form Handling Pattern

### R4: Decision

**React Hook Form** combined with **Zod** schema validation.

### R4: Rationale

- **Performance**: Uncontrolled components reduce re-renders.
- **Validation**: Zod provides composable, type-safe validation rules (e.g., IP address regex, range checks).
- **Developer Experience**: Standardizes form error handling and submission logic.

### Implementation Pattern

```typescript
const networkSchema = z.object({
  ipAddress: z.string().ip(),
  port: z.number().min(1).max(65535)
});

const onSubmit = (data) => service.update(data);
```

---

## R5: UI Component Strategy

### R5: Decision

**@shadcn/ui** with **Camera.UI** Dark Theme overrides.

### R5: Rationale

- **Accessibility**: Built-in ARIA support.
- **Customization**: Tailwind-based styling fits the custom design requirement.

### Key Components

| Component | Usage |
|-----------|-------|
| `Sheet` | Mobile Navigation Drawer |
| `Collapsible` | Mobile Settings Menu |
| `Form` | React Hook Form wrapper components |
| `Dialog` | About / Change Password Modals |
| `Card` | Settings containers |
| `Button`/`Input` | Standard interaction elements |

---

## R5b: React Patterns (from .cursor/rules/react.mdc)

### R5b: Decision

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

### R5c: Decision

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

### R5c: Implementation Examples

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

### R6: Decision

- React Error Boundaries at route level to catch render errors
- Toast notifications for operation success/failure
- Inline form validation errors
- Connection status indicator for backend availability

### R6: Rationale

- Prevents entire app from crashing on component errors
- Provides immediate user feedback
- Clear visual distinction between validation and system errors

### R6: Implementation Pattern

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

### R6: Alternatives Considered

| Alternative | Why Rejected |
|-------------|--------------|
| Global error modal | Interrupts user flow; feels heavy |
| Console-only errors | Poor UX; user doesn't see issues |
| Sentry/error reporting | Overkill for embedded device |

---

## R7: Routing Structure

### R7: Decision

**Layout Wrapper Pattern** with React Router v6.

### Structure

```tsx
<Routes>
  <Route path="/login" element={<LoginPage />} />
  <Route element={<Layout />}> {/* Layout.tsx handles Nav/Header */}
    <Route path="/live" element={<LiveViewPage />} />
    <Route path="/settings/*" element={<SettingsRoutes />} />
  </Route>
</Routes>
```

---

## R8: Backend Integration Points

### R8: Mock Strategy

**Node.js Mock Server** for local development.

- **Tool**: Simple Node.js script (using `fastify` or `http`).
- **Function**: Serves static XML responses.
- **Usage**: `VITE_USE_MOCK=true` proxies requests to the mock server.

### R8: Endpoints

### ONVIF Service Endpoints (onvif-rust)

| Endpoint | Operations Used |
|----------|-----------------|
| `/onvif/device_service` | GetDeviceInformation, GetSystemDateAndTime, SetSystemDateAndTime, GetUsers, CreateUsers, SetUser, DeleteUsers, SystemReboot |
| `/onvif/media_service` | GetProfiles, CreateProfile, DeleteProfile |
| `/onvif/imaging_service` | GetImagingSettings, SetImagingSettings, GetOptions |
| `/onvif/ptz_service` | (Out of scope this phase - placeholder only) |

### Request Format

All ONVIF operations use SOAP 1.2 over HTTP POST. Authentication is handled via HTTP Basic Auth headers (primary) or WS-UsernameToken (fallback).

### Response Handling

- Success: Parse SOAP body, extract response data
- Fault: Parse SOAP Fault, display user-friendly error message
- Network error: Show connection issue UI
