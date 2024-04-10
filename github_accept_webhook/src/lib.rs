#![feature(cstr_count_bytes)]

use std::collections::HashMap;

use anyhow::{bail, Result};
use err_no::{err_clear, set_err_msg_str, set_err_no};
use shared::http::{HttpMethod, HttpVersion};
use shared::interop::deserialize;
use shared::MiddlewareResult;

use crate::util::get_slice_from_ptr_and_len_safe;

pub mod err_no;
pub mod memory;
mod util;
pub mod verify;

pub struct Request<'a> {
    body: &'a [u8],
    headers: HashMap<&'a str, &'a str>,
    version: HttpVersion,
    method: HttpMethod,
}

#[no_mangle]
pub extern "C" fn middleware(
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

    let Ok(headers) = deserialize(headers_slice) else {
        set_err_no(-1);
        set_err_msg_str("Could not deserialize raw headers");

        return MiddlewareResult::Error;
    };
    let Ok(arguments) = deserialize(arguments_slice) else {
        set_err_no(-1);
        set_err_msg_str("Could not deserialize raw arguments");

        return MiddlewareResult::Error;
    };

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
            set_err_msg_str(&format!("middleware: {:?}", err));

            MiddlewareResult::Error
        }
    }
}

#[inline]
fn handle_request_intern(request: Request<'static>, arguments: HashMap<&str, &str>) -> Result<()> {
    println!("Received body with size of {} bytes", request.body.len());
    println!(
        "Got the headers with the keys: {:?}",
        request.headers.keys()
    );
    println!(
        "Http: version = {:?}, method = {:?}",
        request.version, request.method
    );

    println!("arguments: {:?}", arguments);

    bail!("Some error happened here");

    Ok(())
}
