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
  case 8000:
  case 16000:
  case 22050:
  case 44100:
  case 48000:
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
  return (channels == 1 || channels == 2) ? 1 : 0;
}

/**
 * @brief Validate audio bits per sample
 * @param bits_per_sample Bits per sample to validate
 * @return 1 if valid, 0 if invalid
 * @note Valid values: 8, 16, 24, 32
 */
int audio_validation_validate_bits_per_sample(int bits_per_sample) {
  switch (bits_per_sample) {
  case 8:
  case 16:
  case 24:
  case 32:
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
  config->sample_rate = 16000;              // 16kHz for better AAC quality
  config->channels = 1;                     // Mono channel (AUDIO_CHANNEL_MONO)
  config->bits_per_sample = 16;             // 16-bit samples
  config->codec = PLATFORM_AUDIO_CODEC_AAC; // AAC as default for better compression
  config->bitrate = 64000;                  // 64 kbps default bitrate for AAC

  return 0;
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
  if (config->sample_rate != 8000 && config->sample_rate != 16000 && config->sample_rate != 22050 &&
      config->sample_rate != 44100 && config->sample_rate != 48000) {
    // Support common sample rates for AAC
    return 0;
  }

  // Channel limitations - only mono is supported
  if (config->channels != 1) {
    return 0;
  }

  // Bits per sample - only 16-bit is supported
  if (config->bits_per_sample != 16) {
    return 0;
  }

  // Codec limitations - PCM and AAC are supported
  if (config->codec != PLATFORM_AUDIO_CODEC_PCM && config->codec != PLATFORM_AUDIO_CODEC_AAC) {
    return 0;
  }

  // AAC-specific validation
  if (config->codec == PLATFORM_AUDIO_CODEC_AAC) {
    // AAC bitrate validation (8-128 kbps for mono)
    if (config->bitrate < 8000 || config->bitrate > 128000) {
      return 0;
    }

    // AAC sample rate validation (8kHz, 16kHz, 22.05kHz, 44.1kHz, 48kHz)
    if (config->sample_rate != 8000 && config->sample_rate != 16000 &&
        config->sample_rate != 22050 && config->sample_rate != 44100 &&
        config->sample_rate != 48000) {
      return 0;
    }
  }

  return 1;
}
