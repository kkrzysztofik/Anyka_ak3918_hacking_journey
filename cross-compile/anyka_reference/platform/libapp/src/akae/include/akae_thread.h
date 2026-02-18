
/**
 * Thread control.
 */

#include <akae_object.h>


#if !defined(AKSPC_THREAD_H_)
#define AKSPC_THREAD_H_
AK_C_HEADER_EXTERN_C_BEGIN


/**
 * Thread handle.
 */
typedef AK_int AK_Thread;


/**
 * Invalid thread handle definition.
 */
#define AK_THREAD_INVALID          (-1)

/**
 * Maximum thread name string length.
 */
#define AK_THREAD_MAX_NICKNAME     (16)

/**
 * Maximum number of threads that can be created.
 */
#define AK_THREAD_MAX_BACKLOG      (32)

/**
 * Maximum number of arguments that can be passed in.
 */
#define AK_THREAD_MAX_ARGUMENTS    (32)

/**
 * Default thread priority.
 */
#define AK_THREAD_DEFAULT_PRIO     (0)

/**
 * Highest thread priority; priorities below this value decrease in order.
 */
#define AK_THREAD_HIGHEST_PRIO     (1)

/**
 * Lowest thread priority; priorities above this value increase in order.
 */
#define AK_THREAD_LOWEST_PRIO      (99)


#define AK_THREAD_STAT_IDLE        (AK_MK4CC('i', 'd', 'l', 'e'))
#define AK_THREAD_STAT_TERMINATED  (AK_MK4CC('t', 'e', 'r', 'm'))
#define AK_THREAD_STAT_RUNNING     (AK_MK4CC('r', 'u', 'n', 0))
#define AK_THREAD_STAT_PREPARE     (AK_MK4CC('p', 'r', 'e', 'p'))


/**
 * Get the minimum thread stack size for the current platform.
 */
AK_API AK_size akae_thread_min_stacklen ();
#define AK_THREAD_STACK_MIN        (akae_thread_min_stacklen ())

/**
 * Get the maximum thread stack size for the current platform.
 */
AK_API AK_size akae_thread_max_stacklen ();
#define AK_THREAD_STACK_MAX        (akae_thread_max_stacklen ())

/**
 * Get the default thread stack size for the current platform.
 */
AK_API AK_size akae_thread_default_stacklen ();
#define AK_THREAD_STACK_DEF        (akae_thread_default_stacklen ())


/**
 * Thread implementation definition.\n
 *
 * @param[IN]
 *  th
 *  Thread handle, returned after successful thread creation by @ref akae_thread_create().\n
 *  Internally the handle can be used to call @ref akae_thread_suspend(), @ref akae_thread_terminated(), and similar interfaces.
 *
 * @param[IN]
 *  argc
 *  Number of thread arguments, passed in via @ref akae_thread_create().
 *
 * @param[IN]
 *  argv
 *  Thread argument array; the effective count is given by @ref argc, passed in via @ref akae_thread_create().
 *
 */
typedef AK_void (*AK_ThreadFunc)(AK_Thread th, AK_int argc, AK_voidptr argv[]);


/**
 * Create a thread.\n
 * After the thread is created, the @ref Func thread implementation is called.\n
 * Inside @ref Func, use @ref akae_thread_terminated() to determine whether the thread should continue running.\n
 * If @ref akae_thread_terminated() returns AK_true, the thread should exit,\n
 * and the user returns from @ref Func based on a conditional check.
 *
 * @param[IN]
 *  nickname
 *  Thread nickname used to name the started thread; if AK_null is passed in, the module assigns a default name.
 *
 * @param[IN]
 *  stacklen
 *  Thread stack size.
 *
 * @param[IN]
 *  priority
 *  Thread priority; see @ref AK_THREAD_HIGHEST_PRIO and @ref AK_THREAD_LOWEST_PRIO for the highest and lowest priority definitions.
 *
 * @param[IN]
 *  Func
 *  Thread implementation; the user implements the specific thread logic as needed, which is called back internally.
 *
 * @param[IN]
 *  argc
 *  Number of arguments to pass in; maximum is @ref AK_THREAD_MAX_ARGUMENTS.
 *
 * @param[IN]
 *  argv
 *  Arguments to pass in; only the first @ref argc entries are valid.
 *
 * @return
 *  Returns the thread handle on success, or @ref AK_THREAD_INVALID on failure.
 *
 */
AK_API AK_Thread akae_thread_create (AK_chrcptr nickname,
		AK_size stacklen, AK_size priority,
		AK_ThreadFunc Func, AK_int argc, AK_voidptr argv[]);

/**
 * Simplified quick call for @ref akae_thread_create(),\n
 * using the @ref __Func interface name as the thread name by default.
 */
#define akae_thread_create_default(__Func) \
	akae_thread_create (#__Func, 0, AK_THREAD_DEFAULT_PRIO, __Func, 0, AK_null)

/**
 * Simplified quick call for @ref akae_thread_create(),\n
 * using the @ref __Func interface name as the thread name by default.
 */
#define akae_thread_create_default2(__Func, __argc, __argv) \
	akae_thread_create (#__Func, 0, AK_THREAD_DEFAULT_PRIO, __Func, __argc, __argv)



/**
 * Release a thread.\n
 * Calling this interface sends a termination signal to the thread.\n
 * The user exits from the thread implementation by calling @ref akae_thread_terminated() and getting AK_true.\n
 * For non-detached threads (created with detachable = AK_false in @ref akae_thread_create()),\n
 * this interface waits for the thread implementation to finish before returning.\n
 * For detached threads, this interface returns immediately without waiting for the thread to finish.
 *
 *
 *
 */
AK_API AK_void akae_thread_release (AK_Thread th, AK_boolean wait);

/**
 * Get whether the thread termination flag is set; called inside @ref AK_ThreadFunc,\n
 * used to determine whether to exit the current thread implementation.
 */
AK_API AK_boolean akae_thread_terminated (AK_Thread th);


/**
 * Get the current running status of the thread.
 *
 * @retval AK_THREAD_STAT_IDLE
 *  Thread is suspended and sleeping.
 * @retval AK_THREAD_STAT_RUNNING
 *  Thread is running.
 * @retval AK_THREAD_STAT_PREPARE
 *  Thread is being prepared; it may have just been created and not yet fully started.
 * @retval AK_THREAD_STAT_TERMINATED
 *  Thread has terminated, or an invalid thread handle may have been passed in.
 *
 */
AK_API AK_int akae_thread_status (AK_Thread th);

/**
 * Thread suspend operation, releases CPU resources.\n
 * Only called inside the thread implementation;\n
 * returns immediately when an external request to release the thread is made.
 *
 * @param[IN]
 *  s
 *  Number of seconds to suspend.
 * @param[IN]
 *  ms
 *  Number of milliseconds to suspend.
 * @param[IN]
 *  us
 *  Number of microseconds to suspend.
 *
 */
AK_API AK_void akae_thread_suspend (AK_Thread th, AK_size s, AK_size ms, AK_size us);



/**
 * Set the thread nickname.\n
 * If a AK_null or empty string is set,\n
 * the interface assigns a default nickname.\n
 * The maximum nickname length is @ref AK_THREAD_MAX_NICKNAME.\n
 * Nicknames exceeding the maximum length are truncated at that limit.
 *
 * @param[IN]
 *  Object
 *  Object handle.
 *
 * @param[IN]
 *  nickname
 *  Thread nickname to set; passing AK_null or an empty string resets the nickname to its default value.
 *
 * @retval AK_ERR_INVAL_OBJECT
 *  Set error; an invalid object handle was passed in.
 * @retval AK_OK
 *  Set succeeded.
 *
 */
AK_API AK_int akae_thread_set_nickname (AK_Thread th, AK_chrcptr nickname);

/**
 * Get the thread nickname; see @ref akae_thread_set_nickname().
 * Returns the nickname content via stack memory @ref stack.
 *
 */
AK_API AK_chrptr akae_thread_get_nickname (AK_Thread th, AK_chrptr stack, AK_size stacklen);

/**
 * Iterate over all threads and terminate them one by one.\n
 * This interface blocks until all threads have exited. It is recommended to call this interface before the user process confirms its exit, to ensure all threads have exited cleanly.\n
 * During this period, calling @ref akae_thread_create() and similar functions is mutually exclusive; therefore, before this interface returns,\n
 * the application must ensure no other methods are called.
 *
 */
AK_API AK_void akae_thread_waitall ();

AK_C_HEADER_EXTERN_C_END
#endif ///< THREAD_H_
