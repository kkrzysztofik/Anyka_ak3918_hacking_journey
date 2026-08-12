//! PUT /api/update — accept an upgrade bundle into the spool directory.
//!
//! The body is streamed straight to disk, never buffered: the camera has
//! ~36 MB of RAM and a bundle is ~19 MB. The transfer is staged as
//! `bundle.tar.part`, fsynced, renamed to `bundle.tar`, and only then is
//! `bundle.trigger` created — its existence is what tells the applier the
//! transfer finished.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::Body;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use http_body_util::BodyExt;
use tokio::io::AsyncWriteExt;

/// Spool directory under `[update] root` (`/mnt/anyka_hack`), matching the
/// anyka-init applier's default.
pub const DEFAULT_SPOOL_ROOT: &str = "/mnt/anyka_hack/spool";

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
            return Err(UpdateError::InFlight);
        }
        Err(e) => return Err(e.into()),
    };

    let written = async {
        let mut body = body;
        while let Some(frame) = body.frame().await {
            let frame = frame.map_err(|e| UpdateError::Body(format!("{e}")))?;
            if let Ok(data) = frame.into_data() {
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

#[derive(Debug, thiserror::Error)]
enum UpdateError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("body stream: {0}")]
    Body(String),
    #[error("another upload is already in progress")]
    InFlight,
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
