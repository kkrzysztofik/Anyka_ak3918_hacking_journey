#include <pthread.h>
#include <stdio.h>
#include <sys/stat.h>
#include <errno.h>
#include <fcntl.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

#include "sound.h"
#include "log.h"
#include "ak_global.h"                    /* AUDIO_FUNC_ENABLE / _DISABLE */
#include "ak_ao.h"

/* One DAC, one worker. `playing` is guarded by `lock` rather than atomics
 * because the busy check and the thread spawn must be one critical section —
 * two racing CMD_AUDIO_PLAY calls must not both win. */
static pthread_mutex_t lock = PTHREAD_MUTEX_INITIALIZER;
static int             playing;
static struct sound_req current;

#define SOUND_CHUNK_BYTES   2048
/* A wedged play must not hold the DAC forever and block every later sound.
 * Clips are chimes; anything past this is a fault, not a long file. */
#define SOUND_MAX_MS        30000

/* Amplifier shutdown pin, and it is ACTIVE HIGH — the name reads like an
 * enable, but it is not.
 *
 * Measured on .198 at the 8002D (LM4871-class, BTL) on 2026-08-29:
 *   SPK_PA = 1  ->  outputs 0 V     (shutdown)
 *   SPK_PA = 0  ->  outputs VDD/2   (enabled, 2.5 V on a 5 V rail)
 *
 * The pin is low at boot and ak_ao_enable_speaker() never touches it, so the
 * stock state is "enabled". An earlier revision wrote 1 here believing it was
 * an enable, which silently shut the amplifier off before every play: the SDK
 * still reported success end to end, so it looked like dead hardware.
 *
 * We only ever drive it low, and never restore it: leaving the amp enabled is
 * the boot state, and setting it high again would make the next play silent
 * with no diagnostic. */
#define SPK_PA_SYSFS        "/sys/user-gpio/SPK_PA"

/* Returns 0 on success, -1 if the amplifier could not be enabled.
 *
 * The caller must abort on failure. Playing into a disabled amplifier is
 * silent but reports success at every layer -- ak_ao accepts the frames, the
 * drain reaches FINISHED, and the worker logs event=sound_played -- which is
 * exactly the blind spot that made the SPK_PA polarity bug take a day and a
 * teardown to find. Do not let it fail quietly a second time. */
static int spk_amp_enable(void)
{
    /* open()+write() rather than stdio: O_WRONLY without O_CREAT makes it
     * impossible to create a file here (fopen("w") implies O_CREAT 0666, which
     * CodeQL flags), and a sysfs attribute wants exactly one write() -- stdio
     * would let a later edit split the value across two syscalls, which the
     * kernel handler parses independently. */
    int fd = open(SPK_PA_SYSFS, O_WRONLY);
    if (fd < 0) {
        log_error("[sound] cannot open %s: %s", SPK_PA_SYSFS, strerror(errno));
        return -1;
    }
    if (write(fd, "0", 1) != 1) {
        log_error("[sound] cannot enable amplifier via %s: %s",
                  SPK_PA_SYSFS, strerror(errno));
        close(fd);
        return -1;
    }
    close(fd);
    return 0;
}

static long elapsed_ms(const struct timespec *start)
{
    struct timespec now;
    clock_gettime(CLOCK_MONOTONIC, &now);
    return (now.tv_sec - start->tv_sec) * 1000
         + (now.tv_nsec - start->tv_nsec) / 1000000;
}

static void *sound_thread(void *arg)
{
    (void)arg;
    struct timespec start;
    clock_gettime(CLOCK_MONOTONIC, &start);

    {
        struct stat st;
        if (stat(current.path, &st) != 0 || !S_ISREG(st.st_mode)) {
            log_warn("[sound] refusing non-regular path %s", current.path);
            goto done;
        }
    }

    /* current.path arrives over IPC, so CodeQL flags it as uncontrolled input.
     * It is constrained before reaching here: sound_parse_play_req() requires
     * an absolute path, rejects any "/../" or trailing "/..", and requires the
     * SOUND_PATH_PREFIX ("/mnt/anyka_hack/") prefix; the stat() above rejects
     * anything that is not a regular file. Opened read-only, and the residual
     * exposure -- a caller who already has the admin IPC socket can have any
     * regular file under that prefix rendered as PCM -- is bounded by the
     * prefix and accepted. */
    FILE *fp = fopen(current.path, "rb");
    if (fp == NULL) {
        log_warn("[sound] cannot open %s", current.path);
        goto done;
    }

    struct pcm_param param;
    param.sample_rate = current.sample_rate;
    param.sample_bits = 16;               /* s16le; the only format we ship */
    param.channel_num = current.channel_num;

    void *ao = ak_ao_open(&param);
    if (ao == NULL) {
        log_error("[sound] ak_ao_open failed rate=%u ch=%u",
                  param.sample_rate, param.channel_num);
        fclose(fp);
        goto done;
    }

    /* Order and necessity per ak_ao_demo.c:149-159; the header marks speaker,
     * resample and clear_frame_buffer mandatory before the first send_frame.
     * set_resample is inert in the vendored source (ak_ao.c:1412-1424 has its
     * body commented out and returns AK_SUCCESS) — kept for demo fidelity and
     * because the shipped lib may differ, but do not chase a resample bug here.
     *
     * No ak_ao_set_aslc_volume: ak_ao_demo.c never calls it, and dac_set_aslc
     * (ak_ao.c:373-390) opens the filter before it looks at the value, so even
     * passing 0 forces an sdfilter_open that fails on this chip with
     * "CHIP(14) unsupported". Harmless — the write path skips a NULL aslc
     * (ak_ao.c:566) and it only applies to volume 7-12 anyway — but it dumped
     * a spurious error into every playback log. */
    ak_ao_enable_speaker(ao, AUDIO_FUNC_ENABLE);
    ak_ao_set_dac_volume(ao, current.volume);
    ak_ao_set_resample(ao, AUDIO_FUNC_DISABLE);
    ak_ao_clear_frame_buffer(ao);

    /* The DA is stereo-only: handing it a mono buffer makes each channel take
     * every other sample, so a clip plays at half length and double pitch. We
     * duplicate each sample into L/R before sending, as ak_ao_demo.c does for a
     * mono source. `sent` therefore reaches 2x the file size — DA-side stereo
     * bytes, not a leak. */
    unsigned char buf[SOUND_CHUNK_BYTES];
    unsigned char stereo[SOUND_CHUNK_BYTES * 2];
    size_t n;
    unsigned long long sent = 0;

    /* Abort rather than pump PCM into a silent output stage: every layer above
     * the amplifier would still report success. */
    if (spk_amp_enable() != 0) {
        ak_ao_enable_speaker(ao, AUDIO_FUNC_DISABLE);
        ak_ao_close(ao);
        fclose(fp);
        goto done;
    }

    while ((n = fread(buf, 1, sizeof(buf), fp)) > 0) {
        if (elapsed_ms(&start) > SOUND_MAX_MS) {
            log_warn("[sound] watchdog: aborting %s after %d ms",
                     current.path, SOUND_MAX_MS);
            break;
        }
        /* Not 2 * n: a trailing odd byte has no whole sample and is dropped. */
        int stereo_len = sound_dup_mono_to_stereo(buf, (int)n, stereo);

        /* One call per chunk is complete. ak_ao_send_frame returns AK_SUCCESS
         * (0), NOT a byte count — the header's "real sent data len" is wrong:
         * write_dac_driver's count is folded into da_data_size and the return
         * is overwritten with AK_SUCCESS (ak_ao.c:713-725). It loops write()
         * internally until the whole buffer is consumed and ignores `ms`, so
         * retrying on a 0 return would resend the same chunk forever. */
        if (ak_ao_send_frame(ao, stereo, stereo_len, 0) < 0) {
            log_warn("[sound] send_frame failed after %llu bytes", sent);
            break;
        }
        sent += (unsigned long long)stereo_len;
    }

    ak_ao_notice_frame_end(ao);

    /* Wait for the DA to actually play what we queued: send_frame only means
     * "handed to the driver", so closing here truncates the tail.
     *
     * Only if something was queued. FINISHED is reachable only from PLAYING,
     * which the driver enters only once it reports bytes in flight
     * (ak_ao.c:1062-1083); with sent == 0 — empty clip, or a first send that
     * failed — the handle sits in DATA_NOT_ENOUGH forever and this would spin
     * the full watchdog holding the DAC and SPK_PA, dropping every request in
     * that window. ak_ao_demo.c:222 guards on read_len instead, which still
     * hangs (unbounded!) on an empty file. */
    while (sent > 0 && ak_ao_get_play_status(ao) != AO_PLAY_STATUS_FINISHED) {
        if (elapsed_ms(&start) > SOUND_MAX_MS) {
            log_warn("[sound] watchdog: drain timed out for %s", current.path);
            break;
        }
        struct timespec ts = { .tv_sec = 0, .tv_nsec = 10 * 1000 * 1000 };
        nanosleep(&ts, NULL);
    }

    ak_ao_enable_speaker(ao, AUDIO_FUNC_DISABLE);
    ak_ao_close(ao);
    fclose(fp);
    log_info("event=sound_played path=%s bytes=%llu volume=%d",
             current.path, sent, current.volume);

done:
    pthread_mutex_lock(&lock);
    playing = 0;
    pthread_mutex_unlock(&lock);
    return NULL;
}

int sound_play_async(const struct sound_req *req)
{
    pthread_mutex_lock(&lock);
    if (playing) {
        pthread_mutex_unlock(&lock);
        return 1;                       /* busy */
    }
    current = *req;
    playing = 1;
    pthread_mutex_unlock(&lock);

    pthread_t tid;
    if (pthread_create(&tid, NULL, sound_thread, NULL) != 0) {
        pthread_mutex_lock(&lock);
        playing = 0;
        pthread_mutex_unlock(&lock);
        log_error("[sound] pthread_create failed");
        return -1;
    }
    pthread_detach(tid);
    return 0;
}
