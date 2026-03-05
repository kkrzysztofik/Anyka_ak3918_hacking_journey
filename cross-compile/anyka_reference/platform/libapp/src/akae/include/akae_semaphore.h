/**
 * Semaphore.
 */

#include <akae_typedef.h>


#if !defined(AKAE_SEMAPHORE_H_)
#define AKAE_SEMAPHORE_H_
AK_C_HEADER_EXTERN_C_BEGIN


/**
 * Semaphore handle definition.
 */
#define AK_Semaphore AK_handle


/**
 * Create a semaphore module handle.
 */
AK_API AK_Semaphore akae_sem_create (AK_size value);

/**
 * Destroy the Sem module handle.
 */
AK_API AK_int akae_sem_release (AK_Semaphore handle);

/**
 * Call @ref akae_sem_release() and set the passed-in handle to AK_null.
 */
#define akae_sem_release_clear(__handle) \
	({\
		AK_int const res = akae_sem_release(__handle);\
		if (AK_OK == res) {\
			__handle = AK_INVAL_HANDLE;\
		}\
		res;\
	})


/**
 * @brief
 *  Wait for a semaphore with timeout blocking; used in conjunction with akae_sem_post() @ref akae_sem_post().\n
 *
 * @details
 *  Acquires the semaphore by waiting with a timeout.\n
 *  After the semaphore is acquired, the internal semaphore value decrements by 1.
 *
 * @param[IN] timeo_us
 *  Wait timeout in microseconds: less than 0 means wait indefinitely, equal to 0 means return asynchronously, greater than 0 means wait for that duration.
 *
 * @return
 *  Returns 0 on success, otherwise returns -1.
 */
AK_API AK_int akae_sem_wait (AK_Semaphore handle, AK_ssize timeo_us);

/**
 * @brief
 *  Non-blocking wait for a semaphore; used in conjunction with akae_sem_post() @ref akae_sem_post().\n
 *
 * @details
 *  Acquires a semaphore in a non-blocking manner. After successful acquisition,\n
 *  the internal semaphore value decrements by 1. The current semaphore value can be retrieved via @ref NK_Sem::value().
 *
 * @return
 *  Returns immediately when the semaphore is greater than 0, otherwise returns -1.
 */
AK_API AK_int akae_sem_get (AK_Semaphore handle);

/**
 * @brief
 *  Post a semaphore.
 *
 * @details
 *  Posts a semaphore; after a successful post the internal semaphore value increments by 1.\n
 * The current semaphore value can be retrieved via @ref NK_Sem::value().
 *
 * @return
 *  Returns 0 on success, otherwise returns -1, possibly because the module has already been destroyed.
 */
AK_API AK_int akae_sem_post (AK_Semaphore handle);

/**
 * @brief
 *  Get the current semaphore value.
 *
 */
AK_API AK_size akae_sem_value (AK_Semaphore handle);



AK_C_HEADER_EXTERN_C_END
#endif /* AKAE_SEMAPHORE_H_ */

