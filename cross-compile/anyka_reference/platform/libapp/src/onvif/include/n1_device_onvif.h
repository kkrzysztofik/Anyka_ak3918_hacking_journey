
/**
 * @brief
 *  N1 ONVIF device event and interface definitions.
 * @details
 *  Users implement ONVIF event responses through this module to achieve
 *  the corresponding functionality.
 *
 */

#include "n1_device.h"

#ifndef NK_N1_DEVICE_ONVIF_H_
#define NK_N1_DEVICE_ONVIF_H_
NK_CPP_EXTERN_BEGIN


/**
 * @brief
 *  Related response event definitions.
 */
typedef struct Nk_N1DeviceEventOnvif {

	/**
	 * @brief
	 *  Set the IP auto-adapt switch.
	 *
	 * @details
	 *  IP auto-adapt is an address adaptation strategy designed around ONVIF device
	 *  discovery limitations. When the device receives an ONVIF discovery packet, it
	 *  modifies the local address based on the source address to form a UDP unicast
	 *  reply path.\n
	 *  This event triggers the master switch for the IP auto-adapt feature based on
	 *  external user control logic.
	 *
	 * @param ctx [in]
	 *  User context.
	 * @param on [in]
	 *  Switch flag; NK_True means enabled, NK_False means disabled.
	 *
	 */
	NK_Void
	(*onSetAutoIPAdapt)(NK_PVoid ctx, NK_Boolean on);

	/**
	 * @brief
	 *  Get the IP auto-adapt switch.
	 *
	 * @details
	 *  See @ref onSetAutoIPAdapt. This event retrieves the master switch state of the
	 *  IP auto-adapt feature based on external user control logic.
	 *
	 * @param ctx [in]
	 *  User context.
	 *
	 * @return
	 *  Returns the switch status; NK_True means enabled, NK_False means disabled.
	 */
	NK_Boolean
	(*onGetAutoIPAdapt)(NK_PVoid ctx);

	/**
	 * @brief
	 *  Activate the ONVIF IP auto-adapt feature.
	 *
	 * @details
	 *  The IP auto-adapt feature activation event. Under the precondition that the IP
	 *  auto-adapt feature is enabled (see @ref onSetAutoIPAdapt), the feature may be
	 *  temporarily deactivated by external factors. Triggering this event implements
	 *  user control over activation.
	 *
	 * @param ctx [in]
	 *  User context.
	 * @param actived [in]
	 *  Activity flag; NK_True means active, NK_False means inactive.
	 *
	 */
	NK_Void
	(*onActiveAutoIPAdapt)(NK_PVoid ctx, NK_Boolean actived);

} NK_N1DeviceEventOnvif;

/**
 * @brief
 *  Configure related extension events.
 * @details
 *
 * @param Event [in]
 *  User event definition.
 *
 * @return
 *  Returns 0 on success, -1 on failure.
 */
NK_API NK_Int
NK_N1Device_Onvif(NK_N1DeviceEventOnvif *Event);



NK_CPP_EXTERN_END
#endif /* NK_N1_DEVICE_ONVIF_H_ */
