 # plctag-log
 log adapter for `libplctag`, one component of `libplctag` rust bindings

## Usage

please use it with [crate@plctag]

 by default, `libplctag` logs internal messages to stdout, if you set debug level other than none.
 you can register your own logger by calling [`register_logger`].
 For convenient, [`log_adapt`] register a logger for you and will forward internal log messages to crate`log`.

 ### Note
 `libplctag` will print log messages to stdout even if you register your own logger by `register_logger`.

 ### Examples
 ```rust
 use plctag_log::*;

 log_adapt(); //register logger
 set_debug_level(DebugLevel::Info); // set debug level

 // now, you can receive log messages by any of logging implementations of crate `log`
 ```
## License

MIT
