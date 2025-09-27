/**
 * @file onvif_types.h
 * @brief ONVIF service type definitions and common structures
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ONVIF_TYPES_H
#define ONVIF_TYPES_H

#include <stddef.h>

/* ONVIF service types */
typedef enum {
  ONVIF_SERVICE_DEVICE,
  ONVIF_SERVICE_MEDIA,
  ONVIF_SERVICE_PTZ,
  ONVIF_SERVICE_IMAGING,
  ONVIF_SERVICE_SNAPSHOT
} onvif_service_type_t;

/* ONVIF action types are now handled as string names directly */

#endif /* ONVIF_TYPES_H */
