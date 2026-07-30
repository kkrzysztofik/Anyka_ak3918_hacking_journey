#include "globals.h"
#include "log.h"

/* SDK close entry points used by the session-object sweep. */
#include "ak_vi.h"
#include "ak_venc.h"
#include "ak_ai.h"
#include "ak_aenc.h"

/* Ring header, for the daemon epoch baked into handle tokens. */
#include "vd_ring_buffer.h"

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

/* Low 16 bits of this daemon generation's epoch, or 0 if the ring is absent.
 * Truncation is forced by the 32-bit token layout; see globals.h. It weakens
 * the daemon-side collision margin to 2^-16, which is acceptable because this
 * check is defence in depth -- the client's epoch gate still compares the full
 * 32-bit value before anything is sent. */
static uint16_t vd_daemon_epoch16(void)
{
    if (g_ring_buffer == NULL)
        return 0;
    return (uint16_t)(vd_ring_get_header(g_ring_buffer)->epoch & VD_TOK_EPOCH_MASK);
}

uint64_t vd_obj_token(int slot)
{
    uint32_t token;

    if (slot < 0 || slot >= VD_OBJ_SLOTS || !g_obj_slots[slot].live)
        return 0;

    token  = (uint32_t)(g_obj_slots[slot].kind & VD_TOK_KIND_MASK);
    token |= ((uint32_t)slot & VD_TOK_INDEX_MASK) << VD_TOK_INDEX_SHIFT;
    token |= ((uint32_t)g_obj_slots[slot].slot_gen & VD_TOK_GEN_MASK) << VD_TOK_GEN_SHIFT;
    token |= ((uint32_t)vd_daemon_epoch16()) << VD_TOK_EPOCH_SHIFT;
    return (uint64_t)token;
}

int vd_obj_resolve(uint64_t token, uint8_t expect_kind, void **out_ptr)
{
    uint32_t t = (uint32_t)token;
    uint8_t  kind;
    uint32_t index;
    uint16_t gen;
    uint16_t epoch;

    if (g_ring_buffer == NULL) {
        log_warn("[obj] resolve with no ring; rejecting token 0x%08x", t);
        return -1;
    }

    kind  = (uint8_t)(t & VD_TOK_KIND_MASK);
    index = (t >> VD_TOK_INDEX_SHIFT) & VD_TOK_INDEX_MASK;
    gen   = (uint16_t)((t >> VD_TOK_GEN_SHIFT) & VD_TOK_GEN_MASK);
    epoch = (uint16_t)((t >> VD_TOK_EPOCH_SHIFT) & VD_TOK_EPOCH_MASK);

    if (kind != expect_kind) {
        log_warn("[obj] token 0x%08x kind=%u but command expects %u",
                 t, (unsigned)kind, (unsigned)expect_kind);
        return -1;
    }
    if (epoch != vd_daemon_epoch16()) {
        log_warn("[obj] token 0x%08x from epoch %u, current %u (stale)",
                 t, (unsigned)epoch, (unsigned)vd_daemon_epoch16());
        return -1;
    }
    if (index >= VD_OBJ_SLOTS || !g_obj_slots[index].live) {
        log_warn("[obj] token 0x%08x names slot %u which is not live", t, index);
        return -1;
    }
    if ((g_obj_slots[index].slot_gen & VD_TOK_GEN_MASK) != gen) {
        log_warn("[obj] token 0x%08x slot_gen %u but slot %u is at %u (reused)",
                 t, (unsigned)gen, index,
                 (unsigned)(g_obj_slots[index].slot_gen & VD_TOK_GEN_MASK));
        return -1;
    }

    *out_ptr = g_obj_slots[index].ptr;
    return 0;
}

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
