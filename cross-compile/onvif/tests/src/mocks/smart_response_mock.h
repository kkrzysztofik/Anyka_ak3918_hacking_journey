/**
 * @file smart_response_mock.h
 * @brief Header for smart response mock functions
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef SMART_RESPONSE_MOCK_H
#define SMART_RESPONSE_MOCK_H

#include <stddef.h>

// Mock smart response functions
int mock_smart_response_set_build_result(int result);

int mock_smart_response_get_build_call_count(void);

// Mock initialization
void smart_response_mock_init(void);
void smart_response_mock_cleanup(void);

#endif // SMART_RESPONSE_MOCK_H
