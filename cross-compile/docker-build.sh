#!/bin/bash
# Build script for Anyka cross-compilation Docker image
# Builds the Docker container with ARM cross-compilation toolchain

set -e

# =============================================================================
# Configuration and Constants
# =============================================================================

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Project root (parent of cross-compile directory)
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Default values
DOCKERFILE="${SCRIPT_DIR}/Dockerfile"
IMAGE_TAG="anyka-cross-compile"
BUILD_ARGS=""
NO_CACHE=false

# Colors for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m' # No Color

# =============================================================================
# Logging Functions
# =============================================================================

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# =============================================================================
# Utility Functions
# =============================================================================

# Check if a command exists
command_exists() {
    command -v "$1" &> /dev/null
}

# Print usage information
print_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Build the Anyka cross-compilation Docker image with ARM toolchain.

OPTIONS:
  -h, --help              Show this help message
  -t, --tag TAG           Docker image tag (default: anyka-cross-compile)
  -f, --file FILE         Dockerfile path (default: cross-compile/Dockerfile)
  --no-cache              Build without using cache
  --build-arg KEY=VALUE   Pass build argument to docker build

EXAMPLES:
  $0                                    # Build with default settings
  $0 --tag my-toolchain                  # Build with custom tag
  $0 --no-cache                          # Build without cache
  $0 --build-arg BUILDKIT_INLINE_CACHE=1 # Pass build argument

After building, use the image:
  docker run -it --rm -v \${PWD}:/workspace ${IMAGE_TAG}
  docker run --rm -v \${PWD}:/workspace ${IMAGE_TAG} make -C /workspace/cross-compile/onvif
EOF
}

# =============================================================================
# Validation Functions
# =============================================================================

check_prerequisites() {
    log_info "Checking prerequisites..."

    if ! command_exists docker; then
        log_error "Docker is not installed or not in PATH"
        log_info "Please install Docker: https://docs.docker.com/get-docker/"
        exit 1
    fi

    if ! docker info &> /dev/null; then
        log_error "Docker daemon is not running"
        log_info "Please start Docker daemon and try again"
        exit 1
    fi

    if [ ! -f "${DOCKERFILE}" ]; then
        log_error "Dockerfile not found: ${DOCKERFILE}"
        exit 1
    fi

    log_success "Prerequisites check passed"
}

# =============================================================================
# Argument Parsing
# =============================================================================

parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                print_usage
                exit 0
                ;;
            -t|--tag)
                IMAGE_TAG="$2"
                shift 2
                ;;
            -f|--file)
                DOCKERFILE="$2"
                shift 2
                ;;
            --no-cache)
                NO_CACHE=true
                shift
                ;;
            --build-arg)
                BUILD_ARGS="${BUILD_ARGS} --build-arg $2"
                shift 2
                ;;
            *)
                log_error "Unknown option: $1"
                echo "Use --help for usage information"
                exit 1
                ;;
        esac
    done
}

# =============================================================================
# Main Build Function
# =============================================================================

build_docker_image() {
    log_info "Building Anyka cross-compilation Docker image..."
    log_info "Dockerfile: ${DOCKERFILE}"
    log_info "Image tag: ${IMAGE_TAG}"
    log_info "Project root: ${PROJECT_ROOT}"

    # Build docker command
    local docker_cmd="docker build"

    if [ "${NO_CACHE}" = true ]; then
        docker_cmd="${docker_cmd} --no-cache"
        log_info "Building without cache"
    fi

    if [ -n "${BUILD_ARGS}" ]; then
        docker_cmd="${docker_cmd} ${BUILD_ARGS}"
    fi

    docker_cmd="${docker_cmd} -f ${DOCKERFILE} -t ${IMAGE_TAG} ${PROJECT_ROOT}"

    log_info "Executing: ${docker_cmd}"
    echo ""

    # Execute docker build
    if ${docker_cmd}; then
        log_success "Docker image built successfully!"
        echo ""
        log_info "Usage examples:"
        echo "  Interactive shell:"
        echo "    docker run -it --rm -v \${PWD}:/workspace ${IMAGE_TAG}"
        echo ""
        echo "  Build C ONVIF project:"
        echo "    docker run --rm -v \${PWD}:/workspace ${IMAGE_TAG} make -C /workspace/cross-compile/onvif"
        echo ""
        echo "  Build Rust ONVIF project:"
        echo "    docker run --rm -v \${PWD}:/workspace ${IMAGE_TAG} bash -c 'cd /workspace/cross-compile/onvif-rust && ./scripts/build.sh'"
        echo ""
        echo "  Run any command:"
        echo "    docker run --rm -v \${PWD}:/workspace ${IMAGE_TAG} <command>"
        return 0
    else
        log_error "Docker build failed"
        return 1
    fi
}

# =============================================================================
# Main Execution
# =============================================================================

main() {
    parse_arguments "$@"
    check_prerequisites
    build_docker_image
}

# Run main function
main "$@"
