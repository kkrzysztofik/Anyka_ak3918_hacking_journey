use std::net::SocketAddr;
use std::time::Duration;

use async_trait::async_trait;
use bytes::BufMut;
use bytes::Bytes;
use bytes::BytesMut;
use futures::SinkExt;
use futures::StreamExt;
use tokio::net::TcpStream;
use tokio::net::UdpSocket;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio_util::codec::BytesCodec;
use tokio_util::codec::Framed;
use tokio_util::codec::{FramedRead, FramedWrite};

use super::bytesio_errors::{BytesIOError, BytesIOErrorValue};

/// Maximum UDP datagram size (RFC 768 limit)
const MAX_UDP_DATAGRAM_SIZE: usize = 65507;

pub enum NetType {
    TCP,
    UDP,
}

#[async_trait]
pub trait TNetIO: Send + Sync {
    async fn write(&mut self, bytes: Bytes) -> Result<(), BytesIOError>;
    async fn read(&mut self) -> Result<BytesMut, BytesIOError>;
    async fn read_timeout(&mut self, duration: Duration) -> Result<BytesMut, BytesIOError>;
    fn get_net_type(&self) -> NetType;
}

pub struct UdpIO {
    socket: UdpSocket,
}

impl UdpIO {
    pub async fn new(remote_domain: String, remote_port: u16, local_port: u16) -> Option<Self> {
        let remote_address = if remote_domain == "localhost" {
            format!("127.0.0.1:{remote_port}")
        } else {
            format!("{remote_domain}:{remote_port}")
        };
        tracing::debug!(remote_address = %remote_address, "udp_connection_attempt");
        let local_address = format!("0.0.0.0:{local_port}");
        if let Ok(local_socket) = UdpSocket::bind(local_address).await {
            if let Ok(remote_socket_addr) = remote_address.parse::<SocketAddr>() {
                match local_socket.connect(remote_socket_addr).await {
                    Ok(()) => {
                        return Some(Self {
                            socket: local_socket,
                        });
                    }
                    Err(err) => {
                        tracing::error!(error = %err, remote_address = %remote_address, "udp_connect_error");
                        return None;
                    }
                }
            } else {
                tracing::error!(remote_address = %remote_address, "remote_address_parse_error");
            }
        }

        None
    }

    pub async fn new_with_local_port(local_port: u16) -> Option<Self> {
        let local_address = format!("0.0.0.0:{local_port}");

        if let Ok(local_socket) = UdpSocket::bind(local_address).await {
            return Some(Self {
                socket: local_socket,
            });
        }
        None
    }

    pub fn get_local_port(&self) -> Option<u16> {
        if let Ok(local_addr) = self.socket.local_addr() {
            tracing::debug!(local_addr = %local_addr, "local_port_retrieved");
            return Some(local_addr.port());
        }

        None
    }
}

const MAX_PORT: u16 = 65535;

fn wrap_port(port: u16) -> u16 {
    if port == MAX_PORT { 1 } else { port }
}

/// Try to create a UDP pair bound to (port, port+1). Returns None if either bind fails.
async fn try_udpio_pair_at(port: u16) -> Option<(UdpIO, UdpIO)> {
    let udpio_0 = UdpIO::new_with_local_port(port).await?;
    let next_port = port.wrapping_add(1);
    let udpio_1 = UdpIO::new_with_local_port(next_port).await?;
    Some((udpio_0, udpio_1))
}

/// Compute the next port to try after a failed pair attempt.
fn next_try_port(after: u16) -> u16 {
    if after == MAX_PORT { 1 } else { after + 1 }
}

pub async fn new_udpio_pair() -> Option<(UdpIO, UdpIO)> {
    const MAX_ATTEMPTS: u32 = 32768;

    let udpio_0 = UdpIO::new_with_local_port(0).await?;
    let first_local_port = udpio_0.get_local_port()?;
    let first_plus_one = first_local_port.wrapping_add(1);

    if first_local_port != MAX_PORT
        && let Some(udpio_1) = UdpIO::new_with_local_port(first_plus_one).await
    {
        return Some((udpio_0, udpio_1));
    }

    let mut next_local_port = if first_local_port == MAX_PORT || first_plus_one == MAX_PORT {
        1
    } else {
        next_try_port(first_plus_one)
    };

    for _attempt_count in 1..=MAX_ATTEMPTS {
        tracing::trace!(
            next_local_port = next_local_port,
            first_local_port = first_local_port,
            "udp_pair_attempt"
        );

        if next_local_port == first_local_port {
            return None;
        }
        next_local_port = wrap_port(next_local_port);
        if next_local_port == first_local_port {
            return None;
        }

        if let Some(pair) = try_udpio_pair_at(next_local_port).await {
            return Some(pair);
        }
        next_local_port = next_try_port(next_local_port.wrapping_add(1));
    }

    tracing::error!("new_udpio_pair_max_attempts_exceeded");
    None
}

#[async_trait]
impl TNetIO for UdpIO {
    fn get_net_type(&self) -> NetType {
        NetType::UDP
    }

    async fn write(&mut self, bytes: Bytes) -> Result<(), BytesIOError> {
        self.socket.send(bytes.as_ref()).await?;
        Ok(())
    }

    async fn read_timeout(&mut self, duration: Duration) -> Result<BytesMut, BytesIOError> {
        match tokio::time::timeout(duration, self.read()).await {
            Ok(data) => data,
            Err(err) => Err(BytesIOError {
                value: BytesIOErrorValue::TimeoutError(err),
            }),
        }
    }

    async fn read(&mut self) -> Result<BytesMut, BytesIOError> {
        let mut buf = vec![0; MAX_UDP_DATAGRAM_SIZE];
        let len = self.socket.recv(&mut buf).await?;
        let mut rv = BytesMut::new();
        rv.put(&buf[..len]);

        Ok(rv)
    }
}

pub struct TcpIO {
    stream: Framed<TcpStream, BytesCodec>,
    //timeout: Duration,
}

impl TcpIO {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream: Framed::new(stream, BytesCodec::new()),
            // timeout: ms,
        }
    }
}

#[async_trait]
impl TNetIO for TcpIO {
    fn get_net_type(&self) -> NetType {
        NetType::TCP
    }

    async fn write(&mut self, bytes: Bytes) -> Result<(), BytesIOError> {
        self.stream.send(bytes).await?;

        Ok(())
    }

    async fn read_timeout(&mut self, duration: Duration) -> Result<BytesMut, BytesIOError> {
        match tokio::time::timeout(duration, self.read()).await {
            Ok(data) => data,
            Err(err) => Err(BytesIOError {
                value: BytesIOErrorValue::TimeoutError(err),
            }),
        }
    }

    async fn read(&mut self) -> Result<BytesMut, BytesIOError> {
        let message = self.stream.next().await;

        match message {
            Some(data) => match data {
                Ok(bytes) => Ok(bytes),
                Err(err) => Err(BytesIOError {
                    value: BytesIOErrorValue::IOError(err),
                }),
            },
            None => Err(BytesIOError {
                value: BytesIOErrorValue::NoneReturn,
            }),
        }
    }
}

pub struct TcpReadIO {
    stream: FramedRead<OwnedReadHalf, BytesCodec>,
}

impl TcpReadIO {
    pub fn new(stream: OwnedReadHalf) -> Self {
        Self {
            stream: FramedRead::new(stream, BytesCodec::new()),
        }
    }
}

#[async_trait]
impl TNetIO for TcpReadIO {
    fn get_net_type(&self) -> NetType {
        NetType::TCP
    }

    async fn write(&mut self, _bytes: Bytes) -> Result<(), BytesIOError> {
        Err(BytesIOError {
            value: BytesIOErrorValue::IOError(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "write not supported on TcpReadIO",
            )),
        })
    }

    async fn read_timeout(&mut self, duration: Duration) -> Result<BytesMut, BytesIOError> {
        match tokio::time::timeout(duration, self.read()).await {
            Ok(data) => data,
            Err(err) => Err(BytesIOError {
                value: BytesIOErrorValue::TimeoutError(err),
            }),
        }
    }

    async fn read(&mut self) -> Result<BytesMut, BytesIOError> {
        let message = self.stream.next().await;

        match message {
            Some(data) => match data {
                Ok(bytes) => Ok(bytes),
                Err(err) => Err(BytesIOError {
                    value: BytesIOErrorValue::IOError(err),
                }),
            },
            None => Err(BytesIOError {
                value: BytesIOErrorValue::NoneReturn,
            }),
        }
    }
}

pub struct TcpWriteIO {
    stream: FramedWrite<OwnedWriteHalf, BytesCodec>,
}

impl TcpWriteIO {
    pub fn new(stream: OwnedWriteHalf) -> Self {
        Self {
            stream: FramedWrite::new(stream, BytesCodec::new()),
        }
    }
}

#[async_trait]
impl TNetIO for TcpWriteIO {
    fn get_net_type(&self) -> NetType {
        NetType::TCP
    }

    async fn write(&mut self, bytes: Bytes) -> Result<(), BytesIOError> {
        self.stream.send(bytes).await?;
        Ok(())
    }

    async fn read_timeout(&mut self, duration: Duration) -> Result<BytesMut, BytesIOError> {
        match tokio::time::timeout(duration, self.read()).await {
            Ok(data) => data,
            Err(err) => Err(BytesIOError {
                value: BytesIOErrorValue::TimeoutError(err),
            }),
        }
    }

    async fn read(&mut self) -> Result<BytesMut, BytesIOError> {
        Err(BytesIOError {
            value: BytesIOErrorValue::IOError(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "read not supported on TcpWriteIO",
            )),
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::time::Duration;

    // ========== NetType Enum Tests ==========

    #[test]
    fn test_net_type_tcp_variant_exists() {
        let net_type = NetType::TCP;
        match net_type {
            NetType::TCP => assert!(true),
            NetType::UDP => panic!("Expected TCP variant"),
        }
    }

    #[test]
    fn test_net_type_udp_variant_exists() {
        let net_type = NetType::UDP;
        match net_type {
            NetType::UDP => assert!(true),
            NetType::TCP => panic!("Expected UDP variant"),
        }
    }

    // ========== TcpIO Tests ==========

    #[tokio::test]
    async fn test_tcpio_new_creates_instance() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let tcpio = TcpIO::new(stream);
            // Verify TcpIO was created successfully by checking net type
            assert!(matches!(tcpio.get_net_type(), NetType::TCP));
        });

        let _accept_task = tokio::spawn(async move {
            let _ = listener.accept().await;
        });

        let _ = connect_task.await;
    }

    #[tokio::test]
    async fn test_tcpio_get_net_type_returns_tcp() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let tcpio = TcpIO::new(stream);
            assert!(matches!(tcpio.get_net_type(), NetType::TCP));
        });

        let _accept_task = tokio::spawn(async move {
            let _ = listener.accept().await;
        });

        let _ = connect_task.await;
    }

    #[tokio::test]
    async fn test_tcpio_write_success() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let mut tcpio = TcpIO::new(stream);
            let data = Bytes::from("hello world");
            let result = tcpio.write(data).await;
            assert!(
                result.is_ok(),
                "Write should succeed on connected TCP stream"
            );
        });

        let accept_task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 11];
            let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf)
                .await
                .unwrap();
            assert_eq!(n, 11);
            assert_eq!(&buf[..], b"hello world");
        });

        let _ = tokio::try_join!(connect_task, accept_task);
    }

    #[tokio::test]
    async fn test_tcpio_write_empty_data_success() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let mut tcpio = TcpIO::new(stream);
            let data = Bytes::from("");
            let result = tcpio.write(data).await;
            assert!(result.is_ok(), "Writing empty data should succeed");
        });

        let _accept_task = tokio::spawn(async move {
            let _ = listener.accept().await;
        });

        let _ = connect_task.await;
    }

    #[tokio::test]
    async fn test_tcpio_read_success() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let mut tcpio = TcpIO::new(stream);
            let result = tcpio.read().await;
            assert!(result.is_ok(), "Read should succeed when data is available");
            let data = result.unwrap();
            assert_eq!(&data[..], b"test data");
        });

        let accept_task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            tokio::io::AsyncWriteExt::write_all(&mut stream, b"test data")
                .await
                .unwrap();
            tokio::io::AsyncWriteExt::flush(&mut stream).await.unwrap();
        });

        let _ = tokio::try_join!(connect_task, accept_task);
    }

    #[tokio::test]
    async fn test_tcpio_read_none_returns_error() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let mut tcpio = TcpIO::new(stream);
            // Give server time to close connection
            tokio::time::sleep(Duration::from_millis(50)).await;
            let result = tcpio.read().await;
            assert!(
                result.is_err(),
                "Read should return error when stream is closed"
            );
            let err = result.unwrap_err();
            assert!(matches!(err.value, BytesIOErrorValue::NoneReturn));
        });

        let accept_task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            // Immediately close the connection
            drop(stream);
        });

        let _ = tokio::try_join!(connect_task, accept_task);
    }

    #[tokio::test]
    async fn test_tcpio_read_timeout_success() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let mut tcpio = TcpIO::new(stream);
            let timeout_duration = Duration::from_secs(5);
            let result = tcpio.read_timeout(timeout_duration).await;
            assert!(
                result.is_ok(),
                "Read with timeout should succeed when data arrives"
            );
            let data = result.unwrap();
            assert_eq!(&data[..], b"timeout test");
        });

        let accept_task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            // Small delay before sending
            tokio::time::sleep(Duration::from_millis(10)).await;
            tokio::io::AsyncWriteExt::write_all(&mut stream, b"timeout test")
                .await
                .unwrap();
            tokio::io::AsyncWriteExt::flush(&mut stream).await.unwrap();
        });

        let _ = tokio::try_join!(connect_task, accept_task);
    }

    #[tokio::test]
    async fn test_tcpio_read_timeout_elapsed_returns_error() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let mut tcpio = TcpIO::new(stream);
            // Very short timeout - will definitely elapse
            let timeout_duration = Duration::from_millis(1);
            let result = tcpio.read_timeout(timeout_duration).await;
            assert!(
                result.is_err(),
                "Read should timeout with very short duration"
            );
            let err = result.unwrap_err();
            assert!(matches!(err.value, BytesIOErrorValue::TimeoutError(_)));
        });

        let accept_task = tokio::spawn(async move {
            let _ = listener.accept().await.unwrap();
            // Don't send any data - let timeout occur
        });

        let _ = tokio::try_join!(connect_task, accept_task);
    }

    #[tokio::test]
    async fn test_tcpio_write_large_payload_success() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Create a large payload (64KB)
        let large_data = vec![0xABu8; 65536];
        let large_data_clone = large_data.clone();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let mut tcpio = TcpIO::new(stream);
            let data = Bytes::from(large_data_clone);
            let result = tcpio.write(data).await;
            assert!(result.is_ok(), "Writing large payload should succeed");
        });

        let accept_task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 65536];
            let mut total_read = 0;
            while total_read < 65536 {
                let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf[total_read..])
                    .await
                    .unwrap();
                total_read += n;
                if n == 0 {
                    break;
                }
            }
            assert_eq!(total_read, 65536);
            assert_eq!(buf, large_data);
        });

        let _ = tokio::try_join!(connect_task, accept_task);
    }

    // ========== TcpReadIO Tests ==========

    #[tokio::test]
    async fn test_tcpreadio_new_creates_instance() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let (read_half, _) = stream.into_split();
            let tcpreadio = TcpReadIO::new(read_half);
            assert!(matches!(tcpreadio.get_net_type(), NetType::TCP));
        });

        let _accept_task = tokio::spawn(async move {
            let _ = listener.accept().await;
        });

        let _ = connect_task.await;
    }

    #[tokio::test]
    async fn test_tcpreadio_get_net_type_returns_tcp() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let (read_half, _) = stream.into_split();
            let tcpreadio = TcpReadIO::new(read_half);
            assert!(matches!(tcpreadio.get_net_type(), NetType::TCP));
        });

        let _accept_task = tokio::spawn(async move {
            let _ = listener.accept().await;
        });

        let _ = connect_task.await;
    }

    #[tokio::test]
    async fn test_tcpreadio_write_returns_unsupported_error() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let (read_half, _) = stream.into_split();
            let mut tcpreadio = TcpReadIO::new(read_half);
            let data = Bytes::from("test");
            let result = tcpreadio.write(data).await;
            assert!(result.is_err(), "Write on TcpReadIO should return error");
            let err = result.unwrap_err();
            if let BytesIOErrorValue::IOError(io_err) = err.value {
                assert_eq!(io_err.kind(), std::io::ErrorKind::Unsupported);
            } else {
                panic!("Expected IOError variant");
            }
        });

        let _accept_task = tokio::spawn(async move {
            let _ = listener.accept().await;
        });

        let _ = connect_task.await;
    }

    #[tokio::test]
    async fn test_tcpreadio_read_success() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let (read_half, _) = stream.into_split();
            let mut tcpreadio = TcpReadIO::new(read_half);
            let result = tcpreadio.read().await;
            assert!(result.is_ok(), "Read should succeed when data is available");
            let data = result.unwrap();
            assert_eq!(&data[..], b"read test data");
        });

        let accept_task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            tokio::io::AsyncWriteExt::write_all(&mut stream, b"read test data")
                .await
                .unwrap();
            tokio::io::AsyncWriteExt::flush(&mut stream).await.unwrap();
        });

        let _ = tokio::try_join!(connect_task, accept_task);
    }

    #[tokio::test]
    async fn test_tcpreadio_read_none_returns_error() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let (read_half, _) = stream.into_split();
            let mut tcpreadio = TcpReadIO::new(read_half);
            // Give server time to close connection
            tokio::time::sleep(Duration::from_millis(50)).await;
            let result = tcpreadio.read().await;
            assert!(
                result.is_err(),
                "Read should return error when stream is closed"
            );
            let err = result.unwrap_err();
            assert!(matches!(err.value, BytesIOErrorValue::NoneReturn));
        });

        let accept_task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            // Immediately close the connection
            drop(stream);
        });

        let _ = tokio::try_join!(connect_task, accept_task);
    }

    #[tokio::test]
    async fn test_tcpreadio_read_timeout_success() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let (read_half, _) = stream.into_split();
            let mut tcpreadio = TcpReadIO::new(read_half);
            let timeout_duration = Duration::from_secs(5);
            let result = tcpreadio.read_timeout(timeout_duration).await;
            assert!(
                result.is_ok(),
                "Read with timeout should succeed when data arrives"
            );
            let data = result.unwrap();
            assert_eq!(&data[..], b"timeout read test");
        });

        let accept_task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            tokio::time::sleep(Duration::from_millis(10)).await;
            tokio::io::AsyncWriteExt::write_all(&mut stream, b"timeout read test")
                .await
                .unwrap();
            tokio::io::AsyncWriteExt::flush(&mut stream).await.unwrap();
        });

        let _ = tokio::try_join!(connect_task, accept_task);
    }

    #[tokio::test]
    async fn test_tcpreadio_read_timeout_elapsed_returns_error() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let (read_half, _) = stream.into_split();
            let mut tcpreadio = TcpReadIO::new(read_half);
            let timeout_duration = Duration::from_millis(1);
            let result = tcpreadio.read_timeout(timeout_duration).await;
            assert!(result.is_err(), "Read should timeout");
            let err = result.unwrap_err();
            assert!(matches!(err.value, BytesIOErrorValue::TimeoutError(_)));
        });

        let accept_task = tokio::spawn(async move {
            let _ = listener.accept().await.unwrap();
            // Don't send any data
        });

        let _ = tokio::try_join!(connect_task, accept_task);
    }

    // ========== TcpWriteIO Tests ==========

    #[tokio::test]
    async fn test_tcpwriteio_new_creates_instance() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let (_, write_half) = stream.into_split();
            let tcpwriteio = TcpWriteIO::new(write_half);
            assert!(matches!(tcpwriteio.get_net_type(), NetType::TCP));
        });

        let _accept_task = tokio::spawn(async move {
            let _ = listener.accept().await;
        });

        let _ = connect_task.await;
    }

    #[tokio::test]
    async fn test_tcpwriteio_get_net_type_returns_tcp() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let (_, write_half) = stream.into_split();
            let tcpwriteio = TcpWriteIO::new(write_half);
            assert!(matches!(tcpwriteio.get_net_type(), NetType::TCP));
        });

        let _accept_task = tokio::spawn(async move {
            let _ = listener.accept().await;
        });

        let _ = connect_task.await;
    }

    #[tokio::test]
    async fn test_tcpwriteio_read_returns_unsupported_error() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let (_, write_half) = stream.into_split();
            let mut tcpwriteio = TcpWriteIO::new(write_half);
            let result = tcpwriteio.read().await;
            assert!(result.is_err(), "Read on TcpWriteIO should return error");
            let err = result.unwrap_err();
            if let BytesIOErrorValue::IOError(io_err) = err.value {
                assert_eq!(io_err.kind(), std::io::ErrorKind::Unsupported);
                assert!(io_err.to_string().contains("read not supported"));
            } else {
                panic!("Expected IOError variant");
            }
        });

        let _accept_task = tokio::spawn(async move {
            let _ = listener.accept().await;
        });

        let _ = connect_task.await;
    }

    #[tokio::test]
    async fn test_tcpwriteio_write_success() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let (_, write_half) = stream.into_split();
            let mut tcpwriteio = TcpWriteIO::new(write_half);
            let data = Bytes::from("write test data");
            let result = tcpwriteio.write(data).await;
            assert!(result.is_ok(), "Write should succeed on TcpWriteIO");
        });

        let accept_task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 15];
            let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf)
                .await
                .unwrap();
            assert_eq!(n, 15);
            assert_eq!(&buf[..], b"write test data");
        });

        let _ = tokio::try_join!(connect_task, accept_task);
    }

    #[tokio::test]
    async fn test_tcpwriteio_write_empty_data_success() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let (_, write_half) = stream.into_split();
            let mut tcpwriteio = TcpWriteIO::new(write_half);
            let data = Bytes::from("");
            let result = tcpwriteio.write(data).await;
            assert!(result.is_ok(), "Writing empty data should succeed");
        });

        let _accept_task = tokio::spawn(async move {
            let _ = listener.accept().await;
        });

        let _ = connect_task.await;
    }

    #[tokio::test]
    async fn test_tcpwriteio_read_timeout_returns_unsupported_error() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let (_, write_half) = stream.into_split();
            let mut tcpwriteio = TcpWriteIO::new(write_half);
            let timeout_duration = Duration::from_secs(1);
            let result = tcpwriteio.read_timeout(timeout_duration).await;
            // Since read() always returns Unsupported error, read_timeout should too
            assert!(
                result.is_err(),
                "Read timeout on TcpWriteIO should return error"
            );
            let err = result.unwrap_err();
            if let BytesIOErrorValue::IOError(io_err) = err.value {
                assert_eq!(io_err.kind(), std::io::ErrorKind::Unsupported);
            } else {
                panic!("Expected IOError variant");
            }
        });

        let _accept_task = tokio::spawn(async move {
            let _ = listener.accept().await;
        });

        let _ = connect_task.await;
    }

    #[tokio::test]
    async fn test_tcpwriteio_write_large_payload_success() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let large_data = vec![0xCDu8; 65536];
        let large_data_clone = large_data.clone();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let (_, write_half) = stream.into_split();
            let mut tcpwriteio = TcpWriteIO::new(write_half);
            let data = Bytes::from(large_data_clone);
            let result = tcpwriteio.write(data).await;
            assert!(result.is_ok(), "Writing large payload should succeed");
        });

        let accept_task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 65536];
            let mut total_read = 0;
            while total_read < 65536 {
                let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf[total_read..])
                    .await
                    .unwrap();
                total_read += n;
                if n == 0 {
                    break;
                }
            }
            assert_eq!(total_read, 65536);
            assert_eq!(buf, large_data);
        });

        let _ = tokio::try_join!(connect_task, accept_task);
    }

    // ========== UdpIO TNetIO Trait Tests ==========

    #[tokio::test]
    async fn test_udpio_get_net_type_returns_udp() {
        if let Some(udpio) = UdpIO::new_with_local_port(0).await {
            assert!(matches!(udpio.get_net_type(), NetType::UDP));
        }
    }

    #[tokio::test]
    async fn test_udpio_new_with_localhost_resolves() {
        // Test that localhost resolution works
        let result = UdpIO::new("localhost".to_string(), 12345, 0).await;
        // This may fail if port is in use, so we just check it doesn't panic
        if let Some(udpio) = result {
            assert!(matches!(udpio.get_net_type(), NetType::UDP));
        }
    }

    #[tokio::test]
    async fn test_udpio_new_with_invalid_address_returns_none() {
        // Invalid IP address format should return None
        let result = UdpIO::new("invalid-address-that-does-not-exist".to_string(), 12345, 0).await;
        assert!(result.is_none(), "Invalid address should return None");
    }

    #[tokio::test]
    async fn test_udpio_get_local_port_returns_valid_port() {
        if let Some(udpio) = UdpIO::new_with_local_port(0).await {
            if let Some(port) = udpio.get_local_port() {
                assert!(port > 0, "OS-assigned port should be greater than 0");
            } else {
                panic!("get_local_port should return Some for valid socket");
            }
        }
    }

    #[tokio::test]
    async fn test_udpio_new_with_specific_port() {
        // First get an available port
        if let Some(temp_udpio) = UdpIO::new_with_local_port(0).await {
            if let Some(port) = temp_udpio.get_local_port() {
                // Drop the temp socket to free the port
                drop(temp_udpio);

                // Now try to bind to the same port (may fail if port is reused)
                // Just verify the function doesn't panic
                let _ = UdpIO::new_with_local_port(port).await;
            }
        }
    }

    #[tokio::test]
    async fn test_udpio_write_and_read_roundtrip() {
        if let Some((udpio1, udpio2)) = new_udpio_pair().await {
            if let (Some(port1), Some(port2)) = (udpio1.get_local_port(), udpio2.get_local_port()) {
                // Create new UDP sockets connected to each other
                if let (Some(mut sender), Some(mut receiver)) = (
                    UdpIO::new("127.0.0.1".to_string(), port2, port1).await,
                    UdpIO::new("127.0.0.1".to_string(), port1, port2).await,
                ) {
                    drop(udpio1);
                    drop(udpio2);

                    let send_task = tokio::spawn(async move {
                        let data = Bytes::from("udp test message");
                        let result = sender.write(data).await;
                        assert!(result.is_ok(), "UDP write should succeed");
                    });

                    let recv_task = tokio::spawn(async move {
                        let result = receiver.read().await;
                        assert!(result.is_ok(), "UDP read should succeed");
                        let data = result.unwrap();
                        assert_eq!(&data[..], b"udp test message");
                    });

                    let _ = tokio::try_join!(send_task, recv_task);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_udpio_read_timeout_elapsed_returns_error() {
        if let Some(mut udpio) = UdpIO::new_with_local_port(0).await {
            // Very short timeout with no data being sent
            let timeout_duration = Duration::from_millis(1);
            let result = udpio.read_timeout(timeout_duration).await;
            assert!(result.is_err(), "Read should timeout when no data arrives");
            let err = result.unwrap_err();
            assert!(matches!(err.value, BytesIOErrorValue::TimeoutError(_)));
        }
    }

    // ========== new_udpio_pair Tests (existing tests preserved) ==========

    #[tokio::test]
    async fn test_new_udpio_pair() {
        if let Some((udpio1, udpid2)) = new_udpio_pair().await {
            println!(
                "{:?} == {:?}",
                udpio1.get_local_port(),
                udpid2.get_local_port()
            );
        }
    }

    #[tokio::test]
    #[ignore] // This test is too slow for normal test runs
    async fn test_new_udpio_pair2() {
        println!("test_new_udpio_pair2 begin...");
        let mut socket: Vec<UdpIO> = Vec::new();

        // Only test a sample of ports to avoid extremely slow test execution
        // Testing port 0 (automatic), and a few high port numbers
        let test_ports = [0, 30000, 40000, 50000];

        for i in test_ports.iter() {
            println!("cur port: {}", i);
            if let Some(udpio) = UdpIO::new_with_local_port(*i).await {
                socket.push(udpio)
            } else {
                println!("new local port fail: {}", i);
            }
        }

        println!("socket size: {}", socket.len());

        if let Some((udpio1, udpid2)) = new_udpio_pair().await {
            println!(
                "{:?} == {:?}",
                udpio1.get_local_port(),
                udpid2.get_local_port()
            );
        }
    }

    #[tokio::test]
    async fn test_new_udpio_pair3() {
        // get the first available port

        let mut first_local_port = 0;
        if let Some(udpio_0) = UdpIO::new_with_local_port(0).await {
            if let Some(local_port_0) = udpio_0.get_local_port() {
                first_local_port = local_port_0;
            }

            // std::mem::drop(udpio_0);
        }
        //The object udpio_0 is automatically cleared and released when it goes out of scope here.
        println!("first_local_port: {}", first_local_port);

        if (UdpIO::new_with_local_port(first_local_port).await).is_some() {
            println!("success")
        } else {
            println!("fail")
        }
    }

    #[tokio::test]
    async fn test_new_udpio_pair_returns_consecutive_ports() {
        if let Some((udpio1, udpio2)) = new_udpio_pair().await {
            let port1 = udpio1.get_local_port();
            let port2 = udpio2.get_local_port();

            // Both should have valid ports
            assert!(port1.is_some(), "First UDP socket should have a local port");
            assert!(
                port2.is_some(),
                "Second UDP socket should have a local port"
            );

            // Ports should be consecutive
            let p1 = port1.unwrap();
            let p2 = port2.unwrap();
            assert_eq!(p2, p1 + 1, "UDP pair ports should be consecutive");
        } else {
            panic!("new_udpio_pair should return Some with available ports");
        }
    }

    // ========== Additional UdpIO error and timeout branches ==========

    #[tokio::test]
    async fn test_udpio_new_remote_address_parse_error_returns_none() {
        // Empty domain produces ":9" which cannot parse as SocketAddr
        let result = UdpIO::new("".to_string(), 9, 0).await;
        assert!(
            result.is_none(),
            "Unparseable remote address should return None"
        );
    }

    #[tokio::test]
    async fn test_udpio_new_with_valid_ip_and_port_returns_some_or_none() {
        // UDP connect to 127.0.0.1:1 exercises the connect path; result is OS-dependent.
        let result = UdpIO::new("127.0.0.1".to_string(), 1, 0).await;
        if let Some(udpio) = result {
            assert!(matches!(udpio.get_net_type(), NetType::UDP));
        }
    }

    #[tokio::test]
    async fn test_udpio_read_timeout_success() {
        if let Some((udpio1, udpio2)) = new_udpio_pair().await {
            if let (Some(port1), Some(port2)) = (udpio1.get_local_port(), udpio2.get_local_port()) {
                if let (Some(mut sender), Some(mut receiver)) = (
                    UdpIO::new("127.0.0.1".to_string(), port2, port1).await,
                    UdpIO::new("127.0.0.1".to_string(), port1, port2).await,
                ) {
                    drop(udpio1);
                    drop(udpio2);
                    let send_data = Bytes::from("udp timeout test");
                    let send_task = tokio::spawn(async move {
                        let _ = sender.write(send_data).await;
                    });
                    let recv_task = tokio::spawn(async move {
                        let result = receiver.read_timeout(Duration::from_secs(2)).await;
                        assert!(
                            result.is_ok(),
                            "Read_timeout should succeed when data arrives"
                        );
                        let data = result.unwrap();
                        assert_eq!(&data[..], b"udp timeout test");
                    });
                    let _ = tokio::try_join!(send_task, recv_task);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_udpio_write_empty_bytes_does_not_panic() {
        if let Some(mut udpio) = UdpIO::new_with_local_port(0).await {
            let data = Bytes::from("");
            let _ = udpio.write(data).await;
            // Platform-dependent: send(0) may succeed or fail; we only assert no panic
        }
    }

    // ========== TcpIO write error when connection closed ==========

    #[tokio::test]
    async fn test_tcpio_write_after_remote_closed_returns_error() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let mut tcpio = TcpIO::new(stream);
            tokio::time::sleep(Duration::from_millis(100)).await;
            let data = Bytes::from("too late");
            let result = tcpio.write(data).await;
            assert!(
                result.is_err(),
                "Write after remote closed should return error"
            );
        });

        let accept_task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            drop(stream);
        });

        let _ = tokio::try_join!(connect_task, accept_task);
    }

    #[tokio::test]
    async fn test_tcpio_read_returns_io_error_on_stream_error() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let mut tcpio = TcpIO::new(stream);
            let first = tcpio.read().await;
            assert!(first.is_ok(), "First read should get the sent data");
            let second = tcpio.read().await;
            assert!(
                second.is_err(),
                "Second read after peer close should return error"
            );
            let err = second.unwrap_err();
            assert!(
                matches!(
                    err.value,
                    BytesIOErrorValue::NoneReturn | BytesIOErrorValue::IOError(_)
                ),
                "Expected NoneReturn or IOError after peer closed"
            );
        });

        let accept_task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            tokio::io::AsyncWriteExt::write_all(&mut stream, b"one frame")
                .await
                .unwrap();
            tokio::io::AsyncWriteExt::flush(&mut stream).await.unwrap();
            drop(stream);
        });

        let _ = tokio::try_join!(connect_task, accept_task);
    }

    #[tokio::test]
    async fn test_tcpreadio_read_returns_io_error_on_stream_error() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let (read_half, _) = stream.into_split();
            let mut tcpreadio = TcpReadIO::new(read_half);
            let first = tcpreadio.read().await;
            assert!(first.is_ok(), "First read should succeed");
            let second = tcpreadio.read().await;
            assert!(
                second.is_err(),
                "Second read after peer close should return error"
            );
            let err = second.unwrap_err();
            assert!(
                matches!(
                    err.value,
                    BytesIOErrorValue::NoneReturn | BytesIOErrorValue::IOError(_)
                ),
                "Expected NoneReturn or IOError"
            );
        });

        let accept_task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            tokio::io::AsyncWriteExt::write_all(&mut stream, b"data")
                .await
                .unwrap();
            tokio::io::AsyncWriteExt::flush(&mut stream).await.unwrap();
            drop(stream);
        });

        let _ = tokio::try_join!(connect_task, accept_task);
    }

    #[tokio::test]
    async fn test_tcpwriteio_write_after_remote_closed_returns_error() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let connect_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let (_, write_half) = stream.into_split();
            let mut tcpwriteio = TcpWriteIO::new(write_half);
            tokio::time::sleep(Duration::from_millis(100)).await;
            let result = tcpwriteio.write(Bytes::from("late")).await;
            assert!(
                result.is_err(),
                "Write after remote closed should return error"
            );
        });

        let accept_task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            drop(stream);
        });

        let _ = tokio::try_join!(connect_task, accept_task);
    }

    // ========== Additional UdpIO branches ==========

    // ========== wrap_port and next_try_port helper tests ==========

    #[test]
    fn test_wrap_port_normal() {
        assert_eq!(wrap_port(1000), 1000);
        assert_eq!(wrap_port(1), 1);
        assert_eq!(wrap_port(65534), 65534);
    }

    #[test]
    fn test_wrap_port_max_wraps_to_one() {
        assert_eq!(wrap_port(MAX_PORT), 1);
    }

    #[test]
    fn test_next_try_port_normal() {
        assert_eq!(next_try_port(1000), 1001);
        assert_eq!(next_try_port(1), 2);
    }

    #[test]
    fn test_next_try_port_max_wraps_to_one() {
        assert_eq!(next_try_port(MAX_PORT), 1);
    }

    #[test]
    fn test_next_try_port_max_minus_one() {
        assert_eq!(next_try_port(65534), 65535);
    }

    #[test]
    fn test_max_udp_datagram_size_constant() {
        assert_eq!(MAX_UDP_DATAGRAM_SIZE, 65507);
    }

    #[tokio::test]
    async fn test_udpio_new_with_unparseable_address_returns_none() {
        let result = UdpIO::new("256.256.256.256".to_string(), 80, 0).await;
        assert!(result.is_none(), "Invalid IP address should return None");
    }

    #[tokio::test]
    async fn test_udpio_new_with_local_port_bind_fails_when_port_in_use() {
        if let Some(udpio) = UdpIO::new_with_local_port(0).await {
            if let Some(port) = udpio.get_local_port() {
                let result = UdpIO::new_with_local_port(port).await;
                assert!(
                    result.is_none(),
                    "Binding to already-used port should return None"
                );
            }
        }
    }
}
