# plctag-sys

native libplctag binding

## Build

You can build from source or use pre-built binaries. It depends on how you set ENV variables.

- Use pre-built binaries
- Build from external source
- Build from embedded source

 ## Use pre-built binaries

Download latest binary release of [libplctag](https://github.com/libplctag/libplctag/releases) and extract it to somewhere of your computer.

Set environment variable `LIBPLCTAG_PATH` to the directory of extracted binaries.

## Build from external source
Set environment variable `LIBPLCTAG_SOURCE` to the directory of source code of [libplctag](https://github.com/libplctag/libplctag/).

## Build from embedded source
If environment variable `LIBPLCTAG_PATH` is not present, will build from embedded source of [libplctag](https://github.com/libplctag/libplctag/).