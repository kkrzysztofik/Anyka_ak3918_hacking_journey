use axum::{
  extract::{ConnectInfo, Request},
  http::{Method, StatusCode, Uri},
  response::{IntoResponse, Response},
  routing::{get, post},
  Router,
};
use quick_xml::se::to_string;
use serde::Serialize;
use std::net::SocketAddr;
use tokio::signal;
use tracing::{error, info, warn};

mod ffi;
mod onvif;

#[derive(Serialize)]
#[serde(rename = "response")]
struct XmlResponse {
  #[serde(rename = "message")]
  message: String,
}

#[tokio::main]
async fn main() {
  // Initialize tracing
  tracing_subscriber::fmt()
    .with_env_filter("rust_hello=info")
    .init();

  println!("Hello, Anyka ARM World!");
  println!("Target: {}", std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string()));
  println!("Architecture: ARMv5TEJ (no VFP support)");

  // Test FFI functionality
  unsafe {
    ffi::hello_c();
  }

  let app = Router::new()
    .route("/", get(hello_handler))
    .route("/onvif/device_service", post(onvif_device_service_handler));

  let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
  info!("Starting HTTP server on port 8080...");

  let listener = tokio::net::TcpListener::bind(&addr).await.unwrap_or_else(|e| {
    error!("Failed to bind to {}: {}", addr, e);
    std::process::exit(1);
  });

  let server = axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>());

  // Set up signal handling for graceful shutdown
  let shutdown_signal = async {
    let ctrl_c = async {
      signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
      signal::unix::signal(signal::unix::SignalKind::terminate())
        .expect("failed to install signal handler")
        .recv()
        .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
      _ = ctrl_c => {},
      _ = terminate => {},
    }

    info!("Received shutdown signal. Initiating graceful shutdown...");
  };

  tokio::select! {
    result = server => {
      if let Err(e) = result {
        error!("Server error: {}", e);
      }
    }
    _ = shutdown_signal => {
      info!("Shutdown signal received");
    }
  }

  info!("Server shutdown gracefully");
}

async fn hello_handler(
  ConnectInfo(addr): ConnectInfo<SocketAddr>,
  request: Request,
) -> Response {
  let start_time = std::time::Instant::now();

  let method = request.method().clone();
  let uri = request.uri().clone();
  let headers = request.headers().clone();

  // Log request details
  log_request_details(&method, &uri, &headers, &request, addr);

  // Check method
  if method != Method::GET {
    warn!("Method not allowed: {}", method);
    return (
      StatusCode::METHOD_NOT_ALLOWED,
      "Method not allowed",
    )
      .into_response();
  }

  // Check path
  if uri.path() != "/" {
    warn!("Path not found: {}", uri.path());
    return (StatusCode::NOT_FOUND, "Not Found").into_response();
  }

  // Create response
  let resp = XmlResponse {
    message: "hello-world".to_string(),
  };

  // Serialize to XML
  let xml_header = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
  let xml_body = match to_string(&resp) {
    Ok(body) => body,
    Err(e) => {
      error!("Error encoding XML: {}", e);
      return (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Internal server error",
      )
        .into_response();
    }
  };

  let xml_response = format!("{}{}", xml_header, xml_body);

  // Log response details
  let duration = start_time.elapsed();
  let response_size = xml_response.len();
  log_response_details(StatusCode::OK, response_size, duration);

  // Return response
  (
    StatusCode::OK,
    [("Content-Type", "application/xml; charset=utf-8")],
    xml_response,
  )
    .into_response()
}

fn log_request_details(
  method: &Method,
  uri: &axum::http::Uri,
  headers: &axum::http::HeaderMap,
  request: &Request,
  addr: SocketAddr,
) {
  let remote_addr = addr.to_string();

  let forwarded_for = headers
    .get("x-forwarded-for")
    .and_then(|v| v.to_str().ok())
    .map(|v| format!("{} (forwarded from {})", v, remote_addr))
    .unwrap_or(remote_addr);

  info!(
    "[REQUEST] Method: {} | Path: {} | RemoteAddr: {} | Protocol: {:?}",
    method,
    uri.path(),
    forwarded_for,
    request.version()
  );

  // Log query parameters if present
  if !uri.query().unwrap_or("").is_empty() {
    info!("[REQUEST] Query: {}", uri.query().unwrap());
  }

  // Log important headers
  let important_headers = vec![
    "user-agent",
    "host",
    "accept",
    "accept-encoding",
    "content-type",
    "content-length",
  ];

  for header_name in &important_headers {
    if let Some(value) = headers.get(*header_name) {
      if let Ok(value_str) = value.to_str() {
        info!("[REQUEST] Header {}: {}", header_name, value_str);
      }
    }
  }

  // Log other headers
  let mut other_headers = Vec::new();
  for (name, value) in headers.iter() {
    let name_lower = name.as_str().to_lowercase();
    if !important_headers.contains(&name_lower.as_str()) {
      if let Ok(value_str) = value.to_str() {
        other_headers.push(format!("{}: {}", name, value_str));
      }
    }
  }

  if !other_headers.is_empty() {
    info!("[REQUEST] Other headers: {}", other_headers.join(" | "));
  }
}

fn log_response_details(status: StatusCode, size: usize, duration: std::time::Duration) {
  let status_text = status
    .canonical_reason()
    .unwrap_or("Unknown");

  info!(
    "[RESPONSE] Status: {} {} | Size: {} bytes | Duration: {:?}",
    status.as_u16(),
    status_text,
    size,
    duration
  );
}

/// ONVIF Device Service handler
async fn onvif_device_service_handler(
  ConnectInfo(addr): ConnectInfo<SocketAddr>,
  method: Method,
  uri: Uri,
  _headers: axum::http::HeaderMap,
  body: String,
) -> Response {
  let start_time = std::time::Instant::now();

  // Log request details (simplified version for ONVIF)
  info!(
    "[ONVIF REQUEST] Method: {} | Path: {} | RemoteAddr: {}",
    method,
    uri.path(),
    addr
  );

  // Only accept POST requests
  if method != Method::POST {
    warn!("ONVIF Device Service: Method not allowed: {}", method);
    return (
      StatusCode::METHOD_NOT_ALLOWED,
      [("Content-Type", "application/soap+xml; charset=utf-8")],
      onvif::generate_soap_fault("Client", "Method not allowed"),
    )
      .into_response();
  }

  // Parse SOAP operation
  let operation = match onvif::parse_soap_operation(&body) {
    Some(op) => op,
    None => {
      warn!("ONVIF Device Service: Could not parse SOAP operation from request");
      return (
        StatusCode::BAD_REQUEST,
        [("Content-Type", "application/soap+xml; charset=utf-8")],
        onvif::generate_soap_fault("Client", "Invalid SOAP request: could not parse operation"),
      )
        .into_response();
    }
  };

  info!("ONVIF Device Service: Operation requested: {}", operation);

  // Handle GetDeviceInformation operation
  if operation == "GetDeviceInformation" {
    let device_info = onvif::DeviceInformation::default();
    let soap_response = onvif::generate_get_device_information_response(&device_info);

    let duration = start_time.elapsed();
    let response_size = soap_response.len();
    log_response_details(StatusCode::OK, response_size, duration);

    return (
      StatusCode::OK,
      [("Content-Type", "application/soap+xml; charset=utf-8")],
      soap_response,
    )
      .into_response();
  }

  // Unknown operation
  warn!("ONVIF Device Service: Unknown operation: {}", operation);
  let duration = start_time.elapsed();
  log_response_details(StatusCode::BAD_REQUEST, 0, duration);

  (
    StatusCode::BAD_REQUEST,
    [("Content-Type", "application/soap+xml; charset=utf-8")],
    onvif::generate_soap_fault("Client", &format!("Unknown operation: {}", operation)),
  )
    .into_response()
}
