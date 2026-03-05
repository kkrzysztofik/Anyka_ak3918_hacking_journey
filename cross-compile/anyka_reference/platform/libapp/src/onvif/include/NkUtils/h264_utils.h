/**
 * H.264 processing utility methods.
 */

#include <NkUtils/types.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>


#if !defined(NK_UTILS_H264_UTILS_H_)
#define NK_UTILS_H264_UTILS_H_
NK_CPP_EXTERN_BEGIN

/**
 * Get the H.264 NALU start code length.
 *
 * @param[in]			nalu			NALU data.
 * @param[in]			nalu_len		NALU data length.
 * @param[out]			start_code_len	NALU start code length, typically 3 or 4.
 *
 * @return		Returns 0 on success, -1 on failure.
 *
 */
NK_API NK_Int
NK_H264Utils_StartCodeSize(NK_PByte nalu, NK_Size nalu_len, NK_Size *start_code_len);

/**
 * Strip the H.264 NALU start code.
 *
 * @param[in]			nalu			NALU data.
 * @param[in]			nalu_len		NALU data length.
 * @param[out]			nalu_raw		Address offset of NALU after start code is removed.
 * @param[out]			nalu_raw_len	Length of NALU after start code is removed.
 *
 * @return		Returns 0 on success, -1 on failure (NALU may be invalid).
 */
NK_API NK_Int
NK_H264Utils_StripStartCode(NK_PByte nalu, NK_Size nalu_len, NK_PByte *nalu_raw, NK_Size *nalu_raw_len);

/**
 * Search memory for H.264 NALU start code to locate NALU position.
 *
 * @param[in]			mem				Memory data region.
 * @param[in]			len				Memory data region length.
 * @param[out]			start_code		NALU start code position.
 * @param[out]			start_code_len	NALU start code length.
 *
 * @return		Returns 0 on success, -1 on failure (no NALU data found in region).
 */
NK_API NK_Int
NK_H264Utils_SeekStartCode(NK_PVoid mem, NK_Size len, NK_PByte *start_code, NK_Size *start_code_len);


/**
 * Locate a single NALU in memory.
 *
 * @param[in]			mem				Memory data region.
 * @param[in]			len				Memory data region length.
 * @param[out]			nalu			NALU data.
 * @param[out]			start_code_len	NALU start code length.
 * @param[out]			nalu_len		NALU data length.
 *
 * @return		Returns 0 on success, -1 on failure.
 */
NK_API NK_Int
NK_H264Utils_ExtractNALU(NK_PVoid mem, NK_Size len, NK_PByte *nalu, NK_Size *nalu_start_code_len, NK_Size *nalu_len);


NK_CPP_EXTERN_END
#endif /* NK_UTILS_H264_UTILS_H_ */
