use std::ffi::CStr;
use std::sync::Arc;

use anyhow::Result;
use shared::constants::MAX_ERR_MSG_LEN;
use tokio::sync::Mutex;
use wasmtime::{Instance, Store};
use wasmtime_wasi::WasiP1Ctx;

use crate::exports::{fct_err_clear, fct_get_err_msg, fct_get_err_no};
use crate::wasm_memory::get_slice;

#[derive(Debug)]
pub struct CustomError {
    code: i32,
    msg: String,
}

impl CustomError {
    pub async fn from_wasm(
        instance: Arc<Instance>,
        store: Arc<Mutex<Store<WasiP1Ctx>>>,
    ) -> Result<Option<Self>> {
        let fct_err_no = fct_get_err_no(instance.clone(), store.clone()).await?;
        let fct_err_msg = fct_get_err_msg(instance.clone(), store.clone()).await?;
        let fct_err_clear = fct_err_clear(instance.clone(), store.clone()).await?;

        let err_no = fct_err_no().await?;

        let new_self = if err_no != 0 {
            let msg_ptr = fct_err_msg().await?;

            let mut dst = [0u8; MAX_ERR_MSG_LEN];
            let copied_bytes_from_wasm = get_slice(
                &mut dst,
                msg_ptr as usize,
                &mut *store.lock().await,
                &instance,
            )?;

            let cstr = CStr::from_bytes_until_nul(&dst[0..copied_bytes_from_wasm])?;
            let raw_str = cstr.to_str()?.to_string();

            Ok(Some(CustomError {
                code: err_no,
                msg: raw_str,
            }))
        } else {
            Ok(None)
        };

        fct_err_clear().await?;

        new_self
    }
}
