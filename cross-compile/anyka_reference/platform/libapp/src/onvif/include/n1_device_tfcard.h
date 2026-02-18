
/**
 * N1 device TFCard-related API interfaces.
 */

#include "n1_device.h"

#ifndef NK_N1_DEVICE_TFCARD_H_
#define NK_N1_DEVICE_TFCARD_H_
NK_CPP_EXTERN_BEGIN


/**
 * @brief
 *  TF card status values.
 * @details
 *  Describes the current state of the TF card.
 */
typedef enum Nk_N1TFCardStatus {

	NK_N1_TFC_STAT_NA = (0),
	NK_N1_TFC_STAT_WORK,
	NK_N1_TFC_STAT_EJECTED,
	NK_N1_TFC_STAT_FS_ERROR,
	NK_N1_TFC_STAT_FORMATTING,
	NK_N1_TFC_STAT_ZOMBIE,

} NK_N1TFCardStatus;


/**
 * @brief
 *  N1 device TFCard event definitions.
 */
typedef struct Nk_N1DeviceEventTFCard {

	/**
	 * @brief
	 *  Get the current TF card status event.
	 * @details
	 * @param ctx [in, out]
	 *  Event context.
	 * @return
	 *  Returns the current TF card status.
	 */
	NK_N1TFCardStatus
	(*onStatus)(NK_PVoid ctx);

	/**
	 * @brief
	 *  Check TF card file event.
	 * @details
	 *  This event is triggered when the module needs to check whether the file at
	 *  @ref path exists on the TF card. Returns the file size if it exists, or -1
	 *  if it does not.
	 *
	 * @param ctx [in, out]
	 *  Event context.
	 * @param path [in]
	 *  File path.
	 *
	 * @return
	 *  Returns the file size if the file exists, otherwise returns -1.
	 */
	NK_SSize
	(*onTouch)(NK_PVoid ctx, NK_PChar path);

	/**
	 * @brief
	 *  Remove TF card file event.
	 * @details
	 *  This event is triggered when the module needs to remove the file at @ref path
	 *  from the TF card directory.
	 *
	 * @param ctx [in, out]
	 *  Event context.
	 * @param path [in]
	 *  File path.
	 *
	 */
	NK_Void
	(*onRemove)(NK_PVoid ctx, NK_PChar path);

	/**
	 * @brief
	 *  TF card write data event.
	 * @details
	 *  This event is triggered when the module needs to write data of length
	 *  @ref len to a file path on the TF card.
	 *
	 * @param ctx [in, out]
	 *  Event context.
	 * @param path [in]
	 *  File path.
	 * @param seek [in]
	 *  File position offset; -1 indicates end of file.
	 * @param data [in]
	 *  Start address of the data.
	 * @param len [in]
	 *  Length of the data.
	 *
	 * @return
	 *  Returns the actual number of bytes written.
	 */
	NK_SSize
	(*onWrite)(NK_PVoid ctx, NK_PChar path, NK_SSize seek, NK_PByte data, NK_Size len);

	/**
	 * @brief
	 *  TF card read data event.
	 * @details
	 *  This event is triggered when the module needs to read data of length @ref len
	 *  from a file path on the TF card.
	 *
	 * @param ctx [in, out]
	 *  Event context.
	 * @param ctx [in, out]
	 *  Event context.
	 * @param path [in]
	 *  File path.
	 * @param skip [in]
	 *  File position offset; -1 indicates end of file.
	 * @param buf [out]
	 *  Start address of the memory for the returned data.
	 * @param len [in]
	 *  Length of the data.
	 *
	 * @return
	 *  Returns the actual number of bytes read.
	 */
	NK_SSize
	(*onRead)(NK_PVoid ctx, NK_PChar path, NK_SSize skip, NK_PByte buf, NK_Size len);

} NK_N1DeviceEventTFCard;

/**
 * @brief
 *  Configure N1 device TFCard-related events.
 * @details
 *
 *
 * @param Event [in]
 *  User event definition.
 *
 * @return
 *  Returns 0 on success, -1 on failure.
 */
NK_API NK_Int
NK_N1Device_TFCard(NK_N1DeviceEventTFCard *Event);



NK_CPP_EXTERN_END
#endif /* NK_N1_DEVICE_TFCARD_H_ */
