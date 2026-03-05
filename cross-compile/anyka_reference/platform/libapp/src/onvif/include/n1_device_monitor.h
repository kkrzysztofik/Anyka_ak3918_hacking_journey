
/**
 * @brief
 *  N1 system monitoring event and interface definitions.
 * @details
 *  Users implement system monitoring event responses through this module to
 *  achieve the corresponding functionality.
 *
 */

#include "n1_device.h"

#ifndef NK_N1_DEVICE_MONITOR_H_
#define NK_N1_DEVICE_MONITOR_H_
NK_CPP_EXTERN_BEGIN


/**
 * @brief
 *  Related response event definitions.
 */
typedef struct Nk_N1DeviceEventMonitor {

	/**
	 * @brief
	 *  Get CPU usage.
	 *
	 * @return
	 *  Returns CPU usage as an integer in per-mille (parts per thousand), range 0-1000.
	 *  For example, 50.5% would return 505.
	 *
	 */
	NK_Size
	(*onGetCPUUsage)(NK_PVoid ctx);

	/**
	 * @brief
	 *  Get memory usage.
	 *
	 */
	NK_Size
	(*onGetMemoryUsage)(NK_PVoid ctx);


} NK_N1DeviceEventMonitor;

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
NK_N1Device_Monitor(NK_N1DeviceEventMonitor *Event);



NK_CPP_EXTERN_END
#endif /* NK_N1_DEVICE_MONITOR_H_ */
