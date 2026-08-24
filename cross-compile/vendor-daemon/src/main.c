/**
 * vendor-daemon - Anyka SDK IPC bridge
 *
 * Receives binary IPC commands from the Rust onvif-rust server over a Unix
 * domain socket (/tmp/vendor-daemon.sock) and dispatches them to the real
 * Anyka SDK C API calls.
 *
 * Protocol (little-endian):
 *   Request:  [i32 cmd_id][u32 req_len][req_data bytes]
 *   Response: [i32 status][u32 resp_len][resp_data bytes]
 *
 * Dual Socket Mode:
 *   - Control socket: /tmp/vd-ctrl.sock (formerly /tmp/vendor-daemon.sock)
 *     Handles all IPC commands including lifecycle operations
 *   - Frame main socket: /tmp/vd-frame-main.sock
 *     Dedicated notification channel for main stream push frames
 *   - Frame sub socket: /tmp/vd-frame-sub.sock
 *     Dedicated notification channel for sub stream push frames
 *
 * Shared Memory Ring Buffer (Approach A):
 *   - Zero-copy frame delivery via shared memory
 *   - 20-byte notification protocol on frame socket
 *   - Falls back to socket-based delivery on ring buffer overflow
 *
 * Connection model:
 *   poll()-based multiplexing of up to MAX_CLIENTS concurrent connections.
 *   The first client to issue a lifecycle command (vi_open, venc_open, etc.)
 *   becomes the "control client" — only it may call lifecycle commands.
 *   Additional clients are restricted to streaming ops (set_iframe, set_rc)
 *   and queries (get_error_*, isp_*).  When the control client
 *   disconnects, the slot opens for the next lifecycle command.
 *
 * Build (old Anyka toolchain):
 *   arm-anykav200-linux-uclibcgnueabi-gcc -std=gnu99 -march=armv5te \
 *     -mfloat-abi=soft -o vendor-daemon main.c \
 *     -I<sdk-include-dir> -L<sdk-lib-dir> \
 *     -lplat_vi -lplat_vpss -lplat_venc_cb -lmpi_venc ...
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>   /* strcasecmp */
#include <stdint.h>
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <unistd.h>
#include <sys/socket.h>
#include <poll.h>
#include <pthread.h>
#include "log.h"
#include "ak_common.h"
#include "ak_global.h"
#include "globals.h"
#include "ipc.h"
#include "dispatcher.h"
#include "push.h"
#include "protocol.h"
#include "vd_ring_buffer.h"

/* ---- Log level selection ------------------------------------------------- */

/*
 * The daemon runs two independent log systems and both were pinned to their
 * most verbose setting: log.c for our own lines, and the Anyka SDK's ak_print.
 * At DEBUG the per-frame lines in push.c and ipc.c alone produce tens of
 * megabytes an hour onto vfat on the SD card, contending for the single core
 * that also has to encode and send the video. One name drives both.
 */
struct log_level_name {
    const char *name;
    int         daemon_level;  /* log.c enum:      LOG_TRACE .. LOG_FATAL */
    int         sdk_level;     /* ak_common.h:     enum LOG_LEVEL         */
};

static const struct log_level_name LOG_LEVEL_NAMES[] = {
    { "trace", LOG_TRACE, LOG_LEVEL_DEBUG   },
    { "debug", LOG_DEBUG, LOG_LEVEL_DEBUG   },
    { "info",  LOG_INFO,  LOG_LEVEL_INFO    },
    { "warn",  LOG_WARN,  LOG_LEVEL_WARNING },
    { "error", LOG_ERROR, LOG_LEVEL_ERROR   },
};

/* Quiet enough to stream against, verbose enough to keep the lifecycle and
 * shutdown records that make an incident diagnosable after the fact. */
#define LOG_LEVEL_DEFAULT_INDEX 2  /* "info" */

/**
 * resolve_log_level - Map VENDOR_DAEMON_LOG_LEVEL onto both log systems.
 *
 * @param name  Value of the environment variable, or NULL when unset.
 * @return      Matching table entry; the "info" entry when unset or unknown.
 *              Never NULL, so callers need no fallback of their own.
 */
static const struct log_level_name *resolve_log_level(const char *name)
{
    if (name != NULL && name[0] != '\0') {
        for (size_t i = 0; i < ARRAY_SIZE(LOG_LEVEL_NAMES); i++) {
            if (strcasecmp(name, LOG_LEVEL_NAMES[i].name) == 0) {
                return &LOG_LEVEL_NAMES[i];
            }
        }
    }
    return &LOG_LEVEL_NAMES[LOG_LEVEL_DEFAULT_INDEX];
}

/**
 * signal_handler - SIGINT/SIGTERM/SIGHUP signal handler.
 *
 * First signal sets the global g_shutdown flag to 1, which causes the main
 * event loop to exit cleanly on the next poll() iteration.
 *
 * A second signal exits immediately.  The graceful path joins the push
 * threads, and a push thread parked inside a blocking SDK call cannot be
 * interrupted -- without this escape hatch the daemon would appear to ignore
 * SIGTERM outright, leaving `kill -9` as the only way out.  _exit() is
 * async-signal-safe; exit() is not, as it would run atexit handlers and flush
 * stdio from a signal context.
 *
 * @param sig  Signal number received.
 */
static void signal_handler(int sig)
{
    if (g_shutdown) {
        _exit(128 + sig);
    }
    g_shutdown = 1;
}

/* ---- main ---------------------------------------------------------------- */

/* NOTE: all handlers, the dispatcher, IPC helpers, push threads, and global
 * variable definitions have been moved to the following modules:
 *   globals.c       - global variable storage
 *   ipc.c           - read_exact, write_exact, send_response, socket helpers
 *   push.c          - push-mode frame delivery and push thread
 *   handlers_vi.c   - VI command handlers
 *   handlers_vpss.c - VPSS command handlers
 *   handlers_venc.c - VENC command handlers (non-push)
 *   handlers_audio.c - AI/AENC command handlers
 *   handlers_isp.c  - ISP/imaging command handlers
 *   dispatcher.c    - process_request
 */

/**
 * main - Daemon entry point.
 *
 * Initialises logging (respecting the VENDOR_DAEMON_LOG_FILE environment
 * variable), configures SIGINT/SIGTERM handling, initialises the shared
 * memory ring buffer, creates the control and frame Unix domain sockets,
 * then runs the poll()-based event loop to multiplex client connections
 * until g_shutdown is set.
 *
 * @param argc  Argument count; unused.
 * @param argv  Argument vector; unused.
 * @return      0 on clean shutdown.
 */
int main(int argc, char *argv[])
{
    (void)argc;
    (void)argv;

    /* ================================================================
     * LOG FILE PATH INITIALIZATION
     * ================================================================ */

    /* Determine log file path from environment variable or use default.
     * Env var: VENDOR_DAEMON_LOG_FILE
     * Default: /mnt/logs/vendor_daemon.log
     */
    char log_file_path[LOG_FILE_PATH_MAX];
    const char *env_log_path = getenv("VENDOR_DAEMON_LOG_FILE");
    if (env_log_path && env_log_path[0] != '\0') {
        strncpy(log_file_path, env_log_path, LOG_FILE_PATH_MAX - 1);
        log_file_path[LOG_FILE_PATH_MAX - 1] = '\0';
    } else {
        strncpy(log_file_path, LOG_FILE_PATH_DEFAULT, LOG_FILE_PATH_MAX - 1);
        log_file_path[LOG_FILE_PATH_MAX - 1] = '\0';
    }

    /* ================================================================
     * LOGGING INITIALIZATION
     * ================================================================ */

    /* Open log file with an explicit non-world-writable mode; fopen()'s
     * default creation mode (0666 before umask) would make the file
     * world-writable under a permissive umask. */
    int log_fd = open(log_file_path, O_WRONLY | O_CREAT | O_APPEND, 0644);
    if (log_fd < 0) {
        fprintf(stderr, "Failed to open log file %s: %s\n",
                log_file_path, strerror(errno));
        g_log_fp = NULL;
    } else {
        g_log_fp = fdopen(log_fd, "a");
        if (!g_log_fp) {
            fprintf(stderr, "Failed to open log file %s: %s\n",
                    log_file_path, strerror(errno));
            close(log_fd);
        }
    }

    /* Save original stdout/stderr for restoration on shutdown */
    g_saved_stdout = dup(STDOUT_FILENO);
    g_saved_stderr = dup(STDERR_FILENO);

    /* Redirect stdout/stderr to log file to capture Anyka SDK ak_print() */
    if (g_log_fp) {
        dup2(fileno(g_log_fp), STDOUT_FILENO);
        dup2(fileno(g_log_fp), STDERR_FILENO);
    }

    /* Env var: VENDOR_DAEMON_LOG_LEVEL = trace|debug|info|warn|error.
     * Unset or unrecognised falls back to info rather than failing to start:
     * a typo here should cost log detail, not the video pipeline. */
    const char *env_log_level = getenv("VENDOR_DAEMON_LOG_LEVEL");
    const struct log_level_name *level = resolve_log_level(env_log_level);

    /* Initialize log.c - quiet mode suppresses stderr (we redirected it).
     * Both the global level and the file callback's own level must be set;
     * a callback registered at a more verbose level would emit regardless. */
    log_set_level(level->daemon_level);
    if (g_log_fp) {
        log_add_fp(g_log_fp, level->daemon_level);
        log_set_quiet(true);  /* Only write via file callback, not stderr */
    }

    log_info("========================================");
    log_info("vendor-daemon starting");
    log_info("log file: %s", log_file_path);
    log_info("log level: %s%s", level->name,
             (env_log_level && env_log_level[0] != '\0') ? "" : " (default)");

    /* Configure Anyka SDK logging:
     * - SDK messages go to stdout, which we redirected to our log file
     * - Disable syslog output to avoid duplicate messages
     */
    ak_print_set_level(level->sdk_level);
    ak_print_set_syslog_level(0);
    log_info("SDK logging: level=%d, syslog=disabled", level->sdk_level);

    /* ================================================================
     * SIGNAL HANDLING
     * ================================================================ */

    struct sigaction sa;
    memset(&sa, 0, sizeof(sa));
    sa.sa_handler = signal_handler;
    sigemptyset(&sa.sa_mask);
    sigaction(SIGINT,  &sa, NULL);
    sigaction(SIGTERM, &sa, NULL);
    /* SIGHUP too: the daemon runs in the foreground of run_vendor_daemon.sh with
     * no setsid/nohup, so a dropped telnet session would otherwise kill it via the
     * default action -- no thread stop, no ring teardown. */
    sigaction(SIGHUP,  &sa, NULL);

    /* Ignore SIGPIPE so write errors return EPIPE instead of killing us */
    signal(SIGPIPE, SIG_IGN);

    /* Initialise push stream state */
    memset(g_push_streams, 0, sizeof(g_push_streams));

    /* ================================================================
     * SHARED MEMORY RING BUFFER INITIALIZATION (Approach A)
     * ================================================================ */

    g_ring_buffer = vd_ring_create();
    if (g_ring_buffer) {
        log_info("[daemon] shared memory ring buffer initialized (%d slots, %d bytes/slot)",
                 VD_SHM_SLOT_COUNT, VD_SHM_SLOT_DATA_SIZE);
    } else {
        log_warn("[daemon] shared memory init failed, using socket-only mode");
    }

    /* ================================================================
     * SOCKET SETUP
     * ================================================================ */

    /* Control socket: handles all IPC commands */
    g_ctrl_server_fd = create_unix_socket(CTRL_SOCKET_PATH);
    if (g_ctrl_server_fd < 0) {
        log_fatal("[daemon] control socket creation failed");
        goto shutdown;
    }
    log_info("[daemon] control socket listening on %s", CTRL_SOCKET_PATH);

    /* Dedicated notification channel for main stream. */
    g_frame_main_server_fd = create_unix_socket(FRAME_MAIN_SOCKET_PATH);
    if (g_frame_main_server_fd < 0) {
        log_fatal("[daemon] main frame socket creation failed");
        goto shutdown;
    }
    log_info("[daemon] main frame socket listening on %s", FRAME_MAIN_SOCKET_PATH);

    /* Dedicated notification channel for sub stream. */
    g_frame_sub_server_fd = create_unix_socket(FRAME_SUB_SOCKET_PATH);
    if (g_frame_sub_server_fd < 0) {
        log_fatal("[daemon] sub frame socket creation failed");
        goto shutdown;
    }
    log_info("[daemon] sub frame socket listening on %s", FRAME_SUB_SOCKET_PATH);

    /* ================================================================
     * MAIN EVENT LOOP (poll-based multiplexing)
     * ================================================================
     *
     * fds[0]         = ctrl_server_fd
     * fds[1]         = frame_main_server_fd
     * fds[2]         = frame_sub_server_fd
     * fds[3..nfds-1] = connected client fds
     *
     * This replaces the old blocking accept()-then-inner-loop model so
     * multiple clients can be serviced without queuing in the kernel
     * backlog.  Each client with pending data is serviced round-robin.
     *
     * Each frame socket accepts at most 1 client.
     */

    /* pollfd array: [ctrl_server, frame_main_server, frame_sub_server, clients...] */
    struct pollfd fds[MAX_CLIENTS + 3];
    int nfds = 0;

    struct frame_listen {
        int *server_fd;
        int *client_fd;
        pthread_mutex_t *lock;
        const char *name;
    };
    struct frame_listen frame_chs[] = {
        { &g_frame_main_server_fd, &g_frame_main_client_fd,
          &g_frame_main_client_lock, "main" },
        { &g_frame_sub_server_fd, &g_frame_sub_client_fd,
          &g_frame_sub_client_lock, "sub" },
    };

    memset(fds, 0, sizeof(fds));

    /* Add control server socket */
    fds[nfds].fd = g_ctrl_server_fd;
    fds[nfds].events = POLLIN;
    nfds++;

    /* Add frame main server socket */
    fds[nfds].fd = g_frame_main_server_fd;
    fds[nfds].events = POLLIN;
    nfds++;

    /* Add frame sub server socket */
    fds[nfds].fd = g_frame_sub_server_fd;
    fds[nfds].events = POLLIN;
    nfds++;

    while (!g_shutdown) {
        int ready = poll(fds, nfds, 1000); /* 1s timeout for shutdown check */
        if (ready < 0) {
            if (errno == EINTR)
                continue;
            log_error("poll: %s", strerror(errno));
            break;
        }
        if (ready == 0)
            continue; /* timeout – re-check g_shutdown */

        /* ── Accept new connections on control socket ─────────────────── */
        if (fds[0].revents & POLLIN) {
            int client_fd = accept(g_ctrl_server_fd, NULL, NULL);
            if (client_fd >= 0) {
                if (nfds < MAX_CLIENTS + 3) {
                    fds[nfds].fd = client_fd;
                    fds[nfds].events = POLLIN;
                    fds[nfds].revents = 0;
                    nfds++;
                    log_info("[daemon] control client connected fd=%d (total=%d)",
                             client_fd, nfds - 1);
                } else {
                    log_error("[daemon] max clients (%d) reached, rejecting fd=%d",
                              MAX_CLIENTS, client_fd);
                    close(client_fd);
                }
            } else if (errno != EINTR) {
                log_error("accept (control): %s", strerror(errno));
            }
        }

        /* ── Accept new connections on main/sub frame sockets ─────────── */
        {
            size_t ch;
            for (ch = 0; ch < sizeof(frame_chs) / sizeof(frame_chs[0]); ch++) {
                int poll_idx = (int)ch + 1; /* fds[1]=main, fds[2]=sub */
                struct frame_listen *fl = &frame_chs[ch];
                int client_fd;

                if (!(fds[poll_idx].revents & POLLIN))
                    continue;

                client_fd = accept(*fl->server_fd, NULL, NULL);
                if (client_fd < 0) {
                    if (errno != EINTR)
                        log_error("accept (frame-%s): %s", fl->name, strerror(errno));
                    continue;
                }

                {
                    int has_existing = 0;
                    pthread_mutex_lock(fl->lock);
                    has_existing = (*fl->client_fd >= 0);
                    if (!has_existing)
                        *fl->client_fd = client_fd;
                    pthread_mutex_unlock(fl->lock);

                    if (has_existing) {
                        log_warn("[daemon] %s frame client already connected, rejecting fd=%d",
                                 fl->name, client_fd);
                        close(client_fd);
                    } else if (nfds < MAX_CLIENTS + 3) {
                        fds[nfds].fd = client_fd;
                        fds[nfds].events = POLLIN;
                        fds[nfds].revents = 0;
                        nfds++;
                        log_info("[daemon] %s frame client connected fd=%d", fl->name, client_fd);
                    } else {
                        pthread_mutex_lock(fl->lock);
                        if (*fl->client_fd == client_fd)
                            *fl->client_fd = -1;
                        pthread_mutex_unlock(fl->lock);
                        log_error("[daemon] max clients (%d) reached, rejecting %s frame fd=%d",
                                  MAX_CLIENTS, fl->name, client_fd);
                        close(client_fd);
                    }
                }
            }
        }

        /* ── Service clients with pending data (round-robin) ────────── */
        int i;
        int first_client_idx = 3;  /* fds[0..2] are server sockets */

        for (i = first_client_idx; i < nfds; i++) {
            if (!(fds[i].revents & (POLLIN | POLLHUP | POLLERR)))
                continue;

            {
                int client_fd = fds[i].fd;
                int main_client_fd = -1;
                int sub_client_fd = -1;
                int is_main_frame_client;
                int is_sub_frame_client;

                pthread_mutex_lock(&g_frame_main_client_lock);
                main_client_fd = g_frame_main_client_fd;
                pthread_mutex_unlock(&g_frame_main_client_lock);

                pthread_mutex_lock(&g_frame_sub_client_lock);
                sub_client_fd = g_frame_sub_client_fd;
                pthread_mutex_unlock(&g_frame_sub_client_lock);

                is_main_frame_client = (client_fd == main_client_fd);
                is_sub_frame_client = (client_fd == sub_client_fd);

                if (is_main_frame_client || is_sub_frame_client) {
                    if (fds[i].revents & (POLLHUP | POLLERR)) {
                        if (is_main_frame_client) {
                            pthread_mutex_lock(&g_frame_main_client_lock);
                            if (client_fd == g_frame_main_client_fd) {
                                g_frame_main_client_fd = -1;
                            }
                            pthread_mutex_unlock(&g_frame_main_client_lock);
                        }
                        if (is_sub_frame_client) {
                            pthread_mutex_lock(&g_frame_sub_client_lock);
                            if (client_fd == g_frame_sub_client_fd) {
                                g_frame_sub_client_fd = -1;
                            }
                            pthread_mutex_unlock(&g_frame_sub_client_lock);
                        }
                        log_info("[daemon] %s frame client disconnected fd=%d",
                                 is_sub_frame_client ? "sub" : "main", client_fd);
                        close(client_fd);
                        fds[i] = fds[nfds - 1];
                        nfds--;
                        i--;
                    }
                    continue;
                }

                {
                    int ret = process_request(client_fd);
                    if (ret == -1) {
                        /* Crash-only: this daemon exists to serve one control
                         * client. The sweep that used to run here could not
                         * fully clean the SDK -- the next client's
                         * ak_venc_request_stream returned null. Exiting lets
                         * the kernel close /dev/ion, /dev/video0 and /dev/uio0,
                         * and anyka-init restarts the pair. */
                        log_info("[daemon] control client fd=%d disconnected; exiting", client_fd);
                        _exit(1);
                    } else if (ret == -2) {
                        /* CMD_SHUTDOWN */
                        g_shutdown = 1;
                        break;
                    }
                }
            }
        }
    }

    /* Clean up any remaining client connections */
    {
        int i;
        int first_client_idx = 3;
        for (i = first_client_idx; i < nfds; i++) {
            close(fds[i].fd);
        }
    }

shutdown:
    /* ================================================================
     * SHUTDOWN
     * ================================================================ */

    log_info("[daemon] shutting down");

    /* Close frame sockets */
    if (g_frame_main_server_fd >= 0) {
        close(g_frame_main_server_fd);
        g_frame_main_server_fd = -1;
        unlink(FRAME_MAIN_SOCKET_PATH);
    }
    if (g_frame_sub_server_fd >= 0) {
        close(g_frame_sub_server_fd);
        g_frame_sub_server_fd = -1;
        unlink(FRAME_SUB_SOCKET_PATH);
    }

    /* Close frame clients */
    pthread_mutex_lock(&g_frame_main_client_lock);
    if (g_frame_main_client_fd >= 0) {
        close(g_frame_main_client_fd);
        g_frame_main_client_fd = -1;
    }
    pthread_mutex_unlock(&g_frame_main_client_lock);

    pthread_mutex_lock(&g_frame_sub_client_lock);
    if (g_frame_sub_client_fd >= 0) {
        close(g_frame_sub_client_fd);
        g_frame_sub_client_fd = -1;
    }
    pthread_mutex_unlock(&g_frame_sub_client_lock);

    /* Close control socket */
    if (g_ctrl_server_fd >= 0) {
        close(g_ctrl_server_fd);
        unlink(CTRL_SOCKET_PATH);
    }

    /* Stop push threads before destroying ring buffer */
    log_info("event=push_thread_lifecycle state=shutdown_stop_begin diag_monotonic_ms=%llu",
             (unsigned long long)diag_monotonic_ms());
    int wedged = 0;
    wedged |= (stop_push_slot(0) != 0);
    wedged |= (stop_push_slot(1) != 0);
    log_info("event=push_thread_lifecycle state=shutdown_stop_done wedged=%d diag_monotonic_ms=%llu",
             wedged,
             (unsigned long long)diag_monotonic_ms());

    /* Clean up shared memory ring buffer */
    if (g_ring_buffer) {
        if (wedged) {
            /* A push thread is still live inside the SDK and may still write into
             * the ring.  vd_ring_destroy() munmaps it, which would turn a wedged
             * encoder into a SIGSEGV on the way out.  Leave the mapping for the
             * kernel to reclaim: the shm path is reopened with O_CREAT and
             * re-truncated on next start, so a stale one costs nothing. */
            log_error("event=shutdown state=ring_teardown_skipped reason=push_thread_wedged diag_monotonic_ms=%llu",
                      (unsigned long long)diag_monotonic_ms());
        } else {
            vd_ring_shutdown(g_ring_buffer);
            vd_ring_destroy(g_ring_buffer, 1);
            g_ring_buffer = NULL;
        }
    }

    log_info("vendor-daemon stopped");
    log_info("========================================");

    /* Restore original stdout/stderr */
    if (g_saved_stdout >= 0) {
        dup2(g_saved_stdout, STDOUT_FILENO);
        close(g_saved_stdout);
    }
    if (g_saved_stderr >= 0) {
        dup2(g_saved_stderr, STDERR_FILENO);
        close(g_saved_stderr);
    }
    if (g_log_fp) {
        fclose(g_log_fp);
    }

    return 0;
}
