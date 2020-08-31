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
pub(crate) mod error;
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

/// handle inner log of `libplctag`
pub mod logging {
    use crate::plc;
    use std::ffi::CStr;
    use std::os::raw::c_char;

    #[doc(hidden)]
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

    /// by default, `libplctag` logs inner messages to std output.
    /// you can register your own logger by calling [plc::register_logger](../plc/fn.register_logger.html).
    /// this method will register a logger for you and route log messages to logging crate`log`.
    pub fn log_adapt() {
        unsafe {
            plc::unregister_logger();
            let rc = plc::register_logger(Some(log_route));
            info!("register logger for libplctag: {}", rc);
        }
    }
}
