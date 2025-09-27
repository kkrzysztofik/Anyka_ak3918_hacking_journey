# Technology Stack

## Project Type
Embedded firmware system for IP cameras with ONVIF 2.5 compliance, including cross-compilation toolchain, SD-card development environment, and web-based management interface.

## Core Technologies

### Primary Language(s)
- **Language**: C (embedded systems programming)
- **Runtime/Compiler**: GCC cross-compiler for ARM architecture (Anyka AK3918 SoC)
- **Language-specific tools**: GNU Make, CMake for build system, Doxygen for documentation

### Key Dependencies/Libraries
- **gSOAP 2.8**: SOAP/XML processing for ONVIF services and web service communication
- **libcurl**: HTTP client functionality for network communication
- **libxml2**: XML parsing and generation for ONVIF protocol handling
- **OpenSSL**: Cryptographic functions, SSL/TLS support, and secure communication
- **Anyka SDK**: Hardware abstraction layer and device drivers for AK3918 SoC
- **uClibc**: Embedded C library optimized for resource-constrained systems
- **BusyBox**: Lightweight Unix utilities for embedded Linux environment

### Application Architecture
**Layered Architecture with Platform Abstraction**:
- **Platform Layer**: Hardware abstraction for Anyka AK3918 SoC
- **Core Services Layer**: ONVIF service implementations (Device, Media, PTZ, Imaging)
- **Network Layer**: HTTP/SOAP, RTSP, WS-Discovery protocol implementations
- **Utility Layer**: Shared utilities for memory management, validation, logging
- **Application Layer**: Main daemon and web interface

### Data Storage
- **Primary storage**: SquashFS compressed filesystem images for root filesystem
- **Configuration**: JSON/XML configuration files stored in flash memory
- **Caching**: In-memory caching for frequently accessed data (profiles, settings)
- **Data formats**: XML for ONVIF SOAP messages, JSON for configuration, H.264 for video streams

### External Integrations
- **ONVIF Protocol**: Full ONVIF 2.5 specification compliance for IP camera standards
- **RTSP Streaming**: Real-time video streaming protocol for H.264 video delivery
- **WS-Discovery**: Web Services Discovery for automatic camera detection
- **HTTP/SOAP**: Web service communication for camera control and configuration
- **UDP Multicast**: Network discovery and communication protocols

### Monitoring & Dashboard Technologies
- **Dashboard Framework**: Vanilla JavaScript with HTML5/CSS3 for web interface
- **Real-time Communication**: WebSocket for live updates and control
- **Visualization Libraries**: Canvas API for video display, Chart.js for metrics
- **State Management**: File system as source of truth with JSON configuration

## Development Environment

### Build & Development Tools
- **Build System**: GNU Make with cross-compilation support
- **Package Management**: Custom toolchain with pre-compiled libraries
- **Development workflow**: SD-card based testing, UART debugging, hot reload for web interface

### Code Quality Tools
- **Static Analysis**: cppcheck, clang-static-analyzer, PVS-Studio
- **Formatting**: Custom C formatting standards with 2-space indentation
- **Testing Framework**: Custom test suite in integration-tests/ directory
- **Documentation**: Doxygen with HTML generation for API documentation

### Version Control & Collaboration
- **VCS**: Git with feature branch workflow
- **Branching Strategy**: Feature branches with comprehensive code review
- **Code Review Process**: Mandatory review for all changes with security and performance analysis

### Dashboard Development
- **Live Reload**: File watchers for web interface development
- **Port Management**: Configurable HTTP port (default 80) for web interface
- **Multi-Instance Support**: Single camera per instance, multiple cameras via network

## Deployment & Distribution
- **Target Platform(s)**: Anyka AK3918-based IP cameras (embedded Linux)
- **Distribution Method**: SD-card payload system for development, flash memory for production
- **Installation Requirements**: UART access for debugging, SD card slot for development
- **Update Mechanism**: SD-card based updates for development, OTA updates for production

## Technical Requirements & Constraints

### Performance Requirements
- **Response Time**: Sub-second response for all ONVIF operations
- **Video Streaming**: Real-time H.264 streaming at 1080p@30fps
- **Memory Usage**: Optimized for embedded systems with limited RAM (typically 64-128MB)
- **Startup Time**: Camera ready for operation within 30 seconds of power-on
- **Concurrent Connections**: Support for multiple simultaneous ONVIF clients

### Compatibility Requirements
- **Platform Support**: Anyka AK3918 SoC with ARM architecture
- **Dependency Versions**: gSOAP 2.8+, libcurl 7.0+, OpenSSL 1.1+
- **Standards Compliance**: ONVIF 2.5 specification, RTSP RFC 2326, WS-Discovery

### Security & Compliance
- **Security Requirements**:
  - Input validation and sanitization for all network inputs
  - Secure authentication with digest authentication
  - Buffer overflow prevention and memory safety
  - Secure communication protocols (HTTPS, WSS)
- **Compliance Standards**: ONVIF 2.5 specification compliance
- **Threat Model**:
  - Network-based attacks (injection, DoS)
  - Authentication bypass attempts
  - Memory corruption vulnerabilities
  - Unauthorized access to video streams

### Scalability & Reliability
- **Expected Load**: 1-10 concurrent ONVIF clients per camera
- **Availability Requirements**: 99.9% uptime for camera services
- **Growth Projections**: Support for additional camera models and features

## Technical Decisions & Rationale

### Decision Log
1. **C Language Choice**: Chosen for embedded systems performance, direct hardware access, and compatibility with existing Anyka SDK
2. **ONVIF 2.5 Compliance**: Industry standard for IP camera interoperability, enables integration with existing security systems
3. **SD-card Development Environment**: Safe testing without modifying flash memory, enables rapid iteration and debugging
4. **Platform Abstraction Layer**: Enables code reuse across different camera models and hardware configurations
5. **gSOAP for SOAP Processing**: Mature, well-tested library for ONVIF SOAP message handling
6. **SquashFS for Root Filesystem**: Compressed filesystem saves flash memory space while maintaining functionality
7. **UART Debugging Interface**: Essential for embedded development and troubleshooting
8. **Web-based Management Interface**: User-friendly configuration without requiring specialized tools

## Known Limitations

- **Hardware Dependency**: Firmware is specific to Anyka AK3918 SoC and may not work on other platforms
- **Memory Constraints**: Limited RAM requires careful memory management and optimization
- **Flash Storage**: Limited flash memory requires efficient filesystem design
- **Network Bandwidth**: Video streaming performance depends on network conditions
- **Power Management**: Continuous operation may require external power supply
- **Temperature Sensitivity**: Performance may be affected by operating temperature
- **Single Camera Focus**: Current implementation focuses on single camera management

## Development Workflow

### Build Process
```bash
# Build the ONVIF project
make -C cross-compile/onvif

# Build with verbose output
make -C cross-compile/onvif VERBOSE=1

# Clean build artifacts
make -C cross-compile/onvif clean

# Generate documentation
make -C cross-compile/onvif docs
```

### Testing Process
1. **Compilation Testing**: Verify code compiles without errors or warnings
2. **Static Analysis**: Run automated analysis tools for code quality
3. **SD-card Testing**: Test functionality using SD-card payload system
4. **Integration Testing**: Test with ONVIF clients and verify compliance
5. **Performance Testing**: Measure response times and resource usage
6. **Security Testing**: Validate input handling and security measures

### Documentation Process
1. **Code Documentation**: Update Doxygen comments for all changes
2. **API Documentation**: Generate HTML documentation from source code
3. **User Documentation**: Update user guides and configuration instructions
4. **Technical Documentation**: Document architectural decisions and rationale

## Code Organization Standards

### Include Order (MANDATORY)
1. **System headers first** (e.g., `#include <stdio.h>`, `#include <stdlib.h>`)
2. **Third-party library headers** (e.g., `#include <curl/curl.h>`)
3. **Project-specific headers** (e.g., `#include "platform.h"`, `#include "common.h"`)

### Global Variable Naming (MANDATORY)
- **ALL global variables MUST start with `g_<module>_<variable_name>`**
- **Module prefix** should match the source file or functional area
- **Variable name** should be descriptive and follow snake_case convention

### File Organization
- **Source files**: `onvif_<service>.c` (e.g., `onvif_device.c`)
- **Header files**: `onvif_<service>.h` (e.g., `onvif_device.h`)
- **Utility files**: `<category>_utils.c` (e.g., `memory_utils.c`)
- **Platform files**: `platform_<platform>.c` (e.g., `platform_anyka.c`)

## Security Architecture

### Input Validation
- **All network inputs** must be validated and sanitized
- **Buffer bounds checking** for all string operations
- **XML/SOAP input validation** to prevent injection attacks
- **Numeric range validation** for all parameters

### Memory Management
- **Safe string operations** using `strncpy()` instead of `strcpy()`
- **Memory initialization** with `memset()` or `calloc()`
- **Resource cleanup** in all error paths
- **Buffer overflow prevention** with proper bounds checking

### Authentication & Authorization
- **Digest authentication** for ONVIF services
- **Session management** with secure token handling
- **Role-based access control** for different user types
- **Rate limiting** for authentication attempts

### Network Security
- **HTTPS/WSS support** for secure communication
- **Input sanitization** for all network data
- **Error message handling** without information leakage
- **Secure configuration** with encrypted credentials
