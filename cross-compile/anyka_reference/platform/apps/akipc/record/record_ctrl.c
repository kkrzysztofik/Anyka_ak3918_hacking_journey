/**
* Copyright (C) 2016 Anyka(Guangzhou) Microelectronics Technology CO.,LTD.
* File Name: record_ctrl.c
* Author: Huanghaitao
* Update date: 2016-03-22
* Description:
* Notes:
* History: V2.0.0 react revision
*/

#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <errno.h>
#include <string.h>
#include <fcntl.h>
#include <sys/mount.h>
#include <dirent.h>
#include <regex.h>

#include "record_ctrl.h"
#include "internal_time.h"

#include "ak_common.h"
#include "ak_thread.h"
#include "ak_cmd_exec.h"
#include "ak_sd_card.h"

#include "ak_vi.h"
#include "ak_venc.h"
#include "ak_ai.h"
#include "ak_aenc.h"
#ifdef CONFIG_DANA_SUPPORT
#include "ak_dana.h"
#endif
#include "ak_config.h"
#include "ak_alarm.h"
#include "ak_dvr_record.h"
#include "ak_dvr_file.h"
#include "ak_dvr_disk.h"
//#include "printcolor.h"

//#define CMD_FIND "find %s -type f | xargs ls -tc > /tmp/find_file_list.txt ; cat /tmp/find_file_list.txt | head -n 1 ; rm /tmp/find_file_list.txt -f"
#define CMD_FIND "find %s -type f | xargs ls -tc"
#define CMD_MKFS "mkfs.vfat %s"

#define INVALID_VALUE		-1
#define SECONDS_PER_DAY         (24*60*60)
/* if the sd card space is not enough, we'll check it per 5 seconds. */
#define CHECK_SD_SPACE_TIME     (5*1000)
#define LEN_SHELL_RES           1024
#define FILE_TEST_TF            "/mnt/test_tf"                                                      //test file used to check if TF card is writable
#define LEN_TEST_TF             16                                                                  //write length for testing TF card read-only status
#define LEN_CMD                 256
#define LEN_ERROR               1024
#define DEV_MMCBLK0             "/dev/mmcblk0"
#define DEV_MMCBLK0P1           "/dev/mmcblk0p1"
#define PATH_MOUNT_POINT        "/mnt"
#define CMD_FUSER               "fuser -k %s"
#define CMD_UMOUNT              "umount -lf %s"
#define CMD_MOUNT               "mount %s %s"
#define CMD_MMC_TEST            "mmc_test -D"                                                       //test command used to check whether the TF card is functioning normally
#define CMD_MOUNT_STATUS        "mount | grep -o '/dev/mmcblk0'"

#define LEN_FORMAT_RES          4096
#define LEN_MOUNT_RES           64                                                                  //buffer length for mount return result
#define NUM_TRY_UMOUNT          5                                                                   //retry count for mount or umount
#define FILE_TF_ERR_COUNT       "/sys/ak_info_dump/tf_err_count"                                    //file for retrieving TF card write error count
#define TF_WRITE_ERROR          \
"\n********************************************\n\
WRITE VIDEO TO TF CARD ERROR(ERROR TIMES:%d)\n\
********************************************\n"                                                     //TF card write error message

/* record plan action */
enum record_plan_action {
	RECORD_PLAN_NONE = 0X00,		//no record plan
	RECORD_PLAN_NO_CONFIG,			//record plan info not config
	RECORD_PLAN_START,				//record plan start
	RECORD_PLAN_END,				//record play end
};

/* record trigger action */
enum record_alarm_action {
	RECORD_ALARM_NONE = 0X00,		//no record alarm
	RECORD_ALARM_NO_CONFIG,			//record alarm info not config
	RECORD_ALARM_START,				//record alarm start
	RECORD_ALARM_END,				//record alarm end
};

/* send alarm mode */
enum send_alarm_mode_e {
	SEND_ALARM_MODE_ONLY_PHOTO = 0x00,      //only photo mode
	SEND_ALARM_MODE_BOTH,              		//both photo & record mode
};

/* send alarm mode */
enum alarm_sensitivity_level_e {
	ALARM_SENS_LEVEL_CLOSE = 0x00,			//close sensitivity
	ALARM_SENS_LEVEL_LOW,					//low sensitivity
	ALARM_SENS_LEVEL_MID,					//middle sensitivity
	ALARM_SENS_LEVEL_HIGH,					//high sensitivity
	ALARM_SENS_LEVEL_NUM					//level number
};

/* current plan record info */
typedef struct _plan_record {
	unsigned char status;				//plan record status
	int index;					//current started record plan's index
	int start_wday;     		//plan record start week day
	time_t start_time;     		//plan record start time
	time_t end_time;       		//plan record end time
	time_t next_time;      		//plan record next time
}plan_record_s;

/* current alarm record info */
typedef struct _alarm_record {
	unsigned char status;			//alarm record status
	unsigned int pre_time;      	//alarm record pre time, second
	unsigned int stop_time;      	//alarm record stop time, second
	unsigned int limit_time;      	//alarm record limit time, second
	time_t start_time;				//alarm detection start time
	time_t end_time;     			//end alarm time
}alarm_record_s;

struct record_ctrl_s {
	unsigned char run_flag;	        //record control thread run flag
	unsigned short except_type;	    //except type
	unsigned char detect_type;      //detection type
	unsigned char update_plan_time; //update plan time flag
	unsigned char format_card;      //format card flag
	struct ak_timeval space_time;   //check sd card space time

	void *vi_handle;				//video input handle
	void *ai_handle;				//audio input handle

	ak_pthread_t main_tid;			//read control main thread ID
	enum dvr_record_type record_mode;
	enum dvr_record_type switch_mode;
	plan_record_s plan;				//plan record info
	alarm_record_s alarm;
};

enum format_status {                                                                                //current state of SD card formatting
	FORMAT_FAILED = -1 ,                                                                        //format failed
	FORMAT_SUCCESS = 0 ,                                                                        //format succeeded
	FORMAT_PROC = 1 ,                                                                           //format in progress
	FORMAT_STOP = 2 ,                                                                           //format not started
} ;

static struct record_ctrl_s record_ctrl = {0};

static int i_format_status = FORMAT_STOP ;

static void repair_sd_card_ro(void)
{
	char res[1000] = {0};

	ak_cmd_exec("mount | grep '/dev/mmcblk0p1 on /mnt'", res, sizeof(res));
	if(strlen(res) > 0) {
		ak_print_normal_ex("remount /dev/mmcblk0p1\n");
		ak_cmd_exec("mount -o remount,rw /dev/mmcblk0p1", NULL, 0);
		return;
	}

	memset(res, 0x00, sizeof(res));

	ak_cmd_exec("mount | grep '/dev/mmcblk0 on /mnt'", res, sizeof(res));
	if(strlen(res) > 0) {
		ak_print_normal_ex("remount /dev/mmcblk0\n");
		ak_cmd_exec("mount -o remount,rw /dev/mmcblk0", NULL, 0);
		return;
	}
}

static void get_video_record_error(int error_code, const char *error_desc)
{
	ak_print_normal_ex("--- error_code=%d ---\n", error_code);
	switch(error_code){
	case DVR_EXCEPT_NO_VIDEO://can't capture video data
		break;
	case DVR_EXCEPT_NO_AUDIO://can't capture audio data
		break;
	case DVR_EXCEPT_MUX_SYSERR:
		ak_print_warning_ex("--- mux system error ---\n");
		break;
	case DVR_EXCEPT_SD_NO_SPACE://SD card space not enough
		ak_print_warning_ex("--- SD card no more space ---\n");
		ak_get_ostime(&record_ctrl.space_time);
		break;
	case DVR_EXCEPT_SD_RO://SD card read only
		ak_print_warning_ex("--- SD card read only, we'll repair it ---\n");
		break;
	case DVR_EXCEPT_REC_STOPPED:
		ak_print_warning_ex("--- record stopped because of memory ---\n");
		break;
	default:
		break;
	}

	record_ctrl.except_type |= error_code;
	ak_print_normal_ex("--- except_type=%d ---\n", record_ctrl.except_type);
}

static int report_exception(int except_type)
{
	unsigned short tmp_type = DVR_EXCEPT_NONE;

	if (record_ctrl.except_type & DVR_EXCEPT_SD_REMOVED) {
		tmp_type |= DVR_EXCEPT_SD_REMOVED;
	}
	if (record_ctrl.except_type & DVR_EXCEPT_SD_UMOUNT) {
		tmp_type |= DVR_EXCEPT_SD_UMOUNT;
	}
	if (record_ctrl.except_type & DVR_EXCEPT_SD_NO_SPACE) {
		tmp_type |= DVR_EXCEPT_SD_NO_SPACE;
	}
	if (record_ctrl.except_type & DVR_EXCEPT_SD_WRITE_ERROR) {
		tmp_type |= DVR_EXCEPT_SD_WRITE_ERROR;
	}
	/* we clear current exception type after report */
	record_ctrl.except_type = tmp_type;

	return AK_SUCCESS;
}


/**
 * get_latest_file - get latest file according to time sort
 * @file_name[OUT]:
 * return: 0 success, -1 failed
 * notes:
 */

#if 0 //no need this function
static int get_latest_file(char *file_name, int name_len)
{
	char cmd[256] ;
	struct video_record_config *record_config = ak_config_get_record();
	int i ;

	sprintf(cmd, CMD_FIND, record_config->path);                                                //build command to get the last file
	memset( file_name , 0 , name_len ) ;
	ak_cmd_exec( cmd , file_name, name_len);
	for(i=0;i< strlen(file_name) ;i++){                                                         //remove trailing newlines from the command result
		if( file_name[ i ] == '\n' ){
			file_name[i] = 0 ;
		}
	}

	if(strlen(file_name)>0){
		return AK_SUCCESS;
	}
	else {
		return AK_FAILED;
	}
}
#endif

			                                                                                        /*
			                                                                                         get_file_size - get the current file size
			                                                                                         * pc_filename: path/filename
			                                                                                         * return value: file size; returns AK_FAILED if the file does not exist
			                                                                                        */
int get_file_size( char *pc_filename )
{
	struct stat stat_buf ;
	memset( &stat_buf , 0 , sizeof( struct stat ) ) ;
	if( stat( pc_filename , &stat_buf ) < 0 ){
		return AK_FAILED ;
	}
	return ( unsigned int )stat_buf.st_size ;
}



			                                                                                        /*
			                                                                                         write_file - write the contents of pc_buff to the file specified by pc_filename
			                                                                                         * pc_filename: path/filename
			                                                                                         * pc_buff: data to write
			                                                                                         * i_size:  write length
			                                                                                         * i_size_limit: write length limit,
			                                                                                                         if i_size_limit > 0 and i_size + i_file_size > i_size_limit, clear then write
			                                                                                                         if i_size_limit > 0 and i_size + i_file_size <= i_size_limit, append to end
			                                                                                                         if i_size_limit = 0, append to end
			                                                                                                         if i_size_limit < 0, clear then write
			                                                                                         * return value: number of bytes written
			                                                                                        */
int write_file( char *pc_filename , char *pc_buff , int i_size , int i_size_limit )
{
	int i_fd , i_write = 0 ;
	int i_file_size ;

	if ( i_size <= 0 ) {
		return 0 ;
	}
	if ( ( i_size_limit > 0 ) &&
		 ( ( i_file_size = get_file_size( pc_filename ) ) >= 0 ) &&
		 ( ( i_size_limit == 0 ) || ( ( i_file_size + i_size ) <= i_size_limit ) ) ) {
		i_fd = open( pc_filename , O_WRONLY | O_CREAT | O_APPEND , 0666 ) ;                 //open in append mode at end of current file
	}
	else {
		i_fd = open( pc_filename , O_WRONLY | O_CREAT | O_TRUNC , 0666 ) ;                  //open in file creation (truncate) mode
	}
	if ( i_fd > 0 ) {
		i_write = write( i_fd , pc_buff , i_size ) ;                                        //write to file
		close( i_fd ) ;
	}
	return i_write ;
}

/**
 * check_sd_write_error - get sd card write error times.
 * @void
 * return: 0: no error ; >=1: error times
 * notes:
 */
static int check_sd_write_error(void)
{
	char ac_error[ LEN_TEST_TF ] ;
	FILE *pFILE = NULL ;

	if (( pFILE = fopen( FILE_TF_ERR_COUNT , "r") ) != NULL){
		fgets(ac_error, LEN_TEST_TF, pFILE);
		fclose( pFILE );
		return atoi( ac_error ) ;
	}
	else {
		return 0 ;
	}
}

/**
 * ak_dvr_check_sd_ro - check SD card readonly status
 * @void
 * return: 1 read only, 0 rw, -1 failed
 * notes:
 */
int ak_dvr_check_sd_ro(void)
{
	char ac_test[ LEN_TEST_TF ] ;

	if ( write_file( FILE_TEST_TF , ac_test , LEN_TEST_TF , -1 ) > 0 ) {                            //write to TF card to check if writable
		remove( FILE_TEST_TF ) ;                                                                    //delete the written test file
		return AK_FALSE ;                                                                           //current filesystem is writable, return AK_FALSE
	}
	else {
		return AK_TRUE ;                                                                            //current filesystem is read-only, return AK_TRUE
	}
}

static void check_sd_ro(void)
{
	switch(ak_dvr_check_sd_ro()) {                                                                  //determine if TF card is read-only by attempting a file write
	case AK_TRUE:
		ak_print_normal_ex("The SD Card is Read Only, repair it\n");
		ak_dvr_repair_ro_disk();
		break;
	case AK_FALSE:
		ak_print_normal_ex("The SD Card is OK\n");
		break;
	default:
		ak_print_normal_ex("check SD read only failed\n");
		break;
	}
}

/**
 * stop_record: stop the current record
 * @void:
 * return: none
 * notes: stop condition:
 *		1. any kind of exceptions, ex.: DVR_EXCEPT_SD_NO_SPACE
 *		2. plan record time is out
 */
static void stop_record(void)
{
	ak_print_normal_ex("except_type=0x%X\n", record_ctrl.except_type);
	if (!(DVR_EXCEPT_REC_STOPPED & record_ctrl.except_type)) {
		ak_dvr_record_stop();
	}

	record_ctrl.plan.status = RECORD_PLAN_NONE;
	record_ctrl.alarm.status = RECORD_ALARM_END;

	if (DVR_EXCEPT_SD_RO & record_ctrl.except_type) {
		repair_sd_card_ro();
	}

	if ((DVR_EXCEPT_MUX_SYSERR & record_ctrl.except_type)
		|| (DVR_EXCEPT_SD_RO & record_ctrl.except_type)) {
		/* check sdcard readonly again, if still RO, we will repair it. */
		check_sd_ro();
	}
}

/**
 * init_record_common - init record common part.
 * @file_type: record file type
 * @interval: audio frame interval
 * return: void
 */
static void init_record_common(enum dvr_file_type file_type, int interval)
{
	/* video record param init */
	struct record_param record;
	struct video_record_config *record_config = ak_config_get_record();
	struct camera_disp_config *camera = ak_config_get_camera_info();
	struct video_config *video = ak_config_get_sys_video();

	record.vi_handle = record_ctrl.vi_handle;
	record.ai_handle = record_ctrl.ai_handle;

	/* caution: keep record param as same as that of cloud preview */
	record.width = camera->main_width;
	record.height = camera->main_height;
	if (record.width <= 640) {
		record.enc_chn = ENCODE_SUB_CHN;
		record.video_type = video->sub_enc_type;
	} else {
		record.enc_chn = ENCODE_MAIN_CHN;
		record.video_type = video->main_enc_type;
	}

	record.file_fps = video->main_fps;
	record.file_bps = video->main_min_kbps;
	record.gop_len = video->main_gop_len;
	record.minqp = video->main_min_qp;
	record.maxqp = video->main_max_qp;
	record.br_mode = video->main_video_mode;;
	record.enc_grp = ENCODE_MAINCHN_NET;

	record.sample_rate = record_config->sample_rate;
	record.audio_type = record_config->audio_type;
	record.frame_interval = interval;
	record.file_type = file_type;
	record.duration = (record_config->duration * 1000);
	record.error_report = get_video_record_error;


	record.alarm_pre_time = record_config->alarm_pre_time ;
	record.alarm_stop_time = record_config->alarm_stop_time ;
	record.alarm_limit_time = record_config->alarm_limit_time ;

	ak_dvr_record_init(&record);
}

/**
 * check_plan_continuity - get record plan's next start time in plan table,
 *		and check the plan's continuity.
 * @plan_index[IN]: current started record plan's wday
 * @cur_wday[IN]: current started record plan's wday
 * return: 0 continuity; -1 interrupted
 * notes: we can use this to check the record plan's continuity
 */
static int check_plan_continuity(int plan_index, int cur_wday)
{
	if((plan_index >= MAX_RECORD_PLAN_NUM) || (plan_index < 0)){
		return AK_FAILED;
	}
	if((cur_wday >= DAY_PER_WEEK) || (cur_wday < 0)){
		return AK_FAILED;
	}

	struct video_record_config *record = ak_config_get_record();
	if(NULL == record){
		return AK_FAILED;
	}

	int i = 0;
	int ret = AK_FAILED;
	int diff_day = 0;
	int next_wday = 0;
	time_t next_time = 0;
	time_t cur_time = 0;

	for(i=cur_wday; i<DAY_PER_WEEK; ++i){
		next_wday = i + 1;
		++diff_day;
		if(DAY_PER_WEEK == next_wday){
			next_wday = 0;
		}

		ak_print_normal("\nrecord plan[%d], cur week_enalbe[%d]=%d, "
			"next week_enalbe[%d]=%d\n",
			plan_index, i, record->plan[plan_index].week_enalbe[i],
			next_wday, record->plan[plan_index].week_enalbe[next_wday]);

		if(record->plan[plan_index].active
			&& record->plan[plan_index].week_enalbe[next_wday]){
			cur_time = get_passed_seconds();
			next_time = (SECONDS_PER_DAY * diff_day)
				+ day_secs_to_total(record->plan[plan_index].start_time);

			ak_print_normal("\n\t cur_secs:\t %ld(s), %s\n",
				cur_time, ak_seconds_to_string(cur_time));
			ak_print_normal("\t plan_end_time:\t %ld(s), %s\n",
				record_ctrl.plan.end_time,
				ak_seconds_to_string(record_ctrl.plan.end_time));
			ak_print_normal("\t plan next_time:\t %ld(s), %s\n",
				next_time, ak_seconds_to_string(next_time));
			if((record_ctrl.plan.end_time + 1) >= next_time){
				/* update current record plan info */
				record_ctrl.plan.end_time = (SECONDS_PER_DAY * diff_day)
					+ day_secs_to_total(record->plan[plan_index].end_time);
				record_ctrl.plan.start_wday = next_wday;
				ak_print_normal_ex("diff_day=%d, start_wday=%d\n",
					diff_day, record_ctrl.plan.start_wday);
				ak_print_normal("new plan_end_time: %ld(s), %s\n",
					record_ctrl.plan.end_time,
					ak_seconds_to_string(record_ctrl.plan.end_time));
				ret = AK_SUCCESS;
			}
			break;
		}
	}

	return ret;
}

static void check_plan_record_start(void)
{
	struct video_record_config *record = ak_config_get_record();
	if(NULL == record){
		record_ctrl.plan.status = RECORD_PLAN_NO_CONFIG;
		return;
	}

	int i = 0;
	struct tm *cur = NULL;
	time_t start = 0;
	time_t end = 0;
	time_t cur_secs = 0;

	for(i=0; i<MAX_RECORD_PLAN_NUM; ++i){
		cur = get_tm_time();
#if 0
		ak_print_normal("\n\t record plan[%d], active=%d, week_enalbe[%d]=%d, "
			"start_time: %ld, end_time: %ld\n",
			i, record->plan[i].active,
			cur->tm_wday, record->plan[i].week_enalbe[cur->tm_wday],
			record->plan[i].start_time, record->plan[i].end_time);
#endif

		if(record->plan[i].active && record->plan[i].week_enalbe[cur->tm_wday]){
			cur_secs = get_passed_seconds();
			start = day_secs_to_total(record->plan[i].start_time);
			end = day_secs_to_total(record->plan[i].end_time);
			ak_print_normal("\t\t config->plan[%d].start_time:\t %ld(s)\n",
				i, record->plan[i].start_time);
			ak_print_normal("\t\t config->plan[%d].end_time:\t %ld(s)\n",
				i, record->plan[i].end_time);
			ak_print_normal("\t\t cur_time:\t %ld(s), %s\n",
				cur_secs, ak_seconds_to_string(cur_secs));
			ak_print_normal("\t\t start_time:\t %ld(s), %s\n",
				start, ak_seconds_to_string(start));
			ak_print_normal("\t\t end_time:\t %ld(s), %s\n",
				end, ak_seconds_to_string(end));
			if((cur_secs >= start) && (cur_secs < end)){
				record_ctrl.plan.index = i;
				record_ctrl.plan.start_wday = cur->tm_wday;
				record_ctrl.plan.end_time = end;
				record_ctrl.plan.status = RECORD_PLAN_START;
				ak_dvr_record_start(RECORD_TRIGGER_TYPE_PLAN);

				cur_secs = get_passed_seconds();
				ak_print_normal("\n\t we will start the record plan[%d]\n", i);
				ak_print_normal("\t cur_time:\t %ld(s), %s\n",
					cur_secs, ak_seconds_to_string(cur_secs));
				ak_print_normal("\t start_time:\t %ld(s), %s\n",
					start, ak_seconds_to_string(start));
				ak_print_normal("\t end_time:\t %ld(s), %s\n\n",
					end, ak_seconds_to_string(end));
				break;
			}
		}
	}
}

static void check_plan_record_stop(void)
{
	time_t cur_secs = get_passed_seconds();

	if(cur_secs >= record_ctrl.plan.end_time){
		/* get next record plan start time */
		if(check_plan_continuity(record_ctrl.plan.index,
			record_ctrl.plan.start_wday) < 0){
			ak_print_normal_ex("\n\t we will stop the record plan[%d]\n",
				record_ctrl.plan.index);
			ak_print_normal_ex("\t cur_time: %s\n", ak_seconds_to_string(cur_secs));
			ak_print_normal_ex("\t end_time: %s\n",
				ak_seconds_to_string(record_ctrl.plan.end_time));
			record_ctrl.plan.status = RECORD_PLAN_END;
			stop_record();
		}
	}
}

static void check_sd_card_space(void)
{
	struct ak_timeval cur_time;

	ak_get_ostime(&cur_time);
	long diff_time = ak_diff_ms_time(&cur_time, &(record_ctrl.space_time));

	if(diff_time >= CHECK_SD_SPACE_TIME){
		record_ctrl.except_type &= (~DVR_EXCEPT_SD_NO_SPACE);
		ak_dvr_file_create_list();
		ak_get_ostime(&(record_ctrl.space_time));
	}
}

int fuser_remount( )                                                                                //after TF card re-insertion, some TF cards show full storage capacity; re-mounting is needed to read the data
{
	char ac_cmd[ LEN_CMD ] , *pc_dev = NULL ;

	if ( ak_is_dev_file( DEV_MMCBLK0P1 ) == AK_TRUE ) {
		pc_dev = DEV_MMCBLK0P1 ;
	}
	else if ( ak_is_dev_file( DEV_MMCBLK0 ) == AK_TRUE ) {
		pc_dev = DEV_MMCBLK0 ;
	}
	else {
		return AK_FAILED ;
	}
	snprintf( ac_cmd , LEN_CMD , CMD_FUSER , PATH_MOUNT_POINT ) ;
	system( ac_cmd ) ;

	snprintf( ac_cmd , LEN_CMD , CMD_UMOUNT , PATH_MOUNT_POINT ) ;
	system( ac_cmd ) ;

	snprintf( ac_cmd , LEN_CMD , CMD_MOUNT , pc_dev , PATH_MOUNT_POINT ) ;
	system( ac_cmd ) ;

	return AK_SUCCESS ;
}

static int check_record_exception(void)
{
	int i_error_times ;

	/* re-check sd card space */
	if (record_ctrl.except_type & DVR_EXCEPT_SD_NO_SPACE) {                                         //SD card insufficient space was detected last time; re-check SD card space
		check_sd_card_space();
	}

	if(SD_STATUS_CARD_INSERT & ak_sd_check_insert_status()){                                        //check SD card insertion status; SD card is already inserted
		if((record_ctrl.except_type & DVR_EXCEPT_SD_REMOVED)){                                      //SD card was found removed last check; re-mount SD card
			ak_print_normal_ex("--- SD Card Inserted ---\n");
			fuser_remount( ) ;                                                                      //re-mount SD card
		}
		record_ctrl.except_type &= (~DVR_EXCEPT_SD_REMOVED);                                        //clear DVR_EXCEPT_SD_REMOVED flag
		if(AK_SUCCESS == ak_sd_check_mount_status()){                                               //check mount status; SD card is already mounted
			if(record_ctrl.except_type & DVR_EXCEPT_SD_UMOUNT){                                     //SD card was found unmounted in the last check
				ak_print_normal_ex("--- SD Card mounted ---\n");
				/* check sdcard readonly or not, and repair it. */
				check_sd_ro();                                                                      //check if SD card is in read-only state

				/* sd card may reinsert */
				ak_dvr_file_create_list();                                                          //create index linked list for recording files
			}
			record_ctrl.except_type &= (~DVR_EXCEPT_SD_UMOUNT);                                     //clear DVR_EXCEPT_SD_UMOUNT flag

			if ( (i_error_times = check_sd_write_error( )) > 0 ) {                                  //query SD card write error count
				if((record_ctrl.except_type & DVR_EXCEPT_SD_WRITE_ERROR)==0){                       //no write error was found in the last check
					record_ctrl.except_type |= DVR_EXCEPT_SD_WRITE_ERROR ;                          //set DVR_EXCEPT_SD_WRITE_ERROR flag
					ak_print_error_ex(TF_WRITE_ERROR, i_error_times);                               //print the corresponding error message
				}
			}
			else {
				record_ctrl.except_type &= (~DVR_EXCEPT_SD_WRITE_ERROR) ;                           //clear DVR_EXCEPT_SD_WRITE_ERROR flag
			}
		}else{
			record_ctrl.except_type |= DVR_EXCEPT_SD_UMOUNT;                                        //set DVR_EXCEPT_SD_UMOUNT flag
		}
	}else{                                                                                          //SD card not detected
		if(0 == (record_ctrl.except_type & DVR_EXCEPT_SD_REMOVED)){                                 //DVR_EXCEPT_SD_REMOVED was not set last check, meaning the SD card was just removed this cycle; print related message
			ak_print_normal_ex("--- SD Card has removed ---\n\n");
		}
		record_ctrl.except_type |= DVR_EXCEPT_SD_REMOVED;                                           //set REMOVED and UMOUNT flags
		record_ctrl.except_type |= DVR_EXCEPT_SD_UMOUNT;
	}
	return record_ctrl.except_type;
}

/* check_record_plan - check and change record plan status */
static int check_record_plan(void)
{
	switch(record_ctrl.plan.status){
	case RECORD_PLAN_NONE:
	case RECORD_PLAN_END:
		check_plan_record_start();
		break;
	case RECORD_PLAN_START:
		check_plan_record_stop();
		break;
	default:
		record_ctrl.plan.status = RECORD_PLAN_NONE;
		break;
	}

	return record_ctrl.plan.status;
}

static void check_alarm_record_start(void)
{
	time_t end = 0;
	time_t cur_secs = 0;
	int alarm_time = ak_alarm_get_time();
	static int alarm_time_bak = 0;

	if(0 == alarm_time) {
		return;
	}

	/* calendar time switch time back; alarm trigger time have been record. */
	if( ( alarm_time < alarm_time_bak ) ||
		( ( alarm_time < record_ctrl.alarm.end_time ) &&
		  ( alarm_time_bak != alarm_time ) ) ) {
		ak_print_normal_ex("alarm_time_bak:%d(s), alarm_time:%d(s)\n", alarm_time_bak, alarm_time);
		alarm_time_bak = alarm_time;
	}

	if(alarm_time != alarm_time_bak) {
		alarm_time_bak =alarm_time;
		cur_secs = get_passed_seconds() - record_ctrl.alarm.pre_time;           //start time = current timestamp minus pre-record seconds
		end = cur_secs + record_ctrl.alarm.pre_time + record_ctrl.alarm.stop_time;

		ak_print_normal_ex("alarm.start_time: %ld(s), %s\n", cur_secs, ak_seconds_to_string(cur_secs));
		ak_print_normal_ex("alarm.end_time: %ld(s), %s\n", end, ak_seconds_to_string(end));
		record_ctrl.alarm.start_time = cur_secs;
		record_ctrl.alarm.end_time = end;
		record_ctrl.alarm.status = RECORD_ALARM_START;
		//DEBUG_PRINT( COLOR_MODE_NORMAL , COLOR_BACK_BLACK , COLOR_FRONT_GREEN , "****** START RECORD ******\n" )
		ak_print_normal_ex( "****** START RECORD ******\n" );
		ak_dvr_record_start(RECORD_TRIGGER_TYPE_ALARM);
	}
}

static void check_alarm_record_stop(void)
{
	time_t cur_secs = get_passed_seconds();                                     //get current timestamp
	int alarm_time = ak_alarm_get_time();
	unsigned long i_diff;

	i_diff = record_ctrl.alarm.end_time - record_ctrl.alarm.start_time;
	if ( ( alarm_time != 0 ) && ( i_diff < record_ctrl.alarm.limit_time ) ) {
		record_ctrl.alarm.end_time = cur_secs + record_ctrl.alarm.stop_time;
		i_diff = record_ctrl.alarm.end_time - record_ctrl.alarm.start_time;
		if ( i_diff > record_ctrl.alarm.limit_time ) { //check if recording duration exceeds the recording file limit in seconds
			record_ctrl.alarm.end_time = record_ctrl.alarm.start_time + record_ctrl.alarm.limit_time;
		}
		/*
		DEBUG_PRINT( COLOR_MODE_NORMAL , COLOR_BACK_BLACK , COLOR_FRONT_BLUE ,
		             "alarm_time= %d record_ctrl.alarm.start_time= %lu record_ctrl.alarm.end_time= %lu record_ctrl.alarm.limit_time= %u cur_secs= %lu diff= %lu\n",
		             alarm_time, record_ctrl.alarm.start_time, record_ctrl.alarm.end_time , record_ctrl.alarm.limit_time , cur_secs ,i_diff )
		             */
	}
	/*
	DEBUG_PRINT( COLOR_MODE_NORMAL , COLOR_BACK_BLACK , COLOR_FRONT_BLUE ,
	             "record_ctrl.alarm.limit_time= %u RECORD_FILE_LEN= %ld REMAIN= %lu\n", record_ctrl.alarm.limit_time, i_diff , record_ctrl.alarm.end_time - cur_secs )
	             */
	ak_print_normal_ex( "record_ctrl.alarm.limit_time= %u RECORD_FILE_LEN= %ld REMAIN= %lu\n", record_ctrl.alarm.limit_time, i_diff , record_ctrl.alarm.end_time - cur_secs );
	if ( cur_secs >= record_ctrl.alarm.end_time ){ //check if the last trigger time exceeds the end time or the recording duration limit is met
		ak_print_normal_ex("alarm.start_time: %s\n", ak_seconds_to_string(record_ctrl.alarm.start_time));
		ak_print_normal_ex("alarm.end_time: %s\n", ak_seconds_to_string(record_ctrl.alarm.end_time));
		//DEBUG_PRINT( COLOR_MODE_NORMAL , COLOR_BACK_BLACK , COLOR_FRONT_RED , "****** STOP RECORD ******\n" )
		ak_print_normal_ex( "****** STOP RECORD ******\n" );
		stop_record();
		record_ctrl.alarm.status = RECORD_ALARM_END;
	}
}

/* check_record_alarm - check and change record alarm status */
static int check_record_alarm(void)
{
	switch(record_ctrl.alarm.status){
	case RECORD_ALARM_NONE:
	case RECORD_ALARM_END:
		check_alarm_record_start();
		break;
	case RECORD_ALARM_START:
		check_alarm_record_stop();
		break;
	default:
		record_ctrl.alarm.status = RECORD_ALARM_NONE;
		break;
	}

	return record_ctrl.alarm.status;
}

/**
 * change_mode - change record to new mode.
 * @void:
 * return: 0, success; -1, failed
 */
static int change_mode(void)
{
	/* change record mode */
	if((RECORD_PLAN_START == record_ctrl.plan.status)
		|| (RECORD_ALARM_START == record_ctrl.alarm.status)) {
		stop_record();
	}

	record_ctrl.record_mode = record_ctrl.switch_mode;
	switch(record_ctrl.record_mode){
	case RECORD_TRIGGER_TYPE_ALARM:
		ak_dvr_record_turn_on_trigger(record_ctrl.alarm.pre_time);
		break;
	case RECORD_TRIGGER_TYPE_PLAN:
		ak_dvr_record_turn_off_trigger();
		break;
	case RECORD_TRIGGER_TYPE_RESERVED:
		break;
	default:
		break;
	}

	return AK_SUCCESS;
}

int get_shell_res( char *pc_cmd , char *pc_result , int i_len_res )                                 //get shell command result
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

int remove_enter( char *pc_res , int i_len )                                                        //remove the first 0x0d 0x0a from the shell return result
{
	int i ;

	for( i = 0 ; i < i_len ; i ++ ){
		if ( ( pc_res[ i ] == '\n' ) || ( pc_res[ i ] == '\r' ) ) {
			pc_res[ i ] = 0 ;
			return i - 1 ;
		}
	}
	return i_len ;
}

int tf_mount_status( )                                                                              //current TF card mount status; return value: mounted AK_TRUE, unmounted AK_FALSE
{
	int i_len_res ;
	char ac_res[ LEN_MOUNT_RES ] ;

	i_len_res = get_shell_res( CMD_MOUNT_STATUS , ac_res , LEN_MOUNT_RES ) ;
	if ( i_len_res >= strlen( DEV_MMCBLK0 ) ) {
		return AK_TRUE ;
	}
	else {
		return AK_FALSE ;
	}
}

static int regexpr_compare( char *pc_pattern , int i_flags , char *pc_buff )                            //compare the obtained result using regex
{
	int i_status;
	regex_t regex_t_pattern;
	char ac_error[ LEN_ERROR ] ;

	if ( ( i_status = regcomp( &regex_t_pattern , pc_pattern , i_flags ) ) != REG_NOERROR ) {
		regerror( i_status , &regex_t_pattern , ac_error , LEN_ERROR ) ;
		return REG_NOMATCH ;
	}
	i_status = regexec( &regex_t_pattern , pc_buff , (size_t)0 , NULL , 0 ) ;
	regfree( &regex_t_pattern ) ;

	return i_status ;
}

static int format_card(void)
{
	int i_len_res, i, i_mount_status;
	char result[ LEN_FORMAT_RES ] = {0};
	char ac_cmd[ LEN_CMD ] , *pc_dev = NULL;

	i_format_status = FORMAT_PROC;
	if ((record_ctrl.run_flag) && ((RECORD_PLAN_START == record_ctrl.plan.status)
		|| (RECORD_ALARM_START == record_ctrl.alarm.status))) {
		stop_record();
	}

	if (ak_is_dev_file(DEV_MMCBLK0P1) == AK_TRUE) {                                                 //select the TF card device name
		pc_dev = DEV_MMCBLK0P1;
	}
	else if (ak_is_dev_file(DEV_MMCBLK0) == AK_TRUE) {
		pc_dev = DEV_MMCBLK0;
	}
	else {
		goto format_fail_end;
	}

	system( "lsof" );
	for(i = 0; i < NUM_TRY_UMOUNT; i ++) {                                                          //attempt to unmount the filesystem
		snprintf(ac_cmd , LEN_CMD , CMD_FUSER , PATH_MOUNT_POINT);
		system(ac_cmd);

		snprintf(ac_cmd , LEN_CMD , CMD_UMOUNT , PATH_MOUNT_POINT);
		system(ac_cmd);

		if ((i_mount_status= tf_mount_status()) == AK_FALSE){
			ak_print_notice_ex("TF card umount success.\n");
			break;
		}
	}

	if(i_mount_status == AK_TRUE){                                                                  //unmount failed
		ak_print_notice_ex("TF card umount failed.\n");
		goto format_fail_end;
	}

	i_len_res = get_shell_res(CMD_MMC_TEST , result , LEN_FORMAT_RES);                              //perform media check on the TF card
	ak_print_normal_ex("'%s' i_len_res = %d , '%s'\n", CMD_MMC_TEST , i_len_res , result);
	if ( regexpr_compare( "CARD PASS" , REG_EXTENDED | REG_NEWLINE | REG_NOSUB , result ) == REG_NOMATCH ) {
		ak_print_normal_ex("card test failed\n");
		goto format_fail_end;
	}
	ak_print_normal_ex("card test passed\n");

	ak_print_notice_ex("format card start!\n");
	snprintf(ac_cmd , LEN_CMD , CMD_MKFS , pc_dev);                                                 //create filesystem with mkfs.vfat
	system(ac_cmd);

	if (ak_is_dev_file(DEV_MMCBLK0P1) == AK_TRUE) {                                                 //select TF card device name /dev/mmcblk0p1
		pc_dev = DEV_MMCBLK0P1;
	}
	else if (ak_is_dev_file(DEV_MMCBLK0) == AK_TRUE) {                                              //select TF card device name /dev/mmcblk0
		pc_dev = DEV_MMCBLK0;
	}
	else {
		goto format_fail_end;
	}

	snprintf(ac_cmd , LEN_CMD , CMD_MOUNT , pc_dev , PATH_MOUNT_POINT);
	for(i = 0; i < NUM_TRY_UMOUNT; i ++) {                                                          //attempt to mount the filesystem
		system(ac_cmd);
		if ((i_mount_status = tf_mount_status()) == AK_TRUE) {
			break;
		}
	}
	if (i_mount_status == AK_TRUE) {
		record_ctrl.except_type |= DVR_EXCEPT_SD_UMOUNT;
		goto format_success_end;
	}
	else {
		goto format_fail_end;
	}
format_success_end:
	i_format_status = FORMAT_SUCCESS;
	return AK_SUCCESS;
format_fail_end:
	i_format_status = FORMAT_FAILED;
	return AK_FAILED;
}

/** record_ctrl_main
 * @arg[IN]: thread input arg
 * return: none
 * notes:
 */
static void* record_ctrl_main(void *arg)
{
	ak_print_normal_ex("thread id: %ld\n", ak_thread_get_tid());
	ak_thread_set_name("record_ctrl");

	while(record_ctrl.run_flag){
		check_record_exception();
		if(DVR_EXCEPT_NONE == record_ctrl.except_type){
			//DEBUG_PRINT( COLOR_MODE_NORMAL , COLOR_BACK_BLACK , COLOR_FRONT_BLUE , "record_ctrl.except_type= %d\n", record_ctrl.except_type )
			switch(record_ctrl.record_mode) {
			case RECORD_TRIGGER_TYPE_ALARM:
				//DEBUG_HINT( COLOR_MODE_NORMAL , COLOR_BACK_BLACK , COLOR_FRONT_BLUE )
				check_record_alarm();
				break;
			case RECORD_TRIGGER_TYPE_PLAN:
				//DEBUG_HINT( COLOR_MODE_NORMAL , COLOR_BACK_BLACK , COLOR_FRONT_BLUE )
				check_record_plan();
				break;
			default:
				break;
			}
		}else{
			/* we have aleady started record, then stop it */
			if((RECORD_PLAN_START == record_ctrl.plan.status)
				|| (RECORD_ALARM_START == record_ctrl.alarm.status)){
				stop_record();
			}

			report_exception(record_ctrl.except_type);                                  //retain only DVR_EXCEPT_SD_REMOVED, DVR_EXCEPT_SD_UMOUNT, DVR_EXCEPT_SD_NO_SPACE, DVR_EXCEPT_SD_WRITE_ERROR flags in record_ctrl.except_type
		}

		ak_sleep_ms(300);
		if (record_ctrl.format_card) {
			format_card();
			record_ctrl.format_card = 0;
		}
		if (record_ctrl.update_plan_time) {
			if ((RECORD_TRIGGER_TYPE_PLAN == record_ctrl.record_mode)
				&& (RECORD_PLAN_START == record_ctrl.plan.status)) {
				stop_record();
			}
			record_ctrl.update_plan_time = 0;
		}
		if (record_ctrl.record_mode != record_ctrl.switch_mode) {
			change_mode();
		}
	}

	ak_print_normal_ex("### thread id: %ld exit ###\n", ak_thread_get_tid());

	ak_thread_exit();
	return NULL;
}

/**
 * record_ctrl_update_dvr_plan_time - tell dvr, dvr plan time have been update.
 * @ void:
 * return: 0, success; -1, failed
 */
int record_ctrl_update_dvr_plan_time(void)
{
	ak_print_normal_ex("\n");
	record_ctrl.update_plan_time = 1;

	return AK_SUCCESS;
}

/**
 * record_ctrl_request_format_card - tell dvr to do format card.
 * @ void:
 * return: 0, success; -1, failed
 */
int record_ctrl_request_format_card(void)
{
	ak_print_normal_ex("\n");
	if (record_ctrl.run_flag) {
		record_ctrl.format_card = 1;
	}
	else{
		format_card();
	}
	return AK_SUCCESS;
}

/**
 * record_ctrl_get_format_status - get status of format card.
 * @ void:
 * return: 0, success; 1, in progress; -1, failed
 */
int record_ctrl_get_format_status(void)
{
	return i_format_status ;
}

/**
 * record_ctrl_set_mode - set record control mode.
 * @new_mode[IN]:  new record control mode to be set
 * return: 0, success; -1, failed
 */
int record_ctrl_set_mode(enum dvr_record_type new_mode)
{
	if ((new_mode == record_ctrl.record_mode)
		|| (new_mode < RECORD_TRIGGER_TYPE_RESERVED)
		|| (new_mode > RECORD_TRIGGER_TYPE_ALARM)) {
		ak_print_error_ex("argument fail. new_mode: %d\n", new_mode);
		return AK_FAILED;
	}

	ak_print_notice_ex("record new_mode: %d\n", new_mode);
	record_ctrl.switch_mode = new_mode;

	return AK_SUCCESS;
}

/**
 * record_ctrl_init - init record control env and other sub-modules.
 *      start record control thread.
 * @vi_handle[IN]: success opened video input handle
 * @ai_handle[IN]: success opened audio input handle
 * @file_type[IN]: record file type
 * return: none
 */
void record_ctrl_init(void *vi_handle, void *ai_handle,
					enum dvr_file_type file_type)
{
	if(record_ctrl.run_flag){
		ak_print_normal_ex("already inited...\n");
		return;
	}

	struct video_record_config *record_config = ak_config_get_record();
	ak_print_normal("\n---------------------------------------------------------"
		   "-----------------------\n");
	ak_print_normal("\t entering record_ctrl_init\n");
	ak_print_normal("\t %s, record_mode: %s\n",
		get_readable_time_string(),
		(RECORD_TRIGGER_TYPE_PLAN == record_config->record_mode ?
			"plan record" : "alarm record"));

	int interval = 0;
	int sample_rate = record_config->sample_rate > 0 ? record_config->sample_rate : 8000;

	switch(record_config->audio_type) {
		case AK_AUDIO_TYPE_AAC:
			interval = ((1024 *1000) / sample_rate); /* 1k data in 1 second */
			break;
		case AK_AUDIO_TYPE_AMR:
			interval = AMR_FRAME_INTERVAL;
			break;
		case AK_AUDIO_TYPE_PCM_ALAW:	/* G711, alaw */
		case AK_AUDIO_TYPE_PCM_ULAW:	/* G711, ulaw */
			interval = AUDIO_DEFAULT_INTERVAL;
			break;
		case AK_AUDIO_TYPE_PCM:
			interval = AUDIO_DEFAULT_INTERVAL;
			break;
		case AK_AUDIO_TYPE_MP3:
			if (sample_rate >= 8000 && sample_rate <= 24000) {
				interval = 576*1000/sample_rate;
			} else { // sample_rate =32000 or 44100 or 48000
				interval = 1152*1000/sample_rate;
			}
			break;
		default:
			interval = AUDIO_DEFAULT_INTERVAL;
			break;

	}



	record_ctrl.vi_handle = vi_handle;
	record_ctrl.ai_handle = ai_handle;

	/* init record common info */
	init_record_common(file_type, interval);

	record_ctrl.record_mode = record_config->record_mode;
	record_ctrl.switch_mode = record_ctrl.record_mode;
	record_ctrl.run_flag = AK_TRUE;
	record_ctrl.except_type = (DVR_EXCEPT_SD_REMOVED | DVR_EXCEPT_SD_UMOUNT);
	record_ctrl.detect_type = SYS_DETECT_TYPE_NUM;
	record_ctrl.update_plan_time = 0;
	record_ctrl.format_card = 0;
	/* alarm record info */
	record_ctrl.alarm.status = RECORD_ALARM_NONE;
	record_ctrl.alarm.start_time = 0;
	record_ctrl.alarm.end_time = 0;
	record_ctrl.alarm.pre_time= record_config->alarm_pre_time;
   	record_ctrl.alarm.stop_time = record_config->alarm_stop_time;
	record_ctrl.alarm.limit_time = record_config->alarm_limit_time;
   	if(RECORD_TRIGGER_TYPE_ALARM == record_ctrl.record_mode){
		ak_dvr_record_turn_on_trigger(record_ctrl.alarm.pre_time);
	}

	ak_print_normal_ex("alarm.pre_time=%d, alarm.stop_time=%d\n",
		record_ctrl.alarm.pre_time, record_ctrl.alarm.stop_time);

	/* plan record info */
	record_ctrl.plan.status = RECORD_PLAN_NONE;
	record_ctrl.plan.index = INVALID_VALUE;
	record_ctrl.plan.start_wday = INVALID_VALUE;
	record_ctrl.plan.start_time = 0;
	record_ctrl.plan.end_time = 0;
	record_ctrl.plan.next_time = 0;

	int ret = ak_thread_create(&record_ctrl.main_tid, record_ctrl_main,
		NULL, ANYKA_THREAD_MIN_STACK_SIZE, -1);
	if(AK_FAILED == ret){
		record_ctrl.run_flag = AK_FALSE;
		ak_print_normal_ex("create record_ctrl_main thread failed, ret = %d!\n",
			ret);
	}
}

/** record_ctrl_exit: clear record control env and other sub-modules.
 *      stop record control thread and release resource.
 * @void
 * return: none
 */
void record_ctrl_exit(void)
{
	ak_print_normal("\n\t***** enter %s *****\n", __func__);
	if(record_ctrl.run_flag){
		ak_dvr_record_exit();
		record_ctrl.run_flag = AK_FALSE;
		ak_print_notice_ex("join record control main thread\n");
		ak_thread_join(record_ctrl.main_tid);
		ak_print_notice_ex("record control main thread join OK\n");
		ak_print_normal_ex("record ctrl has stopped!\n");
	}
	ak_print_normal("\t ***** %s OK *****\n\n", __func__);
}
