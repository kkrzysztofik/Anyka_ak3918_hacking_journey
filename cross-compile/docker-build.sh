#!/bin/bash

# Build the cross-compilation Docker image
echo "Building Anyka cross-compilation environment..."
docker build -t anyka-cross-compile .

echo "Docker image built successfully!"
echo ""
echo "Usage:"
echo "  To start interactive shell:"
echo "    docker run -it --rm -v \${PWD}:/workspace anyka-cross-compile"
echo ""
echo "  To build a specific project:"
echo "    docker run --rm -v \${PWD}:/workspace anyka-cross-compile make -C /workspace/onvif"
echo ""
echo "  To run any command:"
echo "    docker run --rm -v \${PWD}:/workspace anyka-cross-compile <command>"
