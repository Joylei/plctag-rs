// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2022, Joylei <leingliu@gmail.com>
// License: MIT

/*!
# plctag-log

log adapter for `libplctag`, one component of `plctag` rust bindings

[![crates.io](https://img.shields.io/crates/v/plctag-log.svg)](https://crates.io/crates/plctag-log)
[![docs](https://docs.rs/plctag-log/badge.svg)](https://docs.rs/plctag-log)
[![build](https://github.com/joylei/plctag-rs/workflows/build/badge.svg?branch=master)](https://github.com/joylei/plctag-rs/actions?query=workflow%3A%22build%22)
[![license](https://img.shields.io/crates/l/plctag.svg)](https://github.com/joylei/plctag-rs/blob/master/LICENSE)

## Usage

please use it with [plctag](https://crates.io/crates/plctag)

by default, `libplctag` logs internal messages to stdout, if you set debug level other than none.
you can register your own logger by calling [`register_logger`].
For convenient, [`log_adapt`] register a logger for you and will forward internal log messages to crate`log`.

Add `plctag-log` to your Cargo.toml

```toml
[dependencies]
plctag-log= "0.1"
```

### Note

`libplctag` will print log messages to stdout even if you register your own logger by `register_logger`.

### Examples

```rust,no_run
use plctag_log::*;

log_adapt(); //register logger
set_debug_level(DebugLevel::Info); // set debug level

// now, you can receive log messages by any of logging implementations of crate `log`
```

## License

MIT

*/
#![warn(missing_docs)]

extern crate plctag_core;
#[macro_use]
extern crate log;

pub use plctag_core::builder::DebugLevel;

use plctag_core::ffi;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// set debug level of `libplctag`
///
/// #Note
/// `libplctag` will print logs to stdout even if you register your own logger by `plc::register_logger`
#[inline]
pub fn set_debug_level(debug: DebugLevel) {
    let level = debug as u8;
    unsafe { ffi::plc_tag_set_debug_level(level as i32) };
}

/// retrieve debug level
#[inline(always)]
pub fn get_debug_level() -> DebugLevel {
    let v = get_int_attr("debug");
    (v as u8).into()
}

/// register a custom logger to receive inner message of `libplctag`
///
/// # Note
/// `libplctag` will print logs to stdout even if you register your own logger by `register_logger`
pub use ffi::plc_tag_register_logger as register_logger;
pub use ffi::plc_tag_unregister_logger as unregister_logger;

#[inline(always)]
fn get_int_attr(attr: &str) -> i32 {
    let attr = CString::new(attr).unwrap();
    unsafe { ffi::plc_tag_get_int_attribute(0, attr.as_ptr(), 0) }
}

#[doc(hidden)]
unsafe extern "C" fn log_route(_tag_id: i32, level: i32, message: *const c_char) {
    let msg = CStr::from_ptr(message).to_string_lossy();
    match level {
        1 => error!("{}", msg),
        2 => warn!("{}", msg),
        3 => info!("{}", msg),
        4 => debug!("{}", msg),
        5 => trace!("{}", msg),
        6 => trace!("{}", msg),
        _ => (),
    }
}

/// by default, `libplctag` logs internal messages to stdout, if you set debug level other than none.
/// you can register your own logger by calling [`register_logger`].
/// For convenient, this method will register a logger for you and will forward internal log messages to crate`log`.
///
/// # Note
/// `libplctag` will print log messages to stdout even if you register your own logger by `register_logger`.
///
/// # Examples
/// ```rust,no_run
/// use plctag_log::*;
///
/// log_adapt(); //register logger
/// set_debug_level(DebugLevel::Info); // set debug level
///
/// // now, you can receive log messages by any of logging implementations of crate `log`
///
/// ```
pub fn log_adapt() {
    unsafe {
        ffi::plc_tag_unregister_logger();
        let rc = ffi::plc_tag_register_logger(Some(log_route));
        debug_assert_eq!(rc, ffi::PLCTAG_STATUS_OK as i32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::*;
    use plctag_core::RawTag;
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
        set_debug_level(DebugLevel::Detail);

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
