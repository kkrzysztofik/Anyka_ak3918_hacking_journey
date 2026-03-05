/**
 * @file medialib_global.h
 * @brief Define the global public types for media lib, video lib and audio lib
 *
 * Copyright (C) 2020 Anyka (Guangzhou) Microelectronics Technology Co., Ltd.
 * @update date 2020-06-11
 * @version 4.3
 */

#ifndef _MEDIA_LIB_GLOBAL_H_
#define _MEDIA_LIB_GLOBAL_H_

#include "anyka_types.h"

typedef struct
{
    T_U16   ResourceID;
    T_U8    *Buff;
    T_U32   Resource_len;
}T_AUDIO_LOADRESOURCE_CB_PARA;

typedef T_VOID (*MEDIALIB_CALLBACK_FUN_PRINTF)(T_pCSTR format, ...);

#if 0
typedef T_S32   SEM_ID;
typedef SEM_ID (*MEDIALIB_CALLBACK_FUN_SEM_CREATE)(T_U32 nSemType, T_U32 nMaxLockCount);
typedef T_BOOL (*MEDIALIB_CALLBACK_FUN_SEM_TAKE)(SEM_ID semID, T_S32 nTimeOut);
typedef T_BOOL (*MEDIALIB_CALLBACK_FUN_SEM_GIVE)(SEM_ID semID);
typedef T_BOOL (*MEDIALIB_CALLBACK_FUN_SEM_FLUSH)(SEM_ID semID);
typedef T_BOOL (*MEDIALIB_CALLBACK_FUN_SEM_DELETE)(SEM_ID semID);
#endif

// file operation
typedef T_S32   (*MEDIALIB_CALLBACK_FUN_OPEN)(T_pVOID lpData);
typedef T_S32   (*MEDIALIB_CALLBACK_FUN_READ)(T_S32 hFile, T_pVOID buf, T_S32 size);
typedef T_S32   (*MEDIALIB_CALLBACK_FUN_WRITE)(T_S32 hFile, T_pVOID buf, T_S32 size);
typedef T_S32   (*MEDIALIB_CALLBACK_FUN_SEEK)(T_S32 hFile, T_S32 offset, T_S32 whence); 
typedef T_S32   (*MEDIALIB_CALLBACK_FUN_TELL)(T_S32 hFile);
typedef T_VOID  (*MEDIALIB_CALLBACK_FUN_CLOSE)(T_S32 hFile);
typedef T_S32   (*MEDIALIB_CALLBACK_FUN_FILE_HANDLE_EXIST)(T_S32 hFile);
typedef T_U32   (*MEDIALIB_CALLBACK_FUN_FILE_GET_LENGTH)(T_S32 hFile);
typedef T_BOOL  (*MEDIALIB_CALLBACK_FUN_FILESYS_ISBUSY)(void);

// memory operation
typedef T_pVOID (*MEDIALIB_CALLBACK_FUN_MALLOC)(T_U32 size);
typedef T_VOID  (*MEDIALIB_CALLBACK_FUN_FREE)(T_pVOID mem);
typedef T_pVOID (*MEDIALIB_CALLBACK_FUN_DMA_MALLOC)(T_U32 size);
typedef T_VOID  (*MEDIALIB_CALLBACK_FUN_DMA_FREE)(T_pVOID mem);
typedef T_pVOID (*MEDIALIB_CALLBACK_FUN_DMA_MEMCPY)(T_pVOID dest, T_pCVOID src, T_U32 size);
typedef T_pVOID (*MEDIALIB_CALLBACK_FUN_VADDR_TO_PADDR)(T_pVOID mem); 
typedef T_U32   (*MEDIALIB_CALLBACK_FUN_MAP_ADDR)(T_U32 phyAddr, T_U32 size); 
typedef T_VOID  (*MEDIALIB_CALLBACK_FUN_UNMAP_ADDR)(T_U32 addr, T_U32 size);

// register operation
typedef T_VOID  (*MEDIALIB_CALLBACK_FUN_REG_BITS_WRITE)(T_U32 phyAddr, T_U32 val, T_U32 mask);

// Dcache operation
typedef T_VOID  (*MEDIALIB_CALLBACK_FUN_FLUSH_DCACHE)(void); // clean and invalidate D-cache
typedef T_VOID  (*MEDIALIB_CALLBACK_FUN_FLUSH_DCACHE_RANGE) (T_U32 start, T_U32 size); // start should be 32byte aligned
typedef T_VOID  (*MEDIALIB_CALLBACK_FUN_INVALID_DCACHE) (T_VOID); // invalidate D-cache
typedef T_VOID  (*MEDIALIB_CALLBACK_FUN_INVALID_DCACHE_RANGE) (T_U32 start, T_U32 size);

// hardware mutex
typedef T_pVOID (*MEDIALIB_CALLBACK_FUN_HW_LOCK)(T_S32 hw_id);
typedef T_S32   (*MEDIALIB_CALLBACK_FUN_HW_UNLOCK)(T_pVOID hLock);

// others
typedef T_VOID  (*MEDIALIB_CALLBACK_FUN_LOADRESOURCE)(T_AUDIO_LOADRESOURCE_CB_PARA *pPara);  // Load resource
typedef T_VOID  (*MEDIALIB_CALLBACK_FUN_RELEASERESOURCE)(T_U8 *Buff);    // Release loaded resource

typedef T_BOOL  (*MEDIALIB_CALLBACK_FUN_RTC_DELAY) (T_U32 ulTicks);


typedef struct
{
    T_S32   real_time;

    union {
        struct{
            T_U32 nCurBitIndex;  // Not used
            T_U32 nFrameIndex;   // Frame index
        } m_ape;
        struct{
            T_U32 Indx;          // Number of packets in the current page starting from the seek position
            T_U32 offset;        // Remaining data size to be decoded in the current page after seek
            T_BOOL flag;         // Seek flag, set to 1 when seeking
            T_U32  last_granu;   // Total samples decoded after completing the previous page
            T_U32  now_granu;    // Total samples decoded after completing the current page
            T_BOOL is_eos;       // End of stream flag
            T_U32  re_data;      // Size of incomplete packets in the current page after seek
            T_U32 pack_no;       // Packet index within the current page where the seek position is located
            T_U32 list[255];     // Record the offset of each packet in an OGG page
        }m_speex;
        struct{
            T_U32 Indx;          // Number of packets in the current page starting from the seek position
            T_U32 offset;        // Remaining data size to be decoded in the current page after seek
            T_BOOL flag;         // Seek flag, set to 1 when seeking
            T_U32  last_granu;   // Total samples decoded after completing the previous page
            T_U32  now_granu;    // Total samples decoded after completing the current page
            T_BOOL is_eos;       // End of stream flag
            T_U32  re_data;      // Size of incomplete packets in the current page after seek
            T_U32 pack_no;       // Packet index within the current page where the seek position is located
            T_U32 list[255];     // Record the offset of each packet in an OGG page
        }m_speexwb;
        struct{
            T_U8    secUse;     // Number of sections already read
            T_U8    secLen;     // Number of sections contained in a page
            T_U8    tmpSec;     // Number of sections already decoded
            T_BOOL  is_eos;     // Whether it is the last page
            T_BOOL  is_bos;     // Whether it is the first page
            T_U8    endpack;    // Position of the last packet in the current page
            // The number of decoded samples is a 64-bit number; currently only taking the lower 32 bits
            T_U32   gos;        // Total samples decoded after completing the current page, lower 32 bits
            T_U32   high_gos;   // Total samples decoded after completing the current page, higher 32 bits (unused, reserved for future use)
            T_U8    list[255];  // Record the size of each section in a page; a page contains at most 255 sections
        }m_vorbis;
        struct{
            T_U32 Indx;         // Number of packets in the current page starting from the seek position
            T_U32 offset;       // Remaining data size to be decoded in the current page after seek
            T_BOOL flag;        // Seek flag, set to 1 when seeking
            T_U32  last_granu;  // Total samples decoded after completing the previous page
            T_U32  now_granu;   // Total samples decoded after completing the current page
            T_BOOL is_eos;      // End of stream flag
            T_U32  re_data;     // Size of incomplete packets in the current page after seek
            T_U32 pack_no;      // Packet index within the current page where the seek position is located
            T_U32 list[255];    // Record the offset of each packet in an OGG page
        }m_opus;
    }m_Private;
}T_AUDIO_SEEK_INFO;

typedef enum 
{
    AUDIOLIB_CHIP_UNKNOW,
    AUDIOLIB_CHIP_AK10XX,
    AUDIOLIB_CHIP_AK10XXC,
    AUDIOLIB_CHIP_AK10XXL,
    AUDIOLIB_CHIP_AK10XXT,
    AUDIOLIB_CHIP_AK11XX,
    AUDIOLIB_CHIP_AK37XX,
    AUDIOLIB_CHIP_AK37XXL,
    AUDIOLIB_CHIP_AK37XXC,
    AUDIOLIB_CHIP_AK39XX,
    AUDIOLIB_CHIP_AK98XX,
    AUDIOLIB_CHIP_AK39XXE,
    AUDIOLIB_CHIP_AK10XXT2,
    AUDIOLIB_CHIP_AK39XXEV2,
    AUDIOLIB_CHIP_AK39XXEV3,
    AUDIOLIB_CHIP_AK10XXT3,
    AUDIOLIB_CHIP_AK39XXEV5,
    AUDIOLIB_CHIP_AK37XXD,
    AUDIOLIB_CHIP_AK10XXF
}T_AUDIO_CHIP_ID;

typedef struct
{
	T_U32	m_SampleRate;		//sample rate, sample per second
	T_U16	m_Channels;			//channel number
	T_U16	m_BitsPerSample;	//bits per sample 

	T_U32	m_ulSize;
	T_U32	m_ulDecDataSize;
	T_U8	*m_pBuffer;
}T_AUDIO_DECODE_OUT;

//just for audio codec lib
typedef T_S32 (*MEDIALIB_CALLBACK_FUN_CMMBSYNCTIME)(T_VOID *pHandle, T_U32 timestamp);
typedef T_VOID (*MEDIALIB_CALLBACK_FUN_CMMBAUDIORECDATA)(T_VOID *pHandle, T_U8 *buf, T_S32 len);
//end of just for audio codec lib
#endif//_MEDIA_LIB_GLOBAL_H_
