// Stub implementations for glibc-specific functions that don't exist in uclibc
// These are used by Rust's standard library but are optional features

#include <errno.h>
#include <pthread.h>

// getauxval - used by std for stack overflow detection
// Returns 0 (not found) which is safe for uclibc
unsigned long getauxval(unsigned long type) {
  (void)type; // Unused parameter
  return 0;   // Indicate value not found
}

// pthread_setname_np - used for setting thread names
// This is optional, so we can just return success
int pthread_setname_np(pthread_t thread, const char *name) {
  (void)thread; // Unused
  (void)name;   // Unused
  return 0;     // Success (thread naming is optional)
}
