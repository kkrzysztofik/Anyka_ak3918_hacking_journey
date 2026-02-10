# Place in /data/
These are the files that go in `/data/`

# Settings

The `gergesettings.txt` file has new entries to control all parts of the camera. There are comments to explain what each option does.

You can simply place an updated copy of `gergesettings.txt` on the SD card in `anyka_hacks` folder and the camera will copy that file. This applies any new settings on the next start of the camera.

# Wifi
It will also set the new wifi credentials for you if they are different. There will be a copy of the newly created `anyka_cfg` and a backup of the old one with your old credentials copied to the SD card.

Even if you turn off wifi, or set incorrect credentials you can simply correct the settings with the SD card and the camera will connect to LAN on the next boot.

**Some special characters don't work in wifi ssid names and passwords** alpha-numerical strings as well as `.` `_` and `-` are tested and working.
This is not a limitation of the hack, but rather the camera wifi scripts.

# Script version updates
`gergehack.sh` changes a lot during testing, so it is also updated from the `anyka_hacks` folder of the SD if you place a modified version there.

# SSH Settings

The SD overlay now supports Dropbear SSH management via `gergesettings.txt`:

- `run_ssh=0|1` enables/disables Dropbear startup from `gergehack.sh`
- `ssh_port=22` sets the SSH listening port
- `ssh_auth_mode=both|key|password` sets authentication behavior
- `ssh_host_key_path=/mnt/anyka_hack/dropbear/dropbear_ecdsa_host_key` points to host key file
- `ssh_authorized_keys_path=/data/.ssh/authorized_keys` controls the key file linked to `/root/.ssh/authorized_keys`

When `run_ssh=1`, `gergehack.sh` will disable telnet automatically for safer defaults.

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
