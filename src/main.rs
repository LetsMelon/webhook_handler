use std::sync::Arc;

use anyhow::Result;
use glue::error::CustomError;
use glue::exports::fct_setup;
use tokio::sync::Mutex;
use wasmtime::{Config, Engine, Linker, Module, Store};
use wasmtime_wasi::WasiCtxBuilder;

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
    let out = fct_setup().await?;
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

    server_handle.await??;

    Ok(())
}
