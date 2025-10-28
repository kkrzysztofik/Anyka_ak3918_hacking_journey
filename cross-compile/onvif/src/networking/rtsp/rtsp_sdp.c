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

#include <bits/types.h>
#include <limits.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "rtsp_types.h"

/* ==================== SDP Functions ==================== */

/**
 * Initialize SDP session
 */
int sdp_init_session(struct sdp_session* sdp, const char* session_name, const char* origin) {
  if (!sdp) {
    return -1;
  }

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
    (void)snprintf(sdp->origin, sizeof(sdp->origin), "- %u %u IN IP4 0.0.0.0", (uint32_t)time(NULL), (uint32_t)time(NULL));
  }

  return 0;
}

/**
 * Cleanup SDP session
 */
void sdp_cleanup_session(struct sdp_session* sdp) {
  if (!sdp) {
    return;
  }

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
int sdp_add_media(struct sdp_session* sdp,
                  sdp_media_type_t type,          // NOLINT(bugprone-easily-swappable-parameters) - SDP protocol-defined parameter order
                  int port,                       // NOLINT(bugprone-easily-swappable-parameters) - SDP protocol-defined parameter order
                  const char* protocol,           // NOLINT(bugprone-easily-swappable-parameters) - SDP protocol-defined parameter order
                  int payload_type,               // NOLINT(bugprone-easily-swappable-parameters) - SDP protocol-defined parameter order
                  const char* encoding,           // NOLINT(bugprone-easily-swappable-parameters) - SDP protocol-defined parameter order
                  int clock_rate, int channels) { // NOLINT(bugprone-easily-swappable-parameters) - SDP protocol-defined parameter order
  if (!sdp) {
    return -1;
  }

  struct sdp_media* media = malloc(sizeof(struct sdp_media));
  if (!media) {
    return -1;
  }

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
int sdp_set_media_direction(struct sdp_session* sdp, sdp_media_type_t type, sdp_direction_t direction) {
  if (!sdp) {
    return -1;
  }

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
  if (!sdp || !control) {
    return -1;
  }

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
  if (!sdp || !fmtp) {
    return -1;
  }

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
  if (!sdp || !rtcp_fb) {
    return -1;
  }

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
  if (!sdp || !extmap) {
    return -1;
  }

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
  if (!sdp || !mid) {
    return -1;
  }

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
  if (!sdp || !ssrc) {
    return -1;
  }

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
 * @brief Helper to write optional SDP field
 */
static int write_optional_field(char* buffer, size_t buffer_size, int* offset, char field_type, const char* value) {
  if (!value || !value[0]) {
    return 0;
  }
  int written = snprintf(buffer + *offset, buffer_size - *offset, "%c=%s\r\n", field_type, value);
  *offset += written;
  return written;
}

/**
 * @brief Get media type string
 */
static const char* get_media_type_string(sdp_media_type_t type) {
  switch (type) {
  case SDP_MEDIA_VIDEO:
    return "video";
  case SDP_MEDIA_AUDIO:
    return "audio";
  case SDP_MEDIA_APPLICATION:
    return "application";
  default:
    return "video";
  }
}

/**
 * @brief Get direction string
 */
static const char* get_direction_string(sdp_direction_t direction) {
  switch (direction) {
  case SDP_DIR_SENDRECV:
    return "sendrecv";
  case SDP_DIR_SENDONLY:
    return "sendonly";
  case SDP_DIR_RECVONLY:
    return "recvonly";
  case SDP_DIR_INACTIVE:
    return "inactive";
  default:
    return "sendrecv";
  }
}

/**
 * @brief Write media description to buffer
 */
static int write_media_description(struct sdp_media* media, char* buffer,
                                   size_t buffer_size, // NOLINT(bugprone-easily-swappable-parameters) - SDP protocol-defined parameter order
                                   int offset) {       // NOLINT(bugprone-easily-swappable-parameters) - SDP protocol-defined parameter order
  int len = offset;

  // Media line
  const char* media_type = get_media_type_string(media->type);
  len += snprintf(buffer + len, buffer_size - len, "m=%s %d %s %d\r\n", media_type, media->port, media->protocol, media->payload_type);

  // RTP map
  len += snprintf(buffer + len, buffer_size - len, "a=rtpmap:%d %s/%d", media->payload_type, media->encoding, media->clock_rate);
  if (media->channels > 0) {
    len += snprintf(buffer + len, buffer_size - len, "/%d", media->channels);
  }
  len += snprintf(buffer + len, buffer_size - len, "\r\n");

  // Optional format parameters
  if (media->fmtp[0]) {
    len += snprintf(buffer + len, buffer_size - len, "a=fmtp:%d %s\r\n", media->payload_type, media->fmtp);
  }

  // Direction attribute
  const char* direction = get_direction_string(media->direction);
  len += snprintf(buffer + len, buffer_size - len, "a=%s\r\n", direction);

  // Optional media attributes
  if (media->control[0]) {
    len += snprintf(buffer + len, buffer_size - len, "a=control:%s\r\n", media->control);
  }
  if (media->rtcp_fb[0]) {
    len += snprintf(buffer + len, buffer_size - len, "a=rtcp-fb:%d %s\r\n", media->payload_type, media->rtcp_fb);
  }
  if (media->extmap[0]) {
    len += snprintf(buffer + len, buffer_size - len, "a=extmap:%s\r\n", media->extmap);
  }
  if (media->mid[0]) {
    len += snprintf(buffer + len, buffer_size - len, "a=mid:%s\r\n", media->mid);
  }
  if (media->ssrc[0]) {
    len += snprintf(buffer + len, buffer_size - len, "a=ssrc:%s\r\n", media->ssrc);
  }

  return len;
}

/**
 * Generate SDP content
 */
int sdp_generate(struct sdp_session* sdp, char* buffer, size_t buffer_size) {
  if (!sdp || !buffer || buffer_size < SDP_MIN_BUFFER_SIZE) {
    return -1;
  }

  int len = 0;

  // Version, origin, session name (required)
  len += snprintf(buffer + len, buffer_size - len, "v=%d\r\n", sdp->version);
  len += snprintf(buffer + len, buffer_size - len, "o=%s\r\n", sdp->origin);
  len += snprintf(buffer + len, buffer_size - len, "s=%s\r\n", sdp->session_name);

  // Optional session-level fields
  write_optional_field(buffer, buffer_size, &len, 'i', sdp->session_info);
  write_optional_field(buffer, buffer_size, &len, 'u', sdp->uri);
  write_optional_field(buffer, buffer_size, &len, 'e', sdp->email);
  write_optional_field(buffer, buffer_size, &len, 'p', sdp->phone);
  write_optional_field(buffer, buffer_size, &len, 'c', sdp->connection);
  write_optional_field(buffer, buffer_size, &len, 'b', sdp->bandwidth);

  // Time (required)
  len += snprintf(buffer + len, buffer_size - len, "t=0 0\r\n");

  // Optional timing fields
  write_optional_field(buffer, buffer_size, &len, 'z', sdp->time_zone);
  write_optional_field(buffer, buffer_size, &len, 'k', sdp->key);
  write_optional_field(buffer, buffer_size, &len, 'a', sdp->attributes);

  // Media descriptions
  struct sdp_media* media = sdp->media;
  while (media) {
    len = write_media_description(media, buffer, buffer_size, len);
    media = media->next;
  }

  return len;
}

/**
 * @brief Helper to safely copy SDP field value
 */
static void copy_sdp_field(char* dest, size_t dest_size, const char* src) {
  strncpy(dest, src, dest_size - 1);
  dest[dest_size - 1] = '\0';
}

/**
 * @brief Parse media line from SDP
 */
static void parse_media_line(struct sdp_session* sdp, const char* line_value) {
  char media_type[SDP_MEDIA_TYPE_SIZE];
  char protocol[SDP_PROTOCOL_SIZE];
  int port = 0;
  int payload_type = 0;

  // Parse media line using strtol for proper error handling
  char* endptr = NULL;
  const char* current = line_value;

  // Parse media type (first word)
  int media_len = 0;
  while (current[media_len] && current[media_len] != ' ' && media_len < SDP_MEDIA_TYPE_SIZE - 1) {
    media_type[media_len] = current[media_len];
    media_len++;
  }
  media_type[media_len] = '\0';

  if (media_len == 0) {
    return; // Invalid format
  }

  // Skip whitespace and parse port
  current += media_len;
  while (*current == ' ') {
    current++;
  }

  long port_long = strtol(current, &endptr, SDP_DECIMAL_BASE);
  if (endptr == current || port_long < 0 || port_long > INT_MAX) {
    return; // Invalid port
  }
  port = (int)port_long;

  // Skip whitespace and parse protocol
  current = endptr;
  while (*current == ' ') {
    current++;
  }

  int protocol_len = 0;
  while (current[protocol_len] && current[protocol_len] != ' ' && protocol_len < SDP_PROTOCOL_SIZE - 1) {
    protocol[protocol_len] = current[protocol_len];
    protocol_len++;
  }
  protocol[protocol_len] = '\0';

  if (protocol_len == 0) {
    return; // Invalid format
  }

  // Skip whitespace and parse payload type
  current += protocol_len;
  while (*current == ' ') {
    current++;
  }

  long payload_long = strtol(current, &endptr, SDP_DECIMAL_BASE);
  if (endptr == current || payload_long < 0 || payload_long > INT_MAX) {
    return; // Invalid payload type
  }
  payload_type = (int)payload_long;

  // All parsing successful, add media
  sdp_media_type_t type = SDP_MEDIA_VIDEO;
  if (strcmp(media_type, "audio") == 0) {
    type = SDP_MEDIA_AUDIO;
  } else if (strcmp(media_type, "application") == 0) {
    type = SDP_MEDIA_APPLICATION;
  }
  sdp_add_media(sdp, type, port, protocol, payload_type, NULL, 0, 0);
}

/**
 * @brief Parse single SDP line based on type
 */
static void parse_sdp_line(struct sdp_session* sdp, const char* line) {
  if (line[1] != '=') {
    return; // Invalid SDP line format
  }

  const char* value = line + 2;
  switch (line[0]) {
  case 'v': {
    char* endptr = NULL;
    long version_long = strtol(value, &endptr, SDP_DECIMAL_BASE);
    if (endptr != value && version_long >= 0 && version_long <= INT_MAX) {
      sdp->version = (int)version_long;
    }
    break;
  }
  case 'o':
    copy_sdp_field(sdp->origin, sizeof(sdp->origin), value);
    break;
  case 's':
    copy_sdp_field(sdp->session_name, sizeof(sdp->session_name), value);
    break;
  case 'i':
    copy_sdp_field(sdp->session_info, sizeof(sdp->session_info), value);
    break;
  case 'u':
    copy_sdp_field(sdp->uri, sizeof(sdp->uri), value);
    break;
  case 'e':
    copy_sdp_field(sdp->email, sizeof(sdp->email), value);
    break;
  case 'p':
    copy_sdp_field(sdp->phone, sizeof(sdp->phone), value);
    break;
  case 'c':
    copy_sdp_field(sdp->connection, sizeof(sdp->connection), value);
    break;
  case 'b':
    copy_sdp_field(sdp->bandwidth, sizeof(sdp->bandwidth), value);
    break;
  case 'z':
    copy_sdp_field(sdp->time_zone, sizeof(sdp->time_zone), value);
    break;
  case 'k':
    copy_sdp_field(sdp->key, sizeof(sdp->key), value);
    break;
  case 'a':
    copy_sdp_field(sdp->attributes, sizeof(sdp->attributes), value);
    break;
  case 'm':
    parse_media_line(sdp, value);
    break;
  default:
    // Unknown SDP line type, ignore
    break;
  }
}

/**
 * @brief Extract single line from SDP text
 */
static const char* extract_sdp_line(const char* line_start, char* line_buf, size_t buf_size) {
  const char* line_end = strstr(line_start, "\r\n");
  if (!line_end) {
    line_end = strchr(line_start, '\n');
  }
  if (!line_end) {
    return NULL;
  }

  size_t line_len = line_end - line_start;
  if (line_len < 2) {
    return line_end + 1; // Skip empty lines
  }

  if (line_len >= buf_size) {
    line_len = buf_size - 1;
  }
  strncpy(line_buf, line_start, line_len);
  line_buf[line_len] = '\0';

  return line_end + 1;
}

/**
 * Parse SDP content
 */
int sdp_parse(struct sdp_session* sdp, const char* sdp_text) {
  if (!sdp || !sdp_text) {
    return -1;
  }

  // Initialize SDP session
  sdp_init_session(sdp, NULL, NULL);

  const char* line_start = sdp_text;
  char line[SDP_LINE_BUFFER_SIZE];

  while (*line_start) {
    line_start = extract_sdp_line(line_start, line, sizeof(line));
    if (!line_start) {
      break;
    }
    if (strlen(line) >= 2) {
      parse_sdp_line(sdp, line);
    }
  }

  return 0;
}

/**
 * Validate SDP content
 */
int sdp_validate(const char* sdp_text) {
  if (!sdp_text) {
    return -1;
  }

  // Check for required SDP fields
  if (strstr(sdp_text, "v=") == NULL) {
    return -1; // Version required
  }
  if (strstr(sdp_text, "o=") == NULL) {
    return -1; // Origin required
  }
  if (strstr(sdp_text, "s=") == NULL) {
    return -1; // Session name required
  }
  if (strstr(sdp_text, "t=") == NULL) {
    return -1; // Time required
  }

  return 0;
}
