# JUAN Board Support Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** The JUAN camera (192.168.2.148) cold-boots into anyka-init, which brings wifi
up itself and supervises vendor-daemon and onvif-rust, as on the fleet.

**Architecture:** This is approach A from `2026-09-28-juan-board-support-design.md`: probe each
seam, add no board concept and no new config keys.
- **Code.** Phase 1 makes three probe-style changes to anyka-init `wifi.rs`: an `atbm6031x`
  chip row, a `gpio-wifi_power` fallback, and a `/usr/bin/wpa_supplicant` fallback. It adds
  the same supplicant fallback to `kill-wpa.sh`.
- **Hook.** A JUAN hook script does three prep lines, then runs the fleet's own
  `Factory/config.sh` unchanged. Env overrides point its deadman at a stock-passthrough script.
- **Config.** Everything else JUAN-specific uses existing per-device `anyka.toml` keys.

**Tech Stack:** Rust (anyka-init, mockall `MockSys`, tempdir tests), busybox sh, the vendored
cargo at `toolchain/arm-anykav200-crosstool-ng/bin/cargo`.

---

## Ground rules for the executor

- Host tests run from `cross-compile/`:
  `../toolchain/arm-anykav200-crosstool-ng/bin/cargo test -p anyka-init --target x86_64-unknown-linux-gnu --lib wifi::`
  Baseline at plan time: **46 passed**.
- ARM builds go through `./scripts/build_payload.sh`. Never build ARM from the workspace
  root (memory: arm-build-needs-crate-dir).
- The git index here is usually fully staged. **Always commit with an explicit pathspec**,
  e.g. `git commit -m "…" -- path1 path2`. A bare `git commit` sweeps in unrelated work.
- Camera shell: `{ sleep 2; printf 'cmd\n'; sleep 3; printf 'exit\n'; sleep 1; } | timeout 30 telnet 192.168.2.148 24`.
  No login. Pace the input.
- `python3 -c` is hooked. Write a script file and run it with `/usr/bin/python3`.
- The JUAN busybox has **no `nc`, `tar` or usable FTP**; anonymous `ftpd` returns 530 and the
  root password is unknown. Files reach the card by `wget` from a temporary host HTTP server
  (Task 7).

---

## Phase 1: boot chain

### Task 1: `atbm6031x` chip row

**Files:**
- Modify: `cross-compile/anyka-init/src/wifi.rs` (`Chip::ALL`, ~line 29; tests module ~line 855)

**Step 1: Write the failing test.** Add it after `test_chip_from_hw_char_h_is_ssv6355`:

```rust
    #[test]
    fn test_chip_from_name_atbm6031x_loads_from_usr_modules() {
        let c = Chip::from_name("atbm6031x").expect("atbm6031x is a known chip");
        // JUAN rootfs ships the module in /usr/modules; there is no tgz to unpack.
        assert_eq!(c.module, "/usr/modules/atbm6031x.ko");
        // Read off the live board: /sys/module/atbm6031x/parameters/wifi_bt_comb = 1.
        assert_eq!(c.args, "wifi_bt_comb=1");
        assert_eq!(c.rmmod, "atbm6031x");
    }
```

**Step 2: Run it and confirm it fails.**
Run: `../toolchain/arm-anykav200-crosstool-ng/bin/cargo test -p anyka-init --target x86_64-unknown-linux-gnu --lib atbm6031x`
Expected: FAIL, `atbm6031x is a known chip`.

**Step 3: Add the row.** Append it to `Chip::ALL` after `ssv6355_ble`:

```rust
        // JUAN board. No hw.conf there, so it is reachable only by a pinned
        // `[wifi] chip`, never through from_hw_char.
        Chip {
            name: "atbm6031x",
            module: "/usr/modules/atbm6031x.ko",
            args: "wifi_bt_comb=1",
            rmmod: "atbm6031x",
            settle: Duration::ZERO,
        },
```

Do **not** add a `from_hw_char` mapping.

**Step 4: Run the wifi tests.** Same command with `--lib wifi::`.
Expected: 47 passed. `test_chip_from_name_round_trips_every_entry` covers the new row automatically.

**Step 5: Commit.**
```bash
git add cross-compile/anyka-init/src/wifi.rs
git commit -m "feat(anyka-init): atbm6031x chip row for the JUAN board" -- cross-compile/anyka-init/src/wifi.rs
```

---

### Task 2: power GPIO falls back to `gpio-wifi_power`

On JUAN, `/sys/user-gpio/wifi_en` does not exist. Sysfs refuses file creation, so step 3's
`fs::write` errors and bring-up fails. The node is named `gpio-wifi_power` there, and it reads
`1` while wifi is up. So `low_high` (0, then 1) is the right polarity, and the order is a
calibration knob in `anyka.toml`.

**Files:**
- Modify: `cross-compile/anyka-init/src/wifi.rs`, in these places:
  - consts (~line 312)
  - `FsLayout` struct and `production()` (~354-392)
  - step 3 in `try_bring_up_with` (~571-576)
  - `test_layout` (~816)

**Step 1: Write the failing test.** Add it after `test_try_bring_up_with_happy_path_over_dhcp`:

```rust
    #[test]
    fn test_try_bring_up_with_uses_gpio_wifi_power_when_wifi_en_is_absent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let layout = test_layout(dir.path());
        std::fs::create_dir_all(format!("{}/wlan0", layout.sys_class_net)).expect("iface dir");
        std::fs::write(format!("{}/wlan0/carrier", layout.sys_class_net), "1").expect("carrier");
        std::fs::write(&layout.proc_route, HAPPY_ROUTE).expect("route");
        std::fs::write(&layout.proc_fib_trie, HAPPY_FIB_TRIE).expect("fib_trie");
        // JUAN: only the gpio- prefixed node exists.
        std::fs::write(&layout.gpio_wifi_power, "1\n\0").expect("power node");

        try_bring_up_with(&happy_mock_sys(), &happy_cfg(), &layout).expect("bring-up must succeed");

        let last = std::fs::read_to_string(&layout.gpio_wifi_power).expect("power node");
        assert_eq!(last, "1", "low_high must leave the radio powered");
        assert!(
            !std::path::Path::new(&layout.gpio_wifi_en).exists(),
            "must not invent the fleet node beside the JUAN one"
        );
    }
```

Also add `gpio_wifi_power: p("gpio-wifi_power"),` to `test_layout`, right after `gpio_wifi_en`.

**Step 2: Run it and confirm it fails.** It won't compile yet: `no field gpio_wifi_power`.

**Step 3: Implement.**

Add a const beside `GPIO_WIFI_EN`:
```rust
/// JUAN board's name for the same line; `wifi_en` does not exist there.
const GPIO_WIFI_POWER: &str = "/sys/user-gpio/gpio-wifi_power";
```

In `FsLayout`, add `pub gpio_wifi_power: String,` after `gpio_wifi_en`. In `production()`, add
`gpio_wifi_power: GPIO_WIFI_POWER.into(),`.

Replace step 3's loop with:
```rust
    // 3. Power sequence (wifi_driver.sh:373-382). Fleet boards name the line
    // wifi_en; the JUAN board names it gpio-wifi_power. Prefer the fleet name so
    // a tempdir test that creates neither keeps writing wifi_en.
    let gpio = if !std::path::Path::new(&layout.gpio_wifi_en).exists()
        && std::path::Path::new(&layout.gpio_wifi_power).exists()
    {
        &layout.gpio_wifi_power
    } else {
        &layout.gpio_wifi_en
    };
    for level in polarity.sequence() {
        std::fs::write(gpio, level).map_err(|e| format!("gpio write {gpio}={level}: {e}"))?;
        sys.sleep(Duration::from_secs(1));
    }
```

**Step 4: Run the wifi tests.** Expected: 48 passed.

**Step 5: Commit** with pathspec `cross-compile/anyka-init/src/wifi.rs`, message
`feat(anyka-init): fall back to gpio-wifi_power for the wifi power line`.

---

### Task 3: supplicant binary falls back to `/usr/bin/wpa_supplicant`

On the fleet, bring-up step 4 untars the supplicant into `/tmp/wpa_supplicant`. JUAN has no
`tar` and no `/data`, and ships a real binary at `/usr/bin/wpa_supplicant`. The choice must be
made *after* step 4, at spawn time.

**Files:**
- Modify: `cross-compile/anyka-init/src/wifi.rs`: `SUPPLICANT_BIN` (~323), `FsLayout`,
  `production()`, `start_supplicant_probing_driver` (~733) and `test_layout`.

**Step 1: Write the failing test.**

```rust
    #[test]
    fn test_try_bring_up_with_spawns_usr_bin_supplicant_when_tmp_copy_is_absent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let layout = test_layout(dir.path());
        std::fs::create_dir_all(format!("{}/wlan0", layout.sys_class_net)).expect("iface dir");
        std::fs::write(format!("{}/wlan0/carrier", layout.sys_class_net), "1").expect("carrier");
        std::fs::write(&layout.proc_route, HAPPY_ROUTE).expect("route");
        std::fs::write(&layout.proc_fib_trie, HAPPY_FIB_TRIE).expect("fib_trie");
        // JUAN: nothing was untarred into /tmp, the rootfs binary is real.
        std::fs::write(&layout.supplicant_fallback, "").expect("fallback bin");

        let mut sys = MockSys::new();
        sys.expect_now().returning(std::time::Instant::now);
        sys.expect_sleep().returning(|_| ());
        sys.expect_insmod().returning(|_| Ok(()));
        sys.expect_rmmod().returning(|_| Ok(()));
        sys.expect_run_to_completion().returning(|_, _| Ok(ExitStatus::Code(0)));
        let want = layout.supplicant_fallback.clone();
        sys.expect_spawn_detached()
            .withf(move |prog, _| prog == want)
            .returning(|_, _| Ok(4242));

        try_bring_up_with(&sys, &happy_cfg(), &layout).expect("bring-up must succeed");
    }
```

In `test_layout`, add `supplicant_bin: p("tmp_wpa_supplicant"),` and
`supplicant_fallback: p("usr_bin_wpa_supplicant"),`.

**Step 2: Run it and confirm it fails.** It won't compile yet: missing fields.

**Step 3: Implement.**

Add a const after `SUPPLICANT_BIN`:
```rust
/// JUAN board: no wifi_tool.tgz and no tar, but the rootfs binary is real.
const SUPPLICANT_FALLBACK: &str = "/usr/bin/wpa_supplicant";
```

In `FsLayout`, add `pub supplicant_bin: String,` and `pub supplicant_fallback: String,`. In
`production()`, add `supplicant_bin: SUPPLICANT_BIN.into(),` and
`supplicant_fallback: SUPPLICANT_FALLBACK.into(),`.

In `start_supplicant_probing_driver`, before the `for` loop:
```rust
    // Decided here, after step 4 had its chance to untar the fleet copy.
    let bin = if !std::path::Path::new(&layout.supplicant_bin).exists()
        && std::path::Path::new(&layout.supplicant_fallback).exists()
    {
        layout.supplicant_fallback.as_str()
    } else {
        layout.supplicant_bin.as_str()
    };
```
Then change the spawn to `sys.spawn_detached(bin, …)`.

**Step 4: Run the wifi tests.** Expected: 49 passed. The existing tests create neither file,
so they still spawn `supplicant_bin`.

**Step 5: Run the whole crate.** `… cargo test -p anyka-init --target x86_64-unknown-linux-gnu`
Expected: all pass. Then run clippy (memory: vendored-clippy-needs-path-prefix):
`PATH=$PWD/../toolchain/arm-anykav200-crosstool-ng/bin:$PATH cargo clippy -p anyka-init --target x86_64-unknown-linux-gnu -- -D warnings`.

**Step 6: Commit** with pathspec `cross-compile/anyka-init/src/wifi.rs`, message
`feat(anyka-init): fall back to /usr/bin/wpa_supplicant when /tmp has none`.

---

### Task 4: `kill-wpa.sh` gets the same fallback

`wire_kill_shim` replaces the configured `exec` with `/bin/sh kill-wpa.sh`. The shim hardcodes
`exec /tmp/wpa_supplicant`, so without this change the *supervised* supplicant dies on JUAN
right after a successful bring-up.

**Files:**
- Modify: `SD_card_contents/anyka_hack/kill-wpa.sh` (last line)

**Step 1: Make the change.** Replace `exec /tmp/wpa_supplicant "$@"` with:
```sh
# The fleet untars it into /tmp; the JUAN rootfs ships a real one in /usr/bin.
B=/tmp/wpa_supplicant
[ -x "$B" ] || B=/usr/bin/wpa_supplicant
exec "$B" "$@"
```

**Step 2: Syntax check.** `sh -n SD_card_contents/anyka_hack/kill-wpa.sh`. Expected: no output.
No unit test: it's two lines and gets verified on hardware in Task 8.

**Step 3: Check it ships.** `grep -rn kill-wpa scripts/package_bundle.sh scripts/build_payload.sh`.
If the bundle doesn't carry it, note that it reaches cameras only through the Task 7 root
staging and the payload push. Fleet cameras are unaffected: `/tmp/wpa_supplicant` is always
`-x` there.

**Step 4: Commit** with pathspec `SD_card_contents/anyka_hack/kill-wpa.sh`, message
`fix(kill-wpa): exec /usr/bin/wpa_supplicant when /tmp has none`.

---

### Task 5: JUAN hook runs the fleet P0 wrapper

**Files:**
- Modify: `SD_card_contents/juan/anyka_ipc_nostrip` (replace the phase 0 recon script whole)
- Create: `SD_card_contents/juan/anyka_ipc_nostrip.stock`
- Modify: `SD_card_contents/Factory/config.sh`: comment only, the line reading "Production
  never sets these"

**Step 1: Write `anyka_ipc_nostrip`.**

```sh
#!/bin/sh
# JUAN (Juanvision) AK3918EV200 entry point.
# See docs/plans/2026-09-28-juan-board-support-design.md.
#
# The stock /usr/sbin/anyka_ipc.sh mounts the card at /mnt/tf and, when
# /mnt/tf/debug.ini exists, bind-mounts this file over /usr/bin/anyka_ipc and
# runs it as root. /mnt/tf/do_not_debug.ini, or no debug.ini, boots stock.
#
# Board prep, then the fleet's own P0 wrapper: slot choice, deadman, respawn.
T=/mnt/tf

# Our binaries' ELF loader and RPATH are /mnt/anyka_hack/lib; /mnt is tmpfs here.
ln -s $T/anyka_hack /mnt/anyka_hack
mkdir -p $T/logs && ln -s $T/logs /mnt/logs

# Only anyka_ipc ever opens it, and anyka_ipc never starts on this path.
rmmod ak39_top_wdt 2>/dev/null

# Deadman: no address 4 min after boot puts the stock passthrough back over
# this file and reboots. JUAN's wifi_manage.sh is a stale rtl8188 script, so
# the vendor-wifi stage is a no-op.
ANYKA_WIFI_MANAGE=/bin/true \
ANYKA_CONFIG_SELF=$T/anyka_ipc_nostrip \
ANYKA_CONFIG_BAK=$T/anyka_ipc_nostrip.stock \
    sh $T/Factory/config.sh

# config.sh backgrounds the supervisor loop and returns. Block, so that
# anyka_ipc.sh does not log "anyka_ipc exit" and touch /etc/jffs2/error_reboot.
while :; do sleep 86400; done
```

**Step 2: Write `anyka_ipc_nostrip.stock`.** This is what the deadman restores: recovery
telnet plus the vendor app, exactly as stock.

```sh
#!/bin/sh
# JUAN stock passthrough. The deadman in Factory/config.sh copies this over
# anyka_ipc_nostrip when our stack never got an address; the next boot then
# runs the vendor app. Copy anyka_ipc_nostrip back from the repo to retry.
telnetd -p 24 -l /bin/sh &
# The shell holds this file open, so a plain umount is EBUSY.
umount -l /usr/bin/anyka_ipc && exec /usr/bin/anyka_ipc
```

**Step 3: Update the `config.sh` comment.** It says the overrides exist only for host tests.
Change that sentence to:
```sh
# The paths below are overridable so the host test in tests/p0_wrapper.rs can
# stub them, and so the JUAN hook (SD_card_contents/juan/anyka_ipc_nostrip)
# can point the deadman at its own restore target. Fleet boot never sets these.
```

**Step 4: Syntax check, and confirm the wrapper tests still pass.**
```bash
sh -n SD_card_contents/juan/anyka_ipc_nostrip && sh -n SD_card_contents/juan/anyka_ipc_nostrip.stock
cd cross-compile && ../toolchain/arm-anykav200-crosstool-ng/bin/cargo test -p anyka-init --target x86_64-unknown-linux-gnu --test p0_wrapper
```
Expected: no syntax output, and every p0_wrapper test passes. The deadman branches the hook
relies on are the ones those tests already cover.

**Step 5: Commit** with pathspec
`SD_card_contents/juan SD_card_contents/Factory/config.sh`, message
`feat(juan): hook runs the fleet P0 wrapper; deadman restores stock`.

---

### Task 6: JUAN `anyka.toml` (per-device, gitignored)

It derives from `.198`'s `.deploy/anyka.toml`: same SSID, DHCP, and it enables only udhcpc,
wpa_supplicant, vendor-daemon and onvif, all present on JUAN.

**Step 1: Generate it.**
```bash
sed -e 's|^sensor_module = .*|# sensor_module: JUAN stock boot already loads every sensor module|' \
    -e 's|^chip = .*|chip = "atbm6031x"|' \
    -e 's|^gpio_polarity = .*|gpio_polarity = "low_high"|' \
    -e 's|^fallback_to_vendor = .*|fallback_to_vendor = false|' \
    -e 's|^ftp = .*|ftp = false|' \
    -e 's|^telnet = .*|telnet = false|' \
    .deploy/anyka.toml > .deploy/juan-anyka.toml
diff .deploy/anyka.toml .deploy/juan-anyka.toml
```
Expected: exactly those six lines differ. `telnet = false` because config.sh already runs
`telnetd -p 24`. `ftp = false` because the unknown root password makes ftpd useless.

**Step 2: Validate it**, since there is no `--check-config` (memory:
bundle-never-carries-anyka-toml). Write `/tmp/juan-stage/check_toml.py`:
```python
import sys, tomllib
c = tomllib.load(open(sys.argv[1], "rb"))
w = c["wifi"]
assert w["chip"] == "atbm6031x", w["chip"]
assert w["dhcp"] is True and w["fallback_to_vendor"] is False
assert "sensor_module" not in c["system"]
print("ok", w["ssid"], sorted(k for k, v in c["services"].items() if v.get("enabled", True)))
```
Run: `/usr/bin/python3 /tmp/juan-stage/check_toml.py .deploy/juan-anyka.toml`.
Expected: `ok kmk ['onvif', 'udhcpc', 'vendor-daemon', 'wpa_supplicant']`.

There's nothing to commit: `.deploy/` is gitignored because it holds credentials.

---

### Task 7: stage and push the card

**Step 1: Build.**
```bash
./scripts/build_payload.sh --skip-www   # WebUI is not needed to prove the boot chain
./scripts/package_bundle.sh             # writes ./bundle.tar
```
If `package_bundle.sh` rejects the stamp, see memory build-stamp-vanishes-from-linked-binary.

**Step 2: Stage the slots layout.** It mirrors `.146`: bundle in `slots/a`, per-device files at the root.
```bash
S=/tmp/juan-stage; rm -rf $S/tf; mkdir -p $S/tf/anyka_hack/{slots/a,onvif,state,lib}
tar -xf bundle.tar -C $S/tf/anyka_hack/slots/a
cp SD_card_contents/anyka_hack/lib/{ld-uClibc.so.1,libc.so.0,libgcc_s.so.1} $S/tf/anyka_hack/lib/
cp SD_card_contents/anyka_hack/kill-wpa.sh $S/tf/anyka_hack/
printf a > $S/tf/anyka_hack/active
cp .deploy/juan-anyka.toml $S/tf/anyka_hack/anyka.toml
cp .deploy/users.toml $S/tf/anyka_hack/onvif/
cp SD_card_contents/anyka_hack/onvif/config.toml $S/tf/anyka_hack/onvif/
mkdir -p $S/tf/Factory && cp SD_card_contents/Factory/config.sh $S/tf/Factory/
cp SD_card_contents/juan/anyka_ipc_nostrip SD_card_contents/juan/anyka_ipc_nostrip.stock $S/tf/
(cd $S/tf && find . -type f ! -name MANIFEST.md5 | sed 's|^\./||' | sort | xargs md5sum > MANIFEST.md5)
```
`profiles.toml` is deliberately absent. Check on `.146` whether onvif-rust creates it; if not,
copy `.146`'s.

**Step 3: Commit `fetch.sh` to the repo** as `SD_card_contents/juan/fetch.sh`, copied from
`/tmp/juan-spike/fetch.sh`. It `wget`s every manifest entry into `/mnt/tf` and runs
`md5sum -c`. Copy it into `$S/tf/`. Commit with its own pathspec, message
`feat(juan): wget+md5 payload fetch for a board with no nc/tar/ftp`.

**Step 4: Push.** The user approved a temporary server for this. Kill it the moment the
fetch finishes.
```bash
(cd /tmp/juan-stage/tf && exec /usr/bin/python3 -m http.server 8799 --bind 192.168.2.10) &
# camera: cd /mnt/tf && wget -q -O fetch.sh http://192.168.2.10:8799/fetch.sh \
#         && sh fetch.sh http://192.168.2.10:8799 > fetch.log 2>&1
# wait, then: cat /mnt/tf/fetch.log  -> "fetch done: N files" and no non-OK lines
pkill -f "http.server 8799 --bind 192.168.2.10"; ss -ltn | grep 8799 || echo stopped
```
Then remove the phase 0 leftovers: `rm -rf /mnt/tf/anyka_hack/onvif/isp_gc1084.conf /mnt/tf/spike.sh`.
The ISP rename must go, because phase 2 selects the conf properly. **Until phase 2 lands, keep
the rename**, or the main stream has no ISP conf. In that case skip deleting
`isp_gc1084.conf`.

---

### Task 8: cold boot on hardware

**Step 1: Reboot and verify it's a new boot.** Send `reboot` over telnet, wait 90 s, then
read `cut -d' ' -f1 /proc/uptime`. It must be under 200. (Memory: verify a reboot by uptime,
not by HTTP returning.)

**Step 2: The boot chain ran.** On the camera:
```sh
ls -l /mnt/anyka_hack /mnt/logs; lsmod | grep -E "atbm|wdt"
ps | grep -E "anyka-init|vendor|onvif|wpa|udhcpc|anyka_ipc" | grep -v grep
grep -E "wifi chip resolved|wpa_supplicant associated|wifi up|vendor fallback" /mnt/logs/anyka-init.log | tail
```
Expected:
- both symlinks exist
- `atbm6031x` is loaded and `ak39_top_wdt` is absent
- anyka-init, vendor-daemon, onvif-rust, `/usr/bin/wpa_supplicant` and udhcpc are running
- **no `anyka_ipc`**
- the log shows `wifi chip resolved chip="atbm6031x"`, then `associated driver="nl80211"`, then `wifi up`

**Step 3: Streams.**
```bash
for s in main sub; do timeout 25 ffprobe -v error -rtsp_transport tcp -show_entries stream=codec_name,width,height -of compact rtsp://admin:admin@192.168.2.148:554/$s; done
```
Expected: h264 1280x720 and 640x360.

**Step 4: It survives a second reboot**, and uptime resets again. Then leave it for 30 min and
re-read `/mnt/logs/anyka-init.log` for crash-loop or monitor-ladder entries.

**If wifi fails.** The deadman restores stock within ~4 min and the camera comes back on
`anyka_ipc`. Read `/mnt/tf/logs/anyka-init.log` over telnet 24 and fix the cause. To retry,
re-copy `anyka_ipc_nostrip` from the repo; the deadman overwrote it. Likely suspects, in order:
- the power polarity (`gpio_polarity = "high_low"`)
- the module needing the power line held before insmod
- the 30 s interface wait

**Step 5: Record.** Update memory `juan-camera-on-148-runs-our-stack` and correct the design
doc. It says "no FTP", but the applets exist and the root password is the blocker. The vendor
fallback is a stale script. `kill-wpa.sh` needed the fallback too. Commit the doc with its
pathspec.

---

## Phase 2: video parity (outline, plan in detail when phase 1 is on hardware)

- **ISP conf by sensor.** `cross-compile/onvif-rust/src/platform/anyka/context.rs:43`
  `ISP_CONFIG_SEARCH_PATHS` is gc1084-only. Read `/sys/ak_info_dump/sensor_id`: `0x1346` maps
  to `sc1346`, and likewise `0x1345` to `sc1345`, `0x1245` to `sc1245`, `0xa63` to `h63`.
  Prepend `/usr/local/isp_<name>.conf`. When the node is absent (the fleet case), keep today's
  list exactly. Test with a tempdir sysfs root. Then drop the phase 0 rename from the card.
- **14 vs 25 fps.** Stock `anyka_ipc` calls `isp_set_sensor_fps 25` after init, and ours stays
  at 14. Measure in daylight first. If it persists, find the vendor-daemon call that stock
  makes and we don't.
- **Daylight check.** Colour cast, then the `anyka-validation` RTSP/FLV/ONVIF harness against .148.
- **WebUI.** Build without `--skip-www`, push, then `curl -u admin:admin http://192.168.2.148/`
  must return 200.

## Phase 3: PTZ and night mode (outline)

- **PTZ.** `hal/anyka/ptz/driver.rs`: when `/dev/motor0` exists, use the V500 family
  (`_IOW('m', nr, int)`, nr 0x00/0x20/0x21/0x40-0x43/0x60-0x63) with the 24-byte
  `motor_parm` and 36-byte `motor_message` from `docs/reference/juan-flash-dump.md`.
  Otherwise keep `/dev/ak-motor0` and the legacy family. Unit-test the ioctl numbers and
  `size_of` against the recovered layouts. On hardware, prove it with a move plus
  `MOTOR_GET_STATUS` read-back, the first board where that is possible. Enable `[ptz]` only
  in JUAN's `config.toml`.
- **Night mode.** `platform/anyka/night_mode.rs` `Paths::node`: try `ircut_a`, then
  `gpio-ircut_a`, and the same for each node. Map `gpio-ir` to `IrLed` and `gpio-light` to
  `WhiteLed`. `ircut_high_is_night` gets measured on hardware (it's a calibration knob), not
  assumed.
