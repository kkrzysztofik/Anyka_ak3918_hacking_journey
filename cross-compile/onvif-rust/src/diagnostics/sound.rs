//! `/api/sound` — list configured event clips and trigger playback.

use axum::Extension;
use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::config::sound::SoundConfig;
use crate::platform::PlatformResult;
use crate::platform::sound::{SharedSoundPlayer, SoundPlayResult};

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

/// Status and body label for a play attempt. `Disabled`/`NoClip` carry no label:
/// nothing was asked of the speaker, so there is no playback status to report.
fn play_outcome(result: SoundPlayResult) -> (StatusCode, Option<&'static str>) {
    match result {
        SoundPlayResult::Accepted => (StatusCode::OK, Some("accepted")),
        SoundPlayResult::Debounced => (StatusCode::OK, Some("debounced")),
        SoundPlayResult::Busy => (StatusCode::CONFLICT, Some("busy")),
        SoundPlayResult::Disabled | SoundPlayResult::NoClip => (StatusCode::NOT_FOUND, None),
    }
}

fn play_result_response(result: PlatformResult<SoundPlayResult>) -> axum::response::Response {
    let play = match result {
        Ok(play) => play,
        Err(e) => {
            tracing::warn!(error = %e, "sound play failed");
            return (StatusCode::SERVICE_UNAVAILABLE, "sound playback failed").into_response();
        }
    };
    match play_outcome(play) {
        (status, Some(label)) => {
            (status, Json(PlaySoundResponse { status: label })).into_response()
        }
        (status, None) => (status, "sound unavailable or unknown event").into_response(),
    }
}

/// GET /api/sound
///
/// The player is `None` on stub / non-Anyka builds; report sound as off rather
/// than failing, so the WebUI renders a disabled card instead of an error.
pub async fn handle_get_sound(
    Extension(player): Extension<Option<SharedSoundPlayer>>,
) -> impl IntoResponse {
    match player {
        None => Json(SoundStatusResponse {
            enabled: false,
            events: Vec::new(),
        }),
        Some(player) => Json(status_from_config(player.config())),
    }
}

/// POST /api/sound/play
pub async fn handle_play_sound(
    Extension(player): Extension<Option<SharedSoundPlayer>>,
    Json(body): Json<PlaySoundRequest>,
) -> impl IntoResponse {
    if body.event.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "event must not be empty").into_response();
    }
    let Some(player) = player else {
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
    fn test_play_outcome_maps_every_result_to_status_and_label() {
        assert_eq!(
            play_outcome(SoundPlayResult::Accepted),
            (StatusCode::OK, Some("accepted"))
        );
        assert_eq!(
            play_outcome(SoundPlayResult::Debounced),
            (StatusCode::OK, Some("debounced"))
        );
        assert_eq!(
            play_outcome(SoundPlayResult::Busy),
            (StatusCode::CONFLICT, Some("busy"))
        );
        assert_eq!(
            play_outcome(SoundPlayResult::Disabled),
            (StatusCode::NOT_FOUND, None)
        );
        assert_eq!(
            play_outcome(SoundPlayResult::NoClip),
            (StatusCode::NOT_FOUND, None)
        );
    }

    #[test]
    fn test_play_result_response_maps_sink_error_to_503() {
        let response = play_result_response(Err(PlatformError::HardwareFailure("boom".into())));
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_get_sound_without_player_returns_disabled_empty() {
        let response = handle_get_sound(Extension(None)).await.into_response();
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
        let response = handle_play_sound(
            Extension(None),
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
        let response = handle_play_sound(
            Extension(None),
            Json(PlaySoundRequest { event: "  ".into() }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
