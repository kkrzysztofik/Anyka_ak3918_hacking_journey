#!/bin/bash
# Generate compile_commands.json for main ONVIF project using Bear
# This script generates compile_commands.json for the main project source files

set -e

echo "Generating compile_commands.json for main ONVIF project using Bear..."

# Clean first
make clean

# Use Bear to capture the build commands
bear -- make

# Check if compile_commands.json was generated
if [ ! -f "compile_commands.json" ]; then
    echo "Error: compile_commands.json was not generated"
    exit 1
fi

echo "Main project compile_commands.json generated successfully"
echo "File size: $(wc -c < compile_commands.json) bytes"
echo "Number of entries: $(jq length compile_commands.json 2>/dev/null || echo 'unknown')"

echo ""
echo "🎯 The compile_commands.json now includes:"
echo "   • Main project source files (src/)"
echo "   • Generated files (src/generated/)"
echo "   • Platform-specific files (src/platform/)"
echo ""
echo "💡 Your IDE should now provide full IntelliSense support for main project files!"
echo ""
echo "📝 To generate compile_commands.json for test files, run:"
echo "   ./tests/scripts/generate_compile_commands.sh"
