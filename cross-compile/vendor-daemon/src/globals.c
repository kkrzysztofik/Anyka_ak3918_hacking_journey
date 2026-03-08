#include "globals.h"

volatile int g_shutdown = 0;

int g_control_fd = -1;

void *g_ring_buffer = NULL;

int g_ctrl_server_fd      = -1;
int g_frame_main_server_fd = -1;
int g_frame_sub_server_fd  = -1;
int g_frame_main_client_fd = -1;
int g_frame_sub_client_fd  = -1;

pthread_mutex_t g_frame_main_client_lock = PTHREAD_MUTEX_INITIALIZER;
pthread_mutex_t g_frame_sub_client_lock  = PTHREAD_MUTEX_INITIALIZER;

int g_saved_stdout = -1;
int g_saved_stderr = -1;
FILE *g_log_fp = NULL;

/* Zero-initialized at start; main() resets with memset() before use. */
struct push_stream_state g_push_streams[PUSH_STREAM_SLOT_COUNT];
