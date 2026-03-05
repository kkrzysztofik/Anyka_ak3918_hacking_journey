/**
 * MD5 hash computation.
 */

#include <akae_object.h>


#if !defined(AKSPC_MD5SUM_H_)
#define AKSPC_MD5SUM_H_
AK_C_HEADER_EXTERN_C_BEGIN


/**
 * MD5 hash result length.
 */
#define AK_MD5SUM_RESULT_LEN (16)



typedef struct _AK_Md5Sum {

	AK_uint32 _magic;     ///< read-only
	AK_uint32 A;           ///< read-only
	AK_uint32 B;           ///< read-only
	AK_uint32 C;           ///< read-only
	AK_uint32 D;           ///< read-only
	AK_uint32 total[2];    ///< read-only
	AK_uint32 buflen;      ///< read-only
	AK_char buffer[128];   ///< read-only
	AK_char base16[AK_MD5SUM_RESULT_LEN * 2 +1]; ///< ///< read-only

} AK_Md5Sum;



/**
 * Initialize an MD5 computation handle.
 *
 *
 * @retval AK_OK
 *  Initialization succeeded.
 * @retval AK_ERR_INVAL_OPERATE
 *  Initialization failed; an invalid parameter was passed.
 */
AK_API AK_int akae_md5sum_init (AK_Md5Sum *Md5);


/**
 * Destroy the handle; the handle cannot be used after destruction.
 *
 */
AK_API AK_int akae_md5sum_destroy (AK_Md5Sum *Md5);

/**
 * Incrementally compute a hash over data.
 *
 * @param[IN] byte
 *  Start address in memory of the data to compute.
 * @param[IN] len
 *  Length of the data to compute.
 *
 * @retval AK_OK
 *  Computation succeeded; the current result can be retrieved via @ref akae_md5sum_result().
 * @retval AK_ERR_INVAL_OBJECT
 *  Computation failed; an invalid handle was passed.
 * @retval AK_ERR_INVAL_PARAM
 *  Computation failed; an invalid parameter was passed.
 *
 */
AK_API AK_int akae_md5sum_update (AK_Md5Sum *Md5, AK_voidptr byte, AK_size len);

/**
 * Get the accumulated MD5 computation result.
 *
 * @retval AK_OK
 *  Result retrieved successfully; the result is stored at the memory location pointed to by @ref stack.
 * @retval AK_ERR_INVAL_OBJECT
 *  Failed to retrieve result; an invalid handle was passed.
 * @retval AK_ERR_INVAL_PARAM
 *  Failed to retrieve result; an invalid parameter was passed.
 * @retval AK_ERR_OUT_OF_RANGE
 *  Failed to retrieve result; the @ref stacklen provided is insufficient.
 */
AK_API AK_int akae_md5sum_result (AK_Md5Sum *Md5, AK_bytptr stack, AK_size stacklen);



AK_API AK_chrptr akae_md5sum_result_base16 (AK_Md5Sum *Md5);



/**
 * Shorthand for computing the MD5 of fixed-length data.
 */
#define AK_MD5SUM_BYTES(__byte, __byte_len) \
	({\
		AK_Md5Sum Md5;\
		akae_md5sum_init (&Md5);\
		akae_md5sum_update (&Md5, __byte, __byte_len);\
		akae_md5sum_result_base16 (&Md5);\
	})

/**
 * Shorthand for computing the MD5 of a string.
 */
#define AK_MD5SUM_STRING(__string) AK_MD5SUM_BYTES(__string, akae_strlen (__string))



AK_C_HEADER_EXTERN_C_END
#endif ///< AKSPC_MD5SUM_H_


