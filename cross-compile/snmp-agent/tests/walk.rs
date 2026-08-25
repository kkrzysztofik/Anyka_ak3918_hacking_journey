//! End-to-end walk against a running agent. No net-snmp dependency, so it gates CI.

use snmp_agent::ber::Oid;
use snmp_agent::pdu::{Pdu, PduType, SNMP_V2C_VERSION, SnmpMessage, SnmpValue, VarBind};
use std::time::Duration;
use tokio::net::UdpSocket;

async fn spawn_agent() -> (u16, tokio::task::JoinHandle<()>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("snmp.toml");
    let pid = dir.path().join("agent.pid");

    let probe = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);
    std::fs::write(
        &cfg,
        format!("enabled = true\nport = {port}\ncommunity = \"public\"\nsys_name = \"walk-cam\"\n"),
    )
    .unwrap();

    let (_tx, rx) = tokio::sync::mpsc::channel(1);
    let handle = tokio::spawn(async move {
        let _ = snmp_agent::server::run(cfg, pid, rx).await;
    });
    tokio::time::sleep(Duration::from_millis(150)).await;
    (port, handle, dir)
}

fn request(pdu_type: PduType, oid: Oid) -> Vec<u8> {
    SnmpMessage {
        version: SNMP_V2C_VERSION,
        community: "public".into(),
        pdu: Pdu {
            pdu_type,
            request_id: 42,
            error_status: 0,
            error_index: 0,
            variable_bindings: vec![VarBind {
                name: oid,
                value: SnmpValue::Null,
            }],
        },
    }
    .encode()
    .unwrap()
}

async fn ask(sock: &UdpSocket, port: u16, bytes: &[u8]) -> Option<SnmpMessage> {
    sock.send_to(bytes, ("127.0.0.1", port)).await.unwrap();
    let mut buf = [0u8; 2048];
    match tokio::time::timeout(Duration::from_millis(600), sock.recv_from(&mut buf)).await {
        Ok(Ok((n, _))) => Some(SnmpMessage::parse(&buf[..n]).expect("agent must emit valid BER")),
        _ => None,
    }
}

#[tokio::test]
async fn test_full_walk_is_ordered_and_terminates() {
    let (port, handle, _dir) = spawn_agent().await;
    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let mut cursor = Oid::from_slice(&[1, 3]).unwrap();
    let mut seen: Vec<(Oid, SnmpValue)> = Vec::new();
    for _ in 0..500 {
        let resp = ask(&client, port, &request(PduType::GetNextRequest, cursor.clone()))
            .await
            .expect("GETNEXT must be answered");
        assert_eq!(resp.pdu.error_status, 0, "v2c reports misses in the varbind");
        let vb = resp.pdu.variable_bindings.into_iter().next().unwrap();
        if vb.value == SnmpValue::EndOfMibView {
            break;
        }
        assert!(
            cursor.0 < vb.name.0,
            "walk must strictly ascend: {:?} -> {:?}",
            cursor.0,
            vb.name.0
        );
        cursor = vb.name.clone();
        seen.push((vb.name, vb.value));
    }

    assert!(seen.len() >= 7, "at least the system group should be present");
    assert_eq!(
        seen[0].0 .0,
        vec![1, 3, 6, 1, 2, 1, 1, 1, 0],
        "walk starts at sysDescr.0"
    );
    assert!(
        seen.iter()
            .any(|(o, _)| o.0.starts_with(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 8])),
        "ifOperStatus column must be walked"
    );

    handle.abort();
}

#[tokio::test]
async fn test_unknown_oid_set_and_response_handling() {
    let (port, handle, _dir) = spawn_agent().await;
    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    // Unknown OID: noError plus an exception in the varbind.
    let resp = ask(
        &client,
        port,
        &request(
            PduType::GetRequest,
            Oid::from_slice(&[1, 3, 6, 1, 2, 1, 99, 1, 0]).unwrap(),
        ),
    )
    .await
    .expect("GET must be answered");
    assert_eq!(resp.pdu.error_status, 0);
    assert_eq!(resp.pdu.variable_bindings[0].value, SnmpValue::NoSuchObject);

    // SET is refused and changes nothing.
    let resp = ask(
        &client,
        port,
        &request(
            PduType::SetRequest,
            Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 5, 0]).unwrap(),
        ),
    )
    .await
    .expect("SET must be answered");
    assert_eq!(resp.pdu.error_status, 17, "notWritable");
    let resp = ask(
        &client,
        port,
        &request(
            PduType::GetRequest,
            Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 5, 0]).unwrap(),
        ),
    )
    .await
    .unwrap();
    assert_eq!(
        resp.pdu.variable_bindings[0].value,
        SnmpValue::OctetString(b"walk-cam".to_vec())
    );

    // A response PDU draws silence — otherwise a spoofed source loops us forever.
    let mut bytes = request(
        PduType::GetRequest,
        Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 1, 0]).unwrap(),
    );
    let i = bytes.iter().position(|&b| b == 0xa0).unwrap();
    bytes[i] = 0xa2;
    assert!(
        ask(&client, port, &bytes).await.is_none(),
        "must not answer a GetResponse"
    );

    // Wrong community draws silence too.
    let mut wrong = request(
        PduType::GetRequest,
        Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 1, 0]).unwrap(),
    );
    let c = wrong.windows(6).position(|w| w == b"public").unwrap();
    wrong[c] = b'X';
    assert!(
        ask(&client, port, &wrong).await.is_none(),
        "must not answer a bad community"
    );

    handle.abort();
}
