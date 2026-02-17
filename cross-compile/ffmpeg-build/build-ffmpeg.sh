#!/bin/bash
#
# Cross-compile FFmpeg for Anyka AK3918 (ARMv5TEJ, uClibc, soft-float)
# Produces a statically-linked ffmpeg binary
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TOOLCHAIN_DIR="/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng"
CROSS_PREFIX="${TOOLCHAIN_DIR}/bin/arm-unknown-linux-uclibcgnueabi-"
SYSROOT="${TOOLCHAIN_DIR}/arm-unknown-linux-uclibcgnueabi/sysroot"

FFMPEG_VERSION="${FFMPEG_VERSION:-7.1}"
SRC_DIR="${SCRIPT_DIR}/ffmpeg-${FFMPEG_VERSION}"
BUILD_DIR="${SCRIPT_DIR}/build"
INSTALL_DIR="${SCRIPT_DIR}/install"
DEPLOY_DIR="/home/kmk/anyka-dev/SD_card_contents/anyka_hack/ffmpeg"

NPROC=$(nproc)

# ── Download source ──────────────────────────────────────────────────────────
download_source() {
    if [[ -d "${SRC_DIR}" ]]; then
        echo "Source directory already exists: ${SRC_DIR}"
        return
    fi

    local tarball="ffmpeg-${FFMPEG_VERSION}.tar.xz"
    if [[ ! -f "${SCRIPT_DIR}/${tarball}" ]]; then
        echo "Downloading FFmpeg ${FFMPEG_VERSION}..."
        wget --https-only --max-redirect=0 --secure-protocol=TLSv1_2 \
            -O "${SCRIPT_DIR}/${tarball}" \
            "https://ffmpeg.org/releases/${tarball}"
    fi

    echo "Extracting..."
    tar xf "${SCRIPT_DIR}/${tarball}" -C "${SCRIPT_DIR}"
}

# ── Configure ────────────────────────────────────────────────────────────────
configure_ffmpeg() {
    mkdir -p "${BUILD_DIR}"
    cd "${BUILD_DIR}"

    echo "Configuring FFmpeg for ARMv5TEJ..."
    "${SRC_DIR}/configure" \
        --prefix="${INSTALL_DIR}" \
        --cross-prefix="${CROSS_PREFIX}" \
        --sysroot="${SYSROOT}" \
        --arch=arm \
        --cpu=arm926ej-s \
        --target-os=linux \
        --enable-cross-compile \
        \
        --enable-static \
        --disable-shared \
        --pkg-config="pkg-config --static" \
        --extra-cflags="-march=armv5te -mtune=arm926ej-s -mfloat-abi=soft -msoft-float -Os -ffunction-sections -fdata-sections" \
        --extra-ldflags="-static -Wl,--gc-sections" \
        \
        --enable-small \
        --enable-gpl \
        \
        --disable-doc \
        --disable-htmlpages \
        --disable-manpages \
        --disable-podpages \
        --disable-txtpages \
        \
        --disable-network \
        --disable-autodetect \
        --disable-asm \
        --disable-neon \
        --disable-vfp \
        --disable-thumb \
        --disable-runtime-cpudetect \
        --disable-debug \
        --disable-stripping \
        \
        --disable-everything \
        --disable-postproc \
        \
        --enable-avutil \
        --enable-avcodec \
        --enable-avformat \
        --enable-avfilter \
        --enable-avdevice \
        --enable-swscale \
        --enable-swresample \
        \
        --enable-ffmpeg \
        --disable-ffplay \
        --disable-ffprobe \
        \
        --enable-indev=v4l2 \
        --enable-protocol=file \
        --enable-protocol=pipe \
        \
        --enable-demuxer=rawvideo \
        --enable-demuxer=h264 \
        --enable-demuxer=hevc \
        --enable-demuxer=aac \
        --enable-demuxer=pcm_s16le \
        --enable-demuxer=pcm_alaw \
        --enable-demuxer=pcm_mulaw \
        --enable-demuxer=v4l2 \
        --enable-demuxer=mov \
        --enable-demuxer=avi \
        --enable-demuxer=matroska \
        --enable-demuxer=concat \
        --enable-demuxer=image2 \
        \
        --enable-muxer=h264 \
        --enable-muxer=hevc \
        --enable-muxer=mp4 \
        --enable-muxer=avi \
        --enable-muxer=matroska \
        --enable-muxer=rawvideo \
        --enable-muxer=image2 \
        --enable-muxer=pcm_s16le \
        --enable-muxer=pcm_alaw \
        --enable-muxer=pcm_mulaw \
        --enable-muxer=null \
        --enable-muxer=segment \
        \
        --enable-decoder=h264 \
        --enable-decoder=hevc \
        --enable-decoder=rawvideo \
        --enable-decoder=aac \
        --enable-decoder=pcm_s16le \
        --enable-decoder=pcm_alaw \
        --enable-decoder=pcm_mulaw \
        --enable-decoder=mjpeg \
        --enable-decoder=bmp \
        \
        --enable-encoder=rawvideo \
        --enable-encoder=mjpeg \
        --enable-encoder=pcm_s16le \
        --enable-encoder=pcm_alaw \
        --enable-encoder=pcm_mulaw \
        \
        --enable-parser=h264 \
        --enable-parser=hevc \
        --enable-parser=aac \
        --enable-parser=mjpeg \
        \
        --enable-filter=scale \
        --enable-filter=format \
        --enable-filter=fps \
        --enable-filter=trim \
        --enable-filter=setpts \
        --enable-filter=aresample \
        --enable-filter=anull \
        --enable-filter=null \
        --enable-filter=copy \
        --enable-filter=crop \
        --enable-filter=overlay \
        --enable-filter=color \
        --enable-filter=transpose \
        --enable-filter=hflip \
        --enable-filter=vflip \
        --enable-filter=rotate \
        \
        --enable-bsf=h264_mp4toannexb \
        --enable-bsf=hevc_mp4toannexb \
        --enable-bsf=extract_extradata \
        --enable-bsf=dump_extradata \
        "$@"

    echo "Configuration complete."
    return 0
}

# ── Build ────────────────────────────────────────────────────────────────────
build_ffmpeg() {
    cd "${BUILD_DIR}"
    echo "Building FFmpeg with ${NPROC} jobs..."
    make -j"${NPROC}"
    echo "Build complete."
    return 0
}

# ── Install & Deploy ─────────────────────────────────────────────────────────
install_ffmpeg() {
    cd "${BUILD_DIR}"
    make install

    local binary="${INSTALL_DIR}/bin/ffmpeg"
    if [[ ! -f "${binary}" ]]; then
        echo "ERROR: ffmpeg binary not found at ${binary}" >&2
        exit 1
    fi

    # Strip the binary for minimum size
    "${CROSS_PREFIX}strip" --strip-all "${binary}"

    # Verify it's the right architecture
    echo ""
    echo "Binary info:"
    file "${binary}"
    ls -lh "${binary}"

    # Deploy
    echo ""
    echo "Deploying to ${DEPLOY_DIR}..."
    mkdir -p "${DEPLOY_DIR}"
    cp "${binary}" "${DEPLOY_DIR}/ffmpeg"
    echo "Deployed."
    return 0
}

# ── Clean ────────────────────────────────────────────────────────────────────
clean() {
    echo "Cleaning build and install directories..."
    rm -rf "${BUILD_DIR}" "${INSTALL_DIR}"
    return 0
}

# ── Main ─────────────────────────────────────────────────────────────────────
usage() {
    echo "Usage: $0 [download|configure|build|install|all|clean]"
    echo ""
    echo "Steps can be run individually or 'all' runs the full pipeline."
    echo ""
    echo "Environment variables:"
    echo "  FFMPEG_VERSION  FFmpeg version to build (default: 7.1)"
    echo ""
    echo "To add network support (RTSP, HTTP), pass extra flags:"
    echo "  $0 configure --enable-network --enable-protocol=tcp --enable-protocol=udp"
    return 0
}

case "${1:-all}" in
    download)   download_source ;;
    configure)  shift; configure_ffmpeg "$@" ;;
    build)      build_ffmpeg ;;
    install)    install_ffmpeg ;;
    all)
        download_source
        configure_ffmpeg
        build_ffmpeg
        install_ffmpeg
        echo ""
        echo "FFmpeg cross-compilation complete!"
        ;;
    clean)      clean ;;
    *)          usage ;;
esac
