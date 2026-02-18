
#include <akae_typedef.h>


#if !defined (AKAE_BASE64_H_)
#define AKAE_BASE64_H_
AK_C_HEADER_EXTERN_C_BEGIN

/**
 * BASE64 encode.
 *
 * @return
 *  Returns the length of the encoded result on success, storing the result at the memory location
 *  pointed to by @ref stack; returns the corresponding error code on failure.
 *
 * @retval 0
 *  Encoding failed; encoding data of length 0 returns length 0, indicating no data to encode.
 * @retval AK_ERR_INVAL_PARAM
 *  Encoding failed; an invalid parameter may have been passed.
 * @retval AK_ERR_OUT_OF_RANGE
 *  Encoding failed; insufficient output memory space.
 *
 */
AK_API AK_ssize akae_base64_encode (AK_voidptr data, AK_size datalen, AK_chrptr stack, AK_size stacklen);

/**
 * BASE64 decode.
 *
 *
 * @return
 *  Returns the length of the decoded result on success, storing the result at the memory location
 *  pointed to by @ref stack; returns the corresponding error code on failure.
 * @retval 0
 *  Decoding failed; decoding data of length 0 returns length 0, indicating no data to decode.
 * @retval AK_ERR_INVAL_PARAM
 *  Decoding failed; an invalid parameter may have been passed.
 * @retval AK_ERR_INVAL_OPERATE
 *  Decoding failed; the input length does not match a decodable length.
 * @retval AK_ERR_OUT_OF_RANGE
 *  Decoding failed; insufficient output memory space.
 *
 */
AK_API AK_ssize akae_base64_decode (AK_chrptr base64, AK_voidptr stack, AK_size stacklen);

AK_C_HEADER_EXTERN_C_END
#endif ///< AKAE_BASE64_H_
