/**
 * @file network_mock.c
 * @brief Implementation of CMocka-based network mock
 * @author kkrzysztofik
 * @date 2025
 */

#include "network_mock.h"

#include <stdbool.h>

#include "cmocka_wrapper.h"

/* ============================================================================
 * Conditional Mock/Real Function Control
 * ============================================================================ */

static bool g_use_real_functions = false;

/**
 * @brief Control whether to use real functions or mocks
 * @param use_real true to use real functions, false for mocks
 */
void network_mock_use_real_function(bool use_real) {
  g_use_real_functions = use_real;
}

/* ============================================================================
 * CMocka Wrapped Socket Functions
 * ============================================================================ */

int __wrap_socket(int domain, int type, int protocol) {
  if (g_use_real_functions) {
    return __real_socket(domain, type, protocol);
  }

  check_expected(domain);
  check_expected(type);
  check_expected(protocol);
  return (int)mock();
}

int __wrap_bind(int sockfd, const struct sockaddr* addr, socklen_t addrlen) {
  if (g_use_real_functions) {
    return __real_bind(sockfd, addr, addrlen);
  }

  check_expected(sockfd);
  check_expected_ptr(addr);
  check_expected(addrlen);
  return (int)mock();
}

int __wrap_listen(int sockfd, int backlog) {
  if (g_use_real_functions) {
    return __real_listen(sockfd, backlog);
  }

  check_expected(sockfd);
  check_expected(backlog);
  return (int)mock();
}

int __wrap_accept(int sockfd, struct sockaddr* addr, socklen_t* addrlen) {
  if (g_use_real_functions) {
    return __real_accept(sockfd, addr, addrlen);
  }

  check_expected(sockfd);
  check_expected_ptr(addr);
  check_expected_ptr(addrlen);
  return (int)mock();
}

int __wrap_connect(int sockfd, const struct sockaddr* addr, socklen_t addrlen) {
  if (g_use_real_functions) {
    return __real_connect(sockfd, addr, addrlen);
  }

  check_expected(sockfd);
  check_expected_ptr(addr);
  check_expected(addrlen);
  return (int)mock();
}

ssize_t __wrap_send(int sockfd, const void* buf, size_t len, int flags) {
  if (g_use_real_functions) {
    return __real_send(sockfd, buf, len, flags);
  }

  check_expected(sockfd);
  check_expected_ptr(buf);
  check_expected(len);
  check_expected(flags);
  return (ssize_t)mock();
}

ssize_t __wrap_recv(int sockfd, void* buf, size_t len, int flags) {
  if (g_use_real_functions) {
    return __real_recv(sockfd, buf, len, flags);
  }

  check_expected(sockfd);
  check_expected_ptr(buf);
  check_expected(len);
  check_expected(flags);
  return (ssize_t)mock();
}

ssize_t __wrap_sendto(int sockfd, const void* buf, size_t len, int flags,
                      const struct sockaddr* dest_addr, socklen_t addrlen) {
  if (g_use_real_functions) {
    return __real_sendto(sockfd, buf, len, flags, dest_addr, addrlen);
  }

  check_expected(sockfd);
  check_expected_ptr(buf);
  check_expected(len);
  check_expected(flags);
  check_expected_ptr(dest_addr);
  check_expected(addrlen);
  return (ssize_t)mock();
}

ssize_t __wrap_recvfrom(int sockfd, void* buf, size_t len, int flags, struct sockaddr* src_addr,
                        socklen_t* addrlen) {
  if (g_use_real_functions) {
    return __real_recvfrom(sockfd, buf, len, flags, src_addr, addrlen);
  }

  check_expected(sockfd);
  check_expected_ptr(buf);
  check_expected(len);
  check_expected(flags);
  check_expected_ptr(src_addr);
  check_expected_ptr(addrlen);
  return (ssize_t)mock();
}

int __wrap_close(int fd) {
  if (g_use_real_functions) {
    return __real_close(fd);
  }

  check_expected(fd);
  return (int)mock();
}

int __wrap_setsockopt(int sockfd, int level, int optname, const void* optval, socklen_t optlen) {
  if (g_use_real_functions) {
    return __real_setsockopt(sockfd, level, optname, optval, optlen);
  }

  check_expected(sockfd);
  check_expected(level);
  check_expected(optname);
  check_expected_ptr(optval);
  check_expected(optlen);
  return (int)mock();
}

int __wrap_getsockopt(int sockfd, int level, int optname, void* optval, socklen_t* optlen) {
  if (g_use_real_functions) {
    return __real_getsockopt(sockfd, level, optname, optval, optlen);
  }

  check_expected(sockfd);
  check_expected(level);
  check_expected(optname);
  check_expected_ptr(optval);
  check_expected_ptr(optlen);
  return (int)mock();
}
