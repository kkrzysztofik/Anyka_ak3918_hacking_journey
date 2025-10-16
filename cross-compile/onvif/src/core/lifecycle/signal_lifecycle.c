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

#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "core/config/config.h"
#include "core/lifecycle/video_lifecycle.h"
#include "platform/platform.h"
#include "utils/error/error_handling.h"

/* Define SA_RESTART if not available */
#ifndef SA_RESTART
#define SA_RESTART 0x10000000
#endif

/* Global signal handling state - static variables with internal linkage only */
static volatile int g_signal_running = 1; // NOLINT

/* ---------------------------- Console Input Blocking -------------------------
 */

/**
 * @brief Block all console input by redirecting stdin to /dev/null
 *
 * This function redirects stdin to /dev/null to prevent any console input
 * from being processed, while still allowing signals (Ctrl+C, Ctrl+D) to work.
 *
 * @return ONVIF_SUCCESS on success, ONVIF_ERROR on failure
 */
static int block_console_input(void) {
  int null_fd = open("/dev/null", O_RDONLY);
  if (null_fd == -1) {
    platform_log_error("Failed to open /dev/null: %s\n", strerror(errno));
    return ONVIF_ERROR_IO;
  }

  if (dup2(null_fd, STDIN_FILENO) == -1) {
    platform_log_error("Failed to redirect stdin to /dev/null: %s\n", strerror(errno));
    close(null_fd);
    return ONVIF_ERROR_IO;
  }

  close(null_fd);
  platform_log_info("Console input blocked - stdin redirected to /dev/null\n");
  return ONVIF_SUCCESS;
}

/* ---------------------------- Signal Handler ------------------------- */

/**
 * @brief Signal handler for graceful termination (SIGINT, SIGTERM).
 * @param sig Signal number received
 */
static void signal_handler(int sig) {
  const char* signal_name = "UNKNOWN";

  switch (sig) {
  case SIGINT:
    signal_name = "SIGINT (Ctrl+C)";
    break;
  case SIGTERM:
    signal_name = "SIGTERM";
    break;
  case SIGHUP:
    signal_name = "SIGHUP";
    break;
  default:
    signal_name = "UNKNOWN";
    break;
  }

  // Only handle the first signal, ignore subsequent ones
  if (g_signal_running) {
    platform_log_notice("Received %s signal, initiating graceful shutdown...\n", signal_name);

    // Set running flag to false to exit main loop
    g_signal_running = 0;

    // Ask RTSP server to stop promptly
    video_lifecycle_stop_servers();
  }
}

/**
 * @brief Check for EOF condition (Ctrl+D) on stdin
 *
 * This function checks if stdin has reached EOF, which occurs when
 * Ctrl+D is pressed. Since we redirect stdin to /dev/null, this
 * should not normally occur, but we check for completeness.
 *
 * @return true if EOF detected, false otherwise
 */
static bool check_eof_condition(void) {
  // Since stdin is redirected to /dev/null, EOF should not occur
  // But we check for completeness in case of unexpected conditions
  if (feof(stdin)) {
    platform_log_notice("EOF detected on stdin (Ctrl+D), initiating graceful shutdown...\n");
    return true;
  }
  return false;
}

/* ---------------------------- Public Interface ------------------------- */

int signal_lifecycle_register_handlers(void) {
  platform_log_info("Registering signal handlers...\n");

  // Block all console input except signals
  if (block_console_input() != ONVIF_SUCCESS) {
    platform_log_error("Failed to block console input\n");
    return ONVIF_ERROR_IO;
  }

  struct sigaction signal_action;
  signal_action.sa_handler = signal_handler;
  sigemptyset(&signal_action.sa_mask);
  signal_action.sa_flags = SA_RESTART; // Restart interrupted system calls

  if (sigaction(SIGINT, &signal_action, NULL) != 0) {
    platform_log_error("Failed to register SIGINT handler\n");
    return ONVIF_ERROR_INITIALIZATION;
  }
  if (sigaction(SIGTERM, &signal_action, NULL) != 0) {
    platform_log_error("Failed to register SIGTERM handler\n");
    return ONVIF_ERROR_INITIALIZATION;
  }
  if (sigaction(SIGHUP, &signal_action, NULL) != 0) {
    platform_log_error("Failed to register SIGHUP handler\n");
    return ONVIF_ERROR_INITIALIZATION;
  }

  /* Ignore SIGPIPE to prevent crashes on broken pipes */
  signal_action.sa_handler = SIG_IGN;
  if (sigaction(SIGPIPE, &signal_action, NULL) != 0) {
    platform_log_error("Failed to register SIGPIPE handler\n");
    return ONVIF_ERROR_INITIALIZATION;
  }

  platform_log_info("Signal handlers registered successfully\n");
  platform_log_info("Console input blocked - only signals (Ctrl+C, Ctrl+D) are processed\n");
  return ONVIF_SUCCESS;
}

bool signal_lifecycle_should_continue(void) {
  return g_signal_running;
}

void signal_lifecycle_run_daemon_loop(const struct application_config* cfg) {
  platform_log_info("ONVIF daemon running... (Press Ctrl+C to stop)\n");

  while (g_signal_running) {
    /* Check for EOF condition (Ctrl+D) */
    if (check_eof_condition()) {
      g_signal_running = 0;
      platform_log_info("EOF detected, exiting main loop...\n");
      break;
    }

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
  }
}

void signal_lifecycle_request_shutdown(void) {
  g_signal_running = 0;
}

bool signal_lifecycle_shutdown_requested(void) {
  return !g_signal_running;
}
