
/**
 * @brief
 *  N1 Hikvision device event and interface definitions.
 * @details
 *  Users implement air-pairing event responses through this module to achieve
 *  the corresponding functionality.\n
 *  The module loads events via the @ref NK_N1Device_Hikvision() interface.\n
 *  This interface must be called only after @ref NK_N1Device_Init() has been
 *  called successfully; otherwise it will fail.
 */

#include "n1_device.h"

#ifndef NK_N1_DEVICE_HIKVISION_H_
#define NK_N1_DEVICE_HIKVISION_H_
NK_CPP_EXTERN_BEGIN

/**
 * @brief
 *  Event definitions.
 */
typedef struct Nk_N1DeviceEventHikvision {

	/**
	 * @brief
	 *  Get listening port event.
	 * @details
	 *
	 * @return
	 *  Returns the listening port number.
	 */
	NK_UInt16
	(*onGetPort)(NK_PVoid ctx);

	/**
	 * @brief
	 *  Set listening port event.
	 * @details
	 *  This event is triggered when the module's listening port changes.\n
	 *  The user saves the changed port to the configuration.
	 *
	 * @param ctx [in,out]
	 * @param port [in]
	 *  Set port number event; the user receives this event.
	 *
	 */
	NK_Void
	(*onSetPort)(NK_PVoid ctx, NK_UInt16 port);


} NK_N1DeviceEventHikvision;

/**
 * @brief
 *  Configure Hikvision protocol-related events.
 * @details
 *
 * @param Event [in]
 *  User event definition.
 *
 * @return
 *  Returns 0 on success, -1 on failure.
 */
NK_API NK_Int
NK_N1Device_Hikvision(NK_N1DeviceEventHikvision *Event);



NK_CPP_EXTERN_END
#endif /* NK_N1_DEVICE_HIKVISION_H_ */
