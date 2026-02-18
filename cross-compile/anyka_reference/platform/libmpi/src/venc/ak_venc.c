#include <stdio.h>
#include <string.h>
#include <errno.h>
#include <stdarg.h>
#include <stdint.h>

#include "list.h"
#include "internal_error.h"
#include "akuio.h"

#include "ak_global.h"
#include "ak_common.h"
#include "ak_thread.h"
#include "ak_error.h"
#include "ak_vi.h"
#include "ak_venc.h"

/* H3 heads */
#include "ak_venc_cb.h"
#include "venc_ipcsrv.h"
#include "akv_interface.h"

#define MPI_VENC		        "<mpi_venc>"
#define VENC_STAT_RATE_TIME		(2)	//per 2 seconds

#define IP_PA_START             (0x20020000)	//IP start addr
#define IP_PA_LEN               (0x10000)		//64k
//#define VENC_CFG_PATH		    "/etc/jffs2/venc.cfg"
static char _cfg_path[64] = "/etc/jffs2/venc.cfg";
#define VENC_CFG_PATH		    _cfg_path

#define VENC_GET_STREAM_DEBUG	(0)
#define VENC_STAT_INFO_DEBUG    (0)

#define VENC_TYPE_MAX_USER		(sizeof(int) * 8)	//max user number in same group
#define ENC_OUT_MAX_SZ 			(512 * 1024)	//encode output buf max lenght
#define ONE_STREAM_MAX 			(50)			//max stream list node
#define CAPTURE_FAIL_LOG_INTERVAL	(50)
#define GET_STREAM_NO_DATA_LOG_INTERVAL	(100)
#define ENCODE_SEND_SLOW_MS		(120)

#define CALC_VIDEO_CAPTURE		(1)		//debug capture
#define CALC_VIDEO_ENCODE		(1)		//debug encode

#define VENC_CALC_CAP_TIME		(0)	//debug capture use time
#define VENC_CALC_ENC_TIME		(1) //debug encode use time

#define VENC_ORIGIN_TS			(0)	//use original timestamp
#define VENC_CALC_TS			(0)	//calculate timestamp
#define VENC_SMOOTH_TS			(1)	//smooth timestamp
#define SHOW_ORIGIN_TS			(0)	//show original timestamp

#define SMOOTH_TS_DEBUG			(0)	//smooth timestamp debug
#define USE_LIB_V1				(0)	//encode lib v1 macro

/* stat venc info */
struct venc_stat {
	int total;      //stream total bytes
	int bps;
	int frame_cnt;  //encode frame count
	float fps;
	int gop_factor;
    int gop_len;
	unsigned long long start_ts;
	struct ak_timeval calc_time;
};

/* use for capture */
struct frame_node {
	int bit_map;						//which group will use it
	struct video_input_frame *vframe;	//video frame pointer
	struct list_head list;				//frames list
};

/* internal stream node */
struct video_streams_t {
	int ref;					//use count
	int bit_map;				//user map
	struct video_stream stream;	//data
	struct list_head 	list; 	//stream list
};

/* two internal thread argument struct */
struct thread_arg {
	int cap_run;				//use for capture thread
	int enc_run;				//use for encode thread
	int sensor_pre_fps;			//record previous sensor capture frames per second
	void *vi_handle;			//record this thread group's vi handle
	char cap_name[16];			//capture thread name (<=15 chars)
	char enc_name[16];			//encode thread name (<=15 chars)
	struct list_head head_frame;//store capture frame, get by encode thread

	ak_pthread_t cap_tid;	 	//thread need join
	ak_pthread_t enc_tid;		//thread need join
	ak_sem_t cap_sem;			//capture semaphore
	ak_sem_t enc_sem;			//encode semaphore
	ak_mutex_t lock;			//list operate lock
	struct list_head list; 		//hang to video_ctrl
};

/* each encode group has one this structure to discribe its info */
struct encode_group {
	int user_count;			//how many user under this encode group
	int req_ref;			//request reference count
	int user_map;			//request user bit map
	int capture_frames;		//capture frame, syn with system's camera device
	int encode_frames;		//encode frames
	int ts_offset;			//enocde ts offset

	unsigned long long frame_index;	//use to control frames per second
	unsigned long long smooth_index;//index of smooth encode ts
	int is_stream_mode;				//1 stream mode, 0 sigle mode
	unsigned long long pre_ts;		//timestamp(ms)

	void *lib_handle;	//encoder lib operate handle
	void *output;		//encode output buf
	void *encbuf;		//encode buf address

	enum encode_group_type 	grp_type;	//group flag
	struct encode_param 	record; 	//record encode param
	struct list_head 		stream_list;	//store stream
	struct list_head 		enc_list;		//register encode handle list

	ak_mutex_t lock;				//mutex lock
	ak_mutex_t close_mutex;			//close mutex
	int streams_count;				//current queue has nodes number

	struct ak_timeval env_time;			//check work env time
	enum video_work_scene pre_scene;	//previous scene
	void *mdinfo;						//pointer to md info
	struct venc_stat stat;			    //venc stat info

	int stream_overflow_print_cnt;		//to control print when stream overflow
};

/* global control handle */
struct video_ctrl_handle {
	unsigned char module_init;	//module init flag
	ak_mutex_t lock;			//mutex lock

	int inited_enc_grp_num;		//record number of current opened group
	int thread_group_num;		//number of thread group
	struct encode_group *grp[ENCODE_GRP_NUM];	//use for stream encode ctrl

	ak_mutex_t cancel_mutex;	//cancel mutex
	struct list_head venc_list;	//video user list
	struct list_head thread_list;	//catpure and encode thread list

#if CALC_VIDEO_CAPTURE
	int frame_num;		//for debug, to calculate frame number
	struct ak_timeval calc_time;	//for debug
#endif

#if CALC_VIDEO_ENCODE
	int stream_num;		//for debug, record stream node number
	struct ak_timeval enc_time;	//for debug, record encode use time
#endif

#if 0
	ak_pthread_t 	wait_irq_thid;	//wait irq thread id
#endif
	int             wait_irq_runflg;	//thread run flag
	/* use for debug value */
	//int save_streams_flg[ENCODE_GRP_NUM]; //save streams to file, 1 is save, 0 is not
};

/* stream handle use for get and release stream */
struct stream_handle {
	void *vi_handle;	//record current stream's vi handle
	void *enc_handle;	//record current stream's encode handle
	int id;				//user id
	unsigned int no_data_cnt;	//consecutive no-data returns from get_stream
	unsigned int delivered_cnt;	//successful get_stream count
};

static const char venc_version[] = "libmpi_venc V1.0.11";

#ifdef AK_RTOS
static struct video_ctrl_handle video_ctrl = {0, 0, 0};
ak_mutex_t *g_venc_open_lock __attribute__((__section__(".lock_init"))) = &video_ctrl.lock;
#else
static struct video_ctrl_handle video_ctrl = {
	0, PTHREAD_MUTEX_INITIALIZER, 0
};

static pthread_mutex_t close_lock=PTHREAD_MUTEX_INITIALIZER;
static pthread_mutex_t cancel_lock=PTHREAD_MUTEX_INITIALIZER;
#endif

/* ------------ internal api start ---------------------*/
#if VENC_SMOOTH_TS
static inline int get_frame_interval(struct encode_group *handle)
{
	if (!handle || !handle->encode_frames)
		return 0;

	return (1000 / handle->encode_frames);
}

static inline int get_smooth_index(struct encode_group *handle,
					unsigned long long ts)
{
	if (!handle || !handle->encode_frames)
		return 0;

	return (((ts * handle->encode_frames) / 1000) - 1);
}

static inline unsigned long long get_cur_ts(unsigned long long smooth_index,
									int encode_frames)
{
	if (0 == encode_frames)
		return 0;

	return ((smooth_index * 1000) / encode_frames);
}
#endif

/*
 * alloc struct frame_node memory
 */
static void *alloc_video_frame(void)
{
	struct frame_node *frame = (struct frame_node *)calloc(1,
			sizeof(struct frame_node));
	if (!frame)
		return NULL;

	frame->vframe = (struct video_input_frame *)calloc(1,
			sizeof(struct video_input_frame));
	if (!frame->vframe) {
		free(frame);
		frame = NULL;
	}

	return frame;
}

/*
 * free struct frame_node memory
 */
static void free_frame(void *frame)
{
	struct frame_node *pframe = (struct frame_node *)frame;

	if (pframe) {
		if (pframe->vframe) {
			free(pframe->vframe);
			pframe->vframe = NULL;
		}

		free(pframe);
		pframe = NULL;
	}
}

static void set_encode_fps(struct encode_group *handle, int fps)
{
	T_AKVLIB_PAR encpar = {0};
	AKV_Encoder_Get_Parameters(handle->lib_handle, &encpar);

	encpar.EncFrameRate = fps;
	ak_print_normal_ex(MPI_VENC "set %d chnannel fps: %d\n",
	    handle->grp_type, fps);

	AKV_Encoder_Set_Parameters(handle->lib_handle, &encpar);
}

/********************************************************************************/
static T_AKV_DMA_BUFFER *venc_cb_dma_alloc(unsigned long size)
{
	T_AKV_DMA_BUFFER *f = (T_AKV_DMA_BUFFER *)calloc(1, sizeof(T_AKV_DMA_BUFFER));

	f->pData = akuio_alloc_pmem(size);
	if (!f->pData) {
		ak_print_error_ex("alloc dma for %lu failed\n", size);
		free(f);
		f = NULL;
		goto exit;
	}
	f->pa = akuio_vaddr2paddr(f->pData);
	f->size = size;
	f->hHandle = f;

exit:
	return f;
}

static int venc_cb_dma_free(T_AKV_DMA_BUFFER* buf)
{
	if (buf) {
		akuio_free_pmem(buf->pData);
		free(buf);
		return 0;
	}

	return -1;
}

static void venc_cb_release_frame_buf(T_AKV_DMA_BUFFER* buf)
{
	return;
}

/**
 * init video encoder, register callback
 * return: 1 seccess, 0 failed
 */
static int venc_encoder_init(void)
{
	T_AKVLIB_CB init_cb_fun = {0};

	init_cb_fun.ak_FunPrintf = print_normal;
	init_cb_fun.ak_FunMalloc = (AKV_CB_FUN_MALLOC)malloc;
	init_cb_fun.ak_FunFree   = (AKV_CB_FUN_FREE)free;

	//read and write register function
	init_cb_fun.ak_FunReadRegister = venc_cb_read_reg;
	init_cb_fun.ak_FunWriteRegister = venc_cb_write_reg;

	init_cb_fun.ak_FunDmaAlloc = (AKV_CB_FUN_DMA_ALLOC)venc_cb_dma_alloc;
	init_cb_fun.ak_FunDmaFree  = (AKV_CB_FUN_DMA_FREE)venc_cb_dma_free;
	init_cb_fun.ak_FunFlushDcache = (AKV_CB_FUN_FLUSH_DCACHE)venc_cb_flush_dcache;

	init_cb_fun.ak_FunReleaseFrameBuf =
	   	(AKV_CB_FUN_RELEASE_FRAME_BUF)venc_cb_release_frame_buf;
	init_cb_fun.ak_FunModuleClock = (AKV_CB_FUN_MODULE_CLOCK)venc_cb_module_clock;
	init_cb_fun.ak_FunModuleReset = akuio_reset_video;

	/* mutex */
	init_cb_fun.ak_FunMutexCreate = (AKV_CB_FUN_MUTEX_CREATE)venc_cb_init_mutex;
	init_cb_fun.ak_FunMutexDelete = (AKV_CB_FUN_MUTEX_DELETE)venc_cb_destroy_mutex;
	init_cb_fun.ak_FunMutexGet    = (AKV_CB_FUN_MUTEX_GET)venc_cb_lock_mutex;
	init_cb_fun.ak_FunMutexRelease = (AKV_CB_FUN_MUTEX_RELEASE)venc_cb_unlock_mutex;

	/* semaphore */
	init_cb_fun.ak_FunSemaphoreCreate = (AKV_CB_FUN_SEMAPHORE_CREATE)venc_cb_sem_init;
	init_cb_fun.ak_FunSemaphoreDelete = venc_cb_sem_destroy;
	init_cb_fun.ak_FunSemaphoreGet = (AKV_CB_FUN_SEMAPHORE_GET)venc_cb_sem_wait;
	init_cb_fun.ak_FunSemaphoreRelease =
		(AKV_CB_FUN_SEMAPHORE_RELEASE)venc_cb_sem_post;

#if USE_LIB_V1
	init_cb_fun.ak_FunEventCreate = (AKV_CB_FUN_EVENT_CREATE)venc_cb_create_event;
	init_cb_fun.ak_FunEventDelete = (AKV_CB_FUN_EVENT_DELETE)venc_cb_delete_event;
	init_cb_fun.ak_FunEventWait = (AKV_CB_FUN_EVENT_WAIT)venc_cb_wait_event;
	init_cb_fun.ak_FunEventSet = (AKV_CB_FUN_EVENT_SET)venc_cb_set_event;
#endif

#if !(USE_LIB_V1)
	init_cb_fun.ak_FunIrqWait = akuio_wait_irq;
	init_cb_fun.ak_FunAtomicIncrement =
	   	(AKV_CB_FUN_ATOMIC_INCREMENT)venc_cb_atomic_increment;
	init_cb_fun.ak_FunAtomicDecrement =
	   	(AKV_CB_FUN_ATOMIC_DECREMENT)venc_cb_atomic_decrement;
#endif

#if USE_LIB_V1
	init_cb_fun.ak_FunFileOpen = (AKV_CB_FUN_FILE_OPEN)fopen;
	init_cb_fun.ak_FunFileClose = (AKV_CB_FUN_FILE_CLOSE)fclose;					
	init_cb_fun.ak_FunFileRead = (AKV_CB_FUN_FILE_READ)fread;
	init_cb_fun.ak_FunFileWrite = (AKV_CB_FUN_FILE_WRITE)fwrite; 
#endif


	/* init reg */
	venc_cb_init_reg(IP_PA_START, IP_PA_LEN);
	if (!venc_cb_reg_ready()) {
		ak_print_error_ex(MPI_VENC
			"venc_encoder_init failed: register map unavailable (pa_start=0x%08x, len=0x%08x)\n",
			(unsigned int)IP_PA_START, (unsigned int)IP_PA_LEN);
		set_error_no(ERROR_TYPE_DEV_OPEN_FAILED);
		return AK_FAILED;
	}
	{
		unsigned long reg_len = venc_cb_reg_len();
		if (reg_len < IP_PA_LEN) {
			ak_print_error_ex(MPI_VENC
				"venc_encoder_init fatal: register map is truncated "
				"(requested=0x%08x mapped=0x%08lx); refusing encoder init\n",
				(unsigned int)IP_PA_LEN, reg_len);
			set_error_no(ERROR_TYPE_DEV_OPEN_FAILED);
			return AK_FAILED;
		}
	}

	return AKV_Encoder_Init(&init_cb_fun);
}

/*
 * venc_handle_init - init video encode user handle
 * index[IN]: group index
 * return: 0 on success, -1 failed
 */
static int venc_handle_init(int index)
{
	/* already opened, just return success */
	if (video_ctrl.grp[index]) {
		ak_print_normal_ex(MPI_VENC "group %d's handle has already opened\n",
		    index);
		return AK_SUCCESS;
	}

	/* allocate handle's memory */
	video_ctrl.grp[index] = (struct encode_group *)calloc(1,
			sizeof(struct encode_group));
	if (!video_ctrl.grp[index]) {
		set_error_no(ERROR_TYPE_MALLOC_FAILED);
		ak_thread_mutex_unlock(&video_ctrl.lock);
		return AK_FAILED;
	}

	/* handle's control resource init */
	ak_thread_mutex_init(&video_ctrl.grp[index]->lock, NULL);
	ak_thread_mutex_init(&video_ctrl.grp[index]->close_mutex, NULL);
	INIT_LIST_HEAD(&video_ctrl.grp[index]->enc_list);
	ak_get_ostime(&(video_ctrl.grp[index]->env_time));
	/* indoor or outdoor mode */
	video_ctrl.grp[index]->pre_scene = VIDEO_SCENE_UNKNOWN;

	return AK_SUCCESS;
}

/*
 * add_user - add user
 * enc_handle[IN], encode handle
 * return current handle's user counter number
 */
static int add_user(void *enc_handle)
{
	struct encode_group *handle = (struct encode_group *)enc_handle;

	handle->user_count++;
	return handle->user_count;
}

/*
 * del_user - delete user
 * enc_handle[IN], encode handle
 * return current handle's user counter number
 */
static int del_user(void *enc_handle)
{
	struct encode_group *handle = (struct encode_group *)enc_handle;

	ak_thread_mutex_lock(&handle->lock);
	if (handle->user_count > 0)
		handle->user_count--;
	ak_thread_mutex_unlock(&handle->lock);

	return handle->user_count;
}

/*
 * release_group_streams - release group's buffered stream
 * enc_handle[IN], encode handle
 * return: 0 on suceess, -1 failed
 * notes: "req_ref" is check by call function
 */
static int release_group_streams(void *enc_handle)
{
	if (!enc_handle) {
		return AK_FAILED;
	}

	struct encode_group *handle = (struct encode_group *)enc_handle;
	struct video_streams_t *pos = NULL;
	struct video_streams_t *n = NULL;

	ak_print_info_ex(MPI_VENC "entry..., before: %d\n", handle->streams_count);
	ak_thread_mutex_lock(&handle->lock);
	list_for_each_entry_safe(pos, n, &handle->stream_list, list) {
		if (pos->stream.data) {
			free(pos->stream.data);
			pos->stream.data = NULL;
		}
		list_del_init(&pos->list);	//remove from stream list
		free(pos);					//free node's memory
		pos = NULL;
		handle->streams_count--;	//decrese total stream number
	}
	/* when release all stream, reinit stream head */
	INIT_LIST_HEAD(&handle->stream_list);
	ak_thread_mutex_unlock(&handle->lock);
	ak_print_info_ex(MPI_VENC "entry..., after: %d\n", handle->streams_count);

	return AK_SUCCESS;
}

/*
 * has_encode_running - check whether its still has encode running
 * return: 1 on has user still running, 0 means all user exited
 */
static int has_encode_running(void)
{
	int i;

	for (i = 0; i < ENCODE_GRP_NUM; i++) {
		if (video_ctrl.grp[i] && (video_ctrl.grp[i]->user_count > 0))
			return 1;
	}

	return 0;
}

/*
 * encode_group_exit - release group's resource, like encode handle
 * enc_handle[IN], encode handle
 */
static void encode_group_exit(void *enc_handle)
{
	struct encode_group *handle = (struct encode_group *)enc_handle;
	ak_print_info_ex(MPI_VENC "enc_handle:%p, type:%d\n",
			handle, handle->record.enc_grp);

	ak_thread_mutex_lock(&handle->lock);
	/* close encode handle */
	if (handle->lib_handle) {
		AKV_Encoder_Close(handle->lib_handle);
		handle->lib_handle = NULL;
		ak_print_normal_ex(MPI_VENC "Video Stream Encoder Close OK\n");
	}

	if (handle->encbuf)
		free(handle->encbuf);

	ak_thread_mutex_unlock(&handle->lock);
	ak_thread_mutex_destroy(&handle->lock);

	ak_thread_mutex_lock(&video_ctrl.lock);
	/* no user anymore, destroy encode */
	if (!has_encode_running()) {
#if 0
		/* cancel wait_irq_thread */
		video_ctrl.wait_irq_runflg = AK_FALSE;
		if (video_ctrl.wait_irq_thid) {
			ak_thread_cancel(video_ctrl.wait_irq_thid);
			ak_thread_join(video_ctrl.wait_irq_thid);
			video_ctrl.wait_irq_thid = 0;
		}
#endif
		AKV_Encoder_Destory();
		ak_thread_mutex_destroy(&video_ctrl.cancel_mutex);
		video_ctrl.module_init = AK_FALSE;

		venc_sys_ipc_unregister();
	}
	ak_thread_mutex_unlock(&video_ctrl.lock);
}

/*
 * venc_release_useless_frames - release all useless yuv frames
 * thread_arg[IN]: thread group's argument
 */
static void venc_release_useless_frames(struct thread_arg *thread_arg)
{
	/* one frame corresponding multiple encode group */
	struct frame_node *pos = NULL;
	struct frame_node *n = NULL;		//frame loop value

	/* traverse frame queue to release resourse */
	ak_thread_mutex_lock(&thread_arg->lock);

	list_for_each_entry_safe(pos, n, &thread_arg->head_frame, list) {
		list_del(&pos->list);
		ak_vi_release_frame(thread_arg->vi_handle, pos->vframe);
		free_frame(pos);
	}

	ak_thread_mutex_unlock(&thread_arg->lock);
	ak_print_normal_ex(MPI_VENC "all useless frames were released\n");
}

/*
 * encode_thread_group_exit - release group thread's resources
 */
static void encode_thread_group_exit(void)
{
	ak_print_info_ex(MPI_VENC "enter, inited_enc_grp_num=%d\n",
		video_ctrl.inited_enc_grp_num);
	/*
	 * All encode has close, so we close all thread group.
	 * Now it is cancel only one thread group, close specific
	 * thread group is not implement.
	 */
	if (!video_ctrl.inited_enc_grp_num) {
		struct thread_arg *arg, *n;

		list_for_each_entry_safe(arg, n, &video_ctrl.thread_list, list) {
			/* exit capture */
			ak_print_notice_ex(MPI_VENC "encode_thread_group_exit: stopping capture tid=%lu\n",
				arg->cap_tid);
			arg->cap_run = 0;
			/*
			 * Clear VI buffers first so any capture side blocked on stale queued
			 * frames gets unblocked before we wait for thread teardown.
			 */
			ak_print_notice_ex(MPI_VENC "encode_thread_group_exit: ak_vi_clear_buffer vi_handle=%p\n",
				arg->vi_handle);
			ak_vi_clear_buffer(arg->vi_handle);
			ak_print_notice_ex(MPI_VENC "encode_thread_group_exit: ak_vi_clear_buffer done\n");
			ak_thread_sem_post(&arg->cap_sem);
			/*
			 * Cooperative exit is preferred, but request pthread cancel as a hard
			 * fallback so shutdown does not hang in capture loops.
			 */
			ak_print_notice_ex(MPI_VENC "encode_thread_group_exit: requesting capture thread cancel tid=%lu\n",
				arg->cap_tid);
			int cap_cancel_ret = ak_thread_cancel(arg->cap_tid);
			if (cap_cancel_ret) {
				ak_print_warning_ex(MPI_VENC
					"capture thread cancel request returned %d, tid=%lu\n",
					cap_cancel_ret, arg->cap_tid);
			}
			ak_print_normal_ex(MPI_VENC "join capture thread, tid=%lu\n",
			    arg->cap_tid);
			int cap_join_ret = ak_thread_join(arg->cap_tid);
			ak_print_notice_ex(MPI_VENC "encode_thread_group_exit: capture join ret=%d tid=%lu\n",
				cap_join_ret, arg->cap_tid);
			if (cap_join_ret) {
				ak_print_error_ex(MPI_VENC
					"join capture thread failed: ret=%d, tid=%lu\n",
					cap_join_ret, arg->cap_tid);
			} else {
				ak_print_notice_ex(MPI_VENC "join capture thread OK\n");
			}

			/* exit encode */
			ak_print_notice_ex(MPI_VENC "encode_thread_group_exit: stopping encode tid=%lu\n",
				arg->enc_tid);
			arg->enc_run = 0;
			ak_thread_sem_post(&arg->enc_sem);
			ak_print_notice_ex(MPI_VENC "encode_thread_group_exit: requesting encode thread cancel tid=%lu\n",
				arg->enc_tid);
			int enc_cancel_ret = ak_thread_cancel(arg->enc_tid);
			if (enc_cancel_ret) {
				ak_print_warning_ex(MPI_VENC
					"encode thread cancel request returned %d, tid=%lu\n",
					enc_cancel_ret, arg->enc_tid);
			}
			ak_print_normal_ex(MPI_VENC "join encode thread, tid=%lu\n",
			    arg->enc_tid);
			int enc_join_ret = ak_thread_join(arg->enc_tid);
			ak_print_notice_ex(MPI_VENC "encode_thread_group_exit: encode join ret=%d tid=%lu\n",
				enc_join_ret, arg->enc_tid);
			if (enc_join_ret) {
				ak_print_error_ex(MPI_VENC
					"join encode thread failed: ret=%d, tid=%lu\n",
					enc_join_ret, arg->enc_tid);
			} else {
				ak_print_notice_ex(MPI_VENC "join encode thread OK\n");
			}

			/// Release the semaphores created during initialization.
			ak_thread_sem_destroy(&arg->cap_sem);
			ak_thread_sem_destroy(&arg->enc_sem);

			/*
			 * release all frame(s) which haven't been encoded but has been
			 * capture to frame queue
			 */
			venc_release_useless_frames(arg);
			ak_thread_mutex_destroy(&arg->lock);

			/* delete current thread from this module */
			list_del(&arg->list);
			free(arg);
			arg = NULL;

			ak_print_info_ex(MPI_VENC "free thread arg OK\n");
			ak_thread_mutex_lock(&video_ctrl.lock);
			video_ctrl.thread_group_num--;
			ak_thread_mutex_unlock(&video_ctrl.lock);
		}
	}
	ak_print_info_ex(MPI_VENC "leave ...\n");
}

/**
 * change_frame_index - change frame index when fps change
 * group_type[IN]: encode group type number
 * ts[IN]: capture frame ts
 * return: none
 */
static void init_frames_ctrl(struct encode_group *grp)
{
	grp->frame_index = 0;
}

/**
 * frames_ctrl - decide whether current group need encode
 * group_type[IN]: encode group type number
 * max_frame[IN]: encode max frame
 * ts[IN]: capture frame ts
 * return:  0 unneed it, 1 need
 */
static int frames_ctrl(int group_type, const int max_frame,
					unsigned long long ts)
{
	int ret = 0;
	struct encode_group *grp = video_ctrl.grp[group_type];
	unsigned int enc_frame = grp->encode_frames;

	/* get camera capture frames and video encode frames */
	if ((max_frame <= enc_frame)
		&& ((max_frame == enc_frame) && (max_frame != 12))) {
		return 1;
	}

	if (enc_frame <= 0) {
		return 0;
	}

	unsigned long long enc_ts = ((grp->frame_index * 1000) / enc_frame);
	if (ts >= enc_ts) {
		if ((enc_ts > 0) && (ts - enc_ts) > 200) {
			ak_print_warning(MPI_VENC "group_type=%d, index=%llu, "
			    "ts=%llu, enc_ts=%llu\n",
				group_type, grp->frame_index, ts, enc_ts);
		}

		ret = 1;
		++grp->frame_index;

		enc_ts = ((grp->frame_index * 1000) / enc_frame);
		if (enc_ts < ts) {
			grp->frame_index = (((ts + (1000/enc_frame)) * enc_frame) / 1000);
		}
	}

	return ret;
}

/**
 * check_sensor_fps_switch - according to current encode fps and capture fps
 *                           to determine endode fps
 * thread_arg[IN]: current thread group's argument, use it's vi handle
 * sensor_fps[IN]: current sensor capture fps
 */
static void check_sensor_fps_switch(struct thread_arg *thread_arg,
				const int sensor_fps)
{
	int changed = 0;

	if (sensor_fps != thread_arg->sensor_pre_fps) {
		if (thread_arg->sensor_pre_fps) {
			ak_print_warning_ex(MPI_VENC "\t*** sensor fps switch %d to %d ***\n\n",
				thread_arg->sensor_pre_fps, sensor_fps);
			changed = 1;
		}

		thread_arg->sensor_pre_fps = sensor_fps;

		int i = 0;
		struct encode_group *grp = NULL;

		/* compare sensor's real fps and appointed record fps */
		for (i = 0; i < ENCODE_GRP_NUM; i++) {
			grp = video_ctrl.grp[i];
			if (!grp)
				continue;
			if (sensor_fps < grp->record.fps) { /* use sensor fps */
				grp->encode_frames = sensor_fps;
			} else { 							/* use original encode fps */
				grp->encode_frames = grp->record.fps;
			}

			/*
			 * if sensor frame rate change cause encode frame rate change too,
			 * we should notify encoder to adjust encode
			 */
			if (changed && (grp->encode_frames > 0)) {
				set_encode_fps(grp, grp->encode_frames);
				init_frames_ctrl(grp);
			}
		}
	}
}

/**
 * add_to_encode_list - add frame to current thread group
 * thread_arg[IN]: current thread group's argument
 * sensor_fps[IN]: pointer to frame which will be add
 */
static void add_to_encode_list(struct thread_arg *thread_arg,
		struct frame_node *frame)
{
	if (!thread_arg)
		return ;

	int i = 0;
	int bit_map = 0;
	const int sensor_fps = ak_vi_get_fps(thread_arg->vi_handle);
	unsigned long long ts = frame->vframe->vi_frame[VIDEO_CHN_MAIN].ts;

	check_sensor_fps_switch(thread_arg, sensor_fps);

	/* uses comfirm */
	for (i = 0; i < ENCODE_GRP_NUM; i++) {
		if (video_ctrl.grp[i]) {
			/* decide whether current frame should be encode */
			if (frames_ctrl(i, sensor_fps, ts))
				set_bit(&bit_map, i);
		}
	}

	if (bit_map) {
		frame->bit_map = bit_map;

		ak_thread_mutex_lock(&thread_arg->lock);
		list_add_tail(&frame->list, &thread_arg->head_frame);
		ak_thread_mutex_unlock(&thread_arg->lock);

		/* notify encode thread to work */
		ak_thread_sem_post(&thread_arg->enc_sem);
	} else {
		/* no one need, release it at once */
		ak_vi_release_frame(thread_arg->vi_handle, frame->vframe);
		free_frame(frame);
	}
}

static void get_venc_start_osd_stat(struct encode_group *venc_handle,
		        const struct video_stream *stream, unsigned long long origin_ts)
{
    venc_handle->stat.start_ts = origin_ts;
    venc_handle->stat.total = stream->len;
    venc_handle->stat.frame_cnt = 1;

    venc_handle->stat.gop_len = ak_venc_get_gop_len(venc_handle);
    int encode_fps = ak_venc_get_fps(venc_handle);
    if ((venc_handle->stat.gop_len > 0) && (encode_fps > 0)) {
		venc_handle->stat.gop_factor = (venc_handle->stat.gop_len / encode_fps);
		if (!venc_handle->stat.gop_factor)
			venc_handle->stat.gop_factor = VENC_STAT_RATE_TIME;
	}

    ak_get_ostime(&(venc_handle->stat.calc_time));
#if VENC_STAT_INFO_DEBUG
    ak_print_notice(MPI_VENC "grp_type=%d, start_ts=%llu, total=%d\n",
        venc_handle->grp_type, venc_handle->stat.start_ts, venc_handle->stat.total);
    ak_print_info(MPI_VENC "grp_type=%d, encode_fps=%d, gop_len=%d, gop_factor=%d\n",
        venc_handle->grp_type, encode_fps,
        venc_handle->stat.gop_len, venc_handle->stat.gop_factor);
    ak_print_info(MPI_VENC "grp_type=%d, stream ts=%llu, len=%d, seq_no=%lu, type=%d\n",
        venc_handle->grp_type,
        stream->ts, stream->len, stream->seq_no, stream->frame_type);
#endif
}

static void calc_venc_osd_stat(struct encode_group *venc_handle,
		        const struct video_stream *stream, unsigned long long origin_ts)
{
    struct ak_timeval cur_time;

    /* real passed time */
    ak_get_ostime(&cur_time);
    long use_time = ak_diff_ms_time(&cur_time, &(venc_handle->stat.calc_time));

    /* video timestamp time */
    unsigned long long diff_ts = (origin_ts - venc_handle->stat.start_ts);

    float factor = venc_handle->stat.gop_factor;
    int stat_time = (factor * 1000);

    if ((use_time >= stat_time)
        || ((diff_ts >= stat_time) && (FRAME_TYPE_I == stream->frame_type))) {
        int interval = 40;
        if (venc_handle->capture_frames > 0) {
            interval = (1000 / venc_handle->capture_frames);
        }
        if ((diff_ts > stat_time) && ((diff_ts - stat_time) >= interval)) {
            venc_handle->stat.frame_cnt -= ((diff_ts - stat_time) / interval);
        }

        venc_handle->stat.bps = (((venc_handle->stat.total / 1024) * 8) / factor);
		venc_handle->stat.fps = (venc_handle->stat.frame_cnt / factor);

#if VENC_STAT_INFO_DEBUG
        ak_print_info(MPI_VENC "grp_type=%d, start_ts=%llu, origin_ts=%llu\n",
            venc_handle->grp_type, venc_handle->stat.start_ts, origin_ts);
		ak_print_info(MPI_VENC "grp_type=%d, use_time=%ld, diff_ts=%llu, stat_time=%d\n",
            venc_handle->grp_type, use_time, diff_ts, stat_time);
        ak_print_info(MPI_VENC "grp_type=%d, factor=%.2f, total=%d, frame_cnt=%d\n",
            venc_handle->grp_type, factor,
            venc_handle->stat.total, venc_handle->stat.frame_cnt);
#endif

		ak_print_info(MPI_VENC "grp_type=%d, rate=%d(kbps), gop=%d, fps=%.1f, fcnt=%d\n",
			venc_handle->grp_type, venc_handle->stat.bps,
			venc_handle->stat.gop_len, venc_handle->stat.fps,
			venc_handle->stat.frame_cnt);

		/* incase of gop changed, so we get gop again */
		get_venc_start_osd_stat(venc_handle, stream, origin_ts);
    } else {
        venc_handle->stat.total += stream->len;
	    ++venc_handle->stat.frame_cnt;
    }
}

/**
 * stat_venc_info - video stat info
 * venc_handle[IN]: venc handle
 * stream[IN]: current stream info
 * return: none
 */
static void stat_venc_info(struct encode_group *venc_handle,
                const struct video_stream *stream, unsigned long long origin_ts)
{
    if (0 == venc_handle->stat.start_ts) {
        if (FRAME_TYPE_I == stream->frame_type) {
            get_venc_start_osd_stat(venc_handle, stream, origin_ts);
        }
    } else {
        calc_venc_osd_stat(venc_handle, stream, origin_ts);
    }
}

#if CALC_VIDEO_CAPTURE
static void calc_video_capture_frame(struct video_input_frame *vframe)
{
	struct ak_timeval cur_time;

	ak_get_ostime(&cur_time);
	++(video_ctrl.frame_num);
	long diff_time = ak_diff_ms_time(&cur_time, &(video_ctrl.calc_time));

	/* calc frame number per ten seconds */
	if(diff_time >= 10*1000){
		int total = video_ctrl.frame_num;
		int seconds =  (diff_time / 1000);
		if (seconds > 0) {
			struct frame *main_frame = &(vframe->vi_frame[VIDEO_CHN_MAIN]);
			ak_print_info(MPI_VENC "new capture ts=%llu, len=%u, seq_no=%lu\n",
				main_frame->ts, main_frame->len, main_frame->seq_no);
			ak_print_info(MPI_VENC "*** second=%d, total=%d, "
				"average=%d ***\n\n", seconds, total, (total / seconds));
		}

		ak_get_ostime(&(video_ctrl.calc_time));
		video_ctrl.frame_num = 0;
	}
}
#endif

#if CALC_VIDEO_ENCODE
static void calc_venc_stream(void)
{
	struct ak_timeval cur_time;

	ak_get_ostime(&cur_time);
	++(video_ctrl.stream_num);
	long diff_time = ak_diff_ms_time(&cur_time, &(video_ctrl.enc_time));

	/* calc frame number per ten seconds */
	if(diff_time >= 10*1000){
		int total = video_ctrl.stream_num;
		int seconds =  (diff_time / 1000);

		if (seconds > 0) {
			ak_print_info(MPI_VENC "*** encode second=%d, total=%d, "
				"average=%d ***\n\n", seconds, total, (total / seconds));
		}

		ak_get_ostime(&(video_ctrl.enc_time));
		video_ctrl.stream_num = 0;
	}
}
#endif

#if VENC_CALC_TS
static unsigned long long calc_stream_ts(struct encode_group *enc_handle,
		unsigned long long origin_ts)
{
	/* origin ts should be capture ts */
	unsigned long long calc_ts = 0;
	int ts_interval = (1000 / enc_handle->encode_frames);

	if (enc_handle->pre_ts > 0) {
		calc_ts = (enc_handle->pre_ts + ts_interval);
		if (abs(calc_ts - origin_ts) > 20) {
			calc_ts = origin_ts;
		}
	} else {
		calc_ts = origin_ts;
		ak_print_notice_ex(MPI_VENC "we get first stream for group %d\n",
				enc_handle->grp_type);
		ak_print_normal_ex(MPI_VENC "encode_frames=%d, ts_interval=%d\n",
				enc_handle->encode_frames, ts_interval);
		ak_print_normal_ex(MPI_VENC "first stream ts=%llu\n", origin_ts);
	}
	enc_handle->pre_ts = calc_ts;

	ak_print_info_ex(MPI_VENC "calc_ts=%llu, origin ts=%llu, diff=%lld\n",
		calc_ts, origin_ts, (calc_ts - origin_ts));

	return calc_ts;
}
#endif

#if VENC_SMOOTH_TS
static unsigned long long smooth_ts(struct encode_group *handle,
		unsigned long long ts, unsigned long seq_no)
{
	if (!handle || !handle->encode_frames)
		return 0;

	unsigned long long cur_ts = 0;
	int actual_fitv = get_frame_interval(handle);

	if (0 == handle->smooth_index) {
		handle->smooth_index = get_smooth_index(handle, ts);
		handle->ts_offset = 0;
		ak_print_normal_ex(MPI_VENC "init smooth_index=%llu\n",
		    handle->smooth_index);
	}
	handle->smooth_index++;
	cur_ts = get_cur_ts(handle->smooth_index, handle->encode_frames);

	if ((cur_ts > (ts + (2 * actual_fitv)))
		|| ((cur_ts + (2 * actual_fitv)) < ts)) {
#if SMOOTH_TS_DEBUG
		ak_print_warning_ex(MPI_VENC "pre_ts=%llu, cur_ts=%llu, ts=%llu, "
		    "seq_no=%lu\n",
			handle->pre_ts, cur_ts, ts, seq_no);
		ak_print_normal_ex(MPI_VENC "acfitv=%d, smooth_index=%llu\n",
			actual_fitv, handle->smooth_index);
#endif

		handle->smooth_index = get_smooth_index(handle, ts);
		handle->smooth_index++;
		cur_ts = get_cur_ts(handle->smooth_index, handle->encode_frames);
		handle->ts_offset = 0;

		if ((cur_ts <= handle->pre_ts) && (cur_ts < ts)) {
			handle->ts_offset = -(ts - cur_ts) / 2;
			ak_print_normal_ex(MPI_VENC "cur_ts too small, cur_ts:%llu, "
			    "pre_ts:%llu, ts_offset:%d\n",
				cur_ts, handle->pre_ts, handle->ts_offset);
		}

#if SMOOTH_TS_DEBUG
		ak_print_notice_ex(MPI_VENC "re-sync: ts=%llu, cur_ts=%llu, "
		    "smooth_index=%llu\n",
			ts, cur_ts, handle->smooth_index);
#endif
	}

	cur_ts -= handle->ts_offset;
	if (cur_ts > ts) {
#if SMOOTH_TS_DEBUG
		ak_print_notice_ex(MPI_VENC "<--smooth--> ts: %llu, cur_ts: %llu, "
		    "offset: %d\n",
			ts, cur_ts, handle->ts_offset);
#endif

		int tmp_offset = handle->ts_offset;
		int tmp = (cur_ts - ts);
		handle->ts_offset += tmp;
		cur_ts -= tmp;

#if SMOOTH_TS_DEBUG
		ak_print_normal_ex(MPI_VENC "cur_ts=%llu, ts=%llu, ts_offset=%d, "
		    "smooth_index=%llu\n",
			cur_ts, ts, handle->ts_offset, handle->smooth_index);
#endif

		if (handle->smooth_index > 1) {
			unsigned long long tmp_enc_ts = get_cur_ts((handle->smooth_index - 1),
				handle->encode_frames);
			int diff = cur_ts + tmp_offset - tmp_enc_ts;

#if SMOOTH_TS_DEBUG
			ak_print_normal_ex(MPI_VENC "diff=%u, ts_offset=%d, encode_frames=%d\n",
				diff, handle->ts_offset, handle->encode_frames);
#endif

			/* adjust enc index */
			if (handle->ts_offset >= diff) {
				handle->smooth_index--;
				handle->ts_offset -= diff;
			}
		}

#if SMOOTH_TS_DEBUG
		ak_print_normal_ex(MPI_VENC "cur_ts=%llu, ts=%llu, ts_offset=%d, "
		    "smooth_index=%llu\n",
			cur_ts, ts, handle->ts_offset, handle->smooth_index);
#endif
	}

	return cur_ts;
}
#endif


/// Cache the currently used VI handle for use by @ref ak_venc_fetch_yuv.
//static void *_vi_handle = NULL;
//static int _vi_width = 0, _vi_height = 0; ///< Cache the width/height of the current VI sub-stream, used to validate calls to @ref ak_venc_fetch_yuv.

/// The width and height requested by the caller; cached by @ref ak_venc_fetch_yuv after validating,
/// then used by the @ref capture_encode_frame thread for processing.
static int _fetch_orign_width = 0, _fetch_orign_height = 0;

/// The video channel requested by the user for capture.
static int _fetch_ch = VIDEO_CHN_SUB;

/// Flag indicating a pending YUV capture request from the user.
static int _fetch_request = 0;

/// Capture scale factor.
static int _fetch_scale = 1;

/// Mutex lock for serializing calls to @ref ak_venc_fetch_yuv.
static pthread_mutex_t _Mutex_fetch = PTHREAD_MUTEX_INITIALIZER;

/// Semaphore signaled when a YUV capture requested by @ref ak_venc_fetch_yuv is complete.
static sem_t _Sem_fetch;

/// YUV frame data structure object.
struct venc_yuv_frame *_fetch_Frame = NULL;

#define _ERR(__fmt...) \
	do {\
		ak_print (LOG_LEVEL_ERROR, "%s() ", __func__);\
		ak_print (LOG_LEVEL_ERROR, ##__fmt);\
		ak_print (LOG_LEVEL_ERROR, "\r\n");\
	} while (0)


int ak_venc_get_yuv (int width, int height, struct venc_yuv_frame *yuv_frame) {

	struct video_channel_attr VideoChannelAttr;
	unsigned char *yuv = NULL;
	int yuvSize = 0;

	if (0 != ak_vi_get_channel_attr(ak_vi_get_handle (VIDEO_DEV0), &VideoChannelAttr)) {
		_ERR ("Get VI Channel Attributes Failed.");
		return -1;
	}

	/// Default image width uses the sub-stream.
	if (0 == width) {
		width = VideoChannelAttr.res[VIDEO_CHN_SUB].width;
	}

	/// Default image height uses the sub-stream.
	if (0 == height) {
		height = VideoChannelAttr.res[VIDEO_CHN_SUB].height;
	}

//	_ERR ("%dx%d %dx%d",
//			VideoChannelAttr.res[VIDEO_CHN_MAIN].width,
//			VideoChannelAttr.res[VIDEO_CHN_MAIN].height,
//			VideoChannelAttr.res[VIDEO_CHN_SUB].width,
//			VideoChannelAttr.res[VIDEO_CHN_SUB].height);

	/// Acquire the mutex lock to prevent concurrent access from two threads.
	if (0 != pthread_mutex_trylock (&_Mutex_fetch)) {
		_ERR ("Get YUV Busy or Stream NOT Ready.");
		return -1;
	}

	if (0 == video_ctrl.thread_group_num) {
		/// If the VENC thread has not been started, YUV data can be retrieved directly from VI.
		struct video_input_frame vi_frame;

		/// Clear the request flag.
		_fetch_request = 0; ///< No need to request via the streaming mode.

		if (width == VideoChannelAttr.res[VIDEO_CHN_SUB].width
				&& height == VideoChannelAttr.res[VIDEO_CHN_SUB].height) {
			_fetch_ch = VIDEO_CHN_SUB;
		} else if (width == VideoChannelAttr.res[VIDEO_CHN_MAIN].width
				&& height == VideoChannelAttr.res[VIDEO_CHN_MAIN].height) {
			_fetch_ch = VIDEO_CHN_MAIN;
		} else {
			_ERR ("Resulution %dx%d or %dx%d Support.",
					VideoChannelAttr.res[VIDEO_CHN_SUB].width,
					VideoChannelAttr.res[VIDEO_CHN_SUB].height,
					VideoChannelAttr.res[VIDEO_CHN_MAIN].width,
					VideoChannelAttr.res[VIDEO_CHN_MAIN].height);

			goto failed;
		}

		/// Allocate physical memory to store the YUV data.
		yuvSize = width * height * 3 / 2;
		yuv = (unsigned char *)(akuio_alloc_pmem (yuvSize));
		if (NULL == yuv) {
			_ERR ("Out of Memory for YUV(Size=%d).", yuvSize);
			goto failed;
		}

		if (0 == ak_vi_get_frame (ak_vi_get_handle (VIDEO_DEV0), &vi_frame)) {

			/// Copy data to the PMEM buffer.
			memcpy (yuv, vi_frame.vi_frame[_fetch_ch].data, yuvSize);

			/// Cache the captured data.
			_fetch_Frame = yuv_frame;
			_fetch_Frame->width = width;
			_fetch_Frame->height = height;
			_fetch_Frame->ts = vi_frame.vi_frame[_fetch_ch].ts;
			_fetch_Frame->seqno = vi_frame.vi_frame[_fetch_ch].seq_no;
			_fetch_Frame->pixels = yuv;
			_fetch_Frame->size = yuvSize; ///< 1.5 x Width x Height.
			_fetch_Frame->y = _fetch_Frame->pixels;
			_fetch_Frame->yStride = _fetch_Frame->width;
			_fetch_Frame->ySize = _fetch_Frame->width * _fetch_Frame->height;
			_fetch_Frame->uv = _fetch_Frame->y + _fetch_Frame->ySize; ///< UV data immediately follows the Y plane.
			_fetch_Frame->uvStride = _fetch_Frame->yStride; ///< Same stride as the Y plane; one row contains interleaved UV data.
			_fetch_Frame->uvSize = _fetch_Frame->ySize / 2;
			_fetch_Frame->u = NULL;
			_fetch_Frame->uStride = 0;
			_fetch_Frame->uSize = 0;
			_fetch_Frame->v = NULL;
			_fetch_Frame->vStride = 0;
			_fetch_Frame->vSize = 0;

			ak_vi_release_frame (ak_vi_get_handle (VIDEO_DEV0), &vi_frame);

			/// Capture successful.
			return AK_SUCCESS;

		} else {

			_ERR ("Get VI(%d) Frame Failed.", _fetch_ch);
			/// Note: PMEM was allocated above and must be freed here.
			goto failed2;
		}
	}

	/// Retrieve the latest VI width/height to validate the requested resolution.
	if (0 == (VideoChannelAttr.res[VIDEO_CHN_SUB].width % width)
			&& 0 == (VideoChannelAttr.res[VIDEO_CHN_SUB].height % height)
			&& width <= VideoChannelAttr.res[VIDEO_CHN_SUB].width
			&& height <= VideoChannelAttr.res[VIDEO_CHN_SUB].height) {

		_fetch_ch = VIDEO_CHN_SUB;
		_fetch_orign_width = VideoChannelAttr.res[VIDEO_CHN_SUB].width;
		_fetch_orign_height = VideoChannelAttr.res[VIDEO_CHN_SUB].height;

	} else if (0 == (VideoChannelAttr.res[VIDEO_CHN_MAIN].width % width)
			&& 0 == (VideoChannelAttr.res[VIDEO_CHN_MAIN].height % height)
			&& width <= VideoChannelAttr.res[VIDEO_CHN_MAIN].width
			&& height <= VideoChannelAttr.res[VIDEO_CHN_MAIN].height) {

		_fetch_ch = VIDEO_CHN_MAIN;
		_fetch_orign_width = VideoChannelAttr.res[VIDEO_CHN_MAIN].width;
		_fetch_orign_height = VideoChannelAttr.res[VIDEO_CHN_MAIN].height;

	} else {

		_ERR ("Resulution %dx%d Not Support.", width, height);
		goto failed;
	}

	if (_fetch_orign_width == width && _fetch_orign_height == height) {
		/// 1:1 capture scale.
		_fetch_scale = 1;

	} else if (_fetch_orign_width / 4 == width && _fetch_orign_height / 4 == height) {

		/// 1:4 capture scale.
		_fetch_scale = 4;

	} else {
		_ERR ("Size Should be %dx%d or %dx%d But %dx%d.",
				_fetch_orign_width, _fetch_orign_height,
				_fetch_orign_width / 4, _fetch_orign_height / 4,
				width, height);
		goto failed;
	}

	/// Allocate physical memory to store the YUV data.
	yuvSize = width * height * 3 / 2;
	yuv = (unsigned char *)(akuio_alloc_pmem (yuvSize));
	if (NULL == yuv) {
		_ERR ("Out of Memory for YUV(Size=%d).", yuvSize);
		goto failed;
	}

	/// Record the width and height requested by the user;
	/// the data copy will be performed by the background thread below.
	_fetch_Frame = yuv_frame;


	/// Initialize the frame data structure.
	_fetch_Frame->width = width;
	_fetch_Frame->height = height;
	_fetch_Frame->ts = 0; ///< To be filled in when the data is captured.
	_fetch_Frame->seqno = 0; ///< To be filled in when the data is captured.

	_fetch_Frame->pixels = yuv;
	_fetch_Frame->size = yuvSize; ///< 1.5 x Width x Height.

	/// Initialize Y-plane dimensions.
	_fetch_Frame->y = _fetch_Frame->pixels;
	_fetch_Frame->yStride = _fetch_Frame->width;
	_fetch_Frame->ySize = _fetch_Frame->width * _fetch_Frame->height;

	/// Initialize UV-plane dimensions.
	_fetch_Frame->uv = _fetch_Frame->y + _fetch_Frame->ySize; ///< UV data immediately follows the Y plane.
	_fetch_Frame->uvStride = _fetch_Frame->yStride; ///< Same stride as the Y plane; one row contains interleaved UV data.
	_fetch_Frame->uvSize = _fetch_Frame->ySize / 2;

	/// NV12 does not support separate U and V planes.
	_fetch_Frame->u = NULL;
	_fetch_Frame->uStride = 0;
	_fetch_Frame->uSize = 0;
	_fetch_Frame->v = NULL;
	_fetch_Frame->vStride = 0;
	_fetch_Frame->vSize = 0;


	/// Submit the request; the capture loop thread will pick it up
	/// and signal a semaphore when the frame is ready.
	_fetch_request = 1;
	ak_sleep_ms (50);
	if (0 == sem_trywait (&_Sem_fetch)) {
		/// Capture succeeded.
		/// Note: after a successful capture one mutex lock and one PMEM allocation
		/// remain outstanding - be careful to avoid a PMEM leak later.
		return 0;
	}

	/// Try once more.
	ak_sleep_ms (50);
	if (0 == sem_trywait (&_Sem_fetch)) {
		return 0;
	}

	/// Capture failed.
	_fetch_request = 0;
	_fetch_Frame = NULL;

failed2:

	/// Note: freeing PMEM here is critical.
	akuio_free_pmem (yuv);
	yuv = NULL;

failed:

	/// Release the mutex lock.
	pthread_mutex_unlock (&_Mutex_fetch);

	/// Return failure.
	return -1;
}


int ak_venc_release_yuv (struct venc_yuv_frame *yuv_frame) {

	if (!yuv_frame) {
		return -1;
	}

	if (!yuv_frame->pixels) {
		return -1;
	}

	/// Free the YUV memory allocated on UIO.
	akuio_free_pmem (yuv_frame->pixels);
	memset (yuv_frame, 0, sizeof (yuv_frame[0]));

	/// Release the mutex lock; after this the caller may invoke the YUV fetch interface again.
	pthread_mutex_unlock (&_Mutex_fetch);

	return 0;
}


/**
 * capture_encode_frame - call vi capture interface to get frame
 * thread_arg[IN]: current thread group's argument
 */
static void capture_encode_frame(struct thread_arg *thread_arg)
{
	int ret = AK_FAILED;
	int err_no = 0;
	unsigned int get_frame_fail_cnt = 0;
	struct frame_node *frame = NULL;

#if VENC_CALC_CAP_TIME
	long cap_time = 0;
	struct ak_timeval pre_tv;
	struct ak_timeval cur_tv;
	ak_get_ostime(&pre_tv);
#endif

#if CALC_VIDEO_CAPTURE
	ak_get_ostime(&video_ctrl.calc_time);
#endif

	/// Initialize the semaphore and mutex; they will be released when this thread exits.
	sem_init (&_Sem_fetch, 0, 0);
//	pthread_mutex_init (&_Mutex_fetch, NULL);


	while (thread_arg->cap_run && (video_ctrl.inited_enc_grp_num > 0)) {
		/*
		 * new one frame object, then get frame store to head_frame,
		 * encode thread will according to the bit_map to decide
		 * which group should be encode.
		 */
		frame = alloc_video_frame();
		if (frame) {
#if VENC_CALC_CAP_TIME
			ak_get_ostime(&cur_tv);
#endif
			/*
			 * get yuv frame here
			 */
			ret = ak_vi_get_frame(thread_arg->vi_handle, frame->vframe);

			/// Handle the YUV capture logic if a request is pending.
			if (0 == ret && _fetch_request) {

				unsigned char *yPlane = frame->vframe->vi_frame[_fetch_ch].data;
				unsigned char *uvPlane = frame->vframe->vi_frame[_fetch_ch].data + _fetch_Frame->ySize;

				/// Record the frame timestamp and sequence number.
				_fetch_Frame->ts = frame->vframe->vi_frame[_fetch_ch].ts;
				_fetch_Frame->seqno = frame->vframe->vi_frame[_fetch_ch].seq_no;

				if (1 == _fetch_scale) {
					/// At 1:1 scale, use a direct memcpy.

					/// Copy the Y plane data.
					memcpy (_fetch_Frame->y, yPlane, _fetch_Frame->ySize);

					/// Copy the UV plane data.
					memcpy (_fetch_Frame->uv, uvPlane, _fetch_Frame->uvSize);

				} else {

					int i = 0, ii = 0;
					int offset = 0;

					/// UV scaling is not supported; only Y data is downscaled here.
					/// Copy the downscaled Y plane data.
					for (i = 0, offset = 0; i < _fetch_orign_height; i += _fetch_scale) {
						int const offset_h = i * _fetch_orign_width; ///< Row pixel offset.
						for (ii = 0; ii < _fetch_orign_width; ii += _fetch_scale) {
							_fetch_Frame->y[offset++] = yPlane[offset_h + ii];
						}
					}


					/// Copy the downscaled UV plane data.
					for (i = 0, offset = 0; i < _fetch_orign_height / 2; i += _fetch_scale) {
						int const offset_h = i * _fetch_orign_width; ///< Row pixel offset.
						for (ii = 0; ii < _fetch_orign_width; ii += _fetch_scale * 2) {
							///< Each row contains both U and V samples interleaved,
							///< so they are processed and copied two at a time.

							_fetch_Frame->uv[offset++] = uvPlane[offset_h + ii]; ///< U sample
							_fetch_Frame->uv[offset++] = uvPlane[offset_h + ii + 1]; ///< V sample
						}
					}

					/// Discard UV plane metadata (UV scaling not supported).
					_fetch_Frame->uv = NULL;
					_fetch_Frame->uvStride = 0;
					_fetch_Frame->uvSize = 0;

				}


				/// Request complete; signal the semaphore so that @ref ak_venc_fetch_yuv can return.
				sem_post (&_Sem_fetch);
				_fetch_request = 0;

			}

#if VENC_CALC_CAP_TIME
			cap_time = ak_diff_ms_time(&cur_tv, &pre_tv);
			if (cap_time > 50) {
				ak_print_notice(MPI_VENC "ak_vi_get_frame diff time=%ld(ms), "
				    "seq_no=%ld\n",
					cap_time, frame->vframe->vi_frame[VIDEO_CHN_MAIN].seq_no);
			}
			ak_get_ostime(&pre_tv);
#endif

#if CALC_VIDEO_CAPTURE
			calc_video_capture_frame(frame->vframe);
#endif

			if (AK_SUCCESS == ret) {
				if (get_frame_fail_cnt > 0) {
					ak_print_notice_ex(MPI_VENC
						"ak_vi_get_frame recovered after %u consecutive failures, vi_handle=%p\n",
						get_frame_fail_cnt, thread_arg->vi_handle);
					get_frame_fail_cnt = 0;
				}
				add_to_encode_list(thread_arg, frame);
			} else {
				err_no = ak_get_error_no();
				++get_frame_fail_cnt;
				if ((get_frame_fail_cnt <= 3)
					|| ((get_frame_fail_cnt % CAPTURE_FAIL_LOG_INTERVAL) == 0)) {
					ak_print_warning_ex(MPI_VENC
						"ak_vi_get_frame failed: count=%u, err_no=%d, vi_handle=%p, inited_grp=%d\n",
						get_frame_fail_cnt, err_no, thread_arg->vi_handle,
						video_ctrl.inited_enc_grp_num);
				}
				/*
				 * ak_vi_get_frame failed, so this frame was never acquired from VI.
				 * Releasing it back to VI here can corrupt VI bookkeeping.
				 */
				free_frame(frame);
			}

			ak_sleep_ms(10);
		} else {
			ak_print_error_ex(MPI_VENC "alloc_video_frame failed\n");
			thread_arg->cap_run = AK_FALSE;
			break;
		}
	}


//	pthread_mutex_destroy (&_Mutex_fetch);
	sem_destroy (&_Sem_fetch);

	ak_print_notice_ex(MPI_VENC "leave...\n");
}

/*
 * capture_thread - capture thread, do capture and then wake up encode thread
 * arg[IN], thread argument, now no use
 * return: when it exit, return NULL
 */
static void *capture_thread(void *arg)
{
	struct thread_arg *thread_arg = (struct thread_arg *)arg;
	ak_print_normal_ex(MPI_VENC "start to capture, thread id: %ld\n",
	    ak_thread_get_tid());

	ak_thread_set_name(thread_arg->cap_name[0] ? thread_arg->cap_name : "venc_capture");

	while (thread_arg->cap_run) {
		ak_print_notice_ex(MPI_VENC "capture thread sleep...\n");
		/* wait signal to start capture */
		ak_thread_sem_wait(&thread_arg->cap_sem);
		ak_print_notice_ex(MPI_VENC "capture thread wakeup...\n");

		if (thread_arg->cap_run) {
			capture_encode_frame(thread_arg);
		}
	}

	ak_print_normal_ex(MPI_VENC "### thread id: %ld exit ###\n",
	    ak_thread_get_tid());
	ak_thread_exit();
	return NULL;
}

/*
 * save_streams - save stream to handle's stream queue
 * vs[IN]: video stream want be save
 * enc_handle[IN]: encode handle
 */
static void save_streams(struct video_streams_t *vs,
		struct encode_group *handle)
{
	/* if queue is full, drop first */
	if (handle->streams_count + 1 > ONE_STREAM_MAX) {
		/*
		 * 1.Drop p frame at first of frame list
		 * 2.If all p frame in one gop was been dropped, we drop the I frame
		 *   to implement drop a whole gop frames.
		 */
		struct video_streams_t *first_frame;
		struct video_streams_t *remove_frame = NULL;

		first_frame = list_first_entry(&handle->stream_list,
				struct video_streams_t, list);

		/* stream overflow, control the print */
		if ((!handle->stream_overflow_print_cnt) ||
				(handle->stream_overflow_print_cnt % handle->encode_frames == 0))
			ak_print_error_ex(MPI_VENC "stream is too many, chn: %d, diff ts: %llu\n",
					handle->grp_type, (vs->stream.ts - first_frame->stream.ts));
		handle->stream_overflow_print_cnt++;

		if (first_frame->stream.frame_type == FRAME_TYPE_I) {
			struct video_streams_t *second_frame;

			second_frame = list_first_entry(handle->stream_list.next,
					struct video_streams_t, list);
			if (second_frame->stream.frame_type == FRAME_TYPE_I) {
				remove_frame = first_frame;	/* drop I frame */
			} else {
				remove_frame = second_frame; /* drop P frame */
			}
		} else /* first frame is P */
			remove_frame = first_frame;

		ak_print_info_ex(MPI_VENC "remove %s Frame, sz=%u, seq_no=%ld\n",
			remove_frame->stream.frame_type == FRAME_TYPE_I ? "I" : "P",
			remove_frame->stream.len, remove_frame->stream.seq_no);
		handle->streams_count--;
		list_del_init(&remove_frame->list);
		free(remove_frame->stream.data);
		free(remove_frame);
	}

	/* save to queue */
	handle->streams_count++;

#if VENC_GET_STREAM_DEBUG
	ak_print_warning_ex(MPI_VENC "grp_type=%d, streams_count=%d\n",
		handle->grp_type, handle->streams_count);
	ak_print_normal_ex(MPI_VENC "vs=%p, stream ts=%llu, len=%u, seq_no=%lu\n",
		vs, vs->stream.ts, vs->stream.len, vs->stream.seq_no);
#endif

	/* add stream to group's queue */
	list_add_tail(&vs->list, &handle->stream_list);
	vs->ref = handle->req_ref;
	vs->bit_map = handle->user_map;
}

/*
 * frame_to_stream - means encode, convert frame to bit stream
 * enc_handle[IN]: encode handle
 * pos[IN]: pointer to frame which will be encode
 */
static void frame_to_stream(struct encode_group *enc_handle,
							struct frame_node *pos)
{
	int chn = enc_handle->record.use_chn;
	const unsigned char *buf = pos->vframe->vi_frame[chn].data;
	unsigned int frame_len = pos->vframe->vi_frame[chn].len;
	unsigned long long ts = pos->vframe->vi_frame[chn].ts;
	enc_handle->mdinfo = pos->vframe->mdinfo;

	#if VENC_CALC_ENC_TIME
	struct ak_timeval tv_start;
	struct ak_timeval tv_end;
	long enc_time = 0;
	#endif
	struct ak_timeval send_start = {0};
	struct ak_timeval send_end = {0};

	struct video_streams_t *vs = (struct video_streams_t *)calloc(1,
		sizeof(struct video_streams_t));
	if (!vs) {
		ak_print_error_ex(MPI_VENC "calloc failed\n");
		return;
	}

	vs->stream.seq_no = pos->vframe->vi_frame[chn].seq_no;
#if VENC_ORIGIN_TS
	vs->stream.ts = ts;
#elif VENC_CALC_TS
	vs->stream.ts = calc_stream_ts(enc_handle, ts);
#elif VENC_SMOOTH_TS
	/* smooth ts */
	vs->stream.ts = smooth_ts(enc_handle, ts, vs->stream.seq_no);
#endif

	/* drop invalid frame, should not happen */
	if (vs->stream.ts == 0) {
		free(vs);
		vs = NULL;
		return;
	}

#if VENC_CALC_ENC_TIME
	ak_get_ostime(&tv_start);
#endif

	/* encode */
	ak_get_ostime(&send_start);
	int send_ret = ak_venc_send_frame(enc_handle, buf, frame_len, &vs->stream);
	ak_get_ostime(&send_end);
	long send_ms = ak_diff_ms_time(&send_end, &send_start);
	if (send_ret) {
		ak_print_error_ex(MPI_VENC
			"encode frame failed: grp_type=%d, seq_no=%lu, frame_len=%u, send_ms=%ld\n",
			enc_handle->grp_type, vs->stream.seq_no, frame_len, send_ms);
	} else {
		if (send_ms > ENCODE_SEND_SLOW_MS) {
			ak_print_warning_ex(MPI_VENC
				"encode frame slow: grp_type=%d, seq_no=%lu, frame_len=%u, send_ms=%ld\n",
				enc_handle->grp_type, vs->stream.seq_no, frame_len, send_ms);
		}
		if (0 == vs->stream.len)
			ak_print_info_ex(MPI_VENC "vs->stream.len=%d\n", vs->stream.len);
	}

#if VENC_CALC_ENC_TIME
	ak_get_ostime(&tv_end);
	enc_time = ak_diff_ms_time(&tv_end, &tv_start);
	if (enc_time > 100) {
		ak_print_warning(MPI_VENC ">>> encode frame type=%d, seq_no=%ld, "
		    "len=%d, time=%ld(ms)\n",
			vs->stream.frame_type, vs->stream.seq_no, vs->stream.len, enc_time);
	}
#endif

#if CALC_VIDEO_ENCODE
	calc_venc_stream();
#endif

    T_AKVLIB_PAR encpar = {0};
	AKV_Encoder_Get_Parameters(enc_handle->lib_handle, &encpar);
    if (encpar.GopLength > 0) {
        stat_venc_info(enc_handle, &(vs->stream), ts);
    }

	/* save bitstream */
	if (vs->stream.len > 0) {
		enc_handle->pre_ts = vs->stream.ts;
		ak_thread_mutex_lock(&enc_handle->lock);
		save_streams(vs, enc_handle);
		ak_thread_mutex_unlock(&enc_handle->lock);
	} else {
		free(vs);
		vs = NULL;
	}

	/* after this group has been encode, clear bit */
	enum encode_group_type type = enc_handle->record.enc_grp;
	clear_bit(&pos->bit_map, type);
}

/*
 * encode_frame - do all encode things
 * thread_arg[IN]: thread group's argument
 */
static void encode_frame(struct thread_arg *thread_arg)
{
	/* one frame corresponding multiple encode group */
	struct encode_group *enc_handle = NULL;	//video encode handle
	struct frame_node *pos = NULL;
	struct frame_node *n = NULL;		//frame loop value
	struct encode_group *enc_snapshot[ENCODE_GRP_NUM];
	int snapshot_cnt = 0;

	/* iterate get frame from frame queue */
	list_for_each_entry_safe(pos, n, &thread_arg->head_frame, list) {
		if (!(thread_arg->enc_run)) {
			ak_print_normal_ex(MPI_VENC "encode need to exit\n");
			break;
		}

		/*
		 * Snapshot the active encoder list under cancel_mutex, then perform the
		 * potentially slow encode work without holding cancel_mutex.
		 *
		 * This avoids shutdown/cancel paths blocking behind long-running encode
		 * operations while still maintaining list integrity.
		 */
		snapshot_cnt = 0;
		ak_thread_mutex_lock(&video_ctrl.cancel_mutex);
		list_for_each_entry(enc_handle, &video_ctrl.venc_list, enc_list) {
			if (snapshot_cnt < ENCODE_GRP_NUM) {
				enc_snapshot[snapshot_cnt++] = enc_handle;
			}
		}
		ak_thread_mutex_unlock(&video_ctrl.cancel_mutex);

		for (int i = 0; i < snapshot_cnt; i++) {
			enc_handle = enc_snapshot[i];
			ak_thread_mutex_lock(&enc_handle->close_mutex);
			if ((enc_handle->req_ref > 0)
				&& test_bit(enc_handle->record.enc_grp, &pos->bit_map)) {
				frame_to_stream(enc_handle, pos);
			}
			ak_thread_mutex_unlock(&enc_handle->close_mutex);

			if (!(thread_arg->enc_run)) {
				ak_print_normal_ex(MPI_VENC "encode need to exit\n");
				break;
			}
		}

			ak_thread_mutex_lock(&thread_arg->lock);
			list_del_init(&pos->list);
			ak_thread_mutex_unlock(&thread_arg->lock);
			int release_ret = ak_vi_release_frame(thread_arg->vi_handle, pos->vframe);
			if (release_ret) {
				ak_print_warning_ex(MPI_VENC
					"ak_vi_release_frame failed: vi_handle=%p, ts=%llu, seq_no=%lu, grp_num=%d\n",
					thread_arg->vi_handle,
					pos->vframe->vi_frame[VIDEO_CHN_MAIN].ts,
					pos->vframe->vi_frame[VIDEO_CHN_MAIN].seq_no,
					video_ctrl.inited_enc_grp_num);
			}
			free_frame(pos);
		}
}

/*
 * encode_thread - encode main thread
 * arg[IN]: thread group's argument
 */
static void *encode_thread(void *arg)
{
	struct thread_arg *thread_arg = (struct thread_arg *)arg;

	long int tid = ak_thread_get_tid();
	ak_print_normal_ex(MPI_VENC "start to encode, thread id: %ld\n", tid);

	ak_thread_set_name(thread_arg->enc_name[0] ? thread_arg->enc_name : "venc_encode");

	while (thread_arg->enc_run) {
		ak_thread_sem_wait(&thread_arg->enc_sem);

		if (thread_arg->enc_run) {
			encode_frame(thread_arg);
		}
	}
	ak_print_normal_ex(MPI_VENC "### thread id: %ld exit ###\n", tid);
	ak_thread_exit();
	return NULL;
}

/*
 * venc_find_user - find correct position to get stream
 * handle[IN]: stream handle
 * return: find return correctly position pointer, failed return NULL;
 */
static void* venc_find_user(struct stream_handle *handle)
{
	unsigned char find = AK_FALSE;
	struct video_streams_t *pos = NULL;
	struct encode_group *enc_handle = (struct encode_group *)(handle->enc_handle);

	list_for_each_entry(pos, &(enc_handle->stream_list), list) {
#if VENC_GET_STREAM_DEBUG
		ak_print_notice_ex(MPI_VENC "bit_map=%d, ref=%d\n",
		    pos->bit_map, pos->ref);
#endif
		if (test_bit(handle->id, &(pos->bit_map))) {
			/* this entry have not gotten yet */
			find = AK_TRUE;
			break;
		}
	}
	if (!find) {
		pos = NULL;
	}

	return pos;
}

/*
 * start_service_work - start one group thread
 * vi[IN]: this group's vi handle
 * return: 0 on success, -1 failed
 */
static int start_service_work(void *vi)
{
	struct thread_arg *arg;

	if (!list_empty(&video_ctrl.thread_list)) {

		arg = list_first_entry(&video_ctrl.thread_list, struct thread_arg, list);
		if (arg->vi_handle == vi) {
			/* same vi handle come in, let it success */
			return AK_SUCCESS;
		} else {
			/*
			 * we not permit to register two or more
			 * different vi handle in same device for
			 * encode, so here we return false.
			 */
			ak_print_error(MPI_VENC "same video input device can just register"
					"only one vi-handle for encode\n");
			return AK_FAILED;
		}
	}

	arg = (struct thread_arg *)calloc(1, sizeof(struct thread_arg));
	if (!arg) {
		return AK_FAILED;
	}

	ak_thread_sem_init(&arg->cap_sem, 0);
	ak_thread_sem_init(&arg->enc_sem, 0);

	ak_thread_mutex_init(&arg->lock, NULL);
	INIT_LIST_HEAD(&arg->head_frame);		//store frame
	list_add_tail(&arg->list, &video_ctrl.thread_list);

	arg->vi_handle = vi;
	arg->cap_run = 1;
	arg->enc_run = 1;
	{
		unsigned int name_tag = ((uintptr_t)vi) & 0xff;
		snprintf(arg->cap_name, sizeof(arg->cap_name), "venc-cap-%02x", name_tag);
		snprintf(arg->enc_name, sizeof(arg->enc_name), "venc-enc-%02x", name_tag);
	}

	ak_thread_create(&arg->cap_tid, capture_thread, (void *)arg,
		ANYKA_THREAD_NORMAL_STACK_SIZE, 90);
	ak_print_normal_ex(MPI_VENC "create capture_thread, tid=%lu\n", arg->cap_tid);

	ak_thread_create(&arg->enc_tid, encode_thread, (void *)arg,
		ANYKA_THREAD_NORMAL_STACK_SIZE, 90);
	ak_print_normal_ex(MPI_VENC "create encode_thread, tid=%lu\n", arg->enc_tid);

	ak_thread_mutex_lock(&video_ctrl.lock);
	video_ctrl.thread_group_num++;	//record thread group number
	ak_thread_mutex_unlock(&video_ctrl.lock);

	/* notify, start to capture and encode */
	ak_thread_sem_post(&arg->cap_sem);
	ak_print_normal_ex(MPI_VENC "service start\n\n");

	return 0;
}

#if 0
static void *wait_irq_thread(void *arg)
{
	ak_print_normal_ex(MPI_VENC "entry\n");
	ak_thread_set_name("venc_wait_irq");

	while (video_ctrl.wait_irq_runflg) {
		int irq_state = -1;

		akuio_wait_irq(&irq_state);
		/*
		 ak_print_normal_ex(MPI_VENC "irq_state: %d\n", irq_state);
		 */
		if (video_ctrl.wait_irq_runflg)
			AKV_Encoder_Interrupt_Handler(irq_state);
	}

	ak_print_normal_ex(MPI_VENC "exit\n");
	ak_thread_exit();
	return NULL;
}

static void start_wait_irq_thread(void)
{
	video_ctrl.wait_irq_runflg = 1;
	ak_thread_create(&video_ctrl.wait_irq_thid, wait_irq_thread, NULL,
			ANYKA_THREAD_NORMAL_STACK_SIZE, -1);

	/* test */
	ak_print_notice_ex(MPI_VENC "free dma size: %lu\n", akuio_get_free_pmem());
}
#endif

static int venc_init_open_param(T_AKV_ENC_OPEN_INPUT *open_input,
		const struct encode_param *param)
{
	/* config file path check */
	if (ak_check_file_exist(VENC_CFG_PATH)) {
		ak_print_error_ex(MPI_VENC "config file %s, %s\n",
		     VENC_CFG_PATH, strerror(errno));
		set_error_no(ERROR_TYPE_FILE_NOT_EXIT);
		return AK_FAILED;
   	}

#if !(USE_LIB_V1)
	/* config file path set will enter debug mode */
	open_input->sCfgFileName = NULL;
	ak_print_notice_ex("we don't use config for encode\n");
#else
	open_input->sCfgFileName = VENC_CFG_PATH;
	ak_print_notice_ex("we use config %s for encode\n", VENC_CFG_PATH);
#endif

	/* h.264 */
	if (param->enc_out_type == H264_ENC_TYPE) {
		switch (param->profile) {
			case PROFILE_MAIN:
				open_input->AkvParams.eProfile = AK_PROFILE_AVC_MAIN;
				break;
			case PROFILE_HIGH:
				open_input->AkvParams.eProfile = AK_PROFILE_AVC_HIGH;
				break;
			case PROFILE_BASE:
				open_input->AkvParams.eProfile = AK_PROFILE_AVC_BASELINE;
				break;
			default:
				ak_print_normal_ex(MPI_VENC "use default profile: main\n");
				open_input->AkvParams.eProfile = AK_PROFILE_AVC_MAIN;
				break;
		}
	} else if (param->enc_out_type == HEVC_ENC_TYPE) {	/* h.265 */
		switch (param->profile) {
			case PROFILE_HEVC_MAIN:
				open_input->AkvParams.eProfile = AK_PROFILE_HEVC_MAIN;
				break;
			case PROFILE_HEVC_MAIN_STILL:
				open_input->AkvParams.eProfile = AK_PROFILE_HEVC_MAIN_STILL;
				break;
			default:
				ak_print_normal_ex(MPI_VENC "use default profile: main\n");
				open_input->AkvParams.eProfile = AK_PROFILE_HEVC_MAIN;
				break;
		}
	} else if (param->enc_out_type == MJPEG_ENC_TYPE) {	/* jpeg */
		open_input->AkvParams.eProfile = AK_PROFILE_JPEG;
	} else {	/* error */
		ak_print_error_ex(MPI_VENC "error enc out type\n");
		return AK_FAILED;
	}

	open_input->AkvParams.FrameWidth  = param->width;
	open_input->AkvParams.FrameHeight = param->height;
	open_input->AkvParams.eRCMode     = (param->br_mode == BR_MODE_CBR) ?
		AK_RC_CBR : AK_RC_VBR;

	open_input->AkvParams.TargetBitRate = param->bps;
	/* max bit rate same as target */
	open_input->AkvParams.MaxBitRate    = param->bps;
	open_input->AkvParams.EncFrameRate  = param->fps;

	open_input->AkvParams.InitialSliceQP = (param->minqp + param->maxqp) / 2;
	open_input->AkvParams.MinQP = param->minqp;
	open_input->AkvParams.MaxQP = param->maxqp;

	open_input->AkvParams.GopLength = param->goplen;
	open_input->AkvParams.GopNumB   = 0;	//no B frame
	open_input->AkvParams.NumSlices = 1;	//[1, 10]
	open_input->AkvParams.SmartMode = 0;	//init for disable

	return AK_SUCCESS;
}

const char *ak_venc_get_version(void)
{
	return venc_version;
}


const char *ak_venc_get_cfg_path (char *stack, int stacklen) {

	if (NULL == stack || stacklen == 0) {
		ak_print_error_ex(MPI_VENC "invalid cfg path.\n");
		return NULL;
	}

	snprintf (stack, stacklen, "%s", VENC_CFG_PATH);
	return stack;
}

int ak_venc_set_cfg_path (const char *path) {

	if (NULL == path || strlen (path) == 0) {
		ak_print_error_ex(MPI_VENC "invalid cfg path.\n");
	}

	snprintf (_cfg_path, sizeof (_cfg_path), "%s", path);
	return 0;
}

/**
 * ak_venc_open - open encoder and set encode param
 * @param[IN]: encode param
 * return: on success return encode handle, failed return NULL.
 */
void* ak_venc_open(const struct encode_param *param)
{
	if (!param) {
		ak_print_error_ex(MPI_VENC "invalid param: %p\n", param);
		set_error_no(ERROR_TYPE_INVALID_ARG);
		return NULL;
	}

	ak_thread_mutex_lock(&close_lock);
	ak_thread_mutex_lock(&video_ctrl.lock);
	/* init model global handle member */
	if (!video_ctrl.module_init) {
		video_ctrl.module_init = AK_TRUE;
		/* register encode callback, init video encoder */
		if (AK_SUCCESS != venc_encoder_init()) {
			video_ctrl.module_init = AK_FALSE;
			ak_thread_mutex_unlock(&video_ctrl.lock);
			ak_thread_mutex_unlock(&close_lock);
			set_error_no(ERROR_TYPE_DEV_OPEN_FAILED);
			return NULL;
		}
		INIT_LIST_HEAD(&video_ctrl.venc_list);
		ak_thread_mutex_init(&video_ctrl.cancel_mutex, NULL);

		venc_sys_ipc_register();
	}

	/* init specifically handle's resource */
	int index = param->enc_grp;
	int venc_init_ret = venc_handle_init(index);
	ak_thread_mutex_unlock(&video_ctrl.lock);
	if (venc_init_ret){
		ak_thread_mutex_unlock(&close_lock);
		return NULL;
	}

	/* init encode handle */
	int type = param->enc_grp;
	struct encode_group *ret_handle = NULL;
	ak_print_normal_ex(MPI_VENC "now init type %d\n", type);

	/* to make multiple thread operate safe */
	struct encode_group *enc_handle = video_ctrl.grp[type];

	ak_thread_mutex_lock(&enc_handle->lock);
	if(enc_handle->user_count > 0) {
		add_user(enc_handle);
		ret_handle = enc_handle;
		ak_print_normal_ex(MPI_VENC "add user ...\n");
		goto venc_open_end;
	}

	/* open new encode group handle */
	int encode_output_buffer_len = ENC_OUT_MAX_SZ;
	if (param->use_chn == ENCODE_SUB_CHN)
		encode_output_buffer_len /= 2;

	enc_handle->encbuf = (unsigned char *)calloc(1, encode_output_buffer_len);
	enc_handle->record.br_mode = param->br_mode;
	memcpy(&enc_handle->record, param, sizeof(struct encode_param));

	/* init input param */
	T_AKV_ENC_OPEN_INPUT open_input = {0};
	AKV_Encoder_Get_Parameters(NULL, &open_input.AkvParams);
	if (venc_init_open_param(&open_input, param)){
		free(enc_handle->encbuf);
		enc_handle->encbuf = NULL;
		ak_print_error_ex(MPI_VENC "venc_init_open_param failed!\n");
		goto venc_open_end;
	}

	enc_handle->lib_handle = AKV_Encoder_Open(&open_input);
	if (!enc_handle->lib_handle) {
		free(enc_handle->encbuf);
		enc_handle->encbuf = NULL;
		ak_print_error_ex(MPI_VENC "AKV_Encoder_Open failed!\n");
		set_error_no(ERROR_TYPE_MEDIA_LIB_FAILED);
		goto venc_open_end;
	}

	ak_print_notice_ex(MPI_VENC "Video encoder open success\n");
	ak_print_normal_ex(MPI_VENC "\n\tparams: w=%d, h=%d, qpmin=%d, qpmax=%d,"
		"bps=%d, gop=%u, fps=%u, profile=%d\n",
		open_input.AkvParams.FrameWidth,
		open_input.AkvParams.FrameHeight,
		open_input.AkvParams.MinQP,
	   	open_input.AkvParams.MaxQP,
		open_input.AkvParams.TargetBitRate,
	   	open_input.AkvParams.GopLength,
		open_input.AkvParams.EncFrameRate,
		open_input.AkvParams.eProfile);

	INIT_LIST_HEAD(&(enc_handle->stream_list));
	/* handle should record which enc_grp its belong to */
	enc_handle->grp_type = type; //record group type
	add_user(enc_handle);
	ret_handle = enc_handle;

	venc_sysipc_bind_chn_handle(enc_handle, type);

	ak_print_normal_ex(MPI_VENC "allocating type=%d, enc_handle: %p\n\n",
			type, enc_handle);

venc_open_end:
	ak_thread_mutex_unlock(&enc_handle->lock);
	ak_thread_mutex_unlock(&close_lock);
	return ret_handle;
}

/*
 * ak_venc_send_frame - encode single frame
 * @enc_handle[IN]: encode handle
 * @mdinfo[IN]: mdinfo pointer
 * return: 0 success; -1 failed
 */
 int ak_venc_set_mdinfo(void *enc_handle, void *mdinfo)
{
	if (enc_handle == NULL) {
		ak_print_error_ex(MPI_VENC "invalid enc_handle\n");
		set_error_no(ERROR_TYPE_POINTER_NULL);
		return AK_FAILED;
	}

	if (mdinfo == NULL) {
		ak_print_error_ex(MPI_VENC "invalid mdinfo\n");
		set_error_no(ERROR_TYPE_POINTER_NULL);
		return AK_FAILED;
	}

	struct encode_group *handle = (struct encode_group *)enc_handle;
	handle->mdinfo = mdinfo;

	return AK_SUCCESS;
}


/*
 * ak_venc_send_frame - encode single frame
 * @enc_handle[IN]: encode handle
 * @frame[IN]: frame which you want to encode
 * @frame_len[IN]: lenght of frame
 * @stream[OUT]: encode output buffer address
 * return: 0 success; -1 failed
 */
int ak_venc_send_frame(void *enc_handle,
		const unsigned char *frame,
		unsigned int frame_len,
		struct video_stream *stream)
{
	if (enc_handle == NULL) {
		ak_print_error_ex(MPI_VENC "invalid enc_handle\n");
		set_error_no(ERROR_TYPE_POINTER_NULL);
		return AK_FAILED;
	}

	struct encode_group *handle = (struct encode_group *)enc_handle;
	int ret = AK_SUCCESS;

	if (handle->record.width * handle->record.height * 3 / 2
			!= frame_len) {
		/// Report the error here first; otherwise an assertion will be hit inside the AKV interface.
		_ERR ("Target Encode Res %lux%lu Error.",
				handle->record.width, handle->record.height);
		return -1;
	}

#if 1
	T_AKV_DMA_BUFFER pdmabuf = {0};

	pdmabuf.pData = (unsigned char *)frame;	//virtual address
	pdmabuf.pa    = akuio_vaddr2paddr((void *)frame);	//physical address
	pdmabuf.size  = frame_len;	//size of buffer
	pdmabuf.hHandle = NULL;		//handle for user

	T_AKV_SRC_FRAME pframe = {0};
	pframe.pdmabuf = &pdmabuf;
	pframe.mdinfo.data   = handle->mdinfo;
	pframe.mdinfo.width  = 32;
	pframe.mdinfo.height = (handle->record.height % 24) == 0 ? 24 : 16;
	pframe.srcTimeStamp = 0;	//ignore it
#else
	T_AKV_DMA_BUFFER pframe = {0};

	pframe.pData = (unsigned char *)frame;	//virtual address
	pframe.pa    = akuio_vaddr2paddr((void *)frame);	//physical address
	pframe.size  = frame_len;	//size of buffer
	pframe.hHandle = NULL;		//handle for user
	pframe.mdinfo  = handle->mdinfo;
#endif

	T_AKV_STREAM_BUFFER pstream = {0};
	pstream.pData = handle->encbuf;	//store output stream
	pstream.BufSize = ENC_OUT_MAX_SZ;	//buf encbuf size

#if 0
	/* start wait-irq-thread in-needed */
	if (video_ctrl.wait_irq_thid == 0)
		start_wait_irq_thread();
#endif

	/* do encode */
	ak_thread_mutex_lock(&video_ctrl.lock);
	AKV_Encoder_Process(handle->lib_handle, &pframe, &pstream);
	ak_thread_mutex_unlock(&video_ctrl.lock);

	/* get code frame */
	if (pstream.OutSize > 0) {
		stream->data = (unsigned char *)calloc(1, pstream.OutSize);
		if (stream->data) {
			memcpy(stream->data, pstream.pData, pstream.OutSize);

			switch (pstream.slicetype) {
			case AK_SLICE_I:
				stream->frame_type = FRAME_TYPE_I;
				break;
			case AK_SLICE_P:
				stream->frame_type = FRAME_TYPE_P;
				break;
			case AK_SLICE_B:
				stream->frame_type = FRAME_TYPE_B;
				break;
			case AK_SLICE_PI:
				stream->frame_type = FRAME_TYPE_PI;
				break;
			}

			/* for debug: save streams */
			venc_save_stream_to_file(handle->grp_type, pstream.pData, pstream.OutSize);
		} else {
			ak_print_error_ex(MPI_VENC "mem alloc %lubytes fail\n", pstream.OutSize);
			stream->len = 0;
			set_error_no(ERROR_TYPE_MALLOC_FAILED);
			ret = AK_FAILED;
			goto send_frame_end;
		}
	}
	stream->len = pstream.OutSize;

send_frame_end:
	if (AK_FAILED == ret) {
		if (stream->data) {
			free(stream->data);
			stream->data = NULL;
		}
	}

	return ret;
}

/*
 * ak_venc_send_frame_ex - encode single frame
 * @enc_handle[IN]: encode handle
 * @frame[IN]: frame which you want to encode
 * @frame_len[IN]: lenght of frame
 * @stream[OUT]: encode output buffer address
 * return: 0 success; -1 failed
 * note: IMPORTANT-make sure your stream->data can contain encoded data
 */
int ak_venc_send_frame_ex(void *enc_handle,
		const unsigned char *frame,
		unsigned int frame_len,
		struct video_stream *stream)
{
	if (enc_handle == NULL) {
		ak_print_error_ex(MPI_VENC "invalid enc_handle\n");
		set_error_no(ERROR_TYPE_POINTER_NULL);
		return AK_FAILED;
	}
	struct encode_group *handle = (struct encode_group *)enc_handle;
	int ret = AK_SUCCESS;
#if 1
	T_AKV_DMA_BUFFER pdmabuf = {0};

	pdmabuf.pData = (unsigned char *)frame;	//virtual address
	pdmabuf.pa    = akuio_vaddr2paddr((void *)frame);	//physical address
	pdmabuf.size  = frame_len;	//size of buffer
	pdmabuf.hHandle = NULL;		//handle for user

	T_AKV_SRC_FRAME pframe = {0};
	pframe.pdmabuf = &pdmabuf;
	pframe.mdinfo.data   = handle->mdinfo;
	pframe.mdinfo.width  = 32;
	pframe.mdinfo.height = (handle->record.height % 24) == 0 ? 24 : 16;
	pframe.srcTimeStamp = 0;	//ignore it
#else
	T_AKV_DMA_BUFFER pframe = {0};

	pframe.pData = (unsigned char *)frame;	//virtual address
	pframe.pa    = akuio_vaddr2paddr((void *)frame);	//physical address
	pframe.size  = frame_len;	//size of buffer
	pframe.hHandle = NULL;		//handle for user
	pframe.mdinfo  = handle->mdinfo;
#endif

	/* Difference with send API next line */
	T_AKV_STREAM_BUFFER pstream = {0};
	pstream.pData = stream->data;	//store output stream
	pstream.BufSize = stream->len;	//buf encbuf size

#if 0
	/* start wait-irq-thread in-needed */
	if (video_ctrl.wait_irq_thid == 0)
		start_wait_irq_thread();
#endif

	/* do encode */
	ak_thread_mutex_lock(&video_ctrl.lock);
	AKV_Encoder_Process(handle->lib_handle, &pframe, &pstream);
	ak_thread_mutex_unlock(&video_ctrl.lock);
	
	if (pstream.OutSize > 0) {
		switch (pstream.slicetype) {
			case AK_SLICE_I:
				stream->frame_type = FRAME_TYPE_I;
				break;
			case AK_SLICE_P:
				stream->frame_type = FRAME_TYPE_P;
				break;
			case AK_SLICE_B:
				stream->frame_type = FRAME_TYPE_B;
				break;
			case AK_SLICE_PI:
				stream->frame_type = FRAME_TYPE_PI;
				break;
		}

	} else {
		stream->len = 0;
		set_error_no(ERROR_TYPE_MALLOC_FAILED);
		ret = AK_FAILED;
		goto send_frame_end;
	}
	stream->len = pstream.OutSize;

send_frame_end:
	if (AK_FAILED == ret) {
		if (stream->data) {
			free(stream->data);
			stream->data = NULL;
		}
	}

	return ret;
}

/*
 * ak_venc_get_fps - get encode frames per second
 * @enc_handle[IN]: enc_handle return by 'ak_venc_open'
 * return: on success the fps, failed 0
 */
int ak_venc_get_fps(void *enc_handle)
{
	struct encode_group *handle = (struct encode_group *)enc_handle;
	if (!handle) {
		ak_print_error_ex(MPI_VENC "un init handle\n");
		set_error_no(ERROR_TYPE_POINTER_NULL);
		return 0;
	}
	T_AKVLIB_PAR encpar = {0};

	ak_thread_mutex_lock(&handle->lock);
	AKV_Encoder_Get_Parameters(handle->lib_handle, &encpar);
	ak_thread_mutex_unlock(&handle->lock);

	return encpar.EncFrameRate;
}

/*
 * ak_venc_set_fps-reset encode fps
 * @enc_handle[IN]: encode handle return by 'ak_venc_open'
 * @fps[IN]: fps you want to set
 * return: 0 on success, -1 no effect
 */
int ak_venc_set_fps(void *enc_handle, int fps)
{
	struct encode_group *handle = (struct encode_group *)enc_handle;
	if (handle == NULL) {
		ak_print_error_ex(MPI_VENC "invalid handle\n");
		set_error_no(ERROR_TYPE_POINTER_NULL);
		return -1;
	}

	ak_thread_mutex_lock(&handle->lock);
	init_frames_ctrl(handle);
	handle->ts_offset = 0;
	handle->record.fps = fps;
	handle->encode_frames = handle->record.fps;
	ak_print_notice_ex(MPI_VENC "set chn[%d] fps to : %d\n",
	    handle->grp_type, fps);

	set_encode_fps(handle, fps);
	ak_thread_mutex_unlock(&handle->lock);

	return 0;
}

/*
 * ak_venc_get_kbps - get encode kbps
 * @enc_handle[IN]:  enc_handle return by 'ak_venc_open'
 * return: >=0 kbps ;-1:failed
 */
int ak_venc_get_kbps(void *enc_handle)
{
	struct encode_group *handle = (struct encode_group *)enc_handle;
	if (!handle) {
		ak_print_error_ex(MPI_VENC "invalid handle\n");
		set_error_no(ERROR_TYPE_POINTER_NULL);
		return -1;
	}

	T_AKVLIB_PAR encpar = {0};
	ak_thread_mutex_lock(&handle->lock);
	AKV_Encoder_Get_Parameters(handle->lib_handle, &encpar);
	ak_thread_mutex_unlock(&handle->lock);

	return encpar.TargetBitRate;
}

/*
 * ak_venc_get_actual_kbps - get encode target bps and max bps
 * @enc_handle[IN]:  enc_handle return by 'ak_venc_open'
 * @target_bps[OUT]: store target bps
 * @max_bps[OUT]:    store max bps
 * return: >=0 kbps ;-1:failed
 */
int ak_venc_get_actual_kbps(void *enc_handle, int *target_bps, int *max_bps)
{
	struct encode_group *handle = (struct encode_group *)enc_handle;
	if (!handle) {
		ak_print_error_ex(MPI_VENC "invalid handle\n");
		set_error_no(ERROR_TYPE_POINTER_NULL);
		return -1;
	}
	if (!target_bps || !max_bps) {
		ak_print_error_ex(MPI_VENC "invalid handle\n");
		set_error_no(ERROR_TYPE_POINTER_NULL);
		return -1;
	}

	T_AKVLIB_PAR encpar = {0};
	ak_thread_mutex_lock(&handle->lock);
	AKV_Encoder_Get_Parameters(handle->lib_handle, &encpar);
	ak_thread_mutex_unlock(&handle->lock);

	*target_bps = encpar.TargetBitRate;
	*max_bps = encpar.MaxBitRate;

	return 0;
}

/*
 * ak_venc_set_rc - reset encode bitpersecond
 * @enc_handle[IN]: encode handle return by 'ak_venc_open'
 * @bps[IN]: bps you want to set
 * return: 0 on success, others faield or no effect
 */
int ak_venc_set_rc(void *enc_handle, int bps)
{
	struct encode_group *handle = (struct encode_group *)enc_handle;
	if (!handle) {
		ak_print_error_ex(MPI_VENC "invalid handle\n");
		set_error_no(ERROR_TYPE_POINTER_NULL);
		return -1;
	}
	T_AKVLIB_PAR encpar = {0};

	ak_thread_mutex_lock(&handle->lock);

	AKV_Encoder_Get_Parameters(handle->lib_handle, &encpar);
	encpar.TargetBitRate = bps;
	encpar.MaxBitRate    = bps;
	AKV_Encoder_Set_Parameters(handle->lib_handle, &encpar);

	ak_thread_mutex_unlock(&handle->lock);

	return 0;
}

/*
 * ak_venc_set_kbps - set encode bsp 
 * @enc_handle[IN]: encode handle return by 'ak_venc_open'
 * @target_bps[IN]: target bps you want to set
 * @max_bps[IN]: max bps you want to set
 * notes: target_bps should <= max_bps
 * return: 0 on success, others faield or no effect
 */
int ak_venc_set_kbps(void *enc_handle, int target_bps, int max_bps)
{
	struct encode_group *handle = (struct encode_group *)enc_handle;
	if (!handle) {
		ak_print_error_ex(MPI_VENC "invalid handle\n");
		set_error_no(ERROR_TYPE_POINTER_NULL);
		return AK_FAILED;
	}
 	/* notes: target_bps should <= max_bps */
	if (target_bps > max_bps) {
		ak_print_error_ex(MPI_VENC "invalid value\n");
		set_error_no(ERROR_TYPE_INVALID_ARG);
		return AK_FAILED;
	}
	ak_print_normal_ex("set encode chn %d, target bps: %d, max bsp: %d\n",
			handle->grp_type, target_bps, max_bps);

	/* get, modify, set */
	T_AKVLIB_PAR encpar = {0};
	ak_thread_mutex_lock(&handle->lock);
	/* do not change others scope */
	AKV_Encoder_Get_Parameters(handle->lib_handle, &encpar);

	encpar.TargetBitRate = target_bps;
	encpar.MaxBitRate    = max_bps;

	AKV_Encoder_Set_Parameters(handle->lib_handle, &encpar);
	ak_thread_mutex_unlock(&handle->lock);

	return AK_SUCCESS;
}

/**
 * ak_venc_set_iframe - set next encode frame to I frame
 * @enc_handle[IN], encode handle, return by 'ak_venc_open'
 * return: 0 on success, -1 failed
 */
int ak_venc_set_iframe(void *enc_handle)
{
	struct encode_group *handle = (struct encode_group *)enc_handle;
	if (handle) {
		ak_print_notice_ex(MPI_VENC "set_iframe: enter grp_type=%d handle=%p\n",
			handle->grp_type, handle);
		ak_print_notice_ex(MPI_VENC "set_iframe: locking enc_handle->lock handle=%p\n",
			handle);
		ak_thread_mutex_lock(&handle->lock);
		ak_print_notice_ex(MPI_VENC "set_iframe: enc_handle->lock acquired handle=%p\n",
			handle);
		ak_print_notice_ex(MPI_VENC "set_iframe: calling AKV_Encoder_Set_EncIFrame handle=%p\n",
			handle);
		AKV_Encoder_Set_EncIFrame(handle->lib_handle);
		ak_thread_mutex_unlock(&handle->lock);
		ak_print_notice_ex(MPI_VENC "set_iframe: enc_handle->lock released handle=%p\n",
			handle);
		return AK_SUCCESS;
	} else {
		set_error_no(ERROR_TYPE_POINTER_NULL);
		return AK_FAILED;
	}
}

/**
 * ak_venc_get_gop_len - get encode gop len
 * @enc_handle[IN]: encode handle
 * return: value of GOP len, -1 failed
 * notes:
 */
int ak_venc_get_gop_len(void *enc_handle)
{
	struct encode_group *handle = (struct encode_group *)enc_handle;
	if (!handle) {
		ak_print_error_ex(MPI_VENC "invalid handle\n");
		set_error_no(ERROR_TYPE_POINTER_NULL);
		return AK_FAILED;
	}

	T_AKVLIB_PAR encpar = {0};

	ak_thread_mutex_lock(&handle->lock);
	AKV_Encoder_Get_Parameters(handle->lib_handle, &encpar);
	ak_thread_mutex_unlock(&handle->lock);

	return encpar.GopLength;
}

/**
 * ak_venc_set_gop_len - set encode gop len
 * @enc_handle[IN]: encode handle
 * @gop_len[IN]: value of GOP len
 * return: 0 success, -1 failed
 * notes: set new gop len after you change encode fps
 */
int ak_venc_set_gop_len(void *enc_handle, int gop_len)
{
	struct encode_group *handle = (struct encode_group *)enc_handle;
	if (handle && (gop_len > 0)) {
		T_AKVLIB_PAR encpar = {0};

		ak_thread_mutex_lock(&handle->lock);

		AKV_Encoder_Get_Parameters(handle->lib_handle, &encpar);
		encpar.GopLength = gop_len;
		AKV_Encoder_Set_Parameters(handle->lib_handle, &encpar);

		ak_thread_mutex_unlock(&handle->lock);

		return AK_SUCCESS;
	} else {
		ak_print_error_ex(MPI_VENC "invalid arg: %p, %d\n", handle, gop_len);
		set_error_no(ERROR_TYPE_INVALID_ARG);
		return AK_FAILED;
	}
}

/**
 * ak_venc_set_roi - set ROI param
 * @enc_handle[IN]: encode handle
 * @roi[IN]: ROI param
 * return: 0 success, -1 failed
 * notes: call this function after ak_venc_open() or ak_venc_set_rc()
 */
int ak_venc_set_roi(void *enc_handle, struct venc_roi_param *roi)
{
	if (!enc_handle) {
		set_error_no(ERROR_TYPE_POINTER_NULL);
		return AK_FAILED;
	}
	if (!roi) {
		set_error_no(ERROR_TYPE_INVALID_ARG);
		return AK_FAILED;
	}

	return AK_SUCCESS;
}

int ak_venc_set_br(void *handle, enum bitrate_ctrl_mode mode)
{
	if (!handle) {
		ak_print_error_ex(MPI_VENC "invalid handle: %p\n", handle);
		set_error_no(ERROR_TYPE_POINTER_NULL);
		return AK_FAILED;
	}
	struct encode_group *enc_handle = (struct encode_group *)handle;
	T_AKVLIB_PAR encpar = {0};

	ak_thread_mutex_lock(&enc_handle->lock);

	enc_handle->record.br_mode = mode;

	AKV_Encoder_Get_Parameters(enc_handle->lib_handle, &encpar);
	encpar.eRCMode = (mode == BR_MODE_CBR) ? AK_RC_CBR : AK_RC_VBR;
	AKV_Encoder_Set_Parameters(enc_handle->lib_handle, &encpar);

	ak_thread_mutex_unlock(&enc_handle->lock);

	ak_print_notice_ex(MPI_VENC "set chn %d br: %s\n", enc_handle->grp_type,
			mode ? "VBR" : "CBR");

	return AK_SUCCESS;
}

/**
 * ak_venc_set_profile - set video profile, main/high/base
 * @enc_handle[IN]: encode handle
 * @profile[IN]: see define of enum profile
 * return: 0 success, -1 failed
 */
int ak_venc_set_profile(void *handle, enum profile_mode profile)
{
	if (!handle) {
		ak_print_error_ex(MPI_VENC "invalid handle: %p\n", handle);
		set_error_no(ERROR_TYPE_POINTER_NULL);
		return AK_FAILED;
	}
	struct encode_group *enc_handle = (struct encode_group *)handle;
	T_AKVLIB_PAR encpar = {0};

	ak_thread_mutex_lock(&enc_handle->lock);
	AKV_Encoder_Get_Parameters(enc_handle->lib_handle, &encpar);

	switch (profile) {
		case PROFILE_MAIN:
			encpar.eProfile = AK_PROFILE_AVC_MAIN;
			break;
		case PROFILE_HIGH:
			encpar.eProfile = AK_PROFILE_AVC_HIGH;
			break;
		case PROFILE_BASE:
			encpar.eProfile = AK_PROFILE_AVC_BASELINE;
			break;
		case PROFILE_HEVC_MAIN:
			encpar.eProfile = AK_PROFILE_HEVC_MAIN;
			break;
		case PROFILE_HEVC_MAIN_STILL:
			encpar.eProfile = AK_PROFILE_HEVC_MAIN_STILL;
			break;
		default:
			ak_print_error_ex(MPI_VENC "unsupport profile: %d\n", profile);
			break;
	}

	AKV_Encoder_Set_Parameters(enc_handle->lib_handle, &encpar);
	ak_thread_mutex_unlock(&enc_handle->lock);

	return AK_SUCCESS;
}

/**
 * ak_venc_set_rc_weight - set video rate control weight
 * @enc_handle[IN]: encode handle
 * @weight[IN]: quality weight, [0, 100]
 * return: 0 success, -1 failed
 * notes:  quality weight 0 is the best quality, 100 is the lowest bitrate.
 */
int ak_venc_set_rc_weight(void *enc_handle, int weight)
{
	if (!enc_handle) {
		ak_print_error_ex(MPI_VENC "invalid NULL handle\n");
		return AK_FAILED;
	}

	if ((weight > 100) || (weight < 0)) {
		ak_print_error_ex(MPI_VENC "invalid param, weight must in range [0, 100]\n");
		return AK_FAILED;
	}

	return AK_SUCCESS;
}

/**
 * ak_venc_set_mjpeg_qlevel - set mjpeg quality level
 * @handle[IN]: encode handle return by ak_venc_open()
 * @level[IN]:quality level, 0-9
 * return: 0 success, specifically error number
 */
int ak_venc_set_mjpeg_qlevel(void *handle, int level)
{
	if (!handle) {
		ak_print_error_ex(MPI_VENC "invalid handle: %p\n", handle);
		set_error_no(ERROR_TYPE_INVALID_USER);
		return AK_FAILED;
	}

	return AK_SUCCESS;
}

/**
 * ak_venc_set_method - set video encode method
 * @enc_handle[IN]: encode handle return by ak_venc_open()
 * @method[IN]: encode method, for more ditail see defination of
 * 				enum enc_method.
 * return: 0 success, -1 failed
 * notes:
 */
int ak_venc_set_method(void *enc_handle, enum enc_method method)
{
	if (!enc_handle) {
		ak_print_error_ex(MPI_VENC "invalid handle: %p\n", enc_handle);
		set_error_no(ERROR_TYPE_INVALID_USER);
		return AK_FAILED;
	}

	return AK_SUCCESS;
}

/**
 * ak_venc_set_smart_config - enable or disable smart function
 * @enc_handle[IN]: encode handle return by ak_venc_open()
 * @cfg[IN]: pointer to venc_smart_cfg
 * return: 0 success, -1 failed
 * notes: call after open before request stream
 */
int ak_venc_set_smart_config(void *enc_handle, struct venc_smart_cfg *cfg)
{
	struct encode_group *handle = (struct encode_group *)enc_handle;
	if (!handle || !cfg) {
		ak_print_error_ex(MPI_VENC "invalid param: %p, %p\n", handle, cfg);
		set_error_no(ERROR_TYPE_INVALID_USER);
		return AK_FAILED;
	}
#if 1
	T_AKVLIB_PAR encpar = {0};

	ak_print_normal_ex("setting smart mode=%d, gop=%d, q=%d, v=%d\n",
			cfg->smart_mode, cfg->smart_goplen, cfg->smart_quality,
			cfg->smart_static_value);
	ak_thread_mutex_lock(&handle->lock);
	AKV_Encoder_Get_Parameters(handle->lib_handle, &encpar);

	encpar.SmartMode 	  	= cfg->smart_mode;
	encpar.SmartGopLength 	= cfg->smart_goplen;
	encpar.SmartQuality   	= cfg->smart_quality;
	encpar.SmartStaticValue = cfg->smart_static_value;

	AKV_Encoder_Set_Parameters(handle->lib_handle, &encpar);

	ak_thread_mutex_unlock(&handle->lock);
#endif

	return AK_SUCCESS;
}

/**
 * ak_venc_close - close video encode
 * @enc_handle[IN]: encode handle return by ak_venc_open()
 * return: 0 success, -1 failed
 * notes: if you request a stream, you should call this function
 *        after you call 'ak_venc_cancel_stream()' success.
 */
int ak_venc_close(void *enc_handle)
{
	if (!enc_handle) {
		ak_print_error_ex(MPI_VENC "invalid handle: %p\n", enc_handle);
		set_error_no(ERROR_TYPE_POINTER_NULL);
		return AK_FAILED;
	}
	ak_thread_mutex_lock(&close_lock);
	struct encode_group *handle = (struct encode_group *)enc_handle;
	enum encode_group_type grp_type = handle->grp_type;

	ak_print_notice_ex(MPI_VENC "start close encode handle, chn: %d\n", grp_type);
	ak_thread_mutex_lock(&handle->close_mutex);
	int user_cnt = del_user(handle);
	/* user decrease to zero, close handle */
	if (user_cnt == 0) {
		ak_print_info_ex(MPI_VENC "release group %d's resource\n", grp_type);
		venc_sysipc_unbind_chn_handle(grp_type);
		encode_group_exit(handle);
	}
	ak_thread_mutex_unlock(&handle->close_mutex);

	if (user_cnt == 0) {
		ak_thread_mutex_destroy(&handle->close_mutex);

		ak_print_normal_ex(MPI_VENC "enc_handle:%p, type:%d exit OK, free handle\n",
			handle, handle->record.enc_grp);

		ak_thread_mutex_lock(&video_ctrl.lock);
		video_ctrl.grp[handle->record.enc_grp] = NULL;
		ak_thread_mutex_unlock(&video_ctrl.lock);

		free(handle);
		handle = NULL;
		ak_print_normal_ex(MPI_VENC "enc_handle free OK\n");
	}
	ak_thread_mutex_unlock(&close_lock);
	ak_print_notice_ex(MPI_VENC "close encode handle OK, chn: %d\n\n", grp_type);

	return AK_SUCCESS;
}

/**
 * ak_venc_request_stream - bind vi and venc handle start capture
 * @vi_handle[IN]: video input handle, return by ak_vi_open()
 * @enc_handle[IN]: encode handle, return by ak_venc_open()
 * return: on success return stream_handle, failed return NULL
 * notes:
 */
void *ak_venc_request_stream(void *vi_handle, void *enc_handle)
{
	if (!enc_handle || !vi_handle) {
		ak_print_error_ex(MPI_VENC "uninit handle! vi_handle=%p, enc_handle=%p\n",
			vi_handle, enc_handle);
		set_error_no(ERROR_TYPE_POINTER_NULL);
		return NULL;
	}

	struct stream_handle *new_handle = (struct stream_handle *)calloc(1,
			sizeof(struct stream_handle));
	if (!new_handle) {
		ak_print_error_ex(MPI_VENC "no mem\n");
		set_error_no(ERROR_TYPE_MALLOC_FAILED);
		return NULL;
	}

	ak_thread_mutex_lock(&cancel_lock);
	struct encode_group *handle = (struct encode_group *)enc_handle;

	/* this means start a new group encode */
	ak_thread_mutex_lock(&handle->lock);
	int i = 0;
	for (i=0; i<VENC_TYPE_MAX_USER; ++i) {
		if (!test_bit(i, &(handle->user_map))) {
			new_handle->id = i;
			break;
		}
	}
	if (i >= VENC_TYPE_MAX_USER) {
		ak_thread_mutex_unlock(&handle->lock);
		free(new_handle);
		new_handle= NULL;
		ak_thread_mutex_unlock(&cancel_lock);
		return NULL;
	}

    new_handle->vi_handle = vi_handle;
	new_handle->enc_handle = enc_handle;
    set_bit(&(handle->user_map), new_handle->id);
	add_ref(&(handle->req_ref), 1);
	ak_print_notice_ex(MPI_VENC "now user's id=%d, req_ref=%d, type=%d\n",
		new_handle->id, handle->req_ref, handle->grp_type);
	ak_print_normal_ex(MPI_VENC "vi_handle=%p, enc_handle=%p\n",
		vi_handle, enc_handle);

	/* if this is first request on current group, initialize as below */
		if (handle->is_stream_mode == 0) {
			handle->capture_frames = ak_vi_get_fps(vi_handle);
			handle->encode_frames = handle->record.fps;
		handle->ts_offset = 0;
		handle->frame_index = 0;
		handle->streams_count = 0;
		handle->is_stream_mode = 1;
			handle->stream_overflow_print_cnt = 0;

			video_ctrl.inited_enc_grp_num++;
			ak_thread_mutex_lock(&video_ctrl.cancel_mutex);
			list_add_tail(&handle->enc_list, &video_ctrl.venc_list);
			ak_thread_mutex_unlock(&video_ctrl.cancel_mutex);
			ak_print_notice_ex(MPI_VENC "init grp_type=%d, new_handle=%p, now group number=%d\n",
				handle->grp_type, new_handle, video_ctrl.inited_enc_grp_num);
		}
	ak_thread_mutex_unlock(&handle->lock);

	/* each unique video input device, create one service thread group */
	if (video_ctrl.module_init && (!video_ctrl.thread_group_num)) {
		INIT_LIST_HEAD(&video_ctrl.thread_list);
		ak_print_normal_ex(MPI_VENC "init thread list\n");
	}
	if (start_service_work(vi_handle)) {
		ak_print_error_ex(MPI_VENC
			"start_service_work failed, vi_handle=%p, grp_type=%d, req_ref=%d, inited_grp=%d\n",
			vi_handle, handle->grp_type, handle->req_ref, video_ctrl.inited_enc_grp_num);
		free(new_handle);
		new_handle= NULL;
	} else {
		ak_print_notice_ex(MPI_VENC
			"stream request ready: grp_type=%d, user_id=%d, req_ref=%d, thread_groups=%d\n",
			handle->grp_type, new_handle->id, handle->req_ref, video_ctrl.thread_group_num);
	}
	ak_thread_mutex_unlock(&cancel_lock);
	return new_handle;
}

/**
 * ak_venc_get_stream - on stream-encode, get encode output stream
 * @stream_handle[IN]: stream handle
 * @stream[IN]: stream data
 * return: 0 success, failed return specifically error
 * notes:
 */
int ak_venc_get_stream(void *stream_handle, struct video_stream *stream)
{
	/* checke user handle and buffers legitimacy */
	if (!stream_handle || !stream) {
		ak_print_error_ex(MPI_VENC "invalid stream handle\n");
		set_error_no(ERROR_TYPE_INVALID_USER);
		return AK_FAILED;
	}

	int ret = AK_FAILED;
	struct stream_handle *handle = (struct stream_handle *)stream_handle;
	struct encode_group *enc_handle = (struct encode_group *)handle->enc_handle;
	int user_id = handle->id;

	ak_thread_mutex_lock(&enc_handle->lock);
	/* check data capability */
	if (list_empty(&enc_handle->stream_list)) {
		++handle->no_data_cnt;
		if ((handle->no_data_cnt <= 3)
			|| ((handle->no_data_cnt % GET_STREAM_NO_DATA_LOG_INTERVAL) == 0)) {
			ak_print_notice_ex(MPI_VENC
				"get_stream no-data: grp_type=%d, user_id=%d, no_data_cnt=%u, streams_count=%d, req_ref=%d\n",
				enc_handle->grp_type, user_id, handle->no_data_cnt,
				enc_handle->streams_count, enc_handle->req_ref);
		}
		set_error_no(ERROR_TYPE_NO_DATA);
		goto venc_get_stream_end;
	}

	if (!test_bit(user_id, &(enc_handle->user_map))) {
	    ak_print_normal_ex(MPI_VENC "handle=%p, user_id=%d, user_map=%d\n",
	        handle, user_id, enc_handle->user_map);
	    set_error_no(ERROR_TYPE_INVALID_USER);
	    goto venc_get_stream_end;
	}

	/*
	 * to find user's stream start position
	 * if found then get stream from this position
	 */
	struct video_streams_t *pos = venc_find_user(handle);
	if (pos) {
#if VENC_GET_STREAM_DEBUG
		ak_print_normal_ex(MPI_VENC "grp_type=%d, steam handle=%p, list head=%p\n",
			enc_handle->grp_type, handle, &enc_handle->stream_list);
		ak_print_normal_ex(MPI_VENC "pos=%p, ts=%llu, len=%u, seq_no=%lu\n",
			pos, pos->stream.ts, pos->stream.len, pos->stream.seq_no);
#endif
	} else {
		set_error_no(ERROR_TYPE_INVALID_USER);
		goto venc_get_stream_end;
	}

	/*
	 * only under the status than the stream's lenght is suitable,
	 * we will copy data to user
	 */
	if ((pos->stream.len > 0) && (pos->stream.len <= ENC_OUT_MAX_SZ)) {
		/* get data, should make sure the pos is right */
		stream->data = (unsigned char *)calloc(1, pos->stream.len);
		if (NULL == stream->data) {
			ak_print_error_ex(MPI_VENC "calloc failed, stream.len=%u\n",
			    pos->stream.len);
			set_error_no(ERROR_TYPE_MALLOC_FAILED);
			goto venc_get_stream_end;
		}
	} else {
		ak_print_notice_ex(MPI_VENC "stream_handle=%p, pos=%p, "
		    "frame_type=%d, ts=%llu, len=%u\n",
			stream_handle, pos, pos->stream.frame_type,
			pos->stream.ts, pos->stream.len);
		set_error_no(ERROR_TYPE_NO_DATA);
		goto venc_get_stream_end;
	}

	if ((handle->no_data_cnt > 0)
		&& ((handle->no_data_cnt >= GET_STREAM_NO_DATA_LOG_INTERVAL)
			|| (handle->no_data_cnt >= handle->delivered_cnt))) {
		ak_print_notice_ex(MPI_VENC
			"get_stream data resumed: grp_type=%d, user_id=%d, no_data_streak=%u\n",
			enc_handle->grp_type, user_id, handle->no_data_cnt);
	}
	handle->no_data_cnt = 0;
	++handle->delivered_cnt;

	/* copy data */
	memcpy(stream->data, pos->stream.data, pos->stream.len);
	stream->len = pos->stream.len;
	stream->ts = pos->stream.ts;
	stream->seq_no = pos->stream.seq_no;
	stream->frame_type = pos->stream.frame_type;
	clear_bit(&(pos->bit_map), handle->id);

#if VENC_GET_STREAM_DEBUG
	ak_print_notice_ex(MPI_VENC "grp_type=%d, id=%d, bit_map=%d\n",
		enc_handle->grp_type, handle->id, pos->bit_map);
	ak_print_notice_ex(MPI_VENC "ref=%d, streams_count=%d\n",
		pos->ref, enc_handle->streams_count);
#endif

	/* release data */
	del_ref(&(pos->ref), 1);
	if (pos->ref <= 0) {
#if VENC_GET_STREAM_DEBUG
		ak_print_notice_ex(MPI_VENC "del grp_type=%d, id=%d, bit_map=%d\n",
			enc_handle->grp_type, handle->id, pos->bit_map);
#endif
		/* if current data idle, release it */
		enc_handle->streams_count--;
		list_del_init(&pos->list);
		free(pos->stream.data);
		pos->stream.data = NULL;
		free(pos);
		pos = NULL;
		enc_handle->stream_overflow_print_cnt = 0;
	}
	ret = AK_SUCCESS;

venc_get_stream_end:
	ak_thread_mutex_unlock(&enc_handle->lock);

	return ret;
}

/**
 * ak_venc_release_stream - release stream resource
 * @stream_handle[IN]: stream handle
 * @stream[IN]: stream return by get()
 * return: 0 success, -1 failed
 * notes:
 */
int ak_venc_release_stream(void *stream_handle, struct video_stream *stream)
{
	if (!stream) {
		ak_print_error_ex(MPI_VENC "invalid stream\n");
		set_error_no(ERROR_TYPE_POINTER_NULL);
		return ERROR_TYPE_POINTER_NULL;
	}

	if(stream->data) {
		free(stream->data);
		stream->data = NULL;
	}

	return AK_SUCCESS;
}

/**
 * ak_venc_get_rate_stat - on stream-encode, get encode rate stat info
 * @stream_handle[IN]: stream handle
 * @stat[IN]: stream rate stat info
 * return: 0 success, -1 failed
 * notes:
 */
int ak_venc_get_rate_stat(void *stream_handle, struct venc_rate_stat *stat)
{
	if (!stream_handle || !stat) {
		ak_print_error_ex(MPI_VENC "invalid stream handle\n");
		set_error_no(ERROR_TYPE_INVALID_USER);
		return AK_FAILED;
	}

	struct stream_handle *handle = (struct stream_handle *)stream_handle;
	struct encode_group *enc_handle = (struct encode_group *)handle->enc_handle;

	stat->bps = enc_handle->stat.bps;
	stat->fps = enc_handle->stat.fps;
	stat->gop = enc_handle->stat.gop_len;

    return AK_SUCCESS;
}

/**
 * ak_venc_cancel_stream - cancel stream encode
 * @stream_handle[IN]: encode handle return by ak_venc_request_stream()
 * return: 0 success, specifically error number
 * notes: 1. make sure do not call ak_venc_get_stream when call this function.
 *      2. if you got a error, you can use 'ak_get_error_no()' or
 *       'ak_get_error_str()' to get more detail.
 */
int ak_venc_cancel_stream(void *stream_handle)
{
	/* user legitimacy check */
	if (!stream_handle) {
		ak_print_error_ex(MPI_VENC "invalid handle: %p\n", stream_handle);
		set_error_no(ERROR_TYPE_INVALID_USER);
		return AK_FAILED;
	}

	struct stream_handle *handle = (struct stream_handle *)stream_handle;
	struct encode_group *enc_handle = (struct encode_group *)handle->enc_handle;
	enum encode_group_type grp_type = enc_handle->grp_type;

	ak_print_notice_ex(MPI_VENC "enter grp_type=%d, id=%d, steam handle=%p\n",
		grp_type, handle->id, stream_handle);
	/* Prevent concurrent ak_venc_close() from freeing enc_handle mid-cancel. */
	ak_print_notice_ex(MPI_VENC "cancel_stream: locking close_lock...\n");
	ak_thread_mutex_lock(&close_lock);
	ak_print_notice_ex(MPI_VENC "cancel_stream: close_lock acquired\n");
	ak_print_notice_ex(MPI_VENC "cancel_stream: locking cancel_lock...\n");
	ak_thread_mutex_lock(&cancel_lock);
	ak_print_notice_ex(MPI_VENC "cancel_stream: cancel_lock acquired\n");

	if (enc_handle) {
		/* reference count decrease */
		ak_print_notice_ex(MPI_VENC "cancel_stream: locking enc_handle->lock enc_handle=%p\n",
			enc_handle);
		ak_thread_mutex_lock(&enc_handle->lock);
		ak_print_notice_ex(MPI_VENC "cancel_stream: enc_handle->lock acquired\n");
		del_ref(&(enc_handle->req_ref), 1);
    	clear_bit(&(enc_handle->user_map), handle->id);
		ak_thread_mutex_unlock(&enc_handle->lock);
		ak_print_notice_ex(MPI_VENC "cancel_stream: enc_handle->lock released\n");

		ak_print_notice_ex(MPI_VENC "req_ref=%d, user_map=%d, is_stream_mode=%d\n",
			enc_handle->req_ref, enc_handle->user_map, enc_handle->is_stream_mode);
		ak_print_notice_ex(MPI_VENC "cancel_stream: idle_check req_ref=%d is_stream_mode=%d\n",
			enc_handle->req_ref, enc_handle->is_stream_mode);

		/* if resource idle, release it */
		if ((enc_handle->req_ref <= 0) && enc_handle->is_stream_mode) {
			/*
			 * Avoid blocking on video_ctrl.lock here.
			 *
			 * In failure cases the encode path may be stuck inside
			 * AKV_Encoder_Process() while holding video_ctrl.lock.
			 * Waiting on that lock in cancel path can wedge teardown.
			 *
			 * Keep this best-effort and lock-free here. cancel_lock serializes
			 * concurrent cancel calls, and this counter only gates thread exit.
			 */
			{
				int prev = video_ctrl.inited_enc_grp_num;
				int now = prev;
				ak_print_notice_ex(MPI_VENC
					"cancel_stream: decrementing inited_enc_grp_num (best-effort) prev=%d\n",
					prev);
				if (prev > 0) {
					now = prev - 1;
				} else {
					now = 0;
				}
				video_ctrl.inited_enc_grp_num = now;
				ak_print_notice_ex(MPI_VENC
					"cancel_stream: inited_enc_grp_num %d -> %d\n",
					prev, now);
			}

			/* take off encode-handle from encode-handle-list first */
			ak_print_notice_ex(MPI_VENC "cancel_stream: locking video_ctrl.cancel_mutex\n");
			ak_thread_mutex_lock(&video_ctrl.cancel_mutex);
			ak_print_notice_ex(MPI_VENC "cancel_stream: video_ctrl.cancel_mutex acquired\n");
			list_del_init(&enc_handle->enc_list);
			ak_thread_mutex_unlock(&video_ctrl.cancel_mutex);
			ak_print_notice_ex(MPI_VENC "cancel_stream: video_ctrl.cancel_mutex released\n");

			/* clear stream mode flag */
			ak_print_notice_ex(MPI_VENC "cancel_stream: clearing is_stream_mode\n");
			ak_thread_mutex_lock(&enc_handle->lock);
			enc_handle->is_stream_mode = 0;
			ak_thread_mutex_unlock(&enc_handle->lock);

			/* release all un-taken-of streams on current group */
			ak_print_notice_ex(MPI_VENC "cancel_stream: release_group_streams begin enc_handle=%p\n",
				enc_handle);
			release_group_streams(enc_handle);
			ak_print_notice_ex(MPI_VENC "cancel_stream: release_group_streams done\n");

			/* release capture and encode thread resource (last group only) */
			ak_print_notice_ex(MPI_VENC "cancel_stream: encode_thread_group_exit begin\n");
			encode_thread_group_exit();
			ak_print_notice_ex(MPI_VENC "cancel_stream: encode_thread_group_exit done\n");
			ak_print_normal_ex(MPI_VENC "release stream resource ok\n");
		}
	}

	/* release user's stream handle */
	ak_print_notice_ex(MPI_VENC
		"cancel stream stats: grp_type=%d, user_id=%d, delivered=%u, no_data_cnt=%u\n",
		grp_type, handle->id, handle->delivered_cnt, handle->no_data_cnt);
	free(handle);
	handle = NULL;
	ak_print_notice_ex(MPI_VENC "cancel_stream: unlocking cancel_lock\n");
	ak_thread_mutex_unlock(&cancel_lock);
	ak_print_notice_ex(MPI_VENC "cancel_stream: unlocking close_lock\n");
	ak_thread_mutex_unlock(&close_lock);

	ak_print_notice_ex(MPI_VENC "grp_type=%d, leave...\n\n", grp_type);

	return AK_SUCCESS;
}
