/**
 * @file thread_mock.h
 * @brief CMocka-based pthread mock helpers
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef THREAD_MOCK_H
#define THREAD_MOCK_H

#include <pthread.h>
#include <stdbool.h>

void thread_mock_use_real_function(bool use_real);

int __wrap_pthread_create(pthread_t* thread, const pthread_attr_t* attr, void* (*start_routine)(void*), void* arg);
int __wrap_pthread_detach(pthread_t thread);

#endif // THREAD_MOCK_H
