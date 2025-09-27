/**
 * @file rtsp_sdp.c
 * @brief RTSP SDP Implementation
 * @author kkrzysztofik
 * @date 2025
 *
 * This file contains all SDP (Session Description Protocol) related functions
 * for the RTSP server, including SDP generation, parsing, and validation.
 */

#include "rtsp_sdp.h"

#include "rtsp_types.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

/* ==================== SDP Functions ==================== */

/**
 * Initialize SDP session
 */
int sdp_init_session(struct sdp_session* sdp, const char* session_name, const char* origin) {
  if (!sdp)
    return -1;

  memset(sdp, 0, sizeof(struct sdp_session));
  sdp->version = 0;
  sdp->media = NULL;

  if (session_name) {
    strncpy(sdp->session_name, session_name, sizeof(sdp->session_name) - 1);
    sdp->session_name[sizeof(sdp->session_name) - 1] = '\0';
  } else {
    strcpy(sdp->session_name, "RTSP Session");
  }

  if (origin) {
    strncpy(sdp->origin, origin, sizeof(sdp->origin) - 1);
    sdp->origin[sizeof(sdp->origin) - 1] = '\0';
  } else {
    snprintf(sdp->origin, sizeof(sdp->origin), "- %u %u IN IP4 0.0.0.0", (uint32_t)time(NULL),
             (uint32_t)time(NULL));
  }

  return 0;
}

/**
 * Cleanup SDP session
 */
void sdp_cleanup_session(struct sdp_session* sdp) {
  if (!sdp)
    return;

  struct sdp_media* media = sdp->media;
  while (media) {
    struct sdp_media* next = media->next;
    free(media);
    media = next;
  }
  sdp->media = NULL;
}

/**
 * Add media to SDP session
 */
int sdp_add_media(struct sdp_session* sdp, sdp_media_type_t type, int port, const char* protocol,
                  int payload_type, const char* encoding, int clock_rate, int channels) {
  if (!sdp)
    return -1;

  struct sdp_media* media = malloc(sizeof(struct sdp_media));
  if (!media)
    return -1;

  memset(media, 0, sizeof(struct sdp_media));
  media->type = type;
  media->port = port;
  media->payload_type = payload_type;
  media->clock_rate = clock_rate;
  media->channels = channels;
  media->direction = SDP_DIR_SENDRECV;

  if (protocol) {
    strncpy(media->protocol, protocol, sizeof(media->protocol) - 1);
    media->protocol[sizeof(media->protocol) - 1] = '\0';
  } else {
    strcpy(media->protocol, "RTP/AVP");
  }

  if (encoding) {
    strncpy(media->encoding, encoding, sizeof(media->encoding) - 1);
    media->encoding[sizeof(media->encoding) - 1] = '\0';
  } else {
    strcpy(media->encoding, "H264");
  }

  // Add to linked list
  media->next = sdp->media;
  sdp->media = media;

  return 0;
}

/**
 * Set media direction
 */
int sdp_set_media_direction(struct sdp_session* sdp, sdp_media_type_t type,
                            sdp_direction_t direction) {
  if (!sdp)
    return -1;

  struct sdp_media* media = sdp->media;
  while (media) {
    if (media->type == type) {
      media->direction = direction;
      return 0;
    }
    media = media->next;
  }

  return -1; // Media not found
}

/**
 * Set media control
 */
int sdp_set_media_control(struct sdp_session* sdp, sdp_media_type_t type, const char* control) {
  if (!sdp || !control)
    return -1;

  struct sdp_media* media = sdp->media;
  while (media) {
    if (media->type == type) {
      strncpy(media->control, control, sizeof(media->control) - 1);
      media->control[sizeof(media->control) - 1] = '\0';
      return 0;
    }
    media = media->next;
  }

  return -1; // Media not found
}

/**
 * Set media fmtp
 */
int sdp_set_media_fmtp(struct sdp_session* sdp, sdp_media_type_t type, const char* fmtp) {
  if (!sdp || !fmtp)
    return -1;

  struct sdp_media* media = sdp->media;
  while (media) {
    if (media->type == type) {
      strncpy(media->fmtp, fmtp, sizeof(media->fmtp) - 1);
      media->fmtp[sizeof(media->fmtp) - 1] = '\0';
      return 0;
    }
    media = media->next;
  }

  return -1; // Media not found
}

/**
 * Set media RTCP feedback
 */
int sdp_set_media_rtcp_fb(struct sdp_session* sdp, sdp_media_type_t type, const char* rtcp_fb) {
  if (!sdp || !rtcp_fb)
    return -1;

  struct sdp_media* media = sdp->media;
  while (media) {
    if (media->type == type) {
      strncpy(media->rtcp_fb, rtcp_fb, sizeof(media->rtcp_fb) - 1);
      media->rtcp_fb[sizeof(media->rtcp_fb) - 1] = '\0';
      return 0;
    }
    media = media->next;
  }

  return -1; // Media not found
}

/**
 * Set media extension map
 */
int sdp_set_media_extmap(struct sdp_session* sdp, sdp_media_type_t type, const char* extmap) {
  if (!sdp || !extmap)
    return -1;

  struct sdp_media* media = sdp->media;
  while (media) {
    if (media->type == type) {
      strncpy(media->extmap, extmap, sizeof(media->extmap) - 1);
      media->extmap[sizeof(media->extmap) - 1] = '\0';
      return 0;
    }
    media = media->next;
  }

  return -1; // Media not found
}

/**
 * Set media MID
 */
int sdp_set_media_mid(struct sdp_session* sdp, sdp_media_type_t type, const char* mid) {
  if (!sdp || !mid)
    return -1;

  struct sdp_media* media = sdp->media;
  while (media) {
    if (media->type == type) {
      strncpy(media->mid, mid, sizeof(media->mid) - 1);
      media->mid[sizeof(media->mid) - 1] = '\0';
      return 0;
    }
    media = media->next;
  }

  return -1; // Media not found
}

/**
 * Set media SSRC
 */
int sdp_set_media_ssrc(struct sdp_session* sdp, sdp_media_type_t type, const char* ssrc) {
  if (!sdp || !ssrc)
    return -1;

  struct sdp_media* media = sdp->media;
  while (media) {
    if (media->type == type) {
      strncpy(media->ssrc, ssrc, sizeof(media->ssrc) - 1);
      media->ssrc[sizeof(media->ssrc) - 1] = '\0';
      return 0;
    }
    media = media->next;
  }

  return -1; // Media not found
}

/**
 * Generate SDP content
 */
int sdp_generate(struct sdp_session* sdp, char* buffer, size_t buffer_size) {
  if (!sdp || !buffer || buffer_size < 256)
    return -1;

  int len = 0;

  // Version
  len += snprintf(buffer + len, buffer_size - len, "v=%d\r\n", sdp->version);

  // Origin
  len += snprintf(buffer + len, buffer_size - len, "o=%s\r\n", sdp->origin);

  // Session name
  len += snprintf(buffer + len, buffer_size - len, "s=%s\r\n", sdp->session_name);

  // Session info
  if (sdp->session_info[0]) {
    len += snprintf(buffer + len, buffer_size - len, "i=%s\r\n", sdp->session_info);
  }

  // URI
  if (sdp->uri[0]) {
    len += snprintf(buffer + len, buffer_size - len, "u=%s\r\n", sdp->uri);
  }

  // Email
  if (sdp->email[0]) {
    len += snprintf(buffer + len, buffer_size - len, "e=%s\r\n", sdp->email);
  }

  // Phone
  if (sdp->phone[0]) {
    len += snprintf(buffer + len, buffer_size - len, "p=%s\r\n", sdp->phone);
  }

  // Connection
  if (sdp->connection[0]) {
    len += snprintf(buffer + len, buffer_size - len, "c=%s\r\n", sdp->connection);
  }

  // Bandwidth
  if (sdp->bandwidth[0]) {
    len += snprintf(buffer + len, buffer_size - len, "b=%s\r\n", sdp->bandwidth);
  }

  // Time
  len += snprintf(buffer + len, buffer_size - len, "t=0 0\r\n");

  // Time zone
  if (sdp->time_zone[0]) {
    len += snprintf(buffer + len, buffer_size - len, "z=%s\r\n", sdp->time_zone);
  }

  // Key
  if (sdp->key[0]) {
    len += snprintf(buffer + len, buffer_size - len, "k=%s\r\n", sdp->key);
  }

  // Attributes
  if (sdp->attributes[0]) {
    len += snprintf(buffer + len, buffer_size - len, "a=%s\r\n", sdp->attributes);
  }

  // Media descriptions
  struct sdp_media* media = sdp->media;
  while (media) {
    // Media type
    const char* media_type_str;
    switch (media->type) {
    case SDP_MEDIA_VIDEO:
      media_type_str = "video";
      break;
    case SDP_MEDIA_AUDIO:
      media_type_str = "audio";
      break;
    case SDP_MEDIA_APPLICATION:
      media_type_str = "application";
      break;
    default:
      media_type_str = "video";
      break;
    }

    len += snprintf(buffer + len, buffer_size - len, "m=%s %d %s %d\r\n", media_type_str,
                    media->port, media->protocol, media->payload_type);

    // RTP map
    len += snprintf(buffer + len, buffer_size - len, "a=rtpmap:%d %s/%d", media->payload_type,
                    media->encoding, media->clock_rate);
    if (media->channels > 0) {
      len += snprintf(buffer + len, buffer_size - len, "/%d", media->channels);
    }
    len += snprintf(buffer + len, buffer_size - len, "\r\n");

    // Format parameters
    if (media->fmtp[0]) {
      len += snprintf(buffer + len, buffer_size - len, "a=fmtp:%d %s\r\n", media->payload_type,
                      media->fmtp);
    }

    // Direction
    const char* direction_str;
    switch (media->direction) {
    case SDP_DIR_SENDRECV:
      direction_str = "sendrecv";
      break;
    case SDP_DIR_SENDONLY:
      direction_str = "sendonly";
      break;
    case SDP_DIR_RECVONLY:
      direction_str = "recvonly";
      break;
    case SDP_DIR_INACTIVE:
      direction_str = "inactive";
      break;
    default:
      direction_str = "sendrecv";
      break;
    }
    len += snprintf(buffer + len, buffer_size - len, "a=%s\r\n", direction_str);

    // Control
    if (media->control[0]) {
      len += snprintf(buffer + len, buffer_size - len, "a=control:%s\r\n", media->control);
    }

    // RTCP feedback
    if (media->rtcp_fb[0]) {
      len += snprintf(buffer + len, buffer_size - len, "a=rtcp-fb:%d %s\r\n", media->payload_type,
                      media->rtcp_fb);
    }

    // Extension map
    if (media->extmap[0]) {
      len += snprintf(buffer + len, buffer_size - len, "a=extmap:%s\r\n", media->extmap);
    }

    // MID
    if (media->mid[0]) {
      len += snprintf(buffer + len, buffer_size - len, "a=mid:%s\r\n", media->mid);
    }

    // SSRC
    if (media->ssrc[0]) {
      len += snprintf(buffer + len, buffer_size - len, "a=ssrc:%s\r\n", media->ssrc);
    }

    media = media->next;
  }

  return len;
}

/**
 * Parse SDP content
 */
int sdp_parse(struct sdp_session* sdp, const char* sdp_text) {
  if (!sdp || !sdp_text)
    return -1;

  // Initialize SDP session
  sdp_init_session(sdp, NULL, NULL);

  const char* line_start = sdp_text;
  while (*line_start) {
    const char* line_end = strstr(line_start, "\r\n");
    if (!line_end)
      line_end = strchr(line_start, '\n');
    if (!line_end)
      break;

    size_t line_len = line_end - line_start;
    if (line_len < 2) {
      line_start = line_end + 1;
      continue;
    }

    char line[512];
    if (line_len >= sizeof(line))
      line_len = sizeof(line) - 1;
    strncpy(line, line_start, line_len);
    line[line_len] = '\0';

    // Parse SDP line
    if (line[0] == 'v' && line[1] == '=') {
      sdp->version = atoi(line + 2);
    } else if (line[0] == 'o' && line[1] == '=') {
      strncpy(sdp->origin, line + 2, sizeof(sdp->origin) - 1);
      sdp->origin[sizeof(sdp->origin) - 1] = '\0';
    } else if (line[0] == 's' && line[1] == '=') {
      strncpy(sdp->session_name, line + 2, sizeof(sdp->session_name) - 1);
      sdp->session_name[sizeof(sdp->session_name) - 1] = '\0';
    } else if (line[0] == 'i' && line[1] == '=') {
      strncpy(sdp->session_info, line + 2, sizeof(sdp->session_info) - 1);
      sdp->session_info[sizeof(sdp->session_info) - 1] = '\0';
    } else if (line[0] == 'u' && line[1] == '=') {
      strncpy(sdp->uri, line + 2, sizeof(sdp->uri) - 1);
      sdp->uri[sizeof(sdp->uri) - 1] = '\0';
    } else if (line[0] == 'e' && line[1] == '=') {
      strncpy(sdp->email, line + 2, sizeof(sdp->email) - 1);
      sdp->email[sizeof(sdp->email) - 1] = '\0';
    } else if (line[0] == 'p' && line[1] == '=') {
      strncpy(sdp->phone, line + 2, sizeof(sdp->phone) - 1);
      sdp->phone[sizeof(sdp->phone) - 1] = '\0';
    } else if (line[0] == 'c' && line[1] == '=') {
      strncpy(sdp->connection, line + 2, sizeof(sdp->connection) - 1);
      sdp->connection[sizeof(sdp->connection) - 1] = '\0';
    } else if (line[0] == 'b' && line[1] == '=') {
      strncpy(sdp->bandwidth, line + 2, sizeof(sdp->bandwidth) - 1);
      sdp->bandwidth[sizeof(sdp->bandwidth) - 1] = '\0';
    } else if (line[0] == 'z' && line[1] == '=') {
      strncpy(sdp->time_zone, line + 2, sizeof(sdp->time_zone) - 1);
      sdp->time_zone[sizeof(sdp->time_zone) - 1] = '\0';
    } else if (line[0] == 'k' && line[1] == '=') {
      strncpy(sdp->key, line + 2, sizeof(sdp->key) - 1);
      sdp->key[sizeof(sdp->key) - 1] = '\0';
    } else if (line[0] == 'a' && line[1] == '=') {
      strncpy(sdp->attributes, line + 2, sizeof(sdp->attributes) - 1);
      sdp->attributes[sizeof(sdp->attributes) - 1] = '\0';
    } else if (line[0] == 'm' && line[1] == '=') {
      // Media line - parse media type, port, protocol, payload type
      char media_type[16], protocol[16];
      int port, payload_type;

      if (sscanf(line + 2, "%15s %d %15s %d", media_type, &port, protocol, &payload_type) == 4) {
        sdp_media_type_t type = SDP_MEDIA_VIDEO;
        if (strcmp(media_type, "audio") == 0)
          type = SDP_MEDIA_AUDIO;
        else if (strcmp(media_type, "application") == 0)
          type = SDP_MEDIA_APPLICATION;

        sdp_add_media(sdp, type, port, protocol, payload_type, NULL, 0, 0);
      }
    }

    line_start = line_end + 1;
  }

  return 0;
}

/**
 * Validate SDP content
 */
int sdp_validate(const char* sdp_text) {
  if (!sdp_text)
    return -1;

  // Check for required SDP fields
  if (strstr(sdp_text, "v=") == NULL)
    return -1; // Version required
  if (strstr(sdp_text, "o=") == NULL)
    return -1; // Origin required
  if (strstr(sdp_text, "s=") == NULL)
    return -1; // Session name required
  if (strstr(sdp_text, "t=") == NULL)
    return -1; // Time required

  return 0;
}
