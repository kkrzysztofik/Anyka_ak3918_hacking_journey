#!/bin/bash
# coverage_report.sh - Generate comprehensive coverage report
# Author: kkrzysztofik
# Date: 2025

set -e

echo "ONVIF Project Coverage Report Generator"
echo "======================================="

# Check if we're in the right directory
if [ ! -f "Makefile" ]; then
    echo "Error: Please run this script from the tests directory"
    exit 1
fi

# Check if lcov is installed
if ! command -v lcov &> /dev/null; then
    echo "Error: lcov is not installed. Please install it with:"
    echo "  sudo apt-get install lcov"
    exit 1
fi

echo "Building tests with coverage..."
make coverage

echo "Running tests to generate coverage data..."
make test

echo "Generating HTML coverage report..."
make coverage-html

echo "Generating coverage summary..."
make coverage-report

echo ""
echo "Coverage Report Generated Successfully!"
echo "======================================"
echo ""
echo "📊 HTML Report: coverage/html/index.html"
echo "📈 Summary: Check the output above for coverage percentages"
echo ""
echo "To view the HTML report:"
echo "  xdg-open coverage/html/index.html"
echo ""
echo "Coverage files:"
echo "  - coverage/coverage.info  - Raw coverage data"
echo "  - coverage/html/          - HTML report directory"
echo "  - *.gcda, *.gcno         - Coverage data files"
echo ""
echo "To clean coverage files:"
echo "  make coverage-clean"
