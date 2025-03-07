use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Error occurs while binding TCP server, error: {source}"))]
    BindTcpServer { source: std::io::Error },

    #[snafu(display("Error occurs while serving HTTP server, error: {message}"))]
    ServeHttpServer { message: String },

    #[snafu(display("Error occurs while serving gRPC server, error: {message}"))]
    ServeGrpcServer { message: String },
}
