/**
* @file	sdcodec.h
* @brief	Anyka Sound Device Module interfaces header file.
*
* This file declare Anyka Sound Device Module interfaces.\n
* Copyright (C) 2014 Anyka (Guangzhou) Microelectronics Technology Co., Ltd.
* @author	Deng Zhou
* @date	2014-02-21
* @version V0.0.1
* @ref
*/

#ifndef __SOUND_DEVICE_CODEC_H__
#define __SOUND_DEVICE_CODEC_H__

#include "medialib_global.h"

#ifdef __cplusplus
extern "C" {
#endif


/** @defgroup AUDIOLIB Audio library
 * @ingroup ENG
 */
/*@{*/


/* @{@name Define audio version*/
/**	Use this to define version string */	
#define AUDIOCODEC_VERSION_STRING		(T_U8 *)"AudioCodec Version V1.16.00_svn5418"
/** @} */
 
#ifdef _WIN32
// #define _SD_MODULE_MIDI_SUPPORT
#define _SD_MODULE_MP3_SUPPORT
#define _SD_MODULE_ENC_MP3_SUPPORT
#define _SD_MODULE_WMA_SUPPORT
#define _SD_MODULE_APE_SUPPORT
#define _SD_MODULE_FLAC_SUPPORT
#define _SD_MODULE_PCM_SUPPORT
#define _SD_MODULE_ADPCM_SUPPORT
#define _SD_MODULE_ENC_ADPCM_SUPPORT
#define _SD_MODULE_AAC_SUPPORT
#define _SD_MODULE_OGG_VORBIS_SUPPORT
#define _SD_MODULE_AMR_SUPPORT
#define _SD_MODULE_AMR_ENC_SUPPORT
#define _SD_MODULE_ENC_AAC_SUPPORT
#define _SD_MODULE_RA8LBR_SUPPORT
#define _SD_MODULE_DRA_SUPPORT
#define _SD_MODULE_AC3_SUPPORT
#define _SD_MODULE_G711_SUPPORT
#define _SD_MODULE_G711_ENC_SUPPORT
#define _SD_MODULE_SBC_SUPPORT
#define _SD_MODULE_SBC_ENC_SUPPORT
#define _SD_MODULE_SPEEX_SUPPORT
#define _SD_MODULE_SPEEX_ENC_SUPPORT
#define _SD_MODULE_SPEEX_WB_SUPPORT
#define _SD_MODULE_SPEEX_WB_ENC_SUPPORT
#define _SD_MODULE_GETSPECTRUM_SUPPORT
#endif 

#define _SD_AUDEC_VOLDB_Q 10

typedef enum
{
	_SD_MEDIA_TYPE_UNKNOWN ,
	_SD_MEDIA_TYPE_MIDI ,
	_SD_MEDIA_TYPE_MP3 ,
	_SD_MEDIA_TYPE_AMR ,
	_SD_MEDIA_TYPE_AAC ,
	_SD_MEDIA_TYPE_WMA ,
	_SD_MEDIA_TYPE_PCM ,
	_SD_MEDIA_TYPE_ADPCM_IMA ,
	_SD_MEDIA_TYPE_ADPCM_MS ,
	_SD_MEDIA_TYPE_ADPCM_FLASH ,
	_SD_MEDIA_TYPE_APE ,
	_SD_MEDIA_TYPE_FLAC ,
	_SD_MEDIA_TYPE_OGG_FLAC ,
	_SD_MEDIA_TYPE_RA8LBR ,
	_SD_MEDIA_TYPE_DRA,
	_SD_MEDIA_TYPE_OGG_VORBIS,
	_SD_MEDIA_TYPE_AC3,
	_SD_MEDIA_TYPE_PCM_ALAW,
	_SD_MEDIA_TYPE_PCM_ULAW,
	_SD_MEDIA_TYPE_SBC,
    _SD_MEDIA_TYPE_MSBC,
	_SD_MEDIA_TYPE_SPEEX,
	_SD_MEDIA_TYPE_SPEEX_WB,
	_SD_MEDIA_TYPE_OPUS	
}T_AUDIO_TYPE;

typedef enum
{
	_SD_BUFFER_FULL ,
	_SD_BUFFER_WRITABLE ,
	_SD_BUFFER_WRITABLE_TWICE ,
	_SD_BUFFER_ERROR
}T_AUDIO_BUF_STATE;

typedef enum
{
	_STREAM_BUF_LEN = 0,
	_STREAM_BUF_REMAIN_DATA,
	_STREAM_BUF_MIN_LEN
}T_AUDIO_INBUF_STATE;

typedef enum
{
    _SD_ENC_SAVE_FRAME_HEAD = 0,
    _SD_ENC_CUT_FRAME_HEAD  = 1
}T_AUDIO_ENC_FRMHEAD_STATE;

/* Define the data packet format returned by SPEEX encoding */
typedef enum{
	AKENC_PACK_LENTAG = 0,  //2-byte frame length + encoded data
	AKENC_PACK_OGG = 1,       //OGG container packetization
    AKENC_PACK_RAWDATA,    //pure encoded data with no additional framing information
    AKENC_PACK_LENSYNC,     //1-byte sync word + 1-byte frame length + 1-byte CRC check + encoded data
}T_AKENC_PACKET_FORMAT;
#define SPEEX_PACK_LENTAG 	AKENC_PACK_LENTAG
#define SPEEX_PACK_OGG 		AKENC_PACK_OGG
#define SPEEX_PACK_RAWDATA 	AKENC_PACK_RAWDATA
#define SPEEX_PACK_LENSYNC 	AKENC_PACK_LENSYNC


typedef struct
{
	MEDIALIB_CALLBACK_FUN_MALLOC			Malloc;
	MEDIALIB_CALLBACK_FUN_FREE				Free;
	MEDIALIB_CALLBACK_FUN_PRINTF			printf;
	MEDIALIB_CALLBACK_FUN_RTC_DELAY			delay;
	MEDIALIB_CALLBACK_FUN_CMMBSYNCTIME		cmmbsynctime;
	MEDIALIB_CALLBACK_FUN_CMMBAUDIORECDATA  cmmbaudiorecdata;
    MEDIALIB_CALLBACK_FUN_INVALID_DCACHE    invDcache;
}T_AUDIO_CB_FUNS;

typedef struct
{
    T_AUDIO_CB_FUNS cb;
    T_U32	m_Type;
}T_AUDIO_LOG_INPUT;

typedef struct
{
    // in
    // user set the quality of extracted sbc frame
    //  0: (default) high quality, 1: middle quality, 2: half of stereo,
    //  other: set bitpool, see spc spec
    T_U8 g_sbc_extract_bitpool;
    // in
    //  0: (default) encode when necessary. frame sizes may vary.
    //  1: force encode. frame size is constant.
    T_U8 g_sbc_extract_force_encode;

    // out
    // mode of current frame
    //  0: mono, 1: dual, 2: stereo, 3: joint stereo
    T_U8 g_sbc_frame_mode;
    // out
    // extract frame size
    T_S16 g_sbc_extract_frame_size; 
    // out
    // sbc ordinally frame size
    T_S16 g_sbc_frame_size; 
    // out
    // extract frame data buffer
    T_U8 g_sbc_extract_frame_buf[200]; 

}T_AUDIO_SBC_EXTRACT;

typedef struct
{
	T_U32	m_Type;				//media type
	T_U32	m_SampleRate;		//sample rate, sample per second
	T_U16	m_Channels;			//channel number
	T_U16	m_BitsPerSample;	//bits per sample

	T_U32   m_InbufLen;         //input buffer length
	T_U8    *m_szData; 
	T_U32   m_szDataLen;

	union {
		struct
		{
            // cmmb_adts_flag: 
            // bit[1]: whether CMMB recording is supported 
            // bit[2]: whether CMMB SBR decoding is supported
            // bit[3]: whether to skip the A2DP AAC payload header
            // Example:
            //       standard AAC bitstream decode              set to 0;
            //       CMMB without SBR decoding (no recording)  set to 1;
            //       CMMB without SBR decoding + recording     set to 2;
            //       CMMB with SBR decoding (no recording)     set to 4;
            //       CMMB with SBR decoding + recording        set to 6;
            //       A2DP AAC requires skipping payload header  set to 8
			T_U32	cmmb_adts_flag;
		}m_aac;
		struct  
		{
			T_U32	nFileSize;
		} m_midi;
        struct
        {
            T_U32   ExtractFlag; // 0: normal decode (no extract), 1: extract left channel, 2: extract right channel, 3: extract and mix
            /* 
              setSWdec:
              for chips with hardware decode capability, select software or hardware decode -- 0: default hardware decode, 1: force software decode;
              for chips without hardware decode capability, this parameter is ignored and software decode is always used.
            */
            T_U8    setSWdec; 
            T_AUDIO_SBC_EXTRACT *tExtractStruct;
        }m_sbc;
        struct  
        {
            T_U32	enhancer;
            T_U32	highpass;
            int  headflag; //SPEEX_WB_PACKET_FORMAT
        } m_speexwb;
	}m_Private;
    /*
    To avoid modifying the header file when updating the library for a platform, this is renamed to FOR_SPOTLIGHT 
    because on the Spotlight platform, SBC decoding calls the audio library directly, whereas non-SBC decoding platforms call the media library;
    and since the Spotlight platform has no volume processing, two additional variables are needed for SBC decoding;
    for non-Spotlight platforms, the platform already calls the audio library for decoding directly and already handles volume, so no volume processing is needed in the audio library.
    */
#if 1 //def FOR_SPOTLIGHT //BLUETOOTH_PLAY 
    /* 
    decode volume enable::
    0: no volume control during decoding in the audio library; raw decoded data is output by default
    1: apply volume control during decoding; the volume level is the value of decVolume, i.e. an externally supplied volume multiplier
    2: apply volume control during decoding; the volume level is the value of decVoldb, i.e. an externally supplied dB value
    */
    T_U32  decVolEna;   
    /* 
    Volume multiplier value; assigned as (T_S32)(x.xx*(1<<10)), x.xx=[0.00~7.99]
    It is recommended not to exceed 1.00*(1<<10), as exceeding this may cause data overflow and audio distortion
    */
	T_U32  decVolume;   // decode volume value::   this volume is effective, when decVolCtl==1
    /* 
    Volume in dB; assigned as (T_S32)(x.xx*(1<<10)), x.xx=[-60.00~8.00]
    It is recommended not to exceed 0 dB, as exceeding this may cause data overflow and audio distortion
    If x.xxx<=-79dB, the output will be silent; if x.xxx>8.0, the output may contain noise.
    */
    T_S32 decVoldb;
#endif
}T_AUDIO_IN_INFO;

typedef struct
{
	T_AUDIO_CB_FUNS		cb_fun;
	T_AUDIO_IN_INFO		m_info;
    T_AUDIO_CHIP_ID     chip;

    T_VOID              *ploginInfo;
}T_AUDIO_DECODE_INPUT;

typedef struct
{
	volatile T_U8	*pwrite;	//pointer of write pos
	T_U32	free_len;	//buffer free length
	volatile T_U8	*pstart;	//buffer start address
	T_U32	start_len;	//start free length
}T_AUDIO_BUFFER_CONTROL;

/* AAC pfofile */
typedef enum 
{
    AAC_PROFILE_MP = 0,		/* unsupport */
    AAC_PROFILE_LC = 1,
    AAC_PROFILE_SSR = 2     /* unsupport */
}T_AUDIO_AACPROFILE;

/* AAC stream information */
typedef struct
{
    T_AUDIO_AACPROFILE profile;
    T_S32   sampleRate;
    T_S32   channel;
}T_AUDEC_AACSTREAMINFO;

typedef enum{ AMR_ENC_MR475 = 0,
			AMR_ENC_MR515,
			AMR_ENC_MR59,
			AMR_ENC_MR67,
			AMR_ENC_MR74,
			AMR_ENC_MR795,
			AMR_ENC_MR102,
			AMR_ENC_MR122,

			AMR_ENC_MRDTX,

			AMR_ENC_N_MODES	/* number of (SPC) modes */

			} T_AUDIO_AMR_ENCODE_MODE ;


typedef struct
{
	T_U32	m_Type;			//media type
	T_U16	m_nChannel;		//stereo (2) or mono (1)
	T_U16	m_BitsPerSample;//fixed at 16 bits (16)
	T_U32	m_nSampleRate;	//sample rate (e.g. 8000)
	union{
		struct{
			T_AUDIO_AMR_ENCODE_MODE mode;
		}m_amr_enc;
		struct{
			T_U32 enc_bits;
		}m_adpcm;
		struct{
			T_U32 bitrate;
			T_BOOL mono_from_stereo;
		}m_mp3;
        struct{
            T_U32   bitrate;
            T_U16	 m_nChannelOut;
            T_U8    cutAdtsHead;      //one of the T_AUDIO_ENC_FRMHEAD_STATE enum values; indicates whether the encoder should return the ADTS header
		}m_aac;
        struct{
            // recommanded config:
            //  16 blocks, 8 subbands, allocation_method = loudness,
            //  -------------------------------------------
            //  | channel_mode |   mono    | joint stereo |
            //  | sample_rate  | 44.1 | 48 | 44.1 | 48    |
            //  | bitpool      | 31   | 29 | 53   | 51    |
            //  | frame_length | 70   | 66 | 119  | 115   |
            //  -------------------------------------------
            T_U8 channel_mode; // 0: mono, 1: dual, 2: stereo, 3: joint stereo
            T_U8 blocks; // 4,8,12,16
            T_U8 subbands; // 4, 8
            T_U8 allocation_method; // 0: loudness, 1: snr
            T_U8 bitpool;
        }m_sbc;
		struct{
			T_U32 bitrate;   
			T_BOOL cbr;
			T_BOOL dtx_disable;
			char *comments[64];
		}m_speex;
        struct{
            T_BOOL cbr;  //1-CBR (constant bitrate), 0-VBR (variable bitrate)
            T_BOOL dtx_disable;
            T_U32 bitrate;//target bitrate. 0: auto set(15000).
            T_U32 quality;//[0,10]: set quality, overwrite bitrate; 0xff: auto set.
            T_U32 complexity;//[1,10]: set complexity, overwrite bitrate; 0: auto set.
            T_U32 plctuning;//[0,100],Tell the encoder to optimize encoding for a certain percentage of packet loss
            T_U32 highpass;//Set the high-pass filter on(1) or off(0)
            char *comments[64];
            T_U8  headflag; //T_AKENC_PACKET_FORMAT
        }m_speexwb;
		struct{
			T_U32  bitrate;     
			T_BOOL cbr;  		//1-CBR (constant bitrate), 0-VBR (variable bitrate)
			T_BOOL dtx_enable;  //0-DTX disabled, 1-DTX enabled
			T_S16  application; //2048:VOIP    2049:AUDIO  2051:LOWDELAY OTHERS:error
			T_S16  signalType;  //3001:VOICE  3002:MUSIC  -1000:AUTO   OTHERS:error
			T_S8   complexity;  //0-10
			T_U8   headflag;    //T_AKENC_PACKET_FORMAT; currently only RAWDATA is supported
			T_U32  stacksz;     // stackaddr's memory size
			T_U8   *stackaddr;  //memory for opus encoder stack
		}m_opus;
	}m_private;
	T_U32 encEndFlag;
}T_AUDIO_ENC_IN_INFO;

typedef struct
{
	T_U16	wFormatTag;
	T_U16	nChannels;
	T_U32	nSamplesPerSec;

	union {
		struct {
			T_U32	nAvgBytesPerSec;
			T_U16	nBlockAlign;
			T_U16	wBitsPerSample;
			T_U16	nSamplesPerPacket;
		} m_adpcm;
	}m_Private;
	
}T_AUDIO_ENC_OUT_INFO;

typedef struct
{
	T_VOID *buf_in;
	T_VOID *buf_out;
	T_U32 len_in;
	T_U32 len_out;
}T_AUDIO_ENC_BUF_STRC;

typedef struct
{
	T_AUDIO_CB_FUNS		cb_fun;
	T_AUDIO_ENC_IN_INFO	enc_in_info;
    T_AUDIO_CHIP_ID     chip;

    T_VOID              *ploginInfo;
}T_AUDIO_REC_INPUT;

typedef enum 
{
	_SD_BM_NORMAL = 0,
	_SD_BM_ENDING = 1,
    _SD_BM_LIVE = 1
} T_AUDIO_BUFFER_MODE;


/**
 * @brief	Get the codec library version information.
 * @author	Deng Zhou
 * @date	2008-04-21
 * @param	[in] T_VOID
 * @return	T_S8 *
 * @retval	Returns the library version number
 */
T_S8 *_SD_GetAudioCodecVersionInfo(void);

/**
 * @brief	Get the codec library version information, including the supported codec formats.
 * @author  Tang Xuechai
 * @date	2014-05-05
 * @param	[in] T_AUDIO_CB_FUNS
 * @return	T_S8 *
 * @retval	Returns the library version number
 */
T_S8 *_SD_GetAudioCodecVersions(T_AUDIO_CB_FUNS *cb);

/**
 * @brief	Set the decode handle and pass it to the decoder for use in callback invocations
 * @author	Deng Zhou
 * @date	2007-10-15
 * @param	[in] audio_input:
 * input structure containing playback information
 * @param	[in] T_VOID *pHandle:
 * the handle to pass in
 * @return	T_VOID *
 * @retval	Returns a pointer to the audio library internal decode structure; NULL indicates failure
 */
T_VOID _SD_SetHandle(T_VOID *audio_decode, T_VOID *pHandle);

/**
 * @brief	Open the audio playback device.
 * @author	Deng Zhou
 * @date	2007-10-15
 * @param	[in] audio_input:
 * input structure containing playback information
 * @param	[in] audio_output:
 * output structure specifying the required PCM format
 * @return	T_VOID *
 * @retval	Returns a pointer to the audio library internal decode structure; NULL indicates failure
 */
T_VOID *_SD_Decode_Open(T_AUDIO_DECODE_INPUT *audio_input, T_AUDIO_DECODE_OUT *audio_output);

/**
 * @brief	For AAC raw data streams without a frame header or file header, set the bitstream attribute information.
 * @author	Tang Xuechai
 * @date	2015-03-31
 * @param	[in] audio_decode:
 *          the audio decode library internal structure, i.e. the pointer returned by _SD_Decode_Open()
 * @param	[in] info:
 *          AAC bitstream attribute information
 * @return	T_S32
 * @retval	T_TRUE: set successfully; 
 * @retval	T_FALSE: set failed
 */
T_S32 _SD_Decode_SetAACStreamInfo(T_VOID *audio_decode, T_AUDEC_AACSTREAMINFO *info);

/**
 * @brief	Audio decode.
 * @author	Deng Zhou
 * @date	2007-10-15
 * @param	[in] audio_decode:
 * the audio decode library internal structure
 * @param	[in] audio_output:
 * output structure specifying the required PCM format
 * @return	T_S32
 * @retval	Returns the size in bytes of the audio data decoded by the library
 */
T_S32 _SD_Decode(T_VOID *audio_decode, T_AUDIO_DECODE_OUT *audio_output);

/**
 * @brief	Close the audio decode device.
 * @author	Deng Zhou
 * @date	2007-10-15
 * @param	[in] audio_decode:
 * the audio decode library internal structure
 * @return	T_S32
 * @retval	AK_TRUE:  closed successfully
 * @retval	AK_FALSE: close encountered an error
 */
T_S32 _SD_Decode_Close(T_VOID *audio_decode);

/**
 * @brief	Audio decode seek.
 * @author	Deng Zhou
 * @date	2007-10-15
 * @param	[in] audio_decode:
 * the audio decode library internal structure
 * @return	T_S32
 * @retval	AK_TRUE:  seek succeeded
 * @retval	AK_FALSE: seek encountered an error
 */
T_S32 _SD_Decode_Seek(T_VOID *audio_decode, T_AUDIO_SEEK_INFO *seek_info);

// #ifdef BLUETOOTH_PLAY
/**
 * @brief	set digital volume
 * @author	Tang Xuechai
 * @date    2012-02-29
 * @param	[in] audio_decode: the audio decode library internal structure
 * @param   [in] volume: target volume value.
 *  volume multiplier; assigned as (T_S32)(x.xx*(1<<10)), x.xx=[0.00~7.99]
 *  It is recommended not to exceed 1.00*(1<<10), as exceeding this may cause data overflow and audio distortion
 * @return	T_S32
 * @retval	AK_TRUE :  set volume sucess
 * @retval	AK_FLASE :	set volume fail
 */
T_S32 _SD_Decode_SetDigVolume(T_VOID *audio_decode, T_U32 volume);

/**
 * @brief	set digital volume
 * @author	Tang Xuechai
 * @date    2012-02-29
 * @param	[in] audio_decode: the audio decode library internal structure
 * @param   [in] volume: target volume dB value.
 *   volume in dB; assigned as (T_S32)(x.xx*(1<<10)), x.xx=[-100.00~8.00]; 1 dB steps are valid in the range [-60.00~8.00]
 *   It is recommended not to exceed 0 dB, as exceeding this may cause data overflow and audio distortion
 *   If x.xxx<=-79dB, the output will be silent; if x.xxx>8.0, the output may contain noise.
 * @return	T_S32
 * @retval	AK_TRUE :  set volume sucess
 * @retval	AK_FLASE :	set volume fail
 */
T_S32 _SD_Decode_SetDigVolumeDB(T_VOID *audio_decode, T_S32 volume);

/**
 * @brief	decode one packet data
 * @author	Tang Xuechai
 * @date    2012-02-30
 * @param	[in] audio_decode: decode struct, get from _SD_Decode_Open
 * @param   [in] in: in data stream
 * @param   [in] isize: in data stream length
 * @param   [in/out] audio_output: output information and pcm
 * @return	T_S32
 * @retval	<=0 : decode error
 * @retval	>0 :  output pcm size (byte)
 */
//T_S32 _SD_Decode_OnePacket(T_VOID *audio_decode, T_U8 *in, T_U32 isize, T_AUDIO_DECODE_OUT *audio_output);
//#endif

/**
 * @brief	Set the minimum buffer delay length for decoding.
 * @author	Tang Xuechai
 * @date	      2012-4-20
 * @param	[in] audio_decode:
 * the audio decode library internal structure
 * @param	[in] len:
 * the desired buffer extension length
 * @return	
 */
T_U32 _SD_SetInbufMinLen(T_VOID *audio_decode, T_U32 len);

/**
 * @brief	Set the decode buffer working mode.
 * @author	Deng Zhou
 * @date	2009-8-7
 * @param	[in] audio_decode:
 * the audio decode library internal structure
 * @param	[in] bufmode:
 * buffer working mode
 * @return	
 */
T_S32 _SD_SetBufferMode(T_VOID *audio_decode, T_AUDIO_BUFFER_MODE buf_mode);

/**
 * @brief	Get the WMA bitrate type: LPC, Mid, or High rate
 * @author	Li Jun
 * @date	2010-1-14
 * @param	[in] audio_decode:
 * the audio decode library internal structure
 * @return  Returns the bitrate type: 0/1/2 corresponding to LPC/Mid/High rate	
 */
T_S32 _SD_GetWMABitrateType(T_VOID *audio_codec);

/**
 * @brief	Query the amount of free space in the internal audio playback buffer.
 * @author	Deng Zhou
 * @date	2007-10-15
 * @param	[in] audio_decode:
 * the audio decode library internal structure
 * @param	[in] buffer_control:
 * internal audio playback buffer status structure
 * @return	T_AUDIO_BUF_STATE
 * @retval	buffer status
 */
T_AUDIO_BUF_STATE _SD_Buffer_Check(T_VOID *audio_decode, T_AUDIO_BUFFER_CONTROL *buffer_control);

/**
 * @brief	Update the write pointer of the internal audio playback buffer.
 * @author	Deng Zhou
 * @date	2007-10-15
 * @param	[in] audio_decode:
 * the audio decode library internal structure
 * @param	[in] len:
 * number of bytes written to the internal audio playback buffer
 * @return	T_S32
 * @retval	AK_TRUE: update succeeded
 * @retval	AK_FALSE: update failed
 */
T_S32 _SD_Buffer_Update(T_VOID *audio_decode, T_U32 len);

/**
 * @brief	Clear the internal audio playback buffer.
 * @author	Deng Zhou
 * @date	2007-10-15
 * @param	[in] audio_decode:
 * the audio decode library internal structure
 * @return	T_S32
 * @retval	AK_TRUE: cleared successfully
 * @retval	AK_FALSE: clear failed
 */
T_S32 _SD_Buffer_Clear(T_VOID *audio_decode);

/**
 * @brief	Open the audio recording device.
 * @author	Deng Zhou
 * @date	2007-10-15
 * @param	[in] enc_input:
 * input information structure for recording
 * @param	[in] enc_output:
 * output information structure for recording
 * @return	T_VOID *
 * @retval	pointer to the internal audio recording structure; NULL indicates open failure
 */
T_VOID *_SD_Encode_Open(T_AUDIO_REC_INPUT *enc_input, T_AUDIO_ENC_OUT_INFO *enc_output);

/**
 * @brief	Encode the captured PCM data.
 * @author	Deng Zhou
 * @date	2007-10-15
 * @param	[in] audio_record: internal audio recording library structure
 * @param	[in] enc_buf_strc:  input/output buffer pointer and length structure
 * @return	T_S32 
 * @retval	
 *   >0: for SBC encoding, the low 16 bits of the return value are the data length and the high 16 bits are the frame count;
 *        for other formats, the low 16 bits are the data length and the high 16 bits are 0;
 *   =0: no valid encoded data output
 *   <0: encoding error with no valid data output
 */
T_S32 _SD_Encode(T_VOID *audio_encode, T_AUDIO_ENC_BUF_STRC *enc_buf_strc);

/**
 * @brief	Close the audio recording device.
 * @author	Deng Zhou
 * @date	2007-10-15
 * @param	[in] audio_encode:
 * internal audio recording library structure
 * @return	T_S32 
 * @retval	AK_TRUE: closed successfully
 * @retval	AK_FALSE: close failed
 */
T_S32 _SD_Encode_Close(T_VOID *audio_encode);

/**
 * @brief	Get the codec elapsed time.
 * @author	Deng Zhou
 * @date	2007-10-15
 * @param	[in] audio_codec:
 * codec library internal structure
 * @param	[in] codec_flag:
 * codec direction flag: 0=decode, 1=encode
 * @return	T_S32 
 * @retval	the retrieved time value
 */
T_S32 _SD_GetCodecTime(T_VOID *audio_codec, T_U8 codec_flag);


/**
 * @brief	Retrieve the current bitstream from the decode buffer for analysis.
 * @author	Li Jun
 * @date	2007-10-15
 * @param	[in] audio_codec:
 * codec library internal structure
 * @param	[in] T_U8 *pBuf:
 * buffer for storing the bitstream
 * @param	[in] T_U32 *len:
 * length of the bitstream in the storage buffer
 * @return	T_VOID 
 */
T_VOID _SD_LogBufferSave(T_U8 *pBuf, T_U32 *len,T_VOID *audio_codec);

/**
 * @brief	Compute the frequency spectrum of the input time-domain PCM audio signal and return it in-place. 
 *          This interface uses the FFT module from the WMA decoder, so it is only available when the WMA decode module is enabled
 * @author	Li Jun
 * @date	2011-4-14
 * @param	[in] T_S32 *data
 * time-domain audio PCM data    
 * @param	[in] T_U16 size
 * length of the time-domain audio PCM data
 * @param	[in] T_AUDIO_CB_FUNS *cbfun
 * callback function structure; malloc, free, and printf must be provided
 * @return	T_S32 
 * AK_FALSE: returned FALSE due to memory allocation failure
 * AK_TRUE: frequency-domain data computed successfully; the result is in data with effective length size/2
 */
T_S32 _SD_GetAudioSpectrum(T_S32 *data, T_U16 size, T_AUDIO_CB_FUNS *cbfun);

/**
 * @brief    Compute the frequency spectrum of the input time-domain PCM audio signal and return it in-place
 * @author	Tang Xuechai
 * @date	2013-11-15
 * @param	[in/out] T_S32 *data
 *               input/output data are all real numbers
 * @param	[in] T_U16 size
 *               input data length, which is the FFT length (i.e. the number of real-part points), not byte count
 * @param	[in] T_AUDIO_CB_FUNS *cbfun
 *               callback function pointers, such as malloc, free, and printf
 * @return	T_S32
 *               the number of returned points equals the number of input points and the result is symmetric
 **/
T_S32 _SD_GetAudioSpectrum_equNum(T_S32 *data, T_U16 size, T_AUDIO_CB_FUNS *cbfun);


/**
 * @brief    Compute the frequency spectrum of the input time-domain PCM audio signal and return it in-place. 
 * @author	Tang Xuechai
 * @date	2013-11-15
 * @param	[in/out] T_S32 *data
 *               input/output data are all complex numbers arranged in real/imaginary/real/imaginary... order
 * @param	[in] T_U16 size
 *               input data length, which is the FFT length (i.e. the number of real-part points), not byte count
 * @param	[in] T_AUDIO_CB_FUNS *cbfun
 *               callback function pointers, such as malloc, free, and printf
 * @return	T_S32
 *               the number of returned points equals the number of input points and the result is symmetric
 **/
T_S32 _SD_GetAudioSpectrumComplex(T_S32 *data, T_U16 size, T_AUDIO_CB_FUNS *cbfun);


// #if ((defined (NEWWAY_FILL_BUF)) || defined(ANDROID))
/**
 * @brief	Get the address pointer of the internal audio playback buffer.
 * @author	Cheng RongFei
 * @date	2011-7-13
 * @param	[in] audio_decode:
 * codec library internal structure
 * @param	[in] len:
 * the amount of data to be written to the buffer in one call
 * @return	T_VOID* 
 * @retval	Returns the address pointer of the buffer
 */
T_VOID* _SD_Buffer_GetAddr(T_VOID *audio_decode, T_U32 len);

/**
 * @brief	Update the write pointer of the internal audio playback buffer.
 * @author	Cheng RongFei
 * @date	2011-7-13
 * @param	[in] audio_decode:
 * the audio decode library internal structure
 * @return	T_S32
 * @retval	AK_TRUE: update succeeded
 * @retval	AK_FALSE: update failed
 */
T_S32 _SD_Buffer_UpdateAddr(T_VOID *audio_decode, T_U32 len);
// #endif

/** 
 * @brief   Finalize encoding
 * @author  Zhou Jiaqing
 * @date   2012-5-16
 * @param  [in] audio_codec: internal audio recording library structure
 *		   [in] enc_buf_strc: input/output buffer
 * @return T_S32
 * @retval length of the last encoded data block                                           
 */
T_S32 _SD_Encode_Last(T_VOID *audio_encode,T_AUDIO_ENC_BUF_STRC *enc_buf_strc);

/** 
 * @brief   Reset the encoder
 * @author  Tang Xuechai
 * @date   2018-2-11
 * @param  [in] audio_codec: internal audio recording library structure
 * @return T_S32
 * @retval	AK_TRUE: update succeeded
 * @retval	AK_FALSE: reset failed                                       
 */
T_S32 _SD_Encode_Reset(T_VOID *audio_encode);

/** 
 * @brief   During encoding, configure whether to return the AAC frame header data
 * @author  Tang Xuechai
 * @date   2013-5-20
 * @param  [in] audio_codec: internal audio recording library structure
 *		   [in] flag: one of the T_AUDIO_ENC_FRMHEAD_STATE enum values
 *                    _SD_ENC_SAVE_FRAME_HEAD: return frame header data
 *                    _SD_ENC_CUT_FRAME_HEAD: do not return frame header data; return only the encoded bitstream data
 * @return T_S32
 * @retval AK_TRUE: set successfully  
 *         AK_FALSE: set failed
 */
T_S32 _SD_Encode_SetFramHeadFlag(T_VOID *audio_encode, int flag);

/**
 * @brief  Call this when switching tracks during continuous playback
 *		   for use only with continuous OGG Vorbis playback       
 * @date  2012-6-6
 * @param [in] audio_decode :audio library internal decode structure
 * @return T_S32
 * @retval >0: operation succeeded  
 *         <0: operation failed
 *         =0: insufficient input data, failed
 */
T_S32 _SD_Decode_ParseFHead(T_VOID *audio_decode);

/**
 * @brief Open a playback file for continuous playback
 *	      for use only with continuous OGG Vorbis playback
 * @date 2012-6-6
 * @param [in] audio_input :audio information input structure
 *        [in] audio_output:PCM information output structure
 * @return T_VOID *
 * @retval Returns a T_VOID pointer on success, or AK_NULL on failure
 */
T_VOID *_SD_Decode_Open_Fast(T_AUDIO_DECODE_INPUT *audio_input, T_AUDIO_DECODE_OUT *audio_output);

/**
 * @brief Get information about the audio library input buffer
 * @date 2012-7-6
 * @param [in] audio_decode :audio decode structure
 *		  [in]  type: T_AUDIO_INBUF_STATE specifying the information to retrieve; valid values are:
 *	 			  _STREAM_BUF_LEN,         function returns the total input buffer length,
 *				  _STREAM_BUF_REMAIN_DATA, function returns the length of undecoded data remaining in the input buffer,
 *				  _STREAM_BUF_MIN_LEN,     function returns the minimum buffer length required for decoding				
 * @return T_S32
 * @retval 0:  buffer empty, no remaining data
 *         >0: length of undecoded data remaining in the buffer
 *         <0: invalid input pointer
 */
T_S32  _SD_Get_Input_Buf_Info(T_VOID *audio_decode,T_AUDIO_INBUF_STATE type);

/** 
 * @brief   Get the SBC decode error flag indicating whether the current frame contains an erroneous bitstream
 * @author  Tang Xuechai
 * @date   2017-8-18
 * @param  [in] audio_codec: decode handle
 * @return T_S32
 * @retval  0:   current frame has no bitstream errors
                  >0: current frame contains bitstream errors
                  <0: decode error
 *         
 */
T_S32 _SD_SBC_GetFrameErrFlag(T_VOID *audio_codec);


const T_VOID *_SD_AAC_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_SBC_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_MP3_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_FLAC_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_ADPCM_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_APE_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_OGG_VORBIS_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_RA8LBR_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_MIDI_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_AMR_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_AC3_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_PCM_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_SPEEX_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_G711_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_WMA_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_SPXWB_login(T_AUDIO_LOG_INPUT *plogInput);

const T_VOID *_SD_G711_Encode_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_AAC_Encode_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_AMR_Encode_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_ADPCM_Encode_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_MP3_Encode_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_SPEEX_Encode_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_SBC_Encode_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_SPXWB_Encode_login(T_AUDIO_LOG_INPUT *plogInput);
const T_VOID *_SD_OPUS_Encode_login(T_AUDIO_LOG_INPUT *plogInput);

#ifdef __cplusplus
}
#endif

#endif

/* end of sdcodec.h */

/*@}*/
