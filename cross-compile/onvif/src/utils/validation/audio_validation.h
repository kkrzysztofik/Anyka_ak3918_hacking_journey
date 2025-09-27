/**
 * @file audio_validation.h
 * @brief Audio parameter validation utilities
 * @author kkrzysztofik
 * @date 2025
 *
 */

#ifndef AUDIO_VALIDATION_H
#define AUDIO_VALIDATION_H

#include "platform/platform_common.h"
#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Validate audio configuration parameters
 * @param config Audio configuration to validate (must not be NULL)
 * @return 1 if valid, 0 if invalid
 * @note Validates sample rate, channels, bits per sample, and codec type
 */
int audio_validation_validate_config(const platform_audio_config_t* config);

/**
 * @brief Validate audio sample rate
 * @param sample_rate Sample rate to validate
 * @return 1 if valid, 0 if invalid
 * @note Valid sample rates: 8000, 16000, 22050, 44100, 48000
 */
int audio_validation_validate_sample_rate(int sample_rate);

/**
 * @brief Validate audio channel count
 * @param channels Number of channels to validate
 * @return 1 if valid, 0 if invalid
 * @note Valid channels: 1 (mono), 2 (stereo)
 */
int audio_validation_validate_channels(int channels);

/**
 * @brief Validate audio bits per sample
 * @param bits_per_sample Bits per sample to validate
 * @return 1 if valid, 0 if invalid
 * @note Valid values: 8, 16, 24, 32
 */
int audio_validation_validate_bits_per_sample(int bits_per_sample);

/**
 * @brief Validate audio codec type
 * @param codec Audio codec to validate
 * @return 1 if valid, 0 if invalid
 * @note Valid codecs: PCM, AAC, G711A, G711U
 */
int audio_validation_validate_codec(platform_audio_codec_t codec);

/**
 * @brief Get default audio configuration
 * @param config Output parameter for default configuration (must not be NULL)
 * @return 0 on success, -1 on error
 * @note Sets safe default values for Anyka platform
 */
int audio_validation_get_default_config(platform_audio_config_t* config);

/**
 * @brief Check if audio configuration is supported by platform
 * @param config Audio configuration to check (must not be NULL)
 * @return 1 if supported, 0 if not supported
 * @note Checks platform-specific limitations
 */
int audio_validation_is_supported(const platform_audio_config_t* config);

#ifdef __cplusplus
}
#endif

#endif /* AUDIO_VALIDATION_H */
