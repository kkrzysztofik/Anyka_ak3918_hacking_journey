/**
 * JSON operations module.
 */

#include <akae_object.h>

#if !defined(AKSPC_JSON_H_)
#define AKSPC_JSON_H_
AK_C_HEADER_EXTERN_C_BEGIN

/**
 * Maximum JSON path text length.
 */
#define AK_JSON_PATH_MAX_LEN     (128)

/**
 * JSON object type definitions.
 */
#define AK_JSON_TYPE_UNDEF       (0) ///< Undefined type.
#define AK_JSON_TYPE_NULL        (1) ///< Null type.
#define AK_JSON_TYPE_BOOLEAN     (2) ///< Boolean type.
#define AK_JSON_TYPE_NUMBER      (3) ///< Number type.
#define AK_JSON_TYPE_STRING      (4) ///< String type.
#define AK_JSON_TYPE_ARRAY       (5) ///< Array type.
#define AK_JSON_TYPE_OBJECT      (6) ///< Object/struct type.

/**
 * Parse JSON text into a data structure.
 *
 * @param[IN] Mallctr
 *  Memory allocator; required for allocating memory when creating the JSON object.
 *
 * @param[IN] text
 *  Text to parse.
 *
 * @return
 *  On success, creates and returns a JSON object handle; returns AK_null on failure,\n
 *  which may indicate invalid parameters or a parse failure on the @ref text content.
 *
 */
AK_API AK_Object akae_json_parse (AK_Object Mallctr, AK_chrcptr text);


/**
 * Duplicate a JSON data object.\n
 * Produces a new object handle with the same content as the original.\n
 * The two objects are independent and do not reference each other.
 *
 * @param[IN] Mallctr
 *  Module memory allocator.
 *
 * @param[IN] Object
 *  JSON data object.
 *
 * @return
 *  Returns the newly created JSON object on success;\n
 *  returns AK_null on failure, which may indicate a parameter error.
 */
AK_API AK_Object akae_json_duplicate (AK_Object Mallctr, AK_Object Object);

/**
 * Release a JSON data object.
 *
 * @param[IN] Object
 *  JSON data object.
 *
 * @retval AK_OK
 *  Release succeeded.
 * @retval AK_ERR_INVAL_OBJECT
 *  Release failed; the object is invalid.
 */
AK_API AK_int akae_json_release (AK_Object Object);

/**
 * Get the memory allocator used by the JSON data object.\n
 * The allocator is passed in when the JSON data object is created.\n
 * This method is mainly used when creating multiple objects to ensure they share the same allocator.
 *
 * @param[IN] Object
 *  JSON data object.
 *
 * @return
 *  Returns the memory allocator handle; returns AK_null on failure, which may indicate a parameter error.
 */
AK_API AK_Object akae_json_mallocator (AK_Object Object);


/**
 * Get the type of a JSON data object.
 *
 * @param[IN] Object
 *  Object handle.
 *
 * @return
 *  Returns the data type on success;\n
 *  returns AK_JSON_TYPE_UNDEF on failure, which may indicate a parameter error.
 *
 */
AK_API AK_int akae_json_typeof (AK_Object Object);


/**
 * Get the key of a JSON data object.
 *
 * @param[IN] Object
 *  Object handle.
 *
 * @return
 *  Returns the key string on success; if the string is "" the node is a root node with no key.\n
 *  Returns AK_null on failure, which may indicate a parameter error.
 *
 */
AK_API AK_chrptr akae_json_keyof (AK_Object Object);


/**
 * Get the boolean value from a JSON boolean data object.
 *
 * @param[IN] Object
 *  Object handle.
 *
 * @param[IN] def
 *  Default value; returned when the data type does not match or a read error occurs.
 *
 * @return
 *  Returns the boolean value of the data object; when parameters are invalid or
 *  the data object is not of boolean type,\n
 *  returns the default value @ref def.
 *
 */
AK_API AK_boolean akae_json_get_boolean (AK_Object Object, AK_boolean def);

/**
 * Get the numeric value from a JSON number data object.
 */
AK_API AK_dbfloat akae_json_get_number (AK_Object Object, AK_dbfloat def);

/**
 * Get the string value from a JSON string data object.
 */
AK_API AK_chrptr akae_json_get_string(AK_Object Object, AK_chrptr def);

/**
 * Get the size of a JSON data object.
 */
AK_API AK_size akae_json_sizeof (AK_Object Object);


AK_API AK_Object akae_json_parent (AK_Object Object);

AK_API AK_Object akae_json_next_object (AK_Object Object);

/**
 * Get the element object at a given index in an array.
 */
AK_API AK_Object akae_json_indexof (AK_Object Array, AK_int which);

/**
 * Get an element object from an object by key.
 */
AK_API AK_Object akae_json_indexof_key (AK_Object Object, AK_chrcptr key);

/**
* Get an element object from an object by key, case-insensitively.
*/
AK_API AK_Object akae_json_indexof_casekey (AK_Object Object, AK_chrcptr key);


/**
 * Create a null-type JSON object.
 */
AK_API AK_Object akae_json_create_null (AK_Object Mallctr);

/**
 * Create a boolean-type JSON object.
 */
AK_API AK_Object akae_json_create_boolean (AK_Object Mallctr, AK_boolean b);


/**
 * Create a number-type JSON object.
 */
AK_API AK_Object akae_json_create_number (AK_Object Mallctr, AK_dbfloat number);


/**
 * Create a string-type JSON object.
 */
AK_API AK_Object akae_json_create_string (AK_Object Mallctr, AK_chrcptr str);


/**
 * Create an array-type JSON object.
 */
AK_API AK_Object akae_json_create_array (AK_Object Mallctr);


/**
 * Create a struct-type JSON object.
 */
AK_API AK_Object akae_json_create_object (AK_Object Mallctr);


/**
 * Create an integer array JSON object.
 */
AK_API AK_Object akae_json_create_integer_array (AK_Object Mallctr, AK_int *numbers, AK_size len);


/**
 * Create a float array JSON object.
 */
AK_API AK_Object akae_json_create_float_array(AK_Object Mallctr, AK_float *numbers, AK_size len);


/**
 * Create a double-precision float array JSON object.
 */
AK_API AK_Object akae_json_create_dfloat_array(AK_Object Mallctr, AK_dbfloat *numbers, AK_size len);


/**
 * Create a string array JSON object.
 */
AK_API AK_Object akae_json_create_string_array (AK_Object Mallctr, AK_chrcptr *array, AK_size len);


/**
 * Append an object to an array.
 */
AK_API AK_Object akae_json_append_object_to_array (AK_Object Array, AK_Object Object);


/**
 * Add a new child object member to a JSON object.\n
 * Note: if a member with the key @ref key already exists, this interface does not overwrite it.\n
 * Instead it appends a new child with the same key, allowing compatibility with certain protocol
 * packet requirements that permit duplicate keys.\n
 * However, this module only supports operating on the first added value for a given key.\n
 *
 * @param[IN] Object
 *  JSON data structure object.
 *
 * @param[IN] key
 *  Key for the JSON data structure entry.
 *
 * @param[IN] AddObject
 *  The JSON object to add.
 *
 * @return
 *  Returns the created object handle on success.\n
 *  Note: on success, memory management of @ref AddObject is taken over by @ref Object;\n
 *  the caller does not need to release @ref AddObject separately.\n
 *  Returns AK_null on failure; in this case, since creation failed, @ref Object cannot take over
 *  memory management of @ref AddObject,\n
 *  so the caller must manually release @ref AddObject.
 *
 */
AK_API AK_Object akae_json_add_object_to_object (AK_Object Object, AK_chrcptr key, AK_Object AddObject);

/**
 * Add a new null-type data entry to an object.
 */
AK_API AK_Object akae_json_add_null_to_object (AK_Object Object, AK_chrptr key);


/**
 * Add a new boolean-type data entry with value @ref b to an object.
 */
AK_API AK_Object akae_json_add_boolean_to_object (AK_Object Object, AK_chrptr key, AK_boolean b);


/**
 * Add a new number-type data entry with value @ref n to an object.
 */
AK_API AK_Object akae_json_add_number_to_object (AK_Object Object, AK_chrptr key, AK_dbfloat n);


/**
 * Add a new string-type data entry with value @ref s to an object.
 */
AK_API AK_Object akae_json_add_string_to_object (AK_Object Object, AK_chrptr key, AK_chrptr s);

/**
 * Detach an object by index.\n
 * The detached object is no longer managed by the original object's memory management;
 * the caller must manually release it when done.\n
 *
 * @param[IN] Object
 *  JSON object.
 *
 * @param[IN] which
 *  Index position, starting from 0. Negative values index in reverse; e.g., -1 indexes from the last element.
 *
 * @return
 *  Returns the handle of the indexed object; returns AK_null on failure.
 */
AK_API AK_Object akae_json_detach_object_by_index (AK_Object Object, AK_int which);

/**
 * Detach an object by key.\n
 * The detached object is no longer managed by the original object's memory management;
 * the caller must manually release it when done.\n
 * See @ref akae_json_detach_object_by_index().
 */
AK_API AK_Object akae_json_detach_object_by_key (AK_Object Object, AK_chrcptr key);

/**
 * Delete an object by index.
 */
AK_API AK_int akae_json_remove_object_by_index (AK_Object Object, AK_int which);


/**
 * Delete an object by key.
 */
AK_API AK_int akae_json_remove_object_by_key (AK_Object Object, AK_chrcptr key);


/**
 * Replace an object by index.
 */
AK_API AK_Object akae_json_replace_object_by_index (AK_Object Array, AK_size which, AK_Object Replace);


/**
 * Replace an object by key.
 */
AK_API AK_Object akae_json_replace_object_by_key (AK_Object Object, AK_chrcptr key, AK_Object Replace);

/**
 * Replace an object.
 */
AK_API AK_Object akae_json_replace_object (AK_Object Object, AK_Object Replace);


/**
 * Force-convert the type of an object.
 */
AK_API AK_int akae_json_convert_type (AK_Object Object, AK_int type);



/**
 *
 * Query an element via a JSON path.\n
 * When @ref Item is Nil, reads and returns the object at that path;\n
 * otherwise updates the object at that path.\n
 * The path expression conforms to a subset of the JSONPath standard; see
 * http://goessner.net/articles/JsonPath/index.html.\n
 * Currently supports indexing via "." and "[]" expressions, for example:\n
 * \n
 * store.book[0].title \n
 * $.store[0].color \n
 * $.store.persion[2][0][0] \n
 * $.store.persion[(@.length-2)][(@.length-1)][0] \n
 * $.store['bicycle'].color \n
 * Other complex expressions are not yet supported.
 *
 * @param[IN] Object
 *  JSON data object.
 *
 * @param[IN] jsonpath
 *  JSON path description; maximum length is @ref AK_JS_PATH_MAX_LEN bytes.\n
 *
 * @return
 *  Returns the object handle on success; returns AK_null on failure, which may indicate a parameter error.
 *
 */
AK_API AK_Object akae_json_path_get_object (AK_Object Object, AK_chrcptr jsonpath);


/**
 * Get a boolean value at a path; returns the default value @ref __def if the path does not exist.
 */
#define akae_json_path_get_boolean(__Object, __jsonpath, __def) \
	akae_json_get_boolean (akae_json_path_get_object((__Object), (__jsonpath)), (__def))

/**
 * Get a numeric value at a path; returns the default value @ref __def if the path does not exist.
 */
#define akae_json_path_get_number(__Object, __jsonpath, __def) \
	akae_json_get_number (akae_json_path_get_object((__Object), (__jsonpath)), (__def))

/**
 * Get a string value at a path; returns the default value @ref __def if the path does not exist.
 */
#define akae_json_path_get_string(__Object, __jsonpath, __def) \
	akae_json_get_string (akae_json_path_get_object((__Object), (__jsonpath)), (__def))


/**
 * Set a JSON object at the given JSON path.\n
 * JSON path rules: see @ref akae_json_path_get_object().\n
 *
 * @param[IN] Object
 *  JSON data object.
 *
 * @param[IN] jsonpath
 *  JSON path description; maximum length is @ref AK_JSON_PATH_MAX_LEN bytes.\n
 *
 * @param[IN] SetObject
 *  When AK_null, reads and returns the object at that path;\n
 *  otherwise updates the object at that path and returns it.\n
 *
 * @return
 *  On success, returns the newly applied JSON data object (@ref SetObject).\n
 *  In this case, memory management of @ref SetObject is taken over by @ref Object;\n
 *  the caller does not need to release @ref SetObject separately.\n
 *  Returns AK_null on failure; in this case, the caller must manually release @ref SetObject.
 *
 */
AK_API AK_Object akae_json_path_set_object (AK_Object Object, AK_chrcptr jsonpath, AK_Object SetObject);


/**
 * Add a boolean-type value at a JSON path.
 */
#define akae_json_path_set_boolean(__Object, __jsonpath, __b) \
	do {\
		AK_Object __NewObject = akae_json_create_boolean (akae_json_mallocator(__Object), __b);\
		if (AK_null != __NewObject) {\
			if (!akae_json_path_set_object ((__Object), (__jsonpath), __NewObject)) {\
				akae_json_release (__NewObject);\
			}\
		}\
	} while (0)

/**
 * Add a numeric-type value at a JSON path.
 */
#define akae_json_path_set_number(__Object, __jsonpath, __n) \
	do {\
		AK_Object __NewObject = akae_json_create_number (akae_json_mallocator(__Object), __n);\
		if (AK_null != __NewObject) {\
			if (!akae_json_path_set_object ((__Object), (__jsonpath), __NewObject)) {\
				akae_json_release (__NewObject);\
			}\
		}\
	} while (0)

/**
 * Add a string-type value at a JSON path.
 */
#define akae_json_path_set_string(__Object, __jsonpath, __s) \
	do {\
		AK_Object __NewObject = akae_json_create_string (akae_json_mallocator(__Object), __s);\
		if (AK_null != __NewObject) {\
			if (!akae_json_path_set_object ((__Object), (__jsonpath), __NewObject)) {\
				akae_json_release (__NewObject);\
			}\
		}\
	} while (0)



/**
 * Remove a JSON object at a JSON path.
 */
AK_API AK_int akae_json_path_remove_object (AK_Object Object, AK_chrptr jsonpath);

/**
 * @brief
 *  Query an element collection via a JSON path.
 *  On success, returns a JSON array created by the internal allocator of @ref Obj.
 *  The caller must manually call @ref akae_json_Free() to release the JSON array after use.
 *
 * @param[IN] Obj
 *  JSON data object.
 *
 * @param[IN] jpath
 *  JSON collection path description.
 *
 * @retval JSON array
 *  The queried data is contained in the JSON array; traverse the collection via @ref akae_json_ArrayOf().\n
 *  If the query fails, the collection is empty.
 *
 * @retval Nil
 *  Query failed; may indicate a parameter error.
 */
AK_API AK_Object akae_json_path_query (AK_Object Object, AK_chrptr jsonpath);




/**
 * Minify JSON text.\n
 * Removes spaces, newlines, and indentation outside of JSON data structures,\n
 * reducing memory consumption of the text.
 *
 * @param[IN,OUT] jsonp
 *  JSON text data.
 *
 * @retval AK_OK
 *  Operation succeeded.
 * @retval AK_ERR_INVAL_PARAM
 *  Operation failed; invalid parameter.
 */
AK_API AK_int akae_json_minify_text (AK_chrptr jsonp);


/**
 * Convert a JSON data object to text.\n
 * The data pointer is retained internally by the module and released automatically when the module
 * is released; the caller does not need to free it separately.
 *
 * @param[IN] Object
 *  JSON data object.
 *
 * @param[IN] pretty
 *  Pretty-print flag; when True, the output is more human-readable but uses more memory.
 *
 * @return
 *  Returns the start address of the string on success; returns AK_null on failure, which may indicate
 *  a parameter error.
 *
 */
AK_API AK_chrptr akae_json_stringify (AK_Object Object, AK_boolean pretty);

/**
 * See @ref akae_json_stringify(); result is written to a stack buffer.
 */
AK_API AK_ssize akae_json_stringify2 (AK_Object Object, AK_boolean pretty, AK_chrptr stack, AK_size stacklen);

/**
 * @brief
 *  Convert to a JavaScript variable definition text.
 *
 *
 */
AK_API AK_ssize
akae_json_ToJSVar(AK_Object Object, AK_chrptr key, AK_chrptr stack, AK_size stacklen);


/**
 * @macro
 *  Iterate over a JSON data object.
 */
#define akae_json_foreach(__Object, __Child) \
	for ((__Child) = akae_json_indexof((__Object), 0);\
		AK_null != (__Child);\
		(__Child) = akae_json_next_object((__Child)))

/**
 * Clear a JSON data object.
 */
#define akae_json_empty_object(__Object) \
	do {\
		if (AK_null != (__Object)) {\
			if (AK_JSON_TYPE_ARRAY == akae_json_typeof (__Object)\
					|| AK_JSON_TYPE_OBJECT == akae_json_typeof (__Object)) {\
				while (akae_json_sizeof(__Object) > 0) {\
					akae_json_remove_object_by_index ((__Object), 0);\
				}\
			}\
		}\
	} while (0)

/**
 * Create a difference patch.\n
 * The patch is itself a JSON data object, created by @ref Alloctr.\n
 * After use, the caller must release the object using @ref akae_json_Free(), otherwise a memory leak
 * will occur.\n
 * See https://tools.ietf.org/html/rfc7396.
 *
 * @param[IN] Mallctr
 *  Memory allocator for creating the difference patch.
 *
 * @param[IN] Object1
 *  Source object for the diff.
 *
 * @param[IN] Object2
 *  Target object for the diff.
 *
 * @return
 *  Returns a diff object on success; returns AK_null on failure, which may indicate a parameter error.
 *
 */
AK_API AK_Object akae_json_create_patch (AK_Object Mallctr, AK_Object Object1, AK_Object Object2);

/**
 * @brief
 *  Merge a patch into a data object.\n
 *  After merging, the @ref Patch object is not released;\n
 *  the caller must release it using @ref akae_json_Free(), otherwise a memory leak will occur.\n
 *  See https://tools.ietf.org/html/rfc7396.
 *
 * @param[IN] Object
 *  Target object; this method modifies the object directly according to the patch content.
 *
 * @param[IN] Patch
 *  Patch object created via @ref akae_json_create_patch().
 *
 * @return
 */
AK_API AK_Object akae_json_merge_patch (AK_Object Object, AK_Object Patch);


AK_C_HEADER_EXTERN_C_END
#endif /* JSON_H_ */
