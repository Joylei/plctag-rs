#[macro_use]
extern crate lazy_static;
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
use may::coroutine::ParkError;
pub use plctag::{Status, TagValue};
use std::fmt::{self, Display};

pub type Result<T> = std::result::Result<T, Error>;

/// tag options;
/// impl Display to returns the platag required path
pub trait TagOptions: Display {
    /// unique key
    fn key(&self) -> &str;
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
    use may::coroutine as co;
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
    fn test_read_write() {
        let worker = go!(|| {
            let tag = TagEntry::new(DummyOptions {});
            co::sleep(Duration::from_millis(500));
            let level: i32 = tag.read_value(0).unwrap();
            assert_eq!(level, 4);

            tag.write_value(0, 1).unwrap();
            let level: i32 = tag.read_value(0).unwrap();
            assert_eq!(level, 1);
        });
        worker.join().unwrap();
    }
}
