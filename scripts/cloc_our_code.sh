#!/bin/bash

# cloc_our_code.sh - Count lines of code for our custom implementation
# Excludes external tools, anyka_reference, generated files, and build artifacts

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "${SCRIPT_DIR}/common.sh"

anyka_check_commands cloc

PROJECT_ROOT="${ANYKA_REPO_ROOT}"

log_info "Counting lines of code for our custom implementation..."
echo -e "${YELLOW}Project root: $PROJECT_ROOT${NC}"
echo

# Change to project root
cd "$PROJECT_ROOT"

# Run cloc with exclusions
cloc \
    --exclude-dir=anyka_reference,gsoap,xiu,patches,node_modules,target,coverage,out,lib,include,wsdl,generated,orig,toolchain,e2e,testspecs,verbose \
    --exclude-ext=o,so,a,log,json,ini,dat,xsd,wsdl,nsmap,lock \
    --exclude-list-file=<(cat << 'EOF'
# Generated files (legacy C++ ONVIF)
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

# Cargo dependencies and build artifacts (Rust)
cross-compile/target/
cross-compile/onvif-rust/target/
cross-compile/streaming-lib/target/
cross-compile/Cargo.lock

# NPM dependencies and build artifacts (WebUI)
cross-compile/www/node_modules/
cross-compile/www/dist/
cross-compile/www/coverage/
cross-compile/www/package-lock.json

# Test coverage reports
validation/rust/coverage/
cross-compile/onvif-rust/coverage/

# External tools and references
cross-compile/anyka_reference/
cross-compile/gsoap/
cross-compile/xiu/
cross-compile/patches/

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
    --include-lang="C,C++,C/C++ Header,Rust,TypeScript,JavaScript,JSX,HTML,CSS,Makefile,TOML,Bourne Shell,Bourne Again Shell,Python,Markdown" \
    --by-file-by-lang \
    --progress-rate=0 \
    . 2>&1 | grep -v -e "Line count, exceeded timeout" -e "^[0-9]* error"

echo
echo -e "${GREEN}Done! This count includes only our custom implementation code.${NC}"
echo -e "${YELLOW}Included projects:${NC}"
echo "  - onvif-rust (Rust ONVIF implementation)"
echo "  - www (React WebUI)"
echo "  - Legacy C++ ONVIF (cross-compile/onvif)"
echo "  - Scripts and validation tools"
echo
echo -e "${YELLOW}Excluded:${NC}"
echo "  - External tools (anyka_reference, gsoap, xiu, patches, toolchain)"
echo "  - Generated files (gsoap output, build artifacts)"
echo "  - Dependencies (cargo target/, node_modules/)"
echo "  - Test coverage reports (coverage/)"
echo "  - Libraries and headers from vendor"
echo "  - Documentation and configuration files"
echo "  - Build outputs and logs"
