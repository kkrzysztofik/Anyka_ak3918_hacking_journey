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
#
# Environment Variables:
#   BUILD_TYPE - Build type: 'debug' or 'release' (default: debug for tests)
#   VERBOSE    - Verbose output: '1' for verbose, '0' for quiet (default: 0)

# Native compilation for unit tests (development machine)
# Use native GCC with ccache for faster incremental builds
CC := ccache gcc
CPP := ccache g++

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

# Dependency generation flags
DEPFLAGS=-MT $@ -MMD -MP -MF $*.d

# Combined flags
CFLAGS=$(BASE_CFLAGS) $(OPT_CFLAGS) $(TEST_CFLAGS)

# Include paths (add native includes if available)
INCLUDES=../include ../src src $(NATIVE_INCLUDES)

# Output directory
OUT_DIR=out

# Parallel builds with load average limiting to prevent memory issues
# Use all available cores but limit based on load average
MAKEFLAGS += -j$(shell nproc) -l$(shell echo "$$(nproc) * 1.5" | bc)

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

# Modular test runner source files
RUNNER_UTILS_SRC = src/runner/test_utils_runner.c
RUNNER_NETWORKING_SRC = src/runner/test_networking_runner.c
RUNNER_PROTOCOL_SRC = src/runner/test_protocol_runner.c
RUNNER_DISPATCHER_SRC = src/runner/test_service_dispatcher_runner.c
RUNNER_PTZ_SRC = src/runner/test_ptz_runner.c
RUNNER_MEDIA_SRC = src/runner/test_media_runner.c
RUNNER_IMAGING_SRC = src/runner/test_imaging_runner.c
RUNNER_INTEGRATION_SRC = src/runner/test_integration_runner.c

# New dynamic test runner with filtering
RUNNER_DYNAMIC_SRC = src/runner/test_runner.c

# Test suite registry sources
# Test suite registry sources for dynamic test runner
TEST_SUITE_REGISTRY_SRCS := \
	src/common/test_suites.c \
	src/unit/test_basic.c \
	src/unit/networking/test_http_metrics_suite.c \
	src/unit/protocol/test_gsoap_protocol_suite.c \
	src/integration/ptz_integration_suite.c \
	src/integration/media_integration_suite.c \
	src/integration/device_integration_suite.c \
	src/integration/imaging_integration_suite.c
# Legacy consolidated test runner (kept for compatibility - DEPRECATED)
RUNNER_LEGACY_SRC = src/runner/test_runner.c.old

# Test source files for each module
UTILS_TEST_SRCS := \
	src/unit/utils/test_memory_utils.c \
	src/unit/utils/test_logging_utils.c

NETWORKING_TEST_SRCS := \
	src/unit/networking/test_http_auth.c \
	src/unit/networking/test_http_metrics_simple.c

PROTOCOL_TEST_SRCS := \
	src/unit/protocol/test_onvif_gsoap.c

DISPATCHER_TEST_SRCS := \
	src/unit/services/common/test_service_dispatcher.c

# DEVICE_TEST_SRCS := \
# 	src/unit/services/device/test_device_service.c

PTZ_TEST_SRCS := \
	src/unit/services/ptz/test_ptz_service.c \
	src/unit/services/ptz/test_onvif_ptz_callbacks.c

MEDIA_TEST_SRCS := \
	src/unit/services/media/test_media_utils.c \
	src/unit/services/media/test_onvif_media_callbacks.c

IMAGING_TEST_SRCS := \
	src/unit/services/imaging/test_onvif_imaging_callbacks.c

INTEGRATION_TEST_SRCS := \
	src/integration/ptz_service_tests.c \
	src/integration/media_service_tests.c \
	src/integration/imaging_service_optimization_test.c \
	src/integration/device_service_tests.c \
	src/integration/soap_error_tests.c

# All test sources (for legacy runner)
TEST_SRCS := $(UTILS_TEST_SRCS) $(NETWORKING_TEST_SRCS) $(PROTOCOL_TEST_SRCS) \
             $(DISPATCHER_TEST_SRCS) $(DEVICE_TEST_SRCS) $(PTZ_TEST_SRCS) $(MEDIA_TEST_SRCS) \
             $(IMAGING_TEST_SRCS) $(INTEGRATION_TEST_SRCS)

# Test helper library source files
TEST_HELPER_SRCS := \
	src/common/test_helpers.c \
	src/common/time_utils.c \
	src/common/generic_mock_framework.c \
	src/common/soap_test_helpers.c

# Mock source files (for unit tests)
MOCK_SRCS := \
	src/mocks/platform_mock.c \
	src/mocks/network_mock.c \
	src/mocks/platform_ptz_mock.c \
	src/mocks/service_handler_mock.c \
	src/mocks/mock_service_dispatcher.c \
	src/mocks/config_mock.c \
	src/mocks/gsoap_mock.c \
	src/mocks/buffer_pool_mock.c \
	src/mocks/smart_response_mock.c

# Mock source files for integration tests (exclude buffer_pool_mock - use real implementation)
MOCK_SRCS_INTEGRATION := \
	src/mocks/platform_mock.c \
	src/mocks/network_mock.c \
	src/mocks/platform_ptz_mock.c \
	src/mocks/service_handler_mock.c \
	src/mocks/mock_service_dispatcher.c \
	src/mocks/config_mock.c \
	src/mocks/gsoap_mock.c \
	src/mocks/smart_response_mock.c

# Source files from main project (for testing - exclude platform_anyka.c)
MAIN_SRCS := \
	../src/utils/memory/memory_manager.c \
	../src/utils/common/time_utils.c \
	../src/utils/memory/smart_response_builder.c \
	../src/protocol/gsoap/onvif_gsoap_core.c \
	../src/protocol/gsoap/onvif_gsoap_response.c \
	../src/protocol/gsoap/onvif_gsoap_media.c \
	../src/protocol/gsoap/onvif_gsoap_ptz.c \
	../src/protocol/gsoap/onvif_gsoap_device.c \
	../src/protocol/gsoap/onvif_gsoap_imaging.c \
	../src/generated/soapC.c \
	../src/utils/logging/logging_utils.c \
	../src/utils/logging/service_logging.c \
	../src/networking/http/http_auth.c \
	../src/networking/http/http_parser.c \
	../src/networking/common/buffer_pool.c \
	../src/utils/validation/common_validation.c \
	../src/utils/validation/input_validation.c \
	../src/utils/security/base64_utils.c \
	../src/utils/security/security_hardening.c \
	../src/utils/string/string_shims.c \
	../src/services/common/service_dispatcher.c \
	../src/services/common/onvif_service_common.c \
	../src/services/device/onvif_device.c \
	../src/services/media/onvif_media.c \
	../src/services/ptz/onvif_ptz.c \
	../src/services/imaging/onvif_imaging.c \
	../src/utils/network/network_utils.c \
	../src/utils/error/error_handling.c

# Object files for each module
UTILS_TEST_OBJS := $(UTILS_TEST_SRCS:src/%.c=$(OUT_DIR)/%.o)
NETWORKING_TEST_OBJS := $(NETWORKING_TEST_SRCS:src/%.c=$(OUT_DIR)/%.o)
PROTOCOL_TEST_OBJS := $(PROTOCOL_TEST_SRCS:src/%.c=$(OUT_DIR)/%.o)
DISPATCHER_TEST_OBJS := $(DISPATCHER_TEST_SRCS:src/%.c=$(OUT_DIR)/%.o)
DEVICE_TEST_OBJS := $(DEVICE_TEST_SRCS:src/%.c=$(OUT_DIR)/%.o)
PTZ_TEST_OBJS := $(PTZ_TEST_SRCS:src/%.c=$(OUT_DIR)/%.o)
MEDIA_TEST_OBJS := $(MEDIA_TEST_SRCS:src/%.c=$(OUT_DIR)/%.o)
IMAGING_TEST_OBJS := $(IMAGING_TEST_SRCS:src/%.c=$(OUT_DIR)/%.o)
INTEGRATION_TEST_OBJS := $(INTEGRATION_TEST_SRCS:src/%.c=$(OUT_DIR)/%.o)

# Runner object files
RUNNER_UTILS_OBJ := $(RUNNER_UTILS_SRC:src/%.c=$(OUT_DIR)/%.o)
RUNNER_NETWORKING_OBJ := $(RUNNER_NETWORKING_SRC:src/%.c=$(OUT_DIR)/%.o)
RUNNER_PROTOCOL_OBJ := $(RUNNER_PROTOCOL_SRC:src/%.c=$(OUT_DIR)/%.o)
RUNNER_DISPATCHER_OBJ := $(RUNNER_DISPATCHER_SRC:src/%.c=$(OUT_DIR)/%.o)
RUNNER_PTZ_OBJ := $(RUNNER_PTZ_SRC:src/%.c=$(OUT_DIR)/%.o)
RUNNER_MEDIA_OBJ := $(RUNNER_MEDIA_SRC:src/%.c=$(OUT_DIR)/%.o)
RUNNER_IMAGING_OBJ := $(RUNNER_IMAGING_SRC:src/%.c=$(OUT_DIR)/%.o)
RUNNER_INTEGRATION_OBJ := $(RUNNER_INTEGRATION_SRC:src/%.c=$(OUT_DIR)/%.o)
RUNNER_DYNAMIC_OBJ := $(RUNNER_DYNAMIC_SRC:src/%.c=$(OUT_DIR)/%.o)
RUNNER_LEGACY_OBJ := $(RUNNER_LEGACY_SRC:src/%.c=$(OUT_DIR)/%.o)

# Test suite registry object files
TEST_SUITE_REGISTRY_OBJS := $(TEST_SUITE_REGISTRY_SRCS:src/%.c=$(OUT_DIR)/%.o)

# Test helper library objects
TEST_HELPER_OBJS := $(TEST_HELPER_SRCS:src/%.c=$(OUT_DIR)/%.o)

# Mock and main project objects
MOCK_OBJS := $(MOCK_SRCS:src/%.c=$(OUT_DIR)/%.o)
MOCK_OBJS_INTEGRATION := $(MOCK_SRCS_INTEGRATION:src/%.c=$(OUT_DIR)/%.o)
MAIN_OBJS := $(MAIN_SRCS:../src/%.c=$(OUT_DIR)/main/%.o)

# All test objects (for dependency tracking)
TEST_OBJS := $(UTILS_TEST_OBJS) $(NETWORKING_TEST_OBJS) $(PROTOCOL_TEST_OBJS) \
             $(DISPATCHER_TEST_OBJS) $(DEVICE_TEST_OBJS) $(PTZ_TEST_OBJS) $(MEDIA_TEST_OBJS) \
             $(IMAGING_TEST_OBJS) $(INTEGRATION_TEST_OBJS)

# Dependency files for incremental builds
TEST_DEPS := $(TEST_OBJS:$(OUT_DIR)/%.o=$(OUT_DIR)/%.d)
MOCK_DEPS := $(MOCK_OBJS:$(OUT_DIR)/%.o=$(OUT_DIR)/%.d)
MAIN_DEPS := $(MAIN_OBJS:$(OUT_DIR)/main/%.o=$(OUT_DIR)/main/%.d)
TEST_HELPER_DEPS := $(TEST_HELPER_OBJS:$(OUT_DIR)/%.o=$(OUT_DIR)/%.d)
RUNNER_DEPS := $(RUNNER_UTILS_OBJ:%.o=%.d) $(RUNNER_NETWORKING_OBJ:%.o=%.d) \
               $(RUNNER_PROTOCOL_OBJ:%.o=%.d) $(RUNNER_DISPATCHER_OBJ:%.o=%.d) \
               $(RUNNER_PTZ_OBJ:%.o=%.d) $(RUNNER_MEDIA_OBJ:%.o=%.d) \
               $(RUNNER_IMAGING_OBJ:%.o=%.d) $(RUNNER_INTEGRATION_OBJ:%.o=%.d)
ALL_DEPS := $(TEST_DEPS) $(MOCK_DEPS) $(MAIN_DEPS) $(TEST_HELPER_DEPS) $(RUNNER_DEPS)

# Modular test executables
TEST_UTILS = $(OUT_DIR)/test_utils
TEST_NETWORKING = $(OUT_DIR)/test_networking
TEST_PROTOCOL = $(OUT_DIR)/test_protocol
TEST_DISPATCHER = $(OUT_DIR)/test_service_dispatcher
TEST_PTZ = $(OUT_DIR)/test_ptz
TEST_MEDIA = $(OUT_DIR)/test_media
TEST_IMAGING = $(OUT_DIR)/test_imaging
TEST_INTEGRATION = $(OUT_DIR)/test_integration

# New dynamic test runner with filtering
TEST_RUNNER_DYNAMIC = $(OUT_DIR)/test_runner

# Legacy consolidated test runner (kept for compatibility - DEPRECATED)
TEST_RUNNER_LEGACY = $(OUT_DIR)/test_runner_legacy

# All modular test targets
MODULAR_TESTS := $(TEST_UTILS) $(TEST_NETWORKING) $(TEST_PROTOCOL) $(TEST_DISPATCHER) \
                 $(TEST_PTZ) $(TEST_MEDIA) $(TEST_IMAGING) $(TEST_INTEGRATION)

.PHONY: all clean clean-deps test test-utils test-networking test-protocol test-dispatcher \
        test-ptz test-media test-imaging test-integration test-all test-modular test-legacy \
        test-dynamic test-dynamic-unit test-dynamic-integration test-dynamic-list \
        test-dynamic-ptz test-dynamic-media \
        coverage coverage-html coverage-report coverage-clean debug release \
        compile-native-libs compile-commands incremental-build help

# Default target - build new dynamic test runner and all modular test suites
all: $(TEST_RUNNER_DYNAMIC) $(MODULAR_TESTS)
	@echo "✅ Dynamic test runner and all modular test suites built successfully"

# Incremental build - only recompile changed files
incremental-build: $(MODULAR_TESTS)
	@echo "Incremental build completed - only changed files were recompiled"

# Legacy test runner (for compatibility)
test-legacy: $(TEST_RUNNER_LEGACY)
	@echo "Running legacy consolidated test runner..."
	@$(TEST_RUNNER_LEGACY)

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
coverage: all
	@echo "Coverage build completed"

# Generate HTML coverage report
coverage-html: coverage
	@echo "Running tests to generate coverage data..."
	@if [ -f $(TEST_TARGET) ]; then \
		echo "Running $(TEST_TARGET)..."; \
		$(TEST_TARGET); \
	fi
	@echo "Generating HTML coverage report..."
	@mkdir -p $(COVERAGE_DIR)
	@$(LCOV) --capture --directory $(OUT_DIR) --output-file $(COVERAGE_INFO)
	@$(LCOV) --remove $(COVERAGE_INFO) '*/tests/*' --output-file $(COVERAGE_INFO)
	@$(LCOV) --remove $(COVERAGE_INFO) '*/generated/*' --output-file $(COVERAGE_INFO)
	@$(GENHTML) $(COVERAGE_INFO) --output-directory $(COVERAGE_HTML)
	@echo "HTML coverage report generated in $(COVERAGE_HTML)/index.html"

# Generate coverage summary report
coverage-report: coverage
	@echo "Running tests to generate coverage data..."
	@if [ -f $(TEST_TARGET) ]; then \
		echo "Running $(TEST_TARGET)..."; \
		$(TEST_TARGET); \
	fi
	@echo "Generating coverage summary..."
	@mkdir -p $(COVERAGE_DIR)
	@$(LCOV) --capture --directory $(OUT_DIR) --output-file $(COVERAGE_INFO)
	@$(LCOV) --remove $(COVERAGE_INFO) '*/tests/*' --output-file $(COVERAGE_INFO)
	@$(LCOV) --remove $(COVERAGE_INFO) '*/generated/*' --output-file $(COVERAGE_INFO)
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

# Modular test execution targets
test: test-modular

test-modular: $(MODULAR_TESTS)
	@echo "╔════════════════════════════════════════════════════════════════╗"
	@echo "║        Running All Modular ONVIF Test Suites                  ║"
	@echo "╚════════════════════════════════════════════════════════════════╝"
	@echo ""
	@failures=0; \
	for test_exec in $(MODULAR_TESTS); do \
		echo "Running $$test_exec..."; \
		$$test_exec || failures=$$((failures + 1)); \
		echo ""; \
	done; \
	if [ $$failures -eq 0 ]; then \
		echo "✅ All test suites passed!"; \
	else \
		echo "❌ $$failures test suite(s) failed!"; \
		exit 1; \
	fi

test-utils: $(TEST_UTILS)
	@echo "Running utility tests..."
	@$(TEST_UTILS)

test-networking: $(TEST_NETWORKING)
	@echo "Running networking tests..."
	@$(TEST_NETWORKING)

test-protocol: $(TEST_PROTOCOL)
	@echo "Running protocol tests..."
	@$(TEST_PROTOCOL)

test-dispatcher: $(TEST_DISPATCHER)
	@echo "Running service dispatcher tests..."
	@$(TEST_DISPATCHER)

test-ptz: $(TEST_PTZ)
	@echo "Running PTZ service tests..."
	@$(TEST_PTZ)

test-media: $(TEST_MEDIA)
	@echo "Running Media service tests..."
	@$(TEST_MEDIA)

test-imaging: $(TEST_IMAGING)
	@echo "Running Imaging service tests..."
	@$(TEST_IMAGING)

test-integration: $(TEST_INTEGRATION)
	@echo "Running integration tests..."
	@$(TEST_INTEGRATION)

test-all: test-modular

# Dynamic test runner targets with filtering
test-dynamic: $(TEST_RUNNER_DYNAMIC)
	@echo "Running all tests with dynamic runner..."
	@$(TEST_RUNNER_DYNAMIC)

test-dynamic-unit: $(TEST_RUNNER_DYNAMIC)
	@echo "Running unit tests only..."
	@$(TEST_RUNNER_DYNAMIC) --type=unit

test-dynamic-integration: $(TEST_RUNNER_DYNAMIC)
	@echo "Running integration tests only..."
	@$(TEST_RUNNER_DYNAMIC) --type=integration

test-dynamic-list: $(TEST_RUNNER_DYNAMIC)
	@echo "Listing available test suites..."
	@$(TEST_RUNNER_DYNAMIC) --list

test-dynamic-ptz: $(TEST_RUNNER_DYNAMIC)
	@echo "Running PTZ tests (unit + integration)..."
	@$(TEST_RUNNER_DYNAMIC) --suite=ptz-service,ptz-adapter,ptz-callbacks,ptz-integration

test-dynamic-media: $(TEST_RUNNER_DYNAMIC)
	@echo "Running Media tests (unit + integration)..."
	@$(TEST_RUNNER_DYNAMIC) --suite=media-utils,media-callbacks,media-integration

# Build rules for each modular test executable (with test helpers)
$(TEST_UTILS): $(RUNNER_UTILS_OBJ) $(UTILS_TEST_OBJS) $(TEST_HELPER_OBJS) $(MOCK_OBJS) $(MAIN_OBJS)
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) $^ $(CMOCKA_LIBS) -o $@
	@echo "✅ Built $@"

$(TEST_NETWORKING): $(RUNNER_NETWORKING_OBJ) $(NETWORKING_TEST_OBJS) $(TEST_HELPER_OBJS) $(MOCK_OBJS) $(MAIN_OBJS)
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) $^ $(CMOCKA_LIBS) -o $@
	@echo "✅ Built $@"

$(TEST_PROTOCOL): $(RUNNER_PROTOCOL_OBJ) $(PROTOCOL_TEST_OBJS) $(TEST_HELPER_OBJS) $(MOCK_OBJS) $(MAIN_OBJS)
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) $^ $(CMOCKA_LIBS) -o $@
	@echo "✅ Built $@"

$(TEST_DISPATCHER): $(RUNNER_DISPATCHER_OBJ) $(DISPATCHER_TEST_OBJS) $(TEST_HELPER_OBJS) $(MOCK_OBJS) $(MAIN_OBJS)
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) $^ $(CMOCKA_LIBS) -o $@
	@echo "✅ Built $@"

$(TEST_PTZ): $(RUNNER_PTZ_OBJ) $(PTZ_TEST_OBJS) $(TEST_HELPER_OBJS) $(MOCK_OBJS) $(MAIN_OBJS)
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) $^ $(CMOCKA_LIBS) -o $@
	@echo "✅ Built $@"

$(TEST_MEDIA): $(RUNNER_MEDIA_OBJ) $(MEDIA_TEST_OBJS) $(TEST_HELPER_OBJS) $(MOCK_OBJS) $(MAIN_OBJS)
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) $^ $(CMOCKA_LIBS) -o $@
	@echo "✅ Built $@"

$(TEST_IMAGING): $(RUNNER_IMAGING_OBJ) $(IMAGING_TEST_OBJS) $(TEST_HELPER_OBJS) $(MOCK_OBJS) $(MAIN_OBJS)
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) $^ $(CMOCKA_LIBS) -o $@
	@echo "✅ Built $@"

$(TEST_INTEGRATION): $(RUNNER_INTEGRATION_OBJ) $(INTEGRATION_TEST_OBJS) $(TEST_HELPER_OBJS) $(MOCK_OBJS_INTEGRATION) $(MAIN_OBJS)
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) $^ $(CMOCKA_LIBS) -o $@
	@echo "✅ Built $@"

# New dynamic test runner with filtering support
$(TEST_RUNNER_DYNAMIC): $(RUNNER_DYNAMIC_OBJ) $(TEST_SUITE_REGISTRY_OBJS) $(TEST_OBJS) $(TEST_HELPER_OBJS) $(MOCK_OBJS_INTEGRATION) $(MAIN_OBJS)
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) $^ $(CMOCKA_LIBS) -o $@
	@echo "✅ Built dynamic test runner $@"

# Legacy consolidated test runner build (for compatibility, with test helpers - DEPRECATED)
$(TEST_RUNNER_LEGACY): $(RUNNER_LEGACY_OBJ) $(TEST_OBJS) $(TEST_HELPER_OBJS) $(MOCK_OBJS) $(MAIN_OBJS)
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) $^ $(CMOCKA_LIBS) -o $@
	@echo "✅ Built legacy test runner $@"

# Object file compilation with dependency generation
$(OUT_DIR)/%.o: src/%.c | $(OUT_DIR)
	@mkdir -p $(dir $@)
	@mkdir -p $(dir $(OUT_DIR)/$*.d)
	$(CC) $(CFLAGS) -MT $@ -MMD -MP -MF $(OUT_DIR)/$*.d $(INCLUDES:%=-I%) -c $< -o $@ -pipe

# Main project object files (exclude platform_anyka.c) with dependency generation
$(OUT_DIR)/main/%.o: ../src/%.c | $(OUT_DIR)
	@mkdir -p $(dir $@)
	@mkdir -p $(dir $(OUT_DIR)/main/$*.d)
	$(CC) $(CFLAGS) -MT $@ -MMD -MP -MF $(OUT_DIR)/main/$*.d $(INCLUDES:%=-I%) -c $< -o $@ -pipe

# Special fast compilation rule for soapC.c (no optimization, no warnings for generated code)
$(OUT_DIR)/main/generated/soapC.o: ../src/generated/soapC.c | $(OUT_DIR)
	@echo "Compiling soapC.c (optimized for speed)..."
	@mkdir -p $(dir $@)
	$(CC) $(BASE_CFLAGS) -O0 -w $(TEST_CFLAGS) $(INCLUDES:%=-I%) -c $< -o $@ -pipe

# Create output directory
$(OUT_DIR):
	mkdir -p $(OUT_DIR)

# Clean dependency files
clean-deps:
	@echo "Cleaning dependency files..."
	@find $(OUT_DIR) -name "*.d" -delete 2>/dev/null || true

# Clean build artifacts
clean: coverage-clean clean-deps
	rm -rf $(OUT_DIR)

# Compile native libraries for unit testing
compile-native-libs:
	@echo "Compiling native gsoap and libb64 libraries for unit testing..."
	@./scripts/compile_local_libs.sh

# Generate compile_commands.json for test files
compile-commands:
	@echo "Generating compile_commands.json for test files..."
	@./scripts/generate_compile_commands.sh

# Include dependency files for incremental builds
-include $(ALL_DEPS)

# Help target
help:
	@echo "╔════════════════════════════════════════════════════════════════╗"
	@echo "║           ONVIF Modular Test Suite - Build System             ║"
	@echo "╚════════════════════════════════════════════════════════════════╝"
	@echo ""
	@echo "📦 Build Targets:"
	@echo "  all              - Build all modular test suites (default)"
	@echo "  incremental-build - Build only changed files (faster for development)"
	@echo "  debug            - Build debug version"
	@echo "  release          - Build release version"
	@echo ""
	@echo "🧪 Test Execution Targets:"
	@echo "  test             - Run all modular test suites"
	@echo "  test-modular     - Run all modular test suites (same as test)"
	@echo "  test-utils       - Run utility tests only"
	@echo "  test-networking  - Run networking tests only"
	@echo "  test-protocol    - Run protocol (gSOAP) tests only"
	@echo "  test-dispatcher  - Run service dispatcher tests only"
	@echo "  test-ptz         - Run PTZ service tests only"
	@echo "  test-media       - Run Media service tests only"
	@echo "  test-imaging     - Run Imaging service tests only"
	@echo "  test-integration - Run integration tests only"
	@echo "  test-all         - Run all tests (alias for test-modular)"
	@echo "  test-legacy      - Run legacy consolidated test runner"
	@echo ""
	@echo "📊 Coverage Targets:"
	@echo "  coverage         - Build with coverage analysis"
	@echo "  coverage-html    - Generate HTML coverage report"
	@echo "  coverage-report  - Generate coverage summary"
	@echo "  coverage-clean   - Clean coverage files"
	@echo ""
	@echo "🛠️  Utility Targets:"
	@echo "  compile-native-libs - Compile native gsoap and libb64 libraries"
	@echo "  compile-commands  - Generate compile_commands.json for test files"
	@echo "  clean            - Clean all build artifacts"
	@echo "  clean-deps       - Clean dependency files only"
	@echo "  help             - Show this help"
	@echo ""
	@echo "✨ Modular Test Suite Benefits:"
	@echo "  • No forward declarations needed"
	@echo "  • Faster incremental builds"
	@echo "  • Better test isolation"
	@echo "  • Can run individual test suites"
	@echo "  • Parallel execution possible"
	@echo ""
	@echo "📁 Test Executables:"
	@echo "  $(OUT_DIR)/test_utils              - Utility tests"
	@echo "  $(OUT_DIR)/test_networking         - Networking tests"
	@echo "  $(OUT_DIR)/test_protocol           - Protocol tests"
	@echo "  $(OUT_DIR)/test_service_dispatcher - Service dispatcher tests"
	@echo "  $(OUT_DIR)/test_ptz                - PTZ service tests"
	@echo "  $(OUT_DIR)/test_media              - Media service tests"
	@echo "  $(OUT_DIR)/test_imaging            - Imaging service tests"
	@echo "  $(OUT_DIR)/test_integration        - Integration tests"
