# Event Audio WebUI Trigger Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Let an Administrator list configured sound events and play one from the Diagnostics page via thin REST over the existing `SharedSoundPlayer`.

**Architecture:** New `diagnostics/sound.rs` handlers (`GET /api/sound`, `POST /api/sound/play`) share the same `SoundPlayer` built at Anyka bring-up. WebUI adds a Diagnostics card (Select + Play) backed by `soundService.ts`. No SOAP, no clip upload, no debounce bypass.

**Tech Stack:** Rust (axum Extension, onvif-rust), React 19 + TanStack Query + Vitest (`cross-compile/www`), existing `authorizedFetch` / Basic Auth.

**Design doc:** `docs/plans/2026-08-26-event-audio-webui-design.md`

**Skills during implementation:** `@camera-webui-components`, `@anyka-webui-testing`, `@superpowers:test-driven-development`

---

## Task 1: Richer `SoundPlayer::play` outcome (TDD)

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/sound.rs`
- Test: same file (`#[cfg(test)]` module)

**Step 1: Write the failing tests**

Add / adjust unit tests so `play` returns a distinct result (not only `Ok(())`):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundPlayResult {
    Accepted,
    Busy,
    Debounced,
    Disabled,
    NoClip,
}
```

Cases:
- disabled → `Ok(Disabled)` (no sink call)
- no clip mapped → `Ok(NoClip)`
- within debounce → `Ok(Debounced)` (no second sink call)
- sink Accepted → `Ok(Accepted)`
- sink Busy → `Ok(Busy)`
- sink Err → `Err(...)`

Update existing tests that asserted `is_ok()` / call counts to match.

**Step 2: Run tests — expect FAIL**

```bash
source ./setenv.sh
cd cross-compile
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust --lib sound -- --nocapture
```

Expected: compile fail (`SoundPlayResult` missing) or assertion fail.

**Step 3: Minimal implementation**

Change `SoundPlayer::play` to return `PlatformResult<SoundPlayResult>` as above. Keep `play_event` fire-and-forget: map `Err` → warn; treat all `Ok(_)` as silent for lifecycle callers.

**Step 4: Run tests — expect PASS**

Same command. Expected: all `sound` tests green.

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/platform/anyka/sound.rs
git commit -m "refactor(sound): return SoundPlayResult from SoundPlayer::play"
```

---

### Task 2: `SoundApiState` + GET/POST handlers (TDD)

**Files:**
- Create: `cross-compile/onvif-rust/src/diagnostics/sound.rs`
- Modify: `cross-compile/onvif-rust/src/diagnostics/mod.rs` (`pub mod sound;`)

**Step 1: Write the failing handler tests**

In `diagnostics/sound.rs` `#[cfg(test)]`, use a small fake player trait **or** build `SoundPlayer` with the existing `FakeSink` behind an `Arc` and wrap it:

```rust
pub struct SoundApiState {
    /// `None` on stub / non-Anyka builds.
    player: Option<SharedSoundPlayer>,
}

#[derive(Serialize)]
pub struct SoundStatusResponse {
    pub enabled: bool,
    pub events: Vec<SoundEventItem>,
}

#[derive(Serialize)]
pub struct SoundEventItem {
    pub id: String,
    pub clip: String,
}

#[derive(Deserialize)]
pub struct PlaySoundRequest {
    pub event: String,
}
```

For unit tests without real `AnykaIpc`, prefer a test-only constructor that holds `Arc<SoundPlayer<FakeSink, _>>` **or** extract a thin `SoundControl` trait used by handlers (`status()` + `play(&str) -> PlatformResult<SoundPlayResult>`). Prefer the trait if `SharedSoundPlayer` cannot be built on host without IPC — keep the production type as `Arc<dyn SoundControl + Send + Sync>` **only if** needed; otherwise keep `Option<SharedSoundPlayer>` and test handlers by calling pure mapping helpers:

```rust
fn status_from_config(cfg: &SoundConfig) -> SoundStatusResponse { ... }
fn http_status_for_play(result: PlatformResult<SoundPlayResult>) -> StatusCode { ... }
```

Minimum tests:
- `status_from_config` sorts events by id, copies enabled
- `http_status_for_play(Ok(Accepted))` → 200
- `Ok(Busy)` → 409
- `Ok(Debounced)` → 200 (body `status=debounced`)
- `Ok(Disabled)` / `Ok(NoClip)` → 404
- `Err(_)` → 503

**Step 2: Run — expect FAIL**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust --lib diagnostics::sound -- --nocapture
```

**Step 3: Implement handlers**

```rust
pub async fn handle_get_sound(
    Extension(state): Extension<Arc<SoundApiState>>,
) -> impl IntoResponse { ... }

pub async fn handle_play_sound(
    Extension(state): Extension<Arc<SoundApiState>>,
    Json(body): Json<PlaySoundRequest>,
) -> impl IntoResponse { ... }
```

- GET with `player: None` → `{ enabled: false, events: [] }`
- GET with player → read config (add `SoundPlayer::config()` accessor or store `SoundConfig` clone in `SoundApiState` at construction)
- POST with no player → 503
- POST empty `event` → 400
- Response JSON for play: `{ "status": "accepted" | "busy" | "debounced" }` (and 404/503 with small error body if that matches network/snmp style)

**Step 4: Tests PASS + Commit**

```bash
git add cross-compile/onvif-rust/src/diagnostics/sound.rs \
        cross-compile/onvif-rust/src/diagnostics/mod.rs
git commit -m "feat(sound): GET/POST /api/sound handlers"
```

---

### Task 3: Mount routes + auth + wire player from app

**Files:**
- Modify: `cross-compile/onvif-rust/src/diagnostics/http.rs` (`required_level_for_path`)
- Modify: `cross-compile/onvif-rust/src/onvif/server.rs` (routes + Extension + `with_sound`)
- Modify: `cross-compile/onvif-rust/src/app.rs` (pass player into server)

**Step 1: Auth tests (failing)**

In `diagnostics/http.rs` tests, add:

```rust
assert_eq!(required_level_for_path("/sound"), AuthLevel::Administrator);
assert_eq!(required_level_for_path("/sound/play"), AuthLevel::Administrator);
```

(Fail-closed `_` already requires Admin — still add explicit arms for clarity.)

**Step 2: Implement path arms + mount**

In `server.rs` next to `/snmp`:

```rust
.route("/sound", get(crate::diagnostics::sound::handle_get_sound))
.route("/sound/play", post(crate::diagnostics::sound::handle_play_sound))
```

`.layer(Extension(sound_state))` where `sound_state` defaults to `SoundApiState::empty()`.

Add `OnvifServer::with_sound_player(...)` (or set on a field before `build_router`).

In `app.rs` after `build_shared_player`, clone the Arc into the server:

```rust
.with_diagnostics(...)
.with_sound(SoundApiState::new(Some(Arc::clone(&sound_player))))
```

Stub / non-Anyka path: leave empty state (routes still mounted when diagnostics is on).

**Step 3: Integration-style router test (optional but preferred)**

Mirror `test_get_network_requires_auth`: unauthenticated GET `/api/sound` → 401 when auth on.

**Step 4: Host tests + Commit**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust --lib
$CARGO clippy --target x86_64-unknown-linux-gnu -p onvif-rust -- -D warnings
git add cross-compile/onvif-rust/src/diagnostics/http.rs \
        cross-compile/onvif-rust/src/onvif/server.rs \
        cross-compile/onvif-rust/src/app.rs
git commit -m "feat(sound): mount /api/sound and wire SharedSoundPlayer"
```

---

### Task 4: WebUI `soundService` (TDD)

**Files:**
- Create: `cross-compile/www/src/services/soundService.ts`
- Create: `cross-compile/www/src/services/soundService.test.ts`

**Step 1: Failing tests**

```typescript
// getSoundStatus parses { enabled, events: [{ id, clip }] }
// playSound('boot_ready') POSTs JSON and resolves on 200
// playSound maps 409 → Error with message containing 'busy' (or typed SoundBusyError)
// playSound maps 404 → Error
```

Use `vi.stubGlobal('fetch', ...)` or the project’s existing `authorizedFetch` mock pattern from `networkService.test.ts`.

**Step 2: Implement service**

```typescript
export interface SoundEventItem { id: string; clip: string }
export interface SoundStatus { enabled: boolean; events: SoundEventItem[] }
export type PlaySoundStatus = 'accepted' | 'busy' | 'debounced';

export async function getSoundStatus(signal?: AbortSignal): Promise<SoundStatus>
export async function playSound(event: string, signal?: AbortSignal): Promise<PlaySoundStatus>
```

**Step 3: `npm run test -- soundService` PASS → Commit**

```bash
cd cross-compile/www && npm run test -- soundService
git add cross-compile/www/src/services/soundService.ts \
        cross-compile/www/src/services/soundService.test.ts
git commit -m "feat(www): soundService for /api/sound"
```

---

### Task 5: Diagnostics Sound card (TDD)

**Files:**
- Modify: `cross-compile/www/src/pages/DiagnosticsPage.tsx`
- Modify: `cross-compile/www/src/pages/DiagnosticsPage.test.tsx`
- Follow: `@camera-webui-components`, `@anyka-webui-testing`

**Step 1: Failing page tests**

Mock `@/services/soundService`:

- When status loads with events → `sound-card`, `sound-event-select`, `sound-play-button` present
- `enabled: false` → play button disabled; `sound-disabled-message` (or similar) present
- Click play → `playSound` called with selected event; success toast
- `playSound` rejects busy → error/busy toast

Use `data-testid` only. `renderWithProviders`, `mockToast` from `src/test/componentTestHelpers.tsx`.

**Step 2: Implement `SoundTestCard`**

Place below `FirmwareUpdateCard`, above `SystemLogsCard`:

- `useQuery(['sound-status'], getSoundStatus)`
- `Select` of event ids; default first event
- `useMutation` → `playSound`; toasts:
  - accepted → success “Playing …”
  - debounced → info “Skipped (debounce)”
  - busy → error “Speaker busy, try again”
  - other → error with message

Industrial Dark: existing `Card` / `Button` / `Select` primitives only.

**Step 3: Tests + verify**

```bash
cd cross-compile/www
npm run test -- DiagnosticsPage
npm run verify
```

```bash
git add cross-compile/www/src/pages/DiagnosticsPage.tsx \
        cross-compile/www/src/pages/DiagnosticsPage.test.tsx
git commit -m "feat(www): Diagnostics sound event play card"
```

---

### Task 6: Device smoke on `.198` (manual)

**Steps:**

1. Build ARM onvif-rust from `cross-compile/onvif-rust` (so `.cargo/config.toml` linker applies); build www (`npm run build`).
2. Deploy binary to active slot (`slots/a/onvif/onvif-rust.bin`) and refresh www assets as usual.
3. Restart pair; confirm lifecycle `boot_ready` still logs `sound_played`.
4. Open WebUI as Admin → Diagnostics → select `boot_ready` → Play.
5. Expect audible chime + `event=sound_played` in `/mnt/logs/vendor_daemon.log`.
6. Rapid second Play within debounce → UI “Skipped (debounce)”, no second `sound_played` (or only one within window).

**Commit** only if deploy scripts/docs changed; otherwise note results in the PR description.

---

### Task 7: Ponytail + quality gates

```bash
source ./setenv.sh
cd cross-compile
$CARGO fmt
$CARGO clippy --target x86_64-unknown-linux-gnu -p onvif-rust -- -D warnings
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust --lib
cd www && npm run verify && npm run test
```

Self-review diff for bloat (extra abstractions, duplicate toasts). Commit any lint-only follow-ups.

Also commit the leftover `sound.rs` `#[cfg(test)] PlatformError` / collapsible-if fix if still uncommitted from Phase 2.

---

## Out of scope (unchanged)

Clip upload, volume UI, editing `[sound.events]`, User-level play, SOAP, `.127` until shell access exists.
