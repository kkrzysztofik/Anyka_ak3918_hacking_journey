# Quickstart: Frontend Development

**Branch**: `003-frontend-onvif-spec` | **Date**: Saturday Dec 13, 2025

## Prerequisites

- Node.js 18+ (LTS recommended)
- npm 9+ or yarn 1.22+
- onvif-rust backend running (or mock server)
- Modern browser (Chrome 90+, Firefox 88+, Safari 14+)

## Project Setup

### 1. Navigate to Frontend Directory

```bash
cd cross-compile/www
```

### 2. Install Dependencies

```bash
npm install
```

### 3. Configure Backend Proxy

Edit `vite.config.ts` if your backend is on a different address:

```typescript
server: {
  proxy: {
    '/onvif': {
      target: 'http://192.168.1.100:8081',  // Your device IP
      changeOrigin: true,
    }
  }
}
```

### 4. Start Development Server

```bash
npm run dev
```

The app will be available at `http://localhost:3000`

---

## Project Structure

```text
cross-compile/www/
├── src/
│   ├── components/       # Reusable UI components
│   ├── pages/           # Route-level pages
│   ├── services/        # API communication
│   ├── store/           # Redux state management
│   ├── hooks/           # Custom React hooks
│   ├── types/           # TypeScript types
│   ├── utils/           # Utility functions
│   └── router/          # React Router config
├── package.json
├── tailwind.config.js
├── tsconfig.json
└── vite.config.ts
```

---

## Development Workflow

### Adding a New Settings Page

<!-- markdownlint-disable MD029 -->
1. **Create the page component**:

```typescript
// src/pages/settings/MySettingsPage.tsx
import { useState, useEffect } from 'react';
import { SettingsCard } from '@/components/settings/SettingsCard';
import { myService } from '@/services/myService';

export const MySettingsPage = () => {
  const [data, setData] = useState(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    myService.getData().then(setData).finally(() => setIsLoading(false));
  }, []);

  const handleSave = async (formData) => {
    await myService.setData(formData);
    // Show success toast
  };

  return (
    <SettingsCard title="My Settings">
      {isLoading ? <Spinner /> : <MyForm data={data} onSave={handleSave} />}
    </SettingsCard>
  );
};
```

2. **Add the service**:

```typescript
// src/services/myService.ts
import { onvifClient } from './onvifClient';

export const myService = {
  async getData() {
    return onvifClient.sendSOAPRequest('device_service', 'GetMyData', {});
  },
  async setData(data) {
    return onvifClient.sendSOAPRequest('device_service', 'SetMyData', data);
  }
};
```

3. **Add route**:

```typescript
// src/router/index.tsx
{
  path: 'my-settings',
  element: <MySettingsPage />
}
```

4. **Add to sidebar navigation** in `Frame.tsx`.
<!-- markdownlint-enable MD029 -->

---

## UI Components (@shadcn/ui)

This project uses **@shadcn/ui** components. Import from `@/components/ui/`:

```tsx
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { Dialog, DialogContent, DialogHeader } from "@/components/ui/dialog";
```

### Installing Additional Components

```bash
npx shadcn@latest add [component-name]

# Examples:
npx shadcn@latest add dialog sonner tabs slider table skeleton
```

### Component Patterns

```tsx
// Button variants (shadcn/ui)
<Button>Save</Button>                    // Default/primary
<Button variant="outline">Cancel</Button> // Outlined
<Button variant="destructive">Delete</Button> // Danger

// Input with label (using useId for accessibility)
const id = useId();
<label htmlFor={id}>IP Address</label>
<Input id={id} value={value} onChange={e => setValue(e.target.value)} />

// Toggle switch
<div className="flex items-center gap-2">
  <Switch id="dhcp" checked={dhcpEnabled} onCheckedChange={setDhcpEnabled} />
  <label htmlFor="dhcp">Enable DHCP</label>
</div>

// Dialog/Modal
<Dialog open={isOpen} onOpenChange={setIsOpen}>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Confirm Action</DialogTitle>
    </DialogHeader>
    {/* Content */}
  </DialogContent>
</Dialog>
```

## React Patterns (from .cursor/rules/react.mdc)

### Custom Hooks Location

Place reusable hooks in `src/components/hooks/`:

```typescript
// src/components/hooks/useDeviceStatus.ts
export const useDeviceStatus = () => {
  const [status, setStatus] = useState<'online' | 'offline'>('checking');
  // ...
  return { status, refresh };
};
```

### Performance Optimization

```typescript
// Memoize expensive components
const ProfileList = React.memo(({ profiles }: Props) => {
  return profiles.map(p => <ProfileCard key={p.token} profile={p} />);
});

// Stable callbacks for child components
const handleSave = useCallback(async () => {
  await saveSettings(formData);
}, [formData]);

// Memoize expensive calculations
const filteredUsers = useMemo(() =>
  users.filter(u => matchesSearch(u, query)),
  [users, query]
);

// Code-split routes
const SettingsPage = React.lazy(() => import('./pages/SettingsPage'));

<Suspense fallback={<LoadingSpinner />}>
  <SettingsPage />
</Suspense>
```

### Accessibility (from .cursor/rules/frontend.mdc)

```tsx
// Use useId for form accessibility
const NameInput = () => {
  const id = useId();
  return (
    <>
      <label htmlFor={id}>Device Name</label>
      <Input id={id} aria-describedby={`${id}-help`} />
      <p id={`${id}-help`} className="text-muted-foreground text-sm">
        Name displayed in network discovery
      </p>
    </>
  );
};

// ARIA for expandable content
<button
  aria-expanded={isOpen}
  aria-controls="settings-panel"
>
  Settings
</button>

// ARIA for navigation
<nav aria-label="Main navigation">
  <a href="/live" aria-current={isLive ? "page" : undefined}>
    Live View
  </a>
</nav>
```

## Theme Colors

```css
/* Camera.UI dark theme (override shadcn defaults) */
--background: #0d0d0d;
--card: #1c1c1e;
--border: #3a3a3c;
--foreground: #ffffff;
--muted-foreground: #a1a1a6;
--primary: #ff3b30;
--destructive: #dc2626;
```

---

## Testing

### Run Type Checks

```bash
npm run type-check
```

### Run Linting

```bash
npm run lint
```

### Run Tests (when configured)

```bash
npm test
```

---

## Building for Production

### Create Production Build

```bash
npm run build
```

Output goes to `../../SD_card_contents/anyka_hack/web_interface/www/`

### Preview Production Build

```bash
npm run preview
```

---

## Backend Communication

### ONVIF Client Usage

```typescript
import { onvifClient } from '@/services/onvifClient';

// Send SOAP request
const response = await onvifClient.sendSOAPRequest(
  'device_service',           // Service endpoint
  'GetDeviceInformation',     // ONVIF action
  { 'tds:GetDeviceInformation': {} }  // Request body
);

// Response structure
{
  success: boolean;
  data?: object;
  error?: string;
}
```

### Error Handling

```typescript
try {
  const result = await myService.doSomething();
  showToast({ type: 'success', message: 'Done!' });
} catch (error) {
  showToast({ type: 'error', message: error.message });
}
```

---

## Common Tasks

### Add Toast Notification

```typescript
import { useToast } from '@/hooks/useToast';

const MyComponent = () => {
  const { showToast } = useToast();

  const handleAction = () => {
    showToast({
      type: 'success',  // 'success' | 'error' | 'warning' | 'info'
      message: 'Settings saved successfully'
    });
  };
};
```

### Check User Role

```typescript
import { useAuth } from '@/hooks/useAuth';

const MyComponent = () => {
  const { user, isAdmin } = useAuth();

  return (
    <div>
      {isAdmin && <AdminOnlyButton />}
      <p>Logged in as: {user?.username}</p>
    </div>
  );
};
```

### Handle Loading State

```typescript
const [isLoading, setIsLoading] = useState(true);
const [error, setError] = useState<string | null>(null);

useEffect(() => {
  loadData()
    .then(setData)
    .catch(e => setError(e.message))
    .finally(() => setIsLoading(false));
}, []);

if (isLoading) return <LoadingSpinner />;
if (error) return <ErrorMessage message={error} />;
return <Content data={data} />;
```

---

## Troubleshooting

### CORS Issues

If you see CORS errors, ensure:

1. Vite proxy is configured correctly in `vite.config.ts`
2. Backend is running and accessible
3. You're using the dev server (not opening HTML directly)

### SOAP Request Failures

Check:

1. Backend is running: `curl http://device-ip:8081/onvif/device_service`
2. Credentials are correct
3. Request body matches ONVIF schema

### TypeScript Errors

```bash
# Check types without building
npm run type-check

# Fix common issues
npx tsc --noEmit
```

---

## Reference Documentation

- [ONVIF Specifications](https://www.onvif.org/specs/)
- [React Documentation](https://react.dev/)
- [Redux Toolkit](https://redux-toolkit.js.org/)
- [Tailwind CSS](https://tailwindcss.com/docs)
- [Vite](https://vitejs.dev/guide/)
