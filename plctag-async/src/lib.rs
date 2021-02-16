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
//!  ```rust
//! use plctag_async::{TagEntry, TagFactory, TagOptions, TagValue};
//! use tokio::runtime;
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

extern crate futures;
extern crate plctag;
extern crate tokio;
#[macro_use]
extern crate log;
use mailbox::Mailbox;
pub use plctag::{RawTag, Status, TagValue};
use std::{
    fmt::{self, Display},
    sync::Arc,
};
use task::JoinError;
use tokio::task;
mod cell;
mod entry;
mod mailbox;

pub use entry::TagEntry;

#[derive(Debug)]
pub enum Error {
    TagError(Status),
    TaskError(JoinError),
    RecvError,
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::TagError(_) => None,
            Error::TaskError(e) => Some(e),
            Error::RecvError => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::TagError(e) => fmt::Display::fmt(e, f),
            Error::TaskError(e) => fmt::Display::fmt(e, f),
            Error::RecvError => write!(f, "Channel Receive Error"),
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
        Error::TaskError(e)
    }
}

use std::result::Result as std_result;
pub type Result<T> = std_result<T, Error>;

/// tag options;
/// impl Display to returns the platag required path
pub trait TagOptions: Display {
    /// unique key
    fn key(&self) -> &str;
}

struct TagFactory {
    mailbox: Arc<Mailbox>,
}

impl TagFactory {
    #[inline]
    pub fn new() -> Self {
        Self {
            mailbox: Arc::new(Mailbox::new()),
        }
    }

    /// create tag. When tag created, will connect automatically in the background until connected
    #[inline]
    async fn create<O: TagOptions>(&self, opts: O) -> TagEntry<O> {
        let path = opts.to_string();
        let token = mailbox::create(&self.mailbox, path).await;
        TagEntry::new(opts, token)
    }
}

impl Default for TagFactory {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::fmt;

    use super::*;

    struct DummyOptions {}

    impl TagOptions for DummyOptions {
        fn key(&self) -> &str {
            "system-tag-debug"
        }
    }

    impl fmt::Display for DummyOptions {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "make=system&family=library&name=debug&debug=4")
        }
    }
    #[test]
    fn test_read_write() -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let factory = TagFactory::new();
            let tag = factory.create(DummyOptions {}).await;
            tag.connect().await;
            let level: i32 = tag.read_value(0).await?;
            assert_eq!(level, 4);

            tag.write_value(0, 1).await?;
            let level: i32 = tag.read_value(0).await?;
            assert_eq!(level, 1);
            Ok(())
        })
    }
}
