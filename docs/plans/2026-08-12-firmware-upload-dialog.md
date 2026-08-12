# Firmware Upload Dialog Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the bare Diagnostics Choose/Upload card with a Dialog wizard: confirm → XHR progress → poll reconnect → version outcome.

**Architecture:** Slim card opens `FirmwareUpgradeDialog`. Upload via `authorizedXhrPut` in `api.ts` (same auth + 401 as `authorizedFetch`, no 10 s timeout). After 202, poll `getDiagnostics` every 2 s for ~5 min; compare `firmware_version`.

**Tech Stack:** React 19, Vitest, Dialog + AlertDialog, native `<progress>` + DnD, no new deps.

**Design:** `docs/plans/2026-08-12-firmware-upload-dialog-design.md`

---

### Task 1: `authorizedXhrPut`

**Files:**
- Modify: `cross-compile/www/src/services/api.ts`
- Modify/create: `cross-compile/www/src/services/api.test.ts`

**Step 1: Failing tests** (mock `XMLHttpRequest`)

- 202 → resolves `{ status, bodyText }`
- non-202 → still resolves status (caller throws) *or* document that helper only transports — prefer resolve always, `uploadFirmware` throws
- progress callback fires
- signal abort rejects
- 401 → clear `onvif_camera_auth` + `auth:unauthorized` event

**Step 2: Implement**

```ts
export type UploadProgress = { loaded: number; total: number };

export function authorizedXhrPut(
  url: string,
  body: Blob,
  options: {
    onProgress?: (p: UploadProgress) => void;
    signal?: AbortSignal;
  } = {},
): Promise<{ status: number; bodyText: string }>
```

No default timeout. Auth injection + 401 handling match `authorizedFetch`.

**Step 3: Green → commit** `feat(www): add authorizedXhrPut for progress uploads`

---

### Task 2: `uploadFirmware` uses XHR

**Files:**
- Modify: `cross-compile/www/src/services/diagnosticsService.ts`
- Modify: `cross-compile/www/src/services/diagnosticsService.test.ts`

**New signature** (break the old `signal?` second arg — only Diagnostics calls it):

```ts
export async function uploadFirmware(
  file: File,
  options?: {
    onProgress?: (p: UploadProgress) => void;
    signal?: AbortSignal;
  },
): Promise<void>
```

Thin wrapper: `authorizedXhrPut('/api/update', file, options)`; 202 ok; else `ApiError`.

Mock `authorizedXhrPut` in service tests.

**Commit:** `feat(www): upload firmware via XHR with progress`

---

### Task 3: `FirmwareUpgradeDialog`

**Files:**
- Create: `cross-compile/www/src/components/FirmwareUpgradeDialog.tsx`
- Create: `cross-compile/www/src/components/FirmwareUpgradeDialog.test.tsx`

**Props:** `open`, `onOpenChange`, `previousVersion: string | null`, `onFinished`.

**Flow:** select → AlertDialog confirm → uploading (`<progress>`) → waiting (poll) → result.

**data-testid** (prefix `firmware-upgrade-`): `dialog`, `input`, `continue-button`, `confirm-button`, `progress`, `waiting`, `result-message`, `error`, `close-button`.

**Rules:** `.tar` only; block empty / non-`.tar` / `>64MB`. Poll every 2 s, ~5 min cap. Outcome = string compare of `firmware_version`. Abort → select. Upload error → show `error`, stay put.

Fake timers for poll. Mock `uploadFirmware` + `getDiagnostics`.

**Tests:** design items 2–8.

**Commit:** `feat(www): add FirmwareUpgradeDialog wizard`

---

### Task 4: Wire Diagnostics card

**Files:**
- Modify: `cross-compile/www/src/pages/DiagnosticsPage.tsx` (`FirmwareUpdateCard`)
- Modify: `cross-compile/www/src/pages/DiagnosticsPage.test.tsx`

Card = title + Upgrade… (`diagnostics-firmware-upgrade-button`). Pass version + refetch. Delete inline picker/upload/queued UI.

**Page tests:** button present; click opens `firmware-upgrade-dialog`. Wizard details stay in Task 3.

**Commit:** `feat(www): open firmware upgrade dialog from diagnostics`

---

### Task 5: Quality gates

```bash
cd cross-compile/www && npm run lint && npm run type-check && npm run test
```

---

### Ponytail ceilings

- `// ponytail: native <progress>; shadcn Progress only if another screen needs it.`
- `// ponytail: version-string equality for outcome; trial-status API if false reverted reports appear.`
- `// ponytail: no upload timeout; add a ceiling if hung PUTs become a support issue.`

### Out of scope

Maintenance move, client checksums, upload libraries, backend changes.
