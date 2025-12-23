FROM ubuntu:24.04

# Install base dependencies for Rust ONVIF project
RUN apt-get update && apt-get install -y --no-install-recommends \
  bc \
  binutils \
  ca-certificates \
  ccache \
  curl \
  file \
  git \
  libc6-dev \
  libssl-dev \
  libtinfo6 \
  libxml2 \
  nodejs \
  npm \
  perl \
  pkg-config \
  python3 \
  python3-pip \
  sudo \
  unzip \
  wget \
  zlib1g \
  && rm -rf /var/lib/apt/lists/* \
  && apt-get clean

# Copy toolchain and optimize size
COPY toolchain/arm-anykav200-crosstool-ng /opt/arm-anykav200-crosstool-ng
RUN rm -rf /opt/arm-anykav200-crosstool-ng/share/doc \
  /opt/arm-anykav200-crosstool-ng/share/man \
  /opt/arm-anykav200-crosstool-ng/share/info \
  /opt/arm-anykav200-crosstool-ng/share/locale \
  /opt/arm-anykav200-crosstool-ng/lib/*.la \
  2>/dev/null || true && \
  find /opt/arm-anykav200-crosstool-ng/bin -type f -executable -exec strip --strip-debug {} \; 2>/dev/null || true

ENV PATH="/opt/arm-anykav200-crosstool-ng/bin:${PATH}"

# Install build-essential for host builds (gcc for x86_64 proc-macros and build scripts)
# Install cargo via rustup (toolchain's cargo is a symlink that won't work in Docker)
RUN apt-get update && apt-get install -y --no-install-recommends build-essential && \
  rm -rf /var/lib/apt/lists/* && apt-get clean && \
  if ! command -v cargo >/dev/null 2>&1; then \
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal --component cargo --component rustfmt --component clippy && \
  /root/.cargo/bin/cargo --version && \
  rm -rf /root/.rustup/toolchains/*/share/doc \
  /root/.rustup/toolchains/*/share/man \
  /root/.rustup/toolchains/*/lib/rustlib/*/doc \
  /root/.rustup/toolchains/*/lib/rustlib/*/src \
  /root/.rustup/toolchains/*/lib/rustlib/src; \
  fi
ENV PATH="${PATH}:/root/.cargo/bin"

# Preinstall cargo-tarpaulin to avoid per-job installs (uses vendored OpenSSL)
# Install Snyk CLI
# Verify installation
RUN cargo install cargo-tarpaulin --locked --features vendored-openssl && \
  rm -rf /root/.cargo/registry/index/* /root/.cargo/git/db/* && \
  curl -Lo snyk-linux https://downloads.snyk.io/cli/stable/snyk-linux \
  && curl -Lo snyk-linux.sha256 https://downloads.snyk.io/cli/stable/snyk-linux.sha256 \
  && sha256sum -c snyk-linux.sha256 \
  && chmod +x snyk-linux \
  && mv snyk-linux /usr/local/bin/snyk \
  && rm snyk-linux.sha256 && \
  arm-unknown-linux-uclibcgnueabi-gcc --version || arm-anykav200-linux-uclibcgnueabi-gcc --version && \
  EXPECTED_RUSTC="/opt/arm-anykav200-crosstool-ng/bin/rustc" && \
  ACTUAL_RUSTC=$(which rustc) && \
  rustc --version && \
  if [ "$ACTUAL_RUSTC" != "$EXPECTED_RUSTC" ]; then \
  echo "ERROR: rustc is not from toolchain! Expected: $EXPECTED_RUSTC, Found: $ACTUAL_RUSTC" && \
  exit 1; \
  fi && \
  cargo --version && \
  clang --version && \
  snyk --version && \
  ccache --version

# Set working directory
WORKDIR /workspace

# Default command
CMD ["/bin/bash"]
