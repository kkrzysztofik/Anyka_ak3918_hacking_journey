## Purpose
Short, actionable guidance for AI coding agents to be immediately productive in this repository.

## Quick mental model (big picture)
- This repo documents reverse-engineering and hacks for Anyka AK3918-based cameras. The codebase is a mix of cross-compile app sources, SD-card payloads, and notes for modifying device rootfs (squashfs `*.sqsh4`).
- Major components:
  - `cross-compile/` — the place for source and build scripts for individual apps (e.g., `libre_anyka_app`, `aenc_demo`, `ak_snapshot`, `ptz_daemon`). Each subproject usually has a `build.sh` or `Makefile`.
  - `SD_card_contents/anyka_hack/` — payload and web UI that run from SD card. This is the easiest test path and the typical dev workflow for testing changes on device.
  - `newroot/` and prepared squashfs images (`busyroot.sqsh4`, `busyusr.sqsh4`, `newroot.sqsh4`) — prebuilt root/user fs images used for flashing.
  - `hack_process/`, `README.md`, `Images/`, and `UART_logs/` — docs and logs useful for debugging and reproducing device behavior.

## Where to start (practical entry points)
- To iterate fast: edit or add files under `cross-compile/<app>/` and test via the SD-card hack in `SD_card_contents/anyka_hack/`.
- Key READMEs:
  - Top-level `README.md` — project goals, features, SD-card hack quick start and important device commands.
  - `cross-compile/README.md` — cross-compile environment notes and `setup.sh` usage.

## Build & cross-compile notes (exact, not generic)
- This project targets an Anyka 32-bit toolchain (`arm-anykav200-*`) that is expected to be installed under `/opt/arm-anykav200-crosstool/usr/bin`. Many `build.sh` scripts assume you have exported that path into `PATH`:
  - Example: `export PATH=$PATH:/opt/arm-anykav200-crosstool/usr/bin`
- **Cross-compilation environment options:**
  - **Docker (Recommended for Windows/Modern Systems):** Use the provided Docker setup in `cross-compile/`:
    - Build image: `cd cross-compile && .\docker-build.ps1` (Windows) or `./docker-build.sh` (Linux/Mac)
    - Interactive shell: `docker run -it --rm -v ${PWD}:/workspace anyka-cross-compile`
    - Direct build: `docker run --rm -v ${PWD}:/workspace anyka-cross-compile make -C /workspace/<project>`
  - **Ubuntu 16.04 (Traditional):** Older 32-bit-friendly Ubuntu (repo uses Ubuntu 16.04 live USB) because toolchain is 32-bit
  - **Pre-built Live USB:** Available from share link in `cross-compile/README.md` with toolchain pre-installed
- Typical compile flow for an app (example `ak_snapshot`):
  - `cd cross-compile/anyka_v380ipcam_experiments/apps/ak_snapshot`
  - ensure `PATH` contains `arm-anykav200-crosstool` bin dir (or use Docker/VM/Live USB image described in `cross-compile/README.md`)
  - `./build` or `./build.sh`

## Device / runtime conventions and important paths
- Rootfs and userfs are squashfs images flashed from the device using `updater`:
  - Read/backup: `/dev/mtdblock4` (root), `/dev/mtdblock5` (user)
  - Install examples: `updater local A=/mnt/newroot.sqsh4` or `updater local B=/mnt/busyusr.sqsh4`
- Web UI served from device port 80 (busybox `httpd`). WebUI assets in `SD_card_contents/anyka_hack/www` (copy to `/etc/jffs2/www` when installing to flash).
- RTSP stream endpoint: `rtsp://<IP>:554/vs0` (unprotected by password). Snapshot BMP: port `3000` (e.g., `http://IP:3000`).
- Useful device files/dirs referenced in repo:
  - `/etc/init.d/rc.local` — modified during permanent hack to run `gergehack.sh`
  - `/usr/bin` and `/usr/sbin` — apps copied here in permanent hack (`libre_anyka_app`, `ptz_daemon_dyn`, etc.)
  - `/etc/jffs2/www` — web UI on flash

## Debugging flows & logs
- Local quick test: use the SD-card hack. It boots without modifying flash and gives telnet root access. Steps are in `README.md`.
- Serial/UART logs: see `UART_logs/` for historical logs and expected boot messages. When reproducing issues, collect the same logs from the device serial console.
- On-device commands often used in docs:
  - `cat /dev/mtdblock4 > /mnt/mtdblock4.bin` (backup)
  - `unsquashfs mtdblock4.bin` / `mksquashfs squashfs-root newroot.sqsh4 -b 131072 -comp xz`

## Project-specific patterns and gotchas (do not assume defaults)
- Toolchain & environment: build scripts assume a host environment that includes 32-bit libs and the specific Anyka toolchain. Running `make` on a modern 64-bit machine without the toolchain or compatibility libs will fail.
- Web UI / auth: the web UI uses an MD5-based token login and is explicitly insecure — do not attempt to retrofit modern auth or assume there is a secure API.
- Packaging format: compressed squashfs images use `*.sqsh4` and specific blocksize and XZ options (see `README.md`). Keep these options when recreating images to avoid flashing issues.
- Many helper scripts expect `busybox` behaviour; the repo replaces/relies on a custom `busybox` in some cases. Do not assume GNU coreutils behavior on device.

## Key files and directories to inspect for changes (examples)
- `cross-compile/libre_anyka_app/` — app combining RTSP, snapshots, motion detect.
- `cross-compile/IOT-ANYKA-PTZdaemon/` — PTZ daemon sources and build scripts.
- `SD_card_contents/anyka_hack/` — web UI, dropbear, helper scripts used for SD-card testing.
- `newroot/` — prepared rootfs images and `updater.txt` notes.
- `UART_logs/` — serial logs that show boot behavior used elsewhere in docs.

## How AI agents should edit & test (short checklist)
1. Make a small change under `cross-compile/<app>/`.
2. Build locally in a compatible environment:
   - **Docker (Recommended):** `docker run --rm -v ${PWD}:/workspace anyka-cross-compile make -C /workspace/<project>` 
   - **Traditional:** Export `PATH` to Anyka toolchain (`export PATH=$PATH:/opt/arm-anykav200-crosstool/usr/bin`)
   - **If unavailable:** Produce a patch and include a short explanation of toolchain expectations.
3. Test using the SD-card payload: update files under `SD_card_contents/anyka_hack/`, write to an SD card, boot camera with SD inserted, verify feature via web UI / RTSP.
4. If modifying rootfs images, document exact `mksquashfs` command and options used.

## Example actionable prompts for maintainers / reviewers
- "I changed `cross-compile/libre_anyka_app/main.c` to add logging — please build using Docker (`docker run --rm -v ${PWD}:/workspace anyka-cross-compile make -C /workspace/libre_anyka_app`) or the Anyka toolchain and test by copying the new binary to `SD_card_contents/anyka_hack/usr/bin/` and booting an SD-card image."
- "Patch updates `www/js/app.js` for the web UI. After applying, pack the SD payload and test UI on device at `http://<device-ip>`; capture network requests and serial log if UI doesn't load."

---
## Reference: `akipc` (vendor reference implementation)
- The `akipc` tree in this repo is a reference implementation (chip/vendor-provided) that shows how the original camera firmware implements APIs, initialization, and configuration. Use it as the canonical example of expected behavior when:
  - reverse-engineering how the camera starts services and uses device files (look for `main`, `service.sh`, or init hooks),
  - matching IPC/CLI commands and config keys used by webUI and other apps,
  - verifying binary and library ABI expectations before replacing or reimplementing a system binary.
- When updating apps under `cross-compile/`, search `akipc/` for the original usage patterns (for example: config file names, system call sequences, and expected filesystem layout). If the original app uses `/usr/sbin/camera.sh` or similar wrappers, mirror the same startup contract in your replacement so existing init scripts keep working.

## `component/` and `platform/` (low-level libs, drivers, board support)
- `component/` contains reusable pieces extracted from stock firmware: drivers, third-party libs, and helper tools. Notable subfolders you can inspect for context:
  - `busybox-1.24.1/` — customized busybox build used on-device (affects available shell tools and `httpd` behavior).
  - `ispdrv_lib/`, `ispsdk_lib/` — ISP libraries and bindings used by snapshot/streaming apps.
  - `wifi/`, `UVC_check_sensor/`, and `spiboot/` — drivers and helper tools that explain kernel module names and expected device nodes.
  - `NetCamera/`, `onvif_lib/` — network-related libraries the original firmware used for ONVIF/RTSP logic.
- `platform/` holds board/platform-specific glue: sensor selection and initialization, board pin mappings, and other low-level integration.
  - Check `platform/` when changing sensor code or board init, and cross-check against `isp_sensor_conf/` files (e.g., `isp_gc1034_0.conf`) so sensor parameters stay compatible with ISP libraries.

## Practical tips (component/platform/akipc)
- If you modify `busybox` or other low-level component binaries, remember the rootfs size and `mksquashfs` options matter: test with the SD-card hack first before flashing.
- When reimplementing a feature from scratch, copy the minimal set of kernel modules and `/usr/lib` dependencies used by the original (`akipc`/`component`) into `SD_card_contents/anyka_hack/` for SD-card testing so you replicate the runtime environment.
- Use the logs in `UART_logs/` when a change affects early boot or module loading — those logs show kernel module names and errors from stock firmware which are directly tied to `component/` modules.

