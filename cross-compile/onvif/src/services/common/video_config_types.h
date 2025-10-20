/**
 * @file video_config_types.h
 * @brief Video configuration type definitions
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef VIDEO_CONFIG_TYPES_H
#define VIDEO_CONFIG_TYPES_H

/* Video configuration constants */
#define VIDEO_PROFILE_NAME_SIZE 64 /* Maximum video profile name length */

/**
 * @brief Video stream encoding configuration
 */
typedef struct {
  char name[VIDEO_PROFILE_NAME_SIZE]; /**< Profile name (customizable) */
  int width;                          /**< Video width in pixels */
  int height;                         /**< Video height in pixels */
  int fps;                            /**< Frames per second */
  int bitrate;                        /**< Bitrate in kbps */
  int gop_size;                       /**< Group of Pictures size */
  int profile;                        /**< Video profile (baseline, main, high) */
  int codec_type;                     /**< Codec type (H.264, H.265, MJPEG) */
  int br_mode;                        /**< Bitrate mode (CBR, VBR) */
} video_config_t;

#endif /* VIDEO_CONFIG_TYPES_H */
