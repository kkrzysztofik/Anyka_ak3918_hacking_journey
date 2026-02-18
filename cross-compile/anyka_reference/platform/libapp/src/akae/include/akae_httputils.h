/**
 * HTTP protocol basic toolkit definitions.
 *
 */

#include <akae_typedef.h>
#include <akae_object.h>
//#include <NkUtils/types.h>
//#include <NkUtils/allocator.h>
//#include <NkUtils/str_list.h>
//#include <NkUtils/assert.h>
#include <akae_verbose.h>


#if !defined(AKAE_HTTPUTILS_H_)
# define AKAE_HTTPUTILS_H_
AK_C_HEADER_EXTERN_C_BEGIN

/**
 * HTTP newline character definition.
 */
#define AK_CRLF "\r\n"

/**
 * @macro
 *  Maximum protocol name length for HTTP-like protocols.
 */
#define AK_HTTP_LIKE_PROTO_NAME_MAX_LEN  (16)

/**
 * Maximum domain name length.
 */
#define AK_HTTP_HOST_MAX_LEN             (256)

/**
 * Maximum path length.
 */
#define AK_HTTP_PATH_MAX_LEN             (256)

/**
 * Maximum protocol query string length.
 */
#define AK_HTTP_QUERY_STRING_MAX_LEN     (1024 * 4)

/**
 * HTTP response code definitions.
 */
enum {

	AK_HTTP_CODE_CONTINUE = (100),
	AK_HTTP_CODE_SWITCHING_PROTOCOLS = (101),
	AK_HTTP_CODE_OK = (200),
	AK_HTTP_CODE_CREATED = (201),
	AK_HTTP_CODE_ACCEPTED = (202),
	AK_HTTP_CODE_NON_AUTHORITATIVE_INFOMATION = (203),
	AK_HTTP_CODE_NO_CONTENT = (204),
	AK_HTTP_CODE_RESET_CONTENT = (205),
	AK_HTTP_CODE_PARTIAL_CONTENT = (206),
	AK_HTTP_CODE_MULTIPLE_CHOICES = (300),
	AK_HTTP_CODE_MOVED_PERMANENTLY = (301),
	AK_HTTP_CODE_FOUND = (302),
	AK_HTTP_CODE_SEE_OTHER = (303),
	AK_HTTP_CODE_NOT_MODIFIED = (304),
	AK_HTTP_CODE_USE_PROXY = (305),
	AK_HTTP_CODE_TEMPORARY_REDIRECT = (307),
	AK_HTTP_CODE_BAD_REQUEST = (400),
	AK_HTTP_CODE_UNAUTHORIZED = (401),
	AK_HTTP_CODE_PAYMENT_REQUIRED = (402),
	AK_HTTP_CODE_FORBIDDEN = (403),
	AK_HTTP_CODE_NOT_FOUND = (404),
	AK_HTTP_CODE_METHOD_NOT_ALLOWED = (405),
	AK_HTTP_CODE_NOT_ACCEPTABLE = (406),
	AK_HTTP_CODE_PROXY_AUTHENTICATION_REQUIRED = (407),
	AK_HTTP_CODE_REQUEST_TIME_OUT = (408),
	AK_HTTP_CODE_CONFLICT = (409),
	AK_HTTP_CODE_GONE = (410),
	AK_HTTP_CODE_LENGTH_REQUIRED = (411),
	AK_HTTP_CODE_PRECONDITION_FAILED = (412),
	AK_HTTP_CODE_REQUEST_ENTITY_TOO_LARGE = (413),
	AK_HTTP_CODE_REQUEST_URI_TOO_LARGE = (414),
	AK_HTTP_CODE_UNSUPPORTED_MEDIA_TYPE = (415),
	AK_HTTP_CODE_REQUESTED_RANGE_NOT_SATISFIABLE = (416),
	AK_HTTP_CODE_EXPECTATION_FAILED = (417),
	AK_HTTP_CODE_INTERNAL_SERVER_ERROR = (500),
	AK_HTTP_CODE_NOT_IMPLEMENTED = (501),
	AK_HTTP_CODE_BAD_GATEWAY = (502),
	AK_HTTP_CODE_SERVICE_UNAVAILABLE = (503),
	AK_HTTP_CODE_GATEWAY_TIME_OUT = (504),
	AK_HTTP_CODE_HTTP_VERSION_NOT_SUPPORTED = (505),

};


/**
 * Get the MIME type of a file by its name.\n
 * The interface obtains the MIME type based on the file extension, so the filename must include
 * an extension; for example, file.txt returns text/plain.
 *
 * @return
 *  Returns the MIME type corresponding to the filename; returns application/octet-stream
 *  if the filename is invalid or the extension is unrecognized.
 *
 */
AK_API AK_chrcptr akae_http_file_mime (AK_chrcptr file_name);

/**
 * Get the default message for an HTTP response code.
 *
 * @param[IN]
 *  code
 *  HTTP response code.
 *
 * @return
 *  The message corresponding to the response code, or AK_null if the code does not exist.
 */
AK_API AK_chrptr akae_http_default_reason_phrase (AK_int code);

/**
 * Perform URI encoding on a string.
 *
 * @details
 *  Does not encode the symbols @ * / +.
 *
 * @param[IN]
 *  str
 *  Original string.
 *
 * @param[OUT]
 *  stack
 *  Starting memory location for the encoded result.
 *
 * @param stacklen [in,out]
 *  Pass in the encoded memory size, returns the encoded string length.
 *
 * @return
 *  Returns the encoded output string length on success and stores the encoded result
 *  in the @ref trans memory area, returns -1 on failure.
 *
 */
AK_API AK_ssize akae_http_str_escape (AK_chrptr str, AK_chrptr stack, AK_size stacklen);

/**
 * Decode a URI string.\n
 * Does not support UTF-8 encoding conversion.\n
 * The input @ref enc and @ref uri may point to the same memory block.
 *
 * @param str [IN]
 *  Original encoded string.
 * @param trans [out]
 *  Starting memory location for the decoded result.
 * @param trans_len [in,out]
 *  Pass in the decoded memory size, returns the decoded string length.
 *
 * @return
 *  Returns 0 on success and stores the decoded result in the @ref trans memory area, returns -1 on failure.
 */
AK_API AK_ssize akae_http_str_unescape (AK_chrptr str, AK_chrptr stack, AK_size stacklen);

/**
 * Encode a URI string,\n
 * does not support UTF-8 encoding conversion.
 *
 * @details
 *  Does not encode the symbols ~ ! @ # $ & * ( ) = : / , ; ? +.
 *
 * @param str [IN]
 *  Original string.
 * @param trans [out]
 *  Starting memory location for the encoded result.
 * @param trans_len [in,out]
 *  Pass in the encoded memory size, returns the encoded string length.
 *
 * @return
 *  Returns 0 on success and stores the encoded result in the @ref trans memory area, returns -1 on failure.
 */
AK_API AK_ssize akae_http_encode_uri (AK_chrptr str, AK_chrptr stack, AK_size stacklen);

/**
 * Decode a URI string.\n
 * Does not support UTF-8 encoding conversion.\n
 * The input @ref enc and @ref uri may point to the same memory block.
 *
 * @param str [IN]
 *  Original encoded string.
 * @param trans [out]
 *  Starting memory location for the decoded result.
 * @param trans_len [in,out]
 *  Pass in the decoded memory size, returns the decoded string length.
 *
 * @return
 *  Returns 0 on success and stores the decoded result in the @ref trans memory area, returns -1 on failure.
 */
AK_API AK_ssize akae_http_decode_uri (AK_chrcptr str, AK_chrptr stack, AK_size stacklen);


/**
 *
 * @details
 *  Does not encode the symbols ~ ! * ( ).
 *
 * @param str [IN]
 *  Original string.
 * @param trans [out]
 *  Starting memory location for the encoded result.
 * @param trans_len [in,out]
 *  Pass in the encoded memory size, returns the encoded string length.
 *
 * @return
 *  Returns 0 on success and stores the encoded result in the @ref trans memory area, returns -1 on failure.
 *
 */
AK_API AK_ssize akae_http_encode_uri_component (AK_chrptr str, AK_chrptr stack, AK_size stacklen);

/**
 * Decode a URI string.\n
 * Does not support UTF-8 encoding conversion.\n
 * The input @ref enc and @ref uri may point to the same memory block.
 *
 *
 * @param str [IN]
 *  Original encoded string.
 * @param trans [out]
 *  Starting memory location for the decoded result.
 * @param trans_len [in,out]
 *  Pass in the decoded memory size, returns the decoded string length.
 *
 * @return
 *  Returns 0 on success and stores the decoded result in the @ref trans memory area, returns -1 on failure.
 */
AK_API AK_ssize akae_http_decode_uri_component (AK_chrptr str, AK_chrptr stack, AK_size stacklen);


/**
 * @brief
 *  Compact a URI. Remove leading and trailing consecutive spaces from the @ref uri string,\n
 *  and remove consecutive / and \ characters in the middle of the string.\n
 *
 * @param[IN]
 *  uri
 *  Input URI string.\n
 *
 * @param[OUT]
 *  stack
 *  Stack area memory location.\n
 *
 * @param[IN]
 *  stacklen
 *  Stack area memory length.\n
 *
 * @retval >=0
 *  Length of the compacted URI; result is in @ref stack.\n
 *
 * @retval -1.
 *  Compaction failed.\n
 */
AK_API AK_ssize akae_http_strip_uri (AK_chrcptr uri, AK_chrptr stack, AK_size stacklen);



/**
 * URL data structure, used to structurally represent URL field contents.
 */
#pragma pack(push, 4)
typedef struct _AK_HttpUrl {

	/// Protocol name.
	AK_char proto[AK_HTTP_LIKE_PROTO_NAME_MAX_LEN + 1];

	/// Domain name.
	AK_char host[AK_HTTP_HOST_MAX_LEN + 1];

	/// Port.
	AK_int port;

	/// Path.
	AK_char path[AK_HTTP_PATH_MAX_LEN + 1];

	/// Query field.
	AK_char query[AK_HTTP_QUERY_STRING_MAX_LEN + 1];

} AK_HttpUrl;
#pragma pack(pop)


/**
 * Parse a URL string into the data structure @ref AK_HttpUrl.\n
 * Users can use this interface to split the individual elements of a URL string for use as other data inputs.
 *
 * @retval AK_OK
 *  Parse successful; retrieve results via the @ref Url data structure.
 * @retval AK_ERR_INVAL_PARAM
 *  Parse failed, possibly due to a parameter error.
 * @retval AK_ERR_INVAL_OPERATE
 *  Parse failed, possibly because a string with unparseable content was passed in.
 */
AK_API AK_int akae_http_parse_url (AK_chrcptr url, AK_HttpUrl *Url);

/**
 * Serialize a URL string.\n
 * Reverse operation of @ref akae_http_parse_url().
 *
 * @return
 *  Returns the string length on success; the string is stored at the memory address of @ref stack.
 * @retval AK_ERR_INVAL_PARAM
 *  Serialization failed, possibly due to incorrect parameters.
 *
 */
AK_API AK_ssize akae_http_stringify_url (AK_HttpUrl *Url, AK_chrptr stack, AK_size stacklen);


/**
 * Print the @ref AK_HttpUrl data structure.
 */
#define AK_HTTP_URL_DUMP(__Url) \
	do{\
		AK_VerboseForm Form;\
		akae_verbose_form_init (&Form, "URL", 64, 4);\
		akae_verbose_form_put_kv (&Form, AK_false, "Protocol", "%s", (__Url)->proto);\
		akae_verbose_form_put_kv (&Form, AK_false, "Host", "%s", (__Url)->host);\
		akae_verbose_form_put_kv (&Form, AK_false, "Port", "%d", (AK_int)((__Url)->port));\
		akae_verbose_form_put_kv (&Form, AK_false, "Path", "%s", (__Url)->path);\
		akae_verbose_form_finish (&Form);\
	} while(0)


#if 0




/**
 * @macro
 *  Print the AK_HTTPHeadField data structure.
 */
#define AK_HTTP_HEAD_FIELD_DUMP(__Object) \
	do{\
		AK_TermTable Table;\
		AK_Size number = 0;\
		AK_chrptr key = AK_AK_null;\
		AK_chrptr value = AK_AK_null;\
		AK_Int i = 0;\
		\
		AK_CHECK_POINT();\
		AK_TermForm_BeginDraw(&Table, "HTTP Head Filed", 96, 4);\
		AK_TermForm_PutKeyValue(&Table, AK_True, "Protocol", "%s %u.%u",\
				(__Object)->protocol, (__Object)->ver_maj, (__Object)->ver_min);\
		if ((__Object)->isRequest) {\
			AK_TermForm_PutKeyValue(&Table, AK_False, "Method", "%s", (__Object)->method);\
			AK_TermForm_PutKeyValue(&Table, AK_True, "Absolute Path", "%s", (__Object)->abs_path);\
			number = (__Object)->numberOfQuery((__Object));\
			AK_TermForm_PutText(&Table, AK_True, "Query String (%u)", number);\
			for (i = 0; i < (AK_Int)number; ++i) {\
				if (0 == (__Object)->indexOfQuery((__Object), i, &key, &value)) {\
					AK_TermForm_PutKeyValue(&Table, (i == number - 1), key, "%s", value);\
				}\
			}\
		} else {\
			AK_TermForm_PutKeyValue(&Table, AK_False, "Code", "%u", (__Object)->code);\
			AK_TermForm_PutKeyValue(&Table, AK_True, "Reason Phrase", "%s", (__Object)->reason_phrase);\
		}\
		\
		number = (__Object)->numberOfOption((__Object));\
		AK_TermForm_PutText(&Table, AK_True, "Option Tag (%u)", number);\
		for (i = 0; i < (AK_Int)number; ++i) {\
			if (0 == (__Object)->indexOfOption((__Object), i, &key, &value)) {\
				AK_TermForm_PutKeyValue(&Table, (i == number - 1), key, "%s", value);\
			}\
		}\
		AK_TermForm_EndDraw(&Table);\
	} while(0)




/**
 * @macro
 *  Quick call to probe HTTP header field.
 */
#define AK_HTTPUtils_DetectHTTPHeadField(__package, __len, __ver_maj, __ver_min) \
	AK_HTTPUtils_DetectIndicatedHeadField((__package), (__len), "HTTP", (__ver_maj), (__ver_min));

/**
 * @macro
 *  Quick call to probe RTSP header field.
 */
#define AK_HTTPUtils_DetectRTSPHeadField(__package, __len, __ver_maj, __ver_min) \
	AK_HTTPUtils_DetectIndicatedHeadField((__package), (__len), "RTSP", (__ver_maj), (__ver_min));




/**
 * @brief
 *  Create a simple packet.
 * @detais
 *  This method expands the @ref Headfield header data structure into a packet and merges the
 *  request content into the user-specified memory space @ref stack.\n
 *  After the data structure is expanded, the @ref Headfield data structure is not released;
 *  the user needs to release it externally.\n
 *  The method internally updates the Content-Type tag in the @ref Headfield data structure.
 *
 * @param Headfield [IN]
 *  Header field data structure.
 * @param content [IN]
 *  Starting address of the send content memory; pass AK_null to indicate no packet content.
 * @param contentlen [IN]
 *  Length of the send content; pass 0 to indicate no packet.
 * @param stack [out]
 *  Starting address of the memory where the packet is returned; retrieve the packet result here after successful packing.
 * @param stacklen [IN]
 *  Available memory size for the returned packet; if the space is insufficient to hold the header and content, packing fails.
 *
 * @return
 *  Returns the packet length on success, returns -1 on failure.
 *
 */
AK_API AK_SSize
AK_HTTPUtils_SimplePacket(AK_HTTPHeadField *Headfield, AK_PByte content, AK_Size contentlen, AK_PByte stack, AK_Size stacklen);

/**
 * @brief
 *  Create a simple packet.
 * @detais
 *  This method expands the @ref Headfield header data structure into a packet and merges the
 *  request content into the user-specified memory space @ref stack.\n
 *  After the data structure is expanded, the @ref Headfield data structure is not released;
 *  the user needs to release it externally.\n
 *  The method internally updates the Content-Type tag in the @ref Headfield data structure.
 *
 * @param ver [IN]
 *  Packet version (e.g., 1.0); pass AK_null to default to version 1.1.
 * @param method [IN]
 *  Request method.
 * @param abspath [IN]
 *  Request absolute path (e.g., http://192.168.0.1:10080/index.html?var=0); internally parses the protocol and address.
 * @param agent [IN]
 *  Proxy name, related to the User-Agent header tag; pass AK_null to not use.
 * @param contenttype [IN]
 *  Send content type, related to the Content-Type header tag; pass AK_null to not use.
 * @param content [IN]
 *  Starting address of the send content memory; pass AK_null to indicate no packet content.
 * @param contentlen [IN]
 *  Length of the send content; pass 0 to indicate no packet.
 * @param stack [out]
 *  Starting address of the memory where the packet is returned; retrieve the packet result here after successful packing.
 * @param stacklen [IN]
 *  Available memory size for the returned packet; if the space is insufficient to hold the header and content, packing fails.
 *
 * @return
 *  Returns the packet length on success, returns -1 on failure.
 *
 */
AK_API AK_SSize
AK_HTTPUtils_SimplePacketRequest(AK_chrptr ver, AK_chrptr method, AK_chrptr abspath, AK_chrptr agent, AK_chrptr contenttype, AK_PByte content, AK_Size contentlen, AK_PByte stack, AK_Size stacklen);


/**
 * @brief
 *  Create a simple packet.
 * @detais
 *  This method expands the @ref Headfield header data structure into a packet and merges the
 *  request content into the user-specified memory space @ref stack.\n
 *  After the data structure is expanded, the @ref Headfield data structure is not released;
 *  the user needs to release it externally.\n
 *  The method internally updates the Content-Type tag in the @ref Headfield data structure.
 *
 * @param ver [IN]
 *  Packet version (e.g., 1.0); pass AK_null to default to version 1.1.
 * @param protocol [IN]
 *  Request protocol; pass AK_null to default to HTTP.
 * @param code [IN]
 *  Response code.
 * @param server [IN]
 *  Server name, related to the Server header tag; pass AK_null to not use.
 * @param contenttype [IN]
 *  Send content type, related to the Content-Type header tag; pass AK_null to not use.
 * @param content [IN]
 *  Starting address of the send content memory; pass AK_null to indicate no packet content.
 * @param contentlen [IN]
 *  Length of the send content; pass 0 to indicate no packet.
 * @param stack [out]
 *  Starting address of the memory where the packet is returned; retrieve the packet result here after successful packing.
 * @param stacklen [IN]
 *  Available memory size for the returned packet; if the space is insufficient to hold the header and content, packing fails.
 *
 * @return
 *  Returns the packet length on success, returns -1 on failure.
 *
 */
AK_API AK_SSize
AK_HTTPUtils_SimplePacketResponse(AK_chrptr ver, AK_chrptr protocol, AK_HTTPCode code, AK_chrptr server, AK_chrptr contenttype, AK_PByte content, AK_Size contentlen, AK_PByte stack, AK_Size stacklen);


#endif



/**
 * Create an HTTPHeadField module handle.
 *
 * @return
 *  Returns the HTTP header field handle on success, otherwise returns AK_null.
 */
AK_API AK_Object akae_http_headfield_create (AK_Object Malloc);

/**
 * @brief
 *  Destroy an HTTPHeadField module handle.
 *
 * @return
 *  Returns the HTTP header field handle on success, otherwise returns AK_null.
 */
AK_API AK_int akae_http_headfield_release (AK_Object Object);
#define akae_http_headfield_release_and_clear(__Object) \
	({\
		AK_int res = akae_http_headfield_release (__Object);\
		if (AK_OK == res) {\
			__Object = AK_null;\
		}\
		res;\
	})


/**
 * Get the memory allocator used by the current module.
 */
AK_API AK_Object akae_http_headfield_get_malloc (AK_Object Object);




/**
 * Set the protocol.
 */
AK_API AK_int akae_http_headfield_setup_protocol (AK_Object Object, AK_chrcptr proto, AK_int version_major, AK_int version_minor);

/**
 * Get the protocol name and version number from the current data structure.
 */
AK_API AK_int akae_http_headfield_protocol (AK_Object Object, AK_chrptr proto, AK_size protolen, AK_int *version_major, AK_int *version_minor);



/**
 * Set as a request data header.
 */
AK_API AK_int akae_http_headfield_setup_request (AK_Object Object, AK_chrcptr method, AK_chrcptr path, AK_chrcptr query);

/**
 * Get the protocol request method from the current data structure.
 */
AK_API AK_chrptr akae_http_headfield_get_request_method (AK_Object Object, AK_chrptr stack, AK_size stacklen);


/**
 * Get the request path from the current data structure; result is stored at the memory location
 * pointed to by @ref stack and returned as a string via the return value.
 *
 * @param[IN] Object
 *  HTTP header field object handle.
 * @param[OUT] stack
 *  Starting address of the result memory.
 * @param[IN] stacklen
 *  Memory size of the result.
 *
 * @return
 *  Returns the result stored at @ref on success, returns AK_null on failure.
 *
 */
AK_API AK_chrptr akae_http_headfield_get_request_path (AK_Object Object, AK_chrptr stack, AK_size stacklen);

/**
 * Set the request path in the current data structure.
 */
AK_API AK_int akae_http_headfield_set_request_path (AK_Object Object, AK_chrcptr path);

/**
 * Set as a response data header.
 */
AK_API AK_int akae_http_headfield_setup_response (AK_Object Object, AK_int code, AK_chrcptr reason_phrase);

AK_API AK_int akae_http_headfield_get_response_result (AK_Object Object);

AK_API AK_int akae_http_headfield_add_query (AK_Object Object, AK_chrcptr opt, AK_chrcptr fmt, ...);


AK_API AK_int akae_http_headfield_drop_query (AK_Object Object, AK_chrcptr opt);


AK_API AK_int akae_http_headfield_locate_query (AK_Object Object, AK_int pos, AK_chrptr *opt, AK_chrptr *data);


AK_API AK_chrcptr akae_http_headfield_indexof_query (AK_Object Object, AK_chrcptr opt, AK_chrcptr def);

AK_API AK_size akae_http_headfield_count_query (AK_Object Object);


/**
 * Add option information to the header field.\n
 * There are two modes of addition: append and update, determined by the @ref append flag.\n
 * Append mode adds data on top of the existing corresponding option, separated by semicolons;\n
 * for example, option: value1 after appending value2 becomes option: value1; value2.\n
 * Update mode replaces the existing option content with the new content.\n
 * For example, option: value1 after updating with value2 becomes option: value2.\n
 * Users can choose the appropriate mode for adding option information based on the characteristics of each option.
 *
 * @param[IN] Object
 *  Module object instance.
 *
 * @param[IN] append
 *  Specifies the addition mode: AK_true means append mode, AK_false means update mode.
 *
 * @param[IN] opt
 *  Option name, such as Content-Length, Content-Type, Server, etc.
 *
 * @param[IN] fmt
 *  Option variable, formatted input.
 *
 * @retval AK_OK
 *  Operation successful; the option value has been updated.
 * @retval AK_ERR_INVAL_OBJECT
 *  Operation failed, possibly due to an invalid handle.
 * @retval AK_ERR_INVAL_PARAM
 *  Operation failed, possibly due to an invalid parameter.
 *
 */
AK_API AK_int akae_http_headfield_add_option (AK_Object Object, AK_boolean append, AK_chrcptr opt, AK_chrcptr fmt, ...);

/**
 * See @ref akae_http_headfield_add_option().\n
 * Quick macro call for update mode.
 */
#define akae_http_headfield_update_option(__Object, __opt, __fmt...) \
		akae_http_headfield_add_option(__Object, AK_false, __opt, ##__fmt)

/**
 * See @ref akae_http_headfield_add_option().\n
 * Quick macro call for append mode.
 */
#define akae_http_headfield_append_option(__Object, __opt, __fmt...) \
		akae_http_headfield_add_option(__Object, AK_true, __opt, ##__fmt)


/**
 * Remove option information.\n
 * Note: if the @ref opt option passed in does not exist, this interface still returns success.
 *
 * @param[IN] Object
 *  Module object instance.
 *
 * @param[IN] opt
 *  Option name, such as Content-Length, Content-Type, Server, etc.
 *
 * @retval AK_OK
 *  Operation successful; the option value has been updated.
 * @retval AK_ERR_INVAL_OBJECT
 *  Operation failed, possibly due to an invalid handle.
 * @retval AK_ERR_INVAL_PARAM
 *  Operation failed, possibly due to an invalid parameter.
 *
 */
AK_API AK_int akae_http_headfield_drop_option (AK_Object Object, AK_chrptr opt);

/**
 * Index option information.
 *
 * @param[IN] Object
 *  Module object instance.
 *
 * @param[IN] opt
 *  Option name, such as Content-Length, Content-Type, Server, etc.
 *
 * @param[IN] def
 *  Default value; returned when the option does not exist.
 *
 * @return
 *  Returns the option value; if the option does not exist returns @ref def;
 *  if an invalid parameter is passed returns AK_null.
 *
 */
AK_API AK_chrptr akae_http_headfield_indexof_option (AK_Object Object, AK_chrcptr opt, AK_chrcptr def);


/**
 * Index option information by insertion order. Generally used to traverse all option information.\n
 * After successful indexing, results are returned via @ref opt and @data parameters.
 *
 * @param[IN] Object
 *  Module object instance.
 *
 * @param[IN] pos
 *  Option insertion order, starting from 0; the first inserted option is 0, the second is 1, and so on.\n
 *  Negative numbers may be passed for reverse indexing; for example, -1 retrieves the last one.
 *
 * @param[OUT] opt
 *  Option name return; after successful indexing this pointer points to the option name.
 *
 * @param[OUT] data
 *  Option value; after successful indexing this pointer points to the option value.
 *
 * @retval AK_OK
 *  Operation successful; results can be obtained via @ref opt and @ref data.
 * @retval AK_ERR_INVAL_OBJECT
 *  Operation failed, possibly due to an invalid handle.
 * @retval AK_ERR_INVAL_PARAM
 *  Operation failed, possibly due to an invalid parameter.
 * @retval AK_ERR_OUT_OF_RANGE
 *  Operation failed; the @ref pos position is out of the retrievable option range.
 *
 */
AK_API AK_int akae_http_headfield_locate_option (AK_Object Object, AK_int pos, AK_chrptr *opt, AK_chrptr *data);

/**
 * Get the number of options.
 *
 * @return
 *  Returns the number of options. Note: if an invalid handle is passed, this interface returns 0.
 */
AK_API AK_size akae_http_headfield_count_option (AK_Object Object);


/**
 * Serialize the header field data. Serialize the header field data of object @ref Object to a string.\n
 * Output is written to the memory location of @ref stack, with length not exceeding @ref stacklen.
 *
 * @return
 *  Returns the valid string length on success; result is stored at the memory location of @ref stack.
 *
 * @retval AK_ERR_INVAL_OBJECT
 *  Operation failed, possibly due to an invalid handle.
 * @retval AK_ERR_INVAL_PARAM
 *
 */
AK_API AK_ssize akae_http_headfield_stringify (AK_Object Object, AK_chrptr stack, AK_size stacklen);

/**
 * Scan the string for an HTTP-like packet and return the length of the header field via the interface,\n
 * returning the protocol name and version number via parameters.
 *
 * @param[IN] str
 * 	Starting memory location of the string.
 * @param[IN] len
 * 	Length of string @ref str.
 * @param[OUT] proto
 * 	Protocol name; returned via this parameter when a packet is found in the scan.
 * @param[IN] protolen
 * 	Maximum length of the protocol name memory; if the detected protocol exceeds this value,
 * 	the result will be truncated to the maximum length.
 * @param[OUT] version_major
 * 	Protocol major version number; returned via this parameter when a packet is found in the scan.
 * @param[OUT] version_minor
 * 	Protocol minor version number; returned via this parameter when a packet is found in the scan.
 *
 * @return
 *  When an HTTP-like packet is found in the scan, returns the packet length and returns the
 *  protocol name and version number via parameters.\n
 *  Returns the corresponding result on scan failure.
 *
 * @retval AK_ERR_INVAL_OBJECT
 *  Scan failed, an invalid object was passed.
 * @retval AK_ERR_INVAL_PARAM
 *  Scan failed, an invalid parameter was passed.
 * @retval AK_ERR_NOT_EXIST
 *  Scan failed, no HTTP-like packet found in the string.
 *
 */
AK_API AK_ssize akae_http_headfield_scan_string (AK_chrcptr str, AK_size len,
		AK_chrptr proto, AK_size protolen, AK_int *version_major, AK_int *version_minor);


/**
 * Attempt to detect an HTTP-like packet for the specified protocol in the string,\n
 * internally calls @ref akae_http_headfield_scan_string() for scanning.
 *
 * @param[IN] str
 * 	Starting memory location of the string.
 * @param[IN] len
 * 	Length of string @ref str.
 * @param[IN] proto
 * 	Specified protocol name; pass AK_null to ignore protocol matching.
 *
 * @return
 *  Returns AK_true if a protocol matching @ref proto is detected in the string;
 *  returns AK_false for no match and other abnormal cases.
 */
AK_API AK_boolean akae_http_headfield_probe_string (AK_chrcptr str, AK_size len, AK_chrcptr proto);



/**
 * Parse HTTP-like header data from a string and create an object instance.\n
 * Internally calls @ref akae_http_headfield_scan_string() and @ref akae_http_headfield_create()
 * to create an object instance.\n
 * The created object is released via @ref akae_http_headfield_release().
 * During parsing, an attempt is made to match the @ref protocol name; if AK_null is passed the
 * protocol is not specified for parsing,\n
 * and if the protocol name does not match, the interface returns failure.
 *
 * @param[IN] Malloc
 *  Allocator used for allocating object instance memory.
 *
 * @param[IN] str
 *  Starting address of the string memory.
 * @param[IN] len
 *  String length.
 * @param[IN] proto
 *  Specified protocol name to parse; returns failure if specified protocol does not match;
 *  protocol name is case-insensitive.
 * @param[OUT] fieldlen
 *  Returns the actual header field length on successful instance creation; passing AK_null does not
 *  affect the result but the user cannot obtain the actual header field length.
 *
 * @return
 *  Returns an object instance if the HTTP-like header in the string is successfully parsed.
 *
 */
AK_API AK_Object akae_http_headfield_parse_string (AK_Object Malloc, AK_chrcptr str, AK_size len, AK_chrcptr proto, AK_size *fieldlen);

/**
 * @macro
 *  Add a Connection option tag.
 *
 * @param __Object
 *  Header field module object instance.
 * @param __sec
 *  Pass 0 to not use Keep-Alive, i.e., adds Connection: close.\n
 *  Pass the corresponding number of seconds to keep the connection with @ref __sec timeout.
 * @param __max
 *  Maximum number of keep-alive connections; only valid when @ref __sec is greater than 0.
 */
#define akae_http_headfield_set_connection(__Object, __sec, __max) \
	do {\
		if (__sec > 0) {\
			akae_http_headfield_add_option ((__Object), AK_false, "Connection", "keep-alive");\
			/*akae_http_headfield_add_option ((__Object), AK_false, "Keep-Alive", "timeout=%u, max=%u", __sec, __max);*/\
		} else {\
			akae_http_headfield_add_option ((__Object), AK_false, "Connection", "close");\
			akae_http_headfield_drop_option ((__Object), "Keep-Alive");\
		}\
	} while(0)

/**
 * @macro
 *  Add a Content-Length option tag.
 */
#define akae_http_headfield_set_content_length(__Object, __len) \
	akae_http_headfield_add_option ((__Object), AK_false, "Content-Length", "%u", (__len))


/**
 * @macro
 *  Get the Content-Length option tag.
 */
#define akae_http_headfield_get_content_length(__Object) \
	akae_atoi(akae_http_headfield_indexof_option ((__Object), "Content-Length", "0"))


/**
 * @macro
 *  Add a Content-Type option tag.
 */
#define akae_http_headfield_set_content_type(__Object, __type) \
	akae_http_headfield_add_option ((__Object), AK_false, "Content-Type", "%s", (__type))


/**
 * @macro
 *  Get the Content-Type option tag.
 */
#define akae_http_headfield_get_content_type(__Object) \
	akae_http_headfield_indexof_option ((__Object), "Content-Type", akae_http_file_mime (AK_null))

/**
 * @macro
 *  Add a Host option tag.
 */
#define akae_http_headfield_set_host(__Object, __host) \
	akae_http_headfield_add_option ((__Object), AK_false, "Host", "%s", __host)


/**
 * @macro
 *  Add a Server option tag.
 */
#define akae_http_headfield_set_server(__Object, __server) \
	do {\
		if (AK_null != (__server)) {\
			akae_http_headfield_add_option((__Object), AK_false, "Server", (__server));\
		} else {\
			akae_http_headfield_drop_option ((__Object), "Server");\
		}\
	} while(0)


/**
 * @macro
 *  Add a Content-Encoding option tag to the header field.
 */
#define akae_http_headfield_set_content_encoding(__Object, __encoding) \
	do {\
		if (AK_null != (__encoding)) {\
			akae_http_headfield_add_option((__Object), AK_false, "Content-Encodeing", "%s", (__encoding));\
		} else {\
			akae_http_headfield_drop_option ((__Object), "Content-Encodeing");\
		}\
	} while(0)


/**
 * @macro
 *  Add a Content-Disposition option tag to the header field, redefining the download filename for the peer.
 */
#define akae_http_headfield_set_content_disposition(__Object, __filename) \
	do {\
		if (AK_null != (__filename)) {\
			akae_http_headfield_add_option((__Object), AK_false, "Content-Disposition", "inline;filename=%s", (__filename));\
		} else {\
			akae_http_headfield_drop_option ((__Object), "Content-Disposition");\
		}\
	} while(0)



AK_C_HEADER_EXTERN_C_END
#endif
