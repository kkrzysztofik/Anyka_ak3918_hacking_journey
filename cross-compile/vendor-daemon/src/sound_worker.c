#include <pthread.h>
#include <stdio.h>
#include <sys/stat.h>
#include <errno.h>
#include <string.h>
#include <time.h>

#include "sound.h"
#include "log.h"
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

    /* The vendor demo pins this to 6 (max) plus 2 of ASLC gain, which is why
     * playback was deafening. We honour the configured level and leave ASLC off. */
    ak_ao_set_dac_volume(ao, current.volume);
    ak_ao_set_aslc_volume(ao, 0);

    unsigned char buf[SOUND_CHUNK_BYTES];
    size_t n;
    unsigned long long sent = 0;
    while ((n = fread(buf, 1, sizeof(buf), fp)) > 0) {
        if (elapsed_ms(&start) > SOUND_MAX_MS) {
            log_warn("[sound] watchdog: aborting %s after %d ms",
                     current.path, SOUND_MAX_MS);
            break;
        }
        {
            size_t off = 0;
            int send_failed = 0;
            while (off < n) {
                if (elapsed_ms(&start) > SOUND_MAX_MS) {
                    log_warn("[sound] watchdog: aborting %s after %d ms",
                             current.path, SOUND_MAX_MS);
                    send_failed = 1;
                    break;
                }
                int rc = ak_ao_send_frame(ao, buf + off, (int)(n - off), 0);
                if (rc < 0) {
                    log_warn("[sound] send_frame failed after %llu bytes", sent);
                    send_failed = 1;
                    break;
                }
                if (rc == 0) {
                    /* Non-blocking: wait briefly rather than spin. */
                    struct timespec ts = { .tv_sec = 0, .tv_nsec = 5 * 1000 * 1000 };
                    nanosleep(&ts, NULL);
                    continue;
                }
                off += (size_t)rc;
                sent += (unsigned long long)rc;
            }
            if (send_failed)
                break;
            if (off < n)
                break;
        }
    }

    ak_ao_notice_frame_end(ao);
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

int sound_is_playing(void)
{
    pthread_mutex_lock(&lock);
    int p = playing;
    pthread_mutex_unlock(&lock);
    return p;
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
