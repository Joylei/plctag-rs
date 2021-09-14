# plctag-sys

native libplctag binding

[![crates.io](https://img.shields.io/crates/v/plctag-sys.svg)](https://crates.io/crates/plctag-sys)
[![docs](https://docs.rs/plctag-sys/badge.svg)](https://docs.rs/plctag-sys)
[![build](https://github.com/joylei/plctag-rs/workflows/build/badge.svg?branch=master)](https://github.com/joylei/plctag-rs/actions?query=workflow%3A%22build%22)
[![license](https://img.shields.io/crates/l/plctag.svg)](https://github.com/joylei/plctag-rs/blob/master/LICENSE)

## Build

You can build from source or use pre-built binaries. It depends on how you set ENV variables.

- Use pre-built binaries
- Build from git submodules

## Use pre-built binaries

Download latest binary release of [libplctag](https://github.com/libplctag/libplctag/releases) and extract it to somewhere of your computer.

Set environment variable `LIBPLCTAG_PATH` to the directory of extracted binaries.

## Build from git submodules

If environment variable `LIBPLCTAG_PATH` does not present, will build from git submodules [libplctag](https://github.com/libplctag/libplctag/).
