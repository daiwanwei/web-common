use std::{
    convert::Infallible,
    future::{Future, IntoFuture},
    net::SocketAddr,
};

use axum::{
    extract::Request,
    response::Response,
    serve::{IncomingStream, WithGracefulShutdown},
};
use snafu::ResultExt;
use tokio::net::TcpListener;
use tower::Service;
use web_grpc::health_check::HealthServer;

use crate::error::{self, Error};

/// # Errors
///
/// * if it cannot bind server
pub async fn run_web_server<R, S, ShutdownSignal>(
    listen_address: SocketAddr,
    service: R,
    shutdown_signal: ShutdownSignal,
) -> Result<(), Error>
where
    R: for<'a> Service<IncomingStream<'a>, Error = Infallible, Response = S>,
    S: Service<Request, Response = Response, Error = Infallible> + Clone + Send + 'static,
    S::Future: Send,
    WithGracefulShutdown<R, S, ShutdownSignal>: IntoFuture<Output = Result<(), std::io::Error>>,
    ShutdownSignal: Future<Output = ()> + Send + 'static,
{
    let listener = TcpListener::bind(&listen_address).await.context(error::BindTcpServerSnafu)?;
    axum::serve(listener, service)
        .with_graceful_shutdown(shutdown_signal)
        .await
        .map_err(|err| Error::ServeHttpServer { message: err.to_string() })
}

pub async fn run_health_check_server<HealthCheckService, ShutdownSignal>(
    listen_address: SocketAddr,
    srv: HealthServer<HealthCheckService>,
    shutdown_signal: ShutdownSignal,
) -> Result<(), Error>
where
    HealthCheckService: Send + Sync + 'static + web_grpc::health_check::Health,
    ShutdownSignal: Future<Output = ()> + Send + 'static,
{
    tonic::transport::Server::builder()
        .add_service(srv)
        .serve_with_shutdown(listen_address, shutdown_signal)
        .await
        .map_err(|err| Error::ServeGrpcServer { message: err.to_string() })
}
