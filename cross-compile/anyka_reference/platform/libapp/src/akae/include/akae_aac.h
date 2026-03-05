/**
 * AAC processing related operation methods.
 */

#include <akae_typedef.h>
#include <akae_log.h>


#if !defined(AKAE_AAC_H_)
#define AKAE_AAC_H_
AK_C_HEADER_EXTERN_C_BEGIN

/**
 * Enum values corresponding to AK_AacAdtsFixedHeader::samplingFreqIndex.
 */
enum {

	AK_AAC_SAMPLE_FREQ_96000 = 0,
	AK_AAC_SAMPLE_FREQ_88200,
	AK_AAC_SAMPLE_FREQ_64000,
	AK_AAC_SAMPLE_FREQ_48000,
	AK_AAC_SAMPLE_FREQ_44100,
	AK_AAC_SAMPLE_FREQ_32000,
	AK_AAC_SAMPLE_FREQ_24000,
	AK_AAC_SAMPLE_FREQ_22050,
	AK_AAC_SAMPLE_FREQ_16000,
	AK_AAC_SAMPLE_FREQ_12000,
	AK_AAC_SAMPLE_FREQ_11025,
	AK_AAC_SAMPLE_FREQ_8000,

	AK_AAC_SAMPLE_FREQ_RESV = 0xf,

};

enum {

	AK_AAC_CH_DEF_IN_AOT_SPEC_CONFIG = 0, ///< Defined in AOT Specifc Config
	AK_AAC_CH1_FC,                   ///< 1 channel: front-center
	AK_AAC_CH2_FLR,                  ///< 2 channels: front-left, front-right
	AK_AAC_CH3_FCLR,                 ///< 3 channels: front-center, front-left, front-right
	AK_AAC_CH4_FCLR_BC,              ///< 4 channels: front-center, front-left, front-right, back-center
	AK_AAC_CH5_FCLR_BLR,             ///< 5 channels: front-center, front-left, front-right, back-left, back-right
	AK_AAC_CH6_FCLR_BCR,             ///< 6 channels: front-center, front-left, front-right, back-left, back-right, LFE-channel
	AK_AAC_CH8_FCLR_SLR_BCR,         ///< 8 channels: front-center, front-left, front-right, side-left, side-right, back-left, back-right, LFE-channel
	AK_AAC_CH_DEF_RESV = 0x0f,

};




/**
 * AAC single-frame ADTS length.
 */
#define AK_AAC_ADTS_SZ  (7)


#pragma pack(push, 1)

typedef struct _AK_AacAdtsFixedHeader {

    AK_uint32 syncword;  //12 bit sync word '1111 1111 1111', marks the start of an ADTS frame
    AK_uint32 id;        //1 bit MPEG identifier: 0 for MPEG-4, 1 for MPEG-2
    AK_uint32 layer;     //2 bit always '00'
    AK_uint32 protectionAbsent;  //1 bit 1 means no CRC, 0 means CRC present
    AK_uint32 profile;           //2 bit indicates which AAC profile level is used
    AK_uint32 samplingFreqIndex; //4 bit indicates the sampling frequency used
    AK_uint32 privateBit;        //1 bit
    AK_uint32 channelCfg; //3 bit indicates the number of channels
    AK_uint32 originalCopy;         //1 bit
    AK_uint32 home;                  //1 bit

} AK_AacAdtsFixedHeader;


typedef struct _AK_AacAdtsVariableHeader {

    /*The following parameters vary per frame*/
    AK_uint32 copyrightIdentificationBit;   //1 bit
    AK_uint32 copyrightIdentificationStart; //1 bit
    AK_uint32 aacFrameLength;               //13 bit total ADTS frame length including ADTS header and raw AAC stream
    AK_uint32 adtsBufferFullness;           //11 bit 0x7FF indicates variable bitrate stream

    /* number_of_raw_data_blocks_in_frame
     * Indicates that there are number_of_raw_data_blocks_in_frame + 1 raw AAC frames in the ADTS frame.
     * So number_of_raw_data_blocks_in_frame == 0
     * means there is one AAC data block in the ADTS frame, not zero.
     * (One raw AAC frame contains 1024 samples and associated data for a period of time.)
     */
    AK_uint32 numberOfRawDataBlockInFrame; //2 bit

} AK_AacAdtsVariableHeader;


/**
 * Print the @ref AK_AacAdtsFixedHeader and @ref AK_AacAdtsVariableHeader object structures.
 */
#define AK_AAC_ADTS_HEADER_DUMP(__FixedHdr, __VarHdr) \
	do {\
		AK_VerboseForm Form;\
		akae_verbose_form_init (&Form, "AAC ADTS Header", 64, 4);\
		if (AK_null != (__FixedHdr)) {\
			akae_verbose_form_put_text (&Form, AK_true,   "Fixed Header");\
			akae_verbose_form_put_kv (&Form, AK_false,     "                id",       "%u", (AK_uint32)(__FixedHdr)->id               );\
			akae_verbose_form_put_kv (&Form, AK_false,     "             layer",       "%u", (AK_uint32)(__FixedHdr)->layer            );\
			akae_verbose_form_put_kv (&Form, AK_false,     "  protectionAbsent",       "%u", (AK_uint32)(__FixedHdr)->protectionAbsent );\
			akae_verbose_form_put_kv (&Form, AK_false,     "           profile",       "%u", (AK_uint32)(__FixedHdr)->profile          );\
			akae_verbose_form_put_kv (&Form, AK_false,     " samplingFreqIndex",       "%u", (AK_uint32)(__FixedHdr)->samplingFreqIndex);\
			akae_verbose_form_put_kv (&Form, AK_false,     "        privateBit",       "%u", (AK_uint32)(__FixedHdr)->privateBit       );\
			akae_verbose_form_put_kv (&Form, AK_false,     "        channelCfg",       "%u", (AK_uint32)(__FixedHdr)->channelCfg       );\
			akae_verbose_form_put_kv (&Form, AK_false,     "      originalCopy",       "%u", (AK_uint32)(__FixedHdr)->originalCopy     );\
			akae_verbose_form_put_kv (&Form, AK_true,      "              home",       "%u", (AK_uint32)(__FixedHdr)->home             );\
		}\
		if (AK_null != (__VarHdr)) {\
			akae_verbose_form_put_text (&Form, AK_true,   "Variable Header");\
			akae_verbose_form_put_kv (&Form, AK_false,     " copyrightIdentificationStart",  "%u",      (AK_uint32)(__VarHdr)->copyrightIdentificationStart  );\
			akae_verbose_form_put_kv (&Form, AK_false,     "               aacFrameLength",  "%u",      (AK_uint32)(__VarHdr)->aacFrameLength                );\
			akae_verbose_form_put_kv (&Form, AK_false,     "           adtsBufferFullness",  "%u/0x%x", (AK_uint32)(__VarHdr)->adtsBufferFullness, (AK_uint32)(__VarHdr)->adtsBufferFullness );\
			akae_verbose_form_put_kv (&Form, AK_false,     "  numberOfRawDataBlockInFrame",  "%u(%u)",  (AK_uint32)(__VarHdr)->numberOfRawDataBlockInFrame, (AK_uint32)(__VarHdr)->numberOfRawDataBlockInFrame + 1 );\
		}\
		akae_verbose_form_finish (&Form);\
	} while (0)


#pragma pack(pop)


/**
 * Parse one AAC frame and extract the ADTS.
 * The input data must be a complete AAC-ADTS frame structure, so @ref datalen must be
 * greater than or equal to one full frame length; otherwise, after parsing the ADTS,
 * a length mismatch will be detected and an error will be returned.
 *
 * @param[IN] data
 *  Start address of the AAC-ADTS binary frame data in memory.
 * @param[IN] datalen
 *  Length of the AAC-ADTS binary frame data.
 * @param[OUT] FixHdr
 *  ADTS data parsed from the frame.
 *
 * @return
 *  On success, the ADTS attributes for this frame are returned via @ref Adts, and the
 *  return value is the frame length (including ADTS length).\n
 *  The caller can offset by @ref AK_AAC_ADTS_SZ to access the raw AAC payload without the ADTS header.
 *
 * @retval AK_ERR_INVAL_PARAM
 *  Invalid parameters passed in.
 * @retval AK_ERR_INVAL_OBJECT
 *  Input data error; @ref data is not valid AAC-ADTS data.
 * @retval AK_ERR_OUT_OF_RANGE
 *  Frame length error; differs from the length specified in the ADTS header.
 *
 */
AK_API AK_ssize akae_aac_disset_frame (AK_bytptr data, AK_size datalen, AK_AacAdtsFixedHeader *FixHdr, AK_AacAdtsVariableHeader *VarHdr);

/**
 * Get the AAC sampling rate by index, where the index corresponds to AK_AacAdtsFixedHeader::samplingFreqIndex.
 *
 * @return
 *  Returns the corresponding sampling rate (in Hz) on success; returns 0 if the index is invalid.
 */
AK_API AK_size akae_aac_get_sample_rate_by_index (AK_int id);

/**
 * Get the AAC sampling rate index by sampling rate; corresponds to @ref akae_aac_get_sample_rate_by_index().
 *
 * @return
 *  Returns the corresponding attribute index for the given sampling rate;
 *  returns @ref AK_AAC_SAMPLE_FREQ_RESERVED if the rate is not in the valid list.
 */
AK_API AK_size akae_aac_get_sample_rate_index (AK_int rate);

/**
 *
 * @return
 *  Returns the corresponding channel count on success; returns 0 if the index is invalid.
 */
AK_API AK_size akae_aac_get_channels_by_index (AK_int idx);


/**
 *
 * @return
 *  Returns the corresponding attribute index for the given channel count;
 *  returns @ref AK_AAC_CH_DEF_IN_AOT_SPEC_CONFIG if the channel count is not in the valid list.
 */
AK_API AK_size akae_aac_get_channels_index (AK_int channels);




AK_C_HEADER_EXTERN_C_END
#endif ///< AKAE_AAC_H_
