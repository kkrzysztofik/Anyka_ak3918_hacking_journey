/**
 * @file network_mock.h
 * @brief CMocka-based network mock using standard function wrapping
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef NETWORK_MOCK_H
#define NETWORK_MOCK_H

#include <stdbool.h>
#include <sys/socket.h>
#include <sys/types.h>

#include "cmocka_wrapper.h"

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * CMocka Wrapped Socket Functions
 * ============================================================================
 * All socket functions are wrapped using CMocka's --wrap linker mechanism.
 * Tests use will_return() and expect_*() to configure mock behavior.
 */

/**
 * @brief CMocka wrapped socket creation
 * @param domain Socket domain
 * @param type Socket type
 * @param protocol Protocol
 * @return Socket file descriptor (configured via will_return)
 */
int __wrap_socket(int domain, int type, int protocol);

/**
 * @brief CMocka wrapped socket bind
 * @param sockfd Socket file descriptor
 * @param addr Address structure
 * @param addrlen Address length
 * @return Result code (configured via will_return)
 */
int __wrap_bind(int sockfd, const struct sockaddr* addr, socklen_t addrlen);

/**
 * @brief CMocka wrapped socket listen
 * @param sockfd Socket file descriptor
 * @param backlog Backlog size
 * @return Result code (configured via will_return)
 */
int __wrap_listen(int sockfd, int backlog);

/**
 * @brief CMocka wrapped socket accept
 * @param sockfd Socket file descriptor
 * @param addr Address structure
 * @param addrlen Address length
 * @return New socket file descriptor (configured via will_return)
 */
int __wrap_accept(int sockfd, struct sockaddr* addr, socklen_t* addrlen);

/**
 * @brief CMocka wrapped socket connect
 * @param sockfd Socket file descriptor
 * @param addr Address structure
 * @param addrlen Address length
 * @return Result code (configured via will_return)
 */
int __wrap_connect(int sockfd, const struct sockaddr* addr, socklen_t addrlen);

/**
 * @brief CMocka wrapped socket send
 * @param sockfd Socket file descriptor
 * @param buf Buffer to send
 * @param len Length of buffer
 * @param flags Send flags
 * @return Number of bytes sent (configured via will_return)
 */
ssize_t __wrap_send(int sockfd, const void* buf, size_t len, int flags);

/**
 * @brief CMocka wrapped socket receive
 * @param sockfd Socket file descriptor
 * @param buf Buffer for received data
 * @param len Length of buffer
 * @param flags Receive flags
 * @return Number of bytes received (configured via will_return)
 */
ssize_t __wrap_recv(int sockfd, void* buf, size_t len, int flags);

/**
 * @brief CMocka wrapped socket sendto
 * @param sockfd Socket file descriptor
 * @param buf Buffer to send
 * @param len Length of buffer
 * @param flags Send flags
 * @param dest_addr Destination address
 * @param addrlen Address length
 * @return Number of bytes sent (configured via will_return)
 */
ssize_t __wrap_sendto(int sockfd, const void* buf, size_t len, int flags,
                      const struct sockaddr* dest_addr, socklen_t addrlen);

/**
 * @brief CMocka wrapped socket recvfrom
 * @param sockfd Socket file descriptor
 * @param buf Buffer for received data
 * @param len Length of buffer
 * @param flags Receive flags
 * @param src_addr Source address
 * @param addrlen Address length
 * @return Number of bytes received (configured via will_return)
 */
ssize_t __wrap_recvfrom(int sockfd, void* buf, size_t len, int flags, struct sockaddr* src_addr,
                        socklen_t* addrlen);

/**
 * @brief CMocka wrapped socket close
 * @param fd File descriptor
 * @return Result code (configured via will_return)
 */
int __wrap_close(int fd);

/**
 * @brief CMocka wrapped socket setsockopt
 * @param sockfd Socket file descriptor
 * @param level Option level
 * @param optname Option name
 * @param optval Option value
 * @param optlen Option length
 * @return Result code (configured via will_return)
 */
int __wrap_setsockopt(int sockfd, int level, int optname, const void* optval, socklen_t optlen);

/**
 * @brief CMocka wrapped socket getsockopt
 * @param sockfd Socket file descriptor
 * @param level Option level
 * @param optname Option name
 * @param optval Option value
 * @param optlen Option length
 * @return Result code (configured via will_return)
 */
int __wrap_getsockopt(int sockfd, int level, int optname, void* optval, socklen_t* optlen);

/* ============================================================================
 * Conditional Mock/Real Function Control
 * ============================================================================ */

/**
 * @brief Control whether to use real functions or mocks
 * @param use_real true to use real functions, false for mocks
 */
void network_mock_use_real_function(bool use_real);

/* ============================================================================
 * CMocka Test Helper Macros - Socket Functions
 * ============================================================================ */

/**
 * @brief Set up expectations for successful socket creation
 * @param domain Expected domain
 * @param type Expected type
 * @param protocol Expected protocol
 * @param fd File descriptor to return
 */
#define EXPECT_SOCKET_SUCCESS(domain, type, protocol, fd)                                          \
  expect_value(__wrap_socket, domain, domain);                                                     \
  expect_value(__wrap_socket, type, type);                                                         \
  expect_value(__wrap_socket, protocol, protocol);                                                 \
  will_return(__wrap_socket, fd)

/**
 * @brief Set up expectations for socket creation failure
 * @param domain Expected domain
 * @param type Expected type
 * @param protocol Expected protocol
 */
#define EXPECT_SOCKET_FAIL(domain, type, protocol)                                                 \
  expect_value(__wrap_socket, domain, domain);                                                     \
  expect_value(__wrap_socket, type, type);                                                         \
  expect_value(__wrap_socket, protocol, protocol);                                                 \
  will_return(__wrap_socket, -1)

/**
 * @brief Set up expectations for successful bind
 * @param sockfd Expected socket fd
 */
#define EXPECT_BIND_SUCCESS(sockfd)                                                                \
  expect_value(__wrap_bind, sockfd, sockfd);                                                       \
  expect_any(__wrap_bind, addr);                                                                   \
  expect_any(__wrap_bind, addrlen);                                                                \
  will_return(__wrap_bind, 0)

/**
 * @brief Set up expectations for successful listen
 * @param sockfd Expected socket fd
 * @param backlog Expected backlog
 */
#define EXPECT_LISTEN_SUCCESS(sockfd, backlog)                                                     \
  expect_value(__wrap_listen, sockfd, sockfd);                                                     \
  expect_value(__wrap_listen, backlog, backlog);                                                   \
  will_return(__wrap_listen, 0)

/**
 * @brief Set up expectations for successful accept
 * @param sockfd Expected socket fd
 * @param new_fd New socket fd to return
 */
#define EXPECT_ACCEPT_SUCCESS(sockfd, new_fd)                                                      \
  expect_value(__wrap_accept, sockfd, sockfd);                                                     \
  expect_any(__wrap_accept, addr);                                                                 \
  expect_any(__wrap_accept, addrlen);                                                              \
  will_return(__wrap_accept, new_fd)

/**
 * @brief Set up expectations for successful connect
 * @param sockfd Expected socket fd
 */
#define EXPECT_CONNECT_SUCCESS(sockfd)                                                             \
  expect_value(__wrap_connect, sockfd, sockfd);                                                    \
  expect_any(__wrap_connect, addr);                                                                \
  expect_any(__wrap_connect, addrlen);                                                             \
  will_return(__wrap_connect, 0)

/**
 * @brief Set up expectations for successful send
 * @param sockfd Expected socket fd
 * @param bytes Number of bytes to return as sent
 */
#define EXPECT_SEND_SUCCESS(sockfd, bytes)                                                         \
  expect_value(__wrap_send, sockfd, sockfd);                                                       \
  expect_any(__wrap_send, buf);                                                                    \
  expect_any(__wrap_send, len);                                                                    \
  expect_any(__wrap_send, flags);                                                                  \
  will_return(__wrap_send, bytes)

/**
 * @brief Set up expectations for successful recv
 * @param sockfd Expected socket fd
 * @param bytes Number of bytes to return as received
 */
#define EXPECT_RECV_SUCCESS(sockfd, bytes)                                                         \
  expect_value(__wrap_recv, sockfd, sockfd);                                                       \
  expect_any(__wrap_recv, buf);                                                                    \
  expect_any(__wrap_recv, len);                                                                    \
  expect_any(__wrap_recv, flags);                                                                  \
  will_return(__wrap_recv, bytes)

/**
 * @brief Set up expectations for successful close
 * @param fd Expected file descriptor
 */
#define EXPECT_CLOSE_SUCCESS(fd)                                                                   \
  expect_value(__wrap_close, fd, fd);                                                              \
  will_return(__wrap_close, 0)

/**
 * @brief Set up expectations for successful setsockopt
 * @param sockfd Expected socket fd
 */
#define EXPECT_SETSOCKOPT_SUCCESS(sockfd)                                                          \
  expect_value(__wrap_setsockopt, sockfd, sockfd);                                                 \
  expect_any(__wrap_setsockopt, level);                                                            \
  expect_any(__wrap_setsockopt, optname);                                                          \
  expect_any(__wrap_setsockopt, optval);                                                           \
  expect_any(__wrap_setsockopt, optlen);                                                           \
  will_return(__wrap_setsockopt, 0)

#ifdef __cplusplus
}
#endif

#endif // NETWORK_MOCK_H
