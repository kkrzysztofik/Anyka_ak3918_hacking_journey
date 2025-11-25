# ONVIF Tools

This directory contains command-line utilities for managing ONVIF configuration.

## Available Tools

### onvif_user_manager

A command-line utility for managing ONVIF user credentials.

#### Usage

```bash
onvif_user_manager --user <username> --password <password> --file <config_file>
onvif_user_manager -u <username> -p <password> -f <config_file>
onvif_user_manager --help
```

#### Options

- `-u, --user USERNAME` - Username to add/update (required)
- `-p, --password PASS` - Password for the user (required)
- `-f, --file CONFIG_FILE` - Path to configuration file (required)
- `-h, --help` - Display help message

#### Examples

```bash
# Add admin user to default config
./onvif_user_manager --user admin --password secret123 --file ./config.ini

# Add user with short options
./onvif_user_manager -u operator -p pass456 -f /etc/onvif/config.ini
```

#### Building

```bash
# Build all tools
make tools

# Build from main ONVIF directory
make tools

# Build specific tool
cd tools/onvif_user_manager && make
```

#### Error Codes

- `0` - Success
- `1` - Error (invalid parameters, file I/O error, etc.)

The tool will display descriptive error messages to stderr for troubleshooting.

## Directory Structure

```text
tools/
├── README.md                    # This file
└── onvif_user_manager/          # User management tool
    ├── Makefile                 # Tool-specific build configuration
    ├── onvif_user_manager.c     # Main tool source
    ├── config_runtime_simple.c   # Simplified config runtime
    └── platform_simple.c       # Simplified platform implementation
```

## Build Output

All build artifacts are placed in the `out/tools/` directory to keep source directories clean:

```text
out/
└── tools/
    └── onvif_user_manager/
        ├── onvif_user_manager    # Compiled binary
        ├── onvif_user_manager.o  # Object files
        ├── config_runtime_simple.o
        └── platform_simple.o
```
