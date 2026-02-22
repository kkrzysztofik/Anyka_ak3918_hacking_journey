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

ANYKA_CC          ?= /home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool/usr/bin/arm-anykav200-linux-uclibcgnueabi-gcc
ANYKA_INCLUDE_DIR ?= $(CURDIR)/include
ANYKA_LIB_DIR     ?= $(CURDIR)/lib

SD_DEPLOY_DIR = /home/kmk/anyka-dev/SD_card_contents/anyka_hack/vendor-daemon

CC = $(ANYKA_CC)

CFLAGS = \
	-march=armv5te \
	-mfloat-abi=soft \
	-fno-PIC \
	-Wall \
	-Wextra \
	-std=gnu99 \
	-I$(ANYKA_INCLUDE_DIR)

# Use --start-group/--end-group to handle any circular dependencies among
# the SDK libraries.  System libs (pthread, rt, m, dl) go outside the group
# since they have no circular deps with the SDK.
# --allow-shlib-undefined: The older libplat_thread.so (9388 bytes, pre-regression)
# doesn't export ak_thread_cond_*, ak_thread_mutexattr_*, ak_thread_sem_{try,timed}wait
# which libplat_venc_cb.so references. uClibc's lazy binding means these are only
# resolved if actually called (they aren't). This flag mirrors that runtime behavior
# at link time. See: SD_card_contents/anyka_hack/libre_anyka_app runs with this
# exact combination proving the code paths are never hit.
LDFLAGS = \
	-L$(ANYKA_LIB_DIR) \
	-Wl,--allow-shlib-undefined \
	-Wl,--start-group \
	  -lplat_vi \
	  -lplat_vpss \
	  -lmpi_venc \
	  -lplat_ai \
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

TARGET    = vendor-daemon.bin
SRC_DIR   = src
BUILD_DIR = build
SRCS      = $(wildcard $(SRC_DIR)/*.c)
OBJS      = $(SRCS:$(SRC_DIR)/%.c=$(BUILD_DIR)/%.o)

.PHONY: all clean install sync-libs

all: $(BUILD_DIR)/$(TARGET)

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
