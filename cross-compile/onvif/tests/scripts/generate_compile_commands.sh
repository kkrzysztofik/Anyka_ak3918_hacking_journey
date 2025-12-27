#!/bin/bash
# Generate compile_commands.json for test files using Bear
# This script generates compile_commands.json specifically for the tests directory

set -e

echo "Generating compile_commands.json for test files using Bear..."

# Change to tests directory
cd "$(dirname "$0")/.." || {
    echo "Error: Cannot change to tests directory" >&2
    exit 1
}

# Clean first
make clean

# Use Bear to capture the build commands
bear -- make

# Check if compile_commands.json was generated
if [[ ! -f "compile_commands.json" ]]; then
    echo "Error: compile_commands.json was not generated in tests directory" >&2
    exit 1
fi

echo "Test compile_commands.json generated successfully"
echo "File size: $(wc -c < compile_commands.json) bytes"
echo "Number of entries: $(jq length compile_commands.json 2>/dev/null || echo 'unknown')"

echo ""
echo "🎯 The tests/compile_commands.json now includes:"
echo "   • Test files (unit/)"
echo "   • Mock files (mocks/)"
echo "   • Main project source files used in tests"
echo ""
echo "💡 Your IDE should now provide full IntelliSense support for test files!"
