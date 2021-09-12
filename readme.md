# plctag-rs

a rust wrapper of [libplctag](https://github.com/libplctag/libplctag), with rust style APIs and useful extensions.

## crates

- [plctag](./) reexports everything from below crates.
- [plctag-sys](./crates/sys) native libplctag binding
- [plctag-core](./crates/core) a rust wrapper of [libplctag](https://github.com/libplctag/libplctag), with rust style APIs and useful extensions.
- [plctag-async](./crates/async) tokio based async wrapper.
- [plctag-log](./crates/log) log adapter for `libplctag`
- [plctag-derive](./crates/derive) macros for plctag-rs

## License

MIT
