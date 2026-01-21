#!/usr/bin/env python3
"""
Extract coverage data from tarpaulin HTML report and identify files with low coverage.

Usage:
    python3 extract_tarpaulin_coverage.py <tarpaulin-report.html> [threshold]
    
Arguments:
    tarpaulin-report.html  - Path to the tarpaulin HTML report
    threshold              - Coverage threshold percentage (default: 80)

Example:
    python3 extract_tarpaulin_coverage.py tarpaulin-report.html 80
"""

import sys
import re


def extract_coverage_data(html_content: str, threshold: float = 80.0) -> list[tuple[float, str, int, int]]:
    """
    Extract coverage data from tarpaulin HTML report.
    
    Args:
        html_content: The HTML content of the tarpaulin report
        threshold: Coverage threshold percentage (files below this are reported)
        
    Returns:
        List of tuples: (coverage_percent, file_path, covered_lines, coverable_lines)
    """
    results = []
    
    # Pattern to match file coverage data in the HTML/JS
    pattern = r'"path":\[(.*?)\],"content":.*?,"traces":.*?,"covered":(\d+),"coverable":(\d+)'
    
    for match in re.finditer(pattern, html_content):
        path_parts = match.group(1)
        covered = int(match.group(2))
        coverable = int(match.group(3))
        
        # Extract filename from path parts
        parts = [p.strip('"') for p in path_parts.split(',')]
        
        # Get the relevant file path (last few parts for readability)
        if len(parts) >= 3:
            rel_path = '/'.join(parts[-3:])
        else:
            rel_path = '/'.join(parts)
        
        if coverable > 0:
            coverage = (covered / coverable) * 100
            if coverage < threshold:
                results.append((coverage, rel_path, covered, coverable))
    
    # Sort by coverage percentage (lowest first)
    results.sort(key=lambda x: x[0])
    
    return results


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)
    
    report_path = sys.argv[1]
    threshold = float(sys.argv[2]) if len(sys.argv) > 2 else 80.0
    
    try:
        with open(report_path, 'r', encoding='utf-8') as f:
            html_content = f.read()
    except FileNotFoundError:
        print(f"Error: File not found: {report_path}")
        sys.exit(1)
    except Exception as e:
        print(f"Error reading file: {e}")
        sys.exit(1)
    
    results = extract_coverage_data(html_content, threshold)
    
    if not results:
        print(f"All files have coverage >= {threshold}%")
        sys.exit(0)
    
    print(f"Files with coverage < {threshold}%:")
    print("-" * 70)
    print(f"{'Coverage':>8} | {'File':<45} | {'Lines'}")
    print("-" * 70)
    
    for coverage, path, covered, coverable in results:
        print(f"{coverage:>7.1f}% | {path:<45} | {covered}/{coverable}")
    
    print("-" * 70)
    print(f"Total: {len(results)} files below {threshold}% threshold")


if __name__ == "__main__":
    main()
