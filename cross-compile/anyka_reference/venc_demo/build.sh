export PATH=$PATH:/opt/arm-anykav200-crosstool/usr/bin
arm-anykav200-linux-uclibcgnueabi-gcc ak_venc_demo.c -std=c99 -D_POSIX_C_SOURCE=199309L -Iinclude  -Llib -lakuio -lakispsdk -lakv_encode -lplat_common -lplat_thread -lplat_vi -lplat_vpss -lplat_ipcsrv -lplat_venc_cb -lmpi_venc -lakstreamenc -o venc_demo.out
