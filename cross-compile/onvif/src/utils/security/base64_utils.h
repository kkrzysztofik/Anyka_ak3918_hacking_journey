/**
 * @file base64_utils.h
 * @brief Base64 encoding and decoding utility functions using libb64
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef BASE64_UTILS_H
#define BASE64_UTILS_H

#include <stddef.h>

/* ============================================================================
 * Function Declarations
 * ============================================================================
 */

/**
 * @brief Decode Base64 encoded string using libb64
 * @param input Base64 encoded input string (must not be NULL)
 * @param output Output buffer for decoded data (must not be NULL)
 * @param output_size Size of output buffer (must be > 0)
 * @return ONVIF_SUCCESS on success, ONVIF error code on failure
 * @note Input validation is performed by libb64 during decoding
 */
int onvif_util_base64_decode(const char* input, char* output, size_t output_size);

/**
 * @brief Encode binary data to Base64 string using libb64
 * @param input Binary input data (must not be NULL)
 * @param input_size Size of input data
 * @param output Output buffer for Base64 string (must not be NULL)
 * @param output_size Size of output buffer (must be > 0)
 * @return ONVIF_SUCCESS on success, ONVIF error code on failure
 * @note Output buffer must be large enough for encoded data
 */
int onvif_util_base64_encode(const unsigned char* input, size_t input_size, char* output, size_t output_size);

#endif /* BASE64_UTILS_H */
