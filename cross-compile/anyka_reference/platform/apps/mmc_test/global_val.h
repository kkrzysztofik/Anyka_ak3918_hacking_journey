#define DEV_MMC           "/dev/mmcblk0"
#define BLK_1G            (2048*1024LL)
#define BLK_128M          (2048*64LL)
#define SEC_SIZE          512LL
#define SIZE_SECTOR       512
#define SIZE_SECTOR_ALIGN 0x1ff
#define NUM_FORMAT_SECTOR 64                                                                        //number of sectors to clear starting from sector 0 when deleting the partition table
#define CHKSZ             14 	//(3G)
#define RANGE             0x100000LL //(128M)
#define COUNT_SECTOR      2048
#define SEC2USEC          1000000
#define MSEC2USEC         1000
#define MSEC2NSEC         1000000
#define LEN_OFFSET        1024
#define LEN_HINT          512
#define LEN_CMD           256
#define LEN_MOUNT         2048
#define LEN_SHELL_RES     4096
#define CMD_MOUNT_STATUS  "mount"
#define CMD_UMOUNT        "umount -l %s"
#define CMD_MOUNT         "mount %s %s"
#define NUM_ASCII         256
#define GIGABYTE          1000000000
#define AK_FALSE          0
#define AK_TRUE           1
#define AK_FAILED        -1
#define AK_SUCCESS        0
#define SECTOR_PER_MB     2048

#define PERCENT_GOOD      0                                                                         //cards with an error rate below this are considered good
#define PERCENT_WARNING   5                                                                         //cards with an error rate below this are considered warning cards

#define DEV_MMCBLK0       "/dev/mmcblk0"
#define DEV_MMCBLK0P1     "/dev/mmcblk0p1"
#define LEN_PATH_FILE     255
#define FAT_FIX_TEMPLATE  "A:%s"
#define FAT_FIX_PATH      "/"
#define PROC_MOUNTS       "/proc/mounts"

#define BYTE2MB           1048576
#define KBYTE             1024
#define HINT_MNT_INFO_ERR "CAN'T FIND MOUNT POINT INFO."
#define HINT_TEST_TIMEOUT "READ/WRITE/ERASE TF CARD TIMEOUT."
#define HINT_NO_ERASE     "DISABLE ERASE TF CARD."
#define HINT_VERSION      "1.0.2"

#define TEST_PER_MB       20                                                                        //sampling points for fixed-capacity test
#define TEST_NUMBER       128                                                                       //default number of fixed test points
#define TEST_SECTOR       1                                                                         //number of test sectors
#define TEST_TIMEOUT      15                                                                        //test timeout
#define TEST_POINT_TIMEOUT 300                                                                      //timeout for a single sample point test
#define TEST_SLEEP        1                                                                         //sleep time per test point
#define TEST_OFFSET       81920                                                                     //starting offset value for the test
#define TEST_ALIGN        2048                                                                      //sector alignment value

#define SPEED_NUM         5                                                                         //number of sample points
#define SPEED_SECTOR      128                                                                       //number of sectors per sample point for speed test
#define SPEED_PER_MB      2                                                                         //offset value for speed test sample points
#define SPEED_OFFSET      40960                                                                     //starting sector offset for speed test
#define SPEED_REQUIRE_MB  1                                                                         //write speed above this value is considered normal

#define HEX_OUTPUT_VIEW   64

#define SEC_WAIT_PT_FRESH 20                                                                        //wait for kernel to reload partition table

#define CMD_SYNC          "sync"
#define CMD_FDISK         "echo -e 'n\np\n1\n\n\nw\n' | fdisk /dev/mmcblk0"
#define CMD_MKFS          "mkfs.vfat %s"
//#define CMD_FDISK_FRESH   "echo -e 'p\n\nv\n\n\nw\n' | fdisk /dev/mmcblk0"
#define CMD_FDISK_FRESH   "fdisk /dev/mmcblk0 << EOF\np\nw\nEOF"
typedef unsigned long long ULL;

enum CARD_STATUS {                                                                                  //defective card test status
	CARD_STATUS_GOOD = 0,
	CARD_STATUS_WARNING,
	CARD_STATUS_ERROR,
};

enum FIX_STATUS {                                                                                   //filesystem check status
	FIX_STATUS_ERROR   = -1,
	FIX_STATUS_NONEED  =  0,
	FIX_STATUS_SUCCESS =  1,
};

enum FORMAT_STATUS {                                                                                //format result status
	FORMAT_STATUS_FAIL  =  0,
	FORMAT_STATUS_SUCCESS =  1,
};

enum RESULT_STATUS {                                                                                //detection result type
	RESULT_STATUS_NOTEST = -1,
	RESULT_STATUS_FAIL   =  0,
	RESULT_STATUS_PASS   =  1,
};

enum SPEED_TYPE {
	SPEED_TYPE_MIN = 0,
	SPEED_TYPE_MAX ,
	SPEED_TYPE_AVG ,
	SPEED_TYPE_NUM ,
};

enum CARD_TEST_TYPE {
	CARD_TEST_TYPE_GLOBAL = 0,
	CARD_TEST_TYPE_READ,
	CARD_TEST_TYPE_WRITE,
	CARD_TEST_TYPE_ERASE,
	CARD_TEST_TYPE_REWRITE,
	CARD_TEST_TYPE_NUM,
};

enum TEST_STATUS {
	TEST_STATUS_GLOBAL = 0,
	TEST_STATUS_SPEED,
	TEST_STATUS_CARD,
	TEST_STATUS_FATFS,
	TEST_STATUS_NUM,
};

enum FIX_RESULT {
	FIX_RESULT_ERROR,
	FIX_RESULT_NONEED,
	FIX_RESULT_SUCCESS,
	FIX_RESULT_UNKNOWN,
	FIX_RESULT_NUM,
};

struct card_result {                                                                                //test result report structure
	int ai_status[ TEST_STATUS_NUM ];                                                               //test results
	ULL ai_status_us[ TEST_STATUS_NUM ];                                                            //test duration
	int ai_status_point[ TEST_STATUS_NUM ];                                                         //test points
	double f_total_size;                                                                            //TF card capacity
	ULL i_total_sector;                                                                             //TF card sector count
	double f_speed_size;                                                                            //data length used for speed test
	double f_speed_percent;                                                                         //error percentage for speed test
	double af_speed_write[ SPEED_TYPE_NUM ];                                                        //speed test write speeds
	double af_speed_read[ SPEED_TYPE_NUM ];                                                         //speed test read speeds
	int ai_card_test[ CARD_TEST_TYPE_NUM ];                                                         //number of defective card test runs
	int ai_card_error[ CARD_TEST_TYPE_NUM ];                                                        //number of errors in defective card tests
	double af_card_percent[ CARD_TEST_TYPE_NUM ];                                                   //error percentage for defective card tests
	int i_fatfs_res;                                                                                //result of filesystem repair
};

#define FIX_HINT_ERROR   "ERROR CARD. CAN'T FIX."
#define FIX_HINT_NONEED  "GOOD CARD. NO NEED FIX."
#define FIX_HINT_SUCCESS "WRONG CARD. FIX SUCCESS."
#define FIX_HINT_UNKNOWN "UNKNOWN CARD."

#define HINT_PASS        "PASS"
#define HINT_FAIL        "FAIL"

#define HINT_MOUNT_SUCCESS "MOUNT SUCCESS"
#define HINT_MOUNT_FAIL    "MOUNT FAIL"

#define HINT_CARD_GOOD    "GOOD CARD"
#define HINT_CARD_WARNING "WARNING CARD"
#define HINT_CARD_ERROR   "ERROR CARD"

#define HINT_TEST_GLOBAL  "GLOBAL"
#define HINT_TEST_READ    "READ"
#define HINT_TEST_WRITE   "WRITE"
#define HINT_TEST_ERASE   "ERASE"
#define HINT_TEST_REWRITE "REWRITE"

extern int gi_fd_dev;
extern int gi_speed_sector;
extern int gi_speed_per_mb;
extern int gi_speed_offset;
extern char gc_prog_run;
extern char gc_view_detail;
extern char gac_hint_status[ ][ LEN_HINT ];
extern char gac_hint_fatfs[ ][ LEN_HINT ];
extern double gf_speed_require;
extern char gc_full_test;
extern int gi_test_sector;
extern int gi_test_timeout;
extern char gc_per_mb;
extern int gi_test_per_mb;
extern int gi_test_offset;
extern int gi_test_num;
extern int gi_test_sleep;
extern char gc_read;
extern char gc_erase;
extern char gc_erase_check;
extern char gc_write;
extern char gc_sector_align;
extern char gc_write_check;
extern char gc_key_value_res;
extern char gc_fatfs_printf;
extern struct card_result g_card_result;
extern int gi_test_point_timeout;
extern char gac_mount_dev[ LEN_MOUNT ];