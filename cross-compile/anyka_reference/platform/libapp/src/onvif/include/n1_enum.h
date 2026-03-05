/**
 * N1 enumeration data definitions.
 *
 */

#include <NkUtils/types.h>
#include <NkUtils/enum.h>

#ifndef NK_N1_ENUM_H_
#define NK_N1_ENUM_H_
NK_CPP_EXTERN_BEGIN

/**
 * Image resolution.
 */
typedef enum Nk_N1ImageSize
{
#define NK_N1_IMG_SZ_VAL(__width, __height) (((__width) * 10000) + (__height))

	NK_N1_IMG_SZ_UNDEF = (-1),
	NK_N1_IMG_SZ_160X90 = NK_N1_IMG_SZ_VAL(160, 90),
	NK_N1_IMG_SZ_160X120 = NK_N1_IMG_SZ_VAL(160, 120),
	NK_N1_IMG_SZ_172X144 = NK_N1_IMG_SZ_VAL(172, 144),
	NK_N1_IMG_SZ_320X180 = NK_N1_IMG_SZ_VAL(320, 180),
	NK_N1_IMG_SZ_320X240 = NK_N1_IMG_SZ_VAL(320, 240),
	NK_N1_IMG_SZ_352X240 = NK_N1_IMG_SZ_VAL(352, 240),
	NK_N1_IMG_SZ_352X288 = NK_N1_IMG_SZ_VAL(352, 288),
	NK_N1_IMG_SZ_360X240 = NK_N1_IMG_SZ_VAL(360, 240),
	NK_N1_IMG_SZ_360X288 = NK_N1_IMG_SZ_VAL(360, 288),
	NK_N1_IMG_SZ_528X384 = NK_N1_IMG_SZ_VAL(528, 384),
	NK_N1_IMG_SZ_640X360 = NK_N1_IMG_SZ_VAL(640, 360),
	NK_N1_IMG_SZ_640X480 = NK_N1_IMG_SZ_VAL(640, 480),
	NK_N1_IMG_SZ_704X240 = NK_N1_IMG_SZ_VAL(704, 240),
	NK_N1_IMG_SZ_704X288 = NK_N1_IMG_SZ_VAL(704, 288),
	NK_N1_IMG_SZ_704X480 = NK_N1_IMG_SZ_VAL(704, 480),
	NK_N1_IMG_SZ_704X576 = NK_N1_IMG_SZ_VAL(704, 576),
	NK_N1_IMG_SZ_720X240 = NK_N1_IMG_SZ_VAL(720, 240),
	NK_N1_IMG_SZ_720X288 = NK_N1_IMG_SZ_VAL(720, 288),
	NK_N1_IMG_SZ_720X480 = NK_N1_IMG_SZ_VAL(720, 480),
	NK_N1_IMG_SZ_720X576 = NK_N1_IMG_SZ_VAL(720, 576),
	NK_N1_IMG_SZ_800X600 = NK_N1_IMG_SZ_VAL(800, 600),
	NK_N1_IMG_SZ_960X480 = NK_N1_IMG_SZ_VAL(960, 480),
	NK_N1_IMG_SZ_960X576 = NK_N1_IMG_SZ_VAL(960, 576),
	NK_N1_IMG_SZ_1280X720 = NK_N1_IMG_SZ_VAL(1280, 720),
	NK_N1_IMG_SZ_1280X960 = NK_N1_IMG_SZ_VAL(1280, 960),
	NK_N1_IMG_SZ_1600X900 = NK_N1_IMG_SZ_VAL(1600, 900),
	NK_N1_IMG_SZ_1600X1200 = NK_N1_IMG_SZ_VAL(1600, 1200),
	NK_N1_IMG_SZ_1920X1080 = NK_N1_IMG_SZ_VAL(1920, 1080),
	NK_N1_IMG_SZ_2048X1520 = NK_N1_IMG_SZ_VAL(2048, 1520),
	NK_N1_IMG_SZ_2592X1520 = NK_N1_IMG_SZ_VAL(2592, 1520),
	NK_N1_IMG_SZ_2592X1944 = NK_N1_IMG_SZ_VAL(2592, 1944),

} NK_N1ImageSize;

/**
 * Get the text representation corresponding to the Nk_N1ImageSize enum value.
 */
extern DECLARE_NK_ENUM_MAP(N1ImageSize);

/**
 * Get the enum value corresponding to the Nk_N1ImageSize text representation.
 */
extern DECLARE_NK_ENUM_UNMAP(N1ImageSize);


/**
 * Encoding bitrate control mode.
 */
typedef enum Nk_N1BitRateCtrlMode
{
	NK_N1_BR_CTRL_MODE_UNDEF = (-1),
	/**
	 * Constant bitrate control.
	 */
	NK_N1_BR_CTRL_MODE_CBR,

	/**
	 * Variable bitrate control.
	 */
	NK_N1_BR_CTRL_MODE_VBR,

} NK_N1BitRateCtrlMode;

/**
 * Get the text representation corresponding to the NK_N1BitRateCtrlMode enum value.
 */
extern DECLARE_NK_ENUM_MAP(N1BitRateCtrlMode);

/**
 * Get the enum value corresponding to the NK_N1BitRateCtrlMode text representation.
 */
extern DECLARE_NK_ENUM_UNMAP(N1BitRateCtrlMode);


typedef enum Nk_N1VideoEncCodec
{
	NK_N1_VENC_CODEC_UNDEF = (-1),
	NK_N1_VENC_CODEC_MPEG,
	NK_N1_VENC_CODEC_H264,
	NK_N1_VENC_CODEC_HEVC,
	NK_N1_VENC_CODEC_H264_PLUS,
	NK_N1_VENC_CODEC_HEVC_PLUS,

} NK_N1VideoEncCodec;

/**
 * Get the text representation corresponding to the Nk_N1VideoEncCodec enum value.
 */
extern DECLARE_NK_ENUM_MAP(N1VideoEncCodec);

/**
 * Get the enum value corresponding to the Nk_N1VideoEncCodec text representation.
 */
extern DECLARE_NK_ENUM_UNMAP(N1VideoEncCodec);


typedef enum Nk_N1PTZCommand {

	NK_N1_PTZ_CMD_UNDEF			= (-1),

	NK_N1_PTZ_CMD_STOP			= (0),

	NK_N1_PTZ_CMD_TILT_UP			= (100),
	NK_N1_PTZ_CMD_TILT_DOWN,

	NK_N1_PTZ_CMD_PAN_LEFT		= (200),
	NK_N1_PTZ_CMD_PAN_RIGHT,
	NK_N1_PTZ_CMD_PAN_AUTO,

	NK_N1_PTZ_CMD_ZOOM_IN			= (300),
	NK_N1_PTZ_CMD_ZOOM_OUT,

	NK_N1_PTZ_CMD_FOCUS_IN		= (400),
	NK_N1_PTZ_CMD_FOCUS_OUT,

	NK_N1_PTZ_CMD_IRIS_OPEN		= (500),
	NK_N1_PTZ_CMD_IRIS_CLOSE,

	NK_N1_PTZ_CMD_SET_PRESET		= (1000),
	NK_N1_PTZ_CMD_GOTO_PRESET,
	NK_N1_PTZ_CMD_CLEAR_PRESET,

} NK_N1PTZCommand;

/**
 * Get the text representation corresponding to the NK_N1PTZCommand enum value.
 */
extern DECLARE_NK_ENUM_MAP(N1PTZCommand);

/**
 * Get the enum value corresponding to the NK_N1PTZCommand text representation.
 */
extern DECLARE_NK_ENUM_UNMAP(N1PTZCommand);


/**
 * IRCut filter working mode.
 */
typedef enum Nk_N1IRCutFilterMode
{
	NK_N1_IRCUT_MODE_UNDEF = (-1),
	NK_N1_IRCUT_MODE_AUTO,
	NK_N1_IRCUT_MODE_DAYLIGHT,
	NK_N1_IRCUT_MODE_NIGHT,

} NK_N1IRCutFilterMode;


/**
 * Get the text representation corresponding to the NK_N1IRCutFilterMode enum value.
 */
extern DECLARE_NK_ENUM_MAP(N1IRCutFilterMode);

/**
 * Get the enum value corresponding to the NK_N1IRCutFilterMode text representation.
 */
extern DECLARE_NK_ENUM_UNMAP(N1IRCutFilterMode);


NK_CPP_EXTERN_END
#endif /* NK_N1_ENUM_H_ */
