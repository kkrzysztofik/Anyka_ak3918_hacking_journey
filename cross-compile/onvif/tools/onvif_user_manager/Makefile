# ONVIF User Manager Tool Makefile
# ================================
#
# Build system for ONVIF user management command-line tool
# Uses native gcc for host machine execution

CC = gcc
CFLAGS = -Wall -Wextra -std=c99 -D_POSIX_C_SOURCE=199309L -DWITH_NO_C_LOCALE -I../../src
LDFLAGS = -lm -lpthread

# Source files for user manager tool
USER_MANAGER_SRCS = \
  onvif_user_manager.c \
  config_runtime_simple.c \
  platform_simple.c \
  ini_merge.c

# Object files (build in out directory)
USER_MANAGER_OBJS = $(addprefix ../../out/tools/onvif_user_manager/, $(USER_MANAGER_SRCS:.c=.o)) \
 ../../out/tools/onvif_user_manager/config_storage.o \
 ../../out/tools/onvif_user_manager/hash_utils.o \
 ../../out/tools/onvif_user_manager/sha256.o

# Target binary (build in out directory)
USER_MANAGER_BIN = ../../out/tools/onvif_user_manager/onvif_user_manager

# Default target
all: $(USER_MANAGER_BIN)

# Create output directory
../../out/tools/onvif_user_manager:
	mkdir -p ../../out/tools/onvif_user_manager

# Build user manager tool
$(USER_MANAGER_BIN): ../../out/tools/onvif_user_manager $(USER_MANAGER_OBJS)
	$(CC) $(USER_MANAGER_OBJS) -o $@ $(LDFLAGS)

# Compile source files
../../out/tools/onvif_user_manager/%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

# Compile config_storage.c from source directory
../../out/tools/onvif_user_manager/config_storage.o: ../../src/core/config/config_storage.c
	@mkdir -p ../../out/tools/onvif_user_manager
	$(CC) $(CFLAGS) -c $< -o $@

# Compile hash_utils.c from source directory
../../out/tools/onvif_user_manager/hash_utils.o: ../../src/utils/security/hash_utils.c
	@mkdir -p ../../out/tools/onvif_user_manager
	$(CC) $(CFLAGS) -c $< -o $@

# Compile sha256.c from source directory
../../out/tools/onvif_user_manager/sha256.o: ../../src/utils/security/sha256.c
	@mkdir -p ../../out/tools/onvif_user_manager
	$(CC) $(CFLAGS) -c $< -o $@

# Clean build artifacts
clean:
	rm -rf ../../out/tools/onvif_user_manager

# Install tool (optional)
install: $(USER_MANAGER_BIN)
	cp $(USER_MANAGER_BIN) /usr/local/bin/

# Uninstall tool (optional)
uninstall:
	rm -f /usr/local/bin/onvif_user_manager

# Help target
help:
	@echo "Available targets:"
	@echo "  all       - Build the user manager tool (default)"
	@echo "  clean     - Remove build artifacts"
	@echo "  install   - Install tool to /usr/local/bin"
	@echo "  uninstall - Remove tool from /usr/local/bin"
	@echo "  help      - Show this help message"

.PHONY: all clean install uninstall help
