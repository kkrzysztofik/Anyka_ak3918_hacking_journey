/**
 * Thread mutex lock object interface definitions.
 */

#include <akae_typedef.h>


#if !defined(AKAE_MUTEX_H_)
#define AKAE_MUTEX_H_
AK_C_HEADER_EXTERN_C_BEGIN


/**
 * Mutex lock object definition.
 */
#define AK_Mutex AK_handle


/**
 * Dynamically allocate a data object.
 */
AK_API AK_Mutex akae_mutex_create (AK_NOARGS);

/**
 * Specify whether to enable a recursive lock via the @ref recursive flag.
 */
AK_API AK_Mutex akae_mutex_create2 (AK_boolean recursive);

/**
 * Release a dynamically allocated object; corresponds to @ref akae_mutex_create() or @ref akae_mutex_create2().
 */
AK_API AK_int akae_mutex_release (AK_Mutex handle);

/**
 * Release and simultaneously clear the handle.
 */
#define akae_mutex_release_clear(__handle) \
	({\
		AK_int const res = akae_mutex_release (__handle);\
		if (res == AK_OK) {\
			__handle = AK_INVAL_HANDLE;\
		}\
		res;\
	})


/**
 * Synchronously acquire the lock.\n
 * If this interface is called from two different threads, the first caller acquires the lock\n
 * and the second caller must wait until the first caller unlocks it, achieving mutual exclusion.\n
 * If called twice consecutively from the same thread,\n
 * the interface returns a deadlock error @ref AK_ERR_DEAD_LOCK.
 *
 */
AK_API AK_int akae_mutex_lock (AK_Mutex handle);

/**
 * Asynchronously acquire the lock.\n
 * If the lock has not yet been acquired, calling this interface acquires it, same as @ref akae_mutex_lock().\n
 * If the lock has already been acquired by another thread, this interface returns an @ref AK_ERR_BUSY error.
 *
 */
AK_API AK_int akae_mutex_try_lock (AK_Mutex handle);


/**
 * Attempt to acquire the lock within a timeout period.
 */
AK_API AK_int akae_mutex_timed_lock (AK_Mutex handle, AK_ssize us);


/**
 * Release the lock.\n
 * Paired with @ref akae_mutex_lock() or @ref akae_mutex_try_lock().
 */
AK_API AK_int akae_mutex_unlock (AK_Mutex handle);




/**
 * Quick call for entering a mutex-protected block.
 */
#define AK_MUTEX_LOCK_PROC(__handle, __proc_exp...) \
	do {\
		if (0 == (__handle)) { /* No need to enter the mutex critical section.*/\
			__proc_exp;\
		} else if (AK_OK == akae_mutex_lock(__handle)) {/* Operate after entering the mutex critical section.*/\
			__proc_exp;\
			akae_mutex_unlock (__handle);\
		} else {\
			akae_log_error ("Mutex Lock @%s() Error.", __func__);/* Report error if entering the mutex critical section fails.*/\
		}\
	} while (0)



/**
 * Global critical section mutex enter/leave flag.
 */
AK_API AK_void akae_mutex_global_critical (AK_boolean enter);

/**
 *
 * Enter the critical section.
 *
 * @post akae_mutex_leave_critical()
 */
#define akae_mutex_enter_critical() \
	do {\
		/* akae_log_debug2 ("Enter Critical."); AK_CHECK_POINT (); */\
		akae_mutex_global_critical(AK_true);\
	} while (0)

/**
 * @pre akae_mutex_enter_critical()
 * Leave the critical section; call this after the critical section operations are complete.
 */
#define akae_mutex_leave_critical() \
	do {\
		akae_mutex_global_critical(AK_false);\
		/* akae_log_debug2 ("Leave Critical."); AK_CHECK_POINT (); */\
	} while (0)


/**
 * Quick call for entering a critical section block.
 */
#define AK_CRITICAL_PROC(__proc_exp...) \
	do {\
		akae_mutex_enter_critical (); /* Enter the critical section. */\
		do {\
			__proc_exp;\
		} while (0);\
		akae_mutex_leave_critical (); /* Leave the critical section. */ \
	} while (0)




AK_C_HEADER_EXTERN_C_END
#endif ///< MUTEX_H_
