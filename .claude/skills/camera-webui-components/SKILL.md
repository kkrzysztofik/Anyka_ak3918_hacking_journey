---
name: camera-webui-components
description: Use when building or modifying React 19 components for the camera WebUI (shadcn/ui, settings pages, forms, dialogs, design system tokens, dark theme).
version: 2.0.0
---

# Camera WebUI Component Development

Build production-grade React 19 components for `cross-compile/www` using shadcn/ui, TypeScript, and the project's design system.

## MANDATORY Design Docs (load before changing UI)

The authoritative design sources — **never invent new colors/typography/layout**:

| Requirement | Source |
|-------------|--------|
| Design system (colors, typography, spacing, components) | `.serena/memories/www-design-system.md` |
| Figma source | `docs/design/ONVIF.fig` |
| Design proposal | `docs/design/design_proposal.md` |
| Design review | `docs/design/DESIGN_REVIEW.md` |
| Theme CSS (source) | `docs/design/styles/globals.css` |
| **Implemented** theme CSS | `cross-compile/www/src/styles/globals.css` |

The **implemented** theme lives in `src/styles/globals.css` — that is ground truth for runtime styling. When the memory doc and implemented CSS disagree, the implemented CSS wins (it is what ships).

## Implemented Theme (src/styles/globals.css)

"Industrial Dark" theme. **Primary action is blue, red is the accent** (not the other way around):

| Purpose | CSS Variable | HSL |
|---------|--------------|-----|
| Background | `--background` | `220 10% 4%` |
| Foreground | `--foreground` | `210 20% 98%` |
| Card | `--card` | `220 10% 10%` |
| Border/Input | `--border` / `--input` | `220 5% 22%` |
| Primary (CTA, authoritative blue) | `--primary` | `217 91% 60%` |
| Accent (attention-grabbing red) | `--accent` | `0 84% 60%` |
| Destructive | `--destructive` | `0 62.8% 30.6%` |
| Muted foreground | `--muted-foreground` | `215 16% 65%` |
| Ring | `--ring` | `217 91% 60%` |
| Radius | `--radius` | `0.5rem` |

Status colors: `--status-connected` `142 71% 45%`, `--status-disconnected` `215 10% 45%`, `--status-warning` `38 92% 50%`, `--status-error` `0 84% 60%`.

### Styling Rules

- **Always use CSS variables via Tailwind** (`bg-card`, `text-foreground`, `border-border`, `bg-primary`). Never hardcode hex/HSL literals.
- Tailwind config: `tailwind.config.js` (project is Tailwind v4 with `@config`). `@layer components` utilities exist in globals.css: `.card`, `.technical-panel`, `.status-badge-connected`, `.empty-state`, `.skeleton*`, `.live-indicator`, `.data-highlight`.
- Typography uses `font-sans` (IBM Plex Sans family, `Inter` fallback) and `font-mono` (IBM Plex Mono) for technical data. See `www-design-system.md` for the type scale.
- Accessibility: WCAG AA contrast, visible focus rings (`:focus-visible` → `--ring`), keyboard navigation, ARIA labels, `prefers-reduced-motion` support (already handled globally in globals.css).

## Component Rules

- Prefer shadcn/ui primitives from `src/components/ui/` — don't invent new base components.
- Strict TypeScript: avoid `any`; use `unknown` + type guards.
- All testable elements get a `data-testid` (required by the testing skill; no role/text/class selectors).
- Place components under `src/components/` (`ui/`, `layout/`, `settings/`, `common/` as appropriate).

## Component Template

```tsx
'use client';

import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Card, CardContent, CardFooter, CardHeader, CardTitle } from '@/components/ui/card';

interface DeviceSettingsProps {
  onSave: (data: DeviceData) => Promise<void>;
  initialData?: DeviceData;
}

interface DeviceData { name: string; model: string; }

export function DeviceSettings({ onSave, initialData = { name: '', model: '' } }: DeviceSettingsProps) {
  const [data, setData] = useState<DeviceData>(initialData);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSave = async () => {
    if (!data.name.trim()) { setError('Device name is required'); return; }
    try {
      setIsLoading(true); setError(null);
      await onSave(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'An error occurred');
    } finally { setIsLoading(false); }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Device Information</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        {error && (
          <div data-testid="error-message" role="alert" className="bg-destructive/10 border border-destructive rounded text-destructive p-3 text-sm">
            {error}
          </div>
        )}
        <div className="space-y-2">
          <label htmlFor="device-name" className="text-sm font-medium">Device Name</label>
          <Input
            id="device-name" data-testid="device-name-input" placeholder="Enter device name"
            value={data.name} onChange={(e) => setData({ ...data, name: e.target.value })}
            disabled={isLoading}
          />
        </div>
        <div className="space-y-2">
          <label htmlFor="device-model" className="text-sm font-medium">Model</label>
          <Input id="device-model" data-testid="device-model-input" placeholder="AK3918"
            value={data.model} onChange={(e) => setData({ ...data, model: e.target.value })} disabled={isLoading} />
        </div>
      </CardContent>
      <CardFooter className="gap-2 justify-end pt-4 border-t border-border">
        <Button variant="outline" data-testid="device-settings-cancel-button" onClick={() => setData(initialData)} disabled={isLoading}>
          Cancel
        </Button>
        <Button data-testid="device-settings-save-button" onClick={handleSave} disabled={isLoading}>
          {isLoading ? 'Saving...' : 'Save Changes'}
        </Button>
      </CardFooter>
    </Card>
  );
}
```

## Common Patterns

- **Forms**: controlled inputs + `data-testid` per field; validation errors as `<p role="alert">` under the field. Prefer React Hook Form for complex forms (see `www-development-standards` memory).
- **Dialogs**: shadcn `Dialog` (`DialogContent data-testid="..."`, footer with Cancel/Confirm buttons).
- **Status/feedback**: `.status-badge-connected` utility, `Skeleton`/`.skeleton-*` for loading, `.empty-state` for empty, `Alert` for errors.
- **Data fetching**: TanStack Query hooks (`queryClient` in `src/lib/queryClient.ts`); mutations via `useMutation`.

## Reference

For detailed patterns see `.serena/memories/www-development-standards.md` (component structure, data attributes, error handling) and `.serena/memories/www-design-system.md` (layout, spacing, components). Quality gates: `cd cross-compile/www && npm run lint && npm run type-check && npm run test`.
