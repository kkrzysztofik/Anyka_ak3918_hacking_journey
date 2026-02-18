/**
 * RTP queue module for handling data distribution in RTSP services.
 * Internally uses the akae_qworm module.
 *
 */

#include <akae_malloc.h>
#include <akae_rtp.h>
#include <akae_aac.h>


#if !defined(AKAE_RTSPQUEUE_H_)
# define AKAE_RTSPQUEUE_H_
AK_C_HEADER_EXTERN_C_BEGIN

/**
 * RTP queue reader handle.
 */
#define AK_RtpQueueReader AK_handle


typedef struct _AK_RtpQueueStatus {

	/// Total bytes written to the queue.
	AK_uint32 bytes;

	/// Write throughput of the queue, in Bytes/s.
	AK_uint32 byterate;

	/// Queue memory usage.
	AK_uint32 memory;

} AK_RtpQueueStatus;


/**
 * Create an RTSP queue.
 *
 * @param[IN] payload_type
 *  Payload data type.
 */
AK_API AK_Object akae_rtp_queue_create (AK_Object Malloc, AK_int payload_type);

/**
 * Release an RTSP queue.
 *
 */
AK_API AK_int akae_rtp_queue_release (AK_Object Object);


/**
 * Get the payload type configured in the buffer, as passed to @ref akae_rtp_queue_create().
 */
AK_API AK_int akae_rtp_queue_payload_type (AK_Object Object);



/**
 * Shorthand for @ref akae_rtp_queue_release().
 */
#define akae_rtp_queue_release_clear(__Object) \
	({\
		AK_int const res = akae_rtp_queue_release (__Object);\
		if (AK_OK == res) {\
			__Object = AK_null;\
		}\
		res;\
	})

/**
 * Get the SPS packet data from the H.264 buffer queue.
 *
 * @param[OUT] stack
 *  Start address of the memory for receiving the SPS data.
 *
 * @param[IN] stacklen
 *  Maximum available memory length for receiving the SPS data.
 *
 * @return
 *  Returns the SPS data length on success; SPS data is stored at the memory address pointed to
 *  by @ref stack. Returns the corresponding error code on failure.\n
 *  Note: if no SPS data has been buffered by the time this interface is called, it returns 0.
 *
 * @retval AK_ERR_NOT_SUPPORT
 *  Failed; this queue does not support retrieving H.264 SPS data.
 * @retval AK_ERR_OUT_OF_MEM
 *  Failed; the @ref stacklen provided is insufficient to return the SPS data.
 *
 */
AK_API AK_ssize akae_rtp_queue_get_h264_sps (AK_Object Object, AK_bytptr stack, AK_size stacklen);

/**
 * Get the PPS packet data from the H.264 buffer queue.
 * See @ref akae_rtp_queue_get_h264_sps().
 *
 */
AK_API AK_ssize akae_rtp_queue_get_h264_pps (AK_Object Object, AK_bytptr stack, AK_size stacklen);

/**
 * Get the SPS and PPS packet data from the H.265 buffer queue.
 */
#define akae_rtp_queue_get_h265_sps akae_rtp_queue_get_h264_sps
#define akae_rtp_queue_get_h265_pps akae_rtp_queue_get_h264_pps


/**
 * Get the VPS packet data from the H.265 buffer queue.
 * See @ref akae_rtp_queue_get_h264_sps().
 *
 */
AK_API AK_ssize akae_rtp_queue_get_h265_vps (AK_Object Object, AK_bytptr stack, AK_size stacklen);

/**
 * Get the ADTS data from the AAC buffer queue.
 *
 */
AK_API AK_ssize akae_rtp_queue_get_aac_adts (AK_Object Object, AK_AacAdtsFixedHeader *Header, AK_bytptr stack, AK_size stacklen);


/**
 * Get the current queue status.
 */
AK_API AK_int akae_rtp_queue_status (AK_Object Object, AK_RtpQueueStatus *Status);


/**
 * Append payload data to the RTP queue.
 */
AK_API AK_int akae_rtp_queue_add_payload (AK_Object Object, AK_uint32 time_ms, AK_bytptr data, AK_size datalen);

/**
 * Acquire a queue reader.
 *
 * @return
 *  Returns a reader handle; returns AK_INVAL_HANDLE on failure.
 */
AK_API AK_RtpQueueReader akae_rtp_queue_get_reader (AK_Object Object);


/**
 * Return a reader handle to the platform after use.
 */
AK_API AK_int akae_rtp_queue_put_reader (AK_Object Object, AK_RtpQueueReader reader);


/**
 * Get a chunked RTP packet from the queue.
 *
 * @verbatim
 *
 *  The @ref data returned from this interface points to a contiguous memory region.
 *  The region begins with an RTP header field structure, immediately followed by
 *  the RTP payload data. The total length of this contiguous memory is returned
 *  as the return value on success.
 *
 *  +----------------+ <-- @ref data
 *  | RTP Headfield  |
 *  |                |
 *  +----------------+
 *  |                |
 *  |    RTP Data    |
 *  |                |
 *  +----+-----------+
 *
 * @endverbatim
 *
 */
AK_API AK_ssize akae_rtp_queue_get_rtp (AK_RtpQueueReader reader, AK_boolean peek, AK_uint32 *time_ms, AK_bytptr *data);




AK_C_HEADER_EXTERN_C_END
#endif ///< AKAE_RTSPQUEUE_H_
