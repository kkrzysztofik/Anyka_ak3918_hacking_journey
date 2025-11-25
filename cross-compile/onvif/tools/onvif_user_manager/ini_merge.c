/**
 * @file ini_merge.c
 * @brief Helper functions to merge user sections into existing INI file
 * @author kkrzysztofik
 * @date 2025
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "core/config/config.h"
#include "utils/error/error_handling.h"

#define MAX_LINE_LENGTH         512
#define MAX_FILE_SIZE           (64 * 1024)
#define USER_SECTION_PREFIX_LEN 5  /* "user_" */
#define TEMP_SUFFIX_LEN         10 /* ".tmp" */
#define SECTION_NAME_BUF_SIZE   32
#define USER_BUFFER_SIZE        8192

/**
 * @brief State structure for processing input file
 */
typedef struct {
  int in_user_section;
  int skip_until_next_section;
  int user_sections_written;
} merge_state_t;

/**
 * @brief Check if a section name is a user section
 */
static int is_user_section(const char* section_name) {
  if (section_name == NULL) {
    return 0;
  }
  return (strncmp(section_name, "user_", USER_SECTION_PREFIX_LEN) == 0);
}

/**
 * @brief Generate user sections buffer from config
 *
 * @param config Application configuration with user credentials
 * @param buffer Output buffer for user sections
 * @param buffer_size Size of output buffer
 * @return Number of bytes written, or -1 on error
 */
static int generate_user_sections_buffer(const struct application_config* config, char* buffer, size_t buffer_size) {
  size_t offset = 0;

  if (config == NULL || buffer == NULL) {
    return -1;
  }

  for (int user_idx = 0; user_idx < MAX_USERS; user_idx++) {
    const struct user_credential* user = &config->users[user_idx];
    char section_name[SECTION_NAME_BUF_SIZE];
    size_t written = 0;

    (void)snprintf(section_name, sizeof(section_name), "user_%d", user_idx + 1);

    /* Write section header */
    written = snprintf(buffer + offset, buffer_size - offset, "[%s]\n", section_name);
    if (written > 0 && (size_t)written < buffer_size - offset) {
      offset += written;
    }

    /* Write username */
    written = snprintf(buffer + offset, buffer_size - offset, "username = %s\n", user->username);
    if (written > 0 && (size_t)written < buffer_size - offset) {
      offset += written;
    }

    /* Write password hash */
    const char* hash_str = (user->password_hash[0] == '\0') ? "" : user->password_hash;
    written = snprintf(buffer + offset, buffer_size - offset, "password_hash = %s\n", hash_str);
    if (written > 0 && (size_t)written < buffer_size - offset) {
      offset += written;
    }

    /* Write active flag */
    written = snprintf(buffer + offset, buffer_size - offset, "active = %d\n\n", user->active);
    if (written > 0 && (size_t)written < buffer_size - offset) {
      offset += written;
    }
  }

  return (int)offset;
}

/**
 * @brief Process a single line from input file
 *
 * @param line Input line
 * @param line_copy Output buffer for line copy
 * @param output_file Output file to write to
 * @param user_buffer User sections buffer to write
 * @param state Merge processing state (modified in place)
 * @return 1 to continue loop, 0 to skip line
 */
static int process_input_line(const char* line, char* line_copy, FILE* output_file, const char* user_buffer, merge_state_t* state) {
  char* trimmed = (char*)line;
  size_t len = strlen(trimmed);

  /* Make a copy to preserve original line */
  strncpy(line_copy, line, MAX_LINE_LENGTH - 1);
  line_copy[MAX_LINE_LENGTH - 1] = '\0';

  /* Remove trailing whitespace from copy for parsing */
  while (len > 0 && (trimmed[len - 1] == ' ' || trimmed[len - 1] == '\t' || trimmed[len - 1] == '\n' || trimmed[len - 1] == '\r')) {
    trimmed[len - 1] = '\0';
    len--;
  }

  /* Check for section header */
  if (trimmed[0] == '[') {
    char* section_end = strchr(trimmed, ']');
    if (section_end == NULL) {
      return 0; /* Invalid section header, skip */
    }

    char section_name[MAX_LINE_LENGTH];
    size_t section_name_len = section_end - trimmed - 1;
    if (section_name_len >= sizeof(section_name)) {
      return 0; /* Section name too long, skip */
    }

    strncpy(section_name, trimmed + 1, section_name_len);
    section_name[section_name_len] = '\0';

    /* If this is a user section, skip it */
    if (is_user_section(section_name)) {
      state->skip_until_next_section = 1;
      state->in_user_section = 1;
      return 0; /* Skip line */
    }

    /* Non-user section - write user sections if we were in them */
    if (state->in_user_section && !state->user_sections_written) {
      (void)fprintf(output_file, "%s", user_buffer);
      state->user_sections_written = 1;
    }
    state->in_user_section = 0;
    state->skip_until_next_section = 0;
    (void)fprintf(output_file, "%s", line_copy);
    return 0; /* Line handled */
  }

  /* If we're in a user section, skip this line */
  if (state->skip_until_next_section) {
    return 0; /* Skip line */
  }

  /* Write non-user section lines */
  (void)fprintf(output_file, "%s", line_copy);
  return 1; /* Continue processing */
}

/**
 * @brief Merge user sections into existing INI file
 *
 * Reads the original file, preserves all non-user sections,
 * replaces user sections with updated data from runtime config,
 * and writes the merged result back.
 */
int ini_merge_user_sections(const char* filepath, const struct application_config* config) {
  FILE* input_file = NULL;
  FILE* output_file = NULL;
  char temp_path[MAX_LINE_LENGTH + TEMP_SUFFIX_LEN];
  char line[MAX_LINE_LENGTH];
  char line_copy[MAX_LINE_LENGTH];
  merge_state_t state = {0};
  char user_section_buffer[USER_BUFFER_SIZE] = {0};

  if (filepath == NULL || config == NULL) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Create temp file path */
  if (snprintf(temp_path, sizeof(temp_path), "%s.tmp", filepath) >= (int)sizeof(temp_path)) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Generate user sections content */
  if (generate_user_sections_buffer(config, user_section_buffer, sizeof(user_section_buffer)) < 0) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  /* Open input file */
  input_file = fopen(filepath, "r");
  if (input_file == NULL) {
    /* File doesn't exist - create new file with just user sections */
    output_file = fopen(filepath, "w");
    if (output_file == NULL) {
      return ONVIF_ERROR_IO;
    }
    (void)fprintf(output_file, "%s", user_section_buffer);
    (void)fclose(output_file);
    return ONVIF_SUCCESS;
  }

  /* Open output temp file */
  output_file = fopen(temp_path, "w");
  if (output_file == NULL) {
    (void)fclose(input_file);
    return ONVIF_ERROR_IO;
  }

  /* Process input file line by line */
  while (fgets(line, sizeof(line), input_file) != NULL) {
    (void)process_input_line(line, line_copy, output_file, user_section_buffer, &state);
  }

  /* Append user sections if not written yet */
  if (!state.user_sections_written) {
    (void)fprintf(output_file, "\n%s", user_section_buffer);
  }

  (void)fclose(input_file);
  (void)fclose(output_file);

  /* Atomically replace original file */
  if (rename(temp_path, filepath) != 0) {
    (void)remove(temp_path);
    return ONVIF_ERROR_IO;
  }

  return ONVIF_SUCCESS;
}
