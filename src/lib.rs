//! a rust wrapper of `libplctag`, with rust style APIs and useful extensions.
//!
//! # Features
//! - synchronous APIs
//! - asynchronous APIs based on `Tokio`; blocking operations are posted to `tokio::task::spawn_blocking`; asynchronous read/write based on event callback.
//! - tag path builder
//! - UDT support
//!
//! # Examples
//! ## read/write tag
//! ```rust,ignore
//! use plctag::{RawTag, TagValue, GetValue, SetValue};
//! let timeout = 100;//ms
//!
//! // YOUR TAG DEFINITION
//! let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16";
//! let tag = RawTag::new(path, timeout).unwrap();
//!
//! //read tag
//! let status = tag.read(timeout);
//! assert!(status.is_ok());
//! let offset = 0;
//! let value:u16 = tag.get_value(offset).unwrap();
//! println!("tag value: {}", value);
//!
//! let value = value + 10;
//! tag.set_value(offset, value).unwrap();
//!
//! //write tag
//! let status = tag.write(timeout);
//! assert!(status.is_ok());
//! println!("write done!");
//!
//! // tag will be destroyed when out of scope or manually call drop()
//! drop(tag);
//! ```
//!
//! ## async read/write tag
//!
//! ```rust,ignore
//! use plctag::future::AsyncTag;
//! use tokio::runtime::Runtime;
//!
//! let mut rt = Runtime::new()::unwrap();
//! rt.block_on(async move {
//!     // YOUR TAG DEFINITION
//!     let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16";
//!     let tag = AsyncTag::new(path).await.unwrap();
//!     
//!     let offset = 0;
//!     let value:u16 = 100;
//!     //write tag
//!     tag.set_and_write(offset, value).await.unwrap();
//!     // read tag
//!     let value:u16 = tag.read_and_get(offset).await.unwrap();
//!     assert_eq!(value, 100);
//! });
//!
//! ```
//!
//! ## UDT
//! read/write UDT
//! ```rust, ignore
//! use plctag::{Accessor, TagValue, RawTag, GetValue, SetValue, Result};
//!
//! // define your UDT
//! #[derive(Default, Debug)]
//! struct MyUDT {
//!     v1:u16,
//!     v2:u16,
//! }
//! impl TagValue for MyUDT {
//!     fn get_value(&mut self, accessor: &dyn Accessor, offset: u32) -> Result<()>{
//!         self.v1.get_value(accessor, offset)?;
//!         self.v2.get_value(accessor, offset + 2)?;
//!         Ok(())
//!     }
//!
//!     fn set_value(&self, accessor: &dyn Accessor, offset: u32) -> Result<()>{
//!         self.v1.set_value(accessor, offset)?;
//!         self.v2.set_value(accessor, offset + 2)?;
//!         Ok(())
//!     }
//! }
//!
//! fn main(){
//!     let timeout = 100;//ms
//!     let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag2&elem_count=2&elem_size=16";// YOUR TAG DEFINITION
//!     let tag = RawTag::new(path, timeout).unwrap();
//!
//!     //read tag
//!     let status = tag.read(timeout);
//!     assert!(status.is_ok());
//!     let offset = 0;
//!     let mut value:MyUDT = tag.get_value(offset).unwrap();
//!     println!("tag value: {:?}", value);
//!
//!     value.v1 = value.v1 + 10;
//!     tag.set_value(offset, value).unwrap();
//!
//!     //write tag
//!     let status = tag.write(timeout);
//!     assert!(status.is_ok());
//!     println!("write done!");
//! }
//!
//! ```
//!
//! ## Builder
//! ```rust,ignore
//! use plctag::builder::*;
//!
//! fn main() {
//!     let timeout = 100;
//!     let res = TagBuilder::new()
//!         .config(|builder| {
//!             builder
//!                 .protocol(Protocol::EIP)
//!                 .gateway("192.168.1.120")
//!                 .plc(PlcKind::ControlLogix)
//!                 .name("MyTag1")
//!                 .element_size(16)
//!                 .element_count(1)
//!                 .path("1,0")
//!                 .read_cache_ms(0);
//!         })
//!         .create(timeout);
//!     assert!(res.is_ok());
//!     let tag = res.unwrap();
//!     let status = tag.status();
//!     assert!(status.is_ok());
//! }
//! ```
//!
//! ## Logging adapter for `libplctag`
//! ```rust,ignore
//! use plctag::logging::log_adapt;
//! use plctag::plc::set_debug_level;
//! use plctag::DebugLevel;
//!
//! log_adapt(); //register logger
//! set_debug_level(DebugLevel::Info); // set debug level
//!
//! // now, you can receive log messages by any of logging implementations of crate `log`
//!
//! ```
//!
//! # Thread-safety
//! Operations in `libplctag` are guarded with mutex, so they are somewhat thread safe, also most operations
//!  will block current thread for a short while.
//! But imagine that one thread sets a value for a tag, another thread can set a different value for the same
//! tag once it acquires the mutex lock before the previous thread perform other operations on the tag.
//! It is that you still need some sync mechanism to make sure your sequence of operations
//! are atomic.
//!

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

pub mod builder;
pub(crate) mod debug;
pub mod error;
pub(crate) mod ffi;
#[cfg(feature = "async")]
pub mod future;
pub mod plc;
pub(crate) mod raw;
pub(crate) mod status;
#[cfg(any(feature = "async", feature = "value"))]
pub(crate) mod value;

pub use debug::DebugLevel;
pub use raw::RawTag;
pub use status::Status;

#[cfg(any(feature = "async", feature = "value"))]
pub use value::{Accessor, Bit, GetValue, SetValue, TagValue};

pub type Result<T> = std::result::Result<T, error::Error>;

pub mod prelude {
    pub use crate::builder::{PathBuilder, TagBuilder};
    #[cfg(any(feature = "async", feature = "value"))]
    pub use crate::{Accessor, Bit, GetValue, SetValue, TagValue};
    pub use crate::{DebugLevel, RawTag, Result, Status};
}

/// handle internal logs of `libplctag`
pub mod logging {
    use crate::plc;
    use crate::status;
    use std::ffi::CStr;
    use std::os::raw::c_char;

    #[doc(hidden)]
    #[no_mangle]
    unsafe extern "C" fn log_route(_tag_id: i32, level: i32, message: *const c_char) {
        let msg = CStr::from_ptr(message).to_string_lossy();
        match level {
            1 => error!("{}", msg),
            2 => warn!("{}", msg),
            3 => info!("{}", msg),
            4 => debug!("{}", msg),
            5 => trace!("{}", msg),
            _ => (),
        }
    }

    /// by default, `libplctag` logs internal messages to std output.
    /// you can register your own logger by calling [plc::register_logger](../plc/fn.register_logger.html).
    /// For convenient, this method will register a logger for you and route log messages to crate`log`.
    ///
    /// # Note
    /// `libplctag` will print logs to stdout even if you register your own logger by `plc::register_logger`
    ///
    /// # Examples
    /// ```rust,ignore
    /// use plctag::logging::log_adapt;
    /// use plctag::plc::set_debug_level;
    /// use plctag::DebugLevel;
    ///
    /// log_adapt(); //register logger
    /// set_debug_level(DebugLevel::Info); // set debug level
    ///
    /// // now, you can receive log messages by any of logging implementations of crate `log`
    ///
    /// ```
    pub fn log_adapt() {
        unsafe {
            plc::unregister_logger();
            let rc = plc::register_logger(Some(log_route));
            debug_assert_eq!(rc, status::PLCTAG_STATUS_OK);
            info!("register logger for libplctag: {}", rc);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::plc;
        use crate::DebugLevel;
        use crate::RawTag;
        use log::*;
        use std::sync::{Arc, Mutex};

        struct MemLogger {
            buf: Arc<Mutex<Vec<String>>>,
        }

        impl MemLogger {
            fn new() -> Self {
                Self {
                    buf: Arc::new(Mutex::new(vec![])),
                }
            }

            fn buf(&self) -> Vec<String> {
                self.buf.lock().unwrap().clone()
            }

            fn init(&self) {
                log::set_max_level(LevelFilter::Trace);
                let _ = log::set_boxed_logger(Box::new(self.clone()));
            }
        }

        impl Clone for MemLogger {
            fn clone(&self) -> Self {
                Self {
                    buf: self.buf.clone(),
                }
            }
        }

        impl Log for MemLogger {
            fn enabled(&self, meta: &log::Metadata<'_>) -> bool {
                meta.level() <= Level::Error
            }
            fn log(&self, record: &log::Record<'_>) {
                self.buf
                    .lock()
                    .unwrap()
                    .push(format!("{} - {}", record.target(), record.args()));
            }
            fn flush(&self) {}
        }

        #[test]
        fn test_log_adapt() {
            let logger = MemLogger::new();
            logger.init();
            log_adapt();
            plc::set_debug_level(DebugLevel::Detail);

            let res = RawTag::new("make=system&family=library&name=debug&debug=4", 100);
            assert!(res.is_ok());
            let tag = res.unwrap();
            let status = tag.status();
            assert!(status.is_ok());

            let buf = logger.buf();
            assert!(buf.len() > 0);
            let msg = buf.join("\r\n");
            assert!(msg.contains("plc_tag_create"));
        }
    }
}
