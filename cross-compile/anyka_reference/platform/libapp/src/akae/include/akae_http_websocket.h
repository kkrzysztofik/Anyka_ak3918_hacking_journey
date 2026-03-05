
/**
 * WebSocket function interface definitions.
 * See https://tools.ietf.org/html/rfc6455.
 *
 */

#include <akae_typedef.h>
#include <akae_socket.h>
#include <akae_httpserver.h>

#if !defined(AKAE_HTTP_WEBSOCKET_H_)
#define AKAE_HTTP_WEBSOCKET_H_
AK_C_HEADER_EXTERN_C_BEGIN


/**
 * @macro
 *  WebSocket frame data type definitions.
 */
#define AK_HTTP_WSOCK_OPT_CONTINUATION (0x00) ///< Denotes a continuation frame.
#define AK_HTTP_WSOCK_OPT_TEXT         (0x01) ///< Denotes a text frame.
#define AK_HTTP_WSOCK_OPT_BINARY       (0x02) ///< Denotes a binary frame.
#define AK_HTTP_WSOCK_OPT_CONN_CLOSE   (0x08) ///< Denotes a connection close.
#define AK_HTTP_WSOCK_OPT_PING         (0x09) ///< Denotes a ping.
#define AK_HTTP_WSOCK_OPT_PONG         (0x0a) ///< Denotes a pong.

/**
 * @macro
 *  Masking key length, in bytes.
 */
#define AK_HTTP_WSOCK_MASK_KEY_LEN          (4)

/**
 * @macro
 *  Minimum frame header length, in bytes.
 */
#define AK_HTTP_WS_MIN_FRAME_HEAD_SZ   (2)

/**
 * @macro
 *  Maximum frame header length.
 */
#define AK_HTTP_WS_MAX_FRAME_HEAD_SZ   (AK_HTTP_WS_MIN_FRAME_HEAD_SZ + 8 + AK_HTTP_WSOCK_MASK_KEY_LEN)

/**
 * @brief
 *  WebSocket frame header field data structure,\n
 *  corresponding to WebSocket version 13.
 */
typedef struct _AK_HttpWebSocketFrame {

	AK_size size;              ///< Header field length, in bytes.

	/// FIN flag bit.
	AK_boolean fin;

	///< Mask flag.
	AK_boolean mask;

	///< Data type.
	AK_uint32 optype;

	///< Extra length descriptor size, may be 0, 2, or 8.
	AK_size extraLenBytes;

	///< Payload data length.
	AK_size64 payloadlen;

	/// Masking key array.
	AK_byte maskingKey[AK_HTTP_WSOCK_MASK_KEY_LEN];

} AK_HttpWebSocketFrame;


//AK_API AK_Socket
//AK_HTTPWS_Handshake (AK_Object Object, AK_PChar url, AK_PChar username, AK_PChar passphrase, AK_int us);

/**
 * @brief
 *  WebSocket handshake response.
 * @brief
 *  Called within the CGI implementation, targeting WebSocket version 13 and related versions.\n
 *  If the current CGI session is a WebSocket session,\n
 *  internally calls the @ref Conn socket send interface to reply with handshake confirmation.
 *
 * @param[IN] wsock
 *  Websocket connection TCP handle.
 * @param Session [in]
 *  CGI session.
 *
 * @return
 *  Returns True on successful handshake response, otherwise returns False.
 */
AK_API AK_boolean akae_http_websocket_handshake_answer (AK_HttpServerSession *Session);


/**
 * Within the timeout period, wait for a data frame from the peer and retrieve frame parameters.
 *
 * @param[IN] wsock
 *  Websocket connection TCP handle.
 *
 * @param[IN] timeo
 *  Wait timeout, in microseconds,\n
 *  internally calls the @ref akae_socket_can_read() interface.
 *  0 means no wait timeout, -1 means wait indefinitely until an error is returned.
 *
 * @param[OUT] Headfield
 *  After successfully waiting, retrieves the frame header field information, then receives
 *  payload data via @ref akae_http_websocket_recv_payload().
 *
 * @retval AK_OK
 *  Successfully waited for one frame of data.
 * @retval AK_ERR_TIME_OUT
 *  Wait timed out, no data from peer.
 * @retval AK_ERR_PEER_BROKEN
 *  Wait failed, peer disconnected.
 * @retval AK_ERR_EXCEPTION
 *  Wait failed, peer sent an abnormal packet during communication.
 *
 */
AK_API AK_int akae_http_websocket_wait_frame (AK_Socket wsock, AK_int timeo, AK_HttpWebSocketFrame *Headfield);

/**
 * Server wait array.
 */
AK_API AK_int akae_http_websocket_server_wait_frame (AK_Socket wsock, AK_int timeo, AK_HttpWebSocketFrame *Headfield);


AK_API AK_int akae_http_websocket_client_wait_frame (AK_Socket wsock, AK_int timeo, AK_HttpWebSocketFrame *Headfield);


/**
 * @brief
 *  Receive data frame.
 * @details
 *  This method is used to receive the payload data of a frame and save it to the @ref stack memory address.\n
 *  Calling this interface returns immediately without blocking, so before calling it you must first call
 *  @ref AK_HTTPWS_PredictFrame() or @ref AK_HTTPWS_WaitFrame() to ensure there is data to receive,\n
 *  and allocate storage memory space according to the result of @ref AK_HTTPWS_PredictFrame() or
 *  @ref AK_HTTPWS_WaitFrame(), configuring the @ref stacklen size.\n
 *  If the received frame payload data size exceeds @ref stacklen, this method returns 0 and does not read.
 *
 * @param[IN] wsock
 *  Websocket connection TCP handle.
 * @param[IN] Headfield
 *  Frame header field information, obtained via @ref akae_http_websocket_wait_frame().
 * @param stack [out]
 *  Starting address of the memory location for the frame's payload data.
 * @param stacklen [in]
 *  Memory size of the frame's payload data, in bytes.
 *
 * @return
 *  Returns the size of the payload data on success (may be 0), returns -1 on error.
 *
 */
AK_API AK_ssize akae_http_websocket_recv_payload (AK_Socket wsock, AK_HttpWebSocketFrame *Headfield, AK_bytptr stack, AK_size64 stacklen);



/**
 * @macro
 *  Macro operation to apply mask to payload data.
 */
#define AK_HTTP_WSOCK_MASK_PAYLOAD(__mask, __payload, __payloadlen) \
	do {\
		AK_int __i = 0;\
		for (__i = 0; __i < __payloadlen; ++__i) {\
			*((AK_bytptr)(__payload) + __i) = *((AK_bytptr)(__payload) + __i) ^ *((AK_bytptr)(__mask) + __i % 4);\
		}\
	} while(0)

/**
 * @macro
 *  Macro operation to remove mask from payload data.
 */
#define AK_HTTP_WSOCK_UMASK_PAYLOAD(__mask, __payload, __payloadlen) AK_HTTP_WSOCK_MASK_PAYLOAD(__mask, __payload, __payloadlen)

/**
 * @brief
 *  Send frame data.
 * @details
 *  Send one frame of data to the peer.\n
 *  Note: This interface is a generic frame sending method, but in the WebSocket protocol, @ref mask
 *  differs between sending to the server and sending to the client.\n
 *  See @ref akae_http_websocket_server_send_frame() and @ref akae_http_websocket_client_send_frame() for details.
 *
 * @param[IN] wsock
 *  Websocket connection TCP handle.
 * @param optype [in]
 *  Payload data type.
 * @param mask [in]
 *  Mask flag, indicates whether the sent data uses a data mask.
 * @param data [in]
 *  Starting address of the payload data memory.
 * @param len [in]
 *  Payload data length.
 *
 * @retval AK_OK
 *  Send successful.
 * @retval AK_ERR_INVAL_HANDLE
 *  Send failed, possibly due to a listen failure.
 * @retval AK_ERR_INVAL_PARAM
 *  Send failed, possibly due to a parameter error.
 */
AK_API AK_int akae_http_websocket_send_frame (AK_Socket wsock, AK_uint32 optype, AK_boolean mask, AK_voidptr payload, AK_size64 payloadlen);

/**
 * Server sends one frame of data to the client.
 */
#define akae_http_websocket_server_send_frame(__wsock, __optype, __payload, __payloadlen) \
	akae_http_websocket_send_frame (__wsock, __optype, AK_false, (AK_voidptr)(__payload), __payloadlen)

/**
 * Client sends one frame of data to the server.
 */
#define akae_http_websocket_client_send_frame(__wsock, __optype, __payload, __payloadlen) \
	akae_http_websocket_send_frame (__wsock, __optype, AK_true, (AK_voidptr)(__payload), __payloadlen)


/**
 * @macro
 *  Macro operation for server to send Ping control data.
 */
#define akae_http_websocket_server_ping(__wsock) \
	akae_http_websocket_server_send_frame (__wsock, AK_HTTP_WSOCK_OPT_PING, AK_null, 0)

/**
 * @macro
 *  Macro operation for server to send Ping control data with user data.
 */
#define akae_http_websocket_server_ping2(__wsock, __payload, __payloadlen) \
	((__payloadlen) <= 125 ? \
			akae_http_websocket_server_send_frame (__wsock, AK_HTTP_WSOCK_OPT_PING, (AK_voidptr)(__payload), __payloadlen) : AK_ERR_OUT_OF_RANGE)


/**
 * @macro
 *  Macro operation for client to send Ping control data.
 */
#define akae_http_websocket_client_ping(__wsock) \
		akae_http_websocket_client_send_frame (__wsock, AK_HTTP_WSOCK_OPT_PING, AK_null, 0)

/**
 * @macro
 *  Macro operation for client to send Ping control data with user data.
 */
#define akae_http_websocket_client_ping2(__wsock, __payload, __payloadlen) \
	((__payloadlen) <= 125 ? \
			akae_http_websocket_client_send_frame (__wsock, AK_HTTP_WSOCK_OPT_PING, (AK_voidptr)(__payload), __payloadlen) : AK_ERR_OUT_OF_RANGE)


/**
 * @macro
 *  Macro operation for server to send Pong control data.
 */
#define akae_http_websocket_server_pong(__wsock) \
	akae_http_websocket_server_send_frame (__wsock, AK_HTTP_WSOCK_OPT_PONG, AK_null, 0)

/**
 * @macro
 *  Macro operation for server to send Pong control data with user data.
 */
#define akae_http_websocket_server_pong2(__wsock, __payload, __payloadlen) \
	((__payloadlen) <= 125 ? \
			akae_http_websocket_server_send_frame (__wsock, AK_HTTP_WSOCK_OPT_PONG, (AK_voidptr)(__payload), __payloadlen) : AK_ERR_OUT_OF_RANGE)

/**
 * @macro
 *  Macro operation for server to send Pong control data.
 */
#define akae_http_websocket_client_pong(__wsock) \
	akae_http_websocket_client_send_frame (__wsock, AK_HTTP_WSOCK_OPT_PONG, AK_null, 0)

/**
 * @macro
 *  Macro operation for server to send Pong control data with user data.
 */
#define akae_http_websocket_client_pong2(__wsock, __payload, __payloadlen) \
	((__payloadlen) <= 125 ? \
			akae_http_websocket_client_send_frame (__wsock, AK_HTTP_WSOCK_OPT_PONG, (AK_voidptr)(__payload), __payloadlen) : AK_ERR_OUT_OF_RANGE)


AK_API AK_int akae_http_websocket_server_ping_pong (AK_Socket wsock, AK_int nonce, AK_int us);

AK_API AK_int akae_http_websocket_client_ping_pong (AK_Socket wsock, AK_int nonce, AK_int us);


/**
 * @macro
 *  Macro operation to send Close control data.
 */
#define akae_http_websocket_close(__wsock) \
	akae_http_websocket_send_frame(__wsock, AK_HTTP_WSOCK_OPT_CONN_CLOSE, AK_false, AK_null, 0)

/**
 * @macro
 *  Macro operation to send text data.
 */
#define akae_http_websocket_client_send_text(__wsock, __text) \
	akae_http_websocket_send_frame(__wsock, AK_HTTP_WSOCK_OPT_TEXT, AK_true, (AK_bytptr)(__text), akae_strlen(__text))

/**
 * @macro
 *  Macro operation to send text data.
 */
#define akae_http_websocket_server_send_text(__wsock, __text) \
	akae_http_websocket_send_frame(__wsock, AK_HTTP_WSOCK_OPT_TEXT, AK_false, (AK_bytptr)(__text), akae_strlen(__text))


/**
 * @macro
 *  Macro operation to send binary data.
 */
#define akae_http_websocket_client_send_binary(__wsock, __data, __datalen) \
	akae_http_websocket_send_frame(__wsock, AK_HTTP_WSOCK_OPT_BINARY, AK_true, (AK_bytptr)(__data), (__datalen))

/**
 * @macro
 *  Macro operation to send binary data.
 */
#define akae_http_websocket_server_send_binary(__wsock, __data, __datalen) \
	akae_http_websocket_send_frame(__wsock, AK_HTTP_WSOCK_OPT_BINARY, AK_false, (AK_bytptr)(__data), (__datalen))


AK_C_HEADER_EXTERN_C_END
#endif ///< AKAE_HTTP_WEBSOCKET_H_
