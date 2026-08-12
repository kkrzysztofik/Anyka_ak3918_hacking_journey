//! PUT /api/update — accept an upgrade bundle into the spool directory.
//!
//! The body is streamed straight to disk, never buffered: the camera has
//! ~36 MB of RAM and a bundle is ~19 MB. The transfer is staged as
//! `bundle.tar.part`, fsynced, renamed to `bundle.tar`, and only then is
//! `bundle.trigger` created — its existence is what tells the applier the
//! transfer finished.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use http_body_util::BodyExt;
use tokio::io::AsyncWriteExt;

/// Spool directory under `[update] root` (`/mnt/anyka_hack`), matching the
/// anyka-init applier's default.
pub const DEFAULT_SPOOL_ROOT: &str = "/mnt/anyka_hack/spool";

/// Ceiling on an accepted bundle.
///
/// The route is deliberately exempt from the server's `DefaultBodyLimit`,
/// because axum's `Body` extractor streams without consulting it — so without
/// an explicit counter here the write is unbounded and a stuck or hostile
/// client fills the card. A full `/mnt` takes logging, the storm-guard state
/// file and the spool itself down with it. Bundles are ~19 MB; 64 MB leaves
/// generous headroom for a bigger payload without leaving the door open.
pub const MAX_BUNDLE_BYTES: u64 = 64 * 1024 * 1024;

/// How long a `bundle.tar.part` may sit before it is treated as abandoned.
///
/// The `.part` file doubles as an in-flight lock, but nothing cleans it up if
/// the process dies mid-upload — which happens here, since the supervisor
/// restarts onvif-rust and killing it also takes down vendor-daemon. Without
/// this, one interrupted upload wedges the endpoint at 409 forever and the
/// only fix is telnet, on the camera the WebUI upload exists to avoid.
const STALE_PART_AFTER: Duration = Duration::from_secs(600);

/// Shared state for the update endpoint.
#[derive(Clone)]
pub struct UpdateState {
    pub spool_root: PathBuf,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self {
            spool_root: PathBuf::from(DEFAULT_SPOOL_ROOT),
        }
    }
}

/// Accept an upgrade bundle and queue it for the applier.
///
/// Returns 202: the update is queued, not yet applied. The reboot happens on
/// the applier's next poll.
pub async fn handle_update(Extension(state): Extension<Arc<UpdateState>>, body: Body) -> Response {
    match receive_bundle(&state.spool_root, body).await {
        Ok(()) => (StatusCode::ACCEPTED, "update queued").into_response(),
        Err(UpdateError::InFlight) => {
            tracing::warn!("bundle upload rejected: another upload is in progress");
            (StatusCode::CONFLICT, "upload already in progress").into_response()
        }
        Err(UpdateError::TooLarge) => {
            tracing::warn!(
                limit = MAX_BUNDLE_BYTES,
                "bundle upload rejected: too large"
            );
            (StatusCode::PAYLOAD_TOO_LARGE, "bundle too large").into_response()
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                spool = %state.spool_root.display(),
                "bundle upload failed"
            );
            (StatusCode::INTERNAL_SERVER_ERROR, "upload failed").into_response()
        }
    }
}

/// Stream `body` into the spool directory.
///
/// `bundle.tar.part` is created exclusively, so a concurrent upload fails on
/// `AlreadyExists` instead of interleaving with the first. On any failure the
/// partial file is removed, so nothing stale survives to confuse the next
/// attempt.
async fn receive_bundle(spool_root: &Path, body: Body) -> Result<(), UpdateError> {
    tokio::fs::create_dir_all(spool_root).await?;
    let part = spool_root.join("bundle.tar.part");
    let mut file = match tokio::fs::File::create_new(&part).await {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            if !reclaim_stale_part(&part).await {
                return Err(UpdateError::InFlight);
            }
            tokio::fs::File::create(&part).await?
        }
        Err(e) => return Err(e.into()),
    };

    let written = async {
        let mut body = body;
        let mut total: u64 = 0;
        while let Some(frame) = body.frame().await {
            let frame = frame.map_err(|e| UpdateError::Body(format!("{e}")))?;
            if let Ok(data) = frame.into_data() {
                total += data.len() as u64;
                if total > MAX_BUNDLE_BYTES {
                    return Err(UpdateError::TooLarge);
                }
                file.write_all(&data).await?;
            }
        }
        file.sync_all().await?;
        Ok::<(), UpdateError>(())
    }
    .await;

    if let Err(e) = written {
        let _ = tokio::fs::remove_file(&part).await;
        return Err(e);
    }

    tokio::fs::rename(&part, spool_root.join("bundle.tar")).await?;
    tokio::fs::File::create(spool_root.join("bundle.trigger")).await?;
    Ok(())
}

/// Is an existing `.part` old enough to be treated as abandoned?
///
/// Removing it is safe even in the unlikely case a real upload is still
/// running: that writer holds its own file handle, so it keeps writing to a
/// now-unlinked inode and its final rename fails harmlessly. Refusing forever
/// is the worse failure.
async fn reclaim_stale_part(part: &Path) -> bool {
    let Ok(meta) = tokio::fs::metadata(part).await else {
        // It vanished between the create and the stat — treat as reclaimable.
        return true;
    };
    let stale = meta
        .modified()
        .ok()
        .and_then(|m| m.elapsed().ok())
        .is_some_and(|age| age > STALE_PART_AFTER);
    if stale {
        tracing::warn!(
            path = %part.display(),
            "removing an abandoned partial upload left by an interrupted transfer"
        );
        let _ = tokio::fs::remove_file(part).await;
    }
    stale
}

#[derive(Debug, thiserror::Error)]
enum UpdateError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("body stream: {0}")]
    Body(String),
    #[error("another upload is already in progress")]
    InFlight,
    #[error("bundle exceeds {MAX_BUNDLE_BYTES} bytes")]
    TooLarge,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use bytes::Bytes;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tower::ServiceExt;

    // ── receive_bundle ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_receive_writes_tar_then_trigger() {
        let d = tempfile::tempdir().unwrap();
        let spool = d.path().join("spool");
        let body = Body::new(http_body_util::Full::new(Bytes::from_static(
            b"pretend tar",
        )));

        receive_bundle(&spool, body).await.unwrap();

        assert_eq!(
            std::fs::read(spool.join("bundle.tar")).unwrap(),
            b"pretend tar"
        );
        assert!(
            spool.join("bundle.trigger").is_file(),
            "trigger must be created after the tar"
        );
        assert!(
            !spool.join("bundle.tar.part").exists(),
            "staging file must be gone after a successful upload"
        );
    }

    /// A body that yields one chunk and then fails mid-stream.
    struct ErrorAfterData {
        sent: bool,
    }

    impl http_body::Body for ErrorAfterData {
        type Data = Bytes;
        type Error = std::io::Error;

        fn poll_frame(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Option<Result<http_body::Frame<Bytes>, Self::Error>>> {
            if !self.sent {
                self.sent = true;
                Poll::Ready(Some(Ok(http_body::Frame::data(Bytes::from_static(
                    b"partial",
                )))))
            } else {
                Poll::Ready(Some(Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionReset,
                    "simulated mid-stream failure",
                ))))
            }
        }
    }

    /// A body that yields the same chunk `remaining` times.
    ///
    /// Streams rather than materializing the whole payload: the point is to
    /// push past the size ceiling, and allocating it contiguously aborts the
    /// test process on a machine with a modest heap.
    struct RepeatBody {
        chunk: Bytes,
        remaining: usize,
    }

    impl http_body::Body for RepeatBody {
        type Data = Bytes;
        type Error = std::io::Error;

        fn poll_frame(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Option<Result<http_body::Frame<Bytes>, Self::Error>>> {
            if self.remaining == 0 {
                return Poll::Ready(None);
            }
            self.remaining -= 1;
            let chunk = self.chunk.clone();
            Poll::Ready(Some(Ok(http_body::Frame::data(chunk))))
        }
    }

    #[tokio::test]
    async fn test_receive_on_stream_error_leaves_no_trigger() {
        let d = tempfile::tempdir().unwrap();
        let spool = d.path().join("spool");

        let err = receive_bundle(&spool, Body::new(ErrorAfterData { sent: false }))
            .await
            .unwrap_err();

        assert!(
            matches!(err, UpdateError::Body(_)),
            "expected a body-stream error, got {err:?}"
        );
        assert!(
            !spool.join("bundle.trigger").exists(),
            "a failed transfer must never leave a trigger"
        );
        assert!(
            !spool.join("bundle.tar").exists(),
            "a failed transfer must not leave a bundle"
        );
        assert!(
            !spool.join("bundle.tar.part").exists(),
            "the partial file must be cleaned up on failure"
        );
    }

    #[tokio::test]
    async fn test_receive_rejects_a_second_concurrent_upload() {
        let d = tempfile::tempdir().unwrap();
        let spool = d.path().join("spool");
        std::fs::create_dir_all(&spool).unwrap();
        // Simulate an upload already in flight.
        std::fs::write(spool.join("bundle.tar.part"), b"in progress").unwrap();

        let body = Body::new(http_body_util::Full::new(Bytes::from_static(
            b"second upload",
        )));
        let err = receive_bundle(&spool, body).await.unwrap_err();

        assert!(
            matches!(err, UpdateError::InFlight),
            "a second upload must be rejected, got {err:?}"
        );
    }

    #[tokio::test]
    async fn test_receive_reclaims_an_abandoned_part_file() {
        let d = tempfile::tempdir().unwrap();
        let spool = d.path().join("spool");
        std::fs::create_dir_all(&spool).unwrap();

        // A .part left behind by a process that died mid-upload. Backdate it
        // past the staleness window; without reclaim this wedges the endpoint
        // at 409 forever and only telnet can clear it.
        let part = spool.join("bundle.tar.part");
        std::fs::write(&part, b"orphaned").unwrap();
        let old = std::time::SystemTime::now() - STALE_PART_AFTER - Duration::from_secs(60);
        std::fs::File::options()
            .write(true)
            .open(&part)
            .unwrap()
            .set_modified(old)
            .unwrap();

        let body = Body::new(http_body_util::Full::new(Bytes::from_static(b"new bundle")));
        receive_bundle(&spool, body)
            .await
            .expect("a stale .part must not block a fresh upload");

        assert!(spool.join("bundle.trigger").is_file());
        assert_eq!(
            std::fs::read(spool.join("bundle.tar")).unwrap(),
            b"new bundle"
        );
    }

    #[tokio::test]
    async fn test_receive_still_rejects_a_fresh_part_file() {
        let d = tempfile::tempdir().unwrap();
        let spool = d.path().join("spool");
        std::fs::create_dir_all(&spool).unwrap();
        // Just-created .part: a genuine concurrent upload, not an orphan.
        std::fs::write(spool.join("bundle.tar.part"), b"in progress").unwrap();

        let body = Body::new(http_body_util::Full::new(Bytes::from_static(b"second")));
        let err = receive_bundle(&spool, body).await.unwrap_err();

        assert!(
            matches!(err, UpdateError::InFlight),
            "a recent .part is a live upload, got {err:?}"
        );
    }

    #[tokio::test]
    async fn test_receive_rejects_a_body_past_the_size_ceiling() {
        let d = tempfile::tempdir().unwrap();
        let spool = d.path().join("spool");

        // axum's Body extractor ignores DefaultBodyLimit, so this counter is
        // the only thing standing between a stuck client and a full SD card.
        const CHUNK: usize = 1024 * 1024;
        let body = Body::new(RepeatBody {
            chunk: Bytes::from(vec![0u8; CHUNK]),
            remaining: (MAX_BUNDLE_BYTES as usize / CHUNK) + 1,
        });
        let err = receive_bundle(&spool, body).await.unwrap_err();

        assert!(
            matches!(err, UpdateError::TooLarge),
            "an oversized bundle must be refused, got {err:?}"
        );
        assert!(
            !spool.join("bundle.trigger").exists(),
            "nothing may be queued"
        );
        assert!(
            !spool.join("bundle.tar.part").exists(),
            "the partial write must be cleaned up, not left as a lock"
        );
    }

    #[tokio::test]
    async fn test_receive_retry_after_failure_succeeds() {
        let d = tempfile::tempdir().unwrap();
        let spool = d.path().join("spool");

        // First attempt fails mid-stream.
        receive_bundle(&spool, Body::new(ErrorAfterData { sent: false }))
            .await
            .unwrap_err();

        // Second attempt must be able to start fresh.
        let body = Body::new(http_body_util::Full::new(Bytes::from_static(b"retry")));
        receive_bundle(&spool, body).await.unwrap();
        assert_eq!(std::fs::read(spool.join("bundle.tar")).unwrap(), b"retry");
    }

    // ── handle_update over the axum router ─────────────────────────────────

    fn update_app(spool_root: PathBuf) -> axum::Router {
        axum::Router::new()
            .route("/update", axum::routing::put(handle_update))
            .layer(axum::Extension(Arc::new(UpdateState { spool_root })))
    }

    #[tokio::test]
    async fn test_handle_update_returns_202_and_queues() {
        let d = tempfile::tempdir().unwrap();
        let app = update_app(d.path().join("spool"));

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/update")
                    .header("content-type", "application/octet-stream")
                    .body(Body::new(http_body_util::Full::new(Bytes::from_static(
                        b"bundle",
                    ))))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);
        assert!(d.path().join("spool/bundle.tar").is_file());
        assert!(d.path().join("spool/bundle.trigger").is_file());
    }
}
