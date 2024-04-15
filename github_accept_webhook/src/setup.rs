use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

use crate::err_no::{err_clear, set_err_msg_str, set_err_no};

#[repr(C)]
pub enum SetupResult {
    Ok = 0,
    Error,
}

#[no_mangle]
pub extern "C" fn _setup() -> SetupResult {
    err_clear();

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_filter(tracing_subscriber::filter::LevelFilter::TRACE);

    match tracing_subscriber::registry().with(fmt_layer).try_init() {
        Ok(_) => SetupResult::Ok,
        Err(err) => {
            set_err_no(1);
            set_err_msg_str(&format!("{:?}", err));

            SetupResult::Error
        }
    }
}
