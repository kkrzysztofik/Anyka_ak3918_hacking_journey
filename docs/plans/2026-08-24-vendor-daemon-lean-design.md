# Vendor-daemon lean trim (C-only)

Date: 2026-08-24
Status: approved
Branch / worktree: `feat/vendor-daemon-lean` @ `.worktrees/vendor-daemon-lean`

## Problem

A ponytail-audit of `cross-compile/vendor-daemon/` found ~2k lines of
complexity that do not buy behavior: a full kernel `list.h`, unused rxi/log
APIs, consumer-only ring helpers (Rust already owns the consumer in
`shm_ring.rs`), a VPSS no-op module, duplicated frame-socket accept paths, and
essay-length doxygen on thin IPC handlers.

## Decisions

| Choice | Decision |
|--------|----------|
| Scope | **A — C-only lean.** Keep both frame sockets and packed ring ABI. |
| Ring consumer APIs | **Delete** from `vd_ring_buffer.h` (`open` / `read` / `release` / `is_shutdown`). Rust remains the consumer. |
| Approach | **Surgical trim** (not aggressive header merge, not docs-only). |
| Logging | **Trim** rxi/log unused surface; do **not** replace with fprintf. |

## In scope

1. Shrink `include/list.h` to `struct list_head` (+ `INIT_LIST_HEAD` / whatever
   `ak_aenc.h` needs to compile). No `list_*` use in `src/`.
2. Trim `src/log.c` / `src/log.h`: keep level macros, `log_log`, `log_set_level`,
   `log_set_quiet`, `log_add_fp`. Remove unused `log_set_lock`,
   `log_level_string`, `log_trace`, and the 32-slot multi-callback capacity
   beyond what `log_add_fp` needs.
3. Delete consumer-only helpers from `include/vd_ring_buffer.h`. Keep layout,
   create / write / evict / reset / shutdown / destroy, and getters. Adjust
   `tests/test_ring_epoch.c` if needed.
4. Fold `CMD_VPSS_INIT` / `CMD_VPSS_DESTROY` into `dispatcher.c` as
   `STATUS_OK` no-ops; delete `handlers_vpss.c` / `handlers_vpss.h`.
5. Table-driven main/sub frame accept in `main.c` (still two sockets, two locks).
6. Compress handler doxygen to one-liners; fix stale main.c claim of
   “socket-based delivery on ring overflow”. Keep push/globals essays that
   explain join / orphan / timeout.

## Out of scope

- Merging `/tmp/vd-frame-main.sock` and `/tmp/vd-frame-sub.sock`
- Packed ABI field removal (`checksum`, `socket_fallback_count`, flag bits)
- Full log library replacement
- Changes to `onvif-rust` / `shm_ring.rs`
- Deleting `VD_STREAM_AUDIO` (needed by live-audio design)
- Touching `osd_ipcsrv_stubs.c`, `osd_vpss_wrap.c`, `vi_attr_wrap.c`, token
  registry, or push join/orphan logic
- Merging the seven `handlers_*.h` files; dropping stdout/stderr save/restore

## Ordering

`list.h` → log trim → ring consumer delete + host test → VPSS fold → accept
dedupe → doxygen / comment pass → cross build.

## Verification

- Host: compile and run `tests/test_ring_epoch.c`
- Cross: `make` in `cross-compile/vendor-daemon` (release)
- Grep: no `handlers_vpss`, no deleted `vd_ring_*` consumer APIs, no removed
  log APIs
- Smoke: ctrl + both frame sockets still created; `CMD_VPSS_*` still OK

## Success

Smaller daemon tree with the same sockets, packed layout, and IPC behavior.
Live-audio’s `VD_STREAM_AUDIO` untouched. No full log rewrite.
