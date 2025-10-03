# ONVIF Project Makefile
# =======================
#
# Build system with enhanced debugging support
#
# Environment Variables:
#   BUILD_TYPE - Build type: 'debug' or 'release' (default: release)
#
# Build Commands:
#   make                    - Build release version
#   make debug              - Build debug version with symbols (automatically sets BUILD_TYPE=debug)
#   make release            - Explicitly build release version
#   make clean              - Clean build artifacts
#   make docs               - Generate documentation
#   make lint               - Lint all global C source files
#   make lint-check         - Check for linting issues in global source files (exit 1 if issues found)
#   make lint-dry-run       - Show what would be linted (dry run)
#   make lint-format        - Lint with format checking
#   make lint-errors        - Check for errors only
#   make lint-tests         - Lint all C test source files
#   make lint-tests-check   - Check for linting issues in test source files
#   make lint-tests-dry-run - Show what test files would be linted (dry run)
#   make lint-tests-format  - Lint test files with format checking
#   make lint-tests-errors  - Check for errors in test files only
#   make lint-all           - Lint all source files (global + test)
#   make lint-all-check     - Check for linting issues in all source files
#   make format             - Format all C source files
#   make format-check       - Check if files are properly formatted
#
# Examples:
#   make debug              - Automatically enables debug build with symbols
#   make release            - Build optimized release version
#   BUILD_TYPE=debug make - Override defaults for custom builds
#
export PATH := $(PATH):/opt/arm-anykav200-crosstool/usr/bin
PREFIX=arm-anykav200-linux-uclibcgnueabi-
CC=${PREFIX}gcc
CPP=${PREFIX}g++
# Build configuration with environment variable support
BUILD_TYPE ?= release

# Base compiler flags with enhanced security and performance
BASE_CFLAGS=-std=c99 -D_POSIX_C_SOURCE=199309L -DANYKA_PLATFORM=1 -fno-builtin -ffunction-sections -fdata-sections -fstrict-aliasing -fpie -Wl,--hash-style=sysv -DWITH_NO_C_LOCALE

# Optimization and debug flags based on build type
ifeq ($(BUILD_TYPE), debug)
    OPT_CFLAGS=-g -O0 -DDEBUG_BUILD -Wall -Wextra -Wstrict-aliasing -Wstrict-overflow -Wunused
    TARGET_SUFFIX=_debug
else
    OPT_CFLAGS=-Os -Wall -Wextra -Wstrict-aliasing -Wstrict-overflow -Wunused
    TARGET_SUFFIX=
endif

CFLAGS=$(BASE_CFLAGS) $(OPT_CFLAGS)
# gSOAP libraries (libxml2 removed)
GSOAP_LIBS = -lgsoap -lgsoapck

# Base64 library
BASE64_LIBS = -lb64

# Enhanced linker flags with dead code elimination and security
LINKERFLAG=-L./lib -lakuio -lakispsdk -lakv_encode -lplat_common -lplat_thread -lplat_vi -lplat_vpss -lplat_ipcsrv -lplat_venc_cb -lmpi_venc -lakstreamenc -lpthread -lakae -lakaudiocodec -lakmedia -lakv_encode -lapp_net -lapp_rtsp -lmpi_aed -lmpi_aenc -lplat_ai -lplat_drv $(GSOAP_LIBS) $(BASE64_LIBS) -lm

# Updated include paths with generated gSOAP files
INCLUDES=include src

# Output directory for all build artifacts
OUT_DIR=out

SRCS := \
	src/core/main/onvifd.c \
	src/platform/platform_anyka.c \
	src/platform/adapters/ptz_adapter.c \
	src/core/config/config.c \
	src/core/lifecycle/service_manager.c \
	src/core/lifecycle/video_lifecycle.c \
	src/core/lifecycle/network_lifecycle.c \
	src/core/lifecycle/platform_lifecycle.c \
	src/core/lifecycle/config_lifecycle.c \
	src/core/lifecycle/signal_lifecycle.c \
	src/services/common/onvif_service_common.c \
	src/services/common/service_dispatcher.c \
	src/services/device/onvif_device.c \
	src/services/media/onvif_media.c \
	src/services/ptz/onvif_ptz.c \
	src/services/imaging/onvif_imaging.c \
	src/services/snapshot/onvif_snapshot.c \
	src/networking/http/http_server.c \
	src/networking/http/http_parser.c \
	src/networking/http/http_auth.c \
	src/networking/common/connection_manager.c \
	src/networking/common/thread_pool.c \
	src/networking/common/buffer_pool.c \
	src/networking/common/epoll_server.c \
	src/networking/rtsp/rtsp_multistream.c \
	src/networking/rtsp/rtsp_auth.c \
	src/networking/rtsp/rtsp_sdp.c \
	src/networking/rtsp/rtsp_rtcp.c \
	src/networking/rtsp/rtsp_session.c \
	src/networking/rtsp/rtsp_rtp.c \
	src/networking/discovery/ws_discovery.c \
	src/protocol/response/onvif_service_handler.c \
	src/generated/soapC.c \
	src/protocol/gsoap/onvif_gsoap_core.c \
	src/protocol/gsoap/onvif_gsoap_response.c \
	src/protocol/gsoap/onvif_gsoap_media.c \
	src/protocol/gsoap/onvif_gsoap_ptz.c \
	src/protocol/gsoap/onvif_gsoap_device.c \
	src/protocol/gsoap/onvif_gsoap_imaging.c \
	src/utils/common/time_utils.c \
	src/utils/memory/memory_manager.c \
	src/utils/memory/smart_response_builder.c \
	src/utils/string/string_shims.c \
	src/utils/error/error_handling.c \
	src/utils/network/network_utils.c \
	src/utils/logging/logging_utils.c \
	src/utils/logging/service_logging.c \
	src/utils/logging/platform_logging.c \
	src/utils/validation/common_validation.c \
	src/utils/validation/input_validation.c \
	src/utils/validation/audio_validation.c \
	src/utils/security/security_hardening.c \
	src/utils/security/base64_utils.c \
	src/utils/stream/stream_config_utils.c

OBJS := $(SRCS:src/%.c=$(OUT_DIR)/%.o)

.PHONY = all clean debug release docs docs-clean docs-open static-analysis clang-analyze cppcheck-analyze snyk-analyze compile-commands lint lint-check lint-dry-run lint-format lint-errors lint-tests lint-tests-check lint-tests-dry-run lint-tests-format lint-tests-errors lint-all lint-all-check test test-unit test-integration test-list test-coverage test-coverage-html test-coverage-report test-coverage-clean

# Documentation directory
DOCS_DIR = docs

# Limit parallel jobs to prevent memory issues
MAKEFLAGS += -j1

# Main targets
all: $(OUT_DIR)/onvifd

# Debug build with environment variable support
debug:
	@echo "Building debug version with maximum debugging enabled"
	@$(MAKE) clean BUILD_TYPE=debug
	@$(MAKE) $(OUT_DIR)/onvifd_debug BUILD_TYPE=debug

# Documentation targets
docs: $(DOCS_DIR)/html/index.html

docs-clean:
	rm -rf $(DOCS_DIR)

# Generate Doxygen documentation
$(DOCS_DIR)/html/index.html: Doxyfile $(SRCS) $(wildcard src/*.h) $(wildcard include/*.h)
	@echo "Generating Doxygen documentation..."
	@mkdir -p $(DOCS_DIR)
	@doxygen Doxyfile
	@echo "Documentation generated successfully in $(DOCS_DIR)/html/"

# Release target (optimized build)
release:
	@echo "Building release version..."
	@$(MAKE) clean
	@$(MAKE) $(OUT_DIR)/onvifd

$(OUT_DIR)/onvifd: $(OBJS)
	${CC} ${CFLAGS} $(OBJS) ${LINKERFLAG} -o $@

$(OUT_DIR)/onvifd_debug: $(OBJS)
	@echo "Linking debug binary with debug symbols..."
	${CC} ${CFLAGS} $(OBJS) ${LINKERFLAG} -o $@
	@echo "Debug build completed: $@"

$(OUT_DIR)/%.o: src/%.c | $(OUT_DIR)
	@mkdir -p $(dir $@)
	${CC} ${CFLAGS} ${INCLUDES:%=-I%} -c $< -o $@ -pipe

$(OUT_DIR):
	mkdir -p $(OUT_DIR)

# Static analysis targets
static-analysis: clang-analyze cppcheck-analyze snyk-analyze
	@echo "All static analysis completed!"

clang-analyze:
	@echo "Running Clang Static Analyzer..."
	@mkdir -p analysis-results/clang
	@find src -name '*.c' -exec clang --analyze -Xanalyzer -analyzer-output=html -Xanalyzer -analyzer-output-dir=analysis-results/clang {} \;
	@echo "Clang analysis completed. Results in analysis-results/clang/"

cppcheck-analyze:
	@echo "Running Cppcheck..."
	@mkdir -p analysis-results
	@cppcheck --enable=all --xml --output-file=analysis-results/cppcheck-results.xml --suppress=missingIncludeSystem src/
	@echo "Cppcheck analysis completed. Results in analysis-results/cppcheck-results.xml"

snyk-analyze:
	@echo "Running Snyk Code Analysis..."
	@mkdir -p analysis-results
	@snyk code test --json --json-file-output=analysis-results/snyk-results.json src/
	@snyk code test --sarif --sarif-file-output=analysis-results/snyk-results.sarif src/
	@echo "Snyk analysis completed. Results in analysis-results/"

# Generate compile_commands.json for clangd
compile-commands:
	@echo "Generating compile_commands.json for main project..."
	@./scripts/generate_compile_commands.sh
	@echo "Generating compile_commands.json for test files..."
	@$(MAKE) -C tests compile-commands

# Code formatting targets
format:
	@echo "Formatting all C source files..."
	@./scripts/format_code.sh

format-check:
	@echo "Checking code formatting..."
	@./scripts/format_code.sh --check

format-dry-run:
	@echo "Showing what would be formatted (dry run)..."
	@./scripts/format_code.sh --dry-run


# Code linting targets
lint:
	@echo "Linting all global C source files..."
	@./scripts/lint_code.sh

lint-check:
	@echo "Checking for linting issues in global source files..."
	@./scripts/lint_code.sh --check

lint-dry-run:
	@echo "Showing what would be linted (dry run)..."
	@./scripts/lint_code.sh --dry-run

lint-format:
	@echo "Linting with format checking..."
	@./scripts/lint_code.sh --format

lint-errors:
	@echo "Checking for linting errors only..."
	@./scripts/lint_code.sh --severity error

# Test code linting targets
lint-tests:
	@echo "Linting all C test source files..."
	@./tests/scripts/lint_test_code.sh

lint-tests-check:
	@echo "Checking for linting issues in test source files..."
	@./tests/scripts/lint_test_code.sh --check

lint-tests-dry-run:
	@echo "Showing what test files would be linted (dry run)..."
	@./tests/scripts/lint_test_code.sh --dry-run

lint-tests-format:
	@echo "Linting test files with format checking..."
	@./tests/scripts/lint_test_code.sh --format

lint-tests-errors:
	@echo "Checking for linting errors in test files only..."
	@./tests/scripts/lint_test_code.sh --severity error

# Lint all (global + test files)
lint-all: lint lint-tests
	@echo "Linting completed for all source files"

lint-all-check: lint-check lint-tests-check
	@echo "Linting check completed for all source files"

# Testing targets with optional SUITE parameter
# Usage:
#   make test                    - Run all tests (196 tests)
#   make test SUITE=ptz-service  - Run specific suite
#   make test-unit               - Run all unit tests (129 tests)
#   make test-unit SUITE=http-*  - Run specific unit test suites
test:
ifdef SUITE
	@echo "Running tests for suite: $(SUITE)..."
	@$(MAKE) -C tests test SUITE=$(SUITE)
else
	@echo "Running all tests..."
	@$(MAKE) -C tests test
endif

test-unit:
ifdef SUITE
	@echo "Running unit tests for suite: $(SUITE)..."
	@$(MAKE) -C tests test-unit SUITE=$(SUITE)
else
	@echo "Running unit tests..."
	@$(MAKE) -C tests test-unit
endif

test-integration:
ifdef SUITE
	@echo "Running integration tests for suite: $(SUITE)..."
	@$(MAKE) -C tests test-integration SUITE=$(SUITE)
else
	@echo "Running integration tests..."
	@$(MAKE) -C tests test-integration
endif

test-list:
	@echo "Listing available test suites..."
	@$(MAKE) -C tests test-list

test-coverage:
	@echo "Running tests with coverage analysis..."
	@$(MAKE) -C tests coverage


test-coverage-html:
	@echo "Generating HTML coverage report..."
	@$(MAKE) -C tests coverage-html

test-coverage-report:
	@echo "Generating coverage summary..."
	@$(MAKE) -C tests coverage-report

test-coverage-clean:
	@echo "Cleaning coverage files..."
	@$(MAKE) -C tests coverage-clean

clean:
	rm -rf $(OUT_DIR)
	@$(MAKE) -C tests clean
