use std::collections::HashMap;
use std::ffi::CStr;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use glue::wasm_memory::{get_slice, WasmMemory};
use http_body_util::{BodyExt, Full};
use hyper::body::{Body, Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use shared::constants::MAX_ERR_MSG_LEN;
use shared::http::{HttpMethod, HttpVersion};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use wasmtime::{Instance, Store};
use wasmtime_wasi::WasiP1Ctx;

use crate::config_file::ConfigFile;

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

async fn handle_request(
    config: Arc<ConfigFile>,
    request: Request<Incoming>,
    instance: Arc<Instance>,
    store: Arc<Mutex<Store<WasiP1Ctx>>>,
) -> Result<Response<Full<Bytes>>> {
    if request.uri().path() == config.route.path {
        dbg!(&request);

        let upper = request.body().size_hint().upper().unwrap_or(u64::MAX);
        if upper > MAX_BODY_SIZE {
            return Ok(Response::builder()
                .status(StatusCode::PAYLOAD_TOO_LARGE)
                .body(Full::new(Bytes::from(format!(
                    "Body too big, max allowed body size is {} bytes, but received {} bytes\n",
                    MAX_BODY_SIZE, upper
                ))))?);
        }

        let fct_handle_request = instance
            .get_typed_func::<(i32, i32, i32, i32, i32, i32, i32, i32), i32>(
                &mut *store.lock().await,
                "middleware",
            )?;

        let fct_get_err_no =
            instance.get_typed_func::<(), i32>(&mut *store.lock().await, "get_err_no")?;
        let fct_get_err_msg =
            instance.get_typed_func::<(), i32>(&mut *store.lock().await, "get_err_msg")?;

        let headers = request
            .headers()
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_str().unwrap()))
            .collect::<HashMap<String, &str>>();

        let serialized_map = postcard::to_allocvec(&headers).unwrap();
        let hashmap = WasmMemory::new(&serialized_map, instance.clone(), store.clone()).await?;

        let raw_arguments = postcard::to_allocvec(&HashMap::<&str, &str>::new()).unwrap();
        let arguments = WasmMemory::new(&raw_arguments, instance.clone(), store.clone()).await?;

        let body_wasm = WasmMemory::new(
            &request.collect().await?.to_bytes(),
            instance.clone(),
            store.clone(),
        )
        .await?;

        let request_result = fct_handle_request
            .call_async(
                &mut *store.lock().await,
                (
                    body_wasm.ptr(),
                    body_wasm.len() as i32,
                    hashmap.ptr(),
                    hashmap.len() as i32,
                    HttpMethod::POST as i32,
                    HttpVersion::Http1_1 as i32,
                    arguments.ptr(),
                    arguments.len() as i32,
                ),
            )
            .await?;
        dbg!(request_result);

        let err_no = fct_get_err_no
            .call_async(&mut *store.lock().await, ())
            .await?;

        if request_result != 0 || err_no != 0 {
            let err_msg_ptr = fct_get_err_msg
                .call_async(&mut *store.lock().await, ())
                .await?;

            let mut dst = [0u8; MAX_ERR_MSG_LEN];
            get_slice(
                &mut dst,
                err_msg_ptr as u32,
                &mut *store.lock().await,
                &instance,
            )?;

            let cstr = CStr::from_bytes_until_nul(&dst)?;
            let raw_str = cstr.to_str()?;
            dbg!(raw_str);
        }

        Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::new()))?)
    } else {
        not_found(&request).await
    }
}

pub async fn start(instance: Arc<Instance>, store: Arc<Mutex<Store<WasiP1Ctx>>>) -> Result<()> {
    let config = Arc::new(crate::config_file::parse_config_file(
        "./webhook_handler_demo_config.yml",
    )?);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;

        let io = TokioIo::new(stream);

        let config = config.clone();

        let instance = instance.clone();
        let store = store.clone();

        tokio::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(|request| async {
                        handle_request(config.clone(), request, instance.clone(), store.clone())
                            .await
                    }),
                )
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }

    Ok(())
}
