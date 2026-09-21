//! `/api/imaging` — the advanced ISP knobs ONVIF does not model.
//!
//! ONVIF's `ImagingSettings20` has no hue, no mains frequency and no picture
//! style, but the WebUI's Imaging tab promises all three, so they travel
//! over this JSON endpoint while the SOAP surface keeps modelling what it
//! can. GET/PUT are both mounted only behind the auth middleware, like
//! `/api/network` (a state change must not exist at all when auth is off).

use axum::Json;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::diagnostics::state::DiagnosticsState;

/// Current values, ONVIF-style (hue 0-100, 50 = neutral).
#[derive(Debug, Clone, Serialize)]
pub struct AdvancedImagingView {
    pub hue: f32,
    pub power_hz: u16,
    pub style_id: u8,
}

/// Partial patch; absent fields are left alone.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AdvancedImagingPatch {
    pub hue: Option<f32>,
    pub power_hz: Option<u16>,
    pub style_id: Option<u8>,
}

/// JSON `{"error": message}` with the status chosen at the failure site.
#[derive(Debug)]
pub(crate) struct ApiError {
    status: StatusCode,
    message: String,
}

#[derive(Debug, Serialize)]
struct ApiErrorBody {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ApiErrorBody {
                error: self.message,
            }),
        )
            .into_response()
    }
}

async fn imaging_control(
    state: &DiagnosticsState,
) -> Result<std::sync::Arc<dyn crate::platform::ImagingControl>, ApiError> {
    state
        .platform()
        .and_then(|p| p.imaging_control())
        .ok_or_else(|| ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            message: "imaging control unavailable".to_string(),
        })
}

fn platform_error_to_api(e: crate::platform::PlatformError) -> ApiError {
    // Client-addressable rejections (hue out of range, hz not 50/60, style
    // out of range): a 400 the UI can surface, not a 500.
    match e {
        crate::platform::PlatformError::InvalidParameter(msg) => ApiError {
            status: StatusCode::BAD_REQUEST,
            message: msg,
        },
        other => ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: other.to_string(),
        },
    }
}

/// GET /api/imaging
pub(crate) async fn handle_get_advanced_imaging(
    Extension(state): Extension<std::sync::Arc<DiagnosticsState>>,
) -> Result<Json<AdvancedImagingView>, ApiError> {
    let control = imaging_control(&state).await?;
    let settings = control.get_settings().await.map_err(|e| {
        tracing::warn!(error = %e, "GET /api/imaging: reading settings failed");
        platform_error_to_api(e)
    })?;
    Ok(Json(AdvancedImagingView {
        hue: settings.hue,
        power_hz: settings.power_hz,
        style_id: settings.style_id,
    }))
}

/// PUT /api/imaging
pub(crate) async fn handle_put_advanced_imaging(
    Extension(state): Extension<std::sync::Arc<DiagnosticsState>>,
    Json(patch): Json<AdvancedImagingPatch>,
) -> Result<Json<AdvancedImagingView>, ApiError> {
    let control = imaging_control(&state).await?;
    let current = control.get_settings().await.map_err(|e| {
        tracing::warn!(error = %e, "PUT /api/imaging: reading settings failed");
        platform_error_to_api(e)
    })?;
    let mut next = current;
    if let Some(hue) = patch.hue {
        next.hue = hue;
    }
    if let Some(hz) = patch.power_hz {
        next.power_hz = hz;
    }
    if let Some(style) = patch.style_id {
        next.style_id = style;
    }

    match control.set_settings(&next).await {
        Ok(()) => {
            let applied = control.get_settings().await.map_err(|e| {
                tracing::warn!(error = %e, "PUT /api/imaging: readback failed");
                platform_error_to_api(e)
            })?;
            Ok(Json(AdvancedImagingView {
                hue: applied.hue,
                power_hz: applied.power_hz,
                style_id: applied.style_id,
            }))
        }
        Err(e) => {
            tracing::warn!(error = %e, "PUT /api/imaging: set failed");
            Err(platform_error_to_api(e))
        }
    }
}
