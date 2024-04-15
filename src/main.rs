use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::sync::Arc;

use anyhow::Result;
use glue::error::CustomError;
use glue::exports::fct_setup;
use glue::wasm_memory::WasmMemory;
use tokio::sync::Mutex;
use wasmtime::{Config, Engine, Instance, Linker, Module, Store};
use wasmtime_wasi::{WasiCtxBuilder, WasiP1Ctx};

mod config_file;
mod server;

#[tokio::main]
async fn main() -> Result<()> {
    let engine = Engine::new(
        Config::default()
            .async_support(true)
            .dynamic_memory_guard_size(1 << 24),
    )?;

    let buffer = tokio::fs::read("./target/wasm32-wasi/release/github_accept_webhook.wasm").await?;

    let mut linker = Linker::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_async(&mut linker, |s| s)?;

    let wasi = WasiCtxBuilder::new()
        .inherit_stderr()
        .inherit_stdout() // TODO map stdout to maybe log and append with something like: "WASM: "
        .build_p1();
    let mut store = Store::new(&engine, wasi);

    let module = Module::from_binary(&engine, &buffer)?;

    linker.module_async(&mut store, "", &module).await?;

    let instance = linker.instantiate_async(&mut store, &module).await?;

    let instance = Arc::new(instance);
    let store = Arc::new(Mutex::new(store));

    let fct_setup = fct_setup(instance.clone(), store.clone()).await?;
    let out = fct_setup().await?; // TODO error handling: get the error message from wasm
    if out != 0 {
        let error = CustomError::from_wasm(instance.clone(), store.clone())
            .await?
            .unwrap();
        dbg!(error);

        panic!("Can't init the wasm module");
    }

    let server_handle = tokio::spawn({
        let instance = instance.clone();
        let store = store.clone();

        async { crate::server::start(instance, store).await }
    });

    let verify_result = verify(
        b"It's a Secret to Everybody",
        b"Hello World!",
        {
            let mut map = HashMap::new();

            map.insert(
                "x-hub-signature-256",
                "sha256=757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17",
            );

            map
        },
        instance.clone(),
        store.clone(),
    )
    .await?;
    dbg!(verify_result);

    server_handle.await??;

    Ok(())
}

async fn verify(
    secret: &[u8],
    payload: &[u8],
    hashmap: HashMap<&str, &str>,
    instance: Arc<Instance>,
    store: Arc<Mutex<Store<WasiP1Ctx>>>,
) -> anyhow::Result<i32> {
    let secret = WasmMemory::new(secret, instance.clone(), store.clone()).await?;
    let payload = WasmMemory::new(payload, instance.clone(), store.clone()).await?;

    let serialized_map = postcard::to_allocvec(&hashmap).unwrap();
    let hashmap = WasmMemory::new(&serialized_map, instance.clone(), store.clone()).await?;

    let fct_verify = instance.get_typed_func::<(i32, i32, i32, i32, i32, i32), i32>(
        &mut *store.lock().await,
        "verify",
    )?;

    let result = fct_verify
        .call_async(
            &mut *store.lock().await,
            (
                secret.ptr(),
                secret.len() as i32,
                payload.ptr(),
                payload.len() as i32,
                hashmap.ptr(),
                hashmap.len() as i32,
            ),
        )
        .await?;

    Ok(result)
}
