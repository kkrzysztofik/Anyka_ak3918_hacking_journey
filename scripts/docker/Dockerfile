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

# Toolchain bin first; ~/.cargo/bin only for tools installed with `cargo install`
# (e.g. tarpaulin). Never put rustup proxies ahead of the vendored toolchain.
ENV PATH="/opt/arm-anykav200-crosstool-ng/bin:/root/.cargo/bin:${PATH}"

# Host gcc for x86_64 proc-macros/build scripts. Use vendored cargo/rustc/clippy
# from /opt (real binaries — do NOT install rustup stable; rustup cargo-clippy
# proxies clash with 1.97.1-dev → E0514). Preinstall tarpaulin for CI.
RUN apt-get update && apt-get install -y --no-install-recommends build-essential && \
  rm -rf /var/lib/apt/lists/* && apt-get clean && \
  test "$(command -v cargo)" = "/opt/arm-anykav200-crosstool-ng/bin/cargo" && \
  test "$(command -v rustc)" = "/opt/arm-anykav200-crosstool-ng/bin/rustc" && \
  cargo install cargo-tarpaulin --locked --features vendored-openssl && \
  rm -rf /root/.cargo/registry/index/* /root/.cargo/git/db/* && \
  (arm-unknown-linux-uclibcgnueabi-gcc --version || arm-anykav200-linux-uclibcgnueabi-gcc --version) && \
  EXPECTED_RUSTC="/opt/arm-anykav200-crosstool-ng/bin/rustc" && \
  ACTUAL_RUSTC=$(command -v rustc) && \
  rustc --version && \
  if [ "$ACTUAL_RUSTC" != "$EXPECTED_RUSTC" ]; then \
  echo "ERROR: rustc is not from toolchain! Expected: $EXPECTED_RUSTC, Found: $ACTUAL_RUSTC" && \
  exit 1; \
  fi && \
  rustc --version | grep -q '1\.97' && \
  cargo --version && \
  clang --version 2>&1 | tee /tmp/clang-version.txt | grep -q '22\.1\.8' && \
  ccache --version && \
  command -v cargo-tarpaulin && \
  command -v clippy-driver

# Set working directory
WORKDIR /workspace

# Default command
CMD ["/bin/bash"]
