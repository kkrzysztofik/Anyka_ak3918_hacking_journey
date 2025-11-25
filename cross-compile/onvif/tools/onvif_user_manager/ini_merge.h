/**
 * @file ini_merge.h
 * @brief Header for INI file merging functions
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef INI_MERGE_H
#define INI_MERGE_H

#include "core/config/config.h"

/**
 * @brief Merge user sections into existing INI file
 *
 * Preserves all non-user sections from the original file and
 * replaces user sections with updated data from runtime config.
 *
 * @param filepath Path to the INI file to update
 * @param config Current runtime configuration
 * @return ONVIF_SUCCESS on success, error code otherwise
 */
int ini_merge_user_sections(const char* filepath, const struct application_config* config);

#endif /* INI_MERGE_H */
