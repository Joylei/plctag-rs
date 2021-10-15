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
plctag-log= "0.2"
```

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
