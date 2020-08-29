//! a rust wrapper of `libplctag`, with rust style Apis and useful extensions.
//!
//! # Thread-safety
//! Operations in `libplctag` are guarded with mutex, so they are somewhat thread safe, also most operations
//!  will block current thread for a short while.
//! And imagine that one thread sets a value for a tag, another thread can set a different value for the same
//! tag once it acquires the mutex lock before the previous thread perform other operations on the tag.
//! It is that you still need some sync mechanism to make sure your sequence of operations
//! are atomic.
//!
//! # Basic Usage
//! ```
//! extern crate plgtag;
//! use plctag::RawTag;
//!
//! // some tag path definition
//! let path = "protocol=ab-eip&plc=controllogix&gateway=192.168.1.120&path=1,0&name=MyTag&elem_count=1";
//! let timeout = 100; // in ms
//! let res = RawTag::new(path, timeout);
//! let status = tag.read(timeout);
//! if status.is_ok()
//! ```

#[cfg(feature = "async")]
#[macro_use]
extern crate lazy_static;
#[cfg(feature = "async")]
extern crate futures;
#[macro_use]
extern crate log;
#[cfg(feature = "async")]
extern crate parking_lot;
#[cfg(any(feature = "async", feature = "value"))]
extern crate paste;
#[cfg(feature = "async")]
extern crate tokio;

pub(crate) mod debug;
pub(crate) mod ffi;
#[cfg(feature = "async")]
pub mod future;
pub mod options;
pub(crate) mod plc;
pub(crate) mod raw;
pub(crate) mod status;
#[cfg(any(feature = "async", feature = "value"))]
pub(crate) mod value;

pub use debug::DebugLevel;
pub use raw::*;
pub use status::{Result, Status};

#[cfg(any(feature = "async", feature = "value"))]
pub use value::{Bit, TagValue};

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
                "libplctag: tag({}) - {}",
                tag_id,
                CStr::from_ptr(message).to_string_lossy()
            ),
            2 => warn!(
                "libplctag: tag({}) - {}",
                tag_id,
                CStr::from_ptr(message).to_string_lossy()
            ),
            3 => info!(
                "libplctag: tag({}) - {}",
                tag_id,
                CStr::from_ptr(message).to_string_lossy()
            ),
            4 => debug!(
                "libplctag: tag({}) - {}",
                tag_id,
                CStr::from_ptr(message).to_string_lossy()
            ),
            5 => trace!(
                "libplctag: tag({}) - {}",
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
            info!("register logger for libplctag: {}", rc);
        }
    }
}
