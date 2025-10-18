FROM ubuntu:24.04

# Set up 32-bit architecture support
RUN dpkg --add-architecture i386

# Update and install dependencies
RUN apt-get update && apt-get install -y \
    git \
    wget \
    file \
    binutils \
    binutils-multiarch \
    libc6:i386 \
    libncurses6:i386 \
    libstdc++6:i386 \
    zlib1g:i386 \
    build-essential \
    gcc-multilib \
    g++-multilib \
    doxygen \
    graphviz \
    make \
    clang \
    clang-tools \
    clangd \
    cppcheck \
    curl \
    unzip \
    python3 \
    python3-pip \
    && rm -rf /var/lib/apt/lists/*

# Copy and extract local Anyka toolchain
COPY toolchain/arm-anykav200-crosstool.zip /tmp/
COPY toolchain/arm-anykav200-gdb-7.12.tar.gz /tmp/
RUN cd /tmp \
    && unzip -q arm-anykav200-crosstool.zip \
    && tar -xzf arm-anykav200-gdb-7.12.tar.gz \
    && mv arm-anykav200-crosstool /opt/ \
    && mv gdb /opt/arm-anykav200-crosstool/usr/bin/ \
    && mv gdbserver /opt/arm-anykav200-crosstool/usr/bin/ \
    && mv arm-anykav200-linux-uclibcgnueabi-gdb /opt/arm-anykav200-crosstool/usr/bin/ \
    && rm arm-anykav200-crosstool.zip arm-anykav200-gdb-7.12.tar.gz

# Add toolchain to PATH
ENV PATH="/opt/arm-anykav200-crosstool/usr/bin:${PATH}"

# Install Snyk CLI
RUN curl -Lo snyk-linux https://downloads.snyk.io/cli/stable/snyk-linux \
    && curl -Lo snyk-linux.sha256 https://downloads.snyk.io/cli/stable/snyk-linux.sha256 \
    && sha256sum -c snyk-linux.sha256 \
    && chmod +x snyk-linux \
    && mv snyk-linux /usr/local/bin/snyk \
    && rm snyk-linux.sha256

# Install clangd-tidy (faster alternative to clang-tidy)
# Using --break-system-packages for Docker container (safe in isolated environment)
RUN pip3 install --break-system-packages clangd-tidy

# Verify installation
RUN arm-anykav200-linux-uclibcgnueabi-gcc -v
RUN doxygen --version
RUN dot -V
RUN clang --version
RUN clangd --version
RUN cppcheck --version
RUN snyk --version
RUN clangd-tidy --version

# Set working directory
WORKDIR /workspace

# Default command
CMD ["/bin/bash"]
