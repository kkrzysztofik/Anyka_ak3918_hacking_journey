# ONVIF Project Test Makefile
# ===========================
#
# Unit testing build system using CMocka framework
#
# Build Commands:
#   make                    - Build all tests
#   make test              - Build and run all tests
#   make test-utils        - Build and run utility tests
#   make test-services     - Build and run service tests
#   make test-networking   - Build and run networking tests
#   make test-integration  - Build and run integration tests
#   make clean             - Clean test build artifacts
#   make coverage          - Build with coverage analysis
#   make valgrind          - Run tests with valgrind
#
# Environment Variables:
#   BUILD_TYPE - Build type: 'debug' or 'release' (default: debug for tests)
#   VERBOSE    - Verbose output: '1' for verbose, '0' for quiet (default: 0)

# Native compilation for unit tests (development machine)
# Use native GCC for testing on development machine
CC=gcc
CPP=g++

# Build configuration
BUILD_TYPE ?= debug
VERBOSE ?= 0

# Base compiler flags for native testing
BASE_CFLAGS=-std=c99 -D_POSIX_C_SOURCE=199309L -DTEST_BUILD -fno-builtin -fstrict-aliasing -Wall -Wextra

# Test-specific flags
TEST_CFLAGS=-DUNIT_TESTING -DPLATFORM_MOCK -DWITH_NO_C_LOCALE

# Optimization and debug flags
ifeq ($(BUILD_TYPE), debug)
    OPT_CFLAGS=-g -O0 -DDEBUG_BUILD -Wall -Wextra -Wstrict-aliasing -Wstrict-overflow -Wunused
    TARGET_SUFFIX=_debug
else
    OPT_CFLAGS=-Os -Wall -Wextra -Wstrict-aliasing -Wstrict-overflow -Wunused
    TARGET_SUFFIX=
endif

# Coverage flags
COVERAGE_CFLAGS=-fprofile-arcs -ftest-coverage
COVERAGE_LDFLAGS=-lgcov

# Coverage tools
LCOV=lcov
GENHTML=genhtml
COVERAGE_DIR=coverage
COVERAGE_INFO=$(COVERAGE_DIR)/coverage.info
COVERAGE_HTML=$(COVERAGE_DIR)/html

# Combined flags
CFLAGS=$(BASE_CFLAGS) $(OPT_CFLAGS) $(TEST_CFLAGS)

# Include paths (add native includes if available)
INCLUDES=../include ../src src $(NATIVE_INCLUDES)

# Output directory
OUT_DIR=out

# Native libraries for unit testing (prefer native over cross-compiled)
NATIVE_LIB_DIR = lib
NATIVE_LIB_AVAILABLE = $(shell test -d $(NATIVE_LIB_DIR) && test -f $(NATIVE_LIB_DIR)/libgsoap.a && echo "yes" || echo "no")

# gSOAP libraries (use native if available, otherwise fall back to cross-compiled)
ifeq ($(NATIVE_LIB_AVAILABLE), yes)
    GSOAP_LIBS = -L$(NATIVE_LIB_DIR) -lgsoap -lgsoapck
    BASE64_LIBS = -L$(NATIVE_LIB_DIR) -lb64
    NATIVE_INCLUDES = -I../src
    $(info Using native libraries for unit testing)
else
    GSOAP_LIBS = -L../lib -lgsoap -lgsoapck
    BASE64_LIBS = -L../lib -lb64
    NATIVE_INCLUDES =
    $(warning Native libraries not found. Run ./scripts/compile_local_libs.sh to build them for unit testing)
endif

# CMocka library for native testing
CMOCKA_LIBS=-lcmocka -lpthread -lm $(GSOAP_LIBS) $(BASE64_LIBS)

# Unit test source files (individual function testing only)
TEST_SRCS := \
	src/runner/test_runner.c \
	src/unit/networking/test_http_auth.c \
	src/unit/utils/test_simple.c \
	src/unit/utils/test_memory_utils.c \
	src/unit/utils/test_logging_utils.c \
	src/unit/protocol/test_onvif_gsoap.c \
	src/unit/services/common/test_service_dispatcher.c \
	src/unit/services/media/test_media_utils.c \
	src/unit/services/media/test_onvif_media_callbacks.c \
	src/unit/services/ptz/test_ptz_service.c \
	src/unit/services/ptz/test_onvif_ptz_callbacks.c \

# Mock source files
MOCK_SRCS := \
	src/mocks/platform_mock.c \
	src/mocks/network_mock.c \
	src/mocks/platform_ptz_mock.c \
	src/mocks/service_handler_mock.c \
	src/mocks/mock_service_dispatcher.c

# Source files from main project (for testing - exclude platform_anyka.c)
MAIN_SRCS := \
	../src/utils/memory/memory_manager.c \
	../src/utils/memory/smart_response_builder.c \
	../src/protocol/gsoap/onvif_gsoap.c \
	../src/generated/soapC.c \
	../src/utils/logging/logging_utils.c \
	../src/utils/logging/service_logging.c \
	../src/networking/http/http_auth.c \
	../src/utils/validation/common_validation.c \
	../src/utils/validation/input_validation.c \
	../src/utils/security/base64_utils.c \
	../src/utils/string/string_shims.c \
	../src/services/common/service_dispatcher.c \
	../src/services/common/onvif_service_common.c \
	../src/services/media/onvif_media.c \
	../src/services/ptz/onvif_ptz.c \
	../src/utils/network/network_utils.c \
	../src/utils/error/error_handling.c

# Object files
TEST_OBJS := $(TEST_SRCS:src/%.c=$(OUT_DIR)/%.o)
MOCK_OBJS := $(MOCK_SRCS:src/%.c=$(OUT_DIR)/%.o)
MAIN_OBJS := $(MAIN_SRCS:../src/%.c=$(OUT_DIR)/main/%.o)

# Unit test executables
TEST_TARGETS := \
	$(OUT_DIR)/test_utils \
	$(OUT_DIR)/test_all

.PHONY: all clean test test-utils test-all coverage coverage-html coverage-report coverage-clean valgrind debug release compile-native-libs compile-commands

# Default target
all: $(TEST_TARGETS)

# Debug build
debug:
	@echo "Building debug version of tests..."
	@$(MAKE) clean BUILD_TYPE=debug
	@$(MAKE) all BUILD_TYPE=debug

# Release build
release:
	@echo "Building release version of tests..."
	@$(MAKE) clean BUILD_TYPE=release
	@$(MAKE) all BUILD_TYPE=release

# Coverage build
coverage: CFLAGS += $(COVERAGE_CFLAGS)
coverage: CMOCKA_LIBS += $(COVERAGE_LDFLAGS)
coverage: clean all
	@echo "Coverage build completed"

# Generate HTML coverage report
coverage-html: coverage
	@echo "Generating HTML coverage report..."
	@mkdir -p $(COVERAGE_DIR)
	@$(LCOV) --capture --directory $(OUT_DIR) --output-file $(COVERAGE_INFO)
	@$(LCOV) --remove $(COVERAGE_INFO) '/usr/*' --output-file $(COVERAGE_INFO)
	@$(LCOV) --remove $(COVERAGE_INFO) '*/tests/*' --output-file $(COVERAGE_INFO)
	@$(GENHTML) $(COVERAGE_INFO) --output-directory $(COVERAGE_HTML)
	@echo "HTML coverage report generated in $(COVERAGE_HTML)/index.html"

# Generate coverage summary report
coverage-report: coverage
	@echo "Generating coverage summary..."
	@mkdir -p $(COVERAGE_DIR)
	@$(LCOV) --capture --directory $(OUT_DIR) --output-file $(COVERAGE_INFO)
	@$(LCOV) --remove $(COVERAGE_INFO) '/usr/*' --output-file $(COVERAGE_INFO)
	@$(LCOV) --remove $(COVERAGE_INFO) '*/tests/*' --output-file $(COVERAGE_INFO)
	@echo "Coverage Summary:"
	@$(LCOV) --summary $(COVERAGE_INFO)

# Clean coverage files
coverage-clean:
	@echo "Cleaning coverage files..."
	@rm -rf $(COVERAGE_DIR)
	@rm -f *.gcda *.gcno *.gcov
	@if [ -d "$(OUT_DIR)" ]; then \
		find $(OUT_DIR) -name "*.gcda" -delete; \
		find $(OUT_DIR) -name "*.gcno" -delete; \
	fi

# Unit test targets
test: all
	@echo "Running all unit tests..."
	@for test in $(TEST_TARGETS); do \
		if [ -f $$test ]; then \
			echo "Running $$test..."; \
			$$test; \
		fi; \
	done

test-utils: $(OUT_DIR)/test_utils
	@echo "Running utility unit tests..."
	@$(OUT_DIR)/test_utils

test-all: $(OUT_DIR)/test_all
	@echo "Running all unit tests..."
	@$(OUT_DIR)/test_all

# Valgrind testing (requires valgrind)
valgrind: all
	@echo "Running tests with valgrind..."
	@for test in $(TEST_TARGETS); do \
		if [ -f $$test ]; then \
			echo "Running valgrind on $$test..."; \
			valgrind --leak-check=full --show-leak-kinds=all $$test; \
		fi; \
	done

# Unit test executable builds
$(OUT_DIR)/test_utils: $(TEST_OBJS) $(MOCK_OBJS) $(MAIN_OBJS)
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) $^ $(CMOCKA_LIBS) -o $@

$(OUT_DIR)/test_all: $(TEST_OBJS) $(MOCK_OBJS) $(MAIN_OBJS)
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) $^ $(CMOCKA_LIBS) -o $@

# Object file compilation
$(OUT_DIR)/%.o: src/%.c | $(OUT_DIR)
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) $(INCLUDES:%=-I%) -c $< -o $@ -pipe

# Main project object files (exclude platform_anyka.c)
$(OUT_DIR)/main/%.o: ../src/%.c | $(OUT_DIR)
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) $(INCLUDES:%=-I%) -c $< -o $@ -pipe

# Create output directory
$(OUT_DIR):
	mkdir -p $(OUT_DIR)

# Clean build artifacts
clean: coverage-clean
	rm -rf $(OUT_DIR)

# Compile native libraries for unit testing
compile-native-libs:
	@echo "Compiling native gsoap and libb64 libraries for unit testing..."
	@./scripts/compile_local_libs.sh

# Generate compile_commands.json for test files
compile-commands:
	@echo "Generating compile_commands.json for test files..."
	@./scripts/generate_compile_commands.sh

# Help target
help:
	@echo "Available targets:"
	@echo "  all              - Build all tests"
	@echo "  test             - Build and run all tests"
	@echo "  test-utils       - Build and run utility unit tests"
	@echo "  test-all         - Run all unit tests"
	@echo "  coverage         - Build with coverage analysis"
	@echo "  coverage-html    - Generate HTML coverage report"
	@echo "  coverage-report  - Generate coverage summary"
	@echo "  coverage-clean   - Clean coverage files"
	@echo "  valgrind         - Run tests with valgrind"
	@echo "  debug            - Build debug version"
	@echo "  release          - Build release version"
	@echo "  compile-native-libs - Compile native gsoap and libb64 libraries"
	@echo "  compile-commands  - Generate compile_commands.json for test files"
	@echo "  clean            - Clean build artifacts"
	@echo "  help             - Show this help"
