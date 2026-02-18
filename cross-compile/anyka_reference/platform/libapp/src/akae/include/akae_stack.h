
/**
 * Linked list application implementation.
 */

#include <akae_object.h>


#if !defined(AK_STACK_H_)
#define AK_STACK_H_
AK_C_HEADER_EXTERN_C_BEGIN

/**
 * Create a stack structure object.
 */
AK_API AK_Object akae_stack_create (AK_Object Malloc, AK_boolean threadsafe);
/**
 * Release a dynamically allocated object; corresponds to @ref akae_stack_create().
 */
AK_API AK_int akae_stack_release (AK_Object Object);

/**
 * Call @ref akae_stack_release() and set the passed-in handle to AK_null.
 */
#define akae_stack_release_clear(__Object) \
	({\
		AK_int const res = akae_stack_release(__Object);\
		if (AK_OK == res) {\
			__Object = AK_null;\
		}\
		res;\
	})

/**
 * Push a data item onto the top of the stack.\n
 * Note: the memory pointed to by the @ref data pointer is managed by the caller; the module does not manage it internally (including reclamation or destruction).\n
 * The module acts purely as a container that maintains a series of memory pointers.
 *
 *
 * @return
 *  Returns AK_OK on success, otherwise returns a failure code.
 */
AK_API AK_int akae_stack_push (AK_Object Object, AK_voidptr data);


/**
 * Pop a data item from the top of the stack (FILO - first in, last out).\n
 *
 * @verbatim
 *
 *   akae_stack_push()
 *   +
 *   |     +-+-+-+-+-+-+-+-+-+-+
 *   +-----> N,N-1,...3,2,1,0  |
 *   +-----+                   |
 *   |     +-+-+-+-+-+-+-+-+-+-+
 *   v
 *   akae_stack_pop()
 *
 *
 * @endverbatim
 *
 * @return
 *  Returns AK_OK on success, otherwise returns a failure code.
 */
AK_API AK_int akae_stack_pop (AK_Object Object, AK_voidptr *data);


/**
 * Dequeue a data item from the bottom of the stack (FIFO - first in, first out).\n
 *
 *
 * @verbatim
 *
 *   akae_stack_push()
 *   +
 *   |     +-+-+-+-+-+-+-+-+-+-+
 *   +-----> N,N-1,...3,2,1,0  |
 *         |                   +--+
 *         +-+-+-+-+-+-+-+-+-+-+  |
 *                                v
 *                       akae_stack_shift()
 *
 * @endverbatim
 *
 * @retval AK_ERR_NOT_EXIST
 *  Operation failed; the queue is empty and no data can be retrieved.
 */
AK_API AK_int akae_stack_shift (AK_Object Object, AK_voidptr *data);

/**
 * Peek at the data content of the queue without modifying the queue structure.\n
 * Unlike @ref akae_stack_pop() and @ref akae_stack_shift().
 *
 */
AK_API AK_int akae_stack_peek (AK_Object Object, AK_int id, AK_voidptr *data);

/**
 * Get the depth (size) of the stack structure.
 *
 * @return
 *  Returns the depth (number of elements) of the stack on success, or 0 on failure.\n
 *  Note: a return value of 0 is ambiguous -- it means either the stack is empty or an invalid handle was passed in.
 */
AK_API AK_size akae_stack_depth (AK_Object Object);




AK_C_HEADER_EXTERN_C_END
#endif ///< AK_STACK_H_
