// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2020-2021, Joylei <leingliu@gmail.com>
// License: MIT

/*!
# plctag-async

tokio based async wrapper for `libplctag`.

[![crates.io](https://img.shields.io/crates/v/plctag-async.svg)](https://crates.io/crates/plctag-async)
[![docs](https://docs.rs/plctag-async/badge.svg)](https://docs.rs/plctag-async)
[![build](https://github.com/joylei/plctag-rs/workflows/Test%20and%20Build/badge.svg?branch=master)](https://github.com/joylei/plctag-rs/actions?query=workflow%3A%22Test+and+Build%22)
[![license](https://img.shields.io/crates/l/plctag.svg)](https://github.com/joylei/plctag-rs/blob/master/LICENSE)

## How to use

Add `plctag-async` to your Cargo.toml

```toml
[dependencies]
plctag-async= "0.1"
```

## Examples

```rust,ignore
use plctag_async::{AsyncTag, Error, TagEntry};
use tokio::runtime;

let rt = runtime::Runtime::new().unwrap()?;
rt.block_on(async {
   let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16";// YOUR TAG DEFINITION

   let tag = TagEntry::create(path).await.unwrap();
   let tag_ref = tag.get().await.unwrap();
   let offset = 0;
   let value:u16 = tag_ref.read_value(offset).await.unwrap();
   println!("tag value: {}", value);

   let value = value + 10;
   tag_ref.write_value(offset, value).await.unwrap();
});
```

## Thread-safety

It's thread-safe to perform operations with `plctag-async`.

## Build & Test

Please refer to `How to use` to setup build environment.

Because mutithread will cause troubles, you need to run tests with:

```shell
cargo test -- --test-threads=1
```

## License

MIT

*/
#![warn(missing_docs)]

extern crate plctag_core;
extern crate tokio;
#[macro_use]
extern crate log;
#[macro_use]
extern crate async_trait;

mod cell;
mod entry;
mod op;
mod pool;

pub use entry::TagEntry;
pub use op::AsyncTag;

use plctag_core::{RawTag, Status};
use std::{fmt, sync::Arc};
use tokio::task::{self, JoinError};

/// Tag instance will be put into pool for reuse.
///
/// # Note
/// - Tag instances will not drop if the [`PoolEntry`] or [`Pool`] is still on the stack
///
pub type Pool = pool::Pool<RawTag>;
/// Tag Instance in the pool
pub type PoolEntry = pool::Entry<RawTag>;
/// Tag exclusive reference
pub type TagRef<'a> = private::TagRef<'a, RawTag>;
/// result for [`plctag-async`]
pub type Result<T> = std::result::Result<T, Error>;

/// errors for [`plctag-async`]
#[derive(Debug)]
pub enum Error {
    /// plc tag error
    TagError(Status),
    /// tokio task join error
    JoinError(tokio::task::JoinError),
    /// other error
    Other(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::TagError(_) => None,
            Error::JoinError(e) => Some(e),
            Error::Other(e) => Some(e.as_ref()),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::TagError(e) => write!(f, "TagError - {}", e),
            Error::JoinError(e) => write!(f, "{}", e),
            Error::Other(e) => write!(f, "{}", e),
        }
    }
}

impl From<Status> for Error {
    fn from(s: Status) -> Self {
        Error::TagError(s)
    }
}

impl From<JoinError> for Error {
    fn from(e: JoinError) -> Self {
        Error::JoinError(e)
    }
}

mod private {
    /// exclusive tag ref to ensure thread and operations safety
    pub struct TagRef<'a, T> {
        pub(crate) tag: &'a T,
        #[allow(dead_code)]
        pub(crate) lock: tokio::sync::MutexGuard<'a, ()>,
    }

    impl<T> AsRef<T> for TagRef<'_, T> {
        #[inline(always)]
        fn as_ref(&self) -> &T {
            &self.tag
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_entry() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let path = "make=system&family=library&name=debug&debug=4";
            let entry = TagEntry::create(path).await?;
            let tag = entry.get().await?;

            let level: i32 = tag.read_value(0).await?;
            assert_eq!(level, 4);

            tag.write_value(0, 1).await?;
            let level: i32 = tag.read_value(0).await?;
            assert_eq!(level, 1);
            Ok(())
        })
    }

    #[test]
    fn test_pool() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let pool = Pool::new();
            let path = "make=system&family=library&name=debug&debug=4";

            //retrieve 1st
            {
                let entry = pool.entry(path).await?;
                let tag = entry.get().await?;

                let level: i32 = tag.read_value(0).await?;
                assert_eq!(level, 4);

                tag.write_value(0, &1_i32).await?;
                let level: i32 = tag.read_value(0).await?;
                assert_eq!(level, 1);
            }

            //retrieve 2nd
            {
                let entry = pool.entry(path).await?;
                let tag = entry.get().await?;

                let level: i32 = tag.read_value(0).await?;
                assert_eq!(level, 1);
            }
            Ok(())
        })
    }
}
