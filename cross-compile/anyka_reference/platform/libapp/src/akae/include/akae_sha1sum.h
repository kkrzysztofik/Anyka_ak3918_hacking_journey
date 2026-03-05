/**
 * SHA1 hash computation.
 */

#include <akae_object.h>


#if !defined(AKAE_SHA1SUM_H_)
#define AKAE_SHA1SUM_H_
AK_C_HEADER_EXTERN_C_BEGIN


/**
 * SHA1 hash computation result length.
 */
#define AK_SHA1SUM_RESULT_LEN (20)


typedef struct _AK_Sha1Sum {

	AK_uint32 _magic;
    AK_uint32 state[5];
    AK_uint32 count[2];
    AK_byte buffer[64];
    AK_char base16[AK_SHA1SUM_RESULT_LEN * 2 + 1];

} AK_Sha1Sum;


/**
 * Initialize a SHA1 computation handle.
 *
 * @retval AK_OK
 *  Initialization succeeded.
 * @retval AK_ERR_INVAL_OPERATE
 *  Initialization failed due to invalid parameter.
 */
AK_API AK_int akae_sha1sum_init (AK_Sha1Sum *Sha1);


/**
 * Destroy the handle; after destruction the handle can no longer be used.
 *
 */
AK_API AK_int akae_sha1sum_destroy (AK_Sha1Sum *Sha1);

/**
 * Accumulate hash data for computation.
 *
 * @param[IN] byte
 *  Starting address of the data to be processed.
 * @param[IN] len
 *  Length of the data to be processed.
 *
 * @retval AK_OK
 *  Computation succeeded; the current result can be retrieved via @ref akae_sha1sum_result().
 * @retval AK_ERR_INVAL_OBJECT
 *  Computation failed; an invalid handle was passed in.
 * @retval AK_ERR_INVAL_PARAM
 *  Computation failed; an invalid parameter was passed in.
 *
 */
AK_API AK_int akae_sha1sum_update (AK_Sha1Sum *Sha1, AK_voidcptr byte, AK_size len);

/**
 * Retrieve the accumulated SHA1 computation result.
 *
 * @retval AK_OK
 *  Result retrieved successfully; the result is stored at the memory location indicated by @ref stack.
 * @retval AK_ERR_INVAL_OBJECT
 *  Failed to retrieve result; an invalid handle was passed in.
 * @retval AK_ERR_INVAL_PARAM
 *  Failed to retrieve result; an invalid parameter was passed in.
 * @retval AK_ERR_OUT_OF_RANGE
 *  Failed to retrieve result; the @ref stacklen length passed in is insufficient.
 */
AK_API AK_int akae_sha1sum_result (AK_Sha1Sum *Sha1, AK_bytptr stack, AK_size stacklen);


AK_API AK_chrptr akae_sha1sum_result_base16 (AK_Sha1Sum *Sha1);


/**
 * Quick call for SHA1 computation on fixed-length data.
 */
#define AK_SHA1SUM_BYTES(__byte, __byte_len) \
	({\
		AK_Sha1Sum Sha1;\
		akae_sha1sum_init (&Sha1);\
		akae_sha1sum_update (&Sha1, __byte, __byte_len);\
		akae_sha1sum_result_base16 (&Sha1);\
	})

/**
 * Quick call for SHA1 computation on a string.
 */
#define AK_SHA1SUM_STRING(__string) AK_SHA1SUM_BYTES(__string, akae_strlen (__string))



AK_C_HEADER_EXTERN_C_END
#endif ///< AKAE_SHA1SUM_H_

