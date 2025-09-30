#!/bin/bash
# Common utility functions for ONVIF project scripts
# Shared functionality between format_code.sh and lint_code.sh

# =============================================================================
# Configuration and Constants
# =============================================================================

# Get script directory and project root
if [[ -z "${SCRIPT_DIR:-}" ]]; then
    readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fi
if [[ -z "${PROJECT_ROOT:-}" ]]; then
    readonly PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
fi

# Colors for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly CYAN='\033[0;36m'
readonly NC='\033[0m'

# =============================================================================
# Logging Functions
# =============================================================================

log_info() {
    echo -e "${BLUE}$1${NC}"
}

log_success() {
    echo -e "${GREEN}$1${NC}"
}

log_warning() {
    echo -e "${YELLOW}$1${NC}"
}

log_error() {
    echo -e "${RED}$1${NC}"
}

log_debug() {
    echo -e "${CYAN}$1${NC}"
}

# =============================================================================
# File Discovery Functions
# =============================================================================

find_specific_files() {
    local files=()
    IFS=',' read -ra FILE_ARRAY <<< "$FILES_ONLY"

    for file in "${FILE_ARRAY[@]}"; do
        # Remove leading/trailing whitespace
        file=$(echo "$file" | xargs)

        # Convert to absolute path if relative
        if [[ ! "$file" = /* ]]; then
            file="$PROJECT_ROOT/$file"
        fi

        if [[ -f "$file" ]]; then
            files+=("$file")
        else
            log_warning "Warning: File not found: $file"
        fi
    done

    printf '%s\n' "${files[@]}"
}

find_all_c_files() {
    find "$PROJECT_ROOT/src" "$PROJECT_ROOT/tests" \( -name "*.c" -o -name "*.h" \) -print0 2>/dev/null | \
        while IFS= read -r -d '' file; do
            echo "$file"
        done
}

find_global_c_files() {
    find "$PROJECT_ROOT/src" \( -name "*.c" -o -name "*.h" \) -print0 2>/dev/null | \
        while IFS= read -r -d '' file; do
            echo "$file"
        done
}

find_test_c_files() {
    find "$PROJECT_ROOT/tests/src" \( -name "*.c" -o -name "*.h" \) -print0 2>/dev/null | \
        while IFS= read -r -d '' file; do
            echo "$file"
        done
}

find_c_files() {
    if [[ -n "${FILES_ONLY:-}" ]]; then
        find_specific_files
    else
        find_all_c_files
    fi
}

# =============================================================================
# File Utility Functions
# =============================================================================

is_generated_file() {
    [[ "$1" == *"/generated/"* ]] || [[ "$1" == *"/build_native/"* ]]
}

get_relative_path() {
    echo "${1#$PROJECT_ROOT/}"
}

# =============================================================================
# Common Processing Functions
# =============================================================================

skip_generated_file() {
    # Skip generated files entirely - they should not be processed
    local file="$1"
    local relative_file
    relative_file=$(get_relative_path "$file")

    log_warning "~ $relative_file (skipped - generated file)"
    return 0  # No change, not an error
}

process_file_with_mode() {
    local file="$1"
    local mode="$2"  # "check", "dry-run", or "process"
    local process_func="$3"  # Function to call for processing
    local check_func="$4"   # Function to call for checking
    local dry_run_func="$5" # Function to call for dry run
    local relative_file
    relative_file=$(get_relative_path "$file")

    if [[ "${VERBOSE:-false}" == "true" ]]; then
        log_info "Processing: $relative_file"
    fi

    # Skip generated files entirely
    if is_generated_file "$file"; then
        skip_generated_file "$file"
        return 0
    fi

    # Process regular files based on mode
    case "$mode" in
        "check")
            "$check_func" "$file"
            ;;
        "dry-run")
            "$dry_run_func" "$file"
            ;;
        "process")
            "$process_func" "$file"
            ;;
        *)
            log_error "Unknown processing mode: $mode"
            return 2
            ;;
    esac
}

# =============================================================================
# Common Summary Functions
# =============================================================================

show_summary() {
    local total_files="$1"
    local changed_files="$2"
    local error_files="$3"
    local mode="$4"  # "check", "dry-run", or "process"
    local tool_name="$5"  # Tool name for display

    echo ""
    case "$mode" in
        "check")
            if [[ $error_files -eq 0 ]]; then
                log_success "All files pass ${tool_name} checks!"
                return 0
            else
                log_error "$error_files file(s) have ${tool_name} issues"
                return 1
            fi
            ;;
        "dry-run")
            log_info "Summary: $changed_files file(s) would be processed"
            return 0
            ;;
        "process")
            log_info "Summary: $changed_files file(s) processed, $error_files error(s)"
            return $((error_files > 0 ? 1 : 0))
            ;;
        *)
            log_error "Unknown summary mode: $mode"
            return 2
            ;;
    esac
}

# =============================================================================
# Common Argument Parsing Functions
# =============================================================================

parse_common_arguments() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                return 0  # Let calling script handle help
                ;;
            -d|--dry-run)
                DRY_RUN=true
                shift
                ;;
            -c|--check)
                CHECK_ONLY=true
                shift
                ;;
            -f|--files)
                FILES_ONLY="$2"
                shift 2
                ;;
            *)
                # Unknown option, let calling script handle it
                return 1
                ;;
        esac
    done
    return 0
}

# =============================================================================
# Common Main Execution Functions
# =============================================================================

run_common_main() {
    local tool_name="$1"
    local check_prereq_func="$2"  # Function to check prerequisites
    local process_files_func="$3" # Function to process files

    log_info "${tool_name}"
    echo "$(printf '=%.0s' $(seq 1 ${#tool_name}))"

    # Check prerequisites
    if [[ -n "$check_prereq_func" ]]; then
        "$check_prereq_func"
    fi

    # Find and process files
    log_info "Scanning for C source files..."
    local files
    mapfile -t files < <(find_c_files)

    # Process the files
    "$process_files_func" "${files[@]}"
}

# =============================================================================
# Utility Functions for Scripts
# =============================================================================

# Function to check if a command exists
command_exists() {
    command -v "$1" &> /dev/null
}

# Function to get version of a command
get_command_version() {
    local cmd="$1"
    local version_pattern="$2"

    if command_exists "$cmd"; then
        local version
        version=$($cmd --version 2>/dev/null | grep -o "$version_pattern" | head -1)
        echo "$version"
    else
        echo ""
    fi
}

# Function to show installation instructions
show_installation_instructions() {
    local tool_name="$1"
    local ubuntu_package="$2"
    local centos_package="$3"
    local download_url="$4"

    log_error "Error: $tool_name is not installed"
    cat << EOF
Please install $tool_name:
  Ubuntu/Debian: sudo apt-get install $ubuntu_package
  CentOS/RHEL: sudo yum install $centos_package
EOF
    if [[ -n "$download_url" ]]; then
        echo "  Or download from: $download_url"
    fi
}

# Function to check if compile_commands.json exists and generate if needed
check_compile_commands() {
    if [[ ! -f "$PROJECT_ROOT/compile_commands.json" ]]; then
        log_warning "Warning: compile_commands.json not found"
        log_info "Generating compile_commands.json..."
        if [[ -f "$PROJECT_ROOT/scripts/generate_compile_commands.sh" ]]; then
            "$PROJECT_ROOT/scripts/generate_compile_commands.sh"
        else
            log_error "Error: generate_compile_commands.sh not found"
            log_info "Please run 'make compile-commands' to generate compile_commands.json"
            return 1
        fi
    fi
    return 0
}
