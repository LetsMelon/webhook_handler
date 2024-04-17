pub mod constants;
pub mod docker;
pub mod http;
pub mod interop;

#[derive(Debug)]
#[repr(C)]
pub enum MiddlewareResult {
    Continue = 0,
    Error,
}
