#ifndef _CHKDSK_H_
#define _CHKDSK_H_
#define CLUSTER_MAP_NUMBER  100
typedef struct _CLUSTER_MAP_ARRAY_INFO *PSECTOR_MAP_ARRAY_INFO;

typedef struct _CLUSTER_MAP_INFO
{
	T_U32 sectorAddr;	// SECTOR corresponding to this buffer. Since the FAT table cannot be too large, it is defined as T_U16 to save memory. If this value is -1, it means this buffer is not used.
	T_U32 bitNum;		// Records how many bits in the bitmap are set to 1. If it equals ClusterPerSector, it indicates the bitmap is all 1s, and this buffer can be released.
	T_U8 *bitMap;		// Bitmap data. Set to 1 when pointed to or empty, otherwise 0. If all are 1, this buffer will be released (set sectorAddr = T_U16_MAX) for other sectors to use. The buffer size is BytesPerSector/8.
} CLUSTER_MAP_INFO, *PCLUSTER_MAP_INFO;

typedef struct _CLUSTER_MAP_ARRAY_INFO
{
	T_U32 index;		// Cluster buffer, allocated in batches of 100.
	PSECTOR_MAP_ARRAY_INFO next;		// Points to the next large buffer
	PCLUSTER_MAP_INFO pMapArray;		// Cluster buffer, allocated in batches of 100.
	PSECTOR_MAP_ARRAY_INFO pCurSecMap;		// Points to the small buffer currently in use.
}SECTOR_MAP_ARRAY_INFO;


enum{MARK_FAT_OK, FAT_LINK_ERROR, MARK_MALLOC_ERROR, FAT_READ_ERROR};

#define TestBitMap(BitMap, item)    (((BitMap)[(item)>>3]&(1<<((item)&7))))
#define SetBitMap(BitMap, item)     ((BitMap)[(item)>>3] |= (1<<((item)&7)))
#define ClrBitMap(BitMap, item)     ((BitMap)[(item)>>3] &= ~(1<<((item)&7)))

typedef void F_ChkDskCallback(T_VOID *pData, T_U8 percent);
T_U32 FAT_GetFatLinkInfo(T_U8 * pFatBuf, T_U16 offset, T_U8 FSType);
T_U32 FAT_GetFatLinkInfo_chkdsk(T_U8 * pFatBuf, T_U16 offset, T_U8 FSType);
T_BOOL Fat_ChkDsk(T_U32  DriverID, F_ChkDskCallback pCallBack, T_VOID *CallbackData);
T_VOID FAT_SetFatLinkInfo(T_U8 * pFatBuf, T_U16 offset, T_U8 FSType, T_U32 newValue);


#endif

