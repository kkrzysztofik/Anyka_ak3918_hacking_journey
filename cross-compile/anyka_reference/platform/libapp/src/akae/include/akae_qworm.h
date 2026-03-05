
/**
 * Queue Write One Read Many (QWORM) module.
 */

#include <akae_object.h>


#if !defined(AKAE_QWORN_H_)
#define AKAE_QWORN_H_
AK_C_HEADER_EXTERN_C_BEGIN


/**
 * Queue reader handle.
 */
#define AK_QwormReader AK_handle


/**
 * Create a queue and return its object handle.
 */
AK_API AK_Object akae_qworm_create (AK_Object Malloc, AK_size max_reader);

/**
 * Release a queue.
 *
 */
AK_API AK_int akae_qworm_release (AK_Object Object);

/**
 * Request to write data to the queue.\n
 * The write process supports chunked data writing. The caller first invokes this interface with the
 * relevant data parameters to initiate the request.\n
 * After the request succeeds, @ref akae_qworm_write_chunk() may be called multiple times to write
 * data in chunks,\n
 * until @ref akae_qworm_submit_write() is called to commit and complete one full write.\n
 * This interface is thread-safe, so it can be called from multiple threads for single-writer queue access.\n
 * However, since data ordering may become disrupted, it is not recommended by design to call this from
 * multiple threads simultaneously.
 *
 * @param[IN] Object
 *  Object handle, obtained from @ref akae_qworm_create().
 *
 * @param[IN] seekable
 *  Seekable flag; marks this data as seekable, used as a boundary for queue recycling.
 *
 * @param[IN] size
 *  Requested write data length. After a successful request, @ref akae_qworm_write_chunk() may be called
 *  multiple times within this length limit;\n
 *  the total written size must not exceed this value.
 *
 * @param[IN] timestamp
 *  Data timestamp indicating when the data was produced; in principle this is a monotonically increasing
 *  number in microseconds.\n
 *  Passing 0 causes the module to use the system time to generate a timestamp for this data.
 *
 * @param[IN] timeo_us
 *  Timeout wait: passing -1 means wait indefinitely, passing 0 means return immediately without waiting,
 *  passing a value greater than 0 means wait for that many microseconds.\n
 *  In the latter two cases, if the wait times out, @ref AK_ERR_BUSY is returned.
 *
 * @retval AK_OK
 *  Request succeeded; @ref akae_qworm_write_chunk() can now be called to write data to the queue.
 *
 * @retval AK_ERR_INVAL_OBJECT
 *  Invalid object handle.
 *
 * @retval AK_ERR_NOT_PERM
 *  Operation not permitted; the queue may have no seekable data yet, requiring the caller to first
 *  insert a seekable entry.
 *
 * @retval AK_ERR_BUSY
 *  Request timed out; another caller may currently be writing to the queue via this interface in
 *  another thread.
 *
 * @retval AK_ERR_OUT_OF_MEM
 *  Insufficient memory to allocate and write this amount of data to the queue.
 *
 */
AK_API AK_int akae_qworm_request_write (AK_Object Object, AK_boolean seekable, AK_size size, AK_size64 timestamp,  AK_int timeo_us);

/**
 * See @ref akae_qworm_request_write(); synchronous request shorthand.
 */
#define akae_qworm_wait_write(__Object, __seekable, __size, __timestamp) \
	akae_qworm_request_write (__Object, __seekable, __size, __timestamp, -1)

/**
 * See @ref akae_qworm_request_write(); asynchronous request shorthand.
 */
#define akae_qworm_try_write(__Object, __seekable, __size, __timestamp) \
	akae_qworm_request_write (__Object, __seekable, __size, __timestamp, 0)

/**
 * Write a data chunk.
 * This interface must be called after a successful @ref akae_qworm_request_write() call.
 *
 * @param Object
 * @param data
 * @param datalen
 * @return
 */
AK_API AK_int akae_qworm_write_chunk (AK_Object Object, AK_voidptr data, AK_size datalen);

/**
 * Submit the write.
 *
 * @param Object
 * @return
 */
AK_API AK_int akae_qworm_submit_write (AK_Object Object);

/**
 * Single-shot data write shorthand.
 */
#define akae_qworm_write(__Object, __seekable, __timestamp,  __data, __datalen) \
	do {\
		if (AK_OK == akae_qworm_wait_write(__Object, __seekable, __datalen, __timestamp)) {\
			akae_qworm_write_chunk (__Object, __data, __datalen);\
			akae_qworm_submit_write (__Object);\
		}\
	} while (0)


/**
 * Acquire a reader from the queue.\n
 * The caller can obtain a reader handle to read data from the queue, implementing one-writer
 * many-reader logic.\n
 * The maximum number of readers is specified in @ref akae_qworm_create().\n
 * If the number of acquired readers exceeds this limit, @ref AK_ERR_OUT_OF_RANGE is returned.\n
 * This interface is not thread-safe; thread safety must be ensured by the caller.
 *
 * @param[IN] Object
 *  Queue object handle, created via @ref akae_qworm_create().
 *
 * @return
 *  Returns a reader handle (greater than 0) on success; returns the corresponding error code on failure.
 *
 * @retval AK_OK
 *  Acquisition succeeded.
 *
 * @retval AK_ERR_INVAL_OBJECT
 *  Invalid object handle.
 *
 * @retval AK_ERR_OUT_OF_RANGE
 *  The number of acquired readers exceeds the limit set by @ref akae_qworm_create().
 *
 * @retval AK_ERR_BUSY
 *  The system handle pool is saturated; no system handle can be allocated.
 *
 */
AK_API AK_QwormReader akae_qworm_get_reader (AK_Object Object);

/**
 * Release a reader.\n
 * When the upper layer has finished reading, this interface must be called to release the reader.\n
 * Paired with @ref akae_qworm_get_reader().\n
 * After release, the reader pool inside the queue module recycles the slot for future reader requests.\n
 * Since there is a maximum reader count limit (specified in @ref akae_qworm_create()),\n
 * upper-layer applications must release readers as soon as they are done reading.\n
 * Note: when the reader handle is released, internal resources are freed, so callers must ensure
 * the handle is no longer in use before releasing it.\n
 *
 * @param[IN] reader
 *  Reader handle.
 *
 * @return
 *  Returns AK_OK on success; returns the corresponding error code on failure.
 *
 * @retval AK_OK
 *  Release succeeded.
 *
 * @retval AK_ERR_INVAL_HANDLE
 *  Invalid object handle.
 */
AK_API AK_int akae_qworm_put_reader (AK_QwormReader reader);

/**
 * Seek the reader to the latest seekable data entry.\n
 * Note: if the latest seekable data in the queue is older than the reader's current position,
 * this operation is ignored and still returns @ref AK_OK.
 *
 * @param[IN] reader
 *  Queue reader handle.
 *
 * @retval AK_OK
 *  Operation succeeded.
 * @retval AK_ERR_INVAL_HANDLE
 *  Operation failed; an invalid handle may have been passed.
 *
 *
 */
AK_API AK_int akae_qworm_reader_seek (AK_QwormReader reader);

/**
 * Read one queue entry from the queue. On each successful read, the reader advances to the next entry.\n
 * Continuously calling this interface reads entries in write order, implementing a FIFO queue.\n
 * If the upper layer wants the reader to remain at the current position after reading (e.g., for
 * multiple reads of the same entry), call @ref akae_qworm_reader_peek() instead.
 *
 * @param[IN] reader
 *  Queue reader handle, obtained via @ref akae_qworm_get_reader() or @ref akae_qworm_group().
 *
 * @param[OUT] sequence
 *  Queue data sequence number; returned on success. This is a monotonically increasing integer
 *  incremented by 1 for each new entry written to the queue.
 *
 * @param[OUT] seekable
 *  Seekable flag for the queue entry; returned on success, corresponding to the attribute set
 *  when the data was written.
 *
 * @param[OUT] timestamp
 *  Data timestamp; returned on success, corresponding to the attribute set when the data was written.
 *
 * @param[OUT] data
 *  Start address in memory of the queue data; returned on success, corresponding to the attribute
 *  set when the data was written.
 *
 * @return
 *  Returns the valid length of the data at @ref data on success.\n
 *  Returns 0 if the queue is empty in this reader's session (read speed exceeds write speed).
 *
 * @retval 0
 *  Read failed; the queue is empty. The caller may adjust the reading pace.
 *
 * @retval AK_ERR_INVAL_HANDLE
 *  Read failed; an invalid handle @ref reader was passed.
 *
 */
AK_API AK_ssize akae_qworm_reader_read (AK_QwormReader reader, AK_size *sequence, AK_boolean *seekable, AK_size64 *timestamp, AK_voidptr *data);

/**
 * Shorthand for reading only data via @ref akae_qworm_reader_read().
 */
#define akae_qworm_reader_read_data_only(__reader, __data) \
		akae_qworm_reader_read (__reader, AK_null, AK_null, AK_null, (__data))

/**
 * See @ref akae_qworm_reader_read(). The difference is that after calling this interface,\n
 * the reader's position does not advance; the caller can read the same queue entry multiple times.
 */
AK_API AK_ssize akae_qworm_reader_peek (AK_QwormReader reader, AK_size *sequence, AK_boolean *seekable, AK_size64 *timestamp, AK_voidptr *data);

/**
 * Shorthand for reading only data via @ref akae_qworm_reader_peek().
 */
#define akae_qworm_reader_peek_data_only(__reader, __data) \
		akae_qworm_reader_peek (__reader, AK_null, AK_null, AK_null, (__data))

/**
 * Maximum queue capacity per group.
 */
#define AK_QWORM_GROUP_MAX_QUEUE (4)


/**
 * Create a group reader.\n
 * The caller can use this interface to create a group reader,\n
 * i.e., a single reader handle that reads from multiple queues.\n
 * The read ordering strategy differs by mode: in synchronous mode (i.e., @ref sync is AK_true),\n
 * the reader waits until all queues have data available, then compares timestamps to read the entry
 * with the earliest timestamp.\n
 * This may cause read delays if some queues write data slowly.\n
 * Non-synchronous mode does not compare timestamps; it prioritizes queues that currently have data.
 *
 * On successful return, one reader is acquired from each queue, reducing the available reader count
 * in each queue.
 *
 * @param[IN] n
 *  Number of queues to pass in; if 1, the behavior is the same as @ref akae_qworm_get_reader().\n
 *  A maximum of @ref AK_QWORM_GROUP_MAX_QUEUE queues may be passed.
 *
 * @param[IN] Queue
 *  One or more queue objects; a maximum of @ref AK_QWORM_GROUP_MAX_QUEUE may be passed.
 *
 * @return
 *  Returns a reader handle; the caller can use this handle with reader operations such as
 *  @ref akae_qworm_reader_read().
 */
AK_API AK_QwormReader akae_qworm_group (AK_boolean sync, AK_size n, AK_Object Oueue, ...);

/**
 * Release a group reader; corresponds to @ref akae_qworm_group().
 */
#define akae_qworm_ungroup(__reader) akae_qworm_put_reader(__reader)



/**
 * Get the latest write timestamp of the queue.\n
 * The queue timestamp is updated each time the upper layer calls the write interface
 * @ref akae_qworm_request_write().\n
 * This property can be used to qualitatively assess the current state of the queue.
 *
 * @return
 *  The current queue timestamp.
 */
AK_API AK_size64 akae_qworm_property_timestamp_written (AK_Object Object);

/**
 * Get the queue data write throughput in bytes per second (bytes/s).
 */
AK_API AK_size akae_qworm_property_byterate_written (AK_Object Object);

/**
 * Total bytes written to the queue over its lifetime.
 */
AK_API AK_size akae_qworm_property_bytes_written (AK_Object Object);

/**
 * Queue memory usage; qualitatively reflects the current media memory overhead.
 */
AK_API AK_size akae_qworm_property_memory_usage (AK_Object Object);



AK_C_HEADER_EXTERN_C_END
#endif ///< MUTEX_H_
