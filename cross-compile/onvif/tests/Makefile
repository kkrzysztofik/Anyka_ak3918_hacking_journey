# ONVIF Project Test Makefile
# ===========================
#
# Unit testing build system using CMocka framework
#
# Build Commands:
#   make                    - Build all tests
#   make test              - Build and run all tests (output to console + OUT.log)
#   make test-utils        - Build and run utility tests
#   make test-services     - Build and run service tests
#   make test-networking   - Build and run networking tests
#   make test-integration  - Build and run integration tests
#   make clean             - Clean test build artifacts
#   make coverage          - Build with coverage analysis
#
# Environment Variables:
#   BUILD_TYPE - Build type: 'debug' or 'release' (default: debug for tests)

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
	config_storage_save \
	config_runtime_get_int \
	config_runtime_set_int \
	config_runtime_get_string \
	config_runtime_set_string \
	config_runtime_get_bool \
	config_runtime_set_bool \
	config_runtime_get_float \
	config_runtime_set_float \
	config_runtime_init \
	config_runtime_cleanup \
	config_runtime_apply_defaults \
	config_runtime_snapshot \
	config_runtime_get_generation \
	config_runtime_queue_persistence_update \
	config_runtime_process_persistence_queue \
	config_runtime_get_persistence_status \
	config_runtime_get_stream_profile \
	config_runtime_set_stream_profile \
	config_runtime_validate_stream_profile \
	config_runtime_get_stream_profile_count \
	config_runtime_get_ptz_profile_presets \
	config_runtime_set_ptz_profile_presets \
	config_runtime_validate_ptz_profile_presets \
	config_runtime_hash_password \
	config_runtime_verify_password \
	config_runtime_add_user \
	config_runtime_remove_user \
	config_runtime_update_user_password \
	config_runtime_authenticate_user \
	config_runtime_enumerate_users \
	onvif_service_dispatcher_register_service \
	onvif_service_dispatcher_unregister_service \
	onvif_service_dispatcher_init \
	onvif_service_dispatcher_cleanup \
	buffer_pool_init \
	buffer_pool_cleanup \
	onvif_gsoap_init \
	onvif_gsoap_cleanup \
	onvif_gsoap_reset \
	onvif_gsoap_has_error \
	onvif_gsoap_get_error \
	onvif_gsoap_get_response_data \
	onvif_gsoap_get_response_length \
	onvif_gsoap_generate_response_with_callback \
	onvif_gsoap_generate_fault_response \
	buffer_pool_get \
	buffer_pool_return \
	buffer_pool_get_stats \
	get_buffer_pool_stats \
	smart_response_build_with_dynamic_buffer \
	smart_response_build_with_buffer_pool \
	smart_response_build \
	smart_response_estimate_size \
	onvif_service_dispatcher_dispatch \
	socket \
	bind \
	listen \
	accept \
	connect \
	send \
	recv \
	sendto \
	recvfrom \
	close \
	setsockopt \
	getsockopt \
	http_server_start \
	http_server_stop \
	process_connection \
	http_validate_authentication

# Generate linker wrap flags for CMocka function interception
WRAP_FLAGS := $(addprefix -Wl$(comma)--wrap=,$(WRAP_FUNCTIONS))

# CMocka library for native testing
CMOCKA_LIBS=-lcmocka -lpthread -lm $(GSOAP_LIBS) $(BASE64_LIBS)

# Dynamic test runner with filtering (replaces all old individual runners)
RUNNER_DYNAMIC_SRC = src/runner/test_runner.c

# Test suite registry sources for dynamic test runner
TEST_SUITE_REGISTRY_SRCS := \
	src/common/test_suites.c \
	src/unit/networking/test_http_metrics_suite.c \
	src/unit/protocol/test_gsoap_protocol_suite.c \
	src/integration/ptz_integration_suite.c \
	src/integration/media_integration_suite.c \
	src/integration/device_integration_suite.c \
	src/integration/imaging_integration_suite.c \
	src/integration/snapshot_integration_suite.c \
	src/integration/networking/http_auth_integration_suite.c \
	src/integration/network_integration_suite.c \
	src/integration/core/config/test_config_performance_suite.c \
	src/integration/core/config/test_config_security_suite.c

# Test source files for each module
UTILS_TEST_SRCS := \
	src/unit/utils/test_memory_utils.c \
	src/unit/utils/test_logging_utils.c \
	src/unit/utils/test_hash_utils.c \
	src/unit/core/config/test_config_runtime.c \
	src/unit/core/config/test_config_storage.c

NETWORKING_TEST_SRCS := \
	src/unit/networking/test_http_auth.c \
	src/unit/networking/test_http_metrics_simple.c \
	src/unit/networking/test_http_server_auth.c

PROTOCOL_TEST_SRCS := \
	src/unit/protocol/test_onvif_gsoap_core.c \
	src/unit/protocol/test_onvif_gsoap_media.c \
	src/unit/protocol/test_onvif_gsoap_ptz.c \
	src/unit/protocol/test_onvif_gsoap_device.c \
	src/unit/protocol/test_onvif_gsoap_imaging.c \
	src/utils/test_gsoap_utils.c \
	src/unit/protocol/test_onvif_gsoap_response_generation.c \
	src/unit/protocol/test_gsoap_response_suite.c \
	src/unit/protocol/test_onvif_gsoap_edge_cases.c \
	src/unit/protocol/test_gsoap_edge_suite.c \
	src/data/response_test_data.c

DISPATCHER_TEST_SRCS := \
	src/unit/services/common/test_service_dispatcher.c \
	src/unit/services/common/test_onvif_service_handler.c

DEVICE_TEST_SRCS := \
	src/unit/services/device/test_device_service.c

PTZ_TEST_SRCS := \
	src/unit/services/ptz/test_ptz_service.c \
	src/unit/services/ptz/test_onvif_ptz_callbacks.c \
	src/unit/services/ptz/test_ptz_unit_suite.c \
	src/unit/ptz_adapter_tests.c

MEDIA_TEST_SRCS := \
	src/unit/services/media/test_media_utils.c \
	src/unit/services/media/test_onvif_media_callbacks.c \
	src/unit/services/media/test_media_callbacks_suite.c

IMAGING_TEST_SRCS := \
	src/unit/services/imaging/test_onvif_imaging_callbacks.c

INTEGRATION_TEST_SRCS := \
	src/integration/ptz_service_tests.c \
	src/integration/media_service_tests.c \
	src/integration/imaging_service_optimization_test.c \
	src/integration/services/test_imaging_integration.c \
	src/integration/services/test_snapshot_integration.c \
	src/integration/device_service_tests.c \
	src/integration/soap_error_tests.c \
	src/integration/networking/test_http_auth_integration.c \
	src/integration/networking/test_network_integration.c \
	src/performance/test_config_performance.c \
	src/security/test_config_security.c

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
	src/mocks/platform_mock.c \
	src/mocks/platform_ptz_mock.c \
	src/mocks/network_mock.c \
	src/mocks/ptz_adapter_mock.c \
	src/mocks/mock_service_dispatcher.c \
	src/mocks/config_mock.c \
	src/mocks/buffer_pool_mock.c \
	src/mocks/smart_response_mock.c \
	src/mocks/gsoap_mock.c \
	src/mocks/media_mock.c \
	src/mocks/http_server_mock.c \
	src/mocks/service_handler_mock.c \

# Mock source files for integration tests (exclude buffer_pool_mock - use real implementation)
MOCK_SRCS_INTEGRATION := \
	src/mocks/platform_mock.c \
	src/mocks/platform_ptz_mock.c \
	src/mocks/network_mock.c \
	src/mocks/ptz_adapter_mock.c \
	src/mocks/mock_service_dispatcher.c \
	src/mocks/config_mock.c \
	src/mocks/smart_response_mock.c \
	src/mocks/gsoap_mock.c \
	src/mocks/media_mock.c \
	src/mocks/http_server_mock.c \
	src/mocks/service_handler_mock.c \

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
	../src/networking/http/http_server.c \
	../src/networking/common/buffer_pool.c \
	../src/utils/validation/common_validation.c \
	../src/utils/validation/input_validation.c \
	../src/utils/security/base64_utils.c \
	../src/utils/security/security_hardening.c \
	../src/utils/security/sha256.c \
	../src/utils/security/hash_utils.c \
	../src/utils/string/string_shims.c \
	../src/services/common/service_dispatcher.c \
	../src/services/common/onvif_service_common.c \
	../src/services/device/onvif_device.c \
	../src/services/media/onvif_media.c \
	../src/services/ptz/onvif_ptz.c \
	../src/services/imaging/onvif_imaging.c \
	../src/services/snapshot/onvif_snapshot.c \
	../src/platform/adapters/ptz_adapter.c \
	../src/utils/network/network_utils.c \
	../src/utils/error/error_handling.c \
	../src/core/config/config_runtime.c \
	../src/core/config/config_storage.c

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

# Single executable target (no shared libraries)
TEST_RUNNER = $(OUT_DIR)/test_runner

.PHONY: all clean clean-deps test test-unit test-integration test-list \
        coverage coverage-html coverage-report coverage-clean debug release \
        compile-native-libs compile-commands help

# Default target - build single executable test runner
all: $(TEST_RUNNER)
	@echo "✅ Single executable test runner built successfully"

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
#   make test                    - Run all tests (output to console + OUT.log)
#   make test SUITE=ptz-service  - Run specific suite (output to console + OUT.log)
#   make test-unit SUITE=ptz-*   - Run all PTZ unit tests (output to console + OUT.log)
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

# Single executable test runner build rule
# Links all object files directly with CMocka wrap flags applied
$(TEST_RUNNER): $(RUNNER_DYNAMIC_OBJ) $(TEST_SUITE_REGISTRY_OBJS) $(TEST_OBJS) $(MOCK_OBJS) $(MAIN_OBJS) $(TEST_HELPER_OBJS)
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) $(WRAP_FLAGS) $(RUNNER_DYNAMIC_OBJ) $(TEST_SUITE_REGISTRY_OBJS) $(TEST_OBJS) $(MOCK_OBJS) $(MAIN_OBJS) $(TEST_HELPER_OBJS) \
		$(CMOCKA_LIBS) -o $@
	@echo "✅ Built single executable test runner $@"

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
	@echo "║           ONVIF Single Executable Test Runner - Build System  ║"
	@echo "╚════════════════════════════════════════════════════════════════╝"
	@echo ""
	@echo "📦 Build Targets:"
	@echo "  all              - Build single executable test runner (default)"
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
	@echo "✨ Single Executable Test Runner Features:"
	@echo "  • Unified test runner with advanced filtering"
	@echo "  • 196 tests across 17 test suites"
	@echo "  • Filter by suite name, type, or pattern"
	@echo "  • Detailed test execution reports"
	@echo "  • Better test isolation and organization"
	@echo "  • No shared library dependencies"
	@echo ""
	@echo "📁 Test Executable:"
	@echo "  $(OUT_DIR)/test_runner - Single executable test runner (all 196 tests)"
