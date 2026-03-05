/**
 * NkUtils memory allocator.
 * This module handles dynamic memory allocation and management.
 * This module does not support thread safety.
 *
 */

#include "object.h"

#ifndef NK_ALLOCATOR_H_
#define NK_ALLOCATOR_H_
NK_CPP_EXTERN_BEGIN

/**
 * @brief
 * Memory allocator module handle.
 *
 * Through the memory allocator, dynamic allocation within a relatively large memory block can be achieved.
 *
 */
typedef struct Nk_Allocator
{
#define THIS struct Nk_Allocator *const Allocator

	/**
	 * Module base interface.
	 */
	NK_Object Object;

	/**
	 * Allocate a memory block.
	 *
	 * @param[in]		size		Size of the memory block to allocate.
	 *
	 * @return	Returns the start address of the allocated memory block on success, NK_Nil otherwise.
	 *
	 */
	NK_PVoid
	(*alloc)(THIS, NK_Size size);


	/**
	 * Reallocate memory.\n
	 * Will extend from existing memory if resources allow.
	 *
	 * @param[in]		ptr			Address of the previously allocated memory block.
	 * @param[in]		size		New size to allocate.
	 *
	 * @return	Returns the start address of the reallocated memory on success, NK_Nil otherwise. On failure, the originally allocated memory remains usable.
	 */
	NK_PVoid
	(*realloc)(THIS, NK_PVoid ptr, NK_Size size);

	/**
	 * Allocate memory and copy a string.\n
	 * The module allocates an appropriately sized memory block via @ref NK_Allocator::alloc based on the input string length,\n
	 * then copies the string data into that memory block.\n
	 * Memory obtained after cloning must be freed via the @ref free method.
	 *
	 * @param[in]		str			The string to copy.
	 *
	 * @return	Returns the start address of the allocated memory block on success, NK_Nil on failure.
	 *
	 */
	NK_PChar
	(*strdup)(THIS, NK_PChar str);

	/**
	 * Similar to @ref strdup, copies at most n characters.
	 *
	 * @param[in]		str			The string to copy.
	 * @param[in]		n			Maximum number of bytes to copy.
	 *
	 * @return	Returns the start address of the allocated memory block on success, NK_Nil on failure.
	 */
	NK_PChar
	(*strndup)(THIS, NK_PChar str, NK_Size n);

	/**
	 * Copy a memory block.\n
	 * After copying, a new memory block is produced with the same content as the original.\n
	 * The produced memory block must be freed via the @ref free method.\n
	 * When calling this interface, the available length of the memory block at @ref mem must be greater than or equal to @ref size, otherwise a potential error may occur.
	 *
	 * @param[in]		mem			Start address of the memory block to copy.
	 * @param[in]		size		Maximum number of bytes to copy.
	 *
	 * @return	Returns the start address of the allocated memory block on success, NK_Nil on failure.
	 */
	NK_PVoid
	(*memdup)(THIS, NK_PVoid mem, NK_Size size);


	/**
	 * Reclaim memory space.
	 *
	 * @param[in]		ptr			Address of the memory block allocated by @ref alloc or @ref realloc.
	 *
	 * @return	None.
	 *
	 */
	NK_Void
	(*freep)(THIS, NK_PVoid ptr);

	/**
	 * Get the length of the allocated memory block.
	 *
	 * @param[in]		ptr			Address of the memory block allocated by @ref alloc or @ref realloc.
	 *
	 * @return
	 * If using the OS memory allocator, this interface always returns 0.\n
	 * If using a user-defined memory allocator,\n
	 * this interface returns the length of the allocated memory block on success, or -1 if the memory address is invalid.
	 *
	 */
	NK_SSize
	(*length)(THIS, NK_PVoid ptr);

	/**
	 * Return total capacity of the allocator's internal buffer.
	 *
	 * @return
	 * When using a user-assigned memory buffer,\n
	 * this interface returns the total capacity of the allocator's internal buffer.\n
	 * When using the OS memory allocator, this interface always returns -1.
	 *
	 */
	NK_SSize
	(*capacity)(THIS);

	/**
	 * Return used amount of the allocator's internal buffer.
	 *
	 * @return
	 * When using a user-assigned memory buffer,\n
	 * this interface returns the total capacity of the allocator's internal buffer.\n
	 * When using the OS memory allocator, this interface always returns -1.
	 */
	NK_SSize
	(*used)(THIS);

#undef THIS
} NK_Allocator;

/**
 * Create module handle.\n
 * Pass in a fixed-size memory region for the allocator to manage.\n
 * After the allocator takes over, the user can dynamically allocate memory blocks within the region as needed.\n
 * Due to algorithm overhead, the passed-in memory block size must not be less than 1K bytes, otherwise creation fails.\n
 * It is recommended to use a memory region larger than 4K bytes.
 *
 * @param[in]		mem			Start address of the memory region for allocation.
 * @param[in]		len			Size of the memory region; must be greater than 1K.
 *
 * @return		Returns the module handle on success, Nil on failure.
 */
extern NK_Allocator *
NK_Alloc_Create(NK_PByte mem, NK_Size len);

/**
 * Destroy module handle.\n
 *
 * @param[in]		Allocator_r	Reference to the module handle.
 * @param[out]		mem_r		On successful release, retrieves the start address of the memory region passed in at creation time.
 *
 * @return		Returns the length of the memory block passed in at creation time; @ref mem_r retrieves the start address of the memory region passed in at creation time.
 */
extern NK_SSize
NK_Alloc_Free(NK_Allocator **Allocator_r, NK_PByte *mem_r);


NK_CPP_EXTERN_END
#endif /* NK_ALLOCATOR_H_ */

