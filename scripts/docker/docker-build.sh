#!/bin/bash
# Build script for Anyka cross-compilation Docker image
# Builds the Docker container with ARM cross-compilation toolchain

set -euo pipefail

# =============================================================================
# Configuration and Constants
# =============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "${SCRIPT_DIR}/../common.sh"
PROJECT_ROOT="${ANYKA_REPO_ROOT}"

# Default values
DOCKERFILE="${SCRIPT_DIR}/Dockerfile"
IMAGE_TAG="anyka-cross-compile"
BUILD_ARGS=""
NO_CACHE=false
CI_IMAGE=false

# =============================================================================
# Utility Functions
# =============================================================================

# Print usage information
print_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Build the Anyka cross-compilation Docker image with ARM toolchain.

OPTIONS:
  -h, --help              Show this help message
  -t, --tag TAG           Docker image tag (default: anyka-cross-compile)
  -f, --file FILE         Dockerfile path (default: scripts/docker/Dockerfile)
  --ci                    Build slim host-CI image (Dockerfile.ci; runs prepare-ci-toolchain.sh)
  --no-cache              Build without using cache
  --build-arg KEY=VALUE   Pass build argument to docker build

EXAMPLES:
  $0                                    # Build full cross-compile image
  $0 --ci -t kkrzysztofik/anyka-cross-compile:rust-1.97.1-ci
  $0 --tag my-toolchain                  # Build with custom tag
  $0 --no-cache                          # Build without cache

After building, use the image:
  docker run -it --rm -v \${PWD}:/workspace ${IMAGE_TAG}
  docker run --rm -v \${PWD}:/workspace ${IMAGE_TAG} make -C /workspace/cross-compile/onvif
EOF
    return 0
}

# =============================================================================
# Validation Functions
# =============================================================================

check_prerequisites() {
    log_info "Checking prerequisites..."

    if ! command -v docker &>/dev/null; then
        log_error "Docker is not installed or not in PATH"
        log_info "Please install Docker: https://docs.docker.com/get-docker/"
        exit 1
    fi

    if ! docker info &>/dev/null; then
        log_error "Docker daemon is not running"
        log_info "Please start Docker daemon and try again"
        exit 1
    fi

    if [[ ! -f "${DOCKERFILE}" ]]; then
        log_error "Dockerfile not found: ${DOCKERFILE}"
        exit 1
    fi

    log_success "Prerequisites check passed"
    return 0
}

# =============================================================================
# Argument Parsing
# =============================================================================

parse_arguments() {
    while [[ $# -gt 0 ]]; do
        local arg="$1"
        case "${arg}" in
            -h|--help)
                print_usage
                exit 0
                ;;
            -t|--tag)
                local tag="$2"
                IMAGE_TAG="${tag}"
                shift 2
                ;;
            -f|--file)
                local file="$2"
                DOCKERFILE="${file}"
                shift 2
                ;;
            --ci)
                CI_IMAGE=true
                DOCKERFILE="${SCRIPT_DIR}/Dockerfile.ci"
                if [[ "${IMAGE_TAG}" = "anyka-cross-compile" ]]; then
                    IMAGE_TAG="kkrzysztofik/anyka-cross-compile:rust-1.97.1-ci"
                fi
                shift
                ;;
            --no-cache)
                NO_CACHE=true
                shift
                ;;
            --build-arg)
                local build_arg="$2"
                BUILD_ARGS="${BUILD_ARGS} --build-arg ${build_arg}"
                shift 2
                ;;
            *)
                log_error "Unknown option: ${arg}"
                echo "Use --help for usage information" >&2
                exit 1
                ;;
        esac
    done
    return 0
}

# =============================================================================
# Main Build Function
# =============================================================================

build_docker_image() {
    if [[ "${CI_IMAGE}" = true ]]; then
        log_info "Preparing staged host-CI toolchain..."
        bash "${SCRIPT_DIR}/prepare-ci-toolchain.sh"
        log_info "Building Anyka host-CI Docker image..."
    else
        log_info "Building Anyka cross-compilation Docker image..."
    fi
    log_info "Dockerfile: ${DOCKERFILE}"
    log_info "Image tag: ${IMAGE_TAG}"
    log_info "Project root: ${PROJECT_ROOT}"

    # Build docker command
    local docker_cmd="docker build"

    if [[ "${NO_CACHE}" = true ]]; then
        docker_cmd="${docker_cmd} --no-cache"
        log_info "Building without cache"
    fi

    if [[ -n "${BUILD_ARGS}" ]]; then
        docker_cmd="${docker_cmd} ${BUILD_ARGS}"
    fi

    local build_context="${PROJECT_ROOT}"
    if [[ "${CI_IMAGE}" = true ]]; then
        # Narrow context avoids sending the full toolchain tree to the daemon.
        build_context="${SCRIPT_DIR}"
    fi

    docker_cmd="${docker_cmd} -f ${DOCKERFILE} -t ${IMAGE_TAG} ${build_context}"

    log_info "Executing: ${docker_cmd}"
    echo ""

    # Execute docker build
    if ${docker_cmd}; then
        log_success "Docker image built successfully!"
        echo ""
        log_info "Usage examples:"
        if [[ "${CI_IMAGE}" = true ]]; then
            echo "  Interactive shell (host Rust fmt/clippy/llvm-cov):"
            echo "    docker run -it --rm -v \${PWD}:/workspace ${IMAGE_TAG}"
            echo ""
            echo "  Clippy (matches CI rust-lint job):"
            echo "    docker run --rm -v \${PWD}:/workspace ${IMAGE_TAG} bash -c 'cd /workspace/cross-compile && cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings'"
            echo ""
            echo "  Coverage (matches CI rust-coverage job):"
            echo "    docker run --rm -v \${PWD}:/workspace ${IMAGE_TAG} bash -c 'cd /workspace/cross-compile && cargo llvm-cov --workspace --target x86_64-unknown-linux-gnu --all-features --cobertura --output-path coverage/cobertura.xml'"
            echo ""
            echo "  Push to registry after rebuild:"
            echo "    docker push ${IMAGE_TAG}"
        else
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
        fi
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
    local args=("$@")
    parse_arguments "${args[@]}"
    check_prerequisites
    build_docker_image
    return 0
}

# Run main function
main "$@"
