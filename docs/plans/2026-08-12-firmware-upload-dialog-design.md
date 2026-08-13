# Firmware Upload Dialog — Design

Date: 2026-08-12
Scope: WebUI on PR #74 (`PUT /api/update` already exists). No backend changes.
Parent: `docs/plans/2026-08-12-firmware-upgrade-path-design.md`

## Problem

Diagnostics ships a bare Choose/Upload card: no progress for a ~19 MB bundle,
no confirm that the camera will reboot (and may auto-rollback), and no signal
when the device comes back whether the new slot committed or reverted.

## Decisions

| Decision | Choice |
|---|---|
| Entry | Slim Diagnostics card → Dialog |
| Confirm | `AlertDialog` before PUT (not a wizard step) |
| Progress | XHR upload + native `<progress>` (no new deps, no shadcn Progress) |
| Drop zone | Native drag-drop + hidden file input |
| After 202 | Poll `GET /api/diagnostics` every 2 s, ~5 min cap |
| Outcome | Compare pre-upload `firmware_version` vs post-reconnect |
| Location | Stay on Diagnostics (Maintenance move = later) |

## Flow

```
Upgrade… → Select file → AlertDialog confirm → Uploading (%)
  → Waiting (poll) → Result (committed | probably reverted | unreachable)
```

1. Open dialog: remember `firmware_version` from the page’s diagnostics data.
2. Select: `.tar` only; disable Continue if empty, non-`.tar`, or `>64 MB`.
3. Confirm copy: reboot ~2 min; auto-rollback if services don’t bind.
4. Upload: `uploadFirmware(file, { onProgress, signal })` via XHR, same auth
   headers / 401 path as `authorizedFetch`.
5. On 202: poll diagnostics until success or timeout.
6. Result: version changed → committed; same → probably reverted; timeout →
   unreachable. Close → refetch diagnostics.

## Errors

- Upload fails → message on upload step; retry allowed.
- Abort → back to select.
- Poll timeout → “Camera still unreachable. Refresh later.”
- Non-202 statuses surface as `ApiError` text (no special-case matrix).

## Components

- `FirmwareUpdateCard` — title + Upgrade… only (version already on DeviceInfo).
- `FirmwareUpgradeDialog` — select / uploading / waiting-result; confirm via
  existing `AlertDialog`.

## Out of scope

- Moving entry to Settings → Maintenance.
- Client-side checksum / manifest parse.
- Upload libraries (axios, Uppy, react-dropzone).
- Backend or trial-logic changes.

## Tests

Vitest + `data-testid`, mock upload/diagnostics:

1. Opens from Upgrade…
2. Continue disabled without valid file
3. Confirm before PUT
4. Progress callback updates UI
5. 202 → waiting
6. New version → committed copy
7. Same version → reverted copy
8. Upload error → shown, retryable

## Ponytail notes

- `// ponytail: native <progress>; add shadcn Progress only if another screen needs the styled primitive.`
- `// ponytail: version-string equality for outcome; richer trial status API if false “reverted” reports show up.`
