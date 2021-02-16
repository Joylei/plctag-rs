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
//! use plctag_async::{TagEntry, TagFactory, TagOptions, TagValue};
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
extern crate plctag;
extern crate plctag_sys;
extern crate uuid;

mod entry;
//mod event;
mod mailbox;

pub use entry::TagEntry;
use mailbox::Mailbox;
use may::coroutine::ParkError;
pub use plctag::{Status, TagValue};
use std::{
    fmt::{self, Display},
    sync::Arc,
};

pub type Result<T> = std::result::Result<T, Error>;

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

    /// create tag. When tag created, will connect automatically in the background forever
    #[inline]
    fn create<O: TagOptions>(&self, opts: O) -> TagEntry<O> {
        let path = opts.to_string();
        let token = mailbox::create(&self.mailbox, path);
        TagEntry::new(opts, token)
    }
}

impl Default for TagFactory {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum Error {
    /// tag error with status
    TagError(Status),
    /// coroutine park error
    ParkError,
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::TagError(_) => None,
            Error::ParkError => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::TagError(e) => fmt::Display::fmt(e, f),
            Error::ParkError => write!(f, "Coroutine Park Error"),
        }
    }
}

impl From<Status> for Error {
    fn from(s: Status) -> Self {
        Error::TagError(s)
    }
}

impl From<ParkError> for Error {
    fn from(e: ParkError) -> Self {
        Error::ParkError
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;
    use std::time::Duration;

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
    fn test_connected() {
        let factory = TagFactory::new();
        let tag = factory.create(DummyOptions {});
        let connected = tag.connect(Some(Duration::from_millis(150)));
        assert!(connected);

        let connected = tag.connect(Some(Duration::from_millis(150)));
        assert!(connected);

        let connected = tag.connect(Some(Duration::from_millis(150)));
        assert!(connected);
    }

    #[test]
    fn test_read_write() {
        let factory = TagFactory::new();
        let tag = factory.create(DummyOptions {});
        let connected = tag.connect(Some(Duration::from_millis(150)));
        assert!(connected);
        let level: i32 = tag.read_value(0).unwrap();
        assert_eq!(level, 4);

        tag.write_value(0, 1).unwrap();
        let level: i32 = tag.read_value(0).unwrap();
        assert_eq!(level, 1);
    }
}
