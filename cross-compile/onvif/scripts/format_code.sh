#!/bin/bash
# Code formatting script for ONVIF project
# Uses clang-format to maintain consistent code style

set -uo pipefail

# =============================================================================
# Load Common Functions
# =============================================================================

# Source common functions
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

# =============================================================================
# Script-Specific Configuration
# =============================================================================

# Default configuration
DRY_RUN=false
FILES_ONLY=""
CHECK_ONLY=false

# =============================================================================
# Script-Specific Functions
# =============================================================================

print_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Format C source files in the ONVIF project using clang-format

OPTIONS:
  -h, --help          Show this help message
  -d, --dry-run       Show what would be changed without making changes
  -c, --check         Check if files are properly formatted (exit 1 if not)
  -f, --files FILE    Format specific files (comma-separated)

EXAMPLES:
  $0                  # Format all C files in the project
  $0 --dry-run        # Show what would be changed
  $0 --check          # Check if files are properly formatted
  $0 --files src/core/main.c,src/services/device/onvif_device.c
EOF
    return 0
}

check_clang_format() {
    if ! command_exists clang-format; then
        show_installation_instructions "clang-format" "clang-format" "clang-tools-extra" ""
        exit 1
    fi

    local version
    version=$(get_command_version clang-format '[0-9]\+\.[0-9]\+')
    log_info "Using clang-format version: $version"
    return 0
}

check_file_formatting() {
    local file="$1"
    local relative_file
    relative_file=$(get_relative_path "$file")

    if clang-format --dry-run --Werror "$file" &>/dev/null; then
        log_success "✓ $relative_file (properly formatted)"
        return 0
    else
        log_error "✗ $relative_file (not properly formatted)"
        return 1
    fi
}

show_dry_run() {
    local file="$1"
    local relative_file
    relative_file=$(get_relative_path "$file")

    if clang-format --dry-run --Werror "$file" &>/dev/null; then
        log_success "✓ $relative_file (no changes needed)"
        return 0
    else
        log_warning "~ $relative_file (would be reformatted)"
        return 1
    fi
}

format_regular_file() {
    local file="$1"
    local relative_file
    relative_file=$(get_relative_path "$file")
    local temp_file
    temp_file=$(mktemp)

    if clang-format "$file" > "$temp_file"; then
        if ! cmp -s "$file" "$temp_file"; then
            mv "$temp_file" "$file"
            log_error "✓ $relative_file (formatted)"
            rm -f "$temp_file"
            return 1  # Changed
        else
            rm -f "$temp_file"
            log_success "✓ $relative_file (no changes needed)"
            return 0  # No change
        fi
    else
        rm -f "$temp_file"
        log_error "✗ $relative_file (formatting failed)"
        return 2  # Error
    fi
}

process_file() {
    local file="$1"

    # Determine processing mode
    local mode
    if [[ "$CHECK_ONLY" == "true" ]]; then
        mode="check"
    elif [[ "$DRY_RUN" == "true" ]]; then
        mode="dry-run"
    else
        mode="process"
    fi

    # Use common processing function
    process_file_with_mode "$file" "$mode" \
        "format_regular_file" \
        "check_file_formatting" \
        "show_dry_run"
    return 0
}

format_files() {
    local files=("$@")
    local total_files=${#files[@]}
    local changed_files=0
    local error_files=0

    if [[ $total_files -eq 0 ]]; then
        log_warning "No C files found to format"
        return 0
    fi

    log_info "Found $total_files C file(s) to process"
    echo ""

    for file in "${files[@]}"; do
        process_file "$file"
        case $? in
            1) ((changed_files++)) ;;
            2) ((error_files++)) ;;
        esac
    done

    # Determine mode for summary
    local mode
    if [[ "$CHECK_ONLY" == "true" ]]; then
        mode="check"
    elif [[ "$DRY_RUN" == "true" ]]; then
        mode="dry-run"
    else
        mode="process"
    fi

    show_summary "$total_files" "$changed_files" "$error_files" "$mode" "formatting"

    # Return appropriate exit code
    if [[ $error_files -gt 0 ]]; then
        return 2  # Errors occurred
    elif [[ "$CHECK_ONLY" == "true" && $changed_files -gt 0 ]]; then
        return 1  # Files need formatting (for check mode)
    else
        return 0  # Success
    fi
}

# =============================================================================
# Argument Parsing
# =============================================================================

parse_arguments() {
    while [[ $# -gt 0 ]]; do
        local arg="$1"
        case "$arg" in
            -h|--help)
                print_usage
                exit 0
                ;;
            *)
                # Try common argument parsing first
                if ! parse_common_arguments "$@"; then
                    log_error "Unknown option: $arg"
                    print_usage
                    exit 1
                fi
                # parse_common_arguments handles the shifting
                break
                ;;
        esac
    done
    return 0
}

# =============================================================================
# Main Execution
# =============================================================================

main() {
    # Check prerequisites
    check_clang_format

    # Find and format files
    log_info "Scanning for C source files..."
    local files
    mapfile -t files < <(find_c_files)

    # Format the files
    format_files "${files[@]}"
    local exit_code=$?

    # Exit with appropriate code
    exit $exit_code
}

# Execute main function
parse_arguments "$@"
main
