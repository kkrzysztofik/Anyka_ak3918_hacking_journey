#!/bin/bash

# cloc_our_code.sh - Count lines of code for our custom implementation
# Excludes external tools, anyka_reference, generated files, and build artifacts

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if cloc is installed
if ! command -v cloc &> /dev/null; then
    echo -e "${RED}Error: cloc is not installed. Please install it first:${NC}"
    echo "  Ubuntu/Debian: sudo apt-get install cloc"
    echo "  macOS: brew install cloc"
    echo "  Or download from: https://github.com/AlDanial/cloc"
    exit 1
fi

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo -e "${BLUE}Counting lines of code for our custom implementation...${NC}"
echo -e "${YELLOW}Project root: $PROJECT_ROOT${NC}"
echo

# Change to project root
cd "$PROJECT_ROOT"

# Run cloc with exclusions
cloc \
    --exclude-dir=anyka_reference,gsoap,node_modules,out,lib,include,wsdl,generated,orig,toolchain,e2e,testspecs,verbose \
    --exclude-ext=o,so,a,log,json,ini,dat,xsd,wsdl,nsmap \
    --exclude-list-file=<(cat << 'EOF'
# Generated files
cross-compile/onvif/src/generated/
cross-compile/onvif/out/
cross-compile/onvif/lib/
cross-compile/onvif/include/
cross-compile/onvif/wsdl/
cross-compile/onvif/compile_commands.json
cross-compile/onvif/Doxyfile
cross-compile/onvif/*.log
cross-compile/onvif/out.log
cross-compile/onvif/RECV.log
cross-compile/onvif/TEST.log

# External tools and references
cross-compile/anyka_reference/
cross-compile/gsoap/
cross-compile/www/

# Build artifacts and generated content
cross-compile/onvif/out/
cross-compile/onvif/lib/
cross-compile/onvif/include/
cross-compile/onvif/wsdl/
cross-compile/onvif/compile_commands.json
cross-compile/onvif/Doxyfile
cross-compile/onvif/*.log

# Other external content
orig/
toolchain/
e2e/
testspecs/
verbose/

# Documentation and config files
*.md
*.txt
*.pdf
*.png
*.jpg
*.jpeg
*.gif
*.svg
*.ico
*.ini
*.cfg
*.conf
*.json
*.xml
*.wsdl
*.xsd
*.dat
*.log
*.o
*.so
*.a
*.exe
*.dll
*.dylib
EOF
) \
    --include-lang="C,C++,C/C++ Header,Makefile,Shell,Python,JavaScript,HTML,CSS,SQL,Markdown" \
    --by-file-by-lang \
    --progress-rate=0 \
    .

echo
echo -e "${GREEN}Done! This count includes only our custom implementation code.${NC}"
echo -e "${YELLOW}Excluded:${NC}"
echo "  - External tools (anyka_reference, gsoap, toolchain)"
echo "  - Generated files (gsoap output, build artifacts)"
echo "  - Libraries and headers from vendor"
echo "  - Documentation and configuration files"
echo "  - Build outputs and logs"
