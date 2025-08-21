## akipc deep analysis

This document summarizes a static analysis of the `akipc` application found in
`cross-compile/platform/apps/akipc/`.

Checklist
- [x] Code structure & build flow
- [x] Startup / runtime sequence and entry points
- [x] Modules, responsibilities and public APIs
- [x] Threads, signals and IPC channels (FIFO, IPC server)
- [x] Configuration, file paths and hardware interfaces
- [x] Security / robustness issues and likely vulnerabilities
- [x] Concrete remediation suggestions and next steps

Progress note
- Inspected: `main/ipc_main.c`, `include/*` (`ak_config.h`, `monitor.h`, `record_ctrl.h`, `ak_misc.h`), `monitor/monitor.c`, `misc/ak_misc.c`, `misc/ak_misc_ipcsrv.c`, `record/record_ctrl.c`, `config/ak_config.c` and `dana/*`.

## High-level overview
- Purpose: `akipc` is the Anyka IPC platform application used to boot and orchestrate video/audio capture, cloud/RTSP services, recording, and a local IPC command server.
- Entry point: `main/ipc_main.c::main()`
- Build: `cross-compile/platform/apps/akipc/Makefile` runs `make` in subfolders: `main`, `misc`, `config`, `record`, `monitor`, `output` (and `dana` if enabled).
- Config file: `/etc/jffs2/anyka_cfg.ini` (managed by `ak_config`).

## Key modules and responsibilities
- `main/ipc_main.c` — bootstrap, signal handling, start/stop lifecycle, calls into init functions for video/audio/other platform components.
- `monitor/monitor.c` — monitor thread writing heartbeats to `/tmp/daemon_fifo`.
- `misc/ak_misc.c` — IR/ircut/photosensitive switching, day/night management, voice tips integration.
- `misc/ak_misc_ipcsrv.c` — registers IPC handlers (`misc_set_status`, etc.) with internal IPC server (`ak_cmd_*` APIs).
- `config/ak_config.c` — INI parsing and getters/setters for many configuration structs (`video_config`, `video_record_config`, `camera_disp_config`, etc.).
- `record/record_ctrl.c` — SD card, mount, recording lifecycle, exception handling and DVR interactions.
- `dana/*` — optional cloud integration (enabled by `CONFIG_DANA_SUPPORT`).

## Startup / runtime sequence (condensed)
1. `main()` prints version info, registers signals, starts monitor thread.
2. `init_software()` registers IPC server (`ak_cmd_server_register`), loads INI (`ak_config_init_ini()`), matches sensor configs, opens video (`ak_vi_open`) and audio (`ak_ai_open`, `ak_ao_open`), registers `misc` IPC handlers, initializes IR and voice tips, and starts RTSP/record/cloud modules per config.
3. Main loop sleeps until `ipc_run_flag` cleared; shutdown calls `exit_ipc_app()` to clean resources.

## Threads, signals and IPC
- Threads observed: monitor thread, photosensitive/ircut threads (`ak_misc`), record control thread(s) (`record_ctrl_init()`), plus threads spawned by RTSP/cloud components.
- Signals: `ipc_main.c` registers SA_SIGINFO handler for SIGINT, SIGTERM, SIGUSR1, SIGUSR2, SIGALRM, SIGHUP (SIGPIPE/SIGCHLD ignored). The handler performs backtrace and toggles `ipc_run_flag`.
- IPC:
  - Local FIFO: `/tmp/daemon_fifo` — monitor writes heartbeat messages.
  - Internal IPC server: `ANYKA_IPC_PORT` used with `ak_cmd_server_register`, modules register commands with `ak_cmd_register_msg_handle`.

## Important files & paths
- INI: `/etc/jffs2/anyka_cfg.ini`
- ISP configs: scanned in `/etc/jffs2/` and `/usr/local/` (files like `isp_*.conf`)
- FIFO: `/tmp/daemon_fifo`
- Start marker: `/tmp/ak_ipc_start_time`
- SD mount/device paths used by `record_ctrl`: `/dev/mmcblk0*`, mount point `/mnt`

## Notable risks and issues (concise)
1. Monitor thread may `exit(EXIT_SUCCESS)` from a worker thread on FIFO write error. That unconditionally terminates the whole process from a thread context and bypasses the normal cleanup path.
2. Many `ak_cmd_exec()` usages build shell commands with `sprintf` and interpolate config-sourced strings (paths, filenames). This creates command injection risk if config or IPC-controlled values are not strictly validated.
3. `monitor.c` calls `open(FIFO_LOCATION, O_WRONLY)` blocking if no reader; this can hang thread startup. Opening blocking without a reader can deadlock or stall init.
4. IPC handlers accept file names and execute actions (play audio tips, trigger tools). If the IPC server is reachable without authentication, attackers may abuse these commands.
5. Several uses of `sprintf` into fixed buffers (e.g., `msg_buf[20]`) — prefer `snprintf` and explicit bounds checks.
6. Signal handler does non-trivial work; functions called from a signal handler must be signal-safe.

## Concrete remediation recommendations (prioritized)
1. Replace shell-shelling sinks (calls built with `ak_cmd_exec()` + `sprintf`) with safe APIs or sanitize inputs strictly. Where listing files is required, use `opendir/readdir` instead of `system("find ...")`.
2. Make monitor FIFO non-blocking and avoid calling `exit()` from threads — signal main thread for controlled shutdown.
3. Harden `ak_cmd` IPC: restrict access (local-only domain socket or loopback), add authentication/authorization or whitelist allowed commands, and validate IPC inputs before use.
4. Use `snprintf` and explicit buffer length checks everywhere; audit other `sprintf` usages.
5. Rework signal handler to be minimal: set atomic flag, write to pipe/eventfd, and let main loop perform heavy cleanup.
6. Add input validation for config file values (no shell metacharacters, path canonicalization) and reject unexpected values.
7. Add static analysis (cppcheck/clang-tidy), unit tests and a CI job to build and run smoke tests.

## Suggested next actions (I can help with any of these)
- Produce a list of all `ak_cmd_exec()` call sites and identify which take externally-controlled input.
- Patch `monitor/monitor.c` to open FIFO non-blocking, handle absent reader without blocking, and avoid `exit()` in worker threads. (small targeted patch available)
- Add safe replacements for one or two dangerous shell calls in `record_ctrl.c` (e.g., replace `find` usage with a safe C implementation).

## Verification & notes
- Files read for this analysis: `main/ipc_main.c`, `include/ak_config.h`, `include/monitor.h`, `include/ak_misc.h`, `include/record_ctrl.h`, `monitor/monitor.c`, `misc/ak_misc.c`, `misc/ak_misc_ipcsrv.c`, `record/record_ctrl.c`, `config/ak_config.c` plus `dana/` sources.
- This analysis is static and based on repository sources; dynamic runtime behavior (IPC network exposure, exact permissions, or platform-specific device behavior) should be checked on a running device.

---

If you'd like, I can now either:

- produce the `ak_cmd_exec()` call-site audit (highly recommended), or
- implement the safer monitor thread patch and run a quick build check.

Tell me which you prefer and I'll proceed.
