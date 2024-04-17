/// The max size in bytes of the error message coming from wasm/wasi.
///
/// Be aware that the string is encoded as a `CStr`, so the last byte of the message is a `\0`.
/// So the max possible char length is `MAX_ERR_MSG_LEN` - 1.
pub const MAX_ERR_MSG_LEN: usize = 1 << 10;

pub const NO_ERROR: usize = 0;
