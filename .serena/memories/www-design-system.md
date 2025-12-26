# WWW Design System - Camera WebUI

## 🚨 MANDATORY DESIGN COMPLIANCE

**CRITICAL**: All visual implementation MUST faithfully reproduce the design assets located in **`.ai/design`**.

- **Source of Truth**: `.ai/design/ONVIF.fig` (Figma File) and `.ai/design_proposal.md`
- **UI Components**: Use `shadcn/ui` components (in `src/components/ui`) as the base. Customize them to match the Figma designs.
- **Theme**: Strictly follow `.ai/design/styles/globals.css`

**DO NOT** invent new designs or deviate from the specified typography, spacing, or color palette unless explicitly authorized. The user expects pixel-perfect implementation of the provided design proposal.

## Visual Design System

### Theme: "Camera.UI" Dark + Red Accents

The design follows a dark theme optimized for monitoring applications with red accent colors for active states and CTAs.

### Color Palette

| Purpose | Color | Hex |
|---------|-------|-----|
| Main Background | Deep black | `#0d0d0d` |
| Navigation Areas | Dark gray | `#121212` |
| Cards/Panels | Medium dark gray | `#1c1c1e` |
| Borders/Dividers | Subtle gray | `#3a3a3c` |
| Primary Text | White | `#ffffff` |
| Secondary Text | Gray | `#a1a1a6` |
| Disabled Text | Muted gray | `#6b6b6f` |
| Accent (CTAs) | Red | `#ff3b30` / `#dc2626` |
| Online Status | Green | `#34c759` |

### Typography

- **Font Family**: Inter
- **Font Weights**: Regular (400), Medium (500), Semi-Bold (600)
- **Font Sizes**: 12px, 14px, 16px, 18px, 20px, 24px, 28px

### Spacing & Layout

- **Spacing Scale**: 8px, 16px, 24px, 32px, 48px
- **Border Radius**: 8px (inputs/small elements), 12px (cards/buttons)
- **Sidebar Widths**: 76px (icon nav), 280px (settings categories)

## Layout Architecture

### 1. Dashboard (Live View) - Two-Column

```text
┌─────────────────────────────────────────┐
│ [76px Icon Nav] │ [Full-width Video]   │
│                 │ + Floating PTZ       │
│                 │ + Status Overlays    │
└─────────────────────────────────────────┘
```

- Left: Icon navigation sidebar (76px)
- Right: Full-width video feed with floating/overlay controls

### 2. Settings View - Three-Column

```text
┌─────────────────────────────────────────────────────┐
│ [76px] │ [280px Categories] │ [Main Content]        │
│  Nav   │   + Icons          │   + Settings Forms    │
│        │   + Descriptions   │   + Action Buttons    │
└─────────────────────────────────────────────────────┘
```

- Left: Icon navigation sidebar (76px)
- Middle: Settings category sidebar (280px) with icons and descriptions
- Right: Main content area with settings forms

## Settings Categories (7 Total)

| Category | Description |
|----------|-------------|
| **Identification** | Device name, model, status, editable name/location |
| **Network** | IP address, DNS settings, network ports |
| **Time Settings** | Timezone and NTP server configuration |
| **Maintenance** | System updates, backups, system logs |
| **Imaging Settings** | Camera image controls (brightness, contrast, saturation) |
| **User Management** | Account and permission management |
| **Profiles** | Configuration profiles and presets |

## Design Assets Location

Design assets are located in `.ai/design/`:

```text
.ai/design/
├── ONVIF.fig              # Figma source file
├── App.tsx                # Reference component
├── components/            # Figma-exported components
│   ├── ui/                # 48 Radix UI components
│   └── figma/             # Figma screen exports
├── styles/
│   └── globals.css        # CSS custom properties and theme
└── guidelines/            # (Empty - for future design guidelines)
```

## Responsive Design

| Screen Size | Behavior |
|-------------|----------|
| Desktop | Fixed sidebars, multi-column layouts, full feature set |
| Mobile | Collapsible navigation, stacked layouts, touch-optimized controls |

## Modal Dialogs

- **About Modal**: Device info, firmware version, build time, license
- **Current User Popup**: Profile menu from bottom of icon sidebar
- **Change Password Modal**: Password change form
- **User Modal**: Create/edit user accounts
- **Profiles Modal**: Create/edit configuration profiles

## Implementation Priority

1. **Theme & Icons**: Color palette, typography, icon system
2. **Layout System**: Adaptive 2-column/3-column layouts
3. **Dashboard**: Simplified video player with floating controls
4. **Settings**: Category navigation and form components
