
/**
 * @brief
 *  N1 RTSP device event and interface definitions.
 * @details
 *  Users implement air-pairing event responses through this module to achieve
 *  the corresponding functionality.\n
 *  The module loads events via the @ref NK_N1Device_RTSP() interface.\n
 *  This interface must be called only after @ref NK_N1Device_Init() has been
 *  called successfully; otherwise it will fail.
 */

#include "n1_device.h"

#ifndef NK_N1_DEVICE_RTSP_H_
#define NK_N1_DEVICE_RTSP_H_
NK_CPP_EXTERN_BEGIN

/**
 * @brief
 *  Event definitions.
 */
typedef struct Nk_N1DeviceEventRTSP {

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


} NK_N1DeviceEventRTSP;

/**
 * @brief
 *  Configure RTSP protocol-related events.
 * @details
 *
 * @param Event [in]
 *  User event definition.
 *
 * @return
 *  Returns 0 on success, -1 on failure.
 */
NK_API NK_Int
NK_N1Device_RTSP(NK_N1DeviceEventRTSP *Event);

/**
 * @brief
 *  Add an RTSP listening stream.
 * @brief
 *
 * @param addr [in]
 * @param chid [in]
 * @param streamid [in]
 *
 * @return
 *  Returns 0 on success, -1 on failure.
 */
NK_API NK_Int
NK_N1Device_AddRTSPStream(const NK_PChar addr, NK_Int chid, NK_Int streamid);



NK_CPP_EXTERN_END
#endif /* NK_N1_DEVICE_RTSP_H_ */
