---
name: vendor-daemon-ipc
description: Use when writing or debugging the vendor-daemon C bridge — IPC wire format, control and frame sockets, the shared-memory frame ring, poll() multiplexing, Anyka SDK calls, and ARMv5TE/uClibc cross-compilation.
version: 1.0.0
---

# vendor-daemon IPC Bridge (C)

`vendor-daemon` is the C process between `onvif-rust` and the proprietary Anyka
SDK. It listens on Unix domain sockets for binary IPC commands, dispatches them
to the SDK (`ak_vi`, `ak_vpss`, `ak_venc`, `ak_ai`, `ak_aenc`), and delivers
H.264 frames through a shared-memory ring plus frame-socket notifications.

Runs on ARMv5TE with uClibc under a 24 MB system memory budget.

## Source layout

```
cross-compile/vendor-daemon/
├── Makefile
├── src/
│   ├── main.c              # entry point, poll() multiplexer
│   ├── dispatcher.c/.h     # command dispatch
│   ├── ipc.c/.h            # socket read/write, send_response()
│   ├── protocol.h          # command ids and payload layouts
│   ├── push.c/.h           # frame push into the ring + notifications
│   ├── handlers_vi.c       # video input
│   ├── handlers_venc.c     # encoder
│   ├── handlers_isp.c      # ISP / image tuning
│   ├── handlers_audio.c    # audio in/encode
│   ├── handlers_osd.c      # OSD
│   ├── sound.c, sound_worker.c
│   ├── globals.c/.h, log.c/.h
│   └── vi_attr_wrap.c, osd_vpss_wrap.c, osd_ipcsrv_stubs.c
├── tests/                  # host-side unit tests (see Testing)
├── include/                # Anyka SDK headers + vd_ring_buffer.h — DO NOT MODIFY
└── lib/                    # Anyka SDK .so files — DO NOT MODIFY
```

## IPC protocol

### Wire format (little-endian)

```
Request:   [i32 cmd_id][u32 req_len][req_data : req_len bytes]
Response:  [i32 status][u32 resp_len][resp_data : resp_len bytes]
```

Responses are written by `send_response(fd, status, data, len)` in `ipc.h`.

### Socket endpoints

| Socket | Purpose |
|---|---|
| `/tmp/vd-ctrl.sock` | control commands (lifecycle, config, queries) |
| `/tmp/vd-frame-main.sock` | main stream frame notifications |
| `/tmp/vd-frame-sub.sock` | sub stream frame notifications |

### Frame notification — 20 bytes, packed

From `include/vd_ring_buffer.h`, size enforced by `_Static_assert`:

```c
struct vd_frame_notify {
    uint32_t slot_index;   /* which ring slot holds the frame */
    uint32_t frame_len;    /* frame data length */
    uint32_t flags;        /* VD_NOTIFY_* bits */
    uint32_t stream_id;    /* stream encoded into the slot */
    uint32_t seq_no;       /* frame sequence number */
} __attribute__((packed));
```

Flags: `VD_NOTIFY_LAST_FRAGMENT` (1<<0), `VD_NOTIFY_SOCKET_FALLBACK` (1<<1),
`VD_NOTIFY_FRAME_DROPPED` (1<<2).

### Shared-memory frame ring

| Constant | Value |
|---|---|
| `VD_SHM_PATH` | `/tmp/vendor-frame-ring.shm` |
| `VD_SHM_MAGIC` / `VD_SHM_VERSION` | `0x56444653` ("VDFS") / 4 |
| `VD_SHM_SLOT_COUNT` | 8 |
| `VD_SHM_SLOT_SIZE` | 256 KiB |
| `VD_SHM_SLOT_DATA_SIZE` | `VD_SHM_SLOT_SIZE - 64` = 262080 bytes |

Slot states: `VD_SLOT_EMPTY`, `VD_SLOT_WRITING`, `VD_SLOT_READY`,
`VD_SLOT_READING` (consumer lease).

**A frame larger than `VD_SHM_SLOT_DATA_SIZE` cannot be stored.** `push.c` drops
it. This has bitten the main stream before — when diagnosing "stream silently
produces nothing", check frame size against the slot size first.

### Client model

- The first client to issue a **lifecycle** command (`vi_open`, `venc_open`, …)
  becomes the **control client**; only it may issue lifecycle commands.
- Other clients may call streaming ops (`set_iframe`, `set_rc`) and read-only
  queries (`get_error_*`, `isp_*`).
- When the control client disconnects, any client may claim the role.

## Build

The Makefile prefers the repo-local crosstool-ng toolchain and falls back to the
legacy one; override with `ANYKA_CC=...`.

```
toolchain/arm-anykav200-crosstool-ng/bin/arm-unknown-linux-uclibcgnueabi-gcc   # preferred
toolchain/arm-anykav200-crosstool/usr/bin/arm-anykav200-linux-uclibcgnueabi-gcc # fallback
```

Core flags: `-std=gnu99 -march=armv5te -fno-PIC -Wall -Wextra`, plus
`-O2 -DNDEBUG` (release) or `-O0 -g3 -DDEBUG -fno-omit-frame-pointer` (debug).

```bash
make -C cross-compile/vendor-daemon          # release (default)
make -C cross-compile/vendor-daemon debug    # symbols + -DDEBUG
make -C cross-compile/vendor-daemon test     # host-side unit tests
make -C cross-compile/vendor-daemon clean

# stage the binary for deployment
cp cross-compile/vendor-daemon/build/vendor-daemon.bin \
   SD_card_contents/anyka_hack/vendor-daemon/
```

For the Rust side of the build and the vendored Rust toolchain, see the
`anyka-embedded-build` skill. For shipping a build to a camera, see
`anyka-firmware-upgrade`.

## Mandatory coding standards

### Bounded string functions only

```c
/* CORRECT */
snprintf(buf, sizeof(buf), "error: %d", code);
strncpy(dst, src, sizeof(dst) - 1);
dst[sizeof(dst) - 1] = '\0';

/* FORBIDDEN — stack/heap smash */
sprintf(buf, "error: %d", code);
strcpy(dst, src);
```

### Check every SDK return code

```c
int ret = ak_vi_open(VI_CHN_MAIN);
if (ret != AK_SUCCESS) {
    log_error("ak_vi_open failed: %d", ret);
    return -1;
}
```

`AK_SUCCESS` is 0, failures are negative; `ak_error.h` has the detailed codes.

### Validate IPC input before use

```c
if (req_len > MAX_PAYLOAD_SIZE) {
    log_error("cmd %d: oversized payload %u", cmd_id, req_len);
    return send_error_response(fd, STATUS_INVALID_ARG);
}
/* only now is req_data[0..req_len-1] safe */
```

### Logging macros, never printf

```c
log_info("vi_open: channel=%d", chn);
log_warn("frame buffer overflow, dropping frame %u", id);
log_error("venc_open failed with code %d", ret);
```

`printf`/`fprintf` are forbidden outside `-DDEBUG` blocks.

### Allocation

Prefer stack buffers for fixed sizes. On the heap, always check and always null
after free:

```c
uint8_t *payload = malloc(req_len);
if (!payload) {
    log_error("OOM allocating %u bytes", req_len);
    return send_error_response(fd, STATUS_NOMEM);
}
free(payload);
payload = NULL;
```

### poll() multiplexer

```c
struct pollfd fds[MAX_FDS];
int nfds = 0;
fds[nfds].fd = ctrl_listen_fd;
fds[nfds].events = POLLIN;
nfds++;

int ready = poll(fds, nfds, POLL_TIMEOUT_MS);
if (ready < 0) {
    if (errno == EINTR) continue;   /* signal, retry */
    log_error("poll() failed: %s", strerror(errno));
    break;
}
```

## Key SDK APIs

```c
/* ak_vi.h */
int ak_vi_open(int chn);
int ak_vi_close(int chn);
int ak_vi_set_frame_rate(int chn, int fps);
int ak_vi_get_frame(int chn, struct ak_video_frame *frame);
int ak_vi_release_frame(int chn, struct ak_video_frame *frame);

/* ak_venc.h */
int ak_venc_open(int chn, const struct ak_venc_attr *attr);
int ak_venc_close(int chn);
int ak_venc_get_stream(int chn, struct ak_stream_info *stream);
int ak_venc_release_stream(int chn, struct ak_stream_info *stream);
int ak_venc_set_iframe(int chn);
int ak_venc_set_rc(int chn, const struct ak_venc_rc *rc);
```

The vendored SDK source under `cross-compile/anyka_reference/` is reliable for
signatures and struct layouts, but not for return values or timing — the
daemon's own counters beat reading it.

## Testing

Host-side unit tests exist and run on the build host, not the camera:

```bash
make -C cross-compile/vendor-daemon test
```

Currently `test_sound_parse`, `test_push_slots`, `test_ring_epoch`. The two
ring/push tests `#include` the translation unit under test; new tests follow
`tests/test_<unit>.c` and are added to `HOST_TESTS` in the Makefile.

Beyond that: debug build on device (`make debug`, deploy, read `log_debug`
output), and Rust-side integration tests that mock the IPC socket so
`onvif-rust` can be tested without the real daemon.

## Self-review checklist

- [ ] No `sprintf` / `strcpy` / `gets`
- [ ] Every IPC `len` bounds-checked before the payload is read
- [ ] Every `malloc` result checked, pointer nulled after `free`
- [ ] Every SDK return value checked and logged
- [ ] Logging via `log.h` macros only
- [ ] Builds clean under `-Wall -Wextra`
- [ ] `make test` passes
