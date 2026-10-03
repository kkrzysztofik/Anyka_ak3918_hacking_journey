# JUAN AK3918EV200 board support

Date: 2026-09-28
Status: design approved (approach A); phase 0 spike passed on hardware

## Goal

Full fleet parity for the JUAN / Juanvision camera at 192.168.2.148: anyka-init
supervises vendor-daemon and onvif-rust, with WebUI, A/B slots and one bundle
shared with the four Cloud39EV2 cameras. Background on the firmware:
`docs/reference/juan-flash-dump.md`.

## Measured board facts

All read off the live camera, not the dump.

| Seam | Fleet (Cloud39EV2) | JUAN |
|---|---|---|
| Sensor | GC1084, `sensor_gc1084.ko` insmodded by anyka-init | SC1346, all four sensor `.ko` loaded by stock boot |
| Sensor resolution | 1280x720 | 1280x720: SDK `ak_vi_get_sensor_resolution`, kernel HTS 1800 × VTS 750 @ 40.5 MHz (30 fps ceiling) |
| ISP conf | `isp_gc1084.conf` | `/usr/local/isp_sc1346.conf`; `/sys/ak_info_dump/sensor_id` = `0x1346` |
| Entry hook | `/mnt/Factory/config.sh` via `FACTORY_TEST` | `/mnt/tf/debug.ini` + `/mnt/tf/anyka_ipc_nostrip`, bind-mounted over `/usr/bin/anyka_ipc` |
| SD mount | `/mnt` | `/mnt/tf` (`/mnt` is tmpfs) |
| Watchdog | none | `ak39_top_wdt`, held only by `anyka_ipc` |
| Wifi | ssv6355 / zt9101, `wext`, `/tmp/ko/*.ko`, `hw.conf` | `atbm6031x.ko` in `/usr/modules`, `nl80211`, no `hw.conf` |
| Wifi credentials | `anyka.toml` | stock keeps them in `/etc/jffs2/config/network.json` |
| Wifi power GPIO | `/sys/user-gpio/wifi_en` | `/sys/user-gpio/gpio-wifi_power` |
| Night GPIOs | `ircut_a`, `ircut_b`, … | `gpio-ircut_a`, `gpio-ircut_b`, `gpio-ir`, `gpio-light` |
| Motor | built-in, `/dev/ak-motor{0,1}`, legacy ioctls | `ak_motor.ko`, `/dev/motor{0,1}`, V500 ioctls, 24/36-byte structs |
| Busybox | `nc`, `tar`, `ftpd` | v1.24.1: no `nc`/`tar`; `ftpd` exists but anonymous login gets 530 and the root password is unknown; has `wget`, `md5sum` |
| Vendor wifi fallback | `wifi_manage.sh` works | `wifi_manage.sh` is a stale rtl8188/`wext` script; `anyka_ipc` loads the ATBM itself (`wifi_bt_comb=1`) |
| Supplicant | untarred to `/tmp/wpa_supplicant` | real binary at `/usr/bin/wpa_supplicant`, `-Dnl80211` |
| RAM | 36 MB | 36 MB; ~2.7 MB free with our stack running |

## Phase 0 result (2026-09-28)

This run used the committed onvif-rust `fa09f6e7`, the repo's vendor-daemon and
its `lib/` set, all unmodified. The payload lived in `/mnt/tf/anyka_hack`, with
`/mnt/anyka_hack` and `/mnt/logs` symlinked into `/mnt/tf`. The ISP conf was
copied under the name onvif-rust searches for. Sequence:
`killall IOTDaemon_start.sh iot.Daemon`, then `killall -9 anyka_ipc`, then
`rmmod ak39_top_wdt`.

- RTSP `/main` h264 1280x720 + AAC, `/sub` 640x360, FLV `/live/main.flv`: all decode.
- The image is focused and exposed, and the OSD renders.
- `rmmod ak39_top_wdt` succeeds once `anyka_ipc` is gone, and the camera does not reboot.
- onvif-rust's ELF interpreter and RPATH are `/mnt/anyka_hack/lib`, so the
  `/mnt/anyka_hack` symlink is load-bearing. `lib/` must carry `ld-uClibc.so.1`,
  `libc.so.0` and `libgcc_s.so.1`.

Open from the spike, recheck in daylight:
- The sensor runs at **14 fps** under our stack. Stock `anyka_ipc` sets 25 after
  init. It was midnight, so this may be the ISP conf's low-light frame-rate drop.
- A warm colour cast under tungsten light, with the IR-cut position undriven.

## Approach: probe each seam, no board concept

Every hardcode becomes "use what exists on this camera". No `board` key and no
new config keys: a new key is a hard parse error for the older anyka-init in the
other slot (`deny_unknown_fields`). Values that genuinely differ per camera go in
**existing** `anyka.toml` keys, and that file is already per-device because bundles
never carry it.

Rejected: an explicit `board = "juan"` profile (new key, and it restates facts the
device reports), and a forked JUAN payload (a permanent maintenance tax).

## Phases

### 1. Boot chain

- **Hook** (`SD_card_contents/juan/anyka_ipc_nostrip`, replacing the phase 0
  recon script). It does three prep lines:
  - symlink `/mnt/anyka_hack` and `/mnt/logs` into `/mnt/tf`
  - `rmmod ak39_top_wdt`

  Then it runs the fleet's own `Factory/config.sh` unchanged, which covers
  telnet 24, slot choice, the respawn loop and the deadman. Env overrides aim
  the deadman's restore at `anyka_ipc_nostrip.stock`, a stock passthrough (the
  equivalent of `config.sh.gerge.bak`), and make its vendor-wifi stage a no-op.
  The kill switch stays the stock `/mnt/tf/do_not_debug.ini`.
- **Supervised supplicant.** `wire_kill_shim` replaces the service `exec` with
  `kill-wpa.sh`, which hardcodes `/tmp/wpa_supplicant`. It needs the same
  `/usr/bin` fallback as bring-up does.
- **Wifi.** Changes:
  - add an `atbm6031x` row to `wifi::Chip::ALL` with
    `module = /usr/modules/atbm6031x.ko`
  - when `wifi_en` is absent, probe the power GPIO as `gpio-wifi_power`
  - use supplicant driver `nl80211`

  `anyka.toml` on this camera sets `chip = "atbm6031x"` and leaves
  `sensor_module` unset. Both keys already exist. The install step copies SSID
  and password from `network.json`.
- **Unknown:** does the ATBM driver come up without `anyka_ipc`? The stock app
  loaded it, so our bring-up has never run on this chip. `fallback_to_vendor =
  false`, because the vendor script is stale. The real fallback is the
  deadman's restore to stock.

### 2. Video parity

- The ISP conf is chosen by `/sys/ak_info_dump/sensor_id`, falling back to the
  existing gc1084 search list when the node is absent (the fleet case).
- Explain or fix the 14 vs 25 fps gap.
- Verify in daylight: colour, and RTSP/FLV/ONVIF through the `anyka-validation` harness.

### 3. PTZ and night mode

- PTZ: when `/dev/motor0` exists, use the V500 ioctl family with 24/36-byte
  `MotorParm`/`MotorMessage`. Otherwise use the legacy `/dev/ak-motor0` path.
  This is the first board where `MOTOR_GET_STATUS` can read position back.
- Night-mode GPIOs: try `ircut_a`, then `gpio-ircut_a`, and the same for each
  node. Map `gpio-ir` and `gpio-light` onto the existing `IrLed`/`WhiteLed`
  nodes. Polarity is a calibration knob, measured on hardware.

## Deploy path

With no `nc`/`tar` and no usable FTP login, the path is a temporary `python3 -m http.server` bound to the dev box
LAN IP, serving an md5 `MANIFEST`. The camera runs a `fetch.sh` that `wget`s
each entry and runs `md5sum -c`. `push_bundle.sh`/`PUT /api/update` take over
once anyka-init runs, because onvif-rust's update endpoint needs none of those
applets.

## Testing

- Phases 1 and 3 add unit tests beside the existing ones:
  - `wifi.rs` gets the new chip row plus GPIO-path probing
  - `night_mode.rs` gets `gpio-` name resolution
  - `ptz/driver.rs` gets struct sizes and ioctl numbers against the dump's
    recovered layouts
- Each phase closes on hardware: a cold boot of .148 with no hand steps, then
  ffprobe of RTSP/FLV, and for phase 3 a PTZ move with position read back.
