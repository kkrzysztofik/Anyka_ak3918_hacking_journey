# Tasks: Frontend ↔ ONVIF Services

**Feature**: Frontend ONVIF Implementation
**Spec**: `specs/003-frontend-onvif-spec/spec.md`
**Plan**: `specs/003-frontend-onvif-spec/plan.md`

## Phase 1: Setup (Project Initialization)

*Goal: Initialize the React project with all core dependencies and configuration.*

- [x] T001 Initialize Vite project with React 19 & TypeScript in `cross-compile/www/`
- [x] T002 Install core dependencies (React 19, TanStack Query, Axios, React Router, fast-xml-parser, Zod, React Hook Form) in `cross-compile/www/package.json`
- [x] T003 Configure Tailwind CSS and PostCSS in `cross-compile/www/tailwind.config.js` and `postcss.config.js`
- [x] T004 Initialize Shadcn/UI (new-york style) and add core components (Button, Input, Card, Sheet, Form) in `cross-compile/www/src/components/ui/` (Reference `.ai/design/components/ui`)
- [x] T005 Create project directory structure (pages, services, hooks, lib, types) in `cross-compile/www/src/`
- [x] T006 Configure Vite for Gzip/Brotli compression and Path Aliases (@/) in `cross-compile/www/vite.config.ts`

## Phase 2: Foundational (Blocking Prerequisites)

*Goal: Establish the core architecture (Auth, API Client, State Management) needed for all features.*

- [x] T007 Implement shared `soapEnvelope` builder and XML parser config in `cross-compile/www/src/services/soap/client.ts`
- [x] T008 Implement `QueryClient` singleton with default staleTime/caching rules in `cross-compile/www/src/lib/queryClient.ts`
- [x] T009 Implement `AuthContext` and `useAuth` hook for managing Basic Auth credentials in `cross-compile/www/src/hooks/useAuth.tsx`
- [x] T010 Implement Axios Interceptor to inject Basic Auth headers in `cross-compile/www/src/services/api.ts`
- [x] T011 Create Main Layout component (Sidebar, Header, Mobile Nav) in `cross-compile/www/src/Layout.tsx`
- [x] T012 Set up React Router with ProtectedRoute wrapper and Code Splitting (Suspense) in `cross-compile/www/src/router/index.tsx`
- [x] T013 Create global `globals.css` with "Camera.UI" Dark Theme variables in `cross-compile/www/src/styles/globals.css`
- [x] T013a Create simple Node.js Mock Server for local development/integration testing in `mock-server/server.js`
- [ ] T013b Implement `static_file_server` in `cross-compile/onvif-rust` to serve `www/dist` with Gzip/Brotli support (DEFERRED)

## Phase 3: User Story 1 - Project Bootstrap (P1)

*Goal: User can sign in, see connection status, and navigate the app skeleton.*

- [x] T014 [US1] Create Login Page with form validation (Zod) in `cross-compile/www/src/pages/LoginPage.tsx`
- [x] T015 [US1] Implement `authService.login` (verify credentials via simple DeviceInfo call) in `cross-compile/www/src/services/authService.ts`
- [x] T016 [US1] Create "Connection Status" indicator component in `cross-compile/www/src/components/common/ConnectionStatus.tsx`
- [x] T017 [US1] Implement "Dashboard" landing page (device info + quick actions) in `cross-compile/www/src/pages/DashboardPage.tsx`
- [x] T018 [US1] Verify graceful degradation when backend is unreachable (Sonner toast notifications)

## Phase 4: User Story 2 - Identification Settings & About (P2)

*Goal: View and edit device identity (Name, Loc) and see read-only HW info.*

- [x] T019 [US2] Implement `deviceService.getDeviceInformation` SOAP call in `cross-compile/www/src/services/deviceService.ts`
- [x] T020 [US2] Implement `deviceService.setScopes` (for Location/Name updates) in `cross-compile/www/src/services/deviceService.ts`
- [x] T021 [US2] Create Zod schema for Identification settings in `cross-compile/www/src/lib/schemas/identification.ts`
- [x] T022 [US2] Create Identification Settings Page with Read-Only fields (FW, Serial) and Editable fields (Name) in `cross-compile/www/src/pages/settings/IdentificationPage.tsx`
- [x] T023 [US2] Create "About Device" modal/dialog component in `cross-compile/www/src/components/common/AboutDialog.tsx`

## Phase 5: User Story 3 - Network Settings (P3)

*Goal: Configure IP, DNS, and Ports.*

- [x] T024 [US3] Implement `networkService.getNetworkInterfaces` and `getDNS` in `cross-compile/www/src/services/networkService.ts`
- [x] T025 [US3] Implement `networkService.setNetworkInterfaces` (IP/DHCP) in `cross-compile/www/src/services/networkService.ts`
- [x] T026 [US3] Create Zod schema for Network Config (IP validation) in `cross-compile/www/src/lib/schemas/network.ts`
- [x] T027 [US3] Create Network Settings Page with DHCP toggle logic in `cross-compile/www/src/pages/settings/NetworkPage.tsx`
- [x] T028 [US3] Add "Connectivity Risk" warning dialog before saving network changes

## Phase 6: User Story 4 - Time Settings (P4)

*Goal: Configure NTP vs Manual Time.*

- [x] T029 [US4] Implement `timeService.getSystemDateAndTime` and `setSystemDateAndTime` in `cross-compile/www/src/services/timeService.ts`
- [x] T030 [US4] Create Time Settings Page (Manual Date picker vs NTP Mode) in `cross-compile/www/src/pages/settings/TimePage.tsx`

## Phase 7: User Story 5 - User Management (P5)

*Goal: Manage users and passwords.*

- [x] T031 [US5] Implement `userService.getUsers`, `createUsers`, `deleteUsers`, `setUser` in `cross-compile/www/src/services/userService.ts`
- [x] T032 [US5] Create User List Table component in `cross-compile/www/src/pages/settings/UserManagementPage.tsx`
- [x] T033 [US5] Create "Add User" and "Change Password" dialogs in `cross-compile/www/src/components/users/UserDialogs.tsx`

## Phase 8: User Story 6 - Imaging Settings (P6)

*Goal: Adjust Brightness, Contrast, etc.*

- [x] T034 [US6] Implement `imagingService.getImagingSettings` and `setImagingSettings` in `cross-compile/www/src/services/imagingService.ts`
- [x] T035 [US6] Create Imaging Sliders component (Brightness, Contrast, Saturation) in `cross-compile/www/src/pages/settings/ImagingPage.tsx`

## Phase 9: User Story 7 - Maintenance (P7)

*Goal: Reboot, Reset, Backup.*

- [x] T036 [US7] Implement `deviceService.systemReboot` and `factoryDefault` in `cross-compile/www/src/services/maintenanceService.ts`
- [x] T037 [US7] Create Maintenance Page with "Danger Zone" actions in `cross-compile/www/src/pages/settings/MaintenancePage.tsx`

## Phase 10: User Story 8 - Profiles (P8)

*Goal: Manage Media Profiles.*

- [x] T038 [US8] Implement `profileService.getProfiles`, `createProfile`, `deleteProfile` in `cross-compile/www/src/services/profileService.ts`
- [x] T039 [US8] Create Profile List and Edit View in `cross-compile/www/src/pages/settings/ProfilesPage.tsx`

## Phase 11: Placeholders (US9 & US10)

*Goal: Show Diagnostics and Live View placeholders.*

- [x] T040 [US9] Create Diagnostics Page with mock charts/logs in `cross-compile/www/src/pages/DiagnosticsPage.tsx`
- [x] T041 [US10] Create Live View Page with "Stream Unavailable" placeholder and disabled PTZ controls in `cross-compile/www/src/pages/LiveViewPage.tsx`

## Phase 12: Polish & Optimization

- [x] T042 Ensure all routes are lazy-loaded for code splitting
- [x] T043 Audit ARIA labels and keyboard navigation
- [x] T044 Verify production build size and optimizations
