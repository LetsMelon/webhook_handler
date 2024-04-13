use std::fmt::Debug;
use std::sync::Arc;

use anyhow::{bail, Result};
use tokio::sync::Mutex;
use wasmtime::{Instance, Store};
use wasmtime_wasi::WasiP1Ctx;

use crate::exports::{fct_alloc, fct_dealloc, get_memory};

async fn copy_slice(
    data: &[u8],
    instance: Arc<Instance>,
    store: Arc<Mutex<Store<WasiP1Ctx>>>,
) -> Result<(i32, usize)> {
    let memory = get_memory(&instance, &mut *store.lock().await)?;

    let alloc = fct_alloc(instance, store.clone()).await?;
    let ptr = alloc(data.len()).await?;

    unsafe {
        let raw = memory
            .data_ptr(&mut *store.lock().await)
            .offset(ptr as isize);
        raw.copy_from(data.as_ptr(), data.len());
    }

    Ok((ptr, data.len()))
}

pub fn get_slice(
    dst: &mut [u8],
    offset: usize,
    mut store: &mut Store<WasiP1Ctx>,
    instance: &Instance,
) -> Result<usize> {
    let memory = get_memory(&instance, &mut store)?;
    let memory_size = memory.data_size(&mut store);

    if offset > memory_size {
        bail!(
            "Can't copy from a offset outside of the memory range, possible range is 0-{}",
            memory_size
        );
    }

    let len = dst.len();
    let data = memory.data_mut(&mut store);
    dst.copy_from_slice(&data[offset..(offset + len).min(memory_size)]);

    let copied_data = (offset + len).min(memory_size) - offset;

    Ok(copied_data)
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

        let (ptr, len) = copy_slice(bytes, instance.clone(), store_clone.clone()).await?;

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
        async fn inner_drop(obj: &WasmMemory) -> Result<()> {
            let dealloc = fct_dealloc(obj.instance.clone(), obj.store.clone()).await?;
            dealloc(obj.ptr(), obj.len()).await?;

            Ok(())
        }

        tokio_async_drop::tokio_async_drop!({
            inner_drop(self).await.unwrap();
        });
    }
}
