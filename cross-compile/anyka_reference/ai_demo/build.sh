export PATH=$PATH:/opt/arm-anykav200-crosstool/usr/bin
arm-anykav200-linux-uclibcgnueabi-gcc ak_ai_demo.c -std=c99 -D_POSIX_C_SOURCE=199309L  -Iinclude  -Llib -lplat_thread -lplat_common -ldl -lpthread -lplat_ipcsrv -lplat_ai -lakaudiofilter -o ai_demo
