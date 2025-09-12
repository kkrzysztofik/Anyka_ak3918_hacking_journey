/**
 * @file error_handling.h
 * @brief Common error handling utilities and standardized return codes.
 */

#ifndef ONVIF_ERROR_HANDLING_H
#define ONVIF_ERROR_HANDLING_H

#include "platform/platform.h"

/* Standardized return codes */
#define ONVIF_SUCCESS         0
#define ONVIF_ERROR          -1
#define ONVIF_ERROR_NULL     -2
#define ONVIF_ERROR_INVALID  -3
#define ONVIF_ERROR_MEMORY   -4
#define ONVIF_ERROR_IO       -5
#define ONVIF_ERROR_NETWORK  -6
#define ONVIF_ERROR_TIMEOUT  -7
#define ONVIF_ERROR_NOT_FOUND -8
#define ONVIF_ERROR_ALREADY_EXISTS -9
#define ONVIF_ERROR_NOT_SUPPORTED -10

/* Error handling macros */
#define ONVIF_CHECK_NULL(ptr) \
    do { \
        if ((ptr) == NULL) { \
            platform_log_error("Null pointer error at %s:%d\n", __FILE__, __LINE__); \
            return ONVIF_ERROR_NULL; \
        } \
    } while(0)

#define ONVIF_CHECK_RETURN(expr, error_code) \
    do { \
        int _ret = (expr); \
        if (_ret != 0) { \
            platform_log_error("Operation failed at %s:%d with code %d\n", __FILE__, __LINE__, _ret); \
            return (error_code); \
        } \
    } while(0)

#define ONVIF_CLEANUP_AND_RETURN(cleanup_code, return_code) \
    do { \
        cleanup_code; \
        return (return_code); \
    } while(0)

/* Resource management macros */
#define ONVIF_FREE_IF_NOT_NULL(ptr) \
    do { \
        if ((ptr) != NULL) { \
            free(ptr); \
            (ptr) = NULL; \
        } \
    } while(0)

#define ONVIF_CLOSE_IF_VALID(fd) \
    do { \
        if ((fd) >= 0) { \
            close(fd); \
            (fd) = -1; \
        } \
    } while(0)

#endif /* ONVIF_ERROR_HANDLING_H */
