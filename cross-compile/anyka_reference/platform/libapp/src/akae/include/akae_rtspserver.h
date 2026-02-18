/**
 * RTSP server function interface definitions.
 *
 */

#include <akae_rtsp.h>
#include <akae_httputils.h>
#include <akae_verbose.h>
#include <akae_tosser.h>
#include <akae_malloc.h>
#include <akae_rtpqueue.h>


#if !defined(AKAE_RTSPSERVER_H_)
# define AKAE_RTSPSERVER_H_
AK_C_HEADER_EXTERN_C_BEGIN


/**
 * Maximum URL addresses for the RTSP server.
 */
#define AK_RTSP_SERVER_MAX_URL       (16)

/**
 * Minimum receive media buffer packet count for RTSP.
 */
#define AK_RTSP_SERVER_MIN_BUFCNT    (8)

/**
 * Default receive media buffer packet count for RTSP.
 */
#define AK_RTSP_SERVER_DEF_BUFCNT    (64)

/**
 * Maximum receive media buffer packet count for RTSP.
 */
#define AK_RTSP_SERVER_MAX_BUFCNT    (128)


/**
 * Create an RTSP server.
 * Note: after successful creation, the server does not yet begin listening.\n
 * It only starts working after @ref akae_rtsp_server_start() is called.
 *
 * @param[IN] Malloc
 *  Memory allocator used internally by the object.
 *
 * @return
 *  Returns the object handle on success.
 *
 */
AK_API AK_Object akae_rtsp_server_create (AK_Object Malloc);

/**
 * Release an RTSP server.
 *
 * @param[IN] Object
 *  Server object created by @ref akae_rtsp_server_create().
 *
 * @retval AK_OK
 *  Released successfully.
 * @retval AK_ERR_INVAL_OBJECT
 *  Operation failed; possibly an invalid parameter was passed.
 *
 */
AK_API AK_int akae_rtsp_server_release (AK_Object Object);

/**
 * Enable verbose debug protocol message output.
 */
AK_API AK_void akae_rtsp_server_verbose (AK_Object Object, AK_boolean enabled);


AK_API AK_void akae_rtsp_server_verbose_http (AK_Object Object, AK_boolean enabled);


/**
 * Start the RTSP server; after this call succeeds, external clients can begin accessing it.
 *
 * @param[IN] Object
 *  Server object obtained from akae_rtsp_server_create().
 * @param[IN] port
 *  TCP port number for the server to listen on.
 *
 * @retval AK_OK
 *  Operation succeeded.
 */
AK_API AK_int akae_rtsp_server_start (AK_Object Object, AK_uint16 port);

/**
 * Stop the RTSP server; the reverse of @ref akae_rtsp_server_start().
 *
 * @retval AK_OK
 *  Operation succeeded.
 */
AK_API AK_int akae_rtsp_server_finish (AK_Object Object);


/**
 * RTSP send/receive service user callback interface definition.
 * This interface is defined and implemented by the caller and handles stream data
 * processing within the RTSP service session.
 *
 * @param[IN] Rtsp
 *  RTSP session handle, delivered by the module; passed to interfaces like
 *  @ref akae_rtsp_server_send_h264_1slice() as the session send handle.
 *
 * @param[IN] userdata
 *  Session user data, passed in via @ref akae_rtsp_server_add_url().
 *
 * @param[IN] th
 *  Session thread handle; within the callback implementation, use
 *  @ref akae_thread_terminated(th) to determine whether the current session
 *  should continue sending.
 *
 */
typedef AK_void (*AK_RtspRecvSendRtpFunc)(AK_voidptr Rtsp, AK_voidptr userdata, AK_Thread th);



/**
 * Register a URL listener address. On success, returns the registry handle
 * for subsequent operations on this session.
 *
 * @param[IN] Object
 *  Server object created by @ref akae_rtsp_server_create().
 *
 * @param[IN] url
 *  Listener address; after a successful operation, this address begins accepting requests.
 *
 * @param[IN] Func
 *  RTSP RTP data sending user thread. When a data request arrives, this user callback is triggered.\n
 *  User context can be passed through @ref userdata inside the callback.
 *
 * @param[IN] userdata
 *  User context pointer; this data is passed to the user when @ref Func is triggered.
 *
 * @return
 *  Returns a registry value greater than 0 on success for interface operations;
 *  returns the corresponding error code on failure.
 *
 * @retval AK_ERR_INVAL_OBJECT
 *  Operation failed; an invalid object was passed.
 * @retval AK_ERR_INVAL_PARAM
 *  Operation failed; possibly an invalid parameter was passed.
 *
 */
AK_API AK_int akae_rtsp_server_register_url (AK_Object Object, AK_chrptr url, AK_RtspRecvSendRtpFunc Func, AK_voidptr userdata);


/**
 * Set the receive buffer count.\n
 * The @var bufcnt parameter limits the number of buffered media packets.\n
 * The maximum and minimum values are defined by @ref AK_RTSP_SERVER_MAX_BUFCNT and
 * @ref AK_RTSP_SERVER_MIN_BUFCNT respectively.\n
 * If this interface is not called after @ref akae_rtsp_server_add_url(),
 * the internal default is @ref AK_RTSP_SERVER_MIN_BUFCNT.
 *
 */
AK_API AK_int akae_rtsp_server_set_url_receiver(AK_Object Object, const AK_int registry, AK_size bufcnt);

/**
 * Set the URL re-entrant property to avoid stream buffer conflicts caused by concurrent access.
 */
AK_API AK_int akae_rtsp_server_set_url_reentrant(AK_Object Object, const AK_int registry, AK_boolean enabled);


/**
 * Unregister a URL listener address; the reverse of @ref akae_rtsp_server_add_url().
 *
 * @param[IN] Object
 *  Server object created by @ref akae_rtsp_server_create().
 *
 * @param[IN] registry
 *  Listener address registry number created by @ref akae_rtsp_server_add_url().
 *
 * @retval AK_OK
 *  Operation succeeded.
 * @retval AK_ERR_INVAL_OBJECT
 *  Operation failed; an invalid object was passed.
 * @retval AK_ERR_INVAL_PARAM
 *  Operation failed; possibly an invalid parameter was passed.
 */
AK_API AK_int akae_rtsp_server_unregister_url(AK_Object Object, const AK_int registry);

/**
 * Declare a video service payload type.
 *
 * @param[IN] Object
 *  Server object created by @ref akae_rtsp_server_create().
 * @param[IN] registry
 *  Listener address registry number created by @ref akae_rtsp_server_add_url().
 * @param[IN] payload
 *  Video payload type, e.g. @ref AK_RTP_PT_H264.
 * @param[IN] clock_hz
 *  Video clock base; for video this is the capture clock frequency, for audio this is the sample rate.
 *
 * @retval AK_OK
 *  Operation succeeded.
 * @retval AK_ERR_INVAL_OBJECT
 *  Operation failed; an invalid object was passed.
 */
AK_API AK_int akae_rtsp_server_describe_video(AK_Object Object, const AK_int registry, AK_uint32 payload,
		AK_size clock_hz, AK_Object RtpQueue);


/**
 * Declare an upstream (reverse) video service payload type.
 *
 */
AK_API AK_int akae_rtsp_server_describe_upstream_video (AK_Object Object, AK_int const registry, AK_uint32 payload, AK_size clock_hz);


/**
 * Quick calls for specific media types based on @ref akae_rtsp_server_describe_video().
 */
#define akae_rtsp_server_describe_h264(__Object, __registry, __RtpQueue)     akae_rtsp_server_describe_video(__Object, __registry, AK_RTP_PT_H264, 90000, __RtpQueue)
#define akae_rtsp_server_describe_h265(__Object, __registry, __RtpQueue)     akae_rtsp_server_describe_video(__Object, __registry, AK_RTP_PT_H265, 90000, __RtpQueue)
#define akae_rtsp_server_describe_jpeg(__Object, __registry, __RtpQueue)     akae_rtsp_server_describe_video(__Object, __registry, AK_RTP_PT_JPEG, 90000, __RtpQueue)

/**
 * Declare a service audio payload type.
 *
 */
AK_API AK_int akae_rtsp_server_describe_audio(AK_Object Object, const AK_int registry, AK_uint32 payload,
		AK_size channel, AK_size sample_rate, AK_Object RtpQueue);


#define akae_rtsp_server_describe_g711a(__Object, __registry, __RtpQueue)    akae_rtsp_server_describe_audio(__Object, __registry, AK_RTP_PT_PCMA, 1, 8000, __RtpQueue)
#define akae_rtsp_server_describe_g711u(__Object, __registry, __RtpQueue)    akae_rtsp_server_describe_audio(__Object, __registry, AK_RTP_PT_PCMU, 1, 8000, __RtpQueue)
#define akae_rtsp_server_describe_aac(__Object, __registry, __RtpQueue)      akae_rtsp_server_describe_audio(__Object, __registry, AK_RTP_PT_AAC, 1, 8000, __RtpQueue)



/**
 * Declare an upstream audio service payload type.
 *
 */
AK_API AK_int akae_rtsp_server_describe_upstream_audio(AK_Object Object, const AK_int registry, AK_uint32 payload,
			AK_size channel, AK_size sample_rate);

#define akae_rtsp_server_describe_upstream_g711a(__Object, __registry) akae_rtsp_server_describe_upstream_audio(__Object, __registry, AK_RTP_PT_PCMA, 1, 8000)
#define akae_rtsp_server_describe_upstream_g711u(__Object, __registry) akae_rtsp_server_describe_upstream_audio(__Object, __registry, AK_RTP_PT_PCMU, 1, 8000)
#define akae_rtsp_server_describe_upstream_aac(__Object, __registry)   akae_rtsp_server_describe_upstream_audio(__Object, __registry, AK_RTP_PT_AAC, 1, 16000)


/**
 * RTSP server event type definitions.
 */
enum {

	/// Undefined event.
	AK_RTSP_EVT_UNDEF = (-1),

	/// RTSP connect event; triggered when a client connects to the corresponding URL.
	AK_RTSP_EVT_URL_CONNECT,

	/// RTSP disconnect event; triggered when a client disconnects from the corresponding URL.
	AK_RTSP_EVT_URL_DISCONNECT,

	/// URL active event; distinct from @ref AK_RTSP_EVT_URL_CONNECTED,\n
	/// triggered when the first @ref AK_RTSP_EVT_URL_CONNECTED event fires.
	AK_RTSP_EVT_URL_ACTIVE,

	/// URL deactive event; distinct from @ref AK_RTSP_EVT_URL_DISCONNECTED,\n
	/// triggered when the last @ref AK_RTSP_EVT_URL_DISCONNECTED event fires,\n
	/// i.e. when the server's URL service transitions from active to inactive.
	AK_RTSP_EVT_URL_DEACTIVE,

	/// Enum count marker.
	AK_RTSP_EVT_BUTT,

};


/**
 * RTSP server event parameter definition.
 */
typedef struct _AK_RtspServerEventPara {

	///
	/// URL path.
	/// Valid for events:
	/// @ref AK_RTSP_EVT_URL_CONNECTED
	/// @ref AK_RTSP_EVT_URL_DISCONNECTED
	/// @ref AK_RTSP_EVT_URL_ACTIVE
	/// @ref AK_RTSP_EVT_URL_DEACTIVE
	///
	AK_chrptr path;

	///
	/// URL connection count.
	/// Valid for events:
	/// @ref AK_RTSP_EVT_URL_CONNECTED
	/// @ref AK_RTSP_EVT_URL_DISCONNECTED
	///
	AK_int connected;

} AK_RtspServerEventPara;


/**
 * RTSP server client request/de-request event callback definition.
 */
typedef AK_void (*AK_RtspServerOnEvent)(AK_voidptr Rtsp, AK_int evt, AK_voidptr userdata, AK_RtspServerEventPara *Para);


/**
 * Register an event listener callback, e.g. for @ref AK_RTSP_EVT_ACTIVE_URL.
 * The server triggers this user callback when the event fires; the user can use
 * it to record events based on upper-layer business needs.\n
 * Calling this interface again will overwrite the previously registered callback.
 *
 * @param[IN] Rtsp
 *  The registered RTSP session handle.
 * @param[IN] evt
 *  Event type, e.g. @ref AK_RTSP_EVT_ACTIVE_URL.
 * @param[IN] Func
 *  User callback implementation; passing NULL is equivalent to calling
 *  @ref akae_rtsp_server_drop_event_listener().\n
 *  AK_RtspServerOnEvent implementation.
 * @param[IN] userdata
 *  Event trigger user context; passed back to the client via @ref EventFunc.
 *
 */
AK_API AK_int akae_rtsp_server_add_event_listener (AK_Object Object, AK_int const registry, AK_int evt, AK_RtspServerOnEvent Func, AK_voidptr userdata);

/**
 * Remove an event listener.
 *
 * @param[IN] Rtsp
 *  The registered RTSP session handle.
 * @param[IN] evt
 *  Event type.
 */
AK_API AK_int akae_rtsp_server_drop_event_listener (AK_Object Object, AK_int const registry, AK_int evt);


/**
 * Retrieve stream media data.\n
 * This interface may only be called within the @ref AK_RtspRecvSendRtpFunc callback.\n
 * This interface must be called quickly enough to avoid data loss due to insufficient internal memory.\n
 * The buffer size can be increased slightly via @ref akae_rtsp_server_set_url_receiver(),
 * up to a maximum of @ref AK_RTSP_SERVER_MAX_BUFCNT.
 *
 * @verbatim
 *
 * akae_rtsp_server_get_stream() and akae_rtsp_server_release_stream() workflow.
 *
 *                                                                     +-----------------+
 *   Client Send                                             +---------> User Process    |
 *   Media-1                                                 |         | for Application |
 *                                                           |         +-----------------+
 *      +            Media Data Pipe         Server          |                  |
 *      |      +--------------------------+  Receive  +------+-------+          | After Processing
 *      +----->+  |  |  |  |  |  |  |  |  |  Media-*  |              |          |
 *             |  |  |  |  |  |  |  |  |  +-----------> get_stream() |  Loop    |
 *      +----->+  |  |  |  |  |  |  |  |  |           |              |          |
 *      |      +--------------------------+           +------+-------+          |
 *      +                                                    |                  |
 *                                                           |         +--------v---------+
 *   Client Send                                             |         | release_stream() |
 *   Media-2                                                 +---------+                  |
 *                                                                     +------------------+
 *
 * @endverbatim
 *
 * @param[IN] Rtsp
 *  RTSP session context, passed in via the @ref AK_RtspRecvSendRtpFunc callback.
 * @param[OUT] Rtp
 *  RTP data information for the retrieved data.
 * @param[OUT] data
 *  Start address of the memory containing the retrieved data.
 *
 * @return
 *  On success, the data attribute parameters and start address are returned via @ref Rtp and @ref data,
 *  and the data length is returned as the return value.
 *  Returns the corresponding error code on failure.
 *
 * @retval AK_ERR_INVAL_OBJECT
 *  Operation error; possibly an invalid object was passed.
 * @retval AK_ERR_INVAL_PARAM
 *  Operation error; possibly invalid parameters were passed.
 * @retval AK_ERR_NOT_EXIST
 *  Operation error; no data in the buffer.
 * @retval AK_ERR_INVAL_OPERATE
 *  Operation error; @ref akae_rtsp_server_release_stream() was not called in the previous cycle to release data.
 */
AK_API AK_ssize akae_rtsp_server_get_stream (AK_voidptr Rtsp, AK_RtpHeadfield *Rtp, AK_bytptr *data);

/**
 * Release stream media data.\n
 * This interface may only be called within the @ref AK_RtspRecvSendRtpFunc callback.\n
 * Corresponds to @ref akae_rtsp_server_get_stream().\n
 * If @ref akae_rtsp_server_get_stream() has been called but this release interface
 * has not been called yet, calling @ref akae_rtsp_server_get_stream() again to
 * obtain new data will fail.
 *
 */
AK_API AK_int akae_rtsp_server_release_stream (AK_voidptr Rtsp);


AK_C_HEADER_EXTERN_C_END
#endif ///< AKAE_RTSPSERVER_H_
