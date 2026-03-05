
/**
 * N1 device API interfaces.
 */

#include "n1_def.h"

#ifndef NK_N1_DEVICE_H_
#define NK_N1_DEVICE_H_
NK_CPP_EXTERN_BEGIN


#define NK_N1_DEV_EXCEP_CLASS_READ_FRAME_TIMEO  (0x10010) ///< Read frame timeout exception.


/**
 * @brief
 *  N1 device exception definition.
 */
typedef struct Nk_N1DeviceException {

	/// Exception classification.
	NK_Size classify;

	/// classify value is NK_N1_DEV_EXCEP_CLASS_READ_FRAME_TIMEO.
	struct {
		/// Channel index that timed out.
		NK_Int chid, streamid;
		/// Timeout duration in seconds.
		NK_Size seconds;

	} ReadFrameTimeout;

} NK_N1DeviceException;


/**
 * @brief N1 device module runtime context.
 */
typedef struct Nk_N1Device {

	/**
	 * Serial number of the current device.\n
	 * Passed in by the user during initialization, used as the unique identifier for this device on the LAN.\n
	 * If this value is not set, the internal module will randomly generate a string as the unique identifier for the current device.\n
	 * Version > 1.8.0, no longer used afterward.\n
	 */
	NK_Char device_id[128];

	/**
	 * Cloud device serial number.\n
	 * This serial number is the unique identity code for the device in internet communication, uniformly assigned during production and stored on the device.\n
	 * Version > 1.8.0, no longer used afterward.
	 */
	NK_Char cloud_id[32];

	/**
	 * Port used by the module to listen for network events.\n
	 * Since the module uses TCP as the network connection method, make sure this port does not conflict with other TCP-bound ports outside the module.\n
	 * It is recommended to use a port above 1024.
	 */
	NK_UInt16 port;

	/**
	 * User context.\n
	 * Used to share data between the module and the external user when events are triggered.
	 */
	NK_PVoid user_ctx;


	struct {

		/**
		 * @brief
		 *  Get device capabilities event.
		 * @details
		 *  This method is triggered during module initialization to retrieve the device capability set. When the device has not implemented this event,\n
		 *  the module internally uses default values; see @ref NK_N1DeviceCapabilities for details.
		 * @param ctx [in,out]
		 *  User event context, passed in when calling @ref NK_N1Device_Init().
		 * @param Capabilities [out]
		 *  Device capability set.
		 *
		 */
		NK_Void
		(*onCapabilities)(NK_PVoid ctx, NK_N1DeviceCapabilities *Capabilities);


		/**
		 * Live stream snapshot event.\n
		 * Triggered when the client needs to preview a live image from the camera, requesting one thumbnail from the device.\n
		 * Thumbnails only support JPEG file format.
		 *
		 * @param[in]		channel_id		Snapshot channel, starting from 0.
		 * @param[in]		width			Image width.
		 * @param[in]		height			Image height.
		 * @param[in]		pic				Image buffer.
		 * @param[in,out]	size			Buffer/image size.
		 *
		 * @retval NK_N1_ERR_NONE					Live snapshot succeeded.
		 * @retval NK_N1_ERR_DEVICE_BUSY			Device is busy; the client may retry periodically.
		 * @retval NK_N1_ERR_DEVICE_NOT_SUPPORT		Device does not support this snapshot feature; the client will stop requesting snapshots from this device.
		 *
		 */
		NK_N1Error
		(*onLiveSnapshot)(NK_Int channel_id, NK_Size width, NK_Size height, NK_PByte pic, NK_Size *size);


		/**
		 * @brief
		 *  Live stream connection event.
		 * @details
		 *  Triggered when a client connects.\n
		 *  The implementation should initialize the session @ref Session data structure based on the incoming Session::channel_id and Session::stream_id,\n
		 *  and return an appropriate value to guide the module's response according to the specific implementation.\n
		 *  @ref Session::user_session is used to preserve the user session;\n
		 *  the implementation can retain information about the live media data source here.\n
		 *
		 * @param Session [in,out]
		 *  Session context; the module shares data with the implementation through the session context.
		 * @param ctx [in,out]
		 *  User context, passed in when calling @ref NK_N1Device_Init.
		 *
		 * @retval NK_N1_ERR_NONE
		 *  Connection succeeded; the client continues to receive media data.
		 * @retval NK_N1_ERR_DEVICE_BUSY
		 *  Device is busy; return this value when the user's media source resource request fails.
		 * @retval NK_N1_ERR_DEVICE_NOT_SUPPORT
		 *  Device does not support this connection request; return this value when the channel and stream requested by the client exceed the device's supported range.
		 * @retval NK_N1_ERR_NOT_AUTHORIZATED
		 *  User authentication failed; the requesting client user does not have live streaming permission for this media.
		 *
		 */
		NK_N1Error
		(*onLiveConnected)(NK_N1LiveSession *Session, NK_PVoid ctx);

		/**
		 * @brief
		 *  Live stream disconnection event.
		 * @details
		 *  Triggered by the module when a client disconnects.\n
		 *
		 * @param Session [in,out]
		 *  Session context; the module shares data with the implementation through the session context.
		 * @param ctx [in,out]
		 *  User context, passed in when calling @ref NK_N1Device_Init.
		 *
		 * @retval NK_N1_ERR_NONE
		 *  Disconnection succeeded; the client no longer receives media data.
		 *
		 */
		NK_N1Error
		(*onLiveDisconnected)(NK_N1LiveSession *Session, NK_PVoid ctx);

		/**
		 * Live stream read event.\n
		 * Triggered when a client requests media frame data while the stream is being played.
		 *
		 * @param[in,out]	Session			Session context; the module shares data with the implementation through the session context.
		 * @param[in,out]	ctx				User context, passed in when calling @ref NK_N1Device_Init.
		 * @param[out]		payload_type	Type of the data payload read.
		 * @param[out]		ts_ms			Timestamp of the data read (unit: milliseconds).
		 * @param[out]		data_r			Memory address of the data read.
		 *
		 * @retval Greater than 0							Read succeeded; returns the length of data read.
		 * @retval Equal to 0								Read failed; no data available.
		 * @retval Less than 0								Read failed; an error occurred during reading, and the session must exit.
		 *
		 */
		NK_SSize
		(*onLiveReadFrame)(NK_N1LiveSession *Session, NK_PVoid ctx,
				NK_N1DataPayload *payload_type, NK_UInt32 *ts_ms, NK_PByte *data_r, NK_N1VideoFrameType *frametype);

		/**
		 * Live stream read completion event.\n
		 * After the @ref onLiveReadFrame() event is triggered, the library internally holds a reference to the data's memory address.\n
		 * This event is triggered after the data has been consumed, allowing the user to release the referenced data resources.
		 *
		 *
		 * @param[in,out]	Session			Session context; the module shares data with the implementation through the session context.
		 * @param[in,out]	ctx				User context, passed in when calling @ref NK_N1Device_Init.
		 * @param[in]		data_r			Memory address of the data referenced when @ref onLiveReadFrame() was triggered.
		 * @param[in]		size			Size of the data referenced when @ref onLiveReadFrame() was triggered.
		 *
		 * @retval NK_N1_ERR_NONE					Operation succeeded.
		 */
		NK_N1Error
		(*onLiveAfterReadFrame)(NK_N1LiveSession *Session, NK_PVoid ctx,
				NK_PByte *data_r, NK_Size size);

		/**
		 * @brief
		 *  Live stream attach event.
		 * @details
		 *  Triggered when the first client requests a connection to the corresponding channel stream (see @ref chid @ref streamid), notifying the user that live streaming for that stream has started.\n
		 *
		 * @param ctx [in,out]
		 *  User event context, passed in when calling @ref NK_N1Device_Init().
		 * @param chid [in]
		 *  Multimedia channel number corresponding to the stream connection, incrementing from 0 in logical order.
		 * @param streamid [in]
		 *  Stream number under the multimedia channel corresponding to the stream connection, incrementing from 0 in logical order.
		 *
		 */
		NK_Void
		(*onAttachStream)(NK_PVoid ctx, NK_Int chid, NK_Int streamid);

		/**
		 * @brief
		 *  Live stream detach event.
		 * @details
		 *  Triggered when the last client disconnects from the corresponding channel stream (see @ref chid @ref streamid), notifying the user that live streaming for that stream has stopped.\n.
		 *
		 * @param ctx [in,out]
		 *  User event context, passed in when calling @ref NK_N1Device_Init().
		 * @param chid [in]
		 *  Multimedia channel number corresponding to the stream connection, incrementing from 0 in logical order.
		 * @param streamid [in]
		 *  Stream number under the multimedia channel corresponding to the stream connection, incrementing from 0 in logical order.
		 *
		 */
		NK_Void
		(*onDetachStream)(NK_PVoid ctx, NK_Int chid, NK_Int streamid);

		/**
		 * @brief
		 *  Stream recommendation event.
		 * @details
		 *  Triggered when the module has a recommendation for adjusting the current stream's network throughput.
		 *
		 * @param ctx [in,out]
		 *  User event context, passed in when calling @ref NK_N1Device_Init().
		 * @param chid [in]
		 *  Channel ID, starting from 0.
		 * @param streamid [in]
		 *  Stream ID, starting from 0.
		 * @param kbps [in]
		 *  Recommended stream bitrate in kbps.
		 *
		 */
		NK_Void
		(*onRecommedStream)(NK_PVoid ctx, NK_Int chid, NK_Int streamid, NK_Int kbps, NK_Int fps);

		/**
		 * @brief
		 *  Listen port change event.
		 * @details
		 *  Triggered when an external communication causes the port to change. The user does not need to intervene in the port change implementation;\n
		 *  they only need to save the new port to the configuration for use when the system starts next time.
		 *
		 * @param[in,out] ctx
		 *  User context, passed in when calling @ref NK_N1Device_Init.
		 * @param[in] port
		 *  The new port number.
		 *
		 */
		NK_Void
		(*onPortChanged)(NK_PVoid ctx, NK_UInt16 port);

		/**
		 * LAN configuration event.
		 *
		 * @param[in,out]	ctx				User context, passed in when calling @ref NK_N1Device_Init.
		 * @param[in]		set_or_get		Set/get flag; when this value is True, Setup is an input parameter, otherwise it is an output parameter.
		 * @param[in,out]	Setup			Configuration data structure.
		 *
		 * @retval NK_N1_ERR_NONE					Configuration or operation succeeded.
		 * @retval NK_N1_ERR_DEVICE_NOT_SUPPORT		Device does not support this configuration or operation.
		 * @retval NK_N1_ERR_INVALID_PARAM			Invalid parameter passed to configuration or operation.
		 * @retval NK_N1_ERR_NOT_AUTHORIZATED		User authentication failed; the requesting client user does not have this configuration permission.
		 */
		NK_N1Error
		(*onLanSetup)(NK_PVoid ctx, NK_Boolean set_or_get, NK_N1LanSetup *Setup);


		/**
		 * @brief
		 *  Detect wired network link.
		 */
		NK_Boolean
		(*onDetectRJ45Connected)(NK_PVoid ctx);

		/**
		 * @brief
		 *  Get video encoder parameters.
		 * @details
		 *
		 *
		 * @param ctx [in,out]
		 *  User context, passed in when calling @ref NK_N1Device_Init.
		 * @param chid [in]
		 *  Channel ID, starting from 0.
		 * @param streamid [in]
		 *  Stream ID, starting from 0.
		 * @param Encoder [out]
		 *  Video encoder data structure.
		 *
		 * @return
		 *  Returns True on success, with specific parameters returned in @ref Encoder; returns False otherwise.
		 *
		 */
		NK_Boolean
		(*onGetVideoEncoder)(NK_PVoid ctx, NK_Int chid, NK_Int streamid, NK_N1VideoEncoder *Encoder);


		/**
		 * @brief
		 *  Set video encoder parameters.
		 * @details
		 *  See @ref onGetVideoEncoder().
		 *
		 * @return
		 *  Returns True on success, False otherwise.
		 */
		NK_Boolean
		(*onSetVideoEncoder)(NK_PVoid ctx, NK_Int chid, NK_Int streamid, NK_N1VideoEncoder *Encoder);

		/**
		 * @brief
		 *  Get IR-cut filter operating mode event.
		 * @details
		 *  Returns the result corresponding to the current IR-cut filter operating mode.
		 *
		 * @param ctx [in,out]
		 *  User context, passed in when calling @ref NK_N1Device_Init.
		 * @param chid [in]
		 *  Video input channel ID, starting from 0.
		 * @param mode [out]
		 *  Filter operating mode.
		 * @param writable [out]
		 *  If the filter cannot be configured, this variable returns the read-only flag NK_False.
		 *
		 */
		NK_Void
		(*onGetIRCutFilter)(NK_PVoid ctx, NK_Int chid, NK_N1IRCutFilterMode *mode, NK_Boolean *writable);

		/**
		 * @brief
		 *  Set IR-cut filter operating mode event.
		 * @details
		 *  Sets the operating mode according to the specified IR-cut filter mode.
		 *
		 * @param ctx [in,out]
		 *  User context, passed in when calling @ref NK_N1Device_Init.
		 * @param chid [in]
		 *  Video input channel ID, starting from 0.
		 * @param mode [in]
		 *  Filter operating mode.
		 *
		 */
		NK_Void
		(*onSetIRCutFilter)(NK_PVoid ctx, NK_Int chid, NK_N1IRCutFilterMode mode);

		/**
		 * @brief
		 *  Reserved event.
		 */
		NK_Void
		(*onGetTime)(NK_PVoid ctx);

		/**
		 * @brief
		 *  Reserved event.
		 */
		NK_Void
		(*onSetTime)(NK_PVoid ctx);

		/**
		 * @brief
		 *  Reserved event.
		 */
		NK_Void
		(*onGetEther)(NK_PVoid ctx, NK_Boolean wifi, NK_N1EthConfig *EthCfg);

		/**
		 * @brief
		 *  Reserved event.
		 */
		NK_Void
		(*onSetEther)(NK_PVoid ctx, NK_Boolean wifi, NK_N1EthConfig *EthCfg);

		/**
		 * @brief
		 *  Device exception event.
		 * @details
		 *
		 * @param ctx [in,out]
		 *  User context, passed in when calling @ref NK_N1Device_Init.
		 * @param Excp [in]
		 *  Exception data structure.
		 */
		NK_Void
		(*onCatchException)(NK_PVoid ctx, NK_N1DeviceException *Excp);

		/**
		 * @brief
		 *  Device reset event.
		 * @details
		 *  Triggered when an external entity requests the device to perform a reset.
		 *
		 * @param ctx [in,out]
		 *  User context, passed in when calling @ref NK_N1Device_Init.
		 *
		 */
		NK_Void
		(*onReset)(NK_PVoid ctx);

		/**
		 * @brief
		 *  Device reboot event.
		 * @details
		 *  Triggered when an external entity requests the device to reboot.
		 *
		 * @param ctx [in,out]
		 *  User context, passed in when calling @ref NK_N1Device_Init.
		 *
		 */
		NK_Void
		(*onReboot)(NK_PVoid ctx);

		/**
		 * @brief
		 *  Get third-party unique identifier.
		 * @details
		 *  Related to device discovery; a third party passes in the device's factory-assigned unique identifier\n
		 *  to ensure device uniqueness when discovering devices on the LAN.
		 *
		 * @param ctx [in,out]
		 *  User context, passed in when calling @ref NK_N1Device_Init.
		 * @param stack [out]
		 *  Returns the starting address of the UID memory.
		 * @param stacklen [in]
		 *  Size of the UID memory.
		 *
		 */
		NK_Void
		(*onGet3rdUID)(NK_PVoid ctx, NK_PChar stack, NK_Size stacklen);

		/**
		 * @brief
		 *  Force I-frame data acquisition interface.
		 * @details
		 *  Notifies the device to encode an I-frame for transmission.\n
		 *  Forces encoding of one I-frame.
		 *
		 * @param ctx [in,out]
		 *  User context, passed in when calling @ref NK_N1Device_Init.
		 * @param chid [in]
		 *  Channel ID, starting from 0.
		 * @param streamid [in]
		 *  Stream ID, starting from 0.
		 * @param
		 *  Returns void.
		 *
		 */
		NK_Void
		(*onMakeIFrame)(NK_PVoid ctx, NK_Int chid, NK_Int streamid);
		/**
		 * @brief
		 *  Reserved extension events.
		 */
		NK_Void
		(*reserved[8])(NK_PVoid ctx);

	} Event;

/**
 * Backward-compatible definition.
 */
#define EventSet Event

} NK_N1Device;


/**
 * @brief
 *  Get the N1 version number.
 * @brief
 *  Used to determine whether the binary library file matches the header file, avoiding potential issues caused by mismatched data structure sizes.
 *
 * @param ver_maj [out]
 *  Compared against @ref NK_N1_VER_MAJ.
 * @param ver_min [out]
 *  Compared against @ref NK_N1_VER_MIN.
 * @param ver_rev [out]
 *  Compared against @ref NK_N1_VER_REV.
 * @param ver_num [out]
 *  Compared against @ref NK_N1_VER_NUM.
 *
 * @retval
 *  Returns the version code, e.g. version 1.1.0.5 returns 01010005.
 *
 */
NK_API NK_Size
NK_N1Device_Version(NK_UInt32 *ver_maj, NK_UInt32 *ver_min, NK_UInt32 *ver_rev, NK_UInt32 *ver_num);

/**
 * @macro
 *  Version validity check.
 */
#define NK_N1_VALIDATE_VERSION() \
	((NK_N1Device_Version(NK_Nil, NK_Nil, NK_Nil, NK_Nil) == NK_N1_VER_CODE()) ? NK_True : NK_False)


/**
 * @brief
 *  Module initialization interface.
 * @details
 *  This interface must be called to initialize the module before use; internally calls @ref NK_N1Device_InitEx(license, Device, 0).
 *
 * @param license [in]
 *  User license credential.
 * @param Device [in]
 *  Device parameters.
 *
 * @return
 *  Returns 0 on success, -1 otherwise.
 *
 * @see NK_N1Device_InitV2()
 *
 */
NK_API NK_Int
NK_N1Device_Init(NK_PChar license, NK_N1Device *Device);


/**
 * @brief
 *  Extended module initialization interface.
 * @details
 *  Use this interface for module initialization. Unlike the @ref NK_N1Device_InitEx() interface,\n
 *  this interface uses the @ref memsz parameter to determine whether the module operates in media buffer mode.\n
 *  When @ref memsz is set to 0, non-buffered mode is used; the user must handle the media streaming buffer themselves.\n
 *  When @ref memsz is set to a valid size, the user sends real-time video data to the module's internal buffer via interfaces such as @ref NK_N1Device_SendH264() when media is requested,\n
 *  which greatly reduces the development complexity for the user.
 *
 * @param license [in]
 *  Path to the user license file.
 * @param memsz [in]
 *  Media buffer size. When 0, the module does not use an internal media buffer; when non-zero, specifies the total size of the internal media buffer.\n
 *  The module will allocate the corresponding heap memory internally for the media buffer; effective when greater than 1MB.
 * @param Device [in]
 *  Device parameters.
 *
 * @return
 *  Returns 0 on success, -1 otherwise.
 */
NK_API NK_Int
NK_N1Device_InitV2(NK_PChar license, NK_Size memsz, NK_N1Device *Device);

/**
 * @macro
 *  Legacy compatibility interface.
 */
#define NK_N1Device_InitEx(__license, __Device, __opt) NK_N1Device_InitV2(__license, 0, __Device)

/**
 * @macro
 *  Legacy compatibility interface.
 */
#define NK_N1Device_InitEx2(__license, __memsz, __Device, __opt) NK_N1Device_InitV2(__license, __memsz, __Device)

/**
 * Destroy the N1 device runtime environment.\n
 * @ref NK_N1Device_Init must be called before this interface for it to succeed.\n
 * When the interface returns 0 successfully, the user context passed to @ref NK_N1Device_Init can be retrieved via @ref user_ctx_r.\n
 * The caller may release resources associated with the passed-in user context as needed by the design.
 *
 * @param[out]			Device				Returns the user context passed to @ref NK_N1Device_Init at initialization.
 *
 * @return		Returns 0 on success, -1 on failure.
 */
NK_API NK_Int
NK_N1Device_Destroy(NK_N1Device *Device);


/**
 * @brief
 *  Send a notification to clients.
 *
 * @param chid [in]
 *  Media channel sequence number, starting from 0.
 *
 * @return
 *  Returns True on success, False on failure;\n
 *  may fail due to too many pending notifications or unsupported type.
 */
NK_API NK_Boolean
NK_N1Device_Notify(NK_Int chid, NK_N1Notification *Notif);

/**
 * @brief
 *  SDK uptime in nanoseconds.
 * @details
 *  Get the SDK uptime, counted from after @ref NK_N1Device_Init() is called (unit: nanoseconds).
 *
 * @return
 *  Uptime in nanoseconds.
 */
NK_API NK_Size64
NK_N1Device_UptimeNano();

#if 0

/**
 * @brief
 *  SDK uptime in microseconds.
 */
static NK_Size64
NK_N1Device_UptimeMacro() {
	return (NK_N1Device_UptimeNano() + 500) / 1000;
}


/**
 * @brief
 *  SDK uptime in milliseconds.
 */
static NK_Size64
NK_N1Device_UptimeMilli() {
	return (NK_N1Device_UptimeMacro() + 500) / 1000;
}

/**
 * @brief
 *  SDK uptime in seconds.
 */
/*static NK_Size
NK_N1Device_Uptime() {
	return (NK_N1Device_UptimeMilli() + 500) / 1000;
}*/

#endif


/**
 * @brief
 *  Get the N1 device serial number.
 *
 * @return
 *  Returns the N1 device serial number; returns an empty string (length 0) if no valid authentication serial number exists.
 */
NK_API NK_PChar
NK_N1Device_GetID();


/**
 * @brief
 *  Get the N1 device serial number.
 * @details
 *  Get the N1 device serial number along with the authorization year and month for that serial number.
 *
 * @param authyear [out]
 *  Authorization year.
 * @param authmonth [out]
 *  Authorization month.
 *
 * @return
 *  Returns the N1 device serial number; returns an empty string (length 0) if no valid authentication serial number exists.
 */
NK_API NK_PChar
NK_N1Device_GetIDV2(NK_Size *authyear, NK_Size *authmonth);



/**
 * @brief
 *  Send one H.264 frame to the buffer.
 * @details
 *  In media buffer mode, when the device receives a network media request, the @ref Live::onAttachStream event is triggered.\n
 *  The user must call this interface continuously to send real-time H.264 video encoded data (when operating as an H.264 media on-demand system).
 *
 * @param sessionid [in]
 *  Session ID; reserved parameter, always 0.
 * @param chid [in]
 *  Live channel ID, starting from 0.
 * @param streamid [in]
 *  Live stream ID, starting from 0.
 * @param ts_us [in]
 *  Relative timestamp for this frame, in microseconds (1/1000000 second).
 * @param keyFrame [in]
 *  Key frame flag; True if the current data is a key frame, False otherwise.
 * @param data [in]
 *  Starting address of the frame data in memory.
 * @param len [in]
 *  Length of the frame data.
 *
 * @return
 *  Returns True on success, False on failure.
 */
NK_API NK_Boolean
NK_N1Device_SendH264(NK_Int sessionid, NK_Int chid, NK_Int streamid, NK_UInt64 ts_us, NK_Boolean keyFrame, NK_PVoid data, NK_Size len, NK_N1VideoFrameType frametype);

/**
 * @macro
 *  Live send method, only for live video, with timestamps defined by the module.
 */
#define NK_N1Device_SendNoTsH264(__chid, ___streamid, __keyFrame, __data, __len, frametype) \
	NK_N1Device_SendH264(0, __chid, ___streamid, NK_N1Device_UptimeMacro(), __keyFrame, __data, __len, frametype);

/**
 * @brief
 *  Send one HEVC frame to the buffer.
 * @details
 *  In media buffer mode, when the module requests a media service, the @ref Live::onAttachStream event is triggered.\n
 *  The user must call this interface continuously to send real-time HEVC video encoded data (when operating as an HEVC media on-demand system).
 *
 * @param sessionid [in]
 *  Session ID; reserved parameter, always 0.
 * @param chid [in]
 *  Live channel ID, starting from 0.
 * @param streamid [in]
 *  Live stream ID, starting from 0.
 * @param ts_us [in]
 *  Relative timestamp for this frame, in microseconds (1/1000000 second).
 * @param keyFrame [in]
 *  Key frame flag; True if the current data is a key frame, False otherwise.
 * @param data [in]
 *  Starting address of the frame data in memory.
 * @param len [in]
 *  Length of the frame data.
 *
 * @return
 *  Returns True on success, False on failure.
 *
 */
NK_API NK_Boolean
NK_N1Device_SendHEVC(NK_Int sessionid, NK_Int chid, NK_Int streamid, NK_UInt64 ts_us, NK_Boolean keyFrame, NK_PVoid data, NK_Size len, NK_N1VideoFrameType frametype);


/**
 * @macro HEVC alias method definition.
 */
#define NK_N1Device_SendH265 NK_N1Device_SendHEVC

/**
 * @macro
 *  Live send method, only for live video, with timestamps defined by the module.
 */
#define NK_N1Device_SendNoTsHEVC(__chid, ___streamid, __keyFrame, __data, __len, __frametype) \
	NK_N1Device_SendHEVC(0, __chid, ___streamid, NK_N1Device_UptimeMacro(), __keyFrame, __data, __len, __frametype);
#define NK_N1Device_SendNoTsH265 NK_N1Device_SendNoTsHEVC

/**
 * @brief
 *  Send one G.711 audio packet.
 * @details
 *  In media buffer mode, when the module requests a media service, use this method to send one G.711 audio packet;\n
 *  only supports 8k - 16bits sample rate.
 *
 * @param sessionid [in]
 *  Session ID; reserved parameter, always 0.
 * @param chid [in]
 *  Live channel ID, starting from 0.
 * @param ts_us [in]
 *  Relative timestamp for this frame, in microseconds (1/1000000 second).
 * @param data [in]
 *  Starting address of the frame data in memory.
 * @param len [in]
 *  Length of the frame data.
 * @param options [in]
 *  Reserved option; always 0.
 *
 * @return
 *  Returns True on success, False on failure.
 *
 */
NK_API NK_Boolean
NK_N1Device_SendG711(NK_Int sessionid, NK_Int chid, NK_UInt64 ts_us, NK_PVoid data, NK_Size len, NK_UInt32 options);

/**
 * @macro
 *  Live send method, only for live video, with timestamps defined by the module.
 */
#define NK_N1Device_SendNoTsG711(__chid, __data, __len, __options) \
	NK_N1Device_SendG711(0, __chid, NK_N1Device_UptimeMacro(), __data, __len, __options);


NK_CPP_EXTERN_END
#endif /* NK_N1_DEVICE_H_ */
