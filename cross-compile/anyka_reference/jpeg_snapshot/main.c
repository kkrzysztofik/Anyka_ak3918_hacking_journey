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

// App includes
//#include "convert.h"
#include "log.h"
//#include "app_settings.h"
#include "utils.h"
#include "http_server.h"
#include "snapshot.h"

volatile sig_atomic_t stop;					//stop if error occurs
#define cfg "/etc/jffs2"

// Camera vars
void *vi_handle = NULL;						//vi operating handle
struct video_channel_attr attr;				//vi channel attribute
struct video_resolution res;				//max sensor resolution

struct snapshot_t snapshot_ref = {
	.count = 5, //start off by taking some images even if there is no http request just to fill the two buffers
	.capture = 0, //capture first image to buffer 0
	.res_w = 1280, //default resolution
	.res_h = 720,
	.jpeg = {{0},{0}} //image buffers
};

void my_handler(int s){
   printf("Caught signal %d - Exiting... \n",s);
   stop=1;
   ak_sleep_ms(200);
   exit(1);
}

void stop_capture(){
	logi("stop_capture");
	// step 7: release resource
	ak_vi_capture_off(vi_handle);
	ak_vi_close(vi_handle);
	stop_server();
}

int capture_init(){
	logi("image module init");

	int ret = -1;								//return value
	int w_main = 640;
	int h_main = 480;
	int w_sub = 640;
	int h_sub = 480;

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
	ak_vi_set_flip_mirror(vi_handle, 1, 1); //flip + mirror = rotate 180 degrees

	return 1;
}

static void *venc_demo_open_encoder(int index)
{
	struct encode_param param = {0};

	switch (index) {
	case 0:
		param.width = snapshot_ref.res_w;
		param.height = snapshot_ref.res_h;
		param.minqp = 20;
		param.maxqp = 51;
		param.fps = 25;
		param.goplen = param.fps * 2;	   //current gop is stationary
		param.bps = 2000;				   //kbps
		param.profile = PROFILE_MAIN;	   //main profile
		param.use_chn = ENCODE_SUB_CHN;   //use main yuv channel data
		param.enc_grp = ENCODE_RECORD;     //assignment from enum encode_group_type
		param.br_mode = BR_MODE_CBR;	   //default is cbr
		param.enc_out_type = H264_ENC_TYPE;//h.264
		break;
	case 1:
		param.width = snapshot_ref.res_w;
		param.height = snapshot_ref.res_h;
		param.minqp = 20;
		param.maxqp = 51;
		param.fps = 10;
		param.goplen = param.fps * 2;
		param.bps = 500;	//kbps
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
/*
void bmp_capture(unsigned char *rawimg){

	//char bmp_data_path[50];
	//sprintf(bmp_data_path, "%s/preview.bmp", out_dir);
	//logi(" >>>>> Saving snapshot: %s", bmp_data_path);
	//YUVToBMP(<output_file>, <raw_YUV_img>, <type_of_conversion>, res_w, res_h);
	//YUVToBMP565(bmp_data_path, rawimg, YUV420ToRGB24, res_w, res_h);
	//unsigned char *rgb_24 = NULL;
	//rgb_24 = (unsigned char *)malloc(snapshot_ref.res_w*snapshot_ref.res_h*3);
	//YUV420ToRGB24(rgb_24,rawimg,snapshot_ref.res_w,snapshot_ref.res_h);
	//ak_print_normal("[%d] saving to RGB656 buffer: %d\n", snapshot_ref.count, snapshot_ref.capture);
	if (snapshot_ref.capture == 0){
		//RGB24ToRGB565(snapshot_ref.rgb_565_n0,rgb_24,snapshot_ref.res_w,snapshot_ref.res_h);
                YUV420ToRGB565(snapshot_ref.rgb_565_n0,rawimg,snapshot_ref.res_w,snapshot_ref.res_h);
		snapshot_ref.capture = 1;
	}else{
		//RGB24ToRGB565(snapshot_ref.rgb_565_n1,rgb_24,snapshot_ref.res_w,snapshot_ref.res_h);
                YUV420ToRGB565(snapshot_ref.rgb_565_n1,rawimg,snapshot_ref.res_w,snapshot_ref.res_h);
		snapshot_ref.capture = 0;
	}
	//free(rgb_24);
	//usleep(250); // wait to write
	snapshot_ref.count++;
	//snapshot_ref.capture = 0;
	//pthread_cond_signal(&snapshot_ref.ready);
}*/

void capture_loop(){

	struct video_input_frame frame;
	void *jpeg_encoder = venc_demo_open_encoder(ENCODE_PICTURE);

	logv("capture start");
	//snapshot_ref.rgb_565_n0 = (unsigned short *)malloc(snapshot_ref.res_w*snapshot_ref.res_h*2);
	//snapshot_ref.rgb_565_n1 = (unsigned short *)malloc(snapshot_ref.res_w*snapshot_ref.res_h*2);

	// To get frame by loop
	while (!stop) {

		__LOG_TIME_START();
		//get frame
		if (!ak_vi_get_frame(vi_handle, &frame)) {
			//check if image is needed
			if(snapshot_ref.count > 0){
				if ((snapshot_ref.res_w < 640) || (snapshot_ref.res_h < 480)){
					ak_venc_send_frame(jpeg_encoder, frame.vi_frame[VIDEO_CHN_SUB].data,
					frame.vi_frame[VIDEO_CHN_SUB].len, &snapshot_ref.jpeg[snapshot_ref.capture]);
				}else{
					ak_venc_send_frame(jpeg_encoder, frame.vi_frame[VIDEO_CHN_MAIN].data,
					frame.vi_frame[VIDEO_CHN_MAIN].len, &snapshot_ref.jpeg[snapshot_ref.capture]);
				}
				printf("JPEG [%d] picture size: %d\n", snapshot_ref.capture, snapshot_ref.jpeg[snapshot_ref.capture].len);
				snapshot_ref.count--;
				snapshot_ref.capture = !snapshot_ref.capture; //mark current image for reading and reserve the other for writing next image
				free(snapshot_ref.jpeg[snapshot_ref.capture].data);
			}
			// release frame data
			ak_vi_release_frame(vi_handle, &frame);
		} else {
			// not ready， sleep to release CPU
			ak_sleep_ms(10);
		}

		__LOG_TIME_END("capture");

		ak_sleep_ms(25); // Free CPU to do other things
						 // 25ms -> 60~70% CPU
	}
	ak_venc_close(jpeg_encoder);
	jpeg_encoder = NULL;
	free(snapshot_ref.jpeg[0].data);
	free(snapshot_ref.jpeg[1].data);
	stop_capture();
}
void help_message(char* argv[]){
        fprintf(stderr, "Usage: %s -w <width> -h <height>\n", argv[0]);
        fprintf(stderr, "Example: %s -w 1280 -h 720\n", argv[0]);
        fprintf(stderr, "\nNOTE: If no arguments are given, the default resolution is: %d x %d\n", snapshot_ref.res_w, snapshot_ref.res_h);
}

int parse_args(int argc, char* argv[]){

    for (;;) {
        int opt = getopt(argc, argv, ":d:w:h:s");
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
        /*case 'd':
            fprintf(stdout, "Using directory: %s\n", optarg);
            out_dir = optarg;
            break;
        case 's':
            //enable_sound = 1;
            break;*/
        case 'w':
            snapshot_ref.res_w = atoi(optarg);
            break;
        case 'h':
            snapshot_ref.res_h = atoi(optarg);
            break;
        }
    }

    return 1;
}

int main(int argc, char *argv[]) {

	if (parse_args(argc, argv) < 0) return -1;

	signal(SIGINT, my_handler);

	logi("capture_init...");
	int status = capture_init();

	start_server(3000, "/tmp", &snapshot_ref);

	if(status){
		logi("loop...");
		capture_loop();
	}

	return 0;
}
