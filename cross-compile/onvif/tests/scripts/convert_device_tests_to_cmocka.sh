#!/bin/bash
# Script to convert device service tests from old mocks to CMocka patterns
# This script automates the conversion of mock API calls

FILE="/home/kmk/anyka-dev/cross-compile/onvif/tests/src/unit/services/device/test_device_service.c"

# Create backup
cp "$FILE" "${FILE}.backup"

# Convert gSOAP mock calls
sed -i 's/mock_gsoap_set_init_result(\([^)]*\))/expect_function_call(__wrap_onvif_gsoap_init);\n  will_return(__wrap_onvif_gsoap_init, \1)/g' "$FILE"
sed -i 's/mock_gsoap_set_cleanup_result(\([^)]*\))/expect_function_call(__wrap_onvif_gsoap_cleanup);\n  will_return(__wrap_onvif_gsoap_cleanup, \1)/g' "$FILE"
sed -i 's/mock_gsoap_set_generate_response_result(\([^)]*\))/expect_function_call(__wrap_onvif_gsoap_generate_response_with_callback);\n  will_return(__wrap_onvif_gsoap_generate_response_with_callback, \1)/g' "$FILE"

# Convert buffer pool mock calls
sed -i 's/mock_buffer_pool_set_init_result(\([^)]*\))/expect_function_call(__wrap_buffer_pool_init);\n  will_return(__wrap_buffer_pool_init, \1)/g' "$FILE"
sed -i 's/mock_buffer_pool_set_cleanup_result(\([^)]*\))/expect_function_call(__wrap_buffer_pool_cleanup);\n  will_return(__wrap_buffer_pool_cleanup, \1)/g' "$FILE"

# Convert config mock calls
sed -i 's/mock_config_set_get_string_result(\([^)]*\))/expect_function_call(__wrap_config_get_string);\n  will_return(__wrap_config_get_string, \1)/g' "$FILE"
sed -i 's/mock_config_set_get_int_result(\([^)]*\))/expect_function_call(__wrap_config_get_int);\n  will_return(__wrap_config_get_int, \1)/g' "$FILE"

# Convert platform mock calls
sed -i 's/mock_platform_set_get_system_info_result(\([^)]*\))/expect_function_call(__wrap_platform_get_system_info);\n  will_return(__wrap_platform_get_system_info, \1)/g' "$FILE"
sed -i 's/mock_platform_set_system_result(\([^)]*\))/expect_function_call(__wrap_platform_system);\n  will_return(__wrap_platform_system, \1)/g' "$FILE"

# Convert service dispatcher mock calls
sed -i 's/mock_service_dispatcher_set_register_result(\([^)]*\))/expect_function_call(__wrap_onvif_service_dispatcher_register_service);\n  will_return(__wrap_onvif_service_dispatcher_register_service, \1)/g' "$FILE"
sed -i 's/mock_service_dispatcher_set_unregister_result(\([^)]*\))/expect_function_call(__wrap_onvif_service_dispatcher_unregister_service);\n  will_return(__wrap_onvif_service_dispatcher_unregister_service, \1)/g' "$FILE"

# Convert smart response mock calls
sed -i 's/mock_smart_response_set_create_result(\([^)]*\))/expect_function_call(__wrap_smart_response_create);\n  will_return(__wrap_smart_response_create, \1)/g' "$FILE"

# Remove call count assertions - CMocka validates via expect_function_call
sed -i '/assert_int_equal([^,]*, mock_gsoap_get_init_call_count())/d' "$FILE"
sed -i '/assert_int_equal([^,]*, mock_gsoap_get_cleanup_call_count())/d' "$FILE"
sed -i '/assert_int_equal([^,]*, mock_gsoap_get_generate_response_call_count())/d' "$FILE"
sed -i '/assert_int_equal([^,]*, mock_buffer_pool_get_init_call_count())/d' "$FILE"
sed -i '/assert_int_equal([^,]*, mock_buffer_pool_get_cleanup_call_count())/d' "$FILE"
sed -i '/assert_int_equal([^,]*, mock_service_dispatcher_get_register_call_count())/d' "$FILE"
sed -i '/assert_int_equal([^,]*, mock_service_dispatcher_get_unregister_call_count())/d' "$FILE"
sed -i '/assert_int_equal([^,]*, mock_platform_get_system_call_count())/d' "$FILE"

# Remove comment lines about verifying calls
sed -i '/\/\/ Verify.*was initialized/d' "$FILE"
sed -i '/\/\/ Verify.*was called/d' "$FILE"
sed -i '/\/\/ Verify.*call/d' "$FILE"

echo "Conversion complete. Backup saved to ${FILE}.backup"
echo "Please review the changes carefully before committing."
