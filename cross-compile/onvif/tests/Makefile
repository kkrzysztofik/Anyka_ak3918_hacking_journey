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
BASE_CFLAGS=-std=c99 -D_POSIX_C_SOURCE=199309L -DTEST_BUILD -fno-builtin -fstrict-aliasing -Wall -Wextra -fPIC

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
LIB_DIR=$(OUT_DIR)/lib

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

# Comma variable for Makefile syntax (needed for addprefix with multiple arguments)
comma := ,

# Platform functions to wrap for CMocka mocking
WRAP_FUNCTIONS := \
	platform_init \
	platform_cleanup \
	platform_log_error \
	platform_log_warning \
	platform_log_notice \
	platform_log_info \
	platform_log_debug \
	platform_vi_match_sensor \
	platform_vi_open \
	platform_vi_close \
	platform_vi_get_sensor_resolution \
	platform_vi_switch_day_night \
	platform_vi_set_flip_mirror \
	platform_vi_capture_on \
	platform_vi_start_global_capture \
	platform_vi_capture_off \
	platform_vi_set_channel_attr \
	platform_vi_get_fps \
	platform_vpss_effect_set \
	platform_vpss_effect_get \
	platform_venc_init \
	platform_validate_venc_config \
	platform_venc_cleanup \
	platform_venc_get_frame \
	platform_sleep_ms \
	platform_venc_release_frame \
	platform_venc_get_stream \
	platform_venc_release_stream \
	platform_venc_request_stream \
	platform_venc_cancel_stream \
	platform_venc_get_stream_by_handle \
	platform_venc_release_stream_by_handle \
	platform_venc_get_buffer_status \
	platform_ai_open \
	platform_ai_close \
	platform_aenc_init \
	platform_aenc_cleanup \
	platform_aenc_get_frame \
	platform_aenc_release_frame \
	platform_aenc_get_stream \
	platform_aenc_release_stream \
	platform_ptz_init \
	platform_ptz_cleanup \
	platform_ptz_set_degree \
	platform_ptz_check_self \
	platform_ptz_move_to_position \
	platform_ptz_get_step_position \
	platform_ptz_get_status \
	platform_ptz_set_speed \
	platform_ptz_turn \
	platform_ptz_turn_stop \
	ptz_adapter_init \
	ptz_adapter_cleanup \
	ptz_adapter_get_status \
	ptz_adapter_absolute_move \
	ptz_adapter_relative_move \
	ptz_adapter_continuous_move \
	ptz_adapter_stop \
	ptz_adapter_set_preset \
	ptz_adapter_goto_preset \
	platform_irled_init \
	platform_irled_cleanup \
	platform_irled_set_mode \
	platform_irled_get_status \
	platform_snapshot_init \
	platform_snapshot_cleanup \
	platform_snapshot_capture \
	platform_snapshot_release \
	platform_config_load \
	platform_config_save \
	platform_config_get_string \
	platform_config_get_int \
	platform_get_system_info \
	onvif_service_dispatcher_register_service \
	onvif_service_dispatcher_unregister_service \
	onvif_service_dispatcher_init \
	onvif_service_dispatcher_cleanup \
	onvif_gsoap_init \
	onvif_gsoap_cleanup \
	onvif_gsoap_reset \
	onvif_gsoap_has_error \
	onvif_gsoap_get_error \
	onvif_gsoap_get_response_data \
	onvif_gsoap_get_response_length \
	onvif_gsoap_generate_response_with_callback \
	onvif_gsoap_generate_fault_response \
	buffer_pool_init \
	buffer_pool_cleanup

# Generate linker wrap flags for CMocka function interception
WRAP_FLAGS := $(addprefix -Wl$(comma)--wrap=,$(WRAP_FUNCTIONS))

# CMocka library for native testing
CMOCKA_LIBS=-lcmocka -lpthread -lm $(GSOAP_LIBS) $(BASE64_LIBS)

# Dynamic test runner with filtering (replaces all old individual runners)
RUNNER_DYNAMIC_SRC = src/runner/test_runner.c

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

DEVICE_TEST_SRCS := \
	src/unit/services/device/test_device_service.c

PTZ_TEST_SRCS := \
	src/unit/services/ptz/test_ptz_service.c \
	src/unit/services/ptz/test_onvif_ptz_callbacks.c \
	src/unit/ptz_adapter_tests.c

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
	src/common/soap_test_helpers.c

# Mock source files (for unit tests)
MOCK_SRCS := \
	src/mocks/platform_mock_cmocka.c \
	src/mocks/platform_ptz_mock.c \
	src/mocks/network_mock_cmocka.c \
	src/mocks/ptz_adapter_mock_cmocka.c \
	src/mocks/mock_service_dispatcher_cmocka.c \
	src/mocks/config_mock_cmocka.c \
	src/mocks/buffer_pool_mock_cmocka.c \
	src/mocks/smart_response_mock_cmocka.c \
	src/mocks/gsoap_mock_cmocka.c \

# Mock source files for integration tests (exclude buffer_pool_mock - use real implementation)
MOCK_SRCS_INTEGRATION := \
	src/mocks/platform_mock_cmocka.c \
	src/mocks/platform_ptz_mock.c \
	src/mocks/network_mock_cmocka.c \
	src/mocks/ptz_adapter_mock_cmocka.c \
	src/mocks/mock_service_dispatcher_cmocka.c \
	src/mocks/config_mock_cmocka.c \
	src/mocks/smart_response_mock_cmocka.c \
	src/mocks/gsoap_mock_cmocka.c \

# Source files from main project (for testing - exclude platform_anyka.c - use platform mocks)
MAIN_SRCS := \
	../src/utils/memory/memory_manager.c \
	../src/utils/common/time_utils.c \
	../src/utils/memory/smart_response_builder.c \
	../src/protocol/gsoap/onvif_gsoap_core.c \
	../src/protocol/gsoap/onvif_gsoap_response.c \
	../src/protocol/response/onvif_service_handler.c \
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
	../src/platform/adapters/ptz_adapter.c \
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

# Dynamic test runner object file
RUNNER_DYNAMIC_OBJ := $(RUNNER_DYNAMIC_SRC:src/%.c=$(OUT_DIR)/%.o)

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
RUNNER_DEPS := $(RUNNER_DYNAMIC_OBJ:%.o=%.d)
ALL_DEPS := $(TEST_DEPS) $(MOCK_DEPS) $(MAIN_DEPS) $(TEST_HELPER_DEPS) $(RUNNER_DEPS)

# Shared library targets for modular test linking
MOCK_LIB = $(LIB_DIR)/libonvif_test_mocks.so
MAIN_LIB = $(LIB_DIR)/libonvif_main.so
HELPER_LIB = $(LIB_DIR)/libonvif_test_helpers.so

# Dynamic test runner executable
TEST_RUNNER = $(OUT_DIR)/test_runner

.PHONY: all clean clean-deps test test-unit test-integration test-list \
        coverage coverage-html coverage-report coverage-clean debug release \
        compile-native-libs compile-commands help

# Default target - build dynamic test runner
all: $(TEST_RUNNER)
	@echo "✅ Dynamic test runner built successfully"

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

# Test execution targets with optional SUITE parameter
# Usage:
#   make test                    - Run all tests
#   make test SUITE=ptz-service  - Run specific suite
#   make test-unit SUITE=ptz-*   - Run all PTZ unit tests
test: $(TEST_RUNNER)
ifdef SUITE
	@echo "Running tests for suite: $(SUITE)..."
	@$(TEST_RUNNER) --suite=$(SUITE)
else
	@echo "Running all tests..."
	@$(TEST_RUNNER)
endif

test-unit: $(TEST_RUNNER)
ifdef SUITE
	@echo "Running unit tests for suite: $(SUITE)..."
	@$(TEST_RUNNER) --type=unit --suite=$(SUITE)
else
	@echo "Running unit tests only..."
	@$(TEST_RUNNER) --type=unit
endif

test-integration: $(TEST_RUNNER)
ifdef SUITE
	@echo "Running integration tests for suite: $(SUITE)..."
	@$(TEST_RUNNER) --type=integration --suite=$(SUITE)
else
	@echo "Running integration tests only..."
	@$(TEST_RUNNER) --type=integration
endif

test-list: $(TEST_RUNNER)
	@echo "Listing available test suites..."
	@$(TEST_RUNNER) --list

# Shared library build rules with modular linking
$(MOCK_LIB): $(MOCK_OBJS) | $(LIB_DIR)
	@echo "Building unit test mocks shared library..."
	$(CC) -shared -o $@ $^
	@echo "✅ Built $@"

$(MAIN_LIB): $(MAIN_OBJS) | $(LIB_DIR)
	@echo "Building main project shared library..."
	$(CC) -shared $(WRAP_FLAGS) -o $@ $^ $(GSOAP_LIBS) $(BASE64_LIBS)
	@echo "✅ Built $@"

$(HELPER_LIB): $(TEST_HELPER_OBJS) | $(LIB_DIR)
	@echo "Building test helpers shared library..."
	$(CC) -shared -o $@ $^
	@echo "✅ Built $@"

# Dynamic test runner build rule with shared library linking
# Uses $ORIGIN/lib RPATH so executable finds shared libs in lib/ subdirectory
# NOTE: WRAP_FLAGS are defined but not used yet - will be enabled when tests migrate to CMocka mocks
$(TEST_RUNNER): $(RUNNER_DYNAMIC_OBJ) $(TEST_SUITE_REGISTRY_OBJS) $(TEST_OBJS) $(MOCK_LIB) $(MAIN_LIB) $(HELPER_LIB)
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) $(WRAP_FLAGS) $(RUNNER_DYNAMIC_OBJ) $(TEST_SUITE_REGISTRY_OBJS) $(TEST_OBJS) \
		-L$(LIB_DIR) -Wl,-rpath,'$$ORIGIN/lib' \
		-lonvif_test_mocks -lonvif_main -lonvif_test_helpers \
		$(CMOCKA_LIBS) -o $@
	@echo "✅ Built dynamic test runner $@"

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

# Create output directories
$(OUT_DIR):
	mkdir -p $(OUT_DIR)

$(LIB_DIR):
	mkdir -p $(LIB_DIR)

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
	@echo "║           ONVIF Dynamic Test Runner - Build System            ║"
	@echo "╚════════════════════════════════════════════════════════════════╝"
	@echo ""
	@echo "📦 Build Targets:"
	@echo "  all              - Build dynamic test runner (default)"
	@echo "  debug            - Build debug version"
	@echo "  release          - Build release version"
	@echo ""
	@echo "🧪 Test Execution Targets:"
	@echo "  test             - Run all tests (196 tests across 17 suites)"
	@echo "  test-unit        - Run unit tests only (129 tests)"
	@echo "  test-integration - Run integration tests only (67 tests)"
	@echo "  test-list        - List all available test suites"
	@echo ""
	@echo "🔍 Suite Filtering (use SUITE parameter):"
	@echo "  make test SUITE=ptz-service                       - Run specific suite"
	@echo "  make test SUITE=ptz-service,ptz-callbacks        - Run multiple suites"
	@echo "  make test-unit SUITE=http-auth,http-metrics      - Run specific unit tests"
	@echo "  make test-integration SUITE=ptz-integration      - Run specific integration tests"
	@echo ""
	@echo "📋 Direct Runner Usage:"
	@echo "  $(OUT_DIR)/test_runner --suite=ptz-service    - Run specific suite"
	@echo "  $(OUT_DIR)/test_runner --type=unit            - Run unit tests"
	@echo "  $(OUT_DIR)/test_runner --list                 - List all suites"
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
	@echo "✨ Dynamic Test Runner Features:"
	@echo "  • Unified test runner with advanced filtering"
	@echo "  • 196 tests across 17 test suites"
	@echo "  • Filter by suite name, type, or pattern"
	@echo "  • Detailed test execution reports"
	@echo "  • Better test isolation and organization"
	@echo ""
	@echo "📁 Test Executable:"
	@echo "  $(OUT_DIR)/test_runner - Dynamic test runner (all 196 tests)"
