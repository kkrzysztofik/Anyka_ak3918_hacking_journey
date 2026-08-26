# vendor-daemon Makefile
#
# lib/ and include/ directories are LOCAL to this source tree:
#
#   include/  — 9 curated Anyka SDK headers:
#                 ak_aenc.h  ak_ai.h  ak_common.h  ak_error.h  ak_global.h
#                 ak_venc.h  ak_vi.h  ak_vpss.h  list.h
#
#   lib/      — 16 best-of-breed SDK .so files selected from
#                 IOT-ANYKA-PTZdaemon and libre_anyka_app:
#                 libakaudiocodec.so  libakaudiofilter.so  libakispsdk.so
#                 libakstreamenc.so   libakuio.so          libakv_encode.so
#                 libmpi_aenc.so      libmpi_venc.so       libplat_ai.so
#                 libplat_common.so   libplat_drv.so       libplat_ipcsrv.so
#                 libplat_thread.so   libplat_venc_cb.so   libplat_vi.so
#                 libplat_vpss.so
#
# Configurable via environment variables:
#   ANYKA_CC          - cross-compiler binary
#   ANYKA_INCLUDE_DIR - SDK include directory (default: ./include)
#   ANYKA_LIB_DIR     - SDK shared-library directory (default: ./lib)

# Prefer repo-local crosstool-ng (works on modern x86_64 hosts). Override with ANYKA_CC=...
REPO_ROOT         := $(abspath $(CURDIR)/../..)
TOOLCHAIN_NG_CC   := $(REPO_ROOT)/toolchain/arm-anykav200-crosstool-ng/bin/arm-unknown-linux-uclibcgnueabi-gcc
TOOLCHAIN_LEGACY_CC := $(REPO_ROOT)/toolchain/arm-anykav200-crosstool/usr/bin/arm-anykav200-linux-uclibcgnueabi-gcc

ifeq ($(wildcard $(TOOLCHAIN_NG_CC)),)
  ANYKA_CC ?= $(TOOLCHAIN_LEGACY_CC)
else
  ANYKA_CC ?= $(TOOLCHAIN_NG_CC)
endif

ANYKA_INCLUDE_DIR ?= $(CURDIR)/include
ANYKA_LIB_DIR     ?= $(CURDIR)/lib

SD_DEPLOY_DIR ?= $(REPO_ROOT)/SD_card_contents/anyka_hack/vendor-daemon

CC = $(ANYKA_CC)

# Build mode selection:
#   make                -> release (default)
#   make release        -> explicit release build
#   make debug          -> debug build with symbols
BUILD_MODE ?= release

COMMON_CFLAGS = \
	-march=armv5te \
	-mfloat-abi=soft \
	-fno-PIC \
	-Wall \
	-Wextra \
	-std=gnu99 \
	-I$(ANYKA_INCLUDE_DIR)

RELEASE_CFLAGS = -O2 -DNDEBUG
DEBUG_CFLAGS   = -O0 -g3 -DDEBUG -fno-omit-frame-pointer

ifeq ($(BUILD_MODE),debug)
TARGET    = vendor-daemon-debug.bin
MODE_CFLAGS = $(DEBUG_CFLAGS)
else
TARGET    = vendor-daemon.bin
MODE_CFLAGS = $(RELEASE_CFLAGS)
endif

CFLAGS = $(COMMON_CFLAGS) $(MODE_CFLAGS)

# Use --start-group/--end-group to handle any circular dependencies among
# the SDK libraries.  System libs (pthread, rt, m, dl) go outside the group
# since they have no circular deps with the SDK.
# --allow-shlib-undefined: The older libplat_thread.so (9388 bytes, pre-regression)
# doesn't export ak_thread_cond_*, ak_thread_mutexattr_*, ak_thread_sem_{try,timed}wait
# which libplat_venc_cb.so references. uClibc's lazy binding means these are only
# resolved if actually called (they aren't). This flag mirrors that runtime behavior
# at link time. See: SD_card_contents/anyka_hack/libre_anyka_app runs with this
# exact combination proving the code paths are never hit.
# -lmpi_osd comes from the IOT-ANYKA-PTZdaemon bundle, not this SDK generation.
# It needs ak_cmd_register_module / ak_cmd_unregister_module, which our
# libplat_ipcsrv.so does not export (it has ak_cmd_server_register instead).
# ak_osd_init ALWAYS calls osd_sys_ipc_register (disassembly of V1.1.03), which
# reaches those symbols — Stage A only proved the .so loads; live init crashes
# without stubs.  osd_ipcsrv_stubs.c provides no-ops.  Verified on .198 2026-08-24.
LDFLAGS = \
	-L$(ANYKA_LIB_DIR) \
	-Wl,--allow-shlib-undefined -Wl,--export-dynamic \
	-Wl,--start-group \
	  -lplat_vi \
	  -lplat_vpss \
	  -lmpi_osd \
	  -lmpi_venc \
	  -lplat_ai \
	  -lplat_ao \
	  -lmpi_aenc \
	  -lplat_common \
	  -lakispsdk \
	  -lakv_encode \
	  -lplat_venc_cb \
	  -lakaudiocodec \
	  -lakaudiofilter \
	  -lplat_thread \
	  -lplat_ipcsrv \
	  -lakuio \
	  -lplat_drv \
	  -lakstreamenc \
	-Wl,--end-group \
	-lpthread -lrt -lm -ldl
# osd_vpss_wrap.c uses dlsym(RTLD_NEXT, "ak_vpss_osd_set_param").

SRC_DIR   = src
BUILD_DIR = build
SRCS      = $(wildcard $(SRC_DIR)/*.c)
OBJS      = $(SRCS:$(SRC_DIR)/%.c=$(BUILD_DIR)/%.o)

.PHONY: all release debug clean install sync-libs test

all: $(BUILD_DIR)/$(TARGET)

release:
	$(MAKE) BUILD_MODE=release all

debug:
	$(MAKE) BUILD_MODE=debug all

HOST_CC ?= gcc

# Host-side unit tests. Each test has its own link recipe because the older
# suites embed translation units (push.c) or rely on header-only ring helpers.
test: $(BUILD_DIR)/test_sound_parse $(BUILD_DIR)/test_push_slots $(BUILD_DIR)/test_ring_epoch
	$(BUILD_DIR)/test_sound_parse
	$(BUILD_DIR)/test_push_slots
	$(BUILD_DIR)/test_ring_epoch

$(BUILD_DIR)/test_sound_parse: tests/test_sound_parse.c src/sound.c
	@mkdir -p $(BUILD_DIR)
	$(HOST_CC) -std=gnu99 -Wall -Wextra -Isrc -Iinclude -o $@ tests/test_sound_parse.c src/sound.c

$(BUILD_DIR)/test_push_slots: tests/test_push_slots.c src/push.c
	@mkdir -p $(BUILD_DIR)
	$(HOST_CC) -std=gnu99 -Wall -Wextra -Isrc -Iinclude -o $@ tests/test_push_slots.c

$(BUILD_DIR)/test_ring_epoch: tests/test_ring_epoch.c
	@mkdir -p $(BUILD_DIR)
	$(HOST_CC) -std=gnu99 -Wall -Wextra -Isrc -Iinclude -o $@ tests/test_ring_epoch.c

$(BUILD_DIR)/$(TARGET): $(OBJS)
	$(CC) $(CFLAGS) -o $@ $^ $(LDFLAGS)

$(BUILD_DIR)/%.o: $(SRC_DIR)/%.c | $(BUILD_DIR)
	$(CC) $(CFLAGS) -c -o $@ $<

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

clean:
	rm -rf $(BUILD_DIR)

# Install the binary and all SDK .so files to the SD card vendor-daemon directory.
install: $(BUILD_DIR)/$(TARGET)
	install -m 755 $(BUILD_DIR)/$(TARGET) $(SD_DEPLOY_DIR)/$(TARGET)
	mkdir -p $(SD_DEPLOY_DIR)/lib
	cp -v $(ANYKA_LIB_DIR)/*.so $(SD_DEPLOY_DIR)/lib/

# Sync only the lib/ directory to the SD card (no rebuild needed).
sync-libs:
	mkdir -p $(SD_DEPLOY_DIR)/lib
	cp -v $(ANYKA_LIB_DIR)/*.so $(SD_DEPLOY_DIR)/lib/
