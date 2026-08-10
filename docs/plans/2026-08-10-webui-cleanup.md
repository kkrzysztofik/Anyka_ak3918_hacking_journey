# WebUI Cleanup Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove ~3450 lines of dead code and duplicate abstractions from `cross-compile/www` so the Diagnostics feature is built on one card family, one error path, and one set of types.

**Architecture:** Four independently revertable PRs ordered by blast radius — pure deletions, behavior-preserving dedup, the credential-storage change, then design-system consolidation. Design doc: `docs/plans/2026-08-10-webui-cleanup-design.md`.

**Tech Stack:** React 19, TypeScript (dual TS 6 / TS 7 type-check), Vite 8, Vitest 4, Tailwind v4, shadcn/ui, TanStack Query.

---

## How TDD applies here

This is deletion and refactor work, so classic red-green does not apply to most tasks. The discipline is adapted, not skipped:

- **Deletion tasks** — the gate runs before and after. Coverage must not drop; if it does, something live was deleted. No new tests are written.
- **Refactor tasks (PR2)** — the existing suite is the red-green. It must pass **unmodified**. A test that needs editing means the refactor changed behavior, which is a defect in the refactor.
- **Task 1 only** — `find-dead-exports.mjs` is genuinely new logic and gets real TDD. The repo already tests its build scripts (`scripts/analyze-bundle.test.mjs`, `scripts/precompress.test.mjs`) and `vite.config.ts` includes `scripts/**/*.{test,spec}.mjs` in the Vitest run, so it fits the existing pattern.

## The gate

Every task ends with this passing. `type-check` runs both compilers; do not skip the TS 6 pass.

```bash
cd cross-compile/www && npm run lint && npm run type-check && npm run test
```

Record baseline coverage before starting PR1:

```bash
cd cross-compile/www && npm run test:coverage 2>&1 | tail -20
```

Write the total line-coverage percentage into the PR description. Every PR1 task must leave it equal or higher.

## Branch

Work continues on `chore/webui-cleanup`, already created and holding the design doc. Open PR1 from it; branch PR2, PR-auth, and PR3 from `main` after their predecessor merges.

---

# PR1 — `chore(webui): remove dead code`

Target: ~-2190 lines, -4 deps. One commit per task so review can go finding-by-finding.

### Task 1: Land the dead-export scanner

New logic, so this one is real TDD.

**Files:**
- Create: `cross-compile/www/scripts/find-dead-exports.mjs`
- Test: `cross-compile/www/scripts/find-dead-exports.test.mjs`

**Step 1: Write the failing test**

```javascript
import { describe, expect, it } from 'vitest';
import { findDeadExports } from './find-dead-exports.mjs';

describe('findDeadExports', () => {
  it('reports an export no other file references', () => {
    const files = new Map([
      ['./a.ts', 'export function used() {}\nexport function orphan() {}'],
      ['./b.ts', "import { used } from './a';\nused();"],
    ]);
    expect(findDeadExports(files)).toEqual([{ file: './a.ts', symbol: 'orphan' }]);
  });

  it('ignores references from the defining file itself', () => {
    const files = new Map([['./a.ts', 'export function solo() {}\nsolo();']]);
    expect(findDeadExports(files)).toEqual([{ file: './a.ts', symbol: 'solo' }]);
  });

  it('treats a type-only export as referenced when another file imports it', () => {
    const files = new Map([
      ['./t.ts', 'export interface Shape { x: number }'],
      ['./u.ts', "import type { Shape } from './t';"],
    ]);
    expect(findDeadExports(files)).toEqual([]);
  });
});
```

**Step 2: Run it and confirm it fails**

```bash
cd cross-compile/www && npx vitest run scripts/find-dead-exports.test.mjs
```

Expected: FAIL — `Failed to resolve import "./find-dead-exports.mjs"`.

**Step 3: Write the implementation**

```javascript
import { globSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const EXPORT_RE =
  /^export\s+(?:async\s+)?(?:function|const|let|class|interface|type|enum)\s+([A-Za-z_$][\w$]*)/gm;

/**
 * @param {Map<string, string>} files path -> source text
 * @returns {Array<{file: string, symbol: string}>}
 */
export function findDeadExports(files) {
  const dead = [];
  for (const [file, text] of files) {
    for (const match of text.matchAll(EXPORT_RE)) {
      const symbol = match[1];
      const word = new RegExp(`\\b${symbol}\\b`);
      let used = false;
      for (const [other, otherText] of files) {
        if (other === file) continue;
        if (word.test(otherText)) {
          used = true;
          break;
        }
      }
      if (!used) dead.push({ file, symbol });
    }
  }
  return dead;
}

export function readSourceFiles(root, { includeTests }) {
  const listed = globSync('**/*.{ts,tsx}', { cwd: root });
  const wanted = includeTests
    ? listed
    : listed.filter((f) => !/\.test\./.test(f) && !f.startsWith('test/'));
  return new Map(wanted.map((f) => [f, readFileSync(resolve(root, f), 'utf8')]));
}

if (import.meta.url === `file://${process.argv[1]}`) {
  // --prod-only ignores tests, surfacing exports kept alive only by their own test file.
  const includeTests = !process.argv.includes('--prod-only');
  const files = readSourceFiles(resolve(import.meta.dirname, '../src'), { includeTests });
  for (const { file, symbol } of findDeadExports(files)) {
    console.log(`${file}\t${symbol}`);
  }
}
```

**Step 4: Run it and confirm it passes**

```bash
cd cross-compile/www && npx vitest run scripts/find-dead-exports.test.mjs
```

Expected: PASS, 3 tests.

**Step 5: Capture the baseline inventory**

```bash
cd cross-compile/www && node scripts/find-dead-exports.mjs > /tmp/dead-before.txt
node scripts/find-dead-exports.mjs --prod-only > /tmp/dead-prod-before.txt
wc -l /tmp/dead-before.txt /tmp/dead-prod-before.txt
```

Expected: roughly 69 and 18 lines. Keep both files — later tasks diff against them.

**Step 6: Commit**

```bash
rtk git add scripts/find-dead-exports.mjs scripts/find-dead-exports.test.mjs
rtk git commit -m "test(webui): add a dead-export scanner for the cleanup"
```

> Two deliberate corners. The regex matches identifiers textually, so a symbol sharing a name with an unrelated local reads as "used" — that direction is safe, it under-reports rather than deleting something live. And `fs.globSync` replaces shelling out to `find`: stdlib, no `child_process`, no shell-injection surface. A real resolver is not worth it for a one-off sweep.

---

### Task 2: Delete `UserDialogs`

Largest single cut. `UserManagementPage` builds its dialogs inline; Layout imports the other `ChangePasswordDialog` from `@/components/ChangePasswordDialog`.

**Files:**
- Delete: `cross-compile/www/src/components/users/UserDialogs.tsx`
- Delete: `cross-compile/www/src/components/users/UserDialogs.test.tsx`

**Step 1: Confirm zero production importers**

```bash
cd cross-compile/www/src && grep -rn "UserDialogs" --include='*.tsx' . | grep -v '\.test\.'
```

Expected: no output. If anything prints, stop and re-scope.

**Step 2: Delete**

```bash
cd /home/kmk/dev/anyka-dev && rtk git rm cross-compile/www/src/components/users/UserDialogs.tsx cross-compile/www/src/components/users/UserDialogs.test.tsx
rmdir cross-compile/www/src/components/users 2>/dev/null || true
```

**Step 3: Run the gate**

```bash
cd cross-compile/www && npm run lint && npm run type-check && npm run test
```

Expected: all pass, test count drops by the `UserDialogs.test.tsx` cases.

**Step 4: Commit**

```bash
rtk git commit -m "chore(webui): delete UserDialogs, which had no production importers"
```

---

### Task 3: Delete `cameraConfig`

Six exports, zero callers. The WebUI is served from the camera; `location.host` is the address.

**Files:**
- Delete: `cross-compile/www/src/config/cameraConfig.ts`
- Delete: `cross-compile/www/src/config/cameraConfig.test.ts`

**Step 1: Confirm no production importer**

```bash
cd cross-compile/www/src && grep -rn "cameraConfig" --include='*.ts' --include='*.tsx' . | grep -v 'config/cameraConfig'
```

Expected: no output.

**Step 2: Delete and run the gate**

```bash
cd /home/kmk/dev/anyka-dev && rtk git rm cross-compile/www/src/config/cameraConfig.ts cross-compile/www/src/config/cameraConfig.test.ts
rmdir cross-compile/www/src/config 2>/dev/null || true
cd cross-compile/www && npm run lint && npm run type-check && npm run test
```

**Step 3: Commit**

```bash
rtk git commit -m "chore(webui): delete cameraConfig, unreferenced since the UI is same-origin"
```

---

### Task 4: Delete `SystemInfo` and the `.card` class

`SystemInfo` has no importers, uses `React.FC`, and hardcodes ports 554/3000/8080 when the device serves everything on :80 same-origin. `.card` is its only consumer.

**Files:**
- Delete: `cross-compile/www/src/components/SystemInfo.tsx`
- Delete: `cross-compile/www/src/components/SystemInfo.test.tsx`
- Modify: `cross-compile/www/src/styles/globals.css` — remove the `.card` rule (~line 109)

**Step 1: Confirm both are orphaned**

```bash
cd cross-compile/www/src && grep -rn "SystemInfo" --include='*.tsx' . | grep -v 'SystemInfo.tsx\|SystemInfo.test\|types/index'
grep -rn 'className="card"' --include='*.tsx' .
```

Expected: no output from either after `SystemInfo` is accounted for.

**Step 2: Delete the component, remove the `.card` rule, run the gate, commit**

```bash
cd /home/kmk/dev/anyka-dev && rtk git rm cross-compile/www/src/components/SystemInfo.tsx cross-compile/www/src/components/SystemInfo.test.tsx
# then edit globals.css to drop the .card block
cd cross-compile/www && npm run lint && npm run type-check && npm run test
rtk git commit -am "chore(webui): delete SystemInfo and its now-orphaned .card utility"
```

---

### Task 5: Prune the dead test helpers

`test/schemaTestHelpers.ts` and `test/serviceTestHelpers.ts` are wholly unreferenced. Another ~20 exports across the remaining helpers have no callers.

**Files:**
- Delete: `cross-compile/www/src/test/schemaTestHelpers.ts`
- Delete: `cross-compile/www/src/test/serviceTestHelpers.ts`
- Modify: `cross-compile/www/src/test/componentTestHelpers.tsx` — drop `createTestQueryClient`, `MOCK_ENDPOINTS`, `renderPageAndWait`, `submitDialog`, `toggleSwitch`, `performUserMenuAction`, `MockDialog`, `MockButton`, `testErrorToast`, `testSuccessToast`
- Modify: `cross-compile/www/src/test/dialogTestHelpers.ts` — drop `testDialogCancelByButton`, `testDialogCancelByButtonWithCallback`, `assertDialogClosed`, `testDialogCancel`, `testDialogCancelWithCallback`, `testDialogLoadingState`, `renderDialogAndWait`
- Modify: `cross-compile/www/src/test/formTestHelpers.ts` — drop `waitForFormValue`, `fillFormFields`, `submitFormAndWait`, `testFormValidation`, `fillAndSubmitForm`
- Modify: `cross-compile/www/src/test/mutationTestHelpers.ts` — drop `testMutationWithLoadingState`, `testMutationWithError`, `testMutationWithNonErrorRejection`
- Modify: `cross-compile/www/src/test/utils.ts` — drop `createMockResponse`, `createTestSOAPResponse`, `createSOAPFaultResponse`, `createSOAPSuccessResponse`, `mockApiClient`

**Step 1: Verify against the scanner before touching anything**

```bash
cd cross-compile/www && node scripts/find-dead-exports.mjs | grep '^test/'
```

Cross-check the printed list against the names above. Delete only what both agree on.

**Step 2: Watch for internal callers**

`createMockResponse` is called by `createMockSOAPResponse` and `createMockSOAPFaultResponse` inside `test/utils.ts`, both of which stay. Inline it or keep it unexported — do not delete the body.

**Step 3: Run the gate and commit**

```bash
cd cross-compile/www && npm run lint && npm run type-check && npm run test
rtk git commit -am "chore(webui): prune unreferenced test helpers"
```

---

### Task 6: Prune `types/index.ts`

Keep exactly `SystemInfo`, `DataPoint`, `SystemUtilizationResponse` — the Diagnostics telemetry contract. `SystemInfoProps` dies with Task 4.

**Files:**
- Modify: `cross-compile/www/src/types/index.ts` — delete `CameraConfig`, `PTZDirection`, `PTZSpeeds`, `ImagingSettings`, `ONVIFStatus`, `ONVIFResponse`, `DeviceInfo`, `SystemCapabilities`, `DeviceStatus`, `Endpoint`, `SidebarProps`, `HeaderProps`, `VideoFeedProps`, `PTZControlsProps`, `SystemInfoProps`, `SOAPFault`, `SOAPRequest`
- Modify: `cross-compile/www/src/services/authService.ts:9` — import `DeviceInfo` from `@/services/deviceService` instead of `@/types`

`DeviceInfo`, `ImagingSettings`, and `PTZDirection` are re-declared and actually used in their own service modules; only the barrel copies go.

**Step 1: Make the edits, run the gate, commit**

```bash
cd cross-compile/www && npm run lint && npm run type-check && npm run test
rtk git commit -am "chore(webui): keep only the telemetry types in the types barrel"
```

Expected: `type-check` is the real check here — a wrongly deleted type fails compilation immediately.

---

### Task 7: Cut the speculative CSS utilities

**Files:**
- Modify: `cross-compile/www/src/styles/globals.css`
- Modify: `cross-compile/www/tailwind.config.js`
- Modify: `.serena/memories/www-design-system.md`

**Delete:** `.bg-noise`, `.bg-gradient-mesh`, `.technical-section`, `.stagger-1` through `.stagger-4`, `.btn-press`, `.ptz-button`, `.sidebar-icon`, `.focus-ring`, `.skeleton-text`, `.skeleton-title`, `.skeleton-avatar`, `.section-divider`, `.list-item-interactive`, `.status-warning`, `.status-error`, `.card-elevated`, plus the `accordion-down` / `accordion-up` keyframes and `animate-spin-slow` in `tailwind.config.js`.

**Keep:** `.technical-panel`, `.technical-data`, `.mono-value`, `.badge-technical`, `.data-highlight`, `.value-transition`, `.empty-state*`, `.skeleton`, `.page-enter`, `.status-connected`, `.status-pulse`, `.live-indicator`, `.status-badge-connected`.

**Step 1: Re-verify each deletion has no call site**

```bash
cd cross-compile/www/src && for c in bg-noise bg-gradient-mesh technical-section stagger-1 stagger-2 stagger-3 stagger-4 btn-press ptz-button sidebar-icon focus-ring skeleton-text skeleton-title skeleton-avatar section-divider list-item-interactive status-warning status-error card-elevated; do
  n=$(grep -rlw "$c" . --include='*.tsx' | wc -l); echo "$n $c";
done
```

Expected: every line reads `0`.

**Step 2: Update the design memory**

In `.serena/memories/www-design-system.md`, the line listing `@layer components` utilities currently reads `.card`, `.technical-panel`, `.status-badge-connected`, `.empty-state`, `.skeleton*`, `.live-indicator`, `.data-highlight`, `.focus-ring`. Rewrite it to match what survives — `.card` and `.focus-ring` are gone, and `.skeleton*` narrows to `.skeleton`.

**Step 3: Run the gate and commit**

```bash
cd cross-compile/www && npm run lint && npm run type-check && npm run test
rtk git commit -am "chore(webui): cut CSS utilities with no call site, sync the design memory"
```

---

### Task 8: Trim the SOAP client

**Files:**
- Modify: `cross-compile/www/src/services/soap/client.ts`
- Modify: `cross-compile/www/src/services/profileService.ts:7,385,464`
- Modify: `cross-compile/www/src/services/soap/client.test.ts`

**Delete from `soapBodies`:** `getNetworkInterfaces`, `getDNS`, `getUsers`, `getProfiles`, `getImagingSettings`, `systemReboot`, `setSystemFactoryDefault`, `getSystemBackup`, `restoreSystem`, `getPTZStatus`. Each duplicates XML the services already inline.

**Also delete:** `escapeXmlAttribute` (a verbatim alias of `escapeXml`; update its two callers in `profileService`), `builderOptions`, the `builder` instance, and the `XMLBuilder` import — nothing has ever called it.

**Step 1: Confirm the ten keys are unused**

```bash
cd cross-compile/www/src && for k in getNetworkInterfaces getDNS getUsers getProfiles getImagingSettings systemReboot setSystemFactoryDefault getSystemBackup restoreSystem getPTZStatus; do
  echo "$(grep -rn "soapBodies\.$k" --include='*.ts' . | grep -v '\.test\.' | wc -l) $k";
done
```

Expected: every line reads `0`.

**Step 2: Delete, fix the two `escapeXmlAttribute` call sites, drop the matching cases in `client.test.ts`, run the gate, commit**

```bash
cd cross-compile/www && npm run lint && npm run type-check && npm run test
rtk git commit -am "chore(webui): drop unused soapBodies builders and the never-used XMLBuilder"
```

---

### Task 9: Delete exports kept alive only by their tests

**Files:**
- Modify: `cross-compile/www/src/services/authService.ts` — delete `checkDeviceReachable`
- Modify: `cross-compile/www/src/services/profileService.ts` — delete `getProfile`, `getVideoEncoderConfigurations`
- Modify: `cross-compile/www/src/services/deviceService.ts` — delete `setDeviceInformation` (a one-line pass-through to `setScopes`; point `IdentificationPage.tsx:41,85` at `setScopes`), and unexport `getScopes` / `setScopes` if nothing outside the module uses them
- Modify: the corresponding `.test.ts` files — delete the cases covering the removed functions

**Step 1: Regenerate the prod-only list**

```bash
cd cross-compile/www && node scripts/find-dead-exports.mjs --prod-only
```

Work only from what this prints. `DEFAULT_HEADERS`, `DEFAULT_TIMEOUT_MS`, and `ApiError` also appear — leave them; they are the public surface of the API client and reasonable to export even at one internal caller.

**Step 2: Delete, run the gate, commit**

```bash
cd cross-compile/www && npm run lint && npm run type-check && npm run test
rtk git commit -am "chore(webui): delete service functions with no production caller"
```

---

### Task 10: Drop four dependencies

**Files:**
- Modify: `cross-compile/www/package.json`
- Modify: `cross-compile/www/postcss.config.js`
- Modify: `cross-compile/www/vite.config.ts:35`

Remove `dompurify` (never imported), `msw` (never imported), `eslint-plugin-tailwindcss` (absent from `eslint.config.js`), and `autoprefixer` (Tailwind v4's `@tailwindcss/postcss` prefixes via Lightning CSS). Drop the `autoprefixer` entry from `postcss.config.js` and the `dompurify` clause from `getChunkName`.

Keep `@fontsource/inter` — the design memory names it as the fallback face, and Task 11 makes it load.

**Step 1: Confirm no imports**

```bash
cd cross-compile/www && grep -rn "dompurify\|DOMPurify\|from 'msw'" src scripts 2>/dev/null
```

Expected: no output.

**Step 2: Remove, reinstall, verify the bundle**

```bash
cd cross-compile/www && npm uninstall dompurify msw eslint-plugin-tailwindcss autoprefixer
npm run analyze
```

Compare against the pre-PR1 `npm run analyze` output — total bundle size must not grow, and no chunk should still name a removed package.

**Step 3: Run the gate and commit**

```bash
npm run lint && npm run type-check && npm run test
rtk git commit -am "chore(webui): drop four dependencies nothing imports"
```

> `npm run build` writes into `SD_card_contents/`. `npm run analyze` builds to a temp dir instead, so it will not dirty the tree.

---

### Task 11: Make the fonts actually load

`src/index.css` holds the IBM Plex `@import`s and is imported by nothing — not `main.tsx`, not `App.tsx`, not `index.html`. The UI has been silently falling back to system fonts.

**Files:**
- Modify: `cross-compile/www/src/index.css` — add `@import '@fontsource/inter';`
- Modify: `cross-compile/www/src/main.tsx` — add `import './index.css';`

**Step 1: Make the edits**

In `main.tsx`, add the import above `import App from './App';`.

**Step 2: Verify visually**

```bash
cd cross-compile/www && npm run dev
```

Load the page and confirm in DevTools that body text computes to IBM Plex Sans, not the system default. This is the one PR1 change with a visible effect.

**Step 3: Run the gate and commit**

```bash
npm run lint && npm run type-check && npm run test
rtk git commit -am "fix(webui): import index.css so the IBM Plex faces actually load"
```

---

### Task 12: Trim the build config

**Files:**
- Modify: `cross-compile/www/vite.config.ts`
- Modify: `cross-compile/www/tsconfig.json:29-33`

In `vite.config.ts`: drop `react-router-dom` from `optimizeDeps.include` (the project uses `react-router` v8), delete the unread `__APP_VERSION__` define, replace `chunkFileNames: () => 'js/[name]-[hash].js'` with the plain string, replace the hand-rolled `assetFileNames` extension parsing with

```typescript
assetFileNames: (info) =>
  info.names?.[0]?.endsWith('.css')
    ? 'css/[name]-[hash][extname]'
    : 'assets/[name]-[hash][extname]',
```

and remove the stale first JSDoc block above `getChunkName`. Keep the `/utilization` proxy — Diagnostics needs it.

In `tsconfig.json`, delete the `@/components/*`, `@/services/*`, `@/hooks/*`, `@/types/*` path entries; `@/*` already covers them.

**Step 1: Verify the build output layout is unchanged**

```bash
cd cross-compile/www && npm run analyze
```

Expected: same `js/`, `css/`, `assets/` structure and comparable chunk sizes.

**Step 2: Run the gate and commit**

```bash
npm run lint && npm run type-check && npm run test
rtk git commit -am "chore(webui): trim vite and tsconfig cruft"
```

> The `paths` change is the one edit most likely to diverge between TS 6 and TS 7. Confirm both passes of `type-check` succeed, not just the first.

---

### Task 13: Close out PR1

**Step 1: Re-run the scanner and confirm the cascade is clean**

```bash
cd cross-compile/www && node scripts/find-dead-exports.mjs > /tmp/dead-after.txt
diff /tmp/dead-before.txt /tmp/dead-after.txt
```

Anything newly appearing is a cascade orphan exposed by these deletions. Delete it and repeat until the list stops growing.

**Step 2: Confirm coverage did not drop**

```bash
cd cross-compile/www && npm run test:coverage 2>&1 | tail -20
```

Expected: total line coverage equal or **higher** than the recorded baseline. A drop means something live was deleted — bisect the task commits.

**Step 3: Open the PR**

```bash
cd /home/kmk/dev/anyka-dev && rtk git push -u origin chore/webui-cleanup
rtk gh pr create --title "chore(webui): remove dead code" --body "..."
```

Include the before/after coverage numbers and the `npm run analyze` delta in the body.

---

# PR2 — `refactor(webui): dedup`

Target: ~-420 lines, zero visual change. **Success criterion: no test file is modified.** If a test needs editing, the refactor changed behavior — fix the refactor, not the test. The one legitimate exception is noted in Task 14.

Branch from `main` once PR1 merges.

### Task 14: forwardRef → ref-as-prop

React 19 passes `ref` as an ordinary prop; `forwardRef` is ceremony. 51 wrappers across 15 files.

**Files (all in `cross-compile/www/src/components/ui/`):** `select.tsx` (7), `alert-dialog.tsx` (6), `card.tsx` (6), `form.tsx` (5), `settings-card.tsx` (5), `dialog.tsx` (4), `sheet.tsx` (4), `status-card.tsx` (4), `collapsible.tsx` (3), `radio-group.tsx` (2), `button.tsx`, `input.tsx`, `label.tsx`, `slider.tsx`, `switch.tsx` (1 each)

Do `settings-card.tsx` anyway — PR3 deletes it, but doing it here keeps this task a mechanical sweep rather than a list of exceptions.

**Step 1: Convert one file first**

Start with `input.tsx` (single wrapper). The shape:

```typescript
// before
const Input = React.forwardRef<HTMLInputElement, InputProps>(({ className, ...props }, ref) => (
  <input ref={ref} className={cn(BASE, className)} {...props} />
));
Input.displayName = 'Input';

// after
function Input({ className, ...props }: InputProps) {
  return <input className={cn(BASE, className)} {...props} />;
}
```

`ref` arrives inside `...props` and lands on the DOM node unchanged. Drop `displayName` — a named function declaration already supplies it.

**Step 2: Verify before continuing**

```bash
cd cross-compile/www && npx vitest run src/components/ui/input.test.tsx
```

Expected: PASS, unmodified. If a test asserted on ref forwarding it may need updating — that is the one sanctioned test edit in PR2. Flag it in the PR description rather than editing quietly.

**Step 3: Sweep the remaining 14 files, then run the gate and commit**

```bash
npm run lint && npm run type-check && npm run test
rtk git commit -am "refactor(webui): drop forwardRef, React 19 passes ref as a prop"
```

---

### Task 15: Replace the data-testid casts

29 copies of `data-testid={props['data-testid' as keyof typeof props] || 'x'}`.

**Files:** the same 15 `components/ui/*.tsx` files

**Step 1: Convert the pattern**

```typescript
// before
function Card({ className, ...props }: Props) {
  return <div className={cn(BASE, className)}
    data-testid={props['data-testid' as keyof typeof props] || 'card'} {...props} />;
}

// after
function Card({ className, 'data-testid': testId = 'card', ...props }: Props) {
  return <div className={cn(BASE, className)} data-testid={testId} {...props} />;
}
```

Same resolution order — an explicit prop wins, the default applies otherwise — without the cast.

**Step 2: Run the gate and commit**

```bash
cd cross-compile/www && npm run lint && npm run type-check && npm run test
rtk git commit -am "refactor(webui): destructure data-testid instead of casting props"
```

Expected: tests pass **unmodified**. Every testid selector in the suite must still resolve — that is the proof the default survived.

---

### Task 16: Adopt `handleMutationError`

23 hand-rolled `onError` toasts. `handleMutationError` already exists in `src/utils/errorHandling.ts` and only `IdentificationPage` and `UserManagementPage` use it.

**Files:**
- `pages/LiveViewPage.tsx:72,82,92,102,117,131`
- `pages/settings/MaintenancePage.tsx:64,82,100,152,158,173`
- `pages/settings/ProfilesPage.tsx:108,123,153,534,557`
- `pages/settings/ImagingPage.tsx:151,160,192`
- `pages/settings/NetworkPage.tsx:156`
- `pages/settings/TimePage.tsx:141`
- `pages/settings/UserManagementPage.tsx:172`

**Step 1: Convert each site**

```typescript
// before
onError: (error) => {
  toast.error('PTZ move failed', {
    description: error instanceof Error ? error.message : 'An error occurred',
  });
},

// after
onError: (error) => handleMutationError(error, 'PTZ move failed'),
```

Keep every message string exactly as it is — the tests assert on them.

**Step 2: Run the gate and commit**

```bash
cd cross-compile/www && npm run lint && npm run type-check && npm run test
rtk git commit -am "refactor(webui): route mutation errors through handleMutationError"
```

Expected: unmodified tests pass. `mutationTestHelpers.testMutationWithErrorToast` asserts on both title and description, so a changed string fails loudly.

---

### Task 17: Collapse the LiveView D-pad

Eight buttons, each ~20 lines, differing only in direction and icon rotation.

**Files:** `cross-compile/www/src/pages/LiveViewPage.tsx:399-584`

**Step 1: Extract the button and the table**

```typescript
const DIRECTIONS = [
  { dir: 'up-left', label: 'Pan up-left', rotate: '-rotate-45' },
  { dir: 'up', label: 'Pan up' },
  { dir: 'up-right', label: 'Pan up-right', rotate: 'rotate-45' },
  { dir: 'left', label: 'Pan left', icon: ArrowLeft },
  { dir: 'right', label: 'Pan right', icon: ArrowRight },
  { dir: 'down-left', label: 'Pan down-left', rotate: '-rotate-[135deg]' },
  { dir: 'down', label: 'Pan down', icon: ArrowDown },
  { dir: 'down-right', label: 'Pan down-right', rotate: 'rotate-[135deg]' },
] as const;
```

The `PtzButton` component keeps the existing `onMouseDown` / `onMouseUp` / `onMouseLeave` / `onKeyDown` / `onKeyUp` handlers and emits `data-testid={`liveview-ptz-${dir}-button`}` — the exact ids the suite already uses. Render rows by slicing `DIRECTIONS`, with the Home button between `left` and `right`.

**Step 2: Run the gate and commit**

```bash
cd cross-compile/www && npm run lint && npm run type-check && npm run test
rtk git commit -am "refactor(webui): generate the PTZ d-pad from a direction table"
```

Expected: `LiveViewPage.test.tsx` passes unmodified — it selects by those testids, so a mismatch fails immediately.

> Keep the keyboard handlers. `Enter` / `Space` press-and-hold is the only way to drive PTZ without a mouse; collapsing it into `onClick` would cut an accessibility path.

---

### Task 18: Dedup `profileService`

**Files:** `cross-compile/www/src/services/profileService.ts`

Two edits:
- `getVideoEncoderConfigurations` (263-292) and `getVideoEncoderConfiguration` (318-345) repeat the same 25-line mapping. Extract `mapEncoderConfig(config: Record<string, unknown>): VideoEncoderConfiguration` and call it from both.
- `parseH264Options` (489) and `parseJpegOptions` (535) are the same function minus three fields. Merge into one `parseCodecOptions(node, { withH264Extras })`, or have `parseJpegOptions` build the shared part and let the H264 path extend it.

**Step 1: Refactor, run the gate, commit**

```bash
cd cross-compile/www && npm run lint && npm run type-check && npm run test
rtk git commit -am "refactor(webui): dedup the encoder-config mapping and codec options"
```

Expected: `profileService.test.ts` (769 lines) passes unmodified. It is the densest service suite in the repo and is the real check on this task.

---

### Task 19: Dedup `Layout`

**Files:** `cross-compile/www/src/Layout.tsx`

Four edits:
- `NavLinkItem`'s two branches (93-215) duplicate the icon/label/description block. Extract `NavItemBody({ item, isActive })` and use it in both.
- `item.path.replaceAll('/', '-').replace(/^-/, '')` appears six times. Hoist to `const id = navId(item.path)` once per component.
- `allNavItems` (380-388) is a `useMemo` with `[]` deps over the module constant `navItems`. Hoist it to module scope as a plain `const`.
- `hover:bg-dark-hover` (280, 298, 309, 327) names a colour absent from `tailwind.config.js` — it renders nothing. Replace with `hover:bg-white/5`, matching the neighbouring hover states.

**Step 1: Refactor, run the gate, commit**

```bash
cd cross-compile/www && npm run lint && npm run type-check && npm run test
rtk git commit -am "refactor(webui): dedup NavLinkItem and fix the no-op dark-hover class"
```

Expected: `Layout.test.tsx` (357 lines) passes unmodified.

> `bg-dark-hover` is a visible change — those buttons gain a hover state they never had. It belongs in PR3 by that rule, but it is one class and inseparable from this dedup. Call it out in the PR body.

---

### Task 20: Give `soapRequest` a config parameter

`authService` bypasses `soapRequest` and re-implements `getDeviceInformation`'s field mapping, purely because it needs a custom `Authorization` header.

**Files:**
- Modify: `cross-compile/www/src/services/soap/client.ts:32-59`
- Modify: `cross-compile/www/src/services/authService.ts:21-83`

**Step 1: Thread the config through**

```typescript
export async function soapRequest<T>(
  endpoint: string,
  body: string,
  responseTarget?: string,
  config?: ApiRequestConfig,
): Promise<T> {
  const response = await apiClient.post(endpoint, createSOAPEnvelope(body), config);
  // ...unchanged
}
```

**Step 2: Collapse `verifyCredentials`**

Call `getDeviceInformation` — which will need the same optional config parameter — and delete the local `extractDeviceInfo` (69-83). Keep `verifyCredentials`'s error mapping intact: `LoginPage` and `ChangePasswordDialog` both depend on the 401 → "Invalid username or password" translation.

**Step 3: Run the gate and commit**

```bash
cd cross-compile/www && npm run lint && npm run type-check && npm run test
rtk git commit -am "refactor(webui): let soapRequest take a config so authService can reuse deviceService"
```

Expected: `authService.test.ts` passes unmodified.

---

# PR-auth — `refactor(webui): drop client-side credential encryption`

Target: ~-440 lines. Independent of PR2 and PR3 — touches no file they touch. Branch from `main`.

### Task 21: Delete the crypto layer

The module-level AES key dies on reload, so `decrypt` already fails after every refresh and the user already re-authenticates. The `memoryAuth` branch has identical observable semantics with no crypto.

**Files:**
- Delete: `cross-compile/www/src/utils/crypto.ts`
- Delete: `cross-compile/www/src/utils/crypto.test.ts`
- Modify: `cross-compile/www/src/hooks/useAuth.tsx` — delete `StoredAuthData`, `getInitialStoredData`, `AUTH_STORAGE_KEY`, the `storedData` state and every branch reading it; keep `memoryAuth` as the only credential store
- Modify: `cross-compile/www/src/hooks/useAuth.test.tsx` — delete the encrypted-storage cases
- Modify: `cross-compile/www/src/services/api.ts:115` — drop `sessionStorage.removeItem('onvif_camera_auth')` from the 401 handler; the `auth:unauthorized` event already drives logout
- Modify: `cross-compile/www/src/test/componentTestHelpers.tsx` — delete `createEncryptedFixture` and update its four callers

**Step 1: Simplify `useAuth`**

`login` stops being async-with-fallback and becomes a plain `setMemoryAuth({ username, password })`. `getCredentials` and `getBasicAuthHeader` lose their decrypt paths and their `try/catch`. `logout` keeps clearing state but drops `clearSessionKey`.

**Step 2: Run the gate**

```bash
cd cross-compile/www && npm run lint && npm run type-check && npm run test
```

**Step 3: Verify by hand**

```bash
npm run dev
```

Log in, navigate between pages (session holds), refresh (login prompt returns). That last step must match today's behaviour — it is the whole argument for the change.

**Step 4: Commit and open the PR**

```bash
rtk git commit -am "refactor(webui): keep credentials in memory, drop the AES-GCM ceremony"
```

---

# PR3 — `refactor(webui): design system`

Target: ~-415 lines, visible changes. Tests change by design here, so they are **not** the signal — hardware verification is. Branch from `main` after PR2 merges.

### Task 22: Migrate `SettingsCard` to shadcn `Card`

`AGENTS.md` says prefer shadcn primitives and do not invent base components. `SettingsCard` is the invented one; `Card` is the documented pattern.

**Files:**
- Modify: `cross-compile/www/src/components/ui/card.tsx` — add a `size` variant so `p-5` / `border-b p-5` are reachable
- Modify: `pages/LiveViewPage.tsx`, `pages/settings/{Identification,Network,Time,Imaging,Profiles,UserManagement,Maintenance}Page.tsx`
- Delete: `cross-compile/www/src/components/ui/settings-card.tsx`, `settings-card.test.tsx`
- Modify: the page test files — 15 `settings-card-*` testid references

**Step 1: Add the variant**

`SettingsCard` differs from `Card` only in padding and border: `p-5` vs `p-6`, header `border-b p-5` vs `flex flex-col space-y-1.5 p-6`. Add `size?: 'default' | 'compact'` via `cva`, matching how `button.tsx` and `badge.tsx` already do variants.

**Step 2: Migrate one page and verify**

Start with `pages/settings/TimePage.tsx` (smallest consumer). Rename `SettingsCard*` → `Card*` with `size="compact"`, then update `TimePage.test.tsx`'s testid references from `settings-card-*` to `card-*`.

```bash
cd cross-compile/www && npx vitest run src/pages/settings/TimePage.test.tsx
```

**Step 3: Migrate the remaining six pages plus LiveViewPage, delete `settings-card.tsx`, run the gate**

```bash
npm run lint && npm run type-check && npm run test
```

**Step 4: Update the design memory and commit**

`.serena/memories/www-design-system.md` documents the `Card` pattern already — add a note that `size="compact"` is the settings-page default so the next page does not reinvent `SettingsCard`.

```bash
rtk git commit -am "refactor(webui): fold SettingsCard into the shadcn Card primitive"
```

---

### Task 23: Put `StatusCard` on theme tokens

Not a duplicate of `Card` — it is a composite (image + label/value grid) used as a device-summary header on two pages. Its defect is hardcoded hex bypassing the theme.

**Files:** `cross-compile/www/src/components/ui/status-card.tsx`

| Hardcoded | Token |
|---|---|
| `border-[#3a3a3c]` | `border-border` |
| `bg-[#1c1c1e]` | `bg-card` |
| `bg-[#2c2c2e]` | `bg-muted` |
| `text-[#6b6b6f]` | `text-muted-foreground` |

Also convert the pixel literals (`mb-[32px]`, `rounded-[12px]`, `p-[24px]`, `gap-[24px]`, `size-[120px]`, `text-[13px]`, `text-[15px]`) to scale utilities where one exists.

**Step 1: Swap, run the gate, commit**

```bash
cd cross-compile/www && npm run lint && npm run type-check && npm run test
rtk git commit -am "refactor(webui): put StatusCard on theme tokens instead of hex literals"
```

> The hex values were sampled from the design at a point when the palette was the older "Camera.UI red" scheme. The tokens are near-equivalents, not exact — expect a subtle shift on Identification and Network. That is the point: they now track the theme.

---

### Task 24: Delete `ConnectionStatus`

`ConnectionStatusBadge` has exactly one call site, `Layout.tsx:435`, passing the literal `status="connected"`. The 3-state `ConnectionStatus` variant has none.

**Files:**
- Delete: `cross-compile/www/src/components/common/ConnectionStatus.tsx`, `ConnectionStatus.test.tsx`
- Modify: `cross-compile/www/src/Layout.tsx:31,435`
- Modify: `cross-compile/www/src/Layout.test.tsx` — drop the `connection-status-badge-*` assertions

**Step 1: Replace the call site**

Substitute the `.status-badge-connected` utility that survived Task 7:

```tsx
<div className="status-badge-connected" data-testid="layout-connection-status">Connected</div>
```

**Step 2: Run the gate and commit**

```bash
cd cross-compile/www && npm run lint && npm run type-check && npm run test
rtk git commit -am "refactor(webui): drop ConnectionStatus, its only caller passed a literal"
```

> This preserves today's behaviour: a hardcoded "Connected". Wiring real connectivity is a feature, not cleanup — leave it for Diagnostics, which will have live telemetry to drive it.

---

### Task 25: Delete LiveView's fake panels

**Files:** `cross-compile/www/src/pages/LiveViewPage.tsx:279-373`, `LiveViewPage.test.tsx`

Delete the Stream URL bar (`rtsp://192.168.1.100:554/main`, hardcoded, with a Copy button that has no handler), the Stream Info card (1920x1080 / 4096 Kbps / 30 fps / H.264, all literals), and the Network Stats card (0.0% / 45 ms / 4.2 Mbps, all literals). Drop the now-unused `Copy`, `Activity`, and `Wifi` imports, and the matching test assertions.

Keep the video placeholder and stream-type toggle — those are real UI awaiting a real stream.

**Step 1: Delete, run the gate, commit**

```bash
cd cross-compile/www && npm run lint && npm run type-check && npm run test
rtk git commit -am "refactor(webui): delete LiveView panels showing hardcoded values"
```

---

### Task 26: Verify PR3 on hardware

Tests were edited in this PR, so they are not an independent signal.

**Step 1: Build and deploy**

```bash
cd cross-compile/www && npm run build
```

Deploy to `.198` per the `anyka-embedded-build` skill.

**Step 2: Load every affected page**

```bash
curl -sS -o /dev/null -w '%{http_code}\n' http://192.168.2.198/
```

Then in a browser, walk all seven settings pages plus Live View. Check specifically:
- **Identification, Network** — `StatusCard` renders with theme colours, no unstyled boxes
- **All seven settings pages** — card padding and borders match the pre-change look
- **Live View** — PTZ d-pad still drives the camera, presets still load

Verify with a real request, not a file listing.

**Step 3: Open the PR**

Include before/after screenshots of Identification and one settings page.

---

## Done

Diagnostics starts on a tree with one card family, one error path, one set of types, and ~3450 fewer lines.

Total: ~-2190 (PR1) + ~-420 (PR2) + ~-440 (PR-auth) + ~-415 (PR3).

**Dropped from scope:** the `fast-xml-parser` `isArray` option. `safeString` returns its default when handed an array, so a blanket rule turns affected fields into `"Unknown"` silently — no crash, no type error, no lint warning. See the Rejected section of the design doc.
