use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use crate::config_file::ConfigFile;

async fn not_found(request: &Request<Incoming>) -> Result<Response<Full<Bytes>>> {
    Ok(Response::builder()
        .status(404)
        .body(Full::new(Bytes::from(format!(
            "Couldn't find handler for the route {:?} '{:?}'\n",
            request.method(),
            request.uri()
        ))))?)
}

async fn handle_request(
    config: Arc<ConfigFile>,
    request: Request<Incoming>,
) -> Result<Response<Full<Bytes>>> {
    not_found(&request).await
}

#[tokio::main]
pub async fn start() -> Result<()> {
    let config = Arc::new(crate::config_file::parse_config_file(
        "./webhook_handler_demo_config.yml",
    )?);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;

        let io = TokioIo::new(stream);

        let config = config.clone();

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(|request| async { handle_request(config.clone(), request).await }),
                )
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }

    Ok(())
}
