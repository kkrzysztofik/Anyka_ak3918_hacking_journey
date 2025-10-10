/**
 * @file sha256.h
 * @brief SHA256 hash algorithm implementation (extracted from BusyBox)
 * @author kkrzysztofik
 * @date 2025
 *
 * This is a minimal SHA256 implementation extracted from BusyBox 1.24.1
 * for use in the ONVIF project. Original implementation by Denys Vlasenko
 * and others, licensed under GPLv2.
 */

#ifndef ONVIF_SHA256_H
#define ONVIF_SHA256_H

#include <stddef.h>
#include <stdint.h>

/* SHA256 context structure */
typedef struct sha256_ctx {
  uint8_t wbuffer[64]; /* Working buffer, 64-byte aligned */ // NOLINT
  uint64_t total64;                                          /* Total bytes processed */
  uint32_t hash[8]; /* Hash state (8 x 32-bit words) */      // NOLINT
} sha256_ctx_t;

/* SHA256 digest size in bytes */
#define SHA256_DIGEST_SIZE 32

/**
 * @brief Initialize SHA256 context
 * @param ctx SHA256 context to initialize
 */
void sha256_begin(sha256_ctx_t* ctx);

/**
 * @brief Update SHA256 hash with new data
 * @param ctx SHA256 context
 * @param buffer Input data buffer
 * @param len Length of input data in bytes
 */
void sha256_hash(sha256_ctx_t* ctx, const void* buffer, size_t len);

/**
 * @brief Finalize SHA256 hash and output digest
 * @param ctx SHA256 context
 * @param resbuf Output buffer for 32-byte digest (must be at least 32 bytes)
 */
void sha256_end(sha256_ctx_t* ctx, void* resbuf);

#endif /* ONVIF_SHA256_H */
