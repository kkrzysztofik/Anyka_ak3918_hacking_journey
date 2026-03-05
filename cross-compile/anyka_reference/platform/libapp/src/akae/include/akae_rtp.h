/**
 * RFC document location: https://tools.ietf.org/html/rfc6184
 *
 */


#include <akae_typedef.h>
#include <akae_socket.h>

#if !defined (AKAE_RTP_H_)
#define AKAE_RTP_H_
AK_C_HEADER_EXTERN_C_BEGIN


/**
 * MTU size of a single RTP packet.
 */
#define AK_RTP_MTU_SZ          (1258)

/**
 * RTP payload data types.
 */
#define AK_RTP_PT_UNDEF        (-1)
#define AK_RTP_PT_PCMU         (0)
#define AK_RTP_PT_RESERVED1    (1)
#define AK_RTP_PT_RESERVED2    (2)
#define AK_RTP_PT_GSM          (3)
#define AK_RTP_PT_G723         (4)
#define AK_RTP_PT_DVI4_8000    (5)
#define AK_RTP_PT_DVI4_16000   (6)
#define AK_RTP_PT_LPC          (7)
#define AK_RTP_PT_PCMA         (8)
#define AK_RTP_PT_G722         (9)
#define AK_RTP_PT_L161         (10) ///< 44.1khz ,2 channels  //10
#define AK_RTP_PT_L162         (11) ///< 44.1khz, 1 channel
#define AK_RTP_PT_QCELP        (12)
#define AK_RTP_PT_CN           (13)
#define AK_RTP_PT_MPA          (14)
#define AK_RTP_PT_G728         (15)
#define AK_RTP_PT_DVI4_11025   (16)
#define AK_RTP_PT_DVI4_22050   (17)
#define AK_RTP_PT_G729         (18)
#define AK_RTP_PT_CEIB         (25)
#define AK_RTP_PT_JPEG         (26)
#define AK_RTP_PT_H261         (31)
#define AK_RTP_PT_MPV          (32)
#define AK_RTP_PT_MP2T         (33)
#define AK_RTP_PT_H263         (34)
#define AK_RTP_PT_H264         (96)
#define AK_RTP_PT_H265         (96 + 0x10000)  ///< Distinguish to H.264
#define AK_RTP_PT_AAC          (97)

#define AK_RTP_PT_HEVC         (AK_RTP_PT_H265)



/**
 * Get the payload name by payload type number.
 *
 * @return
 *  Returns the payload name on success, or AK_null on failure.
 */
AK_API AK_chrptr akae_rtp_payload_name (AK_int payload_type);


/**
 * Get the payload type number by payload name.
 *
 * @return
 *  Returns the payload type number on success, or @ref AK_RTP_PT_UNDEFINE on failure.
 */
AK_API AK_int akae_rtp_payload_type (AK_chrptr name);



#define AK_RTP_PSLICE          (1)
#define AK_RTP_ISLICE          (5)
#define AK_RTP_SEI             (6)
#define AK_RTP_SPS             (7)
#define AK_RTP_PPS             (8)

#define AK_RTP_STAP_A          (24) //single-time aggregation packet
#define AK_RTP_STAP_B          (25)
#define AK_RTP_MTAP16          (26) //multi-time aggregation packet
#define AK_RTP_MTAP24          (27)
#define	AK_RTP_FU_A            (28) //fragmentation unit
#define AK_RTP_FU_B            (29) //fragmentation unit

#define AK_RTP_H265_TAIL_N         (1)
#define AK_RTP_H265_TAIL_R         (2)
#define AK_RTP_H265_IDR_W_RADL     (19)
#define AK_RTP_H265_IDR_N_LP       (20)
#define AK_RTP_H265_VPS_NUT        (32)
#define AK_RTP_H265_SPS_NUT	       (33)
#define AK_RTP_H265_PPS_NUT	       (34)
#define AK_RTP_H265_AUD_NUT	       (35)
#define AK_RTP_H265_EOS_NUT	       (36)
#define AK_RTP_H265_EOB_NUT	       (37)
#define AK_RTP_H265_FD_NUT         (38)
#define AK_RTP_H265_PREFIX_SEI_NUT (39)
#define AK_RTP_H265_SUFFIX_SEI_NUT (40)
#define AK_RTP_H265_NUT_END        (47)

#define AK_RTP_H265_AP		48
#define AK_RTP_H265_FU		49




#pragma pack(push, 1)
typedef struct _AK_RtpHeadfield {

// 0                   1                   2                   3
// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |V=2|P|X| CC|M|     PT      |       sequence number             |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                           timestamp                           |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |           synchronization source (SSRC) identifier            |
// +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
// |            contributing source (CSRC) identifiers             |
// |                             ....                              |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    AK_byte      csrc_cnt     : 4;
    AK_byte      extension    : 1;
    AK_byte      padding      : 1;
    AK_byte      version      : 2;
    AK_byte      payload_type : 7;
    AK_byte      marker       : 1;
    AK_uint16    seqno;
    AK_uint32    timestamp;
    AK_uint32    ssrc;

    /// CSRC is ignored here.

} AK_RtpHeadfield;

typedef struct _AK_RtpOverRtspInterleaved {

	AK_char   dollar;
	AK_uint8  channel;
	AK_size16 length;

} AK_RtpOverRtspInterleaved;


typedef union _AK_NalUnitH264 {

	struct {

		AK_byte type               : 5;
		AK_byte nal_ref_idc        : 2;
		AK_byte forbidden_zero     : 1; // must be 0
	};

	AK_byte byte;

} AK_NalUnitH264;

/**
 * FU indicator
 */
#define AK_RtpFragUnitH264 AK_NalUnitH264
#define AK_RtpFragUnitH265 AK_NalUnitH265

/**
 * FU header
 */
typedef union _AK_RtpFragUnitHeader {

	struct {
		AK_byte type:6;
		AK_byte stop_bit:1;
		AK_byte start_bit:1;
	};

	AK_byte bytes;

} AK_RtpFragUnitHeader;

typedef union _AK_NalUnitH265 {

	struct {
		AK_uint16 tid : 3;
		AK_uint16 layer_id : 6;
		AK_uint16 type : 6;
		AK_uint16 forbidden_zero_bit : 1; // must be 0
	};

	AK_uint16 bytes;

} AK_NalUnitH265;


typedef struct _AK_RtpJpegHeader {

        AK_uint32  tspec : 8;       ///< type-specific field
        AK_uint32  off   : 24;      ///< fragment byte offset
        AK_uint8   type;            ///< id of jpeg decoder params
        AK_uint8   q;               ///< quantization factor (or table id)
        AK_uint8   width;           ///< frame width in 8 pixel blocks
        AK_uint8   height;          ///< frame height in 8 pixel blocks

} AK_RtpJpegHeader;


typedef struct _AK_RtpJpegHeaderQTable {

        AK_uint8  mbz;
        AK_uint8  precision;
        AK_uint16 length;

} AK_RtpJpegHeaderQTable;


//typedef union _AK_FUForH265 {
//
//	struct {
//		AK_char type:6;
//		AK_char stop_bit:1;
//		AK_char start_bit:1;
//	};
//
//	AK_char padding;
//
//} AK_FUForH265;


#pragma pack(pop)

/**
 * RTP session definition.\n
 * A new RTP session is created for each RTSP SETUP request.
 */
typedef struct _AK_RtpSession {

	/// Memory allocator.
	AK_Object Malloc;

	/// Connected socket handle.
	AK_Socket sock;

	/// Interleaved channel number; valid when using TCP transport, set to -1 for UDP transport.
	AK_int interleaved;

	/// Media sampling clock rate.
	AK_size clock_hz;

	/// RTP SSRC, created at initialization time; read-only.
	AK_uint32 ssrc;

	/// Sequence number, initialized to 0.
	AK_size seqno;

	/// Base timestamp (unit: milliseconds).
	AK_uint32 baseTimestamp;

	/// Latest timestamp (unit: milliseconds).
	AK_uint32 lastestTimestamp;

	/// Packet count.
	AK_uint32 packetNumber, octetBytes;

	/// Media stream related data object definition.
	/// The object body includes buffer pipes and temporary variables for media streams.
	/// When media is received, it is first buffered into the pipe inside this data structure.
	/// Video slice pipe: for H.264 and H.265, frame boundaries cannot be determined until the marker bit is received.
	/// Therefore H.264 and H.265 data must first be buffered in the slice pipe,
	/// and only read out to the media pipe once a complete frame has been received.
	struct {

		AK_size size; ///< Cumulative received size.
		AK_Object Buffer; ///< Secondary slice buffer for H.264 and H.265.

	} RecvSlice;

} AK_RtpSession;


/**
 * Initialize RTP session parameters.
 *
 * @param[IN] sock
 *  Socket handle; the socket passed in must be a valid connected socket with a reachable peer
 *  (i.e., connect() has been called successfully), otherwise the call returns failure.
 * @param[IN[ interleaved
 *  For TCP connections that use interleaved transport.
 * @param[OUT] Session
 *  RTP session context; returned through this variable upon successful initialization.
 *
 * @return
 */
AK_API AK_int akae_rtp_session_init (AK_Object Malloc, AK_uint32 ssrc, AK_Socket sock, AK_int interleaved, AK_size clock_hz, AK_RtpSession *Session);

/**
 * Destroy an RTP session; paired with @ref akae_rtp_session_init.
 *
 */
AK_API AK_void akae_rtp_session_destroy (AK_RtpSession *Session);


/**
 * @brief
 *  Send one H.264 slice.
 * @details
 *  When calling this interface, no H.264 slice type detection is performed internally.\n
 *  The data at the memory location pointed to by @ref slice is directly RTP-packetized and sent.\n
 *  The caller must ensure that @ref slice contains a complete H.264 slice (decodable).\n
 *  Using this interface can further improve send performance, suitable for encoders with slice-level encoding support.
 *
 * @param[IN] Session
 *  Session handle, initialized via @ref akae_rtp_session_init().
 * @param[IN] time_ms
 *  Data timestamp (unit: milliseconds).
 * @param[IN] data
 *  Start address in memory of the H.264 slice data.
 * @param[IN] datalen
 *  Length of the H.264 slice data.
 *
 * @return
 *  Returns AK_True on success, AK_False on failure.
 */
AK_API AK_boolean akae_rtp_session_send_h264_1slice (AK_RtpSession *Session, AK_uint32 time_ms, AK_bytptr data, AK_size datalen);


/**
 * @brief
 *  Send one H.265 slice.
 */
AK_API AK_boolean akae_rtp_session_send_h265_1slice (AK_RtpSession *Session, AK_uint32 time_ms, AK_bytptr data, AK_size datalen);

AK_API AK_boolean akae_rtp_session_send_audio (AK_RtpSession *Session, AK_int payload_type, AK_uint32 time_ms, AK_bytptr data, AK_size datalen);

/**
 * Receive one RTP packet, process it, and buffer it.
 * After successful reception, the data is buffered into the internal pipe within @var RtpSession.
 *
 * @param[IN] RtpSession
 *  The current RTP receive session.
 *
 * @param[IN] rtp_len
 *  Length of the RTP packet to receive.
 *
 * @return
 *  Returns AK_true on success, AK_false on failure.
 */
AK_API AK_boolean akae_rtp_session_recv_and_buffer (AK_RtpSession *RtpSession, AK_size rtp_len, AK_Object Buffer);




AK_C_HEADER_EXTERN_C_END
#endif /* AKAE_RTP_H_ */
