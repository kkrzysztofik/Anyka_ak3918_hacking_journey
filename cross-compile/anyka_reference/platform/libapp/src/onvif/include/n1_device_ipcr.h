
/**
 * @brief
 *  N1 device IPC router-related API interface definitions.
 * @details
 *
 */

#include "n1_device.h"

#ifndef NK_N1_DEVICE_IPCR_H_
#define NK_N1_DEVICE_IPCR_H_
NK_CPP_EXTERN_BEGIN


/**
 * @brief
 *  N1 device two-way audio related event definitions.
 */
typedef struct Nk_N1DeviceEventIPCR {

	/**
	 * @brief
	 *  Set/clear the router serial number.
	 * @details
	 *  Sets the serial number of the currently connected router. This event is triggered
	 *  when the IPC router is paired. The user should save the serial number for later
	 *  retrieval via the @ref onGetRID event.\n
	 *  When @ref rid is passed as Nil, it indicates clearing the corresponding router
	 *  serial number connection.
	 *
	 * @param ctx [in,out]
	 *  Event context.
	 * @param rid [in]
	 *  Router serial number.
	 */
	NK_Void
	(*onSetRID)(NK_PVoid *ctx, NK_PChar rid);

	/**
	 * @brief
	 *  Get the paired router serial number.
	 * @details
	 *  If the current device has already been paired with an IPC router, returns that
	 *  router's serial number. If no router pairing has been performed, this event
	 *  returns immediately.
	 *
	 * @param ctx [in,out]
	 *  Event context.
	 * @param stack [out]
	 *  Start address of the memory for the serial number result.
	 * @param stacklen [in]
	 *  Size of the memory for the serial number result.
	 */
	NK_Void
	(*onGetRID)(NK_PVoid *ctx, NK_PChar stack, NK_Size stacklen);

	/**
	 * @brief
	 *  Paired router keepalive event.
	 *
	 * @param ctx [in,out]
	 *  Event context.
	 *
	 */
	NK_Void
	(*onKeepAlive)(NK_PVoid *ctx);


} NK_N1DeviceEventIPCR;

/**
 * @brief
 *  Configure N1 device production-related events.
 * @details
 *
 * @param Event [in]
 *  User event definition.
 *
 * @return
 *  Returns 0 on success, -1 on failure.
 */
NK_API NK_Int
NK_N1Device_IPCR(NK_N1DeviceEventIPCR *Event);



NK_CPP_EXTERN_END
#endif /* NK_N1_DEVICE_IPCR_H_ */
