use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use config_parser::internal::ConfigFileInternal;
use glue::error::CustomError;
use glue::wasm_memory::WasmMemory;
use http_body_util::{BodyExt, Full};
use hyper::body::{Body, Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use shared::http::{HttpMethod, HttpVersion};
use shared::interop::serialize;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use wasmtime::{Instance, Store};
use wasmtime_wasi::WasiP1Ctx;

const MAX_BODY_SIZE: u64 = 1 << 16; // 64kB

async fn not_found(request: &Request<Incoming>) -> Result<Response<Full<Bytes>>> {
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Full::new(Bytes::from(format!(
            "Couldn't find handler for the route {:?} '{:?}'\n",
            request.method(),
            request.uri()
        ))))?)
}

async fn call_wasm_validator(
    request: Request<Incoming>,
    instance: Arc<Instance>,
    store: Arc<Mutex<Store<WasiP1Ctx>>>,
) -> Result<()> {
    let fct_http_validator = instance
        .get_typed_func::<(i32, i32, i32, i32, i32, i32, i32, i32), i32>(
            &mut *store.lock().await,
            "http_validator",
        )?;

    let headers = request
        .headers()
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_str().unwrap()))
        .collect::<HashMap<String, &str>>();

    let hashmap = WasmMemory::new(&serialize(&headers)?, instance.clone(), store.clone()).await?;
    let arguments = WasmMemory::new(
        &serialize(&{
            let mut map = HashMap::<&str, &str>::new();
            map.insert("secret", "It's a Secret to Everybody");
            map
        })?, // TODO use value from config
        instance.clone(),
        store.clone(),
    )
    .await?;

    let http_method = HttpMethod::try_from(request.method())?;
    let http_version = HttpVersion::try_from(request.version())?;

    let body_wasm = WasmMemory::new(
        &request.collect().await?.to_bytes(),
        instance.clone(),
        store.clone(),
    )
    .await?;

    let request_result = fct_http_validator
        .call_async(
            &mut *store.lock().await,
            (
                body_wasm.ptr(),
                body_wasm.len() as i32,
                hashmap.ptr(),
                hashmap.len() as i32,
                http_method as i32,
                http_version as i32,
                arguments.ptr(),
                arguments.len() as i32,
            ),
        )
        .await?;

    let err_msg = CustomError::from_wasm(instance.clone(), store.clone()).await?;
    dbg!(request_result, err_msg);

    Ok(())
}

async fn validator_request(request: Request<Incoming>) -> Result<Response<Full<Bytes>>> {
    let upper = request.body().size_hint().upper().unwrap_or(u64::MAX);
    if upper > MAX_BODY_SIZE {
        return Ok(Response::builder()
            .status(StatusCode::PAYLOAD_TOO_LARGE)
            .body(Full::new(Bytes::from(format!(
                "Body is too big, max allowed body size is {} bytes, but received a size hint of {} bytes\n",
                MAX_BODY_SIZE, upper
            ))))?);
    }

    // call_wasm_validator(request, instance, store).await?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Full::new(Bytes::new()))?)
}

async fn handle_request(
    config: Arc<ConfigFileInternal>,
    request: Request<Incoming>,
) -> Result<Response<Full<Bytes>>> {
    if request.uri().path() == config.route.path {
        validator_request(request).await
    } else {
        not_found(&request).await
    }
}

pub async fn start(config: Arc<ConfigFileInternal>) -> Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], config.config.expose));

    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;

        println!("Got a new connection");

        let io = TokioIo::new(stream);
        let config = config.clone();

        // let instance = instance.clone();
        // let store = store.clone();
        //
        // tokio::spawn(async move {
        //     if let Err(err) = http1::Builder::new()
        //         .serve_connection(
        //             io,
        //             service_fn(|request| async {
        //                 handle_request(config.clone(), request, instance.clone(), store.clone())
        //                     .await
        //             }),
        //         )
        //         .await
        //     {
        //         eprintln!("Error serving connection: {:?}", err);
        //     }
        // });
    }

    Ok(())
}
