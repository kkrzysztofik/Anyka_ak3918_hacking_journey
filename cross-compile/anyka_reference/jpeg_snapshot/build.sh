export PATH=$PATH:/opt/arm-anykav200-crosstool/usr/bin
arm-anykav200-linux-uclibcgnueabi-gcc main.c -std=c99 -D_POSIX_C_SOURCE=199309L -Iinclude  -Llib -lakuio -lakispsdk -lakv_encode -lplat_common -lplat_thread -lplat_vi -lplat_vpss -lplat_ipcsrv -lplat_venc_cb -lmpi_venc -lakstreamenc -lpthread -o jpeg_snapshot
