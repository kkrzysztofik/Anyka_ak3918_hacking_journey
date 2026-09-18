use snmp_agent::server::{self, DEFAULT_PIDFILE};
use std::path::PathBuf;
use tokio::signal::unix::{SignalKind, signal};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt()
        .with_timer(LocalTimer)
        .with_ansi(false)
        .init();
    let config_path = server::parse_args(std::env::args());
    tracing::info!(?config_path, "snmp-agent starting");

    let (tx, rx) = tokio::sync::mpsc::channel(1);
    match signal(SignalKind::hangup()) {
        Ok(mut sighup) => {
            tokio::spawn(async move {
                while sighup.recv().await.is_some() {
                    // Depth 1: a reload already queued subsumes this one.
                    let _ = tx.try_send(());
                }
            });
        }
        Err(e) => tracing::error!(error = %e, "SIGHUP unavailable; config reload disabled"),
    }

    if let Err(e) = server::run(config_path, PathBuf::from(DEFAULT_PIDFILE), rx).await {
        tracing::error!(error = %e, "snmp-agent exited");
        std::process::exit(1);
    }
}

/// Timestamps log lines in the process's configured zone (the `TZ` env var).
///
/// Duplicated from `anyka-init/src/logging.rs` (no shared crate for logging
/// helpers): libc `localtime_r` + `strftime`, with the microsecond and offset
/// parts assembled by hand so it works on C libraries without the `%f`/`%:z`
/// extensions (the camera's libc generation is unknown).
pub struct LocalTimer;

impl LocalTimer {
    /// Write an RFC3339-style timestamp in the current zone; returns length.
    pub fn format_now(buf: &mut [u8]) -> usize {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs() as i64; // time_t
        let mut zeroed: libc::tm = unsafe { std::mem::zeroed() };
        // glibc caches the parsed zone and does not re-read a changed TZ env
        // var on its own (verified on glibc 2.43); tzset() forces the reparse.
        // Declared locally: the libc crate only exposes tzset for Windows.
        // SAFETY: tzset reads the process env and updates libc's own state;
        // it is async-signal-safe and cannot fail.
        unsafe extern "C" {
            fn tzset();
        }
        unsafe { tzset() };
        // SAFETY: localtime_r writes into the caller-provided struct and
        // never reads it uninitialized; it returns null on a broken clock,
        // in which case the zeroed struct (1970) is used instead of a deref.
        let ptr = unsafe { libc::localtime_r(&secs, &mut zeroed) };
        let tm = if ptr.is_null() {
            &zeroed
        } else {
            unsafe { &*ptr }
        };
        let mut out = [0u8; 20];
        // SAFETY: `out` fits "YYYY-MM-DDTHH:MM:SS" (19 chars + NUL) and the
        // format string is a NUL-terminated C string literal.
        let n = unsafe {
            libc::strftime(
                out.as_mut_ptr().cast::<libc::c_char>(),
                out.len(),
                c"%Y-%m-%dT%H:%M:%S".as_ptr(),
                tm,
            )
        };
        let micros = now.subsec_micros();
        let offset = tm.tm_gmtoff;
        let mut s = String::from_utf8_lossy(&out[..n]).into_owned();
        s.push_str(&format!(".{micros:06}"));
        s.push(if offset < 0 { '-' } else { '+' });
        let abs = offset.unsigned_abs();
        s.push_str(&format!("{:02}:{:02}", abs / 3600, (abs % 3600) / 60));
        let len = s.len().min(buf.len());
        buf[..len].copy_from_slice(s.as_bytes()[..len].try_into().unwrap_or(&[0u8; 0]));
        len
    }
}

impl tracing_subscriber::fmt::time::FormatTime for LocalTimer {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        let mut buf = [0u8; 64];
        let n = Self::format_now(&mut buf);
        write!(w, "{}", String::from_utf8_lossy(&buf[..n]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_timer_uses_the_process_tz() {
        // set_var here touches process-global libc TZ state; restore UTC when
        // done so no later test in this binary sees a shifted zone.
        unsafe { std::env::set_var("TZ", "UTC") };
        let mut buf = [0u8; 64];
        let n = LocalTimer::format_now(&mut buf);
        let s = String::from_utf8_lossy(&buf[..n]);
        assert!(!s.ends_with("Z"), "expected an offset, got {s}");
        unsafe { std::env::set_var("TZ", "CET-1CEST,M3.5.0,M10.5.0/3") };
        let n2 = LocalTimer::format_now(&mut buf);
        let s2 = String::from_utf8_lossy(&buf[..n2]);
        assert!(s2.contains("+01") || s2.contains("+02"), "got {s2}");
        unsafe { std::env::set_var("TZ", "UTC") };
    }
}
