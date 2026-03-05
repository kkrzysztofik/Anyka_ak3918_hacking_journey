use {
    super::HttpFlv,
    crate::common::auth::Auth,
    crate::hub::define::StreamHubEventSender,
    async_trait::async_trait,
    axum::{
        body::Body,
        extract::{ConnectInfo, Request, State},
        handler::Handler,
        http::{HeaderValue, StatusCode},
        response::Response,
    },
    futures::channel::mpsc::unbounded,
    std::net::SocketAddr,
    tokio::net::TcpListener,
};

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;
static NOTFOUND: &[u8] = b"Not Found";
static UNAUTHORIZED: &[u8] = b"Unauthorized";
static BADREQUEST: &[u8] = b"Bad Request";

pub(crate) async fn handle_connection(
    State((event_producer, auth)): State<(StreamHubEventSender, Option<Auth>)>, // event_producer: ChannelEventProducer
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    req: Request<Body>,
) -> Response<Body> {
    let path = req.uri().path();
    let query_string: Option<String> = req.uri().query().map(|s| s.to_string());
    let authorization_header = req
        .headers()
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());

    match path.find(".flv") {
        Some(index) if index > 0 => {
            let (left, _) = path.split_at(index);
            let rv: Vec<_> = left.split('/').collect();

            if rv.len() < 3 {
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(BADREQUEST.into())
                    .unwrap_or_else(|_| Response::new(Body::from(BADREQUEST)));
            }

            let app_name = String::from(rv[1]);
            let stream_name = String::from(rv[2]);

            if let Some(auth_val) = auth
                && auth_val
                    .authenticate_request(
                        &stream_name,
                        &query_string,
                        authorization_header.as_deref(),
                        true,
                    )
                    .is_err()
            {
                return Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .header("WWW-Authenticate", auth_val.basic_challenge())
                    .body(UNAUTHORIZED.into())
                    .unwrap_or_else(|_| Response::new(Body::from(UNAUTHORIZED)));
            }

            let (http_response_data_producer, http_response_data_consumer) = unbounded();

            let mut flv_handler = HttpFlv::new(
                app_name,
                stream_name,
                event_producer,
                http_response_data_producer,
                req.uri().to_string(),
                remote_addr,
            );

            tokio::spawn(async move {
                if let Err(err) = flv_handler.run().await {
                    tracing::error!(error = %err, "flv_handler_error");
                }
            });

            let mut resp = Response::new(Body::from_stream(http_response_data_consumer));
            resp.headers_mut()
                .insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
            resp.headers_mut()
                .insert("Content-Type", HeaderValue::from_static("video/x-flv"));
            resp.headers_mut()
                .insert("Cache-Control", HeaderValue::from_static("no-cache"));

            resp
        }

        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(NOTFOUND.into())
            .unwrap_or_else(|_| Response::new(Body::from(NOTFOUND))),
    }
}

/// Trait for HTTP-FLV server implementations
///
/// This trait provides the abstraction for HTTP-FLV server functionality,
/// allowing platform-specific implementations (e.g., Anyka hardware integration).
#[async_trait]
pub trait HttpFlvServer: Send + Sync {
    /// Create a new HTTP-FLV server instance
    fn new(address: String, event_producer: StreamHubEventSender, auth: Option<Auth>) -> Self
    where
        Self: Sized;

    /// Run the HTTP-FLV server
    async fn run(&mut self) -> Result<()>;
}

/// Default implementation of the HTTP-FLV server trait
pub struct DefaultHttpFlvServer {
    address: String,
    event_producer: StreamHubEventSender,
    auth: Option<Auth>,
}

#[async_trait]
impl HttpFlvServer for DefaultHttpFlvServer {
    /// Create a new HTTP-FLV server instance
    fn new(address: String, event_producer: StreamHubEventSender, auth: Option<Auth>) -> Self {
        Self {
            address,
            event_producer,
            auth,
        }
    }

    /// Run the HTTP-FLV server
    async fn run(&mut self) -> Result<()> {
        let sock_addr: SocketAddr = self.address.parse()?;

        let listener = TcpListener::bind(sock_addr).await?;

        tracing::info!(address = %sock_addr, "httpflv_server_start");

        let handle_connection =
            handle_connection.with_state((self.event_producer.clone(), self.auth.clone()));

        axum::serve(
            listener,
            handle_connection.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await?;

        Ok(())
    }
}

// Legacy function for backward compatibility
pub async fn run(
    event_producer: StreamHubEventSender,
    port: usize,
    auth: Option<Auth>,
) -> Result<()> {
    let listen_address = format!("0.0.0.0:{port}");
    let mut server = DefaultHttpFlvServer::new(listen_address, event_producer, auth);
    server.run().await
}
