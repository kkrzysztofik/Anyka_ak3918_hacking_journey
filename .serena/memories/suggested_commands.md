# Essential Development Commands

## Primary Build Commands
```bash
# Build ONVIF server (primary development target)
make -C cross-compile/onvif

# Debug build with symbols
make -C cross-compile/onvif debug

# Release build (optimized)
make -C cross-compile/onvif release

# Clean build artifacts
make -C cross-compile/onvif clean
```

## Unit Testing Commands (MANDATORY)
```bash
# Install unit test dependencies
./cross-compile/onvif/tests/install_dependencies.sh

# Run all unit tests
make -C cross-compile/onvif test

# Run utility unit tests specifically
make -C cross-compile/onvif test-utils

# Run tests with coverage analysis
make -C cross-compile/onvif test-coverage

# Generate HTML coverage report
make -C cross-compile/onvif test-coverage-html

# Run tests with valgrind (memory checking)
make -C cross-compile/onvif test-valgrind
```

## Documentation Commands (MANDATORY)
```bash
# Generate Doxygen documentation
make -C cross-compile/onvif docs

# View documentation
xdg-open cross-compile/onvif/docs/html/index.html
```

## Static Analysis Commands
```bash
# Run all static analysis tools
make -C cross-compile/onvif static-analysis

# Individual analysis tools
make -C cross-compile/onvif clang-analyze
make -C cross-compile/onvif cppcheck-analyze
make -C cross-compile/onvif snyk-analyze
```

## SD Card Testing Workflow (Primary Development Method)
```bash
# Build and copy to SD payload
cd cross-compile/onvif && make
cp out/onvifd ../../SD_card_contents/anyka_hack/usr/bin/

# Copy configuration
cp configs/anyka_cfg.ini ../../SD_card_contents/anyka_hack/onvif/

# Deploy SD card to device and test (safe - leaves original firmware intact)
```

## Integration Testing Commands
```bash
# Run Python-based integration tests
cd integration-tests
pip install -r requirements.txt
python run_tests.py

# Run specific test categories
python run_tests.py --category device
python run_tests.py --category ptz
python run_tests.py --category media
```

## Code Quality Commands
```bash
# Format code
make -C cross-compile/onvif format

# Check formatting
make -C cross-compile/onvif format-check

# Lint code
make -C cross-compile/onvif lint

# Generate compile_commands.json for clangd
make -C cross-compile/onvif compile-commands
```

## Docker Development (Alternative)
```bash
# Build Docker development environment
cd cross-compile
./docker-build.sh  # Linux/Mac
# or
.\docker-build.ps1  # Windows PowerShell

# Compile using Docker
docker run --rm -v ${PWD}:/workspace anyka-cross-compile make -C /workspace/onvif
```

## System Commands (WSL Ubuntu)
```bash
# Check bash version (must use bash syntax)
bash --version

# List project structure
find . -type f -name "*.c" | head -10
find . -type f -name "*.h" | head -10

# Search for patterns in code
grep -r "pattern" cross-compile/onvif/src/ -n

# Check file status
test -f cross-compile/onvif/out/onvifd && echo "Binary exists" || echo "Binary missing"
```