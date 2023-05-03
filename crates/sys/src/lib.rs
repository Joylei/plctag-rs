// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2022, Joylei <leingliu@gmail.com>
// License: MIT

/*!
# plctag-sys

native libplctag binding

[![crates.io](https://img.shields.io/crates/v/plctag-sys.svg)](https://crates.io/crates/plctag-sys)
[![docs](https://docs.rs/plctag-sys/badge.svg)](https://docs.rs/plctag-sys)
[![build](https://github.com/joylei/plctag-rs/workflows/build/badge.svg?branch=master)](https://github.com/joylei/plctag-rs/actions?query=workflow%3A%22build%22)
[![license](https://img.shields.io/crates/l/plctag.svg)](https://github.com/joylei/plctag-rs/blob/master/LICENSE)

## Build

You can build from source or use pre-built binaries. It depends on how you set ENV variables.

- Use pre-built binaries
- Build from external source
- Build from embedded source

## Use pre-built binaries

Download latest binary release of [libplctag](https://github.com/libplctag/libplctag/releases) and extract it to somewhere of your computer.

Set environment variable `LIBPLCTAG_PATH` to the directory of extracted binaries.

## Build from git submodules

If environment variable `LIBPLCTAG_PATH` does not present, will build from git submodules [libplctag](https://github.com/libplctag/libplctag/).

## Static build

env `PLCTAG_STATIC`: use static build, true if the value is one of `1`, `true`, `on`, `yes`
env `PLCTAG_DYNAMIC`: use dynamic build, true if the value is one of `1`, `true`, `on`, `yes`

Will check if static build in the order of:
`PLCTAG_STATIC, PLCTAG_DYNAMIC, rustflags: +crt-static`

*/
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
