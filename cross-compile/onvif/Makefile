export PATH := $(PATH):/opt/arm-anykav200-crosstool/usr/bin
PREFIX=arm-anykav200-linux-uclibcgnueabi-
CC=${PREFIX}gcc
CPP=${PREFIX}g++
CFLAGS=-std=c99 -D_POSIX_C_SOURCE=199309L
LINKERFLAG=-L./lib -lakuio -lakispsdk -lakv_encode -lplat_common -lplat_thread -lplat_vi -lplat_vpss -lplat_ipcsrv -lplat_venc_cb -lmpi_venc -lakstreamenc -lpthread -lakae -lakaudiocodec -lakmedia -lakv_encode -lapp_net -lapp_rtsp -lmpi_aed -lmpi_aenc -lplat_ai -lplat_drv -lm
INCLUDES=include src src/services src/adapters src/utils src/server src/tools

SRCS := \
	src/main/onvifd.c \
	src/adapters/ptz_adapter.c \
	src/utils/config.c \
	src/services/device/onvif_device.c \
	src/services/media/onvif_media.c \
	src/services/ptz/onvif_ptz.c \
	src/services/imaging/onvif_imaging.c \
	src/server/http_server.c \
	src/server/rtsp_server.c \
	src/server/ws_discovery.c \
	src/utils/network_utils.c \
	src/utils/utils.c

.PHONY = all clean

all: clean onvifd

onvifd: $(SRCS)
	${CC} ${CFLAGS} ${INCLUDES:%=-I%} $(SRCS) ${LINKERFLAG} -o $@

clean:
	rm -f onvifd
