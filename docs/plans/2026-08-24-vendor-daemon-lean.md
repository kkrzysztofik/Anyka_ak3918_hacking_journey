# Vendor-daemon lean trim Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Shrink `cross-compile/vendor-daemon/` dead weight without changing sockets, packed ring ABI, or replacing the log library.

**Architecture:** Surgical C-only cuts from the approved design
(`docs/plans/2026-08-24-vendor-daemon-lean-design.md`): minimal `list.h`,
trimmed rxi/log surface, daemon-only ring header, VPSS folded into dispatcher,
table-driven dual-socket accept, shorter handler docs.

**Tech Stack:** C99 (gnu99), Anyka cross `Makefile`, host `gcc` for
`tests/test_ring_epoch.c`. Worktree: `.worktrees/vendor-daemon-lean` on
`feat/vendor-daemon-lean`.

**Design:** @docs/plans/2026-08-24-vendor-daemon-lean-design.md

---

## Task 1: Shrink `list.h` to what `ak_aenc.h` needs

**Files:**
- Modify: `cross-compile/vendor-daemon/include/list.h` (replace body)
- Verify: `cross-compile/vendor-daemon/include/ak_aenc.h` still compiles

**Step 1: Replace `list.h` with a minimal header**

`ak_aenc.h` only embeds `struct list_head` in `struct aenc_entry`. No
`src/*.c` calls any `list_*` API. Replace the ~764-line kernel list with:

```c
#ifndef _LINUX_LIST_H_
#define _LINUX_LIST_H_

#ifdef __cplusplus
extern "C" {
#endif

struct list_head {
	struct list_head *next, *prev;
};

static inline void INIT_LIST_HEAD(struct list_head *list)
{
	list->next = list;
	list->prev = list;
}

#ifdef __cplusplus
}
#endif

#endif /* _LINUX_LIST_H_ */
```

**Step 2: Host-compile a tiny include check**

Run from `cross-compile/vendor-daemon`:

```bash
echo '#include "ak_aenc.h"
int main(void) { struct aenc_entry e; (void)e; return 0; }' \
  | gcc -std=gnu99 -Iinclude -x c - -c -o /tmp/list_check.o
```

Expected: exit 0, no errors.

**Step 3: Commit**

```bash
git add cross-compile/vendor-daemon/include/list.h
git commit -m "$(cat <<'EOF'
refactor(vendor-daemon): shrink list.h to struct list_head

ak_aenc.h only needs the list_head field; daemon never walks lists.
EOF
)"
```

---

## Task 2: Trim unused rxi/log APIs (keep the library)

**Files:**
- Modify: `cross-compile/vendor-daemon/src/log.h`
- Modify: `cross-compile/vendor-daemon/src/log.c`
- Verify: `cross-compile/vendor-daemon/src/main.c` still calls
  `log_set_level` / `log_add_fp` / `log_set_quiet`

**Step 1: Confirm current call sites**

```bash
rg -n 'log_(set_lock|level_string|trace|add_callback|add_fp|set_quiet|set_level)' \
  cross-compile/vendor-daemon/src
```

Expected: only `log.c` definitions + `main.c` using set_level / add_fp / quiet.
No `log_trace` / `log_set_lock` / `log_level_string` call sites.

**Step 2: Slim `log.h`**

Keep: `LOG_TRACE`…`LOG_FATAL` enum (indices still used),
`log_debug`…`log_fatal` macros, `log_set_level`, `log_set_quiet`, `log_add_fp`,
`log_log`.

Remove from the public header:
- `#define log_trace(...)`
- `log_level_string`
- `log_set_lock`
- `log_LockFn` typedef (if only used by set_lock)
- `log_add_callback` (make it static in `.c`, or fold into `log_add_fp`)

**Step 3: Slim `log.c`**

- Drop `log_set_lock`, `lock()`/`unlock()` that only call the unset lock, and
  `log_level_string`.
- Reduce callback capacity: either `MAX_CALLBACKS 1` or a single
  `file_fp` / `file_level` pair set by `log_add_fp` (preferred — no array loop).
- Keep `stdout_callback` path behind `!L.quiet` (main sets quiet=true when
  file logging is on).
- Keep MIT copyright block.

Example shape for single-fp path:

```c
static struct {
  int level;
  bool quiet;
  FILE *fp;
  int fp_level;
} L;

int log_add_fp(FILE *fp, int level) {
  L.fp = fp;
  L.fp_level = level;
  return 0;
}

void log_log(int level, const char *file, int line, const char *fmt, ...) {
  /* quiet stderr path + optional L.fp file_callback unchanged */
}
```

**Step 4: Host-compile objects that include log.h**

```bash
cd cross-compile/vendor-daemon
gcc -std=gnu99 -Iinclude -Isrc -c src/log.c -o /tmp/log.o
```

Expected: exit 0.

**Step 5: Commit**

```bash
git add cross-compile/vendor-daemon/src/log.c cross-compile/vendor-daemon/src/log.h
git commit -m "$(cat <<'EOF'
refactor(vendor-daemon): trim unused rxi/log surface

Keep set_level/set_quiet/add_fp; drop lock, level_string, trace, multi-callback.
EOF
)"
```

---

## Task 3: Delete consumer-only ring helpers

**Files:**
- Modify: `cross-compile/vendor-daemon/include/vd_ring_buffer.h`
- Test: `cross-compile/vendor-daemon/tests/test_ring_epoch.c` (already uses
  only `vd_ring_create` / `vd_ring_get_header` — should still pass)

**Step 1: Run host test (baseline green)**

```bash
cd cross-compile/vendor-daemon
gcc -std=gnu99 -Iinclude -o /tmp/test_ring_epoch tests/test_ring_epoch.c
/tmp/test_ring_epoch
```

Expected: `test_ring_epoch: PASS`

**Step 2: Delete these functions and their doxygen from `vd_ring_buffer.h`**

- `vd_ring_open` (~lines 347–393)
- `vd_ring_read` (~lines 510–560)
- `vd_ring_release` (~lines 562–596)
- `vd_ring_is_shutdown` (~lines 642–651)

Keep: layout structs/constants, getters, `vd_ring_new_epoch`, `vd_ring_create`,
`vd_ring_write`, `vd_ring_evict_oldest_pframe`, `vd_ring_reset`,
`vd_ring_shutdown`, `vd_ring_destroy`.

Do **not** remove packed fields (`checksum`, `socket_fallback_count`,
`VD_NOTIFY_SOCKET_FALLBACK`, `VD_STREAM_AUDIO`).

**Step 3: Re-run host test**

Same commands as Step 1.

Expected: `test_ring_epoch: PASS`

**Step 4: Grep for deleted symbols in vendor-daemon**

```bash
rg -n 'vd_ring_(open|read|release|is_shutdown)\b' cross-compile/vendor-daemon
```

Expected: no matches (or only historical comments — prefer zero).

**Step 5: Commit**

```bash
git add cross-compile/vendor-daemon/include/vd_ring_buffer.h
git commit -m "$(cat <<'EOF'
refactor(vendor-daemon): drop consumer ring helpers from C header

Daemon-only surface; Rust shm_ring.rs remains the consumer.
EOF
)"
```

---

## Task 4: Fold VPSS no-ops into dispatcher

**Files:**
- Modify: `cross-compile/vendor-daemon/src/dispatcher.c`
- Delete: `cross-compile/vendor-daemon/src/handlers_vpss.c`
- Delete: `cross-compile/vendor-daemon/src/handlers_vpss.h`

**PERMISSION NOTE:** `AGENTS.md` forbids deleting files without explicit user
permission. Do **not** treat the approved design, this plan, or any conditional
note as permission. Always stop and obtain express written confirmation before
`git rm` / deleting `handlers_vpss.*`. Proceed with the `dispatcher.c` inline
no-ops only as authorized; perform file deletions only after that confirmation.

**Step 1: Inline handlers in `dispatcher.c`**

Remove `#include "handlers_vpss.h"`.

Replace the two cases with:

```c
    case CMD_VPSS_INIT:
    case CMD_VPSS_DESTROY:
        /* libre_anyka_app: no exported ak_vpss_{init,destroy}; VI owns VPSS. */
        ret = send_response(fd, STATUS_OK, NULL, 0);
        break;
```

(Keep `is_lifecycle_cmd` listing `CMD_VPSS_INIT` / `CMD_VPSS_DESTROY`.)

**Step 2: Remove the VPSS module files (only after explicit user permission)**

```bash
git rm cross-compile/vendor-daemon/src/handlers_vpss.c \
       cross-compile/vendor-daemon/src/handlers_vpss.h
```

Makefile uses `$(wildcard $(SRC_DIR)/*.c)` — no Makefile edit needed.

**Step 3: Grep**

```bash
rg -n 'handlers_vpss|handle_vpss_' cross-compile/vendor-daemon
```

Expected: no matches.

**Step 4: Commit**

```bash
git add cross-compile/vendor-daemon/src/dispatcher.c
git commit -m "$(cat <<'EOF'
refactor(vendor-daemon): fold VPSS no-ops into dispatcher

SDK has no ak_vpss_init/destroy exports; drop the empty handler module.
EOF
)"
```

---

## Task 5: Table-driven frame-socket accept

**Files:**
- Modify: `cross-compile/vendor-daemon/src/main.c` (accept loop ~372–442)

**Step 1: Introduce a small channel table near the poll setup**

Keep two sockets and two locks. Collapse duplicated accept into one loop.

Sketch:

```c
struct frame_listen {
    int *server_fd;
    int *client_fd;
    pthread_mutex_t *lock;
    const char *name;   /* "main" / "sub" for logs */
    int poll_idx;       /* explicit fds[] index; do not derive from loop order */
};

struct frame_listen frame_chs[] = {
    { &g_frame_main_server_fd, &g_frame_main_client_fd,
      &g_frame_main_client_lock, "main", 1 },
    { &g_frame_sub_server_fd, &g_frame_sub_client_fd,
      &g_frame_sub_client_lock, "sub", 2 },
};
```

Iterate with `ARRAY_SIZE(frame_chs)`; use `fl->poll_idx` for `revents` / accept.
One accept body: lock → reject if client already set → else install fd → unlock
→ add to poll or roll back.

**Step 2: Confirm control-socket accept path unchanged**

`fds[0]` control accept stays as-is.

**Step 3: Commit**

```bash
git add cross-compile/vendor-daemon/src/main.c
git commit -m "$(cat <<'EOF'
refactor(vendor-daemon): table-driven main/sub frame accept

Same two sockets and locks; drop the copy-pasted accept blocks.
EOF
)"
```

---

## Task 6: Doxygen / comment pass

**Files:**
- Modify: `cross-compile/vendor-daemon/src/handlers_*.c` (not vpss — gone)
- Modify: `cross-compile/vendor-daemon/src/main.c` (file header + dual-path lie)
- Optionally light trim: `ipc.c` / `dispatcher.c` obvious blocks
- **Do not** gut push/globals essays about join / orphan / timeout

**Step 1: Fix stale main.c header claim**

Remove or rewrite the lines that say frame delivery falls back to socket
payload on ring overflow. Reality: ring write + 20-byte notify only; overflow
drops / evicts and may send `VD_NOTIFY_FRAME_DROPPED`.

**Step 2: Compress handler doxygen**

For each `handle_*` in handlers_*.c, replace multi-line `@param fd/req/req_len`
blocks with a one-liner, e.g.:

```c
/* CMD_AI_OPEN — open AI; resp [u64 token]. Wire: rate/bits/ch u32x3. */
```

Keep wire-format notes when non-obvious (OSD payloads, ISP effects).

**Step 3: Commit**

```bash
git add cross-compile/vendor-daemon/src/handlers_*.c \
        cross-compile/vendor-daemon/src/main.c
git commit -m "$(cat <<'EOF'
docs(vendor-daemon): compress handler doxygen; fix overflow comment

Drop essay stubs on thin IPC handlers; correct socket-fallback claim.
EOF
)"
```

---

## Task 7: Full verification

**Step 1: Host ring test**

```bash
cd cross-compile/vendor-daemon
gcc -std=gnu99 -Iinclude -o /tmp/test_ring_epoch tests/test_ring_epoch.c
/tmp/test_ring_epoch
```

Expected: `PASS`

**Step 2: Cross build**

```bash
cd cross-compile/vendor-daemon
make clean && make release
```

Expected: `build/vendor-daemon.bin` linked successfully.

**Step 3: Regression greps**

```bash
rg -n 'handlers_vpss|handle_vpss_|vd_ring_(open|read|release|is_shutdown)\b|log_set_lock|log_level_string|log_trace\(' \
  cross-compile/vendor-daemon
```

Expected: no matches in live code.

**Step 4: Final commit only if verification left dirty files**

If fmt/whitespace only, amend only when hooks auto-touched and commit rules
allow; otherwise new commit or leave clean.

**Step 5: Stop — hand off for review / optional SD smoke**

Optional on-device: start daemon, confirm `/tmp/vd-ctrl.sock`,
`/tmp/vd-frame-main.sock`, `/tmp/vd-frame-sub.sock` exist; send `CMD_VPSS_INIT`
still OK. Not required to close the plan if cross build + host test pass.

---

## Out of scope reminder (do not do)

- Single frame socket
- ABI field / flag removal
- Full log rewrite to fprintf
- `onvif-rust` edits
- Removing `VD_STREAM_AUDIO`
- Merging all `handlers_*.h`
- Deleting wrap/stub files
