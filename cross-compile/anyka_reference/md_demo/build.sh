export PATH=$PATH:/opt/arm-anykav200-crosstool/usr/bin
arm-anykav200-linux-uclibcgnueabi-gcc ak_vpss_md_demo.c -std=c99 -D_POSIX_C_SOURCE=199309L -Iinclude  -Llib -lakuio -lakispsdk -lplat_common -lplat_thread -lplat_vi -lplat_vpss  -lplat_ipcsrv -o md_demo
