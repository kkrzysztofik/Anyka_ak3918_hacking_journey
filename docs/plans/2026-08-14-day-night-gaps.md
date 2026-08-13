# Day/Night Gap Closure Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Find out why our night image is darker than the stock firmware's, and close the four switching-logic gaps we have against the vendor.

**Architecture:** Two independent tracks over one layering rule — the C daemon reads the SDK and does no interpretation, onvif-rust holds all policy and all tests. Track A adds an ISP AE read, diffs it against the sensor config on hardware, and only then changes anything. Track B reimplements the vendor's AWB gate and multi-sample voting on the libraries we already ship, config-gated and defaulted off.

**Tech Stack:** C99 cross-compiled with the vendored `arm-unknown-linux-uclibcgnueabi` toolchain against `libplat_vpss.so` / `libplat_vi.so`; Rust (tokio, async-trait, mockall) targeting `x86_64-unknown-linux-gnu` for tests and ARMv5TE for the camera; Python 3 via `uv run` for host-side analysis.

**Design:** `docs/plans/2026-08-14-day-night-gaps-design.md`
**Background:** `docs/reference/vendor-day-night-implementation.md`

---

## Before you start

Read the design doc. Two things in it are load-bearing and easy to violate:

1. **The `None` rule.** Every signal source has exactly one failure representation, `None`, and `None` always means *hold the current mode*. Never return a value that classifies. This rule was broken once already (a forwarded `-1` would have switched night vision off at night), so treat any new `Option` returned from the HAL as safety-critical.
2. **C does not interpret.** New daemon handlers copy a struct out and send the bytes. Decoding belongs in Rust where the tests are.

**Toolchain setup** — every `cargo` command in this plan needs this on `PATH` first, or clippy dies with E0514:

```bash
export PATH="/home/kmk/dev/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin:$PATH"
```

Run `cargo` from `cross-compile/` (the workspace root), not the repo root.

**Task ordering:** Tasks 1–4 are Track A and must run in order; **Task 4 is a hardware gate that decides whether Task 5 happens at all**. Tasks 6–8 are Track B and are independent of Track A — they may be done in parallel or first.

---

## Track A — night image quality

### Task 1: Add the AE attribute struct to the daemon's header

`struct vpss_isp_ae_attr` is already in our shipped header. This task only verifies that and pins the wire size, because the Rust decoder in Task 3 depends on the exact layout.

**Files:**
- Read: `cross-compile/vendor-daemon/include/ak_vpss.h:127-145`

**Step 1: Confirm the struct and compute its size**

```bash
sed -n '127,145p' cross-compile/vendor-daemon/include/ak_vpss.h
```

Expected: 11 scalar `unsigned long` fields, plus `envi_gain_range[10][2]`, plus `hist_weight[16]`, plus 4 trailing `OE_*` scalars.

On ARMv5TE `unsigned long` is 4 bytes, so the layout is:

| offset (bytes) | field |
|---|---|
| 0 | `exp_time_max` |
| 4 | `exp_time_min` |
| 8 | `d_gain_max` |
| 12 | `d_gain_min` |
| 16 | `isp_d_gain_min` |
| 20 | `isp_d_gain_max` |
| 24 | `a_gain_max` |
| 28 | `a_gain_min` |
| 32 | `exp_step` |
| 36 | `exp_stable_range` |
| 40 | `target_lumiance` |
| 44 | `envi_gain_range[10][2]` (80 bytes) |
| 124 | `hist_weight[16]` (64 bytes) |
| 188 | `OE_suppress_en` |
| 192 | `OE_detect_scope` |
| 196 | `OE_rate_max` |
| 200 | `OE_rate_min` |
| **204** | **total size** |

**Step 2: Verify no padding surprises**

Every field is 4-byte aligned and 4 bytes wide, so the struct is 204 bytes with no padding. Record 204 as the wire contract — Task 3 asserts it.

No commit; this is a reading task.

---

### Task 2: Add `CMD_ISP_GET_AE_ATTR` to the daemon

**Files:**
- Modify: `cross-compile/vendor-daemon/src/protocol.h` (command enum)
- Modify: `cross-compile/vendor-daemon/src/handlers_isp.h`
- Modify: `cross-compile/vendor-daemon/src/handlers_isp.c`
- Modify: `cross-compile/vendor-daemon/src/dispatcher.c`

**Step 1: Add the command ID**

In `protocol.h`, after `CMD_ISP_GET_LUM_FACTOR = 107,`:

```c
    CMD_ISP_GET_AE_ATTR            = 108,
```

**Step 2: Declare the handler**

In `handlers_isp.h`, after the `handle_isp_get_lum_factor` declaration:

```c
int handle_isp_get_ae_attr(int fd, const uint8_t *req, uint32_t req_len);
```

**Step 3: Implement the handler**

Append to `handlers_isp.c`. Note it does no interpretation — it copies the struct out and sends the bytes.

```c
/**
 * handle_isp_get_ae_attr - Return the ISP's live AE attributes verbatim.
 *
 * The profile loaded by a day/night switch sets the exposure and gain ceilings
 * (`a_gain_max` is 24 by day and 10 at night on the gc1084), and this is the
 * only way to see what the ISP actually holds rather than what we asked for.
 * Deliberately uninterpreted: the caller decodes. See
 * docs/reference/vendor-day-night-implementation.md.
 *
 * Empty request. Response payload: struct vpss_isp_ae_attr = 204 bytes.
 */
int handle_isp_get_ae_attr(int fd, const uint8_t *req, uint32_t req_len)
{
    void *vi;
    struct vpss_isp_ae_attr attr;

    (void)req;
    (void)req_len;

    if (isp_first_vi(&vi) != 0) {
        log_warn("[isp] get_ae_attr: no VI registered");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    memset(&attr, 0, sizeof(attr));
    if (ak_vpss_isp_get_ae_attr(vi, &attr) != 0) {
        log_warn("[isp] get_ae_attr: ak_vpss_isp_get_ae_attr failed");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    log_debug("[isp] get_ae_attr vi=%p a_gain_max=%lu exp_time_max=%lu target_lum=%lu",
              vi, attr.a_gain_max, attr.exp_time_max, attr.target_lumiance);
    return send_response(fd, STATUS_OK, &attr, sizeof(attr));
}
```

**Step 4: Wire the dispatcher**

In `dispatcher.c`, after the `CMD_ISP_GET_LUM_FACTOR` case:

```c
    case CMD_ISP_GET_AE_ATTR:
        ret = handle_isp_get_ae_attr(fd, req_buf, req_len);
        break;
```

**Step 5: Build**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/vendor-daemon && make
```

Expected: compiles and links with no warnings. `-Wall -Wextra` is on; any warning is a defect.

**Step 6: Verify the symbol resolves**

```bash
/home/kmk/dev/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin/arm-unknown-linux-uclibcgnueabi-nm \
  -D build/vendor-daemon.bin | grep ak_vpss_isp_get_ae_attr
```

Expected: `U ak_vpss_isp_get_ae_attr` — undefined at link, resolved at runtime from `libplat_vpss.so`, same as the existing `ak_vpss_isp_get_ae_run_info`.

**Step 7: Commit**

```bash
git add cross-compile/vendor-daemon/src/
git commit -m "feat(vendor-daemon): expose live ISP AE attributes over IPC"
```

---

### Task 3: Decode the AE attributes in Rust

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/anyka/ipc/mod.rs` (command constant + name)
- Modify: `cross-compile/onvif-rust/src/hal/common/imaging.rs` (trait method + struct)
- Modify: `cross-compile/onvif-rust/src/hal/anyka/ipc/imaging.rs` (impl + tests)
- Modify: `cross-compile/onvif-rust/src/hal/stub/imaging.rs` (stub impl)

**Step 1: Write the failing test**

In `cross-compile/onvif-rust/src/hal/anyka/ipc/imaging.rs`, inside `mod tests`:

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_ae_attr_decodes_gain_and_exposure_ceilings() {
    // A 204-byte struct of little-endian u32. Field offsets are load-bearing:
    // reading a_gain_max at the wrong offset silently returns d_gain_min, which
    // is a plausible-looking number and would send the whole investigation wrong.
    let mut payload = vec![0u8; 204];
    payload[0..4].copy_from_slice(&2250u32.to_le_bytes()); // exp_time_max
    payload[24..28].copy_from_slice(&10u32.to_le_bytes()); // a_gain_max
    payload[40..44].copy_from_slice(&40u32.to_le_bytes()); // target_lumiance

    let daemon = FakeDaemon::start(move |cmd_id, req| {
        assert_eq!(cmd_id, CMD_ISP_GET_AE_ATTR);
        assert!(req.is_empty());
        (AK_SUCCESS_I32, payload.clone())
    });
    let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
    ipc.set_epochs_for_test(1, 1);

    let attr = <AnykaIpc as ImagingHalTrait>::get_ae_attr(&ipc)
        .await
        .expect("attr");
    assert_eq!(attr.exp_time_max, 2250);
    assert_eq!(attr.a_gain_max, 10);
    assert_eq!(attr.target_lumiance, 40);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_ae_attr_wrong_length_is_none() {
    // A short payload means a daemon/struct mismatch. Decoding it would produce
    // silently wrong ceilings, so reject rather than pad.
    let daemon = FakeDaemon::start(|_c, _r| (AK_SUCCESS_I32, vec![0u8; 200]));
    let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
    ipc.set_epochs_for_test(1, 1);
    assert_eq!(<AnykaIpc as ImagingHalTrait>::get_ae_attr(&ipc).await, None);
}
```

**Step 2: Run to verify it fails**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile
export PATH="/home/kmk/dev/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin:$PATH"
cargo test --target x86_64-unknown-linux-gnu -p onvif-rust get_ae_attr
```

Expected: compile error — `get_ae_attr` not found on the trait.

**Step 3: Add the constant**

In `hal/anyka/ipc/mod.rs`, after `CMD_ISP_GET_LUM_FACTOR`:

```rust
const CMD_ISP_GET_AE_ATTR: i32 = 108;
```

and in the command-name match, after the `CMD_ISP_GET_LUM_FACTOR` arm:

```rust
            CMD_ISP_GET_AE_ATTR => "ISP_GET_AE_ATTR",
```

**Step 4: Add the public struct and trait method**

In `hal/common/imaging.rs`, above the trait:

```rust
/// Live ISP auto-exposure limits, as reported by the sensor profile in force.
///
/// Mirrors `struct vpss_isp_ae_attr` (204 bytes, ARMv5TE). Only the fields we
/// have a use for are decoded; the rest of the struct is carried by the daemon
/// but not modelled here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AeAttr {
    pub exp_time_max: u32,
    pub exp_time_min: u32,
    pub d_gain_max: u32,
    pub a_gain_max: u32,
    pub target_lumiance: u32,
}

/// Wire size of `struct vpss_isp_ae_attr` on the camera.
pub(crate) const AE_ATTR_WIRE_LEN: usize = 204;
```

and inside the trait, after `get_lum_factor`:

```rust
    /// Live ISP AE limits, or `None` if unavailable.
    async fn get_ae_attr(&self) -> Option<AeAttr>;
```

**Step 5: Implement for AnykaIpc**

In `hal/anyka/ipc/imaging.rs`, add to the impl block:

```rust
    async fn get_ae_attr(&self) -> Option<AeAttr> {
        fn le_u32(d: &[u8], off: usize) -> u32 {
            u32::from_le_bytes([d[off], d[off + 1], d[off + 2], d[off + 3]])
        }
        match self.request_async(CMD_ISP_GET_AE_ATTR, &[]).await {
            Ok((status, data)) if status == AK_SUCCESS_I32 && data.len() == AE_ATTR_WIRE_LEN => {
                Some(AeAttr {
                    exp_time_max: le_u32(&data, 0),
                    exp_time_min: le_u32(&data, 4),
                    d_gain_max: le_u32(&data, 8),
                    a_gain_max: le_u32(&data, 24),
                    target_lumiance: le_u32(&data, 40),
                })
            }
            Ok((status, data)) => {
                error!(status, len = data.len(), "get_ae_attr bad daemon response");
                None
            }
            Err(e) => {
                error!(error = %e, "get_ae_attr IPC failed");
                None
            }
        }
    }
```

Add `AE_ATTR_WIRE_LEN` and `AeAttr` to the `use crate::hal::common::imaging::...` import, and `CMD_ISP_GET_AE_ATTR` to the `use super::{...}` list.

**Step 6: Implement for the stub**

In `hal/stub/imaging.rs`:

```rust
    async fn get_ae_attr(&self) -> Option<crate::hal::common::imaging::AeAttr> {
        None
    }
```

**Step 7: Run tests to verify they pass**

```bash
cargo test --target x86_64-unknown-linux-gnu -p onvif-rust get_ae_attr
```

Expected: 2 passed.

**Step 8: Run the full suite for regressions**

```bash
cargo test --target x86_64-unknown-linux-gnu -p onvif-rust
```

Expected: all pass (2453+ at time of writing). `MockImagingHalTrait` gains a method, so any test constructing one and calling a path that reaches `get_ae_attr` needs an expectation — there should be none, since nothing calls it yet.

**Step 9: Surface it in diagnostics**

In `cross-compile/onvif-rust/src/platform/common/traits.rs`, add to `VisionDiagnostics` after `ae_luma`:

```rust
    /// Live ISP AE ceilings, or `None` if unavailable. Present so a night image
    /// complaint can be diagnosed without a redeploy.
    pub ae_a_gain_max: Option<u32>,
    pub ae_exp_time_max: Option<u32>,
    pub ae_target_lumiance: Option<u32>,
```

Populate them in `NightModeController::live_diagnostics` in `platform/anyka/night_mode.rs` by adding `self.ffi.get_ae_attr()` to the existing `tokio::join!`, and set the three fields from it. Fix any other `VisionDiagnostics` construction sites the compiler flags.

**Step 10: Run the full suite again, then commit**

```bash
cargo test --target x86_64-unknown-linux-gnu -p onvif-rust
git add cross-compile/onvif-rust/src/
git commit -m "feat(onvif-rust): decode live ISP AE limits and surface in diagnostics"
```

---

### Task 4: HARDWARE GATE — capture and diff

**This task produces no code. Its output decides whether Task 5 exists.**

**Files:**
- Create: `scripts/debugging/isp_conf_diff.py`

**Step 1: Write the conf parser with its self-check**

Create `scripts/debugging/isp_conf_diff.py`. It parses `isp_gc1084.conf` into modules and prints the `ISP_EXP` values for a given mode block.

```python
#!/usr/bin/env python3
"""Parse an Anyka isp_<sensor>.conf and report per-mode ISP module contents.

The file is N mode blocks of equal size; each block is a header followed, from
offset 0x200, by a flat sequence of [u16 module_id][u16 module_len][data]
records walked by isp_switch_mode(). See
docs/reference/vendor-day-night-implementation.md.
"""

from __future__ import annotations

import struct
import sys

MODULE_TABLE_START = 0x200
NAMES = {
    0: "BB", 1: "LSC", 2: "RAW_LUT", 3: "NR", 4: "3DNR", 5: "GB", 6: "DEMOSAIC",
    7: "GAMMA", 8: "CCM", 9: "FCS", 10: "WDR", 11: "SHARP", 12: "SATURATION",
    13: "CONTRAST", 14: "RGB2YUV", 15: "YUVEFFECT", 16: "DPC", 17: "WEIGHT",
    18: "AF", 19: "WB", 20: "EXP", 21: "MISC", 22: "Y_GAMMA", 23: "HUE",
    28: "SENSOR",
}


def parse_modules(block: bytes) -> list[tuple[int, int, int]]:
    """Return [(module_id, offset, length)] for one mode block."""
    out, off = [], MODULE_TABLE_START
    while off + 4 <= len(block):
        mid, mlen = struct.unpack_from("<HH", block, off)
        if mlen < 4 or off + mlen > len(block) or mid not in NAMES:
            break
        out.append((mid, off, mlen))
        off += mlen
    return out


def split_blocks(data: bytes, count: int = 3) -> list[bytes]:
    size = len(data) // count
    if size * count != len(data):
        raise ValueError(f"{len(data)} bytes does not divide into {count} blocks")
    return [data[i * size : (i + 1) * size] for i in range(count)]


def demo() -> None:
    """Self-check: the shipped conf must parse to a complete module table."""
    import pathlib

    path = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "/tmp/isp_gc1084.conf")
    data = path.read_bytes()
    blocks = split_blocks(data)
    mods = parse_modules(blocks[0])
    assert len(mods) == 25, f"expected 25 modules, got {len(mods)}"
    end = mods[-1][1] + mods[-1][2]
    assert end <= len(blocks[0]) and len(blocks[0]) - end < 16, (
        f"module table ends at 0x{end:05x}, block is 0x{len(blocks[0]):05x}"
    )
    print(f"OK: {len(blocks)} blocks x {len(blocks[0])} bytes, {len(mods)} modules")

    for name, blk in (("day", blocks[0]), ("night", blocks[1])):
        for mid, off, mlen in parse_modules(blk):
            if NAMES[mid] == "EXP":
                body = blk[off + 4 : off + mlen]
                words = struct.unpack_from("<%dI" % (len(body) // 4), body)
                print(f"{name:6s} EXP first 12 words: {words[:12]}")


if __name__ == "__main__":
    demo()
```

**Step 2: Run the self-check**

```bash
cd /home/kmk/dev/anyka-dev
uv run python3 scripts/debugging/isp_conf_diff.py /tmp/isp_gc1084.conf
```

Expected: `OK: 3 blocks x 34746 bytes, 25 modules`, then the day and night `EXP` words.

If `/tmp/isp_gc1084.conf` is absent, pull it first:

```bash
uv run python3 scripts/debugging/cam_exec.py --host 127.0.0.1 --port 12321 \
  'nc -l -p 9932 < /data/sensor/isp_gc1084.conf &'
ssh root@192.168.3.137 'nc 192.168.30.121 9932 > /tmp/isp_gc1084.conf'
scp root@192.168.3.137:/tmp/isp_gc1084.conf /tmp/isp_gc1084.conf
```

`.121` is jumphost-only. Open both tunnels before any hardware step:

```bash
ssh -N -L 12321:192.168.30.121:24   root@192.168.3.137 &   # telnet, for cam_exec.py
ssh -N -L 12180:192.168.30.121:8080 root@192.168.3.137 &   # HTTP-FLV, for frame capture
```

**Step 3: Commit the script**

```bash
git add scripts/debugging/isp_conf_diff.py
git commit -m "test(debugging): add ISP conf module parser with self-check"
```

**Step 4: Deploy the new daemon and onvif-rust to `.121`**

Use the project's normal deploy path (`scripts/deploy_onvif.sh` / the slot bundle flow). Confirm the supervisor restarted — the memory note applies: config knobs only take effect on a *restarted* supervisor, so a redeploy alone proves nothing.

**Step 5: Capture the AE attr in both modes**

With the camera in night mode (AUTO, after dark), read diagnostics and record `ae_a_gain_max`, `ae_exp_time_max`, `ae_target_lumiance`. Then force day with `SetImagingSettings IrCutFilter=ON` and capture again. Restore `AUTO` afterwards.

**Step 6: Compare and decide**

| observation | meaning | next |
|---|---|---|
| night attr matches night `EXP` conf (`a_gain_max` ≈ 10) | switch is complete; the profile is the ceiling | **do Task 5** |
| night attr matches *day* conf while mode says night | the switch is not sticking | **stop**; new investigation, this plan does not apply |
| attr matches neither | something reprograms AE behind us | **stop**; find the writer first |

**Step 7: Record the finding**

Append the captured values and the decision to `docs/reference/vendor-day-night-implementation.md` §8, and commit. This is the evidence Task 5 is justified by — do not skip it.

---

### Task 5: CONDITIONAL — override the night AE ceiling

**Only if Task 4 Step 6 selected "do Task 5".**

**Files:**
- Modify: `cross-compile/vendor-daemon/src/{protocol.h,handlers_isp.h,handlers_isp.c,dispatcher.c}`
- Modify: `cross-compile/onvif-rust/src/config/types.rs`
- Modify: `cross-compile/onvif-rust/src/hal/{common,anyka/ipc,stub}/imaging.rs`
- Modify: `cross-compile/onvif-rust/src/platform/anyka/night_mode.rs`

**Step 1: Add `CMD_ISP_SET_AE_ATTR = 109` to the daemon**

The handler must **read-modify-write**. Constructing a fresh struct would zero `hist_weight`, `envi_gain_range` and `target_lumiance` — worse than a dark image.

```c
/**
 * handle_isp_set_ae_attr - Override selected AE ceilings, preserving the rest.
 *
 * Wire format: [i32 a_gain_max][i32 exp_time_max], each <= 0 meaning "leave
 * alone". Read-modify-write is mandatory: the struct carries hist_weight[16],
 * envi_gain_range[10][2] and target_lumiance, none of which we model, and a
 * fresh struct would zero them.
 */
int handle_isp_set_ae_attr(int fd, const uint8_t *req, uint32_t req_len)
{
    void *vi;
    struct vpss_isp_ae_attr attr;
    int32_t a_gain_max, exp_time_max;

    if (req_len < 8)
        return send_response(fd, STATUS_ERROR, NULL, 0);
    if (isp_first_vi(&vi) != 0) {
        log_warn("[isp] set_ae_attr: no VI registered");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    memset(&attr, 0, sizeof(attr));
    if (ak_vpss_isp_get_ae_attr(vi, &attr) != 0) {
        log_warn("[isp] set_ae_attr: read failed; not writing");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    a_gain_max = req_read_i32(req, 0);
    exp_time_max = req_read_i32(req, 4);
    if (a_gain_max > 0)
        attr.a_gain_max = (unsigned long)a_gain_max;
    if (exp_time_max > 0)
        attr.exp_time_max = (unsigned long)exp_time_max;

    log_debug("[isp] set_ae_attr a_gain_max=%lu exp_time_max=%lu",
              attr.a_gain_max, attr.exp_time_max);
    return send_response(fd, ak_vpss_isp_set_ae_attr(vi, &attr), NULL, 0);
}
```

Wire the dispatcher and header as in Task 2. Build and commit.

**Step 2: Add config keys with a failing validation test**

In `cross-compile/onvif-rust/src/config/types.rs`, add to `NightConfig`:

```rust
    /// Override the night profile's analogue gain ceiling. `None` leaves the
    /// profile's own value. The night block ships 10 where day allows 24.
    pub night_a_gain_max: Option<i32>,
    /// Override the night profile's max exposure time. `None` leaves it.
    pub night_exp_time_max: Option<i32>,
```

Default both to `None`. Add validation rejecting values `<= 0`, and a test asserting a zero override is rejected.

**Step 3: Add the HAL method, stub, and a read-modify-write test**

Mirror Task 3. The test that earns its place: setting only `a_gain_max` must leave `exp_time_max` unchanged.

**Step 4: Re-apply after each night transition**

In `NightModeController::apply`, after the ISP switch succeeds and only for `DayNight::Night`, apply the configured overrides. `isp_switch_mode` reloads the AE module on every switch, so this must run every time, not once at startup.

Add a test: applying Night twice issues the override twice.

**Step 5: Full suite, then commit**

```bash
cargo test --target x86_64-unknown-linux-gnu -p onvif-rust
git commit -m "feat(night): allow overriding the night AE ceiling from config"
```

**Step 6: Validate on hardware**

Deploy, set `night_a_gain_max = 24`, restart the supervisor, wait for night, capture a frame and measure:

```bash
ffmpeg -y -loglevel error -i "http://admin:admin@127.0.0.1:12180/live/main.flv" \
  -frames:v 1 -q:v 2 /tmp/after.jpg
ffmpeg -loglevel info -i /tmp/after.jpg \
  -vf "signalstats,metadata=print:key=lavfi.signalstats.YAVG" -f null - 2>&1 | grep YAVG
```

**Allow 30 s of settle after any mode or lamp change before capturing.** AE re-ramps, and an early grab reads low enough to invert the comparison — this has already caused one wrong conclusion. Compare against the baseline below.

**`.121` night baseline, captured 2026-08-14 01:20 local:**

| metric | value |
|---|---|
| `YAVG` | 6.51 |
| `SATAVG` | 0 (true monochrome) |
| `IR_LED` | 1 |
| `WHITE_LED` | **1** |
| AE luma | 3–4, occasional 16 |

Note the white lamp is already on and the frame is still black — on `.146` the same lamp produced `YAVG 110`. Do not assume `.121`'s illumination is comparable; the scene is the variable, so only compare `.121` against `.121`.

---

## Track B — switching gaps

Independent of Track A. Both features are config-gated and **default off**, so deploying them cannot regress switching that currently works.

### Task 6: Expose AWB statistics from the daemon

**Files:**
- Modify: `cross-compile/vendor-daemon/include/ak_vpss.h` (add the struct)
- Modify: `cross-compile/vendor-daemon/src/{protocol.h,handlers_isp.h,handlers_isp.c,dispatcher.c}`

**Step 1: Add the AWB stat struct to our header**

Copy `struct vpss_isp_awb_stat_info` from `cross-compile/anyka_reference/platform/libplat/include/ak_vpss.h:292` into `cross-compile/vendor-daemon/include/ak_vpss.h`. We only consume `total_cnt[10]`, but the struct must be complete or `isp_get_statinfo` writes past our buffer.

**Step 2: Declare `isp_get_statinfo`**

No header we ship declares it, but `libplat_vi.so` exports it (`T isp_get_statinfo`). Declare it in `handlers_isp.c` alongside the existing `isp_get_cur_lum_factor` extern, with the same provenance comment. Prototype from `anyka_reference/platform/libplat/src/include/isp_basic.h:100`:

```c
extern int isp_get_statinfo(int module_id, void *buf, unsigned int *size);
```

`ISP_AWBSTAT` is module id **27** (`isp_basic.h:11` enum, counting from `ISP_BB = 0`). Define it locally with a comment rather than vendoring the whole enum.

**Step 3: Implement `CMD_ISP_GET_AWB_STAT = 110`**

Empty request. Response: the 10 `total_cnt` values as `[i32 x 10]` = 40 bytes. This is the one place C narrows the struct, and it is a copy not an interpretation.

**Step 4: Build, verify symbols resolve, commit**

```bash
cd cross-compile/vendor-daemon && make
git commit -m "feat(vendor-daemon): expose AWB colour-bin statistics over IPC"
```

---

### Task 7: AWB gate in the night controller

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/{common,anyka/ipc,stub}/imaging.rs`
- Modify: `cross-compile/onvif-rust/src/config/types.rs`
- Modify: `cross-compile/onvif-rust/src/platform/anyka/night_mode.rs`

**Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn test_awb_gate_holds_when_colour_bins_disagree() {
    // The gate is the whole point: a confident dark luminance reading must not
    // switch the camera if the colour statistics still look like daylight.
    // ... lum factor says night, awb bins say day, assert current_mode is None
}

#[tokio::test]
async fn test_awb_gate_holds_when_statistics_are_unavailable() {
    // Unavailable statistics are not consent. Matches the vendor, whose
    // night_mode_cmp_awb returns AK_FAILED and so fails its == STATE_DAY test.
}

#[tokio::test]
async fn test_awb_gate_disabled_by_default_does_not_block() {
    // Default-off: existing behaviour must be untouched until explicitly enabled.
}
```

**Step 2: Run to verify they fail**

**Step 3: Add config**

```rust
    /// Require the AWB colour statistics to agree before switching. Off by
    /// default: enable per-camera after validating across one dusk.
    pub awb_gate_enabled: bool,
    /// Per-bin night thresholds; the vendor's `[autoir]` night_cnt values.
    pub night_cnt: [i32; 5],
    /// Per-bin day thresholds; the vendor's `[autoir]` day_cnt values.
    pub day_cnt: [i32; 10],
```

Defaults: `false`, `[1200; 5]`, `[600_000; 10]`.

**Step 4: Implement the gate**

Add `get_awb_stat` to the HAL trait and both impls, then a `fn awb_agrees(&self, stats, target) -> Option<bool>` on the controller. In `tick`, between classification and `resolve`, hold when the gate is enabled and `awb_agrees` is `None` or `Some(false)`.

**Step 5: Run the tests, then the full suite**

**Step 6: Commit**

```bash
git commit -m "feat(night): gate day/night transitions on AWB colour statistics"
```

---

### Task 8: N-sample voting with early lock release

**Files:**
- Modify: `cross-compile/onvif-rust/src/config/types.rs`
- Modify: `cross-compile/onvif-rust/src/platform/anyka/night_mode.rs`

**Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn test_vote_commits_when_all_samples_agree() { }

#[tokio::test]
async fn test_vote_holds_when_samples_split() {
    // An off-by-one in the counter silently disables voting, and the camera
    // goes back to switching on a single sample without any test noticing.
}

#[tokio::test]
async fn test_vote_with_one_unavailable_sample_is_inconclusive() {
    // The tempting bug is to skip the failed sample and vote on the other two.
    // Three samples where one failed is not evidence.
}

#[tokio::test]
async fn test_unanimous_vote_releases_the_lock_early() {
    // Otherwise B3 is dead code that looks alive: the lock would still run to
    // its full lock_time_ms and nothing would fail.
}
```

**Step 2: Run to verify they fail**

**Step 3: Add config**

```rust
    /// Samples per decision. 1 keeps today's single-sample behaviour; the
    /// vendor uses 3 (`CHECK_TIME`).
    pub vote_samples: u8,
```

Default `1` — off, per the design. Validation: reject `0`.

**Step 4: Implement**

Extract the per-tick sampling into a `vote()` returning `Option<DayNight>`: sample `vote_samples` times from the chosen source, return `None` if any sample is unavailable or the samples disagree. Do not mix sources within a vote — the source is chosen once per tick, and lum-factor and AE-luma have opposite polarity.

Thread unanimity into `decide` so a unanimous vote contradicting the current mode bypasses the remaining `lock_time_ms`.

**Step 5: Run the tests, then the full suite**

**Step 6: Commit**

```bash
git commit -m "feat(night): require unanimous multi-sample votes to switch"
```

---

### Task 9: Hardware validation of Track B

**Step 1:** Deploy with both flags off. Confirm `night sample` lines are unchanged from the pre-deploy baseline.

**Step 2:** Enable `vote_samples = 3` alone. Restart the supervisor. Watch one dusk.

**Step 3:** Enable `awb_gate_enabled = true`. Restart. Watch one dusk.

**Step 4:** Record both dusk timings in `docs/reference/vendor-day-night-implementation.md` §8 and commit.

One flag at a time — a combined enable that misbehaves gives no information about which mechanism caused it.

---

## Out of scope

- `WHITE_LED` as a night illuminator (excluded by request).
- The `profiles.toml` fps mismatch (declares 30, sensor delivers 15 day / 10 night).
- Swapping `libplat_vpss.so`.
- Vendoring `ak_isp_sdk.h` for sharp/WDR/BLC/CCM/NR attrs — the escape hatch if Task 4 shows AE is unremarkable.
