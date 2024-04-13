use std::future::Future;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::Mutex;
use wasmtime::{Instance, Memory, Store};
use wasmtime_wasi::WasiP1Ctx;

// use paste::paste;
// macro_rules! wasm_export_function {
//     ($name:ident, input: ($($ident_name:ident: $ident_type:ty),+), output: $output_type:ty) => {
//         paste! {
//             #[doc = "Wrapper function to get the exported function `" $name "` from wasm."]
//             pub async fn [<fct_ $name>](
//                 instance: Arc<Instance>,
//                 store: Arc<Mutex<Store<WasiP1Ctx>>>
//             ) ->  Result<impl FnOnce($($ident_type)*) -> impl Future<Output = Result<$output_type>>>  {
//                 let wasm_fct = instance.get_typed_func(&mut *store.lock().await, stringify!($name))?;
//
//                  Ok(move |$($ident_name: $ident_type),+| async move {
//                      let mut store = store.lock().await;
//
//                      Ok(wasm_fct.call_async(&mut *store, ($($ident_name as i32),*)).await?)
//                  })
//             }
//         }
//     };
// }

// wasm_export_function!(test_fct_1, input: (ptr: i32), output: i32);
// wasm_export_function!(test_fct_2, input: (ptr: i32), output: ());
// wasm_export_function!(test_fct_3, input: (ptr: i32, len: usize), output: ());

pub async fn fct_alloc(
    instance: Arc<Instance>,
    store: Arc<Mutex<Store<WasiP1Ctx>>>,
) -> Result<impl FnOnce(usize) -> impl Future<Output = Result<i32>>> {
    let wasm_fct = instance.get_typed_func::<i32, i32>(&mut *store.lock().await, "alloc")?;

    Ok(move |size| async move {
        let mut store = store.lock().await;

        Ok(wasm_fct.call_async(&mut *store, size as i32).await?)
    })
}

pub async fn fct_dealloc(
    instance: Arc<Instance>,
    store: Arc<Mutex<Store<WasiP1Ctx>>>,
) -> Result<impl FnOnce(i32, usize) -> impl Future<Output = Result<()>>> {
    let wasm_fct =
        instance.get_typed_func::<(i32, i32), ()>(&mut *store.lock().await, "dealloc")?;

    Ok(move |ptr, size| async move {
        let mut store = store.lock().await;

        Ok(wasm_fct.call_async(&mut *store, (ptr, size as i32)).await?)
    })
}

pub async fn fct_get_err_no(
    instance: Arc<Instance>,
    store: Arc<Mutex<Store<WasiP1Ctx>>>,
) -> Result<impl FnOnce() -> impl Future<Output = Result<i32>>> {
    let wasm_fct = instance.get_typed_func::<(), i32>(&mut *store.lock().await, "get_err_no")?;

    Ok(move || async move {
        let mut store = store.lock().await;

        Ok(wasm_fct.call_async(&mut *store, ()).await?)
    })
}

pub async fn fct_get_err_msg(
    instance: Arc<Instance>,
    store: Arc<Mutex<Store<WasiP1Ctx>>>,
) -> Result<impl FnOnce() -> impl Future<Output = Result<i32>>> {
    let wasm_fct = instance.get_typed_func::<(), i32>(&mut *store.lock().await, "get_err_msg")?;

    Ok(move || async move {
        let mut store = store.lock().await;

        Ok(wasm_fct.call_async(&mut *store, ()).await?)
    })
}

pub async fn fct_err_clear(
    instance: Arc<Instance>,
    store: Arc<Mutex<Store<WasiP1Ctx>>>,
) -> Result<impl FnOnce() -> impl Future<Output = Result<()>>> {
    let wasm_fct = instance.get_typed_func::<(), ()>(&mut *store.lock().await, "err_clear")?;

    Ok(move || async move {
        let mut store = store.lock().await;

        Ok(wasm_fct.call_async(&mut *store, ()).await?)
    })
}

#[inline]
pub fn get_memory(instance: &Instance, store: &mut Store<WasiP1Ctx>) -> Result<Memory> {
    instance
        .get_memory(store, "memory")
        .context("expected memory not found")
}
