# ISP Sole-VI Wire Format Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make ISP effect and IR day/night IPC cmds resolve the sole live VI from an `i32`-only payload so video actually switches to night (not just GPIO LEDs).

**Architecture:** Daemon-only hard cut. Reuse the `get_ae_luma` sole-VI lookup in `handle_isp_effect` and `handle_isp_set_ir_filter`. Rust already sends 4-byte values — no HAL/platform changes.

**Tech Stack:** vendor-daemon C, vendored ARM toolchain via `make -C cross-compile/vendor-daemon release`, deploy to `.198`.

**Design:** `docs/plans/2026-08-04-isp-sole-vi-wire-format-design.md`

---

### Task 1: File-local `isp_first_vi` + hard-cut `handle_isp_effect`

**Files:**
- Modify: `cross-compile/vendor-daemon/src/handlers_isp.c`

**Step 1: No C unit harness — implement, verify with `make release`**

**Step 2: Add file-local helper** (above the handlers):

```c
/* ponytail: sole-VI, pass token only if multi-VI appears. */
static int isp_first_vi(void **out)
{
    int i;
    for (i = 0; i < VD_OBJ_SLOTS; i++) {
        if (g_obj_slots[i].live && g_obj_slots[i].kind == VD_OBJ_KIND_VI) {
            *out = g_obj_slots[i].ptr;
            return 0;
        }
    }
    return -1;
}
```

**Step 3: Rewrite `handle_isp_effect`**

Wire format comment → `[i32 value] = 4 bytes`.

```c
int handle_isp_effect(int fd, const uint8_t *req, uint32_t req_len,
                      int effect_type, const char *name)
{
    void *vi_handle;
    int32_t value;
    int ret;

    if (req_len != 4) {
        log_warn("[isp] %s: req too short (%u)", name, req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    if (isp_first_vi(&vi_handle) != 0) {
        log_warn("[isp] %s: no VI registered", name);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    value = req_read_i32(req, 0);
    log_debug("[isp] %s vi=%p value=%d", name, vi_handle, (int)value);
    ret = ak_vpss_effect_set(vi_handle, (enum vpss_effect_type)effect_type, (int)value);
    return send_response(fd, ret, NULL, 0);
}
```

Remove `vd_obj_resolve` for this path.

**Step 4: Fold `get_ae_luma` VI loop onto `isp_first_vi`** (same file, keep behaviour).

**Step 5: Build**

```bash
make -C cross-compile/vendor-daemon release
```

Expected: success.

**Step 6: Commit**

```bash
git add cross-compile/vendor-daemon/src/handlers_isp.c
git commit -m "fix(vendor-daemon): ISP effects use sole-VI i32 payload"
```

---

### Task 2: Hard-cut `handle_isp_set_ir_filter`

**Files:**
- Modify: `cross-compile/vendor-daemon/src/handlers_isp.c`

**Step 1: Rewrite IR handler**

Wire format comment → `[i32 mode] = 4 bytes` (0 day, 1 night).

```c
int handle_isp_set_ir_filter(int fd, const uint8_t *req, uint32_t req_len)
{
    void *vi_handle;
    int32_t mode;
    enum video_daynight_mode dn;
    int ret;

    if (req_len != 4)
        return send_response(fd, STATUS_ERROR, NULL, 0);
    if (isp_first_vi(&vi_handle) != 0) {
        log_warn("[isp] set_ir_filter: no VI registered");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    mode = req_read_i32(req, 0);
    dn = (mode != 0) ? VI_MODE_NIGHT : VI_MODE_DAY;
    log_debug("[isp] set_ir_filter vi=%p mode=%d", vi_handle, (int)dn);
    ret = ak_vi_switch_mode(vi_handle, dn);
    return send_response(fd, ret, NULL, 0);
}
```

**Step 2: Build**

```bash
make -C cross-compile/vendor-daemon release
```

Expected: success.

**Step 3: Commit**

```bash
git add cross-compile/vendor-daemon/src/handlers_isp.c
git commit -m "fix(vendor-daemon): set_ir_filter sole-VI i32 hard cut"
```

---

### Task 3: Deploy vendor-daemon to `.198` and verify

**Step 1: Install + transfer**

```bash
make -C cross-compile/vendor-daemon release install
# nc + camera_shell.py → /tmp/vendor-daemon.bin.new →
# killall both bins, mv into /mnt/anyka_hack/vendor-daemon/, wait for respawn
```

**Step 2: Force night**

ONVIF `SetImagingSettings` `IrCutFilter=OFF` on token `VideoSource_1` (admin/admin).

Expect: `IR_LED=1`, stream looks IR/night (not day colour under IR glow).  
Expect: no new `ISP day/night switch failed … isp=-1` while attached with live VI.

**Step 3: Force day**

`IrCutFilter=ON` → `IR_LED=0`, colour day look returns.

**Step 4: Optional** brightness tweak via imaging — visible change.

**Step 5: Commit** nothing (binaries stay untracked). Note results in the final handoff.

---

### Task 4: Ponytail-review the daemon diff

Diff `handlers_isp.c` vs design. Cut anything beyond sole-VI + hard cut. Commit shrinks if any.

---

## Execution handoff

After this plan is saved and committed, use **executing-plans** or implement task-by-task. Do not expand into attach-flap or multi-VI tokens unless a follow-up plan is written.
