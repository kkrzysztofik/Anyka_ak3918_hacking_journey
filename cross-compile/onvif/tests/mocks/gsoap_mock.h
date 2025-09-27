/**
 * @file gsoap_mock.h
 * @brief Mock gSOAP functions for unit testing
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef GSOAP_MOCK_H
#define GSOAP_MOCK_H

#include <stddef.h>
#include <stdint.h>

// Forward declarations and basic structures
struct soap;
struct SOAP_ENV__Fault;

// Mock soap structure
struct soap {
  int error;
  struct SOAP_ENV__Fault* fault;
  void* user;
  size_t count;
  size_t length;
  char* buf;
  size_t buflen;
};

// Mock fault structure
struct SOAP_ENV__Fault {
  char* faultstring;
  char* faultcode;
};

// Mock gSOAP functions
struct soap* soap_new(void);
void soap_free(struct soap* soap);
void soap_init(struct soap* soap);
void soap_done(struct soap* soap);
void soap_destroy(struct soap* soap);
void soap_end(struct soap* soap);

struct SOAP_ENV__Fault* soap_new_SOAP_ENV__Fault(struct soap* soap, int flag);
char* soap_strdup(struct soap* soap, const char* s);

// Mock constants
#define SOAP_FAULT 500
#define SOAP_OK 0

// Mock fault codes
#define SOAP_FAULT_VERSION_MISMATCH 1
#define SOAP_FAULT_MUST_UNDERSTAND 2
#define SOAP_FAULT_CLIENT 3
#define SOAP_FAULT_SERVER 4

#endif /* GSOAP_MOCK_H */