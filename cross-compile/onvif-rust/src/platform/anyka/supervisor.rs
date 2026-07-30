//! Sole owner of the vendor-daemon attachment.
//!
//! Detection sites (control owner thread, frame reader, epoch poller) only *report*
//! peer loss; they never attach. This is an invariant, not a convention: the daemon's
//! single-owner guards reject a concurrent second attacher rather than serialising it
//! (`dispatcher.c` `acquire_control` for control, `main.c` for the frame sockets), so
//! two simultaneous attaches leave a half-attached mess rather than one winner.

use std::time::Duration;

use tokio::sync::watch;

use crate::hal::anyka::ipc::AnykaIpc;

/// First retry delay; doubles up to [`BACKOFF_MAX`].
const BACKOFF_START: Duration = Duration::from_millis(500);
/// Cap on the retry delay.
const BACKOFF_MAX: Duration = Duration::from_secs(15);
/// Consecutive attach failures before the breaker opens.
///
/// This does not exist to save CPU — a connect attempt per second is noise on this
/// box. It bounds cumulative damage to the vendor SDK from repeated
/// VI_OPEN/VENC_OPEN churn, and it stops a future respawn loop from being amplified
/// into a crash loop if attach is what kills the daemon.
const ATTACH_FAILURE_LIMIT: u32 = 10;

/// What the rest of the application observes about the attachment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// Attached and the hardware pipeline is initialised.
    Available,
    /// Not attached; the supervisor is retrying.
    Unavailable,
    /// The breaker is open. No further attach attempts without intervention.
    GivenUp,
}

/// How often the ring epoch is polled while attached.
const EPOCH_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// One tick of the epoch poller: refresh from the ring, publish loss if it moved.
///
/// This is the only thing that detects a restart while idle. With push stopped and
/// no RTSP client there is no frame traffic, so no socket ever errors and no EOF ever
/// arrives — the camera would otherwise sit streaming nothing indefinitely. The cost
/// is a single volatile `u32` read of an already-mapped page.
pub fn poll_epoch_once(ipc: &AnykaIpc, tx: &watch::Sender<Availability>) {
    if !ipc.refresh_observed_epoch() {
        // send_if_modified keeps the channel quiet while the loss persists, so
        // watchers wake once per transition rather than once per tick.
        tx.send_if_modified(|current| {
            if *current == Availability::Available {
                *current = Availability::Unavailable;
                true
            } else {
                false
            }
        });
    }
}

/// Poll the ring epoch until loss is detected, then return.
///
/// Separated from [`poll_epoch_once`] so the tick logic stays synchronously testable.
pub async fn watch_epoch_until_loss(ipc: &AnykaIpc, tx: &watch::Sender<Availability>) {
    let mut ticker = tokio::time::interval(EPOCH_POLL_INTERVAL);
    loop {
        ticker.tick().await;
        poll_epoch_once(ipc, tx);
        if *tx.borrow() != Availability::Available {
            return;
        }
    }
}

/// Exponential retry delay, capped at [`BACKOFF_MAX`].
///
/// Deliberately not jittered: there is exactly one supervisor on this box, so there
/// is no thundering herd to spread out, and a predictable delay is easier to read in
/// a log during hardware bring-up.
pub struct Backoff {
    next: Duration,
}

impl Backoff {
    pub fn new() -> Self {
        Self { next: BACKOFF_START }
    }

    /// Return the current delay and double it for next time, saturating at the cap.
    pub fn next(&mut self) -> Duration {
        let current = self.next;
        self.next = (current * 2).min(BACKOFF_MAX);
        current
    }

    /// Return to [`BACKOFF_START`] after a successful attach.
    pub fn reset(&mut self) {
        self.next = BACKOFF_START;
    }
}

impl Default for Backoff {
    fn default() -> Self {
        Self::new()
    }
}

/// Counts consecutive attach failures and latches open at [`ATTACH_FAILURE_LIMIT`].
///
/// Once open it stays open: reopening on its own would reintroduce exactly the
/// unbounded VI_OPEN/VENC_OPEN churn the breaker exists to stop.
pub struct CircuitBreaker {
    consecutive_failures: u32,
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            consecutive_failures: 0,
        }
    }

    pub fn record_failure(&mut self) {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
    }

    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
    }

    pub fn is_open(&self) -> bool {
        self.consecutive_failures >= ATTACH_FAILURE_LIMIT
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hal::anyka::ipc::AnykaIpc;
    use tokio::sync::watch;

    #[tokio::test]
    async fn poller_reports_loss_when_the_ring_epoch_changes() {
        let ipc = AnykaIpc::new_detached().unwrap();
        ipc.attach_anon_ring_for_test(11);
        let (tx, mut rx) = watch::channel(Availability::Available);

        // Daemon restarts: stamp a new generation into the ring.
        ipc.stamp_ring_epoch_for_test(12);

        poll_epoch_once(&ipc, &tx);

        assert_eq!(*rx.borrow_and_update(), Availability::Unavailable);
    }

    #[tokio::test]
    async fn poller_is_quiet_while_the_epoch_holds() {
        let ipc = AnykaIpc::new_detached().unwrap();
        ipc.attach_anon_ring_for_test(11);
        let (tx, mut rx) = watch::channel(Availability::Available);

        poll_epoch_once(&ipc, &tx);

        assert_eq!(*rx.borrow_and_update(), Availability::Available);
    }

    #[tokio::test]
    async fn poller_reports_loss_when_detached() {
        // No ring mapped at all: the supervisor must learn the attachment is gone.
        let ipc = AnykaIpc::new_detached().unwrap();
        let (tx, mut rx) = watch::channel(Availability::Available);

        poll_epoch_once(&ipc, &tx);

        assert_eq!(*rx.borrow_and_update(), Availability::Unavailable);
    }

    #[test]
    fn backoff_grows_and_caps() {
        let mut b = Backoff::new();
        assert_eq!(b.next(), Duration::from_millis(500));
        assert_eq!(b.next(), Duration::from_secs(1));
        assert_eq!(b.next(), Duration::from_secs(2));
        assert_eq!(b.next(), Duration::from_secs(4));
        assert_eq!(b.next(), Duration::from_secs(8));
        assert_eq!(b.next(), BACKOFF_MAX);
        assert_eq!(b.next(), BACKOFF_MAX, "must cap, not grow unbounded");
        b.reset();
        assert_eq!(b.next(), Duration::from_millis(500));
    }

    #[test]
    fn circuit_breaker_opens_after_the_threshold_and_stays_open() {
        let mut cb = CircuitBreaker::new();
        for _ in 0..ATTACH_FAILURE_LIMIT - 1 {
            cb.record_failure();
            assert!(!cb.is_open(), "must not trip early");
        }
        cb.record_failure();
        assert!(cb.is_open(), "must trip at the limit");
        cb.record_failure();
        assert!(cb.is_open(), "must stay open");
    }

    #[test]
    fn circuit_breaker_resets_on_success() {
        let mut cb = CircuitBreaker::new();
        cb.record_failure();
        cb.record_failure();
        cb.record_success();
        for _ in 0..ATTACH_FAILURE_LIMIT - 1 {
            cb.record_failure();
        }
        assert!(!cb.is_open(), "success must clear the count");
    }
}
