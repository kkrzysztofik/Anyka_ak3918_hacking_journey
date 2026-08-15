#!/bin/bash
#static
#arm-linux-gnueabi-g++ ptz_daemon_cpp.cpp -L./libs -ldl -lplat_drv -lplat_common -lplat_thread -lpthread -D_GLIBCXX_USE_CXX11_ABI=0 -static -o ptz_daemon
#dynamic
export PATH=$PATH:/opt/arm-anykav200-crosstool/usr/bin
arm-anykav200-linux-uclibcgnueabi-g++ ptz_daemon_cpp.cpp -std=c++11 -D_POSIX_C_SOURCE=199309L -L./libs -ldl -lplat_drv -lplat_common -lplat_thread -lpthread -D_GLIBCXX_USE_CXX11_ABI=0 -o ptz_daemon_dyn
