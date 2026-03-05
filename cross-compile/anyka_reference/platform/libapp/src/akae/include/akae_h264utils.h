/**
 * H.264 processing related operation methods.
 */

#include <akae_typedef.h>
#include <akae_log.h>


#if !defined(AKAE_H264UTILS_H_)
#define AKAE_H264UTILS_H_
AK_C_HEADER_EXTERN_C_BEGIN


/**
 * Get the NALU start code length. Result is 3 (short start code) or 4 (long start code).
 * Returns length 0 on failure, meaning the memory data is abnormal or no H.264 start code was found.
 *
 */
AK_API AK_size akae_h264_size_start_code (AK_bytptr nalu, AK_size len);


/**
 * Remove the start code from NALU data, returning the NALU data memory location
 * without the start code via @ref ns_nalu and @ref ns_len.
 *
 * @retval AK_OK
 *  Operation succeeded.
 * @retval AK_ERR_INVAL_PARAM
 *  Operation failed, possibly due to invalid parameters passed in.
 */
AK_API AK_int akae_h264_nalu_remove_start_code (AK_bytptr nalu, AK_size len, AK_bytptr *ns_nalu, AK_size *ns_len);


/**
 * Search for the H.264 NALU start code in memory @ref mem to locate the NALU position.
 * Returns the memory start position where the start code was found, and the length of the start code.
 *
 * @param[IN] mem
 *  Start address of the memory region to search.
 * @param[IN] len
 *  Length of the memory region to search.
 * @param[OUT] s
 *  Returns the start position of the found start code.
 * @param[OUT] slen
 *  Returns the length of the found start code.
 *
 * @retval AK_OK
 *  Successfully found a NALU start code.
 * @retval AK_ERR_INVAL_PARAM
 *  Search failed, possibly due to invalid parameters passed in.
 * @retval AK_ERR_NOT_EXIST
 *  Search failed, start code not found.
 *
 */
AK_API AK_int akae_h264_seize_start_code (AK_voidptr mem, AK_size len, AK_bytptr *s, AK_size *slen);



/**
 * Search for H.264 NALU data position in memory @ref mem.
 * After finding it, the result is returned via @ref nalu and @ref nalu_len.
 *
 * @param[IN] mem
 *  Start address of the memory region to search.
 * @param[IN] len
 *  Length of the memory region to search.
 * @param[OUT] nalu
 *  Returns the memory start address of the NALU data.
 * @param[OUT] nalu_len
 *  Returns the length of the NALU data.
 * @param[OUT] nalu_slen
 *  Returns the length of the NALU start code.
 *
 * @retval AK_OK
 *  Successfully found a NALU start code.
 * @retval AK_ERR_INVAL_PARAM
 *  Search failed, possibly due to invalid parameters passed in.
 * @retval AK_ERR_NOT_EXIST
 *  Search failed, start code not found.
 */
AK_API AK_int akae_h264_seize_nalu (AK_voidptr mem, AK_size len, AK_bytptr *nalu, AK_size *nalu_len, AK_size *nalu_slen);


AK_C_HEADER_EXTERN_C_END
#endif ///< AKAE_H264UTILS_H_
