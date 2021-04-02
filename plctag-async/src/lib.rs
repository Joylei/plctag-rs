//! # plctag-async
//!
//! async wrapper based on [plctag-rs](../plctag).
//!
//! ## How to use
//!
//! Download latest binary release of [libplctag](https://github.com/libplctag/libplctag/releases) and extract it to somewhere of your computer.
//!
//! Set environment variable `LIBPLCTAG_PATH` to the directory of extracted binaries.
//!
//! Add `plctag` to your Cargo.toml
//!
//! ```toml
//! [dependencies]
//! plctag= { git="https://github.com/Joylei/plctag-rs.git", path="plctag-async"}
//! ```
//!
//! You're OK to build your project.
//!
//! ## Examples
//!
//!  ```rust,ignore
//! use plctag_async::{TagEntry, TagFactory, TagOptions, GetValue, SetValue};
//! use tokio::runtime;
//! use std::fmt;
//!
//! struct MyTagOptions {
//!     pub key: String,
//!     pub path: String,
//! }
//!
//! impl TagOptions for MyTagOptions {
//!     fn key(&self)->&str{
//!         &self.key
//!     }
//! }
//!
//! impl fmt::Display for MyTagOptions{
//!     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//!         write!(f, "{}", self.path)
//!     }
//! }
//!
//! let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16";// YOUR TAG DEFINITION
//!
//! let rt = runtime::Runtime::new().unwrap()?;
//! rt.block_on(async {
//!     let factory = TagFactory::new();
//!     let opts = MyTagOptions {
//!         key: String::from("192.168.1.120;MyTag1"),
//!         path: path.to_owned(),
//!     };
//!     let tag = factory.create(opts).await;
//!     tag.connect().await;
//!     let offset = 0;
//!     let value:u16 = tag.read_value(offset).await.unwrap();
//!     println!("tag value: {}", value);
//!
//!     let value = value + 10;
//!     tag.write_value(offset).await.unwrap();
//! });
//!  ```

pub extern crate plctag;
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

use plctag::{RawTag, Status};
use std::{fmt, sync::Arc};
use tokio::task::{self, JoinError};

/// Tag instance will be put into pool for reuse.
///
/// # Note
/// - Tag instances will not drop if the [`PoolEntry`] or [`Pool`] is still on the stack
///
/// ---
/// To remove tag instance from [`Pool`], you can call [`Pool::remove`]
pub type Pool = pool::Pool<RawTag>;
pub type PoolEntry = pool::Entry<RawTag>;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub enum Error {
    TagError(Status),
    JoinError,
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::TagError(e) => fmt::Display::fmt(e, f),
            Error::JoinError => write!(f, "Task Join Error"),
        }
    }
}

impl From<Status> for Error {
    fn from(s: Status) -> Self {
        Error::TagError(s)
    }
}

impl From<JoinError> for Error {
    fn from(_e: JoinError) -> Self {
        Error::JoinError
    }
}

/// exclusive tag ref to ensure thread and operations safety
pub struct TagRef<'a, T> {
    tag: &'a T,
    #[allow(dead_code)]
    lock: tokio::sync::MutexGuard<'a, ()>,
}

impl<T> AsRef<T> for TagRef<'_, T> {
    #[inline(always)]
    fn as_ref(&self) -> &T {
        &self.tag
    }
}

#[cfg(test)]
mod tests {
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
