export PATH := $(PATH):/opt/arm-anykav200-crosstool/usr/bin
PREFIX=arm-anykav200-linux-uclibcgnueabi-
CC=${PREFIX}gcc
CPP=${PREFIX}g++
CFLAGS=-std=c99 -D_POSIX_C_SOURCE=199309L -DANYKA_PLATFORM=1
LINKERFLAG=-L./lib -lakuio -lakispsdk -lakv_encode -lplat_common -lplat_thread -lplat_vi -lplat_vpss -lplat_ipcsrv -lplat_venc_cb -lmpi_venc -lakstreamenc -lpthread -lakae -lakaudiocodec -lakmedia -lakv_encode -lapp_net -lapp_rtsp -lmpi_aed -lmpi_aenc -lplat_ai -lplat_drv -lm
INCLUDES=include src src/services src/adapters src/utils src/server src/server/common src/server/http src/server/rtsp src/server/discovery src/tools src/platform

SRCS := \
	src/main/onvifd.c \
	src/platform/platform_anyka.c \
	src/utils/config.c \
	src/utils/resource_cleanup.c \
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

.PHONY = all clean

all: clean onvifd

onvifd: $(SRCS)
	${CC} ${CFLAGS} ${INCLUDES:%=-I%} $(SRCS) ${LINKERFLAG} -o $@

clean:
	rm -f onvifd
