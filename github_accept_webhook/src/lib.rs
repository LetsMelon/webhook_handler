#![feature(cstr_count_bytes)]

use std::collections::HashMap;
use std::fmt::Debug;

use anyhow::Result;
use err_no::{err_clear, set_err_msg_str, set_err_no};
use shared::http::{HttpMethod, HttpVersion};
use shared::interop::deserialize;
use shared::MiddlewareResult;
use tracing::*;

use crate::util::get_slice_from_ptr_and_len_safe;

pub mod err_no;
pub mod memory;
pub mod setup;
mod util;
pub mod verify;

pub struct Request<'a> {
    body: &'a [u8],
    headers: HashMap<&'a str, &'a str>,
    version: HttpVersion,
    method: HttpMethod,
}

impl Debug for Request<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Request")
            .field("body", &self.body.len())
            .field("headers", &self.headers)
            .field("version", &self.version)
            .field("method", &self.method)
            .finish()
    }
}

#[no_mangle]
#[instrument(skip_all)]
pub extern "C" fn http_validator(
    body_ptr: *const u8,
    body_len: u32,
    headers_ptr: *const u8,
    headers_len: u32,
    http_method: HttpMethod,
    http_version: HttpVersion,

    arguments_ptr: *const u8,
    arguments_len: u32,
) -> MiddlewareResult {
    err_clear();

    let Ok(body_slice) = get_slice_from_ptr_and_len_safe(body_ptr, body_len) else {
        return MiddlewareResult::Error;
    };
    let Ok(headers_slice) = get_slice_from_ptr_and_len_safe(headers_ptr, headers_len) else {
        return MiddlewareResult::Error;
    };
    let Ok(arguments_slice) = get_slice_from_ptr_and_len_safe(arguments_ptr, arguments_len) else {
        return MiddlewareResult::Error;
    };

    let headers = match deserialize(headers_slice) {
        Ok(item) => item,
        Err(err) => {
            set_err_no(-2);
            set_err_msg_str(&format!("Deserialize error: {:?}", err));

            return MiddlewareResult::Error;
        }
    };
    let arguments = match deserialize(arguments_slice) {
        Ok(item) => item,
        Err(err) => {
            set_err_no(-3);
            set_err_msg_str(&format!("Deserialize error: {:?}", err));

            return MiddlewareResult::Error;
        }
    };

    info!("Calling the internal validator");

    match handle_request_intern(
        Request {
            body: body_slice,
            headers,
            version: http_version,
            method: http_method,
        },
        arguments,
    ) {
        Ok(_) => MiddlewareResult::Continue,
        Err(err) => {
            set_err_no(1);
            set_err_msg_str(&format!("validator: {:?}", err));

            MiddlewareResult::Error
        }
    }
}

#[inline]
#[instrument(err, ret)]
fn handle_request_intern(request: Request<'static>, arguments: HashMap<&str, &str>) -> Result<()> {
    info!("Finish with the validator");

    Ok(())
}
