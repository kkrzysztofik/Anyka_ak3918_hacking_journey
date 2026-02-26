# WWW Design System - Camera WebUI

## 🚨 MANDATORY DESIGN COMPLIANCE

**CRITICAL**: All visual implementation MUST faithfully reproduce the design assets.

| Requirement | Source |
|-------------|--------|
| Design Source | `.ai/design/ONVIF.fig` (Figma) |
| Design Proposal | `.ai/design_proposal.md` |
| Theme CSS | `.ai/design/styles/globals.css` |
| UI Components | `src/components/ui/` (shadcn/ui) |

**DO NOT** invent new designs or deviate from typography, spacing, or colors unless explicitly authorized.

## Theme: "Camera.UI" Dark + Red Accents

Optimized for monitoring applications with dark theme and red accent colors.

### Color Palette

| Purpose | Color | Hex | CSS Variable |
|---------|-------|-----|--------------|
| Background (main) | Deep black | `#0d0d0d` | `--background` |
| Background (nav) | Dark gray | `#121212` | `--nav-background` |
| Background (cards) | Medium dark | `#1c1c1e` | `--card` |
| Border/Dividers | Subtle gray | `#3a3a3c` | `--border` |
| Text (primary) | White | `#ffffff` | `--foreground` |
| Text (secondary) | Gray | `#a1a1a6` | `--muted-foreground` |
| Text (disabled) | Muted gray | `#6b6b6f` | `--disabled` |
| Accent (CTA) | Red | `#ff3b30` | `--primary` |
| Accent (hover) | Dark red | `#dc2626` | `--primary-hover` |
| Success | Green | `#34c759` | `--success` |
| Warning | Yellow | `#ff9500` | `--warning` |
| Error | Red | `#ff3b30` | `--destructive` |

### Typography

| Element | Font | Weight | Size |
|---------|------|--------|------|
| Font Family (Primary) | IBM Plex Sans | - | - |
| Font Family (Mono) | IBM Plex Mono | - | - |
| Font Family (Fallback) | Inter | - | - |
| Headings (h1) | IBM Plex Sans | Semi-Bold (600) | 28px |
| Headings (h2) | IBM Plex Sans | Semi-Bold (600) | 24px |
| Headings (h3) | IBM Plex Sans | Medium (500) | 20px |
| Body | IBM Plex Sans | Regular (400) | 14px |
| Small/Caption | IBM Plex Sans | Regular (400) | 12px |
| Buttons | IBM Plex Sans | Medium (500) | 14px |
| Labels | IBM Plex Sans | Medium (500) | 12px |
| Code/Terminal | IBM Plex Mono | Regular (400) | 13px |

**Font packages**: `@fontsource-variable/ibm-plex-sans`, `@fontsource/ibm-plex-mono`, `@fontsource/inter`

### Spacing Scale

| Name | Size | Usage |
|------|------|-------|
| xs | 4px | Tight spacing, icons |
| sm | 8px | Default padding, gaps |
| md | 16px | Section padding |
| lg | 24px | Card padding |
| xl | 32px | Section margins |
| 2xl | 48px | Page margins |

### Border Radius

| Element | Radius |
|---------|--------|
| Buttons | 8px |
| Inputs | 8px |
| Cards | 12px |
| Dialogs | 12px |
| Tooltips | 6px |

## Layout Architecture

### Dashboard (Live View) - Two-Column

```
┌─────────────────────────────────────────┐
│ [76px Icon Nav] │ [Full-width Video]   │
│                 │ + Floating PTZ       │
│                 │ + Status Overlays    │
└─────────────────────────────────────────┘
```

| Area | Width | Purpose |
|------|-------|---------|
| Icon Nav | 76px fixed | Primary navigation |
| Content | Remaining | Video feed + controls |

### Settings View - Three-Column

```
┌─────────────────────────────────────────────────────┐
│ [76px] │ [280px Categories] │ [Main Content]        │
│  Nav   │   + Icons          │   + Settings Forms    │
│        │   + Descriptions   │   + Action Buttons    │
└─────────────────────────────────────────────────────┘
```

| Area | Width | Purpose |
|------|-------|---------|
| Icon Nav | 76px fixed | Primary navigation |
| Categories | 280px fixed | Settings category list |
| Content | Remaining | Settings forms |

## Settings Categories

| Category | Icon | Description |
|----------|------|-------------|
| Identification | Info | Device name, model, status |
| Network | Network | IP, DNS, ports |
| Time Settings | Clock | Timezone, NTP |
| Maintenance | Tool | Updates, backups, logs |
| Imaging Settings | Camera | Brightness, contrast |
| User Management | Users | Accounts, permissions |
| Profiles | Settings | Configuration presets |

## Component Patterns

### Buttons

```typescript
// Primary action (red accent)
<Button variant="default">Save Changes</Button>

// Secondary action
<Button variant="outline">Cancel</Button>

// Destructive action
<Button variant="destructive">Delete</Button>

// Ghost/subtle action
<Button variant="ghost">More Options</Button>
```

### Form Fields

```typescript
// Standard input
<div className="space-y-2">
  <Label htmlFor="name">Device Name</Label>
  <Input 
    id="name" 
    data-testid="device-name-input"
    placeholder="Enter device name" 
  />
</div>

// With validation error
<div className="space-y-2">
  <Label htmlFor="ip">IP Address</Label>
  <Input 
    id="ip" 
    data-testid="network-ip-input"
    className="border-destructive" 
  />
  <p className="text-sm text-destructive">Invalid IP format</p>
</div>
```

### Cards

```typescript
<Card className="bg-card border-border">
  <CardHeader>
    <CardTitle>Device Information</CardTitle>
    <CardDescription>View and edit device details</CardDescription>
  </CardHeader>
  <CardContent>
    {/* Content */}
  </CardContent>
  <CardFooter>
    <Button data-testid="card-save-button">Save</Button>
  </CardFooter>
</Card>
```

### Dialogs

```typescript
<Dialog>
  <DialogTrigger asChild>
    <Button data-testid="open-dialog-button">Open</Button>
  </DialogTrigger>
  <DialogContent data-testid="user-dialog">
    <DialogHeader>
      <DialogTitle>Edit User</DialogTitle>
      <DialogDescription>
        Make changes to user settings
      </DialogDescription>
    </DialogHeader>
    {/* Form content */}
    <DialogFooter>
      <Button variant="outline" data-testid="dialog-cancel-button">
        Cancel
      </Button>
      <Button data-testid="dialog-save-button">
        Save
      </Button>
    </DialogFooter>
  </DialogContent>
</Dialog>
```

## Responsive Design

| Breakpoint | Width | Behavior |
|------------|-------|----------|
| Mobile | < 768px | Collapsible nav, stacked layouts |
| Tablet | 768-1024px | Condensed sidebar |
| Desktop | > 1024px | Full multi-column layout |

### Mobile Adaptations

- Navigation collapses to hamburger menu
- Settings categories become dropdown/accordion
- Forms stack vertically
- Touch-friendly hit targets (min 44px)

## Loading & Error States

### Loading

```typescript
// Full page loading
<div className="flex items-center justify-center h-full">
  <Spinner data-testid="loading-spinner" />
</div>

// Inline loading
<Button disabled>
  <Spinner className="mr-2 h-4 w-4" />
  Saving...
</Button>
```

### Error States

```typescript
// Error message
<Alert variant="destructive" data-testid="error-alert">
  <AlertTitle>Error</AlertTitle>
  <AlertDescription>
    Failed to load device information
  </AlertDescription>
</Alert>

// Empty state
<div className="text-center py-12" data-testid="empty-state">
  <p className="text-muted-foreground">No devices found</p>
</div>
```

## Accessibility Requirements

| Requirement | Implementation |
|-------------|----------------|
| Color Contrast | WCAG AA minimum (4.5:1) |
| Focus States | Visible focus rings |
| Keyboard Navigation | All interactive elements focusable |
| Screen Readers | Proper ARIA labels |
| Motion | Respect `prefers-reduced-motion` |

## Design Assets Location

```
.ai/design/
├── ONVIF.fig              # Figma source file
├── App.tsx                # Reference component
├── design_proposal.md     # Design specifications
├── components/
│   ├── ui/                # 48 Radix UI components
│   └── figma/             # Figma screen exports
└── styles/
    └── globals.css        # CSS custom properties
```

## Implementation Priority

1. **Theme & Colors**: Apply color palette and CSS variables
2. **Typography**: Font family, weights, sizes
3. **Layout System**: Sidebar navigation, content areas
4. **Components**: shadcn/ui customization
5. **Forms**: Input styling, validation states
6. **Responsive**: Mobile adaptations

## Related Memories

- `www-project-context` - Project structure and tech stack
- `www-development-standards` - Coding standards and testing
- `review-prompt-www` - Code review guidelines
