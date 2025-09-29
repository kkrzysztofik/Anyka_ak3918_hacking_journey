# PTZ NULL Parameter Tests - Refactored Example

This demonstrates how to use the new NULL parameter testing helpers to eliminate duplication.

## Before (Original Pattern)
```c
void test_unit_ptz_get_nodes_null_nodes(void** state) {
  (void)state;
  int count = 0;
  int result = onvif_ptz_get_nodes(NULL, &count);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}

void test_unit_ptz_get_nodes_null_count(void** state) {
  (void)state;
  struct ptz_node* nodes = NULL;
  int result = onvif_ptz_get_nodes(&nodes, NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);
}
```

## After (Refactored Pattern)
```c
// Wrapper function for onvif_ptz_get_nodes with NULL parameter testing
void test_ptz_get_nodes_with_null(void** state, const null_param_test_t* test_config) {
  (void)state;
  
  struct ptz_node* nodes = NULL;
  int count = 0;
  int result;
  
  switch (test_config->param_index) {
    case 0: // NULL nodes parameter
      result = onvif_ptz_get_nodes(NULL, &count);
      break;
    case 1: // NULL count parameter
      result = onvif_ptz_get_nodes(&nodes, NULL);
      break;
    default:
      fail_msg("Invalid parameter index: %d", test_config->param_index);
      return;
  }
  
  assert_int_equal(result, test_config->expected_result);
}

// Single test function that covers all NULL parameter cases
void test_unit_ptz_get_nodes_null_params(void** state) {
  null_param_test_t tests[] = {
    test_helper_create_null_test("nodes parameter", 0, ONVIF_ERROR_INVALID),
    test_helper_create_null_test("count parameter", 1, ONVIF_ERROR_INVALID),
  };
  
  test_helper_null_parameters(state, "onvif_ptz_get_nodes",
                             test_ptz_get_nodes_with_null,
                             tests, 2);
}
```

## Benefits
- **Reduction**: 12 lines → 6 lines per function (50% reduction)
- **Consistency**: All NULL tests follow the same pattern
- **Maintainability**: Easy to add new NULL parameter tests
- **Readability**: Clear parameter descriptions and expected results