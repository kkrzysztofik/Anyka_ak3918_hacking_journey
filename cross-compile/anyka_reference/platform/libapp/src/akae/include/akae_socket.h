
/**
 * Socket.
 */

#include <akae_object.h>


#if !defined(AKSPC_SOCKET_H_)
#define AKSPC_SOCKET_H_
AK_C_HEADER_EXTERN_C_BEGIN



/**
 * Maximum number of available socket handles.
 */
#define AK_SOCK_MAX_BACKLOG  (256)

/**
 * TCP state definitions.
 */
#define AK_SOCK_TCP_STAT_ESTABLISHED   AK_MK2CC('e', 's')
#define AK_SOCK_TCP_STAT_SYN_SENT      AK_MK2CC('s', 's')
#define AK_SOCK_TCP_STAT_SYN_RECV      AK_MK2CC('s', 'r')
#define AK_SOCK_TCP_STAT_FIN_WAIT1     AK_MK2CC('f', '1')
#define AK_SOCK_TCP_STAT_FIN_WAIT2     AK_MK2CC('f', '2')
#define AK_SOCK_TCP_STAT_TIME_WAIT     AK_MK2CC('t', 'w')
#define AK_SOCK_TCP_STAT_CLOSE         AK_MK2CC('c', 'l')
#define AK_SOCK_TCP_STAT_CLOSE_WAIT    AK_MK2CC('c', 'w')
#define AK_SOCK_TCP_STAT_LAST_ACK      AK_MK2CC('l', 'a')
#define AK_SOCK_TCP_STAT_LISTEN        AK_MK2CC('l', 't')
#define AK_SOCK_TCP_STAT_CLOSING       AK_MK2CC('c', 'n')
#define AK_SOCK_TCP_STAT_UNKNOWN       (-1)


/**
 * Socket handle.
 */
#define AK_Socket AK_handle


/**
 * Initialize a socket.\n
 * Initialization requires passing in a static object memory block.\n
 * Returns an @ref AK_Object object handle on success, or AK_null on failure.
 *
 * @param[OUT]
 *  Object
 *  Object memory buffer in stack space, used for initialization.
 *
 * @param[IN]
 *  Protocol field; options include tcp, udp, icmp depending on user needs.
 *
 * @return
 *  Returns the socket handle on success, otherwise returns AK_null.
 *
 */
AK_API AK_Socket akae_socket_create (AK_chrcptr protocol);

/**
 * Quick call to create a TCP socket.
 */
#define akae_socket_create_tcp() akae_socket_create("tcp")

/**
 * Quick call to create a UDP socket.
 */
#define akae_socket_create_udp() akae_socket_create("udp")

/**
 * Quick call to create an ICMP socket.
 */
#define akae_socket_create_icmp() akae_socket_create("icmp")


/**
 * Destroy a socket.\n
 * Paired with @ref akae_socket_create().\n
 * Internally calls @ref akae_socket_close() to close the socket.\n
 * The user can call this interface directly to both close the socket and destroy the handle,\n
 * or call @ref akae_socket_close() first to close the socket, then call this interface to destroy the handle.
 *
 * @param[IN]
 *  Object
 *  Socket handle.
 *
 * @retval AK_ERR_INVAL_OBJECT
 *  Destruction failed; the socket handle is invalid.
 * @retval AK_OK
 *  Destruction succeeded.
 */
AK_API AK_int akae_socket_release (AK_Socket handle);
#define akae_socket_release_clear(__h) \
	({\
		AK_int const res = akae_socket_release (__h);\
		if (AK_OK == res) {\
			__h = AK_INVAL_HANDLE;\
		}\
		res;\
	})


/**
 * Within a certain time period, determine whether the socket is readable.
 * If the remote end has sent data to the local side,\n
 * the socket is readable and returns AK_true; otherwise returns AK_false.\n
 * The @ref wait_us parameter can limit the determination time; passing AK_null means
 * wait indefinitely unless the socket is closed.\n
 * Passing a data pointer with value 0 means return immediately.\n
 * Passing a specific microsecond value (us) controls the wait time.\n
 *
 * @param[IN]
 *  Object
 *  Socket object handle.
 *
 * @param[IN]
 *  wait_us
 *  Wait time in microseconds.\n
 *  Passing -1 means wait indefinitely; passing 0 means check and return immediately without waiting;\n
 *  passing a value greater than 0 means wait for the specified number of microseconds.
 *
 * @return
 *  Returns AK_true if readable, otherwise AK_false.
 */
AK_API AK_boolean akae_socket_can_read (AK_Socket handle, AK_int wait_us);

/**
 * Determine whether the socket is writable; see @ref akae_socket_can_read().
 */
AK_API AK_boolean akae_socket_can_write (AK_Socket handle, AK_int wait_us);

/**
 * Set the local address and port.\n
 * This interface can only be called once during the lifetime of a handle;
 * once the local address and port are set, they cannot be changed.
 *
 * @param[IN]
 *  handle
 *  Socket handle.
 *
 * @param[IN]
 *  addr
 *  Set the local address; pass AK_null to bind to any address.
 *
 * @param[IN]
 *  port
 *  Set the local port.
 *
 * @retval AK_OK
 *  Set successfully.
 * @retval AK_ERR_INVAL_HANDLE
 *  Set failed; invalid handle passed.
 * @retval AK_ERR_OUT_OF_RANGE
 *  Set failed; port value is out of range.
 * @retval AK_ERR_INVAL_PARAM
 *  Set failed; invalid address parameter passed.
 * @retval AK_ERR_INVAL_OPERATE
 *  This interface can only be called once per handle lifetime; calling it again returns an invalid operation error.
 *
 */
AK_API AK_int akae_socket_bind (AK_Socket handle, AK_chrptr addr, AK_int port);


AK_API AK_int akae_socket_local_addr (AK_Socket handle, AK_chrptr addr, AK_int *port);

/**
 * Listen on the port for incoming data.
 */
AK_API AK_int akae_socket_listen (AK_Socket handle, AK_size backlog);

AK_API AK_int akae_socket_connect (AK_Socket handle, AK_chrptr addr, AK_int port);

AK_API AK_int akae_socket_timed_connect (AK_Socket handle, AK_chrptr addr, AK_int port, AK_int timeo_us);

AK_API AK_int akae_socket_peer_addr (AK_Socket handle, AK_chrptr addr, AK_int *port);



/**
 *
 * @param[IN]
 *  hSockAK_ERR_OUT_OF_BACKLOG
 *  Socket handle created by @ref akae_socket_create().
 *
 * @param[OUT]
 *  addr
 *  Remote address of the accepted connection; pass AK_null if not needed.
 *
 * @param[OUT]
 *  port
 *  Remote port of the accepted connection; pass AK_null if not needed.
 *
 * @return
 *  Returns a connected socket on successful acceptance; this socket is taken over by the caller
 *  and must be closed via @ref akae_socket_release() when no longer needed.\n
 *  Returns the corresponding error code on failure.
 *
 *
 * @retval AK_ERR_INVAL_HANDLE
 *  An invalid socket handle was passed,\n
 *  or @ref akae_socket_release() was called externally while this interface was executing.
 *
 * @retval AK_ERR_OUT_OF_BACKLOG
 *  No socket handles available for allocation. The maximum number is @ref AK_SOCK_MAX_BACKLOG.\n
 *  If the number of allocated sockets exceeds this limit, new ones cannot be created.
 *  The user should check for handle leaks.
 *
 */
AK_API AK_Socket akae_socket_accept (AK_Socket handle, AK_chrptr addr, AK_int *port);

/**
 * See @ref akae_socket_accept() with timeout control support.
 *
 * @param[IN]
 *  timeo
 *  If timeo = -1, this method behaves the same as @ref akae_socket_accept().
 *  If timeo = 0, no timeout wait occurs; returns immediately.
 *  If time > 0, waits up to @ref timeo microseconds for an incoming connection.
 *
 * @retval AK_ERR_OUT_OF_TIMEO
 *  Returned when no connection was received within the timeout period.
 *
 */
AK_API AK_Socket akae_socket_accept2 (AK_Socket handle, AK_ssize timeo, AK_chrptr addr, AK_int *port);

AK_API AK_int akae_socket_get_ttl (AK_Socket handle, AK_size *ttl);

AK_API AK_int akae_socket_set_ttl (AK_Socket handle, AK_size ttl);

AK_API AK_int akae_socket_set_send_timeout (AK_Socket handle, AK_ssize us);

AK_API AK_int akae_socket_set_recv_timeout (AK_Socket handle, AK_ssize us);

AK_API AK_int akae_socket_get_send_timeout (AK_Socket handle, AK_ssize *us);

AK_API AK_int akae_socket_get_recv_timeout (AK_Socket handle, AK_ssize *us);


AK_API AK_ssize akae_socket_send_buffer (AK_Socket handle, AK_bytptr buffer, AK_size len);

AK_API AK_ssize akae_socket_send_buffer_all (AK_Socket handle, AK_bytptr buffer, AK_size len);

AK_API AK_ssize akae_socket_recv_buffer (AK_Socket handle, AK_bytptr buffer, AK_size len);

AK_API AK_ssize akae_socket_recv_buffer_all (AK_Socket handle, AK_bytptr buffer, AK_size len);

AK_API AK_ssize akae_socket_peek_buffer (AK_Socket handle, AK_bytptr buffer, AK_size len);

AK_API AK_ssize akae_socket_flush_buffer (AK_Socket handle, AK_size len);

/**
 *
 * @param handle
 * @param buffer
 * @param len
 * @return
 */
AK_API AK_ssize akae_socket_recv_buffer_all (AK_Socket handle, AK_bytptr buffer, AK_size len);

AK_API AK_ssize akae_socket_send_buffer_to (AK_Socket handle,
		AK_bytptr buffer, AK_size len, AK_chrptr toIP, AK_uint16 toPort);

AK_API AK_ssize akae_socket_recv_buffer_from (AK_Socket handle,
		AK_bytptr buffer, AK_size len, AK_chrptr fromIP, AK_uint16 *fromPort);

/**
 * Set whether TCP uses NODELAY mode.
 *
 * @return
 *  Returns AK_OK on success, otherwise returns the corresponding error code.
 */
AK_API AK_int akae_socket_tcp_nodelay (AK_Socket handle, AK_boolean enabled);


AK_C_HEADER_EXTERN_C_END
#endif ///< SOCKET_H_
