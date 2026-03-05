/**
 * @file      sdfilter.h
 * @brief    Anyka Sound Device Module interfaces header file.
 *
 * This file declare Anyka Sound Device Module interfaces.\n
 * Copyright (C) 2008 Anyka (GuangZhou) Software Technology Co., Ltd.
 * @author    Deng Zhou
 * @date    2008-04-10
 * @version V0.0.1
 * @ref
 */

#ifndef __SOUND_FILTER_H__
#define __SOUND_FILTER_H__

#include "medialib_global.h"

#ifdef __cplusplus
extern "C" {
#endif


/** @defgroup Audio Filter library
 * @ingroup ENG
 */
/*@{*/

/* @{@name Define audio version*/
/** Use this to define version string */   
/* Note: If the structure is modified, the middle version number must be updated */
#define AUDIO_FILTER_VERSION_STRING        (T_U8 *)"AudioFilter Version V1.13.09"
/** @} */

   
#define    _SD_VOLCTL_VOLDB_Q  10
#define    _SD_EQ_MAX_BANDS    10
#define    _SD_MDRC_MAX_BANDS  4
#define    _SD_REECHO_DECAY_Q  10

typedef enum
{
    _SD_FILTER_UNKNOWN ,
    _SD_FILTER_EQ ,
    _SD_FILTER_WSOLA ,
    _SD_FILTER_RESAMPLE,
    _SD_FILTER_3DSOUND,
    _SD_FILTER_DENOICE,
    _SD_FILTER_AGC,
    _SD_FILTER_VOICECHANGE,
    _SD_FILTER_PCMMIXER,
    _SD_FILTER_3DENHANCE,
    _SD_FILTER_MVBASS,
    _SD_FILTER_ASLC,
    _SD_FILTER_TONE_DETECTION,
    _SD_FILTER_VOLUME_CONTROL,
    _SD_FILTER_REECHO,
    _SD_FILTER_MDRC,
    _SD_FILTER_DEVOCAL,
    _SD_FILTER_TYPE_MAX
}T_AUDIO_FILTER_TYPE;

typedef enum
{
    _SD_EQ_MODE_NORMAL,
    _SD_EQ_MODE_CLASSIC,
    _SD_EQ_MODE_JAZZ,
    _SD_EQ_MODE_POP,
    _SD_EQ_MODE_ROCK,
    _SD_EQ_MODE_EXBASS,
    _SD_EQ_MODE_SOFT,
    _SD_EQ_USER_DEFINE,
} T_EQ_MODE;

//to define the filter type
typedef enum
{
    FILTER_TYPE_NO ,
    FILTER_TYPE_HPF ,
    FILTER_TYPE_LPF ,
    FILTER_TYPE_HSF ,
    FILTER_TYPE_LSF ,
    FILTER_TYPE_PF1    //PeaKing filter
}T_EQ_FILTER_TYPE;


typedef enum
{
    _SD_WSOLA_0_5 ,
    _SD_WSOLA_0_6 ,
    _SD_WSOLA_0_7 ,
    _SD_WSOLA_0_8 ,
    _SD_WSOLA_0_9 ,
    _SD_WSOLA_1_0 ,
    _SD_WSOLA_1_1 ,
    _SD_WSOLA_1_2 ,
    _SD_WSOLA_1_3 ,
    _SD_WSOLA_1_4 ,
    _SD_WSOLA_1_5 ,
    _SD_WSOLA_1_6 ,
    _SD_WSOLA_1_7 ,
    _SD_WSOLA_1_8 ,
    _SD_WSOLA_1_9 ,
    _SD_WSOLA_2_0 
}T_WSOLA_TEMPO;

typedef enum
{
    _SD_WSOLA_ARITHMATIC_0 , // 0:WSOLA, fast but tone bad
    _SD_WSOLA_ARITHMATIC_1   // 1:PJWSOLA, slow but tone well
}T_WSOLA_ARITHMATIC;


typedef enum
{
    RESAMPLE_ARITHMETIC_0 = 0,  // 0: Good sound quality, only sampling between fixed sample rates
    RESAMPLE_ARITHMETIC_1         // 1: Poor sound quality, sampling between any sample rates
}RESAMPLE_ARITHMETIC;

typedef enum
{
    _SD_OUTSR_UNKNOW = 0,
    _SD_OUTSR_48KHZ = 1,
    _SD_OUTSR_44KHZ,
    _SD_OUTSR_32KHZ,
    _SD_OUTSR_24KHZ,
    _SD_OUTSR_22KHZ,
    _SD_OUTSR_16KHZ,
    _SD_OUTSR_12KHZ,
    _SD_OUTSR_11KHZ,
    _SD_OUTSR_8KHZ
}T_RES_OUTSR;

typedef enum
{
    PITCH_NORMAL = 0,
    PITCH_CHILD_VOICE ,
    PITCH_MACHINE_VOICE,
    PITCH_ECHO_EFFECT,
    PITCH_ROBOT_VOICE,
    PITCH_RESERVE
}T_PITCH_MODES;

typedef enum
{
    VOLCTL_VOL_MUTIPLE = 0,
    VOLCTL_VOL_DB = 2,
}VOLCTL_VOL_MODE;

typedef struct
{
    int num;
    struct 
    {
        int x;
        int y;
    }stone[10];
    int lookAheadTime;  //ms
    int gainAttackTime;  //ms
    int gainReleaseTime;  //ms
}T_FILTER_MILESTONE;

typedef struct
{
    T_U8  bands;
    /* 
     whether need to bypass total limiter
     0: do total limit; 
     1: bypass total limit
    */
    T_U8  limiterBypass;
    /* 
     whether does bandi Chorus to output  
     0: bandi chorus
     1: bandi doff 
    */
    T_U8  bandiDoff[_SD_MDRC_MAX_BANDS];
    /* 
     whether does bandi's DRC bypass
     0: do bandi's DRC; 
     1: bypass bandi's DRC
    */
    T_U8  bandiDrcBypass[_SD_MDRC_MAX_BANDS];

    /* set boundary freqs */
    T_U32 boundaryFreqs[_SD_MDRC_MAX_BANDS-1];

    // define bands' drc para
    int bandsLookAheadTime;  //ms
    struct
    {
        struct 
        {
            int x;
            int y;
        }stone[2];
        int gainAttackTime;  //ms
        int gainReleaseTime;  //ms
    }drcband[_SD_MDRC_MAX_BANDS];

    // define output limiter para
    struct
    {
        struct 
        {
            int x;
            int y;
        }stone[2];
        int lookAheadTime;  //ms
        int gainAttackTime;  //ms
        int gainReleaseTime;  //ms
    }drctotal;
}T_FILTER_MDRC_PARA;


typedef struct
{
    MEDIALIB_CALLBACK_FUN_MALLOC                Malloc;
    MEDIALIB_CALLBACK_FUN_FREE                  Free;
    MEDIALIB_CALLBACK_FUN_PRINTF                printf;
    MEDIALIB_CALLBACK_FUN_FLUSH_DCACHE_RANGE    flushDCache;
    MEDIALIB_CALLBACK_FUN_RTC_DELAY             delay;
    MEDIALIB_CALLBACK_FUN_INVALID_DCACHE        invDcache;
}T_AUDIO_FILTER_CB_FUNS;

struct sd_param_eq {
    T_U32 eqmode; // T_EQ_MODE
    
    /* 
    Set total gain value (db), note: preGain assignment format is (T_S16)(x.xxx*(1<<10))
    */
    T_S16 preGain;      //-12 <= x.xxx <= 12
    
    // For User Presets
    T_U32 bands;      //1~10
    T_U32 bandfreqs[_SD_EQ_MAX_BANDS];
    /* 
    Set gain value for each frequency band, note: bandgains assignment format is (T_S16)(x.xxx*(1<<10))
    */
    T_S16 bandgains[_SD_EQ_MAX_BANDS];  // -32.0 < x.xxx < 32.0
    /* 
    Set Q value for each frequency band, note:
    1. bandQ assignment format is (T_U16)(x.xxx*(1<<10))
    2. If bandQ is set to 0, the internal default value of the library is used (T_U16)(1.22*(1<<10))
    3. x.xxx < SampleRate/(2*CenterFreq of the band), and x.xxx must be less than 64.000
    */
    T_U16 bandQ[_SD_EQ_MAX_BANDS];     // q < sr/(2*f)
    T_U16 bandTypes[_SD_EQ_MAX_BANDS]; // T_EQ_FILTER_TYPE

    /*
    Smoothing transition parameters between modes when calling "_SD_Filter_SetParam()" function to change EQ parameters
    */
    T_U8  smoothEna;   // 0-no smoothing; 1-with smoothing
    T_U16 smoothTime;  // smoothing time (ms). If set to 0, the library internal default value is used (256.0*1000/SampleRate)

    /*** for ffeq dc_remove ***/
    T_U8  dcRmEna;
    T_U32 dcfb;

    /*** for EQ aslc ***/
    T_U8  aslcEna;
    T_U16 aslcLevelMax;

    /*** hw specific params ***/
    T_U8  numFrameDescriptor; // number of frame descriptors
    T_U16 frameSize; // frame size, samples
};
struct sd_param_devocal {
    T_U16 frameSize; // frame size, samples
    T_U16 bassFreq;  // low frequency
    T_U16 trebleFreq;// high frequency
    T_U16 strength; // 1~5, bigger is more cancelling
};
struct sd_param_wsola {
    T_U32 tempo;            // T_WSOLA_TEMPO
    T_U32 arithmeticChoice; // T_WSOLA_ARITHMATIC
};
struct sd_param_3dsound {
    T_U8 is3DSurround;
};
struct sd_param_resample {
    // Target sample rate 1:48k 2:44k 3:32k 4:24K 5:22K 6:16K 7:12K 8:11K 9:8K
    T_U32 outSrindex; // T_RES_OUTSR

    // Set maximum input length (bytes), used as a basis for dynamic allocation during open.
    // Later when actually calling resample, the input length cannot exceed this value.
    T_U32 maxinputlen; 

    // Due to the limitation that outSrindex can only be one of those in the enum, use this parameter when the desired target sample rate is a value outside the enum.
    // This parameter is no longer the index of the sample rate, but directly the value of the target sample rate. For example, 8000, 16000 ...
    // If you want this parameter to take effect, you must set outSrindex=0
    T_U32 outSrFree; 
    
    T_U32 reSampleArithmetic;
    T_U32 outChannel;
};
struct sd_param_agc {
    T_U16 AGClevel;  // make sure AGClevel < 32767
    /* used in AGC_1 */
    T_U32  max_noise;
    T_U32  min_noise;
    /* used in AGC_2 */
    T_U8  noiseReduceDis;  // Whether to disable built-in noise reduction
    T_U8  agcDis;  // Whether to disable built-in AGC function
    /*
    agcPostEna: When agcDis==0, sets whether to actually perform AGC in AGC2 library:
    0: means actually perform AGC in the library, i.e., the data from filter_control has already been processed with AGC;
    1: means the library only calculates the AGC gain value, and does not actually perform AGC processing; the actual AGC is handled subsequently by the external caller.
    */
    T_U8  agcPostEna;  
    T_U16 maxGain;  // Maximum amplification factor
    T_U16 minGain;  // Minimum amplification factor
    T_U32 dc_freq;  // hz
    T_U32 nr_range; // 1~300, lower means more obvious noise reduction effect
};
struct sd_param_nr {
    T_U32 ASLC_ena;  // 0:disable aslc;  1:enable aslc
    T_U32 NR_Level;  // 0 ~ 4 bigger means stronger noise reduction
};
struct sd_param_pitch {
    T_U32 pitchMode;  // T_PITCH_MODES
    /*
     The pitchTempo parameter takes effect only when PITCH_CHILD_VOICE==pitchMode.
     The range of pitchTempo parameter is [0-10], 0~5 increases pitch, 5 normal pitch, 5~10 decreases pitch.
    */
    T_U8      pitchTempo; 
};
struct sd_param_reecho {
    /* Whether to enable reverb effect, 1 for enabled, 0 for closed */
    T_S32 reechoEna;  
    /*
    Attenuation factor, format is (T_S32)(0.xx * (1<<_SD_REECHO_DECAY_Q))
    For example, to set the parameter to 0.32, assign this variable (T_S32)(0.32 * (1<<_SD_REECHO_DECAY_Q))
    */
    T_S32 degree;      // 0-no reverb effect
    /*
    Set room size, recommended range 0-300 
    */
    T_U16 roomsize;    // 0-use default value (71).
    /*
    Set maximum reverb time (ms), i.e., how long it takes for reverb to disappear.
    Note: The longer this setting, the larger the required buffer, so it's not recommended to set it too large.
          Generally recommended within 1000; if memory is sufficient, it can be set larger.
          If memory is insufficient, reduce this value.
    */
    T_U16 reechoTime;  // 0-use default value (840)
    /*
    Whether to output the original main sound simultaneously. 
    0: Do not output original main sound, only the reflected sound;
    1: Output main sound and reflected sound together.
    */
    T_U8  needMainBody; //0 or 1
};
struct sd_param_3DEnhance {
    /* 
    Set total gain value (db),
    Note: preGain assignment format is (T_S16)(x.xxx*(1<<10)), 
    Limit -12 <= x.xxx <= 12
    */
    T_S16 preGain;  
    T_S16 cutOffFreq;
    /* 
    Set 3D depth,
    Note: depth assignment format is (T_S16)(x.xxx*(1<<10)), 
    Limit -1 < x.xxx < 1
    */
    T_S16 depth;   
    /*** for 3D Enhance's aslc, resvered***/
    T_U8   aslcEna;
    T_U16  aslcLevelMax;
};
struct sd_param_mvBass {
    /* 
    Set total gain value (db),
    Note: preGain assignment format is (T_S16)(x.xxx*(1<<10)), 
    Limit -12 <= x.xxx <= 12
    */
    T_S16 preGain;
    T_S16 cutOffFreq;  
    /* 
    Set enhancement magnitude,
    Note: bassGain assignment format is (T_S16)(x.xxx*(1<<10)), 
    Limit 0 < x.xxx < 12
    */
    T_S16 bassGain;
    /*** for MVBass's aslc ***/
    T_U8   aslcEna;
    T_U16  aslcLevelMax;
};
struct sd_param_aslc {
    T_BOOL aslcEna;
    T_U16  aslcLimitLevel;  // Limiting threshold
    T_U16  aslcStartLevel;  // Starting energy for limiting
    /* 
    jointChannels:
       0: Independent gain calculation and processing for both channels;
       1: Merge both channels by averaging, then calculate one gain, output same data for both channels;
       2: Interleave mixed data from both channels to find maximum and calculate gain, then use one gain value to process and output both channels.
    */            
    T_U16  jointChannels;

    /*
     maxLenin: Set maximum input PCM data length, in bytes.
     Allocates memory according to maximum length in advance during parameter setting.
     Solves the problem of multiple memory allocations leading to failure when data length changes.
    */
    T_U16  maxLenin;  
};
struct sd_param_volumeControl {
    /* 
    set volume mode::
    VOLCTL_VOL_MUTIPLE: Volume value is volume, i.e., externally passed volume multiplier
    VOLCTL_VOL_DB:      Volume value is voldb, i.e., externally passed db value
    */
    T_U16 setVolMode;

    /* 
    Set volume multiplier, (T_U16)(x.xx*(1<<10)), x.xx=[0.00~7.99] represents multiplier
    It's recommended not to exceed 1.00*(1<<10), as it may cause data overflow and sound distortion.
    */
    T_U16 volume; 

    /* 
    Set volume DB, assignment format is (T_S32)(x.xx*(1<<10)), x.xx=[-60.00~8.00]
    It's recommended not to exceed 0db, as it may cause data overflow and sound distortion.
    If x.xxx <= -79db, the output will be silent; if x.xxx > 8.0, it may result in noise.
    */
    T_S32 voldb;

    /* To prevent "pop" sounds during volume changes, smoothing is applied; this sets the transition time. */
    T_U16 volSmoothTime;  //ms
};
struct sd_param_toneDetection {
    T_U32 baseFreq;
};
struct sd_param_mdrc {
    /*
     maxLenin: Set maximum input PCM data length, in bytes.
     Allocates memory according to maximum length in advance during parameter setting.
     Solves the problem of multiple memory allocations leading to failure when data length changes.
    */
    T_U16  maxLenin;  
};

typedef struct
{
    T_U32    m_Type;             //T_AUDIO_FILTER_TYPE
    T_U32    m_SampleRate;       //sample rate, sample per second
    T_U16    m_Channels;         //channel number
    T_U16    m_BitsPerSample;    //bits per sample 

    union {
        struct sd_param_eq              m_eq;
        struct sd_param_devocal         m_devocal;
        struct sd_param_wsola           m_wsola;
        struct sd_param_3dsound         m_3dsound;
        struct sd_param_resample        m_resample;
        struct sd_param_agc             m_agc;
        struct sd_param_nr              m_NR;
        struct sd_param_pitch           m_pitch;
        struct sd_param_reecho          m_reecho;
        struct sd_param_3DEnhance       m_3DEnhance;
        struct sd_param_mvBass          m_mvBass;
        struct sd_param_aslc            m_aslc;
        struct sd_param_volumeControl   m_volumeControl;
        struct sd_param_toneDetection   m_toneDetection;
        struct sd_param_mdrc            m_mdrc;
    }m_Private;
}T_AUDIO_FILTER_IN_INFO;

typedef struct
{
    const char              *strVersion;
    T_AUDIO_CHIP_ID          chip;
    T_AUDIO_FILTER_CB_FUNS   cb_fun;
    T_AUDIO_FILTER_IN_INFO   m_info;

    const T_VOID            *ploginInfo;
}T_AUDIO_FILTER_INPUT;

typedef struct
{
    T_VOID *buf_in;
    T_U32   len_in;
    T_VOID *meta_in;
    
    T_VOID *buf_out;
    T_U32   len_out;
    T_VOID *meta_out;
    
    T_VOID *buf_in2;  //for mix pcm samples
    T_U32   len_in2;
}T_AUDIO_FILTER_BUF_STRC;

typedef struct
{
    T_AUDIO_FILTER_CB_FUNS cb;
    T_U32    m_Type;
}T_AUDIO_FILTER_LOG_INPUT;

//////////////////////////////////////////////////////////////////////////

/**
 * @brief    Get audio filter library version information.
 * @author    Deng Zhou
 * @date    2009-04-21
 * @param    [in] T_VOID
 * @return    T_S8 *
 * @retval    Returns audio filter library version number
 */
T_S8 *_SD_GetAudioFilterVersionInfo(void);

/**
 * @brief    Get audio filter library version information, including supported functions.
 * @author  Tang Xuechai
 * @date    2014-05-05
 * @param    [in] T_AUDIO_FILTER_CB_FUNS
 * @return    T_S8 *
 * @retval    Returns library version number
 */
T_S8 *_SD_GetAudioFilterVersions(T_AUDIO_FILTER_CB_FUNS *cb);

/**
 * @brief    Check if header version matches library version.
 * @author  Huang Liang
 * @date    2019-08-09
 * @param   [in] filter_input
 * @return    T_S32
 * @retval    T_TRUE or  T_FALSE
 */
T_S32 _SD_CheckAudioFilterVersion(T_AUDIO_FILTER_INPUT *filter_input);

/**
 * @brief    Open audio filter device.
 * @author    Deng Zhou
 * @date    2008-04-10
 * @param    [in] filter_input:
 * Input structure for audio filter
 * @return    T_VOID *
 * @retval    Returns pointer to internal structure, NULL means failure
 */
T_VOID *_SD_Filter_Open(T_AUDIO_FILTER_INPUT *filter_input);

/**
 * @brief    Audio filter processing.
 * @author    Deng Zhou
 * @date    2008-04-10
 * @param    [in] audio_filter:
 * Internal filter state structure
 * @param    [in] audio_filter_buf:
 * Input/output buffer structure
 * @return    T_S32
 * @retval    Returns processed audio data size in bytes
 */
T_S32 _SD_Filter_Control(T_VOID *audio_filter, T_AUDIO_FILTER_BUF_STRC *audio_filter_buf);

/**
 * @brief    Close audio filter device.
 * @author    Deng Zhou
 * @date    2008-04-10
 * @param    [in] audio_decode:
 * Internal filter state structure
 * @return    T_S32
 * @retval    AK_TRUE :  Success
 * @retval    AK_FALSE : Failure
 */
T_S32 _SD_Filter_Close(T_VOID *audio_filter);

/**
 * @brief    Set filter parameters: play speed, EQ mode.
 *          If any of m_SampleRate, m_BitsPerSample, m_Channels is 0, no effect is applied, returns AK_TRUE.
 * @author    Wang Bo
 * @date    2008-10-07
 * @param    [in] audio_filter:
 * Internal filter state structure
 * @param    [in] info:
 * Audio information structure
 * @return    T_S32
 * @retval    AK_TRUE :  Success
 * @retval    AK_FALSE : Failure
 */
T_S32 _SD_Filter_SetParam(T_VOID *audio_filter, T_AUDIO_FILTER_IN_INFO *info);

/**
 * @brief    Set ASLC module limit curve.
 * @author    Tang Xuechai
 * @date    2015-04-17
 * @param    [in] audio_filter: Internal filter state structure
 * @param    [in] fmileStones: ASLC limit curve parameters, refer to audio library interface manual
 * @return    T_S32
 * @retval    AK_TRUE :  Success
 * @retval    AK_FALSE : Failure
 */
T_S32 _SD_Filter_SetAslcMileStones(T_VOID *audio_filter, T_FILTER_MILESTONE *fmileStones);

/**
 * @brief     Set ASLC module silence detection amplitude threshold.
 * @author    Tang Xuechai
 * @date    2018-01-22
 * @param    [in] audio_filter: Internal filter state structure
 * @param    [in] silenceLevel: Silence threshold; PCM amplitude below this is considered silence.
 * @return    T_S32
 * @retval    AK_TRUE :  Success
 * @retval    AK_FALSE : Failure
 */
T_S32 _SD_Filter_SetAslcSilenceLevel(T_VOID *audio_filter, T_U32 silenceLevel);

/**
 * @brief     Set ASLC module silence detection continuous duration threshold.
 * @author    Tang Xuechai
 * @date    2018-01-22
 * @param    [in] audio_filter: Internal filter state structure
 * @param    [in] silenceTime: Silence duration threshold; entering silence state after this duration.
 * @return    T_S32
 * @retval    AK_TRUE :  Success
 * @retval    AK_FALSE : Failure
 */
T_S32 _SD_Filter_SetAslcSilenceTime(T_VOID *audio_filter, T_U32 silenceTime);

/**
 * @brief    Set MDRC module limit curve.
 * @author    Tang Xuechai
 * @date    2017-07-21
 * @param    [in] audio_filter: Internal filter state structure
 * @param    [in] fmdrc: MDRC parameters, refer to audio library interface manual
 * @return    T_S32
 * @retval    AK_TRUE :  Success
 * @retval    AK_FALSE : Failure
 */
T_S32 _SD_Filter_SetMdrcPara(T_VOID *audio_filter, T_FILTER_MDRC_PARA *fmdrc);

/**
 * @brief    Fast audio scaling (resampling).
 * @author    Tang_Xuechai
 * @date        2013-07-03
 * @param    [in] audio_filter:
 *               Internal filter state structure
 * @param    [out] dstData 
 *               Output PCM data
 * @param    [in] srcData:
 *               Input PCM data
 * @param    [in] srcLen 
 *               Byte length of input PCM data
 * @return    T_S32
 * @retval    >=0 :  Output PCM data size in bytes
 * @retval    <0  :  Scaling failed
 */
T_S32  _SD_Filter_Audio_Scale(T_VOID *audio_filter, T_S16 dstData[], T_S16 srcData[], T_U32 srcLen);


/**
* @brief    Convert EQ frequency domain parameters to time domain parameters.
* @author    Tang Xuechai
* @date        2015-03-24
* @param    [in] audio_filter:
*           Internal filter state structure from _SD_Filter_Open
* @param    [in] info:
*           Audio information structure
* @return    T_VOID *
* @retval    Returns pointer to time domain parameters, NULL means failure
*/
T_VOID *_SD_Filter_GetEqTimePara(T_VOID *audio_filter, T_AUDIO_FILTER_IN_INFO *info);

/**
* @brief    Pass current EQ time domain parameters to the EQ library.
* @author    Tang Xuechai
* @date        2015-03-24
* @param    [in] audio_filter:
*           Internal filter state structure from _SD_Filter_Open
* @param    [in] peqTime:
*           Time domain parameter pointer
* @return    T_S32
* @retval    AK_TRUE :  Success
* @retval    AK_FALSE:  Failure
*/
T_S32 _SD_Filter_SetEqTimePara(T_VOID *audio_filter, T_VOID *peqTime);

/**
* @brief    Release memory occupied by EQ time domain parameters.
* @author    Tang Xuechai
* @date        2015-03-24
* @param    [in] audio_filter:
*           Internal filter state structure from _SD_Filter_Open
* @param    [in] peqTime:
*           Time domain parameter pointer
* @return    T_S32
* @retval    AK_TRUE :  Success
* @retval    AK_FALSE:  Failure
*/
T_S32 _SD_Filter_DestoryEqTimePara(T_VOID *audio_filter, T_VOID *peqTime);


/**
 * @brief    Set volume value for the volume control module.
 * @author    Tang Xuechai
 * @date    2015-08-11
 * @param    [in] audio_filter: Internal filter state structure
 * @param    [in] volume: Target volume multiplier.
 *    Volume multiplier, (T_U16)(x.xx*(1<<10)), x.xx=[0.00~7.99]
 *    Recommended not to exceed 1.00*(1<<10) to avoid distortion.
 * @return    T_S32
 * @retval    AK_TRUE :  Success
 * @retval    AK_FALSE : Failure
 */
T_S32 _SD_Filter_SetVolume(T_VOID *audio_filter, T_U16 volume);

/**
 * @brief    Set volume value for the volume control module in DB.
 * @author    Tang Xuechai
 * @date    2015-08-11
 * @param    [in] audio_filter: Internal filter state structure
 * @param    [in] volume: Target volume in DB.
 *    Volume DB, assignment format is (T_S32)(x.xx*(1<<10)), x.xx=[-100.00~8.00], step 1db valid in [-60.00~8.00]
 *    Recommended not to exceed 0db to avoid distortion.
 *    If x.xxx <= -79db, the output will be silent; if x.xxx > 8.0, it may result in noise.
 * @return    T_S32
 * @retval    AK_TRUE :  Success
 * @retval    AK_FALSE : Failure
 */

T_S32 _SD_Filter_SetVolumeDB(T_VOID *audio_filter, T_S32 volume);

/**
* @brief    Set filter parameters: set smoothing time for volume changes.
* @param    [in] audio_filter: Internal filter state structure
* @param    [in] stime: Smoothing time in ms, from silent to 0db.
* @return      T_S32
* @retval       AK_TRUE :  Success
* @retval       AK_FALSE : Failure
*/
T_S32 _SD_Filter_Volctl_SetSmoothTime(T_VOID *audio_filter, T_U32 stime);

/**
* @brief    Get filter parameters: get currently active volume multiplier.
* @param    [in] audio_filter: Internal filter state structure
* @return      T_S32
* @retval       >=0 :  Volume multiplier
* @retval       <0:     Failure
*/
T_S32 _SD_Filter_Volctl_GetCurVolume(T_VOID *audio_filter);

const T_VOID *_SD_EQ_login(T_AUDIO_FILTER_LOG_INPUT *plogInput);
const T_VOID *_SD_3DEnhance_login(T_AUDIO_FILTER_LOG_INPUT *plogInput);
const T_VOID *_SD_3DSound_login(T_AUDIO_FILTER_LOG_INPUT *plogInput);
const T_VOID *_SD_ASLC_login(T_AUDIO_FILTER_LOG_INPUT *plogInput);
const T_VOID *_SD_mvBass_login(T_AUDIO_FILTER_LOG_INPUT *plogInput);
const T_VOID *_SD_NR_login(T_AUDIO_FILTER_LOG_INPUT *plogInput);
const T_VOID *_SD_AGC_login(T_AUDIO_FILTER_LOG_INPUT *plogInput);
const T_VOID *_SD_VolCtl_login(T_AUDIO_FILTER_LOG_INPUT *plogInput);
const T_VOID *_SD_WSOLA_login(T_AUDIO_FILTER_LOG_INPUT *plogInput);
const T_VOID *_SD_pitch_login(T_AUDIO_FILTER_LOG_INPUT *plogInput);
const T_VOID *_SD_Mixer_login(T_AUDIO_FILTER_LOG_INPUT *plogInput);
const T_VOID *_SD_Reecho_login(T_AUDIO_FILTER_LOG_INPUT *plogInput);
const T_VOID *_SD_Resample_login(T_AUDIO_FILTER_LOG_INPUT *plogInput);
const T_VOID *_SD_toneDetection_login(T_AUDIO_FILTER_LOG_INPUT *plogInput);
const T_VOID *_SD_MDRC_login(T_AUDIO_FILTER_LOG_INPUT *plogInput);
const T_VOID *_SD_Devocal_login(T_AUDIO_FILTER_LOG_INPUT *plogInput);

#ifdef __cplusplus
}
#endif

#endif
/* end of sdfilter.h */
/*@}*/
