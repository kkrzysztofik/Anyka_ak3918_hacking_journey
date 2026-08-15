#!/bin/sh
#
# fix_line_endings.sh - Convert Windows line endings (CRLF) to Unix line endings (LF)
#
# Description:
#   Fixes line ending issues in shell scripts that cause syntax errors on Unix systems.
#   Converts Windows-style CRLF (\r\n) to Unix-style LF (\n) line endings using in-place editing.
#
# Usage:
#   ./fix_line_endings.sh [OPTIONS] [SCRIPT_FILES...]
#   ./fix_line_endings.sh --all                    # Fix all .sh files in current directory
#   ./fix_line_endings.sh --recursive              # Fix all .sh files recursively
#   ./fix_line_endings.sh script1.sh script2.sh    # Fix specific scripts
#
# Options:
#   --all, -a          Fix all .sh files in current directory
#   --recursive, -r    Fix all .sh files recursively
#   --backup, -b       Create backup files (.bak)
#   --verbose, -v      Enable verbose output
#   --help, -h         Show this help message
#
# Author: Anyka Hack Project
# Version: 1.0
# Last Modified: $(date '+%Y-%m-%d')

# =============================================================================
# GLOBAL VARIABLES AND CONFIGURATION
# =============================================================================

SCRIPT_NAME="$(basename "$0")"
VERBOSE_MODE=0
CREATE_BACKUP=0
FIX_ALL=0
FIX_RECURSIVE=0
TOTAL_FILES=0
FIXED_FILES=0
SKIPPED_FILES=0

# =============================================================================
# UTILITY FUNCTIONS
# =============================================================================

print_verbose() {
  if [ "$VERBOSE_MODE" = "1" ]; then
    printf "VERBOSE: %s\n" "$*"
  fi
}

print_info() {
  printf "INFO: %s\n" "$*"
}

print_success() {
  printf "SUCCESS: %s\n" "$*"
}

print_warning() {
  printf "WARNING: %s\n" "$*"
}

print_error() {
  printf "ERROR: %s\n" "$*"
}

# Show usage information
show_usage() {
  cat << EOF
Usage: $SCRIPT_NAME [OPTIONS] [SCRIPT_FILES...]

Convert Windows line endings (CRLF) to Unix line endings (LF) in shell scripts.

OPTIONS:
  -a, --all           Fix all .sh files in current directory
  -r, --recursive     Fix all .sh files recursively
  -b, --backup        Create backup files (.bak)
  -v, --verbose       Enable verbose output
  -h, --help          Show this help message

EXAMPLES:
  $SCRIPT_NAME script1.sh script2.sh
  $SCRIPT_NAME --all --backup
  $SCRIPT_NAME --recursive --verbose

EOF
}

# Check if file has Windows line endings
has_crlf() {
  local file="$1"
  if [ -f "$file" ] && [ -r "$file" ]; then
    # Check if file contains carriage return characters
    if od -c "$file" 2>/dev/null | grep -q '\r'; then
      return 0  # Has CRLF
    else
      return 1  # No CRLF
    fi
  else
    return 1  # File not readable
  fi
}

# Fix line endings in a single file
fix_file() {
  local file="$1"
  local backup_file="${file}.bak"

  if [ ! -f "$file" ]; then
    print_error "File not found: $file"
    return 1
  fi

  if [ ! -r "$file" ]; then
    print_error "File not readable: $file"
    return 1
  fi

  if [ ! -w "$file" ]; then
    print_error "File not writable: $file"
    return 1
  fi

  # Check if file has CRLF line endings
  if ! has_crlf "$file"; then
    print_verbose "No CRLF line endings found in: $file"
    SKIPPED_FILES=$((SKIPPED_FILES + 1))
    return 0
  fi

  print_verbose "Fixing line endings in: $file"

  # Create backup if requested
  if [ "$CREATE_BACKUP" = "1" ]; then
    if cp "$file" "$backup_file" 2>/dev/null; then
      print_verbose "Created backup: $backup_file"
    else
      print_warning "Could not create backup for: $file"
    fi
  fi

  # Convert CRLF to LF using sed in-place editing (no temporary files needed)
  if sed -i 's/\r$//' "$file" 2>/dev/null; then
    print_success "Fixed line endings in: $file"
    FIXED_FILES=$((FIXED_FILES + 1))
    return 0
  else
    print_error "Failed to convert line endings in: $file"
    return 1
  fi
}

# Find all shell scripts
find_scripts() {
  local pattern="$1"
  local scripts=""

  if [ "$pattern" = "recursive" ]; then
    scripts=$(find . -name "*.sh" -type f 2>/dev/null)
  else
    scripts=$(ls *.sh 2>/dev/null)
  fi

  if [ -z "$scripts" ]; then
    print_error "No shell scripts found"
    return 1
  fi

  echo "$scripts"
}

# =============================================================================
# MAIN EXECUTION
# =============================================================================

# Parse command line arguments
while [ $# -gt 0 ]; do
  case "$1" in
    -a|--all)
      FIX_ALL=1
      shift
      ;;
    -r|--recursive)
      FIX_RECURSIVE=1
      shift
      ;;
    -b|--backup)
      CREATE_BACKUP=1
      shift
      ;;
    -v|--verbose)
      VERBOSE_MODE=1
      shift
      ;;
    -h|--help)
      show_usage
      exit 0
      ;;
    -*)
      print_error "Unknown option: $1"
      show_usage
      exit 1
      ;;
    *)
      # Script file argument
      break
      ;;
  esac
done

# Determine which scripts to fix
if [ $FIX_ALL -eq 1 ]; then
  SCRIPT_FILES=$(find_scripts "current")
elif [ $FIX_RECURSIVE -eq 1 ]; then
  SCRIPT_FILES=$(find_scripts "recursive")
else
  SCRIPT_FILES="$@"
fi

# Check if we have scripts to fix
if [ -z "$SCRIPT_FILES" ]; then
  print_error "No scripts specified for fixing"
  show_usage
  exit 1
fi

# Count total files
TOTAL_FILES=$(echo "$SCRIPT_FILES" | wc -w)

print_info "Starting line ending fix for $TOTAL_FILES script(s)"
print_info "Backup mode: $([ $CREATE_BACKUP -eq 1 ] && echo "enabled" || echo "disabled")"
print_info "Verbose mode: $([ $VERBOSE_MODE -eq 1 ] && echo "enabled" || echo "disabled")"
echo

# Fix each script
for script in $SCRIPT_FILES; do
  fix_file "$script"
done

# Print summary
echo "=========================================="
print_info "Line Ending Fix Summary:"
echo "  Total files: $TOTAL_FILES"
print_success "  Fixed: $FIXED_FILES"
print_warning "  Skipped: $SKIPPED_FILES"
echo "=========================================="

# Exit with appropriate code
if [ $FIXED_FILES -gt 0 ]; then
  exit 0
else
  exit 1
fi
