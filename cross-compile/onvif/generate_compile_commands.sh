#!/bin/bash
# Generate compile_commands.json using Bear with additional Anyka reference includes
# This replaces the Python script with a more robust solution

set -e

echo "Generating compile_commands.json using Bear..."

# Clean first
make clean

# Use Bear to capture the build commands
bear -- make

# Check if compile_commands.json was generated
if [ ! -f "compile_commands.json" ]; then
    echo "Error: compile_commands.json was not generated"
    exit 1
fi

echo "compile_commands.json generated successfully"
echo "File size: $(wc -c < compile_commands.json) bytes"
echo "Number of entries: $(jq length compile_commands.json 2>/dev/null || echo 'unknown')"
