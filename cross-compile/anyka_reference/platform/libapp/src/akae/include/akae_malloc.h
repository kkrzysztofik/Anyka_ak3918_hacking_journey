
/**
 * Memory allocator.\n
 * The user hands over a block of memory to the allocator for management; the allocator internally
 * manages the memory efficiently,\n
 * allowing the user to perform multiple dynamic allocations.\n
 * Note: once memory is taken over by the allocator, external read/write operations on it are
 * prohibited to avoid corrupting the management state.
 */

#include <akae_object.h>

#if !defined(AKAE_MALLOC_H_)
#define AKAE_MALLOC_H_
AK_C_HEADER_EXTERN_C_BEGIN


/**
 * Minimum memory length the user must provide for the allocator to take over;
 * at least the defined amount of memory must be given to the memory allocator.
 */
#define AK_MALLOC_MIN_GIVEN_MEM_LEN (1024)

/**
 * Memory allocator runtime status.
 */
typedef struct _AK_MallocatorStatus {

	/// Number of free memory blocks.
	AK_size free_blocks;

	/// Number of allocated memory blocks.
	AK_size allocated_blocks;

	/// Total capacity.
	AK_size capacity;

	/// Current usage.
	AK_size usage;

	/// Peak usage.
	AK_size usage_peak;

} AK_MallocatorStatus;


/**
 * Create a memory allocator.
 *
 * @param[IN]
 *  threadsafe
 *  Thread-safety setting; choose whether to enable thread safety for the allocator.\n
 *  Enabling thread safety incurs some performance overhead; if the allocator is not used in a
 *  thread-safe environment, it is not necessary to enable it.
 *
 * @param[IN]
 *  givenMemAddr
 *  Starting address of the user-provided memory, which the allocator will take over.
 *
 * @param[IN]
 *  givenMemLen
 *  Length of the user-provided memory; this value must be greater than
 *  @ref AK_MALLOC_MIN_GIVEN_MEM_LEN\n
 *  for the allocator to be created successfully, otherwise creation will fail.
 *
 * @return
 *  Returns the allocator object handle on success; returns AK_null on failure.
 *
 */
AK_API AK_Object akae_malloc_create (AK_boolean threadsafe, AK_bytptr givenMemAddr, AK_size givenMemLen);

/**
 * Destroy a memory allocator.\n
 *
 * @param[IN]
 *  Object
 *  Allocator object handle, created by @ref akae_malloc_create().
 *  After successful release, this handle pointer will no longer be used.
 *
 * @param[OUT]
 *  givenMemAddr
 *  Starting address of the user-provided memory passed in at @ref akae_malloc_create();\n
 *  returned to the user at release time; pass AK_null if the user does not need to retrieve it.
 *
 * @param[OUT]
 *  givenMemLen
 *  Length of the user-provided memory passed in at @ref akae_malloc_create();\n
 *  returned to the user at release time; pass AK_null if the user does not need to retrieve it.
 *
 * @retval AK_OK
 *  Released successfully.
 * @retval AK_ERR_INVAL_OBJECT
 *  Release failed, an invalid object was passed.
 * @retval AK_ERR_INTERNAL
 *  Release failed, a system-level exception occurred.
 *
 */
AK_API AK_int akae_malloc_release (AK_Object Object, AK_bytptr *givenMemAddr, AK_size *givenMemSize);

/**
 * Allocate a block of memory from the allocator.
 *
 * @param[IN]
 *  Object
 *  Allocator object handle, created by @ref akae_malloc_create().
 *
 * @param[IN]
 *  len
 *  Length of memory to allocate; must be greater than 0, otherwise allocation returns failure.
 *
 * @return
 *  Returns the starting address of the memory on success; returns AK_null on failure.
 */
AK_API AK_voidptr akae_malloc_traced_alloc (AK_Object Object, AK_size size, AK_chrcptr filename, AK_uint32 lineno);

/**
 * Quick call with trace information.
 */
#define akae_malloc_alloc(__Object, __size) akae_malloc_traced_alloc(__Object, __size, akae_basename (__FILE__), __LINE__)


/**
 * Quick call with trace information; also initializes the allocated memory to 0.
 */
#define akae_malloc_alloc_zero(__Object, __size) \
	akae_memzero (akae_malloc_traced_alloc(__Object, __size, akae_basename (__FILE__), __LINE__), (__size))




/**
 * Change the size of an already allocated memory block.\n
 * The user passes in the already allocated memory address @ref preptr and the new length @ref newlen;\n
 * the allocator will reallocate a memory block of size @ref newlen for the user.\n
 * Notes:\n
 * If allocation fails, the @ref preptr memory is retained;\n
 * if allocation succeeds, a new memory starting address is returned and the memory at @ref preptr
 * is reclaimed by the allocator;\n
 * the user must not continue using it.\n
 * If @ref newlen is 0, it means releasing the memory at @ref preptr, behaving the same as
 * @ref akae_malloc_free().
 * After release, @ref preptr also cannot be used.
 *
 */
AK_API AK_voidptr akae_malloc_realloc (AK_Object Object, AK_voidptr preptr, AK_size newlen, AK_chrcptr filename, AK_uint32 lineno);

/**
 * Quick call with trace information.
 */
#define akae_malloc_realloc(__Object, __preptr, __size) akae_malloc_realloc(__Object, __preptr, __size, akae_basename (__FILE__), __LINE__)


/**
 * Allocate a block of memory from the allocator and copy a string into it.
 * Internally calls @ref akae_malloc_alloc().
 *
 */
AK_API AK_chrptr akae_malloc_traced_strdup (AK_Object Object, AK_chrcptr str, AK_chrcptr filename, AK_uint32 lineno);

/**
 * Quick call with trace information.
 */
#define akae_malloc_strdup(__Object, __str) akae_malloc_traced_strdup(__Object, __str, akae_basename (__FILE__), __LINE__)


/**
 * Duplicate a string of specified length; see @ref akae_malloc_strdup().
 *
 */
AK_API AK_chrptr akae_malloc_traced_strndup (AK_Object Object, AK_chrcptr str, AK_size n, AK_chrcptr filename, AK_uint32 lineno);

/**
 * Quick call with trace information.
 */
#define akae_malloc_strndup(__Object, __str, __n) akae_malloc_traced_strndup(__Object, __str, __n, akae_basename (__FILE__), __LINE__)


/**
 * Allocate a block of memory from the allocator and copy a memory block into it.
 * Internally calls @ref akae_malloc_alloc().
 *
 */
AK_API AK_voidptr akae_malloc_traced_memdup (AK_Object Object, AK_voidptr mem, AK_size len, AK_chrcptr filename, AK_uint32 lineno);

/**
 * Quick call with trace information.
 */
#define akae_malloc_memdup(__Object, __mem, __len) akae_malloc_traced_memdup(__Object, __mem, __len, akae_basename (__FILE__), __LINE__)



/**
 * Free the memory pointed to by @ref ptr,\n
 * the memory area corresponding to this address must have been allocated by @ref Object,
 * otherwise the release will fail.\n
 * If @ref ptr is AK_null, this interface returns success.\n
 * It is recommended to set @ref ptr = AK_null after calling this interface to effectively
 * avoid double-freeing memory.
 *
 * @param[IN]
 *  Object
 *  Memory allocator object handle.
 *
 * @param[IN]
 *  ptr
 *  Starting address of the memory to be released.
 *
 * @retval AK_OK
 *  Successfully released the memory corresponding to @ref ptr.
 * @retval AK_ERR_INVAL_OBJECT
 *  Release failed, an invalid object handle was passed.
 * @retval AK_ERR_INVAL_PARAM
 *  Release failed, an invalid memory pointer was passed; the memory area may not have been
 *  allocated by @ref Object.
 * @retval AK_ERR_NOT_PERM
 *  Release failed, the memory may have been released twice.
 */
AK_API AK_int akae_malloc_free (AK_Object Object, AK_voidptr ptr);

/**
 * Call @ref akae_malloc_free() and set the passed pointer to null.
 */
#define akae_malloc_free_clear(__Object, __ptr) \
	({\
		AK_int res = akae_malloc_free (__Object, __ptr);\
		if (AK_OK == res) {\
			(__ptr) = AK_null;\
		}\
		res;\
	})


/**
 * Similar in function to @ref akae_malloc_free(), but does not perform memory defragmentation.\n
 * Users can choose, when performing consecutive memory releases,\n
 * to first call @ref akae_malloc_free_lazy() consecutively,\n
 * and then finally call @ref akae_malloc_free(),\n
 * for a certain performance saving.
 *
 */
AK_API AK_int akae_malloc_free_lazy (AK_Object Object, AK_voidptr ptr);


/**
 * Storage space defragmentation.\n
 * Used in conjunction with the @ref akae_malloc_free_lazy() method.
 * Each call to @ref akae_malloc_free() performs memory defragmentation.\n
 * In this case, if the user releases too many data blocks in a single operation via frequent
 * @ref akae_malloc_free() calls, performance is affected,\n
 * primarily due to redundant operations from frequent defragmentation.\n
 * For upper-level application scenarios involving frequent consecutive memory releases,
 * using @ref akae_malloc_free_lazy() can effectively reduce this overhead;\n
 * after multiple calls to @ref akae_malloc_free_lazy(), a large amount of memory fragmentation
 * will accumulate,\n
 * which can then be cleaned up by manually calling this interface.
 *
 */
AK_API AK_int akae_malloc_defragment (AK_Object Object);


/**
 * Get the length of allocated memory (in bytes).\n
 * Due to alignment optimization logic, the actual allocated memory length may be slightly larger
 * than the requested value.
 *
 * @return
 *  Returns the length of the allocated memory on success; returns 0 on failure.
 */
AK_API AK_size akae_malloc_memlen (AK_Object Object, AK_voidptr ptr);


/**
 * Get the allocatable capacity of the allocator (in bytes).
 */
AK_API AK_size akae_malloc_capacity (AK_Object Object);


/**
 * Get the allocation usage of the allocator (in bytes).
 */
AK_API AK_size akae_malloc_usage (AK_Object Object);

/**
 * Get the peak allocation usage of the allocator (in bytes);
 * use this interface to roughly assess whether the allocator is likely to overload.
 */
AK_API AK_size akae_malloc_usage_peak (AK_Object Object);

/**
 * Get the runtime status of the memory allocator.
 */
AK_API AK_int akae_malloc_status (AK_Object Object, AK_MallocatorStatus *Status);


/**
 * Safe memory copy operation based on allocator-allocated memory.\n
 * Internally calls @ref akae_malloc_memlen() to first obtain the allocated memory length.\n
 * Then uses the length limit to implement safe copying to avoid memory out-of-bounds access.
 */
AK_API AK_voidptr akae_malloc_safe_memcpy (AK_Object Object, AK_voidptr dest, AK_voidptr src, AK_size n);

/**
 * Safe string copy operation based on allocator-allocated memory.\n
 * See @ref akae_malloc_safe_memcpy().
 */
AK_API AK_chrptr akae_malloc_safe_strcpy (AK_Object Object, AK_chrptr dest, AK_chrptr src);


AK_C_HEADER_EXTERN_C_END
#endif ///< AKAE_MALLOC_H_
