//! # plctag-co
//!
//! coroutine wrapper based on [plctag-rs](../plctag).
//!
//! ## Usage
//! Download latest binary release of [libplctag](https://github.com/libplctag/libplctag/releases) and extract it to somewhere of your computer.
//!
//! Set environment variable `LIBPLCTAG_PATH` to the directory of extracted binaries.
//!
//! Add `plctag` to your Cargo.toml
//!
//! ```toml
//! [dependencies]
//! plctag= { git="https://github.com/Joylei/plctag-rs.git", path="plctag-co"}
//! may="*"
//! ```
//!
//! ## Examples
//!
//!  ```rust,ignore
//! use plctag_async::{TagEntry, TagFactory, TagOptions, GetValue, SetValue};
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
//! let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16";// YOUR TAG DEFINITION
//!
//! let factory = TagFactory::new();
//! let opts = MyTagOptions {
//!     key: String::from("192.168.1.120;MyTag1"),
//!     path: path.to_owned(),
//! };
//! let tag = factory.create(opts);
//! let connected = tag.connect(Some(Duration::from_millis(150)));
//! assert!(connected);
//! let offset = 0;
//! let value:u16 = tag.read_value(offset).unwrap();
//! println!("tag value: {}", value);
//!
//! let value = value + 10;
//! tag.write_value(offset, value).unwrap();
//!  ```

#[macro_use]
extern crate log;
#[macro_use]
extern crate may;
extern crate once_cell;
pub extern crate plctag;

mod entry;
//mod event;
mod cell;
mod op;
mod pool;

pub use entry::TagEntry;
use may::coroutine::ParkError;
pub use op::AsyncTag;
pub use plctag::{GetValue, RawTag, SetValue, Status};
use std::{
    fmt,
    sync::{Arc, PoisonError},
};

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
    /// tag error with status
    TagError(Status),
    /// coroutine park error
    ParkError,
    PoisonError,
    Timeout,
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
            Error::ParkError => write!(f, "Coroutine Park Error"),
            Error::PoisonError => write!(f, "Lock Poisoned"),
            Error::Timeout => write!(f, "Operation Timeout"),
        }
    }
}

impl From<Status> for Error {
    fn from(s: Status) -> Self {
        Error::TagError(s)
    }
}

impl From<ParkError> for Error {
    fn from(_e: ParkError) -> Self {
        Error::ParkError
    }
}

impl<T> From<PoisonError<T>> for Error {
    fn from(_e: PoisonError<T>) -> Self {
        Error::PoisonError
    }
}

/// exclusive tag ref to ensure thread and operations safety
pub struct TagRef<'a, T> {
    tag: &'a T,
    #[allow(dead_code)]
    lock: may::sync::MutexGuard<'a, ()>,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn test_entry() -> anyhow::Result<()> {
        let path = "make=system&family=library&name=debug&debug=4";
        let entry = TagEntry::create(path, Some(Duration::from_millis(500)))?;
        let tag = entry.get()?;

        let level: i32 = tag.read_value(0, Some(Duration::from_millis(500)))?;
        assert_eq!(level, 4);

        tag.write_value(0, 1, Some(Duration::from_millis(500)))?;
        let level: i32 = tag.read_value(0, Some(Duration::from_millis(500)))?;
        assert_eq!(level, 1);
        Ok(())
    }

    #[test]
    fn test_pool() -> anyhow::Result<()> {
        let pool = Pool::new();
        let path = "make=system&family=library&name=debug&debug=4";

        //retrieve 1st
        {
            let entry = pool.entry(path, None)?;
            let tag = entry.get(None)?;

            let level: i32 = tag.read_value(0, None)?;
            assert_eq!(level, 4);

            tag.write_value(0, 1, None)?;
            let level: i32 = tag.read_value(0, None)?;
            assert_eq!(level, 1);
        }

        //retrieve 2nd
        {
            let entry = pool.entry(path, None)?;
            let tag = entry.get(None)?;

            let level: i32 = tag.read_value(0, None)?;
            assert_eq!(level, 1);
        }
        Ok(())
    }
}
