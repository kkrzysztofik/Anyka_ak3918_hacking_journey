# Technology Stack and Build System

## Programming Languages
- **C99** - Primary development language with strict standards compliance
- **Shell/Bash** - Build scripts, development automation, and system integration
- **Python** - Integration testing framework with pytest
- **JavaScript/HTML/CSS** - Web interface components

## Cross-Compilation Toolchain
- **Target Platform**: Anyka AK3918 SoC (ARM-based)
- **Toolchain**: `arm-anykav200-linux-uclibcgnueabi-gcc`
- **Toolchain Path**: `/opt/arm-anykav200-crosstool/usr/bin`
- **Build Environment**: WSL2 Ubuntu 24.04 with native cross-compilation

## Core Libraries and Dependencies
- **gSOAP** - SOAP/XML web services framework (`-lgsoap -lgsoapck`)
- **Anyka SDK Libraries**:
  - Video: `libakuio`, `libakispsdk`, `libakv_encode`, `libmpi_venc`
  - Audio: `libakae`, `libakaudiocodec`, `libmpi_aenc`, `libmpi_aed`
  - Platform: `libplat_common`, `libplat_thread`, `libplat_vi`, `libplat_vpss`
  - Streaming: `libakstreamenc`, `libapp_rtsp`, `libapp_net`
- **Base64** - Encoding utilities (`-lb64`)
- **CMocka** - Unit testing framework for C
- **pthread** - Threading support

## Build System
- **Primary Build Tool**: GNU Make with comprehensive Makefile
- **Build Types**: 
  - `debug` - Debug symbols, -g -O0, extensive warnings
  - `release` - Optimized -Os, production build
- **Docker Support**: Ubuntu 24.04 based container with full toolchain
- **Static Analysis**: Clang Static Analyzer, Cppcheck, Snyk Code

## Development Tools
- **Documentation**: Doxygen with GraphViz for API documentation
- **Code Quality**: clang-format, custom linting scripts
- **Memory Analysis**: Valgrind, AddressSanitizer support
- **IDE Support**: clangd configuration for modern editors
- **Version Control**: Git with comprehensive .gitignore

## Testing Frameworks
- **Unit Testing**: CMocka framework for isolated function testing
- **Integration Testing**: Python-based test suite with pytest
- **Coverage Analysis**: gcov/lcov for coverage reporting
- **Memory Testing**: Valgrind integration for leak detection
- **Static Analysis**: Multiple tools (Clang, Cppcheck, Snyk) integrated into build system