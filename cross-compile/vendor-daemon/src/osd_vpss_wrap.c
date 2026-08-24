#define _GNU_SOURCE
/*
 * Interpose ak_vpss_osd_set_param so libmpi_osd's MEM_ATTR payload matches
 * what this camera's ISP actually reads.
 *
 * libmpi_osd writes isp_osd_mem_attr as (chn, dma_paddr, size) into
 * vpss_osd_param.data.  On .198 the ISP path effectively treats the start of
 * AK_ISP_USER_PARAM as the mem-attr (id takes the place of chn), so it sees:
 *   paddr = data[0] = our chn (0 → null)
 *   size  = data[4] = our dma_paddr (dmesg: size:-2134570752 == 0x80c50900)
 *
 * Rewrite data to (dma_paddr, size) before the real call.  MAIN vs SUB is
 * already selected by param->id; the rect index is not recoverable from the
 * CID, but Stage B / our handlers only use rect 0 on first bring-up.
 */
#include <string.h>
#include <stdint.h>

#include "ak_vpss.h"
#include "log.h"

/* Real symbol from libplat_vpss.so — linked normally; we wrap it by providing
 * a global with the same name that the dynamic loader binds first for the
 * executable, and call through an asm alias to the shared-library version.
 *
 * Simpler approach that avoids alias games: keep the real name for the wrap
 * and resolve the library symbol with a constructor + dlsym(RTLD_NEXT). */
#include <dlfcn.h>

typedef int (*ak_vpss_osd_set_param_fn)(const void *, struct vpss_osd_param *);

static ak_vpss_osd_set_param_fn real_set_param;

static void resolve_real_set_param(void) __attribute__((constructor));
static void resolve_real_set_param(void)
{
    real_set_param = (ak_vpss_osd_set_param_fn)dlsym(RTLD_NEXT,
                                                     "ak_vpss_osd_set_param");
}

int ak_vpss_osd_set_param(const void *vi_handle, struct vpss_osd_param *param)
{
    if (!real_set_param)
        resolve_real_set_param();
    if (!real_set_param)
        return -1;

    if (param &&
        (param->id == OSD_SET_MAIN_DMA_MEM_REQUST ||
         param->id == OSD_SET_SUB_DMA_MEM_REQUST)) {
        /* libmpi: [i32 chn][ptr paddr][u32 size] */
        int32_t chn;
        uintptr_t paddr;
        uint32_t size;
        memcpy(&chn, param->data, sizeof(chn));
        memcpy(&paddr, param->data + 4, sizeof(paddr));
        memcpy(&size, param->data + 8, sizeof(size));

        /* Device: ISP reads USER_PARAM as mem-attr from byte 0, so id occupies
         * the chn slot and data[0] must be paddr. */
        memset(param->data, 0, sizeof(param->data));
        memcpy(param->data, &paddr, sizeof(paddr));
        memcpy(param->data + 4, &size, sizeof(size));
        log_info("[osd] mem_attr rewrite chn=%d paddr=%p size=%u",
                 (int)chn, (void *)paddr, size);
    } else if (param &&
               (param->id == OSD_SET_MAIN_CHANNEL_DATA ||
                param->id == OSD_SET_SUB_CHANNEL_DATA)) {
        /* libmpi isp_osd_context_attr:
         * [i32 chn][ptr addr][u32 w][u32 h][u16 x][u16 y][u16 alpha][u16 en]
         * Same byte-0 shift as mem_attr — drop leading chn. */
        int32_t chn;
        uintptr_t addr;
        uint32_t w, h;
        uint16_t x, y, alpha, en;
        memcpy(&chn, param->data, 4);
        memcpy(&addr, param->data + 4, 4);
        memcpy(&w, param->data + 8, 4);
        memcpy(&h, param->data + 12, 4);
        memcpy(&x, param->data + 16, 2);
        memcpy(&y, param->data + 18, 2);
        memcpy(&alpha, param->data + 20, 2);
        memcpy(&en, param->data + 22, 2);

        memset(param->data, 0, sizeof(param->data));
        memcpy(param->data + 0, &addr, 4);
        memcpy(param->data + 4, &w, 4);
        memcpy(param->data + 8, &h, 4);
        memcpy(param->data + 12, &x, 2);
        memcpy(param->data + 14, &y, 2);
        memcpy(param->data + 16, &alpha, 2);
        memcpy(param->data + 18, &en, 2);
        log_info("[osd] ctx_attr rewrite chn=%d addr=%p %ux%u @%u,%u a=%u en=%u",
                 (int)chn, (void *)addr, w, h, x, y, alpha, en);
    }

    return real_set_param(vi_handle, param);
}
