#!/bin/bash
# Code linting script for ONVIF project
# Uses clangd-tidy to perform comprehensive code analysis and linting

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
SINGLE_FILE=""
CHECK_ONLY=false
FORMAT_CHECK=false
SEVERITY_LEVEL="error"
CHANGED_FILES=false
PR_DIFF=false

# =============================================================================
# Script-Specific Functions
# =============================================================================

print_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Lint global C source files in the ONVIF project using clangd-tidy
(Excludes test files - use tests/scripts/lint_test_code.sh for test files)

OPTIONS:
  -h, --help              Show this help message
  -d, --dry-run           Show what would be linted without making changes
  -c, --check             Check if files have linting issues (exit 1 if issues found)
  -f, --files FILE        Lint specific files (comma-separated)
  --file FILE             Lint a single file
  --changed               Lint only files modified/created since last git commit
  --pr-diff               Lint only files modified in PR (uses GITHUB_BASE_REF env var)
  --format                Also check code formatting with clang-format
  --severity LEVEL        Fail on severity level: error, warn, info, hint (default: hint)

EXAMPLES:
  $0                      # Lint all global C files in the project
  $0 --dry-run            # Show what would be linted
  $0 --check              # Check if files have linting issues
  $0 --file src/core/main.c                    # Lint a single file
  $0 --files src/core/main.c,src/services/device/onvif_device.c
  $0 --changed            # Lint only changed files since last commit
  $0 --pr-diff            # Lint only files modified in PR
  $0 --format             # Also check code formatting
  $0 --severity error     # Only fail on errors
EOF
}

check_clangd_tidy() {
    # Check for clangd-tidy
    if ! command_exists clangd-tidy; then
        log_error "Error: clangd-tidy is not installed"
        cat << EOF
Please install clangd-tidy:
  pip install clangd-tidy

  Or from source:
  https://github.com/lljbash/clangd-tidy
EOF
        exit 1
    fi

    # Check for clangd (required by clangd-tidy)
    if ! command_exists clangd; then
        show_installation_instructions "clangd" "clangd" "clang-tools-extra" "https://clangd.llvm.org/installation"
        exit 1
    fi

    local clangd_tidy_version
    clangd_tidy_version=$(get_command_version clangd-tidy '[0-9]\+\.[0-9]\+\.[0-9]\+')
    local clangd_version
    clangd_version=$(get_command_version clangd '[0-9]\+\.[0-9]\+\.[0-9]\+')
    log_info "Using clangd-tidy version: $clangd_tidy_version with clangd version: $clangd_version"
}

check_compile_commands() {
    if [[ ! -f "$PROJECT_ROOT/compile_commands.json" ]]; then
        log_warning "Warning: compile_commands.json not found"
        log_info "Generating compile_commands.json..."
        if [[ -f "$PROJECT_ROOT/scripts/generate_compile_commands.sh" ]]; then
            if [[ -x "$PROJECT_ROOT/scripts/generate_compile_commands.sh" ]]; then
                log_info "Running generate_compile_commands.sh..."
                "$PROJECT_ROOT/scripts/generate_compile_commands.sh"
            else
                log_error "Error: generate_compile_commands.sh is not executable"
                log_info "Fixing permissions..."
                chmod +x "$PROJECT_ROOT/scripts/generate_compile_commands.sh"
                "$PROJECT_ROOT/scripts/generate_compile_commands.sh"
            fi
        else
            log_error "Error: generate_compile_commands.sh not found"
            log_info "Please run 'make compile-commands' to generate compile_commands.json"
            return 1
        fi
    fi
    return 0
}

build_clangd_tidy_command() {
    local files=("$@")
    local cmd="clangd-tidy"

    # Add compile commands directory
    cmd="$cmd -p $PROJECT_ROOT"

    # Add severity level
    cmd="$cmd --fail-on-severity $SEVERITY_LEVEL"

    # Add format check if requested
    if [[ "$FORMAT_CHECK" == "true" ]]; then
        cmd="$cmd --format"
    fi

    # Always use verbose output for better user experience
    cmd="$cmd --verbose"

    # Add color output
    cmd="$cmd --color always"

    # Add all files
    for file in "${files[@]}"; do
        cmd="$cmd \"$file\""
    done

    echo "$cmd"
}

run_clangd_tidy() {
    local files=("$@")
    local temp_file
    temp_file=$(mktemp)
    local exit_code=0

    local cmd
    cmd=$(build_clangd_tidy_command "${files[@]}")

    # Run clangd-tidy and capture output
    if eval "$cmd" > "$temp_file" 2>&1; then
        # No issues found
        log_success "✓ All files pass linting checks"
        rm -f "$temp_file"
        return 0
    else
        # Issues found
        exit_code=$?
        if [[ -s "$temp_file" ]]; then
            cat "$temp_file"
        fi
        rm -f "$temp_file"
        return $exit_code
    fi
}

lint_files() {
    local files=("$@")
    local total_files=${#files[@]}

    if [[ $total_files -eq 0 ]]; then
        log_warning "No C files found to lint"
        return 0
    fi

    log_info "Found $total_files C file(s) to process"
    echo ""

    # Filter out generated files
    local filtered_files=()
    for file in "${files[@]}"; do
        if ! is_generated_file "$file"; then
            filtered_files+=("$file")
        else
            local relative_file
            relative_file=$(get_relative_path "$file")
            log_warning "~ $relative_file (skipped - generated file)"
        fi
    done

    if [[ ${#filtered_files[@]} -eq 0 ]]; then
        log_info "No files to lint after filtering generated files"
        return 0
    fi

    # Run clangd-tidy on all files at once (much faster)
    run_clangd_tidy "${filtered_files[@]}"
    local exit_code=$?

    # Return the clangd-tidy exit code
    return $exit_code
}

find_changed_files() {
    # Check if we're in a git repository
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        log_error "Error: Not in a git repository"
        return 1
    fi

    local changed_files=()
    local git_root
    git_root=$(git rev-parse --show-toplevel)

    # Get modified, staged, and untracked files (only global source files)
    while IFS= read -r file; do
        # Only include C/C++ source files in src directory (exclude tests)
        # Handle both git root relative paths and working directory relative paths
        if [[ "$file" =~ ^(cross-compile/onvif/)?src/.*\.(c|h|cpp|hpp|cc|hh)$ ]]; then
            # Convert to absolute path if relative
            if [[ "$file" != /* ]]; then
                # If it starts with cross-compile/onvif/, use git root
                if [[ "$file" =~ ^cross-compile/onvif/ ]]; then
                    file="$git_root/$file"
                else
                    # Otherwise, it's relative to current working directory
                    file="$PROJECT_ROOT/$file"
                fi
            fi
            # Check if file exists after converting to absolute path
            if [[ -f "$file" ]]; then
                changed_files+=("$file")
            fi
        fi
    done < <(git diff --name-only HEAD && git diff --cached --name-only && git ls-files --others --exclude-standard)

    # Output the files (one per line)
    printf '%s\n' "${changed_files[@]}"
}

find_pr_diff_files() {
    # Check if we're in a git repository
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        log_error "Error: Not in a git repository"
        log_info "Current working directory: $(pwd)"
        log_info "Directory contents:"
        ls -la . || true
        log_info "Parent directory contents:"
        ls -la .. || true
        log_info "Checking for .git directory:"
        find . -name ".git" -type d 2>/dev/null || log_info "No .git directory found"
        return 1
    fi

    # Check for GITHUB_BASE_REF environment variable
    if [[ -z "${GITHUB_BASE_REF:-}" ]]; then
        log_error "Error: GITHUB_BASE_REF environment variable not set"
        log_info "This flag is intended for use in GitHub Actions PR workflows"
        return 1
    fi

    local pr_files=()
    local git_root
    git_root=$(git rev-parse --show-toplevel)

    # Debug: Log environment and git information
    log_info "=== PR DIFF DEBUG INFO ==="
    log_info "GITHUB_BASE_REF: ${GITHUB_BASE_REF:-'not set'}"
    log_info "GITHUB_HEAD_REF: ${GITHUB_HEAD_REF:-'not set'}"
    log_info "GITHUB_REF: ${GITHUB_REF:-'not set'}"
    log_info "Git root: $git_root"
    log_info "Project root: $PROJECT_ROOT"
    log_info "Current working directory: $(pwd)"
    log_info "Git branch: $(git branch --show-current 2>/dev/null || echo 'detached HEAD')"
    log_info "Git HEAD: $(git rev-parse HEAD 2>/dev/null || echo 'unknown')"

    # Fetch the base branch to ensure we have it locally
    log_info "Fetching base branch: origin/$GITHUB_BASE_REF"
    if ! git fetch origin "$GITHUB_BASE_REF"; then
        log_error "Error: Failed to fetch base branch origin/$GITHUB_BASE_REF"
        return 1
    fi

    # Debug: Show all files changed in PR
    log_info "All files changed in PR (origin/$GITHUB_BASE_REF...HEAD):"
    local all_changed_files
    all_changed_files=$(git diff --name-only "origin/$GITHUB_BASE_REF...HEAD" 2>/dev/null || echo "")
    if [[ -n "$all_changed_files" ]]; then
        echo "$all_changed_files" | while IFS= read -r file; do
            log_info "  - $file"
        done
    else
        log_warning "  No files found in git diff"
    fi

    # Get files changed in PR (base branch to HEAD)
    local total_files=0
    local c_files=0
    local filtered_files=0

    # Debug: Show the regex pattern being used
    log_info "Using regex pattern for C/C++ files: ^(cross-compile/onvif/)?src/.*\.(c|h|cpp|hpp|cc|hh)$"

    while IFS= read -r file; do
        total_files=$((total_files + 1))
        log_info "Processing file $total_files: $file"

        # Only include C/C++ source files in src directory (exclude tests)
        # Handle both git root relative paths and working directory relative paths
        if [[ "$file" =~ ^(cross-compile/onvif/)?src/.*\.(c|h|cpp|hpp|cc|hh)$ ]]; then
            c_files=$((c_files + 1))
            log_info "  ✓ Matches C/C++ pattern in src/ directory"

            # Convert to absolute path if relative
            local abs_file="$file"
            if [[ "$file" != /* ]]; then
                # If it starts with cross-compile/onvif/, use git root
                if [[ "$file" =~ ^cross-compile/onvif/ ]]; then
                    abs_file="$git_root/$file"
                    log_info "  → Converted to absolute path: $abs_file"
                else
                    # Otherwise, it's relative to current working directory
                    abs_file="$PROJECT_ROOT/$file"
                    log_info "  → Converted to absolute path: $abs_file"
                fi
            fi

            # Check if file exists after converting to absolute path
            if [[ -f "$abs_file" ]]; then
                filtered_files=$((filtered_files + 1))
                pr_files+=("$abs_file")
                log_info "  ✓ File exists, added to lint list"
            else
                log_warning "  ✗ File does not exist: $abs_file"
            fi
        else
            log_info "  - Skipped (not a C/C++ file in src/ directory)"
        fi
    done < <(git diff --name-only "origin/$GITHUB_BASE_REF...HEAD")

    log_info "=== PR DIFF SUMMARY ==="
    log_info "Total files changed: $total_files"
    log_info "C/C++ files in src/: $c_files"
    log_info "Files added to lint list: $filtered_files"
    log_info "========================="

    # Output the files (one per line)
    printf '%s\n' "${pr_files[@]}"
}

# =============================================================================
# Argument Parsing
# =============================================================================

parse_arguments() {
    # First, handle common arguments
    parse_common_arguments "$@"

    # Then handle script-specific arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                print_usage
                exit 0
                ;;
            --file)
                SINGLE_FILE="$2"
                shift 2
                ;;
            --format)
                FORMAT_CHECK=true
                shift
                ;;
            --severity)
                SEVERITY_LEVEL="$2"
                shift 2
                ;;
            --changed)
                CHANGED_FILES=true
                shift
                ;;
            --pr-diff)
                PR_DIFF=true
                shift
                ;;
            -c|--check|-d|--dry-run|-f|--files)
                # These are already handled by parse_common_arguments
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                print_usage
                exit 1
                ;;
        esac
    done
}

# =============================================================================
# Main Execution
# =============================================================================

main() {
    # Check prerequisites
    check_clangd_tidy
    check_compile_commands

    # Determine which files to lint
    local files=()

    if [[ -n "$SINGLE_FILE" ]]; then
        # Lint single file
        if [[ ! -f "$SINGLE_FILE" ]]; then
            log_error "Error: File '$SINGLE_FILE' does not exist"
            exit 1
        fi

        # Convert to absolute path if relative
        if [[ "$SINGLE_FILE" != /* ]]; then
            SINGLE_FILE="$PROJECT_ROOT/$SINGLE_FILE"
        fi

        files=("$SINGLE_FILE")
        log_info "Linting single file: $(get_relative_path "$SINGLE_FILE")"
    elif [[ -n "$FILES_ONLY" ]]; then
        # Lint specific files (comma-separated)
        IFS=',' read -ra file_array <<< "$FILES_ONLY"
        for file in "${file_array[@]}"; do
            # Trim whitespace
            file=$(echo "$file" | xargs)
            if [[ ! -f "$file" ]]; then
                log_error "Error: File '$file' does not exist"
                exit 1
            fi
            # Convert to absolute path if relative
            if [[ "$file" != /* ]]; then
                file="$PROJECT_ROOT/$file"
            fi
            files+=("$file")
        done
        log_info "Linting specified files: ${#files[@]} file(s)"
    elif [[ "$CHANGED_FILES" == "true" ]]; then
        # Lint only changed files since last commit
        log_info "Finding C source files changed since last commit..."
        mapfile -t files < <(find_changed_files)
        if [[ ${#files[@]} -eq 0 ]]; then
            log_info "No C source files have been modified since last commit"
            exit 0
        fi
        log_info "Found ${#files[@]} changed C file(s) to lint"
    elif [[ "$PR_DIFF" == "true" ]]; then
        # Lint only files modified in PR
        log_info "Finding C source files modified in PR..."
        mapfile -t files < <(find_pr_diff_files)
        if [[ ${#files[@]} -eq 0 ]]; then
            log_info "No C source files have been modified in this PR"
            exit 0
        fi
        log_info "Found ${#files[@]} PR-modified C file(s) to lint"

        # Debug: Show the actual files that will be linted
        log_info "Files to be linted:"
        for file in "${files[@]}"; do
            local relative_file
            relative_file=$(get_relative_path "$file")
            log_info "  - $relative_file"
        done
    else
        # Find and lint all global C files (excluding test files)
        log_info "Scanning for global C source files..."
        mapfile -t files < <(find_global_c_files)
    fi

    # Lint the files
    lint_files "${files[@]}"
    local exit_code=$?

    # Exit with appropriate code
    exit $exit_code
}

# Execute main function
parse_arguments "$@"
main
