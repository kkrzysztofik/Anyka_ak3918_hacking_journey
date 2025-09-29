/**
 * @file gsoap_mock.h
 * @brief Header for gsoap mock functions
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef GSOAP_MOCK_H
#define GSOAP_MOCK_H

#include <stddef.h>

// Mock gsoap functions
int mock_gsoap_set_init_result(int result);
int mock_gsoap_set_cleanup_result(int result);
int mock_gsoap_set_generate_response_result(int result);
int mock_gsoap_set_get_response_data_result(const char* data, size_t size);

int mock_gsoap_get_init_call_count(void);
int mock_gsoap_get_cleanup_call_count(void);
int mock_gsoap_get_generate_response_call_count(void);
int mock_gsoap_get_get_response_data_call_count(void);

// Mock initialization
void gsoap_mock_init(void);
void gsoap_mock_cleanup(void);

#endif // GSOAP_MOCK_H
