//! SNTP client, replacing the fire-and-forget `ntpd -n -N -p <server> &` in
//! `gergehack.sh:357`.
//!
//! This is not cosmetic. `onvif-rust`'s `ws_security.rs:85` sets
//! `clock_skew_seconds: 300` and `:234-239` rejects any WS-UsernameToken
//! `Created` timestamp outside +/- 5 minutes of now. A camera at the epoch
//! rejects **every** authenticated ONVIF request.
//!
//! The response is unauthenticated UDP from the network and its content sets
//! the system clock, so `parse_response` validates aggressively.

use crate::config::TimeCfg;
use crate::sys::Sys;
use std::io::Read;
use std::net::{ToSocketAddrs, UdpSocket};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Seconds between the NTP epoch (1900-01-01) and the Unix epoch.
pub const NTP_UNIX_OFFSET: u64 = 2_208_988_800;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NtpError {
    #[error("not a server reply (mode {0})")]
    BadMode(u8),
    #[error("server reports itself unsynchronised (LI=3)")]
    Unsynchronised,
    #[error("unusable stratum {0}")]
    BadStratum(u8),
    #[error("originate timestamp does not echo our nonce")]
    NonceMismatch,
    #[error("transmit timestamp is zero")]
    ZeroTimestamp,
    #[error("implausible time: unix {0}")]
    Implausible(u64),
}

#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub min_unix: u64,
    pub max_unix: u64,
}

fn be64(pkt: &[u8; 48], off: usize) -> u64 {
    // Offsets are compile-time constants inside a fixed-size array, so this
    // slice is always exactly 8 bytes.
    let mut b = [0u8; 8];
    b.copy_from_slice(&pkt[off..off + 8]);
    u64::from_be_bytes(b)
}

/// Build a client request. `nonce` goes in the transmit-timestamp field; a
/// conformant server echoes it verbatim into the originate field, which is how
/// `parse_response` rejects off-path spoofing.
pub fn build_request(nonce: u64) -> [u8; 48] {
    let mut p = [0u8; 48];
    p[0] = 0b00_100_011; // LI=0, VN=4, Mode=3 (client)
    p[40..48].copy_from_slice(&nonce.to_be_bytes());
    p
}

pub fn parse_response(
    pkt: &[u8; 48],
    sent_nonce: u64,
    bounds: &Bounds,
) -> Result<SystemTime, NtpError> {
    let li = pkt[0] >> 6;
    let mode = pkt[0] & 0b111;
    if mode != 4 {
        return Err(NtpError::BadMode(mode));
    }
    if li == 3 {
        return Err(NtpError::Unsynchronised);
    }
    let stratum = pkt[1];
    if stratum == 0 || stratum > 15 {
        return Err(NtpError::BadStratum(stratum));
    }
    if be64(pkt, 24) != sent_nonce {
        return Err(NtpError::NonceMismatch);
    }
    let transmit = be64(pkt, 40);
    if transmit == 0 {
        return Err(NtpError::ZeroTimestamp);
    }

    let secs_1900 = transmit >> 32;
    let unix = secs_1900
        .checked_sub(NTP_UNIX_OFFSET)
        .ok_or(NtpError::Implausible(0))?;
    if unix < bounds.min_unix || unix > bounds.max_unix {
        return Err(NtpError::Implausible(unix));
    }

    let frac_nanos = (((transmit & 0xFFFF_FFFF) * 1_000_000_000) >> 32) as u32;
    Ok(UNIX_EPOCH + Duration::new(unix, frac_nanos))
}

/// Read a 64-bit nonce from `/dev/urandom`. Falls back to a monotonic-derived
/// value if unavailable — weaker, but the alternative is skipping the
/// anti-spoofing check entirely.
pub fn random_nonce() -> u64 {
    let mut buf = [0u8; 8];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom")
        && f.read_exact(&mut buf).is_ok()
    {
        return u64::from_be_bytes(buf);
    }
    std::time::Instant::now().elapsed().as_nanos() as u64 ^ 0x0005_DEEC_E66D_u64
}

pub fn query(server: &str, timeout: Duration, bounds: &Bounds) -> anyhow::Result<SystemTime> {
    let addr = format!("{server}:123")
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow::anyhow!("no address for {server}"))?;

    let sock = UdpSocket::bind("0.0.0.0:0")?;
    sock.set_read_timeout(Some(timeout))?;
    sock.set_write_timeout(Some(timeout))?;

    let nonce = random_nonce();
    sock.send_to(&build_request(nonce), addr)?;

    let mut buf = [0u8; 48];
    let (n, from) = sock.recv_from(&mut buf)?;
    if n != 48 {
        anyhow::bail!("short NTP reply: {n} bytes");
    }
    if from.ip() != addr.ip() {
        anyhow::bail!("reply from {} but queried {}", from.ip(), addr.ip());
    }
    Ok(parse_response(&buf, nonce, bounds)?)
}

/// Query each configured server in turn; step the clock on the first success.
/// Returns the applied delta in seconds, or `None` if nothing was applied.
pub fn sync_once(sys: &dyn Sys, cfg: &TimeCfg) -> Option<i64> {
    let bounds = Bounds {
        min_unix: cfg.min_plausible_unix,
        max_unix: cfg.max_plausible_unix,
    };
    for server in &cfg.servers {
        match query(server, Duration::from_secs(5), &bounds) {
            Ok(t) => {
                let before = sys.realtime();
                let delta = delta_secs(before, t);
                if delta.unsigned_abs() < cfg.step_threshold_sec {
                    tracing::debug!(server, delta, "clock already within threshold");
                    return Some(0);
                }
                match sys.set_realtime(t) {
                    Ok(()) => {
                        tracing::info!(server, delta_sec = delta, "stepped system clock");
                        return Some(delta);
                    }
                    Err(e) => tracing::error!(server, error = %e, "clock_settime failed"),
                }
            }
            Err(e) => tracing::warn!(server, error = %e, "NTP query failed"),
        }
    }
    None
}

fn delta_secs(from: SystemTime, to: SystemTime) -> i64 {
    match to.duration_since(from) {
        Ok(d) => d.as_secs() as i64,
        Err(e) => -(e.duration().as_secs() as i64),
    }
}

/// P2.5: bounded best-effort first sync. Never blocks boot beyond
/// `first_sync_timeout_sec`.
pub fn first_sync(sys: &dyn Sys, cfg: &TimeCfg) -> bool {
    if !cfg.enabled {
        return false;
    }
    let deadline = sys.now() + Duration::from_secs(cfg.first_sync_timeout_sec);
    loop {
        if sync_once(sys, cfg).is_some() {
            return true;
        }
        if sys.now() >= deadline {
            tracing::warn!(
                timeout_sec = cfg.first_sync_timeout_sec,
                "no NTP sync before boot deadline; continuing with a wrong clock. \
                 Authenticated ONVIF requests will fail until the resync thread succeeds."
            );
            return false;
        }
        // Bounded by `deadline` above, so a retry_interval longer than the
        // timeout simply means one attempt.
        std::thread::sleep(Duration::from_secs(cfg.retry_interval_sec.min(2)));
    }
}

/// Background resync loop, started after P3.
pub fn resync_loop(sys: &dyn Sys, cfg: &TimeCfg) {
    // Until the clock has been set once, retry at `retry_interval_sec`, not
    // `resync_interval_sec`. P2.5 gives up after 15s so that boot is not held
    // hostage to the network, which means a slow wifi association routinely
    // lands here with the clock still at the epoch. Sleeping the full 6h resync
    // interval first would leave ws_security.rs:85 (clock_skew_seconds = 300)
    // rejecting every authenticated ONVIF request for those 6 hours.
    let mut synced = false;
    loop {
        std::thread::sleep(Duration::from_secs(resync_wait_secs(synced, cfg)));
        if sync_once(sys, cfg).is_some() {
            synced = true;
        }
    }
}

/// How long `resync_loop` waits before its next attempt.
///
/// Split out so the fast-retry-until-first-success rule is testable without
/// running the loop.
pub fn resync_wait_secs(synced: bool, cfg: &TimeCfg) -> u64 {
    if synced {
        cfg.resync_interval_sec
    } else {
        cfg.retry_interval_sec
    }
}

#[cfg(test)]
mod parse_tests {
    use super::*;

    const NONCE: u64 = 0xDEAD_BEEF_CAFE_F00D;
    /// 2026-08-01T00:00:00Z as NTP seconds (unix + 2_208_988_800).
    const GOOD_NTP_SECS: u64 = 1_785_542_400 + NTP_UNIX_OFFSET;

    fn bounds() -> Bounds {
        Bounds {
            min_unix: 1_767_225_600,
            max_unix: 2_524_608_000,
        }
    }

    /// A well-formed server reply: LI=0, VN=4, Mode=4, stratum 2.
    fn good_packet() -> [u8; 48] {
        let mut p = [0u8; 48];
        p[0] = 0b00_100_100;
        p[1] = 2;
        p[24..32].copy_from_slice(&NONCE.to_be_bytes());
        p[40..48].copy_from_slice(&(GOOD_NTP_SECS << 32).to_be_bytes());
        p
    }

    #[test]
    fn test_parse_accepts_well_formed_reply() {
        let t = parse_response(&good_packet(), NONCE, &bounds()).expect("must accept");
        let unix = t
            .duration_since(std::time::UNIX_EPOCH)
            .expect("post-epoch")
            .as_secs();
        assert_eq!(unix, 1_785_542_400);
    }

    #[test]
    fn test_parse_rejects_non_server_mode() {
        let mut p = good_packet();
        p[0] = 0b00_100_011; // mode 3 = client
        assert_eq!(
            parse_response(&p, NONCE, &bounds()),
            Err(NtpError::BadMode(3))
        );
    }

    #[test]
    fn test_parse_rejects_leap_indicator_alarm() {
        let mut p = good_packet();
        p[0] |= 0b11_000_000; // LI = 3, server says it is unsynchronised
        assert_eq!(
            parse_response(&p, NONCE, &bounds()),
            Err(NtpError::Unsynchronised)
        );
    }

    #[test]
    fn test_parse_rejects_kiss_of_death_stratum_zero() {
        let mut p = good_packet();
        p[1] = 0;
        assert_eq!(
            parse_response(&p, NONCE, &bounds()),
            Err(NtpError::BadStratum(0))
        );
    }

    #[test]
    fn test_parse_rejects_unsynchronised_stratum_16() {
        let mut p = good_packet();
        p[1] = 16;
        assert_eq!(
            parse_response(&p, NONCE, &bounds()),
            Err(NtpError::BadStratum(16))
        );
    }

    #[test]
    fn test_parse_rejects_nonce_mismatch() {
        // This is the anti-spoofing check. Without it any host that can guess
        // our query can set the camera's clock.
        let p = good_packet();
        assert_eq!(
            parse_response(&p, NONCE ^ 1, &bounds()),
            Err(NtpError::NonceMismatch)
        );
    }

    #[test]
    fn test_parse_rejects_zero_transmit_timestamp() {
        let mut p = good_packet();
        p[40..48].copy_from_slice(&0u64.to_be_bytes());
        assert_eq!(
            parse_response(&p, NONCE, &bounds()),
            Err(NtpError::ZeroTimestamp)
        );
    }

    #[test]
    fn test_parse_rejects_time_before_lower_bound() {
        let mut p = good_packet();
        // 2000-01-01, well before min_plausible.
        let secs = 946_684_800u64 + NTP_UNIX_OFFSET;
        p[40..48].copy_from_slice(&(secs << 32).to_be_bytes());
        assert!(matches!(
            parse_response(&p, NONCE, &bounds()),
            Err(NtpError::Implausible(_))
        ));
    }

    #[test]
    fn test_parse_rejects_time_after_upper_bound() {
        let mut p = good_packet();
        let secs = 4_000_000_000u64 + NTP_UNIX_OFFSET;
        p[40..48].copy_from_slice(&(secs << 32).to_be_bytes());
        assert!(matches!(
            parse_response(&p, NONCE, &bounds()),
            Err(NtpError::Implausible(_))
        ));
    }

    #[test]
    fn test_parse_rejects_pre_1900_timestamp_without_panicking() {
        let mut p = good_packet();
        p[40..48].copy_from_slice(&(1u64 << 32).to_be_bytes());
        assert!(matches!(
            parse_response(&p, NONCE, &bounds()),
            Err(NtpError::Implausible(_))
        ));
    }
}

#[cfg(test)]
mod delta_tests {
    use super::*;

    #[test]
    fn test_delta_secs_positive_when_target_is_later() {
        let a = UNIX_EPOCH + Duration::from_secs(1000);
        let b = UNIX_EPOCH + Duration::from_secs(1090);
        assert_eq!(delta_secs(a, b), 90);
    }

    #[test]
    fn test_delta_secs_negative_when_target_is_earlier() {
        let a = UNIX_EPOCH + Duration::from_secs(1090);
        let b = UNIX_EPOCH + Duration::from_secs(1000);
        assert_eq!(delta_secs(a, b), -90);
    }

    #[test]
    fn test_resync_waits_retry_interval_until_first_success() {
        let cfg = crate::config::TimeCfg::default();
        assert_eq!(
            resync_wait_secs(false, &cfg),
            cfg.retry_interval_sec,
            "before the first sync the clock is wrong and ONVIF auth is down; \
             retry fast, not once per resync interval"
        );
        assert_eq!(resync_wait_secs(true, &cfg), cfg.resync_interval_sec);
        assert!(cfg.retry_interval_sec < cfg.resync_interval_sec);
    }
}
