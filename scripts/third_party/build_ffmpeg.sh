#!/usr/bin/env bash
# Build FFmpeg for Anyka AK3918 (ARMv5TEJ, uClibc, soft-float) and stage into SD overlay.
# Produces a statically-linked ffmpeg binary.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "$(cd "${SCRIPT_DIR}/../.." && pwd)/scripts/common.sh"
# shellcheck source=scripts/third_party/build_lib.sh
source "${SCRIPT_DIR}/build_lib.sh"
REPO_ROOT="${ANYKA_REPO_ROOT}"

DEFAULT_VERSION="7.1"
DEFAULT_SHA256=""

version="${DEFAULT_VERSION}"
sha256="${DEFAULT_SHA256}"
url=""
archive_override=""
toolchain_dir="${REPO_ROOT}/toolchain/arm-anykav200-crosstool-ng"
target_triple="arm-unknown-linux-uclibcgnueabi"
output_dir="${REPO_ROOT}/SD_card_contents/anyka_hack/ffmpeg"
work_root="${REPO_ROOT}/target/third_party/ffmpeg"
jobs="$(nproc)"

usage() {
  cat <<USAGE
Usage: $(basename "$0") [options]

Options:
  --version <v>         FFmpeg version (default: ${DEFAULT_VERSION})
  --sha256 <sum>        Source archive SHA256 (recommended for reproducible builds)
  --url <url>           Explicit source URL
  --archive <path>      Use existing source archive instead of downloading
  --toolchain-dir <p>   Toolchain base path (default: repo-relative)
  --target <triple>     Target triple (default: ${target_triple})
  --output-dir <p>      Output directory (default: SD_card_contents/anyka_hack/ffmpeg)
  --work-root <p>       Build working root (default: target/third_party/ffmpeg)
  --jobs <n>            Parallel build jobs (default: nproc)
  -h, --help            Show this help

Examples:
  $(basename "$0")
  $(basename "$0") --version ${DEFAULT_VERSION}
  $(basename "$0") --version 7.0 --sha256 <hex>
USAGE
}

tp_parse_common_args "$@"
if [ "${#TP_EXTRA_ARGS[@]}" -gt 0 ]; then set -- "${TP_EXTRA_ARGS[@]}"; else set --; fi
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output-dir) output_dir="$2"; shift 2 ;;
    -h|--help)    usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage; exit 1 ;;
  esac
done

if [ -z "${url}" ]; then
  url="https://ffmpeg.org/releases/ffmpeg-${version}.tar.xz"
fi

tp_validate_toolchain
tp_validate_jobs

mkdir -p "${work_root}" "${output_dir}"

build_dir="${work_root}/ffmpeg-${version}"
build_out="${build_dir}/build-out"
install_dir="${build_dir}/install"

rm -rf "${build_dir}"
tp_fetch_archive "ffmpeg-${version}" "tar.xz" "${url}" "${sha256}" "${archive_override}" "${build_dir}"
# source_dir set by tp_fetch_archive

cross_prefix="${toolchain_dir}/bin/${target_triple}-"

# ── Configure ────────────────────────────────────────────────────────────────
mkdir -p "${build_out}" "${install_dir}"
cd "${build_out}"

echo "Configuring FFmpeg ${version} for ARMv5TEJ..."
"${source_dir}/configure" \
  --prefix="${install_dir}" \
  --cross-prefix="${cross_prefix}" \
  --sysroot="${sysroot}" \
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
  --enable-bsf=dump_extradata

# ── Build ────────────────────────────────────────────────────────────────────
echo "Building FFmpeg with ${jobs} jobs..."
make -j"${jobs}"

# ── Install ───────────────────────────────────────────────────────────────────
make install

ffmpeg_bin="${install_dir}/bin/ffmpeg"
if [ ! -f "${ffmpeg_bin}" ]; then
  echo "ERROR: ffmpeg binary not found at ${ffmpeg_bin}" >&2
  exit 1
fi

"${cross_prefix}strip" --strip-all "${ffmpeg_bin}"

echo ""
echo "Binary info:"
file "${ffmpeg_bin}"
ls -lh "${ffmpeg_bin}"
# (ffmpeg is statically linked; no library bundling needed)

# ── Deploy ───────────────────────────────────────────────────────────────────
echo ""
echo "Installing to ${output_dir}..."
install -m 0755 "${ffmpeg_bin}" "${output_dir}/ffmpeg"

echo "FFmpeg build complete: ${output_dir}/ffmpeg"
