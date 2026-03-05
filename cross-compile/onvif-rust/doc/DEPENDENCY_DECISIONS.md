# Dependency Decisions

This document explains architectural decisions regarding dependency choices in the ONVIF Rust implementation.

## parking_lot vs tokio::sync

### Decision: Keep Both Dependencies

**Status**: Both `parking_lot` and `tokio::sync` are required and serve different purposes.

### Rationale

#### parking_lot Usage (Sync Contexts)

`parking_lot` is used for synchronization primitives in **synchronous (non-async) code paths**:

- `src/onvif/device/handlers.rs` - Device service handlers (sync)
- `src/onvif/dispatcher.rs` - Service dispatcher (sync)
- `src/config/users/mod.rs` - User storage (sync)
- `src/onvif/ws_security.rs` - WS-Security nonce cache (sync, per CRIT-005)
- `src/platform/anyka.rs` - Platform abstraction (sync)
- `src/onvif/media/profile_manager.rs` - Profile management (sync)
- `src/onvif/ptz/state.rs` - PTZ state (sync)
- `src/onvif/imaging/settings_store.rs` - Imaging settings (sync)
- `src/config/runtime.rs` - Configuration runtime (sync)
- `src/platform/stubs.rs` - Platform stubs (sync)

**Why parking_lot for sync code:**

- More efficient: No async overhead (no `.await` required)
- Better performance: Optimized for short critical sections
- No panics: `parking_lot::Mutex::lock()` returns guard directly (no `Result`)
- Lower latency: Designed for blocking operations

#### tokio::sync Usage (Async Contexts)

`tokio::sync` is used for synchronization primitives in **asynchronous code paths**:

- `src/discovery/ws_discovery.rs` - WS-Discovery service (async)
- `src/app.rs` - Application lifecycle (async broadcast channels)
- `src/onvif/server.rs` - Server shutdown coordination (async broadcast)
- `src/config/persistence.rs` - Config persistence (async channels)
- `src/lifecycle/shutdown.rs` - Shutdown coordination (async broadcast)

**Why tokio::sync for async code:**

- Required for `.await`: `tokio::sync::RwLock::read().await` is async-aware
- Integrates with tokio runtime: Properly yields to executor
- Prevents executor blocking: Critical for async performance

### Migration Analysis

**Cannot migrate parking_lot → tokio::sync:**

- All `parking_lot` usages are in sync functions
- Converting to async would require changing function signatures
- Would add unnecessary async overhead to sync code paths
- Would break existing API contracts

**Cannot migrate tokio::sync → parking_lot:**

- `tokio::sync::broadcast` has no equivalent in `parking_lot`
- `tokio::sync::RwLock` in async contexts requires `.await`
- `parking_lot::RwLock` would block the async executor

### Conclusion

Both dependencies are necessary:

- **parking_lot**: For efficient sync code synchronization
- **tokio::sync**: For async code synchronization and channels

This is a **correct architectural decision** that optimizes for performance in each context.

### References

- [parking_lot documentation](https://docs.rs/parking_lot/)
- [tokio::sync documentation](https://docs.rs/tokio/latest/tokio/sync/index.html)
- CRIT-005: Already migrated `std::sync::Mutex` → `parking_lot::Mutex` for better performance
