
/**
 * @brief
 *  Tosser 2.0 module.\n
 *  This module implements TCP plugin service-style listening for TCP transport requests.\n
 *  It enables rapid implementation of passive TCP request servers such as HTTP and RTSP.
 *
 *
 *  Terminology:
 *  Sink - sink
 *  Sunk - response sink
 */


//#include <NkUtils/allocator.h>
//#include <NkEmbedded/socket.h>
//#include <NkEmbedded/thread.h>
#include <akae_typedef.h>
#include <akae_object.h>
#include <akae_thread.h>
#include <akae_socket.h>


#if !defined(AKAE_TOSSER_H_)
#define AKAE_TOSSER_H_
AK_C_HEADER_EXTERN_C_BEGIN

/**
 * Maximum length of a sink name.
 */
#define AK_TOS_SINK_NAME_MAX    (32)


/**
 * Probe packet event. When the module receives a packet and cannot determine
 * which listener it belongs to, this event is triggered for determination.
 *
 * @param[IN]
 *  Tosser
 *  The Tosser module handle for the current event invocation.
 *
 * @param[IN] usrdata
 *  Module handle context, specified by the @ref NK_Tosser2_Create() method.
 *
 * @param[IN] bytes
 *  Start address of the data in memory.
 *
 * @param[IN] len
 *  Valid length of @ref bytes data.
 *
 * @return
 * Returns -1 when the packet is determined not to belong to this listener; the module will not
 * trigger probing again in the current session.
 * Returns 0 when the module cannot determine whether the packet belongs to the current listener;
 * the module will continue probing in the next probe cycle.
 * Returns @ref msg_len when the module determines the packet belongs to the current listener;
 * the module will continue triggering the @ref onEventLoop event.
 *
 */
typedef AK_boolean (*AK_TosserSunkProbe)(AK_Object Tosser, AK_voidptr usrdata, AK_bytptr bytes, AK_size len, AK_size *probe_actlen);


/**
 * Event loop event. Triggered when a session connects to the module and is confirmed
 * to belong to the current listener.
 *
 * @param[IN]		Tosser		The Tosser module handle for the current event invocation.
 * @param[IN]		usrdata			Tosser module handle context, specified by the @ref NK_Tosser2_Create() method.
 * @param[IN]		Thread		Current thread controller; can call @ref NK_Thread::suspend() and NK_Thread::terminate() for thread scheduling.
 * @param[IN]		ConnTCP		Session TCP connection handle.
 *
 *
 */
typedef AK_void (*AK_TosserSunkEventloop)(AK_Object Tosser, AK_size probe_actlen, AK_voidptr usrdata, AK_Thread th, AK_Socket conn);


typedef struct _AK_TosserSink {

	/**
	 * Sink probe event.
	 */
	AK_TosserSunkProbe  OnProbe;

	/**
	 * Sink response event loop.
	 */
	AK_TosserSunkEventloop OnEventloop;


} AK_TosserSink;


/**
 * Create a Tosser handle.
 *
 */
AK_API AK_Object akae_tosser_create (AK_Object Malloc, AK_uint16 port);

/**
 * Release the Tosser module handle.
 *
 * @param[IN]
 *  Object
 *  Module handle.
 *
 * @return
 *  Returns AK_OK on successful release; returns the corresponding return code on failure,
 *  which may indicate an invalid handle.
 *
 */
AK_API AK_int akae_tosser_release (AK_Object Object);


/**
 * Return the listening port.
 *
 *
 * @return
 *  Returns the port number (1-65535) the module is currently listening on;
 *  returns the corresponding error code on failure.
 */
AK_API AK_int akae_tosser_listen_port (AK_Object Object);

/**
 * @brief
 *  Reset the listening port.
 * @details
 *  After resetting, connections can only be listened for on the new port.
 *
 * @param AK_Object Object [in]
 *  this pointer.
 * @param port [in]
 *  Listening port number.
 *
 * @retval AK_OK
 *  Restart succeeded; the module will use the new port for listening.
 * @retval AK_ERR_INVAL_OBJECT
 *  Operation failed; an invalid object handle was passed.
 * @retval AK_ERR_INVAL_OPERATE
 *  Operation failed; the module continues using the old port for listening.
 *  This may be caused by passing an unavailable port.
 *
 */
AK_API AK_int akae_tosser_restart (AK_Object Object, AK_uint16 port);

/**
 * Add a listener (sink).
 *
 */
AK_API AK_int akae_tosser_add_sink (AK_Object Object, AK_chrcptr name, AK_TosserSink *Sink, AK_voidptr usrdata);

/**
 * Remove a listener (sink).
 */
AK_API AK_int akae_tosser_drop_sink (AK_Object Object, AK_chrcptr name);



AK_C_HEADER_EXTERN_C_END
#endif ///< AKAE_TOSSER_H_


