#!/bin/bash
# install_dependencies.sh - Install dependencies for ONVIF unit testing
# Author: kkrzysztofik
# Date: 2025

set -e

echo "Installing dependencies for ONVIF unit testing..."

# Check if running on Ubuntu/Debian
if ! command -v apt-get &> /dev/null; then
    echo "Error: This script is designed for Ubuntu/Debian systems" >&2
    echo "Please install CMocka manually for your system" >&2
    exit 1
fi

# Update package list
echo "Updating package list..."
sudo apt-get update

# Install CMocka development package
echo "Installing CMocka..."
sudo apt-get install -y libcmocka-dev

# Install additional development tools
echo "Installing additional development tools..."
sudo apt-get install -y build-essential gdb lcov wget unzip

# Verify CMocka installation
echo "Verifying CMocka installation..."
if pkg-config --exists cmocka; then
    echo "✅ CMocka installed successfully"
    echo "CMocka version: $(pkg-config --modversion cmocka)"
    echo "CMocka flags: $(pkg-config --cflags --libs cmocka)"
else
    echo "❌ CMocka installation failed" >&2
    exit 1
fi

# Compile native libraries for testing
echo "Compiling native libraries..."
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TESTS_DIR="$(dirname "$SCRIPT_DIR")"

if [[ -f "${SCRIPT_DIR}/compile_local_libs.sh" ]]; then
    bash "${SCRIPT_DIR}/compile_local_libs.sh" || { echo "❌ Library compilation failed" >&2; exit 1; }
else
    echo "❌ compile_local_libs.sh not found in ${SCRIPT_DIR}" >&2
    exit 1
fi

echo ""
echo "🎉 Installation completed successfully!"
echo ""
echo "Available unit test commands:"
echo "  make test-utils       - Run utility unit tests"
echo "  make test-all         - Run all unit tests"
echo "  make test-coverage    - Run tests with coverage"
echo "  make test-coverage-html - Generate HTML coverage report"
echo "  make test-coverage-report - Generate coverage summary"
echo ""
echo "From the main project directory:"
echo "  make test             - Run all unit tests"
echo "  make test-utils       - Run utility unit tests"
echo "  make test-coverage-html - Generate HTML coverage report"
echo ""
echo "Note: Integration tests are handled by a separate project"
