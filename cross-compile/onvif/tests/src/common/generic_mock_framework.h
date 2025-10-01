/**
 * @file generic_mock_framework.h
 * @brief Generic mock framework for reducing test mock duplication
 * @author kkrzysztofik
 * @date 2025
 *
 * This framework provides reusable infrastructure for creating mock implementations
 * with minimal boilerplate. It standardizes common mock patterns including:
 * - Operation result injection
 * - Call count tracking
 * - Parameter capture
 * - Error simulation
 * - Thread-safe state management
 *
 * Benefits:
 * - Reduces ~150-200 lines of duplicated mock boilerplate
 * - Standardizes mock patterns across test suite
 * - Simplifies creation of new mocks
 * - Thread-safe by default
 *
 * Usage Example:
 * @code
 * // Define operations
 * enum my_mock_operations {
 *     MY_MOCK_OP_CONNECT = 0,
 *     MY_MOCK_OP_DISCONNECT,
 *     MY_MOCK_OP_SEND,
 *     MY_MOCK_OP_COUNT
 * };
 *
 * // Create mock instance
 * GENERIC_MOCK_CREATE(my_service, MY_MOCK_OP_COUNT);
 *
 * // Initialize
 * generic_mock_init(&my_service_mock);
 *
 * // Configure operation result
 * generic_mock_set_operation_result(&my_service_mock, MY_MOCK_OP_CONNECT, SUCCESS);
 *
 * // In your mock function implementation
 * int mock_connect(void) {
 *     return generic_mock_execute_operation(&my_service_mock, MY_MOCK_OP_CONNECT, NULL);
 * }
 *
 * // In your test
 * assert_int_equal(1, generic_mock_get_operation_call_count(&my_service_mock, MY_MOCK_OP_CONNECT));
 * @endcode
 */

#ifndef GENERIC_MOCK_FRAMEWORK_H
#define GENERIC_MOCK_FRAMEWORK_H

#include <pthread.h>
#include <stddef.h>

/* ============================================================================
 * Constants
 * ============================================================================ */

#define GENERIC_MOCK_NAME_MAX_LEN 64
#define GENERIC_MOCK_MAX_OPERATIONS 32

/* ============================================================================
 * Type Definitions
 * ============================================================================ */

/**
 * @brief State for a single mock operation
 *
 * Tracks result codes, call counts, and captured parameters for one operation.
 */
typedef struct {
  /** Result code to return when operation is executed */
  int result_code;

  /** Number of times this operation has been called */
  int call_count;

  /** Pointer to last parameters passed (optional, can be NULL) */
  void* last_params;

  /** Size of parameter data (0 if last_params is NULL) */
  size_t params_size;

  /** Whether this operation is enabled (1) or should return error (0) */
  int enabled;
} generic_mock_operation_t;

/**
 * @brief Generic mock instance
 *
 * Represents a complete mock with multiple operations, error simulation,
 * and thread-safe state management.
 */
typedef struct {
  /** Name of the mock (for debugging) */
  char name[GENERIC_MOCK_NAME_MAX_LEN];

  /** Whether the mock has been initialized */
  int initialized;

  /** Number of times init function was called */
  int init_call_count;

  /** Number of times cleanup function was called */
  int cleanup_call_count;

  /** Whether error simulation is enabled (overrides operation results) */
  int error_simulation_enabled;

  /** Error code to return when error simulation is enabled */
  int error_code;

  /** Array of operations this mock supports */
  generic_mock_operation_t* operations;

  /** Number of operations in the operations array */
  int operation_count;

  /** Mutex for thread-safe operation */
  pthread_mutex_t mutex;

  /** Whether mutex has been initialized */
  int mutex_initialized;
} generic_mock_t;

/* ============================================================================
 * Macro Helpers
 * ============================================================================ */

/**
 * @brief Create a generic mock instance with static storage
 *
 * This macro creates the operations array and mock structure with static
 * storage duration, suitable for file-scope mock definitions.
 *
 * @param name Base name for the mock (will create name##_ops and name##_mock)
 * @param op_count Number of operations this mock will support
 *
 * Example:
 * @code
 * GENERIC_MOCK_CREATE(network, 5);
 * // Creates: network_ops[5] and network_mock
 * @endcode
 */
#define GENERIC_MOCK_CREATE(name, op_count)                                                        \
  static generic_mock_operation_t name##_ops[op_count] = {0};                                      \
  static generic_mock_t name##_mock = {.initialized = 0,                                           \
                                       .init_call_count = 0,                                       \
                                       .cleanup_call_count = 0,                                    \
                                       .error_simulation_enabled = 0,                              \
                                       .error_code = 0,                                            \
                                       .operations = name##_ops,                                   \
                                       .operation_count = op_count,                                \
                                       .mutex_initialized = 0}

/**
 * @brief Define standard mock getter functions
 *
 * Creates getter functions for common mock state queries.
 *
 * @param prefix Function name prefix (e.g., "network_mock" for network_mock_get_init_call_count)
 * @param mock_var The mock variable name (e.g., network_mock)
 */
#define GENERIC_MOCK_DEFINE_GETTERS(prefix, mock_var)                                              \
  int prefix##_get_init_call_count(void) {                                                         \
    return generic_mock_get_init_call_count(&mock_var);                                            \
  }                                                                                                \
  int prefix##_get_cleanup_call_count(void) {                                                      \
    return generic_mock_get_cleanup_call_count(&mock_var);                                         \
  }                                                                                                \
  int prefix##_is_error_enabled(void) {                                                            \
    return generic_mock_is_error_simulation_enabled(&mock_var);                                    \
  }

/**
 * @brief Define standard mock setter functions
 *
 * Creates setter functions for common mock state configuration.
 *
 * @param prefix Function name prefix
 * @param mock_var The mock variable name
 */
#define GENERIC_MOCK_DEFINE_SETTERS(prefix, mock_var)                                              \
  void prefix##_enable_error(int error_code) {                                                     \
    generic_mock_enable_error_simulation(&mock_var, error_code);                                   \
  }                                                                                                \
  void prefix##_disable_error(void) {                                                              \
    generic_mock_disable_error_simulation(&mock_var);                                              \
  }

/**
 * @brief Define standard mock init/cleanup functions
 *
 * Creates init and cleanup wrapper functions.
 *
 * @param prefix Function name prefix
 * @param mock_var The mock variable name
 */
#define GENERIC_MOCK_DEFINE_LIFECYCLE(prefix, mock_var)                                            \
  void prefix##_init(void) {                                                                       \
    generic_mock_init(&mock_var);                                                                  \
  }                                                                                                \
  void prefix##_cleanup(void) {                                                                    \
    generic_mock_cleanup(&mock_var);                                                               \
  }                                                                                                \
  void prefix##_reset(void) {                                                                      \
    generic_mock_reset(&mock_var);                                                                 \
  }

/**
 * @brief Complete mock definition with all standard functions
 *
 * Combines lifecycle, getters, and setters into a single macro.
 *
 * @param prefix Function name prefix
 * @param mock_var The mock variable name
 */
#define GENERIC_MOCK_DEFINE_ALL(prefix, mock_var)                                                  \
  GENERIC_MOCK_DEFINE_LIFECYCLE(prefix, mock_var)                                                  \
  GENERIC_MOCK_DEFINE_GETTERS(prefix, mock_var)                                                    \
  GENERIC_MOCK_DEFINE_SETTERS(prefix, mock_var)

/* ============================================================================
 * Core Mock Functions
 * ============================================================================ */

/**
 * @brief Initialize a generic mock instance
 *
 * Sets up the mock with default values and initializes the mutex.
 * Must be called before using the mock.
 *
 * @param mock Pointer to mock instance
 * @return 0 on success, -1 on error
 */
int generic_mock_init(generic_mock_t* mock);

/**
 * @brief Cleanup a generic mock instance
 *
 * Destroys the mutex and resets state. Should be called when done with the mock.
 *
 * @param mock Pointer to mock instance
 */
void generic_mock_cleanup(generic_mock_t* mock);

/**
 * @brief Reset mock state to initial values
 *
 * Resets all call counts and operation states but keeps the mock initialized.
 *
 * @param mock Pointer to mock instance
 */
void generic_mock_reset(generic_mock_t* mock);

/* ============================================================================
 * Operation Management
 * ============================================================================ */

/**
 * @brief Set the result code for an operation
 *
 * Configures what result code the operation should return when executed.
 *
 * @param mock Pointer to mock instance
 * @param operation_index Index of the operation (0-based)
 * @param result_code Result code to return
 * @return 0 on success, -1 on error
 */
int generic_mock_set_operation_result(generic_mock_t* mock, int operation_index, int result_code);

/**
 * @brief Get the call count for an operation
 *
 * Returns how many times the operation has been executed.
 *
 * @param mock Pointer to mock instance
 * @param operation_index Index of the operation
 * @return Call count, or -1 on error
 */
int generic_mock_get_operation_call_count(const generic_mock_t* mock, int operation_index);

/**
 * @brief Execute a mock operation
 *
 * Increments call count, captures parameters (if provided), and returns
 * the configured result code (or error code if error simulation is enabled).
 *
 * @param mock Pointer to mock instance
 * @param operation_index Index of the operation to execute
 * @param params Optional pointer to parameters to capture (can be NULL)
 * @return Configured result code or error code
 */
int generic_mock_execute_operation(generic_mock_t* mock, int operation_index, const void* params);

/**
 * @brief Enable or disable a specific operation
 *
 * When disabled, the operation will return an error code.
 *
 * @param mock Pointer to mock instance
 * @param operation_index Index of the operation
 * @param enabled 1 to enable, 0 to disable
 * @return 0 on success, -1 on error
 */
int generic_mock_set_operation_enabled(generic_mock_t* mock, int operation_index, int enabled);

/**
 * @brief Get the last captured parameters for an operation
 *
 * Returns a pointer to the last parameters passed to the operation.
 *
 * @param mock Pointer to mock instance
 * @param operation_index Index of the operation
 * @return Pointer to last parameters, or NULL if none captured
 */
void* generic_mock_get_last_params(const generic_mock_t* mock, int operation_index);

/* ============================================================================
 * Error Simulation
 * ============================================================================ */

/**
 * @brief Enable error simulation
 *
 * When enabled, all operations will return the specified error code
 * regardless of their configured result codes.
 *
 * @param mock Pointer to mock instance
 * @param error_code Error code to return
 */
void generic_mock_enable_error_simulation(generic_mock_t* mock, int error_code);

/**
 * @brief Disable error simulation
 *
 * Returns to normal operation where operations return their configured results.
 *
 * @param mock Pointer to mock instance
 */
void generic_mock_disable_error_simulation(generic_mock_t* mock);

/**
 * @brief Check if error simulation is enabled
 *
 * @param mock Pointer to mock instance
 * @return 1 if enabled, 0 if disabled, -1 on error
 */
int generic_mock_is_error_simulation_enabled(const generic_mock_t* mock);

/* ============================================================================
 * State Query Functions
 * ============================================================================ */

/**
 * @brief Get the number of times init was called
 *
 * @param mock Pointer to mock instance
 * @return Init call count, or -1 on error
 */
int generic_mock_get_init_call_count(const generic_mock_t* mock);

/**
 * @brief Get the number of times cleanup was called
 *
 * @param mock Pointer to mock instance
 * @return Cleanup call count, or -1 on error
 */
int generic_mock_get_cleanup_call_count(const generic_mock_t* mock);

/**
 * @brief Check if mock is initialized
 *
 * @param mock Pointer to mock instance
 * @return 1 if initialized, 0 if not, -1 on error
 */
int generic_mock_is_initialized(const generic_mock_t* mock);

/**
 * @brief Get total call count across all operations
 *
 * @param mock Pointer to mock instance
 * @return Total call count, or -1 on error
 */
int generic_mock_get_total_call_count(const generic_mock_t* mock);

#endif /* GENERIC_MOCK_FRAMEWORK_H */
