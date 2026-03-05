


#include <akae_rtp.h>
#include <akae_verbose.h>

#if !defined(AKAE_RTCP_H_)
#define AKAE_RTCP_H_
AK_C_HEADER_EXTERN_C_BEGIN

/**
 * RTCP version number.
 * RTC 1889 Version (2)
 */
#define AK_RTCP_VERSION		    (2)

/**
 * Convert RTCP packet absolute length; each RTCP packet length is aligned to 4 bytes.
 */
#define AK_RTCP_PACK_LEN_LONG(__rlen) (((__rlen) + 1) * 4)

/**
 * @macro
 *  Convert RTCP packet relative length.
 */
#define AK_RTCP_PACK_LEN_SHORT(__abslen) (((__abslen) / 4) - 1)



//rtcp packet types
/**
 * SR:   Sender report, for transmission and reception statistics from
 *       participants that are active senders
 *
 * RR:   Receiver report, for reception statistics from participants
 *       that are not active senders and in combination with SR for
 *       active senders reporting on more than 31 sources
 *
 * SDES: Source description items, including CNAME
 *
 * BYE:  Indicates end of participation
 *
 * APP:  Application-specific functions
 *
 */
#define AK_RTCP_PT_FIR      (192) ///< full intra-frame request
#define AK_RTCP_PT_NACK     (193) ///< negative acknowledgement
#define AK_RTCP_PT_SR       (200) ///< sender report
#define AK_RTCP_PT_RR       (201) ///< Receiver Report
#define AK_RTCP_PT_SDES     (202) ///< source description
#define AK_RTCP_PT_BYE      (203) ///< indeicate end of participation
#define AK_RTCP_PT_APP      (204) ///< application specific functions
#define AK_RTCP_PT_XR       (207) ///< rtcp extension

/**
 * RTCP packet SDES type definitions.
 */
#define AK_RTCP_SDES_CNAME      (1)
#define AK_RTCP_SDES_NAME       (2)
#define AK_RTCP_SDES_EMAIL      (3)
#define AK_RTCP_SDES_PHONE      (4)
#define AK_RTCP_SDES_LOC        (5)
#define AK_RTCP_SDES_TOOL       (6)
#define AK_RTCP_SDES_NOTE       (7)
#define AK_RTCP_SDES_PRIV       (8)

#pragma pack(push, 1)

/**
 * RTCP header field definition.
 */
typedef struct _AK_RtcpPacket {

	struct {

		AK_uint8    count: 5;      ///< count of reports
		AK_uint8    padding: 1;       ///< padding flag
		AK_uint8    version: 2; ///< protocol version, always @ref AK_RTCP_VERSION.
		AK_uint8    pt;         ///< RTCP packet type
		AK_uint16   length;     ///< pkt len in words, w/o this word

	} Headfield;

	union {

		struct {

			AK_uint32 ssrc;

			struct {

				AK_uint32 ntpTimestampMSB;
				AK_uint32 ntpTimestampLSB;
				AK_uint32 rtpTimestamp;
				AK_uint32 senderPacketCount;
				AK_uint32 senderOctetCount;

			} Info;

		} SR;

		struct {

			AK_uint32 ssrc;

			struct {

				/**
				 * @brief
				 *  SSRC_n (source identifier): 32 bits.
				 *
				 * @details
				 *  The SSRC identifier of the source to which the information in this \n
				 *  reception report block pertains. \n
				 *
				 */
				AK_uint32 identifier;

				/**
				 * @brief
				 *  fraction lost: 8 bits.
				 *
				 * @details
				 *  The fraction of RTP data packets from source SSRC_n lost since   \n
				 *  previous SR or RR packet was sent, expressed as a fixed point \n
				 *  number with the binary point at the left edge of the field.  (That \n
				 *  is equivalent to taking the integer part after multiplying the \n
				 *  loss fraction by 256.)  This fraction is defined to be the number \n
				 *  of packets lost divided by the number of packets expected, as \n
				 *  defined in the next paragraph.  An implementation is shown in \n
				 *  Appendix A.3.  If the loss is negative due to duplicates, the \n
				 *  fraction lost is set to zero.  Note that a receiver cannot tell \n
				 *  whether any packets were lost after the last one received, and \n
				 *  that there will be no reception report block issued for a source \n
				 *  if all packets from that source sent during the last reporting \n
				 *  interval have been lost. \n
				 *
				 */
				AK_uint32 fractionLost : 8;

				/**
				 * @brief
				 *  cumulative number of packets lost: 24 bits.
				 *
				 * @details
				 *  The total number of RTP data packets from source SSRC_n that have \n
				 *  been lost since the beginning of reception.  This number is \n
				 *  defined to be the number of packets expected less the number of \n
				 *  packets actually received, where the number of packets received \n
				 *  includes any which are late or duplicates.  Thus, packets that \n
				 *  arrive late are not counted as lost, and the loss may be negative \n
				 *  if there are duplicates.  The number of packets expected is \n
				 *  defined to be the extended last sequence number received, as \n
				 *  defined next, less the initial sequence number received.  This may \n
				 *  be calculated as shown in Appendix A.3. \n
				 *
				 */
				AK_int32 cumulativeNumberOfPacketsLost : 24;

				/**
				 * @brief
				 *  extended highest sequence number received: 32 bits.
				 *
				 * @details
				 *  The low 16 bits contain the highest sequence number received in an \n
				 *  RTP data packet from source SSRC_n, and the most significant 16 \n
				 *  bits extend that sequence number with the corresponding count of \n
				 *  sequence number cycles, which may be maintained according to the \n
				 *  algorithm in Appendix A.1.  Note that different receivers within \n
				 *  the same session will generate different extensions to the \n
				 *  sequence number if their start times differ significantly. \n
				 *
				 */
				AK_uint32 extendedHighestSequenceNumberReceived;

				/**
				 * @brief
				 *  interarrival jitter: 32 bits.
				 *
				 * @details
				 *  An estimate of the statistical variance of the RTP data packet \n
				 *  interarrival time, measured in timestamp units and expressed as an \n
				 *  unsigned integer.  The interarrival jitter J is defined to be the \n
				 *  mean deviation (smoothed absolute value) of the difference D in \n
				 *  packet spacing at the receiver compared to the sender for a pair \n
				 *  of packets.  As shown in the equation below, this is equivalent to \n
				 *  the difference in the "relative transit time" for the two packets; \n
				 *  the relative transit time is the difference between a packet's RTP \n
				 *  timestamp and the receiver's clock at the time of arrival, \n
				 *  measured in the same units. \n
				 *  \n
				 *  If Si is the RTP timestamp from packet i, and Ri is the time of \n
				 *  arrival in RTP timestamp units for packet i, then for two packets \n
				 *  i and j, D may be expressed as\n
				 *  \n
				 *     D(i,j) = (Rj - Ri) - (Sj - Si) = (Rj - Sj) - (Ri - Si) \n
				 *  \n
				 *  The interarrival jitter SHOULD be calculated continuously as each \n
				 *  data packet i is received from source SSRC_n, using this \n
				 *  difference D for that packet and the previous packet i-1 in order \n
				 *  of arrival (not necessarily in sequence), according to the formula \n
				 *  \n
				 *     J(i) = J(i-1) + (|D(i-1,i)| - J(i-1))/16 \n
				 *  \n
				 *  Whenever a reception report is issued, the current value of J is \n
				 *  sampled. \n
				 *  \n
				 *  The jitter calculation MUST conform to the formula specified here \n
				 *  in order to allow profile-independent monitors to make valid \n
				 *  interpretations of reports coming from different implementations. \n
				 *  This algorithm is the optimal first-order estimator and the gain \n
				 *  parameter 1/16 gives a good noise reduction ratio while \n
				 *  maintaining a reasonable rate of convergence [22].  A sample \n
				 *  implementation is shown in Appendix A.8.  See Section 6.4.4 for a \n
				 *  discussion of the effects of varying packet duration and delay \n
				 *  before transmission. \n
				 *
				 */
				AK_uint32 interarrivalJitter;

				/**
				 * @brief
				 *  last SR timestamp (LSR): 32 bits.
				 *
				 * @details
				 *  The middle 32 bits out of 64 in the NTP timestamp (as explained in \n
				 *  Section 4) received as part of the most recent RTCP sender report \n
				 *  (SR) packet from source SSRC_n.  If no SR has been received yet, \n
				 *  the field is set to zero. \n
				 *
				 */
				AK_uint32 lastSR;

				/**
				 * @brief
				 *  delay since last SR (DLSR): 32 bits.
				 *
				 * @details
				 *  The delay, expressed in units of 1/65536 seconds, between \n
				 *  receiving the last SR packet from source SSRC_n and sending this \n
				 *  reception report block.  If no SR packet has been received yet \n
				 *  from SSRC_n, the DLSR field is set to zero. \n
				 *  \n
				 *  Let SSRC_r denote the receiver issuing this receiver report. \n
				 *  Source SSRC_n can compute the round-trip propagation delay to \n
				 *  SSRC_r by recording the time A when this reception report block is \n
				 *  received.  It calculates the total round-trip time A-LSR using the \n
				 *  last SR timestamp (LSR) field, and then subtracting this field to \n
				 *  leave the round-trip propagation delay as (A - LSR - DLSR).  This \n
				 *  is illustrated in Fig. 2.  Times are shown in both a hexadecimal \n
				 *  representation of the 32-bit fields and the equivalent floating-point \n
				 *  decimal representation.  Colons indicate a 32-bit field \n
				 *  divided into a 16-bit integer part and 16-bit fraction part. \n
				 *  \n
				 *  This may be used as an approximate measure of distance to cluster \n
				 *  receivers, although some links have very asymmetric delays. \n
				 *
				 */
				AK_uint32  delaySinceLastSR;

			} Source[1];

		} RR;

		/**
		 * SDES data packet.
		 */
		struct {

			struct {

				AK_uint32 identifier;

			} Chunk[1];

		} SDES;

		struct {

			AK_int reserved;

		} BYE;


		/**
		 * Reserved memory space.
		 */
		AK_byte padding[1024 * 4];

	};

	/**
	 * Pointer to the last position in the packet.
	 */
	AK_bytptr bottom;


} AK_RtcpPacket;


/**
  * Print the @ref AK_RtcpPacket object structure.
 */
#define AK_RTCP_PACKET_DUMP(__Packet) \
	do {\
		AK_int __i = 0;\
		AK_VerboseForm Form;\
		akae_verbose_form_init (&Form, "RTCP Packet", 96, 4);\
		akae_verbose_form_put_kv (&Form, AK_false,     "Count",      "%d",  (AK_int)(__Packet)->Headfield.count);\
		akae_verbose_form_put_kv (&Form, AK_false,     "Version",    "%d",  (AK_int)(__Packet)->Headfield.version);\
		if (AK_RTCP_PT_FIR == (__Packet)->Headfield.pt) {\
		} else if (AK_RTCP_PT_FIR  == (__Packet)->Headfield.pt) {\
			akae_verbose_form_put_kv (&Form, AK_false,     "Payload",    "Full Intra-Frame");\
		} else if (AK_RTCP_PT_NACK == (__Packet)->Headfield.pt) {\
			akae_verbose_form_put_kv (&Form, AK_false,     "Payload",    "Negative ACK");\
		} else if (AK_RTCP_PT_SR   == (__Packet)->Headfield.pt) {\
			akae_verbose_form_put_kv (&Form, AK_false,     "Payload",    "Sender Report");\
		} else if (AK_RTCP_PT_RR   == (__Packet)->Headfield.pt) {\
			akae_verbose_form_put_kv (&Form, AK_false,     "Payload",    "Receiver Report");\
		} else if (AK_RTCP_PT_SDES == (__Packet)->Headfield.pt) {\
			akae_verbose_form_put_kv (&Form, AK_false,     "Payload",    "Source Description");\
		} else if (AK_RTCP_PT_BYE  == (__Packet)->Headfield.pt) {\
			akae_verbose_form_put_kv (&Form, AK_false,     "Payload",    "Bye");\
		} else if (AK_RTCP_PT_APP  == (__Packet)->Headfield.pt) {\
			akae_verbose_form_put_kv (&Form, AK_false,     "Payload",    "Application Specific Functions");\
		} else if (AK_RTCP_PT_XR   == (__Packet)->Headfield.pt) {\
			akae_verbose_form_put_kv (&Form, AK_false,     "Payload",    "Extension");\
		}\
		akae_verbose_form_put_kv (&Form, AK_true,     "Length",     "%d/%d",\
				(AK_int)((__Packet)->Headfield.length),\
				(AK_int)(AK_RTCP_PACK_LEN_LONG ((__Packet)->Headfield.length)));\
		if (AK_RTCP_PT_FIR == (__Packet)->Headfield.pt) {\
		} else if (AK_RTCP_PT_FIR  == (__Packet)->Headfield.pt) {\
			akae_verbose_form_put_text (&Form, AK_false,     "FIR");\
		} else if (AK_RTCP_PT_NACK == (__Packet)->Headfield.pt) {\
			akae_verbose_form_put_text (&Form, AK_false,     "NACK");\
		} else if (AK_RTCP_PT_SR   == (__Packet)->Headfield.pt) {\
			akae_verbose_form_put_text (&Form, AK_false,     "SR");\
		} else if (AK_RTCP_PT_RR   == (__Packet)->Headfield.pt) {\
			akae_verbose_form_put_kv (&Form, AK_true,    "SSRC", "%08x", (__Packet)->RR.ssrc);\
			if ((__Packet)->Headfield.count > 0) {\
				for (__i = 0; __i < (__Packet)->Headfield.count; ++__i) {\
					__typeof__ (Packet->RR.Source[0]) *const __RR = &(__Packet)->RR.Source[__i];\
					akae_verbose_form_put_kv (&Form, AK_false, "SSRC",    "%08x", (AK_uint32)(__RR->identifier));\
					akae_verbose_form_put_kv (&Form, AK_false, "Fraction Lost",    "%d", (AK_int)(__RR->fractionLost));\
					akae_verbose_form_put_kv (&Form, AK_false, "Cumulative Number of Packets Lost",    "%d", (AK_int)(__RR->cumulativeNumberOfPacketsLost));\
					akae_verbose_form_put_kv (&Form, AK_false, "Extended Highest Sequence Number Received",    "%d", (AK_int)(__RR->extendedHighestSequenceNumberReceived));\
					akae_verbose_form_put_kv (&Form, AK_false, "Interarrival Jitter",    "%d", (AK_int)(__RR->interarrivalJitter));\
					akae_verbose_form_put_kv (&Form, AK_false, "Last SR",    "%d", (AK_int)(__RR->lastSR));\
					akae_verbose_form_put_kv (&Form, AK_false, "Delay Since Last SR",    "%d", (AK_int)(__RR->delaySinceLastSR));\
				}\
			}\
		} else if (AK_RTCP_PT_SDES == (__Packet)->Headfield.pt) {\
			akae_verbose_form_put_text (&Form, AK_false,     "SDES");\
		} else if (AK_RTCP_PT_BYE  == (__Packet)->Headfield.pt) {\
			akae_verbose_form_put_text (&Form, AK_false,     "BYE");\
		} else if (AK_RTCP_PT_APP  == (__Packet)->Headfield.pt) {\
			akae_verbose_form_put_text (&Form, AK_false,     "APP");\
		} else if (AK_RTCP_PT_XR   == (__Packet)->Headfield.pt) {\
			akae_verbose_form_put_text (&Form, AK_false,     "XR");\
		}\
		akae_verbose_form_finish (&Form);\
	} while (0)




/**
 * RTCP Over RTSP packet header.
 */
#define AK_RtcpOverRtspInterleaved AK_RtpOverRtspInterleaved



typedef struct _AK_RtcpOverRtspPacket {

	AK_RtcpOverRtspInterleaved Interleaved;
	AK_RtcpPacket Rtcp;

} AK_RtcpOverRtspPacket;


/**
 * RTP session definition.\n
 * Each RTSP SETUP operation produces a corresponding RTCP session.
 */
typedef struct _AK_RtcpSession {

	/// TCP transport handle, passed in when initializing with @ref AK_RTCP_InitOverTCP().
	AK_Socket sock;

	/// Interleaved channel number, valid when using TCP transport.
	AK_int interleaved;

//	/// Associated RTP session handle.
	AK_RtpSession *RtpSession;

	/// Number of packets sent.
	AK_size packetSent;

} AK_RtcpSession;

#pragma pack(pop)

/**
 * Initialize a TCP-based RTCP session.
 *
 * @param[IN] sock
 *  Socket used by the RTCP session.
 * @param[IN] interleaved
 *  RTCP over RTSP channel number.
 * @param[IN] RtpSession
 *  Real-time statistics of the RTP session associated with the current RTCP session.
 * @param[OUT] RtcpSession
 *  Returns the RTCP session context after successful initialization.
 *
 * @retval AK_OK
 *  Operation succeeded.
 * @retval AK_ERR_INVAL_PARAM
 *  Operation failed, invalid parameter passed in.
 * @retval AK_ERR_PEER_BROKEN
 *  Operation failed, the passed socket is not a connected socket.
 *
 */
AK_API AK_int akae_rtcp_session_init (AK_Socket sock, AK_int interleaved, AK_RtpSession *Rtp, AK_RtcpSession *RtcpSession);

/**
 * Destroy an RTCP session.
 */
AK_API AK_void akae_rtcp_session_destroy (AK_RtcpSession *RtcpSession);

/**
 * @brief
 *  Send an SR RTCP packet to the receiver.
 */
AK_API AK_boolean akae_rtcp_session_send_sr (AK_RtcpSession *RtcpSession);


/**
 * @brief
 *  Initialize an SDES packet.
 */
AK_API AK_boolean
AK_RTCP_SDES_Reset(const AK_RtcpSession *RtcpSession, AK_RtcpPacket *Package);

/**
 * @brief
 *  Get the size of an SDES packet.
 */
AK_API AK_ssize
AK_RTCP_SDES_Size(AK_RtcpPacket *SDES);

/**
 * @brief
 *  Append text to an SDES packet.
 *
 * @param[in,out] SDES
 *  SDES data object.
 * @param[in,out] identifier
 *  Identifier for each data item within the SDES.
 * @param[in] type
 *  SDES text data type, e.g. @see AK_RTCP_SDES_CNAME.
 * @param[in] str
 *  SDES text content; must use UTF-8 text encoding here.
 *
 */
AK_API AK_void
AK_RTCP_SDES_AddString(AK_RtcpPacket *SDES, AK_byte type, const AK_chrptr str);

/**
 * @macro
 *  Quick call for appending @ref AK_RTCP_SDES_CNAME type packets.
 */
#define AK_RTCP_SDES_AddCanonicalName(__Package, __cname) \
	AK_RTCP_SDES_AddString(__Package, AK_RTCP_SDES_CNAME, __cname)

/**
 * @macro
 *  Quick call for appending @ref AK_RTCP_SDES_NAME type packets.
 */
#define AK_RTCP_SDES_AddName(__Package, __name) \
	AK_RTCP_SDES_AddString(__Package, AK_RTCP_SDES_CNAME, __name)

/**
 * @macro
 *  Quick call for appending @ref AK_RTCP_SDES_EMAIL type packets.
 */
#define AK_RTCP_SDES_AddEmail(__Package, __email) \
	AK_RTCP_SDES_AddString(__Package, AK_RTCP_SDES_CNAME, __email)

/**
 * @macro
 *  Quick call for appending @ref AK_RTCP_SDES_PHONE type packets.
 */
#define AK_RTCP_SDES_AddPhone(__Package, __phone) \
	AK_RTCP_SDES_AddString(__Package, AK_RTCP_SDES_CNAME, __phone)

/**
 * @macro
 *  Quick call for appending @ref AK_RTCP_SDES_LOC type packets.
 */
#define AK_RTCP_SDES_AddLocation(__Package, __loc) \
	AK_RTCP_SDES_AddString(__Package, AK_RTCP_SDES_CNAME, __loc)

/**
 * @macro
 *  Quick call for appending @ref AK_RTCP_SDES_TOOL type packets.
 */
#define AK_RTCP_SDES_AddTool(__Package, __tool) \
	AK_RTCP_SDES_AddString(__Package, AK_RTCP_SDES_CNAME, __tool)

/**
 * @macro
 *  Quick call for appending @ref AK_RTCP_SDES_NOTE type packets.
 */
#define AK_RTCP_SDES_AddApplication(__Package, __app) \
	AK_RTCP_SDES_AddString(__Package, AK_RTCP_SDES_CNAME, __app)

/**
 * @brief
 *  Append text to an SDES packet.
 *
 * @param[in,out] SDES
 *  SDES data object.
 * @param[in,out] identifier
 *  Identifier for each data item within the SDES.
 * @param[in] type
 *  SDES text data type, e.g. @see AK_RTCP_SDES_CNAME.
 * @param[in] key
 *  SDES private extension data keyword; must use UTF-8 text encoding here.
 * @param[in] value
 *  SDES private extension content; must use UTF-8 text encoding here.
 *
 */
AK_API AK_void
AK_RTCP_SDES_AddPrivateExtension(AK_RtcpPacket *SDES, const AK_chrptr prefix, const AK_chrptr text);

/**
 * Receive a socket data packet on an RTCP session.
 *
 * @return
 *  Returns AK_True on success, AK_false on failure.
 */
AK_API AK_boolean akae_rtcp_session_recv_packet (AK_RtcpSession *Session, AK_RtcpPacket *Packet);

/**
 * @brief
 *  Send an SDES data packet.
 *
 * @param[IN] RTCP
 *  RTCP session handle.
 * @param[in] cname
 *  CNAME data content.
 */
AK_API AK_boolean akae_rtcp_session_send_sdes (AK_RtcpSession *RtcpSession, AK_chrptr cname);


AK_C_HEADER_EXTERN_C_END
#endif /* AKAE_RTCP_H_ */
