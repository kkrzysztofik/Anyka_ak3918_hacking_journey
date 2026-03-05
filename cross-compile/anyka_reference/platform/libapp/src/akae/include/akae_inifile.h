


#include <akae_object.h>

#if !defined(AKSPC_INIFILE_H_)
#define AKSPC_INIFILE_H_
AK_C_HEADER_EXTERN_C_BEGIN

/**
 * Maximum section text length.
 */
#define AK_INIFL_MAX_SECTION_LEN     (64)

/**
 * Maximum key text length.
 */
#define AK_INIFL_MAX_KEY_LEN         (128)

/**
 * Maximum value text length.
 */
#define AK_INIFL_MAX_VALUE_LEN       (128)


/**
 * Create an ini module object.\n
 * Once successfully created, it can be operated on via read and write calls.
 *
 * @param[IN] Malloc
 *  Memory allocator; the module uses it internally for resource allocation.
 *
 * @param[IN] threadsafe
 *  Thread-safety flag. When set, thread safety is enabled and the internal interface
 *  uses mutually exclusive reads and writes.\n
 *  Enabling thread safety incurs additional performance overhead.
 *  If the caller can guarantee this module is only called from a single thread,
 *  thread safety can be left disabled.
 *
 * @return
 *  Returns a module handle on success, AK_null on failure.
 *
 */
AK_API AK_Object akae_inifile_create (AK_Object Malloc,
		AK_boolean threadsafe);

/**
 * Parse ini text and simultaneously create an ini module handle.\n
 * Internally calls @ref akae_inifile_create().
 *
 */
AK_API AK_Object akae_inifile_parse (AK_Object Malloc,
		AK_boolean threadsafe,
		AK_chrptr initext);

/**
 * Release an ini module object.
 */
AK_API AK_int akae_inifile_release (AK_Object Object);


/**
 * Read a string from the object.\n
 * If the specified section and key are not found, the default value is returned
 * to ensure the caller always receives an ordered result.
 *
 * @param[IN] Object
 *  ini module object.
 *
 * @param[IN] section
 *  ini section; used together with the key to index the value.
 *  Passing AK_null means indexing a key-value pair with no section.
 *
 * @param[IN] key
 *  ini key; used together with the section to index the value.
 *
 * @param[IN] def
 *  Default value; returned when the specified section and key do not exist.
 *
 * @return
 *  Returns the value at the specified section and key.
 *
 */
AK_API AK_chrptr akae_inifile_read_string (AK_Object Object,
		AK_chrptr section,
		AK_chrptr key,
		AK_chrptr def);

/**
 * Read an integer from the object.\n
 * Internally calls @ref akae_inifile_read_string().
 */
AK_API AK_int akae_inifile_read_integer (AK_Object Object,
		AK_chrptr section,
		AK_chrptr key,
		AK_int def);

/**
 * Write a string to the object.
 *
 * @param[IN] Object
 *  ini module object.
 *
 * @param[IN] section
 *  ini section; used together with the key to index the value.
 *  Passing AK_null means indexing a key-value pair with no section.
 *
 * @param[IN] key
 *  ini key; used together with the section to index the value.
 *
 * @param[IN] val
 *  Value to write.
 *
 */
AK_API AK_int akae_inifile_write_string (AK_Object Object,
		AK_chrptr section,
		AK_chrptr key,
		AK_chrptr val);


/**
 * Write an integer to the object.\n
 * Internally calls @ref akae_inifile_write_string().
 */
AK_API AK_int akae_inifile_write_integer (AK_Object Object,
		AK_chrptr section,
		AK_chrptr key,
		AK_int val);

/**
 * Remove a section.
 */
AK_API AK_int akae_inifile_remove_section (AK_Object Object, AK_chrptr section);


/**
 * Remove data at the specified location.
 * Note: even if no data exists at the specified location, this method still returns success.
 */
AK_API AK_int akae_inifile_remove_key (AK_Object Object,
		AK_chrptr section,
		AK_chrptr key);


/**
 * Serialize the ini object and return the result in memory.
 *
 * @param[IN] pretty
 *  Pretty-print flag. When set, extra spaces are added during serialization for alignment,
 *  which increases the memory used by the result.\n
 *
 * @param[OUT] stack
 *  Start address of the memory buffer for the serialized result.
 *
 * @param[IN] stacklen
 *  Maximum length of the memory buffer for the serialized result.
 *
 * @return
 *  Returns the length of the serialized result on success, or the corresponding error code on failure.
 *
 * @retval AK_ERR_INVAL_OPERATE
 *  Operation failed; an invalid object was passed.
 * @retval AK_ERR_INVAL_PARAM
 *  Operation failed; invalid parameters were passed, possibly @ref stack or @ref stacklen is invalid.
 *
 *
 */
AK_API AK_ssize akae_inifile_stringify (AK_Object Object,
		AK_boolean pretty,
		AK_chrptr stack,
		AK_size stacklen);



AK_C_HEADER_EXTERN_C_END
#endif ///< AKSPC_INIFILE_H_
