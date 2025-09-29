# Codebase Structure and Organization

## Primary Development Areas

### Cross-Compilation Projects (`cross-compile/`)
- **`cross-compile/onvif/`** — **CURRENT FOCUS** - Complete ONVIF 2.5 implementation
  - Full SOAP-based web services stack for IP camera control and streaming
  - Device, Media, PTZ, and Imaging services
  - Platform abstraction layer integration
  - Comprehensive unit testing framework with CMocka

- **`cross-compile/onvif/tests/`** — **MANDATORY** - Unit testing framework
  - CMocka-based tests for utility functions
  - Coverage analysis and memory leak detection
  - All utility functions must have corresponding unit tests

- **`cross-compile/anyka_reference/akipc/`** — Authoritative vendor reference code
  - Complete reference implementation from chip/vendor
  - Canonical example for understanding camera behavior
  - Use for reverse-engineering APIs and initialization sequences

- **`cross-compile/anyka_reference/component/`** — Reusable component libraries
  - Pre-compiled binaries and headers for camera subsystems
  - Drivers, third-party libraries, and helper tools
  - Hardware abstraction components

## Source Code Organization (`cross-compile/onvif/src/`)

### Core System Components (`src/core/`)
```
src/core/
├── main/               # Main daemon entry point (onvifd.c)
├── config/             # Configuration management
└── lifecycle/          # Service lifecycle management
```

### Platform Abstraction (`src/platform/`)
```
src/platform/
├── platform_anyka.c   # Anyka AK3918 implementation
├── platform.h         # Platform interface
└── adapters/           # Hardware-specific adapters
```

### ONVIF Services (`src/services/`)
```
src/services/
├── device/             # Device service implementation
├── media/              # Media service implementation
├── ptz/                # PTZ service implementation
├── imaging/            # Imaging service implementation
├── snapshot/           # Snapshot service implementation
└── common/             # Shared service types and utilities
```

### Network Protocol Layer (`src/networking/`)
```
src/networking/
├── http/               # HTTP/SOAP handling
├── rtsp/               # RTSP streaming implementation
├── discovery/          # WS-Discovery protocol
└── common/             # Shared networking utilities
```

### Protocol Handling (`src/protocol/`)
```
src/protocol/
├── soap/               # SOAP processing
├── xml/                # XML utilities
├── response/           # Response handling
└── gsoap/              # gSOAP integration
```

### Utility Functions (`src/utils/`) — **CRITICAL**
```
src/utils/
├── memory/             # Memory management utilities
├── string/             # String manipulation utilities
├── error/              # Error handling utilities
├── network/            # Network utility functions
├── logging/            # Logging utilities
├── validation/         # Input validation utilities
├── security/           # Security utilities
├── service/            # Service utilities
└── stream/             # Stream configuration utilities
```

## Testing and Documentation Structure

### Unit Testing (`cross-compile/onvif/tests/`)
```
tests/
├── unit/               # Unit test files organized by module
├── mocks/              # Mock implementations for testing
├── scripts/            # Test automation scripts
└── build_native/       # Native build artifacts for tests
```

### E2E Testing (`e2e/`)
```
e2e/
├── tests/              # Python-based E2E tests
├── test_data/          # Test data and configurations
├── logs/               # Test execution logs
└── reports/            # Test coverage and results
```

### Documentation (`cross-compile/onvif/docs/`)
- Generated Doxygen documentation in HTML format
- API documentation for all public functions
- Architecture diagrams and design documentation

## SD Card Payload System (`SD_card_contents/`)
```
SD_card_contents/anyka_hack/
├── usr/bin/            # Compiled binaries for device testing
├── onvif/              # ONVIF configuration files
├── web_interface/      # Web UI components
└── [various system files for camera operation]
```

## Configuration and Build Files
- **Makefiles**: Comprehensive build system with debug/release targets
- **Docker**: Cross-compilation environment setup
- **Doxygen**: Documentation generation configuration
- **clangd**: IDE support and language server configuration
- **Static Analysis**: Configuration for multiple analysis tools

## Key Directory Relationships
- **Development**: `cross-compile/onvif/src/` → Build → `cross-compile/onvif/out/`
- **Testing**: Compiled binary → `SD_card_contents/anyka_hack/usr/bin/`
- **Documentation**: Source files → `cross-compile/onvif/docs/html/`
- **Reference**: `cross-compile/anyka_reference/` for understanding vendor implementations