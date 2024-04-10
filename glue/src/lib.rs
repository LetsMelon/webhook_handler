use std::fmt::Debug;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::Mutex;
use wasmtime::{Instance, Store};
use wasmtime_wasi::WasiP1Ctx;

async fn copy_slice(
    data: &[u8],
    mut store: &mut Store<WasiP1Ctx>,
    instance: &Instance,
) -> Result<(i32, usize)> {
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

pub struct WasmMemory {
    ptr: i32,
    len: usize,

    instance: Arc<Instance>,
    store: Arc<Mutex<Store<WasiP1Ctx>>>,
}

impl Debug for WasmMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmMemory")
            .field("ptr", &self.ptr)
            .field("len", &self.len)
            .finish()
    }
}

impl WasmMemory {
    pub async fn new(
        bytes: &[u8],
        instance: Arc<Instance>,
        store: Arc<Mutex<Store<WasiP1Ctx>>>,
    ) -> Result<Self> {
        let store_clone = store.clone();

        let mut store = store.lock().await;
        let (ptr, len) = copy_slice(bytes, &mut store, &instance).await?;

        Ok(WasmMemory {
            ptr,
            len,
            instance,
            store: store_clone,
        })
    }

    pub fn ptr(&self) -> i32 {
        self.ptr
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl Drop for WasmMemory {
    fn drop(&mut self) {
        tokio_async_drop::tokio_async_drop!({
            let mut store = self.store.lock().await;

            let fct_dealloc = self
                .instance
                .get_typed_func::<(i32, i32), ()>(&mut *store, "dealloc")
                .unwrap();

            fct_dealloc
                .call_async(&mut *store, (self.ptr, self.len as i32))
                .await
                .unwrap();
        });
    }
}
