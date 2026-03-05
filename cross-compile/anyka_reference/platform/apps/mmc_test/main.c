#include <stdio.h>
#include <stdlib.h>
#include <sys/types.h>
#include <unistd.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <errno.h>
#include <string.h>
#include <linux/fs.h>
#include <sys/ioctl.h>
#include <getopt.h>
#include <signal.h>
#include <regex.h>

#include "global_val.h"
#include "main.h"
#include "printcolor.h"
#include "timecount.h"
#include "regexpr.h"
#include "fat_repair.h"
#include "speed_test.h"
#include "card_test.h"

#ifdef MEMWATCH
#include "memwatch.h"
#endif

char gac_hint_status[ ][ LEN_HINT ] = {
	HINT_FAIL,
	HINT_PASS,
};

char gac_hint_fatfs[ ][ LEN_HINT ] = {
	FIX_HINT_ERROR,
	FIX_HINT_NONEED,
	FIX_HINT_SUCCESS,
	FIX_HINT_UNKNOWN,
};

char ac_option_hint[  ][ LEN_HINT ] = {
	"       [Defective card test]: Do not read from TF card. ***** This option will erase card contents! *****",
	"       [Defective card test]: Do not write to TF card",
	"       [Defective card test]: Perform full card scan test",
	"       [Defective card test]: Do not perform erase on TF card",
	"       [Defective card test]: Verify TF card after erasing",
	"       [FAT filesystem]: Only perform FAT filesystem repair",
	"       [FAT filesystem]: Perform FAT filesystem repair after card test",
	"       [Other]: Show detailed detection process",
	"       [Speed test]: Only perform TF card speed test",
	"       [Speed test]: Perform TF card speed test",
	"       [Defective card test]: Do not align sectors to 1MB multiples",
	"       [Defective card test]: Do not verify written data",
	"       [Other]: Output test results in key-value format",
	"       [FAT filesystem]: Enable display of FAT filesystem repair log",
	"       [FAT filesystem]: Only perform filesystem format",
	"[MB]   [Defective card test]: Use fixed-capacity sampling points, default: 20 (MB)",
	"[NUM]  [Defective card test]: Use fixed number of sampling points, default: 128",
	"[NUM]  [Defective card test]: Number of sectors per sampling point, default: 1 (sector)",
	"[SEC]  [Defective card test]: Global timeout for defective card test, default: 15 (s)",
	"[MSEC] [Defective card test]: Sleep time after each sampling point test, default: 1 (ms)",
	"[NUM]  [Defective card test]: Sector offset value; -1 for random offset, default: 81920",
	"[NUM]  [Speed test]: Number of sampling points, default: 5",
	"[NUM]  [Speed test]: Sectors per sampling point for speed test: 128",
	"[MB]   [Speed test]: Interval between speed test sampling points, default: 2 (MB)",
	"[NUM]  [Speed test]: Starting sector offset for speed test, default: 40960",
	"[NUM]  [Speed test]: Required speed performance threshold, default: 1 (MB)",
	"[dev]  [Other]: Storage card device name, default: /dev/mmcblk0",
	"[path] [FAT filesystem]: Specify directory path for repair, default: /",
	"[MSEC] [Defective card test]: Timeout per sampling point, default: 300 (ms)",
	"       [Other]: Help",
};

struct option option_long[ ] = {
	{ "no-read"          , no_argument,       NULL, 'A' },                        //"       [Defective card test]: Do not read from TF card. ***** This option will erase card contents! *****",
	{ "no-write"         , no_argument,       NULL, 'B' },                        //"       [Defective card test]: Do not write to TF card",
	{ "full-test"        , no_argument,       NULL, 'C' },                        //"       [Defective card test]: Perform full card scan test",
	{ "no-erase"         , no_argument,       NULL, 'D' },                        //"       [Defective card test]: Do not perform erase on TF card",
	{ "erase-check"      , no_argument,       NULL, 'E' },                        //"       [Defective card test]: Verify TF card after erasing",
	{ "only-fix-fat-fs"  , no_argument,       NULL, 'F' },                        //"       [FAT filesystem]: Only perform FAT filesystem repair",
	{ "with-fix-fat-fs"  , no_argument,       NULL, 'G' },                        //"       [FAT filesystem]: Perform FAT filesystem repair after card test",
	{ "view-detail"      , no_argument,       NULL, 'H' },                        //"       [Other]: Show detailed detection process",
	{ "only-speed-test"  , no_argument,       NULL, 'I' },                        //"       [Speed test]: Only perform TF card speed test",
	{ "with-speed-test"  , no_argument,       NULL, 'J' },                        //"       [Speed test]: Perform TF card speed test",
	{ "no-sector-align"  , no_argument,       NULL, 'K' },                        //"       [Defective card test]: Do not align sectors to 1MB multiples",
	{ "no-write-check"   , no_argument,       NULL, 'L' },                        //"       [Defective card test]: Do not verify written data",
	{ "key-value-res"    , no_argument,       NULL, 'M' },                        //"       [Other]: Output test results in key-value format",
	{ "view-fatfs-log"   , no_argument,       NULL, 'N' },                        //"       [FAT filesystem]: Enable display of FAT filesystem repair log",
	{ "only-format"      , no_argument,       NULL, 'O' },                        //"       [FAT filesystem]: Only perform filesystem format",
	{ "test-per-mb"      , required_argument, NULL, 'a' },                        //"[MB]   [Defective card test]: Use fixed-capacity sampling points, default: 20 (MB)",
	{ "test-number"      , required_argument, NULL, 'b' },                        //"[NUM]  [Defective card test]: Use fixed number of sampling points, default: 128",
	{ "test-sector"      , required_argument, NULL, 'c' },                        //"[NUM]  [Defective card test]: Number of sectors per sampling point, default: 1 (sector)",
	{ "test-timeout"     , required_argument, NULL, 'd' },                        //"[SEC]  [Defective card test]: Global timeout for defective card test, default: 15 (s)",
	{ "test-sleep"       , required_argument, NULL, 'e' },                        //"[MSEC] [Defective card test]: Sleep time after each sampling point test, default: 1 (ms)",
	{ "test-offset"      , required_argument, NULL, 'f' },                        //"[NUM]  [Defective card test]: Sector offset value; -1 for random offset, default: 81920",
	{ "speed-number"     , required_argument, NULL, 'g' },                        //"[NUM]  [Speed test]: Number of sampling points, default: 5",
	{ "speed-sector"     , required_argument, NULL, 'i' },                        //"[NUM]  [Speed test]: Sectors per sampling point for speed test: 128",
	{ "speed-per-mb"     , required_argument, NULL, 'j' },                        //"[MB]   [Speed test]: Interval between speed test sampling points, default: 2 (MB)",
	{ "speed-offset"     , required_argument, NULL, 'k' },                        //"[NUM]  [Speed test]: Starting sector offset for speed test, default: 40960",
	{ "speed-require"    , required_argument, NULL, 'l' },                        //"[NUM]  [Speed test]: Required speed performance threshold, default: 1 (MB)",
	{ "dev"              , required_argument, NULL, 'm' },                        //"[dev]  [Other]: Storage card device name, default: /dev/mmcblk0",
	{ "fat-fs-path"      , required_argument, NULL, 'n' },                        //"[path] [FAT filesystem]: Specify directory path for repair, default: /",
	{ "test-pnt-timeout" , required_argument, NULL, 'o' },                        //"[MSEC] [Defective card test]: Timeout per sampling point, default: 300 (ms)",
	{ "help"             , no_argument,       NULL, 'h' },                        //"       [Other]: Help",
};

char gc_read = AK_TRUE;
char gc_write = AK_TRUE;
char gc_write_check = AK_TRUE;
char gc_erase = AK_TRUE;
char gc_erase_check = AK_FALSE;

char gc_no_random = AK_FALSE;
char gc_no_order = AK_FALSE;
char gc_per_mb = AK_FALSE;
char gc_full_test = AK_FALSE;
char gc_sector_align = AK_TRUE;
int gi_test_per_mb = TEST_PER_MB;
int gi_test_num = TEST_NUMBER;
int gi_test_sector = TEST_SECTOR;
int gi_test_timeout = TEST_TIMEOUT ;
int gi_test_point_timeout = TEST_POINT_TIMEOUT;
int gi_test_sleep = TEST_SLEEP ;
int gi_test_offset = TEST_OFFSET;
char *gpc_dev = DEV_MMC;
char gc_prog_run = AK_TRUE;
char gc_view_detail = AK_FALSE;
int gi_fd_dev;
unsigned long long gi_total_size= 0;
char gc_mount = AK_FALSE;
char gac_mount_dev[ LEN_MOUNT ] = {0};
char ac_mount_dir[ LEN_MOUNT ];
char gc_with_fat_fix = AK_FALSE;
char gc_only_fat_fix = AK_FALSE;
char *gpc_fat_fix_path = FAT_FIX_PATH;
unsigned int i_fs_sector = 0 ;

char gc_with_speed = AK_FALSE;
char gc_only_speed = AK_FALSE;
int gi_speed_sector = SPEED_SECTOR;
int gi_speed_per_mb = SPEED_PER_MB;
int gi_speed_offset = SPEED_OFFSET;
double gf_speed_require = SPEED_REQUIRE_MB;

char gc_key_value_res = AK_FALSE;
char gc_fatfs_printf = AK_FALSE;
struct card_result g_card_result;
char gc_format = AK_FALSE;

int main(int argc, char** argv)
{
#ifdef MEMWATCH
	DEBUG_PRINT( COLOR_MODE_NORMAL , COLOR_BACK_BLACK , COLOR_FRONT_BLUE , "getenv( %s )= '%s'" , ENV_FLAG_KEY , getenv( ENV_FLAG_KEY ) )
	mw_init( );
#endif

	int i_option, i_color, i;
	struct timeval timeval_start, timeval_last;
	//struct card_result g_card_result_bak;

	DEBUG_SET_MAINPROG_NAME
	timeval_mark( &timeval_start );

	init_res( );

	while( ( i_option = getopt_long( argc, argv, "ABCDEFGHIJKLMNOa:b:c:d:e:f:g:i:j:k:l:m:n:h", option_long, NULL ) ) != -1 ) {
		switch( i_option ) {
			case 'A' :
				gc_read = AK_FALSE;
				break;
			case 'B' :
				gc_write = AK_FALSE;
				break;
			case 'C' :
				gc_full_test = AK_TRUE;
				gi_test_timeout = 0;                                                                //reset the global card scan detection timeout to 0
				break;
			case 'D' :
				gc_erase = AK_FALSE;
				break;
			case 'E' :
				gc_erase_check = AK_TRUE;
				break;
			case 'F' :
				gc_only_fat_fix = AK_TRUE;
				break;
			case 'G' :
				gc_with_fat_fix = AK_TRUE;
				break;
			case 'H' :
				gc_view_detail = AK_TRUE;
				break;
			case 'I' :
				gc_only_speed = AK_TRUE;
				break;
			case 'J' :
				gc_with_speed = AK_TRUE;
				break;
			case 'K' :
				gc_sector_align = AK_FALSE;
				break;
			case 'L' :
				gc_write_check = AK_FALSE;
				break;
			case 'M' :
				gc_key_value_res = AK_TRUE;
				break;
			case 'N' :
				gc_fatfs_printf = AK_TRUE;
				break;
			case 'O' :
				gc_format = AK_TRUE;
				break;
			case 'a' :
				gi_test_per_mb = atoi( optarg );
				gc_per_mb = AK_TRUE;
				break;
			case 'b' :
				gi_test_num = atoi( optarg );
				gc_per_mb = AK_FALSE;
				break;
			case 'c' :
				gi_test_sector = atoi( optarg );
				break;
			case 'd' :
				gi_test_timeout = atoi( optarg );
				break;
			case 'e' :
				gi_test_sleep = atoi( optarg );
				break;
			case 'f' :
				gi_test_offset = atoi( optarg );
				break;
			case 'g' :
				g_card_result.ai_status_point[ TEST_STATUS_SPEED ] = atoi( optarg );
				break;
			case 'i' :
				gi_speed_sector = atoi( optarg );
				break;
			case 'j' :
				gi_speed_per_mb = atoi( optarg );
				break;
			case 'k' :
				gi_speed_offset = atoi( optarg );
				break;
			case 'l' :
				gf_speed_require = atof( optarg );
				break;
			case 'm' :
				gpc_dev = optarg;
				break;
			case 'n' :
				gpc_fat_fix_path = optarg;
				break;
			case 'o' :
				gi_test_point_timeout = atoi( optarg );
				break;
			case 'h' :
				help( );
				return 0;
			default :
				help( );
				return 0;
		}
	}
	signal( SIGINT , prog_exit );
	signal( SIGTERM, prog_exit );
	if( gc_key_value_res == AK_FALSE ) {
		DEBUG_VAL( COLOR_MODE_NORMAL, COLOR_BACK_BLACK, COLOR_FRONT_GREEN, "%s %s@%s %s\n", gpc_prog_name , HINT_VERSION, __DATE__ , __TIME__ )
	}
	umount_disk( );
	if ( open_disk( gpc_dev ) == AK_FAILED ) {
		goto progame_end;
	}

	if( gc_format == AK_TRUE ) {
		if ( gi_fd_dev > 0 ) {
			ak_format_tf( );
			close(gi_fd_dev);
			gi_fd_dev = 0 ;
		}
		remount_disk( );
		goto progame_end;
	}

	if( gi_fd_dev <= 0 ) {
		g_card_result.ai_status[ TEST_STATUS_GLOBAL ] = RESULT_STATUS_FAIL;
	}


	if ( ( gc_only_fat_fix != AK_TRUE ) &&
		 ( gc_only_speed != AK_TRUE ) &&
		 ( gi_fd_dev > 0 ) ) {                                                                      //****** defective card detection ******
		g_card_result.ai_status[ TEST_STATUS_CARD ] = start_check_disk( );
		/*
		if ( ( g_card_result.ai_status[ TEST_STATUS_CARD ] == RESULT_STATUS_FAIL ) &&
		     ( gc_erase == AK_TRUE ) ){                                                             //if a test with erase fails, run another test without erase to confirm whether the failure was caused by the erase
			gc_erase = AK_FALSE;
			g_card_result_bak = g_card_result;
			g_card_result.ai_status[ TEST_STATUS_CARD ] = start_check_disk( );
			if ( g_card_result.ai_status[ TEST_STATUS_CARD ] == RESULT_STATUS_FAIL ){
				g_card_result = g_card_result_bak;
			}
		}
		*/
	}

	if ( ( g_card_result.ai_status[ TEST_STATUS_GLOBAL ] != RESULT_STATUS_FAIL ) &&
		 ( ( gc_with_speed == AK_TRUE ) ||
		   ( gc_only_speed == AK_TRUE ) ) &&
		 ( gi_fd_dev > 0 ) ) {                                                                      //****** speed test ******
		g_card_result.ai_status[ TEST_STATUS_SPEED ] = start_speed_test( );
	}

	if ( gi_fd_dev > 0 ) {
		close(gi_fd_dev);
		gi_fd_dev = 0 ;
	}

	if ( ( gc_with_fat_fix == AK_TRUE ) ||
	     ( gc_only_fat_fix == AK_TRUE ) ) {                                                         //****** filesystem check ******
		if ( strlen( gac_mount_dev ) > 0 ) {
			g_card_result.ai_status[ TEST_STATUS_FATFS ] = ak_repair_tf( gac_mount_dev, gpc_fat_fix_path);
		}
		else if ( gc_view_detail == AK_TRUE ) {
			DEBUG_VAL( COLOR_MODE_NORMAL, COLOR_BACK_BLACK, COLOR_FRONT_RED, "%s\n", HINT_MNT_INFO_ERR);
		}
	}

	remount_disk( );

	char c_test = AK_FALSE;
	for( i = TEST_STATUS_SPEED; i < TEST_STATUS_NUM ; i ++ ) {                                      //check whether the global test passed
		if ( g_card_result.ai_status[ i ] != RESULT_STATUS_NOTEST ) {
			c_test = AK_TRUE;
			break;
		}
	}
	if ( c_test == AK_TRUE ) {
		g_card_result.ai_status[ TEST_STATUS_GLOBAL ] = RESULT_STATUS_PASS;
		for( i = TEST_STATUS_SPEED; i < TEST_STATUS_NUM ; i ++ ) {
			if ( g_card_result.ai_status[ i ] == RESULT_STATUS_FAIL ) {
				g_card_result.ai_status[ TEST_STATUS_GLOBAL ] = RESULT_STATUS_FAIL;
				break;
			}
		}
	}

	if ( g_card_result.ai_status[ TEST_STATUS_GLOBAL ] == RESULT_STATUS_PASS )  {
		i_color = COLOR_FRONT_GREEN;
	}
	else {
		i_color = COLOR_FRONT_RED;
	}

	if ( ( g_card_result.ai_status_us[ TEST_STATUS_GLOBAL ] = timeval_count( &timeval_start, timeval_mark( &timeval_last ) ) ) > 0 ) {
	}

	if( gc_key_value_res == AK_FALSE ) {
		DEBUG_VAL( COLOR_MODE_NORMAL, COLOR_BACK_BLACK, i_color,
		           "###########################################\n"
		           "TEST %s\n"
		           "TOTAL SEC= %.2f\n"
		           "%.2fMB/s %.2fMB/s %.2f %.2f %.2f %.2f\n"
		           "###########################################\n",
		           gac_hint_status[ g_card_result.ai_status[ TEST_STATUS_GLOBAL ] ],
		           ( double )g_card_result.ai_status_us[ TEST_STATUS_GLOBAL ] / SEC2USEC,
		           g_card_result.af_speed_read[ SPEED_TYPE_MAX ],
		           g_card_result.af_speed_write[ SPEED_TYPE_MAX ],
		           ( double )g_card_result.ai_status_us[ TEST_STATUS_SPEED ] / SEC2USEC,
		           ( double )g_card_result.ai_status_us[ TEST_STATUS_CARD] / SEC2USEC,
		           ( double )g_card_result.ai_status_us[ TEST_STATUS_FATFS ] / SEC2USEC,
		           ( double )g_card_result.ai_status_us[ TEST_STATUS_GLOBAL ] / SEC2USEC )
	}
	else {
		print_key_value_result( );
	}

progame_end:
#ifdef MEMWATCH
	mw_status( 0 );
	mw_exit( );
#endif
	return 0;
}

int get_shell_res( char *pc_cmd , char *pc_result , int i_len_res )                          //get shell command result
{
	FILE *pFILE_shell = NULL ;
	int i_res = 0 ;

	memset( pc_result , 0 , i_len_res ) ;
	if ( ( pFILE_shell = popen( pc_cmd , "r" ) ) != NULL ) {
		i_res = fread( pc_result , sizeof( char ) , i_len_res , pFILE_shell ) ;
		if ( i_res < i_len_res ) {
			pc_result[ i_res ] = 0 ;
		}
		else {
			pc_result[ i_res - 1 ] = 0 ;
		}
		pclose( pFILE_shell ) ;
	}

	return i_res ;
}

int open_disk(char *dev)
{
	gi_fd_dev = open(dev, O_RDWR|O_EXCL|O_SYNC|O_DIRECT|O_LARGEFILE, 0666); /*O_DIRECT | O_LARGEFILE*/
	if (gi_fd_dev < 0) {
		if ( gc_view_detail == AK_TRUE ) {
			DEBUG_VAL( COLOR_MODE_NORMAL, COLOR_BACK_BLACK, COLOR_FRONT_RED,
			           "dev= '%s' strerror(%d)= '%s'\n", dev, errno, strerror(errno));
		}
		return AK_FAILED;
	}
	gi_total_size = get_disk_size(gi_fd_dev);
	g_card_result.i_total_sector = gi_total_size / SEC_SIZE;
	g_card_result.f_total_size = ( double )gi_total_size / GIGABYTE;
	if( gc_key_value_res == AK_FALSE ) {
		DEBUG_VAL( COLOR_MODE_NORMAL, COLOR_BACK_BLACK, COLOR_FRONT_BLUE,
		           "TOTAL_SIZE: %.1fG TOTAL_SECTOR: %llu\n",
		           g_card_result.f_total_size , g_card_result.i_total_sector);
	}

	return AK_SUCCESS;
}

void release_disk(int fd)
{
	close(fd);
}

off64_t get_disk_size(int fd)
{
	off64_t length=0;
	if (ioctl(fd, BLKGETSIZE64, &length) < 0) {
		if ( gc_view_detail == AK_TRUE ) {
			DEBUG_VAL( COLOR_MODE_NORMAL, COLOR_BACK_BLACK, COLOR_FRONT_RED,"strerror(%d)= '%s'\n", errno, strerror(errno));
		}
		return -1;
	}
	return length;
}

int help( )                                                                                         //print help based on option_long
{
	int i;

	DEBUG_VAL( COLOR_MODE_NORMAL, COLOR_BACK_BLACK, COLOR_FRONT_GREEN, "%s %s@%s %s\n", gpc_prog_name , HINT_VERSION, __DATE__ , __TIME__ )
	for( i = 0; i < sizeof( option_long ) / sizeof( struct option ); i ++ ) {
		DEBUG_VAL( COLOR_MODE_NORMAL, COLOR_BACK_BLACK, COLOR_FRONT_GREEN, "\t--%-16s -%c %s\n", option_long[ i ].name, option_long[ i ].val, ac_option_hint[ i ] )
	}
	return 0;
}

void prog_exit( int i_sig )
{
    gc_prog_run = AK_FALSE;
    return;
}

int umount_disk( )
{
	int i_mount_dev, i_mount_dir;
	char ac_mount_res[ LEN_MOUNT ], ac_cmd_umount[ LEN_MOUNT ];

	get_shell_res( CMD_MOUNT_STATUS, ac_mount_res, LEN_MOUNT );
	i_mount_dev = ak_regexpr_get( ac_mount_res , gac_mount_dev , LEN_MOUNT , 2 ,
	                              ACTION_PTRN_MATCH_ONE , "^/dev/mmcblk.*\\s+" , REG_EXTENDED | REG_NEWLINE ,
	                              ACTION_PTRN_DELETE_ALL , "\\s+.*" ,            REG_EXTENDED | REG_NEWLINE ) ;
	i_mount_dir = ak_regexpr_get( ac_mount_res , ac_mount_dir , LEN_MOUNT , 2 ,
	                              ACTION_PTRN_MATCH_ONE ,  "^/dev/mmcblk.*?on\\s+\\S+" , REG_EXTENDED | REG_NEWLINE ,
	                              ACTION_PTRN_DELETE_ALL , "(^/dev/mmcblk.*on\\s+)|(\\s+)" , REG_EXTENDED | REG_NEWLINE ) ;
	if ( ( i_mount_dev > 0 ) && ( i_mount_dir > 0 ) ) {                                             //mount point found
		if ( ( gc_with_fat_fix == AK_TRUE ) || ( gc_only_fat_fix == AK_TRUE ) ) {
			i_fs_sector = get_fs_sector( ac_mount_dir ) ;
		}
		gc_mount = AK_TRUE;
		snprintf( ac_cmd_umount , LEN_MOUNT, CMD_UMOUNT , ac_mount_dir ) ;
		system( ac_cmd_umount ) ;
		if( gc_key_value_res == AK_FALSE ) {
			DEBUG_VAL( COLOR_MODE_NORMAL, COLOR_BACK_BLACK, COLOR_FRONT_BLUE,"%s\n", ac_cmd_umount);
		}
	}
	else {
		if (is_dev_file(DEV_MMCBLK0P1) == AK_TRUE) {                                             //select TF card device name /dev/mmcblk0p1
			snprintf( gac_mount_dev , LEN_MOUNT , "%s" , DEV_MMCBLK0P1 );
		}
		else if (is_dev_file(DEV_MMCBLK0) == AK_TRUE) {                                          //select TF card device name /dev/mmcblk0
			snprintf( gac_mount_dev , LEN_MOUNT , "%s" , DEV_MMCBLK0 );
		}
	}

	return 0;
}

int if_mount_disk( )
{
	int i_mount_dev;
	char ac_mount_res[ LEN_MOUNT ] , ac_mount_dev[ LEN_MOUNT ] ;

	get_shell_res( CMD_MOUNT_STATUS, ac_mount_res, LEN_MOUNT );
	i_mount_dev = ak_regexpr_get( ac_mount_res , ac_mount_dev , LEN_MOUNT , 2 ,
	                              ACTION_PTRN_MATCH_ONE , "^/dev/mmcblk.*\\s+" , REG_EXTENDED | REG_NEWLINE ,
	                              ACTION_PTRN_DELETE_ALL , "\\s+.*" ,            REG_EXTENDED | REG_NEWLINE ) ;
	//DEBUG_PRINT( COLOR_MODE_NORMAL, COLOR_BACK_BLACK, COLOR_FRONT_BLUE, "ac_mount_dev= '%s'", ac_mount_dev)
	if ( i_mount_dev > 0 ) {
		return AK_TRUE;
	}
	else {
		return AK_FALSE;
	}
}

int remount_disk( )
{
	char ac_cmd_mount[ LEN_MOUNT ], c_res = AK_FAILED ;
	int i;

	if( gc_mount == AK_TRUE ) {
		snprintf( ac_cmd_mount, LEN_MOUNT, CMD_MOUNT, gac_mount_dev, ac_mount_dir );
		for( i = 0 ; i < SEC_WAIT_PT_FRESH ; i ++ ) {                                               //attempt to read the p1 partition
			//DEBUG_PRINT( COLOR_MODE_NORMAL, COLOR_BACK_BLACK, COLOR_FRONT_BLUE, "%s", ac_cmd_mount )
			sync();
			ioctl( gi_fd_dev, BLKRRPART, NULL );
			if ( is_dev_file( gac_mount_dev ) == AK_TRUE ) {
				if( gc_key_value_res == AK_FALSE ) {
					DEBUG_VAL( COLOR_MODE_NORMAL, COLOR_BACK_BLACK, COLOR_FRONT_BLUE, "%s\n", ac_cmd_mount )
				}
				system( ac_cmd_mount );
				if( if_mount_disk( ) == AK_TRUE ){
					c_res = AK_SUCCESS;
					break;
				}
			}
			if( gc_key_value_res == AK_FALSE ) {
				DEBUG_VAL( COLOR_MODE_NORMAL, COLOR_BACK_BLACK, COLOR_FRONT_BLUE, "%s\n", CMD_FDISK_FRESH )
			}
			system( CMD_FDISK_FRESH );
			sleep( 1 );
		}
		if( gc_key_value_res == AK_FALSE ) {
			if ( c_res == AK_SUCCESS ) {
				DEBUG_VAL( COLOR_MODE_NORMAL, COLOR_BACK_BLACK, COLOR_FRONT_GREEN,
				           "###########################################\n"
				           "%s\n"
				           "###########################################\n", HINT_MOUNT_SUCCESS )
			}
			else {
				DEBUG_VAL( COLOR_MODE_NORMAL, COLOR_BACK_BLACK, COLOR_FRONT_RED,
				           "###########################################\n"
				           "%s\n"
				           "###########################################\n", HINT_MOUNT_FAIL )
			}
		}
	}
	return c_res;
}

/*
int is_dev_file(const char *file_path)
{
	struct stat sb;
	if (lstat(file_path, &sb)) {
		return AK_FAILED;
	}
	return ( S_ISBLK(sb.st_mode) || S_ISCHR(sb.st_mode) ) ;
}
*/

int init_res( )
{
	int i;

	memset( &g_card_result , 0 , sizeof( struct card_result ) ) ;
	g_card_result.ai_status_point[ TEST_STATUS_SPEED ] = SPEED_NUM;
	for( i = 0 ; i < TEST_STATUS_NUM ; i ++ ) {
		g_card_result.ai_status[ i ] = RESULT_STATUS_NOTEST;
	}
	return 0;
}

int print_key_value_result( )
{
	printf( "%s %d\n"
	        "%s %.2f\n"
	        "%s %llu\n"
	        "%s %.2f\n"
	        "%s %d\n"
	        "%s %d\n"
	        "%s %.2f\n"
	        "%s %.2f\n"
	        "%s %.2f\n"

	        "%s %.2f\n"
	        "%s %.2f\n"
	        "%s %.2f\n"
	        "%s %.2f\n"
	        "%s %.2f\n"
	        "%s %.2f\n"

	        "%s %d\n"
	        "%s %.2f\n"

	        "%s %d\n"
	        "%s %d\n"
	        "%s %.2f\n"

	        "%s %d\n"
	        "%s %d\n"
	        "%s %.2f\n"

	        "%s %d\n"
	        "%s %d\n"
	        "%s %.2f\n"

	        "%s %d\n"
	        "%s %d\n"
	        "%s %.2f\n"

	        "%s %d\n"
	        "%s %d\n"
	        "%s %.2f\n"

	        "%s %d\n"
	        "%s %.2f\n"
	        "%s %d\n",

	        "GLOBAL", g_card_result.ai_status[ TEST_STATUS_GLOBAL ],
	        "TOTAL_SIZE", g_card_result.f_total_size ,
	        "TOTAL_SECTOR", g_card_result.i_total_sector ,
	        "TOTAL_SEC", ( double )g_card_result.ai_status_us[ TEST_STATUS_GLOBAL ] / SEC2USEC,
	        "SPEED", g_card_result.ai_status[ TEST_STATUS_SPEED ],
	        "SPEED_POINT", g_card_result.ai_status_point[ TEST_STATUS_SPEED ] ,
	        "SPEED_SEC", ( double )g_card_result.ai_status_us[ TEST_STATUS_SPEED ] / SEC2USEC,
	        "SPEED_ERROR_PERCENT", g_card_result.f_speed_percent,
	        "SPEED_SIZE", g_card_result.f_speed_size,

	        "SPEED_READ_AVG", g_card_result.af_speed_read[ SPEED_TYPE_AVG ],
	        "SPEED_READ_MIN", g_card_result.af_speed_read[ SPEED_TYPE_MIN ],
	        "SPEED_READ_MAX", g_card_result.af_speed_read[ SPEED_TYPE_MAX ],
	        "SPEED_WRITE_AVG", g_card_result.af_speed_write[ SPEED_TYPE_AVG ],
	        "SPEED_WRITE_MIN", g_card_result.af_speed_write[ SPEED_TYPE_MIN ],
	        "SPEED_WRITE_MAX", g_card_result.af_speed_write[ SPEED_TYPE_MAX ],

	        "CARD", g_card_result.ai_status[ TEST_STATUS_CARD ],
	        "CARD_SEC", ( double )g_card_result.ai_status_us[ TEST_STATUS_CARD ] / SEC2USEC,

	        "CARD_POINT", g_card_result.ai_card_test[ CARD_TEST_TYPE_GLOBAL ] ,
	        "CARD_ERROR", g_card_result.ai_card_error[ CARD_TEST_TYPE_GLOBAL ] ,
	        "CARD_ERR_PER", g_card_result.af_card_percent[ CARD_TEST_TYPE_GLOBAL ] ,

	        "CARD_READ_POINT", g_card_result.ai_card_test[ CARD_TEST_TYPE_READ ] ,
	        "CARD_READ_ERROR", g_card_result.ai_card_error[ CARD_TEST_TYPE_READ ] ,
	        "CARD_READ_ERR_PER", g_card_result.af_card_percent[ CARD_TEST_TYPE_READ ] ,

	        "CARD_WRITE_POINT", g_card_result.ai_card_test[ CARD_TEST_TYPE_WRITE ] ,
	        "CARD_WRITE_ERROR", g_card_result.ai_card_error[ CARD_TEST_TYPE_WRITE ] ,
	        "CARD_WRITE_ERR_PER", g_card_result.af_card_percent[ CARD_TEST_TYPE_WRITE ] ,

	        "CARD_ERASE_POINT", g_card_result.ai_card_test[ CARD_TEST_TYPE_ERASE ] ,
	        "CARD_ERASE_ERROR", g_card_result.ai_card_error[ CARD_TEST_TYPE_ERASE ] ,
	        "CARD_ERASE_ERR_PER", g_card_result.af_card_percent[ CARD_TEST_TYPE_ERASE ] ,

	        "CARD_REWRITE_POINT", g_card_result.ai_card_test[ CARD_TEST_TYPE_REWRITE ] ,
	        "CARD_REWRITE_ERROR", g_card_result.ai_card_error[ CARD_TEST_TYPE_REWRITE ] ,
	        "CARD_REWRITE_ERR_PER", g_card_result.af_card_percent[ CARD_TEST_TYPE_REWRITE ] ,

	        "FATFS", g_card_result.ai_status[ TEST_STATUS_FATFS ],
	        "FATFS_SEC", ( double )g_card_result.ai_status_us[ TEST_STATUS_FATFS ] / SEC2USEC,
	        "FATFS_RES", g_card_result.i_fatfs_res );

	/*


FATFS_HINT "GOOD CARD. NO NEED FIX."
*/
	return 0;
}


