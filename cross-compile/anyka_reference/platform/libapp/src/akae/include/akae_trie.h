
#include <akae_typedef.h>
#include <akae_object.h>

#if !defined(AKAE_TRIE_H_)
# define AKAE_TRIE_H_
AK_C_HEADER_EXTERN_C_BEGIN

/**
 * @macro
 *  Maximum key length.
 */
#define AK_STRLST_MAX_KEY_LEN (256)



/**
 * Create a trie module. After the handle is created, all @ref key indexing is case-sensitive.\n
 * For case-insensitive indexing, see @ref akae_trie_create2().
 *
 * @param[IN] Malloc
 *  Memory allocator for the list module.
 *
 * @return
 *  Returns the module handle on success, or AK_null on failure.
 */
AK_API AK_Object akae_trie_create (AK_Object Malloc);

/**
 * See @ref akae_trie_create(); the @ref case_sensitive parameter controls case sensitivity.
 */
AK_API AK_Object akae_trie_create2 (AK_Object Malloc, AK_boolean case_sensitive);

/**
 * Release the trie module handle.\n
 * Memory used by the trie module is reclaimed via the allocator passed in at creation time.
 *
 */
AK_API AK_int akae_trie_release (AK_Object Object);


/**
 * Get the case-sensitivity flag of the current module.\n
 * Note: passing an invalid handle to this interface also returns AK_false. Therefore the accuracy of the return value depends on correct parameter usage.
 *
 */
AK_API AK_boolean akae_trie_case_sensitive (AK_Object Object);


/**
 * Get the list length.\n
 * Note: passing an invalid parameter handle to this interface returns 0. Therefore the accuracy of the return value depends on correct parameter usage.
 *
 * @return
 *  Returns the length (number of elements) of the list.
 */
AK_API AK_size akae_trie_length (AK_Object Object);

/**
 * @brief
 *  Insert a data block pointed to by @ref data with length @ref data_len into the container, indexed by the @ref key entry.\n
 *  The container copies an internal replica of the data based on the passed-in length.\n
 *
 * @param overwrite [in]
 *  Overwrite flag; when set to False, if the key already exists in the container, the existing entry is not overwritten and an additional entry with the same key is appended.
 * @param key [in]
 *  Entry string; length must be greater than 0; case-sensitive.
 * @param data [in]
 *  Data corresponding to the entry.
 * @param datalen [in]
 *  Data length; the module stores an internal memory block of equal length.
 *
 * @return
 *  Returns 0 on success, otherwise returns -1.
 *
 */
AK_API AK_int akae_trie_insert (AK_Object Object, AK_chrcptr key, AK_bytptr data, AK_size datalen);

/**
 * Remove the data corresponding to the @ref key value from the trie.
 *
 * @param[IN] key
 *  Entry string; length must be greater than 0; case sensitivity is determined by the @ref akae_trie_create2() initialization parameter.
 *
 * @retval AK_OK
 *  Removal succeeded.
 *  Returns 0 on success, or -1 if the key does not exist.
 */
AK_API AK_int akae_trie_remove (AK_Object Object, AK_chrcptr key);


/**
 * Index data by key value.\n
 * The result is returned via parameters @ref data and @ref datalen.\n
 * Note: passing AK_null for @ref data or @ref datalen does not cause an error, but the caller cannot retrieve the indexed result from those parameters.\n
 * The indexed result is a reference; modifying the memory block will corrupt the source data. Avoid performing removal operations on this data while it is in use.
 *
 * @param[IN] key
 *  Index key value.
 *
 * @param[OUT] data
 *  Starting address of the returned result; after successful indexing, this address points to the start of the data.
 *
 * @param[OUT] datalen
 *  Data length; after successful indexing, this value returns the length of the data.
 *
 * @retval AK_OK
 *  Indexing succeeded.
 * @retval AK_ERR_INVAL_OBJECT
 *  Indexing failed; an invalid object handle was passed in.
 * @retval AK_ERR_INVAL_PARAM
 *  Indexing failed; possibly an invalid @ref key parameter was passed in.
 * @retval AK_ERR_NOT_EXIST
 *  Indexing failed; no data exists for the given key.
 *
 *
 */
AK_API AK_int akae_trie_indexof (AK_Object Object, AK_chrcptr key, AK_bytptr *data, AK_size *datalen);


/**
 * Index data by key value.\n
 * The result is returned via parameters @ref data and @ref datalen.\n
 * Note: passing AK_null for @ref data or @ref datalen does not cause an error, but the caller cannot retrieve the indexed result from those parameters.\n
 * The indexed result is a reference; modifying the memory block will corrupt the source data. Avoid performing removal operations on this data while it is in use.
 *
 * @param[IN] pos
 *  Position value in the trie.
 *
 * @param[OUT] data
 *  Starting address of the returned result; after successful indexing, this address points to the start of the data.
 *
 * @param[OUT] datalen
 *  Data length; after successful indexing, this value returns the length of the data.
 *
 * @retval AK_OK
 *  Indexing succeeded.
 * @retval AK_ERR_INVAL_OBJECT
 *  Indexing failed; an invalid object handle was passed in.
 * @retval AK_ERR_INVAL_PARAM
 *  Indexing failed; possibly an invalid @ref key parameter was passed in.
 * @retval AK_ERR_NOT_EXIST
 *  Indexing failed; no data exists for the given key.
 *
 *
 */
AK_API AK_int akae_trie_locate (AK_Object Object, AK_int pos, AK_chrptr *key, AK_bytptr *data, AK_size *datalen);

/**
 * Check whether a given key exists in the trie.
 *
 * @param[IN] Object
 *  Object handle.
 *
 * @param[IN] key
 *  Index key value.
 *
 * @retval AK_true
 *  Data for the key exists.
 *
 * @ref AK_false
 *  Key data does not exist; passing an invalid parameter to this interface also returns AK_false. Therefore the accuracy of the return value depends on correct parameter usage.
 */
AK_API AK_boolean akae_trie_has_key (AK_Object Object, AK_chrcptr key);


/**
 * Read and remove a data entry from the trie.\n
 * Functionally similar to @ref akae_trie_indexof().\n
 * After reading the data, it is removed from the trie; this differs from @ref akae_trie_indexof().
 *
 * @param[IN] Object
 *  Object handle.
 *
 * @param[IN] key
 *  Index key value.
 *
 * @param[OUT] stack
 *  Stack memory address used to store the returned data.
 *
 * @param[IN] stacklen
 *  Stack memory size passed in.
 *
 * @retval AK_OK
 *  Operation succeeded.
 * @retval AK_ERR_INVAL_OBJECT
 *  Operation error; an invalid object handle was passed in.
 * @retval AK_ERR_INVAL_PARAM
 *  Operation error; an invalid parameter was passed in.
 * @retval AK_ERR_OUT_OF_MEM
 *  Operation error; the stack memory passed in is insufficient to hold the result.
 *
 */
AK_ssize akae_trie_get_off (AK_Object Object, AK_chrcptr key, AK_bytptr stack, AK_size stacklen);


AK_C_HEADER_EXTERN_C_END
#endif ///< AKAE_TRIE_H_
