/**
 * @file buffer_pool_mock.h
 * @brief Header for buffer pool mock functions
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef BUFFER_POOL_MOCK_H
#define BUFFER_POOL_MOCK_H

#include <stddef.h>

// Buffer pool structure
struct buffer_pool_t;
typedef struct buffer_pool_t buffer_pool_t;

// Mock buffer pool functions
int mock_buffer_pool_set_init_result(int result);
int mock_buffer_pool_set_cleanup_result(int result);

int mock_buffer_pool_get_init_call_count(void);
int mock_buffer_pool_get_cleanup_call_count(void);

// Buffer pool actual functions
int buffer_pool_init(buffer_pool_t* pool);
void buffer_pool_cleanup(buffer_pool_t* pool);
void* buffer_pool_get(buffer_pool_t* pool);
void buffer_pool_return(buffer_pool_t* pool, void* buffer);
int buffer_pool_get_stats(buffer_pool_t* pool, int* used, int* total, size_t* memory_used);

// Mock initialization
void buffer_pool_mock_init(void);
void buffer_pool_mock_cleanup(void);

#endif // BUFFER_POOL_MOCK_H
