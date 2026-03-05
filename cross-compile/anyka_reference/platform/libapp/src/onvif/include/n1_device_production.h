
/**
 * N1 device production docking-related API interfaces.
 */

#include "n1_device.h"

#if !defined(NK_N1_DEVICE_PRODUCTION_H_)
#define NK_N1_DEVICE_PRODUCTION_H_
NK_CPP_EXTERN_BEGIN


/**
 * @brief
 *  N1 audio prompt language definitions.
 */
typedef enum Nk_N1SoundPrompt {

	NK_N1_SOUND_PROMPT_NA = (0),
	NK_N1_SOUND_PROMPT_ENGLISH = (1000),
	NK_N1_SOUND_PROMPT_CHINESE_MAINLAND = (2000),

} NK_N1SoundPrompt;


/**
 * @brief
 *  N1 device production docking event definitions.
 */
typedef struct Nk_N1DeviceEventProduction {


	/**
	 * @brief
	 *  Detect IO input state.
	 * @details
	 *  For input interfaces (such as buttons, external alarm inputs, etc.), returns
	 *  the IO state via the return value for testing purposes.\n
	 *  IO name table.\n
	 *  +--------------+--------------+--------------+--------------------------------------------+\n
	 *  | ioName       | id           | Get / Put    | Description                                |\n
	 *  +--------------+--------------+--------------+--------------------------------------------+\n
	 *  | RstKey       | 0            | Get          | Reset button: True = pressed, False = released. |\n
	 *  +--------------+--------------+--------------+--------------------------------------------+\n
	 *
	 * @param ctx [in,out]
	 *  User context, passed in when calling the @ref NK_N1Device_InitEx() interface.
	 * @param name [in]
	 *  IO name; this name must be agreed upon between the user and the module.
	 * @param id [in]
	 *  IO port index number.
	 *
	 * @return
	 *  For button-type IO interfaces, True means button pressed, False means released.\n
	 *  For level-type IO interfaces, True means high level, False means low level.
	 *
	 */
	NK_Boolean
	(*onGetIO)(NK_PVoid ctx, const NK_PChar name, NK_Int id);


	/**
	 * @brief
	 *  Set IO output state.
	 * @details
	 *  See @ref onGetIO() event.
	 *
	 * @param ctx [in,out]
	 *  User context, passed in when calling the @ref NK_N1Device_InitEx() interface.
	 * @param name [in]
	 *  IO name; this name must be agreed upon between the user and the module.
	 * @param id [in]
	 *  IO port index number.
	 * @param stat [in]
	 *  IO output state, depending on the specific type.
	 *
	 */
	NK_Void
	(*onPutIO)(NK_PVoid ctx, const NK_PChar name, NK_Int id, NK_Boolean stat);

	/**
	 * @brief
	 *  Write the factory device unique code.\n
	 *  Used for factory initialization configuration during production.
	 *
	 * @param[in,out] ctx
	 *  User context, passed in when calling the @ref NK_N1Device_InitEx() interface.
	 * @param[in] id
	 *  Device unique code string.
	 * @param[in] len
	 *  Length of the device unique code string.
	 *
	 *
	 */
	NK_Void
	(*onWriteUID)(NK_PVoid ctx, const NK_PChar uid, NK_Size len);

	/**
	 * @brief
	 *  Get the factory device unique code written by the @ref onWriteUID() event.\n
	 * @details
	 *  This event is triggered internally when the @ref NK_N1Device_GetID() interface
	 *  is called.\n
	 *  Therefore, @ref NK_N1Device_GetID() must not be called inside the event
	 *  implementation.
	 *
	 * @param ctx [in,out]
	 *  User context, passed in when calling the @ref NK_N1Device_InitEx() interface.
	 * @param stack [out]
	 *  Start address of the memory for the returned device unique code string.
	 * @param stacklen [in]
	 *  Length of the stack buffer passed in.
	 *
	 */
	NK_Void
	(*onReadUID)(NK_PVoid ctx, NK_PChar stack, NK_Size stacklen);


	/**
	 * @brief
	 *  Get EEPROM chip data.
	 * @details
	 *  This event is triggered internally when the @ref NK_N1Device_GetID() interface
	 *  is called.\n
	 *  Therefore, @ref NK_N1Device_GetID() must not be called inside the event
	 *  implementation.
	 *
	 * @param ctx [in,out]
	 *  User context, passed in when calling the @ref NK_N1Device_InitEx() interface.
	 * @param datasz [in]
	 *  Required EEPROM data size; currently only 256 is supported.
	 * @param data [out]
	 *  EEPROM data content; data length must match @ref datasz.
	 * @param mask0 [out]
	 *  Data mask, reserved.
	 * @param mask1 [out]
	 *  Data mask, reserved.
	 *
	 */
	NK_Void
	(*onE2PROM)(NK_PVoid ctx, NK_Size datasz, NK_PByte data, NK_UInt32 *mask0, NK_UInt32 *mask1);


	/**
	 * @brief
	 *  Get device audio prompt event.
	 * @brief
	 *  Gets the current device audio prompt language from factory settings.
	 *
	 * @param ctx [in,out]
	 *  User context, passed in when calling the @ref NK_N1Device_InitEx() interface.
	 * @param lang [out]
	 *  Current audio prompt language.
	 */
	NK_Void
	(*onGetSoundPrompt)(NK_PVoid ctx, NK_N1SoundPrompt *lang);


	/**
	 * @brief
	 *  Set device audio prompt event.
	 * @brief
	 *  See @ref onGetPrompt.
	 */
	NK_Void
	(*onSetSoundPrompt)(NK_PVoid ctx, NK_N1SoundPrompt lang);


} NK_N1DeviceEventProduction;

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
NK_N1Device_Production(NK_N1DeviceEventProduction *Event);



NK_CPP_EXTERN_END
#endif /* NK_N1_DEVICE_PRODUCTION_H_ */
