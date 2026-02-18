/**
 * RTSP protocol basic toolkit definitions.
 *
 */

#include <akae_rtsp.h>
#include <akae_malloc.h>
#include <akae_verbose.h>


#if !defined(AKAE_RTSPCLIENT_H_)
# define AKAE_RTSPCLIENT_H_
AK_C_HEADER_EXTERN_C_BEGIN

typedef struct _AK_RtspClientStatus {

	/// Connection flag; returns AK_true when currently in a connected state.
	AK_boolean connected;

	/// Server media carrier types.
	struct {

		/// Server download media carrier types.
		/// This media is downloaded by the client to be played locally.
		/// The Up prefix indicates media uploaded by the client to the server for interaction.
		struct {

			/// Type number.
			AK_int type;

			/// Type name.
			AK_char name[8];

		} Video, Audio, UpVideo, UpAudio;

	} Payload;

	/// Memory usage information.
	struct {
		/// Total memory.
		AK_size total;
		/// Memory usage.
		AK_size usage;

	} Memory;

	/// Reconnect information.
	struct {

		/// Total number of reconnections.
		AK_size total;
		/// Successful reconnections.
		AK_size success;

	} Reconnect;

	/// Number of received data packets.
	struct {

		struct {

			AK_size video;
			AK_size audio;

		} Rtp;

		struct {

			AK_size rr;

		} Rtcp;

	} RecvPacket;


} AK_RtspClientStatus;

/**
 * Print the @ref AK_RtspClientStatus object structure.
 */
#define AK_RTSP_CLIENT_STATUS_DUMP(__Status) \
	do {\
		AK_VerboseForm Form;\
		akae_verbose_form_init (&Form, "RTSP Clietn Status", 64, 4);\
		akae_verbose_form_put_kv (&Form, AK_true,     "Connected",       "%s", (__Status)->connected ? "Yes" : "No");\
		akae_verbose_form_put_kv (&Form, AK_true,     "Reconnect",       "%u/%u", (__Status)->Reconnect.success, (__Status)->Reconnect.total);\
		akae_verbose_form_put_text (&Form, AK_true,   "Payload Type");\
		akae_verbose_form_put_kv (&Form, AK_false,    "Video",       "%2d - %s", (__Status)->Payload.Video.type, (__Status)->Payload.Video.name);\
		akae_verbose_form_put_kv (&Form, AK_false,    "Audio",       "%2d - %s", (__Status)->Payload.Audio.type, (__Status)->Payload.Audio.name);\
		akae_verbose_form_put_kv (&Form, AK_false,    "Video Upstream",       "%2d - %s", (__Status)->Payload.UpVideo.type, (__Status)->Payload.UpVideo.name);\
		akae_verbose_form_put_kv (&Form, AK_true,     "Audio Upstream",       "%2d - %s", (__Status)->Payload.UpAudio.type, (__Status)->Payload.UpAudio.name);\
		akae_verbose_form_put_kv (&Form, AK_true,     "Memory",          "%u.%03u/%u.%03uKB - %d%%",\
				(__Status)->Memory.usage / 1000, (__Status)->Memory.usage % 1000,\
				(__Status)->Memory.total / 1000, (__Status)->Memory.total % 1000,\
				(__Status)->Memory.usage * 100 / (__Status)->Memory.total);\
		akae_verbose_form_put_kv (&Form, AK_false,    "Received RTP - Video",     "%u", (__Status)->RecvPacket.Rtp.video);\
		akae_verbose_form_put_kv (&Form, AK_false,    "Received RTP - Audio",     "%u", (__Status)->RecvPacket.Rtp.audio);\
		akae_verbose_form_finish (&Form);\
	} while (0)


/**
 * Create an RTSP client object.\n
 * A memory allocator must be passed in at creation time; this allocator is used for buffering
 * when the client receives data.
 *
 * @post The created object must be released by calling @ref akae_rtsp_client_release().
 *
 * @param[IN] Malloc
 *  Internal buffer allocator.
 * @param[IN] mem
 *  Specifies the memory size to use; passing 0 means use as much of @ref Malloc's memory as possible.\n
 *  If the @ref Malloc memory allocator is provided exclusively for this module by design, passing 0 is sufficient.
 *
 * @return
 *  Returns the RTSP client object handle on success; returns AK_null on failure.
 */
AK_API AK_Object akae_rtsp_client_create (AK_Object Malloc, AK_size mem);

/**
 * Release an RTSP client object.
 *
 * @pre The object to be released was created by @ref akae_rtsp_client_create().
 *
 * @param[IN] Object
 *  Module object, created by @ref akae_rtsp_client_create().
 *
 * @retval AK_OK
 *  Operation successful.
 * @retval AK_ERR_INVAL_OBJECT
 *  Operation failed, an invalid object was passed.
 */
AK_API AK_int akae_rtsp_client_release (AK_Object Object);

/**
 * Enable display of debug protocol packet output.
 */
AK_API AK_void akae_rtsp_client_verbose (AK_Object Object, AK_boolean enabled);


/**
 * Connect to the @var url server address.\n
 * The first call to this interface initiates a connection to the @var url server address;\n
 * after a successful connection, data can be retrieved via @ref akae_rtsp_client_get_stream().\n
 * After this interface is called successfully, if the connection is interrupted internally,
 * it will continuously attempt to reconnect to ensure reliable connectivity.\n
 * If this interface is called twice consecutively, the previous connection is first disconnected,
 * and a new connection is made to the new @var url server address;\n
 * the result of the new address connection is returned via the interface.
 *
 * @post
 *  Call after successful creation of the object by @ref akae_rtsp_client_create().
 *
 * @param[IN] Object
 *  Module object, created by @ref akae_rtsp_client_create().
 * @param[IN] url
 *  Server address (peer address), e.g., rtsp://192.168.1.100:8554/stream.
 * @param[IN] timeo_ms
 *  Connection timeout; pass -1 for maximum timeout, 0 for minimum timeout,
 *  otherwise the timeout value in milliseconds.
 *
 * @retval AK_OK
 *  Operation successful.
 * @retval AK_ERR_INVAL_OBJECT
 *  Operation failed, an invalid object was passed.
 * @retval AK_ERR_NOT_ACCESS
 *  Operation succeeded but address connection failed.
 *
 */
AK_API AK_int akae_rtsp_client_connect (AK_Object Object, AK_chrptr url, AK_ssize timeo_ms);


/**
 * Disconnect.
 */
AK_API AK_int akae_rtsp_client_disconnect (AK_Object Object);

/**
 * Get streaming media data.\n
 * This interface must be called frequently enough to avoid data loss caused by insufficient
 * internal module memory.
 * If the upper-level business risks blocking this interface's data retrieval, slightly increase
 * the available memory of the allocator passed to @ref akae_rtsp_client_create().
 *
 * @param[OUT] Rtp
 * @param[OUT] data
 * @param[IN] timeo
 *  Wait timeout in microseconds; less than 0 means wait indefinitely, 0 means async return,
 *  greater than 0 means the wait duration.
 *
 * @return
 *  On success, data attributes and starting address are returned via @ref Rtp and @ref data,
 *  and the data length is returned;\n
 *  if there is no data in the buffer by the @ref timeo timeout, returns 0;\n
 *  returns the corresponding error code on failure.
 *
 * @retval AK_ERR_INVAL_OBJECT
 *  Operation error, possibly an invalid object was passed.
 * @retval AK_ERR_INVAL_PARAM
 *  Operation error, possibly an invalid parameter was passed.
 * @retval AK_ERR_INVAL_OPERATE
 *  Operation error, @ref akae_rtsp_client_release_stream() was not called in the previous cycle
 *  to release the used data.
 */
AK_API AK_ssize akae_rtsp_client_wait_stream (AK_Object Object, AK_RtpHeadfield *Rtp, AK_bytptr *data, AK_size timeo);

/**
 * Quick call for @ref akae_rtsp_client_wait_stream().
 */
#define akae_rtsp_client_get_stream(__Object, __Rtp, __data) akae_rtsp_client_wait_stream(__Object, __Rtp, __data, 0)


/**
 * Release streaming media data.
 * Corresponds to @ref akae_rtsp_client_wait_stream().
 *
 */
AK_API AK_int akae_rtsp_client_release_stream (AK_Object Object);

/**
 * Get the current client status information.
 */
AK_API AK_int akae_rtsp_client_status (AK_Object Object, AK_RtspClientStatus *Status);


/**
 *
 * @param Object
 * @param time_ms
 * @param data
 * @param datalen
 *
 * @retval AK_ERR_INVAL_OBJECT
 *  Operation failed, possibly an invalid object was passed.
 * @retval AK_ERR_INVAL_PARAM
 *  Operation failed, possibly an invalid parameter was passed.
 * @retval AK_ERR_NOT_ACCESS
 *  Operation failed, the current server connection does not support reverse audio transmission.
 * @retval AK_ERR_PEER_BROKEN
 *  Operation failed, connection disconnected.
 *
 */
AK_API AK_int akae_rtsp_client_send_audio (AK_Object Object, AK_size time_ms, AK_voidptr data, AK_size datalen);


AK_C_HEADER_EXTERN_C_END
#endif ///< AKAE_RTSPCLIENT_H_
