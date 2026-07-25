# Phase 3 Report: Async Persistence Off Executor

## Status

DONE

## Summary

Phase 3 moved sync SD-card persistence out of async ONVIF handler paths by generalizing the existing config persistence debounce worker and wiring request-save handles for config, users, profiles, and imaging settings.

Handlers now mutate in-memory state and enqueue non-blocking save requests. Atomic tmp + `sync_all` + rename writes are preserved in the shared `atomic_write` path and run through `tokio::task::spawn_blocking`.

## Implementation

- Generalized `ConfigPersistenceService` into `PersistenceService` plus cloneable `PersistenceHandle`.
- Kept `ConfigPersistenceService` as a thin wrapper over the generic service.
- Made `SnapshotFn` `Send + Sync` so persistence services can be spawned safely.
- Added `PendingWrite` snapshots carrying path, bytes, and optional file mode.
- Wrapped `ConfigStorage::save` in `spawn_blocking` and made it async.
- Added `UserStorage::set_persistence`, `request_save`, and `to_toml_bytes`; device user handlers call only `request_save()`.
- Added app startup wiring for user/profile persistence services in both normal and custom-platform constructors.
- Added profile persistence handle plumbing through `AppState`, `OnvifServer`, `MediaService`, and `ProfileManager`.
- Removed the synchronous `ProfileManager::persist_all` fallback; it now snapshots memory and enqueues only.
- Added imaging persistence handle/service support; `SetImagingSettings` force persistence enqueues only.

## Tests Added/Updated

- `test_set_settings_force_persistence_enqueues_save_without_direct_write`
- `test_profile_persistence_enqueues_save_without_direct_write`
- Updated `ConfigStorage` save/load tests to await async save.
- Existing persistence debounce/shutdown tests continue to cover generic service flush behavior.

## Verification

- RED check: focused tests initially failed on missing async/enqueue APIs and `ConfigStorage::save` not being awaitable.
- Focused tests: `cargo test --target x86_64-unknown-linux-gnu persistence_enqueues` passed.
- Focused config tests: `cargo test --target x86_64-unknown-linux-gnu config_storage` passed.
- Broader tests: `cargo test --target x86_64-unknown-linux-gnu --lib` passed.
- Full tests: `cargo test --target x86_64-unknown-linux-gnu` passed.
- Formatting: `cargo fmt --check` passed.
- Clippy: `cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings` passed.
- Docs: `cargo doc --no-deps` completed with pre-existing rustdoc warnings outside Phase 3.
- UBS: inconclusive; scoped scan produced no output for more than five minutes and was stopped.

## Concerns

- Device-level p99 latency and power-cycle survival still require hardware validation with SD-card fsync latency, as described in the Phase 3 brief.
- `cargo doc --no-deps` still emits existing rustdoc link warnings in unrelated modules.
- Workspace had pre-existing unrelated dirty files before this task; the commit should include only Phase 3 Rust files and this report.
