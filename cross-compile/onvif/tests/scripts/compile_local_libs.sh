#!/bin/bash
# compile_local_libs.sh
# Compile gsoap and libb64 libraries for native x86_64 unit testing

set -e  # Exit on any error

# Native compilation for unit tests (development machine)
# Use native GCC for testing on development machine
CC=gcc
CPP=g++

# Check for required dependencies
echo "=== Checking Dependencies ==="
for cmd in gcc g++ make wget unzip tar; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "❌ Error: $cmd not found. Install with: sudo apt-get install build-essential wget unzip" >&2
        exit 1
    fi
done
echo "✅ All dependencies found"

# Script directory and paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TESTS_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$TESTS_DIR")"
NATIVE_LIB_DIR="${TESTS_DIR}/lib"
BUILD_DIR="${TESTS_DIR}/build_native"

# Create directories
mkdir -p "${NATIVE_LIB_DIR}"
mkdir -p "${BUILD_DIR}"

BUILD_GSOAP=1
BUILD_LIBB64=1

if compgen -G "${NATIVE_LIB_DIR}/libgsoap*.a" >/dev/null || compgen -G "${NATIVE_LIB_DIR}/libgsoap*.so*" >/dev/null; then
    echo "=== gSOAP libraries already compiled; skipping rebuild ==="
    BUILD_GSOAP=0
fi

if [[ -f "${NATIVE_LIB_DIR}/libb64.a" ]]; then
    echo "=== libb64 already compiled; skipping rebuild ==="
    BUILD_LIBB64=0
fi

echo "=== Compiling Native Libraries for Unit Testing ==="
echo "Project root: ${PROJECT_ROOT}"
echo "Native lib dir: ${NATIVE_LIB_DIR}"
echo "Build dir: ${BUILD_DIR}"

# Function to download and extract source
download_and_extract() {
    local url="$1"
    local filename="$2"
    local extract_dir="$3"

    if [[ ! -f "${BUILD_DIR}/${filename}" ]]; then
        echo "Downloading ${filename}..."
        wget -q -O "${BUILD_DIR}/${filename}" "${url}" || { echo "❌ Download failed" >&2; exit 1; }
    fi

    if [[ ! -d "${BUILD_DIR}/${extract_dir}" ]]; then
        echo "Extracting ${filename}..."
        cd "${BUILD_DIR}"
        if [[ "$filename" == *.tar.gz ]]; then
            tar -xzf "${filename}" || { echo "❌ Extract failed" >&2; exit 1; }
        elif [[ "$filename" == *.tar.bz2 ]]; then
            tar -xjf "${filename}" || { echo "❌ Extract failed" >&2; exit 1; }
        elif [[ "$filename" == *.zip ]]; then
            unzip -q "${filename}" || { echo "❌ Extract failed" >&2; exit 1; }
        fi
    fi
    return 0
}

if [[ "${BUILD_GSOAP}" -eq 1 ]]; then
    echo ""
    echo "=== Compiling gSOAP 2.8.139 ==="
    GSOAP_VERSION="2.8.139"
    GSOAP_URL="https://sourceforge.net/projects/gsoap2/files/gsoap_${GSOAP_VERSION}.zip"
    GSOAP_FILENAME="gsoap_${GSOAP_VERSION}.zip"
    GSOAP_DIR="gsoap-2.8"

    download_and_extract "${GSOAP_URL}" "${GSOAP_FILENAME}" "${GSOAP_DIR}"

    echo "Configuring gSOAP..."
    cd "${BUILD_DIR}/${GSOAP_DIR}"

    # Clear any cross-compilation environment variables
    unset CC
    unset CXX
    unset AR
    unset RANLIB
    unset STRIP

    # Use native compilers (but keep PATH for finding tools)
    export CC=gcc
    export CXX=g++
    export AR=ar
    export RANLIB=ranlib
    export STRIP=strip

    echo "Configuring and building gSOAP with -fPIC for shared library support..."
    export CFLAGS="-fPIC -O2"
    export CXXFLAGS="-fPIC -O2"
    ./configure --prefix="${BUILD_DIR}/gsoap_install" --enable-debug --disable-ssl || { echo "❌ Configure failed" >&2; exit 1; }
    make -j$(nproc) || { echo "❌ Build failed" >&2; exit 1; }
    make install || { echo "❌ Install failed" >&2; exit 1; }

    # Copy only library files to lib directory (no includes - they're in main project)
    echo "Installing gSOAP libraries..."
    mkdir -p "${NATIVE_LIB_DIR}"
    cp "${BUILD_DIR}/gsoap_install/lib"/*.a "${NATIVE_LIB_DIR}/" 2>/dev/null || true
    cp "${BUILD_DIR}/gsoap_install/lib"/*.so* "${NATIVE_LIB_DIR}/" 2>/dev/null || true
else
    echo ""
    echo "=== Using existing gSOAP libraries ==="
fi

if [[ "${BUILD_LIBB64}" -eq 1 ]]; then
    echo ""
    echo "=== Compiling libb64 v2.0.0.1 ==="
    LIBB64_URL="https://github.com/libb64/libb64/archive/refs/tags/v2.0.0.1.zip"
    LIBB64_FILENAME="libb64-2.0.0.1.zip"
    LIBB64_DIR="libb64-2.0.0.1"

    download_and_extract "${LIBB64_URL}" "${LIBB64_FILENAME}" "${LIBB64_DIR}"

    cd "${BUILD_DIR}/${LIBB64_DIR}"

    # Build and install libb64 with -fPIC
    echo "Building libb64 with -fPIC for shared library support..."
    export CFLAGS="-fPIC -O2"
    make -C src || { echo "❌ Build failed" >&2; exit 1; }

    echo "Installing libb64..."
    mkdir -p "${NATIVE_LIB_DIR}"
    cp src/libb64.a "${NATIVE_LIB_DIR}/" || { echo "❌ Library copy failed" >&2; exit 1; }
else
    echo ""
    echo "=== Using existing libb64 library ==="
fi

echo ""
echo "=== Local Libraries Compilation Complete ==="
echo "Libraries installed in: ${NATIVE_LIB_DIR}"
echo "gSOAP libraries: $(ls ${NATIVE_LIB_DIR}/libgsoap* 2>/dev/null | wc -l) files"
echo "libb64 libraries: $(ls ${NATIVE_LIB_DIR}/libb64* 2>/dev/null | wc -l) files"
echo "Ready for unit testing!"
