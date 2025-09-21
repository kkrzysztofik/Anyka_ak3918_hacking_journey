/**
 * @file signal_lifecycle.h
 * @brief Signal handling lifecycle management
 *
 * This module provides centralized management for signal handling,
 * graceful shutdown, and daemon loop control.
 *
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ONVIF_SIGNAL_LIFECYCLE_H
#define ONVIF_SIGNAL_LIFECYCLE_H

#include <stdbool.h>

#include "core/config/config.h"

/**
 * @brief Register signal handlers for graceful shutdown
 * @return 0 on success, -1 on failure
 */
int signal_lifecycle_register_handlers(void);

/**
 * @brief Check if daemon should continue running
 * @return true if daemon should continue, false if shutdown requested
 */
bool signal_lifecycle_should_continue(void);

/**
 * @brief Get signal count for timeout handling
 * @return Number of signals received
 */
int signal_lifecycle_get_signal_count(void);

/**
 * @brief Run the main daemon loop with signal handling
 * @param cfg Application configuration (for logging)
 */
void signal_lifecycle_run_daemon_loop(const struct application_config *cfg);

/**
 * @brief Request graceful shutdown
 */
void signal_lifecycle_request_shutdown(void);

/**
 * @brief Check if shutdown was requested
 * @return true if shutdown was requested, false otherwise
 */
bool signal_lifecycle_shutdown_requested(void);

#endif /* ONVIF_SIGNAL_LIFECYCLE_H */
