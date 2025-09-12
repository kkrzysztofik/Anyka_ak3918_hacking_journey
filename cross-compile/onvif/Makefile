export PATH := $(PATH):/opt/arm-anykav200-crosstool/usr/bin
PREFIX=arm-anykav200-linux-uclibcgnueabi-
CC=${PREFIX}gcc
CPP=${PREFIX}g++
CFLAGS=-std=c99 -D_POSIX_C_SOURCE=199309L -DANYKA_PLATFORM=1 -Os -fno-builtin -fno-stack-protector -Wl,--hash-style=sysv
LINKERFLAG=-L./lib -Wl,--gc-sections -lakuio -lakispsdk -lakv_encode -lplat_common -lplat_thread -lplat_vi -lplat_vpss -lplat_ipcsrv -lplat_venc_cb -lmpi_venc -lakstreamenc -lpthread -lakae -lakaudiocodec -lakmedia -lakv_encode -lapp_net -lapp_rtsp -lmpi_aed -lmpi_aenc -lplat_ai -lplat_drv -lm
INCLUDES=include src src/common src/platform src/utils

# Output directory for all build artifacts
OUT_DIR=out

SRCS := \
	src/main/onvifd.c \
	src/platform/platform_anyka.c \
	src/utils/config.c \
	src/utils/resource_cleanup.c \
	src/utils/response_buffer.c \
	src/utils/memory_debug.c \
	src/services/service_manager.c \
	src/services/device/onvif_device.c \
	src/services/media/onvif_media.c \
	src/services/ptz/onvif_ptz.c \
	src/services/imaging/onvif_imaging.c \
	src/server/http/http_server.c \
	src/server/http/http_parser.c \
	src/server/http/http_onvif_adapter.c \
	src/server/common/connection_manager.c \
	src/server/common/thread_pool.c \
	src/server/common/buffer_pool.c \
	src/server/common/epoll_server.c \
	src/server/rtsp/rtsp_server_main.c \
	src/server/rtsp/rtsp_auth.c \
	src/server/rtsp/rtsp_sdp.c \
	src/server/rtsp/rtsp_rtcp.c \
	src/server/rtsp/rtsp_session.c \
	src/server/rtsp/rtsp_rtp.c \
	src/server/discovery/ws_discovery.c \
	src/utils/network_utils.c \
	src/utils/xml_utils.c \
	src/utils/logging_utils.c

OBJS := $(SRCS:src/%.c=$(OUT_DIR)/%.o)

.PHONY = all clean debug

# Limit parallel jobs to prevent memory issues
MAKEFLAGS += -j1

all: clean $(OUT_DIR)/onvifd

# Debug target to build individual files
debug: clean
	@echo "Building individual files to identify memory issues..."
	@mkdir -p $(OUT_DIR)
	@for src in $(SRCS); do \
		echo "Building $$src..."; \
		obj=$$(echo $$src | sed 's|src/|$(OUT_DIR)/|' | sed 's|\.c|\.o|'); \
		mkdir -p $$(dirname $$obj); \
		${CC} ${CFLAGS} ${INCLUDES:%=-I%} -c $$src -o $$obj -pipe || exit 1; \
	done
	@echo "All files compiled successfully"

$(OUT_DIR)/onvifd: $(OBJS)
	${CC} ${CFLAGS} $(OBJS) ${LINKERFLAG} -o $@

$(OUT_DIR)/%.o: src/%.c | $(OUT_DIR)
	@mkdir -p $(dir $@)
	${CC} ${CFLAGS} ${INCLUDES:%=-I%} -c $< -o $@ -pipe

$(OUT_DIR):
	mkdir -p $(OUT_DIR)

clean:
	rm -rf $(OUT_DIR)
