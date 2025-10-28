/**
 * @file audio_validation.c
 * @brief Audio parameter validation utilities implementation
 * @author kkrzysztofik
 * @date 2025
 *
 * This file provides validation functions for audio configuration parameters
 * to ensure they are within valid ranges and compatible with the Anyka
 * platform.
 */

#include "audio_validation.h"

#include "platform/platform_common.h"
#include "utils/error/error_handling.h"

/* Audio sample rate constants (Hz) */
#define AUDIO_SAMPLE_RATE_8K  8000  /* 8 kHz sample rate */
#define AUDIO_SAMPLE_RATE_16K 16000 /* 16 kHz sample rate */
#define AUDIO_SAMPLE_RATE_22K 22050 /* 22.05 kHz sample rate */
#define AUDIO_SAMPLE_RATE_44K 44100 /* 44.1 kHz sample rate */
#define AUDIO_SAMPLE_RATE_48K 48000 /* 48 kHz sample rate */

/* Audio bits per sample constants */
#define AUDIO_BITS_PER_SAMPLE_8  8  /* 8-bit samples */
#define AUDIO_BITS_PER_SAMPLE_16 16 /* 16-bit samples */
#define AUDIO_BITS_PER_SAMPLE_24 24 /* 24-bit samples */
#define AUDIO_BITS_PER_SAMPLE_32 32 /* 32-bit samples */

/* Audio channel constants */
#define AUDIO_CHANNELS_MONO   1 /* Mono audio */
#define AUDIO_CHANNELS_STEREO 2 /* Stereo audio */

/* Audio bitrate constants (bps) */
#define AUDIO_BITRATE_AAC_MIN     8000   /* Minimum AAC bitrate (8 kbps) */
#define AUDIO_BITRATE_AAC_DEFAULT 64000  /* Default AAC bitrate (64 kbps) */
#define AUDIO_BITRATE_AAC_MAX     128000 /* Maximum AAC bitrate (128 kbps) */

/* ============================================================================
 * PUBLIC API - Validation Functions
 * ============================================================================ */

/**
 * @brief Validate audio configuration parameters
 * @param config Audio configuration to validate (must not be NULL)
 * @return 1 if valid, 0 if invalid
 * @note Validates sample rate, channels, bits per sample, and codec type
 */
int audio_validation_validate_config(const platform_audio_config_t* config) {
  if (!config) {
    return 0;
  }

  if (!audio_validation_validate_sample_rate(config->sample_rate)) {
    return 0;
  }

  if (!audio_validation_validate_channels(config->channels)) {
    return 0;
  }

  if (!audio_validation_validate_bits_per_sample(config->bits_per_sample)) {
    return 0;
  }

  if (!audio_validation_validate_codec(config->codec)) {
    return 0;
  }

  return 1;
}

/**
 * @brief Validate audio sample rate
 * @param sample_rate Sample rate to validate
 * @return 1 if valid, 0 if invalid
 * @note Valid sample rates: 8000, 16000, 22050, 44100, 48000
 */
int audio_validation_validate_sample_rate(int sample_rate) {
  switch (sample_rate) {
  case AUDIO_SAMPLE_RATE_8K:
  case AUDIO_SAMPLE_RATE_16K:
  case AUDIO_SAMPLE_RATE_22K:
  case AUDIO_SAMPLE_RATE_44K:
  case AUDIO_SAMPLE_RATE_48K:
    return 1;
  default:
    return 0;
  }
}

/**
 * @brief Validate audio channel count
 * @param channels Number of channels to validate
 * @return 1 if valid, 0 if invalid
 * @note Valid channels: 1 (mono), 2 (stereo)
 */
int audio_validation_validate_channels(int channels) {
  return (channels == AUDIO_CHANNELS_MONO || channels == AUDIO_CHANNELS_STEREO) ? 1 : 0;
}

/**
 * @brief Validate audio bits per sample
 * @param bits_per_sample Bits per sample to validate
 * @return 1 if valid, 0 if invalid
 * @note Valid values: 8, 16, 24, 32
 */
int audio_validation_validate_bits_per_sample(int bits_per_sample) {
  switch (bits_per_sample) {
  case AUDIO_BITS_PER_SAMPLE_8:
  case AUDIO_BITS_PER_SAMPLE_16:
  case AUDIO_BITS_PER_SAMPLE_24:
  case AUDIO_BITS_PER_SAMPLE_32:
    return 1;
  default:
    return 0;
  }
}

/**
 * @brief Validate audio codec type
 * @param codec Audio codec to validate
 * @return 1 if valid, 0 if invalid
 * @note Valid codecs: PCM, AAC, G711A, G711U
 */
int audio_validation_validate_codec(platform_audio_codec_t codec) {
  switch (codec) {
  case PLATFORM_AUDIO_CODEC_PCM:
  case PLATFORM_AUDIO_CODEC_AAC:
  case PLATFORM_AUDIO_CODEC_G711A:
  case PLATFORM_AUDIO_CODEC_G711U:
    return 1;
  default:
    return 0;
  }
}

/* ============================================================================
 * PUBLIC API - Configuration Helpers
 * ============================================================================ */

/**
 * @brief Get default audio configuration
 * @param config Output parameter for default configuration (must not be NULL)
 * @return 0 on success, -1 on error
 * @note Sets safe default values for Anyka platform with AAC support
 */
int audio_validation_get_default_config(platform_audio_config_t* config) {
  if (!config) {
    return -1;
  }

  // Set safe defaults based on ai_demo and akipc reference implementations
  // Enhanced for AAC codec support
  config->sample_rate = AUDIO_SAMPLE_RATE_16K;        // 16kHz for better AAC quality
  config->channels = AUDIO_CHANNELS_MONO;             // Mono channel
  config->bits_per_sample = AUDIO_BITS_PER_SAMPLE_16; // 16-bit samples
  config->codec = PLATFORM_AUDIO_CODEC_AAC;           // AAC as default for better compression
  config->bitrate = AUDIO_BITRATE_AAC_DEFAULT;        // 64 kbps default bitrate for AAC

  return ONVIF_SUCCESS;
}

/**
 * @brief Check if audio configuration is supported by platform
 * @param config Audio configuration to check (must not be NULL)
 * @return 1 if supported, 0 if not supported
 * @note Checks platform-specific limitations with enhanced AAC support
 */
int audio_validation_is_supported(const platform_audio_config_t* config) {
  if (!config) {
    return 0;
  }

  // Basic validation first
  if (!audio_validation_validate_config(config)) {
    return 0;
  }

  // Platform-specific limitations based on Anyka AK3918 capabilities
  // From ai_demo and akipc analysis, enhanced for AAC support:

  // Sample rate limitations - enhanced for AAC
  if (config->sample_rate != AUDIO_SAMPLE_RATE_8K && config->sample_rate != AUDIO_SAMPLE_RATE_16K && config->sample_rate != AUDIO_SAMPLE_RATE_22K &&
      config->sample_rate != AUDIO_SAMPLE_RATE_44K && config->sample_rate != AUDIO_SAMPLE_RATE_48K) {
    // Support common sample rates for AAC
    return 0;
  }

  // Channel limitations - only mono is supported
  if (config->channels != AUDIO_CHANNELS_MONO) {
    return 0;
  }

  // Bits per sample - only 16-bit is supported
  if (config->bits_per_sample != AUDIO_BITS_PER_SAMPLE_16) {
    return 0;
  }

  // Codec limitations - PCM and AAC are supported
  if (config->codec != PLATFORM_AUDIO_CODEC_PCM && config->codec != PLATFORM_AUDIO_CODEC_AAC) {
    return 0;
  }

  // AAC-specific validation
  if (config->codec == PLATFORM_AUDIO_CODEC_AAC) {
    // AAC bitrate validation (8-128 kbps for mono)
    if (config->bitrate < AUDIO_BITRATE_AAC_MIN || config->bitrate > AUDIO_BITRATE_AAC_MAX) {
      return 0;
    }

    // AAC sample rate validation (8kHz, 16kHz, 22.05kHz, 44.1kHz, 48kHz)
    if (config->sample_rate != AUDIO_SAMPLE_RATE_8K && config->sample_rate != AUDIO_SAMPLE_RATE_16K && config->sample_rate != AUDIO_SAMPLE_RATE_22K &&
        config->sample_rate != AUDIO_SAMPLE_RATE_44K && config->sample_rate != AUDIO_SAMPLE_RATE_48K) {
      return 0;
    }
  }

  return 1;
}
