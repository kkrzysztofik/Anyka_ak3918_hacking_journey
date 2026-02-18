/**
 * HTTP protocol basic toolkit definitions.
 *
 */

#include <akae_typedef.h>
#include <akae_httputils.h>
#include <akae_verbose.h>
#include <akae_tosser.h>


#if !defined(AKAE_HTTPSERVER_H_)
# define AKAE_HTTPSERVER_H_
AK_C_HEADER_EXTERN_C_BEGIN


/**
 * Maximum HTTP header size the server can handle, including requests and responses.
 */
#define AK_HTTP_SERVER_MAX_HEADFIELD_LEN   (1024 * 8)

/**
 * Maximum URL length the server supports for parsing.
 */
#define AK_HTTP_SERVER_MAX_URL_LEN         (256)

/**
 * Server keep-alive wait time.
 */
#define AK_HTTP_SERVER_KEEP_ALIVE_SEC      (16)

/**
 * Maximum number of server keep-alive connections.
 */
#define AK_HTTP_SERVER_KEEP_ALIVE_MAX      (128)




/**
 * @brief
 *  HTTP server configuration data structure.
 */
#pragma pack (push, 4)
typedef struct _AK_HttpServerProperty {

	/**
	 * Resource writable flag, default is AK_false.
	 */
	AK_boolean resource_writables;

	/**
	 * Keep-Alive retention time,\n
	 * if this property is 0 it means Keep-Alive is not used.\n
	 * In HTTP 1.1 protocol it is enabled by default with value 60, i.e., default 60-second Keep-Alive timeout.
	 */
	AK_size keepalive_sec;

	/**
	 * Keep-Alive wait count,\n
	 * valid when @ref keepalive_sec is greater than 0.\n
	 */
	AK_size keepalive_max;

	AK_size maxAttachedCache;

} AK_HttpServerProperty;


typedef struct _AK_HttpServerSession {

	/// Memory allocator used by the server.
	AK_Object Malloc;

	/// TCP connection socket.
	AK_Socket tcp;

	/// Keep-Alive parameters.
	struct {

		/// Remaining seconds for the kept-alive session.
		AK_ssize seconds;

		/// Maximum number of times to keep the session alive.
		AK_ssize max;

	} Keepalive;

	/**
	 * Current HTTP session authenticated user.
	 */
//	NK_HTTPAccessUsers *AccessUsers;

	struct {

		/// Raw content of the request header field packet.
		AK_char headfield[AK_HTTP_SERVER_MAX_HEADFIELD_LEN + 1];

		/// Valid length of the request header field packet.
		AK_ssize headfieldlen;

		/// Request header field data structure.
		AK_Object Headfield;

		/// Request data content length.
		AK_size content_length;

	} Request;

	struct {

		/**
		 * Response packet header field data structure.
		 */
		AK_Object Headfield;

		/**
		 * Response data content.
		 */
		struct {

			/**
			 * Starting address of the server response data area.\n
			 * Macro calls such as @ref NK_HTTP_RESP_PUT_DATA() affect this value.
			 */
			AK_bytptr data;

			/**
			 * Starting address of the memory for response data when it is referenced data.
			 * Macro calls such as @ref NK_HTTP_RESP_PUT_DATA_REF() affect this value.
			 */
			AK_bytptr dataRef;

			/**
			 * Server response data length and maximum buffer length.\n
			 */
			AK_size datalen, capacity;

			/**
			 * File for server response,\n
			 * @ref NK_HTTP_RESP_PUT_FILE() macro call affects this value.
			 */
			AK_chrptr datafile;

		} Content;

//		/**
//		 * OPTIONS request response flag.
//		 */
//		NK_UInt32 optionsAllow;

	} Response;

} AK_HttpServerSession;
#pragma pack (pop)


/**
 * Server CGI user event definition.
 *
 * @param[IN] Session
 *  HTTP server session context.
 *
 * @param[IN] th
 *  HTTP server thread session handle.
 *
 * @param[IN] sock
 *  HTTP server TCP socket handle.
 *
 * @param[IN] userdata
 *  User data, passed in via @ref akae_http_server_add_cgi() or @ref akae_http_server_add_cgi_prefix().
 *
 */
typedef AK_void (*AK_HttpServerCgiHandle)(AK_HttpServerSession *Session, AK_Thread th, AK_voidptr userdata);


/**
 * Create an HTTP server.
 *
 * @param[IN] Malloc
 *  Memory allocator for creating the HTTP server; the allocator must support thread safety, otherwise creation will fail.
 *
 * @return
 *  Returns the module object on success, otherwise returns AK_null.
 */
AK_API AK_Object akae_http_server_create (AK_Object Malloc);


/**
 * Release the HTTP server object handle.
 *
 * @param[IN] Object
 *  HTTP server module object handle.
 *
 * @retval AK_OK
 *  Released successfully.
 * @retval AK_ERR_INVAL_OBJECT
 *  Release failed, possibly due to an abnormal module object handle.
 */
AK_API AK_int akae_http_server_release (AK_Object Object);

/**
 * Enable/disable debug print output.
 */
AK_API AK_int akae_http_server_verbose (AK_Object Object, AK_boolean enabled);

/**
 * Set a request path alias.
 */
AK_API AK_int akae_http_server_alias_path (AK_Object Object, AK_chrcptr origin_path, AK_chrcptr alias_path);

AK_API AK_int akae_http_server_redirect_path (AK_Object Object, AK_chrcptr origin_path, AK_chrcptr redirect_path);

/**
 * Bind the HTTP server handle to a Tosser module for operation.\n
 * The HTTP server can bind to a Tosser module listener, or listen independently; see @ref akae_http_server_start().
 *
 * @param[IN] Object
 *  HTTP server module object handle.
 * @param[IN] Tosser
 *  Tosser module object handle.
 *
 * @retval AK_OK
 *  Operation successful.
 * @retval AK_ERR_INVAL_OBJECT
 *  Operation failed, handle is invalid.
 *
 */
AK_API AK_int akae_http_server_bind_tosser (AK_Object Object, AK_Object Tosser);

/**
 * Unbind the HTTP server.
 *
 * @param[IN] Object
 *  HTTP server module object handle.
 *
 * @retval AK_OK
 *  Operation successful.
 * @retval AK_ERR_INVAL_OBJECT
 *  Operation failed, handle is invalid.
 */
AK_API AK_int akae_http_server_unbind_tosser (AK_Object Object);

AK_API AK_int akae_http_server_start (AK_Object Object, AK_uint16 port);

AK_API AK_int akae_http_server_finish (AK_Object Object);


AK_API AK_int akae_http_server_add_cgi (AK_Object Object, AK_chrcptr path, AK_HttpServerCgiHandle CgiHandle, AK_voidptr userdata);

AK_API AK_int akae_http_server_add_cgi_prefix (AK_Object Object, AK_chrcptr path_prefix, AK_HttpServerCgiHandle CgiHandle, AK_voidptr userdata);

AK_API AK_int akae_http_server_drop_cgi (AK_Object Object, AK_chrcptr path);

AK_API AK_int akae_http_server_set_resource_fspath (AK_Object Object, AK_chrcptr fspath);

AK_API AK_chrptr akae_http_server_get_resource_fspath (AK_Object Object, AK_chrptr stack, AK_size stacklen);




AK_API AK_void akae_http_server_session_response_disable_keepalive (AK_HttpServerSession *Session);

AK_API AK_void akae_http_server_session_response_put_result (AK_HttpServerSession *Session, AK_int status, AK_chrcptr phrase);

#define akae_http_server_session_response_put_option(__Session, __opt, __fmt...) \
	({\
		if (!(__Session)->Response.Headfield) {\
			akae_http_server_session_response_put_result (__Session, AK_HTTP_CODE_OK, AK_null);\
		}\
		akae_http_headfield_add_option ((__Session)->Response.Headfield, AK_false, __opt, ##__fmt);\
	})

/**
 * Update the response content in the HTTP server session context.
 *
 * @param[IN] Session
 *  Session context.
 * @param[IN] data
 *  Starting address of the memory containing the HTTP response content data.
 * @param[IN] datalen
 *  Length of the HTTP response content data.
 *
 */
AK_API AK_void akae_http_server_session_response_put_content (AK_HttpServerSession *Session, AK_voidcptr data, AK_size datalen);

AK_API AK_void akae_http_server_session_response_put_file (AK_HttpServerSession *Session, AK_chrcptr filepath);


AK_C_HEADER_EXTERN_C_END
#endif ///< AKAE_HTTPSERVER_H_


