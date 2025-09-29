#!/bin/bash
# run_all_tests.sh - Meta-runner script for executing all ONVIF test suites
# Author: kkrzysztofik
# Date: 2025

set -e

# ANSI color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Test suite definitions
declare -a TEST_SUITES=(
    "out/test_utils:Utility Tests"
    "out/test_networking:Networking Tests"
    "out/test_protocol:Protocol Tests"
    "out/test_service_dispatcher:Service Dispatcher Tests"
    "out/test_ptz:PTZ Service Tests"
    "out/test_media:Media Service Tests"
    "out/test_imaging:Imaging Service Tests"
    "out/test_integration:Integration Tests"
)

# Test results
declare -i total_suites=0
declare -i passed_suites=0
declare -i failed_suites=0
declare -a failed_suite_names=()

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Parse command line arguments
VERBOSE=0
STOP_ON_FAILURE=0
PARALLEL=0

print_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -v, --verbose       Enable verbose output"
    echo "  -s, --stop          Stop on first failure"
    echo "  -p, --parallel      Run tests in parallel (experimental)"
    echo "  -h, --help          Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                  Run all test suites"
    echo "  $0 -v               Run with verbose output"
    echo "  $0 -s               Stop on first failure"
}

while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE=1
            shift
            ;;
        -s|--stop)
            STOP_ON_FAILURE=1
            shift
            ;;
        -p|--parallel)
            PARALLEL=1
            shift
            ;;
        -h|--help)
            print_usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            print_usage
            exit 1
            ;;
    esac
done

# Print header
echo -e "${BOLD}╔════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}║        ONVIF Modular Test Suite - Running All Tests           ║${NC}"
echo -e "${BOLD}╚════════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check if test executables exist
echo -e "${CYAN}Checking test executables...${NC}"
all_exist=1
for suite_def in "${TEST_SUITES[@]}"; do
    IFS=':' read -r suite_path suite_name <<< "$suite_def"
    if [[ ! -f "$suite_path" ]]; then
        echo -e "${RED}✗${NC} $suite_name: $suite_path not found"
        all_exist=0
    else
        echo -e "${GREEN}✓${NC} $suite_name: $suite_path"
    fi
done

if [[ $all_exist -eq 0 ]]; then
    echo -e "\n${RED}Error:${NC} Some test executables are missing. Run 'make' to build them."
    exit 1
fi

echo ""
echo -e "${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BOLD}                    Executing Test Suites${NC}"
echo -e "${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo ""

# Start timer
start_time=$(date +%s)

# Function to run a single test suite
run_test_suite() {
    local suite_path=$1
    local suite_name=$2
    local result=0

    echo -e "${BLUE}▶${NC} ${BOLD}Running: $suite_name${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

    if [[ $VERBOSE -eq 1 ]]; then
        "$suite_path"
        result=$?
    else
        output=$("$suite_path" 2>&1)
        result=$?
        if [[ $result -ne 0 ]]; then
            echo "$output"
        fi
    fi

    echo ""
    return $result
}

# Run test suites
for suite_def in "${TEST_SUITES[@]}"; do
    IFS=':' read -r suite_path suite_name <<< "$suite_def"

    ((total_suites++))

    if run_test_suite "$suite_path" "$suite_name"; then
        echo -e "${GREEN}✅ PASSED:${NC} $suite_name"
        ((passed_suites++))
    else
        echo -e "${RED}❌ FAILED:${NC} $suite_name"
        ((failed_suites++))
        failed_suite_names+=("$suite_name")

        if [[ $STOP_ON_FAILURE -eq 1 ]]; then
            echo -e "\n${YELLOW}Stopping due to failure (--stop flag)${NC}"
            break
        fi
    fi

    echo ""
done

# End timer
end_time=$(date +%s)
duration=$((end_time - start_time))

# Print summary
echo -e "${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BOLD}                       Test Summary${NC}"
echo -e "${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${BOLD}Total Test Suites:${NC}  $total_suites"
echo -e "${GREEN}${BOLD}Passed:${NC}             $passed_suites"
echo -e "${RED}${BOLD}Failed:${NC}             $failed_suites"
echo -e "${BOLD}Duration:${NC}           ${duration}s"
echo ""

if [[ $failed_suites -eq 0 ]]; then
    echo -e "${GREEN}${BOLD}╔════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}${BOLD}║              ✅ ALL TEST SUITES PASSED! ✅                     ║${NC}"
    echo -e "${GREEN}${BOLD}╚════════════════════════════════════════════════════════════════╝${NC}"
    exit 0
else
    echo -e "${RED}${BOLD}Failed Test Suites:${NC}"
    for suite_name in "${failed_suite_names[@]}"; do
        echo -e "${RED}  ✗ $suite_name${NC}"
    done
    echo ""
    echo -e "${RED}${BOLD}╔════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${RED}${BOLD}║              ❌ SOME TEST SUITES FAILED ❌                     ║${NC}"
    echo -e "${RED}${BOLD}╚════════════════════════════════════════════════════════════════╝${NC}"
    exit 1
fi