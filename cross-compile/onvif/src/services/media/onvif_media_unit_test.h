/**
 * @file onvif_media_unit_test.h
 * @brief Unit-testing accessors for ONVIF media service internal helpers
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef SERVICES_MEDIA_ONVIF_MEDIA_UNIT_TEST_H
#define SERVICES_MEDIA_ONVIF_MEDIA_UNIT_TEST_H

#ifdef UNIT_TESTING

#include "protocol/gsoap/onvif_gsoap_core.h"

int onvif_media_unit_init_cached_uris(void);
void onvif_media_unit_reset_cached_uris(void);
int onvif_media_unit_get_cached_rtsp_uri(const char* profile_token, char* uri_buffer, size_t buffer_size);
const char* onvif_media_unit_get_main_profile_token(void);
const char* onvif_media_unit_get_sub_profile_token(void);
int onvif_media_unit_validate_profile_token(const char* token);
int onvif_media_unit_validate_protocol(const char* protocol);
int onvif_media_unit_parse_value_from_request(const char* request_body, onvif_gsoap_context_t* gsoap_ctx, const char* xpath, char* value,
                                              size_t value_size);

#endif // UNIT_TESTING

#endif // SERVICES_MEDIA_ONVIF_MEDIA_UNIT_TEST_H
