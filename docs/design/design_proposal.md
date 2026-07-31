# Web Admin Panel Design Proposal (v4 - Simplified)

## 🎯 Scope & Constraints

- **Single Camera Only**: The interface is dedicated to one device.
- **Single Stream**: One primary live feed.
- **No Recordings**: Feature removed.
- **Theme**: "Camera.UI" Dark + Red Accents.

---

## 🎨 Visual Design System

### Color Palette

- **Backgrounds**:
  - Deep black (#0d0d0d) for main background
  - Dark gray (#121212) for navigation areas
  - Medium dark gray (#1c1c1e) for cards and panels
- **Borders**: Subtle gray (#3a3a3c) for all borders and dividers
- **Text Colors**:
  - White for primary text
  - Gray (#a1a1a6) for secondary text
  - Muted gray (#6b6b6f) for disabled/read-only text
- **Accent Colors**:
  - Red (#ff3b30, #dc2626) for active states, CTAs, and highlights
- **Status Colors**:
  - Green (#34c759) for online/active indicators

### Typography

- **Font Family**: Inter
- **Font Weights**: Regular (400), Medium (500), Semi-Bold (600)
- **Sizes**: 12px, 14px, 16px, 18px, 20px, 24px, 28px

### Spacing & Layout

- **Spacing Scale**: 8px, 16px, 24px, 32px, 48px
- **Border Radius**: 8px for inputs and small elements, 12px for cards and buttons
- **Sidebar Widths**: 76px (icon nav), 280px (settings categories)

---

## 🎨 Visual Mockups

### 1. Dashboard (Live View) - **SIMPLIFIED**

Removed the "Camera List" sidebar. The dashboard is now a clean, immersive view of the single live stream.

- **Layout**: 2-Columns (Icon Nav | Main Video).
- **Controls**: Floating/Overlay PTZ controls and status pills to maximize video area.

![Dashboard Mockup (First Iteration)](./img/dashboard_mockup.png)

![Dashboard Design (Figma)](./img/dashboard_figma.png)

### 2. Login Screen

![Login Mockup (First Iteration)](./img/login_mockup.png)

![Login Design (Figma)](./img/login_figma.png)

### 3. Diagnostics

The Diagnostics view provides system monitoring and diagnostic information.

![Diagnostics Design (Figma)](./img/diagnostics_figma.png)

### 4. About Modal

The About modal is accessible from the user profile menu and displays:

- Device information and firmware version
- Build time information
- Home URL link
- Proprietary license information and terms
- Device serial number and MAC address
- OK button to close the modal

![About Modal Design (Figma)](./img/about_modal_figma.png)

### 3. Settings Interface

Uses a 3-column layout to organize the configuration categories effectively:

- Left icon navigation sidebar
- Middle settings category sidebar with icons and descriptions
- Right main content area with settings forms

![Settings Mockup (First Iteration)](./img/settings_mockup.png)

#### Identification Settings

The Identification Settings page provides device information and configuration:

- **Device Status Section**: Displays device name, model, and online status indicator
- **Device Configuration Section**: Editable fields for Name and Location
- **Hardware Information Section**: Read-only display of manufacturer, model, hardware version, and firmware version
- **Network & System Information Section**: Read-only display of device ID, IP address, MAC address, and ONVIF version
- **Action Buttons**: Save Changes (primary red button) and Reset to Default (secondary button)
- **Information Notice**: Explains which fields are editable vs read-only

![Settings Design - Identification (Figma)](./img/settings_identification_figma.png)

#### Network Settings

Configure IP address, DNS settings, and network ports.

![Settings Design - Network (Figma)](./img/settings_network_figma.png)

#### Time Settings

Set timezone and configure NTP server settings.

![Settings Design - Time (Figma)](./img/settings_time_figma.png)

#### Maintenance Settings

Manage system updates, backups, and view system logs.

![Settings Design - Maintenance (Figma)](./img/settings_maintenance_figma.png)

#### Imaging Settings

Control camera image settings and exposure parameters.

![Settings Design - Imaging (Figma)](./img/settings_imaging_figma.png)

#### User Management Settings

Manage user accounts and permissions.

![Settings Design - User Management (Figma)](./img/settings_user_management_figma.png)

#### Profiles Settings

Configure and manage configuration profiles and presets.

![Settings Design - Profiles (Figma)](./img/settings_profiles_figma.png)

### 5. Modals & Popups

#### Current User Popup

The current user popup menu appears when clicking the user profile button at the bottom of the icon sidebar.

![Current User Popup (Figma)](./img/current_user_popup_figma.png)

#### Change Password Modal

Modal for changing the current user's password.

![Change Password Modal (Figma)](./img/change_password_modal_figma.png)

#### User Modal

Modal for creating or editing user accounts.

![User Modal (Figma)](./img/user_modal_figma.png)

#### Profiles Modal

Modal for creating or editing configuration profiles.

![Profiles Modal (Figma)](./img/profiles_modal_figma.png)

---

## 🏗️ Revised Architecture

### Layout Architecture

The application uses adaptive layouts that change based on the current view:

1. **Dashboard (Live View)**: Two-column layout
    - *Left*: Icon navigation sidebar (76px) - primary navigation icons
    - *Right*: Full-width video feed area with floating/overlay controls

2. **Settings View**: Three-column layout
    - *Left*: Icon navigation sidebar (76px) - primary navigation icons
    - *Middle*: Settings category sidebar (280px) - settings category list with icons and descriptions
    - *Right*: Main content area - settings forms and configuration panels

### Settings Categories

The settings interface is organized into 7 main categories:

- **Identification**: Device name and identification information
- **Network**: IP, DNS, and port configuration
- **Time Settings**: Timezone and NTP server configuration
- **Maintenance**: Updates, backups, and system logs
- **Imaging Settings**: Camera image controls and exposure settings
- **User Management**: Account and permission management
- **Profiles**: Configuration profiles and presets

---

## 🔄 User Experience Flows

### Navigation Flow

1. **Primary Navigation**: User clicks icon in left sidebar (Live Video, Settings, Diagnostics)
2. **Settings Navigation**: When in Settings view, user selects category from middle sidebar
3. **Content Display**: Selected category's form/content appears in main content area

### Settings Flow

1. User navigates to Settings view
2. User selects a category from the settings sidebar (e.g., Identification, Network)
3. Settings form for that category displays in main content area
4. User views/edits settings fields
5. User clicks "Save Changes" to apply modifications

### Modal Flow

1. User clicks user profile button (bottom of icon sidebar)
2. User selects action from profile menu (About, Change Password, Sign Out)
3. Modal overlay appears with relevant content
4. User interacts with modal or closes it

---

## 📱 Responsive Design

The interface adapts to different screen sizes:

- **Desktop**: Fixed sidebars, multi-column layouts, full feature set
- **Mobile**:
  - Collapsible navigation with hamburger menu
  - Stacked layouts for settings forms
  - Touch-optimized controls and button sizes
  - Modal dialogs optimized for smaller screens

---

## 🚀 Implementation Priority

1. **Theme & Icons**: Setup color palette, typography, and icon system
2. **Layout System**: Implement adaptive layout system (2-column for dashboard, 3-column for settings)
3. **Dashboard**: Build the simplified video player view with floating controls
4. **Settings**: Build the settings category navigation and form components
