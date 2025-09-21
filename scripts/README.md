# ONVIF Device Management Scripts

This directory contains bash scripts for managing the ONVIF daemon on the Anyka AK3918 camera device. These scripts follow the project guidelines for native cross-compilation and comprehensive error handling.

## Scripts Overview

### 1. `deploy_onvif.sh` - Deploy ONVIF Binaries
Intelligently checks which ONVIF binaries are available and uploads only those that exist to the device via FTP.

**Usage:**
```bash
./deploy_onvif.sh [device_ip] [username] [password]
```

**Parameters:**
- `device_ip` (optional): Device IP address (default: 192.168.1.100)
- `username` (optional): FTP username (default: admin)
- `password` (optional): FTP password (default: admin)

**Features:**
- Automatically detects available binaries (onvifd, onvifd_debug)
- Uploads only binaries that exist
- Provides detailed status reporting
- Validates input parameters and dependencies

**Example:**
```bash
# Use default values
./deploy_onvif.sh

# Specify custom device
./deploy_onvif.sh 192.168.1.50 admin mypassword
```

### 2. `run_onvif.sh` - Execute ONVIF Daemon
Connects to the device via telnet and executes the ONVIF daemon with comprehensive error handling and automatic log saving.

**Usage:**
```bash
./run_onvif.sh [device_ip] [username] [password] [release|debug]
```

**Parameters:**
- `device_ip` (optional): Device IP address (default: 192.168.1.100)
- `username` (optional): Telnet username (default: admin)
- `password` (optional): Telnet password (default: admin)
- `mode` (optional): Binary mode - 'release' or 'debug' (default: debug)

**Features:**
- Validates all input parameters
- Gracefully stops existing daemon processes
- Configures core dump handling automatically
- Provides system memory status
- Includes connection timeout handling
- **Automatically saves device output to logs** for debugging
- Captures and displays all telnet output

**Example:**
```bash
# Run debug version
./run_onvif.sh

# Run release version
./run_onvif.sh 192.168.1.100 admin admin release
```

### 3. `collect_coredump.sh` - Collect Core Dumps
Downloads core dump files from the device via FTP for debugging purposes with enhanced reporting.

**Usage:**
```bash
./collect_coredump.sh [device_ip] [username] [password]
```

**Parameters:**
- `device_ip` (optional): Device IP address (default: 192.168.1.100)
- `username` (optional): FTP username (default: admin)
- `password` (optional): FTP password (default: admin)

**Features:**
- Validates input parameters
- Provides detailed download statistics
- Calculates total size of collected dumps
- Includes GDB analysis instructions
- Enhanced error handling

**Example:**
```bash
# Collect core dumps
./collect_coredump.sh

# Analyze core dumps with gdb
gdb ../cross-compile/onvif/out/onvifd_debug ../debugging/coredump/core.*
```

## Prerequisites

### Required Tools
- `ftp` - For file transfer operations
- `telnet` - For remote command execution
- `gdb` - For core dump analysis (optional)
- `timeout` - For connection timeout handling (usually pre-installed)

### Installation on Ubuntu/Debian:
```bash
sudo apt-get install ftp telnet gdb
```

### Installation on Windows (WSL):
```bash
sudo apt-get install ftp telnet gdb
```

## Workflow

### Typical Development Workflow

1. **Build the project using native cross-compilation:**
   ```bash
   make -C cross-compile/onvif
   ```

2. **Deploy available binaries to device:**
   ```bash
   cd scripts
   ./deploy_onvif.sh 192.168.1.100 admin admin
   ```

3. **Execute daemon on device:**
   ```bash
   ./run_onvif.sh 192.168.1.100 admin admin debug
   ```

4. **If daemon crashes, collect core dumps:**
   ```bash
   ./collect_coredump.sh 192.168.1.100 admin admin
   ```

5. **Analyze core dumps:**
   ```bash
   gdb ../cross-compile/onvif/out/onvifd_debug ../debugging/coredump/core.*
   ```

6. **View execution logs:**
   ```bash
   ls -la ../debugging/logs/
   cat ../debugging/logs/onvif_execution_*.log
   ```

## Device Configuration

### FTP Server Setup
Ensure the device has an FTP server running and accessible:
- Default port: 21
- Username/password: admin/admin (configurable)

### Telnet Server Setup
Ensure the device has a telnet server running:
- Default port: 23
- Username/password: admin/admin (configurable)

### Core Dump Configuration
The scripts automatically configure core dump handling on the device:
- Core dumps are saved to `/tmp/core.%e.%p`
- Core dump size limit is set to unlimited
- Core dumps are collected to `../debugging/coredump/`

### Log File Management
The `run_onvif.sh` script automatically saves device output for debugging:
- Log files are saved to `../debugging/logs/`
- Files are named with timestamp: `onvif_execution_YYYYMMDD_HHMMSS.log`
- All telnet output is captured and saved
- Log files help with debugging daemon startup and execution issues

## Troubleshooting

### Common Issues

1. **"ftp command not found"**
   - Install FTP client: `sudo apt-get install ftp`

2. **"telnet command not found"**
   - Install telnet client: `sudo apt-get install telnet`

3. **"Binary not found on device"**
   - Run `deploy_onvif.sh` first to upload binaries

4. **"Connection refused"**
   - Check device IP address and network connectivity
   - Verify FTP/telnet services are running on device

5. **"Permission denied"**
   - Check username/password credentials
   - Verify user has appropriate permissions

### Debug Mode vs Release Mode

- **Debug mode** (`onvifd_debug`): Includes debug symbols and verbose logging
- **Release mode** (`onvifd`): Optimized for production use

Use debug mode for development and troubleshooting, release mode for production deployment.

## Security Notes

- Default credentials are used for convenience in development
- Change default passwords in production environments
- Consider using SSH instead of telnet for production deployments
- Ensure FTP connections are secured in production environments
