
/**
 * @brief
 *  N1 device firmware upgrade event and interface definitions.
 * @details
 *  Users implement firmware upgrade event responses through this module to realize the corresponding functionality.\n
 *  The module loads events via the @ref NK_N1Device_Upgrade() interface.\n
 *  This interface must be called only after @ref NK_N1Device_Init() succeeds, otherwise it will fail.
 *
 */

#include "n1_device.h"

#ifndef NK_N1_DEVICE_UPGRADE_H_
#define NK_N1_DEVICE_UPGRADE_H_
NK_CPP_EXTERN_BEGIN


/**
 * @brief
 *  N1 device firmware upgrade event definitions.
 */
typedef struct Nk_N1DeviceEventUpgrade {

	/**
	 * @brief
	 *  Get user memory.
	 *
	 * @param[in,out] ctx
	 *  User context, passed in when calling @ref NK_N1Device_InitEx().
	 * @param[in] len
	 *  Length of user memory to obtain.
	 * @param[out] mem
	 *  Returns the start address of user memory.
	 *
	 */
	NK_Void
	(*onGetUserMem)(NK_PVoid ctx, NK_Size len, NK_PVoid *mem);

	/**
	 * @brief
	 *  Release user memory.
	 *
	 * @param ctx
	 *
	 * @param[in] len
	 *  Length of the allocated user memory.
	 *
	 * @param[out] mem
	 *  Start address of user memory.
	 *
	 *
	 * @return
	 */
	NK_Void
	(*onPutUserMem)(NK_PVoid ctx, NK_Size len, NK_PVoid mem);


	/**
	 * @brief
	 *  Firmware image upgrade: import firmware file event.\n
	 *  When this event implementation returns NK_N1_OK, the @ref onFirmwareUpgradeFile event is triggered.
	 *
	 * @param[in,out] ctx
	 *  User context, passed in when calling @ref NK_N1Device_Init.
	 *
	 * @param[in] file
	 *  Start address of firmware file memory buffer.
	 *
	 * @param[in] size
	 *  Size of memory occupied by the firmware file.
	 *
	 * @retval NK_N1_OK
	 *  Firmware has no anomalies; returning this value causes the system to trigger @ref onFirmwareUpgradeFile.
	 *
	 * @retval NK_N1_ERR_UPGRADE_NOT_SUPPORT
	 *  Returning this value indicates the current device does not support firmware upgrade.
	 *
	 * @retval NK_N1_ERR_FIRMWARE_FILE_ERROR
	 *  Returning this value indicates a firmware file error; returned when firmware file validation fails.
	 *
	 * @retval NK_N1_ERR_FIRMWARE_VER_TOO_OLD
	 *  Firmware version is too old; upgrade is rejected.
	 *
	 */
	NK_N1Error
	(*onQualifyFile)(NK_PVoid ctx, NK_PByte file, NK_Size size);

	/**
	 * @brief
	 *  Firmware image upgrade: upgrade firmware file event.
	 *
	 * @param[in,out] ctx
	 *  User context, passed in when calling @ref NK_N1Device_Init.
	 *
	 * @param[in] file
	 *  Start address of firmware file memory buffer.
	 *
	 * @param[in] size
	 *  Size of memory occupied by the firmware file.
	 *
	 * @param[out] rate
	 *  Firmware upgrade progress; the event implementation continuously updates this value based on upgrade progress.
	 *
	 * @retval NK_N1_OK
	 *  Upgrade complete.
	 *
	 * @retval NK_N1_ERR_UPGRADE_INTERRUPT
	 *  Error occurred during upgrade; process interrupted.
	 */
	NK_N1Error
	(*onUpgradeFile)(NK_PVoid ctx, NK_PByte file, NK_Size size, NK_Size *rate);

	/**
	 * @brief
	 *  Firmware image upgrade: upgrade ROM firmware file event.\n
	 *  When the user implements this event, the module internally implements @ref onQualifyFile for validity verification based on ROM file characteristics, then triggers this event.\n
	 *  When this event is implemented, the @ref onQualifyFile and @ref onUpgradeFile events will not be triggered.\n
	 *
	 * @param[in,out] ctx
	 *  User context, passed in when calling @ref NK_N1Device_Init.
	 * @param[in] version
	 *  Firmware version code; decompose into individual version codes using @ref NK_N1_VER_DEC().
	 * @param[in] file
	 *  Start address of firmware file memory buffer.
	 * @param[in] size
	 *  Size of memory occupied by the firmware file.
	 * @param[out] rate
	 *  Firmware upgrade progress; the event implementation continuously updates this value based on upgrade progress.
	 *
	 * @retval NK_N1_OK
	 *  Upgrade complete.
	 *
	 * @retval NK_N1_ERR_UPGRADE_INTERRUPT
	 *  Error occurred during upgrade; process interrupted.
	 */
	NK_N1Error
	(*onUpgradeROM)(NK_PVoid ctx, NK_Size version, NK_PByte file, NK_Size size, NK_Size *rate);


	/**
	 * Firmware image upgrade end event.
	 * Triggered after @ref onFirmwareUpgradeFile or @ref onFirmwareUpgradeFileROM completes.
	 *
	 * @param[in,out]	ctx
	 *
	 */
	NK_N1Error
	(*onUpgradeEnd)(NK_PVoid ctx);

} NK_N1DeviceEventUpgrade;


typedef struct Nk_N1DeviceUpgrade {
	/**
	 * Device port number.
	 *
	 * @param[in]	port,
	 * This port is the same as the N1 device port number.
	 */
	NK_Int devport;
	/**
	 * Separate interface for getting upgrade progress, distinct from onUpgradeROM progress retrieval.
	 * Registered via @ref NK_N1UpgradeRate_GetEvent.
	 *
	 * @param[in,out]	Rate
	 *
	 * @param[in,out]	ctx
	 */
	NK_Int
	(*onGetUpgradeRate)(NK_PVoid ctx, NK_Int *Rate);
} NK_N1DeviceUpgrade;

/**
 * @brief
 *  Configure N1 firmware upgrade related events.
 * @details
 *
 * @param Event [in]
 *  User event definition.
 *
 * @return
 *  Returns 0 on success, -1 otherwise.
 */
NK_API NK_Int
NK_N1Device_Upgrade(NK_N1DeviceEventUpgrade *Event);

/**
 * @brief
 *  Configure the N1 separate upgrade progress retrieval interface.
 * @details
 * This interface is valid after device deregistration @ref NK_N1Device_Destroy or when the N1 device @ref NK_N1Device_InitV2 has not been initialized.
 *
 * @param startup [in]
 *  Whether to enable.
 *
 * @param DeviceUpgrade [in]
 *  User event definition.
 *
 * @ret Returns 0 on success, -1 otherwise.
 */
NK_API NK_Int
NK_N1UpgradeRate_GetEvent(NK_Boolean startup, NK_N1DeviceUpgrade *DeviceUpgrade);


NK_CPP_EXTERN_END
#endif /* NK_N1_DEVICE_UPGRADE_H_ */
