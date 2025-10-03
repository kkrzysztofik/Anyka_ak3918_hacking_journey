/**
 * @file network_mock_cmocka.c
 * @brief Implementation of CMocka-based network mock
 * @author kkrzysztofik
 * @date 2025
 */

#include "network_mock_cmocka.h"

#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <sys/socket.h>
#include <sys/types.h>

#include <cmocka.h>

/* ============================================================================
 * CMocka Wrapped Socket Functions
 * ============================================================================ */

int __wrap_socket(int domain, int type, int protocol) {
  check_expected(domain);
  check_expected(type);
  check_expected(protocol);
  return (int)mock();
}

int __wrap_bind(int sockfd, const struct sockaddr* addr, socklen_t addrlen) {
  check_expected(sockfd);
  check_expected_ptr(addr);
  check_expected(addrlen);
  return (int)mock();
}

int __wrap_listen(int sockfd, int backlog) {
  check_expected(sockfd);
  check_expected(backlog);
  return (int)mock();
}

int __wrap_accept(int sockfd, struct sockaddr* addr, socklen_t* addrlen) {
  check_expected(sockfd);
  check_expected_ptr(addr);
  check_expected_ptr(addrlen);
  return (int)mock();
}

int __wrap_connect(int sockfd, const struct sockaddr* addr, socklen_t addrlen) {
  check_expected(sockfd);
  check_expected_ptr(addr);
  check_expected(addrlen);
  return (int)mock();
}

ssize_t __wrap_send(int sockfd, const void* buf, size_t len, int flags) {
  check_expected(sockfd);
  check_expected_ptr(buf);
  check_expected(len);
  check_expected(flags);
  return (ssize_t)mock();
}

ssize_t __wrap_recv(int sockfd, void* buf, size_t len, int flags) {
  check_expected(sockfd);
  check_expected_ptr(buf);
  check_expected(len);
  check_expected(flags);
  return (ssize_t)mock();
}

ssize_t __wrap_sendto(int sockfd, const void* buf, size_t len, int flags,
                      const struct sockaddr* dest_addr, socklen_t addrlen) {
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
  check_expected(sockfd);
  check_expected_ptr(buf);
  check_expected(len);
  check_expected(flags);
  check_expected_ptr(src_addr);
  check_expected_ptr(addrlen);
  return (ssize_t)mock();
}

int __wrap_close(int fd) {
  check_expected(fd);
  return (int)mock();
}

int __wrap_setsockopt(int sockfd, int level, int optname, const void* optval, socklen_t optlen) {
  check_expected(sockfd);
  check_expected(level);
  check_expected(optname);
  check_expected_ptr(optval);
  check_expected(optlen);
  return (int)mock();
}

int __wrap_getsockopt(int sockfd, int level, int optname, void* optval, socklen_t* optlen) {
  check_expected(sockfd);
  check_expected(level);
  check_expected(optname);
  check_expected_ptr(optval);
  check_expected_ptr(optlen);
  return (int)mock();
}
