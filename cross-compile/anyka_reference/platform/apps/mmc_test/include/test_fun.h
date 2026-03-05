#ifndef _TEST_FUN_H_
#define _TEST_FUN_H_

#include "file.h"
#include "mtdlib.h"

typedef struct tag_DriverTestInfo

{

    T_U32 SectorSize;//sector size (in bytes)

    T_U32 ClusterSize;//cluster size (in bytes)

    T_U8 FatType;//filesystem type of the disk this file resides on, e.g. FAT32

}T_DRIVERTESTINFO, *T_PDRIVERTESTINFO;

typedef struct tag_FileTestInfo

{

   T_U32 BufferSize;//buffer size in use (in bytes)

T_U32 FileSize;//expected FileSize to be used (in bytes)



}T_FILETESTINFO, *T_PFILETESTINFO;


typedef struct _FAT_INFO_TMP
{
	T_U32 start;		//starting cluster number
	T_U32 cnt;		//number of consecutive clusters
}FAT_INFO_TMP, *PFAT_INFO_TMP;




/************************************************************************
 * NAME:        Driver_GetFormatType
 * FUNCTION  This function gets the filesystem format supported by the corresponding drive
 * PARAM:      T_U8 DriverID : ID corresponding to the drive
                        T_U8 *SupportType : filesystem types supported by the drive; 0 means not supported

 * RETURN:     whether the corresponding drive failed during operation; otherwise success.
**************************************************************************/

T_BOOL Driver_GetFormatType(T_U8 DriverID,T_U8 *SupportType);


/************************************************************************
 * NAME:        Driver_GetTestInfo
 * FUNCTION  This function gets some test information about the drive
 * PARAM:      T_U8 DriverID : ID corresponding to the drive
                        T_PDRIVERTESTINFO  pDriverTestInfo : the test info structure to be filled

 * RETURN:     whether the corresponding drive failed during operation; otherwise success.
**************************************************************************/

T_BOOL Driver_GetTestInfo(T_U8 DriverID,  T_PDRIVERTESTINFO  pDriverTestInfo);


/************************************************************************
 * NAME:        File_GetTestInfo
 * FUNCTION  This function gets some test information about the corresponding file
 * PARAM:      T_U32 file : file handle
                        T_PFILETESTINFO FileTestInfo : the test info structure to be filled

 * RETURN:     whether the file operation failed; otherwise success.
**************************************************************************/

T_BOOL File_GetTestInfo(T_U32 file,  T_PFILETESTINFO FileTestInfo);


/************************************************************************
 * NAME:        FS_GetDirectoryFdtInfo
 * FUNCTION  This function gets how many empty FDT entries the filesystem directory can hold and the count of actually empty FDT entries
 * PARAM:      T_U32 FolderHandle : folder handle
                        T_U32 *TolFdt : total number of empty FDT entries in the filesystem (including used and those already deleted with 0xE5 marker)
                        T_U32 *SeriousFdt : count of truly empty FDT entries in the filesystem directory
 * RETURN:     whether the filesystem operation failed; otherwise success.
**************************************************************************/

T_BOOL FS_GetDirectoryFdtInfo(T_U32 FolderHandle, T_U32 *TolFdt,
    T_U32 *SeriousFdt, T_U32 *SectorAddr, T_U32 * SectorOff);


/************************************************************************
 * NAME:        FS_GetLastUpdateCluster
 * FUNCTION  This function gets the cluster number of the last updated cluster in the file
 * PARAM:      T_U32 FileHandle : file handle
 * RETURN:     returns the cluster number of the last updated cluster.
**************************************************************************/

T_U32 FS_GetLastUpdateCluster(T_U32 FileHandle);

/************************************************************************
 * NAME:        FS_DriverGetNextCluster
 * FUNCTION  This function gets info about the next cluster in the FAT corresponding to the drive
 * PARAM:      T_U32 DriverID : ID corresponding to the drive
                        T_U32 CurCluster : current cluster info
 * RETURN:    directly returns the cluster info from the FAT table.
**************************************************************************/

T_U32 FS_DriverGetNextCluster(T_U32 DriverID, T_U32 CurCluster);

/************************************************************************
 * NAME:        FS_DriverGetClusterPostion
 * FUNCTION  This function gets the position info of the corresponding cluster on the storage medium
 * PARAM:      T_U32 DriverID : ID corresponding to the drive
                        T_U32 CurCluster : current cluster info
                        T_U32 *SectorAddr: sector address where the current cluster resides on the medium
                        T_U32 *SectorOff : sector offset where the current cluster resides on the medium
 * RETURN:    returns whether the operation succeeded
**************************************************************************/

T_BOOL FS_DriverGetClusterPostion(T_U32 DriverID, T_U32 CurCluster, T_U32 *SectorAddr, T_U32 *SectorOff);

/************************************************************************
 * NAME:        FS_GetFileFatBufInfo
 * FUNCTION  This function gets file FAT buffer information
 * PARAM:      T_PFILE FileHandle : file handle
 * RETURN:     returns the buffer of the cached data
 example:
    t_u8 *data; //buffer to be obtained; NewFatInfo.start is the starting cluster of the first segment
    NewFatInfo.cnt is the count of that segment
    while (*data)
    {
        data += Cluster_GetBlock(data , &NewFatInfo.cnt, &NewFatInfo.start, &relative, &normal);
        Fat_AddUseFatBuf(driver, &NewFatInfo);
    }
**************************************************************************/
T_U8 Cluster_GetBlock(const T_U8 *data, T_U32* count, T_U32* RealAddr, T_U32* relative, T_BOOL* normal);

T_U8* FS_GetFileFatBufInfo(T_U32 FileHandle, T_U32 *TolSize);

/************************************************************************
 * NAME:        FS_GetDriverFatBufInfo
 * FUNCTION  This function gets the internal FAT buffer cache information of the filesystem
 * PARAM:      T_U32 DriverID : ID corresponding to the drive
                        PFAT_INFO FatInfo : BUFFER for storing the cache info
                        T_U8 flag : whether to read the filesystem cache data only; if TRUE, returns free writable cache; otherwise returns used cache
 * RETURN:    returns the number of valid entries in the buffer
**************************************************************************/

T_U32 FS_GetDriverFatBufInfo(T_U8 DriverID, PFAT_INFO_TMP FatInfo, T_U8 flag);

/************************************************************************
 * NAME:        FS_GetDriverFatSectorSize
 * FUNCTION  This function gets FAT sector information in the drive
 * PARAM:      T_U32 DriverID : ID corresponding to the drive
                        T_U32 *FatNum : total number of clusters this drive can hold

 * RETURN:    returns the number of sectors the FAT can occupy
**************************************************************************/

T_U32 FS_GetDriverFatSectorSize(T_U8 DriverID, T_U32 *FatNum);

/************************************************************************
 * NAME:        FS_GetDriverFatInfo
 * FUNCTION  This function reads FAT information
 * PARAM:      T_U32 DriverID : ID corresponding to the drive
                        T_U32 addr: FAT sector address; this address can be the FAT sector address or the first cluster number of the FAT; if 0, the address unit is sectors
                        T_U32 size: number of FAT clusters to read
                        T_U8 * FatBuf: buffer for fat data
 * RETURN:     returns whether the operation succeeded or failed
**************************************************************************/

T_BOOL FS_GetDriverFatInfo(T_U8 DriverID, T_U32 addr, T_U32 size, T_U8 * FatBuf);
T_U32 Driver_GetObjDriver(T_U8 DeviceId);
T_U8  Driver_GetObjLength(T_VOID);
T_U32 FS_Driver_Format(T_PMEDIUM medium, T_U32 start,
                        T_U32 total, T_U32 BufLen, T_U32 code,T_U8 FormatType);


/************************************************************************
 * NAME:        File_FindFirst
 * FUNCTION  find some files in input path by bufCtrl condition
 * PARAM:      const T_U16 *path
                      T_PFINDBUFCTRL bufCtrl
 * RETURN:     it will return the find handle, return 0 if it fails, otherwise return the handle
**************************************************************************/

T_U32 TEST_File_FindFirst(const T_U16 *path, T_PFINDBUFCTRL bufCtrl);

/************************************************************************
 * NAME:        File_FindNext
 * FUNCTION  find next files with the finding hanle
 * PARAM:      T_U32 pFindCtrl
                      T_S32 Cnt     the need file number
 * RETURN:     it will return the found file number
**************************************************************************/

T_U32 TEST_File_FindNext(T_U32 pFindCtrl, T_S32 Cnt);

/************************************************************************
 * NAME:        File_FindClose
 * FUNCTION   it will release all resource of the finding handle.
 * PARAM:      T_U32 pFindCtrl
 * RETURN:     NONE
**************************************************************************/

T_VOID TEST_File_FindClose(T_U32 pFindCtrl);

/************************************************************************
 * NAME:        File_FindInfo
 * FUNCTION  get file info of the input pFindCtrl
 * PARAM:      T_U32 pFindCtrl
                      T_U32 position :the file's position of pFindCtrl's blink
                      T_U32 *FileCnt :     file count
                      T_U32 *FolderCnt :     folder count
 * RETURN:     it will return file info structure pointer
**************************************************************************/

T_PFILEINFO TEST_File_FindInfo(T_U32 pFindCtrl, T_U32 Position, T_U32 *FileCnt, T_U32 *FolderCnt);


/************************************************************************
 * NAME:        File_Close
 * FUNCTION  reflush write data to medium and delete refrence from system.
                      and destroy a file object to free space.
 * PARAM:      T_U32 file
 * RETURN:     NONE
**************************************************************************/

T_VOID TEST_File_Close(T_U32 file);


/************************************************************************
 * NAME:        File_OpenAsc
 * FUNCTION  it will open file with the asc file name and open mode
 * PARAM:      T_U32 parent   the parent directory handle
                       const T_U8* FileName     the file name
                       T_U32 mode                     the open mode
 * RETURN:     return the file hanle, we will use File_IsFile to decide if the file is exist.
**************************************************************************/

T_U32 TEST_File_OpenAsc(T_U32 parent, const T_U8* FileName, T_U32 mode);


/************************************************************************
 * NAME:        File_OpenUnicode
 * FUNCTION  it will open file with the unicode file name and open mode
 * PARAM:      T_U32 parent   the parent directory handle
                       const T_U8* FileName     the file name
                       T_U32 mode                     the open mode
 * RETURN:     return the file hanle, we will use File_Exist to decide if the file is exist.
**************************************************************************/

T_U32 TEST_File_OpenUnicode(T_U32 parent, const T_U16* FileName, T_U32 mode);

/************************************************************************
 * NAME:        File_IsFolder
 * FUNCTION  it will check the file handle if it is directory
 * PARAM:      T_PFILE file
 * RETURN:     if file is exist and file is a directory return AK_TRUE else return AK_FALSE
**************************************************************************/

T_BOOL TEST_File_IsFolder(T_U32 file);

/************************************************************************
 * NAME:        File_IsFile
 * FUNCTION  it will check the file handle if it is file
 * PARAM:      T_PFILE file
 * RETURN:     if file is exist and file is a data file return AK_TRUE else return AK_FALSE
**************************************************************************/

T_BOOL TEST_File_IsFile(T_U32 file);

/************************************************************************
 * NAME:        File_Exist
 * FUNCTION  it will check if the file handle is exist
 * PARAM:      T_PFILE file
 * RETURN:     return AK_TRUE if the handle is exist ,otherwise return AK_FALSE
**************************************************************************/

T_BOOL TEST_File_Exist(T_U32 file);

/************************************************************************
 * NAME:        File_GetLength
 * FUNCTION  it will get the file handle iformation
 * PARAM:      T_PFILE file
                       T_U32* excess        the high 32 bit of the file length, or the folder number
 * RETURN:     return file's length low 32 bit. and set high. If the file is a directory, then
                       high records its sub folder's num and the return value is its sub file's num
**************************************************************************/

T_U32 TEST_File_GetLength(T_U32 file, T_U32* excess);


/************************************************************************
 * NAME:        File_GetOccupiedSpace
 * FUNCTION  it will get the file handle iformation
 * PARAM:      T_PFILE file
                       T_U32* high        the high 32 bit of the file length
 * RETURN:     the file had used the low 32 bit of the file of the room
**************************************************************************/

T_U32 TEST_File_GetOccupiedSpace(T_U32 file, T_U32 *high);

/************************************************************************
 * NAME:        File_DelAsc
 * FUNCTION  delete a old file or folder.
 * PARAM:      const T_U8 *FileName the asc file name
 * RETURN:     return ak_true if it is ok, otherwise return ak_false
**************************************************************************/

T_BOOL TEST_File_DelAsc(const T_U8 *FileName);

/************************************************************************
 * NAME:        File_DelUnicode
 * FUNCTION  delete a old file or folder.
 * PARAM:      const T_U8 *FileName the unicode file name
 * RETURN:     return ak_true if it is ok, otherwise return ak_false
**************************************************************************/
T_BOOL TEST_File_DelUnicode(const T_U16 *FileName);

/************************************************************************
 * NAME:        File_RenameTo
 * FUNCTION  rename or move file
 * PARAM:      T_PFILE source
                      T_PFILE dest
 * RETURN:     return ak_true if it is ok, otherwise return ak_false
**************************************************************************/

T_BOOL TEST_File_RenameTo(T_U32 source, T_U32 dest);

/************************************************************************
 * NAME:        File_Seek
 * FUNCTION  move data file read /write pointer.
 * PARAM:      T_PFILE file
                      T_U32 LowPos      the file low 32 bit
                      T_U32 *HighPos    the file high 32 bit, it will return the high 32 bit of file point
 * RETURN:     return the  low 32 bit of the file point
**************************************************************************/

T_U32 TEST_File_Seek(T_U32 file, T_U32 LowPos, T_U32 *HighPos);

/************************************************************************
 * NAME:        File_Read
 * FUNCTION  read data from a data file.
 * PARAM:      T_PFILE file
                      T_pVOID buf
                      T_U32 byts
 * RETURN:     return the read size
**************************************************************************/

T_U32 TEST_File_Read(T_U32 file, T_pVOID buf, T_U32 byts);

/************************************************************************
 * NAME:        File_Write
 * FUNCTION  write data from a data file.
 * PARAM:      T_PFILE file
                      T_pVOID buf
                      T_U32 byts
 * RETURN:     return the write size
**************************************************************************/

T_U32 TEST_File_Write(T_U32 file, T_pVOID buf, T_U32 size);

/************************************************************************
 * NAME:        File_DelDir
 * FUNCTION  delete all file and folder of a folder.
 * PARAM:      T_PFILE folder
 * RETURN:     return the delete file and folder number
**************************************************************************/

T_U32 TEST_File_DelDir(T_U32 folder);

/************************************************************************
 * NAME:        File_DelFile
 * FUNCTION  delete file
 * PARAM:      T_PFILE file
 * RETURN:     return ak_true if it is ok, otherwise return ak_false
**************************************************************************/

T_BOOL TEST_File_DelFile(T_U32 file);

/************************************************************************
 * NAME:        File_MkdirsAsc
 * FUNCTION  create a folder by ascii path, include middle folder
 * PARAM:      const T_U8* path
 * RETURN:     return ak_true if it is ok, otherwise return ak_false
**************************************************************************/

//create a folder by ascii path, include middle folder.
T_BOOL TEST_File_MkdirsAsc(const T_U8* path);

/************************************************************************
 * NAME:        File_MkdirsUnicode
 * FUNCTION  create a folder by unicode path, include middle folder
 * PARAM:      const T_U16* path
 * RETURN:     return ak_true if it is ok, otherwise return ak_false
**************************************************************************/

T_BOOL TEST_File_MkdirsUnicode(const T_U16* path);

/************************************************************************
 * NAME:        File_Flush
 * FUNCTION  flush last data to file.
 * PARAM:      T_PFILE file
 * RETURN:     NONE
**************************************************************************/

T_VOID TEST_File_Flush(T_U32 file);

/************************************************************************
 * NAME:        File_SetDefault
 * FUNCTION  if the file is folder, set it is defualt path
 * PARAM:      T_PFILE file
 * RETURN:     return ak_true if it is ok, otherwise return ak_false
**************************************************************************/

T_BOOL TEST_File_SetDefault(T_U32 file);

/************************************************************************
 * NAME:        File_SetDefault
 * FUNCTION  get file's attribute by ascii path, such as folder(0x10)
 * PARAM:      const T_U8* path
 * RETURN:     return the file current attribute
**************************************************************************/

T_U32 TEST_File_GetAttrAsc(const T_U8* path);

/************************************************************************
 * NAME:        File_GetAttrUnicode
 * FUNCTION  get file's attribute by unicode path, such as folder(0x10)
 * PARAM:      const T_U16* path
 * RETURN:     return the file current attribute
**************************************************************************/

T_U32 TEST_File_GetAttrUnicode(const T_U16* path);

/************************************************************************
 * NAME:        File_GetAttrUnicode
 * FUNCTION  set file's attribute by ascii path.
 * PARAM:      const T_U16* path
                      E_SETFILEATTR attribute
 * RETURN:     return ak_true if it is ok, otherwise return ak_false
**************************************************************************/

T_BOOL TEST_File_SetAttrAsc(const T_U8* path, E_SETFILEATTR attribute);


/************************************************************************
 * NAME:        File_SetAttrUnicode
 * FUNCTION  set file's attribute by unicode path.
 * PARAM:      const T_U16* path
                      E_SETFILEATTR attribute
 * RETURN:     return ak_true if it is ok, otherwise return ak_false
**************************************************************************/

T_BOOL TEST_File_SetAttrUnicode(const T_U16* path, E_SETFILEATTR attribute);

/************************************************************************
 * NAME:        File_Truncate
 * @FUNCTION: Truncate an open file.
 * @PARAM:  T_PFILE file -- the file you want to truncate.
                     T_U32 length -- the new length of the file,
                        it must be less than the file's length
 * @RETURN  : truncate successfully or not
 * @Attention: The file to be truncated couldn't be File_Destroy().
**************************************************************************/

T_BOOL TEST_File_Truncate(T_U32 file, T_U32 low, T_U32 high);

/************************************************************************
 * NAME:        File_SetBufferSize
 * FUNCTION  SectorSize must be 2^n! If not, the file->BufLenMask may not work!
 * PARAM:      T_PFILE file
                      T_U32 SectorNum
 * RETURN:     return ak_true if it is ok, otherwise return ak_false
**************************************************************************/

T_BOOL TEST_File_SetBufferSize(T_U32 file, T_U32 SectorNum);

/************************************************************************
 * @FUNCTION: Get the Mod time of the input file.
 * @PARAM   : T_PFILE file -- the file whose time you want to get.
              T_PFILETIME fileTime -- the structure used to put file time.
 * @RETURN  : Get ModTime successfully or not.
**************************************************************************/
T_BOOL TEST_File_GetModTime(T_U32 file, T_PFILETIME fileTime);

/************************************************************************
 * @FUNCTION: Get the Create time of the input file.
 * @PARAM   : T_PFILE file -- the file whose time you want to get.
              T_PFILETIME fileCTime -- the structure used to put file time.
 * @RETURN  : Get CreateTime successfully or not.
**************************************************************************/
T_BOOL TEST_File_GetCreateTime(T_U32 file, T_PFILETIME fileCTime);

/************************************************************************
 * NAME:        File_GetFileinfo
 * FUNCTION  get file info of the input file
 * PARAM:      T_PFILE file
                       T_PFILEINFO fileInfo
 * RETURN:     return ak_true if it is ok, otherwise return ak_false
**************************************************************************/

T_BOOL TEST_File_GetFileinfo(T_U32 file, T_PFILEINFO fileInfo);

/************************************************************************
 * NAME:        File_GetFileinfo
 * FUNCTION  copy  source file to dest file by ascii format.
 * PARAM:      const T_U8* srcPath
                      const T_U8* dstPath
                      T_BOOL replace    if it is ak_TRUE, we will replace the same file name
 * RETURN:     return ak_true if it is ok, otherwise return ak_false
**************************************************************************/

T_BOOL  TEST_File_CopyAsc(const T_U8* srcPath, const T_U8* dstPath, T_BOOL replace, F_CopyCallback pCallBack, T_VOID *pCallBackData);


/************************************************************************
 * NAME:        File_GetFileinfo
 * FUNCTION  copy  source file to dest file by unicode format.
 * PARAM:      const T_U16* srcPath
                      const T_U16* dstPath
                      T_BOOL replace    if it is ak_TRUE, we will replace the same file name
 * RETURN:     return ak_true if it is ok, otherwise return ak_false
**************************************************************************/

T_BOOL  TEST_File_CopyUnicode(const T_U16* srcPath, const T_U16* dstPath, T_BOOL replace, F_CopyCallback pCallBack, T_VOID *pCallBackData);

/************************************************************************
 * NAME:        File_GetFilePtr
 * FUNCTION  get the file point
 * PARAM:      T_PFILE file
                      T_U32 *HighPos    the file high 32 bit, it will return the high 32 bit of file point
 * RETURN:     return the  low 32 bit of the file point
**************************************************************************/

T_U32 TEST_File_GetFilePtr(T_U32 file, T_U32 *high);

/************************************************************************
 * NAME:        File_SetFilePtr
 * FUNCTION  set the file point
 * PARAM:      T_PFILE file
                      T_U32 offset      the new offset
                      T_U16 origin      the mode
 * RETURN:     return the  low 32 bit of the file point
**************************************************************************/

T_U32 TEST_File_SetFilePtr(T_U32 file, T_S32 offset, T_U16 origin);

/************************************************************************
 * NAME:        File_GetPathObj
 * FUNCTION  get file's absolute path. it include device number, abstract path and last name.
 * PARAM:      T_PFILE file
 * RETURN:     return the  path name information
**************************************************************************/

T_U32 TEST_File_GetPathObj(T_U32 file);

/************************************************************************
 * NAME:        File_GetAbsPath
 * FUNCTION  get file name from the handle of File_GetPathObj
 * PARAM:      T_U32 obj
 * RETURN:     return the  path name
**************************************************************************/
T_U16* TEST_File_GetAbsPath(T_U32 obj);

/************************************************************************
 * NAME:        File_DestroyPathObj
 * FUNCTION  free the ram of File_GetPathObj
 * PARAM:      T_U32 obj
 * RETURN:     NONE
**************************************************************************/
T_VOID TEST_File_DestroyPathObj(T_U32 obj);

/************************************************************************
 * NAME:        File_SetFileSize
 * FUNCTION  it will malloc some room to the file, and make the file have serious room
 * PARAM:      T_PFILE file
                      T_U32 fileSize    the file size
 * RETURN:     return the  path name information
**************************************************************************/
T_BOOL TEST_File_SetFileSize(T_U32 file, T_U32 fileSize);

/************************************************************************
 * NAME:        TEST_File_Close
 * FUNCTION  reflush write data to medium and delete refrence from system.
                      and destroy a file object to free space without file_flush.
 * PARAM:      T_PFILE file
 * RETURN:     NONE
**************************************************************************/

T_VOID TEST_File_CloseNOFlush(T_U32 handle);

T_VOID TEST_Global_UninstallDriver(T_U32 obj);


T_VOID TEST_Driver_Destroy(T_U32 driver);
#endif

