/*
 * Copyright (c) 2021 Ricardo JL Rufino.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, version 3.
 *
 * This program is distributed in the hope that it will be useful, but
 * WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
 * General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

/**
* Copyright (C) Ricardo JL Rufino
* Description: Capture camera for Anyka
* Created on: 17 de fev. de 2022
*/

/**
* Modified by Gerge
* Description: RTSP, JPEG snapshot and motion detection
* V1 created on: 09/04/2024
*/
#include <stdio.h>
#include <string.h>
#include <dirent.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <unistd.h>

#include <errno.h>
#include <signal.h>
#include <sys/time.h>
#include <time.h>

#include "ak_common.h"
#include "ak_vi.h"
#include "ak_thread.h"
#include "ak_ipc_srv.h"
#include "ak_venc.h"
#include "ak_sd_card.h"
#include "ak_dvr_file.h"
#include "record_common.h"

// App includes
//#include "convert.h"
#include "log.h"
//#include "app_settings.h"
#include "utils.h"
#include "http_server.h"
#include "snapshot.h"

//rtsp includes
#include <stdlib.h>
//#include <getopt.h>
#include "ak_rtsp.h"
#include "ak_ai.h"
#include "ak_aenc.h"
#include "ak_drv_ir.h"

//motion detect
#include "ak_vpss.h"

#define SAVE_PATH        	"/mnt/video_encode"	//default save path
#define IRCUT_A_FILE_NAME       "/sys/user-gpio/gpio-ircut_a"
#define IRLED_FILE_NAME		"/sys/user-gpio/IR_LED"
#define STRING_LEN	            128

struct video_handle {
	void *venc_handle;					//video encode handle
	void *stream_handle;				//video stream handle
	enum encode_group_type enc_type;	//current encode type
	ak_pthread_t ak_venc_tid;			//thread id
};

struct resolution_t {
	unsigned int width;
	unsigned int height;
	unsigned char str[20];
};

struct md_judge_param {
	int level_threshold;		//value range: [0, 65536)
	int blocks_threshold;		//md blocks trigger md
};

enum md_judge_result {
	JUDGE_RESULT_ERR = 0,
	JUDGE_RESULT_MD,
	JUDGE_RESULT_NO_MD
};

enum day_ctrl_level {
	DAY_LEVEL_RESERVED = 0x00,
	DAY_LEVEL_HH,		//high-high
	DAY_LEVEL_HL,		//high-low
	DAY_LEVEL_LH,		//low-high
	DAY_LEVEL_LL,		//low-low
	DAY_LEVEL_MAX
};

enum day_night_switch_mode {
	SET_NIGHT_MODE,
	SET_DAY_MODE,
	SET_AUTO_MODE,
	SET_CLOSE_MODE
};

enum ak_photosensitive_mode {
	HARDWARE_PHOTOSENSITIVE,	// hardware photosensitive
	AUTO_PHOTOSENSITIVE		// auto ir switch
};

struct ak_misc {
    int ircut_run_flag;	    //photosensitive and ircut switch run flag
    int ps_switch_enable;   //store photosensitive switch status
    int day_level_ctrl;		//day level default led-ircut config
    int ipc_run_flag;		//ipc run flag
    int pre_status;			//current state,0 night, 1 day
    void *vi_handle;		//global vi handle
    ak_pthread_t ircut_tid;
	int cur_set_mode;
	int thread_run;
	enum ak_photosensitive_mode ps_mode;
	thread_func ps_fun_thread;// //photosensitive thread function
};


static struct ak_misc misc_ctrl = {0};

#define DEFAULT_FPS                 20
#define DEFAULT_MINQP               20
#define DEFAULT_MAXQP               51
#define DEFAULT_GOP                 2
#define DEFAULT_MODE           BR_MODE_CBR                                                     //BR_MODE_CBR,BR_MODE_VBR
#define DEFAULT_TYPE           H264_ENC_TYPE	//H264_ENC_TYPE HEVC_ENC_TYPE,HEVC_ENC_TYPE
#define DEFAULT_MAIN_KBPS           2000
#define DEFAULT_SUB_KBPS            200
#define DEFAULT_MAIN_SUFFIX         "vs0"
#define DEFAULT_SUB_SUFFIX          "vs1"


volatile sig_atomic_t stop;					//stop if error occurs
#define cfg "/etc/jffs2"

// Camera vars
void *vi_handle = NULL;						//vi operating handle
struct video_resolution res;				//max sensor resolution
int w_main = 1280;
int h_main = 720;
int w_sub = 640;
int h_sub = 360;
int flipmirror=0;	//option -u "upside-down" mode rotate 180
int irinvert=1;		//option -i value 1-4 can invert IR chutter and image colour

struct video_handle ak_venc[ENCODE_GRP_NUM];
int md_record_frames =0; //countdown of how many more frames to record
#define md_max_frames 600 //max frames in one file
int motion_record_sec = 0; //user defined record time after motion trigger
//int md_record_running = 0; //set to 1 when recording

struct snapshot_t snapshot_ref = {
	.count = 0,  //at the start encode this many JPEGs
	.capture = 0, //capture first image to buffer 0
	.res_w = 1280, //default resolution
	.res_h = 720,
	.jpeg = {{0},{0}} //image buffers
};

void stop_capture(){
	logi("stop_capture");
	ak_rtsp_stop(VIDEO_CHN_MAIN);
	ak_rtsp_stop(VIDEO_CHN_SUB);
	ak_rtsp_exit();
	// step 7: release resource
	stop_server();
}

void my_handler(int s){
   printf("Caught signal %d - Exiting... \n",s);
   stop=1;
   stop_capture();
   //exit(1);
}
//return: 0 success, -1 failed
static int check_sd(){
	int ret=-1;
	if(SD_STATUS_CARD_INSERT & ak_sd_check_insert_status()){  //check sd card insert or not
		ak_print_normal("SD Card Inserted\n");
		if(AK_SUCCESS == ak_sd_check_mount_status()){
			ak_print_normal("SD Card mounted\n");
		} else {
			ak_print_normal("SD Card not mounted\n");
			return -1;
		}
	} else {
		ak_print_normal("SD Card not Inserted\n");
		return -1;
	}
	return 0;
}

static void *venc_open_file(int enc_type)
{
	char file[128] = {0};
	char time_str[32] = {0};
	const char *suffix[2] = {".str", ".jpg"};
	struct ak_date date;

	// get time string
	ak_get_localdate(&date);
	ak_date_to_string(&date, time_str);

	// distinguish stream and jpeg
	if (enc_type < ENCODE_PICTURE) {
		sprintf(file, "%s/%s_%d_%s", SAVE_PATH, time_str, enc_type, suffix[0]);
		ak_print_normal_ex("[ H264 ] save file: %s\n", file);
	} else {
		static int jpg_cnt = 0;

		sprintf(file, "%s/%s_%d_%s", SAVE_PATH, time_str, jpg_cnt, suffix[1]);
		ak_print_normal_ex("[ JPG ] save file: %s\n", file);
		jpg_cnt++;
	}

	// open for all mode
	FILE *fp = NULL;
	fp = fopen(file, "a");
	if (!fp) {
		//ak_print_error_ex("open [%s] failed, %s\n", file, strerror(errno));
		return NULL;
	}

	return fp;
}

static void venc_save_data(FILE *filp, unsigned char *data, int len)
{
	int ret = len;
	if (filp) {
		/* make sure all data were writ to file */
		do {
			ret -= fwrite(data, 1, ret, filp);
		} while (ret > 0);
	}
}

static void *venc_demo_open_encoder(int index)
{
	struct encode_param param = {0};

	switch (index) {
	case 0:
		param.width = w_main;
		param.height = h_main;
		param.minqp = 20;
		param.maxqp = 51;
		param.fps = ak_vi_get_fps(vi_handle);
		param.goplen = param.fps * 2;	   //current gop is stationary
		param.bps = 2000;				   //kbps
		param.profile = PROFILE_MAIN;	   //main profile
		param.use_chn = ENCODE_MAIN_CHN;   //use main yuv channel data
		param.enc_grp = ENCODE_RECORD;     //assignment from enum encode_group_type
		param.br_mode = BR_MODE_CBR;	   //default is cbr
		param.enc_out_type = H264_ENC_TYPE;//h.264
		break;
	case 1:
		param.width = w_main;
		param.height = h_main;
		param.minqp = 20;
		param.maxqp = 51;
		param.fps = 10;
		param.goplen = param.fps * 2;
		param.bps = 2000;	//kbps
		param.profile = PROFILE_MAIN;		//same as above
		param.use_chn = ENCODE_MAIN_CHN;
		param.enc_grp = ENCODE_MAINCHN_NET;	//just this scope difference
		param.br_mode = BR_MODE_CBR;
		param.enc_out_type = H264_ENC_TYPE;
		break;
	case 2:
		param.width = snapshot_ref.res_w;
		param.height = snapshot_ref.res_h;
		param.minqp = 20;
		param.maxqp = 51;
		param.fps = 10;
		param.goplen = param.fps * 2;
		param.bps = 300;	//kbps
		param.profile = PROFILE_MAIN;
		param.use_chn = ENCODE_SUB_CHN;		//use sub yuv channel data
		param.enc_grp = ENCODE_SUBCHN_NET;	//same as above
		param.br_mode = BR_MODE_CBR;
		param.enc_out_type = H264_ENC_TYPE;
		break;
	case 3:
		param.width = snapshot_ref.res_w;
		param.height = snapshot_ref.res_h;
		param.minqp = 20;
		param.maxqp = 51;
		param.fps = 10;
		param.goplen = param.fps * 2;
		param.bps = 500;	//kbps
		param.profile = PROFILE_MAIN;
		param.use_chn = ENCODE_SUB_CHN;
		param.enc_grp = ENCODE_PICTURE;		//jpeg encode
		param.br_mode = BR_MODE_CBR;
		param.enc_out_type = MJPEG_ENC_TYPE;	//jpeg encode
		break;
	default:
		return NULL;
		break;
	}

	return ak_venc_open(&param);
}

static void venc_stop_stream(enum encode_group_type grp)
{
	struct video_handle *enc = &ak_venc[grp];

	if (!enc->ak_venc_tid)
		return;
	/* wait thread exit */
	ak_thread_join(enc->ak_venc_tid);

	/* release resource */
	ak_venc_cancel_stream(enc->stream_handle);
	enc->stream_handle = NULL;
	ak_venc_close(enc->venc_handle);
	enc->venc_handle = NULL;
}

static void *venc_save_stream_thread(void *arg)
{
	long int tid = ak_thread_get_tid();
	ak_print_normal_ex("start a work thread, tid: %ld\n", tid);

	struct video_handle *handle = (struct video_handle *)arg;

	while (!stop) {
		while ((md_record_frames == 0)&&(!stop)){
			//don't use h264 flames
			ak_sleep_ms(10);
			//h264 encoder buffer will be full and only the last 2s will be kept
		}
		/* first, open file */
		if ((md_record_frames > 0)&&(!stop)){
			FILE *fp = venc_open_file(handle->enc_type);
			//ak_print_error_ex("########### open ###########\n");
			int frame_num = 0;
			//md_record_running = 1;

			/*
			* according number of total save frame, iterate
			* specific times
			*/
			while ((frame_num <= md_max_frames) && (md_record_frames > 0)&&(!stop)) {
				struct video_stream stream = {0};
				/* get stream */
				int ret = ak_venc_get_stream(handle->stream_handle, &stream);
				if (ret) {
					ak_sleep_ms(10);
					continue;
				}
				ak_print_normal_ex("[tid: %ld, chn: %d] get stream, size: %d, frame: %d\n",
						tid, handle->enc_type, stream.len, frame_num);

				venc_save_data(fp, stream.data, stream.len);

				/* release frame */
				ret = ak_venc_release_stream(handle->stream_handle, &stream);
				frame_num++;
				md_record_frames--;

				//if ((frame_num % 3) == 0)
				//	ak_sleep_ms(10);
			}

			/* when write done, close file */
			if (fp)
				fclose(fp);
			//md_record_running = 0;
		}
	}
	/* stop video encode */
	venc_stop_stream(handle->enc_type);
	ak_print_normal_ex("### thread id: %ld exit ###\n", ak_thread_get_tid());
	ak_thread_exit();

	return NULL;
}

static int venc_start_stream(enum encode_group_type grp)
{
	struct video_handle *handle = &ak_venc[grp];

	printf("stream encode mode, start encode group: %d\n", grp);

	/* init encode handle */
	handle->venc_handle = venc_demo_open_encoder(grp);
	if (!handle->venc_handle) {
		printf("video encode open type: %d failed\n", grp);
		return -1;
	}

	/* request stream, video encode module will start capture and encode */
	handle->stream_handle = ak_venc_request_stream(vi_handle,
			handle->venc_handle);
	if (!handle->stream_handle) {
		printf("request stream failed\n");
		return -1;
	}
	handle->enc_type = grp;
	/* create thread to get stream and do you primary mission */
	ak_thread_create(&handle->ak_venc_tid, venc_save_stream_thread,
			(void *)handle, 100*1024, 90);

	return 0;
}

//motion detection
static int md_demo(const void *vi_handle, const struct md_judge_param *judge)
{
	struct vpss_md_info md;

	int ret = ak_vpss_md_get_stat(vi_handle, &md);
	if (ret) {
		//ak_print_error_ex("ak_md get fail\n");
		ret = JUDGE_RESULT_ERR;
	} else {
		int i, j;
		int md_cnt = 0;
		//check motion level against threshold in all 16x32 blocks
		for(i=0; i<16; i++) {
			for(j=0; j<32; j++) {
				if(md.stat[i][j] > judge->level_threshold)
					md_cnt++;
			}
		}

		if (md_cnt > judge->blocks_threshold)
			ret = JUDGE_RESULT_MD;
		else
			ret = JUDGE_RESULT_NO_MD;
	}

	return ret;
}

int capture_init(){
	logi("image module init");

	int ret = -1;								//return value
	struct video_channel_attr attr;				//vi channel attribute

	attr.crop.left = 0;
	attr.crop.top  = 0;
	attr.crop.width = 1280;
	attr.crop.height = 720;
	if ((snapshot_ref.res_w < 640) || (snapshot_ref.res_h < 480)){
		w_sub = snapshot_ref.res_w;
		h_sub = snapshot_ref.res_h;
	}else{
		w_main = snapshot_ref.res_w;
		h_main = snapshot_ref.res_h;
	}

	attr.res[VIDEO_CHN_SUB].width = w_sub;
	attr.res[VIDEO_CHN_SUB].height = h_sub;
	attr.res[VIDEO_CHN_SUB].max_width = w_sub; //seems to have no effect
	attr.res[VIDEO_CHN_SUB].max_height = h_sub; //seems to have no effect

	attr.res[VIDEO_CHN_MAIN].width = w_main;
	attr.res[VIDEO_CHN_MAIN].height = h_main;
	attr.res[VIDEO_CHN_MAIN].max_width = w_sub; //janky but works to set lower resolutions
	attr.res[VIDEO_CHN_MAIN].max_height = h_sub; //possible values 2x2 to 640x480 (don't ask me why, I did not write this stupid library) it seems to control sub channel rather than main

	printf("VIDEO_CHN_SUB index %d Resolution: %d x %d\n", VIDEO_CHN_SUB, w_sub, h_sub);
	printf("VIDEO_CHN_MAIN index %d Resolution: %d x %d\n", VIDEO_CHN_MAIN, w_main, h_main);

	// step 1: match sensor
	// the location of isp config can either a file or a directory
	ret = ak_vi_match_sensor(cfg);
	if (ret) {
		loge("match sensor failed\n");
		return -1;
	}

	// step 2: open video input device
	vi_handle = ak_vi_open(VIDEO_DEV0);
	if (NULL == vi_handle) {
		loge("vi device open failed\n");
		return -1;
	}

	// step 3: get sensor support max resolution
	ret = ak_vi_get_sensor_resolution(vi_handle, &res);
	logv("ak_vi_get_sensor_resolution\n");
	if (ret) {
		ak_vi_close(vi_handle);
		return -1;
	} else {
		attr.crop.width = res.width;
		attr.crop.height = res.height;
	}

	// step 4: set vi working parameters
	// default parameters: 25fps, day mode, auto frame-control
	ret = ak_vi_set_channel_attr(vi_handle, &attr);
	logv("ak_vi_set_channel_attr\n");
	if (ret) {
		loge("ak_vi_set_channel_attr FAIL: GO TO CLOSE\n");
		ak_vi_close(vi_handle);
		return -1;
	}

	// step 5: start capture frames
	ret = ak_vi_capture_on(vi_handle);
	logv("ak_vi_capture_on\n");
	if (ret) {
		ak_vi_close(vi_handle);
		return -1;
	}
	if (flipmirror == 1){
		ak_vi_set_flip_mirror(vi_handle, 1, 1); //flip + mirror = rotate 180 degrees
	}

	return 1;
}

static int camera_set_ir(int value, char *name)
{
	char cmd[STRING_LEN];
	char result[STRING_LEN];

	memset(result, '\0', STRING_LEN);
	sprintf(cmd, "echo %d > %s", value, name);
	system(cmd);

	return 0;
}


/**
 * ak_misc_set_video_day_night: set video day or night mode, according to IR value
 * @vi_handle: opened vi handle
 * @ir_val: IR value, [0, 1]
 * @day_level: day control level, [1, 4]
 * return: 0 success, -1 failed
 */
int ak_misc_set_video_day_night(void *vi_handle, int ir_val, int day_level)
{
	if (!vi_handle) {
		return AK_FAILED;
	}

	int day_val = 0;
	int ir_switch_val = 0;

	switch (day_level) {
	case DAY_LEVEL_HH:
		ir_switch_val = ir_val;
		day_val = ir_val;
		break;
	case DAY_LEVEL_HL:
		ir_switch_val = !ir_val;
		day_val = ir_val;
		break;
	case DAY_LEVEL_LH:
		ir_switch_val = !ir_val;
		day_val = !ir_val;
		break;
	case DAY_LEVEL_LL:
		ir_switch_val = ir_val;
		day_val = !ir_val;
		break;
	default:
		return AK_FAILED;
	}

	int ret = AK_FAILED;
	if (day_val) {
		ak_print_notice("now set to day\n");
		camera_set_ir(!ir_val, IRCUT_A_FILE_NAME);
		ret = ak_vi_switch_mode(vi_handle, VI_MODE_DAY);
		camera_set_ir(!ir_val, IRLED_FILE_NAME);
	} else {
		ak_print_notice("now set to night\n");
		camera_set_ir(!ir_val, IRLED_FILE_NAME);
		ret = ak_vi_switch_mode(vi_handle, VI_MODE_NIGHT);
		camera_set_ir(!ir_val, IRCUT_A_FILE_NAME);
	}

	ak_sleep_ms(300);
	/* to notify av mode to modify gop */


	return ret;
}


static void *photosensitive_switch_th_ex(void *arg)
{
	long int tid = ak_thread_get_tid();
	int ir_val = 0;

	ak_print_normal_ex("Thread start, id: %ld\n", tid);
	ak_thread_set_name("PS_switch");

	misc_ctrl.thread_run = AK_TRUE;
	/* get ir state and switch day-night */
	while (misc_ctrl.thread_run) {
		//ak_print_normal_ex("misc_ctrl.cur_set_mode=%d\n", misc_ctrl.cur_set_mode);
		if (misc_ctrl.cur_set_mode != SET_AUTO_MODE)
			break;

		ir_val = ak_drv_ir_get_input_level();

		if (ir_val != -1 && misc_ctrl.pre_status != ir_val) {
	        ak_print_notice("prev_state=%d, ir_val=%d\n",
		        misc_ctrl.pre_status, ir_val);
			ak_misc_set_video_day_night(vi_handle, !ir_val,
			    irinvert);
			misc_ctrl.pre_status = ir_val;
		}

		ak_sleep_ms(5000);
	}

	ak_print_normal_ex("Thread exit, id: %ld\n", tid);
	ak_thread_exit();
	return NULL;
}

int ak_misc_start_photosensitive_switch_ex(enum ak_photosensitive_mode ps_mode,
							 enum day_night_switch_mode day_night_mode)
{

	if (misc_ctrl.ircut_run_flag) {
		//ak_print_error_ex("misc already start\n");
		return AK_SUCCESS;
	}
	misc_ctrl.vi_handle = ak_vi_get_handle(VIDEO_DEV0);
	misc_ctrl.ircut_run_flag = AK_TRUE;

	misc_ctrl.cur_set_mode = day_night_mode;
	misc_ctrl.ps_mode = ps_mode;
	misc_ctrl.pre_status = -1;

	int ret = AK_SUCCESS;


		misc_ctrl.ps_fun_thread = photosensitive_switch_th_ex;

		ret = ak_thread_create(&misc_ctrl.ircut_tid, misc_ctrl.ps_fun_thread,
			NULL, 100 *1024, -1);

	return ret;
}


static void init_other_platform(void){
	ak_drv_ir_init();
	//start photosensitive ircut detect service
	ak_misc_start_photosensitive_switch_ex(HARDWARE_PHOTOSENSITIVE, 2);
}

void run_rtsp_stuff(){
	struct rtsp_param param = {{{0}}};

	// main channel config (640x480 to 1280x720)
	param.rtsp_chn[0].current_channel = 0;
	param.rtsp_chn[0].width 	= w_main;
	param.rtsp_chn[0].height 	= h_main;

	param.rtsp_chn[0].fps 	 	= DEFAULT_FPS;
	param.rtsp_chn[0].max_kbps 	= DEFAULT_MAIN_KBPS;
	param.rtsp_chn[0].min_qp	= DEFAULT_MINQP;
	param.rtsp_chn[0].max_qp 	= DEFAULT_MAXQP;
	param.rtsp_chn[0].gop_len	= DEFAULT_GOP;

	param.rtsp_chn[0].video_enc_type = DEFAULT_TYPE;
	param.rtsp_chn[0].video_br_mode  = DEFAULT_MODE;

	param.rtsp_chn[0].vi_handle = vi_handle;
	strcpy(param.rtsp_chn[0].suffix_name, "vs0");

	// sub channel config (up to 640x480)
	param.rtsp_chn[1].current_channel = 1;
	param.rtsp_chn[1].width 	= w_sub;
	param.rtsp_chn[1].height 	= h_sub;

	param.rtsp_chn[1].fps 	 	= DEFAULT_FPS;
	param.rtsp_chn[1].max_kbps 	= DEFAULT_SUB_KBPS;
	param.rtsp_chn[1].min_qp	= DEFAULT_MINQP;
	param.rtsp_chn[1].max_qp 	= DEFAULT_MAXQP;
	param.rtsp_chn[1].gop_len	= DEFAULT_GOP;

	param.rtsp_chn[1].video_enc_type = DEFAULT_TYPE;
	param.rtsp_chn[1].video_br_mode  = DEFAULT_MODE;

	param.rtsp_chn[1].vi_handle = vi_handle;
	strcpy(param.rtsp_chn[1].suffix_name, "vs1");

	// init rtsp
	if (ak_rtsp_init(&param)) {
		ak_print_notice_ex("\n\t---- init rtsp failed ---- !\n");
		//return -1;
	}else{
		// start rtsp service
		ak_rtsp_start(VIDEO_CHN_MAIN);
		ak_rtsp_start(VIDEO_CHN_SUB);
		//photosensitive IR
		init_other_platform();
	}
}


void capture_loop(){

	//struct video_input_frame frame;
	void *jpeg_encoder = venc_demo_open_encoder(ENCODE_PICTURE);
	void *jpeg_stream = ak_venc_request_stream(vi_handle, jpeg_encoder);
	struct md_judge_param judge;
	int skip_md = 40;
	int trigger_count =0;
	judge.level_threshold = 5000;
	judge.blocks_threshold = 12;

	logv("capture start");

	if (motion_record_sec > 0){
		if (!check_sd()) {
			venc_start_stream(ENCODE_RECORD); //only encode if motion record flag is set
		}
	}

	// To get frame by loop
	while (!stop) {
		if ((motion_record_sec > 0) && (md_record_frames < 50)){
			if (skip_md == 0){
				if (JUDGE_RESULT_MD == md_demo(vi_handle, &judge)){
					ak_print_normal("motion detected!!\n");
					trigger_count++;
					if (trigger_count >= 2){ //only record when triggered 2 consecutive times to avoid false trigger
						md_record_frames=ak_vi_get_fps(vi_handle)*motion_record_sec;
						trigger_count=0;
					}
				}else{
					ak_print_normal("no motion\n");
					if (trigger_count == 1) trigger_count=0;
				}
				skip_md=2;
			}else{
				skip_md--;
			}
		}


		if (!ak_venc_get_stream(jpeg_stream, &snapshot_ref.jpeg[snapshot_ref.capture])) {
			snapshot_ref.capture = !snapshot_ref.capture; //mark current image for reading and reserve the other for writing next image
			free(snapshot_ref.jpeg[snapshot_ref.capture].data);
		} else {
			// not ready， sleep to release CPU
			ak_sleep_ms(10);
		}
		//__LOG_TIME_END("capture");

		ak_sleep_ms(100); // Free CPU to do other things
						// 25ms -> 60~70% CPU
	}
	printf("Capture loop exiting.\n");
	//ak_sleep_ms(2000);
	ak_venc_cancel_stream(jpeg_stream);
	jpeg_stream = NULL;
	ak_venc_close(jpeg_encoder);
	jpeg_encoder = NULL;
	ak_vi_capture_off(vi_handle);
	ak_vi_close(vi_handle);
}
void help_message(char* argv[]){
        fprintf(stderr, "Usage: %s -w <width> -h <height>\n", argv[0]);
        fprintf(stderr, "Example: %s -w 1280 -h 720\n", argv[0]);
        fprintf(stderr, "\nNOTE: If no arguments are given, the default resolution is: %d x %d\n", snapshot_ref.res_w, snapshot_ref.res_h);
        fprintf(stderr, "to enable motion trigger: %s -m <record_seconds>\n", argv[0]);
		fprintf(stderr, "to rotate image 180: %s -u\n", argv[0]);
		fprintf(stderr, "to inver IR and day/night: %s -i <value 1-4>\n", argv[0]);
}

int parse_args(int argc, char* argv[]){

    for (;;) {
        int opt = getopt(argc, argv, "w:h:m:i:u");
        if (opt == -1)
            break;
        switch (opt) {
        case '?':
            fprintf(stderr, "%s: Unexpected option: %c\n", argv[0], optopt);
            help_message(argv);
            return -1;
        case ':':
            fprintf(stderr, "%s: Missing value for: %c\n", argv[0], optopt);
            help_message(argv);
            return -1;
        case 'w':
            snapshot_ref.res_w = atoi(optarg);
            break;
        case 'h':
            snapshot_ref.res_h = atoi(optarg);
            break;
        case 'm':
            motion_record_sec = atoi(optarg);
            break;
        case 'u':
            flipmirror = 1;
            break;
        case 'i':
            irinvert = atoi(optarg);
	    if (irinvert > 4){
		irinvert=4;
	    }
	    if (irinvert < 1){
		irinvert=1;
	    }
            break;
        }
    }

    return 1;
}

int main(int argc, char *argv[]) {

	if (parse_args(argc, argv) < 0) return -1;

	signal(SIGINT, my_handler);
	if (access(SAVE_PATH, W_OK) != 0) {
		if (mkdir(SAVE_PATH, 0755)) {
			printf("mkdir: %s, %s\n", SAVE_PATH, strerror(errno));
			return -1;
		}
	}

	logi("capture_init...");
	int status = capture_init();

	start_server(3000, "/tmp", &snapshot_ref);

	if(status){
		logi("loop...");
		run_rtsp_stuff(); //start up RTSP before entering loop
		ak_sleep_ms(5000);
		capture_loop();
	}

	return 0;
}
