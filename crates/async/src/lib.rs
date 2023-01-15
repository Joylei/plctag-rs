// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2022, Joylei <leingliu@gmail.com>
// License: MIT

/*!
# plctag-async

async wrapper for `libplctag`.

[![crates.io](https://img.shields.io/crates/v/plctag-async.svg)](https://crates.io/crates/plctag-async)
[![docs](https://docs.rs/plctag-async/badge.svg)](https://docs.rs/plctag-async)
[![build](https://github.com/joylei/plctag-rs/workflows/build/badge.svg?branch=master)](https://github.com/joylei/plctag-rs/actions?query=workflow%3A%22build%22)
[![license](https://img.shields.io/crates/l/plctag.svg)](https://github.com/joylei/plctag-rs/blob/master/LICENSE)

## How to use

Add `plctag-async` to your Cargo.toml

```toml
[dependencies]
plctag-async= "0.4"
```

## Examples

```rust,no_run
use plctag_async::{Error, AsyncTag};
use tokio::runtime;

let rt = runtime::Runtime::new().unwrap();
rt.block_on(async {
   let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16";// YOUR TAG DEFINITION

   let mut tag = AsyncTag::create(path).await.unwrap();
   let offset = 0;
   let value:u16 = tag.read_value(offset).await.unwrap();
   println!("tag value: {}", value);

   let value = value + 10;
   tag.write_value(offset, value).await.unwrap();
});
```

## License

MIT

*/
#![warn(missing_docs)]

extern crate plctag_core;
mod entry;

pub use entry::AsyncTag;

use plctag_core::{RawTag, Status};
use std::{fmt, sync::Arc};

/// result for [`plctag-async`]
pub type Result<T> = std::result::Result<T, Error>;

/// errors for [`plctag-async`]
#[derive(Debug)]
pub enum Error {
    /// plc tag error
    TagError(Status),
    /// other error
    Other(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::TagError(_) => None,
            Error::Other(e) => Some(e.as_ref()),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::TagError(e) => write!(f, "TagError - {}", e),
            Error::Other(e) => write!(f, "{}", e),
        }
    }
}

impl From<Status> for Error {
    fn from(s: Status) -> Self {
        Error::TagError(s)
    }
}
