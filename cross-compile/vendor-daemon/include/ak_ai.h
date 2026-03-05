#ifndef _AK_AI_H_
#define _AK_AI_H_

#include "ak_global.h"

#define AUDIO_DEFAULT_INTERVAL		100	//100ms one frame
#define AMR_FRAME_INTERVAL		    20	//AMR frame interval 20ms

/* NOTE: Functions below are verified against libplat_ai.so exports (libre_anyka_app SDK) */

enum ai_source {
	AI_SOURCE_AUTO,		//pcm driver decide adc source automatically,
						//and it is default configuration in pcm driver
	AI_SOURCE_MIC,		//set adc mic source manually
	AI_SOURCE_LINEIN	//set adc linein source manually
};

/* audio status: nr&agc resample aec nr_max*/
struct ai_runtime_status {
	int actual_rate;
	int nr_agc_enable;
	int nr_max_enable;
	int resample_enable;
	int aec_enable;
};


/** 
 * ak_ai_print_filter_info - print audio filter version & support functions
 * notes: filter such as: EQ, 3D, RESAMPLE, AGC and so on
 */
void ak_ai_print_filter_info(void);

/**
 * ak_ai_get_version - get audio in version
 * return: version string
 * notes:
 */
const char* ak_ai_get_version(void);

/**
 * ak_ai_open - open audio in device
 * @input[IN]: open audio input param
 * return: opened handle, NULL failed
 * notes: sample_bits set 16 bit
 */
void* ak_ai_open(const struct pcm_param *param);

/**
 * ak_ai_get_params - start ADC capture
 * @handle[IN]: audio in opened handle
 * return: 0 success, -1 failed
 * notes: call after set all kind of ADC attr
 */
int ak_ai_get_params(void *ai_handle, struct pcm_param *param);

/**
 * ak_ai_start_capture - start ADC capture
 * @handle[IN]: audio in opened handle
 * return: 0 success, -1 failed
 * notes: call after set all kind of ADC attr
 */
int ak_ai_start_capture(void *handle);

/**
 * ak_ai_stop_capture - stop ADC capture
 * @handle[IN]: audio in opened handle
 * return: 0 success, -1 failed
 * notes:
 */
int ak_ai_stop_capture(void *handle);

/**
 * ak_ai_get_frame - get audio frame
 * @handle[IN]: audio in opened handle
 * @frame[OUT]: audio frame info
 * @ms[IN]: <0 block mode; =0 non-block mode; >0 wait time
 * return: 0 success, -1 failed
 * notes:
 */
int ak_ai_get_frame(void *handle, struct frame *frame, long ms);

/**
 * ak_ai_release_frame - release one frame data
 * @handle[IN]: audio in opened handle
 * @frame[IN]: audio frame info
 * return: 0 success, -1 failed
 * notes: use "ak_ai_get_frame" get frame data,after used,release frame
 */
int ak_ai_release_frame(void *handle, struct frame *frame);

/**
 * ak_ai_set_capture_size - set audio capture size
 *		that read from AD driver each time
 * @handle[IN]: audio in opened handle
 * @capture_size[IN]: appointed frame interval, [512, 2048], unit: byte
 * return: 0 success, -1 failed
 * notes: 1. if you do not call, the default frame size is 2048
 *      2. set capture size before start capture.
 */
int ak_ai_set_capture_size(void *handle, unsigned int capture_size);

/**
 * ak_ai_set_frame_interval - set audio frame interval, unit: ms
 * @handle[IN]: audio in opened handle
 * @frame_interval[IN]: audio frame interval, [10, 125], unit: ms
 * return: 0 success, -1 failed
 * notes: You must set frame interval before call ak_ai_get_frame()
 */
int ak_ai_set_frame_interval(void *handle, int frame_interval);

/**
 * ak_ai_clear_frame_buffer - frame buffer clean
 * @handle[IN]: audio in opened handle
 * return: 0 success, -1 failed
 * notes: clean buffer after "ak_ai_open"
 */
int ak_ai_clear_frame_buffer(void *handle);

/**  
 * ak_ai_set_resample - set audio resampling
 * @handle[IN]: audio in opened handle
 * @enable[IN]: 0 disable, 1 enable
 * return: 0 success, -1 failed
 */
int ak_ai_set_resample(void *handle, int enable);

/**
 * ak_ai_set_nr_agc - adc nr&agc switch
 * @handle[IN]: opened audio input handle
 * @enable[IN]: 0 disable nr&agc, 1 enable nr&agc.
 * return: 0 success, -1 failed
 * notes: nr&agc only support 8k sample rate
 */
int ak_ai_set_nr_agc(void *handle, int enable);

/**
 * ak_ai_set_nr - adc nr switch
 * @handle[IN]: opened audio input handle
 * @enable[IN]: 0 disable nr, 1 enable nr.
 * return: 0 success, -1 failed
 * notes: This function is use to enable nr without agc,
		  if want to enable nr and agc,use ak_ai_set_nr_agc with enable;
		  if want to disable nr and agc,use ak_ai_set_nr_agc with disable.
 */
int ak_ai_set_nr(void *handle, int enable);

/**
 * ak_ai_set_nr_max - enable max nr level
 * @handle[IN]: audio in opened handle
 * @enable[IN]: 0 disable nr max, 1 enable nr max.
 * return: 0 success, -1 failed
 * notes: call after set ak_ai_set_nr_agc
 */
int ak_ai_set_nr_max(void *handle, int enable);

/**
 * ak_ai_set_aec - AEC switch
 * @handle[IN]: opened audio input handle
 * @enable[IN]: 0 disable AEC, 1 enable AEC
 * return: 0 success, -1 failed
 * notes: when AEC enable, must set ai and ao
 */
int ak_ai_set_aec(void *handle, int enable);

/**
 * ak_ai_set_source - set audio input source, linein or mic
 * @handle[IN]: opened audio input handle
 * @src[IN]: appointed source, default AI_SOURCE_AUTO
 * return: 0 success, -1 failed
 * notes:
 */
int ak_ai_set_source(void *handle, enum ai_source src);

/**
 * ak_ai_get_runtime_status - get ai run time status
 * @handle[IN]: audio in opened handle
 * @status[OUT]: get ai other status :nr&agc resample aec volume source
 * return: 0 success, -1 failed
 * notes: call after set all kind of ADC attr
 */
int ak_ai_get_runtime_status(void *handle, 
						struct ai_runtime_status *status);

/**
 * ak_ai_close - close audio input
 * @handle[IN]: opened audio input handle
 * return: 0 success, -1 failed
 * notes:
 */
int ak_ai_close(void *handle);

#endif
