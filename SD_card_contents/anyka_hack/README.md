# Anyka SD-card hack overlay

Files under this directory are copied to the camera's SD card and mounted at
`/mnt/anyka_hack/`. Boot is driven by `Factory/config.sh` (started by the
vendor's `service.sh` when `/mnt/Factory` exists), which launches
`anyka-init.bin`.

# Settings

Configuration is `/mnt/anyka_hack/anyka.toml` (also present on the SD card as
`anyka_hack/anyka.toml`). It is **parsed as TOML**, never shell-sourced.

Edit wifi credentials in the `[wifi]` section before first boot. Updating the
camera means rewriting the SD card (or editing `anyka.toml` in place on a
mounted card) and rebooting — there is no longer a `/data/gergesettings.txt`
copy step.

# Wifi

`anyka-init` rewrites `/etc/jffs2/anyka_cfg.ini` when `[wifi]` differs from the
on-disk credentials, keeping a `.old` backup. Incorrect credentials can be
fixed by editing `anyka.toml` on the SD card and rebooting; recovery telnet on
port 24 stays up if `[system].telnet = true` (and is always started briefly by
the P0 wrapper).

**Some special characters don't work in wifi ssid names and passwords** —
alphanumeric plus `.` `_` and `-` are tested and working. This is a limitation
of the camera wifi scripts, not the hack.

# Updating

Swap or rewrite the SD card contents and reboot. The supervisor binary is
`anyka_hack/anyka-init.bin`, built by `./scripts/build_sd_contents.sh`.

Dropbear SSH is controlled by `[services.dropbear]` in `anyka.toml` (`enabled`,
`args`). There are no separate `ssh_*` keys.

# Third-Party Build Scripts

Host-side helper scripts are available to build and package ARM binaries into the SD overlay:

```bash
# Build Dropbear and copy dropbearmulti into SD_card_contents/anyka_hack/dropbear/
./scripts/third_party/build_dropbear.sh

# Build htop and copy binary into SD_card_contents/anyka_hack/bin/htop
./scripts/third_party/build_htop.sh

# Build perf and copy launcher + binary into SD_card_contents/anyka_hack/bin/perf
# (defaults to static linking for best runtime compatibility on AK3918)
./scripts/third_party/build_perf.sh

# Build strace and copy launcher + binary into SD_card_contents/anyka_hack/bin/strace
# (defaults to static linking for best runtime compatibility on AK3918)
./scripts/third_party/build_strace.sh
```

All scripts support explicit version/checksum overrides with `--version` and `--sha256`.
If the build host has no internet access, pass local tarballs with `--archive`.
`build_dropbear.sh` defaults to static linking for runtime compatibility on older AK3918 firmware.
`build_htop.sh` installs an `htop` launcher plus bundled terminfo entries to avoid SSH `$TERM` incompatibility issues.
`build_perf.sh` and `build_strace.sh` install wrapper scripts that prepend `/mnt/anyka_hack/lib` to `LD_LIBRARY_PATH` when runtime libraries are bundled.
`build_perf.sh` is pinned to Linux `3.4.35` by default (Anyka target baseline).
`build_perf.sh` now defaults to static linking; pass `--link-mode dynamic` if you explicitly want bundled shared libraries.
`build_strace.sh` now defaults to static linking; pass `--link-mode dynamic` if you explicitly want bundled shared libraries.
For best `perf` compatibility, build from the same kernel version as the camera (`uname -r`), e.g.:
`./scripts/third_party/build_perf.sh --version "$(uname -r | sed 's/[^0-9.].*$//')" --sha256 <kernel-tarball-sha256>`.

# Profiling on device

After deploying SD overlay files, SSH to the camera and use:

```sh
# One-shot helper (runs strace + perf, writes timestamped bundle)
/mnt/anyka_hack/profile_onvif.sh --duration 30 --freq 49

# Verify tools
/mnt/anyka_hack/bin/perf --help
/mnt/anyka_hack/bin/strace -V

# Find onvif-rust PID
pidof onvif-rust

# strace sample (syscall hotspots)
/mnt/anyka_hack/bin/strace -tt -T -f -p "$(pidof onvif-rust)" -o /tmp/onvif.strace

# perf sample (CPU hotspots)
/mnt/anyka_hack/bin/perf record -F 49 -g -p "$(pidof onvif-rust)" -- sleep 30
/mnt/anyka_hack/bin/perf report --stdio > /tmp/onvif.perf.txt
```

If `/mnt/anyka_hack/profile_onvif.sh` prints `strace failed to attach`, run:

```sh
id
PID="$(pidof onvif-rust | awk '{print $1}')"
/mnt/anyka_hack/bin/strace -tt -T -f -p "${PID}" -o /tmp/onvif.strace
cat /proc/sys/kernel/yama/ptrace_scope 2>/dev/null || echo "no yama ptrace_scope"
```

Common fixes:
- Run from a root shell (`uid=0`).
- If Yama is enabled and `ptrace_scope` is not `0`, set it as root: `echo 0 > /proc/sys/kernel/yama/ptrace_scope`.
- Re-check PID (`pidof onvif-rust`) in case process restarted.
- If perf prints `sys_perf_event_open() ... Function not implemented` / `No CONFIG_PERF_EVENTS=y`, your kernel lacks perf events support. `profile_onvif.sh` will fall back to strace-only capture and write `perf_unavailable.txt` in the run directory.
