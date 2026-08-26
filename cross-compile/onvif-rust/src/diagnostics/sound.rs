//! `/api/sound` — list configured event clips and trigger playback.

use std::sync::Arc;

use axum::Extension;
use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::config::sound::SoundConfig;
use crate::platform::PlatformResult;
use crate::platform::sound::{SharedSoundPlayer, SoundPlayResult};

/// Shared state for sound REST handlers.
pub struct SoundApiState {
    /// `None` on stub / non-Anyka builds.
    player: Option<SharedSoundPlayer>,
}

impl SoundApiState {
    pub fn empty() -> Self {
        Self { player: None }
    }

    pub fn new(player: Option<SharedSoundPlayer>) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SoundStatusResponse {
    pub enabled: bool,
    pub events: Vec<SoundEventItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SoundEventItem {
    pub id: String,
    pub clip: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlaySoundRequest {
    pub event: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PlaySoundResponse {
    pub status: &'static str,
}

fn status_from_config(cfg: &SoundConfig) -> SoundStatusResponse {
    let mut events: Vec<SoundEventItem> = cfg
        .events
        .iter()
        .map(|(id, clip)| SoundEventItem {
            id: id.clone(),
            clip: clip.clone(),
        })
        .collect();
    events.sort_by(|a, b| a.id.cmp(&b.id));
    SoundStatusResponse {
        enabled: cfg.enabled,
        events,
    }
}

fn http_status_for_play(result: PlatformResult<SoundPlayResult>) -> StatusCode {
    match result {
        Ok(SoundPlayResult::Accepted | SoundPlayResult::Debounced) => StatusCode::OK,
        Ok(SoundPlayResult::Busy) => StatusCode::CONFLICT,
        Ok(SoundPlayResult::Disabled | SoundPlayResult::NoClip) => StatusCode::NOT_FOUND,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

fn play_status_label(result: SoundPlayResult) -> Option<&'static str> {
    match result {
        SoundPlayResult::Accepted => Some("accepted"),
        SoundPlayResult::Busy => Some("busy"),
        SoundPlayResult::Debounced => Some("debounced"),
        SoundPlayResult::Disabled | SoundPlayResult::NoClip => None,
    }
}

fn play_result_response(result: PlatformResult<SoundPlayResult>) -> axum::response::Response {
    match result {
        Ok(play) => {
            let status = http_status_for_play(Ok(play));
            match play_status_label(play) {
                Some(label) => (status, Json(PlaySoundResponse { status: label })).into_response(),
                None => (status, "sound unavailable or unknown event").into_response(),
            }
        }
        Err(e) => (StatusCode::SERVICE_UNAVAILABLE, e.to_string()).into_response(),
    }
}

/// GET /api/sound
pub async fn handle_get_sound(
    Extension(state): Extension<Arc<SoundApiState>>,
) -> impl IntoResponse {
    match &state.player {
        None => Json(SoundStatusResponse {
            enabled: false,
            events: Vec::new(),
        }),
        Some(player) => Json(status_from_config(player.config())),
    }
}

/// POST /api/sound/play
pub async fn handle_play_sound(
    Extension(state): Extension<Arc<SoundApiState>>,
    Json(body): Json<PlaySoundRequest>,
) -> impl IntoResponse {
    if body.event.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "event must not be empty").into_response();
    }
    let Some(player) = &state.player else {
        return (StatusCode::SERVICE_UNAVAILABLE, "sound player unavailable").into_response();
    };
    play_result_response(player.play(body.event.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::PlatformError;

    fn cfg_with_events(enabled: bool, events: &[(&str, &str)]) -> SoundConfig {
        let mut c = SoundConfig {
            enabled,
            ..SoundConfig::default()
        };
        for (id, clip) in events {
            c.events.insert((*id).into(), (*clip).into());
        }
        c
    }

    #[test]
    fn test_status_from_config_sorts_events_by_id_and_copies_enabled() {
        let cfg = cfg_with_events(
            true,
            &[
                ("zebra", "z.raw"),
                ("boot_ready", "boot.raw"),
                ("alert", "a.raw"),
            ],
        );
        let status = status_from_config(&cfg);
        assert!(status.enabled);
        assert_eq!(
            status.events,
            vec![
                SoundEventItem {
                    id: "alert".into(),
                    clip: "a.raw".into(),
                },
                SoundEventItem {
                    id: "boot_ready".into(),
                    clip: "boot.raw".into(),
                },
                SoundEventItem {
                    id: "zebra".into(),
                    clip: "z.raw".into(),
                },
            ]
        );
    }

    #[test]
    fn test_http_status_for_play_accepted_is_ok() {
        assert_eq!(
            http_status_for_play(Ok(SoundPlayResult::Accepted)),
            StatusCode::OK
        );
    }

    #[test]
    fn test_http_status_for_play_busy_is_conflict() {
        assert_eq!(
            http_status_for_play(Ok(SoundPlayResult::Busy)),
            StatusCode::CONFLICT
        );
    }

    #[test]
    fn test_http_status_for_play_debounced_is_ok_with_debounced_label() {
        assert_eq!(
            http_status_for_play(Ok(SoundPlayResult::Debounced)),
            StatusCode::OK
        );
        assert_eq!(
            play_status_label(SoundPlayResult::Debounced),
            Some("debounced")
        );
    }

    #[test]
    fn test_http_status_for_play_disabled_and_noclip_are_not_found() {
        assert_eq!(
            http_status_for_play(Ok(SoundPlayResult::Disabled)),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            http_status_for_play(Ok(SoundPlayResult::NoClip)),
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn test_http_status_for_play_err_is_service_unavailable() {
        assert_eq!(
            http_status_for_play(Err(PlatformError::HardwareFailure("boom".into()))),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[tokio::test]
    async fn test_get_sound_without_player_returns_disabled_empty() {
        let state = Arc::new(SoundApiState::empty());
        let response = handle_get_sound(Extension(state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let parsed: SoundStatusResponse = serde_json::from_slice(&body).expect("json");
        assert!(!parsed.enabled);
        assert!(parsed.events.is_empty());
    }

    #[tokio::test]
    async fn test_play_sound_without_player_returns_503() {
        let state = Arc::new(SoundApiState::empty());
        let response = handle_play_sound(
            Extension(state),
            Json(PlaySoundRequest {
                event: "boot_ready".into(),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_play_sound_empty_event_returns_400() {
        let state = Arc::new(SoundApiState::empty());
        let response = handle_play_sound(
            Extension(state),
            Json(PlaySoundRequest { event: "  ".into() }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
