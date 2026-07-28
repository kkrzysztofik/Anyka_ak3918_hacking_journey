# Removal Plan: `dashmap` → `Mutex<HashMap>` — `cross-compile/onvif-rust`

Source: dependency review triggered by a toolchain failure during PR #46. `dashmap` is
declared at `onvif-rust/Cargo.toml:39` (`dashmap = "6.1"`) and used in three places, two
of them real. The crate buys shard-striped locking, which is worthless on the AK3918
(single-core armv5te) and costs a documented deadlock hazard, an inexact memory cap, and
a broken build. Priority order below = execution order.

**Scope note:** do this on its own branch off `main`. It touches two security modules and
must not be folded into a streaming/RTSP PR.

---

## Why remove it

**1. The concurrency argument does not apply.**
Target is `armv5te-unknown-linux-uclibceabi`, a single-core ARM9. Sharded locking exists to
let many cores touch disjoint buckets at once. On one core every critical section is
serialized regardless. The guarded region here is a hash lookup plus two integer ops.

**2. Production traffic through it is one call per HTTP request.**
`src/onvif/server.rs:170` → `check_rate_limit`. That is the only non-test caller of either
map outside the security modules.

**3. It is actively costing correctness and clarity today.**

| Location | Cost |
| --- | --- |
| `security/rate_limit.rs:158-160` | Deadlock hazard documented in a comment: `len()`/`retain()` must not be called while holding an entry guard, because that guard holds a shard write lock and those methods need other shards. |
| `security/rate_limit.rs:166-168` | `MAX_TRACKED_IPS` is only a *soft* cap. `len()` and `entry()` race, so the map can exceed it. A single lock makes it exact for free. |
| `security/rate_limit.rs:111` | `last_inline_cleanup: Arc<Mutex<Option<Instant>>>` — a `std::sync::Mutex` already sits beside the lock-free map to work around the above. |

**4. It is the only reason the toolchain-hijack bug reaches this repo.**
Every published `dashmap` ships a stray `rust-toolchain.toml` (not in the crate's
`exclude` list). Cargo compiles each dependency with cwd set to its own source directory,
so the rustup shim picks that file up and swaps the compiler mid-build. Verified against
the published tarballs:

| version | pinned channel | result |
| --- | --- | --- |
| 6.1.0 (current) | `1.65` | `--check-cfg` rejected — stable only from 1.80 |
| 6.2.1 (latest stable) | `1.85` | flag check passes, then 9 × `E0514: compiled by an incompatible version of rustc` |
| 7.0.0-rc2 | `1.70` | same failure as 6.1.0 |

There is no version to upgrade into. The current workaround is deleting the extracted file
from `~/.cargo/registry/src/.../dashmap-6.1.0/`, which any cache wipe undoes.

**Counterargument, recorded honestly:** on a multi-core SoC under heavy concurrent ONVIF
load, sharding would start to earn its keep. That is not this device, and
`Mutex<HashMap>` → `DashMap` is a trivial reversal if it ever becomes one.

---

## Phase 1 — `security/rate_limit.rs`

**Field change:** `counts: Arc<DashMap<IpAddr, RequestCount>>` → a single guarded struct so
the cleanup throttle stops being a second lock:

```rust
struct RateLimitState {
    counts: HashMap<IpAddr, RequestCount>,
    last_inline_cleanup: Option<Instant>,
}
// field: state: Arc<Mutex<RateLimitState>>
```

`#[derive(Clone)]` at `:102` still works — `Arc` is what makes it cloneable, not `DashMap`.

**Tasks:**

1. Drop `use dashmap::DashMap;` and `use dashmap::mapref::entry::Entry;` (`:52-53`).
   `std::collections::hash_map::Entry` has the same `Occupied`/`Vacant` shape.
2. Constructors `:124` and `:139` build the new state struct.
3. Rewrite `check_rate_limit` (`:155-204`). This is where the win lands: take the lock
   once and hold it across check + cap + insert. That deletes the TOCTOU comment
   (`:166-168`), the deadlock comment (`:158-160`), the double `len()` check
   (`:169`, `:190`), and the separate throttle mutex — roughly 30 lines.
4. **Do not call `self.cleanup()` while holding the guard** — `std::sync::Mutex` is not
   reentrant, so that is an instant self-deadlock. Split it:
   `fn cleanup_locked(state: &mut RateLimitState)` for the in-lock path, and keep the
   public `cleanup()` (`:249`) as lock-then-delegate.
5. Adjust guard-deref call sites: `get_mut` (`:161`) yields `&mut V` directly, so
   `r.value_mut()` becomes `r`; `vac.insert(...)` (`:197`) returns `&mut V`, not a
   `RefMut`. `get` (`:220`, `:233`) returns `Option<&V>` and the existing
   `.map(|entry| entry.count)` still compiles.
6. `retain` (`:251`) has an identical `FnMut(&K, &mut V) -> bool` signature — body unchanged.
7. `remove` (`:256`), `len` (`:271`) — add the lock, otherwise unchanged.
8. **`Debug` impl (`:319`) is a deadlock trap.** It reads `self.counts.len()`; if anything
   ever formats a `RateLimiter` while the lock is held, it hangs. Use `try_lock` and print
   a placeholder on contention.
9. Reuse the existing poison convention already in this file at `:175`:
   `.unwrap_or_else(|poisoned| poisoned.into_inner())`. Never `unwrap()` a lock here — a
   panicked request must not take the server down.
10. `start_cleanup_task` (`:288`) needs no structural change: the guard is never held
    across an `.await`, so a `std::sync::Mutex` is correct in the async task.

## Phase 2 — `security/brute_force.rs`

Smaller — no cap logic, no inline cleanup, so it is a near-mechanical swap.

**Tasks:**

1. Drop `use dashmap::DashMap;` (`:28`); field `records` (`:86`) →
   `Arc<Mutex<HashMap<IpAddr, FailureRecord>>>`; constructor `:106`.
2. Lock and adjust deref at: `get` (`:120`, `:189`, `:198`), `entry().or_insert_with()`
   (`:141`), `remove` (`:180`), `entry().or_default()` (`:216`), `get_mut` (`:226`),
   `retain` (`:237`), `len` (`:249`, `:264`).
3. Same `Debug` (`:264`) and poison rules as Phase 1.
4. Consider giving this module the same `MAX_TRACKED_IPS` bound the rate limiter has — a
   LAN scan with unique source IPs grows `records` without limit today. **Out of scope
   for this change; open separately if wanted.**

## Phase 3 — `utils/memory.rs` (feature-gated)

`allocations: dashmap::DashMap<usize, AllocationInfo>` (`:211`, `:222`) sits behind
`#[cfg(feature = "memory-profiling")]` (declared at `Cargo.toml:69`, not in `default`).

**Tasks:**

1. Swap to `Mutex<HashMap<usize, AllocationInfo>>`.
2. `track_allocation` (`:232`) re-sums the whole map on every insert — inside one lock this
   is now an obvious O(n)-per-allocation cost. Leave the behaviour alone in this change,
   but mark it: `// ponytail: O(n) resum per allocation, keep a running total if profiling
   ever gets used in anger`.
3. This code is not in a default build, so it will not be caught by a normal
   `cargo check`. See verification below.

## Phase 4 — drop the dependency

1. Delete `dashmap = "6.1"` from `onvif-rust/Cargo.toml:39`.
2. `cargo update` to drop it (and `hashbrown`, `lock_api`, `parking_lot_core`,
   `crossbeam-utils` if nothing else pulls them) from `Cargo.lock`.
3. Once merged, the registry-cache `rm` workaround is permanently unnecessary — note that
   in the PR body so nobody reintroduces it.

---

## Verification

Nothing here counts as done without these passing:

```bash
cd cross-compile
../toolchain/arm-anykav200-crosstool-ng/bin/cargo fmt --check
../toolchain/arm-anykav200-crosstool-ng/bin/cargo clippy --target x86_64-unknown-linux-gnu --all-targets -- -D warnings
../toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu -p onvif-rust

# Phase 3 is invisible to the above — it is feature-gated:
../toolchain/arm-anykav200-crosstool-ng/bin/cargo check --target x86_64-unknown-linux-gnu -p onvif-rust --features memory-profiling

# and the real target still has to build:
../toolchain/arm-anykav200-crosstool-ng/bin/cargo build --release --target armv5te-unknown-linux-uclibceabi -p onvif-rust
```

Baseline to preserve: `cargo test -p onvif-rust` is **2171 passed / 0 failed / 17 ignored**
as of `d4c4ead`. The existing suites in both security modules already cover windowing,
blocking, expiry, and cleanup, so they are the regression net — do not rewrite them to fit
the new internals.

**Behaviour change to call out in review:** `MAX_TRACKED_IPS` becomes an exact cap instead
of a soft one. Any test asserting the map may transiently exceed it is asserting the old
race and should be tightened, not deleted.

## Effort

19 map call sites across the two security modules — `len`(6), `get`(5), `entry`(3),
`remove`(2), `get_mut`(2), `retain`(1) — every one a `std::collections::HashMap` method
with identical semantics. Expect the diff to be net negative: the deadlock and TOCTOU
workarounds in `rate_limit.rs` come out with it.
