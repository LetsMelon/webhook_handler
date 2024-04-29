#[cfg(not(target_family = "wasm"))]
compile_error!("shared can only be compiled with a wasm target");

pub mod constants;
pub mod docker;
pub mod err_no;
pub mod http;
pub mod interop;
pub mod memory;
pub mod setup;

#[derive(Debug)]
#[repr(C)]
pub enum MiddlewareResult {
    Continue = 0,
    Error,
}
