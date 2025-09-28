#!/bin/sh
#
# validate_scripts.sh - Shell script validation utility for busybox sh compatibility
#
# Description:
#   Validates shell scripts for POSIX compliance and busybox sh compatibility.
#   Checks for common issues that can cause scripts to fail on embedded systems
#   with limited shell implementations.
#
# Usage:
#   ./validate_scripts.sh [OPTIONS] [SCRIPT_FILES...]
#   ./validate_scripts.sh --all                    # Validate all .sh files in current directory
#   ./validate_scripts.sh --recursive              # Validate all .sh files recursively
#   ./validate_scripts.sh script1.sh script2.sh    # Validate specific scripts
#
# Options:
#   --all, -a          Validate all .sh files in current directory
#   --recursive, -r    Validate all .sh files recursively
#   --verbose, -v      Enable verbose output
#   --fix, -f          Show suggested fixes for issues
#   --help, -h         Show this help message
#
# Validation Checks:
#   - Built-in busybox sh syntax checking (sh -n)
#   - Shebang line compatibility (#!/bin/sh vs #!/bin/bash)
#   - POSIX shell syntax compliance
#   - Busybox-specific compatibility issues
#   - Common bash-isms that don't work in busybox sh
#   - File operation safety
#   - Process management compatibility
#   - Error handling patterns
#
# Dependencies:
#   - busybox sh or POSIX-compliant shell
#   - awk, grep, sed, head, tail utilities
#   - /proc filesystem for system checks
#
# Author: Anyka Hack Project
# Version: 1.0
# Last Modified: $(date '+%Y-%m-%d')

# =============================================================================
# GLOBAL VARIABLES AND CONFIGURATION
# =============================================================================

SCRIPT_NAME="$(basename "$0")"
VERBOSE_MODE=0
SHOW_FIXES=0
VALIDATE_ALL=0
VALIDATE_RECURSIVE=0
TOTAL_SCRIPTS=0
PASSED_SCRIPTS=0
FAILED_SCRIPTS=0
WARNING_SCRIPTS=0

# Color codes for output (if terminal supports it)
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# =============================================================================
# UTILITY FUNCTIONS
# =============================================================================

# Check if terminal supports colors
supports_color() {
  [ -t 1 ] && [ "${TERM:-}" != "dumb" ] && [ -n "${TERM:-}" ]
}

# Colored output functions
print_error() {
  if supports_color; then
    printf "${RED}ERROR:${NC} %s\n" "$*"
  else
    printf "ERROR: %s\n" "$*"
  fi
}

print_warning() {
  if supports_color; then
    printf "${YELLOW}WARNING:${NC} %s\n" "$*"
  else
    printf "WARNING: %s\n" "$*"
  fi
}

print_success() {
  if supports_color; then
    printf "${GREEN}SUCCESS:${NC} %s\n" "$*"
  else
    printf "SUCCESS: %s\n" "$*"
  fi
}

print_info() {
  if supports_color; then
    printf "${BLUE}INFO:${NC} %s\n" "$*"
  else
    printf "INFO: %s\n" "$*"
  fi
}

print_verbose() {
  if [ "$VERBOSE_MODE" = "1" ]; then
    printf "VERBOSE: %s\n" "$*"
  fi
}

# Show usage information
show_usage() {
  cat << EOF
Usage: $SCRIPT_NAME [OPTIONS] [SCRIPT_FILES...]

Validate shell scripts for busybox sh compatibility and POSIX compliance.

OPTIONS:
  -a, --all           Validate all .sh files in current directory
  -r, --recursive     Validate all .sh files recursively
  -v, --verbose       Enable verbose output
  -f, --fix           Show suggested fixes for issues
  -h, --help          Show this help message

EXAMPLES:
  $SCRIPT_NAME script1.sh script2.sh
  $SCRIPT_NAME --all
  $SCRIPT_NAME --recursive --verbose
  $SCRIPT_NAME --fix script.sh

VALIDATION CHECKS:
  - Shebang line compatibility
  - POSIX shell syntax compliance
  - Busybox-specific compatibility
  - Common bash-isms detection
  - File operation safety
  - Process management compatibility
  - Error handling patterns

EOF
}

# =============================================================================
# VALIDATION FUNCTIONS
# =============================================================================

# Check shebang line
check_shebang() {
  local file="$1"
  local line1=$(head -n1 "$file" 2>/dev/null)

  case "$line1" in
    "#!/bin/sh"|"#!/bin/sh"*)
      print_verbose "✓ Correct shebang: $line1"
      return 0
      ;;
    "#!/bin/bash"|"#!/bin/bash"*)
      print_error "Incorrect shebang: $line1 (should be #!/bin/sh for busybox compatibility)"
      [ "$SHOW_FIXES" = "1" ] && printf "  Fix: Change to #!/bin/sh\n"
      return 1
      ;;
    "#!/usr/bin/env sh"|"#!/usr/bin/env sh"*)
      print_verbose "✓ Correct shebang: $line1"
      return 0
      ;;
    "#!/usr/bin/env bash"|"#!/usr/bin/env bash"*)
      print_error "Incorrect shebang: $line1 (should be #!/bin/sh for busybox compatibility)"
      [ "$SHOW_FIXES" = "1" ] && printf "  Fix: Change to #!/bin/sh\n"
      return 1
      ;;
    "")
      print_error "Empty file or no shebang line"
      return 1
      ;;
    *)
      print_warning "Unknown shebang: $line1 (recommend #!/bin/sh)"
      return 1
      ;;
  esac
}

# Check syntax using busybox sh -n
check_syntax() {
  local file="$1"
  local syntax_errors=""

  # Use busybox sh -n for syntax checking
  if command -v busybox >/dev/null 2>&1; then
    syntax_errors=$(busybox sh -n "$file" 2>&1)
  else
    # Fallback to system sh if busybox not available
    syntax_errors=$(sh -n "$file" 2>&1)
  fi

  if [ $? -eq 0 ]; then
    print_verbose "✓ Syntax check passed"
    return 0
  else
    print_error "Syntax errors found:"
    echo "$syntax_errors" | while read -r line; do
      printf "  %s\n" "$line"
    done
    [ "$SHOW_FIXES" = "1" ] && printf "  Fix: Review and correct syntax errors listed above\n"
    return 1
  fi
}

# Check for bash-specific features
check_bash_isms() {
  local file="$1"
  local issues=0

  # Skip validation patterns in the validation script itself
  local is_validation_script=0
  if [ "$file" = "scripts/validate_scripts.sh" ] || [ "$file" = "./scripts/validate_scripts.sh" ]; then
    is_validation_script=1
  fi

  # Check for indirect variable expansion (skip if it's in validation patterns)
  if [ "$is_validation_script" = "0" ] && grep -q '\${![^}]*}' "$file" 2>/dev/null; then
    print_error "Bash-specific indirect variable expansion found: \${!var}"
    [ "$SHOW_FIXES" = "1" ] && printf "  Fix: Use eval \"value=\\$$var\" instead\n"
    issues=$((issues + 1))
  fi

  # Check for array syntax
  if grep -q '\[[0-9]\]' "$file" 2>/dev/null; then
    print_warning "Array syntax detected (may not work in all busybox versions)"
    [ "$SHOW_FIXES" = "1" ] && printf "  Fix: Use separate variables or eval\n"
    issues=$((issues + 1))
  fi

  # Check for process substitution (skip if it's in validation patterns)
  if [ "$is_validation_script" = "0" ] && (grep -q '<(' "$file" 2>/dev/null || grep -q '>(' "$file" 2>/dev/null); then
    print_error "Process substitution found (bash-specific)"
    [ "$SHOW_FIXES" = "1" ] && printf "  Fix: Use pipes or temporary files\n"
    issues=$((issues + 1))
  fi

  # Check for here strings (skip if it's in validation patterns)
  if [ "$is_validation_script" = "0" ] && grep -q '<<<' "$file" 2>/dev/null; then
    print_error "Here string found (bash-specific)"
    [ "$SHOW_FIXES" = "1" ] && printf "  Fix: Use echo \"text\" | command\n"
    issues=$((issues + 1))
  fi

  # Check for brace expansion
  if grep -q '{[^}]*,[^}]*}' "$file" 2>/dev/null; then
    print_warning "Brace expansion detected (may not work in all busybox versions)"
    [ "$SHOW_FIXES" = "1" ] && printf "  Fix: Use explicit loops or case statements\n"
    issues=$((issues + 1))
  fi

  return $issues
}

# Check for unsafe file operations
check_file_operations() {
  local file="$1"
  local issues=0

  # Check for unsafe redirections
  if grep -q '>>' "$file" 2>/dev/null && ! grep -q '2>/dev/null' "$file" 2>/dev/null; then
    print_warning "File redirection without error suppression"
    [ "$SHOW_FIXES" = "1" ] && printf "  Fix: Add 2>/dev/null for error suppression\n"
    issues=$((issues + 1))
  fi

  # Check for missing error handling in critical operations
  if grep -q 'rm ' "$file" 2>/dev/null && ! grep -q 'rm.*2>/dev/null' "$file" 2>/dev/null; then
    print_warning "rm command without error suppression"
    [ "$SHOW_FIXES" = "1" ] && printf "  Fix: Add 2>/dev/null to rm commands\n"
    issues=$((issues + 1))
  fi

  # Check for unsafe variable assignments (skip if it's in validation patterns)
  if [ "$is_validation_script" = "0" ] && grep -q '`[^`]*`' "$file" 2>/dev/null; then
    print_warning "Backtick command substitution (use \$() for better compatibility)"
    [ "$SHOW_FIXES" = "1" ] && printf "  Fix: Use \$(command) instead of \`command\`\n"
    issues=$((issues + 1))
  fi

  return $issues
}

# Check for process management issues
check_process_management() {
  local file="$1"
  local issues=0

  # Skip validation patterns in the validation script itself
  local is_validation_script=0
  if [ "$file" = "scripts/validate_scripts.sh" ] || [ "$file" = "./scripts/validate_scripts.sh" ]; then
    is_validation_script=1
  fi

  # Check for background processes in if statements (skip if it's in validation patterns)
  if [ "$is_validation_script" = "0" ] && grep -q 'if.*&;' "$file" 2>/dev/null; then
    print_error "Background process in if statement (unreliable)"
    [ "$SHOW_FIXES" = "1" ] && printf "  Fix: Separate background execution from if condition\n"
    issues=$((issues + 1))
  fi

  # Check for missing PID file management (only for actual background processes, not printf format strings)
  if grep -q ' [^"]*&$' "$file" 2>/dev/null && ! grep -qi 'pid.*file' "$file" 2>/dev/null; then
    print_warning "Background process without PID file management"
    [ "$SHOW_FIXES" = "1" ] && printf "  Fix: Add PID file management for background processes\n"
    issues=$((issues + 1))
  fi

  # Check for unsafe kill commands
  if grep -q 'kill ' "$file" 2>/dev/null && ! grep -q 'kill.*2>/dev/null' "$file" 2>/dev/null; then
    print_warning "kill command without error suppression"
    [ "$SHOW_FIXES" = "1" ] && printf "  Fix: Add 2>/dev/null to kill commands\n"
    issues=$((issues + 1))
  fi

  return $issues
}

# Check for arithmetic issues
check_arithmetic() {
  local file="$1"
  local issues=0

  # Check for arithmetic in test conditions
  if grep -q '\[.*\$((.*)).*\]' "$file" 2>/dev/null; then
    print_warning "Arithmetic in test condition (may not work in all busybox versions)"
    [ "$SHOW_FIXES" = "1" ] && printf "  Fix: Calculate arithmetic separately before test\n"
    issues=$((issues + 1))
  fi

  # Check for complex arithmetic expressions
  if grep -q '\$((.*[+\-*/].*[+\-*/].*))' "$file" 2>/dev/null; then
    print_warning "Complex arithmetic expression (may not work in all busybox versions)"
    [ "$SHOW_FIXES" = "1" ] && printf "  Fix: Break into simpler expressions\n"
    issues=$((issues + 1))
  fi

  return $issues
}

# Check for complex pipelines
check_pipelines() {
  local file="$1"
  local issues=0

  # Check for very long pipelines (more than 4 commands)
  if grep -q '|.*|.*|.*|.*|' "$file" 2>/dev/null; then
    print_warning "Very long pipeline detected (may fail on limited busybox implementations)"
    [ "$SHOW_FIXES" = "1" ] && printf "  Fix: Break into smaller pipelines or use temporary files\n"
    issues=$((issues + 1))
  fi

  # Check for complex awk scripts
  if grep -q 'awk.*{.*for.*}' "$file" 2>/dev/null; then
    print_warning "Complex awk script detected (may not work in all busybox versions)"
    [ "$SHOW_FIXES" = "1" ] && printf "  Fix: Simplify awk script or use multiple simpler commands\n"
    issues=$((issues + 1))
  fi

  return $issues
}

# Check for error handling patterns
check_error_handling() {
  local file="$1"
  local issues=0

  # Check for missing error handling in critical operations
  if grep -q 'cp ' "$file" 2>/dev/null && ! grep -q 'cp.*2>/dev/null' "$file" 2>/dev/null; then
    print_warning "cp command without error suppression"
    [ "$SHOW_FIXES" = "1" ] && printf "  Fix: Add 2>/dev/null to cp commands\n"
    issues=$((issues + 1))
  fi

  # Check for missing error handling in mkdir
  if grep -q 'mkdir ' "$file" 2>/dev/null && ! grep -q 'mkdir.*2>/dev/null' "$file" 2>/dev/null; then
    print_warning "mkdir command without error suppression"
    [ "$SHOW_FIXES" = "1" ] && printf "  Fix: Add 2>/dev/null to mkdir commands\n"
    issues=$((issues + 1))
  fi

  # Check for missing error handling in mv
  if grep -q 'mv ' "$file" 2>/dev/null && ! grep -q 'mv.*2>/dev/null' "$file" 2>/dev/null; then
    print_warning "mv command without error suppression"
    [ "$SHOW_FIXES" = "1" ] && printf "  Fix: Add 2>/dev/null to mv commands\n"
    issues=$((issues + 1))
  fi

  return $issues
}

# Main validation function
validate_script() {
  local file="$1"
  local total_issues=0
  local critical_issues=0
  local warnings=0

  print_info "Validating: $file"

  # Check if file exists and is readable
  if [ ! -f "$file" ]; then
    print_error "File not found: $file"
    return 1
  fi

  if [ ! -r "$file" ]; then
    print_error "File not readable: $file"
    return 1
  fi

  # Check if file is empty
  if [ ! -s "$file" ]; then
    print_error "File is empty: $file"
    return 1
  fi

  # Run all validation checks
  check_shebang "$file" || critical_issues=$((critical_issues + 1))
  check_syntax "$file" || critical_issues=$((critical_issues + 1))
  check_bash_isms "$file" || critical_issues=$((critical_issues + 1))
  check_file_operations "$file" || warnings=$((warnings + 1))
  check_process_management "$file" || critical_issues=$((critical_issues + 1))
  check_arithmetic "$file" || warnings=$((warnings + 1))
  check_pipelines "$file" || warnings=$((warnings + 1))
  check_error_handling "$file" || warnings=$((warnings + 1))

  total_issues=$((critical_issues + warnings))

  # Report results
  if [ $critical_issues -eq 0 ] && [ $warnings -eq 0 ]; then
    print_success "✓ $file - No issues found"
    return 0
  elif [ $critical_issues -eq 0 ]; then
    print_warning "⚠ $file - $warnings warnings, no critical issues"
    return 1
  else
    print_error "✗ $file - $critical_issues critical issues, $warnings warnings"
    return 2
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
      VALIDATE_ALL=1
      shift
      ;;
    -r|--recursive)
      VALIDATE_RECURSIVE=1
      shift
      ;;
    -v|--verbose)
      VERBOSE_MODE=1
      shift
      ;;
    -f|--fix)
      SHOW_FIXES=1
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

# Determine which scripts to validate
if [ $VALIDATE_ALL -eq 1 ]; then
  SCRIPT_FILES=$(find_scripts "current")
elif [ $VALIDATE_RECURSIVE -eq 1 ]; then
  SCRIPT_FILES=$(find_scripts "recursive")
else
  SCRIPT_FILES="$@"
fi

# Check if we have scripts to validate
if [ -z "$SCRIPT_FILES" ]; then
  print_error "No scripts specified for validation"
  show_usage
  exit 1
fi

# Count total scripts
TOTAL_SCRIPTS=$(echo "$SCRIPT_FILES" | wc -w)

print_info "Starting validation of $TOTAL_SCRIPTS script(s)"
print_info "Validation mode: $([ $VERBOSE_MODE -eq 1 ] && echo "verbose" || echo "normal")"
print_info "Fix suggestions: $([ $SHOW_FIXES -eq 1 ] && echo "enabled" || echo "disabled")"
echo

# Validate each script
for script in $SCRIPT_FILES; do
  validate_script "$script"
  case $? in
    0) PASSED_SCRIPTS=$((PASSED_SCRIPTS + 1)) ;;
    1) WARNING_SCRIPTS=$((WARNING_SCRIPTS + 1)) ;;
    2) FAILED_SCRIPTS=$((FAILED_SCRIPTS + 1)) ;;
  esac
  echo
done

# Print summary
echo "=========================================="
print_info "Validation Summary:"
echo "  Total scripts: $TOTAL_SCRIPTS"
print_success "  Passed: $PASSED_SCRIPTS"
print_warning "  Warnings: $WARNING_SCRIPTS"
print_error "  Failed: $FAILED_SCRIPTS"
echo "=========================================="

# Exit with appropriate code
if [ $FAILED_SCRIPTS -gt 0 ]; then
  exit 2
elif [ $WARNING_SCRIPTS -gt 0 ]; then
  exit 1
else
  exit 0
fi
