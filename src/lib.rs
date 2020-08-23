//! a rust wrapper of `libplctag`, with rust style Apis and useful extensions.
//!
//! # Safety
//! Operations in `libplctag` are guarded with mutex, so they are somewhat thread safe, also operations
//!  will block current thread for a while.
//! And imagine that one thread sets a value for a tag, another thread can set a different value for the same
//! tag once it acquires the mutex lock before the previous thread perform other operations on the tag.
//! It is that you still need some sync mechanism to make sure your sequence of operations
//! are atomic.
//!
#[macro_use]
extern crate lazy_static;
#[cfg(feature = "async")]
extern crate futures;
#[macro_use]
extern crate log;
extern crate parking_lot;
extern crate paste;
#[cfg(feature = "async")]
extern crate tokio;

pub(crate) mod controller;
pub(crate) mod debug;
pub(crate) mod event;
pub(crate) mod ffi;
pub mod options;
pub(crate) mod plc;
pub(crate) mod raw;
pub(crate) mod status;
pub(crate) mod tag;
pub(crate) mod value;

pub use debug::DebugLevel;
pub use raw::*;
pub use status::{Result, Status};
pub use value::{Bit, TagValue};

use futures::prelude::*;
use std::error;
use std::ffi::CString;
use std::ops::Deref;
use std::result;
use std::thread::sleep;
use std::time::Duration;
use tokio::prelude::*;

pub type AsyncResult<T> = result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub use plc::*;

pub mod prelude {
    pub use crate::raw::*;
    pub use crate::{Result, Status};
}

pub mod logging {
    use crate::plc;
    use std::ffi::CStr;
    use std::os::raw::c_char;

    #[no_mangle]
    unsafe extern "C" fn log_route(tag_id: i32, level: i32, message: *const c_char) {
        match level {
            1 => error!(
                "plctag: tag({}) - {}",
                tag_id,
                CStr::from_ptr(message).to_string_lossy()
            ),
            2 => warn!(
                "plctag: tag({}) - {}",
                tag_id,
                CStr::from_ptr(message).to_string_lossy()
            ),
            3 => info!(
                "plctag: tag({}) - {}",
                tag_id,
                CStr::from_ptr(message).to_string_lossy()
            ),
            4 => debug!(
                "plctag: tag({}) - {}",
                tag_id,
                CStr::from_ptr(message).to_string_lossy()
            ),
            5 => trace!(
                "plctag: tag({}) - {}",
                tag_id,
                CStr::from_ptr(message).to_string_lossy()
            ),
            _ => (),
        }
    }

    pub fn log_adapt() {
        unsafe {
            plc::unregister_logger();
            let rc = plc::register_logger(Some(log_route));
            info!("register logger: {}", rc);
        }
    }
}
