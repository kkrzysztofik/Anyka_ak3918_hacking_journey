#!/bin/bash
# extract_test_metrics.sh - Extract test and coverage metrics from test output
# Author: kkrzysztofik
# Date: 2025

set -e

# Default values
TEST_TYPE=""
TEST_OUTPUT=""
COVERAGE_INFO=""
OUTPUT_FILE=""

# Parse command line arguments
parse_args() {
  while [[ $# -gt 0 ]]; do
    case $1 in
      --type=*)
        TEST_TYPE="${1#*=}"
        shift
        ;;
      --type)
        TEST_TYPE="$2"
        shift 2
        ;;
      --test-output=*)
        TEST_OUTPUT="${1#*=}"
        shift
        ;;
      --test-output)
        TEST_OUTPUT="$2"
        shift 2
        ;;
      --coverage-info=*)
        COVERAGE_INFO="${1#*=}"
        shift
        ;;
      --coverage-info)
        COVERAGE_INFO="$2"
        shift 2
        ;;
      --output=*)
        OUTPUT_FILE="${1#*=}"
        shift
        ;;
      --output)
        OUTPUT_FILE="$2"
        shift 2
        ;;
      *)
        echo "Unknown option: $1"
        exit 1
        ;;
    esac
  done
}

# Validate required arguments
validate_args() {
  if [ -z "$TEST_TYPE" ]; then
    echo "Error: --type is required"
    exit 1
  fi

  if [ -z "$TEST_OUTPUT" ]; then
    echo "Error: --test-output is required"
    exit 1
  fi

  if [ -z "$COVERAGE_INFO" ]; then
    echo "Error: --coverage-info is required"
    exit 1
  fi

  if [ -z "$OUTPUT_FILE" ]; then
    echo "Error: --output is required"
    exit 1
  fi

  if [ ! -f "$TEST_OUTPUT" ]; then
    echo "Error: Test output file not found: $TEST_OUTPUT"
    exit 1
  fi

  if [ ! -f "$COVERAGE_INFO" ]; then
    echo "Error: Coverage info file not found: $COVERAGE_INFO"
    exit 1
  fi
}

# Parse test output to extract metrics
parse_test_output() {
  local output_file="$1"

  # Initialize variables
  local suites_run=0
  local tests_total=0
  local tests_passed=0
  local tests_failed=0
  local duration=0.0

  # Extract suites run count
  if line=$(grep "Suites run:" "$output_file" | tail -1); then
    suites_run=$(echo "$line" | awk '{print $3}')
  fi

  # Extract tests run count
  if line=$(grep "Tests run:" "$output_file" | tail -1); then
    tests_total=$(echo "$line" | awk '{print $3}')
  fi

  # Extract test duration
  if line=$(grep "Test duration:" "$output_file" | tail -1); then
    duration=$(echo "$line" | awk '{print $3}')
  fi

  # Extract pass/fail status
  if grep -q "✅ All.*test(s) passed" "$output_file"; then
    # All tests passed
    tests_passed=$tests_total
    tests_failed=0
  elif grep -q "❌.*test(s) failed" "$output_file"; then
    # Some tests failed - extract failure count
    if line=$(grep "❌.*test(s) failed" "$output_file" | tail -1); then
      tests_failed=$(echo "$line" | grep -o '[0-9]\+' | head -1)
      tests_passed=$((tests_total - tests_failed))
    fi
  else
    # Fallback: if we have suite failures, count them
    tests_failed=$(grep -oP "Suite \S+: \d+ passed, \K\d+" "$output_file" 2>/dev/null | awk '{s+=$1} END {print s}' || echo "0")
    tests_passed=$((tests_total - tests_failed))
  fi

  # Export variables
  export SUITES_RUN=$suites_run
  export TESTS_TOTAL=$tests_total
  export TESTS_PASSED=$tests_passed
  export TESTS_FAILED=$tests_failed
  export DURATION=$duration
}

# Parse coverage info to extract metrics
parse_coverage() {
  local coverage_info="$1"
  local lines_pct="0.0"
  local functions_pct="0.0"

  # Run lcov summary
  if command -v lcov &> /dev/null; then
    local summary=$(lcov --summary "$coverage_info" 2>&1 | grep -E "lines|functions" || true)

    # Extract lines coverage
    if line=$(echo "$summary" | grep "lines"); then
      lines_pct=$(echo "$line" | grep -oP '\d+\.\d+' | head -1)
    fi

    # Extract functions coverage
    if line=$(echo "$summary" | grep "functions"); then
      functions_pct=$(echo "$line" | grep -oP '\d+\.\d+' | head -1)
    fi
  fi

  # Export variables
  export LINES_PCT=$lines_pct
  export FUNCTIONS_PCT=$functions_pct
}

# Generate JSON output
generate_json() {
  cat > "$OUTPUT_FILE" <<EOF
{
  "test_type": "$TEST_TYPE",
  "tests_total": $TESTS_TOTAL,
  "tests_passed": $TESTS_PASSED,
  "tests_failed": $TESTS_FAILED,
  "suites_run": $SUITES_RUN,
  "duration_seconds": $DURATION,
  "coverage": {
    "lines": "$LINES_PCT",
    "functions": "$FUNCTIONS_PCT"
  }
}
EOF
}

# Main execution
main() {
  parse_args "$@"
  validate_args

  echo "Extracting test metrics from: $TEST_OUTPUT"
  echo "Coverage info from: $COVERAGE_INFO"
  echo "Test type: $TEST_TYPE"

  # Parse test output
  parse_test_output "$TEST_OUTPUT"

  # Parse coverage
  parse_coverage "$COVERAGE_INFO"

  # Generate JSON
  mkdir -p "$(dirname "$OUTPUT_FILE")"
  generate_json

  echo "Metrics extracted to: $OUTPUT_FILE"
  echo "Tests: $TESTS_TOTAL total, $TESTS_PASSED passed, $TESTS_FAILED failed"
  echo "Coverage: Lines=${LINES_PCT}%, Functions=${FUNCTIONS_PCT}%"
}

# Run main function
main "$@"

