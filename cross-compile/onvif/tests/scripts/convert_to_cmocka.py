#!/usr/bin/env python3
"""
Convert device service tests from old mock API to CMocka patterns.
This script performs automated conversion with proper pattern matching.
"""

import re
import sys
from pathlib import Path

# Conversion mappings for different mock types
CONVERSIONS = {
    # gSOAP mocks
    r'mock_gsoap_set_init_result\(([^)]+)\)':
        r'expect_function_call(__wrap_onvif_gsoap_init);\n  will_return(__wrap_onvif_gsoap_init, \1)',
    r'mock_gsoap_set_cleanup_result\(([^)]+)\)':
        r'expect_function_call(__wrap_onvif_gsoap_cleanup);\n  will_return(__wrap_onvif_gsoap_cleanup, \1)',
    r'mock_gsoap_set_generate_response_result\(([^)]+)\)':
        r'expect_function_call(__wrap_onvif_gsoap_generate_response_with_callback);\n  will_return(__wrap_onvif_gsoap_generate_response_with_callback, \1)',
    r'mock_gsoap_set_get_response_data_result\(([^,]+),\s*([^)]+)\)':
        r'expect_function_call(__wrap_onvif_gsoap_get_response_data);\n  will_return(__wrap_onvif_gsoap_get_response_data, \1)',

    # Buffer pool mocks
    r'mock_buffer_pool_set_init_result\(([^)]+)\)':
        r'expect_function_call(__wrap_buffer_pool_init);\n  will_return(__wrap_buffer_pool_init, \1)',
    r'mock_buffer_pool_set_cleanup_result\(([^)]+)\)':
        r'expect_function_call(__wrap_buffer_pool_cleanup);\n  will_return(__wrap_buffer_pool_cleanup, \1)',

    # Config mocks
    r'mock_config_set_get_string_result\(([^)]+)\)':
        r'expect_function_call(__wrap_config_get_string);\n  will_return(__wrap_config_get_string, \1)',
    r'mock_config_set_get_int_result\(([^)]+)\)':
        r'expect_function_call(__wrap_config_get_int);\n  will_return(__wrap_config_get_int, \1)',

    # Platform mocks
    r'mock_platform_set_get_system_info_result\(([^)]+)\)':
        r'expect_function_call(__wrap_platform_get_system_info);\n  will_return(__wrap_platform_get_system_info, \1)',
    r'mock_platform_set_system_result\(([^)]+)\)':
        r'expect_function_call(__wrap_platform_system);\n  will_return(__wrap_platform_system, \1)',

    # Service dispatcher mocks
    r'mock_service_dispatcher_set_register_result\(([^)]+)\)':
        r'expect_function_call(__wrap_onvif_service_dispatcher_register_service);\n  will_return(__wrap_onvif_service_dispatcher_register_service, \1)',
    r'mock_service_dispatcher_set_unregister_result\(([^)]+)\)':
        r'expect_function_call(__wrap_onvif_service_dispatcher_unregister_service);\n  will_return(__wrap_onvif_service_dispatcher_unregister_service, \1)',

    # Smart response mocks
    r'mock_smart_response_set_create_result\(([^)]+)\)':
        r'expect_function_call(__wrap_smart_response_create);\n  will_return(__wrap_smart_response_create, \1)',
}

# Patterns to remove (call count assertions and verification comments)
PATTERNS_TO_REMOVE = [
    r'\s*// Verify.*\n',
    r'\s*assert_int_equal\([^,]+,\s*mock_gsoap_get_\w+_call_count\(\)\);\n',
    r'\s*assert_int_equal\([^,]+,\s*mock_buffer_pool_get_\w+_call_count\(\)\);\n',
    r'\s*assert_int_equal\([^,]+,\s*mock_service_dispatcher_get_\w+_call_count\(\)\);\n',
    r'\s*assert_int_equal\([^,]+,\s*mock_platform_get_\w+_call_count\(\)\);\n',
    r'\s*assert_int_equal\([^,]+,\s*mock_config_get_\w+_call_count\(\)\);\n',
    r'\s*assert_int_equal\([^,]+,\s*mock_smart_response_get_\w+_call_count\(\)\);\n',
]

def convert_file(file_path):
    """Convert a test file from old mock API to CMocka patterns."""
    print(f"Converting {file_path}...")

    # Read the file
    with open(file_path, 'r') as f:
        content = f.read()

    # Create backup
    backup_path = Path(str(file_path) + '.backup')
    with open(backup_path, 'w') as f:
        f.write(content)
    print(f"Backup created: {backup_path}")

    # Apply conversions
    for pattern, replacement in CONVERSIONS.items():
        content = re.sub(pattern, replacement, content)

    # Remove old patterns
    for pattern in PATTERNS_TO_REMOVE:
        content = re.sub(pattern, '', content)

    # Write converted content
    with open(file_path, 'w') as f:
        f.write(content)

    print(f"Conversion complete!")
    print(f"Backup saved to: {backup_path}")
    print(f"Please review changes before committing.")

def main():
    if len(sys.argv) != 2:
        print("Usage: convert_to_cmocka.py <test_file.c>")
        sys.exit(1)

    file_path = Path(sys.argv[1])
    if not file_path.exists():
        print(f"Error: File not found: {file_path}")
        sys.exit(1)

    convert_file(file_path)

if __name__ == '__main__':
    main()
