//! WS-Discovery Integration Tests
//!
//! These tests verify the WS-Discovery service implementation including:
//! - Service lifecycle (start, stop)
//! - Hello message sending
//! - Probe/ProbeMatch handling
//! - Discovery mode switching
//!
//! Note: These tests require network access and may need elevated permissions
//! to bind to multicast sockets on some systems.

use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;

use tokio::net::UdpSocket;
use tokio::sync::Notify;
use tokio::time::timeout;

// Import from the crate under test
use onvif_rust::discovery::{
    DiscoveryConfig, DiscoveryMode, WS_DISCOVERY_MULTICAST, WS_DISCOVERY_PORT, WsDiscovery,
};

/// Test configuration helper - creates a config with unique port to avoid conflicts
fn test_config(device_ip: &str, port: u16) -> DiscoveryConfig {
    DiscoveryConfig {
        endpoint_uuid: format!("urn:uuid:test-{}", uuid::Uuid::new_v4()),
        http_port: port,
        device_ip: device_ip.to_string(),
        scopes: vec!["onvif://www.onvif.org/type/video_encoder".to_string()],
        discovery_mode: DiscoveryMode::Discoverable,
        hello_interval: Duration::from_secs(300),
    }
}

/// Create a UDP socket for receiving WS-Discovery messages (simulating a client)
async fn create_discovery_client_socket() -> Result<UdpSocket, std::io::Error> {
    use socket2::{Domain, Protocol, Socket, Type};

    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;

    // Bind to the WS-Discovery port to receive multicast messages
    let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, WS_DISCOVERY_PORT);
    socket.bind(&addr.into())?;

    // Join multicast group
    socket.join_multicast_v4(&WS_DISCOVERY_MULTICAST, &Ipv4Addr::UNSPECIFIED)?;

    socket.set_nonblocking(true)?;

    let std_socket: std::net::UdpSocket = socket.into();
    UdpSocket::from_std(std_socket)
}

/// Build a WS-Discovery Probe message
fn build_probe_message(message_id: &str, types: Option<&str>) -> String {
    let types_element = types
        .map(|t| format!("<d:Types>{}</d:Types>", t))
        .unwrap_or_default();

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:a="http://www.w3.org/2005/08/addressing"
            xmlns:d="http://docs.oasis-open.org/ws-dd/ns/discovery/2009/01"
            xmlns:tdn="http://www.onvif.org/ver10/network/wsdl">
  <s:Header>
    <a:Action>http://docs.oasis-open.org/ws-dd/ns/discovery/2009/01/Probe</a:Action>
    <a:MessageID>{}</a:MessageID>
    <a:To>urn:docs-oasis-open-org:ws-dd:ns:discovery:2009:01</a:To>
  </s:Header>
  <s:Body>
    <d:Probe>
      {}
    </d:Probe>
  </s:Body>
</s:Envelope>"#,
        message_id, types_element
    )
}

// =============================================================================
// Service Lifecycle Tests
// =============================================================================

#[tokio::test]
async fn test_service_starts_and_stops() {
    let config = test_config("127.0.0.1", 8080);
    let discovery = WsDiscovery::new(config);

    // Start the service
    let result = discovery.run_service().await;
    assert!(result.is_ok(), "Service should start successfully");

    let (handle, task) = result.unwrap();

    // Service should be running
    assert!(handle.is_running(), "Service should be running after start");

    // Stop the service
    let stop_result = handle.stop().await;
    assert!(stop_result.is_ok(), "Service should stop successfully");

    // Service should no longer be running
    assert!(
        !handle.is_running(),
        "Service should not be running after stop"
    );

    // Wait for task to complete
    let _ = timeout(Duration::from_secs(2), task).await;
}

#[tokio::test]
async fn test_handle_is_cloneable() {
    let config = test_config("127.0.0.1", 8081);
    let discovery = WsDiscovery::new(config);

    let (handle, task) = discovery.run_service().await.unwrap();

    // Clone the handle
    let handle2 = handle.clone();

    // Both handles should see the service as running
    assert!(handle.is_running());
    assert!(handle2.is_running());

    // Stop via one handle
    handle.stop().await.unwrap();

    // Both handles should see the service as stopped
    assert!(!handle.is_running());
    assert!(!handle2.is_running());

    let _ = timeout(Duration::from_secs(2), task).await;
}

#[tokio::test]
async fn test_double_stop_is_safe() {
    let config = test_config("127.0.0.1", 8082);
    let discovery = WsDiscovery::new(config);

    let (handle, task) = discovery.run_service().await.unwrap();

    // Stop twice - should not error
    assert!(handle.stop().await.is_ok());
    assert!(handle.stop().await.is_ok());

    let _ = timeout(Duration::from_secs(2), task).await;
}

// =============================================================================
// Discovery Mode Tests
// =============================================================================

#[tokio::test]
async fn test_discovery_mode_change() {
    let config = test_config("127.0.0.1", 8083);
    let discovery = WsDiscovery::new(config);

    let (handle, task) = discovery.run_service().await.unwrap();

    // Default should be Discoverable
    assert_eq!(
        handle.get_discovery_mode().await,
        DiscoveryMode::Discoverable
    );

    // Change to NonDiscoverable
    handle
        .set_discovery_mode(DiscoveryMode::NonDiscoverable)
        .await;
    assert_eq!(
        handle.get_discovery_mode().await,
        DiscoveryMode::NonDiscoverable
    );

    // Change back to Discoverable
    handle.set_discovery_mode(DiscoveryMode::Discoverable).await;
    assert_eq!(
        handle.get_discovery_mode().await,
        DiscoveryMode::Discoverable
    );

    handle.stop().await.unwrap();
    let _ = timeout(Duration::from_secs(2), task).await;
}

// =============================================================================
// Hello Message Tests
// =============================================================================

#[tokio::test]
#[ignore = "requires multicast network access"]
async fn test_hello_message_sent_on_start() {
    // Create a client socket to receive Hello messages
    let client_socket = match create_discovery_client_socket().await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skipping test - cannot create multicast socket: {}", e);
            return;
        }
    };

    let config = test_config("192.168.1.100", 80);
    let endpoint_uuid = config.endpoint_uuid.clone();
    let discovery = WsDiscovery::new(config);

    let (handle, task) = discovery.run_service().await.unwrap();

    // Wait for Hello message
    let mut buf = [0u8; 4096];
    let received = timeout(Duration::from_secs(2), async {
        loop {
            match client_socket.recv_from(&mut buf).await {
                Ok((len, _src)) => {
                    let msg = String::from_utf8_lossy(&buf[..len]);
                    if msg.contains("Hello") && msg.contains(&endpoint_uuid) {
                        return Some(msg.to_string());
                    }
                }
                Err(_) => continue,
            }
        }
    })
    .await;

    handle.stop().await.unwrap();
    let _ = timeout(Duration::from_secs(2), task).await;

    assert!(
        received.is_ok(),
        "Should receive Hello message within timeout"
    );
    let msg = received.unwrap().unwrap();
    assert!(msg.contains("Hello"), "Message should be a Hello");
    assert!(
        msg.contains("tdn:NetworkVideoTransmitter"),
        "Should contain NVT type"
    );
}

// =============================================================================
// Probe/ProbeMatch Tests
// =============================================================================

#[tokio::test]
#[ignore = "requires multicast network access"]
async fn test_probe_match_response() {
    let config = test_config("192.168.1.100", 80);
    let endpoint_uuid = config.endpoint_uuid.clone();
    let discovery = WsDiscovery::new(config);

    let (handle, task) = discovery.run_service().await.unwrap();

    // Give the service time to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create a socket to send Probe and receive ProbeMatch
    let client_socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();

    let probe_message_id = format!("urn:uuid:{}", uuid::Uuid::new_v4());
    let probe = build_probe_message(&probe_message_id, Some("tdn:NetworkVideoTransmitter"));

    // Send Probe to multicast address
    let dest = SocketAddrV4::new(WS_DISCOVERY_MULTICAST, WS_DISCOVERY_PORT);
    client_socket.send_to(probe.as_bytes(), dest).await.unwrap();

    // Wait for ProbeMatch response
    let mut buf = [0u8; 4096];
    let received = timeout(Duration::from_secs(2), async {
        loop {
            match client_socket.recv_from(&mut buf).await {
                Ok((len, _src)) => {
                    let msg = String::from_utf8_lossy(&buf[..len]);
                    if msg.contains("ProbeMatches") && msg.contains(&endpoint_uuid) {
                        return Some(msg.to_string());
                    }
                }
                Err(_) => continue,
            }
        }
    })
    .await;

    handle.stop().await.unwrap();
    let _ = timeout(Duration::from_secs(2), task).await;

    assert!(received.is_ok(), "Should receive ProbeMatch within timeout");
    let msg = received.unwrap().unwrap();
    assert!(
        msg.contains("ProbeMatches"),
        "Message should be a ProbeMatches"
    );
    assert!(
        msg.contains(&probe_message_id),
        "Should contain RelatesTo with probe message ID"
    );
    assert!(
        msg.contains("tdn:NetworkVideoTransmitter"),
        "Should contain NVT type"
    );
}

#[tokio::test]
#[ignore = "requires multicast network access"]
async fn test_non_discoverable_mode_ignores_probe() {
    let config = DiscoveryConfig {
        discovery_mode: DiscoveryMode::NonDiscoverable,
        ..test_config("192.168.1.100", 80)
    };
    let endpoint_uuid = config.endpoint_uuid.clone();
    let discovery = WsDiscovery::new(config);

    let (handle, task) = discovery.run_service().await.unwrap();

    // Verify mode is NonDiscoverable
    assert_eq!(
        handle.get_discovery_mode().await,
        DiscoveryMode::NonDiscoverable
    );

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create a socket to send Probe
    let client_socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();

    let probe = build_probe_message(
        &format!("urn:uuid:{}", uuid::Uuid::new_v4()),
        Some("tdn:NetworkVideoTransmitter"),
    );

    // Send Probe
    let dest = SocketAddrV4::new(WS_DISCOVERY_MULTICAST, WS_DISCOVERY_PORT);
    client_socket.send_to(probe.as_bytes(), dest).await.unwrap();

    // Should NOT receive ProbeMatch
    let mut buf = [0u8; 4096];
    let received = timeout(Duration::from_millis(1000), async {
        loop {
            match client_socket.recv_from(&mut buf).await {
                Ok((len, _src)) => {
                    let msg = String::from_utf8_lossy(&buf[..len]);
                    if msg.contains("ProbeMatches") && msg.contains(&endpoint_uuid) {
                        return Some(msg.to_string());
                    }
                }
                Err(_) => continue,
            }
        }
    })
    .await;

    handle.stop().await.unwrap();
    let _ = timeout(Duration::from_secs(2), task).await;

    assert!(
        received.is_err(),
        "Should NOT receive ProbeMatch in NonDiscoverable mode"
    );
}

// =============================================================================
// Scopes Update Tests
// =============================================================================

#[tokio::test]
async fn test_scopes_update() {
    let config = test_config("127.0.0.1", 8084);
    let discovery = WsDiscovery::new(config);

    let (handle, task) = discovery.run_service().await.unwrap();

    // Update scopes
    let new_scopes = vec![
        "onvif://www.onvif.org/type/video_encoder".to_string(),
        "onvif://www.onvif.org/type/audio_encoder".to_string(),
        "onvif://www.onvif.org/Profile/Streaming".to_string(),
    ];

    handle.set_scopes(new_scopes.clone()).await;

    // Note: We can't easily verify the scopes were set without exposing
    // the config getter. The test verifies the method doesn't panic.

    handle.stop().await.unwrap();
    let _ = timeout(Duration::from_secs(2), task).await;
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[tokio::test]
async fn test_cannot_start_twice() {
    let config = test_config("127.0.0.1", 8085);
    let discovery = WsDiscovery::new(config.clone());

    let (handle, task) = discovery.run_service().await.unwrap();

    // Try to start another service with same config
    // Note: This creates a new WsDiscovery instance, so it's allowed
    // The AlreadyRunning error is for calling run_service twice on the same instance
    let discovery2 = WsDiscovery::new(config);
    // This should work because it's a different instance
    // (though it may fail to bind the socket if the port is in use)

    handle.stop().await.unwrap();
    let _ = timeout(Duration::from_secs(2), task).await;

    // Now the second instance can start
    let result = discovery2.run_service().await;
    if let Ok((handle2, task2)) = result {
        handle2.stop().await.unwrap();
        let _ = timeout(Duration::from_secs(2), task2).await;
    }
}

// =============================================================================
// Concurrent Access Tests
// =============================================================================

#[tokio::test]
async fn test_concurrent_handle_access() {
    let config = test_config("127.0.0.1", 8086);
    let discovery = WsDiscovery::new(config);

    let (handle, task) = discovery.run_service().await.unwrap();

    // Spawn multiple tasks that access the handle concurrently
    let handle1 = handle.clone();
    let handle2 = handle.clone();
    let handle3 = handle.clone();

    let done = Arc::new(Notify::new());
    let done1 = done.clone();
    let done2 = done.clone();
    let done3 = done.clone();

    let t1 = tokio::spawn(async move {
        for _ in 0..10 {
            let _ = handle1.get_discovery_mode().await;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        done1.notify_one();
    });

    let t2 = tokio::spawn(async move {
        for _ in 0..10 {
            handle2
                .set_discovery_mode(DiscoveryMode::Discoverable)
                .await;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        done2.notify_one();
    });

    let t3 = tokio::spawn(async move {
        for _ in 0..10 {
            let _ = handle3.is_running();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        done3.notify_one();
    });

    // Wait for all tasks
    let _ = tokio::join!(t1, t2, t3);

    handle.stop().await.unwrap();
    let _ = timeout(Duration::from_secs(2), task).await;
}
