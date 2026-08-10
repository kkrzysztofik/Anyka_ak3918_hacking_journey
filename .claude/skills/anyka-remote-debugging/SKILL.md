---
name: anyka-remote-debugging
description: Use when debugging the camera on-device — run commands over telnet, collect coredumps via FTP, analyze ARM coredumps with gdb-multiarch (cam_exec, camera_shell, collect_coredump, run_gdb_multiarch_analysis, gdbserver, device shell).
version: 1.0.0
---

# Anyka Camera Remote Debugging

Debug the Anyka AK3918 camera at runtime: shell access, process inspection, log collection, coredump retrieval and analysis.

## Access Model

- **No SSH on the stock camera** — remote shell is **telnet port 24** (root, typically no password). Use `scripts/debugging/cam_exec.py` (Python 3.13 removed `telnetlib`, so it speaks raw telnet via the shared client in `camera_ntp_sync.py`).
- **FTP** (default user `admin`/`admin`) for pulling coredumps/logs off the device.
- Camera IP defaults to `192.168.2.198` in tooling.
- For on-device `gdbserver`, the reference rootfs ships one (`cross-compile/anyka_reference/platform/rootfs/utils/usr/bin/gdbserver`); telnet to the device, copy your binary + gdbserver, and attach.

## Run Commands on the Device

`scripts/debugging/cam_exec.py` — run one or more commands, prints output + exit status:

```bash
scripts/debugging/cam_exec.py 'ps | grep onvif'
scripts/debugging/cam_exec.py 'pidof onvif-rust'
scripts/debugging/cam_exec.py --timeout 30 'cat /mnt/anyka_hack/onvif/log/onvif.log | tail -50'
scripts/debugging/cam_exec.py 'uptime' 'free' 'df -h'          # multi-command (===== cmd ===== headers)
scripts/debugging/cam_exec.py --host 192.168.2.198 --port 24 'cmd'
```

- **Single command**: prints output, `[exit=N]` to stderr; process exit code mirrors the remote command (0 ok, 1 fail/TIMEOUT).
- **Multiple commands**: prints each under a `===== cmd =====` header; exit code 1 if any command failed.
- The script handles IAC negotiation, optional login prompt, disables echo, and uses a sentinel to capture exit status. Reuses the telnet client from `scripts/camera_ntp_sync.py` (single implementation, no duplication).

## Useful On-Device Checks

```bash
# Is onvif-rust running?
cam_exec.py 'pidof onvif-rust'          # empty = not running

# Process list
cam_exec.py 'ps | grep -E "onvif|vendor"'

# ONVIF logs (file-based)
cam_exec.py 'ls -la /mnt/anyka_hack/onvif/log/'
cam_exec.py 'tail -100 /mnt/anyka_hack/onvif/log/onvif.log'

# Resource state
cam_exec.py 'free && df -h && cat /proc/meminfo | head -5'

# Coredump locations
cam_exec.py 'ls -la /mnt/coredumps /mnt/logs /mnt/anyka_hack/onvif 2>/dev/null'
```

## Collect Coredumps

`scripts/debugging/collect_coredump.sh` — FTP scan of the known coredump dirs and download to `debugging/coredump/`:

```bash
scripts/debugging/collect_coredump.sh 192.168.2.198 admin admin
```

Searches (in order): `/mnt/coredumps` (kernel core_pattern target), `/mnt/logs` (old location), `/mnt/anyka_hack/onvif` (legacy cwd). Matches `core.*`, `core.*.*`, `*.core`.

## Analyze Coredumps

`scripts/debugging/run_gdb_multiarch_analysis.sh` — batch gdb-multiarch analysis of an ARM coredump against the onvif-rust binary:

```bash
scripts/debugging/run_gdb_multiarch_analysis.sh <coredump_file> onvif-rust
scripts/debugging/run_gdb_multiarch_analysis.sh core.onvif-rust.12345 onvif-rust
```

- Requires `gdb-multiarch`; sets architecture to `arm`, configures `solib-search-path` (`SD_card_contents/anyka_hack/lib`, `toolchain/.../usr/lib`), and substitute-paths for build dirs.
- Emits: backtrace (`bt full`), `info threads` + `thread apply all bt`, registers, frame, `info proc mappings`, shared libraries, memory dump around `$pc`/`$sp`.
- **Always use this script for coredump analysis — do not run gdb directly.** See `.serena/memories/coredump-analysis-prompt.md` for the analysis methodology and output format.

## Live Debugging with gdbserver (on device)

```bash
# Copy gdbserver (from reference rootfs) + your ARM binary to the device
scripts/debugging/cam_exec.py 'ls /usr/bin/gdbserver || echo missing'

# On device (via telnet): start gdbserver attached to running process
cam_exec.py 'gdbserver :2345 --attach $(pidof onvif-rust)'

# On host: connect gdb-multiarch
gdb-multiarch cross-compile/onvif-rust/target/armv5te-unknown-linux-uclibceabi/release/onvif-rust
(gdb) set architecture arm
(gdb) target remote 192.168.2.198:2345
(gdb) continue
```

Note: on this uClibc target `gdbserver` may be incompatible with the Rust static binary; prefer coredump analysis or `cam_exec` logging when gdbserver fails.

## Common Workflows

| Symptom | Action |
|---------|--------|
| onvif-rust not responding | `pidof onvif-rust`; check `/mnt/anyka_hack/onvif/log/`; restart via start script |
| Process crashed | `collect_coredump.sh` then `run_gdb_multiarch_analysis.sh` |
| Telemetry/perf issues | Use the `anyka-validation` skill (RTSP validation tool) |
| Protocol/network issues | Use the `protocol-debugging` skill (Wireshark/tcpdump) |
