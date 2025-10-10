/**
 * @file test_hash_utils.h
 * @brief Header for hash utility unit tests
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef TEST_HASH_UTILS_H
#define TEST_HASH_UTILS_H

#include <stddef.h>

#include "cmocka_wrapper.h"

/* SHA256 core tests */
void test_unit_hash_sha256_empty(void** state);
void test_unit_hash_sha256_known_vector(void** state);
void test_unit_hash_sha256_longer_input(void** state);
void test_unit_hash_sha256_to_hex(void** state);
void test_unit_hash_sha256_compute_hex(void** state);
void test_unit_hash_sha256_null_pointer(void** state);
void test_unit_hash_sha256_to_hex_buffer_too_small(void** state);
void test_unit_hash_sha256_incremental(void** state);

/* Password hashing tests */
void test_unit_hash_password_hashing(void** state);
void test_unit_hash_password_verification(void** state);
void test_unit_hash_password_null_pointers(void** state);
void test_unit_hash_password_buffer_too_small(void** state);
void test_unit_hash_password_invalid_length(void** state);

/* Test suite getter */
const struct CMUnitTest* get_hash_utils_unit_tests(size_t* count);

#endif /* TEST_HASH_UTILS_H */
