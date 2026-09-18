use snmp_agent::server::{self, DEFAULT_PIDFILE};
use std::path::PathBuf;
use tokio::signal::unix::{SignalKind, signal};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Parse TZ once. glibc caches the zone and will not re-read a changed TZ
    // on its own; the supervisor's zone never changes after boot.
    // Declared locally: the libc crate exposes tzset only on Windows.
    // SAFETY: tzset reads the process env and updates libc's own state.
    unsafe extern "C" {
        fn tzset();
    }
    unsafe { tzset() };

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

/// Timestamps log lines in the process's zone, which `boot.rs` sets from `TZ`.
///
/// Duplicated from `anyka-init/src/logging.rs` (no shared crate for logging
/// helpers): libc `localtime_r` + `strftime`. `%z` is standard C89 strftime
/// (`+0100`); only `%:z` is a GNU extension, so no hand-assembly is needed.
pub struct LocalTimer;

impl tracing_subscriber::fmt::time::FormatTime for LocalTimer {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // time_t is 32-bit on the camera's uclibc, 64-bit on host glibc.
        let t: libc::time_t = secs.try_into().unwrap_or(libc::time_t::MAX);
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        let mut out = [0u8; 32];
        // SAFETY: localtime_r fills our own `tm` and never reads it
        // uninitialised; on a broken clock it returns null and the zeroed
        // struct (1970) is formatted instead. strftime bounds its write by
        // out.len() and the format string is a NUL-terminated literal.
        let n = unsafe {
            libc::localtime_r(&t, &mut tm);
            libc::strftime(
                out.as_mut_ptr().cast::<libc::c_char>(),
                out.len(),
                c"%Y-%m-%dT%H:%M:%S%z".as_ptr(),
                &tm,
            )
        };
        write!(w, "{}", String::from_utf8_lossy(&out[..n]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_timer_uses_the_process_tz() {
        use tracing_subscriber::fmt::time::FormatTime;
        // SAFETY: single-threaded test; restore UTC at the end so no later test
        // in this binary sees a shifted zone.
        unsafe { std::env::set_var("TZ", "CET-1CEST,M3.5.0,M10.5.0/3") };
        unsafe extern "C" {
            fn tzset();
        }
        unsafe { tzset() };

        let mut s = String::new();
        LocalTimer
            .format_time(&mut tracing_subscriber::fmt::format::Writer::new(&mut s))
            .unwrap();
        assert!(
            s.ends_with("+0100") || s.ends_with("+0200"),
            "expected a CET/CEST offset, got {s}"
        );

        unsafe { std::env::set_var("TZ", "UTC") };
        unsafe { tzset() };
    }
}
