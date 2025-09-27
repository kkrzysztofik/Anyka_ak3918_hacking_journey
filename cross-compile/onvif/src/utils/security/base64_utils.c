/**
 * @file base64_utils.c
 * @brief Base64 encoding and decoding utility functions using libb64
 * @author kkrzysztofik
 * @date 2025
 */

#include "utils/security/base64_utils.h"

#include "utils/error/error_handling.h"

#include <b64/cdecode.h>
#include <b64/cencode.h>
#include <stddef.h>
#include <string.h>

/* ============================================================================
 * Public Functions
 * ============================================================================
 */

int onvif_util_base64_decode(const char* input, char* output, size_t output_size) {
  if (!input || !output || output_size == 0) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  size_t input_len = strlen(input);
  if (input_len == 0) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  // Initialize libb64 decoder
  base64_decodestate decode_state;
  base64_init_decodestate(&decode_state);

  // Decode the input
  size_t decoded_len = base64_decode_block(input, input_len, output, &decode_state);

  // Check if decoding was successful
  if (decoded_len == 0) {
    return ONVIF_ERROR_PARSE_FAILED; // Invalid Base64 input
  }

  // Ensure null termination
  if (decoded_len < output_size) {
    output[decoded_len] = '\0';
    return ONVIF_SUCCESS;
  }

  return ONVIF_ERROR_MEMORY; // Output buffer too small
}

int onvif_util_base64_encode(const unsigned char* input, size_t input_size, char* output,
                             size_t output_size) {
  if (!input || !output || output_size == 0) {
    return ONVIF_ERROR_INVALID_PARAMETER;
  }

  if (input_size == 0) {
    output[0] = '\0';
    return ONVIF_SUCCESS;
  }

  // Initialize libb64 encoder
  base64_encodestate encode_state;
  base64_init_encodestate(&encode_state);

  // Calculate required output size
  size_t required_size = base64_encode_length(input_size, &encode_state);
  if (output_size < required_size) {
    return ONVIF_ERROR_MEMORY; // Output buffer too small
  }

  // Encode the input
  size_t encoded_len = base64_encode_block(input, input_size, output, &encode_state);

  // Finalize encoding (add padding if needed)
  encoded_len += base64_encode_blockend(output + encoded_len, &encode_state);

  // Ensure null termination
  if (encoded_len < output_size) {
    output[encoded_len] = '\0';
    return ONVIF_SUCCESS;
  }

  return ONVIF_ERROR_MEMORY; // Output buffer too small
}
