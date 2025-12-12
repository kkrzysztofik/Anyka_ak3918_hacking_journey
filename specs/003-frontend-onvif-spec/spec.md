# Feature Specification: Frontend ↔ Existing ONVIF Services

**Feature Branch**: `003-frontend-onvif-spec`
**Created**: Friday Dec 12, 2025
**Status**: Draft
**Input**: User description: "Let's prepare a specification for frontend to onvif-rust project. Base it on @.ai/design_proposal.md @.ai/design @cross-compile/www and @.cursor/rules/frontend.mdc @.cursor/rules/react.mdc @.cursor/rules/shadcn.mdc and onvif-rust API"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Project Bootstrap (Priority: P1)

As an operator, I want to sign in and complete the initial “bootstrap” so the admin UI is connected to the device backend and I can proceed to the rest of the app confidently.

**Why this priority**: This is the foundation for all other workflows (a working session + a confirmed connection to the backend).

**Independent Test**: Can be fully tested by signing in, confirming the backend is reachable, and verifying the UI routes are usable and unsupported areas are clearly indicated.

**Acceptance Scenarios**:

1. **Given** the user is signed out, **When** they provide valid credentials, **Then** they are signed in and taken to the app’s initial landing page.
2. **Given** the app is signed in, **When** the backend is unreachable, **Then** the UI shows a clear “cannot connect” state and suggested remediation (e.g., confirm address, network, power) without trapping the user.
3. **Given** the app is signed in, **When** the backend becomes reachable, **Then** the UI confirms connectivity and the user can navigate to Settings, Diagnostics, and other supported areas.
4. **Given** the app is signed in, **When** the user navigates, **Then** features that are not available in this phase (PTZ control, live streaming, telemetry) are clearly marked as unavailable and do not present broken UI.

---

### User Story 2 - Identification Settings & About (Priority: P2)

As an administrator, I want to view device identity and status in the Identification settings, update identification fields (device name, location, hostname), and open an About view so I can configure the device for its installation and quickly retrieve key details for support.

**Why this priority**: Correct identity reduces operational confusion (installations, labels, support calls) and is a core part of the Settings design.

**Independent Test**: Can be fully tested by updating name/location/hostname, saving, and confirming the updated values persist and remain visible after refresh.

**Acceptance Scenarios**:

1. **Given** the backend is reachable, **When** the administrator views device information, **Then** manufacturer/model/firmware/serial/hardware id are displayed.
2. **Given** the administrator opens Identification settings, **When** the content loads, **Then** the UI shows the “Device Status” section (device name, model, and online indicator).
3. **Given** the administrator opens the About view, **When** the content loads, **Then** the UI shows the device identity summary and network identifiers that are available (e.g., MAC address).
4. **Given** the administrator edits the device name and location, **When** they press “Save Changes”, **Then** the system applies the changes and the UI shows a success confirmation.
5. **Given** the administrator edits the hostname, **When** they press “Save Changes”, **Then** the system applies the change and the updated hostname is shown after refresh.
6. **Given** a field is read-only (hardware identifiers), **When** the administrator views settings, **Then** those fields are clearly marked non-editable.

---

### User Story 3 - Network Settings (Priority: P3)

As an administrator, I want to view and update network settings (interfaces, addressing mode, DNS, and service ports) so I can place the device correctly on the network and ensure expected connectivity.

**Why this priority**: Network configuration is necessary for deployment and ongoing reachability and maps directly to the Settings “Network” view.

**Independent Test**: Can be fully tested by viewing current network values, applying a change (where supported), and verifying the UI reflects the updated values with clear warnings about connectivity risk.

**Acceptance Scenarios**:

1. **Given** the administrator opens Network settings, **When** the content loads, **Then** the UI shows current network interface details (IP address, addressing mode, MAC, and gateway where available).
2. **Given** the administrator updates DNS settings, **When** they save, **Then** the system applies the change and the UI confirms success.
3. **Given** the administrator updates service connectivity settings (e.g., ports for relevant services), **When** they save, **Then** the system applies the change and the UI confirms success.
4. **Given** the administrator attempts a network change that could disconnect the session, **When** they save, **Then** the UI clearly warns about the risk and requires explicit confirmation.
5. **Given** the user has insufficient permissions, **When** they attempt to save network changes, **Then** the system blocks the action and explains why.

---

### User Story 4 - Time Settings (Priority: P4)

As an administrator, I want to view and update time settings (manual time vs network time, timezone, and time servers) so device timestamps remain accurate.

**Why this priority**: Time correctness is essential for operational debugging and interoperability.

**Independent Test**: Can be fully tested by viewing time settings, changing the configuration, saving, and verifying the UI reflects the applied mode and values after refresh.

**Acceptance Scenarios**:

1. **Given** the administrator opens Time settings, **When** the content loads, **Then** the UI shows current time configuration and the current timezone.
2. **Given** the administrator selects network time and specifies time servers, **When** they save, **Then** the system applies the change and confirms success.
3. **Given** the administrator selects manual time, **When** they set a date/time and save, **Then** the system applies the change and confirms success.

---

### User Story 5 - User Management & Password Change (Priority: P5)

As an administrator, I want to manage user accounts (create, update role, disable, delete) and allow signed-in users to change their own password so access is controlled appropriately.

**Why this priority**: Proper user management is required for security and operational handoffs.

**Independent Test**: Can be fully tested by creating a new user, changing their role, disabling them, and confirming the changes take effect for sign-in.

**Acceptance Scenarios**:

1. **Given** the administrator opens User Management, **When** the content loads, **Then** the UI shows the existing users and their roles/status.
2. **Given** the administrator creates a user, **When** they save, **Then** the user exists and can sign in according to the assigned role.
3. **Given** the administrator updates a user role or password, **When** they save, **Then** the changes take effect immediately and are confirmed by the UI.
4. **Given** the administrator deletes or disables a user, **When** they confirm, **Then** the user can no longer sign in and the UI reflects the change.
5. **Given** a signed-in user changes their password, **When** they confirm the change, **Then** the new password works and the old password no longer works.

---

### User Story 6 - Imaging Settings (Priority: P6)

As an administrator, I want to view and update basic imaging settings so I can maintain consistent image quality for the installation.

**Why this priority**: Imaging configuration is a common operational need, but it is secondary to deployment-critical settings (identity/network/time).

**Independent Test**: Can be fully tested by updating an imaging setting, saving, and observing the updated values persist and remain visible after refresh.

**Acceptance Scenarios**:

1. **Given** the user is signed in with administrative permissions, **When** they update an imaging setting and press “Save”, **Then** the system applies the change and the UI shows a success confirmation.
2. **Given** the device exposes valid ranges/options for imaging settings, **When** the user views the form, **Then** inputs are constrained to valid values and show helper text (min/max).
3. **Given** the device does not support a specific imaging setting, **When** the user views imaging settings, **Then** unsupported fields are hidden or disabled with an explanation.

---

### User Story 7 - Maintenance Settings (Priority: P7)

As an administrator, I want to perform maintenance operations (reboot, reset to defaults, backup/restore configuration) so I can recover from misconfiguration or apply changes safely.

**Why this priority**: Maintenance is less frequent but critical for recovery and field support.

**Independent Test**: Can be fully tested by triggering a backup, restoring from a backup, and verifying device settings reflect the restore; reboot/reset require controlled test environment.

**Acceptance Scenarios**:

1. **Given** the administrator opens Maintenance settings, **When** the content loads, **Then** the UI shows available maintenance actions and explains their impact.
2. **Given** the administrator initiates a reboot, **When** they confirm, **Then** the system acknowledges the request and the UI shows a reconnection flow.
3. **Given** the administrator initiates a reset to defaults, **When** they confirm, **Then** the system acknowledges the request and the UI clearly warns about irreversible impact.
4. **Given** the administrator initiates a configuration backup, **When** it completes, **Then** the backup can be downloaded or otherwise retrieved.
5. **Given** the administrator restores configuration from a backup, **When** they confirm, **Then** the system restores configuration and the UI shows completion status.

---

### User Story 8 - Profiles (Create/Edit/Delete) (Priority: P8)

As an administrator, I want to create, edit, and delete configuration profiles so I can manage available stream configurations and formats, even if live streaming is not yet enabled in this phase.

**Why this priority**: It’s useful for diagnostics and future readiness, but not required for core configuration.

**Independent Test**: Can be fully tested by creating a profile, editing it, and deleting it, then confirming the profile list updates accordingly.

**Acceptance Scenarios**:

1. **Given** the administrator opens Profiles, **When** the content loads, **Then** the UI lists the available profiles and their configuration summary.
2. **Given** the administrator creates a profile, **When** they save, **Then** the new profile appears in the list with its summary.
3. **Given** the administrator edits a profile configuration, **When** they save, **Then** the updated configuration is reflected after refresh.
4. **Given** the administrator deletes a profile, **When** they confirm, **Then** it is removed from the list and is no longer retrievable.

---

### Edge Cases

- Camera backend intermittently reachable (flapping network): UI remains usable, shows last-known values with “stale” indicator, and retries safely.
- Partial capability support: UI hides/disabled unsupported controls rather than failing the entire page.
- Slow responses: UI shows progress and avoids duplicate actions (no double-save, no command spam).
- Invalid user input: UI prevents or clearly rejects invalid values with actionable messages.
- Concurrent edits: if two sessions change settings, the UI warns about conflicts and shows which values are currently applied.
- Authentication expiry: if the session expires, the user is returned to sign-in without data loss where possible.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST provide a sign-in flow that gates access to device controls and settings.
- **FR-002**: System MUST provide an “About” view that summarizes device identity and key identifiers available from the device.
- **FR-003**: System MUST provide an “Identification” settings view that allows updating device name, location, and hostname where supported, while clearly marking read-only identifiers.
- **FR-004**: System MUST provide a “Network” settings view that displays current interface configuration (IP, addressing mode, MAC, gateway, DNS) and allows administrators to update supported network settings.
- **FR-005**: System MUST provide a “Time” settings view that displays current time configuration and allows administrators to switch between manual time and network time (when supported) and configure relevant time settings.
- **FR-006**: System MUST provide an “Imaging” settings view that displays current imaging settings and allows administrators to update supported imaging controls with validation against allowed ranges/options.
- **FR-007**: System MUST provide a “User Management” view that allows administrators to view and manage user accounts (create, update role, enable/disable, delete) consistent with device permissions.
- **FR-008**: System MUST provide a “Maintenance” view that offers safe maintenance actions (at minimum reboot, reset to defaults, backup/restore configuration) with clear confirmation prompts and status feedback.
- **FR-009**: System MUST provide a “Profiles” view that supports profile lifecycle management (create, edit, delete) and shows a configuration summary for each profile.
- **FR-010**: System MUST clearly indicate which features are not yet available in this phase (PTZ control, live streaming, telemetry), without presenting broken UI.
- **FR-011**: System MUST follow the agreed visual theme (“Camera.UI” dark theme with red accents) and remain accessible (keyboard navigation and appropriate ARIA).
- **FR-012**: System MUST handle backend/network failures gracefully with clear user messaging and without crashing or losing navigation.

### Key Entities *(include if feature involves data)*

- **Device**: Represents the single camera device being managed (identity, network address, capabilities, health state).
- **User**: Represents an authenticated person interacting with the admin panel (role/permission level, status enabled/disabled).
- **Session**: Represents a signed-in session and its validity (expiry, sign-out).
- **Identification Settings**: Represents editable identity attributes (device name, location, hostname) and read-only identifiers.
- **Network Settings**: Represents interface configuration, DNS, gateway, and service connectivity settings.
- **Time Settings**: Represents time mode (manual/network time), timezone, and time server configuration.
- **Maintenance Operation**: Represents a maintenance action request and its outcome (e.g., reboot acknowledged, backup created, restore completed).
- **Imaging Settings**: Represents configurable image parameters (e.g., brightness/contrast/saturation).
- **Configuration Change**: Represents a setting update request, outcome, and whether a restart is required.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A signed-in user can complete bootstrap (reach the app landing page and see a clear connected/not-connected state) within 10 seconds on a typical local network.
- **SC-002**: A signed-in user can determine supported vs unsupported feature areas (e.g., imaging available; PTZ/live streaming/telemetry not yet) within 30 seconds without documentation.
- **SC-003**: An administrator can complete a user-management task (create, update role, disable, or delete a user) in under 2 minutes under normal conditions.
- **SC-004**: An administrator can complete one configuration task in each of: Identification, Network, and Time settings in under 5 minutes total under normal conditions.
- **SC-005**: Configuration edits (where supported and permitted) persist correctly: after saving and reloading the page, the updated values remain visible in 100% of tested cases.

## Assumptions & Scope Boundaries

- Single camera only (no device list / multi-device switching).
- Primary usage is on a trusted local network; security still required (sign-in, least privilege, safe maintenance confirmations).
- The backend may expose optional capabilities (e.g., imaging); UI adapts to capability presence rather than forcing all features.
- PTZ control, RTSP streaming, and device telemetry are explicitly out of scope for this phase and will be specified later when implemented.
