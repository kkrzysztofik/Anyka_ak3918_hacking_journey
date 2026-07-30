#include "globals.h"
#include "log.h"

/* SDK close entry points used by the session-object sweep. */
#include "ak_vi.h"
#include "ak_venc.h"
#include "ak_ai.h"
#include "ak_aenc.h"

volatile sig_atomic_t g_shutdown = 0;

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

/* Built empty at daemon start, so it can never hold an object from a previous
 * generation. */
struct vd_obj_slot g_obj_slots[VD_OBJ_SLOTS];

int vd_obj_register(uint8_t kind, void *ptr)
{
    int i;

    if (ptr == NULL || kind == VD_OBJ_KIND_NONE)
        return -1;

    for (i = 0; i < VD_OBJ_SLOTS; i++) {
        if (g_obj_slots[i].live)
            continue;
        /* Bump on allocation, not on release: a token from the slot's previous
         * occupant must stop matching the moment the slot is reused, even
         * within the same daemon generation. */
        g_obj_slots[i].slot_gen++;
        g_obj_slots[i].ptr  = ptr;
        g_obj_slots[i].kind = kind;
        g_obj_slots[i].live = 1;
        return i;
    }

    log_error("[obj] registry full (%d slots); leaking kind=%u ptr=%p",
              VD_OBJ_SLOTS, (unsigned)kind, ptr);
    return -1;
}

void vd_obj_unregister(uint8_t kind, void *ptr)
{
    int i;

    for (i = 0; i < VD_OBJ_SLOTS; i++) {
        if (g_obj_slots[i].live &&
            g_obj_slots[i].kind == kind &&
            g_obj_slots[i].ptr == ptr) {
            g_obj_slots[i].live = 0;
            g_obj_slots[i].ptr  = NULL;
            return;
        }
    }
}

/* Close order: streams depend on encoders, encoders on inputs. Tearing down in
 * the other direction hands the SDK a closed input under a running encoder. */
static const uint8_t g_obj_close_order[] = {
    VD_OBJ_KIND_STREAM,
    VD_OBJ_KIND_VENC,
    VD_OBJ_KIND_AENC,
    VD_OBJ_KIND_VI,
    VD_OBJ_KIND_AI,
};

static void vd_obj_close_one(uint8_t kind, void *ptr)
{
    int ret = 0;

    switch (kind) {
    case VD_OBJ_KIND_STREAM:
        /* Not ak_venc_cancel_stream(): that call can block indefinitely on a
         * wedged encoder, and this runs on the accept loop. stop_push_slot()
         * already cancels the streams the push threads own, which is every
         * stream this daemon hands out. */
        return;
    case VD_OBJ_KIND_VENC:
        ret = ak_venc_close(ptr);
        break;
    case VD_OBJ_KIND_AENC:
        ret = ak_aenc_close(ptr);
        break;
    case VD_OBJ_KIND_VI:
        ret = ak_vi_close(ptr);
        break;
    case VD_OBJ_KIND_AI:
        ret = ak_ai_close(ptr);
        break;
    default:
        return;
    }

    if (ret != 0)
        log_warn("[obj] close kind=%u ptr=%p returned %d (continuing)",
                 (unsigned)kind, ptr, ret);
    else
        log_info("[obj] closed leaked kind=%u ptr=%p", (unsigned)kind, ptr);
}

void vd_obj_close_all(void)
{
    size_t k;
    int i;

    for (k = 0; k < sizeof(g_obj_close_order) / sizeof(g_obj_close_order[0]); k++) {
        uint8_t kind = g_obj_close_order[k];
        for (i = 0; i < VD_OBJ_SLOTS; i++) {
            if (!g_obj_slots[i].live || g_obj_slots[i].kind != kind)
                continue;
            vd_obj_close_one(kind, g_obj_slots[i].ptr);
            g_obj_slots[i].live = 0;
            g_obj_slots[i].ptr  = NULL;
        }
    }

    /* Anything left is a kind with no close path (streams); drop it so the
     * next session starts from an empty table regardless. */
    for (i = 0; i < VD_OBJ_SLOTS; i++) {
        g_obj_slots[i].live = 0;
        g_obj_slots[i].ptr  = NULL;
    }
}
