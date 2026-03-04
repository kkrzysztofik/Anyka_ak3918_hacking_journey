//! Response handling module for the dispatcher.
//!
//! This module contains functions for building SOAP responses and handling errors.

use axum::{
    http::StatusCode,
    http::header,
    response::{IntoResponse, Response},
};

use crate::onvif::dispatcher::ServiceHandler;
use crate::onvif::error::OnvifError;
use std::sync::Arc;

/// Build an error response from an OnvifError.
pub(super) fn error_response(error: OnvifError) -> Response {
    let status = error.http_status();
    let fault_xml = error.to_soap_fault();

    (
        status,
        [(header::CONTENT_TYPE, "application/soap+xml; charset=utf-8")],
        fault_xml,
    )
        .into_response()
}

/// Handle the operation and return response.
pub(super) async fn handle_operation_self(
    handler: &Arc<dyn ServiceHandler>,
    action: &str,
    body_xml: &str,
) -> Response {
    match handler.handle_operation(action, body_xml).await {
        Ok(response_body) => {
            let response_xml = crate::onvif::soap::build_soap_response(&response_body);
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/soap+xml; charset=utf-8")],
                response_xml,
            )
                .into_response()
        }
        Err(e) => {
            tracing::warn!("Operation {} failed: {:?}", action, e);
            error_response(e)
        }
    }
}
