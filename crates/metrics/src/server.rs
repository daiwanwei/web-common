use std::{future::Future, net::SocketAddr, str::FromStr, sync::LazyLock};

use axum::{
    body::Body,
    extract::Extension,
    http::{header, HeaderValue},
    response::Response,
    routing, Router,
};
use bytes::{BufMut, BytesMut};
use mime::Mime;
use prometheus::{Encoder, TextEncoder};
use snafu::ResultExt;
use tokio::net::TcpListener;

use crate::{
    error::{self, Error},
    traits,
};

// FIXME: use `OPENMETRICS_TEXT`
#[allow(dead_code)]
static OPENMETRICS_TEXT: LazyLock<Mime> = LazyLock::new(|| {
    Mime::from_str("application/openmetrics-text; version=1.0.0; charset=utf-8")
        .expect("is valid mime type; qed")
});
static ENCODER: LazyLock<TextEncoder> = LazyLock::new(TextEncoder::new);

async fn metrics<Metrics>(Extension(metrics): Extension<Metrics>) -> Response<Body>
where
    Metrics: traits::Metrics + 'static,
{
    let mut buffer = BytesMut::new().writer();
    ENCODER
        .encode(&metrics.gather(), &mut buffer)
        .expect("`Writer<BytesMut>` should not encounter io error; qed");

    let mut res = Response::new(Body::from(buffer.into_inner().freeze()));
    drop(
        res.headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_static(ENCODER.format_type())),
    );
    res
}

fn metrics_index<Metrics>(m: Metrics) -> Router
where
    Metrics: traits::Metrics + 'static,
{
    Router::new().route("/metrics", routing::get(metrics::<Metrics>)).layer(Extension(m))
}

/// # Errors
///
/// * if it cannot bind server
pub async fn run_server<Metrics, ShutdownSignal>(
    listen_address: SocketAddr,
    metrics: Metrics,
    shutdown_signal: ShutdownSignal,
) -> Result<(), Error>
where
    Metrics: Clone + traits::Metrics + Send + 'static,
    ShutdownSignal: Future<Output = ()> + Send + 'static,
{
    let middleware_stack = tower::ServiceBuilder::new();

    let router = Router::new()
        .merge(metrics_index(metrics))
        .layer(middleware_stack)
        .into_make_service_with_connect_info::<SocketAddr>();

    let listener =
        TcpListener::bind(&listen_address).await.context(error::BindMetricsServerSnafu)?;
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal)
        .await
        .map_err(|err| Error::ServeMetricsServer { message: err.to_string() })
}
