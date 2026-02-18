
/**
 * @brief
 *  N1 air-pairing event and interface definitions.
 * @details
 *  Users implement air-pairing event responses through this module to realize the corresponding functionality.\n
 *  The module loads events via the @ref NK_N1Device_AirPair() interface.\n
 *  This interface must be called only after @ref NK_N1Device_Init() succeeds, otherwise it will fail.
 *
 */

#include "n1_device.h"

#ifndef NK_N1_DEVICE_AIRPAIR_H_
#define NK_N1_DEVICE_AIRPAIR_H_
NK_CPP_EXTERN_BEGIN


/**
 * @brief
 *  Related response event definitions.
 */
typedef struct Nk_N1DeviceEventAirPair {

	/**
	 * WiFi hotspot scan event.\n
	 * Triggered when the module needs to detect nearby WiFi NVRs.\n
	 * Users implement this interface to inform the module of nearby WiFi hotspot information.
	 *
	 * @param[in,out]	ctx				User context, passed in when calling @ref NK_N1Device_Init.
	 * @param[out]		HotSpots		Stack buffer for hotspot data structures.
	 * @param[in,out]	n_hotSpots		Input: stack buffer capacity for hotspot structures; output: actual number of hotspots scanned.
	 *
	 * @retval NK_N1_ERR_NONE					Scan succeeded; results saved in @ref HotSpots array.
	 *
	 */
	NK_N1Error
	(*onScanWiFiHotSpot)(NK_PVoid ctx, NK_WiFiHotSpot *HotSpots, NK_Size *n_hotSpots);


	/**
	 * WiFi hotspot connection event.\n
	 * For wireless-capable devices, triggered when the module wants the user to connect to an NVR device.\n
	 * When this event is triggered, a configuration event for @ref NK_N1LanSetup::WiFiNVR is also initiated.\n
	 * Unlike the @ref onLanSetup event, when the configuration event for @ref NK_N1LanSetup::WiFiNVR is triggered here,\n
	 * the user does not need to save the @ref NK_N1LanSetup::WiFiNVR configuration data,\n
	 * whereas the @ref onLanSetup event requires the user to save the data structure corresponding to @ref NK_N1LanSetup::WiFiNVR.\n
	 *
	 * @param[in,out]	ctx				User context, passed in when calling @ref NK_N1Device_Init.
	 * @param[in,out]	Setup			LAN configuration data structure.
	 *
	 * @retval NK_N1_ERR_NONE					Hotspot connection succeeded.
	 */
	NK_N1Error
	(*onConnectWiFiHotSpot)(NK_PVoid ctx, NK_N1LanSetup *Setup);

} NK_N1DeviceEventAirPair;

/**
 * @brief
 *  Configure related extension events.
 * @details
 *
 * @param Event [in]
 *  User event definition.
 *
 * @return
 *  Returns 0 on success, -1 otherwise.
 */
NK_API NK_Int
NK_N1Device_AirPair(NK_N1DeviceEventAirPair *Event);


/**
 * @brief
 *  WiFi NVR pairing control.
 * @details
 *  If the device has Wi-Fi connectivity, call this interface to pair and connect with a Wi-Fi NVR.\n
 *  The @ref enabled parameter controls whether to enable or disable Wi-Fi NVR pairing.\n
 *  When Wi-Fi NVR pairing is enabled,\n
 *  a background thread is started to maintain the wireless connection with the Wi-Fi NVR until pairing is disabled.\n
 *  While maintaining the wireless connection with the Wi-Fi NVR,\n
 *  the NK_N1DeviceEventAirPair::onScanHotSpots and NK_N1DeviceEventAirPair::onConnectWiFiHotSpot events will be triggered;\n
 *  users must implement these interfaces for Wi-Fi NVR pairing to function correctly.
 *
 * @param enabled [in]
 *  Enable/disable Wi-Fi NVR pairing flag.
 *
 * @return
 *  Returns 0 on success, -1 on failure.
 */
NK_API NK_Int
NK_N1Device_PairWiFiNVR(NK_Boolean enabled);



NK_CPP_EXTERN_END
#endif /* NK_N1_DEVICE_AIRPAIR_H_ */
