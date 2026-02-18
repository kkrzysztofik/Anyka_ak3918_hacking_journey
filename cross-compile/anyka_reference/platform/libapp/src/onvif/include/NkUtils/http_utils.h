/**
 * HTTP protocol utility logic layer.
 *
 */

#include <NkUtils/types.h>
#include <NkUtils/allocator.h>
//#include <NkUtils/str_list.h>
#include <NkUtils/assert.h>

#ifndef NK_HTTP_UTILS_H_
# define NK_HTTP_UTILS_H_
NK_CPP_EXTERN_BEGIN


/**
 * HTTP line terminator definition.
 */
#define NK_HTTP_CRLF "\r\n"

/**
 * HTTP response code definitions.
 */
typedef enum Nk_HTTPCode
{
	NK_HTTP_CODE_CONTINUE = (100),                       //!< NK_HTTP_CODE_CONTINUE
	NK_HTTP_CODE_SWITCHING_PROTOCOLS = (101),            //!< NK_HTTP_CODE_SWITCHING_PROTOCOLS
	NK_HTTP_CODE_OK = (200),                             //!< NK_HTTP_CODE_OK
	NK_HTTP_CODE_CREATED = (201),                        //!< NK_HTTP_CODE_CREATED
	NK_HTTP_CODE_ACCEPTED = (202),                       //!< NK_HTTP_CODE_ACCEPTED
	NK_HTTP_CODE_NON_AUTHORITATIVE_INFOMATION = (203),   //!< NK_HTTP_CODE_NON_AUTHORITATIVE_INFOMATION
	NK_HTTP_CODE_NO_CONTENT = (204),                     //!< NK_HTTP_CODE_NO_CONTENT
	NK_HTTP_CODE_RESET_CONTENT = (205),                  //!< NK_HTTP_CODE_RESET_CONTENT
	NK_HTTP_CODE_PARTIAL_CONTENT = (206),                //!< NK_HTTP_CODE_PARTIAL_CONTENT
	NK_HTTP_CODE_MULTIPLE_CHOICES = (300),               //!< NK_HTTP_CODE_MULTIPLE_CHOICES
	NK_HTTP_CODE_MOVED_PERMANENTLY = (301),              //!< NK_HTTP_CODE_MOVED_PERMANENTLY
	NK_HTTP_CODE_FOUND = (302),                          //!< NK_HTTP_CODE_FOUND
	NK_HTTP_CODE_SEE_OTHER = (303),                      //!< NK_HTTP_CODE_SEE_OTHER
	NK_HTTP_CODE_NOT_MODIFIED = (304),                   //!< NK_HTTP_CODE_NOT_MODIFIED
	NK_HTTP_CODE_USE_PROXY = (305),                      //!< NK_HTTP_CODE_USE_PROXY
	NK_HTTP_CODE_TEMPORARY_REDIRECT = (307),             //!< NK_HTTP_CODE_TEMPORARY_REDIRECT
	NK_HTTP_CODE_BAD_REQUEST = (400),                    //!< NK_HTTP_CODE_BAD_REQUEST
	NK_HTTP_CODE_UNAUTHORIZED = (401),                   //!< NK_HTTP_CODE_UNAUTHORIZED
	NK_HTTP_CODE_PAYMENT_REQUIRED = (402),               //!< NK_HTTP_CODE_PAYMENT_REQUIRED
	NK_HTTP_CODE_FORBIDDEN = (403),                      //!< NK_HTTP_CODE_FORBIDDEN
	NK_HTTP_CODE_NOT_FOUND = (404),                      //!< NK_HTTP_CODE_NOT_FOUND
	NK_HTTP_CODE_METHOD_NOT_ALLOWED = (405),             //!< NK_HTTP_CODE_METHOD_NOT_ALLOWED
	NK_HTTP_CODE_NOT_ACCEPTABLE = (406),                 //!< NK_HTTP_CODE_NOT_ACCEPTABLE
	NK_HTTP_CODE_PROXY_AUTHENTICATION_REQUIRED = (407),  //!< NK_HTTP_CODE_PROXY_AUTHENTICATION_REQUIRED
	NK_HTTP_CODE_REQUEST_TIME_OUT = (408),               //!< NK_HTTP_CODE_REQUEST_TIME_OUT
	NK_HTTP_CODE_CONFLICT = (409),                       //!< NK_HTTP_CODE_CONFLICT
	NK_HTTP_CODE_GONE = (410),                           //!< NK_HTTP_CODE_GONE
	NK_HTTP_CODE_LENGTH_REQUIRED = (411),                //!< NK_HTTP_CODE_LENGTH_REQUIRED
	NK_HTTP_CODE_PRECONDITION_FAILED = (412),            //!< NK_HTTP_CODE_PRECONDITION_FAILED
	NK_HTTP_CODE_REQUEST_ENTITY_TOO_LARGE = (413),       //!< NK_HTTP_CODE_REQUEST_ENTITY_TOO_LARGE
	NK_HTTP_CODE_REQUEST_URI_TOO_LARGE = (414),          //!< NK_HTTP_CODE_REQUEST_URI_TOO_LARGE
	NK_HTTP_CODE_UNSUPPORTED_MEDIA_TYPE = (415),         //!< NK_HTTP_CODE_UNSUPPORTED_MEDIA_TYPE
	NK_HTTP_CODE_REQUESTED_RANGE_NOT_SATISFIABLE = (416),//!< NK_HTTP_CODE_REQUESTED_RANGE_NOT_SATISFIABLE
	NK_HTTP_CODE_EXPECTATION_FAILED = (417),             //!< NK_HTTP_CODE_EXPECTATION_FAILED
	NK_HTTP_CODE_INTERNAL_SERVER_ERROR = (500),          //!< NK_HTTP_CODE_INTERNAL_SERVER_ERROR
	NK_HTTP_CODE_NOT_IMPLEMENTED = (501),                //!< NK_HTTP_CODE_NOT_IMPLEMENTED
	NK_HTTP_CODE_BAD_GATEWAY = (502),                    //!< NK_HTTP_CODE_BAD_GATEWAY
	NK_HTTP_CODE_SERVICE_UNAVAILABLE = (503),            //!< NK_HTTP_CODE_SERVICE_UNAVAILABLE
	NK_HTTP_CODE_GATEWAY_TIME_OUT = (504),               //!< NK_HTTP_CODE_GATEWAY_TIME_OUT
	NK_HTTP_CODE_HTTP_VERSION_NOT_SUPPORTED = (505),     //!< NK_HTTP_CODE_HTTP_VERSION_NOT_SUPPORTED

} NK_HTTPCode;


/**
 * Get file MIME type by file name.\n
 * The interface gets MIME type from file extension; the file name must contain an extension, e.g. file.txt.
 *
 */
extern NK_PChar
NK_HTTPUtils_FileMIME(NK_PChar file_name);


/**
 * Get the default message for an HTTP response.
 *
 * @param[in]		code		HTTP response code.
 *
 * @return	The default response message string.
 */
extern NK_PChar
NK_HTTPUtils_ReasonPhrase(NK_UInt32 code);

/**
 * Encode a URI string.\n
 * Supports UTF-8 encoding conversion.
 *
 * @param[in]			uri				URI text to encode.
 * @param[out]			enc				Buffer for the encoded URI text; retrieve the encoded URI text from here on success.
 * @param[in,out]		enc_len			Length of the encoded URI output buffer; on success, returns the length of the encoded URI text from here.
 *
 * @return		Returns 0 on success, -1 on failure.
 */
extern NK_Int
NK_HTTPUtils_EncodeURI(NK_PChar uri, NK_PChar enc, NK_Size *enc_len);

/**
 * Decode a URI string.\n
 * Supports UTF-8 encoding conversion.\n
 * Note that @ref enc and @ref uri can point to the same memory block.
 *
 * @param[in]			enc				Encoded URI text.
 * @param[out]			uri				Buffer for the decoded URI text; retrieve the URI text from here on success.
 * @param[in,out]		uri_len			Length of the decoded URI output buffer; on success, returns the length of the URI text from here.
 *
 * @return		Returns 0 on success, -1 on failure.
 */
extern NK_Int
NK_HTTPUtils_DecodeURI(NK_PChar enc, NK_PChar uri, NK_Size *uri_len);

/**
 * URL data structure.
 */
typedef struct Nk_HTTPURL
{
	NK_PChar protocol;
	NK_PChar host;
	NK_UInt16 port;
	NK_PChar abs_path;
	NK_PChar query;

	/**
	 * Reserved field.
	 */
	NK_Byte reserved[1024 * 5];
} NK_HTTPURL;

/**
 * Print the @ref NK_HTTPURL data structure.
 */
#define NK_HTTP_URL_DUMP(__URL) \
	do{\
		NK_TermTable Tbl;\
		NK_TermTbl_BeginDraw(&Tbl, "URL", 96, 4);\
		NK_TermTbl_PutKeyValue(&Tbl, NK_True, "Protocol", "%s", (__URL)->protocol);\
		NK_TermTbl_PutKeyValue(&Tbl, NK_True, "Host", "%s", (__URL)->host);\
		NK_TermTbl_PutKeyValue(&Tbl, NK_True, "Port", "%d", (NK_Int)((__URL)->port));\
		NK_TermTbl_PutKeyValue(&Tbl, NK_True, "Absolute Path", "%s", (__URL)->abs_path);\
		if (NK_Nil != (__URL)->query) {\
			NK_TermTbl_PutKeyValue(&Tbl, NK_True, "Query String", "%s", (__URL)->query);\
		}\
		NK_TermTbl_EndDraw(&Tbl);\
	} while(0)


/**
 * Parse a URL string.
 */
extern NK_Int
NK_HTTPUtils_ParseURL(NK_PChar url, NK_HTTPURL *URL);


/**
 * HTTPHeadField module object.
 */
typedef struct Nk_HTTPHeadField {
#define THIS struct Nk_HTTPHeadField *const

	/**
	 * Module object interface.
	 */
	NK_Object Object;

	/**
	 * Request/response identifier.
	 */
	NK_Boolean isRequest;

	/**
	 * Protocol.
	 */
	NK_PChar protocol;

	/**
	 * Version number.
	 */
	NK_UInt32 ver_maj, ver_min;

	/**
	 * Valid when isRequest is True.
	 */
	struct {
		NK_PChar method;
		NK_PChar abs_path;
		NK_PChar query;
	};
	/**
	 * Valid when isRequest is False.
	 */
	struct {
		NK_UInt32 code;
		NK_PChar reason_phrase;
	};

	/**
	 * Set the protocol.
	 */
	NK_Int
	(*setProtocol)(THIS, NK_PChar protocol, NK_Int ver_maj, NK_Int ver_min);

	/**
	 * Set as a request header.
	 */
	NK_Int
	(*setRequest)(THIS, NK_PChar method, NK_PChar abs_path, NK_PChar query);

	/**
	 * Set as a response header.
	 */
	NK_Int
	(*setResponse)(THIS, NK_UInt32 status_code, NK_PChar reason_phrase);

	/**
	 * Add a header query item.\n
	 * Only valid in request mode; otherwise returns -1.
	 *
	 */
	NK_Int
	(*addQuery)(THIS, NK_PChar key, NK_PChar fmt, ...);

	/**
	 * Remove a header query item.\n
	 * Only valid in request mode; otherwise returns -1.
	 *
	 * @return		Returns 0 if the key was not found or deletion succeeded; returns -1 on other errors.
	 */
	NK_Int
	(*dropQuery)(THIS, NK_PChar key);

	/**
	 * Get the number of Query items.
	 */
	NK_Size
	(*numberOfQuery)(THIS);

	/**
	 * Get Query information by index.
	 *
	 * @param[in]			id				Query item index number.
	 * @param[out]			key				Key corresponding to the Query item.
	 * @param[out]			value			Value corresponding to the Query item key.
	 *
	 * @return		Returns 0 on success, otherwise returns -1.
	 */
	NK_Int
	(*indexOfQuery)(THIS, NK_Int id, NK_PChar *key, NK_PChar *value);

	/**
	 * Get header query information.\n
	 * Only valid in request mode; otherwise returns -1.
	 *
	 */
	NK_PChar
	(*getQuery)(THIS, NK_PChar key, NK_PChar def);

	/**
	 * Check whether a header has a certain query item.
	 *
	 */
	NK_Boolean
	(*hasQuery)(THIS, NK_PChar key);

	/**
	 * Add a header option item.
	 *
	 */
	NK_Int
	(*addOption)(THIS, NK_Boolean overwrite, NK_PChar opt, NK_PChar fmt, ...);

	/**
	 * Remove a header option item.
	 *
	 * @return		Returns 0 if the key was not found or deletion succeeded; returns -1 on other errors.
	 */
	NK_Int
	(*dropOption)(THIS, NK_Boolean all, NK_PChar opt);

	/**
	 * Get the number of option tags.
	 *
	 * @return		Count of option tag selections.
	 */
	NK_Size
	(*numberOfOption)(THIS);

	/**
	 * Get the tag name and information by tag index.
	 *
	 * @param[in]			id				Option tag index number.
	 * @param[out]			key				Tag name corresponding to the index.
	 * @param[out]			value			Tag information corresponding to the index.
	 *
	 * @return		Returns 0 on success; retrieve the option tag content from @ref opt and @ref value.
	 */
	NK_Int
	(*indexOfOption)(THIS, NK_Int id, NK_PChar *key, NK_PChar *value);


	/**
	 * Get information by option tag name.
	 *
	 * @param[in]			key				Option tag name.
	 * @param[in]			def				Default option tag value; used as the return value when @ref opt does not exist; can be Nil.
	 *
	 * @return		Returns the option tag value; when the tag does not exist, returns the @ref def default value.
	 */
	NK_PChar
	(*getOption)(THIS, NK_PChar key, NK_PChar def);

	/**
	 * Check whether a certain option in the header exists.
	 *
	 */
	NK_SSize
	(*hasOption)(THIS, NK_PChar opt);

	/**
	 * Serialize to text content.
	 *
	 */
	NK_Int
	(*toText)(THIS, NK_PChar text, NK_Size *text_len);

#undef THIS
} NK_HTTPHeadField;

/**
 * Print the NK_HTTPHeadField data structure.
 */
#define NK_HTTP_HEAD_FIELD_DUMP(__HeadField) \
	do{\
		NK_TermTable Table;\
		NK_Size number = 0;\
		NK_PChar key = NK_Nil;\
		NK_PChar value = NK_Nil;\
		NK_Int i = 0;\
		\
		NK_CHECK_POINT();\
		NK_TermTbl_BeginDraw(&Table, "HTTP Head Filed", 96, 4);\
		NK_TermTbl_PutKeyValue(&Table, NK_True, "Protocol", "%s %u.%u",\
				(__HeadField)->protocol, (__HeadField)->ver_maj, (__HeadField)->ver_min);\
		if ((__HeadField)->isRequest) {\
			NK_TermTbl_PutKeyValue(&Table, NK_False, "Method", "%s", (__HeadField)->method);\
			NK_TermTbl_PutKeyValue(&Table, NK_True, "Absolute Path", "%s", (__HeadField)->abs_path);\
			number = (__HeadField)->numberOfQuery((__HeadField));\
			NK_TermTbl_PutText(&Table, NK_True, "Query String (%u)", number);\
			for (i = 0; i < number; ++i) {\
				if (0 == (__HeadField)->indexOfQuery((__HeadField), i, &key, &value)) {\
					NK_TermTbl_PutKeyValue(&Table, (i == number - 1), key, "%s", value);\
				}\
			}\
		} else {\
			NK_TermTbl_PutKeyValue(&Table, NK_False, "Code", "%u", (__HeadField)->code);\
			NK_TermTbl_PutKeyValue(&Table, NK_True, "Reason Phrase", "%s", (__HeadField)->reason_phrase);\
		}\
		\
		number = (__HeadField)->numberOfOption((__HeadField));\
		NK_TermTbl_PutText(&Table, NK_True, "Option Tag (%u)", number);\
		for (i = 0; i < number; ++i) {\
			if (0 == (__HeadField)->indexOfOption((__HeadField), i, &key, &value)) {\
				NK_TermTbl_PutKeyValue(&Table, (i == number - 1), key, "%s", value);\
			}\
		}\
		NK_TermTbl_EndDraw(&Table);\
	} while(0)



/**
 * Set the Host option tag.
 */
#define NK_HTTP_HEAD_FIELD_HOST(__HeadField, __str_host) \
	do {\
		if (NK_Nil != (__HeadField)) {\
			(__HeadField)->dropOption((__HeadField), NK_True, "Host");\
			(__HeadField)->addOption((__HeadField), NK_False, "Host", "%s", __str_host);\
		}\
	} while(0)

/**
 * Set the Server option tag.
 */
#define NK_HTTP_HEAD_FIELD_SERVER(__HeadField, __str_server, __uint_ver1, __uint_ver2) \
	do {\
		if (NK_Nil != (__HeadField)) {\
			(__HeadField)->dropOption((__HeadField), NK_True, "Server");\
			(__HeadField)->addOption((__HeadField), NK_False, "Server", "%s / %u.%u", (__str_server), (__uint_ver1), (__uint_ver2));\
		}\
	} while(0)

/**
 * Set the Content-Type option tag.
 */
#define NK_HTTP_HEAD_FIELD_CONTENT_TYPE(__HeadField, __str_type) \
	do {\
		if (NK_Nil != (__HeadField)) {\
			(__HeadField)->dropOption((__HeadField), NK_True, "Content-Type");\
			(__HeadField)->addOption((__HeadField), NK_False, "Content-Type", "%s", (__str_type));\
		}\
	} while(0)

/**
 * Set the Content-Length option tag.
 */
#define NK_HTTP_HEAD_FIELD_CONTENT_LENGTH(__HeadField, __uint_len) \
	do {\
		if (NK_Nil != (__HeadField)) {\
			(__HeadField)->dropOption((__HeadField), NK_True, "Content-Length");\
			(__HeadField)->addOption((__HeadField), NK_False, "Content-Length", "%u", (__uint_len));\
		}\
	} while(0)

/**
 * Set the Connection option tag.
 */
#define NK_HTTP_HEAD_FIELD_CONNECTION(__HeadField, __uint_alive_s) \
	do {\
		if (NK_Nil != (__HeadField)) {\
			(__HeadField)->dropOption((__HeadField), NK_True, "Connection");\
			(__HeadField)->dropOption((__HeadField), NK_True, "Keep-Alive");\
			(__HeadField)->addOption((__HeadField), NK_False, "Connection", "%s", (__uint_alive_s > 0) ? "keep-alive" : "close");\
			if (__uint_alive_s > 0) {\
				NK_UInt32 timeout = __uint_alive_s;\
				timeout = timeout < 60U ? timeout : 60U;\
				(__HeadField)->addOption((__HeadField), NK_False, "Keep-Alive", "timeout=%u, max=%u", timeout, 60U);\
			}\
		}\
	} while(0)



/**
 * Get the size of an HTTP header contained in the package.\n
 *
 * @param[in]				protocol			Protocol name; when Nil, defaults to HTTP.
 * @param[in]				package				Received data package.
 * @param[in,out]			pack_size			Input: length of @package data package; output: returns the size of the parsed header.
 *
 * @return		If the package contains an HTTP header, returns the length of the HTTP header; if no HTTP header is found in the package, returns -1.
 */
extern NK_Int
NK_HTTPUtils_ExtractHeadField(NK_PChar protocol, NK_PChar package, NK_Size *pack_size);

/**
 * Parse an HTTP header data structure from the package buffer and extract the content.\n
 * Internally calls @ref NK_HTTPUtils_ExtractHeadField() and @ref NK_HTTPUtils_CreateHeadField().\n
 * When parsing, it will attempt to match the @ref protocol name; if the protocol name is Nil, defaults to HTTP.\n
 * If the protocol name does not match, the interface will return failure.
 *
 */
extern NK_HTTPHeadField *
NK_HTTPUtils_ParseHeadField(NK_Allocator *Alloctr, NK_PChar protocol, NK_PChar package, NK_Size *len);

/**
 * Create an HTTPHeadField module object.
 */
extern NK_HTTPHeadField *
NK_HTTPUtils_CreateHeadField(NK_Allocator *Alloctr, NK_PChar protocol, NK_UInt32 ver_maj, NK_UInt32 ver_min);

/**
 * Destroy an HTTPHeadField module object.
 */
extern NK_Int
NK_HTTPUtils_FreeHeadField(NK_HTTPHeadField **Field_r);



NK_CPP_EXTERN_END
#endif /* NK_HTTP_UTILS_H_ */

