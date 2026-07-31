//! Sole owner of the vendor-daemon attachment.
//!
//! Detection sites (control owner thread, frame reader, epoch poller) only *report*
//! peer loss; they never attach. This is an invariant, not a convention: the daemon's
//! single-owner guards reject a concurrent second attacher rather than serialising it
//! (`dispatcher.c` `acquire_control` for control, `main.c` for the frame sockets), so
//! two simultaneous attaches leave a half-attached mess rather than one winner.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{broadcast, mpsc, watch};

use crate::hal::anyka::ipc::{AnykaIpc, PeerLoss};
use crate::platform::common::Platform;
use crate::platform::{PlatformError, PlatformResult};

use super::AnykaPlatform;

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

pub use crate::platform::common::Availability;

/// How often the ring epoch is polled while attached.
const EPOCH_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// One tick of the epoch poller: refresh from the ring, publish loss if it moved.
///
/// A detection *backstop*, not the primary detector. In this build push stays
/// active regardless of RTSP clients, so a daemon restart trips frame-socket
/// errors within milliseconds and the frame reader / control owner report it
/// first. The poller still earns its keep on two races those miss: a daemon that
/// restarts fast enough to re-stamp the ring before the frame reader notices the
/// old socket died, and a half-dead daemon whose socket never EOFs. Cost is one
/// volatile u32 read of an already-mapped page per tick.
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

/// Wait for peer loss, from whichever detector notices first.
///
/// Detection is layered, fastest first:
/// - frame-socket EOF/error — milliseconds — the primary detector while push is
///   active (which, in this build, is always);
/// - control-socket error — on the next control request;
/// - epoch poller — 1 s — a backstop for restart races and non-EOF daemon death.
///
/// The `reports` channel carries the first two; the ticker drives the poller.
/// (`idle-stop-push`, which would make the poller the sole idle detector, is
/// deliberately not implemented — see the S4 design.)
pub async fn watch_epoch_until_loss(
    ipc: &AnykaIpc,
    tx: &watch::Sender<Availability>,
    reports: &mut mpsc::Receiver<PeerLoss>,
) {
    // Discard reports left over from a previous generation. Tearing down the old
    // attachment — the frame reader hitting EOF, rollback's CLOSE calls racing the
    // dead socket — emits broken-pipe reports that land in this depth-1 channel.
    // One survives into the freshly attached generation and, without this drain,
    // fires on the first `recv()` below, unwinding a healthy attach ~1 ms after it
    // went Available. A real loss for the new generation instead re-arrives from its
    // own traffic, or is caught by the epoch poller within one tick.
    while reports.try_recv().is_ok() {}

    let mut ticker = tokio::time::interval(EPOCH_POLL_INTERVAL);
    loop {
        tokio::select! {
            reported = reports.recv() => {
                match reported {
                    Some(loss) => {
                        tracing::warn!(reason = %loss.reason, "peer loss reported by detection site");
                        let _ = tx.send(Availability::Unavailable);
                        return;
                    }
                    // Senders are held by the IPC client for its whole life, so this
                    // only happens if the client is being torn down.
                    None => return,
                }
            }
            _ = ticker.tick() => {
                poll_epoch_once(ipc, tx);
                if *tx.borrow() != Availability::Available {
                    return;
                }
            }
        }
    }
}

/// The four operations the supervisor drives, as one seam.
///
/// Exists so [`run_supervisor`] can be tested without a daemon or an SDK. The
/// production implementation delegates straight to `AnykaIpc` and `AnykaPlatform`;
/// it deliberately adds no logic of its own, because
/// `AnykaPlatform::initialize`/`rollback_video_pipeline` already own bring-up and
/// unwind and must not be reimplemented here.
#[async_trait::async_trait]
pub trait AttachTarget: Send + Sync {
    /// Connect frame sockets and ring, then pin the daemon epoch.
    async fn attach(&self) -> PlatformResult<u32>;
    /// Run the four hardware bring-up stages.
    async fn initialize(&self) -> PlatformResult<()>;
    /// Unwind a partial or complete bring-up. Best-effort; must not fail.
    async fn rollback(&self);
    /// Release the attachment, making every outstanding handle inert.
    fn detach(&self);
    /// Block until peer loss is observed.
    async fn wait_for_loss(&self, tx: &watch::Sender<Availability>);
}

/// Production [`AttachTarget`]: delegates to the existing platform machinery.
///
/// Deliberately thin. `AnykaPlatform::initialize` already runs the four bring-up
/// stages and `rollback_video_pipeline` already unwinds them in reverse, so this
/// reuses both rather than growing a second copy that can drift.
pub struct PlatformAttachTarget {
    platform: Arc<AnykaPlatform>,
    /// Peer-loss reports from the control owner thread and the frame reader.
    ///
    /// `Mutex` only because `wait_for_loss` takes `&self`; there is exactly one
    /// supervisor, so it is never contended.
    reports: tokio::sync::Mutex<mpsc::Receiver<PeerLoss>>,
}

impl PlatformAttachTarget {
    pub fn new(platform: Arc<AnykaPlatform>) -> PlatformResult<Self> {
        let reports = platform.ipc().take_loss_rx().ok_or_else(|| {
            PlatformError::InitializationFailed(
                "peer-loss receiver already taken; supervisor must be spawned once".to_string(),
            )
        })?;
        Ok(Self {
            platform,
            reports: tokio::sync::Mutex::new(reports),
        })
    }
}

#[async_trait::async_trait]
impl AttachTarget for PlatformAttachTarget {
    async fn attach(&self) -> PlatformResult<u32> {
        self.platform.ipc().attach().await
    }

    async fn initialize(&self) -> PlatformResult<()> {
        self.platform.initialize().await
    }

    async fn rollback(&self) {
        self.platform.rollback_for_supervisor().await;
    }

    fn detach(&self) {
        self.platform.ipc().detach();
    }

    async fn wait_for_loss(&self, tx: &watch::Sender<Availability>) {
        let mut reports = self.reports.lock().await;
        watch_epoch_until_loss(self.platform.ipc(), tx, &mut reports).await;
    }
}

/// Drive attach → initialize → serve → unwind, retrying on failure.
///
/// Only `initialize()` failures against a live daemon count toward the breaker;
/// `attach()` failures (absent or late daemon) retry indefinitely with backoff and
/// never open it. So with a daemon that never appears this function never returns.
/// When the breaker does latch open it publishes [`Availability::GivenUp`], because a
/// backoff that hides a permanent failure would leave the camera dark with evidence
/// only in the logs; publishing `GivenUp` is what makes it visible to the ONVIF services.
pub async fn run_supervisor(
    target: Arc<dyn AttachTarget>,
    tx: &watch::Sender<Availability>,
    mut shutdown: broadcast::Receiver<()>,
) {
    let mut backoff = Backoff::new();
    let mut breaker = CircuitBreaker::new();

    loop {
        if breaker.is_open() {
            tracing::error!(
                event = "attach_given_up",
                limit = ATTACH_FAILURE_LIMIT,
                "vendor-daemon attach failed repeatedly; giving up"
            );
            let _ = tx.send(Availability::GivenUp);
            return;
        }

        let epoch = tokio::select! {
            _ = shutdown.recv() => {
                tracing::info!(event = "supervisor_shutdown", "attach supervisor exiting");
                return;
            }
            result = target.attach() => match result {
                Err(e) => {
                    // Do NOT count this toward the breaker. attach() fails here because
                    // the daemon is absent, not yet listening, or restarting mid-attach
                    // — no SDK call happened, so there is no VI_OPEN/VENC_OPEN churn to
                    // bound. The breaker exists only for a live-but-wedged daemon, which
                    // is the initialize() arm below. Counting absent-daemon failures
                    // here is what made degraded boot give up on a late daemon (S3).
                    tracing::warn!(error = %e, "attach failed; retrying (daemon absent)");
                    let _ = tx.send(Availability::Unavailable);
                    tokio::select! {
                        _ = shutdown.recv() => {
                            tracing::info!(event = "supervisor_shutdown", "attach supervisor exiting");
                            return;
                        }
                        _ = tokio::time::sleep(backoff.next()) => {}
                    }
                    continue;
                }
                Ok(epoch) => epoch,
            }
        };

        let init_result = tokio::select! {
            _ = shutdown.recv() => {
                tracing::info!(event = "supervisor_shutdown", "attach supervisor exiting");
                target.detach();
                return;
            }
            result = target.initialize() => result,
        };

        match init_result {
            Err(e) => {
                tracing::warn!(epoch, error = %e, "bring-up failed; unwinding");
                // Partial bring-up MUST be unwound. Without this, each retry
                // does another VI_OPEN/VENC_OPEN cycle against an SDK that
                // wedges.
                target.rollback().await;
                target.detach();
                let _ = tx.send(Availability::Unavailable);
                breaker.record_failure();
                tokio::select! {
                    _ = shutdown.recv() => {
                        tracing::info!(event = "supervisor_shutdown", "attach supervisor exiting");
                        return;
                    }
                    _ = tokio::time::sleep(backoff.next()) => {}
                }
            }
            Ok(()) => {
                tracing::info!(event = "attach_available", epoch, "vendor daemon available");
                breaker.record_success();
                backoff.reset();
                let _ = tx.send(Availability::Available);

                tokio::select! {
                    _ = shutdown.recv() => {
                        tracing::info!(event = "supervisor_shutdown", "attach supervisor exiting");
                        let _ = tx.send(Availability::Unavailable);
                        target.rollback().await;
                        target.detach();
                        return;
                    }
                    _ = target.wait_for_loss(tx) => {}
                }

                tracing::warn!(epoch, "peer loss observed; unwinding");
                let _ = tx.send(Availability::Unavailable);
                target.rollback().await;
                target.detach();
            }
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
        Self {
            next: BACKOFF_START,
        }
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
    async fn test_poller_ring_epoch_changed_reports_loss() {
        let ipc = AnykaIpc::new_detached().unwrap();
        ipc.attach_anon_ring_for_test(11);
        let (tx, mut rx) = watch::channel(Availability::Available);

        // Daemon restarts: stamp a new generation into the ring.
        ipc.stamp_ring_epoch_for_test(12);

        poll_epoch_once(&ipc, &tx);

        assert_eq!(*rx.borrow_and_update(), Availability::Unavailable);
    }

    #[tokio::test]
    async fn test_poller_epoch_holds_stays_available() {
        let ipc = AnykaIpc::new_detached().unwrap();
        ipc.attach_anon_ring_for_test(11);
        let (tx, mut rx) = watch::channel(Availability::Available);

        poll_epoch_once(&ipc, &tx);

        assert_eq!(*rx.borrow_and_update(), Availability::Available);
    }

    #[tokio::test(start_paused = true)]
    async fn test_watch_stale_report_does_not_unwind_new_generation() {
        // Regression for the hardware flap: tearing down generation N leaves a
        // broken-pipe report buffered in the depth-1 channel. It must not unwind the
        // freshly attached generation N+1, which is healthy and whose epoch matches.
        let ipc = AnykaIpc::new_detached().unwrap();
        ipc.attach_anon_ring_for_test(12); // new generation, ring == attached
        let mut reports = ipc.take_loss_rx().unwrap();

        // A leftover report from before the re-attach is sitting in the channel.
        ipc.report_peer_loss("control socket error: Broken pipe (from dead gen)");

        let (tx, _rx) = watch::channel(Availability::Available);

        // With the stale report drained, nothing ends the wait until real loss;
        // time out the wait to prove it stayed up rather than unwinding.
        let waited = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            watch_epoch_until_loss(&ipc, &tx, &mut reports),
        )
        .await;

        assert!(
            waited.is_err(),
            "wait returned early — the stale report unwound a healthy attach"
        );
        assert_eq!(*tx.borrow(), Availability::Available, "must stay available");
    }

    #[tokio::test(start_paused = true)]
    async fn test_watch_reported_loss_wakes_without_epoch_move() {
        // Control-socket errors and frame EOF arriving *during* the wait must not
        // sit through a poll interval. Distinct from a stale pre-wait report, which
        // is drained: this one is generated after the wait has begun.
        let ipc = std::sync::Arc::new(AnykaIpc::new_detached().unwrap());
        ipc.attach_anon_ring_for_test(11);
        let mut reports = ipc.take_loss_rx().unwrap();
        let (tx, _rx) = watch::channel(Availability::Available);

        let reporter = std::sync::Arc::clone(&ipc);
        tokio::spawn(async move {
            // Let the drain run and the select loop start, then report.
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            reporter.report_peer_loss("control socket error: broken pipe");
        });

        // The ring epoch is untouched, so only the reports channel can end this.
        watch_epoch_until_loss(&ipc, &tx, &mut reports).await;

        assert_eq!(*tx.borrow(), Availability::Unavailable);
        assert_eq!(ipc.observed_epoch_for_test(), 11, "epoch never moved");
    }

    #[tokio::test]
    async fn test_poller_detached_reports_loss() {
        // No ring mapped at all: the supervisor must learn the attachment is gone.
        let ipc = AnykaIpc::new_detached().unwrap();
        let (tx, mut rx) = watch::channel(Availability::Available);

        poll_epoch_once(&ipc, &tx);

        assert_eq!(*rx.borrow_and_update(), Availability::Unavailable);
    }

    #[tokio::test]
    async fn test_poller_already_unavailable_does_not_republish() {
        let ipc = AnykaIpc::new_detached().unwrap();
        let (tx, rx) = watch::channel(Availability::Unavailable);

        poll_epoch_once(&ipc, &tx);

        assert!(
            !rx.has_changed().unwrap_or(true),
            "must stay quiet while already Unavailable"
        );
        assert_eq!(*rx.borrow(), Availability::Unavailable);
    }

    #[test]
    fn test_backoff_and_circuit_breaker_default_construct() {
        assert_eq!(Backoff::default().next(), Duration::from_millis(500));
        assert!(!CircuitBreaker::default().is_open());
    }

    // ------------------------------------------------------------------
    // Supervisor loop
    // ------------------------------------------------------------------

    use crate::platform::PlatformError;
    use std::sync::Mutex as StdMutex;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    /// Scripted stand-in for `attach + AnykaPlatform::initialize + rollback + detach`.
    struct MockTarget {
        attach_ok: bool,
        /// `initialize` returns `Ok` for the first N calls, then `Err` forever.
        init_ok_count: usize,
        attach_calls: AtomicUsize,
        init_calls: AtomicUsize,
        rollback_calls: AtomicUsize,
        detach_calls: AtomicUsize,
        /// Availability observed at the top of each `wait_for_loss`.
        seen_while_waiting: StdMutex<Vec<Availability>>,
    }

    impl MockTarget {
        fn new(attach_ok: bool, init_ok_count: usize) -> Self {
            Self {
                attach_ok,
                init_ok_count,
                attach_calls: AtomicUsize::new(0),
                init_calls: AtomicUsize::new(0),
                rollback_calls: AtomicUsize::new(0),
                detach_calls: AtomicUsize::new(0),
                seen_while_waiting: StdMutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl AttachTarget for MockTarget {
        async fn attach(&self) -> PlatformResult<u32> {
            self.attach_calls.fetch_add(1, AtomicOrdering::SeqCst);
            if self.attach_ok {
                Ok(7)
            } else {
                Err(PlatformError::HardwareUnavailable("no daemon".into()))
            }
        }

        async fn initialize(&self) -> PlatformResult<()> {
            let n = self.init_calls.fetch_add(1, AtomicOrdering::SeqCst);
            if n < self.init_ok_count {
                Ok(())
            } else {
                Err(PlatformError::InitializationFailed("wedged SDK".into()))
            }
        }

        async fn rollback(&self) {
            self.rollback_calls.fetch_add(1, AtomicOrdering::SeqCst);
        }

        fn detach(&self) {
            self.detach_calls.fetch_add(1, AtomicOrdering::SeqCst);
        }

        async fn wait_for_loss(&self, tx: &watch::Sender<Availability>) {
            self.seen_while_waiting.lock().unwrap().push(*tx.borrow());
        }
    }

    #[tokio::test(start_paused = true)]
    async fn test_supervisor_init_failures_opens_breaker_gives_up() {
        // The breaker opens only via the real path: a live daemon (attach
        // succeeds) whose bring-up wedges (initialize always fails). Each failed
        // initialize is a VI_OPEN/VENC_OPEN cycle against the SDK — that churn is
        // what the breaker bounds.
        let target = std::sync::Arc::new(MockTarget::new(true, 0));
        let (tx, _rx) = watch::channel(Availability::Unavailable);

        let (_shutdown_tx, shutdown_rx) = broadcast::channel(1);
        run_supervisor(target.clone(), &tx, shutdown_rx).await;

        assert_eq!(
            target.init_calls.load(AtomicOrdering::SeqCst),
            ATTACH_FAILURE_LIMIT as usize,
            "must stop attempting once the breaker opens"
        );
        assert_eq!(*tx.borrow(), Availability::GivenUp);
    }

    #[tokio::test(start_paused = true)]
    async fn test_supervisor_absent_daemon_never_opens_breaker() {
        // attach() failing because the daemon is not there yet must be retried
        // forever: no SDK call happened, so there is no churn to bound. Only
        // bring-up (initialize) failures against a live daemon count.
        let target = std::sync::Arc::new(MockTarget::new(false, 0));
        let (tx, _rx) = watch::channel(Availability::Unavailable);

        // Run the supervisor with a hard timeout: with the fix it never returns
        // (retries forever), so a timeout is the pass signal.
        // 600s must exceed enough capped (15s) backoff cycles to drive attach_calls
        // past ATTACH_FAILURE_LIMIT; do not trim it or the assertion below weakens.
        let (_shutdown_tx, shutdown_rx) = broadcast::channel(1);
        let stop = tokio::time::timeout(
            std::time::Duration::from_secs(600),
            run_supervisor(target.clone(), &tx, shutdown_rx),
        )
        .await;

        assert!(stop.is_err(), "supervisor gave up on an absent daemon");
        assert!(
            target.attach_calls.load(AtomicOrdering::SeqCst) > ATTACH_FAILURE_LIMIT as usize,
            "must keep retrying past the failure limit, not give up"
        );
        assert_ne!(*tx.borrow(), Availability::GivenUp);
    }

    #[tokio::test(start_paused = true)]
    async fn test_supervisor_failed_initialize_rolls_back_once() {
        // Partial bring-up MUST be unwound, or each retry does another
        // VI_OPEN/VENC_OPEN cycle against an SDK that wedges.
        let target = std::sync::Arc::new(MockTarget::new(true, 0));
        let (tx, _rx) = watch::channel(Availability::Unavailable);

        let (_shutdown_tx, shutdown_rx) = broadcast::channel(1);
        run_supervisor(target.clone(), &tx, shutdown_rx).await;

        let failures = ATTACH_FAILURE_LIMIT as usize;
        assert_eq!(target.init_calls.load(AtomicOrdering::SeqCst), failures);
        assert_eq!(
            target.rollback_calls.load(AtomicOrdering::SeqCst),
            failures,
            "one rollback per failed initialize"
        );
        assert_eq!(target.detach_calls.load(AtomicOrdering::SeqCst), failures);
        assert_eq!(*tx.borrow(), Availability::GivenUp);
    }

    #[tokio::test(start_paused = true)]
    async fn test_supervisor_shutdown_during_attach_retry_exits_cleanly() {
        let target = std::sync::Arc::new(MockTarget::new(false, 0));
        let (tx, _rx) = watch::channel(Availability::Unavailable);
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

        let run = tokio::spawn({
            let target = target.clone();
            let tx = tx.clone();
            async move { run_supervisor(target, &tx, shutdown_rx).await }
        });

        // Let the absent-daemon retry path start, then cancel.
        tokio::time::advance(Duration::from_millis(10)).await;
        let _ = shutdown_tx.send(());

        tokio::time::timeout(Duration::from_secs(1), run)
            .await
            .expect("supervisor must exit on shutdown")
            .expect("supervisor task must not panic");
        assert_ne!(*tx.borrow(), Availability::GivenUp);
    }

    #[tokio::test(start_paused = true)]
    async fn test_supervisor_shutdown_while_available_rolls_back_and_exits() {
        let (tx, _rx) = watch::channel(Availability::Unavailable);
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

        let run = tokio::spawn({
            let target = std::sync::Arc::new(HoldingTarget {
                inner: MockTarget::new(true, 1),
                hold: true,
            });
            let tx = tx.clone();
            async move { run_supervisor(target, &tx, shutdown_rx).await }
        });

        // Advance until Available is published and wait_for_loss is holding.
        tokio::time::advance(Duration::from_millis(50)).await;
        assert_eq!(*tx.borrow(), Availability::Available);
        let _ = shutdown_tx.send(());

        tokio::time::timeout(Duration::from_secs(1), run)
            .await
            .expect("supervisor must exit on shutdown while available")
            .expect("supervisor task must not panic");
        assert_eq!(*tx.borrow(), Availability::Unavailable);
    }

    /// Like [`MockTarget`], but `wait_for_loss` parks until cancelled by shutdown.
    struct HoldingTarget {
        inner: MockTarget,
        hold: bool,
    }

    #[async_trait::async_trait]
    impl AttachTarget for HoldingTarget {
        async fn attach(&self) -> PlatformResult<u32> {
            self.inner.attach().await
        }
        async fn initialize(&self) -> PlatformResult<()> {
            self.inner.initialize().await
        }
        async fn rollback(&self) {
            self.inner.rollback().await;
        }
        fn detach(&self) {
            self.inner.detach();
        }
        async fn wait_for_loss(&self, tx: &watch::Sender<Availability>) {
            self.inner
                .seen_while_waiting
                .lock()
                .unwrap()
                .push(*tx.borrow());
            if self.hold {
                std::future::pending::<()>().await;
            }
        }
    }

    #[tokio::test(start_paused = true)]
    async fn test_supervisor_available_then_unwinds_on_loss() {
        // One clean bring-up, then peer loss, then repeated failures until the
        // breaker opens.
        let target = std::sync::Arc::new(MockTarget::new(true, 1));
        let (tx, _rx) = watch::channel(Availability::Unavailable);

        let (_shutdown_tx, shutdown_rx) = broadcast::channel(1);
        run_supervisor(target.clone(), &tx, shutdown_rx).await;

        assert_eq!(
            *target.seen_while_waiting.lock().unwrap(),
            vec![Availability::Available],
            "Available must be published before the supervisor waits for loss"
        );
        // 1 unwind for the loss + one per subsequent failed initialize.
        assert_eq!(
            target.rollback_calls.load(AtomicOrdering::SeqCst),
            ATTACH_FAILURE_LIMIT as usize + 1
        );
        assert_eq!(*tx.borrow(), Availability::GivenUp);
    }

    #[test]
    fn test_backoff_grows_and_caps() {
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
    fn test_circuit_breaker_opens_after_threshold_stays_open() {
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
    fn test_circuit_breaker_resets_on_success() {
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
