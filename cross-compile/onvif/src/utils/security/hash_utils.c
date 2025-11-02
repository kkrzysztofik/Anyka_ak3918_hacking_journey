/**
 * @file hash_utils.c
 * @brief ONVIF-style hash utility functions implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "hash_utils.h"

#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <unistd.h>

#include "platform/platform.h"
#include "sha256.h"
#include "utils/error/error_handling.h"

/* Salt size for password hashing (16 bytes = 32 hex chars) */
#define SALT_SIZE     16
#define SALT_HEX_SIZE 33 /* 32 hex chars + null terminator */

/* Maximum password length for security */
#define MAX_PASSWORD_LENGTH 256

/* Hexadecimal conversion constants */
#define HEX_BASE        16
#define MAX_BYTE_VALUE  255
#define HEX_NIBBLE_MASK 0xF

int onvif_sha256_compute(const uint8_t* data, size_t len, uint8_t digest[ONVIF_SHA256_DIGEST_SIZE]) {
  if (!data || !digest) {
    platform_log_error("NULL pointer in onvif_sha256_compute\n");
    return ONVIF_ERROR_INVALID;
  }

  sha256_ctx_t ctx;
  sha256_begin(&ctx);
  sha256_hash(&ctx, data, len);
  sha256_end(&ctx, digest);

  return ONVIF_SUCCESS;
}

int onvif_sha256_to_hex(const uint8_t digest[ONVIF_SHA256_DIGEST_SIZE], char* hex_output, size_t output_size) {
  if (!digest || !hex_output) {
    platform_log_error("NULL pointer in onvif_sha256_to_hex\n");
    return ONVIF_ERROR_INVALID;
  }

  if (output_size < ONVIF_SHA256_HEX_SIZE) {
    platform_log_error("Output buffer too small: %zu < %d\n", output_size, ONVIF_SHA256_HEX_SIZE);
    return ONVIF_ERROR_BUFFER_TOO_SMALL;
  }

  static const char hex_chars[] = "0123456789abcdef";
  /* Convert each byte to 2 hex characters */
  for (size_t i = 0; i < ONVIF_SHA256_DIGEST_SIZE; i++) {
    uint8_t byte_value = digest[i];
    hex_output[i * 2] = hex_chars[(byte_value >> 4) & HEX_NIBBLE_MASK];
    hex_output[i * 2 + 1] = hex_chars[byte_value & HEX_NIBBLE_MASK];
  }
  hex_output[ONVIF_SHA256_DIGEST_SIZE * 2] = '\0';

  return ONVIF_SUCCESS;
}

int onvif_sha256_compute_hex(const uint8_t* data, size_t len, char* hex_output, size_t output_size) {
  uint8_t digest[ONVIF_SHA256_DIGEST_SIZE];

  int result = onvif_sha256_compute(data, len, digest);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  return onvif_sha256_to_hex(digest, hex_output, output_size);
}

int onvif_generate_random_bytes(uint8_t* buffer, size_t size) {
  if (!buffer || size == 0) {
    platform_log_error("Invalid parameters for random byte generation\n");
    return ONVIF_ERROR_INVALID;
  }

  /* Open /dev/urandom for cryptographically secure random bytes */
  int random_fd = open("/dev/urandom", O_RDONLY);
  if (random_fd < 0) {
    platform_log_error("Failed to open /dev/urandom: %s\n", strerror(errno));
    return ONVIF_ERROR_IO;
  }

  size_t bytes_read = 0;
  while (bytes_read < size) {
    ssize_t result = read(random_fd, buffer + bytes_read, size - bytes_read);
    if (result < 0) {
      if (errno == EINTR) {
        /* Interrupted by signal, retry */
        continue;
      }
      platform_log_error("Failed to read from /dev/urandom: %s\n", strerror(errno));
      close(random_fd);
      return ONVIF_ERROR_IO;
    }
    if (result == 0) {
      /* Unexpected EOF */
      platform_log_error("Unexpected EOF from /dev/urandom\n");
      close(random_fd);
      return ONVIF_ERROR_IO;
    }
    bytes_read += (size_t)result;
  }

  close(random_fd);
  return ONVIF_SUCCESS;
}

/**
 * @brief Generate random salt for password hashing
 * @param salt Output buffer for salt bytes (SALT_SIZE bytes)
 * @return ONVIF_SUCCESS on success, error code on failure
 */
static int generate_salt(uint8_t salt[SALT_SIZE]) {
  /* Use cryptographically secure random number generator */
  return onvif_generate_random_bytes(salt, SALT_SIZE);
}

/**
 * @brief Convert salt bytes to hexadecimal string
 * @param salt Input salt bytes (SALT_SIZE bytes)
 * @param hex_output Output buffer (must be at least SALT_HEX_SIZE bytes)
 * @return ONVIF_SUCCESS on success, error code on failure
 */
static int salt_to_hex(const uint8_t salt[SALT_SIZE], char hex_output[SALT_HEX_SIZE]) {
  static const char hex_chars[] = "0123456789abcdef";
  for (int i = 0; i < SALT_SIZE; i++) {
    uint8_t byte_value = salt[i];
    hex_output[i * 2] = hex_chars[(byte_value >> 4) & HEX_NIBBLE_MASK];
    hex_output[i * 2 + 1] = hex_chars[byte_value & HEX_NIBBLE_MASK];
  }
  hex_output[SALT_SIZE * 2] = '\0';
  return ONVIF_SUCCESS;
}

static int hex_to_salt(const char* hex_input, uint8_t salt[SALT_SIZE]) {
  if (!hex_input) {
    return ONVIF_ERROR_INVALID;
  }
  if (strlen(hex_input) < SALT_SIZE * 2) {
    return ONVIF_ERROR_INVALID;
  }

  for (int i = 0; i < SALT_SIZE; i++) {
    char hex_pair[3] = {hex_input[i * 2], hex_input[i * 2 + 1], '\0'};
    char* endptr = NULL;
    unsigned long byte = strtoul(hex_pair, &endptr, HEX_BASE);

    if (endptr != hex_pair + 2 || byte > MAX_BYTE_VALUE) {
      return ONVIF_ERROR_INVALID;
    }
    salt[i] = (uint8_t)byte;
  }
  return ONVIF_SUCCESS;
}

static int hash_password_with_salt(const char* password, const uint8_t salt[SALT_SIZE], char* hash_output, size_t output_size) {
  if (output_size < ONVIF_SHA256_HEX_SIZE) {
    return ONVIF_ERROR_BUFFER_TOO_SMALL;
  }

  /* Combine password and salt */
  size_t pwd_len = strlen(password);
  size_t combined_len = pwd_len + SALT_SIZE;
  uint8_t* combined = (uint8_t*)malloc(combined_len);
  if (!combined) {
    platform_log_error("Failed to allocate memory for password hashing\n");
    return ONVIF_ERROR_MEMORY;
  }

  memcpy(combined, password, pwd_len); // NOLINT(bugprone-not-null-terminated-result)
  memcpy(combined + pwd_len, salt, SALT_SIZE);
  /* Note: combined buffer is not null-terminated by design (binary data) */

  /* Compute SHA256 hash */
  uint8_t digest[ONVIF_SHA256_DIGEST_SIZE];
  int result = onvif_sha256_compute(combined, combined_len, digest);

  /* Clear sensitive data */
  memset(combined, 0, combined_len);
  free(combined);

  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* Convert to hex */
  return onvif_sha256_to_hex(digest, hash_output, output_size);
}

int onvif_hash_password(const char* password, char* hash, size_t hash_size) {
  if (!password || !hash) {
    platform_log_error("NULL pointer in onvif_hash_password\n");
    return ONVIF_ERROR_INVALID;
  }

  size_t pwd_len = strlen(password);
  if (pwd_len == 0 || pwd_len > MAX_PASSWORD_LENGTH) {
    platform_log_error("Invalid password length: %zu\n", pwd_len);
    return ONVIF_ERROR_INVALID;
  }

  /* Need space for: salt_hex + '$' + hash_hex + '\0' */
  size_t required_size = SALT_HEX_SIZE - 1 + 1 + ONVIF_SHA256_HEX_SIZE - 1 + 1;
  if (hash_size < required_size) {
    platform_log_error("Hash buffer too small: %zu < %zu\n", hash_size, required_size);
    return ONVIF_ERROR_BUFFER_TOO_SMALL;
  }

  /* Generate random salt */
  uint8_t salt[SALT_SIZE];
  int result = generate_salt(salt);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* Convert salt to hex */
  char salt_hex[SALT_HEX_SIZE];
  result = salt_to_hex(salt, salt_hex);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* Hash password with salt */
  char hash_hex[ONVIF_SHA256_HEX_SIZE];
  result = hash_password_with_salt(password, salt, hash_hex, sizeof(hash_hex));
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* Combine salt and hash: "salt$hash" */
  int snprintf_result = snprintf(hash, hash_size, "%s$%s", salt_hex, hash_hex);
  if (snprintf_result >= (int)hash_size) {
    /* String was truncated, ensure null termination */
    hash[hash_size - 1] = '\0';
  }

  return ONVIF_SUCCESS;
}

int onvif_verify_password(const char* password, const char* hash) {
  if (!password || !hash) {
    platform_log_error("NULL pointer in onvif_verify_password\n");
    return ONVIF_ERROR_INVALID;
  }

  size_t pwd_len = strlen(password);
  if (pwd_len == 0 || pwd_len > MAX_PASSWORD_LENGTH) {
    platform_log_error("Invalid password length: %zu\n", pwd_len);
    return ONVIF_ERROR_INVALID;
  }

  /* Parse stored hash format: "salt$hash" */
  const char* separator = strchr(hash, '$');
  if (!separator) {
    platform_log_error("Invalid hash format: missing separator\n");
    return ONVIF_ERROR_INVALID;
  }

  /* Extract salt hex string */
  size_t salt_hex_len = separator - hash;
  if (salt_hex_len != SALT_HEX_SIZE - 1) {
    platform_log_error("Invalid salt length in hash: %zu\n", salt_hex_len);
    return ONVIF_ERROR_INVALID;
  }

  char salt_hex[SALT_HEX_SIZE];
  memcpy(salt_hex, hash, salt_hex_len);
  salt_hex[salt_hex_len] = '\0';

  /* Convert hex to salt bytes */
  uint8_t salt[SALT_SIZE];
  int result = hex_to_salt(salt_hex, salt);
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* Hash the provided password with extracted salt */
  char computed_hash[ONVIF_SHA256_HEX_SIZE];
  result = hash_password_with_salt(password, salt, computed_hash, sizeof(computed_hash));
  if (result != ONVIF_SUCCESS) {
    return result;
  }

  /* Compare computed hash with stored hash */
  const char* stored_hash = separator + 1;
  if (strcmp(computed_hash, stored_hash) == 0) {
    return ONVIF_SUCCESS;
  }
  return ONVIF_ERROR_AUTH_FAILED;
}
