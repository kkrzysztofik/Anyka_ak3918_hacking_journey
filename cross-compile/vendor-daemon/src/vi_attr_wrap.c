#define _GNU_SOURCE
/*
 * Interpose ak_vi_get_channel_attr so callers (notably libmpi_osd) see the
 * real sub-channel resolution.
 *
 * On this camera's libplat_vi, ak_vi_set_channel_attr stores the sub frame
 * size in MAIN.max_width/max_height (libre_anyka_app quirk) and leaves
 * chn_sub at 0x0.  ISP still scales the sub stream correctly, but OSD's
 * get_resolution / get_max_rect read chn_sub and then refuse set_rect.
 *
 * When readback shows sub width/height <= 0 and main.max looks like a
 * plausible sub size, copy main.max into res[SUB] before returning.
 */
#include <dlfcn.h>
#include <string.h>

#include "ak_vi.h"
#include "log.h"

typedef int (*ak_vi_get_channel_attr_fn)(void *, struct video_channel_attr *);

static ak_vi_get_channel_attr_fn real_get_channel_attr;

static void resolve_real_get_channel_attr(void) __attribute__((constructor));
static void resolve_real_get_channel_attr(void)
{
    real_get_channel_attr = (ak_vi_get_channel_attr_fn)dlsym(
        RTLD_NEXT, "ak_vi_get_channel_attr");
}

int ak_vi_get_channel_attr(void *handle, struct video_channel_attr *attr)
{
    int ret;

    if (!real_get_channel_attr)
        resolve_real_get_channel_attr();
    if (!real_get_channel_attr)
        return -1;

    ret = real_get_channel_attr(handle, attr);
    if (ret != 0 || attr == NULL)
        return ret;

    /* This libplat_vi leaves res[SUB] untouched (zeros if caller memset,
     * stack garbage if not).  Quirk stores the real sub size in main.max. */
    if (attr->res[VIDEO_CHN_MAIN].max_width > 0 &&
        attr->res[VIDEO_CHN_MAIN].max_height > 0 &&
        attr->res[VIDEO_CHN_MAIN].max_width <=
            attr->res[VIDEO_CHN_MAIN].width &&
        attr->res[VIDEO_CHN_MAIN].max_height <=
            attr->res[VIDEO_CHN_MAIN].height &&
        (attr->res[VIDEO_CHN_SUB].width !=
             attr->res[VIDEO_CHN_MAIN].max_width ||
         attr->res[VIDEO_CHN_SUB].height !=
             attr->res[VIDEO_CHN_MAIN].max_height)) {
        attr->res[VIDEO_CHN_SUB].width = attr->res[VIDEO_CHN_MAIN].max_width;
        attr->res[VIDEO_CHN_SUB].height = attr->res[VIDEO_CHN_MAIN].max_height;
        attr->res[VIDEO_CHN_SUB].max_width =
            attr->res[VIDEO_CHN_MAIN].max_width;
        attr->res[VIDEO_CHN_SUB].max_height =
            attr->res[VIDEO_CHN_MAIN].max_height;
        {
            static int logged;
            if (!logged) {
                logged = 1;
                log_info("[vi] get_channel_attr: synthesized sub=%dx%d from main.max",
                         attr->res[VIDEO_CHN_SUB].width,
                         attr->res[VIDEO_CHN_SUB].height);
            }
        }
    }

    return ret;
}
