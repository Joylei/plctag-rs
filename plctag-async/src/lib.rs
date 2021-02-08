extern crate futures;
#[macro_use]
extern crate lazy_static;
extern crate plctag;
extern crate tokio;
#[macro_use]
extern crate log;
use mailbox::Mailbox;
use oneshot::error::RecvError;
pub use plctag::{RawTag, Status, TagValue};
use std::{
    fmt::{self, Display},
    sync::Arc,
};
use task::JoinError;
use tokio::{sync::oneshot, task};
mod entry;
//mod event;
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

impl From<RecvError> for Error {
    fn from(e: RecvError) -> Self {
        Error::RecvError
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

    #[inline]
    async fn create<O: TagOptions>(&self, opts: O) -> TagEntry<O> {
        let path = opts.to_string();
        let token = self.mailbox.create(path).await;
        TagEntry::new(opts, token)
    }
}

impl Default for TagFactory {
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
            tag.wait_ready().await;
            let level: i32 = tag.read_value(0).await?;
            assert_eq!(level, 4);

            tag.write_value(0, 1).await?;
            let level: i32 = tag.read_value(0).await?;
            assert_eq!(level, 1);
            Ok(())
        })
    }
}