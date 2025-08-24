FROM ubuntu:16.04

# Set up 32-bit architecture support
RUN dpkg --add-architecture i386

# Update and install dependencies
RUN apt-get update && apt-get install -y \
    git \
    wget \
    libc6:i386 \
    libncurses5:i386 \
    libstdc++6:i386 \
    zlib1g:i386 \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Download and extract Anyka toolchain
RUN wget https://github.com/helloworld-spec/qiwen/raw/main/anycloud39ev300/SDK/tools/anyka_uclibc_gcc.tar.bz2 \
    && tar -Pxvf anyka_uclibc_gcc.tar.bz2 \
    && rm anyka_uclibc_gcc.tar.bz2

# Add toolchain to PATH
ENV PATH="/opt/arm-anykav200-crosstool/usr/bin:${PATH}"

# Verify installation
RUN arm-anykav200-linux-uclibcgnueabi-gcc -v

# Set working directory
WORKDIR /workspace

# Default command
CMD ["/bin/bash"]
