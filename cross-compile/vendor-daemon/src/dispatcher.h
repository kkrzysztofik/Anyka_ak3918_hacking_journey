#ifndef VENDOR_DAEMON_DISPATCHER_H
#define VENDOR_DAEMON_DISPATCHER_H

/**
 * process_request - read one IPC request and dispatch to handler.
 *
 * Returns:
 *   0  – success, continue loop
 *  -1  – I/O or protocol error; caller should close client fd
 *  -2  – CMD_SHUTDOWN; caller should close client fd AND stop accept loop
 */
int process_request(int fd);

#endif /* VENDOR_DAEMON_DISPATCHER_H */
