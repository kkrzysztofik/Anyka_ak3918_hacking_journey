/**
 * @file signal_lifecycle.c
 * @brief Signal handling lifecycle management implementation
 *
 * This module implements signal handling, graceful shutdown,
 * and daemon loop control operations.
 *
 * @author kkrzysztofik
 * @date 2025
 */

#include "core/lifecycle/signal_lifecycle.h"

#include <signal.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

#include "core/config/config.h"
#include "core/lifecycle/platform_lifecycle.h"
#include "core/lifecycle/video_lifecycle.h"
#include "platform/platform.h"

/* Define SA_RESTART if not available */
#ifndef SA_RESTART
#define SA_RESTART 0x10000000
#endif

/* Global signal handling state - static variables with internal linkage only */
static volatile int g_signal_running = 1;      // NOLINT
static volatile int g_signal_count = 0;        // NOLINT
static volatile int g_signal_last_signal = 0;  // NOLINT

/* ---------------------------- Signal Handler ------------------------- */

/**
 * @brief Signal handler for graceful termination (SIGINT, SIGTERM).
 * @param sig Signal number received
 */
static void signal_handler(int sig) {
  const char *signal_name = "UNKNOWN";

  /* Increment signal counter and record last signal */
  g_signal_count++;
  g_signal_last_signal = sig;

  switch (sig) {
    case SIGINT:
      signal_name = "SIGINT (Ctrl+C)";
      break;
    case SIGTERM:
      signal_name = "SIGTERM (Ctrl+D)";
      break;
    case SIGHUP:
      signal_name = "SIGHUP";
      break;
    default:
      signal_name = "UNKNOWN";
      break;
  }

  if (g_signal_count == 1) {
    platform_log_notice("Received %s signal, initiating graceful shutdown...\n",
                        signal_name);
    platform_log_info(
        "Press Ctrl+C again within 5 seconds to force immediate shutdown\n");

    // Set running flag to false to exit main loop
    g_signal_running = 0;

    // Ask RTSP server to stop promptly
    video_lifecycle_stop_servers();
  } else if (g_signal_count == 2) {
    platform_log_warning(
        "Received second %s signal, forcing immediate shutdown...\n",
        signal_name);
    platform_log_warning("Performing emergency cleanup...\n");

    // Force immediate cleanup
    platform_lifecycle_cleanup();
    exit(1);
  } else {
    platform_log_error("Received %d signals, forcing immediate exit...\n",
                       g_signal_count);
    exit(1);
  }
}

/* ---------------------------- Public Interface ------------------------- */

int signal_lifecycle_register_handlers(void) {
  platform_log_info("Registering signal handlers...\n");

  struct sigaction signal_action;
  signal_action.sa_handler = signal_handler;
  sigemptyset(&signal_action.sa_mask);
  signal_action.sa_flags = SA_RESTART;  // Restart interrupted system calls

  if (sigaction(SIGINT, &signal_action, NULL) != 0) {
    platform_log_error("Failed to register SIGINT handler\n");
    return -1;
  }
  if (sigaction(SIGTERM, &signal_action, NULL) != 0) {
    platform_log_error("Failed to register SIGTERM handler\n");
    return -1;
  }
  if (sigaction(SIGHUP, &signal_action, NULL) != 0) {
    platform_log_error("Failed to register SIGHUP handler\n");
    return -1;
  }

  /* Ignore SIGPIPE to prevent crashes on broken pipes */
  signal_action.sa_handler = SIG_IGN;
  if (sigaction(SIGPIPE, &signal_action, NULL) != 0) {
    platform_log_error("Failed to register SIGPIPE handler\n");
    return -1;
  }

  platform_log_info("Signal handlers registered successfully\n");
  return 0;
}

bool signal_lifecycle_should_continue(void) { return g_signal_running; }

int signal_lifecycle_get_signal_count(void) { return g_signal_count; }

void signal_lifecycle_run_daemon_loop(const struct application_config *cfg) {
  platform_log_info("ONVIF daemon running... (Press Ctrl+C to stop)\n");

  while (g_signal_running) {
    /* Use sleep with signal interruption instead of blocking sleep */
    unsigned int sleep_duration = 1;
    unsigned int sleep_result = sleep(sleep_duration);

    /* Check if sleep was interrupted by a signal */
    if (sleep_result > 0) {
      platform_log_debug("Sleep interrupted by signal, continuing...\n");
    }

    /* Check if we should exit */
    if (!g_signal_running) {
      platform_log_info("Shutdown requested, exiting main loop...\n");
      break;
    }

    /* Check for signal timeout - if we received a signal but haven't exited
     * after 5 seconds, force shutdown */
    if (g_signal_count > 0 && g_signal_count < 2) {
      static int signal_timeout = 0;
      signal_timeout++;

      if (signal_timeout >= 5) {  // 5 seconds timeout
        platform_log_warning(
            "Graceful shutdown timeout reached, forcing immediate "
            "shutdown...\n");
        platform_lifecycle_cleanup();
        exit(1);
      }
    }
  }
}

void signal_lifecycle_request_shutdown(void) { g_signal_running = 0; }

bool signal_lifecycle_shutdown_requested(void) { return !g_signal_running; }
