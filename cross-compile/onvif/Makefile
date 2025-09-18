export PATH := $(PATH):/opt/arm-anykav200-crosstool/usr/bin
PREFIX=arm-anykav200-linux-uclibcgnueabi-
CC=${PREFIX}gcc
CPP=${PREFIX}g++
CFLAGS=-std=c99 -D_POSIX_C_SOURCE=199309L -DANYKA_PLATFORM=1 -Os -fno-builtin -fno-stack-protector -Wl,--hash-style=sysv
LINKERFLAG=-L./lib -Wl,--gc-sections -lakuio -lakispsdk -lakv_encode -lplat_common -lplat_thread -lplat_vi -lplat_vpss -lplat_ipcsrv -lplat_venc_cb -lmpi_venc -lakstreamenc -lpthread -lakae -lakaudiocodec -lakmedia -lakv_encode -lapp_net -lapp_rtsp -lmpi_aed -lmpi_aenc -lplat_ai -lplat_drv -lm
INCLUDES=include src src/core src/platform src/services src/networking src/protocol src/utils

# Output directory for all build artifacts
OUT_DIR=out

SRCS := \
	src/core/main/onvifd.c \
	src/platform/platform_anyka.c \
	src/core/config/config.c \
	src/core/lifecycle/service_manager.c \
	src/services/device/onvif_device.c \
	src/services/media/onvif_media.c \
	src/services/ptz/onvif_ptz.c \
	src/services/imaging/onvif_imaging.c \
	src/services/snapshot/onvif_snapshot.c \
	src/networking/http/http_server.c \
	src/networking/http/http_parser.c \
	src/networking/http/http_onvif_adapter.c \
	src/networking/common/connection_manager.c \
	src/networking/common/thread_pool.c \
	src/networking/common/buffer_pool.c \
	src/networking/common/epoll_server.c \
	src/networking/rtsp/rtsp_server_main.c \
	src/networking/rtsp/rtsp_auth.c \
	src/networking/rtsp/rtsp_sdp.c \
	src/networking/rtsp/rtsp_rtcp.c \
	src/networking/rtsp/rtsp_session.c \
	src/networking/rtsp/rtsp_rtp.c \
	src/networking/discovery/ws_discovery.c \
	src/protocol/soap/onvif_soap.c \
	src/protocol/xml/unified_xml.c \
	src/protocol/response/onvif_service_handler.c \
	src/utils/memory/memory_manager.c \
	src/utils/string/string_shims.c \
	src/utils/error/error_handling.c \
	src/utils/network/network_utils.c \
	src/utils/logging/logging_utils.c \
	src/utils/logging/service_logging.c \
	src/utils/validation/common_validation.c \
	src/utils/validation/input_validation.c \
	src/utils/security/security_hardening.c \
	src/utils/service/common_service_init.c

OBJS := $(SRCS:src/%.c=$(OUT_DIR)/%.o)

.PHONY = all clean debug docs docs-clean docs-open static-analysis clang-analyze cppcheck-analyze snyk-analyze

# Documentation directory
DOCS_DIR = docs

# Limit parallel jobs to prevent memory issues
MAKEFLAGS += -j1

all: clean $(OUT_DIR)/onvifd



# Documentation targets
docs: $(DOCS_DIR)/html/index.html

docs-clean:
	rm -rf $(DOCS_DIR)

docs-open: docs
	@echo "Documentation generated in $(DOCS_DIR)/html/"
	@echo "Open $(DOCS_DIR)/html/index.html in your browser"

# Generate Doxygen documentation
$(DOCS_DIR)/html/index.html: Doxyfile $(SRCS) $(wildcard src/*.h) $(wildcard include/*.h)
	@echo "Generating Doxygen documentation..."
	@mkdir -p $(DOCS_DIR)
	@doxygen Doxyfile
	@echo "Documentation generated successfully in $(DOCS_DIR)/html/"

# Debug target to build with debugging symbols
debug: clean
	@echo "Building debug version with symbols..."
	@mkdir -p $(OUT_DIR)
	@for src in $(SRCS); do \
		echo "Building $$src..."; \
		obj=$$(echo $$src | sed 's|src/|$(OUT_DIR)/|' | sed 's|\.c|\.o|'); \
		mkdir -p $$(dirname $$obj); \
		${CC} -g -O0 -std=c99 -D_POSIX_C_SOURCE=199309L -DANYKA_PLATFORM=1 -fno-builtin -fno-stack-protector -Wl,--hash-style=sysv ${INCLUDES:%=-I%} -c $$src -o $$obj -pipe || exit 1; \
	done
	@echo "Linking debug binary..."
	${CC} -g -O0 -std=c99 -D_POSIX_C_SOURCE=199309L -DANYKA_PLATFORM=1 -fno-builtin -fno-stack-protector -Wl,--hash-style=sysv $(OBJS) ${LINKERFLAG} -o $(OUT_DIR)/onvifd_debug
	@echo "Debug build completed: $(OUT_DIR)/onvifd_debug"

$(OUT_DIR)/onvifd: $(OBJS)
	${CC} ${CFLAGS} $(OBJS) ${LINKERFLAG} -o $@

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

clean: docs-clean
	rm -rf $(OUT_DIR) analysis-results
