# Architecture Decisions: Frontend MVP

**Date**: December 14, 2025
**Source**: Architecture Planning Conversation
**Status**: Approved

## Executive Summary

This document summarizes the architectural decisions made during the planning phase for the ONVIF Frontend MVP. The focus is on a robust, accessible, and standards-compliant implementation that integrates with the existing `onvif-rust` backend.

## 1. Core Architecture

### Communication Layer

* **Protocol**: SOAP 1.2 over HTTP.
* **Client Implementation**: Custom lightweight TypeScript library.
  * **Libraries**: `axios` (transport), `fast-xml-parser` (XML handling).
  * **Reasoning**: Existing Node.js ONVIF libraries are not browser-compatible. A custom client decoupling the UI from SOAP logic allows for easier maintenance and testing.
  * **Location**: `src/services/onvifClient.ts` (and related service files).

### Authentication Strategy

* **Primary Mechanism**: **HTTP Basic Auth**.
  * **Header**: `Authorization: Basic <base64(user:password)>` sent with every request.
  * **Reasoning**: The backend `dispatcher.rs` supports `verify_basic_auth` before WS-Security. This simplifies the client significantly compared to generating `UsernameToken` digests.
* **Fallback**: WS-Security `UsernameToken` (SHA1 digest) logic will be kept as a fallback if needed but not used primarily.
* **State**: Credentials stored in Redux (memory only) for the session duration. Cleared on refresh/close.

### State Management

* **Global State**: **Redux Toolkit**.
  * Auth state, Device info, UI global state (sidebar, theme).
* **Form State**: **React Hook Form**.
  * Local form state management.
* **Validation**: **Zod**.
  * Schema-based validation for all forms (IPs, ranges, required fields).

## 2. UI & UX Architecture

### Component System

* **Library**: **@shadcn/ui** (New York style).
* **Theme**: Custom "Camera.UI" Dark Theme.
  * Overrides in `tailwind.config.js` (`--background: #0d0d0d`, `--primary: #ff3b30`).
* **Key Components to Initialize**:
  * `Button`, `Input`, `Select`, `Card`, `Dialog` (Modals).
  * `Sheet` (Mobile Sidebar), `Collapsible` (Mobile Menu).
  * `Form` (React Hook Form wrapper), `Toast` (Notifications).

### Layout & Navigation

* **Wrapper**: `Layout.tsx` (formerly `Frame.tsx`) handles the shell.
* **Desktop**:
  * Fixed Icon Sidebar (Left).
  * Context-aware Secondary Sidebar (Settings Categories).
* **Mobile**:
  * Hamburger Menu → **Shadcn Sheet** (Drawer).
  * Settings Category Selector → **Shadcn Select/Collapsible**.
* **Routing**: `react-router-dom` with nested routes (e.g., `/settings/network`).

### Responsive Strategy

* Refactor current brittle `div`-based mobile nav to use semantic Shadcn components.
* Ensure ARIA attributes (`aria-expanded`, `aria-controls`) are handled by the components.

## 3. Development Workflow

### Mocking

* **Strategy**: Local **Node.js Mock Server**.
* **Implementation**: A lightweight script (`mock-server.js`) using `fastify` or `express`.
* **Function**: Serves static XML responses matching `onvif-rust` contracts.
* **Usage**: Enabled via `VITE_USE_MOCK=true` proxying in `vite.config.ts`.

### Licensing & About

* **Content**: "About" modal will display mapped device info.
  * `HardwareId` → "MAC Address".
  * `FirmwareVersion` → "Firmware v...".
* **License**: GPL-3 license text will be embedded and displayed.

## 4. Unresolved / Watchlist

* **Time Sync**: With Basic Auth, strict time synchronization for `UsernameToken` nonce validation is no longer a blocker for bootstrapping.
* **CORS**: Rust backend configuration needs `dev_mode` or similar to allow `localhost` origin during development (or use the Vite proxy to bypass CORS).
