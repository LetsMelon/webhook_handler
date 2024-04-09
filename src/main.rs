use anyhow::{Context, Result};
use wasmtime::{Config, Engine, Instance, Linker, Module, Store};
use wasmtime_wasi::{WasiCtxBuilder, WasiP1Ctx};

mod config_file;
mod server;

#[tokio::main]
async fn main() -> Result<()> {
    let engine = Engine::new(&Config::default().async_support(true))?;

    let mut linker = Linker::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_async(&mut linker, |s| s)?;

    let wasi = WasiCtxBuilder::new().inherit_stdout().build_p1(); // TODO map stdout to maybe log and append with something like: "WASM: "
    let mut store = Store::new(&engine, wasi);

    let module = Module::from_binary(
        &engine,
        include_bytes!("../target/wasm32-wasi/release/github_accept_webhook.wasm"),
    )?;

    linker.module_async(&mut store, "", &module).await?;

    let instance = linker.instantiate_async(&mut store, &module).await?;

    let fct_answer = instance.get_typed_func::<(), i32>(&mut store, "answer")?;
    let result = fct_answer.call_async(&mut store, ()).await?;
    dbg!(result);

    let fct_add = instance.get_typed_func::<(i32, i32), i32>(&mut store, "add")?;
    let result = fct_add.call_async(&mut store, (10, 25)).await?;
    dbg!(result);

    let fct_dealloc = instance.get_typed_func::<(i32, i32), ()>(&mut store, "dealloc")?;

    {
        let (secret_ptr, secret_len) =
            copy_slice(b"It's a Secret to Everybody", &mut store, &instance).await?;
        let (signature_ptr, signature_len) = copy_slice(
            &hex_literal::hex!("757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17"),
            &mut store,
            &instance,
        )
        .await?;
        let (payload_ptr, payload_len) =
            copy_slice(b"Hello, World!", &mut store, &instance).await?;

        let fct_verify =
            instance.get_typed_func::<(i32, i32, i32, i32, i32, i32), i32>(&mut store, "verify")?;
        let result = fct_verify
            .call_async(
                &mut store,
                (
                    secret_ptr as i32,
                    secret_len as i32,
                    signature_ptr as i32,
                    signature_len as i32,
                    payload_ptr as i32,
                    payload_len as i32,
                ),
            )
            .await?;
        dbg!(result);

        fct_dealloc
            .call_async(&mut store, (secret_ptr, secret_len as i32))
            .await?;
        fct_dealloc
            .call_async(&mut store, (signature_ptr, signature_len as i32))
            .await?;
        fct_dealloc
            .call_async(&mut store, (payload_ptr, payload_len as i32))
            .await?;
    }

    {
        let (secret_ptr, secret_len) =
            copy_slice(b"It's a Secret to Everybody", &mut store, &instance).await?;
        let (signature_ptr, signature_len) = copy_slice(
            &hex_literal::hex!("757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17"),
            &mut store,
            &instance,
        )
        .await?;
        let (payload_ptr, payload_len) =
            copy_slice(b"Hello, World?", &mut store, &instance).await?;

        let fct_verify =
            instance.get_typed_func::<(i32, i32, i32, i32, i32, i32), i32>(&mut store, "verify")?;
        let result = fct_verify
            .call_async(
                &mut store,
                (
                    secret_ptr as i32,
                    secret_len as i32,
                    signature_ptr as i32,
                    signature_len as i32,
                    payload_ptr as i32,
                    payload_len as i32,
                ),
            )
            .await?;
        dbg!(result);

        fct_dealloc
            .call_async(&mut store, (secret_ptr, secret_len as i32))
            .await?;
        fct_dealloc
            .call_async(&mut store, (signature_ptr, signature_len as i32))
            .await?;
        fct_dealloc
            .call_async(&mut store, (payload_ptr, payload_len as i32))
            .await?;
    }

    Ok(())
}

async fn copy_slice(
    data: &[u8],
    mut store: &mut Store<WasiP1Ctx>,
    instance: &Instance,
) -> anyhow::Result<(i32, usize)> {
    let memory = instance
        .get_memory(&mut store, "memory")
        .context("expected memory not found")?;

    let fct_alloc = instance.get_typed_func::<i32, i32>(&mut store, "alloc")?;

    let ptr = fct_alloc.call_async(&mut store, data.len() as i32).await?;

    unsafe {
        let raw = memory.data_ptr(&mut store).offset(ptr as isize);
        raw.copy_from(data.as_ptr(), data.len());
    }

    Ok((ptr, data.len()))
}
