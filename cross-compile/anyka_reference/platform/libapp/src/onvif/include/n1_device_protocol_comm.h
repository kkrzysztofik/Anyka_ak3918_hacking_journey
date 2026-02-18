
/**
 * @brief
 *  Implementation of common interface functions for N1 Hikvision protocol and
 *  third-party protocol (used to implement a protocol search priority strategy).
 * @details
 *  Users access and configure common interfaces for the Hikvision protocol and
 *  third-party protocols through this module.\n
 *  The module loads events via the @ref NK_N1Device_ProtocolComm() interface.\n
 */

#include "n1_device.h"

#ifndef NK_N1_DEVICE_PROTOCOL_COMM_H_
#define NK_N1_DEVICE_PROTOCOL_COMM_H_
NK_CPP_EXTERN_BEGIN

typedef enum {
	NK_PROTOCOL_SEARCH_TYPE_ZW = 1,
	NK_PROTOCOL_SEARCH_TYPE_HIK,
	NK_PROTOCOL_SEARCH_TYPE_DH,
	NK_PROTOCOL_SEARCH_TYPE_XM,
	NK_PROTOCOL_SEARCH_TYPE_TST,
}NK_ePROTOCOL_SEARCH_TYPE;

typedef struct NK_Search_Device_Type{
	int type;
	struct timespec cpu_clock;   	//
	char mac[32];					//
}NK_stSearch_Device_Type,*NK_lpSearch_Device_Type;

/**
 * @brief
 *  Event definitions.
 */
typedef struct Nk_N1DeviceEventProtocolComm {


	/**
	 * @brief
	 *  Check protocol-related attributes (clock, MAC, etc.).
	 * @details
	 *
	 * @return
	 *   0: not the same device or not sent at the same time;
	 *   1: the same device, the same search sent at the same time;
	 *  -1: error.
	 */
	NK_Int
	(*onCheck_type)(NK_PVoid ctx, NK_stSearch_Device_Type *device);

	/**
	 * @brief
	 *  Set protocol common interface related attributes (clock, MAC, etc.).
	 * @details
	 *
	 * @return
	 *  0 on success, non-zero on failure.
	 */

	NK_Int
	(*onSet_type)(NK_PVoid ctx, NK_stSearch_Device_Type *device);

} NK_N1DeviceEventProtocolComm;

/**
 * @brief
 *  Configure Hikvision protocol and third-party protocol interaction events.
 * @details
 *
 * @param Event [in]
 *  User event definition.
 *
 * @return
 *  Returns 0 on success, -1 on failure.
 */
NK_API NK_Int
NK_N1Device_ProtocolComm(NK_N1DeviceEventProtocolComm *Event);



NK_CPP_EXTERN_END
#endif /* NK_N1_DEVICE_PROTOCOL_COMM_H_ */

