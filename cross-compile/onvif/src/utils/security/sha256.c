/**
 * @file sha256.c
 * @brief SHA256 hash algorithm implementation (extracted from BusyBox)
 * @author kkrzysztofik (extraction and adaptation)
 * @date 2025
 *
 * Original implementation:
 * Copyright (C) 2010 Denys Vlasenko
 * Released into the Public Domain by Ulrich Drepper <drepper@redhat.com>
 * Licensed under GPLv2 or later, see file LICENSE in this source tree.
 *
 * Extracted from BusyBox 1.24.1 for use in ONVIF project.
 *
 * Note: This file contains cryptographic algorithm code with variable names
 * and constants that follow the SHA256 standard (FIPS 180-2). Clang-tidy
 * warnings about short variable names and magic numbers are suppressed
 * to maintain compatibility with the standard and original implementation.
 */

// NOLINTBEGIN

#include "sha256.h"

#include <stdint.h>
#include <string.h>

/* Endianness handling */
#if __BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__
#define SWAP_BE32(x)                                                                               \
  ((((x) & 0xff000000) >> 24) | (((x) & 0x00ff0000) >> 8) | (((x) & 0x0000ff00) << 8) |            \
   (((x) & 0x000000ff) << 24))
#define SWAP_BE64(x)                                                                               \
  ((((x) & 0xff00000000000000ULL) >> 56) | (((x) & 0x00ff000000000000ULL) >> 40) |                 \
   (((x) & 0x0000ff0000000000ULL) >> 24) | (((x) & 0x000000ff00000000ULL) >> 8) |                  \
   (((x) & 0x00000000ff000000ULL) << 8) | (((x) & 0x0000000000ff0000ULL) << 24) |                  \
   (((x) & 0x000000000000ff00ULL) << 40) | (((x) & 0x00000000000000ffULL) << 56))
#define IS_LITTLE_ENDIAN 1
#else
#define SWAP_BE32(x)     (x)
#define SWAP_BE64(x)     (x)
#define IS_LITTLE_ENDIAN 0
#endif

/* Rotation functions */
static inline uint32_t rotr32(uint32_t x, unsigned n) {
  return (x >> n) | (x << (32 - n));
}

/* SHA256 constants from FIPS 180-2:4.2.2 */
static const uint64_t sha256_K[64] = {
  0x428a2f98d728ae22ULL, 0x7137449123ef65cdULL, 0xb5c0fbcfec4d3b2fULL, 0xe9b5dba58189dbbcULL,
  0x3956c25bf348b538ULL, 0x59f111f1b605d019ULL, 0x923f82a4af194f9bULL, 0xab1c5ed5da6d8118ULL,
  0xd807aa98a3030242ULL, 0x12835b0145706fbeULL, 0x243185be4ee4b28cULL, 0x550c7dc3d5ffb4e2ULL,
  0x72be5d74f27b896fULL, 0x80deb1fe3b1696b1ULL, 0x9bdc06a725c71235ULL, 0xc19bf174cf692694ULL,
  0xe49b69c19ef14ad2ULL, 0xefbe4786384f25e3ULL, 0x0fc19dc68b8cd5b5ULL, 0x240ca1cc77ac9c65ULL,
  0x2de92c6f592b0275ULL, 0x4a7484aa6ea6e483ULL, 0x5cb0a9dcbd41fbd4ULL, 0x76f988da831153b5ULL,
  0x983e5152ee66dfabULL, 0xa831c66d2db43210ULL, 0xb00327c898fb213fULL, 0xbf597fc7beef0ee4ULL,
  0xc6e00bf33da88fc2ULL, 0xd5a79147930aa725ULL, 0x06ca6351e003826fULL, 0x142929670a0e6e70ULL,
  0x27b70a8546d22ffcULL, 0x2e1b21385c26c926ULL, 0x4d2c6dfc5ac42aedULL, 0x53380d139d95b3dfULL,
  0x650a73548baf63deULL, 0x766a0abb3c77b2a8ULL, 0x81c2c92e47edaee6ULL, 0x92722c851482353bULL,
  0xa2bfe8a14cf10364ULL, 0xa81a664bbc423001ULL, 0xc24b8b70d0f89791ULL, 0xc76c51a30654be30ULL,
  0xd192e819d6ef5218ULL, 0xd69906245565a910ULL, 0xf40e35855771202aULL, 0x106aa07032bbd1b8ULL,
  0x19a4c116b8d2d0c8ULL, 0x1e376c085141ab53ULL, 0x2748774cdf8eeb99ULL, 0x34b0bcb5e19b48a8ULL,
  0x391c0cb3c5c95a63ULL, 0x4ed8aa4ae3418acbULL, 0x5b9cca4f7763e373ULL, 0x682e6ff3d6b2b8a3ULL,
  0x748f82ee5defb2fcULL, 0x78a5636f43172f60ULL, 0x84c87814a1f0ab72ULL, 0x8cc702081a6439ecULL,
  0x90befffa23631e28ULL, 0xa4506cebde82bde9ULL, 0xbef9a3f7b2c67915ULL, 0xc67178f2e372532bULL};

/* SHA256 block processing function */
static void sha256_process_block64(sha256_ctx_t* ctx) {
  unsigned t;
  uint32_t W[64], a, b, c, d, e, f, g, h;
  const uint32_t* words = (uint32_t*)ctx->wbuffer;

/* Operators defined in FIPS 180-2:4.1.2 */
#define Ch(x, y, z)  ((x & y) ^ (~x & z))
#define Maj(x, y, z) ((x & y) ^ (x & z) ^ (y & z))
#define S0(x)        (rotr32(x, 2) ^ rotr32(x, 13) ^ rotr32(x, 22))
#define S1(x)        (rotr32(x, 6) ^ rotr32(x, 11) ^ rotr32(x, 25))
#define R0(x)        (rotr32(x, 7) ^ rotr32(x, 18) ^ (x >> 3))
#define R1(x)        (rotr32(x, 17) ^ rotr32(x, 19) ^ (x >> 10))

  /* Compute the message schedule according to FIPS 180-2:6.2.2 step 2 */
  for (t = 0; t < 16; ++t) {
    W[t] = SWAP_BE32(words[t]);
  }
  for (; t < 64; ++t) {
    W[t] = R1(W[t - 2]) + W[t - 7] + R0(W[t - 15]) + W[t - 16];
  }

  a = ctx->hash[0];
  b = ctx->hash[1];
  c = ctx->hash[2];
  d = ctx->hash[3];
  e = ctx->hash[4];
  f = ctx->hash[5];
  g = ctx->hash[6];
  h = ctx->hash[7];

  /* The actual computation according to FIPS 180-2:6.2.2 step 3 */
  for (t = 0; t < 64; ++t) {
    /* Fetch upper half of sha256_K[t] (constants are 64-bit but we only need upper 32) */
    uint32_t K_t = (uint32_t)(sha256_K[t] >> 32);
    uint32_t T1 = h + S1(e) + Ch(e, f, g) + K_t + W[t];
    uint32_t T2 = S0(a) + Maj(a, b, c);
    h = g;
    g = f;
    f = e;
    e = d + T1;
    d = c;
    c = b;
    b = a;
    a = T1 + T2;
  }

#undef Ch
#undef Maj
#undef S0
#undef S1
#undef R0
#undef R1

  /* Add the starting values according to FIPS 180-2:6.2.2 step 4 */
  ctx->hash[0] += a;
  ctx->hash[1] += b;
  ctx->hash[2] += c;
  ctx->hash[3] += d;
  ctx->hash[4] += e;
  ctx->hash[5] += f;
  ctx->hash[6] += g;
  ctx->hash[7] += h;
}

/* Feed data through a temporary buffer */
static void common64_hash(sha256_ctx_t* ctx, const void* buffer, size_t len) {
  unsigned bufpos = ctx->total64 & 63;

  ctx->total64 += len;

  while (1) {
    unsigned remaining = 64 - bufpos;
    if (remaining > len) {
      remaining = len;
    }
    /* Copy data into aligned buffer */
    memcpy(ctx->wbuffer + bufpos, buffer, remaining);
    len -= remaining;
    buffer = (const char*)buffer + remaining;
    bufpos += remaining;
    /* Clever way to do "if (bufpos != 64) break; bufpos = 0;" */
    bufpos -= 64;
    if (bufpos != 0) {
      break;
    }
    /* Buffer is filled up, process it */
    sha256_process_block64(ctx);
  }
}

/* Process the remaining bytes in the buffer */
static void common64_end(sha256_ctx_t* ctx, int swap_needed) {
  unsigned bufpos = ctx->total64 & 63;
  /* Pad the buffer to the next 64-byte boundary with 0x80,0,0,0... */
  ctx->wbuffer[bufpos++] = 0x80;

  /* This loop iterates either once or twice, no more, no less */
  while (1) {
    unsigned remaining = 64 - bufpos;
    memset(ctx->wbuffer + bufpos, 0, remaining);
    /* Do we have enough space for the length count? */
    if (remaining >= 8) {
      /* Store the 64-bit counter of bits in the buffer */
      uint64_t t = ctx->total64 << 3;
      if (swap_needed) {
        t = SWAP_BE64(t);
      }
      /* wbuffer is suitably aligned for this */
      uint64_t* ptr = (uint64_t*)(&ctx->wbuffer[64 - 8]);
      *ptr = t;
    }
    sha256_process_block64(ctx);
    if (remaining >= 8) {
      break;
    }
    bufpos = 0;
  }
}

/* SHA256 initialization values from FIPS 180-2:5.3.2 */
static const uint32_t sha256_init_values[8] = {0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
                                               0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19};

void sha256_begin(sha256_ctx_t* ctx) {
  ctx->total64 = 0;
  memcpy(ctx->hash, sha256_init_values, sizeof(sha256_init_values));
}

void sha256_hash(sha256_ctx_t* ctx, const void* buffer, size_t len) {
  common64_hash(ctx, buffer, len);
}

void sha256_end(sha256_ctx_t* ctx, void* resbuf) {
  /* SHA stores total in BE, need to swap on LE arches */
  common64_end(ctx, IS_LITTLE_ENDIAN);

  /* Convert hash to big-endian if needed */
  if (IS_LITTLE_ENDIAN) {
    unsigned i;
    for (i = 0; i < 8; ++i) {
      ctx->hash[i] = SWAP_BE32(ctx->hash[i]);
    }
  }

  memcpy(resbuf, ctx->hash, sizeof(ctx->hash[0]) * 8);
}

// NOLINTEND
