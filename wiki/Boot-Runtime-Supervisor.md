# Boot / Runtime Supervisor (`anyka-init`)

The SD-card hack no longer boots through a pile of shell scripts. A single Rust
binary, **`anyka-init`**, owns bring-up, Wi-Fi, time sync, and service
supervision. Configuration is a TOML file that is **parsed, never evaluated**.

| Piece | Path on camera | Role |
|---|---|---|
| P0 wrapper | `/mnt/Factory/config.sh` | Starts recovery telnet on port 24, then `exec`s the binary |
| Supervisor | `/mnt/anyka_hack/anyka-init.bin` | Boot phases, Wi-Fi, NTP, supervise services |
| Config | `/mnt/anyka_hack/anyka.toml` | Typed settings (wifi, services, monitor, …) |
| Logs | `/mnt/logs/` | `anyka-init.log`, per-service logs |
| Storm state | `/mnt/anyka_hack/state/boot.json` | Reboot-storm counter for safe mode |

This replaces `gergehack.sh`, `common.sh`, `sys_monitor.sh`,
`periodic_reboot.sh`, and the old `run_*.sh` launchers.

## Quick start (operators)

1. Build the SD payload:

   ```bash
   source ./setenv.sh
   ./scripts/build_sd_contents.sh
   ```

2. **Edit Wi-Fi before first boot** in `SD_card_contents/anyka_hack/anyka.toml`:

   ```toml
   [wifi]
   ssid = "YOUR_NETWORK"
   password = "YOUR_PASSWORD"
   ```

   The shipped placeholders `CHANGE_ME` parse cleanly — the camera will boot and
   fail to associate if you forget this step.

3. Copy `anyka_hack/` and `Factory/` onto the card (or use
   `./scripts/copy_sd_contents.sh --sd /path/to/mount`).

4. Insert the card and power on. After ~30 s you should have:

   - Recovery telnet: `telnet <camera-ip> 24`
   - Supervised services: `vendor-daemon`, `onvif-rust`, `wpa_supplicant`, …
   - Logs under `/mnt/logs/`

Removing the SD card (no `Factory/` folder) returns the camera to stock
behaviour. The method does **not** modify boot firmware. When Wi-Fi settings in
`anyka.toml` differ from the stored config, the supervisor may rewrite
`/etc/jffs2/anyka_cfg.ini` (with a `.old` backup).

Hardware smoke-test checklist:
[`docs/reference/anyka-init-smoke-test.md`](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/blob/main/docs/reference/anyka-init-smoke-test.md).

## Boot sequence

```
vendor service.sh sees /mnt/Factory
  → /mnt/Factory/config.sh          # P0: telnetd :24, then exec
    → /mnt/anyka_hack/anyka-init.bin
         P1  load anyka.toml (park forever on hard config failure)
         P2  system setup: TZ, sensor module, wifi bring-up, honour telnet/ftp
         P2.5 first NTP sync (bounded; does not block boot forever)
         P3  start supervised services (wpa_supplicant, vendor-daemon, onvif, …)
         P4  supervise forever (reap, backoff, crash-loop / storm guard)
```

`config.sh` stays shell on purpose: if the bundled loader
`/mnt/anyka_hack/lib/ld-uClibc.so.1` is missing, the kernel cannot start the
ELF, but telnet still comes up.

## What the supervisor does

- **Supervise services** listed under `[services.*]`: start, reap, restart with
  exponential backoff, reboot after a crash-loop cap.
- **Storm guard**: after too many fast reboots, enter **SAFE MODE** — recovery
  telnet stays up, camera services do not start. Clear
  `/mnt/anyka_hack/state/boot.json` after fixing the fault, then reboot.
- **Wi-Fi bring-up** in Rust (chip dispatch, credentials, DHCP or static, link
  health escalation).
- **SNTP time sync** in-process (authenticated ONVIF needs a sane clock).
- **Monitor thread** (optional): link health, DHCP / reassociate / reboot
  ladder.
- **Log rotation** by size for the supervisor log and at service start.

Children get a cleared environment plus only the `env` map from their service
block — this avoids poisoning busybox with the newer uClibc `LD_LIBRARY_PATH`
used by ONVIF / vendor-daemon.

## `anyka.toml` reference

Location on the card / camera: `anyka_hack/anyka.toml` → `/mnt/anyka_hack/anyka.toml`.

Rules that matter in practice:

- Unknown keys are **rejected** (typos fail loud at boot).
- Wrong types fail loud.
- Validated fields include Wi-Fi chip / polarity / security enums, static
  address CIDR shape, and non-zero intervals when features are enabled.

### `[log]`

| Key | Default | Meaning |
|---|---|---|
| `dir` | `/mnt/logs` | Log directory |
| `level` | `info` | tracing filter |
| `max_bytes` | `2000000` | Rotate when larger |
| `keep` | `2` | Rotated generations to keep |

### `[system]`

| Key | Default | Meaning |
|---|---|---|
| `sensor_module` | _(none)_ | Optional `insmod` path (e.g. GC1084) |
| `telnet` | `false` | Keep P0 telnetd after boot |
| `ftp` | `true` | Keep vendor `tcpsvd` FTP |

In **SAFE MODE**, telnet is forced on even if `telnet = false`, so recovery
stays reachable.

### `[wifi]` — edit before first boot

| Key | Notes |
|---|---|
| `ssid` / `password` | Required. Prefer alphanumeric plus `.` `_` `-` |
| `security` | `wpa`, `wep`, or `open` |
| `chip` | `auto` or a known chip name (e.g. `ssv6355_ble`) |
| `gpio_polarity` | `high_low` or `low_high` |
| `interface` | default `wlan0` |
| `dhcp` | `true` (shipped default) or `false` for static |
| `address` / `gateway` / `dns` | Required when `dhcp = false` (`address` is CIDR, e.g. `192.168.2.198/24`) |
| `config_file` | Vendor ini rewritten when credentials differ (default `/etc/jffs2/anyka_cfg.ini`) |
| `connect_timeout_sec` | Association budget |
| `fallback_to_vendor` | Fall back to vendor wifi scripts on hard failure |

Shipped config uses DHCP. For static addressing:

```toml
[wifi]
dhcp = false
address = "192.168.2.198/24"
gateway = "192.168.2.1"
dns = ["192.168.2.1", "8.8.8.8"]

[services.udhcpc]
enabled = false   # must not renew over a static address
```

### `[time]`

SNTP servers, timezone string for the supervisor process, first-sync timeout,
retry / resync intervals, step threshold, and plausible unix bounds
(`max_plausible_unix` is capped for 32-bit `time_t` / year 2038).

### `[supervisor]`

Backoff min/max, crash-loop count/window, storm-guard max reboots, state path,
and uptime after which a healthy boot resets the storm counter.

### `[monitor]`

Optional link-health polling and escalation thresholds (DHCP → restart
supplicant → reboot, with a reboot budget).

### `[reboot]`

Optional periodic reboot (`enabled`, `interval_min`, `jitter_max_sec`). Not
started in SAFE MODE.

### `[services.<name>]`

Each supervised child:

```toml
[services.onvif]
enabled = true
exec = "/mnt/anyka_hack/onvif/onvif-rust.bin"
args = []                          # optional
log = "/mnt/logs/onvif.log"
core_dump = true                   # optional
env = { KEY = "value" }            # optional; sole env after clear
```

Shipped services: `udhcpc`, `wpa_supplicant`, `vendor-daemon`, `onvif`,
`dropbear` (SSH, disabled by default — enable and rebuild/redeploy as needed).

## Building and deploying

```bash
# Full SD payload (vendor-daemon, onvif-rust, anyka-init, WebUI)
./scripts/build_sd_contents.sh

# Skip WebUI or vendor-daemon when iterating
./scripts/build_sd_contents.sh --skip-www
./scripts/build_sd_contents.sh --debug

# Copy to a mounted card or over FTP
./scripts/copy_sd_contents.sh --sd /path/to/mount
./scripts/copy_sd_contents.sh --ftp 192.168.1.100
```

Host tests for the supervisor crate:

```bash
source ./setenv.sh
cd cross-compile/anyka-init
$CARGO test --target x86_64-unknown-linux-gnu
```

## Recovery and SAFE MODE

1. Telnet to port **24** (P0 always starts it; SAFE MODE keeps it).
2. Inspect `/mnt/logs/anyka-init.log` and the failing service log.
3. Fix `anyka.toml` (or the underlying fault).
4. Clear storm state if needed: `rm /mnt/anyka_hack/state/boot.json`
5. Reboot.

If config load fails hard, the process **parks** with recovery telnet still up
rather than guessing wifi/sensor defaults.

## See also

- [[Resources]] — beginner SD-card quick start
- [[ONVIF-Rust-Implementation]] — ONVIF server details
- [[Development-Environment]] — toolchain / `setenv.sh`
- [[Troubleshooting]] — common boot and service issues
- Overlay notes in-tree: [`SD_card_contents/anyka_hack/README.md`](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/blob/main/SD_card_contents/anyka_hack/README.md)
- Design: [`docs/plans/2026-08-01-boot-runtime-rust-design.md`](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/blob/main/docs/plans/2026-08-01-boot-runtime-rust-design.md)
